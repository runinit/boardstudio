//! Copied from rust-ui: `app_crates/registry/src/ui/alert.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - built-in borders removed, cards style (theme contract section 8); focus rings kept; the destructive variant is tinted (`bg-destructive/10`)
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Eq)]
pub enum AlertVariant {
    #[default]
    Default,
    Destructive,
}

impl AlertVariant {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Default => "bg-background text-foreground",
            Self::Destructive => "bg-destructive/10 text-destructive [&>svg]:text-destructive",
        }
    }
}

#[component]
pub fn Alert(
    #[props(default)] variant: AlertVariant,
    #[props(into, default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "relative w-full rounded-lg px-4 py-3 text-sm [&>svg+div]:translate-y-[-3px] [&>svg]:absolute [&>svg]:left-4 [&>svg]:top-4 [&>svg]:text-foreground [&>svg~*]:pl-7",
        variant.as_str(),
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div { "data-slot": "alert", role: "alert", class: "{class}", {children} }
    }
}

#[component]
pub fn AlertTitle(#[props(into, default)] class: Option<String>, children: Element) -> Element {
    let class = tw_merge!(
        "mb-1 font-medium tracking-tight leading-none",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        h4 { "data-slot": "alert-title", class: "{class}", {children} }
    }
}

#[component]
pub fn AlertDescription(
    #[props(into, default)] class: Option<String>,
    children: Element,
) -> Element {
    let class = tw_merge!(
        "text-sm [&_p]:leading-relaxed",
        class.as_deref().unwrap_or("")
    );
    rsx! {
        p { "data-slot": "alert-description", class: "{class}", {children} }
    }
}
