//! Process-owned Provider availability probe coordination and scheduling.

use crate::domain::provider_availability::{self, ProviderAvailabilityResult};
use crate::shared::error::{db_err, AppError, AppResult};
use crate::{blocking, db};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, Weak};
use std::time::{Duration, Instant};
use tauri::Manager;
use tokio::sync::{oneshot, Mutex, OwnedMutexGuard, Semaphore};

use super::gateway_state;

const SCHEDULER_TICK: Duration = Duration::from_secs(1);
const SCHEDULER_SUSPEND_GAP: Duration = Duration::from_secs(10);
const SCHEDULED_PROBE_DELAY_MS: i64 = 5_000;
const SCHEDULED_PROBE_JITTER_SLOTS: i64 = 4;
const SCHEDULED_DUE_GRACE_MS: i64 = 5_000;
const RECOVERY_PROBE_DELAY_MS: i64 = 30_000;
const SCHEDULED_PROVIDER_PAGE_SIZE: usize = 512;
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
                #[cfg(test)]
                next_pause: StdMutex::new(None),
            }),
        }
    }
}

struct RuntimeShared {
    inner: Mutex<RuntimeInner>,
    provider_mutation_gates: StdMutex<HashMap<i64, Weak<Mutex<()>>>>,
    scheduled_limiter: Arc<Semaphore>,
    scheduler_started: AtomicBool,
    #[cfg(test)]
    next_pause: StdMutex<Option<provider_availability::ProbeTestPause>>,
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
    recovery: Option<RecoveryProbeState>,
    recovery_epoch: u64,
    schedule_seen_epoch: u64,
    // A configuration change advances the generation before its database write.
    // Retain the old flight until it completes so a new generation never runs a
    // second network probe for the same Provider concurrently.
    in_flight: Option<InFlightProbe>,
}

struct InFlightProbe {
    generation: u64,
    recovery_epoch: u64,
    budget: Arc<provider_availability::ProbeBudget>,
    waiters: Vec<oneshot::Sender<CompletedProbe>>,
    turn_waiters: Vec<oneshot::Sender<()>>,
}

