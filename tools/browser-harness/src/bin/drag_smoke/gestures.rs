use super::*;

pub(super) async fn set_slider(page: &Page, name: &str, target: i64) -> Result<()> {
    let query = format!(
        "document.querySelector('[role=\"slider\"][aria-label=\"{name}\"]')?.getAttribute('aria-valuenow')"
    );
    // The setup step may mount or become visible asynchronously. Re-attempt
    // focus when it can receive input instead of typing into the old footer.
    driver::wait_for(
        page,
        &format!(
            r#"(() => {{
  const slider = document.querySelector('[role="slider"][aria-label="{name}"]');
  if (!slider || slider.getClientRects().length === 0 || slider.disabled
      || slider.getAttribute('aria-disabled') === 'true' || slider.closest('[inert]')) return false;
  slider.focus();
  return document.activeElement === slider;
}})()"#
        ),
        &format!("slider {name} to mount and receive focus"),
        Duration::from_secs(10),
    )
    .await?;
    loop {
        let current: i64 = driver::eval(page, &query)
            .await?
            .as_str()
            .unwrap_or("0")
            .parse()
            .map_err(|_| fail(format!("slider {name} has no numeric value")))?;
        if current == target {
            break;
        }
        let (key, code, windows_code) = if current < target {
            ("ArrowRight", "ArrowRight", 39)
        } else {
            ("ArrowLeft", "ArrowLeft", 37)
        };
        driver::press_key(page, key, code, windows_code).await?;
        let expected = if current < target {
            current + 1
        } else {
            current - 1
        };
        driver::wait_for(
            page,
            &format!(
                "document.querySelector('[role=\"slider\"][aria-label=\"{name}\"]')?.getAttribute('aria-valuenow') === {}",
                serde_json::to_string(&expected.to_string()).unwrap_or_default()
            ),
            &format!("slider {name} at {expected}"),
            Duration::from_secs(10),
        )
        .await?;
    }
    let final_value = driver::eval(page, &query).await?;
    if final_value.as_str() != Some(&target.to_string()) {
        return Err(fail(format!("slider {name} did not reach {target}")));
    }
    Ok(())
}

pub(super) async fn select_scope(page: &Page, name: &str) -> Result<()> {
    let direct = format!("Select {name}");
    let matches = driver::find_role(page, None, "button", &direct).await?;
    let visible = if matches.len() == 1 {
        driver::is_visible(page, matches[0].0).await
    } else {
        Ok(false)
    };
    if matches.len() == 1 && visible.unwrap_or(false) {
        return driver::click_backend(page, matches[0].0).await;
    }
    let diagnosis = driver::eval(
        page,
        r#"(() => {
      const buttons = [...document.querySelectorAll('button,[role="button"]')].map(el =>
        (el.getAttribute('aria-label') || el.innerText || '').trim().slice(0, 60));
      return JSON.stringify({
        trigger: !!document.querySelector('#m1-layout-select-trigger'),
        buttons,
      });
    })()"#,
    )
    .await
    .unwrap_or(Value::Null);
    if driver::eval(
        page,
        "!!document.querySelector('#m1-layout-select-trigger')",
    )
    .await?
        != Value::Bool(true)
    {
        return Err(fail(format!(
            "no {name} scope control and no scope menu; toolbar: {diagnosis}"
        )));
    }
    driver::click_css(page, "#m1-layout-select-trigger").await?;
    let menu = driver::backend_of(page, "#m1-layout-select-menu").await?;
    let inside = driver::find_role(page, Some(menu), "button", name).await?;
    match inside.as_slice() {
        [single] => driver::click_backend(page, single.0).await,
        [] => Err(fail(format!("no {name} control in the layout scope menu"))),
        many => Err(fail(format!(
            "ambiguous scope lookup: {name} matched {} nodes",
            many.len()
        ))),
    }
}

async fn start_event_capture(page: &Page) -> Result<()> {
    driver::eval(
        page,
        r#"(() => {
    window.__dragEvents = [];
    if (!window.__dragCaptureInstalled) {
      for (const type of ['pointerdown', 'pointermove', 'pointerup', 'pointercancel']) {
        document.addEventListener(type, event => window.__dragEvents.push({
          type,
          time: event.timeStamp,
          trusted: event.isTrusted,
          pointerType: event.pointerType,
          pointerId: event.pointerId,
          button: event.button,
          buttons: event.buttons,
          clientX: event.clientX,
          clientY: event.clientY,
          movementX: event.movementX,
          movementY: event.movementY,
          detail: event.detail,
          isPrimary: event.isPrimary,
          label: event.target.closest?.('[data-transform-handle]')?.getAttribute('aria-label'),
          partId: event.target.closest?.('[data-part-id]')?.getAttribute('data-part-id'),
        }), true);
      }
      window.__dragCaptureInstalled = true;
    }
    return true;
  })()"#,
    )
    .await
    .map(|_| ())
}

