//! Copied from rust-ui: `app_crates/registry/src/ui/skeleton.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Skeleton(#[props(into, optional)] class: Option<String>) -> Element {
    let merged_class = tw_merge!(
        "animate-pulse rounded-md bg-muted",
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-slot": "skeleton", class: "{merged_class}" }
    }
}
