//! Thin direct-CDP client for the browser harness.
//!
//! The harness drives ~17 CDP commands and a handful of events, so it speaks
//! the protocol directly over `async-tungstenite` instead of compiling the
//! thousands of generated `chromiumoxide_cdp` serde structs. One WebSocket
//! multiplexes browser-level commands (`sessionId: None`) and flattened
//! per-target sessions (`sessionId: Some`) by monotonic `id`, exactly like the
//! previous client. Launch flags, the launch timeout/reaping contract, worker
//! session forwarding and invalid-message surfacing keep their old behavior.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use async_tungstenite::tungstenite::protocol::WebSocketConfig;
use async_tungstenite::{tokio::ConnectStream, tungstenite::Message as WsMessage, WebSocketStream};
use futures::{Stream, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

/// Default `Browser::launch` timeout, covering URL discovery and handshake.
pub const LAUNCH_TIMEOUT: Duration = Duration::from_secs(20);
/// Per-command timeout, matching the previous client's request timeout.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Bound for the load event a navigation waits for.
const NAVIGATION_TIMEOUT: Duration = Duration::from_secs(30);

/// A CDP or transport failure.
#[derive(Debug)]
pub enum CdpError {
    /// The WebSocket transport failed.
    Ws(async_tungstenite::tungstenite::Error),
    /// An OS-level failure.
    Io(std::io::Error),
    /// A command or navigation exceeded its bound.
    Timeout,
    /// The transport closed without an error and without a close command.
    NoResponse,
    /// A protocol-level failure with the server's message.
    ChromeMessage(String),
    /// An unparsable frame or an event missing required fields: the raw
    /// payload plus the cause.
    InvalidMessage(String, String),
    /// Launch exceeded its bound during discovery or handshake.
    LaunchTimeout(String),
    /// The browser exited before its DevTools URL appeared.
    LaunchExit(String),
    /// Reading the browser's stderr failed.
    LaunchIo(String),
}

impl std::fmt::Display for CdpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CdpError::Ws(error) => write!(formatter, "{error}"),
            CdpError::Io(error) => write!(formatter, "{error}"),
            CdpError::Timeout => formatter.write_str("Request timed out."),
            CdpError::NoResponse => {
                formatter.write_str("Received no response from the chromium instance.")
            }
            CdpError::ChromeMessage(message)
            | CdpError::LaunchTimeout(message)
            | CdpError::LaunchExit(message)
            | CdpError::LaunchIo(message) => formatter.write_str(message),
            CdpError::InvalidMessage(payload, cause) => {
                write!(formatter, "invalid protocol message: {cause}: {payload}")
            }
        }
    }
}

impl std::error::Error for CdpError {}

impl From<std::io::Error> for CdpError {
    fn from(error: std::io::Error) -> Self {
        CdpError::Io(error)
    }
}

/// A CDP target identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TargetId(String);

impl TargetId {
    /// The raw identifier.
    pub fn inner(&self) -> &String {
        &self.0
    }
}

/// A flattened CDP session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl AsRef<str> for SessionId {
    /// The raw identifier.
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A browser context identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrowserContextId(String);

impl BrowserContextId {
    /// The raw identifier.
    pub fn inner(&self) -> &String {
        &self.0
    }
}

/// A DOM backend node identifier.
pub type BackendNodeId = i64;

/// A runtime remote object identifier.
pub type RemoteObjectId = String;

/// Emulation viewport applied to every new page.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// Page width in CSS pixels.
    pub width: u32,
    /// Page height in CSS pixels.
    pub height: u32,
    /// Device scale factor, defaulting to 1.
    pub device_scale_factor: Option<f64>,
    /// Whether to emulate a mobile device.
    pub emulating_mobile: bool,
    /// Whether the screen orientation is landscape.
    pub is_landscape: bool,
    /// Whether touch is available.
    pub has_touch: bool,
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            width: 800,
            height: 600,
            device_scale_factor: None,
            emulating_mobile: false,
            is_landscape: false,
            has_touch: false,
        }
    }
}

/// `Input.dispatchMouseEvent` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchMouseEventType {
    /// A press of the mouse button.
    MousePressed,
    /// A release of the mouse button.
    MouseReleased,
    /// A mouse movement.
    MouseMoved,
    /// A mouse wheel event.
    MouseWheel,
}

impl DispatchMouseEventType {
    /// The CDP wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            DispatchMouseEventType::MousePressed => "mousePressed",
            DispatchMouseEventType::MouseReleased => "mouseReleased",
            DispatchMouseEventType::MouseMoved => "mouseMoved",
            DispatchMouseEventType::MouseWheel => "mouseWheel",
        }
    }
}

/// `Input.dispatchKeyEvent` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKeyEventType {
    /// A raw key down, used for trusted modifiers.
    RawKeyDown,
    /// A key down.
    KeyDown,
    /// A key up.
    KeyUp,
    /// A character event.
    Char,
}

impl DispatchKeyEventType {
    /// The CDP wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            DispatchKeyEventType::RawKeyDown => "rawKeyDown",
            DispatchKeyEventType::KeyDown => "keyDown",
            DispatchKeyEventType::KeyUp => "keyUp",
            DispatchKeyEventType::Char => "char",
        }
    }
}

/// `Input.dispatchMouseEvent` button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// The left button.
    Left,
    /// The middle button.
    Middle,
    /// The right button.
    Right,
    /// No button.
    None,
}

impl MouseButton {
    /// The CDP wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            MouseButton::Left => "left",
            MouseButton::Middle => "middle",
            MouseButton::Right => "right",
            MouseButton::None => "none",
        }
    }
}

/// A typed CDP event subscription.
pub trait EventKind: DeserializeOwned + Send + 'static {
    /// The CDP `method` this event subscribes to.
    fn method_id() -> &'static str;
}

/// `Runtime.exceptionThrown`, covering uncaught page exceptions and unhandled
/// promise rejections.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventExceptionThrown {
    /// The thrown details.
    pub exception_details: ExceptionDetails,
}

impl EventKind for EventExceptionThrown {
    fn method_id() -> &'static str {
        "Runtime.exceptionThrown"
    }
}

/// The details of a thrown page exception.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionDetails {
    /// The exception text.
    pub text: String,
    /// The exception object, when one was thrown.
    pub exception: Option<RemoteObject>,
}

/// A remote runtime object reference.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    /// The human-readable description, when present.
    pub description: Option<String>,
}

/// `Target.targetDestroyed`, the page boundary marker.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTargetDestroyed {
    /// The destroyed target.
    pub target_id: TargetId,
}

impl EventKind for EventTargetDestroyed {
    fn method_id() -> &'static str {
        "Target.targetDestroyed"
    }
}

/// `Browser.downloadWillBegin`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDownloadWillBegin {
    /// The download GUID.
    pub guid: String,
    /// The suggested file name.
    pub suggested_filename: String,
}

impl EventKind for EventDownloadWillBegin {
    fn method_id() -> &'static str {
        "Browser.downloadWillBegin"
    }
}

/// `Browser.downloadProgress`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDownloadProgress {
    /// The download GUID.
    pub guid: String,
    /// The download state.
    pub state: DownloadProgressState,
}

impl EventKind for EventDownloadProgress {
    fn method_id() -> &'static str {
        "Browser.downloadProgress"
    }
}

/// The state of a download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DownloadProgressState {
    /// The download is progressing.
    InProgress,
    /// The download completed.
    Completed,
    /// The download was canceled.
    Canceled,
}

/// `Browser.getVersion` product info.
#[derive(Debug, Clone)]
pub struct Version {
    /// The product string, e.g. `Chrome/141.0.0.0`.
    pub product: String,
}

/// A stream of one CDP event kind. Ends when its target is destroyed or the
/// transport closes. An event that no longer parses as `T` is reported on the
/// handler instead of silently dropped.
#[derive(Debug)]
pub struct EventStream<T: EventKind> {
    events: mpsc::UnboundedReceiver<(Value, Option<String>)>,
    inner: std::sync::Weak<Inner>,
    marker: std::marker::PhantomData<T>,
}

