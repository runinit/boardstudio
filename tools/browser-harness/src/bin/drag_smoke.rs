//! Real-page drag smoke driven by the Rust browser harness.
//!
//! A faithful port of `scripts/test-drag-interactions.cjs`: trusted pointer
//! drags over Part, Column, Stagger and Ctrl+Offset handles, live preview,
//! pointer capture, durable saves, exact Undo/reload, collision observations
//! and receipt/provenance validation. The old runner stays available for
//! same-build comparison until the parity gate passes.

#[path = "../shutdown.rs"]
mod shutdown;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use boardstudio_browser_harness::driver::{self, DragHold, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};

use chromiumoxide::{Browser, Page};
use serde_json::{json, Value};

const STORAGE_DATABASE: &str = "boardstudio-m1-boardstudio";

fn fail(message: impl Into<String>) -> Error {
    Error::new(message)
}

fn scenario_verdict(outcome: Result<()>, disposal: Result<()>) -> Result<()> {
    match (outcome, disposal) {
        (Err(scenario), Err(disposal)) => Err(fail(format!("{scenario}; {disposal}"))),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

#[cfg(test)]
mod scenario_verdict_tests {
    use super::*;

    #[test]
    fn disposal_failure_fails_and_preserves_scenario_failure() {
        assert!(scenario_verdict(Ok(()), Err(fail("disposal"))).is_err());
        let error = scenario_verdict(Err(fail("scenario")), Err(fail("disposal")))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("scenario") && error.contains("disposal"),
            "{error}"
        );
        assert!(scenario_verdict(Ok(()), Ok(())).is_ok());
    }
}

fn signature_of(value: &Value) -> String {
    // Serialization of a canonical `Value` cannot fail; a silent empty
    // default would let two empty scenes compare equal, so be loud instead.
    serde_json::to_string(&boardstudio_browser_harness::canonical(value))
        .expect("canonical scene serializes")
}

fn iso_now() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let days = (now.as_secs() / 86_400) as i64;
    let secs = (now.as_secs() % 86_400) as i64;
    // Howard Hinnant's days-to-civil conversion.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month = (day_of_year * 5 + 2) / 153;
    let day = day_of_year - (month * 153 + 2) / 5 + 1;
    let month = if month < 10 { month + 3 } else { month - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        secs / 3_600,
        secs % 3_600 / 60,
        secs % 60,
        now.subsec_millis()
    )
}

fn env(name: &str) -> Result<String> {
    std::env::var(name)
        .map_err(|_| fail(format!("drag browser smoke needs {name} from its runner")))
}

fn has_exec_bit(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

fn is_executable(path: &str) -> bool {
    let candidate = if path.contains('/') {
        std::path::PathBuf::from(path)
    } else {
        let Some(paths) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&paths)
            .map(|dir| dir.join(path))
            .find(|candidate| has_exec_bit(candidate))
            .unwrap_or_else(|| std::path::PathBuf::from(path))
    };
    has_exec_bit(&candidate)
}

fn resolve_chrome() -> Result<String> {
    // No personal absolute fallbacks: `CHROME_BIN` first, then the usual
    // executable names from `PATH`. The runner always sets `CHROME_BIN`.
    let get_env = |name: &str| std::env::var(name).ok();
    boardstudio_browser_harness::discover_chrome(&get_env, &is_executable).map_err(|_| {
        fail("drag browser smoke needs Chrome or Chromium; set CHROME_BIN to its executable")
    })
}

/// First differing paths between two canonical values, for actionable failure
/// records. Capped so one structural diff cannot flood the run log.
fn diff_paths(before: &Value, after: &Value) -> Vec<String> {
    fn walk(before: &Value, after: &Value, path: &str, out: &mut Vec<String>) {
        if out.len() >= 8 || before == after {
            return;
        }
        match (before, after) {
            (Value::Object(before_map), Value::Object(after_map)) => {
                let mut keys: Vec<&String> = before_map.keys().chain(after_map.keys()).collect();
                keys.sort();
                keys.dedup();
                for key in keys {
                    match (before_map.get(key), after_map.get(key)) {
                        (Some(a), Some(b)) => walk(a, b, &format!("{path}.{key}"), out),
                        (None, _) => out.push(format!("{path}.{key} added")),
                        (_, None) => out.push(format!("{path}.{key} removed")),
                    }
                    if out.len() >= 8 {
                        return;
                    }
                }
            }
            (Value::Array(before_items), Value::Array(after_items)) => {
                if before_items.len() != after_items.len() {
                    out.push(format!(
                        "{path} length {} != {}",
                        before_items.len(),
                        after_items.len()
                    ));
                    return;
                }
                for (index, (a, b)) in before_items.iter().zip(after_items.iter()).enumerate() {
                    walk(a, b, &format!("{path}[{index}]"), out);
                    if out.len() >= 8 {
                        return;
                    }
                }
            }
            _ => out.push(path.to_owned()),
        }
    }
    let mut out = Vec::new();
    walk(before, after, "$", &mut out);
    out
}

/// The saved project plus its revision and canonical signatures.
///
/// Two signatures are kept because each comparison must stay single-sided:
/// `signature` (Rust-canonical) feeds Rust-side asserts, while
/// `page_signature` (the page's own canonicalization, returned as a string so
/// no float repr crosses the bridge) feeds in-page waits. Mixing the sides
/// breaks waits on floats whose shortest form differs per parser.
struct Document {
    project: Value,
    revision: Value,
    page_signature: String,
}

