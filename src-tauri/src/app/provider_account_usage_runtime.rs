//! Process-owned, demand-driven provider account-usage cache and scheduler.

use crate::domain::provider_account_usage::{
    ProviderAccountUsageRefreshSchedule, ProviderAccountUsageResult, ProviderAccountUsageStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{watch, Mutex, Semaphore};

const CONSUMER_LEASE: Duration = Duration::from_secs(15);
const SCHEDULER_TICK: Duration = Duration::from_secs(1);
pub(crate) const SUCCESS_CACHE_TTL_SECONDS: i64 = 60 * 60;
const MAX_CONCURRENT_PROVIDER_FETCHES: usize = 4;

#[derive(Clone)]
pub(crate) struct ProviderAccountUsageRuntimeState {
    shared: Arc<RuntimeShared>,
}

impl Default for ProviderAccountUsageRuntimeState {
    fn default() -> Self {
        Self {
            shared: Arc::new(RuntimeShared {
                inner: Mutex::new(RuntimeInner::default()),
                fetch_limiter: Arc::new(Semaphore::new(MAX_CONCURRENT_PROVIDER_FETCHES)),
            }),
        }
    }
}

struct RuntimeShared {
    inner: Mutex<RuntimeInner>,
    fetch_limiter: Arc<Semaphore>,
}

#[derive(Default)]
struct RuntimeInner {
    entries: HashMap<i64, RuntimeEntry>,
    scheduler_running: bool,
}

struct RuntimeEntry {
    schedule: Option<ProviderAccountUsageRefreshSchedule>,
    result: Option<ProviderAccountUsageResult>,
    last_attempt_at: Option<i64>,
    desktop_lease_until: Option<Instant>,
    tui_lease_until: Option<Instant>,
    generation: u64,
    in_flight_generation: Option<u64>,
    completion_generation: u64,
    completion: watch::Sender<u64>,
}

impl Default for RuntimeEntry {
    fn default() -> Self {
        let (completion, _) = watch::channel(0);
        Self {
            schedule: None,
            result: None,
            last_attempt_at: None,
            desktop_lease_until: None,
            tui_lease_until: None,
            generation: 0,
            in_flight_generation: None,
            completion_generation: 0,
            completion,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderAccountUsageTarget {
    pub provider_id: i64,
    pub schedule: ProviderAccountUsageRefreshSchedule,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProviderAccountUsageDisplaySnapshot {
    pub state: &'static str,
    pub amount: Option<f64>,
    pub unit: Option<String>,
    pub last_fetched_at: Option<i64>,
}

enum RefreshDecision {
    Cached,
    Lead(u64),
    Wait {
        receiver: watch::Receiver<u64>,
        completion_generation: u64,
    },
}

impl ProviderAccountUsageRuntimeState {
    pub(crate) async fn touch_desktop(
        &self,
        app: &tauri::AppHandle,
        target: ProviderAccountUsageTarget,
    ) {
        self.touch(app, std::iter::once(target), ConsumerKind::Desktop)
            .await;
    }

    pub(crate) async fn touch_tui(
        &self,
        app: &tauri::AppHandle,
        targets: impl IntoIterator<Item = ProviderAccountUsageTarget>,
    ) {
        self.touch(app, targets, ConsumerKind::Tui).await;
    }

    async fn touch(
        &self,
        app: &tauri::AppHandle,
        targets: impl IntoIterator<Item = ProviderAccountUsageTarget>,
        consumer: ConsumerKind,
    ) {
        let lease_until = Instant::now() + CONSUMER_LEASE;
        let mut inner = self.shared.inner.lock().await;
        for target in targets {
            if target.provider_id <= 0 {
                continue;
            }
            let entry = inner.entries.entry(target.provider_id).or_default();
            if entry.schedule != Some(target.schedule) {
                invalidate_entry_for_schedule(entry, target.schedule);
            }
            match consumer {
                ConsumerKind::Desktop => entry.desktop_lease_until = Some(lease_until),
                ConsumerKind::Tui => entry.tui_lease_until = Some(lease_until),
            }
        }
        let should_start = !inner.scheduler_running
            && inner
                .entries
                .values()
                .any(|entry| entry_has_active_consumer(entry, Instant::now()));
        if should_start {
            inner.scheduler_running = true;
        }
        drop(inner);

        if should_start {
            let state = self.clone();
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                state.run_scheduler(app).await;
            });
        }
    }

    pub(crate) async fn invalidate(&self, provider_id: i64) {
        let mut inner = self.shared.inner.lock().await;
        if let Some(entry) = inner.entries.get_mut(&provider_id) {
            entry.generation = entry.generation.wrapping_add(1);
            entry.schedule = None;
            entry.result = None;
            entry.last_attempt_at = None;
            entry.desktop_lease_until = None;
            entry.tui_lease_until = None;
            if entry.in_flight_generation.is_none() {
                publish_completion(entry);
            }
        }
    }

    pub(crate) async fn fetch(
        &self,
        app: tauri::AppHandle,
        provider_id: i64,
        force: bool,
    ) -> ProviderAccountUsageResult {
        let mut force = force;
        loop {
            match self.begin_refresh(provider_id, force).await {
                RefreshDecision::Cached => return self.cached_result(provider_id).await,
                RefreshDecision::Lead(generation) => {
                    self.perform_refresh(app.clone(), provider_id, generation)
                        .await;
                    force = false;
                }
                RefreshDecision::Wait {
                    mut receiver,
                    completion_generation,
                } => {
                    while *receiver.borrow() < completion_generation {
                        if receiver.changed().await.is_err() {
                            break;
                        }
                    }
                    // A forced request coalesces with the request that was already in flight.
                    force = false;
                }
            }
        }
    }

    pub(crate) async fn display_snapshots(
        &self,
        provider_ids: &[i64],
        now_unix: i64,
    ) -> HashMap<i64, ProviderAccountUsageDisplaySnapshot> {
        let inner = self.shared.inner.lock().await;
        provider_ids
            .iter()
            .filter_map(|provider_id| {
                let entry = inner.entries.get(provider_id)?;
                entry.schedule?;
                Some((*provider_id, display_snapshot(entry, now_unix)))
            })
            .collect()
    }

    async fn begin_refresh(&self, provider_id: i64, force: bool) -> RefreshDecision {
        let now_unix = crate::shared::time::now_unix_seconds();
        let mut inner = self.shared.inner.lock().await;
        let Some(entry) = inner.entries.get_mut(&provider_id) else {
            return RefreshDecision::Cached;
        };
        if entry.schedule.is_none() {
            return RefreshDecision::Cached;
        }
        if entry.in_flight_generation.is_some() {
            return RefreshDecision::Wait {
                receiver: entry.completion.subscribe(),
                completion_generation: entry.completion_generation.wrapping_add(1),
            };
        }
        if !force && !entry_is_due(entry, now_unix) {
            return RefreshDecision::Cached;
        }

        let generation = entry.generation;
        entry.in_flight_generation = Some(generation);
        entry.last_attempt_at = Some(now_unix);
        RefreshDecision::Lead(generation)
    }

    async fn perform_refresh(&self, app: tauri::AppHandle, provider_id: i64, generation: u64) {
        let result = match self.shared.fetch_limiter.clone().acquire_owned().await {
            Ok(permit) => {
                let result =
                    crate::commands::providers::fetch_account_usage_uncached(app, provider_id)
                        .await;
                drop(permit);
                result
            }
            Err(_) => Err("account usage scheduler is unavailable".to_string()),
        }
        .unwrap_or_else(|_| {
            ProviderAccountUsageResult::local_status(
                None,
                ProviderAccountUsageStatus::QueryFailed,
                "账户用量查询失败",
            )
        });

        let mut inner = self.shared.inner.lock().await;
        let Some(entry) = inner.entries.get_mut(&provider_id) else {
            return;
        };
        if entry.in_flight_generation != Some(generation) {
            return;
        }
        entry.in_flight_generation = None;
        if entry.generation == generation && entry.schedule.is_some() {
            entry.result = Some(result);
        }
        publish_completion(entry);
    }

    async fn cached_result(&self, provider_id: i64) -> ProviderAccountUsageResult {
        let now_unix = crate::shared::time::now_unix_seconds();
        let inner = self.shared.inner.lock().await;
        let Some(entry) = inner.entries.get(&provider_id) else {
            return unavailable_result();
        };
        let Some(result) = entry.result.as_ref() else {
            return unavailable_result();
        };
        if is_success_result(result) && !is_fresh_success(result, now_unix) {
            return ProviderAccountUsageResult::local_status(
                result.adapter_kind,
                ProviderAccountUsageStatus::QueryFailed,
                "账户用量缓存已过期",
            );
        }
        result.clone()
    }

    async fn run_scheduler(self, app: tauri::AppHandle) {
        loop {
            let due = self.collect_due_provider_ids().await;
            let Some(due) = due else {
                return;
            };
            for provider_id in due {
                let state = self.clone();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = state.fetch(app, provider_id, false).await;
                });
            }
            tokio::time::sleep(SCHEDULER_TICK).await;
        }
    }

    async fn collect_due_provider_ids(&self) -> Option<Vec<i64>> {
        let now = Instant::now();
        let now_unix = crate::shared::time::now_unix_seconds();
        let mut inner = self.shared.inner.lock().await;
        for entry in inner.entries.values_mut() {
            if entry
                .desktop_lease_until
                .is_some_and(|deadline| deadline <= now)
            {
                entry.desktop_lease_until = None;
            }
            if entry
                .tui_lease_until
                .is_some_and(|deadline| deadline <= now)
            {
                entry.tui_lease_until = None;
            }
        }

        let active = inner
            .entries
            .values()
            .any(|entry| entry_has_active_consumer(entry, now));
        if !active {
            inner.scheduler_running = false;
            return None;
        }
        Some(
            inner
                .entries
                .iter()
                .filter(|(_, entry)| {
                    entry_has_active_consumer(entry, now) && entry_is_due(entry, now_unix)
                })
                .map(|(provider_id, _)| *provider_id)
                .collect(),
        )
    }
}

#[derive(Clone, Copy)]
enum ConsumerKind {
    Desktop,
    Tui,
}

fn invalidate_entry_for_schedule(
    entry: &mut RuntimeEntry,
    schedule: ProviderAccountUsageRefreshSchedule,
) {
    entry.generation = entry.generation.wrapping_add(1);
    entry.schedule = Some(schedule);
    entry.result = None;
    entry.last_attempt_at = None;
    if entry.in_flight_generation.is_none() {
        publish_completion(entry);
    }
}

fn publish_completion(entry: &mut RuntimeEntry) {
    entry.completion_generation = entry.completion_generation.wrapping_add(1);
    entry.completion.send_replace(entry.completion_generation);
}

fn entry_has_active_consumer(entry: &RuntimeEntry, now: Instant) -> bool {
    entry.schedule.is_some()
        && (entry
            .desktop_lease_until
            .is_some_and(|deadline| deadline > now)
            || entry.tui_lease_until.is_some_and(|deadline| deadline > now))
}

fn entry_is_due(entry: &RuntimeEntry, now_unix: i64) -> bool {
    let Some(schedule) = entry.schedule else {
        return false;
    };
    if entry.in_flight_generation.is_some() {
        return false;
    }
    let Some(last_attempt_at) = entry.last_attempt_at else {
        return true;
    };
    if entry
        .result
        .as_ref()
        .is_some_and(|result| is_success_result(result) && !is_fresh_success(result, now_unix))
    {
        return true;
    }
    schedule.timed_refresh_enabled
        && now_unix.saturating_sub(last_attempt_at) >= schedule.refresh_interval_seconds
}

fn is_success_result(result: &ProviderAccountUsageResult) -> bool {
    matches!(
        result.status,
        ProviderAccountUsageStatus::Available
            | ProviderAccountUsageStatus::ZeroBalance
            | ProviderAccountUsageStatus::Expired
    )
}

fn is_fresh_success(result: &ProviderAccountUsageResult, now_unix: i64) -> bool {
    let Some(fetched_at) = result.last_fetched_at else {
        return false;
    };
    let age = now_unix.saturating_sub(fetched_at);
    fetched_at <= now_unix && (0..SUCCESS_CACHE_TTL_SECONDS).contains(&age)
}

fn display_snapshot(entry: &RuntimeEntry, now_unix: i64) -> ProviderAccountUsageDisplaySnapshot {
    if entry.in_flight_generation.is_some() {
        return ProviderAccountUsageDisplaySnapshot {
            state: "loading",
            amount: None,
            unit: None,
            last_fetched_at: None,
        };
    }
    let Some(result) = entry
        .result
        .as_ref()
        .filter(|result| is_success_result(result) && is_fresh_success(result, now_unix))
    else {
        return ProviderAccountUsageDisplaySnapshot {
            state: "failed",
            amount: None,
            unit: None,
            last_fetched_at: None,
        };
    };
    let amount = result
        .plan_remaining
        .filter(|value| value.is_finite())
        .or_else(|| result.balance.filter(|value| value.is_finite()));
    ProviderAccountUsageDisplaySnapshot {
        state: "available",
        amount,
        unit: amount.and(result.unit.clone()),
        last_fetched_at: result.last_fetched_at,
    }
}

fn unavailable_result() -> ProviderAccountUsageResult {
    ProviderAccountUsageResult::local_status(
        None,
        ProviderAccountUsageStatus::QueryFailed,
        "账户用量尚未获取",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::provider_account_usage::{
        ProviderAccountUsageAdapterKind, ProviderAccountUsageFreshness,
    };

    fn schedule(timed_refresh_enabled: bool) -> ProviderAccountUsageRefreshSchedule {
        ProviderAccountUsageRefreshSchedule {
            timed_refresh_enabled,
            refresh_interval_seconds: 60,
            revision: 1,
        }
    }

    fn result(fetched_at: i64) -> ProviderAccountUsageResult {
        let mut result = ProviderAccountUsageResult::fetched(
            ProviderAccountUsageAdapterKind::Newapi,
            ProviderAccountUsageStatus::Available,
            fetched_at,
        );
        result.freshness = ProviderAccountUsageFreshness::Fresh;
        result.plan_remaining = Some(12.5);
        result.balance = Some(99.0);
        result.unit = Some("USD".to_string());
        result
    }

    fn entry_with_result(
        timed_refresh_enabled: bool,
        fetched_at: i64,
        last_attempt_at: i64,
    ) -> RuntimeEntry {
        RuntimeEntry {
            schedule: Some(schedule(timed_refresh_enabled)),
            result: Some(result(fetched_at)),
            last_attempt_at: Some(last_attempt_at),
            ..RuntimeEntry::default()
        }
    }

    #[test]
    fn configured_interval_controls_due_refresh_without_affecting_hard_expiry() {
        let timed = entry_with_result(true, 1_000, 1_000);
        assert!(!entry_is_due(&timed, 1_059));
        assert!(entry_is_due(&timed, 1_060));

        let untimed = entry_with_result(false, 1_000, 1_000);
        assert!(!entry_is_due(&untimed, 1_060));
        assert!(entry_is_due(&untimed, 1_000 + SUCCESS_CACHE_TTL_SECONDS));
    }

    #[test]
    fn display_prefers_plan_remaining_and_rejects_stale_or_future_results() {
        let current = entry_with_result(true, 10_000, 10_000);
        let snapshot = display_snapshot(&current, 10_001);
        assert_eq!(snapshot.state, "available");
        assert_eq!(snapshot.amount, Some(12.5));
        assert_eq!(snapshot.unit.as_deref(), Some("USD"));

        assert_eq!(
            display_snapshot(&current, 10_000 + SUCCESS_CACHE_TTL_SECONDS).state,
            "failed"
        );
        assert_eq!(display_snapshot(&current, 9_999).state, "failed");
    }

    #[tokio::test]
    async fn same_provider_refresh_decisions_coalesce_while_in_flight() {
        let state = ProviderAccountUsageRuntimeState::default();
        {
            let mut inner = state.shared.inner.lock().await;
            inner.entries.insert(
                7,
                RuntimeEntry {
                    schedule: Some(schedule(true)),
                    ..RuntimeEntry::default()
                },
            );
        }

        assert!(matches!(
            state.begin_refresh(7, false).await,
            RefreshDecision::Lead(_)
        ));
        assert!(matches!(
            state.begin_refresh(7, true).await,
            RefreshDecision::Wait { .. }
        ));
    }

    #[test]
    fn failed_results_retry_only_on_the_saved_timed_interval() {
        let mut timed = entry_with_result(true, 1_000, 1_000);
        timed.result = Some(ProviderAccountUsageResult::local_status(
            Some(ProviderAccountUsageAdapterKind::Newapi),
            ProviderAccountUsageStatus::QueryFailed,
            "synthetic failure",
        ));
        assert!(!entry_is_due(&timed, 1_059));
        assert!(entry_is_due(&timed, 1_060));

        timed.schedule = Some(schedule(false));
        assert!(!entry_is_due(&timed, 2_000));
    }

    #[test]
    fn consumer_lease_is_independent_of_provider_routing_enablement() {
        let now = Instant::now();
        let entry = RuntimeEntry {
            schedule: Some(schedule(true)),
            desktop_lease_until: Some(now + CONSUMER_LEASE),
            ..RuntimeEntry::default()
        };
        assert!(entry_has_active_consumer(&entry, now));
        assert!(!entry_has_active_consumer(&entry, now + CONSUMER_LEASE));
    }

    #[test]
    fn loading_never_projects_an_older_cached_amount() {
        let mut current = entry_with_result(true, 10_000, 10_000);
        current.in_flight_generation = Some(current.generation);
        let snapshot = display_snapshot(&current, 10_001);
        assert_eq!(snapshot.state, "loading");
        assert_eq!(snapshot.amount, None);
        assert_eq!(snapshot.last_fetched_at, None);
    }
}
