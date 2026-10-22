//! The shared icon module the copied rust-ui components import from (`use crate::ui::icons::Check;`).
//!
//! shortcut: placeholder components that render an empty, decorative `svg` taking a class, so the copies
//! compile without the `icons` crate; ticket 004 replaces them with vendored Lucide drawings.

use dioxus::prelude::*;

macro_rules! placeholder_icons {
    ($($name:ident),* $(,)?) => {$(
        #[component]
        pub fn $name(#[props(into, optional)] class: Option<String>) -> Element {
            rsx! {
                svg {
                    "data-slot": "icon",
                    class: class.unwrap_or_default(),
                    "aria-hidden": "true",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                }
            }
        }
    )*};
}

placeholder_icons!(Check, ChevronDown, ChevronsUpDown, Loader, LoaderCircle);