impl<T: EventKind> EventStream<T> {
    fn new(
        events: mpsc::UnboundedReceiver<(Value, Option<String>)>,
        inner: std::sync::Weak<Inner>,
    ) -> Self {
        Self {
            events,
            inner,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T: EventKind + Unpin> Stream for EventStream<T> {
    type Item = Arc<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.events).poll_recv(cx) {
                Poll::Ready(Some((value, raw))) => {
                    match serde_json::from_value::<T>(value.clone()) {
                        Ok(event) => return Poll::Ready(Some(Arc::new(event))),
                        Err(error) => {
                            if let Some(inner) = self.inner.upgrade() {
                                let payload = raw.unwrap_or_else(|| value.to_string());
                                inner.push_invalid(&payload, error.to_string());
                            }
                        }
                    }
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[derive(Debug)]
struct Subscription {
    target: Option<String>,
    method: &'static str,
    sender: mpsc::UnboundedSender<(Value, Option<String>)>,
}

type CommandOutcome = Result<Value, CdpError>;

#[derive(Debug)]
struct PendingEntry {
    sender: oneshot::Sender<CommandOutcome>,
    started: Instant,
    is_browser_close: bool,
}

#[derive(Debug)]
struct Inner {
    commands: mpsc::UnboundedSender<String>,
    pending: Mutex<HashMap<u64, PendingEntry>>,
    next_id: AtomicU64,
    child: AsyncMutex<Option<tokio::process::Child>>,
    ws_url: String,
    subscriptions: Mutex<Vec<Subscription>>,
    load_waiters: Mutex<HashMap<String, Vec<oneshot::Sender<()>>>>,
    capture_workers: AtomicBool,
    closing: AtomicBool,
    surface_invalid: bool,
    request_timeout: Duration,
    viewport: Option<Viewport>,
    handler_errors: Mutex<Option<mpsc::UnboundedSender<CdpError>>>,
}

impl Inner {
    fn push_handler_error(&self, error: CdpError) {
        if let Some(sender) = self
            .handler_errors
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_ref()
        {
            let _ = sender.send(error);
        }
    }

    fn push_invalid(&self, payload: &str, cause: String) {
        if self.surface_invalid {
            self.push_handler_error(CdpError::InvalidMessage(payload.to_owned(), cause));
        }
    }

    /// Allocate an id, register its pending entry and enqueue the command,
    /// returning the id and its response receiver. The caller awaits the
    /// reply via [`Inner::await_submitted`] with `request_timeout`.
    fn submit_tracked(
        &self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<(u64, oneshot::Receiver<CommandOutcome>), CdpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut message = json!({"id": id, "method": method, "params": params});
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                id,
                PendingEntry {
                    sender,
                    started: Instant::now(),
                    is_browser_close: method == "Browser.close" && session.is_none(),
                },
            );
        if self.commands.send(message.to_string()).is_err() {
            self.pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id);
            return Err(CdpError::ChromeMessage("CDP transport closed".to_owned()));
        }
        Ok((id, receiver))
    }

    /// Await a command submitted via [`Inner::submit_tracked`].
    async fn await_submitted(
        &self,
        id: u64,
        receiver: oneshot::Receiver<CommandOutcome>,
    ) -> Result<Value, CdpError> {
        match tokio::time::timeout(self.request_timeout, receiver).await {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(_)) => Err(CdpError::ChromeMessage(
                "CDP transport closed before a response arrived".to_owned(),
            )),
            Err(_) => {
                self.pending
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&id);
                Err(CdpError::Timeout)
            }
        }
    }

    async fn execute(
        &self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, CdpError> {
        let (id, receiver) = self.submit_tracked(method, params, session)?;
        self.await_submitted(id, receiver).await
    }

    /// Send a command without tracking its response. Responses to untracked
    /// ids are ignored when they arrive.
    fn send_untracked(&self, method: &str, params: Value, session: Option<&str>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut message = json!({"id": id, "method": method, "params": params});
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        let _ = self.commands.send(message.to_string());
    }

    /// Run a worker-setup command, surfacing its failure on the handler.
    fn spawn_internal(
        self: &Arc<Self>,
        method: &'static str,
        params: Value,
        session: Option<String>,
    ) {
        let inner = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(error) = inner.execute(method, params, session.as_deref()).await {
                inner.push_handler_error(CdpError::ChromeMessage(format!(
                    "worker setup {method} failed: {error}"
                )));
            }
        });
    }

    /// Run ordered setup for one attached waiting worker on a single task:
    /// `Network.enable` first when capturing, then resume. A failing
    /// `Network.enable` never resumes a dedicated worker, so uncaptured worker
    /// requests cannot slip through. A captured waiting service worker pipelines
    /// both commands before awaiting either: Chromium 156 routes its
    /// `Network.enable` to the paused worker thread, which stays silent until
    /// `Runtime.runIfWaitingForDebugger` arrives. A service worker the harness
    /// does not capture is detached on the same task, after its resume.
    fn spawn_worker_setup(
        self: &Arc<Self>,
        child: String,
        parent: String,
        is_service_worker: bool,
        capture: bool,
        waiting: bool,
    ) {
        let inner = Arc::clone(self);
        tokio::spawn(async move {
            if is_service_worker && capture && waiting {
                // Enqueue both synchronously — Network.enable first — before
                // awaiting either, so the wire order is fixed without
                // deadlocking the paused worker thread.
                let network = inner.submit_tracked("Network.enable", json!({}), Some(&child));
                let resume = inner.submit_tracked(
                    "Runtime.runIfWaitingForDebugger",
                    json!({}),
                    Some(&child),
                );
                match network {
                    Ok((id, receiver)) => {
                        if let Err(error) = inner.await_submitted(id, receiver).await {
                            inner.push_handler_error(CdpError::ChromeMessage(format!(
                                "worker setup Network.enable failed: {error}"
                            )));
                        }
                    }
                    Err(error) => {
                        inner.push_handler_error(CdpError::ChromeMessage(format!(
                            "worker setup Network.enable failed: {error}"
                        )));
                    }
                }
                match resume {
                    Ok((id, receiver)) => {
                        if let Err(error) = inner.await_submitted(id, receiver).await {
                            inner.push_handler_error(CdpError::ChromeMessage(format!(
                                "worker setup Runtime.runIfWaitingForDebugger failed: {error}"
                            )));
                        }
                    }
                    Err(error) => {
                        inner.push_handler_error(CdpError::ChromeMessage(format!(
                            "worker setup Runtime.runIfWaitingForDebugger failed: {error}"
                        )));
                    }
                }
                return;
            }
            if waiting {
                if capture {
                    if let Err(error) = inner
                        .execute("Network.enable", json!({}), Some(&child))
                        .await
                    {
                        inner.push_handler_error(CdpError::ChromeMessage(format!(
                            "worker setup Network.enable failed: {error}"
                        )));
                        return;
                    }
                }
                if let Err(error) = inner
                    .execute("Runtime.runIfWaitingForDebugger", json!({}), Some(&child))
                    .await
                {
                    inner.push_handler_error(CdpError::ChromeMessage(format!(
                        "worker setup Runtime.runIfWaitingForDebugger failed: {error}"
                    )));
                }
            } else if capture {
                inner.push_handler_error(CdpError::ChromeMessage(format!(
                    "worker {child:?} attached without pause"
                )));
            }
            if is_service_worker && !(capture && waiting) {
                if let Err(error) = inner
                    .execute(
                        "Target.detachFromTarget",
                        json!({"sessionId": child}),
                        Some(&parent),
                    )
                    .await
                {
                    inner.push_handler_error(CdpError::ChromeMessage(format!(
                        "worker setup Target.detachFromTarget failed: {error}"
                    )));
                }
            }
        });
    }

    fn subscribe<T: EventKind>(self: &Arc<Self>, target: Option<String>) -> EventStream<T> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.subscriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(Subscription {
                target,
                method: T::method_id(),
                sender,
            });
        EventStream::new(receiver, Arc::downgrade(self))
    }

    fn publish(&self, target: Option<&str>, method: &str, value: Value, raw: Option<&str>) {
        let subscriptions = self
            .subscriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for subscription in subscriptions.iter() {
            if subscription.target.as_deref() == target && subscription.method == method {
                let _ = subscription
                    .sender
                    .send((value.clone(), raw.map(str::to_owned)));
            }
        }
    }

    fn remove_target_subscriptions(&self, target: &str) {
        self.subscriptions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|subscription| subscription.target.as_deref() != Some(target));
    }

    fn add_load_waiter(&self, session: &str, waiter: oneshot::Sender<()>) {
        self.load_waiters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(session.to_owned())
            .or_default()
            .push(waiter);
    }

    fn wake_load_waiters(&self, session: &str) {
        if let Some(waiters) = self
            .load_waiters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session)
        {
            for waiter in waiters {
                let _ = waiter.send(());
            }
        }
    }

    fn drop_load_waiters(&self, session: &str) {
        self.load_waiters
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session);
    }
}

/// How to launch the browser.
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    executable: std::path::PathBuf,
    user_data_dir: Option<std::path::PathBuf>,
    no_sandbox: bool,
    surface_invalid: bool,
    launch_timeout: Duration,
    request_timeout: Duration,
    viewport: Option<Viewport>,
    window_size: Option<(u32, u32)>,
    envs: HashMap<String, String>,
    custom_args: Vec<String>,
}

