//! Mount the production Keymap panel and accepted projection with real HTML handlers.
#![cfg(all(feature = "page", not(target_arch = "wasm32")))]
#[path = "keymap_panel_lifecycle/harness.rs"]
mod presentation;
