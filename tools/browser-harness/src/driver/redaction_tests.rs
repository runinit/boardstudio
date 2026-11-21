#[tokio::test]
async fn additional_args_reject_harness_switches_and_allow_blink_settings() {
    let dirs = crate::IsolatedDirs::create(std::path::Path::new("/tmp")).unwrap();
    for arg in [
        "--user-data-dir=/tmp/other",
        "user-data-dir=/tmp/other",
        "--user-data-dir",
        "--remote-debugging-port=9222",
        "--window-size=1,1",
        "--no-sandbox",
        "--use-angle=gl",
        "--",
        "--blink-settings",
        "----user-data-dir=/tmp/other",
        "--blink-settings=x --user-data-dir=/tmp/other",
    ] {
        let error = match super::launch_chrome_with_args("/bin/false", &dirs, &[arg]).await {
            Err(error) => error.to_string(),
            Ok(_) => panic!("unexpected browser launch"),
        };
        assert!(
            error.contains("unsupported additional Chrome argument"),
            "{arg}: {error}"
        );
    }
    let error = match super::launch_chrome_with_args(
        "/bin/false",
        &dirs,
        &["--blink-settings=primaryPointerType=4"],
    )
    .await
    {
        Err(error) => error.to_string(),
        Ok(_) => panic!("unexpected browser launch"),
    };
    assert!(
        !error.contains("unsupported additional Chrome argument"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires CHROME_BIN; run explicitly for the real CDP failure control"]
async fn dispatch_failure_is_not_retried_or_masked() {
    use futures::StreamExt;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    let dirs = crate::IsolatedDirs::create(std::path::Path::new("/tmp")).unwrap();
    let (chrome, handler) = super::launch_chrome(&std::env::var("CHROME_BIN").unwrap(), &dirs)
        .await
        .unwrap();
    let drain = super::spawn_handler_drain(
        handler,
        Arc::new(super::HandlerErrors::new()),
        Arc::new(super::PageErrors::new()),
    );
    let mut outcomes = Vec::new();
    for (kind, role) in [
        ("mousePressed", false),
        ("mouseReleased", false),
        ("mousePressed", true),
        ("mouseReleased", true),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = chrome.websocket_address().clone();
        let attempts = Arc::new(AtomicUsize::new(0));
        let count = attempts.clone();
        let relay = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut client = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            let (mut server, _) = async_tungstenite::tokio::connect_async(upstream)
                .await
                .unwrap();
            loop {
                tokio::select! {
                    message = client.next() => {
                        let Some(Ok(message)) = message else { break };
                        if let Ok(command) = serde_json::from_str::<serde_json::Value>(message.to_text().unwrap_or("")) {
                            if command["method"] == "Input.dispatchMouseEvent" && command["params"]["type"] == kind
                                && count.fetch_add(1, Ordering::SeqCst) == 0 {
                                let response = serde_json::json!({"id": command["id"], "sessionId": command["sessionId"], "error": {"code": -32000, "message": "controlled dispatch failure"}});
                                client.send(response.to_string().into()).await.unwrap();
                                continue;
                            }
                        }
                        if server.send(message).await.is_err() { break; }
                    }
                    message = server.next() => {
                        let Some(Ok(message)) = message else { break };
                        if client.send(message).await.is_err() { break; }
                    }
                }
            }
        });
        let (browser, handler) = crate::cdp::Browser::connect(format!("ws://{address}"))
            .await
            .unwrap();
        let proxy_drain = super::spawn_handler_drain(
            handler,
            Arc::new(super::HandlerErrors::new()),
            Arc::new(super::PageErrors::new()),
        );
        let page = browser.new_page("about:blank").await.unwrap();
        super::eval(&page, "document.body.innerHTML = '<button>Probe</button>'; window.clicks = 0; document.querySelector('button').onclick = () => window.clicks++;").await.unwrap();
        let result = if role {
            super::click_role_timeout(&page, None, "button", "Probe", Duration::from_secs(2)).await
        } else {
            let id = super::backend_of(&page, "button").await.unwrap();
            super::click_actionable(&page, id, Duration::from_secs(2)).await
        };
        let clicks = super::eval(&page, "window.clicks").await.unwrap();
        outcomes.push((
            kind,
            role,
            attempts.load(Ordering::SeqCst),
            result.map_err(|error| error.to_string()),
            clicks,
        ));
        drop(page);
        drop(browser);
        proxy_drain.abort();
        relay.abort();
    }
    chrome.close().await.unwrap();
    chrome.wait().await.unwrap().unwrap();
    drop(chrome);
    drain.await.unwrap();
    dirs.preserve.set(false);
    for (kind, role, count, result, clicks) in &outcomes {
        eprintln!("{kind} role={role}: attempts={count}, result={result:?}, clicks={clicks}");
    }
    for (kind, role, count, result, clicks) in outcomes {
        assert_eq!(count, 1, "{kind} role={role} retried dispatch");
        assert!(
            matches!(result, Err(ref message) if message.contains("trusted mouse event failed") && message.contains("controlled dispatch failure")),
            "{result:?}"
        );
        assert_eq!(clicks, 0, "failed dispatch produced a click receipt");
    }
}

#[tokio::test]
#[ignore = "requires CHROME_BIN; run explicitly for the real CDP failure control"]
async fn inflight_dispatch_timeout_closes_target() {
    use futures::StreamExt;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    let dirs = crate::IsolatedDirs::create(std::path::Path::new("/tmp")).unwrap();
    let (chrome, handler) = super::launch_chrome(&std::env::var("CHROME_BIN").unwrap(), &dirs)
        .await
        .unwrap();
    let drain = super::spawn_handler_drain(
        handler,
        Arc::new(super::HandlerErrors::new()),
        Arc::new(super::PageErrors::new()),
    );
    let mut outcomes = Vec::new();
    for (kind, role) in [
        ("mousePressed", false),
        ("mouseReleased", false),
        ("mousePressed", true),
        ("mouseReleased", true),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = chrome.websocket_address().clone();
        let attempts = Arc::new(AtomicUsize::new(0));
        let count = attempts.clone();
        let responses = Arc::new(AtomicUsize::new(0));
        let response_count = responses.clone();
        let relay = tokio::spawn(async move {
            let mut withheld = None;
            let (socket, _) = listener.accept().await.unwrap();
            let mut client = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            let (mut server, _) = async_tungstenite::tokio::connect_async(upstream)
                .await
                .unwrap();
            loop {
                tokio::select! {
                    message = client.next() => {
                        let Some(Ok(message)) = message else { break };
                        if let Ok(command) = serde_json::from_str::<serde_json::Value>(message.to_text().unwrap_or("")) {
                            if command["method"] == "Input.dispatchMouseEvent" && command["params"]["type"] == kind
                                && count.fetch_add(1, Ordering::SeqCst) == 0 {
                                withheld = Some(command["id"].clone());
                            }
                        }
                        if server.send(message).await.is_err() { break; }
                    }
                    message = server.next() => {
                        let Some(Ok(message)) = message else { break };
                        if let Ok(response) = serde_json::from_str::<serde_json::Value>(message.to_text().unwrap_or("")) {
                            if withheld.as_ref() == Some(&response["id"]) {
                                response_count.fetch_add(1, Ordering::SeqCst);
                                continue;
                            }
                        }
                        if client.send(message).await.is_err() { break; }
                    }
                }
            }
        });
        let (browser, handler) = crate::cdp::Browser::connect(format!("ws://{address}"))
            .await
            .unwrap();
        let proxy_drain = super::spawn_handler_drain(
            handler,
            Arc::new(super::HandlerErrors::new()),
            Arc::new(super::PageErrors::new()),
        );
        let page = browser.new_page("about:blank").await.unwrap();
        super::eval(&page, "document.body.innerHTML = '<button>Probe</button>'; window.clicks = 0; document.querySelector('button').onclick = () => window.clicks++;").await.unwrap();
        let result = if role {
            super::click_role_timeout(&page, None, "button", "Probe", Duration::from_secs(2)).await
        } else {
            let id = super::backend_of(&page, "button").await.unwrap();
            super::click_actionable(&page, id, Duration::from_secs(2)).await
        };
        let closed = super::eval(&page, "window.clicks").await.is_err();
        outcomes.push((
            kind,
            role,
            attempts.load(Ordering::SeqCst),
            responses.load(Ordering::SeqCst),
            result.map_err(|error| error.to_string()),
            closed,
        ));
        drop(page);
        drop(browser);
        proxy_drain.abort();
        relay.abort();
    }
    chrome.close().await.unwrap();
    chrome.wait().await.unwrap().unwrap();
    drop(chrome);
    drain.await.unwrap();
    dirs.preserve.set(false);
    for (kind, role, count, responses, result, closed) in &outcomes {
        eprintln!(
            "{kind} role={role}: attempts={count}, responses={responses}, result={result:?}, closed={closed}"
        );
    }
    for (kind, role, count, responses, result, closed) in outcomes {
        assert_eq!(count, 1, "{kind} role={role} retried dispatch");
        assert_eq!(responses, 1, "{kind} role={role} did not reach Chrome");
        assert!(
            matches!(result, Err(ref message) if message.contains("ambiguous input outcome")),
            "{result:?}"
        );
        assert!(closed, "ambiguous input left the target usable");
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn failed_initialization_reaps_spawned_child() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("bs-launch-control-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("browser");
    std::fs::write(&script, "#!/bin/sh\necho $$ > \"$XDG_CONFIG_HOME/pid\"\necho 'DevTools listening on ws://127.0.0.1:1/devtools/browser/test' >&2\nexec /usr/bin/tail -f /dev/null\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let dirs = crate::IsolatedDirs::create(&root).unwrap();
    let result = super::launch_chrome(script.to_str().unwrap(), &dirs).await;
    assert!(result.is_err());
    let pid = std::fs::read_to_string(dirs.xdg_config_home().join("pid")).unwrap();
    assert!(
        !std::path::Path::new(&format!("/proc/{}", pid.trim())).exists(),
        "child must be reaped"
    );
    let profile = dirs.profile_dir.clone();
    drop(dirs);
    assert!(
        !profile.exists(),
        "confirmed launch cleanup removes the profile"
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(target_os = "linux")]
#[tokio::test]
async fn stalled_handshake_times_out_and_reaps_child() {
    use std::os::unix::fs::PermissionsExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
    });
    let root = std::env::temp_dir().join(format!("bs-stall-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("browser");
    std::fs::write(&script, format!("#!/bin/sh\necho $$ > {}/pid\necho 'DevTools listening on ws://{address}/devtools/browser/test' >&2\nexec /usr/bin/tail -f /dev/null\n", root.display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = crate::cdp::BrowserConfig::builder()
        .chrome_executable(&script)
        .launch_timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        crate::cdp::Browser::launch(config),
    )
    .await;
    assert!(
        matches!(result, Ok(Err(crate::cdp::CdpError::LaunchTimeout(_)))),
        "launch must time out during handshake"
    );
    let pid = std::fs::read_to_string(root.join("pid")).unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{}", pid.trim())).exists());
    server.abort();
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cancellation_after_launch_preserves_profile() {
    use std::os::unix::fs::PermissionsExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let _ws = async_tungstenite::tokio::accept_async(socket)
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let root = std::env::temp_dir().join(format!("bs-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("browser");
    std::fs::write(&script, format!("#!/bin/sh\necho $$ > {}/pid\necho 'DevTools listening on ws://{address}/devtools/browser/test' >&2\nexec /usr/bin/tail -f /dev/null\n", root.display())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let dirs = crate::IsolatedDirs::create(&root).unwrap();
    let profile = dirs.profile_dir.clone();
    let (browser, handler) = super::launch_chrome(script.to_str().unwrap(), &dirs)
        .await
        .unwrap();
    drop(handler);
    drop(dirs);
    let preserved = profile.exists();
    browser.kill().await.unwrap().unwrap();
    server.abort();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        preserved,
        "profile must survive cancellation before child termination"
    );
}

#[tokio::test]
async fn transport_loss_fails_consumer_verdict() {
    use futures::StreamExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = async_tungstenite::tokio::accept_async(socket)
            .await
            .unwrap();
        while let Some(message) = ws.next().await {
            let command: serde_json::Value =
                serde_json::from_str(message.unwrap().to_text().unwrap()).unwrap();
            if command["method"] == "Browser.close" {
                ws.send(format!(r#"{{"id":{},"result":{{}}}}"#, command["id"]).into())
                    .await
                    .unwrap();
                break;
            }
        }
        drop(ws);
    });
    let (browser, handler) = crate::cdp::Browser::connect(format!("ws://{address}"))
        .await
        .unwrap();
    let handler_errors = std::sync::Arc::new(super::HandlerErrors::new());
    let page_errors = std::sync::Arc::new(super::PageErrors::new());
    let drain = super::spawn_handler_drain(handler, handler_errors.clone(), page_errors);
    browser.close().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), drain)
        .await
        .unwrap()
        .unwrap();
    let verdict = handler_errors.fail_if_any().unwrap_err().to_string();
    assert!(
        verdict.contains("WebSocket protocol reset without closing handshake"),
        "{verdict}"
    );
    server.await.unwrap();
    drop(browser);
}

#[tokio::test]
async fn late_invalid_event_fails_consumer_after_close_response() {
    use futures::StreamExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = async_tungstenite::tokio::accept_async(socket)
            .await
            .unwrap();
        while let Some(message) = ws.next().await {
            let message = message.unwrap();
            let command: serde_json::Value =
                serde_json::from_str(message.to_text().unwrap()).unwrap();
            if command["method"] == "Browser.close" {
                ws.send(format!(r#"{{"id":{},"result":{{}}}}"#, command["id"]).into())
                    .await
                    .unwrap();
                ws.send(r#"{"method":"Runtime.exceptionThrown","params":{}}"#.into())
                    .await
                    .unwrap();
                ws.close(None).await.unwrap();
                break;
            }
        }
    });
    let (browser, handler) = crate::cdp::Browser::connect(format!("ws://{address}"))
        .await
        .unwrap();
    let handler_errors = std::sync::Arc::new(super::HandlerErrors::new());
    let page_errors = std::sync::Arc::new(super::PageErrors::new());
    let drain = super::spawn_handler_drain(handler, handler_errors.clone(), page_errors);
    browser.close().await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), drain)
        .await
        .unwrap()
        .unwrap();
    let verdict = handler_errors.fail_if_any().unwrap_err().to_string();
    assert!(
        verdict.contains("invalid protocol message") && verdict.contains("Runtime.exceptionThrown"),
        "{verdict}"
    );
    server.await.unwrap();
}

#[test]
fn redacts_nested_header_arrays() {
    let input = r#"{"params":{"nested":[{"headers":[{"name":"Authorization","value":"bearer-secret"},{"name":"Set-Cookie","value":"cookie-secret"},{"name":"Accept","value":"safe"}]}]}}"#;
    let output = super::redact(input);
    assert!(!output.contains("bearer-secret"));
    assert!(!output.contains("cookie-secret"));
    assert!(output.contains("safe"));
}

#[test]
fn reports_all_page_errors() {
    let errors = super::PageErrors::new();
    errors
        .messages
        .lock()
        .unwrap()
        .extend(["first".into(), "second".into()]);
    let message = errors.fail_if_any().unwrap_err().to_string();
    assert!(message.contains("first") && message.contains("second"));
}

#[test]
fn redacts_request_body_entries_and_cookie_lines() {
    let input = r#"{"params":{"postDataEntries":[{"bytes":"body-secret"}],"cookieLine":"cookie-secret","url":"safe"}}"#;
    let output = super::redact(input);
    assert!(!output.contains("body-secret"));
    assert!(!output.contains("cookie-secret"));
    assert!(output.contains("safe"));
}