async fn doc(page: &Page) -> Result<Document> {
    let expression = format!(
        r#"Promise.race([
    new Promise((resolve, reject) => {{
      const request = indexedDB.open({database});
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(Error('IndexedDB project read was blocked'));
      request.onsuccess = () => {{
        const db = request.result;
        try {{
          const query = db.transaction('projects').objectStore('projects').getAll();
          query.onerror = () => {{ db.close(); reject(query.error); }};
          query.onsuccess = () => {{
            db.close();
            if (query.result.length !== 1) {{ reject(Error('expected one local project')); return; }}
            const project = query.result[0];
            const revision = project.revision;
            delete project.revision;
            const canonical = value => Array.isArray(value) ? value.map(canonical)
              : value && typeof value === 'object'
                ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
                : value;
            resolve({{ project, revision, signature: JSON.stringify(canonical(project)) }});
          }};
        }} catch (error) {{
          db.close();
          reject(error);
        }}
      }};
    }}),
    new Promise((_, reject) => setTimeout(() => reject(Error('IndexedDB project read timed out')), 10000)),
  ])"#,
        database = serde_json::to_string(STORAGE_DATABASE).unwrap_or_default()
    );
    let read = tokio::time::timeout(Duration::from_secs(30), driver::eval(page, &expression))
        .await
        .map_err(|_| fail("IndexedDB project read timed out"))??;
    let mut project = read.get("project").cloned().unwrap_or(Value::Null);
    let revision = read.get("revision").cloned().unwrap_or(Value::Null);
    let page_signature = read
        .get("signature")
        .and_then(Value::as_str)
        .ok_or_else(|| fail("expected one local project"))?
        .to_owned();
    if let Value::Object(map) = &mut project {
        map.remove("revision");
    }
    Ok(Document {
        project,
        revision,
        page_signature,
    })
}

async fn scene(page: &Page) -> Result<Value> {
    driver::eval(
        page,
        r#"[...document.querySelectorAll('.m1-scene-part')]
    .map(element => [element.dataset.partId, element.getAttribute('transform')])"#,
    )
    .await
}

async fn saved(page: &Page) -> Result<()> {
    driver::wait_for(
        page,
        r#"document.querySelector('.m1-save-state')?.dataset.state === 'saved'"#,
        "a saved document",
        Duration::from_secs(20),
    )
    .await
}

async fn start_save_tracking(page: &Page) -> Result<()> {
    driver::eval(
        page,
        r#"(() => {
    window.__saveStates = [document.querySelector('.m1-save-state')?.dataset.state];
    new MutationObserver(() => {
      const state = document.querySelector('.m1-save-state')?.dataset.state;
      if (state && window.__saveStates.at(-1) !== state) window.__saveStates.push(state);
    }).observe(document.body, { attributes: true, childList: true, subtree: true, attributeFilter: ['data-state'] });
    return true;
  })()"#,
    )
    .await
    .map(|_| ())
}

async fn wait_for_save_cycle(page: &Page) -> Result<()> {
    driver::wait_for(
        page,
        r#"(() => {
    const states = window.__saveStates || [];
    const saving = states.indexOf('saving', 1);
    return saving >= 0 && states.slice(saving + 1).includes('saved');
  })()"#,
        "a save cycle",
        Duration::from_secs(20),
    )
    .await
}

/// Wait until the saved document's page-canonical form equals `page_signature`.
///
/// Both sides are page-canonicalized strings: the wait never compares a
/// Rust-canonical signature against the live document.
async fn wait_document(page: &Page, page_signature: &str) -> Result<()> {
    let expression = format!(
        r#"(async (expected, database) => {{
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
    if (!project) return false;
    delete project.revision;
    const canonical = value => Array.isArray(value) ? value.map(canonical)
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]))
        : value;
    return JSON.stringify(canonical(project)) === expected;
  }})({}, {})"#,
        serde_json::to_string(page_signature).unwrap_or_default(),
        serde_json::to_string(STORAGE_DATABASE).unwrap_or_default()
    );
    driver::wait_for(
        page,
        &expression,
        "the expected saved document",
        Duration::from_secs(20),
    )
    .await
}

/// Case collision findings count only raw surface observations between a case
/// body and the PCB, as today: an unavailable PCB never passes.
fn observed_case_pcb_pair(text: &str) -> bool {
    let pair = ["[plate", "[bottom"].iter().any(|tag| {
        let mut search = text;
        loop {
            let Some(open) = search.find(tag) else {
                break false;
            };
            let after_tag = &search[open + tag.len()..];
            let Some(close) = after_tag.find(']') else {
                break false;
            };
            let rest = &after_tag[close + 1..];
            if rest.starts_with(" ↔ PCB [pcb-reference]") {
                break true;
            }
            search = rest;
        }
    });
    pair && (text.contains("(surface intersection near")
        || text.contains("(surface intersection observed")
        || text.find("(surface separation ").is_some_and(|at| {
            text[at + "(surface separation ".len()..].starts_with(|c: char| c.is_ascii_digit())
        }))
}

/// One named check plus its details, flattened into the receipt.
struct Check {
    name: String,
    details: BTreeMap<String, Value>,
}

struct Smoke {
    page: Page,
    checks: Vec<Check>,
}

fn record(checks: &mut Vec<Check>, name: &str, details: BTreeMap<String, Value>) {
    checks.push(Check {
        name: name.to_owned(),
        details,
    });
}

