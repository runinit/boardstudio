//! C1 preview consumer for the existing CAD measurement launcher.
#[path = "../cad_responses.rs"]
mod cad_responses;
#[path = "../cad_target_responses.rs"]
mod cad_target_responses;
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::driver::{self, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};
use chromiumoxide::cdp::browser_protocol::{
    network::SetCacheDisabledParams, page::AddScriptToEvaluateOnNewDocumentParams,
};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Capture {
    events: File,
    metrics: File,
    backend: String,
    profile: String,
    workload: String,
    pair: String,
    validation: bool,
    resources: Option<Value>,
}
impl Capture {
    fn event(&mut self, name: &str, data: Value) -> Result<()> {
        let mut event = json!({"unix_ms": SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| Error(e.to_string()))?.as_millis(), "name":name});
        event.as_object_mut().unwrap().extend(
            data.as_object()
                .ok_or_else(|| Error("event data is not an object".into()))?
                .clone(),
        );
        writeln!(self.events, "{event}").map_err(|e| Error(e.to_string()))
    }
    fn row(&mut self, values: &[String]) -> Result<()> {
        let line = values
            .iter()
            .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(self.metrics, "{line}").map_err(|e| Error(e.to_string()))
    }
    fn metric(
        &mut self,
        state: &str,
        ordinal: usize,
        name: &str,
        value: &Value,
        unit: &str,
    ) -> Result<()> {
        let number = value
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .ok_or_else(|| Error(format!("invalid {name} metric")))?;
        self.row(&[
            self.backend.clone(),
            self.profile.clone(),
            self.workload.clone(),
            self.pair.clone(),
            state.into(),
            ordinal.to_string(),
            name.into(),
            number.to_string(),
            unit.into(),
            if self.validation {
                "diagnostic-not-benchmark"
            } else {
                "observed"
            }
            .into(),
        ])
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("CAD capture: {error}");
        std::process::exit(1);
    }
}
async fn bounded<T>(
    future: impl std::future::Future<Output = Result<T>>,
    label: &str,
) -> Result<T> {
    tokio::time::timeout(Duration::from_secs(180), future)
        .await
        .map_err(|_| Error(format!("{label} timed out after 180 seconds")))?
}
async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 7
        || args[1] != "monstertruck"
        || args[3] != "C1plate:preview"
        || !matches!(args[6].as_str(), "validation" | "latency" | "growth20")
    {
        return Err(Error("usage: cad-capture ORIGIN monstertruck PROFILE C1plate:preview OUTPUT PAIR validation|latency|growth20".into()));
    }
    let validation = args[6] == "validation";
    let output = std::path::Path::new(&args[4]);
    let mut capture = Capture {
        events: OpenOptions::new()
            .create(true)
            .append(true)
            .open(output.join("events.log"))
            .map_err(|e| Error(e.to_string()))?,
        metrics: File::create(output.join(if validation {
            "diagnostic.csv"
        } else {
            "runtime.csv"
        }))
        .map_err(|e| Error(e.to_string()))?,
        backend: args[1].clone(),
        profile: args[2].clone(),
        workload: args[3].clone(),
        pair: args[5].clone(),
        validation,
        resources: None,
    };
    capture.row(
        &[
            "backend", "profile", "workload", "pair", "state", "ordinal", "metric", "value",
            "unit", "status",
        ]
        .map(String::from),
    )?;
    capture.event("purpose", json!({"validationOnly":validation,"values":if validation {"diagnostic-not-benchmark"} else {"observation-awaiting-admission"}}))?;
    let outcome = capture_browser(&args, &mut capture).await;
    if let Err(error) = &outcome {
        capture.event("failure", json!({"error":error.to_string()}))?;
    }
    if let Some(resources) = capture.resources.take() {
        capture.event("resources", json!({"responses":resources}))?;
    }
    capture.event("result", json!({"failed":outcome.is_err()}))?;
    outcome
}
async fn capture_browser(args: &[String], capture: &mut Capture) -> Result<()> {
    let chrome = std::env::var("CHROME_BIN").map_err(|_| Error("set CHROME_BIN".into()))?;
    let root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(std::path::Path::new(&root)).map_err(Error)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let page_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, handler_errors.clone(), page_errors.clone());
    let outcome = async {
        let version = browser.version().await.map_err(|e| Error(e.to_string()))?.product;
        capture.event("browser", json!({"version":version.strip_prefix("Chrome/").unwrap_or(&version),"executable":chrome,"flags":["--no-sandbox","--use-angle=swiftshader","--enable-unsafe-swiftshader"],"viewport":[1513,1043],"gc":"natural","httpCache":false,"serviceWorkers":"block"}))?;
        let (context, page) = driver::new_isolated_page(&browser, "about:blank", &page_errors).await?;
        let result = async {
            page.execute(AddScriptToEvaluateOnNewDocumentParams::new("if (navigator.serviceWorker) navigator.serviceWorker.register = async () => { console.warn('Service Worker registration blocked by Playwright'); };")) .await.map_err(|e| Error(e.to_string()))?;
            page.execute(SetCacheDisabledParams::new(true)).await.map_err(|e| Error(e.to_string()))?;
            let listener = cad_target_responses::start(browser.websocket_address(), page.target_id().inner()).await?;
            let work = async {
                bounded(async { page.goto(format!("{}/__perf/cad/scripts/performance-host.html", args[0])).await.map(|_| ()).map_err(|e| Error(e.to_string())) }, "navigation").await?;
                let source = include_str!("../../../../cad/scripts/measure-cad-worker.cjs");
                let arguments = json!({"backend":args[1], "fixture":"C1plate", "operation":"preview", "sequence":args[6], "isStartup":false});
                let expression = format!("(() => {{ const module = {{exports: null}}; {source} return module.exports({arguments}); }})()");
                let prepared = bounded(driver::eval(&page, &expression), "worker preparation").await?;
                capture.event("prepared", prepared.clone())?;
                for (i, value) in prepared["startup"].as_array().ok_or_else(|| Error("missing startup samples".into()))?.iter().enumerate() { capture.metric("worker-startup", i, "worker-create-to-ready", value, "ms")?; }
                tokio::time::sleep(Duration::from_millis(100)).await;
                capture.event("idle-baseline", json!({}))?;
                let count = if capture.validation {1} else if args[6] == "growth20" {20} else {6};
                for i in 0..count {
                    let state = if args[6] == "growth20" {"growth"} else if i == 0 {"cold"} else {"warm"};
                    capture.event("operation-start", json!({"state":state,"ordinal":i}))?;
                    let sample = bounded(driver::eval(&page, "window.__measure.run()"), "worker operation").await?;
                    let mut data = sample.as_object().ok_or_else(|| Error("invalid sample".into()))?.clone();
                    data.insert("state".into(), json!(state)); data.insert("ordinal".into(), json!(i));
                    capture.event("reply-retained", Value::Object(data))?;
                    for (field, metric, unit) in [("duration", "dispatch-to-validated-reply", "ms"), ("triangles", "triangles", "count"), ("bufferBytes", "bufferBytes", "bytes"), ("stepBytes", "stepBytes", "bytes")] { capture.metric(state, i, metric, &sample[field], unit)?; }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    bounded(driver::eval(&page, "window.__measure.drop()"), "drop").await?;
                    capture.event("references-dropped", json!({"state":state,"ordinal":i}))?;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                capture.event("scope-limit", json!({"print":"print-keycaps reply only; Runtime binary-STL/ZIP/download remains U1 admission","startup":"landing plus isolated real worker/reply; frozen editor/project and rendered frame remain U1 admission","memory":"process PSS/RSS in memory.csv; worker WASM linear capacity unavailable through production API","geometry":"shape/census checks supplement required independent untimed admission; observed is not accepted"}))?;
                page_errors.fail_if_any()
            }.await;
            Ok::<_, Error>((work, listener))
        }.await;
        let disposal = driver::dispose_isolated_page(&browser, Some(context), page, &page_errors).await;
        let (work, mut listener) = result?;
        let responses = match tokio::time::timeout(Duration::from_secs(60), &mut listener).await { Ok(Ok(r)) => r, Ok(Err(e)) => Err(Error(e.to_string())), Err(_) => { listener.abort(); Err(Error("response listener did not finish".into())) } };
        if let Ok(ref resources) = responses { capture.resources = Some(json!(resources)); }
        let errors: Vec<_> = [work.as_ref().err(), disposal.as_ref().err(), responses.as_ref().err()].into_iter().flatten().map(ToString::to_string).collect();
        if errors.is_empty() { Ok(()) } else { Err(Error(errors.join("; "))) }
    }.await;
    let (closed, confirmed) =
        shutdown::finish_browser(browser, drain, &handler_errors, outcome.is_ok()).await;
    let cleanup = if confirmed {
        shutdown::remove_profile(&dirs)
    } else {
        std::mem::forget(dirs);
        Err(Error(
            "browser termination unconfirmed; profile preserved".into(),
        ))
    };
    for error in page_errors.messages() {
        capture.event("page-error", json!({"error": error}))?;
    }
    let page = page_errors.fail_if_any();
    let errors: Vec<_> = [
        outcome.as_ref().err(),
        closed.as_ref().err(),
        cleanup.as_ref().err(),
        page.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    .map(ToString::to_string)
    .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error(errors.join("; ")))
    }
}
