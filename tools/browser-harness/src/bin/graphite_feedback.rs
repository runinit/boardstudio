//! Manual Graphite feedback acceptance against an already running site.
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::{
    driver::{self, HandlerErrors, PageErrors},
    Error, IsolatedDirs, Result,
};
use chromiumoxide::{
    cdp::browser_protocol::{
        emulation::{MediaFeature, SetDeviceMetricsOverrideParams, SetEmulatedMediaParams},
        input::{
            DispatchKeyEventParams, DispatchKeyEventType, DispatchMouseEventParams,
            DispatchMouseEventType, MouseButton,
        },
    },
    Page,
};
use serde_json::Value;
use std::{path::Path, sync::Arc, time::Duration};

const WAIT: Duration = Duration::from_secs(30);
fn err(e: impl std::fmt::Display) -> Error {
    Error(e.to_string())
}
async fn wait(p: &Page, expr: &str) -> Result<()> {
    driver::wait_for(p, expr, expr, WAIT).await
}
async fn click(p: &Page, name: &str) -> Result<()> {
    driver::click_role(p, None, "button", name).await
}
async fn css(p: &Page, selector: &str) -> Result<()> {
    let id = driver::backend_of_wait(p, selector, WAIT).await?;
    driver::click_actionable(p, id, WAIT).await
}
async fn eval(p: &Page, expr: &str) -> Result<Value> {
    driver::eval(p, expr).await
}
async fn visible(p: &Page, selector: &str) -> Result<bool> {
    let selector_json = serde_json::to_string(selector).map_err(err)?;
    if eval(p, &format!("!!document.querySelector({selector_json})")).await? != Value::Bool(true) {
        return Ok(false);
    }
    driver::is_visible(p, driver::backend_of(p, selector).await?).await
}
async fn wait_visible(p: &Page, selector: &str, timeout: Duration) -> Result<()> {
    tokio::time::timeout(timeout, async {
        loop {
            if visible(p, selector).await? {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| err(format!("timed out waiting for visible {selector}")))?
}
async fn check(p: &Page, failures: &mut Vec<String>, label: &str, expr: &str) -> Result<()> {
    let passed = eval(p, expr).await? == Value::Bool(true);
    println!("{} {label}", if passed { "PASS" } else { "FAIL" });
    if !passed {
        failures.push(label.into());
    }
    Ok(())
}
fn passed(label: &str) {
    println!("PASS {label}");
}
async fn ready(p: &Page) -> Result<()> {
    driver::wait_for(p, "(() => { const t=document.querySelector('#__dx-toast'); if(!t || t.getAttribute('aria-hidden')==='true') return true; const r=(t.querySelector('.dx-toast-inner')||t).getBoundingClientRect(); return !r.width || r.right<=0 || r.x>=innerWidth })()", "toast to clear", Duration::from_secs(180)).await
}
async fn nav(p: &Page, name: &str) -> Result<()> {
    ready(p).await?;
    click(p, &format!("Open {name} workspace")).await
}
async fn scope(p: &Page, name: &str) -> Result<()> {
    let direct = driver::find_role(p, None, "button", &format!("Select {name}")).await?;
    if let [node] = direct.as_slice() {
        if driver::is_visible(p, node.0).await? {
            return driver::click_actionable(p, node.0, WAIT).await;
        }
    }
    css(p, "#m1-layout-select-trigger").await?;
    let root = driver::backend_of(p, "#m1-layout-select-menu").await?;
    driver::click_role(p, Some(root), "button", name).await
}
async fn set(p: &Page, label: &str, value: &str) -> Result<()> {
    let label = serde_json::to_string(label).map_err(err)?;
    let value = serde_json::to_string(value).map_err(err)?;
    let selector = format!("select[aria-label={label}]");
    let id = driver::backend_of_wait(p, &selector, WAIT).await?;
    if !driver::is_visible(p, id).await? {
        return Err(err("select is not visible"));
    }
    eval(p, &format!(r#"(() => {{ const e=document.querySelector('select[aria-label='+CSS.escape({label})+']'); if(!e) throw Error('missing field '+{label}); if(e.matches(':disabled')) throw Error('disabled field '+{label}); const setter=Object.getOwnPropertyDescriptor(e.tagName==='SELECT'?HTMLSelectElement.prototype:HTMLInputElement.prototype,'value').set; setter.call(e,{value}); if(e.value !== {value}) throw Error('invalid value for '+{label}); e.dispatchEvent(new Event('input',{{bubbles:true}})); e.dispatchEvent(new Event('change',{{bubbles:true}})); return true }})()"#)).await?;
    Ok(())
}
async fn press_key(p: &Page, key: &str, code: &str, virtual_key: i64, text: &str) -> Result<()> {
    for kind in [DispatchKeyEventType::KeyDown, DispatchKeyEventType::KeyUp] {
        let mut event = DispatchKeyEventParams::builder()
            .r#type(kind.clone())
            .key(key)
            .code(code)
            .windows_virtual_key_code(virtual_key);
        if matches!(kind, DispatchKeyEventType::KeyDown) {
            event = event.text(text).unmodified_text(text);
        }
        p.execute(event.build().map_err(err)?).await.map_err(err)?;
    }
    Ok(())
}
async fn press_enter(p: &Page) -> Result<()> {
    press_key(p, "Enter", "Enter", 13, "\r").await
}
async fn drag_offset(
    p: &Page,
    dy: f64,
    cancel: bool,
    observe_preview: bool,
) -> Result<Option<Value>> {
    let id = driver::backend_of_wait(p, "[aria-label=\"Drag to offset column\"]", WAIT).await?;
    let (x, y) = driver::center(p, id).await?;
    if !cancel {
        eval(p, "window.__graphiteFinalDown = null; document.addEventListener('pointerdown', e => { window.__graphiteFinalDown = { id: e.pointerId, trusted: e.isTrusted, target: !!e.target.closest('[aria-label=\"Drag to offset column\"]'), actual: e.target.outerHTML.slice(0, 300) }; }, { once: true, capture: true }); true").await?;
    }
    if cancel {
        eval(p, "document.addEventListener('pointerdown', e => { window.__graphitePointerId = e.pointerId; window.__graphiteMoved = false; document.addEventListener('pointermove', move => { window.__graphiteMoved = move.pointerId === e.pointerId && (move.clientX !== e.clientX || move.clientY !== e.clientY); }, { once: true, capture: true }); }, { once: true, capture: true }); true").await?;
    }
    for (kind, dx, dy, buttons) in
        std::iter::once((DispatchMouseEventType::MouseMoved, 0.0, 0.0, 0))
            .chain(std::iter::once((
                DispatchMouseEventType::MousePressed,
                0.0,
                0.0,
                1,
            )))
            .chain((1..=5).map(|i| {
                (
                    DispatchMouseEventType::MouseMoved,
                    30.0 * i as f64 / 5.0,
                    dy * i as f64 / 5.0,
                    1,
                )
            }))
    {
        let pressed = matches!(kind, DispatchMouseEventType::MousePressed);
        let mut params = DispatchMouseEventParams::builder()
            .r#type(kind)
            .x(x + dx)
            .y(y + dy)
            .modifiers(2)
            .buttons(buttons)
            .button(if buttons == 0 {
                MouseButton::None
            } else {
                MouseButton::Left
            });
        if buttons != 0 {
            params = params.force(0.5);
        }
        if pressed {
            params = params.click_count(1);
        }
        p.execute(params.build().map_err(err)?).await.map_err(err)?;
    }
    if cancel {
        if observe_preview {
            wait(p, "(() => { const r=document.querySelector('.m1-keycap-overlay.is-selected')?.getBoundingClientRect(), b=window.__graphiteInitial; return r && Math.hypot(r.x-b.x,r.y-b.y)>2 })()").await?;
        }
        let captured = eval(p, "window.__graphiteMoved === true && [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')].some(e => e.hasPointerCapture(window.__graphitePointerId))").await? == Value::Bool(true);
        if !captured {
            return Err(err(
                "Ctrl offset Escape cancellation blocked: pointer capture or movement not observed",
            ));
        }
        for kind in [
            DispatchKeyEventType::RawKeyDown,
            DispatchKeyEventType::KeyUp,
        ] {
            p.execute(
                DispatchKeyEventParams::builder()
                    .r#type(kind)
                    .key("Escape")
                    .code("Escape")
                    .windows_virtual_key_code(27)
                    .modifiers(2)
                    .build()
                    .map_err(err)?,
            )
            .await
            .map_err(err)?;
        }
    }
    let final_probe = if cancel {
        None
    } else {
        Some(eval(p, "({ down: window.__graphiteFinalDown, captured: [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')].some(e => e.hasPointerCapture(window.__graphiteFinalDown?.id)) })").await?)
    };
    p.execute(
        DispatchMouseEventParams::builder()
            .r#type(DispatchMouseEventType::MouseReleased)
            .x(x + 30.0)
            .y(y + dy)
            .modifiers(2)
            .buttons(0)
            .button(MouseButton::Left)
            .click_count(1)
            .build()
            .map_err(err)?,
    )
    .await
    .map_err(err)?;
    Ok(final_probe)
}
async fn viewport(p: &Page, width: i64) -> Result<()> {
    let height = if width == 390 { 844 } else { 1043 };
    p.execute(SetDeviceMetricsOverrideParams::new(
        width, height, 1.0, false,
    ))
    .await
    .map_err(err)?;
    wait(
        p,
        &format!("innerWidth === {width} && innerHeight === {height}"),
    )
    .await
}
async fn run(p: &Page, shots: Option<&str>) -> Result<()> {
    let mut f = Vec::new();
    ready(p).await?;
    driver::create_new_keyboard(p).await?;
    for name in ["Split keyboard", "Continue to layout"] {
        click(p, name).await?;
    }
    for name in ["Rows", "Columns"] {
        let selector = format!("[role=slider][aria-label=\"{name}\"]");
        css(p, &selector).await?;
        let expression =
            format!("Number(document.querySelector('{selector}').getAttribute('aria-valuenow'))");
        let value = eval(p, &expression)
            .await?
            .as_i64()
            .ok_or_else(|| err("invalid slider value"))?;
        let (key, code) = if value < 10 {
            ("ArrowRight", 39)
        } else {
            ("ArrowLeft", 37)
        };
        for _ in 0..(10 - value).unsigned_abs() {
            driver::press_key(p, key, key, code).await?;
        }
        wait(p, &format!("{expression} === 10")).await?;
    }
    click(p, "Create matrix").await?;
    wait_visible(
        p,
        ".m1-scene-part[data-part-id^=\"matrix/\"]",
        Duration::from_secs(60),
    )
    .await?;
    click(p, "Hide guide").await?;
    nav(p, "Layout").await?;
    click(p, "Fit board").await?;
    scope(p, "Column").await?;
    check(p, &mut f, "header workspace navigation exposes six workspaces and current Layout", r#"(() => { const all = [...document.querySelectorAll('.m1-workflow-navigation')], n = document.querySelector('.m1-topbar .m1-workflow-navigation'); const names = ['Layout', 'PCB', 'Keymap', 'Keycaps', 'Case', 'Parts']; return all.length === 1 && n?.getAttribute('aria-label') === 'Workspaces' && !document.querySelector('#m1-objects-panel .m1-workflow-navigation') && n.querySelectorAll('button').length === 6 && names.every(name => n.querySelectorAll(`button[aria-label="Open ${name} workspace"][aria-controls="m1-workspace-panel"]`).length === 1) && n.querySelectorAll('[aria-current="page"]').length === 1 && n.querySelector('[aria-label="Open Layout workspace"]')?.getAttribute('aria-current') === 'page'; })()"#).await?;
    css(
        p,
        ".m1-scene-part[data-part-id^=\"matrix/\"][data-part-id$=\"r0c2\"] .m1-part-hit-area",
    )
    .await?;
    let key_size_visible = visible(p, ".m1-key-size-preview svg").await?.to_string();
    for (label, expr) in [
        ("canvas numeric card removed", "document.querySelectorAll('.m1-canvas-measurements').length === 0"),
        ("board picker removed", "document.querySelectorAll('#m1-objects-panel select[aria-label=Board]').length === 0"),
        ("footer grid status removed", "document.querySelectorAll('.m1-canvas-footer-grid').length === 0"),
        ("key size starts expanded at top", "document.querySelectorAll('.m1-layout-key-size-section[open]').length === 1"),
        ("key size SVG preview present", key_size_visible.as_str()),
        ("width/height sliders removed", "document.querySelectorAll('input[aria-label=\"Key width\"],input[aria-label=\"Key height\"]').length === 0")
    ] { check(p,&mut f,label,expr).await?; }
    set(p, "Key size", "2").await?;
    wait(
        p,
        "document.querySelector('.m1-key-size-preview output')?.textContent === '2u × 1u'",
    )
    .await?;
    css(p, "[aria-label=\"Rotate keycap\"]").await?;
    wait(
        p,
        "document.querySelector('.m1-key-size-preview output')?.textContent === '1u × 2u'",
    )
    .await?;
    passed("size and rotate use accepted key geometry");
    click(p, "Undo").await?;
    wait(
        p,
        "document.querySelector('.m1-key-size-preview output')?.textContent === '2u × 1u'",
    )
    .await?;
    click(p, "Undo").await?;
    wait(
        p,
        "document.querySelector('.m1-key-size-preview output')?.textContent === '1u × 1u'",
    )
    .await?;
    let before = eval(
        p,
        "Number(document.querySelector('.m1-inspector-body [aria-label=\"Offset X\"]').value)",
    )
    .await?
    .as_f64()
    .ok_or_else(|| err("offset is not numeric"))?;
    check(
        p,
        &mut f,
        "offset hidden until Ctrl",
        "document.querySelectorAll('[aria-label=\"Drag to offset column\"]').length === 0",
    )
    .await?;
    driver::control(p, DispatchKeyEventType::RawKeyDown).await?;
    wait(
        p,
        "!!document.querySelector('[aria-label=\"Drag to offset column\"]')",
    )
    .await?;
    check(p,&mut f,"Ctrl replaces stagger rather than adding a handle","document.querySelectorAll('[aria-label=\"Drag to stagger\"]').length === 0 && document.querySelectorAll('[aria-label=\"Drag to offset column\"]').length === 1").await?;
    eval(p, "(() => { const r=document.querySelector('.m1-keycap-overlay.is-selected').getBoundingClientRect(); window.__graphiteInitial={x:r.x,y:r.y}; return true })()").await?;
    drag_offset(p, -15.0, true, true).await?;
    let unchanged=format!("Math.abs(Number(document.querySelector('.m1-inspector-body [aria-label=\"Offset X\"]').value)-({before})) < 0.0001");
    wait(p, &unchanged).await?;
    wait(p, "(() => { const r=document.querySelector('.m1-keycap-overlay.is-selected')?.getBoundingClientRect(), b=window.__graphiteInitial; return r && Math.hypot(r.x-b.x,r.y-b.y)<0.5 })()").await?;
    passed("Ctrl offset Escape restores the moved preview");
    let probe = drag_offset(p, 0.0, false, false)
        .await?
        .expect("final drag probe");
    let down = &probe["down"];
    let target_ok = down["trusted"] == true && down["target"] == true;
    let target_label = format!("Ctrl offset final pointerdown reaches handle (observed: {down})");
    if target_ok {
        passed(&target_label);
    } else {
        println!("FAIL {target_label}");
        f.push(target_label);
    }
    let capture_label = "Ctrl offset final drag acquires pointer capture";
    if probe["captured"] == true {
        passed(capture_label);
    } else {
        println!("FAIL {capture_label}");
        f.push(capture_label.into());
    }
    let changed=format!("Math.abs(Number(document.querySelector('.m1-inspector-body [aria-label=\"Offset X\"]').value)-({before})) > 0.0001");
    // The original checks the final value after up to five seconds, including a failed verdict.
    if let Err(e) = driver::wait_for(p, &changed, "offset change", Duration::from_secs(5)).await {
        if !e
            .to_string()
            .starts_with("timed out waiting for offset change")
        {
            return Err(e);
        }
    }
    let committed = eval(p, &changed).await? == Value::Bool(true);
    check(p, &mut f, "Ctrl offset commits a real edit", &changed).await?;
    if committed {
        click(p, "Undo").await?;
        wait(p, &unchanged).await?;
        passed("Ctrl offset is one undo step");
    } else {
        println!("FAIL Ctrl offset is one undo step (blocked: edit did not land)");
        f.push("Ctrl offset is one undo step (blocked: edit did not land)".into());
    }
    driver::control(p, DispatchKeyEventType::KeyUp).await?;
    let delete_visible = (visible(p, ".m1-inspector-body .m1-selection-delete button").await?
        && visible(p, ".m1-canvas-delete button").await?)
        .to_string();
    for (label, expr) in [
      ("stagger returns after Ctrl release", "document.querySelectorAll('[aria-label=\"Drag to stagger\"]').length === 1"),
      ("stagger and splay form a centered accessible rail", r#"(() => { const keys = [...document.querySelectorAll('.m1-keycap-overlay.is-selected')].map(e => e.getBoundingClientRect()); const handles = ['Drag to stagger', 'Drag to splay'].map(label => document.querySelector(`[aria-label="${label}"]`)); if (!keys.length || handles.some(e => !e || e.getAttribute('role') !== 'button' || e.getAttribute('tabindex') !== '0')) return false; const [a, b] = handles.map(e => e.getBoundingClientRect()); const center = (Math.min(...keys.map(r => r.left)) + Math.max(...keys.map(r => r.right))) / 2; return a.width > 0 && b.width > 0 && Math.abs(a.right - b.left) <= 2 && Math.abs(a.top - b.top) <= 2 && Math.abs((a.left + b.right) / 2 - center) <= 3 && a.top < Math.min(...keys.map(r => r.top)); })()"#),
      ("delete is outside Advanced and available on canvas", delete_visible.as_str()),
      ("Splay affects is beside Splay, outside origin disclosure", "!document.querySelector('.m1-inspector-body [aria-label=\"Splay affects\"]').closest('details[aria-label=\"Splay and origin\"]')"),
      ("Layers collapsed by default", "document.querySelector('#m1-layers-trigger').getAttribute('aria-expanded') === 'false'"),
      ("Layers stays at bottom independent of tree length", "(() => {const d=document.querySelector('#m1-layers-trigger').getBoundingClientRect(),p=document.querySelector('#m1-objects-panel').getBoundingClientRect();return d.y>p.y+p.height-110})()")
    ] { check(p,&mut f,label,expr).await?; }
    css(p, "#m1-layers-trigger").await?;
    check(p, &mut f, "Layers opens an accessible inline disclosure", r#"(() => { const t = document.querySelector('#m1-layers-trigger'), l = document.querySelector('#m1-layers-list'), c = document.getElementById(t?.getAttribute('aria-controls')); const r = l?.getBoundingClientRect(); return t?.getAttribute('aria-label') === 'Canvas visibility' && t.getAttribute('aria-expanded') === 'true' && c?.contains(l) && !!r?.width && !!r?.height && !!l.querySelector('button[aria-label="Hide Keycaps"][aria-pressed="true"]'); })()"#).await?;
    click(p, "Hide Keycaps").await?;
    wait(p, r#"!document.querySelector('.m1-keycap-overlay') && !!document.querySelector('#m1-layers-list button[aria-label="Show Keycaps"][aria-pressed="false"]')"#).await?;
    passed("Layers visibility control hides keycaps");
    click(p, "Show Keycaps").await?;
    wait(p, r#"!!document.querySelector('.m1-keycap-overlay') && !!document.querySelector('#m1-layers-list button[aria-label="Hide Keycaps"][aria-pressed="true"]')"#).await?;
    passed("Layers visibility control restores keycaps");
    css(p, "#m1-layers-trigger").await?;
    check(p, &mut f, "Layers disclosure closes", "document.querySelector('#m1-layers-trigger').getAttribute('aria-expanded') === 'false' && !document.querySelector('#m1-layers-list')").await?;
    eval(
        p,
        "document.querySelector('#m1-layout-snap-trigger').focus(); true",
    )
    .await?;
    press_key(p, " ", "Space", 32, " ").await?;
    wait(p, "document.querySelector('#m1-layout-snap-trigger')?.getAttribute('aria-expanded') === 'true' && !!document.querySelector('select[aria-label=\"Snap increment\"]')").await?;
    let increment =
        driver::backend_of_wait(p, "select[aria-label=\"Snap increment\"]", WAIT).await?;
    let increment_visible = driver::is_visible(p, increment).await?;
    check(p, &mut f, "Snap accordion opens with Space and keeps focus", &format!("document.activeElement?.id === 'm1-layout-snap-trigger' && document.querySelector('#m1-layout-snap-trigger').getAttribute('aria-expanded') === 'true' && {increment_visible}")).await?;
    set(p, "Snap increment", "0.5").await?;
    press_enter(p).await?;
    wait(p, "document.querySelector('#m1-layout-snap-trigger')?.getAttribute('aria-expanded') === 'false'").await?;
    check(p, &mut f, "Snap accordion closes by keyboard", "document.activeElement?.id === 'm1-layout-snap-trigger' && document.querySelector('#m1-layout-snap-trigger').getAttribute('aria-expanded') === 'false' && !document.querySelector('select[aria-label=\"Snap increment\"]')").await?;
    press_enter(p).await?;
    wait(p, "document.querySelector('#m1-layout-snap-trigger')?.getAttribute('aria-expanded') === 'true' && !!document.querySelector('select[aria-label=\"Snap increment\"]')").await?;
    check(
        p,
        &mut f,
        "Snap increment persists",
        "document.querySelector('#m1-layout-snap-trigger')?.getAttribute('aria-expanded') === 'true' && document.querySelector('select[aria-label=\"Snap increment\"]')?.value === '0.5'",
    )
    .await?;
    press_enter(p).await?;
    scope(p, "Key").await?;
    css(
        p,
        ".m1-scene-part[data-part-id^=\"matrix/\"][data-part-id$=\"r5c5\"] .m1-part-hit-area",
    )
    .await?;
    let count = eval(p, "document.querySelectorAll('.m1-scene-part').length")
        .await?
        .as_u64()
        .ok_or_else(|| err("missing scene count"))?;
    css(p, ".m1-canvas-delete button").await?;
    wait(
        p,
        &format!("document.querySelectorAll('.m1-scene-part').length < {count}"),
    )
    .await?;
    click(p, "Undo").await?;
    wait(
        p,
        &format!("document.querySelectorAll('.m1-scene-part').length === {count}"),
    )
    .await?;
    passed("canvas delete uses existing typed delete and one Undo restores geometry");
    nav(p, "PCB").await?;
    check(
        p,
        &mut f,
        "board-wide Routing & nets is default",
        "document.querySelector('.m1-pcb-wiring-header h2')?.textContent === 'Routing & nets'",
    )
    .await?;
    let nets_visible = visible(p, ".m1-pcb-net-overview").await?;
    check(
        p,
        &mut f,
        "Nets overview visible",
        &nets_visible.to_string(),
    )
    .await?;
    set(p, "Group objects by", "part").await?;
    check(p,&mut f,"parts grouped by actual categories","['Switches','Diodes'].every(s=>[...document.querySelectorAll('.m1-component')].filter(e=>e.textContent.includes(s)).length===1)").await?;
    set(p, "Group objects by", "layout").await?;
    if !driver::find_role(p, None, "button", "Expand Left half")
        .await?
        .is_empty()
    {
        click(p, "Expand Left half").await?;
    }
    check(
        p,
        &mut f,
        "layout grouping restores matrix tree",
        "[...document.querySelectorAll('.m1-component')].some(e=>e.textContent.includes('keys'))",
    )
    .await?;
    eval(p,"(() => {const e=[...document.querySelectorAll('.m1-component')].find(e=>e.textContent.includes('Main board')); if(!e) throw Error('Main board missing'); e.setAttribute('data-graphite-check-target',''); return true})()").await?;
    css(p, "[data-graphite-check-target]").await?;
    wait(
        p,
        "!!document.querySelector('.m1-pcb-wiring-breadcrumb')?.textContent.includes('Main board')",
    )
    .await?;
    passed("board tree navigation replaces removed board picker");
    for workspace in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"] {
        nav(p, workspace).await?;
        check(p, &mut f, &format!("{workspace}: workspace navigation marks current"), &format!(r#"document.querySelectorAll('.m1-workflow-navigation [aria-current="page"]').length === 1 && document.querySelector('.m1-workflow-navigation [aria-label="Open {workspace} workspace"]')?.getAttribute('aria-current') === 'page'"#)).await?;
        if workspace == "Layout" {
            scope(p, "Column").await?;
            css(p,".m1-scene-part[data-part-id^=\"matrix/\"][data-part-id$=\"r0c2\"] .m1-part-hit-area").await?;
        }
        if workspace == "Case" {
            wait(p, "!!document.querySelector('.m1-case-view')").await?;
            for (label, selector) in [
                (
                    "Case stackup visible without configuration click",
                    ".m1-stackup-preview",
                ),
                (
                    "Case generation controls are in footer",
                    "footer.m1-case-footer",
                ),
            ] {
                let exists = eval(p, &format!("!!document.querySelector('{selector}')")).await?
                    == Value::Bool(true);
                let visible = if exists {
                    driver::is_visible(p, driver::backend_of(p, selector).await?).await?
                } else {
                    false
                };
                check(p, &mut f, label, &visible.to_string()).await?;
            }
            check(p, &mut f, "camera controls joined Case toolbar", "document.querySelectorAll('.m1-case-view-toolbar .m1-case-camera-controls').length===1").await?;
        }
        if workspace == "Parts" {
            let root = driver::backend_of(p, "#m1-objects-panel").await?;
            driver::click_role(p, Some(root), "button", "New assembly").await?;
            wait_visible(p, ".m1-parts-assembly-editor", WAIT).await?;
            passed("left + opens existing assembly editor");
            check(p,&mut f,"New assembly action removed from Inspector","document.querySelectorAll('.m1-inspector-body > .m1-parts-inspector > .m1-parts-assemblies > .m1-parts-create-component').length===0").await?;
            driver::click_role(p, None, "option", "MX Hotswap").await?;
            click(p, "3D model").await?;
            wait_visible(
                p,
                ".m1-case-view-toolbar [aria-label=\"Part preview view\"]",
                Duration::from_secs(60),
            )
            .await?;
            passed("Parts 3D view mode and camera controls share one pill");
            click(p, "2D footprint").await?;
        }
        check(
            p,
            &mut f,
            &format!("{workspace}: routed PCB reference absent"),
            "document.querySelectorAll('.m1-board-reference').length===0",
        )
        .await?;
        for width in [1513, 1024, 390] {
            viewport(p, width).await?;
            check(p,&mut f,&format!("{workspace} {width}: toolbar remains centered"),"(() => {const t=[...document.querySelectorAll('.m1-canvas-toolbar,.m1-case-view-toolbar,.m1-parts-preview > .m1-design-view-group')].find(e=>e.getBoundingClientRect().width && getComputedStyle(e).display!=='none');if(!t)return true;const v=t.closest('.m1-workspace-content');if(!v)return true;const a=t.getBoundingClientRect(),b=v.getBoundingClientRect();return Math.abs(a.x+a.width/2-b.x-b.width/2)<=2})()").await?;
            check(
                p,
                &mut f,
                &format!("{workspace} {width}: no page overflow"),
                "document.documentElement.scrollWidth <= innerWidth",
            )
            .await?;
            if let Some(dir) = shots {
                p.save_screenshot(
                    chromiumoxide::page::ScreenshotParams::builder().build(),
                    Path::new(dir).join(format!("{workspace}-{width}.png")),
                )
                .await
                .map_err(err)?;
            }
        }
        viewport(p, 1513).await?;
    }
    if !f.is_empty() {
        return Err(err(format!("failed checks: {}", f.join(", "))));
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = main_run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn main_run() -> Result<()> {
    let chrome = std::env::var("CHROME_BIN").unwrap_or_else(|_| "/usr/bin/chromium".into());
    let url = std::env::var("BOARDSTUDIO_PREVIEW_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8182/".into());
    let shots = std::env::var("BOARDSTUDIO_SCREENSHOTS")
        .ok()
        .filter(|s| !s.is_empty());
    if let Some(dir) = &shots {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(Path::new(&tmp)).map_err(err)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let he = Arc::new(HandlerErrors::new());
    let pe = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, Arc::clone(&he), Arc::clone(&pe));
    let outcome = async {
        let (context, page) = tokio::time::timeout(
            Duration::from_secs(60),
            driver::new_isolated_page(&browser, "about:blank", &pe),
        )
        .await
        .map_err(err)??;
        let result = async {
            page.execute(
                SetEmulatedMediaParams::builder()
                    .features(vec![MediaFeature::new("prefers-color-scheme", "dark")])
                    .build(),
            )
            .await
            .map_err(err)?;
            viewport(&page, 1513).await?;
            tokio::time::timeout(Duration::from_secs(60), page.goto(&url))
                .await
                .map_err(err)?
                .map_err(err)?;
            run(&page, shots.as_deref()).await
        }
        .await;
        let disposal = driver::dispose_isolated_page(&browser, Some(context), page, &pe).await;
        match (result, disposal) {
            (Err(a), Err(b)) => Err(err(format!("{a}; {b}"))),
            (Err(e), _) | (_, Err(e)) => Err(e),
            _ => Ok(()),
        }
    }
    .await;
    let (shutdown, confirmed) =
        shutdown::finish_browser(browser, drain, &he, outcome.is_ok()).await;
    let cleanup = if confirmed {
        shutdown::remove_profile(&dirs)
    } else {
        eprintln!(
            "browser termination unconfirmed; preserving profile at {}",
            dirs.profile_dir.display()
        );
        std::mem::forget(dirs);
        Ok(())
    };
    let errors = [
        outcome.err(),
        shutdown.err(),
        pe.fail_if_any().err(),
        cleanup.err(),
    ]
    .into_iter()
    .flatten()
    .map(|e| e.to_string())
    .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(err(errors.join("; ")))
    }
}
