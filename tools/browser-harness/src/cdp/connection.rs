use super::*;

pub(super) fn connected(
    config: BrowserConfig,
    url: String,
    ws: WebSocketStream<ConnectStream>,
    child: Option<tokio::process::Child>,
) -> (Browser, Handler) {
    let (commands, command_receiver) = mpsc::unbounded_channel();
    let (handler_sender, handler_receiver) = mpsc::unbounded_channel();
    let inner = Arc::new(Inner {
        commands,
        pending: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(0),
        child: AsyncMutex::new(child),
        ws_url: url,
        subscriptions: Mutex::new(Vec::new()),
        load_waiters: Mutex::new(HashMap::new()),
        capture_workers: AtomicBool::new(false),
        closing: AtomicBool::new(false),
        surface_invalid: config.surface_invalid,
        request_timeout: config.request_timeout,
        viewport: config.viewport,
        handler_errors: Mutex::new(Some(handler_sender)),
    });
    // Queue discovery before the task starts: a caller command must never
    // overtake it, or a server that stops reading at `Browser.close` would
    // reset the connection with our discovery still unread.
    inner.send_untracked("Target.setDiscoverTargets", json!({"discover": true}), None);
    let task_inner = Arc::clone(&inner);
    tokio::spawn(async move { run_connection(task_inner, ws, command_receiver).await });
    (
        Browser {
            inner: inner.clone(),
        },
        Handler {
            errors: handler_receiver,
            inner,
        },
    )
}

pub(super) async fn canonicalize_except_snap(
    path: std::path::PathBuf,
) -> std::io::Result<std::path::PathBuf> {
    let resolved = resolve_bare_name_on_path(&path);
    let canonical = tokio::fs::canonicalize(&resolved).await?;
    if canonical
        .to_str()
        .is_some_and(|path| path.ends_with("/snap"))
    {
        Ok(resolved)
    } else {
        Ok(canonical)
    }
}

/// Resolve a bare executable name against `PATH`, leaving every other path
/// untouched. A name found nowhere falls back to itself so `canonicalize`
/// still reports `NotFound`.
fn resolve_bare_name_on_path(path: &std::path::Path) -> std::path::PathBuf {
    let Some(paths) = std::env::var_os("PATH") else {
        return path.to_path_buf();
    };
    let dirs: Vec<std::path::PathBuf> = std::env::split_paths(&paths).collect();
    resolve_bare_name(path, &dirs)
}

/// Resolve a bare executable name (`"google-chrome"`) to the first executable
/// file of that name on `search_dirs`. Paths with directory components
/// (`"./foo"`, `"bin/chrome"`, `"/usr/bin/chromium"`) are returned untouched,
/// as are names found nowhere.
fn resolve_bare_name(
    path: &std::path::Path,
    search_dirs: &[std::path::PathBuf],
) -> std::path::PathBuf {
    if path.components().count() != 1 || path.is_absolute() {
        return path.to_path_buf();
    }
    for dir in search_dirs {
        let candidate = dir.join(path);
        if candidate.is_file() && is_executable_file(&candidate) {
            return candidate;
        }
    }
    path.to_path_buf()
}