/// Builder for [`BrowserConfig`].
#[derive(Debug, Clone)]
pub struct BrowserConfigBuilder {
    executable: Option<std::path::PathBuf>,
    user_data_dir: Option<std::path::PathBuf>,
    no_sandbox: bool,
    surface_invalid: bool,
    launch_timeout: Duration,
    request_timeout: Duration,
    viewport: Option<Viewport>,
    window_size: Option<(u32, u32)>,
    envs: HashMap<String, String>,
    custom_args: Vec<String>,
}

impl BrowserConfig {
    /// A builder with the previous client's defaults: headless, port 0.
    pub fn builder() -> BrowserConfigBuilder {
        BrowserConfigBuilder {
            executable: None,
            user_data_dir: None,
            no_sandbox: false,
            surface_invalid: false,
            launch_timeout: LAUNCH_TIMEOUT,
            request_timeout: REQUEST_TIMEOUT,
            viewport: Some(Viewport::default()),
            window_size: None,
            envs: HashMap::new(),
            custom_args: Vec::new(),
        }
    }
}

impl BrowserConfigBuilder {
    /// The Chrome executable to launch.
    pub fn chrome_executable(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.executable = Some(path.as_ref().to_path_buf());
        self
    }

    /// The `--user-data-dir` for the run.
    pub fn user_data_dir(mut self, dir: impl AsRef<std::path::Path>) -> Self {
        self.user_data_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Pass `--no-sandbox --disable-setuid-sandbox`.
    pub fn no_sandbox(mut self) -> Self {
        self.no_sandbox = true;
        self
    }

    /// Surface invalid protocol messages as handler errors instead of
    /// dropping them.
    pub fn surface_invalid_messages(mut self) -> Self {
        self.surface_invalid = true;
        self
    }

    /// Bound for launch, covering URL discovery and the WS handshake.
    pub fn launch_timeout(mut self, timeout: Duration) -> Self {
        self.launch_timeout = timeout;
        self
    }

    /// Extra Chrome arguments, with or without leading dashes.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for arg in args {
            self.custom_args.push(arg.as_ref().to_owned());
        }
        self
    }

    /// The emulated viewport applied to every new page.
    pub fn viewport(mut self, viewport: impl Into<Option<Viewport>>) -> Self {
        self.viewport = viewport.into();
        self
    }

    /// The `--window-size` for the run.
    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.window_size = Some((width, height));
        self
    }

    /// An environment variable for the browser child.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.envs.insert(key.into(), value.into());
        self
    }

    /// Build the config; the executable must be set.
    pub fn build(self) -> Result<BrowserConfig, String> {
        Ok(BrowserConfig {
            executable: self.executable.ok_or_else(|| {
                "cannot detect a Chrome executable; configure one explicitly".to_owned()
            })?,
            user_data_dir: self.user_data_dir,
            no_sandbox: self.no_sandbox,
            surface_invalid: self.surface_invalid,
            launch_timeout: self.launch_timeout,
            request_timeout: self.request_timeout,
            viewport: self.viewport,
            window_size: self.window_size,
            envs: self.envs,
            custom_args: self.custom_args,
        })
    }
}

/// Default Chrome flags, carried over from the previous client (via
/// Puppeteer's launcher defaults).
const DEFAULT_ARGS: [&str; 24] = [
    "--disable-background-networking",
    "--enable-features=NetworkService,NetworkServiceInProcess",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-component-extensions-with-background-pages",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-features=TranslateUI",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--force-color-profile=srgb",
    "--metrics-recording-only",
    "--no-first-run",
    "--enable-automation",
    "--password-store=basic",
    "--use-mock-keychain",
    "--enable-blink-features=IdleDetection",
    "--lang=en_US",
];

impl BrowserConfig {
    fn chrome_args(&self) -> Vec<String> {
        let mut args: Vec<String> = DEFAULT_ARGS.map(str::to_owned).to_vec();
        for arg in &self.custom_args {
            let trimmed = arg.trim_start_matches('-');
            args.push(format!("--{trimmed}"));
        }
        if !args
            .iter()
            .any(|arg| arg.starts_with("--remote-debugging-port"))
        {
            args.push("--remote-debugging-port=0".to_owned());
        }
        args.push("--disable-extensions".to_owned());
        let data_dir = self
            .user_data_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("direct-cdp-runner"));
        args.push(format!("--user-data-dir={}", data_dir.display()));
        if let Some((width, height)) = self.window_size {
            args.push(format!("--window-size={width},{height}"));
        }
        if self.no_sandbox {
            args.push("--no-sandbox".to_owned());
            args.push("--disable-setuid-sandbox".to_owned());
        }
        args.push("--headless".to_owned());
        args.push("--hide-scrollbars".to_owned());
        args.push("--mute-audio".to_owned());
        args
    }
}

/// A connected browser: commands, pages and listeners over one WebSocket.
#[derive(Debug, Clone)]
pub struct Browser {
    inner: Arc<Inner>,
}

/// An attached page target with its flattened session.
#[derive(Debug, Clone)]
pub struct Page {
    inner: Arc<Inner>,
    target_id: TargetId,
    session_id: SessionId,
}

/// The connection driver. Yields a handler error per invalid message or
/// transport failure, then ends when the transport closes and every listener
/// has drained.
#[derive(Debug)]
pub struct Handler {
    errors: mpsc::UnboundedReceiver<CdpError>,
    inner: Arc<Inner>,
}

impl Handler {
    /// Observe dedicated worker network traffic as `BoardStudio.NetworkEvent`
    /// on the owning page, before polling this handler.
    pub fn enable_worker_network_capture(&mut self) {
        self.inner.capture_workers.store(true, Ordering::Relaxed);
    }
}

impl Stream for Handler {
    type Item = Result<(), CdpError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.errors).poll_recv(cx) {
            Poll::Ready(Some(error)) => Poll::Ready(Some(Err(error))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Browser {
    /// Launch a new browser and attach to its DevTools WebSocket.
    ///
    /// Fails when initialization exceeds the configured launch timeout. A
    /// failed launch kills and reaps the spawned child; only an unconfirmed
    /// cleanup reports [`CdpError::ChromeMessage`].
    pub async fn launch(config: BrowserConfig) -> Result<(Self, Handler), CdpError> {
        let executable = canonicalize_except_snap(config.executable.clone()).await?;
        let mut command = tokio::process::Command::new(&executable);
        command
            .args(config.chrome_args())
            .envs(&config.envs)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = command.spawn()?;
        let deadline = Instant::now() + config.launch_timeout;
        let initialized = async {
            let url = ws_url_from_output(&mut child, deadline).await?;
            let ws = connect_ws(&url, deadline).await?;
            Ok::<_, CdpError>((url, ws))
        }
        .await;
        match initialized {
            Ok((url, ws)) => Ok(connected(config, url, ws, Some(child))),
            Err(error) => {
                if !matches!(child.try_wait(), Ok(Some(_))) {
                    let cleanup =
                        tokio::time::timeout(Duration::from_secs(10), kill_and_wait(&mut child))
                            .await;
                    if !matches!(cleanup, Ok(Ok(()))) {
                        return Err(CdpError::ChromeMessage(format!(
                            "{error}; browser child cleanup failed or termination \
                             unconfirmed: {cleanup:?}"
                        )));
                    }
                }
                Err(error)
            }
        }
    }

    /// Connect to an already running browser over its DevTools WebSocket URL.
    pub async fn connect(url: impl Into<String>) -> Result<(Self, Handler), CdpError> {
        let url = url.into();
        let config = WebSocketConfig::default()
            .max_message_size(None)
            .max_frame_size(None);
        let (ws, _) = async_tungstenite::tokio::connect_async_with_config(&url, Some(config))
            .await
            .map_err(CdpError::Ws)?;
        let config = BrowserConfig {
            executable: std::path::PathBuf::new(),
            user_data_dir: None,
            no_sandbox: false,
            surface_invalid: true,
            launch_timeout: LAUNCH_TIMEOUT,
            request_timeout: REQUEST_TIMEOUT,
            viewport: None,
            window_size: None,
            envs: HashMap::new(),
            custom_args: Vec::new(),
        };
        Ok(connected(config, url, ws, None))
    }

    /// Run a browser-level command and return its `result`.
    pub async fn execute(&self, method: &str, params: Value) -> Result<Value, CdpError> {
        self.inner.execute(method, params, None).await
    }

    /// Subscribe to a browser-level event.
    pub async fn event_listener<T: EventKind>(&self) -> Result<EventStream<T>, CdpError> {
        Ok(self.inner.subscribe(None))
    }

    /// Version information about the browser.
    pub async fn version(&self) -> Result<Version, CdpError> {
        let result = self.execute("Browser.getVersion", json!({})).await?;
        let product = result
            .get("product")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(Version { product })
    }

    /// The WebSocket URL this browser is attached to.
    pub fn websocket_address(&self) -> &String {
        &self.inner.ws_url
    }

    /// Create a new page in the default browser context.
    pub async fn new_page(&self, url: &str) -> Result<Page, CdpError> {
        self.new_page_inner(url, None).await
    }

    /// Create a new page in `context`.
    pub async fn new_page_in_context(
        &self,
        url: &str,
        context: &BrowserContextId,
    ) -> Result<Page, CdpError> {
        self.new_page_inner(url, Some(context)).await
    }

    async fn new_page_inner(
        &self,
        url: &str,
        context: Option<&BrowserContextId>,
    ) -> Result<Page, CdpError> {
        let mut params = json!({"url": url});
        if let Some(context) = context {
            params["browserContextId"] = json!(context.0);
        }
        let target_id = self
            .execute("Target.createTarget", params)
            .await?
            .get("targetId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CdpError::ChromeMessage("Target.createTarget missing targetId".to_owned())
            })?
            .to_owned();
        let session_id = self
            .execute(
                "Target.attachToTarget",
                json!({"targetId": target_id, "flatten": true}),
            )
            .await?
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CdpError::ChromeMessage("Target.attachToTarget missing sessionId".to_owned())
            })?
            .to_owned();
        let page = Page {
            inner: Arc::clone(&self.inner),
            target_id: TargetId(target_id),
            session_id: SessionId(session_id),
        };
        page.initialize(self.inner.viewport.as_ref()).await?;
        Ok(page)
    }

    /// Create a new empty browser context.
    pub async fn create_browser_context(&self) -> Result<BrowserContextId, CdpError> {
        let id = self
            .execute("Target.createBrowserContext", json!({}))
            .await?
            .get("browserContextId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CdpError::ChromeMessage(
                    "Target.createBrowserContext missing browserContextId".to_owned(),
                )
            })?
            .to_owned();
        Ok(BrowserContextId(id))
    }

    /// Dispose a browser context.
    pub async fn dispose_browser_context(
        &self,
        context: &BrowserContextId,
    ) -> Result<(), CdpError> {
        self.execute(
            "Target.disposeBrowserContext",
            json!({"browserContextId": context.0}),
        )
        .await?;
        Ok(())
    }

    /// Request the browser to close completely. Call [`Browser::wait`] after
    /// this to reap a spawned child.
    pub async fn close(&self) -> Result<(), CdpError> {
        let result = self.execute("Browser.close", json!({})).await;
        if result.is_ok() {
            self.inner.closing.store(true, Ordering::Release);
        }
        result.map(|_| ())
    }

    /// Wait for a spawned browser to exit. `Ok(None)` when connected, not
    /// launched.
    pub async fn wait(&self) -> std::io::Result<Option<std::process::ExitStatus>> {
        if let Some(child) = self.inner.child.lock().await.as_mut() {
            Ok(Some(child.wait().await?))
        } else {
            Ok(None)
        }
    }

    /// Kill a spawned browser and reap it. `None` when connected, not launched.
    pub async fn kill(&self) -> Option<std::io::Result<()>> {
        match self.inner.child.lock().await.as_mut() {
            Some(child) => Some(kill_and_wait(child).await),
            None => None,
        }
    }
}

