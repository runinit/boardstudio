//! Browser driving primitives used by the real-page check binaries.
//!
//! Every helper here exists because a current check script uses it: isolated
//! launch, role lookup, bounded waits, trusted input, evaluation, fatal error
//! capture and screenshots. Nothing more is added.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chromiumoxide::cdp::browser_protocol::accessibility::QueryAxTreeParams;
use chromiumoxide::cdp::browser_protocol::browser::BrowserContextId;
use chromiumoxide::cdp::browser_protocol::dom::{
    BackendNodeId, DescribeNodeParams, GetDocumentParams, QuerySelectorParams,
    ScrollIntoViewIfNeededParams,
};
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams, DispatchMouseEventType,
    MouseButton,
};
use chromiumoxide::cdp::browser_protocol::target::CreateTargetParams;
use chromiumoxide::cdp::browser_protocol::target::{
    CreateBrowserContextParams, DisposeBrowserContextParams, EventTargetDestroyed,
};
use chromiumoxide::cdp::js_protocol::runtime::{EvaluateParams, EventExceptionThrown};
use chromiumoxide::error::CdpError;
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::{Browser, BrowserConfig, Handler, Page};
use futures::StreamExt;
use serde_json::Value;

use crate::{Error, IsolatedDirs, Result};

/// Viewport every real-page check uses.
pub const VIEWPORT_WIDTH: u32 = 1513;
/// Viewport every real-page check uses.
pub const VIEWPORT_HEIGHT: u32 = 1043;

/// Modifier bit for the Control key in `Input.dispatch*` calls.
pub const CTRL_MODIFIER: i64 = 2;

/// Launch flags every real-page check uses.
pub const LAUNCH_ARGS: [&str; 2] = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"];

/// Launch Chrome with the shared flags and viewport under an isolated profile.
///
/// Invalid protocol messages are surfaced and fail the run. The child's
/// `XDG_CONFIG_HOME` is isolated from the user's configuration.
pub async fn launch_chrome(chrome: &str, dirs: &IsolatedDirs) -> Result<(Browser, Handler)> {
    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .user_data_dir(&dirs.profile_dir)
        .no_sandbox()
        .surface_invalid_messages()
        .args(LAUNCH_ARGS)
        .viewport(Viewport {
            width: VIEWPORT_WIDTH,
            height: VIEWPORT_HEIGHT,
            ..Default::default()
        })
        .window_size(VIEWPORT_WIDTH, VIEWPORT_HEIGHT)
        .env(
            "XDG_CONFIG_HOME",
            dirs.xdg_config_home().to_string_lossy().as_ref(),
        )
        .build()
        .map_err(|error| {
            Error(format!(
                "drag browser smoke cannot build a browser config: {error}"
            ))
        })?;
    // The vendored launch path uses ChromeMessage when child cleanup is unconfirmed.
    dirs.preserve.set(true);
    match Browser::launch(config).await {
        Ok(browser) => Ok(browser),
        Err(error) => {
            if !matches!(error, CdpError::ChromeMessage(_)) {
                dirs.preserve.set(false);
            } else {
                eprintln!(
                    "browser termination unconfirmed; preserving profile at {}",
                    dirs.profile_dir.display()
                );
            }
            Err(Error(format!(
                "drag browser smoke cannot start Chrome: {error}"
            )))
        }
    }
}

/// Handler errors observed while the run is active.
///
/// Protocol and command-routing failures, including invalid events, fail the run.
#[derive(Debug, Default)]
pub struct HandlerErrors {
    count: AtomicUsize,
    first: std::sync::Mutex<Vec<String>>,
}

impl HandlerErrors {
    /// No errors recorded yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of handler errors recorded so far.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    fn record(&self, message: String) {
        self.count.fetch_add(1, Ordering::Relaxed);
        let mut first = self
            .first
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        first.push(message);
    }

    /// Fail the run when any handler error was recorded.
    pub fn fail_if_any(&self) -> Result<()> {
        if self.count() == 0 {
            return Ok(());
        }
        let first = self
            .first
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Err(Error(format!(
            "drag browser smoke saw {} browser handler error(s): {}",
            self.count(),
            first.join("; ")
        )))
    }
}

/// Drain the browser handler for the duration of the run.
///
/// Returned errors are recorded with full redacted payloads; the task ends
/// when the caller drops the browser or aborts the handle.
pub fn spawn_handler_drain(
    handler: Handler,
    errors: Arc<HandlerErrors>,
    page_errors: Arc<PageErrors>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut handler = handler;
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                let message = match error {
                    CdpError::InvalidMessage(payload, cause) => {
                        format!(
                            "invalid protocol message: {cause}; payload: {}",
                            redact(&payload)
                        )
                    }
                    CdpError::Ws(error)
                        if format!("{error:?}") == "Protocol(ResetWithoutClosingHandshake)" =>
                    {
                        "WebSocket protocol reset without closing handshake".to_owned()
                    }
                    other => other.to_string(),
                };
                eprintln!("browser handler error: {message}");
                errors.record(message);
            }
        }
        let mut listeners = {
            let mut state = page_errors
                .listeners
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.0 = true;
            std::mem::take(&mut state.1)
        };
        while let Some(result) = listeners.join_next().await {
            if let Err(error) = result {
                errors.record(format!("page error listener failed: {error}"));
            }
        }
    })
}

