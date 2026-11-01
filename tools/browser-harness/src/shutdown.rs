use boardstudio_browser_harness::driver::HandlerErrors;
use boardstudio_browser_harness::{Error, Result};
use chromiumoxide::Browser;
use std::time::Duration;

/// Close and reap the browser; return whether its termination was confirmed.
pub(super) async fn finish_browser(
    mut browser: Browser,
    drain: tokio::task::JoinHandle<()>,
    handler_errors: &HandlerErrors,
    boundaries_complete: bool,
) -> (Result<()>, bool) {
    let limit = Duration::from_secs(60);
    let before_close = handler_errors.fail_if_any();
    let close = tokio::time::timeout(limit, browser.close()).await;
    let wait = if matches!(close, Ok(Ok(_))) {
        Some(tokio::time::timeout(limit, browser.wait()).await)
    } else {
        None
    };
    let mut kill = None;
    let mut final_wait = None;
    if !matches!(wait, Some(Ok(Ok(Some(_))))) {
        kill = Some(tokio::time::timeout(limit, browser.kill()).await);
        final_wait = Some(tokio::time::timeout(limit, browser.wait()).await);
    }
    let status = final_wait.as_ref().or(wait.as_ref());
    let confirmed = matches!(status, Some(Ok(Ok(Some(_)))));
    let success = matches!(status, Some(Ok(Ok(Some(status)))) if status.success());
    drop(browser);
    let mut drain = drain;
    let drained = tokio::time::timeout(limit, &mut drain).await;
    if drained.is_err() {
        drain.abort();
        let _ = drain.await;
    }
    let mut errors = Vec::new();
    if !matches!(close, Ok(Ok(_))) || !success || !matches!(drained, Ok(Ok(()))) || kill.is_some() {
        errors.push(format!("browser shutdown failed: close={close:?}, wait={wait:?}, kill={kill:?}, final_wait={final_wait:?}, drain={drained:?}"));
    }
    if let Err(error) = handler_errors.fail_if_any() {
        let expected_reset = boundaries_complete
            && before_close.is_ok()
            && matches!(close, Ok(Ok(_)))
            && success
            && kill.is_none()
            && matches!(drained, Ok(Ok(())))
            && error.to_string()
                == "drag browser smoke saw 1 browser handler error(s): WebSocket protocol reset without closing handshake";
        if expected_reset {
            eprintln!(
                "browser shutdown: expected post-boundary WebSocket reset after clean child exit"
            );
        } else {
            errors.push(error.to_string());
        }
    }
    let result = if errors.is_empty() {
        Ok(())
    } else {
        Err(Error(errors.join("; ")))
    };
    (result, confirmed)
}

// Both consumers must finish removing their owned profile before success.
pub(super) fn remove_profile(dirs: &boardstudio_browser_harness::IsolatedDirs) -> Result<()> {
    dirs.cleanup_after_termination()
}
