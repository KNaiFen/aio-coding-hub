// Socket and real Extension Host coverage of the public response commit gate.
mod response_commit_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::{mpsc, oneshot, Notify};

    const MODEL: &str = "gpt-commit";
    const PREFIX: &str = "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"A-PRIVATE-ID\",\"model\":\"gpt-commit\"}}\n\nevent: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"A-PRIVATE-TEXT\"}\n\nevent: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\",\"name\":\"must_not_execute\",\"arguments\":\"{}\"}}\n\n";
    const BAD_TAIL: &str = "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"model\":\"wrong-model\"},\"deny\":true}\n\n";
    const GOOD: &str = "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"B-ID\",\"status\":\"completed\",\"model\":\"gpt-commit\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"B-ONLY\"}]}],\"usage\":{\"input_tokens\":3,\"output_tokens\":2}}}\n\n";

    fn plugin(root: &Path, model_rule: bool) -> PluginDetail {
        let mut detail = request_rewrite_plugin();
        detail.summary.plugin_id = if model_rule {
            "community.model-check"
        } else {
            "community.tail-check"
        }
        .into();
        detail.manifest.id = detail.summary.plugin_id.clone();
        detail.install_source = PluginInstallSource::Local;
        detail.installed_dir = Some(root.to_string_lossy().into_owned());
        detail.manifest.capabilities =
            vec!["gateway.hooks".into(), "gateway.provider.switch".into()];
        detail.granted_permissions =
            vec!["response.body.read".into(), "response.headers.read".into()];
        let hook = gateway_hook_mut(&mut detail);
        hook.name = "gateway.response.beforeCommit".into();
        hook.failure_policy = Some("fail-closed".into());
        hook.timeout_ms = Some(5_000);
        hook.request_match = Some(crate::domain::plugins::PluginHookMatch {
            cli_keys: vec!["codex".into()],
            methods: vec!["POST".into()],
            paths: vec!["/v1/responses".into(), "/responses".into()],
        });
        let source = if model_rule {
            include_str!("../../../examples/plugins/codex-model-consistency/extension.cjs")
        } else {
            r#"module.exports.activate = function(api) {
                api.gateway.registerHook("gateway.response.beforeCommit", function(invocation) {
                    const context = invocation.context;
                    if (context.request.cliKey !== "codex" || !context.response.complete) throw new Error("wire contract");
                    if (context.response.body.indexOf('"deny":true') !== -1) return {action:"switchProvider",reasonCode:"tail_denied",message:"A-PRIVATE-TEXT"};
                    return {action:"pass"};
                });
            };"#
        };
        write_extension_host_test_entry(None, root, &detail.manifest, source);
        detail
    }

    fn setup_settings(app: &tauri::AppHandle<tauri::test::MockRuntime>) {
        let mut value = settings::AppSettings::default();
        value.failover_max_attempts_per_provider = 3;
        value.failover_max_providers_to_try = 3;
        settings::write(app, &value).unwrap();
        crate::cli_proxy::set_enabled(app, "codex", true, "http://127.0.0.1:37123").unwrap();
    }

    async fn serve_gateway(
        state: GatewayAppState<tauri::test::MockRuntime>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let router = build_router(state);
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (url, task)
    }

    struct Upstream {
        url: String,
        count: Arc<AtomicUsize>,
        captured: mpsc::UnboundedReceiver<String>,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for Upstream {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn upstream(
        body: &'static str,
        prefix: Option<&'static str>,
        release: Option<Arc<Notify>>,
    ) -> Upstream {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let hits = count.clone();
        let (tx, captured) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let hits = hits.clone();
                let tx = tx.clone();
                let release = release.clone();
                tokio::spawn(async move {
                    let request = read_complete_http_request(&mut socket).await;
                    hits.fetch_add(1, Ordering::SeqCst);
                    let _ = tx.send(request);
                    let marker = if prefix.is_some() {
                        "A-SECRET-HEADER"
                    } else {
                        "B-HEADER"
                    };
                    let headers = format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nx-upstream-private: {marker}\r\nconnection: close\r\n\r\n");
                    if socket.write_all(headers.as_bytes()).await.is_err() {
                        return;
                    }
                    if let Some(prefix) = prefix {
                        if socket.write_all(prefix.as_bytes()).await.is_err() {
                            return;
                        }
                    }
                    if let Some(release) = release {
                        release.notified().await;
                    }
                    let _ = socket.write_all(body.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        Upstream {
            url,
            count,
            captured,
            task,
        }
    }

    fn request_body(extra: &str) -> String {
        format!(
            r#"{{"model":"{MODEL}","input":[{{"role":"user","content":"clean baseline"}}],"stream":true{extra}}}"#
        )
    }

    async fn assert_late_rejection_is_atomic(model_rule: bool) {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("atomic.db")).unwrap();
        let release = Arc::new(Notify::new());
        let mut a = upstream(BAD_TAIL, Some(PREFIX), Some(release.clone())).await;
        let mut b = upstream(GOOD, None, None).await;
        let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
        let bid = insert_codex_provider_with_priority(&db, "B", b.url.clone(), 1);
        for (id, key) in [(aid, "synthetic-a-key"), (bid, "synthetic-b-key")] {
            db.open_connection()
                .unwrap()
                .execute(
                    "UPDATE providers SET api_key_plaintext = ?1 WHERE id = ?2",
                    rusqlite::params![key, id],
                )
                .unwrap();
        }
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), model_rule));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (log_tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, log_tx, pipeline);
        let circuit = state.circuit.clone();
        let sessions = state.session.clone();
        let active = state.active_requests.clone();
        let (gateway, server) = serve_gateway(state).await;
        let client = reqwest::Client::new();
        let mut response = Box::pin(
            client
                .post(format!("{gateway}/codex/v1/responses"))
                .header("x-session-id", "atomic-session-0000000000")
                .header("authorization", "Bearer synthetic-client-key")
                .header("content-type", "application/json")
                .body(request_body(""))
                .send(),
        );
        // Pump the actual HTTP connection until A has received the request, then
        // check that even headers are blocked while A's suffix is held back.
        tokio::select! {
            captured = a.captured.recv() => {
                let captured = captured.unwrap();
                assert!(captured.contains("clean baseline"));
                assert!(captured.to_ascii_lowercase().contains("authorization: bearer synthetic-a-key"));
                assert!(!captured.contains("synthetic-client-key"));
                assert!(!captured.contains("synthetic-b-key"));
            },
            result = &mut response => panic!("response committed before tail: {result:?}")
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut response)
                .await
                .is_err()
        );
        assert_eq!(b.count.load(Ordering::SeqCst), 0);
        release.notify_one();
        let response = tokio::time::timeout(Duration::from_secs(15), response)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("x-upstream-private").unwrap(),
            "B-HEADER"
        );
        let body = response.text().await.unwrap();
        assert!(body.contains("B-ONLY"));
        assert!(!body.contains("A-PRIVATE"));
        assert!(!body.contains("must_not_execute"));
        let captured_b = b.captured.recv().await.unwrap();
        assert!(captured_b.contains("clean baseline"));
        assert!(captured_b
            .to_ascii_lowercase()
            .contains("authorization: bearer synthetic-b-key"));
        assert!(!captured_b.contains("synthetic-client-key"));
        assert!(!captured_b.contains("synthetic-a-key"));
        assert!(!captured_b.contains("A-PRIVATE"));
        assert_eq!(a.count.load(Ordering::SeqCst), 1);
        assert_eq!(b.count.load(Ordering::SeqCst), 1);
        let log = recv_terminal_request_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts.as_array().unwrap().len(), 2);
        assert_eq!(attempts[0]["provider_id"], aid);
        assert_eq!(attempts[0]["status"], 200);
        assert_eq!(attempts[0]["decision"], "switch");
        assert!(attempts[0]["plugin_decision"]["message"].is_null());
        assert_eq!(attempts[1]["provider_id"], bid);
        assert_eq!(attempts[1]["outcome"], "success");
        assert_eq!(log.input_tokens, Some(3));
        assert_eq!(log.output_tokens, Some(2));
        let now = crate::gateway::util::now_unix_seconds() as i64;
        assert_eq!(circuit.snapshot(aid, now).failure_count, 0);
        assert_eq!(
            sessions.get_bound_provider("codex", "atomic-session-0000000000", now),
            Some(bid)
        );
        assert!(active.snapshot().is_empty());
        server.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_real_model_worker_rejects_late_model_without_leaking_headers_or_output(
    ) {
        assert_late_rejection_is_atomic(true).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_ordinary_non_model_worker_uses_the_same_atomic_failover() {
        assert_late_rejection_is_atomic(false).await;
    }

    async fn assert_sessions_keep_their_own_candidate_order(reject_b: bool, reject_q: bool) {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("sessions.db")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let purl = format!("http://{}", listener.local_addr().unwrap());
        let barrier = Arc::new(tokio::sync::Barrier::new(2));
        let pcount = Arc::new(AtomicUsize::new(0));
        let hits = pcount.clone();
        let ptask = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let barrier = barrier.clone();
                let hits = hits.clone();
                tokio::spawn(async move {
                    let request = read_complete_http_request(&mut socket).await;
                    let ordinal = hits.fetch_add(1, Ordering::SeqCst);
                    if ordinal < 2 {
                        barrier.wait().await;
                    }
                    let bad = request
                        .to_ascii_lowercase()
                        .contains("x-session-id: session-a-00000000000000")
                        || (reject_b && ordinal < 2);
                    let body = if bad { BAD_TAIL } else { GOOD };
                    let response = format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                    socket.write_all(response.as_bytes()).await.unwrap();
                });
            }
        });
        let q = upstream(if reject_q { BAD_TAIL } else { GOOD }, None, None).await;
        let r = upstream(GOOD, None, None).await;
        let pid = insert_codex_provider_with_priority(&db, "P", purl, 0);
        let qid = insert_codex_provider_with_priority(&db, "Q", q.url.clone(), 1);
        let rid = insert_codex_provider_with_priority(&db, "R", r.url.clone(), 2);
        let amode = crate::sort_modes::create_mode(&db, "A route").unwrap().id;
        let bmode = crate::sort_modes::create_mode(&db, "B route").unwrap().id;
        crate::sort_modes::set_mode_providers_order(&db, amode, "codex", vec![pid, qid]).unwrap();
        crate::sort_modes::set_mode_providers_order(&db, bmode, "codex", vec![pid, rid]).unwrap();
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(32);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let sessions = state.session.clone();
        let circuit = state.circuit.clone();
        let recent_errors = state.recent_errors.clone();
        let now = crate::gateway::util::now_unix_seconds() as i64;
        sessions.bind_sort_mode(
            "codex",
            "session-a-00000000000000",
            Some(amode),
            Some(vec![pid, qid]),
            now,
            std::time::Instant::now(),
        );
        sessions.bind_sort_mode(
            "codex",
            "session-b-00000000000000",
            Some(bmode),
            Some(vec![pid, rid]),
            now,
            std::time::Instant::now(),
        );
        let (gateway, server) = serve_gateway(state).await;
        let client = reqwest::Client::new();
        let call = |session: &'static str| {
            client
                .post(format!("{gateway}/codex/v1/responses"))
                .header("x-session-id", session)
                .header("content-type", "application/json")
                .body(request_body(""))
                .send()
        };
        let (a, b) = tokio::join!(
            call("session-a-00000000000000"),
            call("session-b-00000000000000")
        );
        assert_eq!(
            a.as_ref().unwrap().status(),
            if reject_q {
                StatusCode::BAD_GATEWAY
            } else {
                StatusCode::OK
            }
        );
        assert_eq!(b.as_ref().unwrap().status(), StatusCode::OK);
        a.unwrap().bytes().await.unwrap();
        b.unwrap().bytes().await.unwrap();
        let first = recv_terminal_request_log(&mut logs).await;
        let second = recv_terminal_request_log(&mut logs).await;
        let (a, b) = if first.session_id.as_deref() == Some("session-a-00000000000000") {
            (first, second)
        } else {
            (second, first)
        };
        if reject_q {
            assert_eq!(a.error_code.as_deref(), Some("GW_RESPONSE_REJECTED"));
        }
        let aa: Value = serde_json::from_str(&a.attempts_json).unwrap();
        let ba: Value = serde_json::from_str(&b.attempts_json).unwrap();
        assert_eq!(aa[0]["provider_id"], pid);
        assert_eq!(aa[1]["provider_id"], qid);
        if reject_b {
            assert_eq!(ba[0]["provider_id"], pid);
            assert_eq!(ba[1]["provider_id"], rid);
        } else {
            assert_eq!(ba.as_array().unwrap().len(), 1);
            assert_eq!(ba[0]["provider_id"], pid);
        }
        assert_eq!(q.count.load(Ordering::SeqCst), 1);
        assert_eq!(r.count.load(Ordering::SeqCst), usize::from(reject_b));
        assert_eq!(
            sessions.get_bound_provider("codex", "session-a-00000000000000", now),
            if reject_q { None } else { Some(qid) }
        );
        assert_eq!(
            sessions.get_bound_provider("codex", "session-b-00000000000000", now),
            Some(if reject_b { rid } else { pid })
        );
        assert_eq!(
            sessions.get_bound_provider_order("codex", "session-a-00000000000000", now),
            Some(vec![pid, qid])
        );
        assert_eq!(
            sessions.get_bound_provider_order("codex", "session-b-00000000000000", now),
            Some(vec![pid, rid])
        );
        assert_eq!(circuit.snapshot(pid, now).failure_count, 0);
        // A subsequent independent session must still reach P; no shared
        // unavailable cache or persistent policy blacklist may be installed.
        drop(recent_errors);
        let c = call("session-c-00000000000000").await.unwrap();
        assert_eq!(c.status(), StatusCode::OK);
        c.bytes().await.unwrap();
        assert_eq!(pcount.load(Ordering::SeqCst), 3);
        let c = recv_terminal_request_log(&mut logs).await;
        assert_eq!(c.session_id.as_deref(), Some("session-c-00000000000000"));
        ptask.abort();
        server.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_one_session_rejecting_does_not_poison_another_session() {
        assert_sessions_keep_their_own_candidate_order(false, false).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_two_rejecting_sessions_switch_to_their_own_next_provider() {
        assert_sessions_keep_their_own_candidate_order(true, false).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_all_rejected_session_does_not_cache_provider_unavailability() {
        assert_sessions_keep_their_own_candidate_order(false, true).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_socket_cancel_closes_upstream_and_keeps_previous_rejection() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("cancel.db")).unwrap();
        let a = upstream(BAD_TAIL, Some(PREFIX), None).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let burl = format!("http://{}", listener.local_addr().unwrap());
        let (started_tx, started_rx) = oneshot::channel();
        let (closed_tx, closed_rx) = oneshot::channel();
        let btask = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_complete_http_request(&mut socket).await;
            socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n").await.unwrap();
            socket.write_all(PREFIX.as_bytes()).await.unwrap();
            let _ = started_tx.send(());
            let mut byte = [0u8];
            let result = socket.read(&mut byte).await;
            let _ = closed_tx.send(matches!(result, Ok(0) | Err(_)));
        });
        let c = upstream(GOOD, None, None).await;
        let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
        let bid = insert_codex_provider_with_priority(&db, "B", burl, 1);
        insert_codex_provider_with_priority(&db, "C", c.url.clone(), 2);
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(32);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let (gateway, server) = serve_gateway(state).await;
        let address = gateway.trim_start_matches("http://");
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        let body = request_body("");
        let wire = format!("POST /codex/v1/responses HTTP/1.1\r\nhost: {address}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nx-session-id: cancelled-session-00000\r\n\r\n{body}", body.len());
        socket.write_all(wire.as_bytes()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(15), started_rx)
            .await
            .unwrap()
            .unwrap();
        drop(socket);
        assert!(tokio::time::timeout(Duration::from_secs(3), closed_rx)
            .await
            .expect("upstream closes after real client disconnect")
            .unwrap());
        let log = recv_terminal_request_log(&mut logs).await;
        assert_eq!(log.error_code.as_deref(), Some("GW_REQUEST_ABORTED"));
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts[0]["provider_id"], aid);
        assert_eq!(attempts[0]["decision"], "switch");
        assert_eq!(attempts[1]["provider_id"], bid);
        assert_eq!(attempts[1]["outcome"], "client_abort");
        assert_eq!(c.count.load(Ordering::SeqCst), 0);
        assert!(active.snapshot().is_empty());
        btask.abort();
        server.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_private_continuation_never_replays_incremental_input() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("continuation.db")).unwrap();
        let a = upstream(BAD_TAIL, Some(PREFIX), None).await;
        let b = upstream(GOOD, None, None).await;
        insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
        insert_codex_provider_with_priority(&db, "B", b.url.clone(), 1);
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let router = build_router(state);
        let request = Request::builder()
            .method("POST")
            .uri("/codex/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(request_body(
                ",\"previous_response_id\":\"private-history-with-sentinel\"",
            )))
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error_code"], "GW_REQUEST_REPLAY_UNSAFE");
        assert_eq!(a.count.load(Ordering::SeqCst), 1);
        assert_eq!(b.count.load(Ordering::SeqCst), 0);
        let log = recv_terminal_request_log(&mut logs).await;
        assert_eq!(log.error_code.as_deref(), Some("GW_REQUEST_REPLAY_UNSAFE"));
        assert!(active.snapshot().is_empty());
        executor.dispose_runtime_caches_for_tests();
    }
    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_compares_each_final_mapped_and_before_send_model_on_the_wire() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("mapped.db")).unwrap();
        let mut a = upstream(BAD_TAIL, Some(PREFIX), None).await;
        let (burl, captured_b, btask) = spawn_capturing_json_upstream(r#"{"id":"accepted","model":"mapped-b-hook","status":"completed","output":[],"usage":{"input_tokens":3,"output_tokens":2}}"#).await;
        let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
        let bid = insert_codex_provider_with_priority(&db, "B", burl, 1);
        for (id, target) in [(aid, "mapped-a"), (bid, "mapped-b")] {
            let policy = serde_json::json!({"version":1,"mode":"all","modelPatterns":[],"mappings":[{"source":MODEL,"target":target}]}).to_string();
            db.open_connection()
                .unwrap()
                .execute(
                    "UPDATE providers SET model_policy_json = ?1 WHERE id = ?2",
                    rusqlite::params![policy, id],
                )
                .unwrap();
        }
        let root = home.path().join("plugin");
        let mut detail = plugin(&root, true);
        detail
            .manifest
            .contributes
            .as_mut()
            .unwrap()
            .gateway_hooks
            .push(PluginHook {
                name: "gateway.request.beforeSend".into(),
                priority: 10,
                failure_policy: Some("fail-closed".into()),
                timeout_ms: None,
                request_match: None,
            });
        detail.granted_permissions.extend([
            "request.body.read".to_string(),
            "request.body.write".to_string(),
        ]);
        let source = format!(
            r#"{}
            const originalActivate = module.exports.activate;
            module.exports.activate = function(api) {{
                originalActivate(api);
                api.gateway.registerHook("gateway.request.beforeSend", function(invocation) {{
                    const request = JSON.parse(invocation.context.request.body);
                    request.model += "-hook";
                    return {{action:"replace",requestBody:JSON.stringify(request)}};
                }});
            }};"#,
            include_str!("../../../examples/plugins/codex-model-consistency/extension.cjs")
        );
        write_extension_host_test_entry(None, &root, &detail.manifest, &source);
        let detail = persist_and_reload_plugin_detail(&db, &detail);
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(32);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let router = build_router(state);
        let request = Request::builder()
            .method("POST")
            .uri("/codex/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(request_body("")))
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let sent_a = a.captured.recv().await.unwrap();
        let sent_b = tokio::time::timeout(Duration::from_secs(2), captured_b)
            .await
            .unwrap()
            .unwrap();
        assert!(sent_a.contains("mapped-a-hook"));
        assert!(!sent_a.contains("mapped-b"));
        let sent_b: Value = serde_json::from_str(&sent_b).unwrap();
        assert_eq!(sent_b["model"], "mapped-b-hook");
        assert_eq!(sent_b["input"][0]["content"], "clean baseline");
        let log = recv_terminal_request_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts[0]["provider_id"], aid);
        assert_eq!(attempts[1]["provider_id"], bid);
        assert_eq!(a.count.load(Ordering::SeqCst), 1);
        btask.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_cannot_exceed_provider_limit_or_forced_provider() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        for forced in [false, true] {
            let mut config = settings::read(app.handle()).unwrap();
            config.failover_max_providers_to_try = if forced { 3 } else { 1 };
            settings::write(app.handle(), &config).unwrap();
            let db = db::init_for_tests(&home.path().join(format!("limit-{forced}.db"))).unwrap();
            let a = upstream(BAD_TAIL, Some(PREFIX), None).await;
            let b = upstream(GOOD, None, None).await;
            let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
            insert_codex_provider_with_priority(&db, "B", b.url.clone(), 1);
            let detail = persist_and_reload_plugin_detail(
                &db,
                &plugin(&home.path().join(format!("plugin-{forced}")), true),
            );
            let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
            let pipeline = GatewayPluginPipeline::for_tests_shared(
                vec![detail],
                executor.clone(),
                GatewayPluginPipelineConfig::default(),
            );
            let (tx, mut logs) = mpsc::channel(16);
            let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
            let router = build_router(state);
            let path = if forced {
                format!("/codex/_aio/provider/{aid}/v1/responses")
            } else {
                "/codex/v1/responses".into()
            };
            let request = Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(request_body("")))
                .unwrap();
            let response = router.oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
            let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            let value: Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(value["error_code"], "GW_RESPONSE_REJECTED");
            assert_eq!(a.count.load(Ordering::SeqCst), 1);
            assert_eq!(b.count.load(Ordering::SeqCst), 0);
            let log = recv_terminal_request_log(&mut logs).await;
            assert_eq!(log.error_code.as_deref(), Some("GW_RESPONSE_REJECTED"));
            executor.dispose_runtime_caches_for_tests();
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_cancelled_replay_keeps_capacity_until_chunk_work_finishes() {
        use crate::gateway::plugins::before_commit::{
            GatewayCommitContext, GatewayCommitFuture, GatewayCommitResult,
        };
        use crate::gateway::plugins::context::GatewayVisibleHookContext;
        use crate::gateway::plugins::pipeline::{GatewayHookFuture, GatewayPluginExecutor};

        struct HeldChunkExecutor {
            started: mpsc::UnboundedSender<()>,
            release: Arc<Notify>,
        }
        impl GatewayPluginExecutor for HeldChunkExecutor {
            fn execute_response_commit_hook(
                &self,
                _: &PluginDetail,
                _: GatewayCommitContext,
                _: Duration,
            ) -> GatewayCommitFuture {
                Box::pin(async { Ok(GatewayCommitResult::Pass) })
            }
            fn execute_request_hook(
                &self,
                _: &PluginDetail,
                _: GatewayVisibleHookContext,
                _: Duration,
            ) -> GatewayHookFuture {
                Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
            }
            fn execute_response_hook(
                &self,
                _: &PluginDetail,
                _: GatewayVisibleHookContext,
                _: Duration,
            ) -> GatewayHookFuture {
                Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
            }
            fn execute_log_hook(
                &self,
                _: &PluginDetail,
                _: GatewayVisibleHookContext,
                _: Duration,
            ) -> GatewayHookFuture {
                Box::pin(async { Ok(GatewayHookResult::continue_unchanged()) })
            }
            fn execute_stream_hook(
                &self,
                _: &PluginDetail,
                context: GatewayVisibleHookContext,
                _: Duration,
            ) -> GatewayHookFuture {
                let started = self.started.clone();
                let release = self.release.clone();
                Box::pin(async move {
                    let wait = release.notified();
                    started.send(()).unwrap();
                    wait.await;
                    assert!(context.stream.chunk.unwrap().contains("B-ONLY"));
                    Ok(GatewayHookResult::continue_unchanged())
                })
            }
        }

        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("replay-capacity.db")).unwrap();
        let upstream = upstream(GOOD, None, None).await;
        insert_codex_provider(&db, upstream.url.clone());
        let detail = plugin(&home.path().join("plugin"), false);
        let (started, mut started_rx) = mpsc::unbounded_channel();
        let release = Arc::new(Notify::new());
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail, stream_chunk_plugin()],
            Arc::new(HeldChunkExecutor {
                started,
                release: release.clone(),
            }),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let router = build_router(state);
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/codex/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body("")))
                .unwrap()
        };
        let first = router.clone().oneshot(request()).await.unwrap();
        let second = router.clone().oneshot(request()).await.unwrap();
        for _ in 0..2 {
            tokio::time::timeout(Duration::from_secs(3), started_rx.recv())
                .await
                .unwrap()
                .unwrap();
        }
        assert_eq!(active.snapshot().len(), 2);
        drop(first);
        // The first Codex relay still owns the buffered replay and is blocked
        // in its chunk hook, even though its downstream body was dropped.
        let third = router.oneshot(request()).await.unwrap();
        let third_status = third.status();
        let upstream_count = upstream.count.load(Ordering::SeqCst);
        // Ensure an incorrectly admitted third replay enters its hook before
        // releasing the barrier, so this test always cleans up every relay.
        if third_status.is_success() {
            tokio::time::timeout(Duration::from_secs(3), started_rx.recv())
                .await
                .unwrap()
                .unwrap();
        }
        release.notify_waiters();
        drop(second);
        drop(third);
        for _ in 0..3 {
            let _ = recv_terminal_request_log(&mut logs).await;
        }
        assert_eq!(third_status, StatusCode::BAD_GATEWAY);
        assert_eq!(upstream_count, 2, "retained replay work still occupies both slots");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_large_sse_replay_preserves_legacy_chunk_budget_and_utf8() {
        use crate::gateway::plugins::before_commit::GatewayCommitResult;
        use crate::gateway::plugins::context::DEFAULT_PLUGIN_CONTEXT_STREAM_BYTES;

        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let mut config = settings::read(app.handle()).unwrap();
        config.enable_response_fixer = false;
        settings::write(app.handle(), &config).unwrap();
        let db = db::init_for_tests(&home.path().join("replay-chunks.db")).unwrap();
        let event = "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"汉😀complete text\"}\n\n";
        let mut expected = event.repeat((128usize * 1024).div_ceil(event.len()));
        expected.push_str(GOOD);
        let upstream_body = expected.clone();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let upstream_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_complete_http_request(&mut socket).await;
            socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n").await.unwrap();
            for chunk in upstream_body.as_bytes().chunks(8 * 1024) {
                socket.write_all(chunk).await.unwrap();
                tokio::task::yield_now().await;
            }
            socket.shutdown().await.unwrap();
        });
        insert_codex_provider(&db, url);
        let detail = plugin(&home.path().join("plugin"), false);
        let mut chunk_plugin = stream_chunk_plugin();
        gateway_hook_mut(&mut chunk_plugin).failure_policy = Some("fail-closed".into());
        let validations = Arc::new(AtomicUsize::new(0));
        let validation_count = validations.clone();
        let full_body_bytes = expected.len();
        let chunks = Arc::new(Mutex::new(Vec::new()));
        let observed = chunks.clone();
        let executor = InMemoryGatewayPluginExecutor::new()
            .with_response_commit_handler(&detail.summary.plugin_id, move |_, context| {
                validation_count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(context.response.decoded_bytes, full_body_bytes);
                assert!(context.response.body.ends_with(GOOD));
                Ok(GatewayCommitResult::Pass)
            })
            .with_stream_handler("test.stream-chunk", move |context| {
                let chunk = context.stream.chunk.unwrap();
                observed.lock().unwrap().push((chunk.len(), context.stream.chunk_truncated));
                GatewayHookResult {
                    stream_chunk: Some(chunk),
                    ..GatewayHookResult::continue_unchanged()
                }
            });
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail, chunk_plugin],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let request = Request::builder()
            .method("POST")
            .uri("/codex/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(request_body("")))
            .unwrap();
        let response = build_router(state).oneshot(request).await.unwrap();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let log = recv_terminal_request_log(&mut logs).await;
        upstream_task.abort();
        assert_eq!(validations.load(Ordering::SeqCst), 1);
        let chunks = chunks.lock().unwrap();
        assert!(chunks.iter().all(|(len, truncated)| {
            *len <= DEFAULT_PLUGIN_CONTEXT_STREAM_BYTES && !truncated
        }), "accepted replay must not force legacy hooks to inspect truncated chunks: {chunks:?}");
        assert!(chunks.len() > 1);
        assert_eq!(body.as_ref(), expected.as_bytes());
        assert_eq!(log.error_code, None);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_chunk_rewrite_preserves_http_body_framing() {
        use crate::gateway::plugins::before_commit::GatewayCommitResult;

        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let mut config = settings::read(app.handle()).unwrap();
        config.enable_response_fixer = false;
        settings::write(app.handle(), &config).unwrap();
        let db = db::init_for_tests(&home.path().join("chunk-framing.db")).unwrap();
        let upstream = upstream(GOOD, None, None).await;
        insert_codex_provider(&db, upstream.url.clone());
        let detail = plugin(&home.path().join("plugin"), false);
        let executor = InMemoryGatewayPluginExecutor::new()
            .with_response_commit_handler(&detail.summary.plugin_id, |_, _| {
                Ok(GatewayCommitResult::Pass)
            })
            .with_stream_handler("test.stream-chunk", |context| GatewayHookResult {
                stream_chunk: Some(
                    context.stream.chunk.unwrap().replace("B-ONLY", "B-ONLY-AFTER-REWRITE"),
                ),
                ..GatewayHookResult::continue_unchanged()
            });
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail, stream_chunk_plugin()],
            Arc::new(executor),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let (gateway, server) = serve_gateway(state).await;
        let response = reqwest::Client::new()
            .post(format!("{gateway}/codex/v1/responses"))
            .header("content-type", "application/json")
            .body(request_body(""))
            .send()
            .await
            .expect("rewriting an accepted SSE body must retain valid HTTP framing");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.bytes().await.expect("complete rewritten SSE body");
        let log = recv_terminal_request_log(&mut logs).await;
        server.abort();
        assert_eq!(body.as_ref(), GOOD.replace("B-ONLY", "B-ONLY-AFTER-REWRITE").as_bytes());
        assert_eq!(log.error_code, None);
    }

    async fn assert_headers_wait_terminal_preserves_sent_attempt(cancel_client: bool) {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let mut config = settings::read(app.handle()).unwrap();
        config.upstream_request_timeout_non_streaming_seconds = 1;
        settings::write(app.handle(), &config).unwrap();
        let db = db::init_for_tests(&home.path().join("headers-wait.db")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (received_tx, received_rx) = oneshot::channel();
        let upstream_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_complete_http_request(&mut socket).await;
            let _ = received_tx.send(request);
            // The complete request reached upstream, but no response headers
            // arrive before the client disconnects or the request deadline.
            let mut byte = [0u8];
            let _ = socket.read(&mut byte).await;
        });
        let provider_id = insert_codex_provider(&db, url);
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let (gateway, server) = serve_gateway(state).await;
        let address = gateway.trim_start_matches("http://");
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        let body = request_body(r#", "reasoning":{"effort":"high"}"#);
        let wire = format!("POST /codex/v1/responses HTTP/1.1\r\nhost: {address}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
        socket.write_all(wire.as_bytes()).await.unwrap();
        let received = tokio::time::timeout(Duration::from_secs(3), received_rx)
            .await
            .unwrap()
            .unwrap();
        assert!(received.contains(r#""effort":"high""#));
        let expected_error = if cancel_client {
            drop(socket);
            "GW_REQUEST_ABORTED"
        } else {
            let mut response = Vec::new();
            tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut response))
                .await
                .unwrap()
                .unwrap();
            assert!(String::from_utf8_lossy(&response).contains("GW_RESPONSE_COMMIT_TIMEOUT"));
            "GW_RESPONSE_COMMIT_TIMEOUT"
        };
        let log = recv_terminal_request_log(&mut logs).await;
        assert_eq!(log.error_code.as_deref(), Some(expected_error));
        assert!(active.snapshot().is_empty());
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts.as_array().unwrap().len(), 1);
        assert_eq!(attempts[0]["provider_id"], provider_id);
        assert_eq!(attempts[0]["error_code"], expected_error);
        assert_eq!(attempts[0]["upstream_sent"], true);
        assert_eq!(attempts[0]["reasoning_effort"], "high");
        upstream_task.abort();
        server.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_headers_wait_deadline_preserves_sent_attempt() {
        assert_headers_wait_terminal_preserves_sent_attempt(false).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_headers_wait_cancel_preserves_sent_attempt() {
        assert_headers_wait_terminal_preserves_sent_attempt(true).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_total_deadline_bounds_non_success_body_and_keeps_terminal_attempt() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let mut config = settings::read(app.handle()).unwrap();
        config.upstream_request_timeout_non_streaming_seconds = 1;
        settings::write(app.handle(), &config).unwrap();
        let db = db::init_for_tests(&home.path().join("deadline.db")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_complete_http_request(&mut socket).await;
            socket.write_all(b"HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: 100\r\nconnection: close\r\n\r\n{").await.unwrap();
            std::future::pending::<()>().await;
        });
        let aid = insert_codex_provider(&db, url);
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let circuit = state.circuit.clone();
        let router = build_router(state);
        let request = Request::builder()
            .method("POST")
            .uri("/codex/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(request_body("")))
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(3), router.oneshot(request))
            .await
            .unwrap()
            .unwrap();
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error_code"], "GW_RESPONSE_COMMIT_TIMEOUT");
        let log = recv_terminal_request_log(&mut logs).await;
        assert_eq!(
            log.error_code.as_deref(),
            Some("GW_RESPONSE_COMMIT_TIMEOUT")
        );
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts[0]["provider_id"], aid);
        assert_eq!(attempts[0]["error_code"], "GW_RESPONSE_COMMIT_TIMEOUT");
        assert!(active.snapshot().is_empty());
        assert_eq!(
            circuit
                .snapshot(aid, crate::gateway::util::now_unix_seconds() as i64)
                .failure_count,
            0
        );
        task.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_nonmatching_scope_preserves_immediate_stream_delivery() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("unmatched.db")).unwrap();
        let release = Arc::new(Notify::new());
        let a = upstream(BAD_TAIL, Some(PREFIX), Some(release.clone())).await;
        insert_codex_provider(&db, a.url.clone());
        let root = home.path().join("plugin");
        let mut detail = plugin(&root, true);
        gateway_hook_mut(&mut detail)
            .request_match
            .as_mut()
            .unwrap()
            .paths = vec!["/v1/chat/completions".into()];
        write_extension_host_test_entry(
            None,
            &root,
            &detail.manifest,
            include_str!("../../../examples/plugins/codex-model-consistency/extension.cjs"),
        );
        let detail = persist_and_reload_plugin_detail(&db, &detail);
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let (gateway, server) = serve_gateway(state).await;
        let request = reqwest::Client::new()
            .post(format!("{gateway}/codex/v1/responses"))
            .header("content-type", "application/json")
            .body(request_body(""));
        let mut response = tokio::time::timeout(Duration::from_secs(3), request.send())
            .await
            .expect("unmatched request does not wait for the suffix")
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["x-upstream-private"], "A-SECRET-HEADER");
        let first = tokio::time::timeout(Duration::from_secs(3), response.chunk())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(!first.is_empty());
        release.notify_one();
        response.bytes().await.unwrap();
        let _ = recv_terminal_request_log(&mut logs).await;
        server.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_block_is_a_policy_rejection_and_never_switches() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("blocked.db")).unwrap();
        let a = upstream(BAD_TAIL, Some(PREFIX), None).await;
        let b = upstream(GOOD, None, None).await;
        let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
        insert_codex_provider_with_priority(&db, "B", b.url.clone(), 1);
        let root = home.path().join("plugin");
        let detail = plugin(&root, false);
        let entry = root.join(detail.manifest.main.as_ref().unwrap());
        let source = std::fs::read_to_string(&entry)
            .unwrap()
            .replace("action:\"switchProvider\"", "action:\"block\"");
        std::fs::write(entry, source).unwrap();
        let detail = persist_and_reload_plugin_detail(&db, &detail);
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let circuit = state.circuit.clone();
        let request = Request::builder()
            .method("POST")
            .uri("/codex/v1/responses")
            .header("content-type", "application/json")
            .body(Body::from(request_body("")))
            .unwrap();
        let response = build_router(state).oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let log = recv_terminal_request_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(log.error_code.as_deref(), Some("GW_RESPONSE_REJECTED"));
        assert_eq!(attempts[0]["outcome"], "response_rejected");
        assert_eq!(attempts[0]["reason_code"], "response_rejected");
        assert_eq!(attempts[0]["decision"], "abort");
        assert_eq!(b.count.load(Ordering::SeqCst), 0);
        assert_eq!(
            circuit
                .snapshot(aid, crate::gateway::util::now_unix_seconds() as i64)
                .failure_count,
            0
        );
        executor.dispose_runtime_caches_for_tests();
    }

    async fn assert_route_change_keeps_in_flight_candidates_and_new_session_binding(
        new_request_finishes_first: bool,
    ) {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("route-change.db")).unwrap();
        let release = Arc::new(Notify::new());
        let mut p = upstream(BAD_TAIL, Some(PREFIX), Some(release.clone())).await;
        let q = upstream(GOOD, None, None).await;
        let r = upstream(GOOD, None, None).await;
        let pid = insert_codex_provider_with_priority(&db, "P", p.url.clone(), 0);
        let qid = insert_codex_provider_with_priority(&db, "Q", q.url.clone(), 1);
        let rid = insert_codex_provider_with_priority(&db, "R", r.url.clone(), 2);
        let old_mode = crate::sort_modes::create_mode(&db, "Old route").unwrap().id;
        let new_mode = crate::sort_modes::create_mode(&db, "New route").unwrap().id;
        crate::sort_modes::set_mode_providers_order(&db, old_mode, "codex", vec![pid, qid])
            .unwrap();
        crate::sort_modes::set_mode_providers_order(&db, new_mode, "codex", vec![rid]).unwrap();
        crate::sort_modes::set_active(&db, "codex", Some(old_mode)).unwrap();
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(32);
        let state =
            gateway_state_with_plugin_pipeline(app.handle().clone(), db.clone(), tx, pipeline);
        let sessions = state.session.clone();
        let router = build_router(state);
        let session = "route-change-session-000000";
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/codex/v1/responses")
                .header("content-type", "application/json")
                .header("x-session-id", session)
                .body(Body::from(request_body("")))
                .unwrap()
        };
        let mut old_response = Box::pin(router.clone().oneshot(request()));
        tokio::select! {
            captured = p.captured.recv() => { assert!(captured.is_some()); },
            response = &mut old_response => panic!("old response escaped the tail barrier: {response:?}"),
        }

        // Mirror the persisted active-mode change and its production session
        // invalidation while P still owns the old request's response suffix.
        crate::sort_modes::set_active(&db, "codex", Some(new_mode)).unwrap();
        assert_eq!(sessions.clear_cli_bindings("codex"), 1);
        if new_request_finishes_first {
            let response = router.clone().oneshot(request()).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            let log = recv_terminal_request_log(&mut logs).await;
            let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
            assert_eq!(attempts.as_array().unwrap().len(), 1);
            assert_eq!(attempts[0]["provider_id"], rid);
        }

        release.notify_one();
        let response = old_response.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let log = recv_terminal_request_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts.as_array().unwrap().len(), 2);
        assert_eq!(attempts[0]["provider_id"], pid);
        assert_eq!(attempts[0]["decision"], "switch");
        assert_eq!(attempts[1]["provider_id"], qid);
        let now = crate::gateway::util::now_unix_seconds() as i64;
        if new_request_finishes_first {
            assert_eq!(
                sessions.get_bound_provider("codex", session, now),
                Some(rid)
            );
            assert_eq!(
                sessions.get_bound_sort_mode_id("codex", session, now),
                Some(Some(new_mode))
            );
        } else {
            assert_eq!(sessions.get_bound_provider("codex", session, now), None);
            assert_eq!(sessions.get_bound_sort_mode_id("codex", session, now), None);
        }

        let response = router.oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let log = recv_terminal_request_log(&mut logs).await;
        let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
        assert_eq!(attempts.as_array().unwrap().len(), 1);
        assert_eq!(attempts[0]["provider_id"], rid);
        assert_eq!(
            sessions.get_bound_provider("codex", session, now),
            Some(rid)
        );
        assert_eq!(
            sessions.get_bound_provider_order("codex", session, now),
            Some(vec![rid])
        );
        assert_eq!(p.count.load(Ordering::SeqCst), 1);
        assert_eq!(q.count.load(Ordering::SeqCst), 1);
        assert_eq!(
            r.count.load(Ordering::SeqCst),
            if new_request_finishes_first { 2 } else { 1 }
        );
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_route_change_preserves_in_flight_order_without_restoring_old_binding()
    {
        assert_route_change_keeps_in_flight_candidates_and_new_session_binding(false).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_old_completion_cannot_overwrite_a_new_route_session_binding() {
        assert_route_change_keeps_in_flight_candidates_and_new_session_binding(true).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_capacity_is_shared_across_gateways_until_delivery_is_released() {
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let db = db::init_for_tests(&home.path().join("shared-capacity.db")).unwrap();
        let upstream = upstream(GOOD, None, None).await;
        insert_codex_provider_with_priority(&db, "A", upstream.url.clone(), 0);
        let detail =
            persist_and_reload_plugin_detail(&db, &plugin(&home.path().join("plugin"), true));
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (tx, mut logs) = mpsc::channel(32);
        let first_state = gateway_state_with_plugin_pipeline(
            app.handle().clone(),
            db.clone(),
            tx.clone(),
            pipeline.clone(),
        );
        let second_state =
            gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let first_router = build_router(first_state);
        let second_router = build_router(second_state);
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/codex/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(request_body("")))
                .unwrap()
        };

        // Both real worker validations finish, but their downstream bodies
        // remain owned by consumers from two separate GatewayAppState values.
        let first = first_router.clone().oneshot(request()).await.unwrap();
        let second = second_router.clone().oneshot(request()).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(upstream.count.load(Ordering::SeqCst), 2);

        let rejected = second_router.clone().oneshot(request()).await.unwrap();
        assert_eq!(rejected.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(rejected.into_body(), 1024 * 1024).await.unwrap();
        let error: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(error["error_code"], "GW_RESPONSE_BUFFER_LIMIT");
        assert_eq!(upstream.count.load(Ordering::SeqCst), 2);

        // Releasing a held delivery returns its process-wide slot without a
        // new Gateway instance, plugin reload, or explicit capacity reset.
        drop(first);
        let admitted = first_router.oneshot(request()).await.unwrap();
        assert_eq!(admitted.status(), StatusCode::OK);
        assert_eq!(upstream.count.load(Ordering::SeqCst), 3);
        for response in [second, admitted] {
            let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            assert!(String::from_utf8_lossy(&body).contains("B-ONLY"));
        }
        // Let all existing finalizers finish before disposing the test worker.
        for _ in 0..4 {
            let _ = recv_terminal_request_log(&mut logs).await;
        }
        executor.dispose_runtime_caches_for_tests();
    }

    async fn assert_slow_terminal_log_has_one_owner(cancel_after_terminal: bool) {
        use tauri::Listener;
        let _env_lock = crate::test_support::test_env_lock();
        let home = tempfile::tempdir().unwrap();
        let _env = isolate_app_env(home.path());
        let app = tauri::test::mock_app();
        setup_settings(app.handle());
        let mut config = settings::read(app.handle()).unwrap();
        config.upstream_request_timeout_non_streaming_seconds = 1;
        config.failover_max_providers_to_try = 1;
        settings::write(app.handle(), &config).unwrap();
        let db = db::init_for_tests(&home.path().join("slow-terminal-log.db")).unwrap();
        // JSON exercises the handler that finalizes before returning a body;
        // rejection exercises provider exhaustion's separate terminal path.
        let body = if cancel_after_terminal {
            r#"{"model":"wrong-model","status":"completed","output":[]}"#
        } else {
            r#"{"model":"gpt-commit","status":"completed","output":[]}"#
        };
        let (url, upstream_task) = spawn_json_upstream(body).await;
        insert_codex_provider_with_priority(&db, "A", url, 0);
        let root = home.path().join("plugin");
        let mut detail = plugin(&root, true);
        detail
            .manifest
            .contributes
            .as_mut()
            .unwrap()
            .gateway_hooks
            .push(PluginHook {
                name: "log.beforePersist".into(),
                priority: 10,
                failure_policy: Some("fail-closed".into()),
                timeout_ms: Some(5_000),
                request_match: None,
            });
        detail.granted_permissions.push("log.redact".into());
        let entry = root.join(detail.manifest.main.as_ref().unwrap());
        let source = format!(
            r#"{}
            const originalActivate = module.exports.activate;
            module.exports.activate = function(api) {{
                originalActivate(api);
                api.gateway.registerHook("log.beforePersist", function(invocation) {{
                    const log = JSON.parse(invocation.context.log.message);
                    if (log.status !== null) {{
                        const until = Date.now() + 1400;
                        while (Date.now() < until) {{}}
                    }}
                    return {{action:"continue"}};
                }});
            }};"#,
            std::fs::read_to_string(&entry).unwrap()
        );
        write_extension_host_test_entry(None, &root, &detail.manifest, &source);
        let detail = persist_and_reload_plugin_detail(&db, &detail);
        let executor = Arc::new(RuntimeGatewayPluginExecutor::with_db(db.clone()));
        let pipeline = GatewayPluginPipeline::for_tests_shared(
            vec![detail],
            executor.clone(),
            GatewayPluginPipelineConfig::default(),
        );
        let (event_tx, mut events) = mpsc::unbounded_channel();
        app.handle().listen_any("gateway:request", move |event| {
            event_tx
                .send(serde_json::from_str::<Value>(event.payload()).unwrap())
                .unwrap();
        });
        let (tx, mut logs) = mpsc::channel(16);
        let state = gateway_state_with_plugin_pipeline(app.handle().clone(), db, tx, pipeline);
        let active = state.active_requests.clone();
        let (gateway, server) = serve_gateway(state).await;
        let mut socket = tokio::net::TcpStream::connect(gateway.trim_start_matches("http://"))
            .await
            .unwrap();
        let body = request_body("");
        let request = format!("POST /codex/v1/responses HTTP/1.1\r\nHost: local.test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let started = std::time::Instant::now();
        socket.write_all(request.as_bytes()).await.unwrap();
        let terminal = tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(active.snapshot().is_empty());
        let expected_error = if cancel_after_terminal {
            Some("GW_RESPONSE_REJECTED")
        } else {
            None
        };
        assert_eq!(terminal["error_code"].as_str(), expected_error);
        if cancel_after_terminal {
            // Closing a real client connection after the terminal event must
            // not cancel the task that now owns the delayed persistence.
            drop(socket);
        } else {
            let mut response = Vec::new();
            tokio::time::timeout(Duration::from_secs(3), socket.read_to_end(&mut response))
                .await
                .unwrap()
                .unwrap();
            assert!(String::from_utf8(response)
                .unwrap()
                .starts_with("HTTP/1.1 200"));
        }
        let log = recv_terminal_request_log(&mut logs).await;
        assert!(
            started.elapsed() >= Duration::from_millis(1300),
            "the real log worker must cross the 1s request deadline"
        );
        assert_eq!(log.error_code.as_deref(), expected_error);
        assert!(
            events.try_recv().is_err(),
            "terminal event must be emitted once"
        );
        while let Ok(log) = logs.try_recv() {
            assert!(log.status.is_none(), "terminal log must be enqueued once");
        }
        server.abort();
        upstream_task.abort();
        executor.dispose_runtime_caches_for_tests();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_slow_terminal_log_crosses_deadline_without_second_terminal() {
        assert_slow_terminal_log_has_one_owner(false).await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn response_commit_slow_terminal_log_survives_client_cancel_without_second_terminal() {
        assert_slow_terminal_log_has_one_owner(true).await;
    }

    include!("routes_response_commit_codex_tests.rs");
}
