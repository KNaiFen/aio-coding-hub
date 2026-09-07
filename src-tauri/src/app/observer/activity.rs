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
            #[cfg(test)]
            NATIVE_COUNTS.with(|counts| { let (begin, end) = counts.get(); counts.set((begin + 1, end)); });
            tracing::debug!("observer activity begin");
            Self(token)
        }
    }

    impl Drop for NativeToken {
        fn drop(&mut self) {
            // SAFETY: this is the unchanged token returned by beginActivity above.
            unsafe { NSProcessInfo::processInfo().endActivity(&self.0) };
            #[cfg(test)]
            NATIVE_COUNTS.with(|counts| { let (begin, end) = counts.get(); counts.set((begin, end + 1)); });
            tracing::debug!("observer activity end");
        }
    }

    thread_local! {
        static TOKENS: RefCell<HashMap<u64, NativeToken>> = RefCell::new(HashMap::new());
    }
    #[cfg(test)]
    thread_local! {
        static NATIVE_COUNTS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
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

    struct Inner<R: tauri::Runtime> {
        app: tauri::AppHandle<R>,
        id: u64,
        state: Mutex<LeaseState>,
        wake: Arc<Notify>,
        task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    }

    pub(super) struct Activity<R: tauri::Runtime = tauri::Wry>(Arc<Inner<R>>);

    impl<R: tauri::Runtime> Clone for Activity<R> {
        fn clone(&self) -> Self { Self(self.0.clone()) }
    }

    impl<R: tauri::Runtime> Activity<R> {
        pub(super) fn new(app: &tauri::AppHandle<R>) -> Self {
            let inner = Arc::new(Inner {
                app: app.clone(),
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                state: Mutex::new(LeaseState::default()),
                wake: Arc::new(Notify::new()),
                task: Mutex::new(None),
            });
            let weak = Arc::downgrade(&inner);
            let wake = inner.wake.clone();
            let task = tokio::spawn(async move {
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
            *inner.task.lock().expect("observer activity task") = Some(task);
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

        pub(super) async fn work(&self) -> Work<R> {
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
            if let Some(task) = self.0.task.lock().expect("observer activity task").take() {
                task.abort();
            }
            self.0.wake.notify_one();
            self.dispatch(None);
        }

        #[cfg(test)]
        pub(super) fn test_state(&self) -> (bool, usize, bool, bool) {
            let state = self.0.state.lock().unwrap();
            let active = TOKENS.with(|tokens| tokens.borrow().contains_key(&self.0.id));
            (active, state.work, state.closed, self.0.task.lock().unwrap().is_some())
        }

        #[cfg(test)]
        pub(super) fn test_counts(&self) -> (usize, usize) {
            NATIVE_COUNTS.with(std::cell::Cell::get)
        }
    }

    pub(super) struct Work<R: tauri::Runtime>(Option<Activity<R>>);

    impl<R: tauri::Runtime> Drop for Work<R> {
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

        #[test]
        fn stale_expiry_and_multiple_work_references_cannot_end_a_new_lease() {
            let now = Instant::now();
            let mut state = LeaseState::default();
            state.renew(now);
            let first_expiry = now + LEASE;
            state.renew(now + Duration::from_secs(14));
            assert!(state.desired(first_expiry));
            assert!(!state.desired(now + Duration::from_secs(29)));
            state.work = 2;
            assert!(state.desired(now + Duration::from_secs(60)));
            state.work -= 1;
            assert!(state.desired(now + Duration::from_secs(64)));
            state.work -= 1;
            assert!(!state.desired(now + Duration::from_secs(65)));
            state.closed = true;
            state.work = 1;
            state.renew(now + Duration::from_secs(66));
            assert!(!state.desired(now + Duration::from_secs(66)));
        }

        #[tokio::test(start_paused = true)]
        async fn controller_expires_shared_lease_releases_work_and_closes_its_task() {
            let app = tauri::test::mock_app();
            let activity = Activity::new(app.handle());
            let id = activity.0.id;
            let initial_counts = NATIVE_COUNTS.with(std::cell::Cell::get);
            let active = || TOKENS.with(|tokens| tokens.borrow().contains_key(&id));
            assert!(!active());
            activity.touch_snapshot().await;
            assert!(active());
            let other_client = activity.clone();
            tokio::time::advance(Duration::from_secs(10)).await;
            other_client.touch_snapshot().await;
            tokio::time::advance(Duration::from_secs(6)).await;
            activity.sync().await;
            assert!(active());
            let first_work = activity.work().await;
            let second_work = activity.work().await;
            tokio::time::advance(Duration::from_secs(50)).await;
            tokio::task::yield_now().await;
            assert!(active());
            drop(first_work);
            activity.sync().await;
            assert!(active());
            drop(second_work);
            activity.sync().await;
            assert!(!active());
            activity.touch_snapshot().await;
            tokio::time::advance(LEASE).await;
            // Only the controller's expiry task may reconcile this expired lease.
            for _ in 0..100 {
                if !active() { break; }
                tokio::task::yield_now().await;
            }
            assert!(!active());
            activity.touch_snapshot().await;
            activity.close();
            activity.sync().await;
            assert!(!active());
            assert!(activity.0.task.lock().unwrap().is_none());
            activity.touch_snapshot().await;
            let late_work = activity.work().await;
            assert!(!active());
            drop(late_work);
            let counts = NATIVE_COUNTS.with(std::cell::Cell::get);
            assert_eq!((counts.0 - initial_counts.0, counts.1 - initial_counts.1), (3, 3));
        }

        #[tokio::test(start_paused = true)]
        async fn expiry_task_ends_a_snapshot_lease_without_another_request() {
            let app = tauri::test::mock_app();
            let activity = Activity::new(app.handle());
            let id = activity.0.id;
            let initial = NATIVE_COUNTS.with(std::cell::Cell::get);
            activity.touch_snapshot().await;
            assert!(TOKENS.with(|tokens| tokens.borrow().contains_key(&id)));
            tokio::time::advance(LEASE).await;
            for _ in 0..100 {
                if !TOKENS.with(|tokens| tokens.borrow().contains_key(&id)) { break; }
                tokio::task::yield_now().await;
            }
            assert!(!TOKENS.with(|tokens| tokens.borrow().contains_key(&id)), "expiry task must end the token");
            let counts = NATIVE_COUNTS.with(std::cell::Cell::get);
            assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (1, 1));
            activity.close();
        }

        #[tokio::test(start_paused = true)]
        async fn cancelled_work_and_closed_controller_release_native_activity() {
            let app = tauri::test::mock_app();
            let activity = Activity::new(app.handle());
            let id = activity.0.id;
            let initial = NATIVE_COUNTS.with(std::cell::Cell::get);
            let active = || TOKENS.with(|tokens| tokens.borrow().contains_key(&id));
            let task_activity = activity.clone();
            let (started, ready) = oneshot::channel();
            let task = tokio::spawn(async move {
                let _work = task_activity.work().await;
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            });
            ready.await.unwrap();
            assert!(active());
            assert_eq!(activity.0.state.lock().unwrap().work, 1);
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            assert_eq!(activity.0.state.lock().unwrap().work, 0);
            assert!(!active());

            assert!(tokio::time::timeout(Duration::from_secs(65), async {
                let _work = activity.work().await;
                assert!(active());
                std::future::pending::<()>().await;
            }).await.is_err());
            assert_eq!(activity.0.state.lock().unwrap().work, 0);
            assert!(!active());

            let failed_activity = activity.clone();
            let failed_task = tokio::spawn(async move {
                let _work = failed_activity.work().await;
                panic!("synthetic observer request failure");
            });
            assert!(failed_task.await.unwrap_err().is_panic());
            assert_eq!(activity.0.state.lock().unwrap().work, 0);
            assert!(!active());

            let work = activity.work().await;
            assert!(active());
            activity.close();
            assert!(!active());
            assert!(activity.0.task.lock().unwrap().is_none());
            drop(work);
            assert_eq!(activity.0.state.lock().unwrap().work, 0);
            activity.touch_snapshot().await;
            assert!(!active());
            let counts = NATIVE_COUNTS.with(std::cell::Cell::get);
            assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (4, 4));
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    pub(super) struct Activity<R: tauri::Runtime>(std::marker::PhantomData<fn() -> R>);
    pub(super) struct Work;
    impl Drop for Work { fn drop(&mut self) {} }

    impl<R: tauri::Runtime> Activity<R> {
        pub(super) fn new(_app: &tauri::AppHandle<R>) -> Self { Self(std::marker::PhantomData) }
        pub(super) async fn touch_snapshot(&self) {}
        pub(super) async fn work(&self) -> Work { Work }
        pub(super) fn close(&self) {}
    }
}

pub(super) struct ObserverActivity<R: tauri::Runtime = tauri::Wry>(platform::Activity<R>);

impl<R: tauri::Runtime> ObserverActivity<R> {
    pub(super) fn new(app: &tauri::AppHandle<R>) -> Self { Self(platform::Activity::new(app)) }
    pub(super) async fn touch_snapshot(&self) { self.0.touch_snapshot().await; }
    pub(super) async fn work(&self) -> impl Drop { self.0.work().await }
    pub(super) fn close(&self) { self.0.close(); }

    #[cfg(all(test, target_os = "macos"))]
    fn test_state(&self) -> (bool, usize, bool, bool) { self.0.test_state() }

    #[cfg(all(test, target_os = "macos"))]
    fn test_counts(&self) -> (usize, usize) { self.0.test_counts() }
}

impl<R: tauri::Runtime> Drop for ObserverActivity<R> {
    fn drop(&mut self) { self.0.close(); }
}

#[cfg(all(test, target_os = "macos"))]
mod handler_tests {
    use super::*;
    use crate::app::observer::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn state(app: &tauri::AppHandle<tauri::test::MockRuntime>) -> ObserverHttpState<tauri::test::MockRuntime> {
        ObserverHttpState {
            activity: Arc::new(ObserverActivity::new(app)),
            app: app.clone(),
            db: Arc::new(Mutex::new(ObserverDbState {
                db: None,
                retry_after: Some(Instant::now() + Duration::from_secs(3600)),
            })),
            token: Arc::from("synthetic-observer-token"),
            limiter: Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_REQUESTS)),
            probe_limiter: Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_PROBES)),
            db_query_limiter: Arc::new(Semaphore::new(1)),
            cache: Arc::new(Mutex::new(HashMap::new())),
            folder_cache: Arc::new(StdMutex::new(FolderLookupCache::default())),
        }
    }

    fn request(method: &str, uri: &str, authorized: bool) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        if authorized {
            builder = builder.header(header::AUTHORIZATION, "Bearer synthetic-observer-token");
        }
        builder.body(Body::empty()).unwrap()
    }

    async fn expire(activity: &ObserverActivity<tauri::test::MockRuntime>) {
        tokio::time::advance(Duration::from_secs(15)).await;
        for _ in 0..100 {
            if !activity.test_state().0 { break; }
            tokio::task::yield_now().await;
        }
        assert!(!activity.test_state().0, "the expiry task must end the native token");
    }

    #[tokio::test(start_paused = true)]
    async fn actual_routes_reject_invalid_requests_without_renewing_activity() {
        let app = tauri::test::mock_app();
        let state = state(app.handle());
        let router = observer_router(state.clone());
        let initial = state.activity.test_counts();
        for (method, uri, auth, expected) in [
            ("GET", "/api/observer/v1/health", true, StatusCode::OK),
            ("GET", "/api/observer/v1/snapshot?cli=codex", false, StatusCode::UNAUTHORIZED),
            ("GET", "/api/observer/v1/snapshot?cli=invalid", true, StatusCode::BAD_REQUEST),
            ("GET", "/api/observer/v1/snapshot?cli=codex&history_limit=51", true, StatusCode::BAD_REQUEST),
            ("GET", "/api/observer/v1/snapshot?cli=codex&include_providers=invalid", true, StatusCode::BAD_REQUEST),
            ("GET", "/api/observer/v1/snapshot?cli=codex&unknown=true", true, StatusCode::BAD_REQUEST),
            ("POST", "/api/observer/v1/providers/1/test-availability", false, StatusCode::UNAUTHORIZED),
            ("POST", "/api/observer/v1/providers/0/test-availability", true, StatusCode::BAD_REQUEST),
            ("POST", "/api/observer/v1/providers/invalid/test-availability", true, StatusCode::BAD_REQUEST),
            ("GET", "/api/observer/v1/unknown", true, StatusCode::NOT_FOUND),
        ] {
            let response = router.clone().oneshot(request(method, uri, auth)).await.unwrap();
            assert_eq!(response.status(), expected, "{uri}");
            assert_eq!(state.activity.test_state().0, false, "{uri}");
            assert_eq!(state.activity.test_state().1, 0, "{uri}");
        }
        assert_eq!(state.activity.test_counts(), initial);

        state.activity.touch_snapshot().await;
        tokio::time::advance(Duration::from_secs(10)).await;
        let response = router.oneshot(request("GET", "/api/observer/v1/snapshot?cli=invalid", true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        tokio::time::advance(Duration::from_secs(5)).await;
        for _ in 0..100 { tokio::task::yield_now().await; }
        assert!(!state.activity.test_state().0, "invalid request must not extend the earlier lease");
        state.activity.close();
    }

    #[tokio::test(start_paused = true)]
    async fn snapshots_renew_on_every_scope_view_cache_and_busy_path() {
        let app = tauri::test::mock_app();
        app.manage(crate::app::gateway_state::GatewayState::default());
        app.manage(crate::app::provider_account_usage_runtime::ProviderAccountUsageRuntimeState::default());
        let state = state(app.handle());
        let router = observer_router(state.clone());
        let initial = state.activity.test_counts();
        for scope in CliScope::VALUES {
            for (history_limit, include_providers) in [(0, false), (50, false), (50, true)] {
                let uri = format!("/api/observer/v1/snapshot?cli={}&history_limit={history_limit}&include_providers={include_providers}", scope.as_str());
                let response = router.clone().oneshot(request("GET", &uri, true)).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                assert!(state.activity.test_state().0);
                let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
                let snapshot: ObserverSnapshotV1 = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(snapshot.scope, scope);
                assert_eq!(snapshot.providers.is_some(), include_providers);

                let key = CacheKey { scope, history_limit, include_providers };
                tokio::time::advance(Duration::from_secs(10)).await;
                {
                    let mut cache = state.cache.lock().await;
                    let cached = cache.get_mut(&key).unwrap();
                    cached.snapshot.generated_at_ms = 123;
                    cached.created_at = Instant::now();
                }
                let cached = router.clone().oneshot(request("GET", &uri, true)).await.unwrap();
                let bytes = axum::body::to_bytes(cached.into_body(), usize::MAX).await.unwrap();
                assert_eq!(serde_json::from_slice::<ObserverSnapshotV1>(&bytes).unwrap().generated_at_ms, 123);
                tokio::time::advance(Duration::from_secs(6)).await;
                for _ in 0..100 { tokio::task::yield_now().await; }
                assert!(state.activity.test_state().0, "cache hit must extend the activity lease");
                expire(&state.activity).await;
            }
        }

        let permits = state.limiter.clone().acquire_many_owned(OBSERVER_MAX_CONCURRENT_REQUESTS as u32).await.unwrap();
        let response = router.clone().oneshot(request("GET", "/api/observer/v1/snapshot?cli=codex", true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(state.activity.test_state().0, "valid busy snapshot still renews");
        drop(permits);
        expire(&state.activity).await;

        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("observer-permit.db")).unwrap();
        state.db.lock().await.db = Some(db);
        state.cache.lock().await.clear();
        let db_permit = state.db_query_limiter.clone().acquire_owned().await.unwrap();
        let response = router.oneshot(request("GET", "/api/observer/v1/snapshot?cli=all&include_providers=true", true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(state.activity.test_state().0, "DB permit expiry must retain the valid request lease");
        drop(db_permit);
        expire(&state.activity).await;
        let counts = state.activity.test_counts();
        assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (17, 17));
        state.activity.close();
    }

    #[tokio::test(start_paused = true)]
    async fn actual_probe_handler_releases_work_on_error_timeout_and_cancellation() {
        let app = tauri::test::mock_app();
        let state = state(app.handle());
        let router = observer_router(state.clone());
        let initial = state.activity.test_counts();
        let uri = "/api/observer/v1/providers/1/test-availability";
        let response = router.clone().oneshot(request("POST", uri, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(state.activity.test_state().0, false);
        assert_eq!(state.activity.test_state().1, 0);

        app.manage(crate::app_state::DbInitState::default());
        let db_state = app.state::<crate::app_state::DbInitState>();
        let db_guard = db_state.0.lock().await;
        let pending = tokio::spawn(router.clone().oneshot(request("POST", uri, true)));
        for _ in 0..100 {
            if state.activity.test_state().1 == 1 { break; }
            tokio::task::yield_now().await;
        }
        assert_eq!(state.activity.test_state().1, 1);
        assert!(state.activity.test_state().0);
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        assert_eq!(state.activity.test_state().1, 0);
        assert!(!state.activity.test_state().0);

        let pending = tokio::spawn(router.clone().oneshot(request("POST", uri, true)));
        for _ in 0..100 {
            if state.activity.test_state().1 == 1 { break; }
            tokio::task::yield_now().await;
        }
        assert_eq!(state.activity.test_state().1, 1);
        tokio::time::advance(OBSERVER_PROBE_TIMEOUT).await;
        assert_eq!(pending.await.unwrap().unwrap().status(), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(state.activity.test_state().1, 0);
        assert!(!state.activity.test_state().0);
        drop(db_guard);

        let permits = state.probe_limiter.clone().acquire_many_owned(OBSERVER_MAX_CONCURRENT_PROBES as u32).await.unwrap();
        let response = router.oneshot(request("POST", uri, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(!state.activity.test_state().0);
        drop(permits);
        let counts = state.activity.test_counts();
        assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (3, 3));
        state.activity.close();
    }

    #[tokio::test]
    async fn actual_probe_handler_releases_work_after_shared_success_and_failure() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::init_for_tests(&temp.path().join("observer-probe.db")).unwrap();
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let conn = db.open_connection().unwrap();
        conn.execute("INSERT INTO providers(provider_uuid, cli_key, name, base_url, api_key_plaintext, created_at, updated_at) VALUES (?1, 'claude', 'observer synthetic', ?2, 'synthetic-key', 1, 1)", rusqlite::params![crate::shared::uuid::new_uuid_v4(), base_url]).unwrap();
        let provider_id = conn.last_insert_rowid();
        drop(conn);
        let upstream = axum::Router::new().route("/v1/messages", axum::routing::post(|| async {
            axum::Json(serde_json::json!({"type":"message","role":"assistant","content":[{"type":"text","text":"OK"}],"stop_reason":"end_turn"}))
        }));
        let server = tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap(); });
        let app = tauri::test::mock_app();
        app.manage(crate::app_state::DbInitState(Mutex::new(Some(db.clone()))));
        app.manage(crate::app::provider_availability_probe_runtime::ProviderAvailabilityProbeRuntimeState::default());
        let state = state(app.handle());
        let router = observer_router(state.clone());
        let initial = state.activity.test_counts();
        let uri = format!("/api/observer/v1/providers/{provider_id}/test-availability");
        let response = router.clone().oneshot(request("POST", &uri, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(serde_json::from_slice::<ObserverProviderAvailabilityTestResult>(&bytes).unwrap().ok);
        assert_eq!(state.activity.test_state().1, 0);
        assert!(!state.activity.test_state().0);
        db.open_connection().unwrap().execute("UPDATE providers SET api_key_plaintext = '' WHERE id = ?1", [provider_id]).unwrap();
        let response = router.oneshot(request("POST", &uri, true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert!(!serde_json::from_slice::<ObserverProviderAvailabilityTestResult>(&bytes).unwrap().ok);
        assert_eq!(state.activity.test_state().1, 0);
        assert!(!state.activity.test_state().0);
        let counts = state.activity.test_counts();
        assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (2, 2));
        state.activity.close();
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn observer_stop_abort_and_cancelled_start_close_the_actual_controller() {
        let _env_lock = crate::test_support::test_env_lock();
        let temp = tempfile::tempdir().unwrap();
        let _home = crate::test_support::ScopedTestEnvVar::set("AIO_CODING_HUB_TEST_HOME", temp.path());
        let app = tauri::test::mock_app();
        app.manage(ObserverRuntimeStateFor::<tauri::test::MockRuntime>::default());
        let runtime_state = app.state::<ObserverRuntimeStateFor<tauri::test::MockRuntime>>();
        for abort in [false, true] {
            runtime_state.stopping.store(false, Ordering::Release);
            start(app.handle().clone()).await.unwrap();
            let activity = {
                let runtime = runtime_state.runtime.lock().await;
                runtime.as_ref().unwrap().activity.clone()
            };
            let initial = activity.test_counts();
            assert!(!activity.test_state().0, "startup alone must not begin activity");
            let state = ObserverHttpState { activity: activity.clone(), ..state(app.handle()) };
            let router = observer_router(state.clone());
            let permits = state.limiter.clone().acquire_many_owned(OBSERVER_MAX_CONCURRENT_REQUESTS as u32).await.unwrap();
            assert_eq!(router.clone().oneshot(request("GET", "/api/observer/v1/snapshot?cli=codex", true)).await.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);
            assert!(activity.test_state().0);
            if abort {
                runtime_state.runtime.lock().await.as_ref().unwrap().task.abort();
                for _ in 0..100 {
                    if activity.test_state().2 { break; }
                    tokio::task::yield_now().await;
                }
                assert!(activity.test_state().2, "server task cancellation must close its controller");
            }
            stop_best_effort(app.handle()).await;
            assert!(runtime_state.runtime.lock().await.is_none());
            assert_eq!(activity.test_state(), (false, 0, true, false));
            assert_eq!(router.oneshot(request("GET", "/api/observer/v1/snapshot?cli=codex", true)).await.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);
            assert_eq!(activity.test_state(), (false, 0, true, false), "late route must not reopen activity");
            drop(permits);
            let counts = activity.test_counts();
            assert_eq!((counts.0 - initial.0, counts.1 - initial.1), (1, 1));
        }
        start(app.handle().clone()).await.unwrap();
        assert!(runtime_state.runtime.lock().await.is_none(), "stopping observer rejects a late start");
        assert!(!descriptor::path(app.handle()).unwrap().exists());
    }
}