fn detail(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

async fn set_slider(page: &Page, name: &str, target: i64) -> Result<()> {
    let query = format!(
        "document.querySelector('[role=\"slider\"][aria-label=\"{name}\"]')?.getAttribute('aria-valuenow')"
    );
    // The old runner's `press` focuses the slider first; bare key events would
    // land on whatever holds focus instead.
    driver::eval(
        page,
        &format!("document.querySelector('[role=\"slider\"][aria-label=\"{name}\"]')?.focus()"),
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

async fn select_scope(page: &Page, name: &str) -> Result<()> {
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
          label: event.target.closest?.('.m1-transform-handle')?.getAttribute('aria-label'),
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
struct LiveDrag {
    events: Value,
    capture: bool,
    scene: Value,
}

/// A drag with release discipline: every failure between begin and end still
/// ends the hold, mirroring the old runner's `release()` guards.
async fn drag(
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
      return !!down && [...document.querySelectorAll('svg.m1-canvas,.m1-transform-handle')]
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
fn assert_unavailable_pcb_pairs_do_not_count() -> Result<()> {
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

struct DragConfig {
    name: &'static str,
    selector: String,
    label: Option<&'static str>,
    dx: f64,
    dy: f64,
    with_control: bool,
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
async fn run_drag(
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

/// The default collision check runs the real page, worker and provider on a
/// 2 x 2 rigid-mount case. The physical PCB must reach a real query against a
/// generated case body.
async fn collision_scenario(
    browser: &Browser,
    url: &str,
    errors: &Arc<PageErrors>,
    checks: &mut Vec<Check>,
) -> Result<()> {
    assert_unavailable_pcb_pairs_do_not_count()?;
    let scenario_started = Instant::now();
    let (context, page) = driver::new_isolated_page(browser, "about:blank", errors).await?;
    let outcome = collision_scenario_inner(&page, url, checks, scenario_started).await;
    let disposal = driver::dispose_isolated_page(browser, Some(context), page, errors).await;
    scenario_verdict(outcome, disposal)
}

async fn goto(page: &Page, url: &str, what: &str) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(60), page.goto(url))
        .await
        .map_err(|_| fail(format!("{what} timed out after 60s")))?
        .map_err(|error| fail(format!("{what} failed: {error}")))?;
    Ok(())
}

async fn collision_scenario_inner(
    page: &Page,
    url: &str,
    checks: &mut Vec<Check>,
    scenario_started: Instant,
) -> Result<()> {
    goto(page, url, "opening the preview").await?;
    driver::click_role(page, None, "button", "Create new keyboard").await?;
    driver::click_role(page, None, "button", "Continue to layout").await?;
    set_slider(page, "Rows", 2).await?;
    set_slider(page, "Columns", 2).await?;
    driver::click_role(page, None, "button", "Create matrix").await?;
    driver::click_role(page, None, "button", "Open Case workspace").await?;
    let case_buttons = driver::find_role(page, None, "button", "").await?;
    let optional = case_buttons
        .iter()
        .find(|(_, label)| label.contains("Case (optional)"))
        .map(|(id, _)| *id);
    match optional {
        Some(id) => driver::click_backend(page, id).await?,
        None => return Err(fail("no Case (optional) control")),
    }
    select_combobox_option(page, "Case mounting style", "Rigid mount").await?;
    driver::wait_for(
        page,
        r#"[...document.querySelectorAll('[role="status"]')].some(element => element.textContent.includes('Exact case geometry ready.'))"#,
        "exact case geometry",
        Duration::from_secs(120),
    )
    .await?;
    let started = Instant::now();
    driver::click_role(page, None, "button", "Check collisions").await?;
    driver::wait_for(
        page,
        r#"(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      return panel && !panel.querySelector('[role="status"]')
        && [...panel.querySelectorAll('p')].some(element => element.textContent.includes('↔') || element.getAttribute('role') === 'alert');
    })()"#,
        "collision findings",
        Duration::from_secs(120),
    )
    .await?;
    let collision_ms = started.elapsed().as_millis() as i64;
    let layout = driver::eval(
        page,
        r#"(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      const container = panel.closest('.m1-case-panel');
      const viewer = document.querySelector('[aria-label^="Interactive 3D Case preview"]');
      const box = element => { const { x, y, width, height } = element.getBoundingClientRect(); return { x, y, width, height }; };
      const outside = element => !element.closest('.m1-case-collision-provenance');
      return {
        alerts: [...panel.querySelectorAll('p[role="alert"]')].map(element => element.textContent),
        findings: [...panel.querySelectorAll('p:not([role="alert"])')]
          .filter(element => outside(element) && element.textContent.includes('↔')).map(element => element.textContent),
        panel: { ...box(panel), scrollHeight: panel.scrollHeight, clientHeight: panel.clientHeight,
          overflowY: getComputedStyle(panel).overflowY },
        container: container && box(container),
        viewer: { ...box(viewer), display: getComputedStyle(viewer).display, visibility: getComputedStyle(viewer).visibility },
        viewport: { width: window.innerWidth, height: window.innerHeight },
      };
    })()"#,
    )
    .await?;
    let alerts = layout
        .get("alerts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !alerts.is_empty() {
        return Err(fail(format!(
            "the default collision check reported no collision error, including PCB unavailable: {}",
            serde_json::to_string(&alerts).unwrap_or_default()
        )));
    }
    let findings = layout
        .get("findings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let finding_texts: Vec<&str> = findings.iter().filter_map(Value::as_str).collect();
    let pcb_pairs = finding_texts
        .iter()
        .filter(|text| observed_case_pcb_pair(text))
        .count();
    if pcb_pairs == 0 {
        return Err(fail(format!(
            "a generated case body has a raw mesh observation against the physical PCB ({} findings)",
            finding_texts.len()
        )));
    }
    record(
        checks,
        "Default collision PCB pair",
        detail(&[
            ("findings", json!(finding_texts.len() as i64)),
            ("observedPcbPairs", json!(pcb_pairs as i64)),
            ("alerts", json!(0)),
            ("collisionMs", json!(collision_ms)),
        ]),
    );
    let panel = layout.get("panel").cloned().unwrap_or(Value::Null);
    let viewer = layout.get("viewer").cloned().unwrap_or(Value::Null);
    let container = layout.get("container").cloned().unwrap_or(Value::Null);
    let viewport = layout.get("viewport").cloned().unwrap_or(Value::Null);
    let number =
        |value: &Value, field: &str| value.get(field).and_then(Value::as_f64).unwrap_or(-1.0);
    let overflow_y = panel.get("overflowY").and_then(Value::as_str).unwrap_or("");
    if overflow_y != "auto" && overflow_y != "scroll" {
        return Err(fail(format!(
            "collision findings scroll inside their panel (overflow-y {overflow_y})"
        )));
    }
    let (scroll_height, client_height) = (
        number(&panel, "scrollHeight"),
        number(&panel, "clientHeight"),
    );
    if !(scroll_height > client_height && client_height > 0.0) {
        return Err(fail(format!(
            "collision findings overflow their panel (scrollHeight {scroll_height}, clientHeight {client_height})"
        )));
    }
    let viewer_display = viewer.get("display").and_then(Value::as_str).unwrap_or("");
    let viewer_visibility = viewer
        .get("visibility")
        .and_then(Value::as_str)
        .unwrap_or("");
    let (viewer_w, viewer_h) = (number(&viewer, "width"), number(&viewer, "height"));
    if !(viewer_display != "none"
        && viewer_visibility == "visible"
        && viewer_w > 0.0
        && viewer_h > 0.0)
    {
        return Err(fail(format!(
            "the Case viewer has a visible, nonzero rectangle ({viewer_w} x {viewer_h})"
        )));
    }
    let (viewer_x, viewer_y) = (number(&viewer, "x"), number(&viewer, "y"));
    let (viewport_w, viewport_h) = (number(&viewport, "width"), number(&viewport, "height"));
    if !(viewer_x >= 0.0
        && viewer_y >= 0.0
        && viewer_x + viewer_w <= viewport_w
        && viewer_y + viewer_h <= viewport_h)
    {
        return Err(fail("the Case viewer lies inside the desktop viewport"));
    }
    let (panel_y, panel_h) = (number(&panel, "y"), number(&panel, "height"));
    let (container_y, container_h) = (number(&container, "y"), number(&container, "height"));
    if container_y >= 0.0
        && !(panel_y >= container_y && panel_y + panel_h <= container_y + container_h)
    {
        return Err(fail(
            "the findings panel stays inside the Case panel instead of pushing past it",
        ));
    }
    if !(viewer_y + viewer_h <= panel_y + 1.0 || panel_y + panel_h <= viewer_y + 1.0) {
        return Err(fail(
            "the findings panel and the Case viewer do not overlap",
        ));
    }
    record(
        checks,
        "Case viewer with overflowing collision findings",
        detail(&[
            ("scrollHeight", json!(scroll_height as i64)),
            ("clientHeight", json!(client_height as i64)),
            ("overflowY", json!(overflow_y)),
            (
                "viewer",
                json!([viewer_w.round() as i64, viewer_h.round() as i64]),
            ),
            ("panelHeight", json!(panel_h.round() as i64)),
        ]),
    );
    println!(
        "drag-smoke: default collision check observed {pcb_pairs} PCB pairs in {collision_ms} ms (scenario {} ms)",
        scenario_started.elapsed().as_millis()
    );
    Ok(())
}

/// Authored product input, restored through normal project storage, like the
/// old runner's qualification fixture. No provider/report doubles.
async fn qualification_scenario(
    browser: &Browser,
    url: &str,
    repo_root: &str,
    errors: &Arc<PageErrors>,
    checks: &mut Vec<Check>,
    unavailable_pcb: bool,
) -> Result<()> {
    let (context, page) = driver::new_isolated_page(browser, "about:blank", errors).await?;
    let mut outcome =
        qualification_scenario_inner(&page, url, repo_root, checks, unavailable_pcb).await;
    if let Err(error) = &outcome {
        if let Ok(Value::String(body)) = driver::eval(
            &page,
            "document.querySelector('body') ? document.querySelector('body').innerText : ''",
        )
        .await
        {
            let mounted: String = body.chars().take(4000).collect();
            outcome = Err(fail(format!("{error}\nMounted page: {mounted}")));
        }
    }
    let disposal = driver::dispose_isolated_page(browser, Some(context), page, errors).await;
    scenario_verdict(outcome, disposal)
}

async fn qualification_scenario_inner(
    page: &Page,
    url: &str,
    repo_root: &str,
    checks: &mut Vec<Check>,
    unavailable_pcb: bool,
) -> Result<()> {
    use chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams;
    page.execute(AddScriptToEvaluateOnNewDocumentParams::from(
        r#"(() => {
    window.collisionPreparations = [];
    const postMessage = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function (message, ...args) {
      if (message?.frame) {
        const frame = JSON.parse(message.frame);
        if (frame.operation === 'read-step') window.collisionPreparations.push({
          sha: frame.sourceSha256, options: frame.stepImportOptions,
        });
      }
      return postMessage.call(this, message, ...args);
    };
  })()"#,
    ))
    .await
    .map_err(|error| fail(format!("cannot install the worker observer: {error}")))?;
    goto(page, url, "opening the preview").await?;
    driver::click_role(page, None, "button", "Create new keyboard").await?;
    driver::click_role(page, None, "button", "Continue to layout").await?;
    saved(page).await?;
    let rewrite = format!(
        r#"(async (unavailable) => new Promise((resolve, reject) => {{
    const request = indexedDB.open({database});
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {{
      const db = request.result;
      const transaction = db.transaction('projects', 'readwrite');
      const store = transaction.objectStore('projects');
      const query = store.getAll();
      query.onsuccess = () => {{
        if (query.result.length !== 1) {{ transaction.abort(); return; }}
        const document = query.result[0], board = document.boards[0];
        document.name = 'Qualification rectangle and original 0805';
        document.outline = [{{ id: board.outlineIds[0], kind: 'rect', center: {{ x: 10, y: 5 }},
          size: {{ x: 20, y: 10 }}, radius: 0, operation: 'add' }}];
        document.definitions = [{{ id: 'qualification-cap', name: 'Original 0805', kind: 'passive',
          courtyard: [{{ x: -.625, y: -1 }}, {{ x: .625, y: -1 }}, {{ x: .625, y: 1 }}, {{ x: -.625, y: 1 }}], pads: [],
          models: [{{ assetId: 'bundled-model:infused-kim/SMD_0805_Capacitor.step', offset: {{ x: 0, y: 0, z: 0 }},
            rotation: {{ x: 0, y: 0, z: 0 }}, scale: {{ x: 1, y: 1, z: 1 }} }}] }}];
        document.parts = [3, 6].map((y, index) => ({{ id: `qualification-cap-${{index}}`, definitionId: 'qualification-cap',
          reference: `C${{index + 1}}`, pose: {{ at: {{ x: 10, y }}, rotation: 0 }}, side: 'front', outline: {{ excluded: true }} }}));
        board.partIds = document.parts.map(part => part.id);
        document.mechanical = {{ boardId: board.id, method: 'printed', mount: 'rigid', plateThickness: 2,
          plateFoamThickness: .5, pcbThickness: 1.6, bottomFoamThickness: .4, batteryHeight: 4,
          bottomThickness: 3, plateToPcb: 3.5, wallThickness: 2, clearance: 0, bottomStyle: 'sheet',
          middleFrame: false, openingAllowance: 0, mounts: [], closureMounts: [],
          profiles: [] }};
        if (unavailable) {{
          delete document.mechanical;
          document.caseBodies = [0, 3].map((z, index) => ({{ id: index ? 'bottom' : 'plate', name: index ? 'Upper plate' : 'Plate',
            boardId: board.id, kind: 'plate', thickness: 2, clearance: 0, z }}));
        }}
        document.revision += 1;
        store.put(document);
      }};
      transaction.oncomplete = () => {{ db.close(); resolve(); }};
      transaction.onabort = transaction.onerror = () => {{ db.close(); reject(transaction.error || Error('fixture transaction aborted')); }};
    }};
  }}))({unavailable})"#,
        database = serde_json::to_string(STORAGE_DATABASE).unwrap_or_default(),
        unavailable = if unavailable_pcb { "true" } else { "false" }
    );
    driver::eval(page, &rewrite).await?;
    page.reload()
        .await
        .map_err(|error| fail(format!("reloading the fixture failed: {error}")))?;
    driver::click_role(page, None, "button", "Open Case workspace").await?;
    driver::wait_for(
        page,
        r#"[...document.querySelectorAll('[role="status"]')].some(element => element.textContent.includes('Exact case geometry ready.'))"#,
        "exact case geometry",
        Duration::from_secs(30),
    )
    .await
    ?;

    async fn run_check(page: &Page, button: &str) -> Result<FixtureRun> {
        driver::eval(
            page,
            "(() => { window.collisionPreparations = []; return true; })()",
        )
        .await?;
        driver::click_role(page, None, "button", button).await?;
        driver::wait_for(
            page,
            r#"(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      return panel && !panel.querySelector('[role="status"]') && panel.textContent.includes('↔');
    })()"#,
            "qualification findings",
            Duration::from_secs(120),
        )
        .await?;
        let observed = driver::eval(
            page,
            r#"(() => {
      const panel = document.querySelector('.m1-case-collision-checks');
      const outside = element => !element.closest('.m1-case-collision-provenance');
      return {
        summaries: [...panel.querySelectorAll('p')].filter(element => outside(element) && element.textContent.includes('↔')).map(element => element.textContent),
        alerts: [...panel.querySelectorAll('[role="alert"]')].map(element => element.textContent),
        provenance: [...panel.querySelectorAll('.m1-case-collision-provenance')].map(element => element.textContent),
        preparations: window.collisionPreparations,
      };
    })()"#,
        )
        .await?;
        Ok(FixtureRun {
            summaries: observed
                .get("summaries")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect(),
            alerts: observed
                .get("alerts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect(),
            provenance: observed
                .get("provenance")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect(),
            preparations: observed.get("preparations").cloned().unwrap_or(Value::Null),
        })
    }

    let defaults = run_check(page, "Check collisions").await?;
    if unavailable_pcb {
        if !defaults
            .alerts
            .iter()
            .any(|text| text.starts_with("PCB unavailable:"))
        {
            return Err(fail("the unavailable PCB run reports PCB unavailable"));
        }
        let pcb_pairs: Vec<&String> = defaults
            .summaries
            .iter()
            .filter(|text| text.contains("[pcb-reference]"))
            .collect();
        if pcb_pairs.len() != 4 {
            return Err(fail(format!(
                "the unavailable PCB run keeps 4 PCB pairs, saw {}",
                pcb_pairs.len()
            )));
        }
        if !pcb_pairs
            .iter()
            .all(|text| text.contains(": Uncertain (no mesh observation;"))
        {
            return Err(fail(
                "unavailable PCB pairs stay uncertain without mesh observation",
            ));
        }
        if !defaults
            .summaries
            .iter()
            .any(|text| text.contains("Clear [1.000, 1.000]"))
        {
            return Err(fail("the unavailable run keeps its supported Clear pair"));
        }
        record(
            checks,
            "Unavailable physical PCB stays uncertain",
            detail(&[
                ("alerts", json!(defaults.alerts.len() as i64)),
                ("pairs", json!(pcb_pairs.len() as i64)),
            ]),
        );
        return Ok(());
    }
    if !defaults.alerts.is_empty() {
        return Err(fail(
            "supported product fixture includes an available physical PCB and vendor assets",
        ));
    }
    if !defaults
        .summaries
        .iter()
        .any(|text| text.starts_with("C1") && text.contains("↔ C2") && text.contains(": Clear"))
    {
        return Err(fail("original 0805 pair reaches a supported Clear band"));
    }
    if !defaults.summaries.iter().any(|text| {
        (text.contains("[plate]") || text.contains("[bottom]"))
            && text.contains("[pcb-reference]")
            && text.contains(": Clear")
    }) {
        return Err(fail(
            "Case/physical-PCB generated pair reaches a supported Clear band",
        ));
    }
    if !defaults.summaries.iter().any(|text| {
        (text.starts_with("C1") || text.starts_with("C2"))
            && (text.contains("[plate]")
                || text.contains("[bottom]")
                || text.contains("[pcb-reference]"))
            && text.contains(": Clear")
    }) {
        return Err(fail(
            "mixed real vendor/generated pair reaches a supported Clear band",
        ));
    }
    if defaults.summaries.len() != 21 {
        return Err(fail(format!(
            "all seven real operands contribute every unordered pair, saw {}",
            defaults.summaries.len()
        )));
    }
    let pairs: std::collections::HashSet<&str> = defaults
        .summaries
        .iter()
        .map(|text| text.split(": ").next().unwrap_or(""))
        .collect();
    if pairs.len() != 21 {
        return Err(fail("every unordered pair is distinct"));
    }
    let vendor_details = defaults
        .provenance
        .first()
        .ok_or_else(|| fail("missing collision provenance"))?;
    let interval: Vec<f64> = vendor_details
        .split("Source surface distance: [")
        .nth(1)
        .and_then(|rest| rest.split(']').next())
        .map(|pair| {
            pair.split(',')
                .filter_map(|bound| bound.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();
    if interval.len() != 2 {
        return Err(fail(
            "structured source interval remains visible separately from rounded summary",
        ));
    }
    let contract_path =
        format!("{repo_root}/cad/wasm/tests/fixtures/generated-analysis-contract.json");
    let contract_text = std::fs::read_to_string(&contract_path)
        .map_err(|error| fail(format!("cannot read the qualification contract: {error}")))?;
    let contract: Value = serde_json::from_str(&contract_text)
        .map_err(|error| fail(format!("the qualification contract is not JSON: {error}")))?;
    let pair_case = contract
        .pointer("/qualificationV2/activation/pairCases/0")
        .ok_or_else(|| fail("the qualification contract has no pair cases"))?;
    let expected_interval: Vec<f64> = pair_case
        .get("expectedSourceIntervalMm")
        .and_then(Value::as_array)
        .map(|bounds| bounds.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let numerical_width: f64 = contract
        .pointer("/qualificationV2/activation/queryLimits/requestedNumericalWidthMm")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if !(interval[0] <= 1.0 && interval[1] >= 1.0) {
        return Err(fail("the 0805 source interval brackets 1 mm"));
    }
    if expected_interval.len() != 2
        || !(interval[0] >= expected_interval[0] - numerical_width
            && interval[1] <= expected_interval[1] + numerical_width)
    {
        return Err(fail("the 0805 source interval matches the contract"));
    }
    if vendor_details
        .matches("declared source precision 0.01 mm once")
        .count()
        != 2
    {
        return Err(fail("both 0805 placements declare source precision once"));
    }
    for marker in [
        "STEP representation #",
        "source face #",
        "conservative box pairs",
    ] {
        if !vendor_details.contains(marker) {
            return Err(fail(format!("provenance names {marker}")));
        }
    }
    if !defaults.provenance.iter().any(|text| {
        text.contains("generated body pcb-reference") && text.contains("source face index")
    }) {
        return Err(fail("provenance covers the generated PCB body"));
    }
    let preparations = defaults
        .preparations
        .as_array()
        .cloned()
        .unwrap_or_default();
    if !preparations.iter().all(|request| {
        request
            .get("options")
            .and_then(|options| options.get("targetDeflectionMm"))
            .is_none_or(Value::is_null)
    }) {
        return Err(fail("default checking does not request tight refinement"));
    }
    let tight = run_check(page, "Check at 0.01 mm").await?;
    let tight_preparations = tight.preparations.as_array().cloned().unwrap_or_default();
    if tight_preparations.len() != 1 {
        return Err(fail(format!(
            "two identical placements share one real provider preparation, saw {}",
            tight_preparations.len()
        )));
    }
    // Exact float equality would depend on the JSON parser's last bit; the
    // authored refinement target is exact at engineering precision.
    let tight_target = tight_preparations[0]
        .get("options")
        .and_then(|options| options.get("targetDeflectionMm"))
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    if (tight_target - 0.01).abs() >= 1e-9 {
        return Err(fail("the tight run requests 0.01 mm refinement"));
    }
    if !tight.alerts.is_empty() {
        return Err(fail("the tight run reports no alerts"));
    }
    if !tight
        .summaries
        .iter()
        .any(|text| text.starts_with("C1") && text.contains("↔ C2") && text.contains(": Uncertain"))
    {
        return Err(fail("0805 source precision refuses the tight request"));
    }
    if !tight
        .summaries
        .first()
        .is_some_and(|text| text.contains("exceeds requested accuracy 0.01 mm"))
    {
        return Err(fail("the tight refusal names the requested accuracy"));
    }
    if tight.summaries.len() != 21 {
        return Err(fail("the tight run keeps all 21 pairs"));
    }
    driver::click_role(page, None, "button", "Check collisions").await?;
    driver::click_role(page, None, "button", "Cancel check").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    // Only finding paragraphs (those containing ↔) count, like the old runner.
    let remaining_findings: i64 = driver::eval(
        page,
        r#"[...document.querySelector('.m1-case-collision-checks').querySelectorAll('p')].filter(element => element.textContent.includes('↔')).length"#,
    )
    .await?
    .as_i64()
    .unwrap_or(-1);
    if remaining_findings != 0 {
        return Err(fail("cancelled real page run cannot publish old findings"));
    }
    record(
        checks,
        "Qualified collision product fixture",
        detail(&[
            ("defaultPairs", json!(defaults.summaries.len() as i64)),
            (
                "supportedClear",
                json!(defaults
                    .summaries
                    .iter()
                    .filter(|text| text.contains(": Clear"))
                    .count() as i64),
            ),
            ("tightPairs", json!(tight.summaries.len() as i64)),
            ("vendorTightUnknown", json!(true)),
            ("realProviderWorkerRuntime", json!(true)),
        ]),
    );
    Ok(())
}

struct FixtureRun {
    summaries: Vec<String>,
    alerts: Vec<String>,
    provenance: Vec<String>,
    preparations: Value,
}

/// Choose a native select option by visible label, firing input/change.
async fn select_combobox_option(page: &Page, name: &str, option_label: &str) -> Result<()> {
    let expression = format!(
        r#"((name, optionLabel) => {{
    const root = document.querySelector(`[aria-label="${{name}}"]`)
      || [...document.querySelectorAll('[role="combobox"]')].find(el => el.getAttribute('aria-label') === name);
    if (!root) throw new Error(`missing combobox ${{name}}`);
    const select = root.tagName === 'SELECT' ? root : root.querySelector('select');
    if (!select) throw new Error(`combobox ${{name}} is not a native select`);
    const option = [...select.options].find(opt => (opt.text || opt.label || '').includes(optionLabel));
    if (!option) throw new Error(`missing option ${{optionLabel}} in ${{name}}`);
    select.value = option.value;
    select.dispatchEvent(new Event('input', {{ bubbles: true }}));
    select.dispatchEvent(new Event('change', {{ bubbles: true }}));
    return true;
  }})({}, {})"#,
        serde_json::to_string(name).unwrap_or_default(),
        serde_json::to_string(option_label).unwrap_or_default()
    );
    driver::eval(page, &expression).await.map(|_| ())
}

