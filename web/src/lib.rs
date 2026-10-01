#![forbid(unsafe_code)]

#[cfg(all(target_arch = "wasm32", feature = "core-worker"))]
mod core_worker;

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod host;

#[cfg(any(feature = "page", feature = "cad-worker"))]
pub mod cad_jobs;

#[cfg(all(target_arch = "wasm32", any(feature = "page", feature = "cad-worker")))]
pub mod cad_worker;

pub const CORE_WORKER_FRAME_VERSION: u8 = 1;

#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod core_protocol;
#[cfg(any(all(target_arch = "wasm32", feature = "page"), test))]
mod persistence_contract;

#[cfg(test)]
mod tests {
    use super::CORE_WORKER_FRAME_VERSION;

    #[test]
    fn worker_frame_version_is_stable() {
        assert_eq!(CORE_WORKER_FRAME_VERSION, 1);
    }
}

#[cfg(all(target_arch = "wasm32", feature = "page"))]
pub mod renderer_host;
