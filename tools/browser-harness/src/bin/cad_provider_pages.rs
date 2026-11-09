//! Packaged CAD provider regressions, one isolated page per suite.
#[path = "../shutdown.rs"]
mod shutdown;

use boardstudio_browser_harness::driver::{self, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{Error, IsolatedDirs, Result};
use serde_json::{Map, Value};
use std::collections::HashSet;
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

fn validate_result(value: &Value, suite: &str) -> Result<Vec<String>> {
    let entries = value
        .as_array()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| Error(format!("invalid results for {suite}: {value}")))?;
    let mut names = HashSet::new();
    let mut failures = Vec::new();
    for entry in entries {
        let name = entry
            .get("case")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error(format!("invalid case for {suite}: {entry}")))?;
        if entry.get("suite").and_then(Value::as_str) != Some(suite) || !names.insert(name) {
            return Err(Error(format!("invalid suite/case for {suite}: {entry}")));
        }
        match entry.get("ok").and_then(Value::as_bool) {
            Some(true) if entry.get("error").is_none() => (),
            Some(false) => {
                let error = entry
                    .get("error")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        Error(format!("missing failure diagnostic for {suite}/{name}"))
                    })?;
                failures.push(format!("{suite}/{name}: {error}"));
            }
            _ => return Err(Error(format!("invalid result for {suite}/{name}: {entry}"))),
        }
    }
    Ok(failures)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("cad provider pages: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .ok_or_else(|| Error("missing loopback URL".into()))?;
    validate_url(&url)?;
    let suites: Vec<_> = args.collect();
    let mut names = HashSet::new();
    if suites.is_empty()
        || suites.iter().any(|s| {
            s.is_empty()
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                || !names.insert(s)
        })
    {
        return Err(Error("expected distinct suite names".into()));
    }
    let chrome = std::env::var("CHROME_BIN").map_err(|_| Error("set CHROME_BIN".into()))?;
    let root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".into());
    let dirs = IsolatedDirs::create(std::path::Path::new(&root)).map_err(Error)?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let drain_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(handler, handler_errors.clone(), drain_errors.clone());
    let mut results = Map::new();
    let mut failures = Vec::new();
    for suite in &suites {
        let page_errors = Arc::new(PageErrors::new());
        let outcome = async {
            let (context, page) = driver::new_isolated_page(&browser, "about:blank", &page_errors).await?;
            let result = async {
                let page_url = format!("{url}?suite={suite}");
                tokio::time::timeout(Duration::from_secs(180), page.goto(&page_url)).await
                    .map_err(|_| Error("navigation timed out".into()))?
                    .map_err(|error| Error(format!("navigation failed: {error}")))?;
                let expected = serde_json::to_string(&page_url).map_err(|e| Error(e.to_string()))?;
                let name = serde_json::to_string(suite).map_err(|e| Error(e.to_string()))?;
                let expression = format!("(async () => {{ if (location.href !== {expected}) throw Error('unexpected page URL: ' + location.href); return (await import('/run.js')).run({name}); }})()");
                tokio::time::timeout(Duration::from_secs(300), driver::eval(&page, &expression)).await
                    .map_err(|_| Error("suite timed out".into()))?
            }.await;
            let disposal = driver::dispose_isolated_page(&browser, Some(context), page, &page_errors).await;
            let page_error = page_errors.fail_if_any();
            let errors: Vec<_> = [result.as_ref().err(), disposal.as_ref().err(), page_error.as_ref().err()]
                .into_iter().flatten().map(ToString::to_string).collect();
            if !errors.is_empty() { return Err(Error(errors.join("; "))); }
            result
        }.await;
        match outcome {
            Ok(value) => match validate_result(&value, suite) {
                Ok(errors) => {
                    failures.extend(errors);
                    results.insert(suite.clone(), value);
                }
                Err(error) => failures.push(format!("{suite}: {error}")),
            },
            Err(error) => failures.push(format!("{suite}: {error}")),
        }
    }
    let (closed, confirmed) =
        shutdown::finish_browser(browser, drain, &handler_errors, failures.is_empty()).await;
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
    for error in [
        closed.err(),
        cleanup.err(),
        drain_errors.fail_if_any().err(),
    ]
    .into_iter()
    .flatten()
    {
        failures.push(error.to_string());
    }
    println!("{}", Value::Object(results));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error(failures.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn url_is_loopback_and_unambiguous() {
        assert!(validate_url("http://127.0.0.1:1234/").is_ok());
        for url in [
            "",
            "http://localhost:1234/",
            "http://127.0.0.1:0/",
            "http://127.0.0.1:1234/x",
            "http://127.0.0.1:1234/?x=1",
        ] {
            assert!(validate_url(url).is_err(), "{url}");
        }
    }

    #[test]
    fn results_require_distinct_named_boolean_cases() {
        let good = json!({"suite":"s","case":"main","ok":true});
        assert!(validate_result(&json!([good.clone()]), "s")
            .unwrap()
            .is_empty());
        assert_eq!(
            validate_result(
                &json!([{"suite":"s","case":"x","ok":false,"error":"oops"}]),
                "s"
            )
            .unwrap()
            .len(),
            1
        );
        for value in [
            json!(null),
            json!([]),
            json!([good.clone(), good]),
            json!([{"suite":"s","case":"x","ok":1}]),
            json!([{"suite":"s","case":"x","ok":false}]),
            json!([{"suite":"other","case":"x","ok":true}]),
        ] {
            assert!(validate_result(&value, "s").is_err(), "{value}");
        }
    }
}
