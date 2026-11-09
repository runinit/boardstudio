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

/// [`bounded`] with the shared bound, panicking with the wait's name on timeout.
pub async fn wait_until_future<T>(what: &str, future: impl Future<Output = T>) -> T {
    match bounded(what, WAIT_BOUND_MS, future).await {
        Ok(value) => value,
        Err(message) => panic!("{message}"),
    }
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

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;
    use std::cell::Cell;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn returns_as_soon_as_the_condition_holds() {
        let polls = Cell::new(0);
        let result = wait_for("three polls", WAIT_BOUND_MS, || {
            polls.set(polls.get() + 1);
            polls.get() == 3
        })
        .await;
        assert_eq!(result, Ok(()));
        assert_eq!(polls.get(), 3, "the condition is not polled after it holds");
    }

    #[wasm_bindgen_test]
    async fn a_condition_that_already_holds_does_not_pause() {
        let polls = Cell::new(0);
        wait_until("already true", || {
            polls.set(polls.get() + 1);
            true
        })
        .await;
        assert_eq!(polls.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn a_step_runs_on_every_poll_until_it_reports_true() {
        let steps = Cell::new(0);
        wait_until_step("the fourth step", async || {
            steps.set(steps.get() + 1);
            steps.get() == 4
        })
        .await;
        assert_eq!(steps.get(), 4, "each poll runs the step once");
    }

    #[wasm_bindgen_test]
    async fn a_future_that_completes_returns_its_value() {
        let value = bounded("a ready future", 40.0, async { 7 }).await;
        assert_eq!(value, Ok(7));
    }

    #[wasm_bindgen_test]
    async fn a_future_that_never_completes_times_out_naming_the_wait() {
        let result = bounded("the never-open gate", 40.0, std::future::pending::<()>()).await;
        assert_eq!(
            result,
            Err("wait_until: the never-open gate did not hold within 40 ms".to_owned())
        );
    }

    #[wasm_bindgen_test]
    async fn a_step_that_never_completes_times_out_at_the_bound() {
        let result = wait_for_step("a hung step", 40.0, async || {
            std::future::pending::<()>().await;
            true
        })
        .await;
        assert_eq!(
            result,
            Err("wait_until: a hung step did not hold within 40 ms".to_owned())
        );
    }

    #[wasm_bindgen_test]
    async fn times_out_naming_the_wait_and_the_bound() {
        let message = wait_for("the never-true flag", 40.0, || false)
            .await
            .expect_err("a condition that never holds must time out");
        assert_eq!(
            message,
            "wait_until: the never-true flag did not hold within 40 ms"
        );
    }
}