/// True when `candidate` is runnable: on Unix it needs an execute bit.
#[cfg(unix)]
fn is_executable_file(candidate: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(candidate)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// True when `candidate` is runnable: without execute bits, being a file is
/// enough.
#[cfg(not(unix))]
fn is_executable_file(_candidate: &std::path::Path) -> bool {
    true
}

pub(super) async fn kill_and_wait(child: &mut tokio::process::Child) -> std::io::Result<()> {
    child.kill().await?;
    child.wait().await.map(|_| ())
}

/// Resolve the DevTools WebSocket URL from the browser's stderr before
/// `deadline`, or fail when the child exits first.
pub(super) async fn ws_url_from_output(
    child: &mut tokio::process::Child,
    deadline: Instant,
) -> Result<String, CdpError> {
    use tokio::io::AsyncBufReadExt;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CdpError::LaunchIo("browser stderr unavailable".to_owned()))?;
    let mut reader = tokio::io::BufReader::new(stderr);
    let mut collected = Vec::<u8>::new();
    let mut line = Vec::<u8>::new();
    loop {
        line.clear();
        tokio::select! {
            _ = tokio::time::sleep_until(deadline.into()) => {
                return Err(CdpError::LaunchTimeout(format!(
                    "launch timed out before the DevTools URL appeared: {}",
                    String::from_utf8_lossy(&collected)
                )));
            }
            status = child.wait() => {
                return Err(match status {
                    Ok(status) => CdpError::LaunchExit(format!(
                        "browser exited with {status} before the DevTools URL appeared: {}",
                        String::from_utf8_lossy(&collected)
                    )),
                    Err(error) => CdpError::LaunchIo(format!(
                        "waiting for the browser child: {error}"
                    )),
                });
            }
            read = reader.read_until(b'\n', &mut line) => {
                match read {
                    Err(error) => {
                        return Err(CdpError::LaunchIo(format!(
                            "reading browser stderr: {error}"
                        )));
                    }
                    Ok(0) => {
                        return Err(CdpError::LaunchIo(format!(
                            "browser stderr ended before the DevTools URL appeared: {}",
                            String::from_utf8_lossy(&collected)
                        )));
                    }
                    Ok(_) => {
                        collected.extend_from_slice(&line);
                        let text = std::str::from_utf8(&line).map_err(|_| {
                            CdpError::LaunchIo(
                                "browser stderr was not valid UTF-8".to_owned(),
                            )
                        })?;
                        if let Some((_, ws)) = text.rsplit_once("listening on ") {
                            let ws = ws.trim();
                            if ws.starts_with("ws") && ws.contains("devtools/browser") {
                                return Ok(ws.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) async fn connect_ws(
    url: &str,
    deadline: Instant,
) -> Result<WebSocketStream<ConnectStream>, CdpError> {
    let config = WebSocketConfig::default()
        .max_message_size(None)
        .max_frame_size(None);
    match tokio::time::timeout_at(
        deadline.into(),
        async_tungstenite::tokio::connect_async_with_config(url, Some(config)),
    )
    .await
    {
        Ok(Ok((ws, _))) => Ok(ws),
        Ok(Err(error)) => Err(CdpError::Ws(error)),
        Err(_) => Err(CdpError::LaunchTimeout(format!(
            "WebSocket handshake with {url} exceeded the launch timeout"
        ))),
    }
}

/// Connection-local routing state: flattened sessions to their targets.
#[derive(Debug, Default)]
struct ConnState {
    sessions: HashMap<String, String>,
    worker_owners: HashMap<String, String>,
    worker_parents: HashMap<String, String>,
}

fn is_network_method(method: &str) -> bool {
    matches!(
        method,
        "Network.requestWillBeSent"
            | "Network.responseReceived"
            | "Network.loadingFinished"
            | "Network.loadingFailed"
    )
}

/// The custom method worker network observations are forwarded as.
pub const NETWORK_EVENT_METHOD: &str = "BoardStudio.NetworkEvent";

/// Required fields of the events the harness parses strictly. Anything else
/// passes through untouched.
fn validate_event(method: &str, params: &Value) -> Result<(), String> {
    match method {
        "Runtime.exceptionThrown" => serde_json::from_value::<EventExceptionThrown>(params.clone())
            .map(|_| ())
            .map_err(|error| format!("Runtime.exceptionThrown invalid: {error}")),
        "Target.targetDestroyed" => serde_json::from_value::<EventTargetDestroyed>(params.clone())
            .map(|_| ())
            .map_err(|error| format!("Target.targetDestroyed invalid: {error}")),
        "Browser.downloadWillBegin" => {
            serde_json::from_value::<EventDownloadWillBegin>(params.clone())
                .map(|_| ())
                .map_err(|error| format!("Browser.downloadWillBegin invalid: {error}"))
        }
        "Browser.downloadProgress" => {
            serde_json::from_value::<EventDownloadProgress>(params.clone())
                .map(|_| ())
                .map_err(|error| format!("Browser.downloadProgress invalid: {error}"))
        }
        "Target.attachedToTarget" => {
            if params.get("sessionId").and_then(Value::as_str).is_none() {
                return Err("Target.attachedToTarget missing sessionId".to_owned());
            }
            if params
                .get("targetInfo")
                .and_then(|info| info.get("targetId"))
                .and_then(Value::as_str)
                .is_none()
            {
                return Err("Target.attachedToTarget missing targetInfo.targetId".to_owned());
            }
            Ok(())
        }
        "Target.detachedFromTarget" => {
            if params.get("sessionId").and_then(Value::as_str).is_some() {
                Ok(())
            } else {
                Err("Target.detachedFromTarget missing sessionId".to_owned())
            }
        }
        _ => Ok(()),
    }
}

async fn run_connection(
    inner: Arc<Inner>,
    ws: WebSocketStream<ConnectStream>,
    mut commands: mpsc::UnboundedReceiver<String>,
) {
    let (mut sink, mut stream) = ws.split();
    let mut state = ConnState::default();
    let mut evict = tokio::time::interval(Duration::from_secs(5));
    evict.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                if let Err(error) = sink.send(WsMessage::Text(command.into())).await {
                    inner.push_handler_error(CdpError::Ws(error));
                    break;
                }
            }
            message = stream.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => on_text(&inner, &mut state, text.as_str()),
                    Some(Ok(WsMessage::Close(_))) | None => {
                        if !inner.closing.load(Ordering::Acquire) {
                            inner.push_handler_error(CdpError::NoResponse);
                        }
                        break;
                    }
                    Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {}
                    Some(Ok(_)) => {
                        inner.push_handler_error(CdpError::ChromeMessage(
                            "unexpected binary websocket message".to_owned(),
                        ));
                        break;
                    }
                    Some(Err(error)) => {
                        inner.push_handler_error(CdpError::Ws(error));
                        break;
                    }
                }
            }
            _ = evict.tick() => evict_timed_out(&inner),
        }
    }
    let pending = std::mem::take(
        &mut *inner
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    for (_, entry) in pending {
        let _ = entry.sender.send(Err(CdpError::ChromeMessage(
            "CDP transport closed before a response arrived".to_owned(),
        )));
    }
    inner
        .subscriptions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    inner
        .load_waiters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    inner
        .handler_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
}

fn evict_timed_out(inner: &Inner) {
    let now = Instant::now();
    let mut pending = inner
        .pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let expired: Vec<u64> = pending
        .iter()
        .filter(|(_, entry)| now > entry.started + inner.request_timeout)
        .map(|(id, _)| *id)
        .collect();
    for id in expired {
        if let Some(entry) = pending.remove(&id) {
            let _ = entry.sender.send(Err(CdpError::Timeout));
        }
    }
}

fn on_text(inner: &Arc<Inner>, state: &mut ConnState, raw: &str) {
    let value: Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            inner.push_invalid(raw, error.to_string());
            return;
        }
    };
    if let Some(id) = value.get("id").and_then(Value::as_u64) {
        on_response(inner, id, &value, raw);
        return;
    }
    let Some(method) = value.get("method").and_then(Value::as_str) else {
        inner.push_invalid(raw, "message has neither id nor method".to_owned());
        return;
    };
    let session = value.get("sessionId").and_then(Value::as_str);
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    if let Err(cause) = validate_event(method, &params) {
        inner.push_invalid(raw, cause);
        return;
    }
    on_event(inner, state, method, session, &params, raw);
}

fn on_response(inner: &Inner, id: u64, value: &Value, raw: &str) {
    let entry = inner
        .pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&id);
    if let Some(entry) = entry {
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let _ = entry.sender.send(Err(CdpError::ChromeMessage(format!(
                "CDP error {code}: {message}"
            ))));
        } else if let Some(result) = value.get("result").filter(|result| result.is_object()) {
            if entry.is_browser_close {
                inner.closing.store(true, Ordering::Release);
            }
            let _ = entry.sender.send(Ok(result.clone()));
        } else {
            let _ = entry.sender.send(Err(CdpError::InvalidMessage(
                raw.to_owned(),
                "CDP response missing result object or error".to_owned(),
            )));
        }
    }
}

