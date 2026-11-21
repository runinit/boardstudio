//! Real-page drag smoke driven by the Rust browser harness.
//!
//! A faithful port of the historical `docs/archive/playwright-runners/test-drag-interactions.cjs`: trusted pointer
//! drags over Part, Column, Stagger and Ctrl+Offset handles, live preview,
//! pointer capture, durable saves, exact Undo/reload, collision observations
//! and receipt/provenance validation. The old runner stays available for
//! same-build comparison until the parity gate passes.

#[path = "drag_smoke/gestures.rs"]
mod gestures;
#[path = "drag_smoke/scenarios.rs"]
mod scenarios;
use scenarios::{collision_scenario, drag_scenarios, qualification_scenario};

#[path = "../shutdown.rs"]
mod shutdown;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use boardstudio_browser_harness::driver::{self, DragHold, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};

use boardstudio_browser_harness::cdp::{Browser, EventKind, Page};
use futures::StreamExt;
use serde_json::{json, Value};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct NetworkObservation {
    session: String,
    method: String,
    params: Value,
}

impl EventKind for NetworkObservation {
    fn method_id() -> &'static str {
        "BoardStudio.NetworkEvent"
    }
}

async fn observe_network(
    page: &Page,
    shared: Arc<std::sync::Mutex<Vec<NetworkObservation>>>,
) -> Result<tokio::task::JoinHandle<Vec<NetworkObservation>>> {
    let mut events = page
        .event_listener::<NetworkObservation>()
        .await
        .map_err(|error| fail(format!("cannot listen for network observations: {error}")))?;
    Ok(tokio::spawn(async move {
        let mut observations = Vec::new();
        while let Some(event) = events.next().await {
            observations.push((*event).clone());
            shared.lock().unwrap().push((*event).clone());
        }
        observations
    }))
}

async fn finish_network_observation(
    listener: Option<tokio::task::JoinHandle<Vec<NetworkObservation>>>,
    registration_error: Option<&str>,
    timeout: Duration,
) -> Value {
    let unavailable = |reason: String| {
        json!({
            "scope": "drag-page-and-attached-workers", "complete": null, "reason": reason
        })
    };
    let Some(mut listener) = listener else {
        return unavailable(
            registration_error
                .unwrap_or("listener unavailable")
                .to_owned(),
        );
    };
    match tokio::time::timeout(timeout, &mut listener).await {
        Ok(Ok(events)) if !events.is_empty() => json!({
            "scope": "drag-page-and-attached-workers", "complete": null, "events": events
        }),
        Ok(Ok(_)) => unavailable("listener returned no events".to_owned()),
        Ok(Err(error)) => unavailable(format!("listener failed: {error}")),
        Err(_) => {
            listener.abort();
            unavailable("listener did not finish before timeout".to_owned())
        }
    }
}

// The current frame and loader identify the document, including after redirects.
fn document_request(
    events: &[NetworkObservation],
    session: &str,
    frame: &str,
    loader: &str,
    url: &str,
) -> Option<(String, Value)> {
    let mut candidate: Option<(String, bool, Option<Value>, bool)> = None;
    for event in events.iter().filter(|e| e.session == session) {
        let p = &event.params;
        if event.method == "Network.requestWillBeSent" && p["type"] == "Document" {
            if p["frameId"].as_str()? != frame || p["loaderId"].as_str()? != loader {
                continue;
            }
            let id = p["requestId"].as_str()?;
            if let Some((previous, _, _, _)) = &candidate {
                if previous != id || p.get("redirectResponse").is_none() {
                    return None;
                }
            }
            candidate = Some((
                id.to_owned(),
                p["request"]["url"].as_str()? == url,
                None,
                false,
            ));
        } else if let Some((id, matches_url, response, finished)) = candidate.as_mut() {
            if p["requestId"].as_str() != Some(id.as_str()) {
                continue;
            }
            match event.method.as_str() {
                "Network.responseReceived" => {
                    if p["type"] != "Document"
                        || p["frameId"].as_str()? != frame
                        || p["loaderId"].as_str()? != loader
                        || p["response"]["url"].as_str()? != url
                        || !*matches_url
                        || response.is_some()
                    {
                        return None;
                    }
                    let r = &p["response"];
                    *response = Some(
                        json!({"from_disk_cache": r.get("fromDiskCache").and_then(Value::as_bool), "from_service_worker": r.get("fromServiceWorker").and_then(Value::as_bool), "from_prefetch_cache": r.get("fromPrefetchCache").and_then(Value::as_bool)}),
                    );
                }
                "Network.loadingFinished" => *finished = response.is_some(),
                "Network.loadingFailed" => *finished = false,
                _ => {}
            }
        }
    }
    let (id, _, response, finished) = candidate?;
    if finished {
        Some((id, response?))
    } else {
        None
    }
}