/// Parse `matrix/<id>/r<row>c<column>` from a part id.
fn parse_part_target(target: &str) -> Result<(String, i64, i64)> {
    let mut parts = target.split('/');
    let matrix_id = parts
        .nth(1)
        .ok_or_else(|| fail(format!("unexpected part id {target}")))?;
    let cell = parts
        .next()
        .ok_or_else(|| fail(format!("unexpected part id {target}")))?;
    let row: i64 = cell
        .strip_prefix('r')
        .and_then(|rest| rest.split('c').next())
        .and_then(|row| row.parse().ok())
        .ok_or_else(|| fail(format!("unexpected part id {target}")))?;
    let column: i64 = cell
        .split('c')
        .nth(1)
        .and_then(|column| column.parse().ok())
        .ok_or_else(|| fail(format!("unexpected part id {target}")))?;
    Ok((matrix_id.to_owned(), row, column))
}

async fn drag_scenarios(smoke: &mut Smoke) -> Result<()> {
    let page = smoke.page.clone();
    goto(
        &page,
        &env("BOARDSTUDIO_PREVIEW_URL")?,
        "opening the preview",
    )
    .await?;
    driver::click_role(&page, None, "button", "Create new keyboard").await?;
    driver::click_role(&page, None, "button", "Split keyboard").await?;
    driver::click_role(&page, None, "button", "Continue to layout").await?;
    set_slider(&page, "Rows", 10).await?;
    set_slider(&page, "Columns", 10).await?;
    driver::click_role(&page, None, "button", "Create matrix").await?;
    println!("drag-smoke: setup created a 10 x 10 matrix");
    let first_key =
        ".m1-scene-part[data-part-id^=\"matrix/\"][data-part-id$=\"r4c2\"] .m1-part-hit-area";
    driver::wait_for(
        &page,
        &format!(
            "!!document.querySelector({})",
            serde_json::to_string(first_key).unwrap_or_default()
        ),
        "the first key",
        Duration::from_secs(60),
    )
    .await?;
    driver::click_role(&page, None, "button", "Hide guide").await?;
    driver::click_role(&page, None, "button", "Fit board").await?;

    select_scope(&page, "Part").await?;
    driver::click_css(&page, first_key).await?;
    let target_part = driver::eval(
        &page,
        r#"document.querySelector('.m1-scene-part[data-part-id^="matrix/"][data-part-id$="r4c2"]').closest('[data-part-id]').dataset.partId"#,
    )
    .await?;
    let target_part = target_part
        .as_str()
        .ok_or_else(|| fail("no part id under the first key"))?;
    let (matrix_id, row, column) = parse_part_target(target_part)?;
    run_drag(
        smoke,
        &DragConfig {
            name: "Part drag",
            selector: format!(".m1-scene-part[data-part-id=\"{target_part}\"] .m1-part-hit-area"),
            label: None,
            dx: 36.0,
            dy: 22.0,
            with_control: false,
        },
        &["offset", "stagger", "cellOffset", "origin"],
        &["cellOffset"],
        &["cellOffset"],
        (matrix_id.clone(), row, column),
    )
    .await?;

    select_scope(&page, "Column").await?;
    driver::click_css(&page, first_key).await?;
    run_drag(
        smoke,
        &DragConfig {
            name: "Column body drag",
            selector: format!(".m1-scene-part[data-part-id=\"{target_part}\"] .m1-part-hit-area"),
            label: None,
            dx: 36.0,
            dy: 22.0,
            with_control: false,
        },
        &[
            "offset",
            "stagger",
            "cellOffset",
            "columnCellOffsets",
            "origin",
        ],
        &["columnCellOffsets"],
        &["columnCellOffsets", "cellOffset"],
        (matrix_id.clone(), row, column),
    )
    .await?;

    saved(&page).await?;
    let cancel_before = doc(&page).await?;
    let cancel_scene = scene(&page).await?;
    {
        let (live, mut hold) = drag(
            "Stagger cancel",
            &page,
            "[aria-label=\"Drag to stagger\"]",
            36.0,
            22.0,
            false,
            &cancel_scene,
        )
        .await?;
        if signature_of(&live.scene) == signature_of(&cancel_scene) {
            driver::end_drag(&page, &mut hold).await.ok();
            return Err(fail("Stagger cancel: a real preview appeared"));
        }
        driver::press_key(&page, "Escape", "Escape", 27).await?;
        driver::end_drag(&page, &mut hold).await?;
    }
    saved(&page).await?;
    wait_document(&page, &cancel_before.page_signature).await?;
    let cancel_after = doc(&page).await?;
    if cancel_after.revision != cancel_before.revision {
        return Err(fail("Stagger cancel created no saved revision"));
    }
    if signature_of(&scene(&page).await?) != signature_of(&cancel_scene) {
        return Err(fail("Stagger cancel restored the exact canvas scene"));
    }
    record(
        &mut smoke.checks,
        "Stagger cancel",
        detail(&[
            ("previewSeen", json!(true)),
            ("pointerCapture", json!(true)),
            ("exactDocument", json!(true)),
            ("exactScene", json!(true)),
        ]),
    );

    run_drag(
        smoke,
        &DragConfig {
            name: "Stagger handle",
            selector: "[aria-label=\"Drag to stagger\"]".to_owned(),
            label: Some("Drag to stagger"),
            dx: 36.0,
            dy: 22.0,
            with_control: false,
        },
        &[
            "offset",
            "stagger",
            "cellOffset",
            "columnCellOffsets",
            "origin",
        ],
        &["stagger"],
        &["stagger"],
        (matrix_id.clone(), row, column),
    )
    .await?;
    run_drag(
        smoke,
        &DragConfig {
            name: "Ctrl Offset handle",
            selector: "[aria-label=\"Drag to offset column\"]".to_owned(),
            label: Some("Drag to offset column"),
            dx: 36.0,
            dy: 22.0,
            with_control: true,
        },
        &[
            "offset",
            "stagger",
            "cellOffset",
            "columnCellOffsets",
            "origin",
        ],
        &["offset"],
        &["offset"],
        (matrix_id.clone(), row, column),
    )
    .await?;

    select_scope(&page, "Matrix").await?;
    driver::click_css(&page, first_key).await?;
    run_drag(
        smoke,
        &DragConfig {
            name: "Matrix move",
            selector: "[aria-label=\"Drag to move matrix\"]".to_owned(),
            label: Some("Drag to move matrix"),
            dx: 40.0,
            dy: 22.0,
            with_control: false,
        },
        &["origin"],
        &["origin"],
        &["origin"],
        (matrix_id.clone(), row, column),
    )
    .await?;
    Ok(())
}