fn on_event(
    inner: &Arc<Inner>,
    state: &mut ConnState,
    method: &str,
    session: Option<&str>,
    params: &Value,
    raw: &str,
) {
    match method {
        "Target.attachedToTarget" => on_attached(inner, state, session, params, raw),
        "Target.detachedFromTarget" => {
            if let Some(detached) = params.get("sessionId").and_then(Value::as_str) {
                let target = state.sessions.remove(detached);
                state.worker_owners.remove(detached);
                state.worker_parents.remove(detached);
                let orphans: Vec<String> = state
                    .worker_parents
                    .iter()
                    .filter(|(_, parent)| parent.as_str() == detached)
                    .map(|(child, _)| child.clone())
                    .collect();
                for orphan in orphans {
                    state.worker_owners.remove(&orphan);
                    state.worker_parents.remove(&orphan);
                }
                if let Some(target) = target {
                    inner.remove_target_subscriptions(&target);
                }
                inner.drop_load_waiters(detached);
            }
            inner.publish(None, method, params.clone(), Some(raw));
        }
        "Target.targetDestroyed" => {
            if let Some(destroyed) = params.get("targetId").and_then(Value::as_str) {
                let owned: Vec<String> = state
                    .worker_owners
                    .iter()
                    .filter(|(_, owner)| owner.as_str() == destroyed)
                    .map(|(session, _)| session.clone())
                    .collect();
                for session in owned {
                    state.worker_owners.remove(&session);
                    state.worker_parents.remove(&session);
                }
                inner.remove_target_subscriptions(destroyed);
            }
            inner.publish(None, method, params.clone(), Some(raw));
        }
        "Page.loadEventFired" => {
            if let Some(session) = session {
                inner.wake_load_waiters(session);
                if let Some(target) = state.sessions.get(session) {
                    inner.publish(Some(target), method, params.clone(), Some(raw));
                }
            }
        }
        _ => {
            if inner.capture_workers.load(Ordering::Relaxed) && is_network_method(method) {
                if let Some(session) = session {
                    let owner = state
                        .worker_owners
                        .get(session)
                        .cloned()
                        .or_else(|| state.sessions.get(session).cloned());
                    if let Some(owner) = owner {
                        inner.publish(
                            Some(&owner),
                            NETWORK_EVENT_METHOD,
                            json!({"session": session, "method": method, "params": params}),
                            None,
                        );
                    }
                }
                if session.is_some_and(|session| state.worker_owners.contains_key(session)) {
                    return;
                }
            } else if session.is_some_and(|session| state.worker_owners.contains_key(session)) {
                return;
            }
            match session {
                None => inner.publish(None, method, params.clone(), Some(raw)),
                Some(session) => {
                    if let Some(target) = state.sessions.get(session) {
                        inner.publish(Some(target), method, params.clone(), Some(raw));
                    }
                }
            }
        }
    }
}

