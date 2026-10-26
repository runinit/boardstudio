//! Negative controls for the Rust browser harness.
//!
//! Each probe must fail loudly with an exact, actionable message: page
//! exceptions and unhandled rejections fail the run (never only logged),
//! protocol command failures, timeouts, startup failures and assertion
//! failures stay distinguishable. Lookup probes cover exact/scoped names and
//! absent/ambiguous/non-actionable targets. IndexedDB isolation between
//! browser contexts is verified on a local page, never in `cargo test`.

#[path = "../shutdown.rs"]
mod shutdown;

use std::sync::Arc;
use std::time::Duration;

use boardstudio_browser_harness::driver::{self, HandlerErrors, PageErrors};
use boardstudio_browser_harness::{IsolatedDirs, Result};

use boardstudio_browser_harness::Error;

fn fail(message: impl Into<String>) -> Error {
    Error::new(message)
}

const FIXTURE: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>probe</title>
<style>button { font-size: 16px; } #zero { width: 0; height: 0; overflow: hidden; border: 0; padding: 0; }</style>
</head><body>
<button id="one">Fit board</button>
<button id="two">Same</button>
<button id="three">Same</button>
<button id="zero" aria-label="Zero target">Zero</button>
<div id="hidden" role="button" aria-label="Hidden control" style="display:none">Hidden</div>
</body></html>"#;

