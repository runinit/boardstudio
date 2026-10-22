//! Copied from rust-ui: `app_crates/registry/src/ui/separator.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Eq)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[component]
pub fn Separator(
    #[props(into, optional)] class: Option<String>,
    #[props(default = SeparatorOrientation::default())] orientation: SeparatorOrientation,
) -> Element {
    let orientation_class = match orientation {
        SeparatorOrientation::Horizontal => "w-full h-[1px]",
        SeparatorOrientation::Vertical => "h-full w-[1px]",
    };
    let merged_class = tw_merge!(
        "shrink-0 bg-border",
        orientation_class,
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-slot": "separator", class: "{merged_class}", role: "separator" }
    }
}