impl Page {
    /// The identifier of this page's target.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// The flattened session of this page.
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Run a command in this page's session and return its `result`.
    pub async fn execute(&self, method: &str, params: Value) -> Result<Value, CdpError> {
        self.inner
            .execute(method, params, Some(&self.session_id.0))
            .await
    }

    /// Subscribe to this page's events.
    pub async fn event_listener<T: EventKind>(&self) -> Result<EventStream<T>, CdpError> {
        Ok(self.inner.subscribe(Some(self.target_id.0.clone())))
    }

    /// Navigate to `url`, resolving once the page reports load. A
    /// server-side navigation error fails without waiting for load.
    pub async fn goto(&self, url: impl AsRef<str>) -> Result<(), CdpError> {
        let (sender, receiver) = oneshot::channel();
        self.inner.add_load_waiter(&self.session_id.0, sender);
        let navigated = self
            .execute("Page.navigate", json!({"url": url.as_ref()}))
            .await;
        let failed = match navigated {
            Ok(result) => result
                .get("errorText")
                .and_then(Value::as_str)
                .map(str::to_owned),
            Err(error) => {
                self.inner.drop_load_waiters(&self.session_id.0);
                return Err(error);
            }
        };
        if let Some(error) = failed {
            self.inner.drop_load_waiters(&self.session_id.0);
            return Err(CdpError::ChromeMessage(error));
        }
        match tokio::time::timeout(NAVIGATION_TIMEOUT, receiver).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(CdpError::ChromeMessage(
                "CDP transport closed during navigation".to_owned(),
            )),
            Err(_) => {
                self.inner.drop_load_waiters(&self.session_id.0);
                Err(CdpError::Timeout)
            }
        }
    }

    /// Reload the page.
    pub async fn reload(&self) -> Result<(), CdpError> {
        self.execute("Page.reload", json!({})).await?;
        Ok(())
    }

    /// Close the page, running its `beforeunload` hooks.
    pub async fn close(self) -> Result<(), CdpError> {
        self.execute("Page.close", json!({})).await?;
        Ok(())
    }

    /// Capture a screenshot and write it to `path`.
    pub async fn save_screenshot(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, CdpError> {
        use base64::Engine;
        self.inner
            .execute(
                "Target.activateTarget",
                json!({"targetId": self.target_id.0}),
                None,
            )
            .await?;
        let result = self.execute("Page.captureScreenshot", json!({})).await?;
        let data = result
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| CdpError::ChromeMessage("screenshot missing data".to_owned()))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|error| {
                CdpError::ChromeMessage(format!("invalid screenshot data: {error}"))
            })?;
        std::fs::write(path.as_ref(), &bytes)?;
        Ok(bytes)
    }

    /// Enable the domains the harness drives and attach to future workers
    /// paused, matching the previous client's target initialization.
    async fn initialize(&self, viewport: Option<&Viewport>) -> Result<(), CdpError> {
        self.execute("Page.enable", json!({})).await?;
        self.execute("Runtime.enable", json!({})).await?;
        self.execute("Network.enable", json!({})).await?;
        self.execute(
            "Security.setIgnoreCertificateErrors",
            json!({"ignore": true}),
        )
        .await?;
        self.execute(
            "Target.setAutoAttach",
            json!({"autoAttach": true, "waitForDebuggerOnStart": true, "flatten": true}),
        )
        .await?;
        if let Some(viewport) = viewport {
            let orientation = if viewport.is_landscape {
                json!({"type": "landscapePrimary", "angle": 90})
            } else {
                json!({"type": "portraitPrimary", "angle": 0})
            };
            self.execute(
                "Emulation.setDeviceMetricsOverride",
                json!({
                    "mobile": viewport.emulating_mobile,
                    "width": viewport.width,
                    "height": viewport.height,
                    "deviceScaleFactor": viewport.device_scale_factor.unwrap_or(1.0),
                    "screenOrientation": orientation,
                }),
            )
            .await?;
            self.execute(
                "Emulation.setTouchEmulationEnabled",
                json!({"enabled": true}),
            )
            .await?;
        }
        Ok(())
    }
}

fn connected(
    config: BrowserConfig,
    url: String,
    ws: WebSocketStream<ConnectStream>,
    child: Option<tokio::process::Child>,
) -> (Browser, Handler) {
    let (commands, command_receiver) = mpsc::unbounded_channel();
    let (handler_sender, handler_receiver) = mpsc::unbounded_channel();
    let inner = Arc::new(Inner {
        commands,
        pending: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(0),
        child: AsyncMutex::new(child),
        ws_url: url,
        subscriptions: Mutex::new(Vec::new()),
        load_waiters: Mutex::new(HashMap::new()),
        capture_workers: AtomicBool::new(false),
        closing: AtomicBool::new(false),
        surface_invalid: config.surface_invalid,
        request_timeout: config.request_timeout,
        viewport: config.viewport,
        handler_errors: Mutex::new(Some(handler_sender)),
    });
    // Queue discovery before the task starts: a caller command must never
    // overtake it, or a server that stops reading at `Browser.close` would
    // reset the connection with our discovery still unread.
    inner.send_untracked("Target.setDiscoverTargets", json!({"discover": true}), None);
    let task_inner = Arc::clone(&inner);
    tokio::spawn(async move { run_connection(task_inner, ws, command_receiver).await });
    (
        Browser {
            inner: inner.clone(),
        },
        Handler {
            errors: handler_receiver,
            inner,
        },
    )
}

async fn canonicalize_except_snap(path: std::path::PathBuf) -> std::io::Result<std::path::PathBuf> {
    let resolved = resolve_bare_name_on_path(&path);
    let canonical = tokio::fs::canonicalize(&resolved).await?;
    if canonical
        .to_str()
        .is_some_and(|path| path.ends_with("/snap"))
    {
        Ok(resolved)
    } else {
        Ok(canonical)
    }
}

