//! Process-owned Provider availability probe coordination and scheduling.

use crate::domain::provider_availability::{self, ProviderAvailabilityResult};
use crate::shared::error::{db_err, AppError, AppResult};
use crate::{blocking, db};
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, Weak};
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::{oneshot, Mutex, OwnedMutexGuard, Semaphore};

const SCHEDULER_TICK: Duration = Duration::from_secs(1);
const SCHEDULER_SUSPEND_GAP: Duration = Duration::from_secs(10);
const SCHEDULED_PROBE_DELAY_MS: i64 = 5_000;
const SCHEDULED_PROBE_JITTER_SLOTS: i64 = 4;
const SCHEDULED_DUE_GRACE_MS: i64 = 5_000;
const MAX_SCHEDULED_PROVIDERS: usize = 512;
const MAX_CONCURRENT_SCHEDULED_PROBES: usize = 4;

#[derive(Clone)]
pub(crate) struct ProviderAvailabilityProbeRuntimeState {
    shared: Arc<RuntimeShared>,
}

impl Default for ProviderAvailabilityProbeRuntimeState {
    fn default() -> Self {
        Self {
            shared: Arc::new(RuntimeShared {
                inner: Mutex::new(RuntimeInner::default()),
                provider_mutation_gates: StdMutex::new(HashMap::new()),
                scheduled_limiter: Arc::new(Semaphore::new(MAX_CONCURRENT_SCHEDULED_PROBES)),
                scheduler_started: AtomicBool::new(false),
                schedule_limit_warned: AtomicBool::new(false),
            }),
        }
    }
}

struct RuntimeShared {
    inner: Mutex<RuntimeInner>,
    provider_mutation_gates: StdMutex<HashMap<i64, Weak<Mutex<()>>>>,
    scheduled_limiter: Arc<Semaphore>,
    scheduler_started: AtomicBool,
    schedule_limit_warned: AtomicBool,
}

#[must_use = "hold this guard until the Provider mutation is durably committed"]
pub(crate) struct ProviderAvailabilityProbeMutationGuard {
    _guard: OwnedMutexGuard<()>,
}

#[derive(Default)]
struct RuntimeInner {
    entries: HashMap<i64, RuntimeEntry>,
    next_generation: u64,
}

impl RuntimeInner {
    fn allocate_generation(&mut self) -> u64 {
        self.next_generation = self.next_generation.wrapping_add(1);
        if self.next_generation == 0 {
            self.next_generation = 1;
        }
        self.next_generation
    }
}

#[derive(Default)]
struct RuntimeEntry {
    generation: u64,
    schedule: Option<ScheduledProbeState>,
    // A configuration change advances the generation before its database write.
    // Retain older flights only long enough to answer callers that started them;
    // a new generation must never join an older request or write its observation.
    in_flight: HashMap<u64, InFlightProbe>,
}

