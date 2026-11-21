use super::*;

/// Evaluate `expression` in the page and return its value by value.
///
/// Promises are awaited. A page-side exception is a fatal `Err`: evaluation
/// errors are never swallowed by waits.
pub async fn eval(page: &Page, expression: &str) -> Result<Value> {
    let result = page
        .execute(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "awaitPromise": true,
                "returnByValue": true,
            }),
        )
        .await
        .map_err(|error| {
            Error(format!(
                "page evaluation failed for {expression:?}: {error}"
            ))
        })?;
    if let Some(details) = result.get("exceptionDetails") {
        let text = details
            .get("exception")
            .and_then(|exception| exception.get("description"))
            .and_then(Value::as_str)
            .or_else(|| details.get("text").and_then(Value::as_str))
            .unwrap_or("unknown evaluation failure");
        return Err(Error(format!(
            "page evaluation failed for {expression:?}: {text}"
        )));
    }
    Ok(result
        .get("result")
        .and_then(|result| result.get("value"))
        .cloned()
        .unwrap_or(Value::Null))
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
        None => page
            .execute("DOM.getDocument", json!({"depth": 0}))
            .await
            .map_err(|error| Error(format!("cannot read the page document: {error}")))?
            .get("root")
            .and_then(|root| root.get("backendNodeId"))
            .and_then(Value::as_i64)
            .ok_or_else(|| Error("page document has no root backend node".to_owned()))?,
    };
    let mut params = json!({"backendNodeId": root, "role": role});
    if !name.is_empty() {
        params["accessibleName"] = json!(name);
    }
    let nodes = page
        .execute("Accessibility.queryAXTree", params)
        .await
        .map_err(|error| Error(format!("role lookup failed for {role} {name:?}: {error}")))?
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(nodes
        .into_iter()
        .filter(|node| node.get("ignored").and_then(Value::as_bool) != Some(true))
        .filter_map(|node| {
            let label = node
                .get("name")
                .and_then(|name| name.get("value"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if !name.is_empty() && label != name {
                return None;
            }
            node.get("backendDOMNodeId")
                .and_then(Value::as_i64)
                .map(|id| (id, label))
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
            .execute("DOM.getDocument", json!({"depth": 0}))
            .await
            .map_err(|error| Error(format!("cannot read the page document: {error}")))?
            .get("root")
            .and_then(|root| root.get("nodeId"))
            .and_then(Value::as_i64)
            .ok_or_else(|| Error("page document has no root node".to_owned()))?;
        let node = page
            .execute(
                "DOM.querySelector",
                json!({"nodeId": root, "selector": selector}),
            )
            .await
            .map_err(|error| Error(format!("no match for selector {selector:?}: {error}")))?
            .get("nodeId")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if node == 0 {
            return Err(Error(format!("no match for selector {selector:?}")));
        }
        match page
            .execute("DOM.describeNode", json!({"nodeId": node}))
            .await
        {
            Ok(described) => {
                return described
                    .get("node")
                    .and_then(|node| node.get("backendNodeId"))
                    .and_then(Value::as_i64)
                    .ok_or_else(|| {
                        Error(format!(
                            "cannot describe node for {selector:?}: missing backend id"
                        ))
                    })
            }
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
                    return Err(error);
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
    page.execute("DOM.scrollIntoViewIfNeeded", json!({"backendNodeId": id}))
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
    let mut params = json!({
        "type": kind.as_str(),
        "x": x,
        "y": y,
        "modifiers": modifiers,
        "buttons": buttons,
        "button": if pressed_or_released || buttons != 0 {
            MouseButton::Left.as_str()
        } else {
            MouseButton::None.as_str()
        },
    });
    if let Some(force) = force {
        params["force"] = json!(force);
    }
    if pressed_or_released {
        params["clickCount"] = json!(1);
    }
    page.execute("Input.dispatchMouseEvent", params)
        .await
        .map_err(|error| match error {
            CdpError::Timeout => MouseError::Timeout,
            other => MouseError::Other(Error(format!("trusted mouse event failed: {other}"))),
        })?;
    Ok(())
}

/// Resolve a backend node to a live JS object id.
async fn resolve_object(page: &Page, id: BackendNodeId) -> Result<RemoteObjectId> {
    page.execute("DOM.resolveNode", json!({"backendNodeId": id}))
        .await
        .map_err(|error| Error(format!("cannot resolve the target node: {error}")))?
        .get("object")
        .and_then(|object| object.get("objectId"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error("target node has no live object".to_owned()))
}

/// Call `function` on a live object and return its value by value, `Null`
/// when the call produced no value.
async fn call_function_on(
    page: &Page,
    object_id: &str,
    function: &str,
    what: &str,
) -> Result<Value> {
    Ok(page
        .execute(
            "Runtime.callFunctionOn",
            json!({
                "functionDeclaration": function,
                "objectId": object_id,
                "returnByValue": true,
            }),
        )
        .await
        .map_err(|error| Error(format!("{what}: {error}")))?
        .get("result")
        .and_then(|result| result.get("value"))
        .cloned()
        .unwrap_or(Value::Null))
}

/// In-page box of a backend node: the same `getBoundingClientRect` geometry a
/// real pointer observes. CDP box models can report zero areas for laid-out
/// controls; the rect cannot.
async fn element_rect(page: &Page, id: BackendNodeId) -> Result<(f64, f64, f64, f64)> {
    let verdict = call_function_on(
        page,
        &resolve_object(page, id).await?,
        r#"function() {
  const rect = this.getBoundingClientRect();
  const style = getComputedStyle(this);
  return {
    x: rect.x, y: rect.y, width: rect.width, height: rect.height,
    shown: style.visibility !== 'hidden' && style.display !== 'none'
  };
}"#,
        "cannot read target geometry",
    )
    .await?;
    if verdict == Value::Null {
        return Err(Error("target geometry has no value".to_owned()));
    }
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
            let close = tokio::time::timeout(
                timeout,
                page.execute(
                    "Target.closeTarget",
                    json!({"targetId": page.target_id().inner()}),
                ),
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
    let verdict = call_function_on(
        page,
        &resolve_object(page, id).await?,
        r#"function() {
  const button = this.closest('button,[role="button"]');
  if (!button) return false;
  return button.disabled === true || button.getAttribute('aria-disabled') === 'true';
}"#,
        "the enabled check failed",
    )
    .await?;
    Ok(verdict == Value::Bool(true))
}

/// Hit-test verdict for a backend node: `ok` when the topmost element at the
/// node's center is the node itself or a descendant, otherwise a short
/// description of what covers it (or `void` for no hit, e.g. offscreen).
async fn receives_pointer_events(page: &Page, id: BackendNodeId) -> Result<String> {
    let object_id = resolve_object(page, id).await?;
    let verdict = call_function_on(
        page,
        &object_id,
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
        "the hit test failed",
    )
    .await?;
    Ok(verdict
        .as_str()
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

/// Create from the landing page only after its Import control reports storage ready.
pub async fn create_new_keyboard(page: &Page) -> Result<()> {
    let readiness = async {
        loop {
            check_app_panic(page).await?;
            let failure = eval(
                page,
                r#"[...document.querySelectorAll('[role="alert"]')]
  .map(node => node.textContent.trim())
  .find(text => text.startsWith('Storage could not be initialized safely.')) || ''"#,
            )
            .await?;
            if let Some(failure) = failure.as_str().filter(|text| !text.is_empty()) {
                return Err(Error(failure.to_owned()));
            }
            match find_role(page, None, "button", "Import project")
                .await?
                .as_slice()
            {
                [button]
                    if is_visible(page, button.0).await?
                        && !is_disabled_button(page, button.0).await? =>
                {
                    return Ok(())
                }
                [] | [_] => {}
                many => {
                    return Err(Error(format!(
                        "ambiguous role lookup: button \"Import project\" matched {} nodes",
                        many.len()
                    )))
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(60), readiness)
        .await
        .map_err(|_| Error("timed out waiting for project storage to initialize".into()))??;
    click_role(page, None, "button", "Create new keyboard").await
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
                    )));
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
            "Input.dispatchKeyEvent",
            json!({
                "type": kind.as_str(),
                "key": key,
                "code": code,
                "windowsVirtualKeyCode": windows_key_code,
            }),
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
        "Input.dispatchKeyEvent",
        json!({
            "type": kind.as_str(),
            "key": "Control",
            "code": "ControlLeft",
            "windowsVirtualKeyCode": 17,
            "modifiers": modifiers,
        }),
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
