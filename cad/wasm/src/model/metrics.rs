// Diagnostics never change CAD results and are inactive outside a profiled worker request.
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = "
export function cad_metrics_now() {
    return globalThis.__boardstudioCadMetrics ? performance.now() : -1;
}
export function cad_metrics_finish(name, start, bytes) {
    const metrics = globalThis.__boardstudioCadMetrics;
    if (!metrics || start < 0) return;
    const stage = metrics.stages[name] ??= { durationMs: 0, calls: 0 };
    stage.durationMs += performance.now() - start;
    stage.calls++;
    metrics.wasmAllocatedBytes.start ??= bytes;
    metrics.wasmAllocatedBytes.end = bytes;
    metrics.wasmAllocatedBytes.peak = Math.max(metrics.wasmAllocatedBytes.peak ?? 0, bytes);
}
export function cad_metrics_count(name, value) {
    const metrics = globalThis.__boardstudioCadMetrics;
    if (metrics) metrics.counters[name] = (metrics.counters[name] ?? 0) + value;
}
")]
extern "C" {
    fn cad_metrics_now() -> f64;
    fn cad_metrics_finish(name: &str, start: f64, bytes: f64);
    fn cad_metrics_count(name: &str, value: f64);
}

pub(super) struct Stage {
    #[cfg(target_arch = "wasm32")]
    name: &'static str,
    #[cfg(target_arch = "wasm32")]
    start: f64,
}

impl Stage {
    pub(super) fn new(_name: &'static str) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            name: _name,
            #[cfg(target_arch = "wasm32")]
            start: cad_metrics_now(),
        }
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if self.start >= 0.0 {
            let memory = wasm_bindgen::memory().unchecked_into::<js_sys::WebAssembly::Memory>();
            let bytes = memory
                .buffer()
                .unchecked_into::<js_sys::ArrayBuffer>()
                .byte_length();
            cad_metrics_finish(self.name, self.start, f64::from(bytes));
        }
    }
}

pub(super) fn count(_name: &'static str, _value: usize) {
    #[cfg(target_arch = "wasm32")]
    cad_metrics_count(_name, _value as f64);
}
