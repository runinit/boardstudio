//! Copied from rust-ui: `app_crates/registry/src/ui/label.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Edits to the upstream file:
//! - every root element carries a `data-slot` (theme contract section 9)
//! - absent `html_for` omits `for` so a nested control retains native implicit association

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
            r#for: html_for.as_deref(),
            class: "{class}",
            {children}
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod association_tests {
    use super::*;
    use crate::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen::JsCast;

    fn fixture() -> Element {
        rsx! {
            Label { "Implicit target" input { aria_label: "Implicit target" } }
            Label { html_for: "explicit-label-target", "Explicit target" }
            input { id: "explicit-label-target", aria_label: "Explicit target" }
            Label { html_for: "", "Empty target" input { aria_label: "Empty target" } }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn label_omits_none_and_preserves_explicit_id_and_empty_targets() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("Label association cases mount", || {
            by_role_name(&root, "textbox", "Empty target").is_some()
        })
        .await;
        let implicit = by_role_name(&root, "textbox", "Implicit target").unwrap();
        let implicit_label = implicit.closest("label").unwrap().unwrap();
        assert_eq!(implicit_label.get_attribute("for"), None);
        assert_eq!(
            js_sys::Reflect::get(implicit_label.as_ref(), &"control".into()).unwrap(),
            wasm_bindgen::JsValue::from(implicit.clone()),
        );
        implicit_label
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        assert_eq!(document.active_element(), Some(implicit.unchecked_into()));

        let explicit = by_role_name(&root, "textbox", "Explicit target").unwrap();
        let explicit_label = root
            .query_selector("label[for='explicit-label-target']")
            .unwrap()
            .unwrap();
        assert_eq!(
            explicit_label.get_attribute("for").as_deref(),
            Some("explicit-label-target")
        );
        assert_eq!(
            js_sys::Reflect::get(explicit_label.as_ref(), &"control".into()).unwrap(),
            wasm_bindgen::JsValue::from(explicit.clone()),
        );
        explicit_label
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        assert_eq!(document.active_element(), Some(explicit.unchecked_into()));

        let empty = by_role_name(&root, "textbox", "Empty target").unwrap();
        let empty_label = empty.closest("label").unwrap().unwrap();
        assert_eq!(empty_label.get_attribute("for").as_deref(), Some(""));
        assert!(
            js_sys::Reflect::get(empty_label.as_ref(), &"control".into())
                .unwrap()
                .is_null()
        );
        root.remove();
    }
}
