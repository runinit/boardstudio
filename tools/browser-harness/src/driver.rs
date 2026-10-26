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
    CreateBrowserContextParams, DisposeBrowserContextParams,
};
use chromiumoxide::cdp::js_protocol::runtime::{EvaluateParams, EventExceptionThrown};
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
/// Uses the default invalid-message handling: unsolicited experimental events
/// degrade to a warning plus single-event loss, exactly as Playwright tolerates
/// protocol additions. Command and subscription failures stay fatal at their
/// call sites. The child's `XDG_CONFIG_HOME` is overridden so a broken user
/// config can never crash the launch.
pub async fn launch_chrome(chrome: &str, dirs: &IsolatedDirs) -> Result<(Browser, Handler)> {
    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .user_data_dir(&dirs.profile_dir)
        .no_sandbox()
        // NOTE: `surface_invalid_messages` was tried here and reverted:
        // Chrome 156 emits ~19 messages per run that chromiumoxide 0.9's
        // snapshot schema cannot parse (benign library/browser skew), which
        // would fail every run. Joint library+browser pinning at cutover is
        // the real fix; until then every required protocol interaction has
        // a timeout or assertion that fails loudly on actual loss, and the
        // skew stays a documented cutover blocker, not silent proof.
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
    Browser::launch(config)
        .await
        .map_err(|error| Error(format!("drag browser smoke cannot start Chrome: {error}")))
}

/// Handler errors observed while the run is active.
///
/// Any error here is a protocol or command-routing failure, never an expected
/// event: the default config already absorbs unsolicited experimental events.
/// A nonzero count fails the run at teardown.
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
        if first.len() < 5 {
            first.push(message);
        }
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
            "drag browser smoke saw {} browser handler error(s); first: {}",
            self.count(),
            first.first().cloned().unwrap_or_default()
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
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut handler = handler;
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                errors.record(redact(&error.to_string()));
            }
        }
    })
}

/// Redact secret-bearing headers from a raw protocol message, then cap its
/// length so one event cannot flood the run log.
fn redact(message: &str) -> String {
    const LIMIT: usize = 8192;
    let scrubbed = match serde_json::from_str::<Value>(message) {
        Ok(mut value) => {
            scrub_json(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| message.to_owned())
        }
        Err(_) => message.to_owned(),
    };
    if scrubbed.len() > LIMIT {
        format!(
            "{}…[truncated {} bytes]",
            &scrubbed[..LIMIT],
            scrubbed.len() - LIMIT
        )
    } else {
        scrubbed
    }
}

fn scrub_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, entry) in map.iter_mut() {
                let lower = key.to_lowercase();
                if (lower == "headers" || lower == "responseheaders" || lower == "requestheaders")
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
            "page raised {} uncaught browser error(s); first: {}",
            messages.len(),
            messages.first().cloned().unwrap_or_default()
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
    tokio::spawn(async move {
        while let Some(event) = exceptions.next().await {
            let detail = &event.exception_details;
            let text = detail
                .exception
                .as_ref()
                .and_then(|exception| exception.description.clone())
                .unwrap_or_else(|| detail.text.clone());
            let mut messages = errors
                .messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            messages.push(text);
        }
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
    loop {
        if eval(page, expression).await? == Value::Bool(true) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error(format!("timed out waiting for {what}: {expression}")));
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
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
    loop {
        match backend_of(page, selector).await {
            Ok(id) => return Ok(id),
            Err(error) => {
                if !error.0.starts_with("no match for selector") || Instant::now() >= deadline {
                    return Err(error);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
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

async fn dispatch_mouse(
    page: &Page,
    kind: DispatchMouseEventType,
    x: f64,
    y: f64,
    buttons: i64,
    modifiers: i64,
) -> Result<()> {
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
    page.execute(
        params
            .build()
            .map_err(|error| Error(format!("cannot build a trusted mouse event: {error}")))?,
    )
    .await
    .map_err(|error| Error(format!("trusted mouse event failed: {error}")))?;
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
    loop {
        match try_click_once(page, id).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                if Instant::now() >= deadline {
                    return Err(error);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn try_click_once(page: &Page, id: BackendNodeId) -> Result<()> {
    let (x, y) = center(page, id).await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let (again_x, again_y) = center(page, id).await?;
    if (again_x - x).abs() > 1.0 || (again_y - y).abs() > 1.0 {
        return Err(Error("target is still moving".to_owned()));
    }
    let hit = receives_pointer_events(page, id).await?;
    if hit != "ok" {
        return Err(Error(format!(
            "target does not receive pointer events yet ({hit})"
        )));
    }
    if is_disabled_button(page, id).await? {
        return Err(Error("target is disabled".to_owned()));
    }
    dispatch_mouse(page, DispatchMouseEventType::MouseMoved, x, y, 0, 0).await?;
    dispatch_mouse(page, DispatchMouseEventType::MousePressed, x, y, 1, 0).await?;
    dispatch_mouse(page, DispatchMouseEventType::MouseReleased, x, y, 0, 0).await
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
    loop {
        check_app_panic(page).await?;
        let matches = find_role(page, root, role, name).await?;
        match matches.as_slice() {
            [single] => {
                // A found control may not be clickable yet; keep polling for
                // actionability inside the same bound instead of failing fast.
                match try_click_once(page, single.0).await {
                    Ok(()) => return Ok(()),
                    Err(error) => {
                        if Instant::now() >= deadline {
                            return Err(error);
                        }
                    }
                }
            }
            [] => {
                if Instant::now() >= deadline {
                    return Err(Error(format!("timed out waiting for {role} {name:?}")));
                }
            }
            many => {
                return Err(Error(format!(
                    "ambiguous role lookup: {role} {name:?} matched {} nodes",
                    many.len()
                )));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
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

/// Dispose a scenario context created by [`new_isolated_page`].
pub async fn dispose_isolated_page(browser: &Browser, context: BrowserContextId) -> Result<()> {
    browser
        .execute(DisposeBrowserContextParams::new(context))
        .await
        .map_err(|error| Error(format!("cannot dispose a browser context: {error}")))?;
    Ok(())
}