struct InFlightProbe {
    generation: u64,
    waiters: Vec<oneshot::Sender<AppResult<ProviderAvailabilityResult>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledProbeConfig {
    interval_minutes: u32,
    revision: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledProbeState {
    config: ScheduledProbeConfig,
    next_boundary_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoadedSchedule {
    provider_id: i64,
    active: bool,
    interval_minutes: u32,
    revision: i64,
    next_boundary_ms: i64,
}

struct LoadedScheduleBatch {
    schedules: Vec<LoadedSchedule>,
    truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledProbeTarget {
    provider_id: i64,
    generation: u64,
    boundary_ms: i64,
}

enum ProbeSource {
    Manual,
    Scheduled { boundary_ms: i64 },
}

enum ProbeDecision {
    Lead {
        generation: u64,
        receiver: oneshot::Receiver<AppResult<ProviderAvailabilityResult>>,
    },
    Wait(oneshot::Receiver<AppResult<ProviderAvailabilityResult>>),
    Stale,
}

impl ProviderAvailabilityProbeRuntimeState {
    pub(crate) fn from_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<Self> {
        app.try_state::<Self>().map(|state| state.inner().clone())
    }

    pub(crate) fn start_scheduler<R: tauri::Runtime>(&self, app: tauri::AppHandle<R>, db: db::Db) {
        if self.shared.scheduler_started.swap(true, Ordering::AcqRel) {
            return;
        }
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            state.run_scheduler(app, db).await;
        });
    }

    fn provider_mutation_gate(&self, provider_id: i64) -> Arc<Mutex<()>> {
        let mut gates = self
            .shared
            .provider_mutation_gates
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        gates.retain(|_, gate| gate.strong_count() > 0);
        if let Some(gate) = gates.get(&provider_id).and_then(Weak::upgrade) {
            return gate;
        }
        let gate = Arc::new(Mutex::new(()));
        gates.insert(provider_id, Arc::downgrade(&gate));
        gate
    }

    pub(crate) async fn begin_mutation(
        &self,
        provider_id: i64,
    ) -> Option<ProviderAvailabilityProbeMutationGuard> {
        if provider_id <= 0 {
            return None;
        }
        let guard = self.provider_mutation_gate(provider_id).lock_owned().await;
        self.invalidate_generation(provider_id).await;
        Some(ProviderAvailabilityProbeMutationGuard { _guard: guard })
    }

    pub(crate) async fn invalidate(&self, provider_id: i64) {
        let _guard = self.begin_mutation(provider_id).await;
    }

    async fn invalidate_generation(&self, provider_id: i64) {
        let mut inner = self.shared.inner.lock().await;
        let generation = inner.allocate_generation();
        {
            let entry = inner.entries.entry(provider_id).or_default();
            entry.generation = generation;
            entry.schedule = None;
        }
        remove_idle_entry(&mut inner, provider_id);
    }

    pub(crate) async fn probe_manual<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db: db::Db,
        provider_id: i64,
    ) -> AppResult<ProviderAvailabilityResult> {
        if provider_id <= 0 {
            return Err(AppError::from("SEC_INVALID_INPUT: invalid provider_id"));
        }
        self.probe(app, db, provider_id, None, ProbeSource::Manual)
            .await
            .ok_or_else(|| AppError::new("SYSTEM_ERROR", "provider probe became stale"))?
    }

    async fn probe<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db: db::Db,
        provider_id: i64,
        expected_generation: Option<u64>,
        source: ProbeSource,
    ) -> Option<AppResult<ProviderAvailabilityResult>> {
        match self.begin_probe(provider_id, expected_generation).await {
            ProbeDecision::Stale => None,
            ProbeDecision::Wait(receiver) => Some(receiver.await.unwrap_or_else(|_| {
                Err(AppError::new(
                    "SYSTEM_ERROR",
                    "provider probe coordinator stopped unexpectedly",
                ))
            })),
            ProbeDecision::Lead {
                generation,
                receiver,
            } => {
                let trace_id = match source {
                    ProbeSource::Manual => manual_trace_id(provider_id),
                    ProbeSource::Scheduled { boundary_ms } => {
                        scheduled_trace_id(provider_id, boundary_ms)
                    }
                };
                // The requester may time out or disconnect. Keep the real probe
                // alive so its shared flight is always completed and released.
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    let result = provider_availability::test_provider_availability(
                        &app,
                        db.clone(),
                        provider_id,
                    )
                    .await;
                    state
                        .finish_probe(&db, provider_id, generation, &trace_id, result)
                        .await;
                });
                Some(receiver.await.unwrap_or_else(|_| {
                    Err(AppError::new(
                        "SYSTEM_ERROR",
                        "provider probe coordinator stopped unexpectedly",
                    ))
                }))
            }
        }
    }

    async fn begin_probe(
        &self,
        provider_id: i64,
        expected_generation: Option<u64>,
    ) -> ProbeDecision {
        let _gate = self.provider_mutation_gate(provider_id).lock_owned().await;
        let mut inner = self.shared.inner.lock().await;
        if !inner.entries.contains_key(&provider_id) {
            let generation = inner.allocate_generation();
            inner.entries.insert(
                provider_id,
                RuntimeEntry {
                    generation,
                    ..RuntimeEntry::default()
                },
            );
        }
        let entry = inner
            .entries
            .get_mut(&provider_id)
            .expect("probe runtime entry");
        if expected_generation.is_some_and(|generation| generation != entry.generation) {
            return ProbeDecision::Stale;
        }
        let generation = entry.generation;
        if let Some(in_flight) = entry.in_flight.get_mut(&generation) {
            let (sender, receiver) = oneshot::channel();
            in_flight.waiters.push(sender);
            return ProbeDecision::Wait(receiver);
        }
        let (sender, receiver) = oneshot::channel();
        entry.in_flight.insert(
            generation,
            InFlightProbe {
                generation,
                waiters: vec![sender],
            },
        );
        ProbeDecision::Lead {
            generation,
            receiver,
        }
    }

    async fn finish_probe(
        &self,
        db: &db::Db,
        provider_id: i64,
        generation: u64,
        trace_id: &str,
        result: AppResult<ProviderAvailabilityResult>,
    ) {
        let mut inner = self.shared.inner.lock().await;
        let Some((in_flight, should_record)) =
            take_finished_flight(&mut inner, provider_id, generation)
        else {
            return;
        };

        // Keep generation validation and the insert ordered against invalidation.
        // Credential writers invalidate first, then persist their new value.
        if should_record {
            if let Ok(probe) = result.as_ref() {
                if let Err(error) = provider_availability::record_probe_observation(
                    db,
                    trace_id,
                    provider_id,
                    crate::shared::time::now_unix_millis(),
                    probe.ok,
                ) {
                    tracing::warn!(
                        error = %error.code(),
                        provider_id,
                        "provider availability probe observation write failed"
                    );
                }
            }
        }

        for waiter in in_flight.waiters {
            let _ = waiter.send(result.clone());
        }
        remove_idle_entry(&mut inner, provider_id);
    }

    async fn run_scheduler<R: tauri::Runtime>(self, app: tauri::AppHandle<R>, db: db::Db) {
        let mut interval = tokio::time::interval(SCHEDULER_TICK);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_tick = Instant::now();
        loop {
            interval.tick().await;
            let tick = Instant::now();
            let skip_missed = tick.duration_since(last_tick) > SCHEDULER_SUSPEND_GAP;
            last_tick = tick;
            let now_ms = crate::shared::time::now_unix_millis();
            let db_for_load = db.clone();
            let schedules = blocking::run("provider_availability_probe_schedule", move || {
                load_schedules(&db_for_load, now_ms)
            })
            .await;
            let batch = match schedules {
                Ok(batch) => batch,
                Err(error) => {
                    tracing::warn!(
                        error = %error.code(),
                        "provider availability probe schedule refresh failed"
                    );
                    continue;
                }
            };
            if batch.truncated {
                if !self
                    .shared
                    .schedule_limit_warned
                    .swap(true, Ordering::AcqRel)
                {
                    tracing::warn!(
                        limit = MAX_SCHEDULED_PROVIDERS,
                        "scheduled Provider availability probe limit reached; keeping bounded prefix"
                    );
                }
            } else {
                self.shared
                    .schedule_limit_warned
                    .store(false, Ordering::Release);
            }
            let targets = self
                .reconcile_schedules(batch.schedules, now_ms, skip_missed)
                .await;
            for target in targets {
                let state = self.clone();
                let app = app.clone();
                let db = db.clone();
                tauri::async_runtime::spawn(async move {
                    state.run_scheduled_probe(app, db, target).await;
                });
            }
        }
    }

    async fn reconcile_schedules(
        &self,
        schedules: Vec<LoadedSchedule>,
        now_ms: i64,
        skip_missed: bool,
    ) -> Vec<ScheduledProbeTarget> {
        let mut inner = self.shared.inner.lock().await;
        reconcile_schedules_inner(&mut inner, schedules, now_ms, skip_missed)
    }

    async fn run_scheduled_probe<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db: db::Db,
        target: ScheduledProbeTarget,
    ) {
        let permit = match self.shared.scheduled_limiter.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => return,
        };
        let result = self
            .probe(
                app,
                db,
                target.provider_id,
                Some(target.generation),
                ProbeSource::Scheduled {
                    boundary_ms: target.boundary_ms,
                },
            )
            .await;
        drop(permit);
        if let Some(Err(error)) = result {
            tracing::warn!(
                error = %error.code(),
                provider_id = target.provider_id,
                "scheduled provider availability probe failed"
            );
        }
    }
}