fn on_attached(
    inner: &Arc<Inner>,
    state: &mut ConnState,
    session: Option<&str>,
    params: &Value,
    raw: &str,
) {
    let Some(child) = params.get("sessionId").and_then(Value::as_str) else {
        return;
    };
    let Some(target) = params
        .get("targetInfo")
        .and_then(|info| info.get("targetId"))
        .and_then(Value::as_str)
    else {
        return;
    };
    state.sessions.insert(child.to_owned(), target.to_owned());
    let Some(parent) = session else {
        inner.publish(None, "Target.attachedToTarget", params.clone(), Some(raw));
        return;
    };
    let kind = params
        .get("targetInfo")
        .and_then(|info| info.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let waiting = params
        .get("waitingForDebugger")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if kind != "worker" && kind != "service_worker" {
        // `Page::initialize` auto-attaches every child target paused, including
        // out-of-process iframes: resume them without touching Network.
        if waiting {
            inner.spawn_internal(
                "Runtime.runIfWaitingForDebugger",
                json!({}),
                Some(child.to_owned()),
            );
        }
        if let Some(owner) = state.sessions.get(parent) {
            inner.publish(
                Some(owner),
                "Target.attachedToTarget",
                params.clone(),
                Some(raw),
            );
        }
        return;
    }
    if let Some(owner) = state.sessions.get(parent).cloned() {
        state.worker_owners.insert(child.to_owned(), owner.clone());
        state
            .worker_parents
            .insert(child.to_owned(), parent.to_owned());
    }
    let capture = inner.capture_workers.load(Ordering::Relaxed);
    inner.spawn_worker_setup(
        child.to_owned(),
        parent.to_owned(),
        kind == "service_worker",
        capture,
        waiting,
    );
    if capture {
        if let Some(owner) = state.sessions.get(parent) {
            inner.publish(
                Some(owner),
                NETWORK_EVENT_METHOD,
                json!({"session": parent, "method": "Target.attachedToTarget", "params": params}),
                None,
            );
        }
    }
}

#[cfg(test)]
#[path = "connection_tests.rs"]
mod tests;