/// The success receipt, byte-compatible with the old runner: field order and
/// compact JSON form feed the launcher's checksum verification.
#[derive(serde::Serialize)]
struct Receipt {
    schema_version: u32,
    run_id: String,
    started_at: String,
    completed_at: String,
    browser: String,
    viewport: Viewport,
    source_head: String,
    source_sha256: String,
    build_sha256: String,
    checks: Vec<ReceiptCheck>,
    passed: bool,
}

#[derive(serde::Serialize)]
struct Viewport {
    width: i64,
    height: i64,
}

#[derive(serde::Serialize)]
struct ReceiptCheck {
    name: String,
    #[serde(flatten)]
    details: BTreeMap<String, Value>,
}

fn receipt_json(receipt: &Receipt) -> String {
    // A receipt must never silently serialize to empty: the launcher checksums
    // these bytes, so fail loud instead of shipping a vacuous receipt.
    serde_json::to_string(receipt).expect("receipt serializes")
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(checks) => {
            println!("PASS drag browser smoke: {checks} checks");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<usize> {
    use sha2::{Digest, Sha256};
    let started_at = iso_now();
    let chrome = resolve_chrome()?;
    let preview_url = env("BOARDSTUDIO_PREVIEW_URL")?;
    let receipt_path = env("BOARDSTUDIO_DRAG_RECEIPT_PATH")?;
    let source_head = env("BOARDSTUDIO_DRAG_SOURCE_HEAD")?;
    let source_sha256 = env("BOARDSTUDIO_DRAG_SOURCE_SHA256")?;
    let build_sha256 = env("BOARDSTUDIO_DRAG_BUILD_SHA256")?;
    let repo_root = env("BOARDSTUDIO_DRAG_REPO_ROOT")?;
    let run_id = std::path::Path::new(&receipt_path)
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .ok_or_else(|| fail("receipt path has no run directory"))?
        .to_owned();

    let tmp_root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    let dirs = IsolatedDirs::create(std::path::Path::new(&tmp_root)).map_err(|error| {
        fail(format!(
            "drag browser smoke cannot isolate its profile: {error}"
        ))
    })?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let page_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(
        handler,
        Arc::clone(&handler_errors),
        Arc::clone(&page_errors),
    );
    let outcome = async {
        let version = tokio::time::timeout(Duration::from_secs(60), browser.version())
            .await
            .map_err(|_| fail("cannot read the browser version: timed out after 60s"))?
            .map_err(|error| fail(format!("cannot read the browser version: {error}")))?;
        let product = version.product;
        // Playwright reports the bare version; strip any `Chrome/` product prefix.
        let browser_version = product.rsplit('/').next().unwrap_or(&product).to_owned();
        println!("drag-smoke: browser {browser_version} (chromiumoxide 0.9.1)");

        // Every launch-path await carries a bound: a wedged browser must fail
        // loudly, never hang the smoke past its step timeout.
        let page = tokio::time::timeout(Duration::from_secs(60), browser.new_page("about:blank"))
            .await
            .map_err(|_| fail("cannot open a page: timed out after 60s"))?
            .map_err(|error| fail(format!("cannot open a page: {error}")))?;
        tokio::time::timeout(
            Duration::from_secs(60),
            driver::spawn_page_error_listener(&page, Arc::clone(&page_errors)),
        )
        .await
        .map_err(|_| fail("cannot listen for page errors: timed out after 60s"))??;
        let mut smoke = Smoke {
            page,
            checks: Vec::new(),
        };

        let scenarios = async {
            drag_scenarios(&mut smoke).await?;
            collision_scenario(&browser, &preview_url, &page_errors, &mut smoke.checks).await?;
            qualification_scenario(
                &browser,
                &preview_url,
                &repo_root,
                &page_errors,
                &mut smoke.checks,
                false,
            )
            .await?;
            qualification_scenario(
                &browser,
                &preview_url,
                &repo_root,
                &page_errors,
                &mut smoke.checks,
                true,
            )
            .await?;
            if smoke.checks.len() < 11 {
                return Err(fail(format!(
                    "smoke completed all drag, cancellation, Undo and collision checks: saw {}",
                    smoke.checks.len()
                )));
            }
            Ok::<_, Error>(())
        }
        .await;
        let disposal =
            driver::dispose_isolated_page(&browser, None, smoke.page, &page_errors).await;
        scenario_verdict(scenarios, disposal)?;
        Ok::<_, Error>((browser_version, smoke.checks))
    }
    .await;
    let (shutdown, confirmed) = shutdown::finish_browser(browser, drain, &handler_errors).await;
    if !confirmed {
        eprintln!(
            "browser termination unconfirmed; preserving profile at {}",
            dirs.profile_dir.display()
        );
        std::mem::forget(dirs);
    }
    let page = page_errors.fail_if_any();
    let errors: Vec<String> = [
        outcome.as_ref().err(),
        shutdown.as_ref().err(),
        page.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    .map(ToString::to_string)
    .collect();
    if !errors.is_empty() {
        return Err(fail(errors.join("; ")));
    }
    let (browser_version, checks) = outcome?;

    let receipt = Receipt {
        schema_version: 1,
        run_id,
        started_at,
        completed_at: iso_now(),
        browser: browser_version,
        viewport: Viewport {
            width: i64::from(driver::VIEWPORT_WIDTH),
            height: i64::from(driver::VIEWPORT_HEIGHT),
        },
        source_head,
        source_sha256,
        build_sha256,
        checks: checks
            .into_iter()
            .map(|check| ReceiptCheck {
                name: check.name,
                details: check.details,
            })
            .collect(),
        passed: true,
    };
    // The checksum covers the compact receipt without its own hash, exactly as
    // the launcher re-computes it. `preserve_order` keeps the declared field
    // order stable in both the digest input and the file.
    let payload = receipt_json(&receipt);
    let digest = hex::encode(Sha256::digest(payload.as_bytes()));
    write_receipt(std::path::Path::new(&receipt_path), &receipt, &digest)?;
    Ok(receipt.checks.len())
}

/// Write the receipt create-new: never emit or overwrite a success receipt
/// for a partial run.
fn write_receipt(path: &std::path::Path, receipt: &Receipt, digest: &str) -> Result<()> {
    use std::io::Write;
    let mut text = receipt_json(receipt);
    text.pop();
    text.push_str(&format!(",\"receipt_sha256\":\"{digest}\"}}"));
    // Pretty form matches the old runner: 2-space indent.
    let pretty: Value = serde_json::from_str(&text)
        .map_err(|error| fail(format!("cannot encode the receipt: {error}")))?;
    let pretty = serde_json::to_string_pretty(&pretty).unwrap_or_default() + "\n";
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| fail(format!("cannot create the receipt directory: {error}")))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            fail(format!(
                "cannot write the receipt to {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(pretty.as_bytes())
        .map_err(|error| fail(format!("cannot write the receipt: {error}")))?;
    Ok(())
}

mod hex {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let mut text = String::with_capacity(bytes.as_ref().len() * 2);
        for byte in bytes.as_ref() {
            text.push(DIGITS[(byte >> 4) as usize] as char);
            text.push(DIGITS[(byte & 0xf) as usize] as char);
        }
        text
    }
}
