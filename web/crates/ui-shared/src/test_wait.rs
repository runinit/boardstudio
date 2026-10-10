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

/// Longest one wait may run before its test fails.
pub const WAIT_BOUND_MS: f64 = 5_000.0;

/// Pause between two polls of a condition that does not hold yet.
pub const POLL_MS: u32 = 5;

/// Polls `condition` until it holds, or fails with `what` once `bound_ms` has elapsed.
///
/// The condition is checked before the first pause, so a condition that already holds returns
/// without yielding. The error names the wait and the bound, for the test failure message.
pub async fn wait_for(
    what: &str,
    bound_ms: f64,
    mut condition: impl FnMut() -> bool,
) -> Result<(), String> {
    let started = js_sys::Date::now();
    loop {
        if condition() {
            return Ok(());
        }
        if js_sys::Date::now() - started >= bound_ms {
            return Err(format!(
                "wait_until: {what} did not hold within {bound_ms} ms"
            ));
        }
        TimeoutFuture::new(POLL_MS).await;
    }
}

/// Waits for `condition` with the shared bound, panicking with the wait's name on timeout.
pub async fn wait_until(what: &str, condition: impl FnMut() -> bool) {
    if let Err(message) = wait_for(what, WAIT_BOUND_MS, condition).await {
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
