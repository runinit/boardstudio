use super::gestures::{
    assert_unavailable_pcb_pairs_do_not_count, drag, run_drag, select_scope, set_slider, DragConfig,
};
use super::*;

/// The default collision check runs the real page, worker and provider on a
/// 2 x 2 rigid-mount case. The physical PCB must reach a real query against a
/// generated case body.
pub(super) async fn collision_scenario(
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
    driver::create_new_keyboard(page).await?;
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
    // Case basics live in the setup guide, which overlays the workspace's
    // collision actions until the user dismisses it (as in the drag scenario).
    driver::click_role(page, None, "button", "Hide guide").await?;
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
      const panel = document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])');
      return panel && !panel.querySelector('[role="status"]')
        && (!!panel.querySelector('[role="alert"]')
          || [...panel.querySelectorAll('p')].some(element => element.textContent.includes('↔')));
    })()"#,
        "collision findings",
        Duration::from_secs(120),
    )
    .await?;
    let collision_ms = started.elapsed().as_millis() as i64;
    let layout = driver::eval(
        page,
        r#"(() => {
      const panel = document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])');
      const container = panel.closest('[aria-label="Case assembly"]');
      const viewer = document.querySelector('[aria-label^="Interactive 3D Case preview"]');
      const box = element => { const { x, y, width, height } = element.getBoundingClientRect(); return { x, y, width, height }; };
      const outside = element => !element.closest('[data-slot="collapsible"]:has(> [aria-label="Witness identity"])');
      return {
        alerts: [...panel.querySelectorAll('[role="alert"]')].map(element => element.textContent),
        findings: [...panel.querySelectorAll('p:not([role="alert"])')]
          .filter(element => outside(element) && element.textContent.includes('↔')).map(element => element.textContent),
        panel: { ...box(panel), scrollHeight: panel.scrollHeight, clientHeight: panel.clientHeight,
          overflowY: getComputedStyle(panel).overflowY },
        container: box(container),
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
pub(super) async fn qualification_scenario(
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
    page.execute(
        "Page.addScriptToEvaluateOnNewDocument",
        json!({"source":
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
  })()"#}),
    )
    .await
    .map_err(|error| fail(format!("cannot install the worker observer: {error}")))?;
    goto(page, url, "opening the preview").await?;
    driver::create_new_keyboard(page).await?;
    driver::click_role(page, None, "button", "Continue to layout").await?;
    driver::click_role(page, None, "button", "Hide guide").await?;
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
      const panel = document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])');
      return panel && !panel.querySelector('[role="status"]') && panel.textContent.includes('↔');
    })()"#,
            "qualification findings",
            Duration::from_secs(120),
        )
        .await?;
        let observed = driver::eval(
            page,
            r#"(() => {
      const panel = document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])');
      const outside = element => !element.closest('[data-slot="collapsible"]:has(> [aria-label="Witness identity"])');
      return {
        summaries: [...panel.querySelectorAll('p')].filter(element => outside(element) && element.textContent.includes('↔')).map(element => element.textContent),
        alerts: [...panel.querySelectorAll('[role="alert"]')].map(element => element.textContent),
        provenance: [...panel.querySelectorAll('[data-slot="collapsible"]:has(> [aria-label="Witness identity"])')].map(element => element.textContent),
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
        r#"[...document.querySelector('[data-slot="collapsible"]:has(> [aria-label="Collision check"])').querySelectorAll('p')].filter(element => element.textContent.includes('↔')).length"#,
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

/// Drive the shared Select through its accessible trigger and scoped option.
/// Trusted input is required: assigning a value bypasses the production control.
async fn select_combobox_option(page: &Page, name: &str, option_label: &str) -> Result<()> {
    let timeout = Duration::from_secs(5);
    driver::click_role_timeout(page, None, "button", name, timeout).await?;
    driver::wait_for(
        page,
        &format!(
            r#"[...document.querySelectorAll('[role="listbox"]')].some(el => el.getAttribute('aria-label') === {} && el.getBoundingClientRect().height > 0)"#,
            json!(name)
        ),
        &format!("open listbox {name}"),
        timeout,
    )
    .await?;
    let lists = driver::find_role(page, None, "listbox", name).await?;
    let [(list, _)] = lists.as_slice() else {
        return Err(fail(format!(
            "expected one open listbox {name}, saw {}",
            lists.len()
        )));
    };
    driver::click_role_timeout(page, Some(*list), "option", option_label, timeout).await?;
    driver::wait_for(
        page,
        &format!(
        r#"((name, optionLabel) => {{
    const trigger = [...document.querySelectorAll('button')].find(el => el.getAttribute('aria-label') === name);
    return trigger?.textContent.trim() === optionLabel && trigger.getAttribute('aria-expanded') === 'false';
  }})({}, {})"#,
        json!(name), json!(option_label)
        ),
        &format!("{name} selected {option_label}"),
        timeout,
    ).await
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

pub(super) async fn drag_scenarios(smoke: &mut Smoke) -> Result<()> {
    let page = smoke.page.clone();
    goto(
        &page,
        &env("BOARDSTUDIO_PREVIEW_URL")?,
        "opening the preview",
    )
    .await?;
    driver::create_new_keyboard(&page).await?;
    driver::click_role(&page, None, "button", "Split keyboard").await?;
    driver::click_role(&page, None, "button", "Continue to layout").await?;
    set_slider(&page, "Rows", 10).await?;
    set_slider(&page, "Columns", 10).await?;
    driver::click_role(&page, None, "button", "Create matrix").await?;
    println!("drag-smoke: setup created a 10 x 10 matrix");
    let first_key =
        "[data-geometry='scene-part'][data-part-id^=\"matrix/\"][data-part-id$=\"r4c2\"] [data-geometry='part-hit-area']";
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
        r#"document.querySelector('[data-geometry="scene-part"][data-part-id^="matrix/"][data-part-id$="r4c2"]').dataset.partId"#,
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
            selector: format!("[data-geometry='scene-part'][data-part-id=\"{target_part}\"] [data-geometry='part-hit-area']"),
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
            selector: format!("[data-geometry='scene-part'][data-part-id=\"{target_part}\"] [data-geometry='part-hit-area']"),
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
    let captured = {
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
        if !live.capture {
            driver::end_drag(&page, &mut hold).await?;
            return Err(fail(
                "Stagger cancel: pointer capture missing during preview",
            ));
        }
        driver::press_key(&page, "Escape", "Escape", 27).await?;
        driver::end_drag(&page, &mut hold).await?;
        live.capture
    };
    // Cancelling clears the preview on a later render without writing a document.
    tokio::time::timeout(Duration::from_secs(10), async {
        while scene(&page).await? != cancel_scene {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        Ok::<(), Error>(())
    })
    .await
    .map_err(|_| fail("Stagger cancel: timed out restoring the exact canvas scene"))??;
    saved(&page).await?;
    wait_document(&page, &cancel_before.page_signature).await?;
    let cancel_after = doc(&page).await?;
    if cancel_after.revision != cancel_before.revision {
        return Err(fail("Stagger cancel created no saved revision"));
    }
    record(
        &mut smoke.checks,
        "Stagger cancel",
        detail(&[
            ("previewSeen", json!(true)),
            ("pointerCapture", json!(captured)),
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
