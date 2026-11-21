use super::*;

#[test]
fn required_events_need_their_fields() {
    assert!(validate_event(
        "Runtime.exceptionThrown",
        &json!({"exceptionDetails": {"text": "boom"}})
    )
    .is_ok());
    assert!(validate_event("Runtime.exceptionThrown", &json!({})).is_err());
    assert!(validate_event("Runtime.exceptionThrown", &json!({"exceptionDetails": {}})).is_err());
    assert!(validate_event("Target.targetDestroyed", &json!({"targetId": "t"})).is_ok());
    assert!(validate_event("Target.targetDestroyed", &json!({})).is_err());
    assert!(validate_event("Network.responseReceived", &json!({})).is_ok());
}

#[test]
fn typed_events_reject_wrongly_typed_fields() {
    assert!(validate_event(
        "Runtime.exceptionThrown",
        &json!({"exceptionDetails": {"text": "boom", "exception": {"description": 42}}})
    )
    .is_err());
    assert!(validate_event(
        "Runtime.exceptionThrown",
        &json!({"exceptionDetails": {"text": "boom", "exception": {"description": "Error"}}})
    )
    .is_ok());
    assert!(validate_event("Target.targetDestroyed", &json!({"targetId": 42})).is_err());
    assert!(validate_event(
        "Browser.downloadWillBegin",
        &json!({"guid": "g", "suggestedFilename": "f"})
    )
    .is_ok());
    assert!(validate_event("Browser.downloadWillBegin", &json!({"guid": "g"})).is_err());
    assert!(validate_event(
        "Browser.downloadProgress",
        &json!({"guid": "g", "state": "completed"})
    )
    .is_ok());
    assert!(validate_event(
        "Browser.downloadProgress",
        &json!({"guid": "g", "state": "inProgress"})
    )
    .is_ok());
    assert!(validate_event(
        "Browser.downloadProgress",
        &json!({"guid": "g", "state": "nope"})
    )
    .is_err());
}

fn test_inner(
    capture: bool,
) -> (
    Arc<Inner>,
    mpsc::UnboundedReceiver<String>,
    mpsc::UnboundedReceiver<CdpError>,
) {
    let (commands, command_receiver) = mpsc::unbounded_channel();
    let (handler_sender, handler_receiver) = mpsc::unbounded_channel();
    let inner = Arc::new(Inner {
        commands,
        pending: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(0),
        child: AsyncMutex::new(None),
        ws_url: "ws://test/".to_owned(),
        subscriptions: Mutex::new(Vec::new()),
        load_waiters: Mutex::new(HashMap::new()),
        capture_workers: AtomicBool::new(capture),
        closing: AtomicBool::new(false),
        surface_invalid: true,
        request_timeout: Duration::from_secs(5),
        viewport: None,
        handler_errors: Mutex::new(Some(handler_sender)),
    });
    (inner, command_receiver, handler_receiver)
}

fn attached_params(child: &str, target: &str, kind: &str, waiting: bool) -> Value {
    json!({
        "sessionId": child,
        "targetInfo": {"targetId": target, "type": kind},
        "waitingForDebugger": waiting,
    })
}

async fn next_command(commands: &mut mpsc::UnboundedReceiver<String>) -> Value {
    let text = tokio::time::timeout(Duration::from_secs(5), commands.recv())
        .await
        .expect("setup command arrived")
        .expect("command channel open");
    serde_json::from_str(&text).expect("command is JSON")
}