/// Serve the fixture on 127.0.0.1 and return the base URL.
async fn serve_fixture() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| {
            fail(format!(
                "negative probe cannot bind a fixture server: {error}"
            ))
        })?;
    let url = format!(
        "http://127.0.0.1:{}/probe",
        listener
            .local_addr()
            .map_err(|error| fail(format!("negative probe has no address: {error}")))?
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

async fn expect_error(label: &str, outcome: Result<()>, needle: &str) -> Result<()> {
    match outcome {
        Ok(()) => Err(fail(format!(
            "negative probe {label}: expected a failure, got success"
        ))),
        Err(error) => {
            let message = error.to_string();
            if message.contains(needle) {
                println!("negative probe {label}: failed as expected ({needle})");
                Ok(())
            } else {
                Err(fail(format!(
                    "negative probe {label}: error {message:?} does not name {needle:?}"
                )))
            }
        }
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => {
            println!("PASS negative probe: all failure modes stay loud and scoped");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let chrome = std::env::var("CHROME_BIN")
        .map_err(|_| fail("negative probe needs CHROME_BIN from its runner"))?;
    let tmp_root = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_owned());
    let dirs = IsolatedDirs::create(std::path::Path::new(&tmp_root)).map_err(|error| {
        fail(format!(
            "negative probe cannot isolate its profile: {error}"
        ))
    })?;
    let (url, server) = serve_fixture().await?;
    let (browser, handler) = driver::launch_chrome(&chrome, &dirs).await?;
    let handler_errors = Arc::new(HandlerErrors::new());
    let page_errors = Arc::new(PageErrors::new());
    let drain = driver::spawn_handler_drain(
        handler,
        Arc::clone(&handler_errors),
        Arc::clone(&page_errors),
    );

    let outcome = async {
    // Startup failure: an executable that is not Chrome cannot launch.
    let bad_dirs = IsolatedDirs::create(std::path::Path::new(&tmp_root)).map_err(|error| {
        fail(format!(
            "negative probe cannot isolate its profile: {error}"
        ))
    })?;
    expect_error(
        "startup",
        driver::launch_chrome("/bin/false", &bad_dirs)
            .await
            .map(|_| ()),
        "cannot start Chrome",
    )
    .await?;

    let page = browser
        .new_page("about:blank")
        .await
        .map_err(|error| fail(format!("negative probe cannot open a page: {error}")))?;
    driver::spawn_page_error_listener(&page, Arc::clone(&page_errors)).await?;
    tokio::time::timeout(Duration::from_secs(60), page.goto(url.as_str()))
        .await
        .map_err(|_| fail("negative probe timed out opening the fixture"))?
        .map_err(|error| fail(format!("negative probe cannot open the fixture: {error}")))?;

    // Page exception: a throwing click handler fails the run, not just logs.
    driver::eval(
        &page,
        r#"document.querySelector('#one').addEventListener('click', () => { throw new Error('probe page error'); });"#,
    )
    .await?;
    driver::click_css(&page, "#one").await?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    expect_error(
        "page exception",
        page_errors.fail_if_any(),
        "probe page error",
    )
    .await?;

    // Exercise the same collector and fatal verdict used by the drag consumer.
    driver::eval(&page, "Promise.reject(new Error('probe rejection')); 0;").await?;
    tokio::time::timeout(Duration::from_secs(10), async {
        while !page_errors.messages().iter().any(|message| message.contains("probe rejection")) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .map_err(|_| fail("negative probe timed out waiting for the rejection"))?;
    expect_error(
        "unhandled rejection",
        page_errors.fail_if_any(),
        "probe rejection",
    )
    .await?;

    // Protocol command failure: evaluating throwing JS is a fatal Err.
    expect_error(
        "command",
        driver::eval(&page, "(() => { throw new Error('probe boom'); })()")
            .await
            .map(|_| ()),
        "probe boom",
    )
    .await?;

    // Timeout: a false condition fails with its bound and condition named.
    expect_error(
        "timeout",
        driver::wait_for(
            &page,
            "false",
            "the impossible condition",
            Duration::from_secs(1),
        )
        .await,
        "the impossible condition",
    )
    .await?;

    // Exact names: `Fit` matches nothing, `Fit board` matches once.
    if !driver::find_role(&page, None, "button", "Fit")
        .await?
        .is_empty()
    {
        return Err(fail(
            "negative probe exact name: `Fit` must not prefix-match",
        ));
    }
    if driver::find_role(&page, None, "button", "Fit board")
        .await?
        .len()
        != 1
    {
        return Err(fail(
            "negative probe exact name: `Fit board` must match once",
        ));
    }
    println!("negative probe exact names: scoped to one match");

    // Absent target: the click bound names the missing control.
    expect_error(
        "absent",
        driver::click_role_timeout(
            &page,
            None,
            "button",
            "No such control",
            Duration::from_secs(2),
        )
        .await,
        "No such control",
    )
    .await?;

    // Ambiguous target: two `Same` buttons fail instead of picking one.
    expect_error(
        "ambiguous",
        driver::click_role_timeout(&page, None, "button", "Same", Duration::from_secs(2)).await,
        "ambiguous",
    )
    .await?;

    // Non-actionable target: a zero-area control has no visible geometry.
    expect_error(
        "non-actionable",
        driver::click_role_timeout(&page, None, "button", "Zero target", Duration::from_secs(2))
            .await,
        "no visible geometry",
    )
    .await?;

    // Hidden target: display:none never becomes actionable.
    expect_error(
        "hidden",
        driver::click_role_timeout(
            &page,
            None,
            "button",
            "Hidden control",
            Duration::from_secs(2),
        )
        .await,
        "Hidden control",
    )
    .await?;

    // IndexedDB isolation: a marker written in one context is absent in another.
    let first = driver::new_isolated_page(&browser, &url, &page_errors).await?;
    driver::eval(
        &first.1,
        r#"new Promise((resolve, reject) => {
      const request = indexedDB.open('probe-isolation');
      request.onsuccess = () => {
        const db = request.result;
        if (!db.objectStoreNames.contains('marks')) {
          db.close();
          const upgrade = indexedDB.open('probe-isolation', 2);
          upgrade.onupgradeneeded = () => upgrade.result.createObjectStore('marks');
          upgrade.onsuccess = () => { upgrade.result.close(); resolve(0); };
          upgrade.onerror = () => reject(upgrade.error);
        } else {
          const put = db.transaction('marks', 'readwrite').objectStore('marks').put('here', 'marker');
          put.onsuccess = () => { db.close(); resolve(0); };
          put.onerror = () => reject(put.error);
        }
      };
      request.onerror = () => reject(request.error);
    })"#,
    )
    .await?;
    driver::dispose_isolated_page(&browser, first.0).await?;
    let second = driver::new_isolated_page(&browser, &url, &page_errors).await?;
    let marker = driver::eval(
        &second.1,
        r#"new Promise(resolve => {
      const request = indexedDB.open('probe-isolation');
      request.onsuccess = () => {
        const db = request.result;
        if (!db.objectStoreNames.contains('marks')) { db.close(); resolve('absent'); return; }
        const get = db.transaction('marks').objectStore('marks').get('marker');
        get.onsuccess = () => { db.close(); resolve(get.result ?? 'absent'); };
        get.onerror = () => resolve('absent');
      };
      request.onerror = () => resolve('absent');
    })"#,
    )
    .await?;
    driver::dispose_isolated_page(&browser, second.0).await?;
    if marker != serde_json::Value::String("absent".to_owned()) {
        return Err(fail(format!(
            "negative probe isolation: second context saw {marker:?}, want absent"
        )));
    }
    println!("negative probe isolation: contexts do not share IndexedDB");

    // Do not wait for the listener: the final verdict must join it.
    driver::eval(&page, "Promise.reject(new Error('probe late rejection')); 0;").await?;
    Ok(())
    }.await;
    let (result, confirmed) = shutdown::finish_browser(browser, drain, &handler_errors).await;
    if !confirmed {
        eprintln!(
            "browser termination unconfirmed; preserving profile at {}",
            dirs.profile_dir.display()
        );
        std::mem::forget(dirs);
    }
    server.abort();
    // The fixture page itself stayed clean apart from the deliberate errors.
    let unexpected: Vec<String> = page_errors
        .messages()
        .into_iter()
        .filter(|message| {
            !message.contains("probe page error")
                && !message.contains("probe rejection")
                && !message.contains("probe late rejection")
        })
        .collect();
    let mut errors: Vec<String> = [outcome.err(), result.err()]
        .into_iter()
        .flatten()
        .map(|error| error.to_string())
        .collect();
    if !page_errors
        .messages()
        .iter()
        .any(|message| message.contains("probe late rejection"))
    {
        errors
            .push("negative probe: late rejection was not collected before the verdict".to_owned());
    } else {
        println!("negative probe late rejection: collected before the verdict");
    }
    if !unexpected.is_empty() {
        errors.push(format!(
            "negative probe page errors: unexpected {unexpected:?}"
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(fail(errors.join("; ")))
    }
}
