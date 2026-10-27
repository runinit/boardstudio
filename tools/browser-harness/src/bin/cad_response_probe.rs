//! Diagnostic CAD worker startup and response capture; not a measurement receipt.
#[path = "../cad_responses.rs"]
mod cad_responses;
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::driver::{self, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};
use chromiumoxide::cdp::browser_protocol::network::{EnableParams, EventResponseReceived};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("CAD response probe: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .ok_or_else(|| Error("usage: cad-response-probe URL".into()))?;
    if args.next().is_some() {
        return Err(Error("usage: cad-response-probe URL".into()));
    }
    let chrome = std::env::var("CHROME_BIN").map_err(|_| Error("set CHROME_BIN".into()))?;
    let root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(std::path::Path::new(&root)).map_err(Error)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let page_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, handler_errors.clone(), page_errors.clone());
    let outcome = async {
        let (context, page) =
            driver::new_isolated_page(&browser, "about:blank", &page_errors).await?;
        let result = async {
            let mut events = page
                .event_listener::<EventResponseReceived>()
                .await
                .map_err(|e| Error(format!("cannot subscribe to responses: {e}")))?;
            page.execute(EnableParams::default())
                .await
                .map_err(|e| Error(format!("cannot enable network events: {e}")))?;
            let listener = tokio::spawn(async move {
                let mut responses = Vec::new();
                while let Some(event) = events.next().await {
                    let r = &event.response;
                    responses.push(cad_responses::resource(
                        r.url.clone(),
                        r.status,
                        r.from_service_worker,
                    ));
                }
                responses
            });
            let navigation = tokio::time::timeout(Duration::from_secs(180), page.goto(&url))
                .await
                .map_err(|_| Error("navigation timed out after 180 seconds".into()))?
                .map(|_| ())
                .map_err(|e| Error(format!("navigation failed: {e}")));
            let startup = match navigation {
                Ok(()) => tokio::time::timeout(Duration::from_secs(180), driver::eval(&page, r#"
                    (async () => {
                      const start = performance.now();
                      const worker = new Worker('/boardstudio/assets/cad-worker/entry.js', { type: 'module' });
                      try {
                        await new Promise((resolve, reject) => {
                          worker.onerror = e => reject(Error(e.message));
                          worker.onmessage = e => {
                            if (e.data.kind === 'ready') resolve();
                            if (e.data.kind === 'init-failed') reject(Error(JSON.stringify(e.data)));
                          };
                        });
                        return performance.now() - start;
                      } finally { worker.terminate(); }
                    })()
                "#)).await.map_err(|_| Error("worker ready watchdog: 180 seconds".into()))?,
                Err(error) => Err(error),
            };
            Ok::<_, Error>((startup, listener))
        }
        .await;
        let disposal =
            driver::dispose_isolated_page(&browser, Some(context), page, &page_errors).await;
        let (startup, listener) = result?;
        disposal?;
        let startup = startup?;
        if !startup.as_f64().is_some_and(|value| value.is_finite() && value >= 0.0) {
            return Err(Error("invalid worker startup duration".into()));
        }
        let responses = tokio::time::timeout(Duration::from_secs(60), listener)
            .await
            .map_err(|_| Error("response listener did not finish".into()))?
            .map_err(|e| Error(format!("response listener failed: {e}")))?;
        if responses.is_empty() {
            return Err(Error("no response events observed".into()));
        }
        Ok::<_, Error>(serde_json::json!({
            "purpose": "diagnostic-not-benchmark",
            "responseScope": "page-target-only",
            "workerCreateToReadyMs": startup,
            "responses": responses,
        }))
    }
    .await;
    let (closed, confirmed) =
        shutdown::finish_browser(browser, drain, &handler_errors, outcome.is_ok()).await;
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
    let page = page_errors.fail_if_any();
    let errors: Vec<String> = [
        outcome.as_ref().err(),
        closed.as_ref().err(),
        cleanup.as_ref().err(),
        page.as_ref().err(),
    ]
    .into_iter()
    .flatten()
    .map(ToString::to_string)
    .collect();
    if !errors.is_empty() {
        return Err(Error(errors.join("; ")));
    }
    let responses = outcome?;
    println!(
        "{}",
        serde_json::to_string(&responses).map_err(|e| Error(e.to_string()))?
    );
    Ok(())
}
