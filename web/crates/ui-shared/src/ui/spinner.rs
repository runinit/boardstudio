//! Copied from rust-ui: `app_crates/registry/src/ui/spinner.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - icons come from `crate::ui::icons` (ticket 004) instead of the `icons` crate
//! - every root element carries a `data-slot` (theme contract section 9)

use crate::ui::icons::{Loader, LoaderCircle};
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Spinner(#[props(into, optional)] class: Option<String>) -> Element {
    let merged_class = tw_merge!("size-4 animate-spin", class.as_deref().unwrap_or(""));

    rsx! {
        LoaderCircle { class: "{merged_class}" }
    }
}

#[component]
pub fn SpinnerCircle(#[props(into, optional)] class: Option<String>) -> Element {
    let merged_class = tw_merge!("size-4 animate-spin", class.as_deref().unwrap_or(""));

    rsx! {
        Loader { class: "{merged_class}" }
    }
}