/// The in-page observation of one drag gesture.
pub(super) struct LiveDrag {
    events: Value,
    pub(super) capture: bool,
    pub(super) scene: Value,
}

/// A drag with release discipline: every failure between begin and end still
/// ends the hold, mirroring the old runner's `release()` guards.
pub(super) async fn drag(
    name: &str,
    page: &Page,
    selector: &str,
    dx: f64,
    dy: f64,
    with_control: bool,
    before_scene: &Value,
) -> Result<(LiveDrag, DragHold)> {
    start_event_capture(page).await?;
    let mut hold = driver::begin_drag(page, selector, dx, dy, with_control, 5).await?;
    // The live preview renders asynchronously; poll the scene itself until it
    // moves, comparing canonical signatures in Rust so no serialization
    // skew can pass the wait before the preview lands.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let current = scene(page).await?;
        if signature_of(&current) != signature_of(before_scene) {
            break;
        }
        if Instant::now() >= deadline {
            driver::end_drag(page, &mut hold).await.ok();
            return Err(fail(format!(
                "{name}: timed out waiting for the drag preview"
            )));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let observed = observe_live_drag(page).await;
    match observed {
        Ok(live) => Ok((live, hold)),
        Err(error) => {
            driver::end_drag(page, &mut hold).await.ok();
            Err(error)
        }
    }
}

async fn observe_live_drag(page: &Page) -> Result<LiveDrag> {
    let events = driver::eval(page, "window.__dragEvents").await?;
    let capture = driver::eval(
        page,
        r#"(() => { const down = window.__dragEvents.find(event => event.type === 'pointerdown');
      return !!down && [...document.querySelectorAll('svg:has([data-geometry="scene-part"]),[data-transform-handle]')]
        .some(element => element.hasPointerCapture(down.pointerId)); })()"#,
    )
    .await?;
    let scene_now = scene(page).await?;
    Ok(LiveDrag {
        events,
        capture: capture == Value::Bool(true),
        scene: scene_now,
    })
}

/// The exact summaries Runtime renders for an unavailable PCB or an uncertain
/// raw observation never count.
pub(super) fn assert_unavailable_pcb_pairs_do_not_count() -> Result<()> {
    for text in [
        "plate [plate] ↔ PCB [pcb-reference]: Uncertain (no mesh observation; left generated body has no certified analysisErrorBoundMm, right generated body has no certified analysisErrorBoundMm, Parry query has no certified numerical error bound)",
        "bottom [bottom] ↔ PCB [pcb-reference]: Uncertain (uncertain observation (query unsupported); raw mesh observation is uncertain: query unsupported)",
        "SW1 [SW1:0] ↔ PCB [pcb-reference]: Uncertain (surface intersection near (2.33, 5.08, 2.00) mm; left asset has no certified analysisErrorBoundMm)",
        "plate [plate] ↔ bottom [bottom]: Uncertain (surface separation 3.500 mm between (0.00, 0.00, 0.00) and (0.00, 0.00, 3.50) mm)",
    ] {
        if observed_case_pcb_pair(text) {
            return Err(fail(format!("not a PCB observation: {text}")));
        }
    }
    for text in [
        "plate [plate] ↔ PCB [pcb-reference]: Uncertain (surface separation 3.500 mm between (-12.40, -13.00, 3.50) and (-12.40, -13.00, 0.00) mm)",
        "bottom [bottom] ↔ PCB [pcb-reference]: Uncertain (surface intersection near (24.68, 28.75, -0.65) mm)",
        "bottom [bottom] ↔ PCB [pcb-reference]: Clear [0.400, 0.400] mm vs 0.000 mm (surface separation 0.400 mm between (0, 0, -2) and (0, 0, -1.6) mm)",
    ] {
        if !observed_case_pcb_pair(text) {
            return Err(fail(format!("missed a PCB observation: {text}")));
        }
    }
    Ok(())
}

pub(super) struct DragConfig {
    pub(super) name: &'static str,
    pub(super) selector: String,
    pub(super) label: Option<&'static str>,
    pub(super) dx: f64,
    pub(super) dy: f64,
    pub(super) with_control: bool,
}

fn drag_events(events: &Value) -> Vec<&Value> {
    events
        .as_array()
        .map(|list| list.iter().collect())
        .unwrap_or_default()
}

