//! Observer-owned user activity; native tokens never leave their owning thread.

#[cfg(target_os = "macos")]
mod platform {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, NSString};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::{oneshot, Notify};
    use tokio::time::Instant;

    const LEASE: Duration = Duration::from_secs(15);
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct NativeToken(Retained<ProtocolObject<dyn NSObjectProtocol>>);

    impl NativeToken {
        fn begin() -> Self {
            let token = NSProcessInfo::processInfo().beginActivityWithOptions_reason(
                NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
                &NSString::from_str("AIO TUI observation"),
            );
            tracing::debug!("observer activity begin");
            Self(token)
        }
    }

    impl Drop for NativeToken {
        fn drop(&mut self) {
            // SAFETY: this is the unchanged token returned by beginActivity above.
            unsafe { NSProcessInfo::processInfo().endActivity(&self.0) };
            tracing::debug!("observer activity end");
        }
    }

    thread_local! {
        static TOKENS: RefCell<HashMap<u64, NativeToken>> = RefCell::new(HashMap::new());
    }

    #[derive(Default)]
    struct LeaseState {
        closed: bool,
        lease_until: Option<Instant>,
        work: usize,
    }

    impl LeaseState {
        fn desired(&self, now: Instant) -> bool {
            !self.closed && (self.work > 0 || self.lease_until.is_some_and(|until| until > now))
        }

        fn renew(&mut self, now: Instant) {
            if !self.closed {
                self.lease_until = Some(now + LEASE);
            }
        }
    }

    struct Inner {
        app: tauri::AppHandle,
        id: u64,
        state: Mutex<LeaseState>,
        wake: Arc<Notify>,
    }

    #[derive(Clone)]
    pub(super) struct Activity(Arc<Inner>);

    impl Activity {
        pub(super) fn new(app: &tauri::AppHandle) -> Self {
            let inner = Arc::new(Inner {
                app: app.clone(),
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                state: Mutex::new(LeaseState::default()),
                wake: Arc::new(Notify::new()),
            });
            let weak = Arc::downgrade(&inner);
            let wake = inner.wake.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let Some(inner) = weak.upgrade() else { break };
                    let deadline = {
                        let mut state = inner.state.lock().expect("observer activity state");
                        if state.closed { break; }
                        if state.lease_until.is_some_and(|until| until <= Instant::now()) {
                            state.lease_until = None;
                        }
                        state.lease_until
                    };
                    Self(inner).sync().await;
                    match deadline {
                        Some(deadline) => tokio::select! {
                            _ = tokio::time::sleep_until(deadline) => {},
                            _ = wake.notified() => {},
                        },
                        None => wake.notified().await,
                    }
                }
            });
            Self(inner)
        }

        fn dispatch(&self, done: Option<oneshot::Sender<()>>) {
            let inner = self.0.clone();
            if self.0.app.run_on_main_thread(move || {
                let state = inner.state.lock().expect("observer activity state");
                TOKENS.with(|tokens| {
                    let mut tokens = tokens.borrow_mut();
                    if state.desired(Instant::now()) {
                        tokens.entry(inner.id).or_insert_with(NativeToken::begin);
                    } else {
                        tokens.remove(&inner.id);
                    }
                });
                if let Some(done) = done { let _ = done.send(()); }
            }).is_err() {
                tracing::warn!(error = "OBS_ACTIVITY_DISPATCH", "observer activity dispatch failed");
            }
        }

        async fn sync(&self) {
            let (tx, rx) = oneshot::channel();
            self.dispatch(Some(tx));
            if rx.await.is_err() {
                tracing::warn!(error = "OBS_ACTIVITY_UNAVAILABLE", "observer activity unavailable");
            }
        }

        pub(super) async fn touch_snapshot(&self) {
            self.0.state.lock().expect("observer activity state").renew(Instant::now());
            self.0.wake.notify_one();
            self.sync().await;
        }

        pub(super) async fn work(&self) -> Work {
            let active = {
                let mut state = self.0.state.lock().expect("observer activity state");
                if state.closed { false } else { state.work += 1; true }
            };
            let work = Work(active.then(|| self.clone()));
            self.sync().await;
            work
        }

        pub(super) fn close(&self) {
            self.0.state.lock().expect("observer activity state").closed = true;
            self.0.wake.notify_one();
            self.dispatch(None);
        }
    }

    pub(super) struct Work(Option<Activity>);

    impl Drop for Work {
        fn drop(&mut self) {
            if let Some(activity) = self.0.take() {
                activity.0.state.lock().expect("observer activity state").work -= 1;
                activity.dispatch(None);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn leases_work_and_close_balance_one_token() {
            let now = Instant::now();
            let mut state = LeaseState::default();
            let mut active = false;
            let mut begins = 0;
            let mut ends = 0;
            let mut reconcile = |state: &LeaseState, at| {
                let desired = state.desired(at);
                begins += usize::from(desired && !active);
                ends += usize::from(!desired && active);
                active = desired;
            };
            reconcile(&state, now);
            state.renew(now);
            reconcile(&state, now);
            state.renew(now + Duration::from_secs(10));
            reconcile(&state, now + LEASE);
            state.work += 1;
            reconcile(&state, now + Duration::from_secs(60));
            state.work -= 1;
            reconcile(&state, now + Duration::from_secs(60));
            state.renew(now + Duration::from_secs(61));
            reconcile(&state, now + Duration::from_secs(61));
            state.closed = true;
            reconcile(&state, now + Duration::from_secs(62));
            state.renew(now + Duration::from_secs(63));
            reconcile(&state, now + Duration::from_secs(63));
            assert_eq!((begins, ends), (2, 2));
            assert!(!active);
        }

        #[test]
        fn native_activity_can_begin_and_end_without_preventing_sleep() {
            let options = NSActivityOptions::UserInitiatedAllowingIdleSystemSleep;
            assert!(!options.contains(NSActivityOptions::IdleSystemSleepDisabled));
            assert!(!options.contains(NSActivityOptions::IdleDisplaySleepDisabled));
            let token = NativeToken::begin();
            drop(token);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    #[derive(Clone)]
    pub(super) struct Activity;
    pub(super) struct Work;
    impl Drop for Work { fn drop(&mut self) {} }

    impl Activity {
        pub(super) fn new(_app: &tauri::AppHandle) -> Self { Self }
        pub(super) async fn touch_snapshot(&self) {}
        pub(super) async fn work(&self) -> Work { Work }
        pub(super) fn close(&self) {}
    }
}

pub(super) struct ObserverActivity(platform::Activity);

impl ObserverActivity {
    pub(super) fn new(app: &tauri::AppHandle) -> Self { Self(platform::Activity::new(app)) }
    pub(super) async fn touch_snapshot(&self) { self.0.touch_snapshot().await; }
    pub(super) async fn work(&self) -> impl Drop { self.0.work().await }
    pub(super) fn close(&self) { self.0.close(); }
}

impl Drop for ObserverActivity {
    fn drop(&mut self) { self.0.close(); }
}
