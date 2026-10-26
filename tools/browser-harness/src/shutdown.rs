use boardstudio_browser_harness::driver::HandlerErrors;
use boardstudio_browser_harness::{Error, Result};
use chromiumoxide::Browser;
use std::time::Duration;

/// Close the browser and drain its handler before deciding whether a run passed.
pub(super) async fn finish_browser(
    mut browser: Browser,
    drain: tokio::task::JoinHandle<()>,
    handler_errors: &HandlerErrors,
) -> Result<()> {
    let close = tokio::time::timeout(Duration::from_secs(60), browser.close()).await;
    if !matches!(close, Ok(Ok(_))) {
        browser.kill().await;
    }
    let wait = tokio::time::timeout(Duration::from_secs(60), browser.wait()).await;
    if !matches!(wait, Ok(Ok(_))) {
        browser.kill().await;
    }
    drop(browser);
    let mut drain = drain;
    let drained = tokio::time::timeout(Duration::from_secs(60), &mut drain).await;
    if drained.is_err() {
        drain.abort();
        let _ = drain.await;
    }
    handler_errors.fail_if_any()?;
    if !matches!(close, Ok(Ok(_))) || !matches!(wait, Ok(Ok(_))) || !matches!(drained, Ok(Ok(()))) {
        return Err(Error(format!(
            "browser shutdown failed: close={close:?}, wait={wait:?}, drain={drained:?}"
        )));
    }
    Ok(())
}