/// Exact in-page verdict for one drag's matrix state.
///
/// The whole comparison runs in the page with page-exact doubles: the Rust
/// side only sees per-field change flags and the column-cell move count, so
/// no float repr crosses the bridge. `fields` selects which of the five state
/// keys participate (the Part drag excludes `columnCellOffsets`, mirroring
/// the old runner).
struct StateVerdict {
    changed: BTreeMap<String, bool>,
    moved_column_cells: usize,
    live_canonical: BTreeMap<String, String>,
}

/// Canonical JSON string of the drag-relevant matrix state, computed in-page.
async fn capture_state_js(
    page: &Page,
    matrix_id: &str,
    row: i64,
    column: i64,
    fields: &[&str],
) -> Result<String> {
    let expression = format!(
        r#"(async (database, matrixId, row, column, fields) => {{
    const project = await new Promise((resolve, reject) => {{
      const request = indexedDB.open(database);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {{
        const db = request.result;
        const query = db.transaction('projects').objectStore('projects').getAll();
        query.onerror = () => reject(query.error);
        query.onsuccess = () => {{ db.close(); resolve(query.result[0]); }};
      }};
    }});
    const matrix = project.matrices.find(item => item.id === matrixId);
    const cells = matrix.cells || [];
    const cell = cells.find(item => item.row === row && item.column === column);
    const state = {{
      offset: matrix.columnLayout?.[column]?.offset || {{ x: 0, y: 0 }},
      stagger: matrix.columnLayout?.[column]?.stagger ?? 0,
      cellOffset: cell?.offset || {{ x: 0, y: 0 }},
      columnCellOffsets: Array.from({{ length: matrix.rows }}, (_, cellRow) =>
        cells.find(item => item.row === cellRow && item.column === column)?.offset || {{ x: 0, y: 0 }}),
      origin: matrix.origin || {{ x: 0, y: 0 }},
    }};
    const canonical = value => Array.isArray(value) ? value.map(canonical)
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
        : value;
    const picked = {{}};
    for (const field of fields) picked[field] = state[field];
    return JSON.stringify(canonical(picked));
  }})({}, {}, {}, {}, {})"#,
        serde_json::to_string(STORAGE_DATABASE).unwrap_or_default(),
        serde_json::to_string(matrix_id).unwrap_or_default(),
        row,
        column,
        serde_json::to_string(fields).unwrap_or_default(),
    );
    let canonical = driver::eval(page, &expression).await?;
    Ok(canonical
        .as_str()
        .ok_or_else(|| fail("the live matrix state is missing"))?
        .to_owned())
}

/// Poll the exact in-page verdict until every expected field changed.
async fn poll_state_verdict(
    page: &Page,
    matrix_id: &str,
    row: i64,
    column: i64,
    fields: &[&str],
    before_js: &str,
    expected_fields: &[&str],
) -> Result<StateVerdict> {
    let expression = format!(
        r#"(async (database, matrixId, row, column, fields, beforeJson, expected) => {{
    const project = await new Promise((resolve, reject) => {{
      const request = indexedDB.open(database);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {{
        const db = request.result;
        const query = db.transaction('projects').objectStore('projects').getAll();
        query.onerror = () => reject(query.error);
        query.onsuccess = () => {{ db.close(); resolve(query.result[0]); }};
      }};
    }});
    const matrix = project.matrices.find(item => item.id === matrixId);
    const cells = matrix.cells || [];
    const cell = cells.find(item => item.row === row && item.column === column);
    const state = {{
      offset: matrix.columnLayout?.[column]?.offset || {{ x: 0, y: 0 }},
      stagger: matrix.columnLayout?.[column]?.stagger ?? 0,
      cellOffset: cell?.offset || {{ x: 0, y: 0 }},
      columnCellOffsets: Array.from({{ length: matrix.rows }}, (_, cellRow) =>
        cells.find(item => item.row === cellRow && item.column === column)?.offset || {{ x: 0, y: 0 }}),
      origin: matrix.origin || {{ x: 0, y: 0 }},
    }};
    const canonical = value => Array.isArray(value) ? value.map(canonical)
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
        : value;
    const before = JSON.parse(beforeJson);
    const live = {{}};
    const changed = {{}};
    for (const field of fields) {{
      live[field] = JSON.stringify(canonical(state[field]));
      changed[field] = live[field] !== JSON.stringify(canonical(before[field]));
    }}
    let movedColumnCells = 0;
    if (Array.isArray(before.columnCellOffsets) && Array.isArray(state.columnCellOffsets)) {{
      const count = Math.min(before.columnCellOffsets.length, state.columnCellOffsets.length);
      for (let index = 0; index < count; index++) {{
        if (JSON.stringify(canonical(state.columnCellOffsets[index]))
            !== JSON.stringify(canonical(before.columnCellOffsets[index]))) {{
          movedColumnCells++;
        }}
      }}
    }}
    return {{
      ready: expected.every(field => changed[field]),
      changed,
      movedColumnCells,
      live,
    }};
  }})({}, {}, {}, {}, {}, {}, {})"#,
        serde_json::to_string(STORAGE_DATABASE).unwrap_or_default(),
        serde_json::to_string(matrix_id).unwrap_or_default(),
        row,
        column,
        serde_json::to_string(fields).unwrap_or_default(),
        serde_json::to_string(before_js).unwrap_or_default(),
        serde_json::to_string(expected_fields).unwrap_or_default(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let verdict = driver::eval(page, &expression).await?;
        if verdict.get("ready") == Some(&Value::Bool(true)) {
            let changed = verdict
                .get("changed")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(key, value)| (key.clone(), value == &Value::Bool(true)))
                        .collect()
                })
                .unwrap_or_default();
            let live_canonical = verdict
                .get("live")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(key, value)| (key.clone(), value.as_str().unwrap_or("?").to_owned()))
                        .collect()
                })
                .unwrap_or_default();
            let moved_column_cells = verdict
                .get("movedColumnCells")
                .and_then(Value::as_u64)
                .unwrap_or(usize::MAX as u64) as usize;
            return Ok(StateVerdict {
                changed,
                moved_column_cells,
                live_canonical,
            });
        }
        if Instant::now() >= deadline {
            return Err(fail("timed out waiting for the changed matrix cells"));
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
}