fn respond_ok(inner: &Arc<Inner>, id: u64) {
    let raw = format!(r#"{{"id":{id},"result":{{}}}}"#);
    let value: Value = serde_json::from_str(&raw).unwrap();
    on_response(inner, id, &value, &raw);
}

#[tokio::test]
async fn worker_setup_enables_network_before_resuming() {
    let (inner, mut commands, _handler) = test_inner(true);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let params = attached_params("child", "worker-target", "worker", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    let first = next_command(&mut commands).await;
    assert_eq!(first["method"], "Network.enable");
    assert_eq!(first["sessionId"], "child");
    let id = first["id"].as_u64().unwrap();

    // Hold the ack: nothing else may be sent before Network.enable resolves.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        commands.try_recv().is_err(),
        "resume sent before Network.enable resolved"
    );
    respond_ok(&inner, id);

    let second = next_command(&mut commands).await;
    assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
    assert_eq!(second["sessionId"], "child");
    respond_ok(&inner, second["id"].as_u64().unwrap());
}

#[tokio::test]
async fn worker_setup_network_failure_never_resumes() {
    let (inner, mut commands, mut handler) = test_inner(true);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let params = attached_params("child", "worker-target", "worker", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    let first = next_command(&mut commands).await;
    assert_eq!(first["method"], "Network.enable");
    let id = first["id"].as_u64().unwrap();
    let reply = format!(r#"{{"id":{id},"error":{{"code":-32000,"message":"nope"}}}}"#);
    let reply_value: Value = serde_json::from_str(&reply).unwrap();
    on_response(&inner, id, &reply_value, &reply);

    // The setup task must return without resuming.
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        commands.try_recv().is_err(),
        "failed Network.enable still resumed"
    );

    let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
        .await
        .expect("handler error arrived")
        .expect("handler channel open");
    let message = error.to_string();
    assert!(
        message.contains("worker setup Network.enable failed"),
        "{message}"
    );
}

#[tokio::test]
async fn service_worker_setup_pipelines_network_before_resume() {
    let (inner, mut commands, mut handler) = test_inner(true);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let params = attached_params("child", "sw-target", "service_worker", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    // Withhold the Network.enable reply until the resume arrives: a
    // sequential setup would deadlock here on a paused service worker.
    let first = next_command(&mut commands).await;
    assert_eq!(first["method"], "Network.enable");
    assert_eq!(first["sessionId"], "child");
    let network_id = first["id"].as_u64().unwrap();

    let second = next_command(&mut commands).await;
    assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
    assert_eq!(second["sessionId"], "child");
    let resume_id = second["id"].as_u64().unwrap();
    assert!(
        network_id < resume_id,
        "Network.enable must precede resume on the wire"
    );

    // Both complete cleanly once the worker unpauses.
    respond_ok(&inner, network_id);
    respond_ok(&inner, resume_id);

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        handler.try_recv().is_err(),
        "pipelined service-worker setup must not report errors"
    );
    assert!(commands.try_recv().is_err(), "unexpected third command");
}

#[tokio::test]
async fn service_worker_setup_network_error_surfaces_on_handler() {
    let (inner, mut commands, mut handler) = test_inner(true);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let params = attached_params("child", "sw-target", "service_worker", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    let first = next_command(&mut commands).await;
    assert_eq!(first["method"], "Network.enable");
    let network_id = first["id"].as_u64().unwrap();

    // The resume is already enqueued to avoid the paused-worker deadlock.
    let second = next_command(&mut commands).await;
    assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
    let resume_id = second["id"].as_u64().unwrap();

    let reply = format!(r#"{{"id":{network_id},"error":{{"code":-32000,"message":"nope"}}}}"#);
    let reply_value: Value = serde_json::from_str(&reply).unwrap();
    on_response(&inner, network_id, &reply_value, &reply);
    respond_ok(&inner, resume_id);

    let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
        .await
        .expect("handler error arrived")
        .expect("handler channel open");
    let message = error.to_string();
    assert!(
        message.contains("worker setup Network.enable failed"),
        "{message}"
    );
}

#[tokio::test]
async fn service_worker_setup_sequences_resume_before_detach() {
    let (inner, mut commands, _handler) = test_inner(false);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let params = attached_params("child", "sw-target", "service_worker", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    let first = next_command(&mut commands).await;
    assert_eq!(first["method"], "Runtime.runIfWaitingForDebugger");
    assert_eq!(first["sessionId"], "child");

    // The detach must wait for the resume ack: one task, in order.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        commands.try_recv().is_err(),
        "detach sent before resume resolved"
    );
    respond_ok(&inner, first["id"].as_u64().unwrap());

    let second = next_command(&mut commands).await;
    assert_eq!(second["method"], "Target.detachFromTarget");
    assert_eq!(second["sessionId"], "parent");
    assert_eq!(second["params"]["sessionId"], "child");
    respond_ok(&inner, second["id"].as_u64().unwrap());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(commands.try_recv().is_err(), "unexpected third command");
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachedProbe {
    session_id: String,
}

impl EventKind for AttachedProbe {
    fn method_id() -> &'static str {
        "Target.attachedToTarget"
    }
}

#[tokio::test]
async fn waiting_iframe_resumes_without_network_enable() {
    let (inner, mut commands, _handler) = test_inner(true);
    let mut state = ConnState::default();
    state
        .sessions
        .insert("parent".to_owned(), "owner".to_owned());
    let mut attached = inner.subscribe::<AttachedProbe>(Some("owner".to_owned()));
    let params = attached_params("child", "iframe-target", "iframe", true);
    let raw = params.to_string();
    on_attached(&inner, &mut state, Some("parent"), &params, &raw);

    let command = next_command(&mut commands).await;
    assert_eq!(command["method"], "Runtime.runIfWaitingForDebugger");
    assert_eq!(command["sessionId"], "child");
    respond_ok(&inner, command["id"].as_u64().unwrap());

    // Capture is on, but a non-worker target must never enable Network.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        commands.try_recv().is_err(),
        "iframe setup must not enable Network"
    );

    let event = tokio::time::timeout(Duration::from_secs(5), attached.next())
        .await
        .expect("attached event published")
        .expect("stream open");
    assert_eq!(event.session_id, "child");
}

#[tokio::test]
async fn malformed_exception_event_surfaces_invalid_message() {
    let (inner, _commands, mut handler) = test_inner(false);
    let mut state = ConnState::default();
    let raw = r#"{"method":"Runtime.exceptionThrown","params":{"exceptionDetails":{"text":"boom","exception":{"description":42}}}}"#;
    on_text(&inner, &mut state, raw);
    let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
        .await
        .expect("handler error arrived")
        .expect("handler channel open");
    assert!(
        matches!(&error, CdpError::InvalidMessage(payload, _) if payload == raw),
        "malformed exception must surface InvalidMessage, got {error:?}"
    );
}

#[tokio::test]
async fn event_stream_reports_unparsable_subscription() {
    let (inner, _commands, mut handler) = test_inner(false);
    let mut stream = inner.subscribe::<EventExceptionThrown>(None);
    inner.publish(
        None,
        "Runtime.exceptionThrown",
        json!({"exceptionDetails": 7}),
        Some("bad-frame"),
    );
    inner.publish(
        None,
        "Runtime.exceptionThrown",
        json!({"exceptionDetails": {"text": "boom"}}),
        Some("good-frame"),
    );
    let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("good event arrived")
        .expect("stream open");
    assert_eq!(event.exception_details.text, "boom");
    let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
        .await
        .expect("handler error arrived")
        .expect("handler channel open");
    assert!(
        matches!(&error, CdpError::InvalidMessage(payload, _) if payload == "bad-frame"),
        "unparsable subscription must surface InvalidMessage, got {error:?}"
    );

    // Without the raw frame, the payload falls back to the value itself.
    let mut stream = inner.subscribe::<EventExceptionThrown>(None);
    let bad = json!({"exceptionDetails": 7});
    inner.publish(None, "Runtime.exceptionThrown", bad.clone(), None);
    inner.publish(
        None,
        "Runtime.exceptionThrown",
        json!({"exceptionDetails": {"text": "again"}}),
        None,
    );
    let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("good event arrived")
        .expect("stream open");
    assert_eq!(event.exception_details.text, "again");
    let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
        .await
        .expect("handler error arrived")
        .expect("handler channel open");
    assert!(
        matches!(&error, CdpError::InvalidMessage(payload, _) if payload == &bad.to_string()),
        "fallback payload must be the value, got {error:?}"
    );
}

#[tokio::test]
async fn browser_close_response_before_close_frame_is_clean() {
    use futures::StreamExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = async_tungstenite::tokio::accept_async(socket)
            .await
            .unwrap();
        while let Some(message) = ws.next().await {
            let command: Value = serde_json::from_str(message.unwrap().to_text().unwrap()).unwrap();
            if command["method"] == "Browser.close" {
                ws.send(format!(r#"{{"id":{},"result":{{}}}}"#, command["id"]).into())
                    .await
                    .unwrap();
                ws.close(None).await.unwrap();
                break;
            }
        }
    });
    let (browser, mut handler) = Browser::connect(format!("ws://{address}")).await.unwrap();
    browser.close().await.expect("Browser.close succeeds");
    let mut errors = Vec::new();
    loop {
        match tokio::time::timeout(Duration::from_secs(5), handler.next()).await {
            Ok(Some(Err(error))) => errors.push(error.to_string()),
            Ok(Some(Ok(()))) => {}
            Ok(None) => break,
            Err(_) => panic!("handler did not end after close"),
        }
    }
    assert!(
        errors.is_empty(),
        "clean close must not report handler errors: {errors:?}"
    );
    server.await.unwrap();
}

#[tokio::test]
async fn close_before_browser_close_response_reports_no_response() {
    use futures::StreamExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = async_tungstenite::tokio::accept_async(socket)
            .await
            .unwrap();
        while let Some(message) = ws.next().await {
            let command: Value = serde_json::from_str(message.unwrap().to_text().unwrap()).unwrap();
            if command["method"] == "Browser.close" {
                ws.close(None).await.unwrap();
                break;
            }
        }
    });
    let (browser, mut handler) = Browser::connect(format!("ws://{address}")).await.unwrap();
    let result = browser.close().await;
    assert!(result.is_err(), "unanswered Browser.close must fail");
    let mut saw_no_response = false;
    loop {
        match tokio::time::timeout(Duration::from_secs(5), handler.next()).await {
            Ok(Some(Err(CdpError::NoResponse))) => saw_no_response = true,
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => panic!("handler did not end after close"),
        }
    }
    assert!(
        saw_no_response,
        "close before the Browser.close response must report NoResponse"
    );
    server.await.unwrap();
    drop(browser);
}

#[tokio::test]
async fn response_without_result_fails_request() {
    let (inner, mut commands, _handler) = test_inner(false);
    let pending = tokio::spawn({
        let inner = Arc::clone(&inner);
        async move { inner.execute("Page.navigate", json!({}), None).await }
    });
    let command = next_command(&mut commands).await;
    let id = command["id"].as_u64().unwrap();
    let raw = format!(r#"{{"id":{id}}}"#);
    let value: Value = serde_json::from_str(&raw).unwrap();
    on_response(&inner, id, &value, &raw);
    match pending.await.unwrap() {
        Err(CdpError::InvalidMessage(payload, cause)) => {
            assert_eq!(payload, raw);
            assert_eq!(cause, "CDP response missing result object or error");
        }
        other => panic!("reply without result must fail, got {other:?}"),
    }
}

#[tokio::test]
async fn response_with_null_result_fails_request() {
    let (inner, mut commands, _handler) = test_inner(false);
    let pending = tokio::spawn({
        let inner = Arc::clone(&inner);
        async move { inner.execute("Page.navigate", json!({}), None).await }
    });
    let command = next_command(&mut commands).await;
    let id = command["id"].as_u64().unwrap();
    let raw = format!(r#"{{"id":{id},"result":null}}"#);
    let value: Value = serde_json::from_str(&raw).unwrap();
    on_response(&inner, id, &value, &raw);
    match pending.await.unwrap() {
        Err(CdpError::InvalidMessage(payload, cause)) => {
            assert_eq!(payload, raw);
            assert_eq!(cause, "CDP response missing result object or error");
        }
        other => panic!("reply with null result must fail, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_browser_close_response_does_not_arm_closing() {
    let (inner, mut commands, _handler) = test_inner(false);
    let pending = tokio::spawn({
        let inner = Arc::clone(&inner);
        async move { inner.execute("Browser.close", json!({}), None).await }
    });
    let command = next_command(&mut commands).await;
    let id = command["id"].as_u64().unwrap();
    let raw = format!(r#"{{"id":{id}}}"#);
    let value: Value = serde_json::from_str(&raw).unwrap();
    on_response(&inner, id, &value, &raw);
    assert!(
        pending.await.unwrap().is_err(),
        "malformed Browser.close reply must fail"
    );
    assert!(
        !inner.closing.load(Ordering::Acquire),
        "malformed reply must not arm closing"
    );
}

#[cfg(unix)]
#[test]
fn bare_name_resolution_skips_non_executable() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().unwrap();
    let plain = root.path().join("plain");
    let armed = root.path().join("armed");
    std::fs::create_dir(&plain).unwrap();
    std::fs::create_dir(&armed).unwrap();
    let decoy = plain.join("fake-chrome");
    let real = armed.join("fake-chrome");
    std::fs::write(&decoy, b"nope").unwrap();
    std::fs::write(&real, b"#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&decoy, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();

    // A non-executable first on the search path is skipped.
    assert_eq!(
        resolve_bare_name(
            std::path::Path::new("fake-chrome"),
            &[plain.clone(), armed.clone()]
        ),
        real
    );
    // The first executable wins once the decoy gains its bit.
    std::fs::set_permissions(&decoy, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        resolve_bare_name(std::path::Path::new("fake-chrome"), &[plain.clone(), armed]),
        decoy
    );
    // Unknown names fall back untouched.
    assert_eq!(
        resolve_bare_name(std::path::Path::new("missing-chrome"), &[plain]),
        std::path::PathBuf::from("missing-chrome")
    );
}

#[test]
fn directory_paths_are_never_resolved() {
    let dirs = vec![
        std::path::PathBuf::from("/bin"),
        std::path::PathBuf::from("/usr/bin"),
    ];
    for path in ["./missing", "bin/chrome", "/no/such", "/"] {
        assert_eq!(
            resolve_bare_name(std::path::Path::new(path), &dirs),
            std::path::PathBuf::from(path),
            "{path}"
        );
    }
    // The PATH-backed wrapper agrees without touching the environment.
    for path in ["./missing", "bin/chrome", "/no/such"] {
        assert_eq!(
            resolve_bare_name_on_path(std::path::Path::new(path)),
            std::path::PathBuf::from(path),
            "{path}"
        );
    }
    // Unknown bare names fall back untouched.
    assert_eq!(
        resolve_bare_name_on_path(std::path::Path::new("boardstudio-no-such-binary")),
        std::path::PathBuf::from("boardstudio-no-such-binary")
    );
}

#[tokio::test]
async fn canonicalize_resolves_absolute_paths() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("chrome");
    std::fs::write(&file, b"x").unwrap();
    let canonical = canonicalize_except_snap(file.clone()).await.unwrap();
    assert_eq!(canonical, tokio::fs::canonicalize(&file).await.unwrap());
}

#[test]
fn launch_args_match_the_previous_client() {
    let config = BrowserConfig::builder()
        .chrome_executable("/bin/chrome")
        .user_data_dir("/tmp/profile")
        .no_sandbox()
        .args(["--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        .args(["blink-settings=primaryPointerType=4"])
        .window_size(1513, 1043)
        .env("XDG_CONFIG_HOME", "/tmp/config")
        .build()
        .unwrap();
    let args = config.chrome_args();
    for expected in [
        "--remote-debugging-port=0",
        "--user-data-dir=/tmp/profile",
        "--no-sandbox",
        "--disable-setuid-sandbox",
        "--use-angle=swiftshader",
        "--enable-unsafe-swiftshader",
        "--blink-settings=primaryPointerType=4",
        "--window-size=1513,1043",
        "--headless",
        "--hide-scrollbars",
        "--mute-audio",
        "--disable-dev-shm-usage",
        "--disable-extensions",
    ] {
        assert!(
            args.contains(&expected.to_owned()),
            "{expected} in {args:?}"
        );
    }
}
