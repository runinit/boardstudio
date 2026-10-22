//! Mounts a few primitives-backed copies to check the wiring: the primitive supplies the role and state, the copy
//! adds its rust-ui classes and `data-slot`. The accessibility floor rows are tickets 006 to 008.

use dioxus::prelude::*;
use wasm_bindgen_test::*;

use crate::test_role::by_role_name;
use crate::test_wait::wait_until;
use crate::ui::checkbox::Checkbox;
use crate::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use crate::ui::toolbar::{Toolbar, ToolbarButton};

fn fixture() -> Element {
    rsx! {
        Checkbox { aria_label: "Snap to grid" }
        Toolbar { aria_label: "History",
            ToolbarButton { index: 0usize, "Undo" }
        }
        Tabs { default_value: "objects", horizontal: true,
            TabsList {
                TabsTrigger { value: "objects", index: 0usize, "Objects" }
                TabsTrigger { value: "inspector", index: 1usize, "Inspector" }
            }
            TabsContent { value: "objects", index: 0usize, "Objects panel" }
            TabsContent { value: "inspector", index: 1usize, "Inspector panel" }
        }
    }
}

#[wasm_bindgen_test]
async fn primitives_backed_copies_carry_role_slot_and_classes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    dioxus_web::launch::launch_virtual_dom(
        VirtualDom::new(fixture),
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the copies render", || {
        by_role_name(&root, "checkbox", "Snap to grid").is_some()
    })
    .await;

    let checkbox = by_role_name(&root, "checkbox", "Snap to grid").unwrap();
    assert_eq!(
        checkbox.get_attribute("data-slot").as_deref(),
        Some("checkbox")
    );
    assert!(
        checkbox.class_name().contains("bg-field"),
        "rust-ui classes on the primitive: {}",
        checkbox.class_name()
    );
    assert_eq!(
        checkbox.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    checkbox.click();
    wait_until("the primitive toggles the checkbox", || {
        by_role_name(&root, "checkbox", "Snap to grid")
            .and_then(|c| c.get_attribute("aria-checked"))
            .as_deref()
            == Some("true")
    })
    .await;

    let toolbar = by_role_name(&root, "toolbar", "History").unwrap();
    assert_eq!(
        toolbar.get_attribute("data-slot").as_deref(),
        Some("toolbar")
    );
    let undo = by_role_name(&root, "button", "Undo").unwrap();
    assert_eq!(
        undo.get_attribute("data-slot").as_deref(),
        Some("toolbar-button")
    );

    let inspector = by_role_name(&root, "tab", "Inspector").unwrap();
    assert_eq!(
        inspector.get_attribute("data-slot").as_deref(),
        Some("tabs-trigger")
    );
    inspector.click();
    wait_until("the primitive selects the Inspector tab", || {
        by_role_name(&root, "tab", "Inspector")
            .and_then(|t| t.get_attribute("aria-selected"))
            .as_deref()
            == Some("true")
    })
    .await;
    root.remove();
}