async fn document_body(
    page: &Page,
    events: &[NetworkObservation],
    preview: &str,
    expected: &str,
) -> Value {
    use base64::Engine;
    let unknown =
        |reason: String| json!({"scope":"page-document-only", "matches":null, "reason":reason});
    let _ = preview;
    let frame = |tree: &Value| {
        tree.get("frameTree")
            .and_then(|tree| tree.get("frame"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let frame_tree = match tokio::time::timeout(
        Duration::from_secs(5),
        page.execute("Page.getFrameTree", json!({})),
    )
    .await
    {
        Ok(Ok(tree)) => frame(&tree),
        _ => return unknown("cannot determine current main frame".into()),
    };
    let field = |name: &str| frame_tree.get(name).and_then(Value::as_str).unwrap_or("");
    let Some((id, provenance)) = document_request(
        events,
        page.session_id().as_ref(),
        field("id"),
        field("loaderId"),
        field("url"),
    ) else {
        return unknown("ambiguous or incomplete current main-frame document".into());
    };
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        page.execute("Network.getResponseBody", json!({"requestId": id})),
    )
    .await;
    let body = match result {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return unknown(format!("body retrieval failed: {e}")),
        Err(_) => return unknown("body retrieval timed out".into()),
    };
    let bytes = if body.get("base64Encoded").and_then(Value::as_bool) == Some(true) {
        match base64::engine::general_purpose::STANDARD
            .decode(body.get("body").and_then(Value::as_str).unwrap_or_default())
        {
            Ok(b) => b,
            Err(e) => return unknown(format!("invalid body encoding: {e}")),
        }
    } else {
        body.get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .as_bytes()
            .to_vec()
    };
    if bytes.len() > 4 * 1024 * 1024 {
        return unknown(format!("body exceeds 4 MiB: {} bytes", bytes.len()));
    }
    let current = match tokio::time::timeout(
        Duration::from_secs(5),
        page.execute("Page.getFrameTree", json!({})),
    )
    .await
    {
        Ok(Ok(tree)) => frame(&tree),
        _ => return unknown("cannot verify current main frame".into()),
    };
    if current.get("id") != frame_tree.get("id")
        || current.get("loaderId") != frame_tree.get("loaderId")
        || current.get("url") != frame_tree.get("url")
    {
        return unknown("main-frame navigation changed during body capture".into());
    }
    let mut digest = document_digest(&bytes, expected);
    digest["provenance"] = provenance;
    digest
}

fn attach_document_snapshot(
    observations: &mut Value,
    snapshot: &[NetworkObservation],
    document: Value,
) {
    observations["events"] = json!(snapshot);
    observations["document_body"] = document;
}

fn document_digest(bytes: &[u8], expected: &str) -> Value {
    use sha2::{Digest, Sha256};
    let digest = hex::encode(Sha256::digest(bytes));
    json!({"scope":"page-document-only", "matches":digest == expected, "sha256":digest, "expected_sha256":expected, "bytes":bytes.len()})
}

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
    fn document_selection_binds_frame_loader_and_preserves_unknown_provenance() {
        let event = |method: &str, params: Value| NetworkObservation {
            session: "page".into(),
            method: method.into(),
            params,
        };
        let events = vec![
            event(
                "Network.requestWillBeSent",
                json!({"requestId":"iframe", "type":"Document", "frameId":"child", "loaderId":"child", "request":{"url":"http://example.test/"}}),
            ),
            event(
                "Network.requestWillBeSent",
                json!({"requestId":"1", "type":"Document", "frameId":"main", "loaderId":"new", "request":{"url":"http://example.test/old"}}),
            ),
            event(
                "Network.requestWillBeSent",
                json!({"requestId":"1", "type":"Document", "frameId":"main", "loaderId":"new", "redirectResponse":{}, "request":{"url":"http://example.test/new"}}),
            ),
            event(
                "Network.responseReceived",
                json!({"requestId":"1", "type":"Document", "frameId":"main", "loaderId":"new", "response":{"url":"http://example.test/new", "fromDiskCache":false}}),
            ),
            event("Network.loadingFinished", json!({"requestId":"1"})),
        ];
        let selected =
            document_request(&events, "page", "main", "new", "http://example.test/new").unwrap();
        assert_eq!(selected.0, "1");
        assert_eq!(selected.1["from_disk_cache"], false);
        assert_eq!(selected.1["from_service_worker"], Value::Null);
        assert!(document_request(&events, "page", "main", "later", "http://other.test/").is_none());
        assert!(document_request(
            &events[..4],
            "page",
            "main",
            "new",
            "http://example.test/new"
        )
        .is_none());
        let mut missing = events.clone();
        missing[3]
            .params
            .as_object_mut()
            .unwrap()
            .remove("loaderId");
        assert!(
            document_request(&missing, "page", "main", "new", "http://example.test/new").is_none()
        );
    }

    #[tokio::test]
    async fn interrupted_capture_keeps_comparison_snapshot() {
        let listener =
            tokio::spawn(async { std::future::pending::<Vec<NetworkObservation>>().await });
        let mut result =
            finish_network_observation(Some(listener), None, Duration::from_millis(1)).await;
        let events = vec![NetworkObservation {
            session: "page".into(),
            method: "Network.requestWillBeSent".into(),
            params: json!({"requestId":"1"}),
        }];
        attach_document_snapshot(&mut result, &events, json!({"matches":null}));
        assert_eq!(result["reason"], "listener did not finish before timeout");
        assert_eq!(result["events"][0]["params"]["requestId"], "1");
        assert_eq!(result["document_body"]["matches"], Value::Null);
    }

    #[test]
    fn document_digest_reports_match_and_mismatch() {
        let actual = document_digest(b"page", "wrong");
        assert_eq!(actual["matches"], false);
        assert_eq!(
            document_digest(b"page", actual["sha256"].as_str().unwrap())["matches"],
            true
        );
    }

    #[test]
    fn receipt_float_fixture_matches_python_regression() {
        assert_eq!(serde_json::to_string(&1e-7_f64).unwrap(), "1e-7");
    }

    #[tokio::test]
    async fn unavailable_network_observation_reports_reason() {
        let result = finish_network_observation(
            None,
            Some("registration timed out"),
            Duration::from_millis(1),
        )
        .await;
        assert_eq!(result["complete"], Value::Null);
        assert_eq!(result["reason"], "registration timed out");
    }

    #[test]
    fn network_observation_preserves_identity_and_unknown_cache_fields() {
        let event = json!({"session":"session-1", "method":"Network.responseReceived", "params": {
            "requestId":"request-1", "response":{"url":"https://example.test/", "status":200}
        }});
        let observed: NetworkObservation = serde_json::from_value(event.clone()).unwrap();
        assert_eq!(serde_json::to_value(observed).unwrap(), event);
        assert!(event["params"]["response"]
            .get("fromServiceWorker")
            .is_none());
    }

    #[test]
    fn suite_selection_parses_all_names_and_rejects_unknown() {
        assert_eq!(parse_suite("all").unwrap(), Suite::All);
        assert_eq!(parse_suite("drag").unwrap(), Suite::Drag);
        assert_eq!(parse_suite("collision").unwrap(), Suite::Collision);
        assert!(parse_suite("").is_err());
        assert!(parse_suite("Drag").is_err());
        assert!(parse_suite("collisions").is_err());
    }

    #[test]
    fn suite_check_counts_partition_all_fifteen_checks() {
        assert_eq!(Suite::All.expected_checks(), 15);
        assert_eq!(Suite::Drag.expected_checks(), 11);
        assert_eq!(Suite::Collision.expected_checks(), 4);
        assert_eq!(
            Suite::Drag.expected_checks() + Suite::Collision.expected_checks(),
            Suite::All.expected_checks()
        );
        assert!(Suite::All.runs_drag() && Suite::All.runs_collision());
        assert!(Suite::Drag.runs_drag() && !Suite::Drag.runs_collision());
        assert!(!Suite::Collision.runs_drag() && Suite::Collision.runs_collision());
    }

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

/// Which scenarios this run executes. `all` preserves the historical
/// single-run behavior; `drag` and `collision` split it into two
/// parallel-capable suites with disjoint checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suite {
    All,
    Drag,
    Collision,
}

impl Suite {
    fn runs_drag(self) -> bool {
        matches!(self, Suite::All | Suite::Drag)
    }

    fn runs_collision(self) -> bool {
        matches!(self, Suite::All | Suite::Collision)
    }

    /// The exact check count a successful run of this suite records, so a
    /// silently dropped scenario fails the run instead of passing short.
    fn expected_checks(self) -> usize {
        match self {
            Suite::All => 15,
            Suite::Drag => 11,
            Suite::Collision => 4,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Suite::All => "drag, cancellation, Undo and collision",
            Suite::Drag => "drag, cancellation and Undo",
            Suite::Collision => "collision",
        }
    }
}

fn parse_suite(value: &str) -> Result<Suite> {
    match value {
        "all" => Ok(Suite::All),
        "drag" => Ok(Suite::Drag),
        "collision" => Ok(Suite::Collision),
        other => Err(fail(format!(
            "unknown drag smoke suite {other:?}; expected one of all, drag, collision"
        ))),
    }
}

/// The selected suite: `--suite <name>` wins, then `BOARDSTUDIO_DRAG_SUITE`,
/// then `all`. Unknown arguments fail loud so a typo cannot silently run the
/// wrong suite.
fn select_suite() -> Result<Suite> {
    let mut args = std::env::args().skip(1);
    let mut flag: Option<String> = None;
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--suite=") {
            flag = Some(value.to_owned());
        } else if arg == "--suite" {
            flag =
                Some(args.next().ok_or_else(|| {
                    fail("drag smoke --suite needs a value: all, drag or collision")
                })?);
        } else {
            return Err(fail(format!("unexpected drag smoke argument {arg:?}")));
        }
    }
    match flag {
        Some(value) => parse_suite(&value),
        None => match std::env::var("BOARDSTUDIO_DRAG_SUITE") {
            Ok(value) if !value.is_empty() => parse_suite(&value),
            _ => Ok(Suite::All),
        },
    }
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
        r#"[...document.querySelectorAll('[data-geometry="scene-part"]')]
    .map(element => [element.dataset.partId, element.getAttribute('transform')])"#,
    )
    .await
}