/// Redact secret-bearing headers without truncating the protocol event.
fn redact(message: &str) -> String {
    match serde_json::from_str::<Value>(message) {
        Ok(mut value) => {
            scrub_json(&mut value);
            value.to_string()
        }
        Err(_) => "[unparseable protocol message redacted]".to_owned(),
    }
}

fn scrub_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let sensitive_header = map.get("name").and_then(Value::as_str).is_some_and(|name| {
                let name = name.to_lowercase();
                name.contains("cookie") || name.contains("authoriz")
            });
            if sensitive_header {
                if let Some(value) = map.get_mut("value") {
                    *value = Value::String("[redacted]".to_owned());
                }
            }
            for (key, entry) in map.iter_mut() {
                let lower = key.to_lowercase();
                if matches!(
                    lower.as_str(),
                    "headerstext"
                        | "postdata"
                        | "postdataentries"
                        | "cookie"
                        | "cookies"
                        | "cookieline"
                ) {
                    *entry = Value::String("[redacted]".to_owned());
                } else if (lower == "headers"
                    || lower == "responseheaders"
                    || lower == "requestheaders")
                    && entry.is_object()
                {
                    if let Value::Object(headers) = entry {
                        for (name, header) in headers.iter_mut() {
                            let header_name = name.to_lowercase();
                            if header_name.contains("cookie")
                                || header_name.contains("authoriz")
                                || header_name == "set-cookie"
                            {
                                *header = Value::String("[redacted]".to_owned());
                            }
                        }
                    }
                } else {
                    scrub_json(entry);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_json(item);
            }
        }
        _ => {}
    }
}

/// Page errors that must fail the run.
///
/// Collects `Runtime.exceptionThrown` events, which cover both uncaught page
/// exceptions and unhandled promise rejections.
#[derive(Debug, Default)]
pub struct PageErrors {
    messages: std::sync::Mutex<Vec<String>>,
    listeners: std::sync::Mutex<(bool, tokio::task::JoinSet<()>)>,
    completions:
        std::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Receiver<()>>>,
}