fn take_finished_flight(
    inner: &mut RuntimeInner,
    provider_id: i64,
    generation: u64,
) -> Option<(InFlightProbe, bool)> {
    let entry = inner.entries.get_mut(&provider_id)?;
    let in_flight = entry.in_flight.remove(&generation)?;
    if in_flight.generation != generation {
        return None;
    }
    let should_record = entry.generation == generation;
    Some((in_flight, should_record))
}

fn remove_idle_entry(inner: &mut RuntimeInner, provider_id: i64) {
    let should_remove = inner
        .entries
        .get(&provider_id)
        .is_some_and(|entry| entry.schedule.is_none() && entry.in_flight.is_empty());
    if should_remove {
        inner.entries.remove(&provider_id);
    }
}

fn reconcile_schedules_inner(
    inner: &mut RuntimeInner,
    schedules: Vec<LoadedSchedule>,
    now_ms: i64,
    skip_missed: bool,
) -> Vec<ScheduledProbeTarget> {
    let mut seen = HashSet::with_capacity(schedules.len());
    let mut targets = Vec::new();
    for loaded in schedules {
        seen.insert(loaded.provider_id);
        if !loaded.active {
            let had_schedule = inner
                .entries
                .get_mut(&loaded.provider_id)
                .is_some_and(|entry| entry.schedule.take().is_some());
            if had_schedule {
                let generation = inner.allocate_generation();
                if let Some(entry) = inner.entries.get_mut(&loaded.provider_id) {
                    entry.generation = generation;
                }
            }
            remove_idle_entry(inner, loaded.provider_id);
            continue;
        }
        let config = ScheduledProbeConfig {
            interval_minutes: loaded.interval_minutes,
            revision: loaded.revision,
        };
        let config_changed = inner.entries.get(&loaded.provider_id).is_none_or(|entry| {
            entry
                .schedule
                .is_none_or(|schedule| schedule.config != config)
        });
        if config_changed {
            let generation = inner.allocate_generation();
            let entry = inner.entries.entry(loaded.provider_id).or_default();
            entry.generation = generation;
            entry.schedule = Some(ScheduledProbeState {
                config,
                next_boundary_ms: loaded.next_boundary_ms,
            });
            continue;
        }

        let entry = inner
            .entries
            .get_mut(&loaded.provider_id)
            .expect("active schedule entry");
        let schedule = entry.schedule.as_mut().expect("active schedule");
        let due_at_ms = scheduled_due_at_ms(loaded.provider_id, schedule.next_boundary_ms);
        if now_ms < due_at_ms {
            continue;
        }
        if skip_missed || now_ms > due_at_ms.saturating_add(SCHEDULED_DUE_GRACE_MS) {
            schedule.next_boundary_ms = loaded.next_boundary_ms;
            continue;
        }
        let boundary_ms = schedule.next_boundary_ms;
        schedule.next_boundary_ms = loaded.next_boundary_ms;
        targets.push(ScheduledProbeTarget {
            provider_id: loaded.provider_id,
            generation: entry.generation,
            boundary_ms,
        });
    }

    let missing_provider_ids = inner
        .entries
        .keys()
        .filter(|provider_id| !seen.contains(provider_id))
        .copied()
        .collect::<Vec<_>>();
    for provider_id in missing_provider_ids {
        let had_schedule = inner
            .entries
            .get_mut(&provider_id)
            .is_some_and(|entry| entry.schedule.take().is_some());
        if had_schedule {
            let generation = inner.allocate_generation();
            if let Some(entry) = inner.entries.get_mut(&provider_id) {
                entry.generation = generation;
            }
        }
        remove_idle_entry(inner, provider_id);
    }
    targets
}

