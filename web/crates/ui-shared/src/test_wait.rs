//! Bounded waits for browser tests.
//!
//! A test waits for the condition its next assertion needs, instead of sleeping for a guessed
//! time. The condition is polled every [`POLL_MS`] (a timer yields to the browser, so Dioxus can
//! render between polls) and the wait returns as soon as it holds. Each wait is bounded by
//! [`WAIT_BOUND_MS`]: a condition that never holds fails the test with the name of the wait.
//! Nothing else in a test may depend on elapsed time.
//!
//! Test-support only: the module exists for `cfg(test)` and the `test-support` feature, on wasm.
use gloo_timers::future::TimeoutFuture;
use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::Poll,
};

/// Longest one wait may run before its test fails.
pub const WAIT_BOUND_MS: f64 = 5_000.0;

/// Pause between two polls of a condition that does not hold yet.
pub const POLL_MS: u32 = 5;

fn timed_out(what: &str, bound_ms: f64) -> String {
    format!("wait_until: {what} did not hold within {bound_ms} ms")
}

/// Awaits `future`, or fails with `what` once `bound_ms` has elapsed. Use it for a gate that
/// production code must open (for example a held Core request that must reach the executor):
/// a gate that never opens fails the test at the bound instead of hanging until the runner's.
pub async fn bounded<T>(
    what: &str,
    bound_ms: f64,
    future: impl Future<Output = T>,
) -> Result<T, String> {
    let mut future = pin!(future);
    let mut timer = TimeoutFuture::new(bound_ms as u32);
    poll_fn(|context| {
        if let Poll::Ready(value) = future.as_mut().poll(context) {
            return Poll::Ready(Ok(value));
        }
        if Pin::new(&mut timer).poll(context).is_ready() {
            return Poll::Ready(Err(timed_out(what, bound_ms)));
        }
        Poll::Pending
    })
    .await
}

/// Polls `step` until it returns true, or fails with `what` once `bound_ms` has elapsed.
///
/// Each poll awaits `step`, so a step can do work before it checks (for example, draining the
/// effects a test has held). The check is made before the first pause, so a step that already
/// holds returns without yielding. The error names the wait and the bound. A step that never
/// completes (it awaits something that never happens) also fails at the bound.
pub async fn wait_for_step(
    what: &str,
    bound_ms: f64,
    mut step: impl AsyncFnMut() -> bool,
) -> Result<(), String> {
    let started = js_sys::Date::now();
    let polling = async {
        loop {
            if step().await {
                return Ok(());
            }
            if js_sys::Date::now() - started >= bound_ms {
                return Err(timed_out(what, bound_ms));
            }
            TimeoutFuture::new(POLL_MS).await;
        }
    };
    bounded(what, bound_ms, polling).await?
}

/// Polls `condition` until it holds, or fails with `what` once `bound_ms` has elapsed.
pub async fn wait_for(
    what: &str,
    bound_ms: f64,
    mut condition: impl FnMut() -> bool,
) -> Result<(), String> {
    wait_for_step(what, bound_ms, async || condition()).await
}

/// Waits for `condition` with the shared bound, panicking with the wait's name on timeout.
pub async fn wait_until(what: &str, condition: impl FnMut() -> bool) {
    if let Err(message) = wait_for(what, WAIT_BOUND_MS, condition).await {
        panic!("{message}");
    }
}

/// Waits for `step` (which may do work before it checks) with the shared bound, panicking with
/// the wait's name on timeout.
pub async fn wait_until_step(what: &str, step: impl AsyncFnMut() -> bool) {
    if let Err(message) = wait_for_step(what, WAIT_BOUND_MS, step).await {
        panic!("{message}");
    }
}
