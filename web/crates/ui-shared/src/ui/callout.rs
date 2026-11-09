//! Copied from rust-ui: `app_crates/registry/src/ui/callout.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - built-in borders removed, cards style (theme contract section 8); focus rings kept, including the `dark:` ones
//! - every root element carries a `data-slot` (theme contract section 9)

use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CalloutVariant {
    #[default]
    Default,
    Info,
    Warning,
}

#[component]
pub fn Callout(
    #[props(into, optional)] class: Option<String>,
    #[props(into, optional)] title: Option<String>,
    #[props(default = CalloutVariant::default())] variant: CalloutVariant,
    children: Element,
) -> Element {
    let variant_class = match variant {
        CalloutVariant::Default => "bg-surface text-surface-foreground",
        CalloutVariant::Info => "bg-info-light text-foreground dark:bg-info-dark/20",
        CalloutVariant::Warning => "bg-warning-light text-foreground dark:bg-warning-dark/20",
    };

    let merged = tw_merge!(
        "relative w-full rounded-xl px-4 py-3 text-sm md:-mx-1 [&_code]:bg-black/5 [&_code]:rounded [&_code]:px-1 [&_code]:py-0.5 dark:[&_code]:bg-white/10",
        variant_class,
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-slot": "callout", "data-name": "Callout", class: "{merged}",
            if let Some(t) = title {
                p { class: "mb-1 font-medium leading-none", "{t}" }
            }
            p { class: "text-sm leading-relaxed text-muted-foreground", {children} }
        }
    }
}
