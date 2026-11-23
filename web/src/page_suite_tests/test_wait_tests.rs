use boardstudio_web_ui_shared::test_wait::{
    WAIT_BOUND_MS, bounded, wait_for, wait_for_step, wait_until, wait_until_step,
};
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