/// Run one drag through preview, durable save and exact Undo, like the old
/// runner's `runDrag`. A reproduced app failure stays a failure here.
pub(super) async fn run_drag(
    smoke: &mut Smoke,
    config: &DragConfig,
    state_fields: &[&str],
    expected_fields: &[&str],
    allowed_fields: &[&str],
    identity: (String, i64, i64),
) -> Result<()> {
    let name = config.name;
    println!("drag-smoke: {name} starts");
    let page = &smoke.page;
    let checks = &mut smoke.checks;
    saved(page).await?;
    let before = doc(page).await?;
    let before_scene = scene(page).await?;
    let (matrix_id, row, column) = identity.clone();
    let before_state_js = capture_state_js(page, &matrix_id, row, column, state_fields).await?;
    let (live, mut hold) = drag(
        name,
        page,
        &config.selector,
        config.dx,
        config.dy,
        config.with_control,
        &before_scene,
    )
    .await?;
    if signature_of(&live.scene) == signature_of(&before_scene) {
        let full = driver::eval(
            page,
            r#"JSON.stringify(window.__dragEvents.map(event => ({
  type: event.type, trusted: event.trusted, button: event.button, buttons: event.buttons,
  clientX: event.clientX, clientY: event.clientY, movementX: event.movementX, movementY: event.movementY,
  detail: event.detail, isPrimary: event.isPrimary, pointerId: event.pointerId, width: event.width,
  label: event.label, partId: event.partId,
})))"#,
        )
        .await
        .unwrap_or(Value::Null);
        driver::end_drag(page, &mut hold).await.ok();
        let events_text = full.as_str().unwrap_or("");
        return Err(fail(format!(
            "{name}: drag produced no visible preview; events={}",
            events_text.chars().take(1500).collect::<String>(),
        )));
    }
    let events = drag_events(&live.events);
    if !events.iter().any(|event| {
        event.get("type") == Some(&json!("pointerdown"))
            && event.get("trusted") == Some(&json!(true))
    }) {
        driver::end_drag(page, &mut hold).await.ok();
        return Err(fail(format!("{name}: trusted pointerdown")));
    }
    if events
        .iter()
        .filter(|event| {
            event.get("type") == Some(&json!("pointermove"))
                && event.get("trusted") == Some(&json!(true))
        })
        .count()
        < 2
    {
        driver::end_drag(page, &mut hold).await.ok();
        return Err(fail(format!("{name}: real pointer movement occurred")));
    }
    if !live.capture {
        driver::end_drag(page, &mut hold).await.ok();
        return Err(fail(format!(
            "{name}: canvas retained pointer capture while dragging"
        )));
    }
    if let Some(label) = config.label {
        if !events
            .iter()
            .any(|event| event.get("label") == Some(&json!(label)))
        {
            driver::end_drag(page, &mut hold).await.ok();
            return Err(fail(format!("{name}: gesture used {label}")));
        }
    }
    start_save_tracking(page).await?;
    driver::end_drag(page, &mut hold).await?;
    wait_for_save_cycle(page).await?;
    let verdict = poll_state_verdict(
        page,
        &matrix_id,
        row,
        column,
        state_fields,
        &before_state_js,
        expected_fields,
    )
    .await?;
    let durable_at = driver::eval(page, "performance.now()").await?;
    let after = doc(page).await?;
    if after.page_signature == before.page_signature {
        return Err(fail(format!("{name}: edit was durably saved")));
    }
    for field in expected_fields {
        if verdict.changed.get(*field) != Some(&true) {
            return Err(fail(format!(
                "{name}: changed {field}; live now={}",
                verdict
                    .live_canonical
                    .get(*field)
                    .cloned()
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect::<String>(),
            )));
        }
    }
    if name == "Column body drag" && verdict.moved_column_cells <= 1 {
        return Err(fail(format!(
            "Column body drag: moved more than one key in its column ({})",
            verdict.moved_column_cells
        )));
    }
    for key in state_fields {
        if allowed_fields.contains(key) {
            continue;
        }
        if verdict.changed.get(*key) == Some(&true) {
            return Err(fail(format!("{name}: preserved {key}")));
        }
    }
    let settled = driver::eval(page, "window.__dragEvents").await?;
    let settled = drag_events(&settled);
    let pointer_down = settled
        .iter()
        .find(|event| event.get("type") == Some(&json!("pointerdown")));
    let pointer_up = settled
        .iter()
        .rev()
        .find(|event| event.get("type") == Some(&json!("pointerup")));
    let (Some(pointer_down), Some(pointer_up)) = (pointer_down, pointer_up) else {
        return Err(fail(format!(
            "{name}: pointerdown/pointerup completed the gesture"
        )));
    };
    if pointer_down.get("pointerType") != Some(&json!("mouse")) {
        return Err(fail(format!("{name}: mouse input was used")));
    }
    if pointer_up.get("trusted") != Some(&json!(true)) {
        return Err(fail(format!(
            "{name}: trusted pointerup completed the gesture"
        )));
    }
    if pointer_up.get("pointerType") != Some(&json!("mouse")) {
        return Err(fail(format!("{name}: pointerup used mouse input")));
    }
    let down_ms = pointer_down
        .get("time")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    let up_ms = pointer_up
        .get("time")
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    let durable_ms = durable_at.as_f64().unwrap_or(f64::NAN);
    let down_to_up = (up_ms - down_ms).round() as i64;
    let release_to_durable = (durable_ms - up_ms).round() as i64;
    if !(down_to_up >= 0) || !(release_to_durable >= 0) {
        return Err(fail(format!("{name}: valid gesture timing")));
    }
    checks.push(Check {
        name: name.to_owned(),
        details: detail(&[
            (
                "changed",
                Value::Array(expected_fields.iter().map(|field| json!(field)).collect()),
            ),
            ("input", json!("mouse")),
            ("events", json!(settled.len() as i64)),
            ("pointerDownToUpMs", json!(down_to_up)),
            ("releaseToDurableMs", json!(release_to_durable)),
            ("savedRevision", after.revision.clone()),
        ]),
    });
    println!(
        "drag-smoke: {name} revisions before={} after={}",
        before.revision, after.revision
    );
    driver::click_role(page, None, "button", "Undo").await?;
    wait_document(page, &before.page_signature).await?;
    saved(page).await?;
    let undone = doc(page).await?;
    // Exact page-canonical comparison, like the old runner: any drift, even
    // sub-ulp float noise from recomputation, fails the restore.
    if undone.page_signature != before.page_signature {
        let differing = diff_paths(&before.project, &undone.project);
        return Err(fail(format!(
            "{name}: Undo restored the complete saved document; differs at {}",
            differing.join(", ")
        )));
    }
    let after_revision = after.revision.as_i64();
    let undone_revision = undone.revision.as_i64();
    if after_revision.is_some()
        && undone_revision.is_some_and(|revision| revision <= after_revision.unwrap_or(0))
    {
        return Err(fail(format!("{name}: Undo was saved as a new revision")));
    }
    if signature_of(&scene(page).await?) != signature_of(&before_scene) {
        return Err(fail(format!(
            "{name}: Undo restored the exact canvas scene"
        )));
    }
    checks.push(Check {
        name: format!("{name} undo"),
        details: detail(&[("exactDocument", json!(true)), ("exactScene", json!(true))]),
    });
    println!("drag-smoke: {name} and exact Undo passed");
    Ok(())
}
