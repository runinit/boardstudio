//! Exercise the production Matrix inspector's actual keyed fields and HTML event handlers.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
#[path = "matrix_field_lifecycle/harness.rs"]
mod presentation;
