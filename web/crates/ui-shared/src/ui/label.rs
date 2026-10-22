//! Copied from rust-ui: `app_crates/registry/src/ui/label.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Label(
    #[props(into, default)] html_for: Option<String>,
    #[props(into, default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        label { "data-slot": "label",
            r#for: html_for.as_deref().unwrap_or(""),
            class: "{class}",
            {children}
        }
    }
}
