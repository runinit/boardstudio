// Included only in the checksum-verified, staged Cadrum diagnostic build.
use std::cell::RefCell;

#[derive(Default)]
struct State {
    options: [bool; 4],
    times: [f64; 10],
    calls: [u32; 10],
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

pub fn configure(skip_edges: bool, skip_ids: bool, timed: bool, split: bool) {
    STATE.with(|state| *state.borrow_mut() = State {
        options: [skip_edges, skip_ids, timed, split],
        ..State::default()
    });
}

pub(crate) fn option(index: usize) -> bool {
    STATE.with(|state| state.borrow().options[index])
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = "export function experiment_browser_now() { return performance.now(); }")]
extern "C" {
    fn experiment_browser_now() -> f64;
}

pub fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { experiment_browser_now() }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.
    }
}

pub(crate) fn start() -> f64 {
    if option(2) { now() } else { -1. }
}

pub(crate) fn finish(index: usize, start: f64) {
    if start < 0. { return; }
    let elapsed = now() - start;
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.times[index] += elapsed;
        state.calls[index] += 1;
    });
}

pub fn snapshot() -> Vec<(f64, u32)> {
    STATE.with(|state| {
        let state = state.borrow();
        state.times.iter().copied().zip(state.calls).collect()
    })
}
