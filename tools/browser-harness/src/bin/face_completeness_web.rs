//! Packaged WEB provider face-completeness regression.
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::driver::{self, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

fn validate_url(url: &str) -> Result<()> {
    let port = url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|s| s.strip_suffix('/'));
    if port
        .and_then(|s| s.parse::<u16>().ok())
        .filter(|port| *port > 0)
        .is_none()
    {
        return Err(Error("expected loopback URL http://127.0.0.1:PORT/".into()));
    }
    Ok(())
}

fn validate_result(value: &Value) -> Result<()> {
    if *value != json!([{"suite": "step-face-completeness", "case": "sliver", "ok": true}]) {
        return Err(Error(format!(
            "invalid face completeness suite/case result: {value}"
        )));
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("face completeness: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .ok_or_else(|| Error("missing loopback URL".into()))?;
    if args.next().is_some() {
        return Err(Error("expected one loopback URL".into()));
    }
    validate_url(&url)?;
    let chrome = std::env::var("CHROME_BIN").map_err(|_| Error("set CHROME_BIN".into()))?;
    let root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(std::path::Path::new(&root)).map_err(Error)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let page_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, handler_errors.clone(), page_errors.clone());
    let outcome = async {
        let (context, page) = driver::new_isolated_page(&browser, "about:blank", &page_errors).await?;
        let result = async {
            tokio::time::timeout(Duration::from_secs(180), page.goto(&url))
                .await.map_err(|_| Error("navigation timed out".into()))?
                .map_err(|error| Error(format!("navigation failed: {error}")))?;
            let expected = serde_json::to_string(&url).map_err(|error| Error(error.to_string()))?;
            let expression = format!("(async () => {{ if (location.href !== {expected}) throw Error('unexpected page URL: ' + location.href); return (await import('/case.js')).run(); }})()");
            let value = tokio::time::timeout(Duration::from_secs(180), driver::eval(&page, &expression))
                .await.map_err(|_| Error("face completeness case timed out".into()))??;
            validate_result(&value)?;
            Ok::<_, Error>(value)
        }.await;
        let disposal = driver::dispose_isolated_page(&browser, Some(context), page, &page_errors).await;
        match (result, disposal) {
            (Err(first), Err(second)) => Err(Error(format!("{first}; {second}"))),
            (Err(error), _) | (_, Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }.await;
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
    println!("{}", outcome?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_loopback_and_unambiguous() {
        assert!(validate_url("http://127.0.0.1:1234/").is_ok());
        for value in [
            "",
            "http://localhost:1234/",
            "http://127.0.0.1:0/",
            "http://127.0.0.1:1234/path",
            "http://127.0.0.1:1234/?x=1",
        ] {
            assert!(validate_url(value).is_err(), "{value}");
        }
    }

    #[test]
    fn exactly_one_successful_suite_and_case_is_required() {
        let good = json!({"suite":"step-face-completeness","case":"sliver","ok":true});
        assert!(validate_result(&json!([good.clone()])).is_ok());
        for value in [
            json!(null),
            json!([]),
            json!([good.clone(), good.clone()]),
            json!([{}]),
            json!([{"suite":"step-face-completeness","case":"sliver","ok":false}]),
            json!([{"suite":"other","case":"sliver","ok":true}]),
        ] {
            assert!(validate_result(&value).is_err(), "{value}");
        }
    }
}