fn load_schedules(db: &db::Db, now_ms: i64) -> AppResult<LoadedScheduleBatch> {
    let conn = db.open_connection()?;
    load_schedules_from_conn(&conn, now_ms)
}

fn load_schedules_from_conn(
    conn: &rusqlite::Connection,
    now_ms: i64,
) -> AppResult<LoadedScheduleBatch> {
    let now_seconds = now_ms.max(0).div_euclid(1_000);
    let mut statement = conn
        .prepare(
            r#"
WITH local_clock AS (
  SELECT
    date(?1, 'unixepoch', 'localtime') AS local_day,
    CAST(strftime('%H', ?1, 'unixepoch', 'localtime') AS INTEGER) * 60
      + CAST(strftime('%M', ?1, 'unixepoch', 'localtime') AS INTEGER) AS minute_of_day
), provider_schedules AS (
  SELECT
    p.id,
    1 AS active,
    p.availability_probe_interval_minutes AS interval_minutes,
    p.updated_at AS revision,
    local_clock.local_day,
    ((local_clock.minute_of_day / p.availability_probe_interval_minutes) + 1)
      * p.availability_probe_interval_minutes AS next_minute
  FROM providers p
  CROSS JOIN local_clock
  WHERE p.enabled = 1
    AND p.availability_probe_enabled = 1
    AND p.availability_probe_interval_minutes BETWEEN 1 AND 1440
  ORDER BY p.id ASC
  LIMIT ?2
)
SELECT
  id,
  active,
  interval_minutes,
  revision,
  CAST(strftime(
    '%s',
    CASE
      WHEN next_minute >= 1440
        THEN date(local_day, '+1 day') || ' 00:00:00'
      ELSE local_day || printf(' %02d:%02d:00', next_minute / 60, next_minute % 60)
    END,
    'utc'
  ) AS INTEGER) * 1000 AS next_boundary_ms
FROM provider_schedules
"#,
        )
        .map_err(|error| db_err!("failed to prepare Provider probe schedules: {error}"))?;
    let rows = statement
        .query_map(
            params![now_seconds, (MAX_SCHEDULED_PROVIDERS + 1) as i64],
            |row| {
                Ok(LoadedSchedule {
                    provider_id: row.get(0)?,
                    active: row.get(1)?,
                    interval_minutes: row.get(2)?,
                    revision: row.get(3)?,
                    next_boundary_ms: row.get(4)?,
                })
            },
        )
        .map_err(|error| db_err!("failed to query Provider probe schedules: {error}"))?;
    let mut schedules = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| db_err!("failed to read Provider probe schedule: {error}"))?;
    let truncated = schedules.len() > MAX_SCHEDULED_PROVIDERS;
    schedules.truncate(MAX_SCHEDULED_PROVIDERS);
    Ok(LoadedScheduleBatch {
        schedules,
        truncated,
    })
}

