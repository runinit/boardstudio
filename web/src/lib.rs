#![forbid(unsafe_code)]
//! Worker entry points packaged by wasm-pack (`core-worker`, `cad-worker`, `service-worker`).
//! The page is the `boardstudio-web` bin. Worker services live in `boardstudio-web-worker-host`.

#[cfg(all(target_arch = "wasm32", feature = "core-worker"))]
mod core_worker;
#[cfg(all(target_arch = "wasm32", feature = "service-worker"))]
mod service_worker;

// The CAD worker's `start_cad_worker` export is defined in the worker-host crate.
#[cfg(all(target_arch = "wasm32", feature = "cad-worker"))]
pub use boardstudio_web_worker_host::cad_worker::start_cad_worker;
