//! Look copied from rust-ui: `app_crates/registry/src/ui/checkbox.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::checkbox` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - built-in border removed and the box filled with `bg-field`, cards style (theme contract section 8); focus ring kept;
//! - icons come from `crate::ui::icons` (ticket 004).

use dioxus::prelude::*;
pub use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::checkbox::{self, CheckboxProps};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

use crate::ui::icons::Check;

#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let base = attributes!(button {
        class: "peer bg-field data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 inline-flex items-center justify-center size-4 shrink-0 rounded-[4px] transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
        "data-slot": "checkbox",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        checkbox::Checkbox {
            checked: props.checked,
            default_checked: props.default_checked,
            required: props.required,
            disabled: props.disabled,
            name: props.name,
            value: props.value,
            on_checked_change: props.on_checked_change,
            attributes: merged,
            checkbox::CheckboxIndicator { class: "flex justify-center items-center text-current transition-none", "data-slot": "checkbox-indicator",
                Check { class: "size-3.5" }
            }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod contract_tests {
    use super::*;
    use crate::test_mount::MountedFixture;
    use crate::{test_role::by_role_name, test_wait::wait_until, ui::button::Button};

    fn fixture() -> Element {
        let mut state = use_signal(|| CheckboxState::Unchecked);
        let mut events = use_signal(Vec::<String>::new);
        rsx! {
            Checkbox { aria_label: "Controlled checkbox", checked: Some(state()),
                on_checked_change: move |next| { events.write().push(format!("controlled:{next:?}")); state.set(next); }
            }
            Checkbox { aria_label: "Owner keeps checked", checked: Some(CheckboxState::Checked),
                on_checked_change: move |next| events.write().push(format!("locked:{next:?}"))
            }
            Checkbox { aria_label: "Disabled mixed checkbox", checked: Some(CheckboxState::Indeterminate), disabled: true,
                on_checked_change: move |_| events.write().push("disabled".into())
            }
            Checkbox { aria_label: "Default checkbox" }
            Button { onclick: move |_| state.set(CheckboxState::Indeterminate), "Set mixed" }
            Button { onclick: move |_| state.set(CheckboxState::Checked), "Set checked" }
            output { role: "status", aria_label: "Checkbox requests", "{events:?}" }
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn public_checkbox_state_preserves_controlled_mixed_disabled_and_default_cases() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
        wait_until("Checkbox contracts mount", || {
            by_role_name(&root, "checkbox", "Controlled checkbox").is_some()
        })
        .await;
        let controlled = by_role_name(&root, "checkbox", "Controlled checkbox").unwrap();
        assert_eq!(
            controlled.get_attribute("aria-checked").as_deref(),
            Some("false")
        );
        controlled.click();
        wait_until("owner accepts checked", || {
            controlled.get_attribute("aria-checked").as_deref() == Some("true")
        })
        .await;
        by_role_name(&root, "button", "Set mixed").unwrap().click();
        wait_until("external owner sets mixed", || {
            controlled.get_attribute("aria-checked").as_deref() == Some("mixed")
        })
        .await;
        controlled.click();
        wait_until("mixed toggles to unchecked", || {
            controlled.get_attribute("aria-checked").as_deref() == Some("false")
        })
        .await;
        by_role_name(&root, "button", "Set checked")
            .unwrap()
            .click();
        wait_until("external owner sets checked", || {
            controlled.get_attribute("aria-checked").as_deref() == Some("true")
        })
        .await;
        controlled.click();
        let locked = by_role_name(&root, "checkbox", "Owner keeps checked").unwrap();
        locked.click();
        let disabled = by_role_name(&root, "checkbox", "Disabled mixed checkbox").unwrap();
        disabled.click();
        let default = by_role_name(&root, "checkbox", "Default checkbox").unwrap();
        assert_eq!(
            default.get_attribute("aria-checked").as_deref(),
            Some("false")
        );
        default.click();
        wait_until("checkbox requests settle", || {
            by_role_name(&root, "status", "Checkbox requests")
                .unwrap()
                .text_content()
                .unwrap()
                .contains("locked:Unchecked")
        })
        .await;
        assert_eq!(
            controlled.get_attribute("aria-checked").as_deref(),
            Some("false")
        );
        assert_eq!(
            locked.get_attribute("aria-checked").as_deref(),
            Some("true")
        );
        assert_eq!(
            disabled.get_attribute("aria-checked").as_deref(),
            Some("mixed")
        );
        assert!(disabled.has_attribute("disabled"));
        assert_eq!(
            default.get_attribute("aria-checked").as_deref(),
            Some("true")
        );
        let requests = by_role_name(&root, "status", "Checkbox requests")
            .unwrap()
            .text_content()
            .unwrap();
        assert_eq!(
            requests.matches("controlled:Checked").count(),
            1,
            "{requests}"
        );
        assert_eq!(
            requests.matches("controlled:Unchecked").count(),
            2,
            "{requests}"
        );
        assert_eq!(
            requests.matches("locked:Unchecked").count(),
            1,
            "{requests}"
        );
        assert!(!requests.contains("disabled"));
        root.remove();
    }
}
