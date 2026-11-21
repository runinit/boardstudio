//! Browser driving primitives used by the real-page check binaries.
//!
//! Every helper here exists because a current check script uses it: isolated
//! launch, role lookup, bounded waits, trusted input, evaluation, fatal error
//! capture and screenshots. Nothing more is added.

mod interaction;
pub use interaction::*;

#[cfg(test)]
mod redaction_tests;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::cdp::{
    BackendNodeId, Browser, BrowserConfig, BrowserContextId, CdpError, DispatchKeyEventType,
    DispatchMouseEventType, EventExceptionThrown, EventTargetDestroyed, Handler, MouseButton, Page,
    RemoteObjectId, Viewport,
};
use futures::StreamExt;
use serde_json::{json, Value};

use crate::{Error, IsolatedDirs, Result};

/// Viewport every real-page check uses.
pub const VIEWPORT_WIDTH: u32 = 1513;
/// Viewport every real-page check uses.
pub const VIEWPORT_HEIGHT: u32 = 1043;

/// Modifier bit for the Control key in `Input.dispatch*` calls.
pub const CTRL_MODIFIER: i64 = 2;

/// Launch flags every real-page check uses.
pub const LAUNCH_ARGS: [&str; 2] = ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"];

/// Retained from the pre-migration tree for the `skew-probe` diagnostic
/// (decoupled Chrome-pin protocol-skew probe): the migration's shared driver
/// surfaces invalid messages and fails, so it no longer needs these counters.
/// Classification of an unparsable (`InvalidMessage`) CDP event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidKind {
    /// The harness drives this domain; losing one of its events may hide a
    /// real failure, so any required invalid fails the run.
    Required,
    /// Outside the harness's used surface; tolerated and counted.
    Benign,
}

/// CDP domains the harness drives. Every command and listener in this module
/// targets one of these, so an unparsable event here may hide a real loss.
const REQUIRED_CDP_PREFIXES: [&str; 7] = [
    "Accessibility.",
    "Browser.",
    "DOM.",
    "Input.",
    "Page.",
    "Runtime.",
    "Target.",
];

/// True when `method` belongs to the harness's used CDP surface.
pub fn required_cdp_method(method: &str) -> bool {
    REQUIRED_CDP_PREFIXES
        .iter()
        .any(|prefix| method.starts_with(prefix))
}

/// Classify an unparsable CDP event method. A missing method is Required:
/// failing loud beats a green run on an event we could not even name.
pub fn classify_invalid(method: &str) -> InvalidKind {
    if method.is_empty() || required_cdp_method(method) {
        InvalidKind::Required
    } else {
        InvalidKind::Benign
    }
}

/// Per-method counts of tolerated unparsable CDP events, for later receipts.
#[derive(Debug, Default)]
pub struct InvalidCounts {
    counts: std::collections::HashMap<String, usize>,
}

impl InvalidCounts {
    /// No invalids recorded yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Count one unparsable event for `method` (`""` when it had none).
    pub fn record(&mut self, method: &str) {
        *self.counts.entry(method.to_owned()).or_default() += 1;
    }

    /// All unparsable events recorded so far.
    pub fn total(&self) -> usize {
        self.counts.values().sum()
    }

    /// Unparsable events on the harness's used surface; nonzero fails a run.
    pub fn required_total(&self) -> usize {
        self.counts
            .iter()
            .filter(|(method, _)| classify_invalid(method) == InvalidKind::Required)
            .map(|(_, count)| count)
            .sum()
    }

    /// Per-method `(method, count, kind)` rows sorted by method.
    pub fn sorted(&self) -> Vec<(String, usize, InvalidKind)> {
        let mut rows: Vec<(String, usize, InvalidKind)> = self
            .counts
            .iter()
            .map(|(method, count)| (method.clone(), *count, classify_invalid(method)))
            .collect();
        rows.sort_by(|left, right| left.0.cmp(&right.0));
        rows
    }
}

