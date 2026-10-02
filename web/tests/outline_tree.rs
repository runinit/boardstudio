// Exercise the actual private tree projection on native Rust so its public
// row/context behavior is testable without compiling the browser-only Dioxus
// root in this integration-test target.
#[path = "support/tree_presentation.rs"]
mod presentation;