impl PageErrors {
    /// No errors collected yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the collected page error messages.
    pub fn messages(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Fail the run when any page error was collected.
    pub fn fail_if_any(&self) -> Result<()> {
        let messages = self.messages();
        if messages.is_empty() {
            return Ok(());
        }
        Err(Error(format!(
            "page raised {} uncaught browser error(s): {}",
            messages.len(),
            messages.join("; ")
        )))
    }
}

/// Subscribe to `Runtime.exceptionThrown` on `page` for the rest of the run.
pub async fn spawn_page_error_listener(page: &Page, errors: Arc<PageErrors>) -> Result<()> {
    let mut exceptions = page
        .event_listener::<EventExceptionThrown>()
        .await
        .map_err(|error| {
            Error(format!(
                "drag browser smoke cannot listen for page errors: {error}"
            ))
        })?;
    let mut listeners = errors
        .listeners
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if listeners.0 {
        return Err(Error(
            "cannot listen for page errors after handler termination".to_owned(),
        ));
    }
    let listener_errors = Arc::clone(&errors);
    let (complete, completion) = tokio::sync::oneshot::channel();
    errors
        .completions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(page.target_id().inner().clone(), completion);
    listeners.1.spawn(async move {
        while let Some(event) = exceptions.next().await {
            let detail = &event.exception_details;
            let text = detail
                .exception
                .as_ref()
                .and_then(|exception| exception.description.clone())
                .unwrap_or_else(|| detail.text.clone());
            let mut messages = listener_errors
                .messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            messages.push(text);
        }
        let _ = complete.send(());
    });
    Ok(())
}

/// Evaluate `expression` in the page and return its value by value.
///
/// Promises are awaited. A page-side exception is a fatal `Err`: evaluation
/// errors are never swallowed by waits.
pub async fn eval(page: &Page, expression: &str) -> Result<Value> {
    let params = EvaluateParams::builder()
        .expression(expression)
        .await_promise(true)
        .return_by_value(true)
        .build()
        .map_err(|error| {
            Error(format!(
                "drag browser smoke cannot build an evaluation: {error}"
            ))
        })?;
    let evaluation = page.evaluate_expression(params).await.map_err(|error| {
        Error(format!(
            "page evaluation failed for {expression:?}: {error}"
        ))
    })?;
    Ok(evaluation.value().cloned().unwrap_or(Value::Null))
}

/// Poll `expression` until it evaluates to `true`.
///
/// `what` names the condition for the timeout error. Evaluation failures
/// propagate immediately; only a `false` result keeps polling. The cadence
/// matches the old runner's request-animation-frame polling so transient
/// states (like an Undo revert) are observed, not stepped over.
pub async fn wait_for(page: &Page, expression: &str, what: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let operation = async {
        loop {
            if eval(page, expression).await? == Value::Bool(true) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    };
    tokio::time::timeout_at(deadline.into(), operation)
        .await
        .unwrap_or_else(|_| Err(Error(format!("timed out waiting for {what}: {expression}"))))
}

/// Nodes matching `role` with exactly `name`, optionally scoped to a subtree.
///
/// An empty `name` matches any accessible name (the caller filters in Rust).
pub async fn find_role(
    page: &Page,
    root: Option<BackendNodeId>,
    role: &str,
    name: &str,
) -> Result<Vec<(BackendNodeId, String)>> {
    let root = match root {
        Some(root) => root,
        None => {
            page.execute(GetDocumentParams::builder().depth(0).build())
                .await
                .map_err(|error| Error(format!("cannot read the page document: {error}")))?
                .result
                .root
                .backend_node_id
        }
    };
    let mut query = QueryAxTreeParams::builder()
        .backend_node_id(root)
        .role(role);
    if !name.is_empty() {
        query = query.accessible_name(name);
    }
    let nodes = page
        .execute(query.build())
        .await
        .map_err(|error| Error(format!("role lookup failed for {role} {name:?}: {error}")))?
        .result
        .nodes;
    Ok(nodes
        .into_iter()
        .filter(|node| !node.ignored)
        .filter_map(|node| {
            let label = node
                .name
                .as_ref()
                .and_then(|name| name.value.as_ref())
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_owned();
            if !name.is_empty() && label != name {
                return None;
            }
            node.backend_dom_node_id.map(|id| (id, label))
        })
        .collect())
}

/// Backend node id of the first match of a CSS selector.
///
/// A query with no match reports `node id 0`, which is a definitive "absent"
/// verdict, not a protocol failure. A describe that lands on a detached id
/// (`Could not find node with given id`) means the page re-rendered between
/// the query and the describe; the whole resolve-then-describe pair retries
/// within a short bound, mirroring Playwright's re-resolving actionability
/// checks, while a persistently missing node stays a lookup failure.
pub async fn backend_of(page: &Page, selector: &str) -> Result<BackendNodeId> {
    let mut last_error = String::new();
    for _ in 0..20 {
        let root = page
            .execute(GetDocumentParams::builder().depth(0).build())
            .await
            .map_err(|error| Error(format!("cannot read the page document: {error}")))?
            .result
            .root
            .node_id;
        let node = page
            .execute(QuerySelectorParams::new(root, selector))
            .await
            .map_err(|error| Error(format!("no match for selector {selector:?}: {error}")))?
            .result
            .node_id;
        if node.inner() == &0 {
            return Err(Error(format!("no match for selector {selector:?}")));
        }
        match page
            .execute(DescribeNodeParams::builder().node_id(node).build())
            .await
        {
            Ok(described) => return Ok(described.result.node.backend_node_id),
            Err(error) => {
                last_error = format!("cannot describe node for {selector:?}: {error}");
                if !last_error.contains("Could not find node with given id") {
                    return Err(Error(last_error));
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(Error(format!(
        "{last_error} (node kept detaching across retries)"
    )))
}

/// Backend node id of the first match of a CSS selector, waiting for it.
///
/// Polls [`backend_of`] while the selector has no match — the toolbar swaps
/// the stagger handle for the offset handle only after Control is held, so
/// the lookup must wait out that re-render like Playwright's `waitFor`. Any
/// other lookup failure propagates immediately; a still-missing selector at
/// the deadline is a lookup failure, not a timeout of the gesture.
pub async fn backend_of_wait(
    page: &Page,
    selector: &str,
    timeout: Duration,
) -> Result<BackendNodeId> {
    let deadline = Instant::now() + timeout;
    let operation = async {
        loop {
            match backend_of(page, selector).await {
                Ok(id) => return Ok(id),
                Err(error) if error.0 != format!("no match for selector {selector:?}") => {
                    return Err(error)
                }
                Err(_) => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout_at(deadline.into(), operation)
        .await
        .unwrap_or_else(|_| {
            Err(Error(format!(
                "no match for selector {selector:?} (timed out)"
            )))
        })
}

/// Center of a backend node in CSS pixels, scrolled into view.
pub async fn center(page: &Page, id: BackendNodeId) -> Result<(f64, f64)> {
    page.execute(
        ScrollIntoViewIfNeededParams::builder()
            .backend_node_id(id)
            .build(),
    )
    .await
    .map_err(|error| Error(format!("cannot scroll target into view: {error}")))?;
    let (x, y, width, height) = element_rect(page, id)
        .await
        .map_err(|error| Error(format!("target has no visible geometry: {error}")))?;
    if width <= 0.0 || height <= 0.0 {
        return Err(Error("target has no visible geometry".to_owned()));
    }
    Ok((x + width / 2.0, y + height / 2.0))
}

enum MouseError {
    Timeout,
    Other(Error),
}

impl From<MouseError> for Error {
    fn from(error: MouseError) -> Self {
        match error {
            MouseError::Timeout => {
                Error(format!("trusted mouse event failed: {}", CdpError::Timeout))
            }
            MouseError::Other(error) => error,
        }
    }
}

async fn dispatch_mouse(
    page: &Page,
    kind: DispatchMouseEventType,
    x: f64,
    y: f64,
    buttons: i64,
    modifiers: i64,
) -> std::result::Result<(), MouseError> {
    let pressed_or_released = matches!(
        kind,
        DispatchMouseEventType::MousePressed | DispatchMouseEventType::MouseReleased
    );
    // Pressure follows the reference runner exactly: a held button reports
    // 0.5 like a real mouse; an untouched hover reports none. The page can
    // observe the difference, so the default zero is not faithful.
    let force = match kind {
        DispatchMouseEventType::MousePressed => Some(0.5),
        DispatchMouseEventType::MouseMoved if buttons != 0 => Some(0.5),
        _ => None,
    };
    let mut params = DispatchMouseEventParams::builder()
        .r#type(kind)
        .x(x)
        .y(y)
        .modifiers(modifiers)
        .buttons(buttons)
        .button(if pressed_or_released || buttons != 0 {
            MouseButton::Left
        } else {
            MouseButton::None
        });
    if let Some(force) = force {
        params = params.force(force);
    }
    if pressed_or_released {
        params = params.click_count(1);
    }
    page.execute(params.build().map_err(|error| {
        MouseError::Other(Error(format!(
            "cannot build a trusted mouse event: {error}"
        )))
    })?)
    .await
    .map_err(|error| match error {
        CdpError::Timeout => MouseError::Timeout,
        other => MouseError::Other(Error(format!("trusted mouse event failed: {other}"))),
    })?;
    Ok(())
}

/// Resolve a backend node to a live JS object id.
async fn resolve_object(
    page: &Page,
    id: BackendNodeId,
) -> Result<chromiumoxide::cdp::js_protocol::runtime::RemoteObjectId> {
    use chromiumoxide::cdp::browser_protocol::dom::ResolveNodeParams;
    let object = page
        .execute(ResolveNodeParams::builder().backend_node_id(id).build())
        .await
        .map_err(|error| Error(format!("cannot resolve the target node: {error}")))?
        .result
        .object;
    object
        .object_id
        .ok_or_else(|| Error("target node has no live object".to_owned()))
}

/// In-page box of a backend node: the same `getBoundingClientRect` geometry a
/// real pointer observes. CDP box models can report zero areas for laid-out
/// controls; the rect cannot.
async fn element_rect(page: &Page, id: BackendNodeId) -> Result<(f64, f64, f64, f64)> {
    use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
    let verdict = page
        .execute(
            CallFunctionOnParams::builder()
                .function_declaration(
                    r#"function() {
  const rect = this.getBoundingClientRect();
  const style = getComputedStyle(this);
  return {
    x: rect.x, y: rect.y, width: rect.width, height: rect.height,
    shown: style.visibility !== 'hidden' && style.display !== 'none'
  };
}"#,
                )
                .object_id(resolve_object(page, id).await?)
                .return_by_value(true)
                .build()
                .map_err(|error| Error(format!("cannot build the geometry read: {error}")))?,
        )
        .await
        .map_err(|error| Error(format!("cannot read target geometry: {error}")))?
        .result
        .result
        .value
        .ok_or_else(|| Error("target geometry has no value".to_owned()))?;
    let number = |field: &str| verdict.get(field).and_then(Value::as_f64).unwrap_or(0.0);
    let shown = verdict
        .get("shown")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !shown {
        return Err(Error("target is not shown".to_owned()));
    }
    Ok((number("x"), number("y"), number("width"), number("height")))
}

/// Whether a backend node has a nonzero visible box.
///
/// Only a hidden or zero-area node reports false; resolution and protocol
/// failures propagate so a dead session cannot look like a hidden control.
pub async fn is_visible(page: &Page, id: BackendNodeId) -> Result<bool> {
    match element_rect(page, id).await {
        Ok((_, _, width, height)) => Ok(width > 0.0 && height > 0.0),
        Err(error) if error.0 == "target is not shown" => Ok(false),
        Err(error) => Err(error),
    }
}

/// Trusted click on the center of a backend node.
///
/// The node must be visible and receive pointer events at its center: a click
/// through a transient overlay is a missed interaction, not a passing one.
pub async fn click_backend(page: &Page, id: BackendNodeId) -> Result<()> {
    click_actionable(page, id, Duration::from_secs(10)).await
}

/// Click with actionability: visible, stable and hit-tested, retried until
/// `timeout`. Transient states (layout, overlays) retry; the last error is
/// reported when the bound expires.
pub async fn click_actionable(page: &Page, id: BackendNodeId, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last = None;
    let operation = async {
        loop {
            match try_click_once(page, id).await {
                Ok(center) => return Ok(center),
                Err(ClickError::Fatal(error)) => return Err(error),
                Err(ClickError::Transient(error)) => last = Some(error),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    match tokio::time::timeout_at(deadline.into(), operation).await {
        Ok(Ok((x, y))) => dispatch_click(page, x, y, deadline, timeout).await,
        Ok(Err(error)) => Err(error),
        Err(_) => Err(Error(format!(
            "timed out waiting for target actionability{}",
            last.map(|error| format!(": {error}")).unwrap_or_default()
        ))),
    }
}

enum ClickError {
    Transient(Error),
    Fatal(Error),
}

impl From<Error> for ClickError {
    fn from(error: Error) -> Self {
        Self::Fatal(error)
    }
}

async fn try_click_once(
    page: &Page,
    id: BackendNodeId,
) -> std::result::Result<(f64, f64), ClickError> {
    async fn actionable_center(
        page: &Page,
        id: BackendNodeId,
    ) -> std::result::Result<(f64, f64), ClickError> {
        center(page, id).await.map_err(|error| {
            if error.0 == "target has no visible geometry"
                || error.0 == "target has no visible geometry: target is not shown"
            {
                ClickError::Transient(error)
            } else {
                ClickError::Fatal(error)
            }
        })
    }
    let (x, y) = actionable_center(page, id).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (again_x, again_y) = actionable_center(page, id).await?;
    if (again_x - x).abs() > 1.0 || (again_y - y).abs() > 1.0 {
        return Err(ClickError::Transient(Error(
            "target is still moving".to_owned(),
        )));
    }
    let hit = receives_pointer_events(page, id).await?;
    if hit == "unreadable hit verdict" {
        return Err(ClickError::Fatal(Error(hit)));
    }
    if hit != "ok" {
        return Err(ClickError::Transient(Error(format!(
            "target does not receive pointer events yet ({hit})"
        ))));
    }
    if is_disabled_button(page, id).await? {
        return Err(ClickError::Transient(Error(
            "target is disabled".to_owned(),
        )));
    }
    Ok((x, y))
}

async fn dispatch_click(
    page: &Page,
    x: f64,
    y: f64,
    deadline: Instant,
    timeout: Duration,
) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(Error("timed out waiting for target actionability".into()));
    }
    let input = async {
        dispatch_mouse(page, DispatchMouseEventType::MouseMoved, x, y, 0, 0).await?;
        dispatch_mouse(page, DispatchMouseEventType::MousePressed, x, y, 1, 0).await?;
        dispatch_mouse(page, DispatchMouseEventType::MouseReleased, x, y, 0, 0).await
    };
    match tokio::time::timeout_at(deadline.into(), input).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(MouseError::Other(error))) => Err(error),
        Ok(Err(MouseError::Timeout)) | Err(_) => {
            use chromiumoxide::cdp::browser_protocol::target::CloseTargetParams;
            let close = tokio::time::timeout(
                timeout,
                page.execute(CloseTargetParams::new(page.target_id().clone())),
            )
            .await;
            let status = match close {
                Ok(Ok(_)) => "closed".to_owned(),
                Ok(Err(error)) => format!("close failed: {error}"),
                Err(_) => "close timed out".to_owned(),
            };
            Err(Error(format!(
                "ambiguous input outcome: input deadline expired; target termination: {status}"
            )))
        }
    }
}

/// Whether the topmost element at the node's center is the node itself or a
/// descendant of it. Resolves the backend node to a live object, so the test
/// cannot drift from the element the click will land on.
/// Whether the node is (inside) a disabled button. Non-button targets are
/// never disabled; a disabled control must not be clicked through.
async fn is_disabled_button(page: &Page, id: BackendNodeId) -> Result<bool> {
    use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
    let verdict = page
        .execute(
            CallFunctionOnParams::builder()
                .function_declaration(
                    r#"function() {
  const button = this.closest('button,[role="button"]');
  if (!button) return false;
  return button.disabled === true || button.getAttribute('aria-disabled') === 'true';
}"#,
                )
                .object_id(resolve_object(page, id).await?)
                .return_by_value(true)
                .build()
                .map_err(|error| Error(format!("cannot build the enabled check: {error}")))?,
        )
        .await
        .map_err(|error| Error(format!("the enabled check failed: {error}")))?
        .result
        .result
        .value;
    Ok(verdict == Some(Value::Bool(true)))
}

/// Hit-test verdict for a backend node: `ok` when the topmost element at the
/// node's center is the node itself or a descendant, otherwise a short
/// description of what covers it (or `void` for no hit, e.g. offscreen).
async fn receives_pointer_events(page: &Page, id: BackendNodeId) -> Result<String> {
    use chromiumoxide::cdp::js_protocol::runtime::CallFunctionOnParams;
    let object_id = resolve_object(page, id).await?;
    let verdict = page
        .execute(
            CallFunctionOnParams::builder()
                .function_declaration(
                    r#"function() {
  const rect = this.getBoundingClientRect();
  const x = rect.x + rect.width / 2, y = rect.y + rect.height / 2;
  const hit = document.elementFromPoint(x, y);
  if (hit === this || this.contains(hit)) return 'ok';
  if (!hit) return 'void at ' + Math.round(x) + ',' + Math.round(y)
    + ' viewport ' + window.innerWidth + 'x' + window.innerHeight;
  const label = hit.tagName + (hit.id ? '#' + hit.id : '')
    + (hit.className && hit.className.baseVal !== undefined ? '' : '.' + String(hit.className).split(' ')[0]);
  const text = (hit.textContent || '').trim().slice(0, 80);
  return 'covered by ' + label + ' saying ' + JSON.stringify(text);
}"#,
                )
                .object_id(object_id)
                .return_by_value(true)
                .build()
                .map_err(|error| Error(format!("cannot build the hit test: {error}")))?,
        )
        .await
        .map_err(|error| Error(format!("the hit test failed: {error}")))?
        .result
        .result
        .value;
    Ok(verdict
        .as_ref()
        .and_then(Value::as_str)
        .unwrap_or("unreadable hit verdict")
        .to_owned())
}

/// Trusted click on the first match of a CSS selector.
pub async fn click_css(page: &Page, selector: &str) -> Result<()> {
    let id = backend_of(page, selector).await?;
    click_backend(page, id).await
}

/// Fail fast when the app has panicked: the Dioxus panic toast covers the
/// toolbar with maximum z-index, so every later click would burn its whole
/// bound on a dead page. Surfacing the panic text beats a timeout elsewhere.
async fn check_app_panic(page: &Page) -> Result<()> {
    let verdict = eval(
        page,
        r#"(() => {
  const text = document.getElementById('__dx-toast-text');
  const toast = document.getElementById('__dx-toast');
  if (text && toast && toast.getAttribute('aria-hidden') !== 'true'
      && text.innerText.includes('panicked')) {
    return text.innerText;
  }
  return '';
})()"#,
    )
    .await?;
    if let Some(panic) = verdict.as_str().filter(|text| !text.is_empty()) {
        return Err(Error(format!("the app panicked: {panic}")));
    }
    Ok(())
}

/// Click the single `role`/`name` control, waiting for it to exist.
///
/// Fails on timeout, on ambiguity (more than one exact match is a caller bug
/// the smoke must surface, not pick from), and on missing visible geometry.
/// Every poll re-resolves the node: a control replaced mid-render keeps a
/// fresh id and rect instead of retrying a stale one to the deadline.
pub async fn click_role(
    page: &Page,
    root: Option<BackendNodeId>,
    role: &str,
    name: &str,
) -> Result<()> {
    click_role_timeout(page, root, role, name, Duration::from_secs(60)).await
}

/// [`click_role`] with an explicit bound, for fast negative probes.
pub async fn click_role_timeout(
    page: &Page,
    root: Option<BackendNodeId>,
    role: &str,
    name: &str,
    timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last = None;
    let operation = async {
        loop {
            check_app_panic(page).await?;
            let matches = find_role(page, root, role, name).await?;
            match matches.as_slice() {
                [single] => match try_click_once(page, single.0).await {
                    Ok(center) => return Ok(center),
                    Err(ClickError::Fatal(error)) => return Err(error),
                    Err(ClickError::Transient(error)) => last = Some(error),
                },
                [] => {}
                many => {
                    return Err(Error(format!(
                        "ambiguous role lookup: {role} {name:?} matched {} nodes",
                        many.len()
                    )))
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    match tokio::time::timeout_at(deadline.into(), operation).await {
        Ok(Ok((x, y))) => dispatch_click(page, x, y, deadline, timeout).await,
        Ok(Err(error)) => Err(error),
        Err(_) => Err(Error(format!(
            "timed out waiting for {role} {name:?}{}",
            last.map(|error| format!(": {error}")).unwrap_or_default()
        ))),
    }
}

/// Send a trusted key press (`keydown` + `keyup`) to the page.
pub async fn press_key(page: &Page, key: &str, code: &str, windows_key_code: i64) -> Result<()> {
    for kind in [
        DispatchKeyEventType::RawKeyDown,
        DispatchKeyEventType::KeyUp,
    ] {
        page.execute(
            DispatchKeyEventParams::builder()
                .r#type(kind)
                .key(key)
                .code(code)
                .windows_virtual_key_code(windows_key_code)
                .build()
                .map_err(|error| Error(format!("cannot build a trusted key event: {error}")))?,
        )
        .await
        .map_err(|error| Error(format!("trusted key event failed: {error}")))?;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(())
}

/// Hold (`RawKeyDown`) or release (`KeyUp`) the Control modifier.
pub async fn control(page: &Page, kind: DispatchKeyEventType) -> Result<()> {
    let modifiers = if matches!(kind, DispatchKeyEventType::RawKeyDown) {
        CTRL_MODIFIER
    } else {
        0
    };
    page.execute(
        DispatchKeyEventParams::builder()
            .r#type(kind)
            .key("Control")
            .code("ControlLeft")
            .windows_virtual_key_code(17)
            .modifiers(modifiers)
            .build()
            .map_err(|error| Error(format!("cannot build a trusted Control event: {error}")))?,
    )
    .await
    .map_err(|error| Error(format!("trusted Control event failed: {error}")))?;
    Ok(())
}

/// A drag gesture held down: the button is pressed and the pointer moved.
///
/// The smoke observes the live preview and pointer capture while the hold is
/// active, then calls [`end_drag`] exactly once. Every failure between begin
/// and end must still end the hold, or later input lands in a broken state.
pub struct DragHold {
    x: f64,
    y: f64,
    modifiers: i64,
    with_control: bool,
    released: bool,
}

/// Trusted mouse move/down/move of the first match of `selector`.
///
/// Moves in `steps` increments so the page observes real pointer movement.
/// The gesture stays held: the caller observes mid-drag state, then ends it.
///
/// With Control the toolbar swaps its handle label (stagger becomes offset),
/// so Control goes down first and the lookup waits for the swapped handle,
/// mirroring the old runner's `keyboard.down` before `waitFor`.
pub async fn begin_drag(
    page: &Page,
    selector: &str,
    dx: f64,
    dy: f64,
    with_control: bool,
    steps: u32,
) -> Result<DragHold> {
    if with_control {
        control(page, DispatchKeyEventType::RawKeyDown).await?;
    }
    let id = if with_control {
        backend_of_wait(page, selector, Duration::from_secs(10)).await?
    } else {
        backend_of(page, selector).await?
    };
    let (x, y) = center(page, id).await?;
    let modifiers = if with_control { CTRL_MODIFIER } else { 0 };
    let begin = async {
        dispatch_mouse(page, DispatchMouseEventType::MouseMoved, x, y, 0, modifiers).await?;
        dispatch_mouse(
            page,
            DispatchMouseEventType::MousePressed,
            x,
            y,
            1,
            modifiers,
        )
        .await?;
        for step in 1..=steps {
            let fraction = f64::from(step) / f64::from(steps);
            dispatch_mouse(
                page,
                DispatchMouseEventType::MouseMoved,
                x + dx * fraction,
                y + dy * fraction,
                1,
                modifiers,
            )
            .await?;
        }
        Ok::<(), Error>(())
    }
    .await;
    if let Err(error) = begin {
        // Never leak a held button: best-effort release before reporting.
        dispatch_mouse(
            page,
            DispatchMouseEventType::MouseReleased,
            x + dx,
            y + dy,
            0,
            modifiers,
        )
        .await
        .ok();
        if with_control {
            control(page, DispatchKeyEventType::KeyUp).await.ok();
        }
        return Err(error);
    }
    Ok(DragHold {
        x: x + dx,
        y: y + dy,
        modifiers,
        with_control,
        released: false,
    })
}

/// Release a held drag: trusted mouse up, then Control up when held.
///
/// Releasing twice is a caller bug and fails loudly.
pub async fn end_drag(page: &Page, hold: &mut DragHold) -> Result<()> {
    if hold.released {
        return Err(Error("drag hold released twice".to_owned()));
    }
    hold.released = true;
    dispatch_mouse(
        page,
        DispatchMouseEventType::MouseReleased,
        hold.x,
        hold.y,
        0,
        hold.modifiers,
    )
    .await?;
    if hold.with_control {
        control(page, DispatchKeyEventType::KeyUp).await?;
    }
    Ok(())
}

/// Open a fresh page in its own browser context (isolated storage).
///
/// The caller must dispose the context when the scenario finishes; an
/// incognito context never shares IndexedDB with any other run.
pub async fn new_isolated_page(
    browser: &Browser,
    url: &str,
    errors: &Arc<PageErrors>,
) -> Result<(BrowserContextId, Page)> {
    let context = browser
        .create_browser_context(CreateBrowserContextParams::default())
        .await
        .map_err(|error| {
            Error(format!(
                "cannot create an isolated browser context: {error}"
            ))
        })?;
    let page = browser
        .new_page(
            CreateTargetParams::builder()
                .url(url)
                .browser_context_id(context.clone())
                .build()
                .map_err(|error| Error(format!("cannot build a page target: {error}")))?,
        )
        .await
        .map_err(|error| Error(format!("cannot open an isolated page: {error}")))?;
    spawn_page_error_listener(&page, Arc::clone(errors)).await?;
    Ok((context, page))
}

/// Close an observed page and dispose its context after its listener drains.
pub async fn dispose_isolated_page(
    browser: &Browser,
    context: Option<BrowserContextId>,
    page: Page,
    errors: &PageErrors,
) -> Result<()> {
    let target = page.target_id().clone();
    let boundary = async {
        let mut markers = browser
            .event_listener::<EventTargetDestroyed>()
            .await
            .map_err(|error| Error(format!("cannot listen for target destruction: {error}")))?;
        page.close()
            .await
            .map_err(|error| Error(format!("cannot close target {target:?}: {error}")))?;
        loop {
            match markers.next().await {
                Some(marker) if marker.target_id == target => break,
                Some(_) => {}
                None => {
                    return Err(Error(format!(
                        "transport ended before target {target:?} destruction"
                    )))
                }
            }
        }
        let completion = errors
            .completions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(target.inner())
            .ok_or_else(|| Error(format!("missing listener for target {target:?}")))?;
        completion.await.map_err(|error| {
            Error(format!(
                "page error listener for target {target:?} failed: {error}"
            ))
        })?;
        Ok(())
    };
    let boundary = tokio::time::timeout(Duration::from_secs(60), boundary)
        .await
        .unwrap_or_else(|_| {
            Err(Error(format!(
                "target {target:?} boundary timed out after 60s"
            )))
        });
    let disposal = if let Some(context) = context {
        tokio::time::timeout(
            Duration::from_secs(60),
            browser.execute(DisposeBrowserContextParams::new(context)),
        )
        .await
        .map_err(|_| Error("browser context disposal timed out after 60s".to_owned()))
        .and_then(|result| {
            result
                .map(|_| ())
                .map_err(|error| Error(format!("cannot dispose a browser context: {error}")))
        })
    } else {
        Ok(())
    };
    match (boundary, disposal) {
        (Err(boundary), Err(disposal)) => Err(Error(format!("{boundary}; {disposal}"))),
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod redaction_tests {
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
        let (mut chrome, handler) =
            super::launch_chrome(&std::env::var("CHROME_BIN").unwrap(), &dirs)
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
            let (browser, handler) = chromiumoxide::Browser::connect(format!("ws://{address}"))
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
                super::click_role_timeout(&page, None, "button", "Probe", Duration::from_secs(2))
                    .await
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
        let (mut chrome, handler) =
            super::launch_chrome(&std::env::var("CHROME_BIN").unwrap(), &dirs)
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
            let (browser, handler) = chromiumoxide::Browser::connect(format!("ws://{address}"))
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
                super::click_role_timeout(&page, None, "button", "Probe", Duration::from_secs(2))
                    .await
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
            eprintln!("{kind} role={role}: attempts={count}, responses={responses}, result={result:?}, closed={closed}");
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
        let config = chromiumoxide::BrowserConfig::builder()
            .chrome_executable(&script)
            .launch_timeout(std::time::Duration::from_millis(100))
            .build()
            .unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            chromiumoxide::Browser::launch(config),
        )
        .await;
        assert!(
            matches!(
                result,
                Ok(Err(chromiumoxide::error::CdpError::LaunchTimeout(_)))
            ),
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
        let (mut browser, handler) = super::launch_chrome(script.to_str().unwrap(), &dirs)
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
        let (mut browser, handler) = chromiumoxide::Browser::connect(format!("ws://{address}"))
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
        let config = chromiumoxide::handler::HandlerConfig {
            ignore_invalid_messages: false,
            ..Default::default()
        };
        let (mut browser, handler) =
            chromiumoxide::Browser::connect_with_config(format!("ws://{address}"), config)
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
            verdict.contains("invalid protocol message")
                && verdict.contains("Runtime.exceptionThrown"),
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
}