/// Extract the CDP `method` from a raw invalid payload, `""` when absent.
pub fn invalid_method(raw: &str) -> String {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

/// Launch Chrome with the shared flags and viewport under an isolated profile.
///
/// Invalid protocol messages are surfaced and fail the run. The child's
/// `XDG_CONFIG_HOME` is isolated from the user's configuration.
pub async fn launch_chrome(chrome: &str, dirs: &IsolatedDirs) -> Result<(Browser, Handler)> {
    launch_chrome_with_args(chrome, dirs, &[]).await
}

/// Launch Chrome with additional arguments for this run.
pub async fn launch_chrome_with_args(
    chrome: &str,
    dirs: &IsolatedDirs,
    args: &[&str],
) -> Result<(Browser, Handler)> {
    for arg in args {
        let Some(value) = arg.strip_prefix("--blink-settings=") else {
            return Err(Error(format!(
                "unsupported additional Chrome argument: {arg}"
            )));
        };
        if value.is_empty()
            || value
                .chars()
                .any(|c| !c.is_ascii_alphanumeric() && c != '=' && c != ',' && c != '_')
        {
            return Err(Error(format!(
                "unsupported additional Chrome argument: {arg}"
            )));
        }
    }
    let config = BrowserConfig::builder()
        .chrome_executable(chrome)
        .user_data_dir(&dirs.profile_dir)
        .no_sandbox()
        .surface_invalid_messages()
        .args(LAUNCH_ARGS)
        .args(args.iter().map(|arg| arg.strip_prefix("--").unwrap_or(arg)))
        .viewport(Viewport {
            width: VIEWPORT_WIDTH,
            height: VIEWPORT_HEIGHT,
            ..Default::default()
        })
        .window_size(VIEWPORT_WIDTH, VIEWPORT_HEIGHT)
        .env(
            "XDG_CONFIG_HOME",
            dirs.xdg_config_home().to_string_lossy().as_ref(),
        )
        .build()
        .map_err(|error| {
            Error(format!(
                "drag browser smoke cannot build a browser config: {error}"
            ))
        })?;
    // The vendored launch path uses ChromeMessage when child cleanup is unconfirmed.
    dirs.preserve.set(true);
    match Browser::launch(config).await {
        Ok(browser) => Ok(browser),
        Err(error) => {
            if !matches!(error, CdpError::ChromeMessage(_)) {
                dirs.preserve.set(false);
            } else {
                eprintln!(
                    "browser termination unconfirmed; preserving profile at {}",
                    dirs.profile_dir.display()
                );
            }
            Err(Error(format!(
                "drag browser smoke cannot start Chrome: {error}"
            )))
        }
    }
}

/// Handler errors observed while the run is active.
///
/// Protocol and command-routing failures, including invalid events, fail the run.
#[derive(Debug, Default)]
pub struct HandlerErrors {
    count: AtomicUsize,
    first: std::sync::Mutex<Vec<String>>,
}

impl HandlerErrors {
    /// No errors recorded yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of handler errors recorded so far.
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    fn record(&self, message: String) {
        self.count.fetch_add(1, Ordering::Relaxed);
        let mut first = self
            .first
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        first.push(message);
    }

    /// Fail the run when any handler error was recorded.
    pub fn fail_if_any(&self) -> Result<()> {
        if self.count() == 0 {
            return Ok(());
        }
        let first = self
            .first
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Err(Error(format!(
            "drag browser smoke saw {} browser handler error(s): {}",
            self.count(),
            first.join("; ")
        )))
    }
}

/// Drain the browser handler for the duration of the run.
///
/// Returned errors are recorded with full redacted payloads; the task ends
/// when the caller drops the browser or aborts the handle.
pub fn spawn_handler_drain(
    handler: Handler,
    errors: Arc<HandlerErrors>,
    page_errors: Arc<PageErrors>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut handler = handler;
        while let Some(event) = handler.next().await {
            if let Err(error) = event {
                let message = match error {
                    CdpError::InvalidMessage(payload, cause) => {
                        format!(
                            "invalid protocol message: {cause}; payload: {}",
                            redact(&payload)
                        )
                    }
                    CdpError::Ws(error)
                        if format!("{error:?}") == "Protocol(ResetWithoutClosingHandshake)" =>
                    {
                        "WebSocket protocol reset without closing handshake".to_owned()
                    }
                    other => other.to_string(),
                };
                eprintln!("browser handler error: {message}");
                errors.record(message);
            }
        }
        let mut listeners = {
            let mut state = page_errors
                .listeners
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.0 = true;
            std::mem::take(&mut state.1)
        };
        while let Some(result) = listeners.join_next().await {
            if let Err(error) = result {
                errors.record(format!("page error listener failed: {error}"));
            }
        }
    })
}

/// Redact secret-bearing headers without truncating the protocol event.
///
/// `pub` for the retained `skew-probe` diagnostic; the shared driver also
/// uses it internally.
pub fn redact(message: &str) -> String {
    match serde_json::from_str::<Value>(message) {
        Ok(mut value) => {
            scrub_json(&mut value);
            value.to_string()
        }
        Err(_) => "[unparseable protocol message redacted]".to_owned(),
    }
}

fn scrub_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let sensitive_header = map.get("name").and_then(Value::as_str).is_some_and(|name| {
                let name = name.to_lowercase();
                name.contains("cookie") || name.contains("authoriz")
            });
            if sensitive_header {
                if let Some(value) = map.get_mut("value") {
                    *value = Value::String("[redacted]".to_owned());
                }
            }
            for (key, entry) in map.iter_mut() {
                let lower = key.to_lowercase();
                if matches!(
                    lower.as_str(),
                    "headerstext"
                        | "postdata"
                        | "postdataentries"
                        | "cookie"
                        | "cookies"
                        | "cookieline"
                ) {
                    *entry = Value::String("[redacted]".to_owned());
                } else if (lower == "headers"
                    || lower == "responseheaders"
                    || lower == "requestheaders")
                    && entry.is_object()
                {
                    if let Value::Object(headers) = entry {
                        for (name, header) in headers.iter_mut() {
                            let header_name = name.to_lowercase();
                            if header_name.contains("cookie")
                                || header_name.contains("authoriz")
                                || header_name == "set-cookie"
                            {
                                *header = Value::String("[redacted]".to_owned());
                            }
                        }
                    }
                } else {
                    scrub_json(entry);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_json(item);
            }
        }
        _ => {}
    }
}