#[derive(Clone)]
struct CompletedProbe {
    result: AppResult<ProviderAvailabilityResult>,
    recovery: Option<RecoveryDirective>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RecoveryDirective {
    generation: u64,
    recovery_epoch: u64,
    due_at_ms: i64,
}

impl RecoveryDirective {
    fn from_completion(generation: u64, recovery_epoch: u64, completed_at_ms: i64) -> Self {
        Self {
            generation,
            recovery_epoch,
            due_at_ms: completed_at_ms.saturating_add(RECOVERY_PROBE_DELAY_MS),
        }
    }
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
struct RecoveryProbeState {
    generation: u64,
    recovery_epoch: u64,
    due_at_ms: i64,
    phase: RecoveryProbePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecoveryProbePhase {
    Pending,
    Claimed,
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
    next_after_provider_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScheduledProbeSource {
    Natural { boundary_ms: i64 },
    Recovery { recovery_epoch: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledProbeTarget {
    provider_id: i64,
    generation: u64,
    due_at_ms: i64,
    source: ScheduledProbeSource,
}

impl ScheduledProbeTarget {
    fn deadline_ms(self) -> i64 {
        match self.source {
            ScheduledProbeSource::Natural { boundary_ms } => {
                scheduled_due_deadline_ms(self.provider_id, boundary_ms)
            }
            ScheduledProbeSource::Recovery { .. } => recovery_due_deadline_ms(self.due_at_ms),
        }
    }

    fn probe_source(self) -> ProbeSource {
        match self.source {
            ScheduledProbeSource::Natural { boundary_ms } => ProbeSource::Scheduled { boundary_ms },
            ScheduledProbeSource::Recovery { recovery_epoch } => ProbeSource::Recovery {
                due_at_ms: self.due_at_ms,
                recovery_epoch,
            },
        }
    }

    fn is_recovery(self) -> bool {
        matches!(self.source, ScheduledProbeSource::Recovery { .. })
    }
}

#[derive(Clone, Copy)]
enum ProbeSource {
    Manual,
    Scheduled { boundary_ms: i64 },
    Recovery { due_at_ms: i64, recovery_epoch: u64 },
}

impl ProbeSource {
    fn is_expired(self, provider_id: i64, now_ms: i64) -> bool {
        match self {
            Self::Manual => false,
            Self::Scheduled { boundary_ms } => {
                now_ms > scheduled_due_deadline_ms(provider_id, boundary_ms)
            }
            Self::Recovery { due_at_ms, .. } => now_ms > recovery_due_deadline_ms(due_at_ms),
        }
    }
}

enum ProbeDecision {
    Lead {
        generation: u64,
        budget: Arc<provider_availability::ProbeBudget>,
        receiver: oneshot::Receiver<CompletedProbe>,
    },
    Wait(oneshot::Receiver<CompletedProbe>),
    WaitForTurn(oneshot::Receiver<()>),
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

    async fn invalidate_generation(&self, provider_id: i64) {
        let mut inner = self.shared.inner.lock().await;
        let generation = inner.allocate_generation();
        {
            let entry = inner.entries.entry(provider_id).or_default();
            entry.generation = generation;
            entry.schedule = None;
            entry.recovery = None;
            if let Some(flight) = &entry.in_flight { flight.budget.invalidate(); }
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
            .result
    }

    async fn probe<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db: db::Db,
        provider_id: i64,
        expected_generation: Option<u64>,
        source: ProbeSource,
    ) -> Option<CompletedProbe> {
        loop {
            if source.is_expired(provider_id, crate::shared::time::now_unix_millis()) {
                return None;
            }
            let expected_recovery = match source {
                ProbeSource::Recovery {
                    due_at_ms,
                    recovery_epoch,
                } => Some((recovery_epoch, due_at_ms)),
                ProbeSource::Manual | ProbeSource::Scheduled { .. } => None,
            };
            let decision = match expected_recovery {
                Some(expected_recovery) => {
                    self.begin_probe_with_recovery(
                        provider_id,
                        expected_generation,
                        Some(expected_recovery),
                    )
                    .await
                }
                None => self.begin_probe(provider_id, expected_generation).await,
            };
            match decision {
                ProbeDecision::Stale => return None,
                ProbeDecision::Wait(receiver) => {
                    return Some(
                        receiver
                            .await
                            .unwrap_or_else(|_| coordinator_stopped_completion()),
                    );
                }
                ProbeDecision::WaitForTurn(receiver) => {
                    let _ = receiver.await;
                }
                ProbeDecision::Lead {
                    generation,
                    budget,
                    receiver,
                } => {
                    let trace_id = match source {
                        ProbeSource::Manual => manual_trace_id(provider_id),
                        ProbeSource::Scheduled { boundary_ms } => {
                            scheduled_trace_id(provider_id, boundary_ms)
                        }
                        ProbeSource::Recovery { due_at_ms, .. } => {
                            recovery_trace_id(provider_id, due_at_ms)
                        }
                    };
                    // The requester may time out or disconnect. Keep the real probe
                    // alive so its shared flight is always completed and released.
                    let state = self.clone();
                    tokio::spawn(async move {
                        let work = async {
                            let result = provider_availability::test_provider_availability_with_budget(
                                &app, db.clone(), provider_id, budget.clone(),
                            ).await;
                            let result = if budget.expired() { budget.timeout_result() } else { result };
                            if result.is_err() {
                                let mut inner = state.shared.inner.lock().await;
                                complete_flight(&mut inner, provider_id, generation, &budget, result, None);
                            } else {
                                state.finish_probe(&app, &db, provider_id, generation,
                                    budget.clone(), &trace_id, result).await;
                            }
                        };
                        if tokio::time::timeout_at(budget.deadline, work).await.is_err() {
                            let mut inner = state.shared.inner.lock().await;
                            complete_flight(&mut inner, provider_id, generation, &budget,
                                budget.timeout_result(), None);
                        }
                    });
                    return Some(
                        receiver
                            .await
                            .unwrap_or_else(|_| coordinator_stopped_completion()),
                    );
                }
            }
        }
    }

    async fn begin_probe(
        &self,
        provider_id: i64,
        expected_generation: Option<u64>,
    ) -> ProbeDecision {
        self.begin_probe_with_recovery(provider_id, expected_generation, None)
            .await
    }

    async fn begin_probe_with_recovery(
        &self,
        provider_id: i64,
        expected_generation: Option<u64>,
        expected_recovery: Option<(u64, i64)>,
    ) -> ProbeDecision {
        if expected_recovery.is_some() && expected_generation.is_none() {
            return ProbeDecision::Stale;
        }
        let _gate = self.provider_mutation_gate(provider_id).lock_owned().await;
        let mut inner = self.shared.inner.lock().await;
        if !inner.entries.contains_key(&provider_id) {
            if expected_generation.is_some() {
                return ProbeDecision::Stale;
            }
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
        if let Some((recovery_epoch, due_at_ms)) = expected_recovery {
            if !recovery_claim_matches(entry, generation, recovery_epoch, due_at_ms) {
                return ProbeDecision::Stale;
            }
        }
        if let Some(in_flight) = entry.in_flight.as_mut() {
            if in_flight.generation == generation {
                let (sender, receiver) = oneshot::channel();
                in_flight.waiters.push(sender);
                return ProbeDecision::Wait(receiver);
            }
            let (sender, receiver) = oneshot::channel();
            in_flight.turn_waiters.push(sender);
            return ProbeDecision::WaitForTurn(receiver);
        }
        let (sender, receiver) = oneshot::channel();
        let budget = provider_availability::ProbeBudget::new();
        #[cfg(test)]
        {
            *budget.pause.lock().expect("probe test pause") = self.shared.next_pause.lock().expect("next probe pause").take();
        }
        entry.in_flight = Some(InFlightProbe {
            generation,
            recovery_epoch: entry.recovery_epoch,
            budget: budget.clone(),
            waiters: vec![sender],
            turn_waiters: Vec::new(),
        });
        ProbeDecision::Lead {
            generation,
            budget,
            receiver,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn finish_probe<R: tauri::Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        db: &db::Db,
        provider_id: i64,
        generation: u64,
        budget: Arc<provider_availability::ProbeBudget>,
        trace_id: &str,
        result: AppResult<ProviderAvailabilityResult>,
    ) {
        let state = self.clone();
        let app = app.clone();
        let db = db.clone();
        let trace_id = trace_id.to_string();
        let fallback = result.clone();
        let work_budget = budget.clone();
        let finish = blocking::run("provider_availability_finish", move || -> AppResult<()> {
            let budget = work_budget;
            budget.checkpoint("completion")?;
            budget.checkpoint("manager")?;
            let circuit = gateway_state::try_with_app_running_gateway(&app, |runtime| {
                runtime.map(|runtime| runtime.availability_probe_circuit())
            }).flatten();
            budget.check()?;
            let completed_at_ms = crate::shared::time::now_unix_millis();
            // An IMMEDIATE transaction orders this prepared fact before later
            // configuration writes. A timed-out or invalidated flight rolls it back.
            let mut conn = db.open_connection()?;
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| db_err!("failed to prepare probe observation: {e}"))?;
            budget.check()?;
            let cli_key = tx.query_row("SELECT cli_key FROM providers WHERE id = ?1", [provider_id], |row| row.get::<_, String>(0))
                .optional().map_err(|e| db_err!("failed to load probe evidence provider: {e}"))?;
            if let Ok(probe) = &result {
                if let Err(error) = provider_availability::record_probe_observation_in_conn(&tx, &trace_id, provider_id, completed_at_ms, probe.ok) {
                    tracing::warn!(error = %error.code(), provider_id, "provider availability observation write failed");
                }
            }
            budget.checkpoint("observation")?;
            budget.checkpoint("circuit")?;
            let mut accepted = false;
            let mut commit = |apply: &mut dyn FnMut() -> Option<bool>| {
                let mut inner = state.shared.inner.blocking_lock();
                if !flight_is_current(&inner, provider_id, generation, &budget) || budget.expired() {
                    return false;
                }
                let recovery = apply().and_then(|half_open| {
                    update_recovery_work_after_circuit_evidence(&mut inner, provider_id, generation,
                        completed_at_ms, result.as_ref().is_ok_and(|probe| probe.ok), half_open)
                });
                accepted = complete_flight(&mut inner, provider_id, generation, &budget, result.clone(), recovery);
                accepted
            };
            let effects = match (circuit.as_ref(), cli_key.as_deref(), result.as_ref()) {
                (Some(circuit), Some(_), Ok(probe)) if probe.provider_id == provider_id => {
                    Some(circuit.record_probe_outcome_if(provider_id,
                        completed_at_ms.div_euclid(1_000), probe.ok, &mut commit))
                }
                _ => { commit(&mut || None); None }
            };
            if accepted {
                // Result publication is the memory commit. Durable I/O and events
                // may finish later, without retaining the flight or mutation gate.
                if tx.commit().is_err() {
                    tracing::warn!(error = "DB_ERROR", provider_id, "provider availability observation commit failed");
                }
            } else {
                drop(tx);
            }
            if let (Some(effects), Some(cli_key), Ok(probe)) = (effects, cli_key.as_deref(), result.as_ref()) {
                crate::gateway::runtime::GatewayRuntime::publish_availability_probe_outcome(
                    Some(&app), &trace_id, cli_key, provider_id,
                    &probe.provider_name, &probe.base_url, completed_at_ms.div_euclid(1_000), effects);
            }
            Ok(())
        }).await;
        if let Err(error) = finish {
            if !budget.expired() {
                tracing::warn!(error = %error.code(), provider_id, "provider availability completion failed");
            }
        }
        let mut inner = self.shared.inner.lock().await;
        let result = if budget.expired() { budget.timeout_result() } else { fallback };
        complete_flight(&mut inner, provider_id, generation, &budget, result, None);
    }

    async fn run_scheduler<R: tauri::Runtime>(self, app: tauri::AppHandle<R>, db: db::Db) {
        let mut interval = tokio::time::interval(SCHEDULER_TICK);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_tick = Instant::now();
        let mut scan_epoch = 1_u64;
        let mut after_provider_id = None;
        loop {
            interval.tick().await;
            let tick = Instant::now();
            let skip_missed = tick.duration_since(last_tick) > SCHEDULER_SUSPEND_GAP;
            last_tick = tick;
            let now_ms = crate::shared::time::now_unix_millis();
            let db_for_load = db.clone();
            let schedules = blocking::run("provider_availability_probe_schedule", move || {
                load_schedules(&db_for_load, now_ms, after_provider_id)
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
            let scan_complete = batch.next_after_provider_id.is_none();
            after_provider_id = batch.next_after_provider_id;
            let targets = self
                .reconcile_schedules(
                    batch.schedules,
                    now_ms,
                    skip_missed,
                    scan_epoch,
                    scan_complete,
                )
                .await;
            if scan_complete {
                scan_epoch = next_schedule_scan_epoch(scan_epoch);
                after_provider_id = None;
            }
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
        scan_epoch: u64,
        scan_complete: bool,
    ) -> Vec<ScheduledProbeTarget> {
        let mut inner = self.shared.inner.lock().await;
        reconcile_schedules_inner(
            &mut inner,
            schedules,
            now_ms,
            skip_missed,
            scan_epoch,
            scan_complete,
        )
    }

    async fn schedule_recovery_probe(&self, provider_id: i64, recovery: RecoveryDirective) {
        let mut inner = self.shared.inner.lock().await;
        queue_recovery_target(&mut inner, provider_id, recovery);
    }

    async fn recovery_target_is_current(&self, target: ScheduledProbeTarget) -> bool {
        let inner = self.shared.inner.lock().await;
        recovery_target_is_current(&inner, target)
    }

    async fn settle_recovery_target(
        &self,
        target: ScheduledProbeTarget,
        recovery: Option<RecoveryDirective>,
    ) {
        let mut inner = self.shared.inner.lock().await;
        settle_recovery_target(&mut inner, target, recovery);
    }

    async fn consume_scheduled_completion(
        &self,
        target: ScheduledProbeTarget,
        completion: Option<&CompletedProbe>,
        _waiter_resumed_at_ms: i64,
    ) {
        // The recovery directive is fixed at HTTP completion; waiter scheduling
        // latency must not shift its due time.
        let recovery = completion.and_then(|completed| completed.recovery);
        if target.is_recovery() {
            self.settle_recovery_target(target, recovery).await;
        } else if let Some(recovery) = recovery {
            self.schedule_recovery_probe(target.provider_id, recovery)
                .await;
        }
    }

    async fn run_scheduled_probe<R: tauri::Runtime>(
        &self,
        app: tauri::AppHandle<R>,
        db: db::Db,
        target: ScheduledProbeTarget,
    ) {
        let now_ms = crate::shared::time::now_unix_millis();
        let remaining_ms = target.deadline_ms().saturating_sub(now_ms);
        if remaining_ms < 0 {
            self.settle_recovery_target(target, None).await;
            return;
        }
        let permit = match tokio::time::timeout(
            Duration::from_millis(u64::try_from(remaining_ms).unwrap_or_default()),
            self.shared.scheduled_limiter.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) | Err(_) => {
                self.settle_recovery_target(target, None).await;
                return;
            }
        };
        if target.is_recovery() {
            if !self.recovery_target_is_current(target).await {
                return;
            }
            if !running_gateway_circuit_is_half_open(
                &app,
                target.provider_id,
                crate::shared::time::now_unix_seconds(),
            ) {
                drop(permit);
                self.settle_recovery_target(target, None).await;
                return;
            }
        }
        let completion = self
            .probe(
                app.clone(),
                db,
                target.provider_id,
                Some(target.generation),
                target.probe_source(),
            )
            .await;
        drop(permit);
        let waiter_resumed_at_ms = crate::shared::time::now_unix_millis();
        self.consume_scheduled_completion(target, completion.as_ref(), waiter_resumed_at_ms)
            .await;
        if let Some(Err(error)) = completion.map(|completed| completed.result) {
            tracing::warn!(
                error = %error.code(),
                provider_id = target.provider_id,
                "scheduled provider availability probe failed"
            );
        }
    }
}

fn coordinator_stopped_completion() -> CompletedProbe {
    CompletedProbe {
        result: Err(AppError::new(
            "SYSTEM_ERROR",
            "provider probe coordinator stopped unexpectedly",
        )),
        recovery: None,
    }
}

fn running_gateway_circuit_is_half_open<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    provider_id: i64,
    now_unix: i64,
) -> bool {
    gateway_state::try_with_app_running_gateway(app, |runtime| {
        runtime.is_some_and(|runtime| runtime.circuit_is_half_open(provider_id, now_unix))
    })
    .unwrap_or(false)
}

fn take_finished_flight(
    inner: &mut RuntimeInner,
    provider_id: i64,
    generation: u64,
    budget: &Arc<provider_availability::ProbeBudget>,
) -> Option<(InFlightProbe, bool)> {
    let entry = inner.entries.get_mut(&provider_id)?;
    let flight = entry.in_flight.as_ref()?;
    if flight.generation != generation || !Arc::ptr_eq(&flight.budget, budget) {
        return None;
    }
    let in_flight = entry.in_flight.take()?;
    let should_record = entry.generation == generation;
    Some((in_flight, should_record))
}

fn flight_is_current(
    inner: &RuntimeInner,
    provider_id: i64,
    generation: u64,
    budget: &Arc<provider_availability::ProbeBudget>,
) -> bool {
    inner.entries.get(&provider_id).is_some_and(|entry| {
        entry.generation == generation && entry.in_flight.as_ref().is_some_and(|flight| {
            flight.generation == generation && flight.recovery_epoch == entry.recovery_epoch
                && Arc::ptr_eq(&flight.budget, budget)
        })
    })
}

fn complete_flight(
    inner: &mut RuntimeInner,
    provider_id: i64,
    generation: u64,
    budget: &Arc<provider_availability::ProbeBudget>,
    result: AppResult<ProviderAvailabilityResult>,
    recovery: Option<RecoveryDirective>,
) -> bool {
    let Some((flight, _)) = take_finished_flight(inner, provider_id, generation, budget) else { return false; };
    budget.invalidate();
    let completed = CompletedProbe { result, recovery };
    for waiter in flight.waiters { let _ = waiter.send(completed.clone()); }
    for waiter in flight.turn_waiters { let _ = waiter.send(()); }
    remove_idle_entry(inner, provider_id);
    true
}

fn invalidate_recovery_work(inner: &mut RuntimeInner, provider_id: i64, generation: u64) {
    let Some(entry) = inner.entries.get_mut(&provider_id) else {
        return;
    };
    if entry.generation != generation {
        return;
    }
    entry.recovery = None;
    entry.recovery_epoch = next_recovery_epoch(entry.recovery_epoch);
}

fn update_recovery_work_after_circuit_evidence(
    inner: &mut RuntimeInner,
    provider_id: i64,
    generation: u64,
    completed_at_ms: i64,
    probe_ok: bool,
    circuit_is_half_open: bool,
) -> Option<RecoveryDirective> {
    if !probe_ok || !circuit_is_half_open {
        invalidate_recovery_work(inner, provider_id, generation);
        return None;
    }
    inner.entries.get(&provider_id).and_then(|entry| {
        (entry.generation == generation).then_some(RecoveryDirective::from_completion(
            generation,
            entry.recovery_epoch,
            completed_at_ms,
        ))
    })
}

fn recovery_target_is_current(inner: &RuntimeInner, target: ScheduledProbeTarget) -> bool {
    let ScheduledProbeSource::Recovery { recovery_epoch } = target.source else {
        return false;
    };
    inner.entries.get(&target.provider_id).is_some_and(|entry| {
        recovery_claim_matches(entry, target.generation, recovery_epoch, target.due_at_ms)
    })
}

fn recovery_claim_matches(
    entry: &RuntimeEntry,
    generation: u64,
    recovery_epoch: u64,
    due_at_ms: i64,
) -> bool {
    entry.generation == generation
        && entry.recovery.is_some_and(|recovery| {
            recovery.generation == generation
                && recovery.recovery_epoch == recovery_epoch
                && recovery.due_at_ms == due_at_ms
                && recovery.phase == RecoveryProbePhase::Claimed
        })
}

fn settle_recovery_target(
    inner: &mut RuntimeInner,
    target: ScheduledProbeTarget,
    recovery: Option<RecoveryDirective>,
) {
    if !target.is_recovery() || !recovery_target_is_current(inner, target) {
        return;
    }
    if let Some(entry) = inner.entries.get_mut(&target.provider_id) {
        entry.recovery = None;
    }
    if let Some(recovery) = recovery {
        queue_recovery_target(inner, target.provider_id, recovery);
    }
    remove_idle_entry(inner, target.provider_id);
}

fn remove_idle_entry(inner: &mut RuntimeInner, provider_id: i64) {
    let should_remove = inner.entries.get(&provider_id).is_some_and(|entry| {
        entry.schedule.is_none() && entry.recovery.is_none() && entry.in_flight.is_none()
    });
    if should_remove {
        inner.entries.remove(&provider_id);
    }
}

fn reconcile_schedules_inner(
    inner: &mut RuntimeInner,
    schedules: Vec<LoadedSchedule>,
    now_ms: i64,
    skip_missed: bool,
    scan_epoch: u64,
    scan_complete: bool,
) -> Vec<ScheduledProbeTarget> {
    let mut targets = Vec::new();
    for loaded in schedules {
        if !loaded.active {
            let had_scheduled_work =
                inner
                    .entries
                    .get_mut(&loaded.provider_id)
                    .is_some_and(|entry| {
                        let had_schedule = entry.schedule.take().is_some();
                        let had_recovery = entry.recovery.take().is_some();
                        had_schedule || had_recovery
                    });
            if had_scheduled_work {
                let generation = inner.allocate_generation();
                if let Some(entry) = inner.entries.get_mut(&loaded.provider_id) {
                    entry.generation = generation;
                    if let Some(flight) = &entry.in_flight { flight.budget.invalidate(); }
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
            if let Some(flight) = &entry.in_flight { flight.budget.invalidate(); }
            entry.schedule_seen_epoch = scan_epoch;
            entry.schedule = Some(ScheduledProbeState {
                config,
                next_boundary_ms: loaded.next_boundary_ms,
            });
            entry.recovery = None;
            continue;
        }

        let entry = inner
            .entries
            .get_mut(&loaded.provider_id)
            .expect("active schedule entry");
        entry.schedule_seen_epoch = scan_epoch;
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
            due_at_ms,
            source: ScheduledProbeSource::Natural { boundary_ms },
        });
    }

    if scan_complete {
        let missing_provider_ids = inner
            .entries
            .iter()
            .filter_map(|(provider_id, entry)| {
                (entry.schedule.is_some() && entry.schedule_seen_epoch != scan_epoch)
                    .then_some(*provider_id)
            })
            .collect::<Vec<_>>();
        for provider_id in missing_provider_ids {
            let had_scheduled_work = inner.entries.get_mut(&provider_id).is_some_and(|entry| {
                let had_schedule = entry.schedule.take().is_some();
                let had_recovery = entry.recovery.take().is_some();
                had_schedule || had_recovery
            });
            if had_scheduled_work {
                let generation = inner.allocate_generation();
                if let Some(entry) = inner.entries.get_mut(&provider_id) {
                    entry.generation = generation;
                    if let Some(flight) = &entry.in_flight { flight.budget.invalidate(); }
                }
            }
            remove_idle_entry(inner, provider_id);
        }
    }
    targets.extend(take_due_recovery_targets(inner, now_ms, skip_missed));
    targets
}

fn take_due_recovery_targets(
    inner: &mut RuntimeInner,
    now_ms: i64,
    skip_missed: bool,
) -> Vec<ScheduledProbeTarget> {
    let provider_ids = inner.entries.keys().copied().collect::<Vec<_>>();
    let mut targets = Vec::new();
    for provider_id in provider_ids {
        let target = {
            let Some(entry) = inner.entries.get_mut(&provider_id) else {
                continue;
            };
            let Some(recovery) = entry.recovery else {
                continue;
            };
            if entry.schedule.is_none() || entry.generation != recovery.generation {
                entry.recovery = None;
                None
            } else if recovery.phase == RecoveryProbePhase::Claimed || now_ms < recovery.due_at_ms {
                None
            } else if skip_missed || now_ms > recovery_due_deadline_ms(recovery.due_at_ms) {
                entry.recovery = None;
                None
            } else {
                entry.recovery = Some(RecoveryProbeState {
                    phase: RecoveryProbePhase::Claimed,
                    ..recovery
                });
                Some(ScheduledProbeTarget {
                    provider_id,
                    generation: recovery.generation,
                    due_at_ms: recovery.due_at_ms,
                    source: ScheduledProbeSource::Recovery {
                        recovery_epoch: recovery.recovery_epoch,
                    },
                })
            }
        };
        if let Some(target) = target {
            targets.push(target);
        }
        remove_idle_entry(inner, provider_id);
    }
    targets
}

fn queue_recovery_target(inner: &mut RuntimeInner, provider_id: i64, recovery: RecoveryDirective) {
    let Some(entry) = inner.entries.get_mut(&provider_id) else {
        return;
    };
    if entry.generation != recovery.generation
        || entry.recovery_epoch != recovery.recovery_epoch
        || entry.schedule.is_none()
        || entry.recovery.is_some()
    {
        return;
    }
    entry.recovery = Some(RecoveryProbeState {
        generation: recovery.generation,
        recovery_epoch: recovery.recovery_epoch,
        due_at_ms: recovery.due_at_ms,
        phase: RecoveryProbePhase::Pending,
    });
}

fn next_recovery_epoch(current: u64) -> u64 {
    let next = current.wrapping_add(1);
    if next == 0 {
        1
    } else {
        next
    }
}

fn next_schedule_scan_epoch(current: u64) -> u64 {
    let next = current.wrapping_add(1);
    if next == 0 {
        1
    } else {
        next
    }
}

fn load_schedules(
    db: &db::Db,
    now_ms: i64,
    after_provider_id: Option<i64>,
) -> AppResult<LoadedScheduleBatch> {
    let conn = db.open_connection()?;
    load_schedules_from_conn(&conn, now_ms, after_provider_id)
}

fn load_schedules_from_conn(
    conn: &rusqlite::Connection,
    now_ms: i64,
    after_provider_id: Option<i64>,
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
    AND p.id > ?2
  ORDER BY p.id ASC
  LIMIT ?3
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
ORDER BY id ASC
"#,
        )
        .map_err(|error| db_err!("failed to prepare Provider probe schedules: {error}"))?;
    let rows = statement
        .query_map(
            params![
                now_seconds,
                after_provider_id.unwrap_or_default(),
                (SCHEDULED_PROVIDER_PAGE_SIZE + 1) as i64
            ],
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
    let has_more = schedules.len() > SCHEDULED_PROVIDER_PAGE_SIZE;
    schedules.truncate(SCHEDULED_PROVIDER_PAGE_SIZE);
    let next_after_provider_id = if has_more {
        schedules.last().map(|schedule| schedule.provider_id)
    } else {
        None
    };
    Ok(LoadedScheduleBatch {
        schedules,
        next_after_provider_id,
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

fn scheduled_due_deadline_ms(provider_id: i64, boundary_ms: i64) -> i64 {
    scheduled_due_at_ms(provider_id, boundary_ms).saturating_add(SCHEDULED_DUE_GRACE_MS)
}

fn recovery_due_deadline_ms(due_at_ms: i64) -> i64 {
    due_at_ms.saturating_add(SCHEDULED_DUE_GRACE_MS)
}

fn scheduled_trace_id(provider_id: i64, boundary_ms: i64) -> String {
    format!("availability-probe:{provider_id}:{boundary_ms}")
}

fn recovery_trace_id(provider_id: i64, due_at_ms: i64) -> String {
    format!("availability-probe:recovery:{provider_id}:{due_at_ms}")
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

    fn queue_recovery_from_completion(
        inner: &mut RuntimeInner,
        provider_id: i64,
        generation: u64,
        completed_at_ms: i64,
    ) {
        let recovery_epoch = inner.entries[&provider_id].recovery_epoch;
        queue_recovery_target(
            inner,
            provider_id,
            RecoveryDirective::from_completion(generation, recovery_epoch, completed_at_ms),
        );
    }

    #[test]
    fn startup_and_configuration_changes_schedule_only_the_next_boundary() {
        let mut inner = RuntimeInner::default();
        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(7, 1, 60_000)],
            55_000,
            false,
            1,
            true,
        );
        assert!(targets.is_empty());
        let first_generation = inner.entries[&7].generation;
        queue_recovery_from_completion(&mut inner, 7, first_generation, 55_000);

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(7, 2, 120_000)],
            65_000,
            false,
            2,
            true,
        );
        assert!(targets.is_empty());
        assert_eq!(
            inner.entries[&7].schedule.unwrap().next_boundary_ms,
            120_000
        );
        assert!(inner.entries[&7].recovery.is_none());
        assert_ne!(inner.entries[&7].generation, first_generation);
    }

    #[test]
    fn recovery_targets_run_once_without_replay_or_replacement() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            1_000,
            false,
            1,
            true,
        );
        let generation = inner.entries[&3].generation;
        let recovery_epoch = inner.entries[&3].recovery_epoch;
        queue_recovery_from_completion(&mut inner, 3, generation, 10_000);
        queue_recovery_from_completion(&mut inner, 3, generation, 20_000);
        let due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;
        assert_eq!(inner.entries[&3].recovery.unwrap().due_at_ms, due_at_ms);

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            due_at_ms - 1,
            false,
            2,
            true,
        );
        assert!(targets.is_empty());

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            due_at_ms,
            false,
            3,
            true,
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].source,
            ScheduledProbeSource::Recovery { recovery_epoch }
        );
        assert_eq!(targets[0].generation, generation);
        assert_eq!(targets[0].due_at_ms, due_at_ms);
        assert_eq!(
            inner.entries[&3].recovery.unwrap().phase,
            RecoveryProbePhase::Claimed
        );
        assert!(recovery_target_is_current(&inner, targets[0]));

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            due_at_ms + 1,
            false,
            4,
            true,
        );
        assert!(targets.is_empty());
    }