async fn saved(page: &Page) -> Result<()> {
    driver::wait_for(
        page,
        r#"document.querySelector('.header-save')?.dataset.state === 'saved'"#,
        "a saved document",
        Duration::from_secs(20),
    )
    .await
}

async fn start_save_tracking(page: &Page) -> Result<()> {
    driver::eval(
        page,
        r#"(() => {
    window.__saveStates = [document.querySelector('.header-save')?.dataset.state];
    new MutationObserver(() => {
      const state = document.querySelector('.header-save')?.dataset.state;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    network_observations: Option<Value>,
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
    // Fail fast on an unknown suite, before launching Chrome.
    let suite = select_suite()?;
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
    let (browser, mut handler) = driver::launch_chrome(&chrome, &dirs).await?;
    handler.enable_worker_network_capture();
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
        println!("drag-smoke: browser {browser_version} (direct CDP)");

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
        let network_events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (listener, registration_error) = match tokio::time::timeout(
            Duration::from_secs(5),
            observe_network(&page, Arc::clone(&network_events)),
        )
        .await
        {
            Ok(Ok(listener)) => (Some(listener), None),
            Ok(Err(error)) => (None, Some(format!("listener registration failed: {error}"))),
            Err(_) => (None, Some("listener registration timed out".to_owned())),
        };
        let mut smoke = Smoke {
            page,
            checks: Vec::new(),
        };

        let scenarios = async {
            if suite.runs_drag() {
                drag_scenarios(&mut smoke).await?;
            }
            if suite.runs_collision() {
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
            }
            let expected = suite.expected_checks();
            if smoke.checks.len() != expected {
                return Err(fail(format!(
                    "smoke completed all {} checks: saw {} of {expected}",
                    suite.describe(),
                    smoke.checks.len()
                )));
            }
            Ok::<_, Error>(())
        }
        .await;
        let snapshot = network_events.lock().unwrap().clone();
        let document = if listener.as_ref().is_none_or(|task| task.is_finished()) {
            json!({"scope":"page-document-only", "matches":null, "reason":"network listener unavailable or ended before capture"})
        } else {
            document_body(
                &smoke.page,
                &snapshot,
                &preview_url,
                &env("BOARDSTUDIO_DRAG_INDEX_SHA256")?,
            ).await
        };
        let document = if listener.as_ref().is_none_or(|task| task.is_finished()) {
            json!({"scope":"page-document-only", "matches":null, "reason":"network listener ended during capture"})
        } else {
            document
        };
        let disposal =
            driver::dispose_isolated_page(&browser, None, smoke.page, &page_errors).await;
        let mut observations = finish_network_observation(
            listener,
            registration_error.as_deref(),
            Duration::from_secs(5),
        )
        .await;
        attach_document_snapshot(&mut observations, &snapshot, document);
        let observations = Some(observations);
        scenario_verdict(scenarios, disposal)?;
        Ok::<_, Error>((browser_version, smoke.checks, observations))
    }
    .await;
    let (shutdown, confirmed) =
        shutdown::finish_browser(browser, drain, &handler_errors, outcome.is_ok()).await;
    let cleanup = if confirmed {
        shutdown::remove_profile(&dirs)
    } else {
        Ok(())
    };
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
        cleanup.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    .map(ToString::to_string)
    .collect();
    if !errors.is_empty() {
        return Err(fail(errors.join("; ")));
    }
    let (browser_version, checks, network_observations) = outcome?;

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
        network_observations,
    };
    // The checksum covers the compact receipt without its own hash, exactly as
    // the launcher re-computes it. Struct field order is stable in both the
    // digest input and the file.
    let payload = receipt_json(&receipt);
    let digest = hex::encode(Sha256::digest(payload.as_bytes()));
    write_receipt(std::path::Path::new(&receipt_path), payload, &digest)?;
    Ok(receipt.checks.len())
}

/// Write the receipt create-new: never emit or overwrite a success receipt
/// for a partial run.
fn write_receipt(path: &std::path::Path, mut text: String, digest: &str) -> Result<()> {
    use std::io::Write;
    text.pop();
    text.push_str(&format!(",\"receipt_sha256\":\"{digest}\"}}"));
    text.push('\n');
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
    file.write_all(text.as_bytes())
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