/// Page errors that must fail the run.
///
/// Collects `Runtime.exceptionThrown` events, which cover both uncaught page
/// exceptions and unhandled promise rejections.
#[derive(Debug, Default)]
pub struct PageErrors {
    messages: std::sync::Mutex<Vec<String>>,
    listeners: std::sync::Mutex<(bool, tokio::task::JoinSet<()>)>,
    completions:
        std::sync::Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Receiver<()>>>,
}

impl PageErrors {
    /// No errors collected yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the collected page error messages.
    pub fn messages(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Fail the run when any page error was collected.
    pub fn fail_if_any(&self) -> Result<()> {
        let messages = self.messages();
        if messages.is_empty() {
            return Ok(());
        }
        Err(Error(format!(
            "page raised {} uncaught browser error(s): {}",
            messages.len(),
            messages.join("; ")
        )))
    }
}

/// Subscribe to `Runtime.exceptionThrown` on `page` for the rest of the run.
pub async fn spawn_page_error_listener(page: &Page, errors: Arc<PageErrors>) -> Result<()> {
    let mut exceptions = page
        .event_listener::<EventExceptionThrown>()
        .await
        .map_err(|error| {
            Error(format!(
                "drag browser smoke cannot listen for page errors: {error}"
            ))
        })?;
    let mut listeners = errors
        .listeners
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if listeners.0 {
        return Err(Error(
            "cannot listen for page errors after handler termination".to_owned(),
        ));
    }
    let listener_errors = Arc::clone(&errors);
    let (complete, completion) = tokio::sync::oneshot::channel();
    errors
        .completions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(page.target_id().inner().clone(), completion);
    listeners.1.spawn(async move {
        while let Some(event) = exceptions.next().await {
            let detail = &event.exception_details;
            let text = detail
                .exception
                .as_ref()
                .and_then(|exception| exception.description.clone())
                .unwrap_or_else(|| detail.text.clone());
            let mut messages = listener_errors
                .messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            messages.push(text);
        }
        let _ = complete.send(());
    });
    Ok(())
}

/// Open a fresh page in its own browser context (isolated storage).
///
/// The caller must dispose the context when the scenario finishes; an
/// incognito context never shares IndexedDB with any other run.
pub async fn new_isolated_page(
    browser: &Browser,
    url: &str,
    errors: &Arc<PageErrors>,
) -> Result<(BrowserContextId, Page)> {
    let context = browser.create_browser_context().await.map_err(|error| {
        Error(format!(
            "cannot create an isolated browser context: {error}"
        ))
    })?;
    let page = browser
        .new_page_in_context(url, &context)
        .await
        .map_err(|error| Error(format!("cannot open an isolated page: {error}")))?;
    spawn_page_error_listener(&page, Arc::clone(errors)).await?;
    Ok((context, page))
}

/// Close an observed page and dispose its context after its listener drains.
pub async fn dispose_isolated_page(
    browser: &Browser,
    context: Option<BrowserContextId>,
    page: Page,
    errors: &PageErrors,
) -> Result<()> {
    let target = page.target_id().clone();
    let boundary = async {
        let mut markers = browser
            .event_listener::<EventTargetDestroyed>()
            .await
            .map_err(|error| Error(format!("cannot listen for target destruction: {error}")))?;
        page.close()
            .await
            .map_err(|error| Error(format!("cannot close target {target:?}: {error}")))?;
        loop {
            match markers.next().await {
                Some(marker) if marker.target_id == target => break,
                Some(_) => {}
                None => {
                    return Err(Error(format!(
                        "transport ended before target {target:?} destruction"
                    )));
                }
            }
        }
        let completion = errors
            .completions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(target.inner())
            .ok_or_else(|| Error(format!("missing listener for target {target:?}")))?;
        completion.await.map_err(|error| {
            Error(format!(
                "page error listener for target {target:?} failed: {error}"
            ))
        })?;
        Ok(())
    };
    let boundary = tokio::time::timeout(Duration::from_secs(60), boundary)
        .await
        .unwrap_or_else(|_| {
            Err(Error(format!(
                "target {target:?} boundary timed out after 60s"
            )))
        });
    let disposal = if let Some(context) = context {
        tokio::time::timeout(
            Duration::from_secs(60),
            browser.dispose_browser_context(&context),
        )
        .await
        .map_err(|_| Error("browser context disposal timed out after 60s".to_owned()))
        .and_then(|result| {
            result
                .map(|_| ())
                .map_err(|error| Error(format!("cannot dispose a browser context: {error}")))
        })
    } else {
        Ok(())
    };
    match (boundary, disposal) {
        (Err(boundary), Err(disposal)) => Err(Error(format!("{boundary}; {disposal}"))),
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    }
}
