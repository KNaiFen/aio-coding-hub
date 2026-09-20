// Opt-in local acceptance: installed Codex -> this gateway -> synthetic A/B.
// No remote providers or real credentials are used. Kept ignored because CI need
// not have Codex installed; run with `response_commit_installed_codex --ignored`.
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
#[ignore = "requires the installed Codex CLI and macOS loopback sandbox"]
async fn response_commit_installed_codex_only_receives_accepted_provider() {
    let _env_lock = crate::test_support::test_env_lock();
    let temp = tempfile::tempdir().unwrap();
    let _env = isolate_app_env(temp.path());
    let app = tauri::test::mock_app();
    setup_settings(app.handle());
    let db = db::init_for_tests(&temp.path().join("codex-acceptance.db")).unwrap();
    let work = temp.path().join("workspace");
    std::fs::create_dir(&work).unwrap();
    let version = std::process::Command::new("codex")
        .arg("--version")
        .output()
        .expect("installed codex executable");
    assert!(version.status.success());

    // A contains a syntactically valid, harmless shell tool. The client must
    // never observe it, so no command_execution event may appear in its JSONL.
    const A: &str = concat!(
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_rejected_A\",\"model\":\"gpt-commit\",\"status\":\"in_progress\",\"output\":[]}}\n\n",
        "event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"id\":\"fc_A\",\"type\":\"function_call\",\"call_id\":\"call_A\",\"name\":\"shell_command\",\"arguments\":\"{\\\"command\\\":\\\"printf A_TOOL_EXECUTED\\\"}\"}}\n\n",
        "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_rejected_A\",\"status\":\"completed\",\"model\":\"wrong-model\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n"
    );
    const B: &str = concat!(
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_accepted_B\",\"model\":\"gpt-commit\",\"status\":\"in_progress\",\"output\":[]}}\n\n",
        "event: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"id\":\"msg_B\",\"type\":\"message\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.content_part.added\ndata: {\"type\":\"response.content_part.added\",\"item_id\":\"msg_B\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\",\"annotations\":[]}}\n\n",
        "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"item_id\":\"msg_B\",\"output_index\":0,\"content_index\":0,\"delta\":\"B-ONLY\"}\n\n",
        "event: response.output_text.done\ndata: {\"type\":\"response.output_text.done\",\"item_id\":\"msg_B\",\"output_index\":0,\"content_index\":0,\"text\":\"B-ONLY\"}\n\n",
        "event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"id\":\"msg_B\",\"type\":\"message\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"B-ONLY\",\"annotations\":[]}]}}\n\n",
        "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_accepted_B\",\"object\":\"response\",\"status\":\"completed\",\"model\":\"gpt-commit\",\"output\":[{\"id\":\"msg_B\",\"type\":\"message\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"B-ONLY\",\"annotations\":[]}]}],\"usage\":{\"input_tokens\":3,\"output_tokens\":2,\"total_tokens\":5}}}\n\n"
    );
    let mut a = codex_acceptance_upstream(A).await;
    let mut b = codex_acceptance_upstream(B).await;
    let aid = insert_codex_provider_with_priority(&db, "A", a.url.clone(), 0);
    let bid = insert_codex_provider_with_priority(&db, "B", b.url.clone(), 1);
    let detail = persist_and_reload_plugin_detail(&db, &plugin(&temp.path().join("plugin"), true));
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
    let profile = temp.path().join("loopback.sb");
    std::fs::write(
        &profile,
        concat!(
            "(version 1)\n(allow default)\n(deny network*)\n",
            "(allow network-outbound (remote ip \"localhost:*\"))\n",
            "(allow network-bind (local ip \"localhost:*\"))\n",
            "(allow network-inbound (local ip \"localhost:*\"))\n",
            "(allow network* (local unix-socket) (remote unix-socket))\n"
        ),
    )
    .unwrap();
    let mut command = tokio::process::Command::new("/usr/bin/sandbox-exec");
    command
        .args(["-f"])
        .arg(&profile)
        .arg("codex")
        .args([
            "exec",
            "--ignore-user-config",
            "--ignore-rules",
            "--ephemeral",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--json",
            "-C",
        ])
        .arg(&work);
    for setting in [
        "model_provider=\"aio_commit_fixture\"".to_string(),
        format!("model=\"{MODEL}\""),
        "model_reasoning_effort=\"low\"".into(),
        "model_providers.aio_commit_fixture.name=\"Local fixture\"".into(),
        format!("model_providers.aio_commit_fixture.base_url=\"{gateway}/codex/v1\""),
        "model_providers.aio_commit_fixture.wire_api=\"responses\"".into(),
        "model_providers.aio_commit_fixture.requires_openai_auth=false".into(),
        "model_providers.aio_commit_fixture.supports_websockets=false".into(),
        "model_providers.aio_commit_fixture.request_max_retries=0".into(),
        "model_providers.aio_commit_fixture.stream_max_retries=0".into(),
        "features.apps=false".into(),
        "features.remote_plugin=false".into(),
        "features.memories=false".into(),
        "features.shell_snapshot=false".into(),
        "history.persistence=\"none\"".into(),
        "web_search=\"disabled\"".into(),
        "check_for_update_on_startup=false".into(),
        "analytics.enabled=false".into(),
        "otel.exporter=\"none\"".into(),
        "otel.trace_exporter=\"none\"".into(),
        "otel.metrics_exporter=\"none\"".into(),
    ] {
        command.arg("-c").arg(setting);
    }
    command
        .arg("Reply with exactly B-ONLY. Do not call tools.")
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .expect("Codex acceptance watchdog")
        .unwrap();
    server.abort();
    executor.dispose_runtime_caches_for_tests();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        output.status.success(),
        "Codex failed: {stdout}; {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let events = stdout
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(events.iter().any(|event| event["type"] == "turn.completed"));
    assert!(events.iter().any(|event| event["item"]["text"] == "B-ONLY"));
    assert!(!stdout.contains("command_execution"));
    assert!(!stdout.contains("A_TOOL_EXECUTED"));
    assert_eq!(a.count.load(Ordering::SeqCst), 1);
    assert_eq!(b.count.load(Ordering::SeqCst), 1);
    // The actual wire model and clean input must agree across both attempts.
    for request in [
        a.captured.recv().await.unwrap(),
        b.captured.recv().await.unwrap(),
    ] {
        assert!(!request.contains("A_TOOL_EXECUTED"));
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["model"], MODEL);
    }
    let log = recv_terminal_request_log(&mut logs).await;
    let attempts: Value = serde_json::from_str(&log.attempts_json).unwrap();
    assert_eq!(attempts[0]["provider_id"], aid);
    assert_eq!(attempts[0]["decision"], "switch");
    assert_eq!(attempts[1]["provider_id"], bid);
    assert_eq!(attempts[1]["outcome"], "success");
    assert!(active.snapshot().is_empty());
    println!("{}: A attempts=1; B attempts=1; accepted text=B-ONLY; tool executions=0; active requests=0", String::from_utf8_lossy(&version.stdout).trim());
}

#[cfg(target_os = "macos")]
async fn codex_acceptance_upstream(body: &'static str) -> Upstream {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let hits = count.clone();
    let (tx, captured) = mpsc::unbounded_channel();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            hits.fetch_add(1, Ordering::SeqCst);
            let request =
                split_raw_http_request(read_complete_http_request_bytes(&mut socket).await);
            let mut decoded = Vec::new();
            if request.has_header_line("content-encoding: gzip") {
                std::io::Read::read_to_end(
                    &mut flate2::read::GzDecoder::new(request.body.as_slice()),
                    &mut decoded,
                )
                .unwrap();
            } else {
                decoded = request.body;
            }
            let _ = tx.send(format!(
                "{}\r\n\r\n{}",
                request.head,
                String::from_utf8(decoded).unwrap()
            ));
            let response = format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", body.len(), body);
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });
    Upstream {
        url,
        count,
        captured,
        task,
    }
}
