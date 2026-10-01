#![forbid(unsafe_code)]

#[cfg(all(target_arch = "wasm32", feature = "core-worker"))]
mod core_worker;
#[cfg(all(target_arch = "wasm32", feature = "service-worker"))]
mod service_worker;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

pub mod offline;

#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod core_protocol;
#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod persistence_contract;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;
