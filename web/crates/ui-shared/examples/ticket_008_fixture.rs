//! Browser entry point for the ticket 008 fixture, without the WASM test harness's main.
#[cfg(target_arch = "wasm32")]
pub use boardstudio_web_ui_shared::ui;
#[cfg(target_arch = "wasm32")]
#[path = "../src/test_role.rs"]
mod test_role;
#[cfg(target_arch = "wasm32")]
#[path = "../src/test_wait.rs"]
mod test_wait;
#[cfg(target_arch = "wasm32")]
#[path = "../src/toolbar_toggle_slider_toast_tests.rs"]
mod fixture;

fn main() {}