    #[test]
    fn valid_half_open_failure_invalidates_pending_and_claimed_recovery_work() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            1_000,
            false,
            1,
            true,
        );
        let generation = inner.entries[&3].generation;
        let old_recovery = RecoveryDirective::from_completion(
            generation,
            inner.entries[&3].recovery_epoch,
            10_000,
        );
        queue_recovery_target(&mut inner, 3, old_recovery);
        let old_due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;

        assert!(update_recovery_work_after_circuit_evidence(
            &mut inner, 3, generation, 10_000, false, false,
        )
        .is_none());
        queue_recovery_target(&mut inner, 3, old_recovery);
        assert!(inner.entries[&3].recovery.is_none());
        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            old_due_at_ms,
            false,
            2,
            true,
        );
        assert!(targets.is_empty());

        queue_recovery_from_completion(&mut inner, 3, generation, 20_000);
        let new_due_at_ms = 20_000 + RECOVERY_PROBE_DELAY_MS;
        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            new_due_at_ms,
            false,
            3,
            true,
        );
        assert_eq!(targets.len(), 1);
        assert!(recovery_target_is_current(&inner, targets[0]));

        assert!(update_recovery_work_after_circuit_evidence(
            &mut inner, 3, generation, 20_000, false, false,
        )
        .is_none());
        assert!(!recovery_target_is_current(&inner, targets[0]));
    }

    #[test]
    fn successful_circuit_closure_invalidates_claimed_recovery_work() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            1_000,
            false,
            1,
            true,
        );
        let generation = inner.entries[&3].generation;
        queue_recovery_from_completion(&mut inner, 3, generation, 10_000);
        let due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;
        let target = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            due_at_ms,
            false,
            2,
            true,
        )
        .into_iter()
        .next()
        .expect("claimed recovery target");
        assert!(recovery_target_is_current(&inner, target));

        assert!(update_recovery_work_after_circuit_evidence(
            &mut inner, 3, generation, 20_000, true, false,
        )
        .is_none());
        assert!(!recovery_target_is_current(&inner, target));
    }

    #[test]
    fn settling_a_recovery_target_replaces_only_the_claimed_target() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            1_000,
            false,
            1,
            true,
        );
        let generation = inner.entries[&3].generation;
        queue_recovery_from_completion(&mut inner, 3, generation, 10_000);
        let due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;
        let mut targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 90_000)],
            due_at_ms,
            false,
            2,
            true,
        );
        let target = targets.pop().expect("due recovery target");
        let next_recovery = RecoveryDirective::from_completion(
            generation,
            inner.entries[&3].recovery_epoch,
            50_000,
        );

        settle_recovery_target(&mut inner, target, Some(next_recovery));

        let recovery = inner.entries[&3].recovery.expect("next recovery target");
        assert_eq!(recovery.phase, RecoveryProbePhase::Pending);
        assert_eq!(recovery.due_at_ms, 50_000 + RECOVERY_PROBE_DELAY_MS);
    }

    #[tokio::test]
    async fn scheduled_completion_uses_probe_completion_time_not_waiter_resume_time() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let (generation, recovery_epoch) = {
            let mut inner = state.shared.inner.lock().await;
            reconcile_schedules_inner(
                &mut inner,
                vec![loaded(3, 1, 90_000)],
                1_000,
                false,
                1,
                true,
            );
            let entry = &inner.entries[&3];
            (entry.generation, entry.recovery_epoch)
        };
        let completed_at_ms = 10_000;
        let waiter_resumed_at_ms = 55_000;
        let target = ScheduledProbeTarget {
            provider_id: 3,
            generation,
            due_at_ms: scheduled_due_at_ms(3, 0),
            source: ScheduledProbeSource::Natural { boundary_ms: 0 },
        };
        let completion = CompletedProbe {
            result: Ok(ProviderAvailabilityResult {
                ok: true,
                provider_id: 3,
                provider_name: "test provider".to_string(),
                base_url: "https://example.test".to_string(),
                status: Some(200),
                latency_ms: 1,
                error: None,
                response_preview: None,
                requested_model: Some("test-model".into()),
                tested_model: Some("test-model".into()),
            }),
            recovery: Some(RecoveryDirective::from_completion(
                generation,
                recovery_epoch,
                completed_at_ms,
            )),
        };

        state
            .consume_scheduled_completion(target, Some(&completion), waiter_resumed_at_ms)
            .await;

        let inner = state.shared.inner.lock().await;
        let due_at_ms = inner.entries[&3].recovery.unwrap().due_at_ms;
        assert_eq!(due_at_ms, completed_at_ms + RECOVERY_PROBE_DELAY_MS);
        assert_ne!(due_at_ms, waiter_resumed_at_ms + RECOVERY_PROBE_DELAY_MS);
    }

    #[test]
    fn recovery_targets_cancel_on_schedule_change_and_missed_deadline() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(4, 1, 90_000)],
            1_000,
            false,
            1,
            true,
        );
        let first_generation = inner.entries[&4].generation;
        queue_recovery_from_completion(&mut inner, 4, first_generation, 10_000);

        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(4, 2, 90_000)],
            2_000,
            false,
            2,
            true,
        );
        assert!(inner.entries[&4].recovery.is_none());
        assert_ne!(inner.entries[&4].generation, first_generation);

        let generation = inner.entries[&4].generation;
        queue_recovery_from_completion(&mut inner, 4, generation, 10_000);
        let due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;
        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(4, 2, 90_000)],
            recovery_due_deadline_ms(due_at_ms).saturating_add(1),
            true,
            3,
            true,
        );
        assert!(targets.is_empty());
        assert!(inner.entries[&4].recovery.is_none());
    }

    #[test]
    fn due_boundaries_run_once_while_suspend_gaps_are_skipped() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 60_000)],
            50_000,
            false,
            1,
            true,
        );
        let due_at = scheduled_due_at_ms(3, 60_000);
        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 120_000)],
            due_at,
            false,
            2,
            true,
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].source,
            ScheduledProbeSource::Natural {
                boundary_ms: 60_000
            }
        );

        let targets = reconcile_schedules_inner(
            &mut inner,
            vec![loaded(3, 1, 180_000)],
            scheduled_due_at_ms(3, 120_000),
            true,
            3,
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
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(1, 1, 60_000)],
            1_000,
            false,
            1,
            true,
        );
        let generation = inner.entries[&1].generation;
        queue_recovery_from_completion(&mut inner, 1, generation, 1_000);
        let mut disabled = loaded(1, 1, 60_000);
        disabled.active = false;
        reconcile_schedules_inner(&mut inner, vec![disabled], 2_000, false, 2, true);
        assert!(!inner.entries.contains_key(&1));
        assert!(generation > 0);
    }

    #[test]
    fn reclaimed_provider_entries_receive_a_new_generation_after_recreation() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(1, 1, 60_000)],
            1_000,
            false,
            1,
            true,
        );
        let first_generation = inner.entries[&1].generation;
        reconcile_schedules_inner(&mut inner, Vec::new(), 2_000, false, 2, true);
        assert!(!inner.entries.contains_key(&1));

        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(1, 1, 60_000)],
            3_000,
            false,
            3,
            true,
        );
        assert_ne!(inner.entries[&1].generation, first_generation);
    }

    #[test]
    fn scheduled_identity_and_jitter_are_stable_and_bounded() {
        assert_eq!(
            scheduled_trace_id(9, 123_000),
            "availability-probe:9:123000"
        );
        assert_eq!(
            recovery_trace_id(9, 153_000),
            "availability-probe:recovery:9:153000"
        );
        let jitter = stable_jitter_ms(9);
        assert!((0..=3_000).contains(&jitter));
        assert_eq!(jitter, stable_jitter_ms(9));
    }

    #[test]
    fn scheduled_probe_source_expires_after_the_due_grace() {
        let source = ProbeSource::Scheduled {
            boundary_ms: 60_000,
        };
        let deadline_ms = scheduled_due_deadline_ms(9, 60_000);
        assert!(!source.is_expired(9, deadline_ms));
        assert!(source.is_expired(9, deadline_ms.saturating_add(1)));
        let recovery = ProbeSource::Recovery {
            due_at_ms: 90_000,
            recovery_epoch: 1,
        };
        let recovery_deadline_ms = recovery_due_deadline_ms(90_000);
        assert!(!recovery.is_expired(9, recovery_deadline_ms));
        assert!(recovery.is_expired(9, recovery_deadline_ms.saturating_add(1)));
        assert!(!ProbeSource::Manual.is_expired(9, i64::MAX));
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

        let batch = load_schedules_from_conn(&connection, 55_000, None).expect("load schedules");
        assert!(batch.next_after_provider_id.is_none());
        assert_eq!(batch.schedules.len(), 1);
        assert_eq!(batch.schedules[0].provider_id, 514);
    }

    #[test]
    fn schedule_loading_pages_through_every_enabled_provider() {
        let connection = schedule_test_connection();
        for provider_id in 1..=(SCHEDULED_PROVIDER_PAGE_SIZE as i64 + 1) {
            insert_schedule_provider(&connection, provider_id, true);
        }

        let first = load_schedules_from_conn(&connection, 55_000, None).expect("load first page");
        assert_eq!(first.schedules.len(), SCHEDULED_PROVIDER_PAGE_SIZE);
        assert_eq!(first.schedules.first().map(|row| row.provider_id), Some(1));
        assert_eq!(
            first.next_after_provider_id,
            Some(SCHEDULED_PROVIDER_PAGE_SIZE as i64)
        );

        let second = load_schedules_from_conn(&connection, 55_000, first.next_after_provider_id)
            .expect("load second page");
        assert!(second.next_after_provider_id.is_none());
        assert_eq!(second.schedules.len(), 1);
        assert_eq!(
            second.schedules[0].provider_id,
            SCHEDULED_PROVIDER_PAGE_SIZE as i64 + 1
        );
    }

    #[test]
    fn partial_schedule_scan_preserves_entries_until_the_cycle_completes() {
        let mut inner = RuntimeInner::default();
        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(1, 1, 60_000), loaded(600, 1, 60_000)],
            1_000,
            false,
            1,
            true,
        );

        reconcile_schedules_inner(
            &mut inner,
            vec![loaded(1, 1, 60_000)],
            2_000,
            false,
            2,
            false,
        );
        assert!(inner.entries.contains_key(&600));
        let generation = inner.entries[&600].generation;
        queue_recovery_from_completion(&mut inner, 600, generation, 2_000);

        reconcile_schedules_inner(&mut inner, Vec::new(), 3_000, false, 2, true);
        assert!(!inner.entries.contains_key(&600));
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
    async fn invalidated_flight_blocks_the_replacement_until_it_finishes() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let (generation, budget) = match state.begin_probe(4, None).await {
            ProbeDecision::Lead { generation, budget, .. } => (generation, budget),
            _ => panic!("first probe must lead"),
        };
        assert!(matches!(
            state.begin_probe(4, None).await,
            ProbeDecision::Wait(_)
        ));
        drop(state.begin_mutation(4).await);
        assert!(matches!(
            state.begin_probe(4, Some(generation)).await,
            ProbeDecision::Stale
        ));
        let turn_receiver = match state.begin_probe(4, None).await {
            ProbeDecision::WaitForTurn(receiver) => receiver,
            _ => panic!("new configuration must wait for the old flight"),
        };
        let in_flight = {
            let mut inner = state.shared.inner.lock().await;
            let (in_flight, should_record) = take_finished_flight(&mut inner, 4, generation, &budget)
                .expect("invalidated flight remains available for completion");
            assert!(
                !should_record,
                "an invalidated probe must not write an observation"
            );
            in_flight
        };
        for waiter in in_flight.turn_waiters {
            let _ = waiter.send(());
        }
        turn_receiver.await.expect("replacement turn released");
        let replacement_generation = match state.begin_probe(4, None).await {
            ProbeDecision::Lead { generation, .. } => generation,
            _ => panic!("new configuration must lead after the old flight finishes"),
        };
        assert_ne!(replacement_generation, generation);
    }

    #[tokio::test]
    async fn timed_out_probe_completes_shared_waiters_and_allows_another_flight() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let (generation, budget, first) = match state.begin_probe(7, None).await {
            ProbeDecision::Lead { generation, budget, receiver } => (generation, budget, receiver),
            _ => panic!("first probe must lead"),
        };
        let second = match state.begin_probe(7, None).await {
            ProbeDecision::Wait(receiver) => receiver,
            _ => panic!("same provider must coalesce"),
        };
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("timeout.db")).unwrap();
        let app = tauri::test::mock_app();
        state.finish_probe(app.handle(), &db, 7, generation, budget, "timeout-probe",
            Err(AppError::new("PROBE_TIMEOUT", "synthetic timeout"))).await;
        for waiter in [first, second] {
            assert_eq!(waiter.await.unwrap().result.unwrap_err().code(), "PROBE_TIMEOUT");
        }
        assert!(matches!(state.begin_probe(7, None).await, ProbeDecision::Lead { .. }));
    }

    #[allow(clippy::await_holding_lock)]
    #[tokio::test(start_paused = true)]
    async fn shared_deadline_releases_waiters_and_isolates_late_blocking_work() {
        let _env_lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().unwrap();
        let _home = crate::test_support::ScopedTestEnvVar::set("AIO_CODING_HUB_TEST_HOME", temp.path());
        let app = tauri::test::mock_app();
        // Tauri blocking jobs run on its runtime; keep this clock from advancing
        // automatically while they communicate with the paused test runtime.
        let clock_driver = tokio::spawn(async { loop { tokio::task::yield_now().await; } });
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let _upstream = crate::test_support::ScopedTestEnvVar::set("AIO_CODING_HUB_TEST_PROBE_OAUTH_BASE_URL", &url);
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = requests.clone();
        let router = axum::Router::new().route("/v1/messages", axum::routing::post(move || {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, Ordering::SeqCst);
                axum::Json(serde_json::json!({"type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn"}))
            }
        })).route("/token", axum::routing::post(|| async {
            axum::Json(serde_json::json!({"access_token":"synthetic-refreshed-token","token_type":"Bearer","expires_in":3600}))
        }));
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap(); });

        for stage in ["configuration", "oauth_read", "oauth_write", "completion", "observation", "configuration_change"] {
            let db = crate::db::init_for_tests(&temp.path().join(format!("{stage}.db"))).unwrap();
            let conn = db.open_connection().unwrap();
            conn.execute("INSERT INTO providers(provider_uuid, cli_key, name, base_url, api_key_plaintext, created_at, updated_at) VALUES (?1, 'claude', 'synthetic probe', ?2, 'synthetic-key', 1, 1)", params![crate::shared::uuid::new_uuid_v4(), url]).unwrap();
            let provider_id = conn.last_insert_rowid();
            drop(conn);
            if matches!(stage, "oauth_read" | "oauth_write") {
                crate::providers::update_oauth_tokens(&db, provider_id, "oauth", "claude_oauth", "synthetic-token", Some("synthetic-refresh-token"),
                    None, &format!("{url}/token"), "synthetic-client", None,
                    Some(crate::shared::time::now_unix_seconds() + if stage == "oauth_write" { -1 } else { 3_600 }), None).unwrap();
            }
            let state = ProviderAvailabilityProbeRuntimeState::default();
            state.reconcile_schedules(vec![loaded(provider_id, 1, 60_000)], 0, false, 1, true).await;
            let (entered, ready) = oneshot::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let (exited, ended) = oneshot::channel();
            *state.shared.next_pause.lock().unwrap() = Some(provider_availability::ProbeTestPause {
                stage: if stage == "configuration_change" { "observation" } else { stage },
                entered, release: blocked, exited,
            });
            let first = tokio::spawn({
                let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                async move { state.probe_manual(app, db, provider_id).await }
            });
            ready.await.unwrap();
            let (generation, old_budget) = {
                let inner = state.shared.inner.lock().await;
                let flight = inner.entries[&provider_id].in_flight.as_ref().unwrap();
                (flight.generation, flight.budget.clone())
            };
            let second = tokio::spawn({
                let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                async move { state.probe_manual(app, db, provider_id).await }
            });
            loop {
                if state.shared.inner.lock().await.entries[&provider_id].in_flight.as_ref().unwrap().waiters.len() == 2 { break; }
                tokio::task::yield_now().await;
            }
            tokio::time::advance(Duration::from_secs(60)).await;
            for waiter in [first, second] {
                match waiter.await.unwrap() {
                    Ok(result) => {
                        assert!(!result.ok, "{stage}");
                        assert!(result.error.as_deref().unwrap().starts_with("PROBE_TIMEOUT"));
                        assert_eq!(result.tested_model.as_deref(), Some("claude-sonnet-4-6"));
                    }
                    Err(error) => {
                        assert_eq!(stage, "configuration", "known models must survive timeout");
                        assert_eq!(error.code(), "PROBE_TIMEOUT");
                    }
                }
            }
            assert!(state.shared.inner.lock().await.entries[&provider_id].in_flight.is_none());
            if stage == "configuration_change" {
                let mutation = state.begin_mutation(provider_id).await.unwrap();
                release.send(()).unwrap();
                ended.await.unwrap();
                let db_update = db.clone();
                blocking::run("probe_timeout_configuration_change", move || -> AppResult<()> {
                    let conn = db_update.open_connection()?;
                    conn.execute("UPDATE providers SET name = 'new configuration' WHERE id = ?1", [provider_id])
                        .map_err(|e| db_err!("test configuration write failed: {e}"))?;
                    Ok(())
                }).await.unwrap();
                drop(mutation);
                let fresh = state.probe_manual(app.handle().clone(), db.clone(), provider_id).await.unwrap();
                assert!(fresh.ok);
                assert_eq!(fresh.provider_name, "new configuration");
                let db_check = db.clone();
                let successes = blocking::run("probe_configuration_observations", move || -> AppResult<i64> {
                    let mut conn = db_check.open_connection()?;
                    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
                    Ok(tx.query_row("SELECT COUNT(*) FROM provider_availability_observations WHERE success = 1", [], |row| row.get(0)).unwrap())
                }).await.unwrap();
                assert_eq!(successes, 1, "old prepared success cannot survive configuration invalidation");
                continue;
            }

            let (entered, ready) = oneshot::channel();
            let (next_release, blocked) = std::sync::mpsc::channel();
            let (exited, next_ended) = oneshot::channel();
            *state.shared.next_pause.lock().unwrap() = Some(provider_availability::ProbeTestPause {
                stage: "configuration", entered, release: blocked, exited,
            });
            let next = tokio::spawn({
                let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                async move { state.probe_manual(app, db, provider_id).await }
            });
            ready.await.unwrap();
            let next_budget = {
                let mut inner = state.shared.inner.lock().await;
                let entry = inner.entries.get_mut(&provider_id).unwrap();
                assert_eq!(entry.generation, generation, "same configuration must remain isolated");
                let next_budget = entry.in_flight.as_ref().unwrap().budget.clone();
                assert!(!Arc::ptr_eq(&old_budget, &next_budget));
                queue_recovery_from_completion(&mut inner, provider_id, generation, 1_000);
                next_budget
            };
            let recovery = state.shared.inner.lock().await.entries[&provider_id].recovery;
            release.send(()).unwrap();
            ended.await.unwrap();
            let db_check = db.clone();
            let observations = blocking::run("probe_timeout_observation_check", move || -> AppResult<i64> {
                let mut conn = db_check.open_connection()?;
                let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
                Ok(tx.query_row("SELECT COUNT(*) FROM provider_availability_observations", [], |row| row.get(0)).unwrap())
            }).await.unwrap();
            assert_eq!(observations, 0, "{stage}: late success must roll back");
            if stage == "oauth_write" {
                assert_eq!(crate::providers::get_oauth_details(&db, provider_id).unwrap().oauth_access_token, "synthetic-token");
            }
            {
                let mut inner = state.shared.inner.lock().await;
                assert!(flight_is_current(&inner, provider_id, generation, &next_budget));
                assert_eq!(inner.entries[&provider_id].recovery, recovery);
                assert!(!complete_flight(&mut inner, provider_id, generation, &old_budget, old_budget.timeout_result(), None));
                assert!(flight_is_current(&inner, provider_id, generation, &next_budget));
            }
            next_release.send(()).unwrap();
            next_ended.await.unwrap();
            assert!(next.await.unwrap().unwrap().ok, "{stage}: next real probe must succeed");
            assert!(state.shared.inner.lock().await.entries[&provider_id].in_flight.is_none());
        }
        assert_eq!(requests.load(Ordering::SeqCst), 9, "no retries or late generation requests");
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
        clock_driver.abort();
        assert!(clock_driver.await.unwrap_err().is_cancelled());
    }

    #[test]
    fn observation_transaction_and_real_circuit_resets_do_not_invert_locks() {
        let clock = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let temp = tempfile::tempdir().unwrap();
        for reset_cli in [false, true] {
            let app = tauri::test::mock_app();
            app.manage(gateway_state::GatewayState::default());
            let gateway = crate::gateway::runtime::GatewayRuntime::for_probe_tests(&clock);
            gateway_state::with_app_running_gateway_slot_mut(app.handle(), |slot| *slot = Some(gateway));
            let db = crate::db::init_for_tests(&temp.path().join(format!("reset-{reset_cli}.db"))).unwrap();
            let conn = db.open_connection().unwrap();
            conn.execute("INSERT INTO providers(provider_uuid, cli_key, name, base_url, api_key_plaintext, created_at, updated_at) VALUES (?1, 'claude', 'synthetic reset', 'https://example.test', 'synthetic-key', 1, 1)", [crate::shared::uuid::new_uuid_v4()]).unwrap();
            let provider_id = conn.last_insert_rowid();
            drop(conn);
            clock.block_on(async {
                let state = ProviderAvailabilityProbeRuntimeState::default();
                let (generation, budget, receiver) = match state.begin_probe(provider_id, None).await {
                    ProbeDecision::Lead { generation, budget, receiver } => (generation, budget, receiver),
                    _ => panic!("first probe leads"),
                };
                let (entered, ready) = oneshot::channel();
                let (release, blocked) = std::sync::mpsc::channel();
                let (exited, ended) = oneshot::channel();
                *budget.pause.lock().unwrap() = Some(provider_availability::ProbeTestPause {
                    stage: "observation", entered, release: blocked, exited,
                });
                let finish = tokio::spawn({
                    let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                    async move {
                        state.finish_probe(&app, &db, provider_id, generation, budget, "reset-race", Ok(ProviderAvailabilityResult {
                            ok: true, provider_id, provider_name: "synthetic reset".into(),
                            base_url: "https://example.test".into(), status: Some(200), latency_ms: 1,
                            error: None, response_preview: None, requested_model: None, tested_model: None,
                        })).await;
                    }
                });
                ready.await.unwrap();
                let (manager_entered, manager_ready) = oneshot::channel();
                let (reset_done, reset_result) = oneshot::channel();
                let reset = std::thread::spawn({
                    let app = app.handle().clone(); let db = db.clone();
                    move || {
                        let result = gateway_state::try_with_app_running_gateway(&app, |running| {
                            manager_entered.send(()).unwrap();
                            if reset_cli {
                                crate::gateway::control_service::GatewayControlService::circuit_reset_cli(running, &db, "claude")
                                    .map(|count| assert_eq!(count, 1))
                            } else {
                                crate::gateway::control_service::GatewayControlService::circuit_reset_provider(running, &db, provider_id)
                            }
                        }).unwrap();
                        reset_done.send(result).unwrap();
                    }
                });
                manager_ready.await.unwrap();
                release.send(()).unwrap();
                ended.await.unwrap();
                tokio::time::timeout(Duration::from_secs(2), finish).await.unwrap().unwrap();
                tokio::time::timeout(Duration::from_secs(2), reset_result).await.unwrap().unwrap().unwrap();
                reset.join().unwrap();
                assert!(receiver.await.unwrap().result.unwrap().ok);
                let count: i64 = db.open_connection().unwrap().query_row(
                    "SELECT COUNT(*) FROM provider_availability_observations WHERE success = 1", [], |row| row.get(0),
                ).unwrap();
                assert_eq!(count, 1);
            });
            gateway_state::take_app_running_gateway(app.handle());
        }
    }

    #[test]
    fn shared_deadline_survives_gateway_and_circuit_lock_waits() {
        let _env_lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().unwrap();
        let _home = crate::test_support::ScopedTestEnvVar::set("AIO_CODING_HUB_TEST_HOME", temp.path());
        let clock = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        for hold_manager in [true, false] {
            let gateway = crate::gateway::runtime::GatewayRuntime::for_probe_tests(&clock);
            let circuit = gateway.availability_probe_circuit();
            let app = tauri::test::mock_app();
            app.manage(gateway_state::GatewayState::default());
            gateway_state::with_app_running_gateway_slot_mut(app.handle(), |slot| *slot = Some(gateway));
            clock.block_on(async {
                tokio::time::pause();
                let clock_driver = tokio::spawn(async { loop { tokio::task::yield_now().await; } });
                let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
                let url = format!("http://{}", listener.local_addr().unwrap());
                let router = axum::Router::new().route("/v1/messages", axum::routing::post(|| async {
                    axum::Json(serde_json::json!({"type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn"}))
                }));
                let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap(); });
                let db = crate::db::init_for_tests(&temp.path().join(format!("lock-{hold_manager}.db"))).unwrap();
                let conn = db.open_connection().unwrap();
                conn.execute("INSERT INTO providers(provider_uuid, cli_key, name, base_url, api_key_plaintext, created_at, updated_at) VALUES (?1, 'claude', 'synthetic lock probe', ?2, 'synthetic-key', 1, 1)", params![crate::shared::uuid::new_uuid_v4(), url]).unwrap();
                let provider_id = conn.last_insert_rowid();
                drop(conn);
                circuit.update_config(crate::circuit_breaker::CircuitBreakerConfig { failure_threshold: 1, open_duration_secs: 1 });
                let now = crate::shared::time::now_unix_seconds();
                circuit.record_failure(provider_id, now - 2, None);
                let state = ProviderAvailabilityProbeRuntimeState::default();
                state.reconcile_schedules(vec![loaded(provider_id, 1, 60_000)], 0, false, 1, true).await;

                let (entered, ready) = oneshot::channel();
                let (release, blocked) = std::sync::mpsc::channel();
                let lock_holder = std::thread::spawn({
                    let app = app.handle().clone(); let circuit = circuit.clone();
                    move || {
                        let hold = || { entered.send(()).unwrap(); blocked.recv().unwrap(); };
                        if hold_manager {
                            gateway_state::try_with_app_running_gateway(&app, |_| hold());
                        } else {
                            circuit.hold_probe_health_for_test(hold);
                        }
                    }
                });
                ready.await.unwrap();
                let (entered, ready) = oneshot::channel();
                let (release_preparation, blocked) = std::sync::mpsc::channel();
                let (exited, prepared) = oneshot::channel();
                *state.shared.next_pause.lock().unwrap() = Some(provider_availability::ProbeTestPause {
                    stage: if hold_manager { "manager" } else { "circuit" }, entered, release: blocked, exited,
                });
                let first = tokio::spawn({
                    let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                    async move { state.probe_manual(app, db, provider_id).await }
                });
                ready.await.unwrap();
                let second = match state.begin_probe(provider_id, None).await {
                    ProbeDecision::Wait(receiver) => receiver,
                    _ => panic!("lock-waiting probe must remain shared"),
                };
                release_preparation.send(()).unwrap();
                prepared.await.unwrap();
                tokio::time::advance(Duration::from_secs(60)).await;
                for result in [first.await.unwrap(), second.await.unwrap().result] {
                    let result = result.unwrap();
                    assert!(!result.ok);
                    assert!(result.error.unwrap().starts_with("PROBE_TIMEOUT"));
                    assert_eq!(result.tested_model.as_deref(), Some("claude-sonnet-4-6"));
                }
                assert!(state.shared.inner.lock().await.entries[&provider_id].in_flight.is_none());
                let next = tokio::spawn({
                    let state = state.clone(); let app = app.handle().clone(); let db = db.clone();
                    async move { state.probe_manual(app, db, provider_id).await }
                });
                loop {
                    if state.shared.inner.lock().await.entries[&provider_id].in_flight.is_some() { break; }
                    tokio::task::yield_now().await;
                }
                release.send(()).unwrap();
                lock_holder.join().unwrap();
                assert!(next.await.unwrap().unwrap().ok);
                assert!(state.shared.inner.lock().await.entries[&provider_id].in_flight.is_none());
                assert_eq!(circuit.snapshot(provider_id, now).state, crate::circuit_breaker::CircuitState::HalfOpen);
                // Only the new flight counted: one further success must still
                // leave the circuit half open, and the following one closes it.
                assert_eq!(circuit.record_success(provider_id, now).after.state, crate::circuit_breaker::CircuitState::HalfOpen);
                assert_eq!(circuit.record_success(provider_id, now).after.state, crate::circuit_breaker::CircuitState::Closed);
                server.abort();
                assert!(server.await.unwrap_err().is_cancelled());
                clock_driver.abort();
                assert!(clock_driver.await.unwrap_err().is_cancelled());
                tokio::time::resume();
            });
            gateway_state::take_app_running_gateway(app.handle());
        }
    }

    #[tokio::test]
    async fn invalidated_claimed_recovery_cannot_start_a_flight() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let target = {
            let mut inner = state.shared.inner.lock().await;
            reconcile_schedules_inner(
                &mut inner,
                vec![loaded(6, 1, 90_000)],
                1_000,
                false,
                1,
                true,
            );
            let generation = inner.entries[&6].generation;
            queue_recovery_from_completion(&mut inner, 6, generation, 10_000);
            let due_at_ms = 10_000 + RECOVERY_PROBE_DELAY_MS;
            let target = reconcile_schedules_inner(
                &mut inner,
                vec![loaded(6, 1, 90_000)],
                due_at_ms,
                false,
                2,
                true,
            )
            .into_iter()
            .next()
            .expect("claimed recovery target");
            assert!(update_recovery_work_after_circuit_evidence(
                &mut inner, 6, generation, 10_000, false, false,
            )
            .is_none());
            target
        };
        let ScheduledProbeSource::Recovery { recovery_epoch } = target.source else {
            panic!("recovery target source");
        };

        assert!(matches!(
            state
                .begin_probe_with_recovery(
                    target.provider_id,
                    Some(target.generation),
                    Some((recovery_epoch, target.due_at_ms)),
                )
                .await,
            ProbeDecision::Stale
        ));
        assert!(state.shared.inner.lock().await.entries[&6]
            .in_flight
            .is_none());
    }

    #[tokio::test]
    async fn coalesced_probe_flight_has_only_one_recording_owner() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        let (generation, budget) = match state.begin_probe(5, None).await {
            ProbeDecision::Lead { generation, budget, .. } => (generation, budget),
            _ => panic!("first probe must lead"),
        };
        assert!(matches!(
            state.begin_probe(5, None).await,
            ProbeDecision::Wait(_)
        ));

        let mut inner = state.shared.inner.lock().await;
        let (in_flight, should_record) = take_finished_flight(&mut inner, 5, generation, &budget)
            .expect("coalesced flight finishes once");
        assert_eq!(in_flight.waiters.len(), 2);
        assert!(should_record);
        assert!(take_finished_flight(&mut inner, 5, generation, &budget).is_none());
    }

    #[tokio::test]
    async fn invalidating_an_idle_provider_does_not_leave_a_runtime_tombstone() {
        let state = ProviderAvailabilityProbeRuntimeState::default();
        drop(state.begin_mutation(99).await);
        {
            let inner = state.shared.inner.lock().await;
            assert!(!inner.entries.contains_key(&99));
        }
        assert!(matches!(
            state.begin_probe(99, Some(1)).await,
            ProbeDecision::Stale
        ));
        assert!(!state.shared.inner.lock().await.entries.contains_key(&99));
    }
}
