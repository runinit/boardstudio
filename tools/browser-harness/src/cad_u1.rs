use boardstudio_browser_harness::{driver, Error, Result};
use chromiumoxide::cdp::browser_protocol::{
    browser::{
        BrowserContextId, DownloadProgressState, EventDownloadProgress, EventDownloadWillBegin,
        SetDownloadBehaviorBehavior, SetDownloadBehaviorParams,
    },
    dom::SetFileInputFilesParams,
    emulation::{MediaFeature, SetEmulatedMediaParams, SetTouchEmulationEnabledParams},
    page::AddScriptToEvaluateOnNewDocumentParams,
};
use chromiumoxide::{Browser, Page};
use futures::StreamExt;
use serde_json::{json, Value};
use std::{
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const WAIT: Duration = Duration::from_secs(60);

fn upload_archive() -> Result<std::path::PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../cad/test/fixtures/performance/U1.boardstudio");
    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| Error(format!("canonicalizing U1 upload {}: {e}", path.display())))?;
    if !canonical.is_file() {
        return Err(Error(format!(
            "U1 upload is not a file: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    #[test]
    fn upload_archive_is_canonical() {
        let path = super::upload_archive().unwrap();
        assert_eq!(path, std::fs::canonicalize(&path).unwrap());
    }
}

async fn wait(page: &Page, expression: &str, label: &str) -> Result<()> {
    driver::wait_for(page, expression, label, WAIT).await
}
async fn click(page: &Page, name: &str) -> Result<()> {
    driver::click_role(page, None, "button", name).await
}
async fn saved(page: &Page) -> Result<()> {
    wait(
        page,
        "document.querySelector('.m1-save-state')?.dataset.state === 'saved'",
        "saved project",
    )
    .await
}
fn write_json(path: &Path, value: &Value) -> Result<()> {
    std::fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value).map_err(|e| Error(e.to_string()))?
        ),
    )
    .map_err(|e| Error(e.to_string()))
}
fn mark(phases: &mut Vec<Value>, name: &str, start: Instant) -> Result<()> {
    phases.push(json!({"name":name,"hostUnixMs":SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Error(e.to_string()))?.as_millis(),"hostMonotonicMs":start.elapsed().as_secs_f64()*1000.0,"purpose":"diagnostic-not-benchmark"}));
    Ok(())
}
async fn document(page: &Page) -> Result<Value> {
    driver::eval(page, r#"new Promise((resolve, reject) => { const request = indexedDB.open('boardstudio-m1-boardstudio'); request.onerror = () => reject(request.error); request.onsuccess = () => { const db = request.result; const query = db.transaction('projects').objectStore('projects').getAll(); query.onerror = () => { db.close(); reject(query.error); }; query.onsuccess = () => { db.close(); query.result.length === 1 ? resolve(query.result[0]) : reject(Error('expected one accepted project')); }; }; })"#).await
}
async fn select(page: &Page, selector: &str, value: &str) -> Result<()> {
    let expression = format!(
        r#"(() => {{ const nodes = document.querySelectorAll({}); if (nodes.length !== 1) throw Error('expected one select'); const el = nodes[0]; if (![...el.options].some(o => o.value === {} || o.label === {})) throw Error('missing option'); el.value = [...el.options].find(o => o.value === {} || o.label === {}).value; el.dispatchEvent(new Event('input', {{bubbles:true}})); el.dispatchEvent(new Event('change', {{bubbles:true}})); }})()"#,
        json!(selector),
        json!(value),
        json!(value),
        json!(value),
        json!(value)
    );
    driver::eval(page, &expression).await?;
    Ok(())
}
async fn download(
    browser: &Browser,
    page: &Page,
    out: &Path,
    button: &str,
    filename: &str,
    phases: &mut Vec<Value>,
    start: Instant,
) -> Result<Value> {
    let mut begun = browser
        .event_listener::<EventDownloadWillBegin>()
        .await
        .map_err(|e| Error(e.to_string()))?;
    let mut progress = browser
        .event_listener::<EventDownloadProgress>()
        .await
        .map_err(|e| Error(e.to_string()))?;
    mark(phases, &format!("{filename}:action"), start)?;
    click(page, button).await?;
    let (guid, suggested) = tokio::time::timeout(Duration::from_secs(180), async {
        let event = begun
            .next()
            .await
            .ok_or_else(|| Error("download event stream closed".into()))?;
        let guid = event.guid.clone();
        loop {
            let event = progress
                .next()
                .await
                .ok_or_else(|| Error("download progress stream closed".into()))?;
            if event.guid != guid {
                continue;
            }
            match event.state {
                DownloadProgressState::Completed => break,
                DownloadProgressState::Canceled => {
                    return Err(Error(format!("download {filename} canceled")));
                }
                DownloadProgressState::InProgress => {}
            }
        }
        Ok::<_, Error>((guid, event.suggested_filename.clone()))
    })
    .await
    .map_err(|_| Error(format!("download {filename} timed out")))??;
    let path = out.join(filename);
    std::fs::rename(out.join(guid), &path)
        .map_err(|e| Error(format!("saving download {filename}: {e}")))?;
    let bytes = std::fs::metadata(&path)
        .map_err(|e| Error(e.to_string()))?
        .len();
    mark(phases, &format!("{filename}:saved-download"), start)?;
    Ok(
        json!({"endpoint":format!("button: {button}"),"file":filename,"suggested":suggested,"bytes":bytes,"purpose":"validation-only; diagnostic, not benchmark"}),
    )
}

pub(super) async fn run(
    browser: &Browser,
    context: &BrowserContextId,
    page: &Page,
    origin: &str,
    out: &Path,
    operation: &str,
    page_errors: &driver::PageErrors,
) -> Result<Value> {
    let start = Instant::now();
    let mut phases = Vec::new();
    let result = async {
        let source = include_str!("../../../cad/scripts/measure-cad-page.cjs");
        let instrument = source
            .split_once("  await page.addInitScript(() => {\n")
            .and_then(|(_, s)| s.split_once("\n  });\n  const saved"))
            .map(|(s, _)| format!("(() => {{\n{s}\n}})()"))
            .ok_or_else(|| Error("U1 instrumentation not found".into()))?;
        page.execute(AddScriptToEvaluateOnNewDocumentParams::new(instrument))
            .await
            .map_err(|e| Error(e.to_string()))?;
        browser
            .execute(
                SetDownloadBehaviorParams::builder()
                    .behavior(SetDownloadBehaviorBehavior::AllowAndName)
                    .browser_context_id(context.clone())
                    .download_path(out.to_string_lossy().to_string())
                    .events_enabled(true)
                    .build()
                    .map_err(Error)?,
            )
            .await
            .map_err(|e| Error(e.to_string()))?;
        page.execute(SetEmulatedMediaParams::builder().features(vec![MediaFeature::new("prefers-color-scheme", "light")]).build()).await.map_err(|e| Error(e.to_string()))?;
        page.execute(SetTouchEmulationEnabledParams::new(false)).await.map_err(|e| Error(e.to_string()))?;
        mark(&mut phases, "navigation", start)?;
        page.goto(format!("{origin}/boardstudio/")).await.map_err(|e| Error(e.to_string()))?;
        if operation == "freeze" {
            click(page, "Create new keyboard").await?;
            click(page, "Continue to layout").await?;
            for name in ["Rows", "Columns"] {
                let selector = format!("[role=slider][aria-label=\"{name}\"]");
                loop {
                    let value = driver::eval(page, &format!("Number(document.querySelector({})?.getAttribute('aria-valuenow'))", json!(selector))).await?.as_i64().ok_or_else(|| Error(format!("missing slider {name}")))?;
                    if value == 2 { break; }
                    driver::eval(page, &format!("document.querySelector({}).focus()", json!(selector))).await?;
                    let (key, code, number) = if value > 2 { ("ArrowLeft", "ArrowLeft", 37) } else { ("ArrowRight", "ArrowRight", 39) };
                    driver::press_key(page, key, code, number).await?;
                    let next = value + if value > 2 { -1 } else { 1 };
                    wait(page, &format!("Number(document.querySelector({})?.getAttribute('aria-valuenow')) === {next}", json!(selector)), name).await?;
                }
            }
            click(page, "Create matrix").await?;
            wait(page, "!!document.querySelector('.m1-scene-part')", "matrix scene").await?;
            saved(page).await?;
            click(page, "Open Case workspace").await?;
            let case = driver::find_role(page, None, "button", "").await?.into_iter().filter(|(_, n)| n.contains("Case (optional)")).collect::<Vec<_>>();
            if case.len()!=1 { return Err(Error("expected Case (optional) button".into())); }
            driver::click_backend(page, case[0].0).await?;
            select(page, "[role=combobox][aria-label='Case mounting style'], select[aria-label='Case mounting style']", "Rigid mount").await?;
            saved(page).await?;
            click(page, "Hide guide").await?;
            click(page, "Open Keycaps workspace").await?;
            wait(page, "!!document.querySelector('#m1-workspace-Keycaps[aria-current=page]')", "Keycaps workspace").await?;
            let controls = driver::eval(page, "({inspector: document.querySelector('#m1-inspector-slot')?.outerHTML, disclosures:[...document.querySelectorAll('details')].map(d=>({title:d.querySelector('summary')?.textContent,open:d.open}))})").await?;
            write_json(&out.join("keycaps-controls.log"), &controls)?;
            let reveal = driver::find_role(page, None, "button", "Show inspector").await?;
            if let [one] = reveal.as_slice() { if driver::is_visible(page, one.0).await? { driver::click_backend(page, one.0).await?; } }
            let node = driver::eval(page, "[...document.querySelectorAll('*')].find(e => e.textContent.trim() === 'Matrix profiles' && e.children.length === 0)?.click() ").await?; let _ = node;
            select(page, "select[aria-label^='Keycap profile for '], [role=combobox][aria-label^='Keycap profile for ']", "dcs").await?;
            saved(page).await?;
            click(page, "Open Case workspace").await?;
        } else if operation == "page" {
            let archive = upload_archive()?;
            let archive = archive.to_str().ok_or_else(|| Error(format!("U1 upload path is not UTF-8: {}", archive.display())))?;
            mark(&mut phases, "frozen-project-import", start)?;
            wait(page, "!!document.querySelector('input.m1-project-file-input:not(:disabled)')", "enabled project import").await?;
            let id = driver::backend_of_wait(page, "input.m1-project-file-input:not(:disabled)", WAIT).await?;
            page.execute(SetFileInputFilesParams::builder().file(archive.to_owned()).backend_node_id(id).build().map_err(Error)?).await.map_err(|e| Error(e.to_string()))?;
            wait(page, "!!document.querySelector('[aria-label=\"Open Case workspace\"]')", "accepted editor").await?;
            saved(page).await?;
            mark(&mut phases, "accepted-editor-ready", start)?; mark(&mut phases, "enter-Case", start)?;
            click(page, "Open Case workspace").await?;
            let guide = driver::find_role(page, None, "button", "Hide guide").await?;
            if let [one] = guide.as_slice() { if driver::is_visible(page, one.0).await? { driver::click_backend(page, one.0).await?; } }
        } else { return Err(Error("invalid U1 operation".into())); }
        wait(page, "[...document.querySelectorAll('[role=status]')].some(e=>e.textContent.includes('Exact case geometry ready.'))", "exact case geometry").await?;
        wait(page, "document.fonts.status === 'loaded' && !matchMedia('(pointer: coarse)').matches && !matchMedia('(prefers-color-scheme: dark)').matches", "desktop light layout readiness").await?;
        saved(page).await?;
        let accepted = document(page).await?;
        if accepted["matrices"].as_array().is_none_or(|m| m.len()!=1 || m[0]["rows"]!=2 || m[0]["columns"]!=2) { return Err(Error("accepted project is not a 2x2 matrix".into())); }
        let items = driver::find_role(page, None, "treeitem", "").await?.into_iter().filter(|(_, n)| n.ends_with("Main board case assembly")).collect::<Vec<_>>();
        if items.len()!=1 { return Err(Error("expected Main board case assembly".into())); }
        driver::click_backend(page, items[0].0).await?;
        for name in ["Assembled", "Shaded", "Isometric"] { click(page, name).await?; }
        let before = driver::eval(page, "window.__u1.drawCalls").await?.as_u64().ok_or_else(|| Error("missing draw count".into()))?;
        click(page, "Fit").await?;
        wait(page, &format!("window.__u1.drawCalls > {before}"), "draw after Fit").await?;
        driver::eval(page, "window.__u1Gl.finish()").await?;
        mark(&mut phases, "accepted-Case-frame-after-camera-and-WebGL-finish", start)?;
        write_json(&out.join("page-layout.json"), &driver::eval(page, include_str!("../../../cad/scripts/u1-layout.js")).await?)?;
        let viewer = driver::eval(page, r#"(() => { const e = document.querySelector('[role=img][aria-label^="Interactive 3D Case preview"]'); if (!e) throw Error('missing viewer'); const r=e.getBoundingClientRect(); if (r.width<=0 || r.height<=0) throw Error('empty viewer'); return {x:r.x,y:r.y,width:r.width,height:r.height}; })()"#).await?;
        page.save_screenshot(chromiumoxide::page::ScreenshotParams::builder().build(), out.join("case-rendered.png")).await.map_err(|e| Error(e.to_string()))?;
        let rendering = driver::eval(page, "({drawCalls:window.__u1.drawCalls, completion:'WebGL2.finish after draw following Fit', glVersion:window.__u1Gl.getParameter(window.__u1Gl.VERSION), selection:[...document.querySelectorAll('[aria-selected=true]')].map(n=>n.getAttribute('aria-label')||n.textContent)})").await?;
        driver::click_css(page, "#m1-tab-Export").await?;
        wait(page, "!!document.querySelector('.m1-export-workspace')", "export workspace").await?;
        let mut downloads = Vec::new();
        if operation == "freeze" { downloads.push(download(browser,page,out,"Download .boardstudio project","U1.boardstudio",&mut phases,start).await?); }
        downloads.push(download(browser,page,out,"Export Generated mechanical package","mechanical.zip",&mut phases,start).await?);
        downloads.push(download(browser,page,out,"Export Keycap STLs","keycaps.zip",&mut phases,start).await?);
        if document(page).await? != accepted { return Err(Error("render/export altered accepted project".into())); }
        let workers = driver::eval(page, "window.__u1").await?;
        let requests = workers["requests"].as_array().ok_or_else(|| Error("missing worker requests".into()))?;
        let prints: Vec<_> = requests.iter().filter(|r| r["operation"] == "print-keycaps").collect();
        if prints.len()!=1 || prints[0]["keycaps"]["specs"].as_array().is_none_or(|v| v.len()!=4) || workers["printBodies"].as_array().is_none_or(|v| v.len()!=4) || prints[0]["identity"]["revision"] != accepted["revision"] { return Err(Error("invalid accepted-revision print request or bodies".into())); }
        let replies = workers["replies"].as_array().ok_or_else(|| Error("missing worker replies".into()))?;
        if !replies.iter().any(|r| r["frame"]["operation"] == "preview" && r["frame"]["outcome"] == "completed" && r["frame"]["identity"]["revision"] == accepted["revision"]) { return Err(Error("missing accepted-revision preview reply".into())); }
        if !replies.iter().any(|r| (r["frame"]["operation"] == "exact" || r["frame"]["operation"] == "export-step") && r["frame"]["outcome"] == "completed" && r["stepBytes"].as_u64().unwrap_or(0)>0) { return Err(Error("missing completed STEP reply".into())); }
        page_errors.fail_if_any()?;
        write_json(&out.join("accepted-project.json"), &accepted)?;
        write_json(&out.join("page-workers.log"), &workers)?;
        let receipt = json!({"validationOnly":true,"projectId":accepted["id"],"revision":accepted["revision"],"boardId":accepted["boards"][0]["id"],"viewport":[1513,1043],"camera":["Assembled","Shaded","Isometric","Fit"],"rendering":rendering,"viewer":viewer,"downloads":downloads,"comparisonPeer":"not exercised here; requires its separately built/admitted full page and worker","phases":phases,"values":"diagnostic-not-benchmark"});
        write_json(&out.join("page-diagnostic.log"), &receipt)?;
        Ok(receipt)
    }.await;
    if let Err(error) = &result {
        let body = driver::eval(page, "document.body?.innerText || ''")
            .await
            .unwrap_or(Value::Null);
        std::fs::write(
            out.join("page-failure.log"),
            format!("{error}\n{}\n", body.as_str().unwrap_or("")),
        )
        .map_err(|e| Error(e.to_string()))?;
    }
    result
}
