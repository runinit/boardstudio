//! Vendored from Lucide's `icons/*.svg` at commit
//! `20aef31db93fe4d9f398dbb15b120d99c48eef95` (ISC; see `web/assets/licenses/lucide-LICENSE.txt`).
//! Check, ChevronDown, and Loader are derived from Feather and retain its MIT notice there.

use crate::ui::class_merge::merge_classes;
use dioxus::prelude::*;

macro_rules! icon {
    ($name:ident, $($path:literal),+ $(,)?) => {
        #[component]
        pub fn $name(#[props(into, optional)] class: Option<String>) -> Element {
            let class = merge_classes!("size-4", class.as_deref().unwrap_or(""));
            rsx! {
                svg {
                    "data-slot": "icon",
                    class,
                    "aria-hidden": "true",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "var(--icon-stroke, 2)",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    $(path { d: $path })+
                }
            }
        }
    };
}

icon!(Check, "M20 6 9 17l-5-5");
icon!(ChevronDown, "m6 9 6 6 6-6");
icon!(ChevronsUpDown, "m7 15 5 5 5-5", "m7 9 5-5 5 5");
icon!(
    Loader,
    "M12 2v4",
    "m16.2 7.8 2.9-2.9",
    "M18 12h4",
    "m16.2 16.2 2.9 2.9",
    "M12 18v4",
    "m4.9 19.1 2.9-2.9",
    "M2 12h4",
    "m4.9 4.9 2.9 2.9",
);
icon!(LoaderCircle, "M21 12a9 9 0 1 1-6.219-8.56");