fn stable_jitter_ms(provider_id: i64) -> i64 {
    let mixed = (provider_id as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(17);
    i64::try_from(mixed % SCHEDULED_PROBE_JITTER_SLOTS as u64).unwrap_or_default() * 1_000
}

fn scheduled_due_at_ms(provider_id: i64, boundary_ms: i64) -> i64 {
    boundary_ms
        .saturating_add(SCHEDULED_PROBE_DELAY_MS)
        .saturating_add(stable_jitter_ms(provider_id))
}

fn scheduled_trace_id(provider_id: i64, boundary_ms: i64) -> String {
    format!("availability-probe:{provider_id}:{boundary_ms}")
}

fn manual_trace_id(provider_id: i64) -> String {
    format!(
        "availability-probe:manual:{provider_id}:{}",
        crate::shared::uuid::new_uuid_v4()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule_test_connection() -> rusqlite::Connection {
        let connection = rusqlite::Connection::open_in_memory().expect("open schedule test db");
        connection
            .execute_batch(
                r#"
CREATE TABLE providers (
  id INTEGER PRIMARY KEY,
  enabled INTEGER NOT NULL,
  availability_probe_enabled INTEGER NOT NULL,
  availability_probe_interval_minutes INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
"#,
            )
            .expect("create schedule test schema");
        connection
    }

    fn insert_schedule_provider(connection: &rusqlite::Connection, provider_id: i64, active: bool) {
        connection
            .execute(
                r#"
INSERT INTO providers(
  id, enabled, availability_probe_enabled,
  availability_probe_interval_minutes, updated_at
) VALUES (?1, ?2, ?2, 10, ?1)
"#,
                params![provider_id, active],
            )
            .expect("insert schedule test provider");
    }

    fn loaded(provider_id: i64, revision: i64, next_boundary_ms: i64) -> LoadedSchedule {
        LoadedSchedule {
            provider_id,
            active: true,
            interval_minutes: 10,
            revision,
            next_boundary_ms,
        }
    }

    #[test]
    fn startup_and_configuration_changes_schedule_only_the_next_boundary() {
        let mut inner = RuntimeInner::default();
        let targets =
            reconcile_schedules_inner(&mut inner, vec![loaded(7, 1, 60_000)], 55_000, false);
        assert!(targets.is_empty());

        let targets =
            reconcile_schedules_inner(&mut inner, vec![loaded(7, 2, 120_000)], 65_000, false);
        assert!(targets.is_empty());
        assert_eq!(
            inner.entries[&7].schedule.unwrap().next_boundary_ms,
            120_000
        );
    }

    #[test]
    fn due_boundaries_run_once_while_suspend_gaps_are_skipped() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(&mut inner, vec![loaded(3, 1, 60_000)], 50_000, false);
        let due_at = scheduled_due_at_ms(3, 60_000);
        let targets =
            reconcile_schedules_inner(&mut inner, vec![loaded(3, 1, 120_000)], due_at, false);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].boundary_ms, 60_000);

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 180_000)],
            scheduled_due_at_ms(3, 120_000),
            true,
        );
        assert!(targets.is_empty());
        assert_eq!(
            inner.entries[&3].schedule.unwrap().next_boundary_ms,
            180_000
        );
    }

    #[test]
    fn disabled_or_missing_providers_invalidate_scheduled_generations() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(&mut inner, vec![loaded(1, 1, 60_000)], 1_000, false);
        let generation = inner.entries[&1].generation;
        let mut disabled = loaded(1, 1, 60_000);
        disabled.active = false;
        reconcile_schedules_inner(&mut inner, vec![disabled], 2_000, false);
        assert!(!inner.entries.contains_key(&1));
        assert!(generation > 0);
    }

    #[test]
    fn reclaimed_provider_entries_receive_a_new_generation_after_recreation() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(&mut inner, vec![loaded(1, 1, 60_000)], 1_000, false);
        let first_generation = inner.entries[&1].generation;
        reconcile_schedules_inner(&mut inner, Vec::new(), 2_000, false);
        assert!(!inner.entries.contains_key(&1));

        reconcile_schedules_inner(&mut inner, vec![loaded(1, 1, 60_000)], 3_000, false);
        assert_ne!(inner.entries[&1].generation, first_generation);
    }

    #[test]
    fn scheduled_identity_and_jitter_are_stable_and_bounded() {
        assert_eq!(
            scheduled_trace_id(9, 123_000),
            "availability-probe:9:123000"
        );
        let jitter = stable_jitter_ms(9);
        assert!((0..=3_000).contains(&jitter));
        assert_eq!(jitter, stable_jitter_ms(9));
    }

    #[test]
    fn scheduled_probe_concurrency_is_capped_at_four() {
        let limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_SCHEDULED_PROBES));
        let permits = (0..MAX_CONCURRENT_SCHEDULED_PROBES)
            .map(|_| {
                limiter
                    .clone()
                    .try_acquire_owned()
                    .expect("scheduled permit")
            })
            .collect::<Vec<_>>();
        assert!(limiter.clone().try_acquire_owned().is_err());
        drop(permits);
        assert!(limiter.try_acquire_owned().is_ok());
    }

    #[test]
    fn schedule_loading_counts_only_enabled_probe_configurations() {
        let connection = schedule_test_connection();
        for provider_id in 1..=513 {
            insert_schedule_provider(&connection, provider_id, false);
        }
        insert_schedule_provider(&connection, 514, true);

        let batch = load_schedules_from_conn(&connection, 55_000).expect("load schedules");
        assert!(!batch.truncated);
        assert_eq!(batch.schedules.len(), 1);
        assert_eq!(batch.schedules[0].provider_id, 514);
    }

    #[test]
    fn schedule_loading_keeps_a_bounded_prefix_instead_of_failing_the_batch() {
        let connection = schedule_test_connection();
        for provider_id in 1..=(MAX_SCHEDULED_PROVIDERS as i64 + 1) {
            insert_schedule_provider(&connection, provider_id, true);
        }

        let batch = load_schedules_from_conn(&connection, 55_000).expect("load schedules");
        assert!(batch.truncated);
        assert_eq!(batch.schedules.len(), MAX_SCHEDULED_PROVIDERS);
        assert_eq!(batch.schedules.first().map(|row| row.provider_id), Some(1));
        assert_eq!(
            batch.schedules.last().map(|row| row.provider_id),
            Some(MAX_SCHEDULED_PROVIDERS as i64)
        );
    }

    #[tokio::test]
    async fn mutation_guard_blocks_probe_start_until_the_persistence_boundary() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let guard = state
            .begin_mutation(12)
            .await
            .expect("valid Provider mutation guard");
        let invalidated_generation = state.shared.inner.lock().await.next_generation;
        let waiting_state = state.clone();
        let (started_sender, started_receiver) = oneshot::channel();
        let probe_task = tokio::spawn(async move {
            let _ = started_sender.send(());
            match waiting_state.begin_probe(12, None).await {
                ProbeDecision::Lead { generation, .. } => generation,
                _ => panic!("probe after mutation must lead a new generation"),
            }
        });

        started_receiver.await.expect("probe task started");
        tokio::task::yield_now().await;
        assert!(!probe_task.is_finished());
        drop(guard);

        let generation = tokio::time::timeout(Duration::from_secs(1), probe_task)
            .await
            .expect("probe starts after mutation commit boundary")
            .expect("probe task succeeds");
        assert_ne!(generation, invalidated_generation);
    }

    #[tokio::test]
    async fn same_provider_probe_decisions_coalesce_and_invalidation_rejects_old_generation() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let generation = match state.begin_probe(4, None).await {
            ProbeDecision::Lead { generation, .. } => generation,
            _ => panic!("first probe must lead"),
        };
        assert!(matches!(
            state.begin_probe(4, None).await,
            ProbeDecision::Wait(_)
        ));
        state.invalidate(4).await;
        {
            let mut inner = state.shared.inner.lock().await;
            let (_, should_record) = take_finished_flight(&mut inner, 4, generation)
                .expect("invalidated flight remains available for completion");
            assert!(
                !should_record,
                "an invalidated probe must not write an observation"
            );
        }
        assert!(matches!(
            state.begin_probe(4, Some(generation)).await,
            ProbeDecision::Stale
        ));
        let replacement_generation = match state.begin_probe(4, None).await {
            ProbeDecision::Lead { generation, .. } => generation,
            _ => panic!("new configuration must start a new single-flight probe"),
        };
        assert_ne!(replacement_generation, generation);
    }

    #[tokio::test]
    async fn invalidating_an_idle_provider_does_not_leave_a_runtime_tombstone() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        state.invalidate(99).await;
        let inner = state.shared.inner.lock().await;
        assert!(!inner.entries.contains_key(&99));
    }
}
