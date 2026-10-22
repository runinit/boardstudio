//! Vendored from Lucide's `icons/*.svg` at commit
//! `20aef31db93fe4d9f398dbb15b120d99c48eef95` (ISC; see `web/assets/licenses/lucide-LICENSE.txt`).
//! Check, ChevronDown, and Loader are derived from Feather and retain its MIT notice there.

use dioxus::prelude::*;
use tw_merge::tw_merge;

macro_rules! icon {
    ($name:ident, $($path:literal),+ $(,)?) => {
        #[component]
        pub fn $name(#[props(into, optional)] class: Option<String>) -> Element {
            let class = tw_merge!("size-4", class.as_deref().unwrap_or(""));
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

#[cfg(all(test, target_arch = "wasm32"))]
mod mount_tests {
    use dioxus::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    use crate::test_role::{accessible_name, by_role_name};
    use crate::test_wait::wait_until;
    use crate::ui::button::Button;
    use crate::ui::icons::{Check, ChevronDown};

    fn fixture() -> Element {
        rsx! {
            Button { Check { class: "size-3.5" } "Save" }
            ChevronDown {}
        }
    }

    #[wasm_bindgen_test]
    async fn vendored_icons_are_decorative_and_use_class_size_and_theme_stroke() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the icon button renders", || {
            by_role_name(&root, "button", "Save").is_some()
        })
        .await;

        let save = by_role_name(&root, "button", "Save").unwrap();
        assert_eq!(accessible_name(&save), "Save");
        let check = save.query_selector("svg").unwrap().unwrap();
        assert_eq!(check.get_attribute("aria-hidden").as_deref(), Some("true"));
        assert_eq!(check.get_attribute("class").as_deref(), Some("size-3.5"));
        assert_eq!(
            check.get_attribute("stroke-width").as_deref(),
            Some("var(--icon-stroke, 2)")
        );
        let default_icon: web_sys::Element = root
            .query_selector_all("svg")
            .unwrap()
            .item(1)
            .unwrap()
            .dyn_into()
            .unwrap();
        assert!(
            default_icon
                .get_attribute("class")
                .unwrap_or_default()
                .contains("size-4")
        );
        root.remove();
    }
}
