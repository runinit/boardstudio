//! Skew probe: surface every unparsable CDP event and prove it is benign.
//!
//! Launches Chrome exactly like [`driver::launch_chrome`] (same discovery,
//! isolated profile, flags, viewport) except `surface_invalid_messages` is
//! enabled here and ONLY here. Production runs must never gate on it.
//!
//! Exit 0 iff: zero command errors, zero handler errors, zero REQUIRED
//! invalids, no unexpected page errors, and the deliberate page exception
//! was observed through `Runtime.exceptionThrown` before its deadline.
//! Prints a `method -> count` table on every path.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use boardstudio_browser_harness::driver::{self, InvalidCounts, PageErrors};
use boardstudio_browser_harness::{discover_chrome, Error, IsolatedDirs, Result};
use chromiumoxide::cdp::js_protocol::runtime::EventExceptionThrown;
use chromiumoxide::error::CdpError;
use futures::StreamExt;

#[path = "../shutdown.rs"]
mod shutdown;

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn fail(message: impl Into<String>) -> Error {
    Error::new(message)
}

fn cleanup_profile(dirs: IsolatedDirs, confirmed: bool) -> Result<()> {
    if confirmed {
        shutdown::remove_profile(&dirs)
    } else {
        let message = format!(
            "browser termination unconfirmed; profile preserved at {}",
            dirs.profile_dir.display()
        );
        std::mem::forget(dirs);
        Err(fail(message))
    }
}

fn teardown_counter_verdict(counters: &Counters) -> Result<()> {
    let count = counters.handler_errors.load(Ordering::Relaxed);
    if count != 0 {
        let first = counters
            .first_handler_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        return Err(fail(format!(
            "skew probe saw {count} handler error(s); first: {}",
            first.first().cloned().unwrap_or_default()
        )));
    }
    let required = counters
        .invalids
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .required_total();
    if required != 0 {
        return Err(fail(format!(
            "skew probe saw {required} REQUIRED invalid(s)"
        )));
    }
    Ok(())
}

const FIXTURE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>skew probe</title></head>
<body><button id="probe-btn" onclick="window.__clicked = true">Probe target</button></body></html>"#;

const DELIBERATE: &str = "skew-probe-deliberate";

/// Shared counters the drain task and the round trips both update.
#[derive(Debug, Default)]
struct Counters {
    invalids: Mutex<InvalidCounts>,
    samples: Mutex<Vec<String>>,
    handler_errors: AtomicUsize,
    first_handler_error: Mutex<Vec<String>>,
}

impl Counters {
    fn record_invalid(&self, raw: &str) {
        let method = driver::invalid_method(raw);
        self.invalids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .record(&method);
        let mut samples = self
            .samples
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if samples.len() < 5 {
            samples.push(driver::redact(raw));
        }
    }

    fn record_handler_error(&self, error: String) {
        self.handler_errors.fetch_add(1, Ordering::Relaxed);
        let mut first = self
            .first_handler_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if first.len() < 3 {
            first.push(driver::redact(&error));
        }
    }

    fn print(&self) {
        let invalids = self
            .invalids
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        println!("skew probe invalid method -> count:");
        for (method, count, kind) in invalids.sorted() {
            let name = if method.is_empty() {
                "<unknown>"
            } else {
                &method
            };
            println!("  {name} -> {count} ({kind:?})");
        }
        println!(
            "skew probe totals: invalids={} required={} handler_errors={}",
            invalids.total(),
            invalids.required_total(),
            self.handler_errors.load(Ordering::Relaxed),
        );
        for sample in self
            .samples
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
        {
            println!("skew probe invalid sample: {sample}");
        }
    }
}

fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if path.contains('/') {
        let candidate = std::path::Path::new(path);
        return candidate.is_file()
            && std::fs::metadata(candidate)
                .map(|meta| meta.permissions().mode() & 0o111 != 0)
                .unwrap_or(false);
    }
    std::env::var_os("PATH")
        .map(|dirs| {
            std::env::split_paths(&dirs).any(|dir| {
                let candidate = dir.join(path);
                candidate.is_file()
                    && std::fs::metadata(&candidate)
                        .map(|meta| meta.permissions().mode() & 0o111 != 0)
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Serve the fixture on 127.0.0.1 and return the base URL.
async fn serve_fixture() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| fail(format!("skew probe cannot bind a fixture server: {error}")))?;
    let url = format!(
        "http://127.0.0.1:{}/probe",
        listener
            .local_addr()
            .map_err(|error| fail(format!("skew probe has no address: {error}")))?
            .port()
    );
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut request = [0u8; 4096];
                if stream.read(&mut request).await.is_err() {
                    return;
                }
                let body = if request.starts_with(b"GET /probe ") {
                    FIXTURE
                } else {
                    "not found"
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(response.as_bytes()).await.ok();
            });
        }
    });
    Ok((url, handle))
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let counters = Arc::new(Counters::default());
    let outcome = run(Arc::clone(&counters)).await;
    counters.print();
    match outcome {
        Ok(()) => {
            println!("PASS skew probe: no required skew, deliberate exception observed");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("FAIL skew probe: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(counters: Arc<Counters>) -> Result<()> {
    let mut command_errors = 0usize;
    macro_rules! step {
        ($label:literal, $expr:expr) => {
            match $expr.await {
                Ok(value) => value,
                Err(error) => {
                    command_errors += 1;
                    return Err(fail(format!(
                        "skew probe {}: command error #{}: {}",
                        $label, command_errors, error
                    )));
                }
            }
        };
    }

    let chrome = discover_chrome(&|name| std::env::var(name).ok(), &is_executable).map_err(fail)?;
    let chrome_version = std::process::Command::new(&chrome)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_owned());
    println!("skew probe chrome: {}", chrome_version.trim());

    let tmp_root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    let dirs = IsolatedDirs::create(std::path::Path::new(&tmp_root))
        .map_err(|error| fail(format!("skew probe cannot isolate its profile: {error}")))?;

    let (browser, handler) = step!("launch", driver::launch_chrome(&chrome, &dirs));
    let mut handler = handler;

    let drain_counters = Arc::clone(&counters);
    let drain = tokio::spawn(async move {
        while let Some(event) = handler.next().await {
            match event {
                Ok(()) => {}
                Err(CdpError::InvalidMessage(raw, _)) => drain_counters.record_invalid(&raw),
                Err(other) => drain_counters.record_handler_error(other.to_string()),
            }
        }
    });

    let outcome = async {
        // Browser domain round trip.
        let version = step!("browser version", browser.version());
        println!("skew probe product: {}", version.product);

        let page_errors = Arc::new(PageErrors::new());
        let page = step!("open page", browser.new_page("about:blank"));
        step!(
            "listen for page errors",
            driver::spawn_page_error_listener(&page, Arc::clone(&page_errors))
        );
        let mut exceptions = step!(
            "listen for exceptions",
            page.event_listener::<EventExceptionThrown>()
        );

        let (url, server) = serve_fixture().await?;
        let _server = AbortOnDrop(server);
        step!("open fixture", async {
            tokio::time::timeout(Duration::from_secs(60), page.goto(url.as_str()))
                .await
                .map_err(|_| fail("skew probe timed out opening the fixture"))?
                .map_err(|error| fail(format!("skew probe cannot open the fixture: {error}")))
        });

        // DOM round trip: document, query, describe.
        step!("dom round trip", driver::backend_of(&page, "#probe-btn"));
        // Runtime round trip.
        let sum = step!("runtime round trip", driver::eval(&page, "1 + 1"));
        if sum.as_i64() != Some(2) {
            return Err(fail(format!(
                "skew probe runtime round trip saw {sum:?}, want 2"
            )));
        }
        // Accessibility round trip.
        let nodes = step!(
            "accessibility round trip",
            driver::find_role(&page, None, "button", "Probe target")
        );
        if nodes.len() != 1 {
            return Err(fail(format!(
                "skew probe accessibility round trip matched {} nodes, want 1",
                nodes.len()
            )));
        }
        // Input round trip: trusted click flips the fixture flag.
        step!("input round trip", driver::click_css(&page, "#probe-btn"));
        let clicked = step!(
            "input effect",
            driver::eval(&page, "window.__clicked === true")
        );
        if clicked != serde_json::Value::Bool(true) {
            return Err(fail("skew probe input round trip had no page effect"));
        }
        // Target round trip: create and dispose an isolated context.
        let isolated_errors = Arc::new(PageErrors::new());
        let (context, isolated) = step!(
            "target create",
            driver::new_isolated_page(&browser, &url, &isolated_errors)
        );
        step!(
            "target dispose",
            driver::dispose_isolated_page(&browser, Some(context), isolated, &isolated_errors)
        );
        isolated_errors
            .fail_if_any()
            .map_err(|error| fail(error.to_string()))?;

        // Deliberate page exception via Runtime.evaluate: the eval itself must
        // succeed (no command error); the uncaught throw must arrive through
        // Runtime.exceptionThrown before the deadline. Missing it is a timeout.
        step!(
            "arm deliberate exception",
            driver::eval(
                &page,
                "setTimeout(() => { throw new Error('skew-probe-deliberate'); }, 0); 'armed';"
            )
        );
        let observed = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = exceptions.next().await {
                let detail = &event.exception_details;
                let text = detail
                    .exception
                    .as_ref()
                    .and_then(|exception| exception.description.clone())
                    .unwrap_or_else(|| detail.text.clone());
                if text.contains(DELIBERATE) {
                    return true;
                }
            }
            false
        })
        .await
        .unwrap_or(false);
        if !observed {
            return Err(fail(
                "skew probe deliberate exception: no exceptionThrown event before the deadline",
            ));
        }
        println!("skew probe deliberate exception: observed ({DELIBERATE})");

        // Grace period so trailing invalids arrive before teardown.
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verdicts, all fatal on nonzero.
        if command_errors != 0 {
            return Err(fail(format!(
                "skew probe saw {command_errors} command error(s)"
            )));
        }
        let unexpected: Vec<String> = page_errors
            .messages()
            .into_iter()
            .filter(|message| !message.contains(DELIBERATE))
            .collect();
        if !unexpected.is_empty() {
            return Err(fail(format!(
                "skew probe page errors: unexpected {unexpected:?}"
            )));
        }
        Ok(())
    }
    .await;
    let errors = driver::HandlerErrors::new();
    let (shutdown_result, confirmed) =
        shutdown::finish_browser(browser, drain, &errors, outcome.is_ok()).await;
    let cleanup = cleanup_profile(dirs, confirmed);
    let failures: Vec<String> = [
        outcome.err(),
        shutdown_result.err(),
        teardown_counter_verdict(&counters).err(),
        cleanup.err(),
    ]
    .into_iter()
    .flatten()
    .map(|error| error.to_string())
    .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(fail(failures.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfirmed_shutdown_preserves_profile() {
        let root = tempfile::tempdir().unwrap();
        let dirs = IsolatedDirs::create(root.path()).unwrap();
        let profile = dirs.profile_dir.clone();
        assert!(cleanup_profile(dirs, false).is_err());
        assert!(profile.exists());
    }

    #[test]
    fn handler_error_during_teardown_is_fatal() {
        let counters = Counters::default();
        counters.record_handler_error("late handler error".to_owned());
        assert!(teardown_counter_verdict(&counters).is_err());
    }

    #[test]
    fn required_invalid_during_teardown_is_fatal() {
        let counters = Counters::default();
        counters.record_invalid(r#"{"method":"Page.frameNavigated"}"#);
        assert!(teardown_counter_verdict(&counters).is_err());
    }

    #[test]
    fn probe_uses_guarded_launcher() {
        let source = include_str!("skew_probe.rs");
        assert!(!source.contains(concat!("Browser::", "launch(config)")));
    }
}
