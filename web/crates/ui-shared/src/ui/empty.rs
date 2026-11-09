//! Copied from rust-ui: `app_crates/registry/src/ui/empty.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - built-in borders removed, cards style (theme contract section 8); focus rings kept (the dashed outline)
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Empty(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex flex-col items-center justify-center gap-4 rounded-lg p-8 text-center",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "empty", class: "{merged}", {children} } }
}

#[component]
pub fn EmptyHeader(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex flex-col items-center gap-2",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "empty-header", class: "{merged}", {children} } }
}

#[component]
pub fn EmptyTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "text-lg font-semibold leading-none",
        class.as_deref().unwrap_or("")
    );
    rsx! { h3 { "data-slot": "empty-title", class: "{merged}", {children} } }
}

#[component]
pub fn EmptyDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! { p { "data-slot": "empty-description", class: "{merged}", {children} } }
}

#[component]
pub fn EmptyContent(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex items-center justify-center gap-2",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "empty-content", class: "{merged}", {children} } }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub enum EmptyMediaVariant {
    #[default]
    Default,
    Icon,
}

#[component]
pub fn EmptyMedia(
    #[props(into, optional)] class: Option<String>,
    #[props(default = EmptyMediaVariant::default())] variant: EmptyMediaVariant,
    children: Element,
) -> Element {
    let variant_class = match variant {
        EmptyMediaVariant::Default => "bg-transparent",
        EmptyMediaVariant::Icon => {
            "bg-muted text-foreground flex size-10 shrink-0 items-center justify-center rounded-lg [&_svg:not([class*='size-'])]:size-6"
        }
    };
    let merged = tw_merge!(
        "flex shrink-0 items-center justify-center mb-2 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        variant_class,
        class.as_deref().unwrap_or("")
    );
    rsx! { div { "data-slot": "empty-media", class: "{merged}", {children} } }
}