/// Resolve a bare executable name against `PATH`, leaving every other path
/// untouched. A name found nowhere falls back to itself so `canonicalize`
/// still reports `NotFound`.
fn resolve_bare_name_on_path(path: &std::path::Path) -> std::path::PathBuf {
    let Some(paths) = std::env::var_os("PATH") else {
        return path.to_path_buf();
    };
    let dirs: Vec<std::path::PathBuf> = std::env::split_paths(&paths).collect();
    resolve_bare_name(path, &dirs)
}

/// Resolve a bare executable name (`"google-chrome"`) to the first executable
/// file of that name on `search_dirs`. Paths with directory components
/// (`"./foo"`, `"bin/chrome"`, `"/usr/bin/chromium"`) are returned untouched,
/// as are names found nowhere.
fn resolve_bare_name(
    path: &std::path::Path,
    search_dirs: &[std::path::PathBuf],
) -> std::path::PathBuf {
    if path.components().count() != 1 || path.is_absolute() {
        return path.to_path_buf();
    }
    for dir in search_dirs {
        let candidate = dir.join(path);
        if candidate.is_file() && is_executable_file(&candidate) {
            return candidate;
        }
    }
    path.to_path_buf()
}

/// True when `candidate` is runnable: on Unix it needs an execute bit.
#[cfg(unix)]
fn is_executable_file(candidate: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(candidate)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// True when `candidate` is runnable: without execute bits, being a file is
/// enough.
#[cfg(not(unix))]
fn is_executable_file(_candidate: &std::path::Path) -> bool {
    true
}

async fn kill_and_wait(child: &mut tokio::process::Child) -> std::io::Result<()> {
    child.kill().await?;
    child.wait().await.map(|_| ())
}

/// Resolve the DevTools WebSocket URL from the browser's stderr before
/// `deadline`, or fail when the child exits first.
async fn ws_url_from_output(
    child: &mut tokio::process::Child,
    deadline: Instant,
) -> Result<String, CdpError> {
    use tokio::io::AsyncBufReadExt;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| CdpError::LaunchIo("browser stderr unavailable".to_owned()))?;
    let mut reader = tokio::io::BufReader::new(stderr);
    let mut collected = Vec::<u8>::new();
    let mut line = Vec::<u8>::new();
    loop {
        line.clear();
        tokio::select! {
            _ = tokio::time::sleep_until(deadline.into()) => {
                return Err(CdpError::LaunchTimeout(format!(
                    "launch timed out before the DevTools URL appeared: {}",
                    String::from_utf8_lossy(&collected)
                )));
            }
            status = child.wait() => {
                return Err(match status {
                    Ok(status) => CdpError::LaunchExit(format!(
                        "browser exited with {status} before the DevTools URL appeared: {}",
                        String::from_utf8_lossy(&collected)
                    )),
                    Err(error) => CdpError::LaunchIo(format!(
                        "waiting for the browser child: {error}"
                    )),
                });
            }
            read = reader.read_until(b'\n', &mut line) => {
                match read {
                    Err(error) => {
                        return Err(CdpError::LaunchIo(format!(
                            "reading browser stderr: {error}"
                        )));
                    }
                    Ok(0) => {
                        return Err(CdpError::LaunchIo(format!(
                            "browser stderr ended before the DevTools URL appeared: {}",
                            String::from_utf8_lossy(&collected)
                        )));
                    }
                    Ok(_) => {
                        collected.extend_from_slice(&line);
                        let text = std::str::from_utf8(&line).map_err(|_| {
                            CdpError::LaunchIo(
                                "browser stderr was not valid UTF-8".to_owned(),
                            )
                        })?;
                        if let Some((_, ws)) = text.rsplit_once("listening on ") {
                            let ws = ws.trim();
                            if ws.starts_with("ws") && ws.contains("devtools/browser") {
                                return Ok(ws.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn connect_ws(
    url: &str,
    deadline: Instant,
) -> Result<WebSocketStream<ConnectStream>, CdpError> {
    let config = WebSocketConfig::default()
        .max_message_size(None)
        .max_frame_size(None);
    match tokio::time::timeout_at(
        deadline.into(),
        async_tungstenite::tokio::connect_async_with_config(url, Some(config)),
    )
    .await
    {
        Ok(Ok((ws, _))) => Ok(ws),
        Ok(Err(error)) => Err(CdpError::Ws(error)),
        Err(_) => Err(CdpError::LaunchTimeout(format!(
            "WebSocket handshake with {url} exceeded the launch timeout"
        ))),
    }
}

/// Connection-local routing state: flattened sessions to their targets.
#[derive(Debug, Default)]
struct ConnState {
    sessions: HashMap<String, String>,
    worker_owners: HashMap<String, String>,
    worker_parents: HashMap<String, String>,
}

fn is_network_method(method: &str) -> bool {
    matches!(
        method,
        "Network.requestWillBeSent"
            | "Network.responseReceived"
            | "Network.loadingFinished"
            | "Network.loadingFailed"
    )
}

/// The custom method worker network observations are forwarded as.
pub const NETWORK_EVENT_METHOD: &str = "BoardStudio.NetworkEvent";

/// Required fields of the events the harness parses strictly. Anything else
/// passes through untouched.
fn validate_event(method: &str, params: &Value) -> Result<(), String> {
    match method {
        "Runtime.exceptionThrown" => serde_json::from_value::<EventExceptionThrown>(params.clone())
            .map(|_| ())
            .map_err(|error| format!("Runtime.exceptionThrown invalid: {error}")),
        "Target.targetDestroyed" => serde_json::from_value::<EventTargetDestroyed>(params.clone())
            .map(|_| ())
            .map_err(|error| format!("Target.targetDestroyed invalid: {error}")),
        "Browser.downloadWillBegin" => {
            serde_json::from_value::<EventDownloadWillBegin>(params.clone())
                .map(|_| ())
                .map_err(|error| format!("Browser.downloadWillBegin invalid: {error}"))
        }
        "Browser.downloadProgress" => {
            serde_json::from_value::<EventDownloadProgress>(params.clone())
                .map(|_| ())
                .map_err(|error| format!("Browser.downloadProgress invalid: {error}"))
        }
        "Target.attachedToTarget" => {
            if params.get("sessionId").and_then(Value::as_str).is_none() {
                return Err("Target.attachedToTarget missing sessionId".to_owned());
            }
            if params
                .get("targetInfo")
                .and_then(|info| info.get("targetId"))
                .and_then(Value::as_str)
                .is_none()
            {
                return Err("Target.attachedToTarget missing targetInfo.targetId".to_owned());
            }
            Ok(())
        }
        "Target.detachedFromTarget" => {
            if params.get("sessionId").and_then(Value::as_str).is_some() {
                Ok(())
            } else {
                Err("Target.detachedFromTarget missing sessionId".to_owned())
            }
        }
        _ => Ok(()),
    }
}

async fn run_connection(
    inner: Arc<Inner>,
    ws: WebSocketStream<ConnectStream>,
    mut commands: mpsc::UnboundedReceiver<String>,
) {
    let (mut sink, mut stream) = ws.split();
    let mut state = ConnState::default();
    let mut evict = tokio::time::interval(Duration::from_secs(5));
    evict.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                if let Err(error) = sink.send(WsMessage::Text(command.into())).await {
                    inner.push_handler_error(CdpError::Ws(error));
                    break;
                }
            }
            message = stream.next() => {
                match message {
                    Some(Ok(WsMessage::Text(text))) => on_text(&inner, &mut state, text.as_str()),
                    Some(Ok(WsMessage::Close(_))) | None => {
                        if !inner.closing.load(Ordering::Acquire) {
                            inner.push_handler_error(CdpError::NoResponse);
                        }
                        break;
                    }
                    Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {}
                    Some(Ok(_)) => {
                        inner.push_handler_error(CdpError::ChromeMessage(
                            "unexpected binary websocket message".to_owned(),
                        ));
                        break;
                    }
                    Some(Err(error)) => {
                        inner.push_handler_error(CdpError::Ws(error));
                        break;
                    }
                }
            }
            _ = evict.tick() => evict_timed_out(&inner),
        }
    }
    let pending = std::mem::take(
        &mut *inner
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    for (_, entry) in pending {
        let _ = entry.sender.send(Err(CdpError::ChromeMessage(
            "CDP transport closed before a response arrived".to_owned(),
        )));
    }
    inner
        .subscriptions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    inner
        .load_waiters
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    inner
        .handler_errors
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
}

fn evict_timed_out(inner: &Inner) {
    let now = Instant::now();
    let mut pending = inner
        .pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let expired: Vec<u64> = pending
        .iter()
        .filter(|(_, entry)| now > entry.started + inner.request_timeout)
        .map(|(id, _)| *id)
        .collect();
    for id in expired {
        if let Some(entry) = pending.remove(&id) {
            let _ = entry.sender.send(Err(CdpError::Timeout));
        }
    }
}

fn on_text(inner: &Arc<Inner>, state: &mut ConnState, raw: &str) {
    let value: Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            inner.push_invalid(raw, error.to_string());
            return;
        }
    };
    if let Some(id) = value.get("id").and_then(Value::as_u64) {
        on_response(inner, id, &value, raw);
        return;
    }
    let Some(method) = value.get("method").and_then(Value::as_str) else {
        inner.push_invalid(raw, "message has neither id nor method".to_owned());
        return;
    };
    let session = value.get("sessionId").and_then(Value::as_str);
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    if let Err(cause) = validate_event(method, &params) {
        inner.push_invalid(raw, cause);
        return;
    }
    on_event(inner, state, method, session, &params, raw);
}

fn on_response(inner: &Inner, id: u64, value: &Value, raw: &str) {
    let entry = inner
        .pending
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&id);
    if let Some(entry) = entry {
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            let code = error.get("code").and_then(Value::as_i64).unwrap_or(0);
            let _ = entry.sender.send(Err(CdpError::ChromeMessage(format!(
                "CDP error {code}: {message}"
            ))));
        } else if let Some(result) = value.get("result").filter(|result| result.is_object()) {
            if entry.is_browser_close {
                inner.closing.store(true, Ordering::Release);
            }
            let _ = entry.sender.send(Ok(result.clone()));
        } else {
            let _ = entry.sender.send(Err(CdpError::InvalidMessage(
                raw.to_owned(),
                "CDP response missing result object or error".to_owned(),
            )));
        }
    }
}

fn on_event(
    inner: &Arc<Inner>,
    state: &mut ConnState,
    method: &str,
    session: Option<&str>,
    params: &Value,
    raw: &str,
) {
    match method {
        "Target.attachedToTarget" => on_attached(inner, state, session, params, raw),
        "Target.detachedFromTarget" => {
            if let Some(detached) = params.get("sessionId").and_then(Value::as_str) {
                let target = state.sessions.remove(detached);
                state.worker_owners.remove(detached);
                state.worker_parents.remove(detached);
                let orphans: Vec<String> = state
                    .worker_parents
                    .iter()
                    .filter(|(_, parent)| parent.as_str() == detached)
                    .map(|(child, _)| child.clone())
                    .collect();
                for orphan in orphans {
                    state.worker_owners.remove(&orphan);
                    state.worker_parents.remove(&orphan);
                }
                if let Some(target) = target {
                    inner.remove_target_subscriptions(&target);
                }
                inner.drop_load_waiters(detached);
            }
            inner.publish(None, method, params.clone(), Some(raw));
        }
        "Target.targetDestroyed" => {
            if let Some(destroyed) = params.get("targetId").and_then(Value::as_str) {
                let owned: Vec<String> = state
                    .worker_owners
                    .iter()
                    .filter(|(_, owner)| owner.as_str() == destroyed)
                    .map(|(session, _)| session.clone())
                    .collect();
                for session in owned {
                    state.worker_owners.remove(&session);
                    state.worker_parents.remove(&session);
                }
                inner.remove_target_subscriptions(destroyed);
            }
            inner.publish(None, method, params.clone(), Some(raw));
        }
        "Page.loadEventFired" => {
            if let Some(session) = session {
                inner.wake_load_waiters(session);
                if let Some(target) = state.sessions.get(session) {
                    inner.publish(Some(target), method, params.clone(), Some(raw));
                }
            }
        }
        _ => {
            if inner.capture_workers.load(Ordering::Relaxed) && is_network_method(method) {
                if let Some(session) = session {
                    let owner = state
                        .worker_owners
                        .get(session)
                        .cloned()
                        .or_else(|| state.sessions.get(session).cloned());
                    if let Some(owner) = owner {
                        inner.publish(
                            Some(&owner),
                            NETWORK_EVENT_METHOD,
                            json!({"session": session, "method": method, "params": params}),
                            None,
                        );
                    }
                }
                if session.is_some_and(|session| state.worker_owners.contains_key(session)) {
                    return;
                }
            } else if session.is_some_and(|session| state.worker_owners.contains_key(session)) {
                return;
            }
            match session {
                None => inner.publish(None, method, params.clone(), Some(raw)),
                Some(session) => {
                    if let Some(target) = state.sessions.get(session) {
                        inner.publish(Some(target), method, params.clone(), Some(raw));
                    }
                }
            }
        }
    }
}

fn on_attached(
    inner: &Arc<Inner>,
    state: &mut ConnState,
    session: Option<&str>,
    params: &Value,
    raw: &str,
) {
    let Some(child) = params.get("sessionId").and_then(Value::as_str) else {
        return;
    };
    let Some(target) = params
        .get("targetInfo")
        .and_then(|info| info.get("targetId"))
        .and_then(Value::as_str)
    else {
        return;
    };
    state.sessions.insert(child.to_owned(), target.to_owned());
    let Some(parent) = session else {
        inner.publish(None, "Target.attachedToTarget", params.clone(), Some(raw));
        return;
    };
    let kind = params
        .get("targetInfo")
        .and_then(|info| info.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let waiting = params
        .get("waitingForDebugger")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if kind != "worker" && kind != "service_worker" {
        // `Page::initialize` auto-attaches every child target paused, including
        // out-of-process iframes: resume them without touching Network.
        if waiting {
            inner.spawn_internal(
                "Runtime.runIfWaitingForDebugger",
                json!({}),
                Some(child.to_owned()),
            );
        }
        if let Some(owner) = state.sessions.get(parent) {
            inner.publish(
                Some(owner),
                "Target.attachedToTarget",
                params.clone(),
                Some(raw),
            );
        }
        return;
    }
    if let Some(owner) = state.sessions.get(parent).cloned() {
        state.worker_owners.insert(child.to_owned(), owner.clone());
        state
            .worker_parents
            .insert(child.to_owned(), parent.to_owned());
    }
    let capture = inner.capture_workers.load(Ordering::Relaxed);
    inner.spawn_worker_setup(
        child.to_owned(),
        parent.to_owned(),
        kind == "service_worker",
        capture,
        waiting,
    );
    if capture {
        if let Some(owner) = state.sessions.get(parent) {
            inner.publish(
                Some(owner),
                NETWORK_EVENT_METHOD,
                json!({"session": parent, "method": "Target.attachedToTarget", "params": params}),
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_events_need_their_fields() {
        assert!(validate_event(
            "Runtime.exceptionThrown",
            &json!({"exceptionDetails": {"text": "boom"}})
        )
        .is_ok());
        assert!(validate_event("Runtime.exceptionThrown", &json!({})).is_err());
        assert!(
            validate_event("Runtime.exceptionThrown", &json!({"exceptionDetails": {}})).is_err()
        );
        assert!(validate_event("Target.targetDestroyed", &json!({"targetId": "t"})).is_ok());
        assert!(validate_event("Target.targetDestroyed", &json!({})).is_err());
        assert!(validate_event("Network.responseReceived", &json!({})).is_ok());
    }

    #[test]
    fn typed_events_reject_wrongly_typed_fields() {
        assert!(validate_event(
            "Runtime.exceptionThrown",
            &json!({"exceptionDetails": {"text": "boom", "exception": {"description": 42}}})
        )
        .is_err());
        assert!(validate_event(
            "Runtime.exceptionThrown",
            &json!({"exceptionDetails": {"text": "boom", "exception": {"description": "Error"}}})
        )
        .is_ok());
        assert!(validate_event("Target.targetDestroyed", &json!({"targetId": 42})).is_err());
        assert!(validate_event(
            "Browser.downloadWillBegin",
            &json!({"guid": "g", "suggestedFilename": "f"})
        )
        .is_ok());
        assert!(validate_event("Browser.downloadWillBegin", &json!({"guid": "g"})).is_err());
        assert!(validate_event(
            "Browser.downloadProgress",
            &json!({"guid": "g", "state": "completed"})
        )
        .is_ok());
        assert!(validate_event(
            "Browser.downloadProgress",
            &json!({"guid": "g", "state": "inProgress"})
        )
        .is_ok());
        assert!(validate_event(
            "Browser.downloadProgress",
            &json!({"guid": "g", "state": "nope"})
        )
        .is_err());
    }

    fn test_inner(
        capture: bool,
    ) -> (
        Arc<Inner>,
        mpsc::UnboundedReceiver<String>,
        mpsc::UnboundedReceiver<CdpError>,
    ) {
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let (handler_sender, handler_receiver) = mpsc::unbounded_channel();
        let inner = Arc::new(Inner {
            commands,
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(0),
            child: AsyncMutex::new(None),
            ws_url: "ws://test/".to_owned(),
            subscriptions: Mutex::new(Vec::new()),
            load_waiters: Mutex::new(HashMap::new()),
            capture_workers: AtomicBool::new(capture),
            closing: AtomicBool::new(false),
            surface_invalid: true,
            request_timeout: Duration::from_secs(5),
            viewport: None,
            handler_errors: Mutex::new(Some(handler_sender)),
        });
        (inner, command_receiver, handler_receiver)
    }

    fn attached_params(child: &str, target: &str, kind: &str, waiting: bool) -> Value {
        json!({
            "sessionId": child,
            "targetInfo": {"targetId": target, "type": kind},
            "waitingForDebugger": waiting,
        })
    }

    async fn next_command(commands: &mut mpsc::UnboundedReceiver<String>) -> Value {
        let text = tokio::time::timeout(Duration::from_secs(5), commands.recv())
            .await
            .expect("setup command arrived")
            .expect("command channel open");
        serde_json::from_str(&text).expect("command is JSON")
    }

    fn respond_ok(inner: &Arc<Inner>, id: u64) {
        let raw = format!(r#"{{"id":{id},"result":{{}}}}"#);
        let value: Value = serde_json::from_str(&raw).unwrap();
        on_response(inner, id, &value, &raw);
    }

    #[tokio::test]
    async fn worker_setup_enables_network_before_resuming() {
        let (inner, mut commands, _handler) = test_inner(true);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let params = attached_params("child", "worker-target", "worker", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        let first = next_command(&mut commands).await;
        assert_eq!(first["method"], "Network.enable");
        assert_eq!(first["sessionId"], "child");
        let id = first["id"].as_u64().unwrap();

        // Hold the ack: nothing else may be sent before Network.enable resolves.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            commands.try_recv().is_err(),
            "resume sent before Network.enable resolved"
        );
        respond_ok(&inner, id);

        let second = next_command(&mut commands).await;
        assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
        assert_eq!(second["sessionId"], "child");
        respond_ok(&inner, second["id"].as_u64().unwrap());
    }

    #[tokio::test]
    async fn worker_setup_network_failure_never_resumes() {
        let (inner, mut commands, mut handler) = test_inner(true);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let params = attached_params("child", "worker-target", "worker", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        let first = next_command(&mut commands).await;
        assert_eq!(first["method"], "Network.enable");
        let id = first["id"].as_u64().unwrap();
        let reply = format!(r#"{{"id":{id},"error":{{"code":-32000,"message":"nope"}}}}"#);
        let reply_value: Value = serde_json::from_str(&reply).unwrap();
        on_response(&inner, id, &reply_value, &reply);

        // The setup task must return without resuming.
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(
            commands.try_recv().is_err(),
            "failed Network.enable still resumed"
        );

        let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
            .await
            .expect("handler error arrived")
            .expect("handler channel open");
        let message = error.to_string();
        assert!(
            message.contains("worker setup Network.enable failed"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn service_worker_setup_pipelines_network_before_resume() {
        let (inner, mut commands, mut handler) = test_inner(true);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let params = attached_params("child", "sw-target", "service_worker", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        // Withhold the Network.enable reply until the resume arrives: a
        // sequential setup would deadlock here on a paused service worker.
        let first = next_command(&mut commands).await;
        assert_eq!(first["method"], "Network.enable");
        assert_eq!(first["sessionId"], "child");
        let network_id = first["id"].as_u64().unwrap();

        let second = next_command(&mut commands).await;
        assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
        assert_eq!(second["sessionId"], "child");
        let resume_id = second["id"].as_u64().unwrap();
        assert!(
            network_id < resume_id,
            "Network.enable must precede resume on the wire"
        );

        // Both complete cleanly once the worker unpauses.
        respond_ok(&inner, network_id);
        respond_ok(&inner, resume_id);

        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(
            handler.try_recv().is_err(),
            "pipelined service-worker setup must not report errors"
        );
        assert!(commands.try_recv().is_err(), "unexpected third command");
    }

    #[tokio::test]
    async fn service_worker_setup_network_error_surfaces_on_handler() {
        let (inner, mut commands, mut handler) = test_inner(true);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let params = attached_params("child", "sw-target", "service_worker", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        let first = next_command(&mut commands).await;
        assert_eq!(first["method"], "Network.enable");
        let network_id = first["id"].as_u64().unwrap();

        // The resume is already enqueued to avoid the paused-worker deadlock.
        let second = next_command(&mut commands).await;
        assert_eq!(second["method"], "Runtime.runIfWaitingForDebugger");
        let resume_id = second["id"].as_u64().unwrap();

        let reply = format!(r#"{{"id":{network_id},"error":{{"code":-32000,"message":"nope"}}}}"#);
        let reply_value: Value = serde_json::from_str(&reply).unwrap();
        on_response(&inner, network_id, &reply_value, &reply);
        respond_ok(&inner, resume_id);

        let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
            .await
            .expect("handler error arrived")
            .expect("handler channel open");
        let message = error.to_string();
        assert!(
            message.contains("worker setup Network.enable failed"),
            "{message}"
        );
    }

    #[tokio::test]
    async fn service_worker_setup_sequences_resume_before_detach() {
        let (inner, mut commands, _handler) = test_inner(false);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let params = attached_params("child", "sw-target", "service_worker", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        let first = next_command(&mut commands).await;
        assert_eq!(first["method"], "Runtime.runIfWaitingForDebugger");
        assert_eq!(first["sessionId"], "child");

        // The detach must wait for the resume ack: one task, in order.
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            commands.try_recv().is_err(),
            "detach sent before resume resolved"
        );
        respond_ok(&inner, first["id"].as_u64().unwrap());

        let second = next_command(&mut commands).await;
        assert_eq!(second["method"], "Target.detachFromTarget");
        assert_eq!(second["sessionId"], "parent");
        assert_eq!(second["params"]["sessionId"], "child");
        respond_ok(&inner, second["id"].as_u64().unwrap());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(commands.try_recv().is_err(), "unexpected third command");
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct AttachedProbe {
        session_id: String,
    }

    impl EventKind for AttachedProbe {
        fn method_id() -> &'static str {
            "Target.attachedToTarget"
        }
    }

    #[tokio::test]
    async fn waiting_iframe_resumes_without_network_enable() {
        let (inner, mut commands, _handler) = test_inner(true);
        let mut state = ConnState::default();
        state
            .sessions
            .insert("parent".to_owned(), "owner".to_owned());
        let mut attached = inner.subscribe::<AttachedProbe>(Some("owner".to_owned()));
        let params = attached_params("child", "iframe-target", "iframe", true);
        let raw = params.to_string();
        on_attached(&inner, &mut state, Some("parent"), &params, &raw);

        let command = next_command(&mut commands).await;
        assert_eq!(command["method"], "Runtime.runIfWaitingForDebugger");
        assert_eq!(command["sessionId"], "child");
        respond_ok(&inner, command["id"].as_u64().unwrap());

        // Capture is on, but a non-worker target must never enable Network.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            commands.try_recv().is_err(),
            "iframe setup must not enable Network"
        );

        let event = tokio::time::timeout(Duration::from_secs(5), attached.next())
            .await
            .expect("attached event published")
            .expect("stream open");
        assert_eq!(event.session_id, "child");
    }

    #[tokio::test]
    async fn malformed_exception_event_surfaces_invalid_message() {
        let (inner, _commands, mut handler) = test_inner(false);
        let mut state = ConnState::default();
        let raw = r#"{"method":"Runtime.exceptionThrown","params":{"exceptionDetails":{"text":"boom","exception":{"description":42}}}}"#;
        on_text(&inner, &mut state, raw);
        let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
            .await
            .expect("handler error arrived")
            .expect("handler channel open");
        assert!(
            matches!(&error, CdpError::InvalidMessage(payload, _) if payload == raw),
            "malformed exception must surface InvalidMessage, got {error:?}"
        );
    }

    #[tokio::test]
    async fn event_stream_reports_unparsable_subscription() {
        let (inner, _commands, mut handler) = test_inner(false);
        let mut stream = inner.subscribe::<EventExceptionThrown>(None);
        inner.publish(
            None,
            "Runtime.exceptionThrown",
            json!({"exceptionDetails": 7}),
            Some("bad-frame"),
        );
        inner.publish(
            None,
            "Runtime.exceptionThrown",
            json!({"exceptionDetails": {"text": "boom"}}),
            Some("good-frame"),
        );
        let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("good event arrived")
            .expect("stream open");
        assert_eq!(event.exception_details.text, "boom");
        let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
            .await
            .expect("handler error arrived")
            .expect("handler channel open");
        assert!(
            matches!(&error, CdpError::InvalidMessage(payload, _) if payload == "bad-frame"),
            "unparsable subscription must surface InvalidMessage, got {error:?}"
        );

        // Without the raw frame, the payload falls back to the value itself.
        let mut stream = inner.subscribe::<EventExceptionThrown>(None);
        let bad = json!({"exceptionDetails": 7});
        inner.publish(None, "Runtime.exceptionThrown", bad.clone(), None);
        inner.publish(
            None,
            "Runtime.exceptionThrown",
            json!({"exceptionDetails": {"text": "again"}}),
            None,
        );
        let event = tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("good event arrived")
            .expect("stream open");
        assert_eq!(event.exception_details.text, "again");
        let error = tokio::time::timeout(Duration::from_secs(5), handler.recv())
            .await
            .expect("handler error arrived")
            .expect("handler channel open");
        assert!(
            matches!(&error, CdpError::InvalidMessage(payload, _) if payload == &bad.to_string()),
            "fallback payload must be the value, got {error:?}"
        );
    }

    #[tokio::test]
    async fn browser_close_response_before_close_frame_is_clean() {
        use futures::StreamExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            while let Some(message) = ws.next().await {
                let command: Value =
                    serde_json::from_str(message.unwrap().to_text().unwrap()).unwrap();
                if command["method"] == "Browser.close" {
                    ws.send(format!(r#"{{"id":{},"result":{{}}}}"#, command["id"]).into())
                        .await
                        .unwrap();
                    ws.close(None).await.unwrap();
                    break;
                }
            }
        });
        let (browser, mut handler) = Browser::connect(format!("ws://{address}")).await.unwrap();
        browser.close().await.expect("Browser.close succeeds");
        let mut errors = Vec::new();
        loop {
            match tokio::time::timeout(Duration::from_secs(5), handler.next()).await {
                Ok(Some(Err(error))) => errors.push(error.to_string()),
                Ok(Some(Ok(()))) => {}
                Ok(None) => break,
                Err(_) => panic!("handler did not end after close"),
            }
        }
        assert!(
            errors.is_empty(),
            "clean close must not report handler errors: {errors:?}"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn close_before_browser_close_response_reports_no_response() {
        use futures::StreamExt;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            while let Some(message) = ws.next().await {
                let command: Value =
                    serde_json::from_str(message.unwrap().to_text().unwrap()).unwrap();
                if command["method"] == "Browser.close" {
                    ws.close(None).await.unwrap();
                    break;
                }
            }
        });
        let (browser, mut handler) = Browser::connect(format!("ws://{address}")).await.unwrap();
        let result = browser.close().await;
        assert!(result.is_err(), "unanswered Browser.close must fail");
        let mut saw_no_response = false;
        loop {
            match tokio::time::timeout(Duration::from_secs(5), handler.next()).await {
                Ok(Some(Err(CdpError::NoResponse))) => saw_no_response = true,
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => panic!("handler did not end after close"),
            }
        }
        assert!(
            saw_no_response,
            "close before the Browser.close response must report NoResponse"
        );
        server.await.unwrap();
        drop(browser);
    }

    #[tokio::test]
    async fn response_without_result_fails_request() {
        let (inner, mut commands, _handler) = test_inner(false);
        let pending = tokio::spawn({
            let inner = Arc::clone(&inner);
            async move { inner.execute("Page.navigate", json!({}), None).await }
        });
        let command = next_command(&mut commands).await;
        let id = command["id"].as_u64().unwrap();
        let raw = format!(r#"{{"id":{id}}}"#);
        let value: Value = serde_json::from_str(&raw).unwrap();
        on_response(&inner, id, &value, &raw);
        match pending.await.unwrap() {
            Err(CdpError::InvalidMessage(payload, cause)) => {
                assert_eq!(payload, raw);
                assert_eq!(cause, "CDP response missing result object or error");
            }
            other => panic!("reply without result must fail, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn response_with_null_result_fails_request() {
        let (inner, mut commands, _handler) = test_inner(false);
        let pending = tokio::spawn({
            let inner = Arc::clone(&inner);
            async move { inner.execute("Page.navigate", json!({}), None).await }
        });
        let command = next_command(&mut commands).await;
        let id = command["id"].as_u64().unwrap();
        let raw = format!(r#"{{"id":{id},"result":null}}"#);
        let value: Value = serde_json::from_str(&raw).unwrap();
        on_response(&inner, id, &value, &raw);
        match pending.await.unwrap() {
            Err(CdpError::InvalidMessage(payload, cause)) => {
                assert_eq!(payload, raw);
                assert_eq!(cause, "CDP response missing result object or error");
            }
            other => panic!("reply with null result must fail, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_browser_close_response_does_not_arm_closing() {
        let (inner, mut commands, _handler) = test_inner(false);
        let pending = tokio::spawn({
            let inner = Arc::clone(&inner);
            async move { inner.execute("Browser.close", json!({}), None).await }
        });
        let command = next_command(&mut commands).await;
        let id = command["id"].as_u64().unwrap();
        let raw = format!(r#"{{"id":{id}}}"#);
        let value: Value = serde_json::from_str(&raw).unwrap();
        on_response(&inner, id, &value, &raw);
        assert!(
            pending.await.unwrap().is_err(),
            "malformed Browser.close reply must fail"
        );
        assert!(
            !inner.closing.load(Ordering::Acquire),
            "malformed reply must not arm closing"
        );
    }

    #[cfg(unix)]
    #[test]
    fn bare_name_resolution_skips_non_executable() {
        use std::os::unix::fs::PermissionsExt;

        let root = tempfile::tempdir().unwrap();
        let plain = root.path().join("plain");
        let armed = root.path().join("armed");
        std::fs::create_dir(&plain).unwrap();
        std::fs::create_dir(&armed).unwrap();
        let decoy = plain.join("fake-chrome");
        let real = armed.join("fake-chrome");
        std::fs::write(&decoy, b"nope").unwrap();
        std::fs::write(&real, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&decoy, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();

        // A non-executable first on the search path is skipped.
        assert_eq!(
            resolve_bare_name(
                std::path::Path::new("fake-chrome"),
                &[plain.clone(), armed.clone()]
            ),
            real
        );
        // The first executable wins once the decoy gains its bit.
        std::fs::set_permissions(&decoy, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            resolve_bare_name(std::path::Path::new("fake-chrome"), &[plain.clone(), armed]),
            decoy
        );
        // Unknown names fall back untouched.
        assert_eq!(
            resolve_bare_name(std::path::Path::new("missing-chrome"), &[plain]),
            std::path::PathBuf::from("missing-chrome")
        );
    }

    #[test]
    fn directory_paths_are_never_resolved() {
        let dirs = vec![
            std::path::PathBuf::from("/bin"),
            std::path::PathBuf::from("/usr/bin"),
        ];
        for path in ["./missing", "bin/chrome", "/no/such", "/"] {
            assert_eq!(
                resolve_bare_name(std::path::Path::new(path), &dirs),
                std::path::PathBuf::from(path),
                "{path}"
            );
        }
        // The PATH-backed wrapper agrees without touching the environment.
        for path in ["./missing", "bin/chrome", "/no/such"] {
            assert_eq!(
                resolve_bare_name_on_path(std::path::Path::new(path)),
                std::path::PathBuf::from(path),
                "{path}"
            );
        }
        // Unknown bare names fall back untouched.
        assert_eq!(
            resolve_bare_name_on_path(std::path::Path::new("boardstudio-no-such-binary")),
            std::path::PathBuf::from("boardstudio-no-such-binary")
        );
    }

    #[tokio::test]
    async fn canonicalize_resolves_absolute_paths() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("chrome");
        std::fs::write(&file, b"x").unwrap();
        let canonical = canonicalize_except_snap(file.clone()).await.unwrap();
        assert_eq!(canonical, tokio::fs::canonicalize(&file).await.unwrap());
    }

    #[test]
    fn launch_args_match_the_previous_client() {
        let config = BrowserConfig::builder()
            .chrome_executable("/bin/chrome")
            .user_data_dir("/tmp/profile")
            .no_sandbox()
            .args(["--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
            .args(["blink-settings=primaryPointerType=4"])
            .window_size(1513, 1043)
            .env("XDG_CONFIG_HOME", "/tmp/config")
            .build()
            .unwrap();
        let args = config.chrome_args();
        for expected in [
            "--remote-debugging-port=0",
            "--user-data-dir=/tmp/profile",
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--blink-settings=primaryPointerType=4",
            "--window-size=1513,1043",
            "--headless",
            "--hide-scrollbars",
            "--mute-audio",
            "--disable-dev-shm-usage",
            "--disable-extensions",
        ] {
            assert!(
                args.contains(&expected.to_owned()),
                "{expected} in {args:?}"
            );
        }
    }
}
