//! PCB-owned workspace surface composition.
use super::objects;
use super::pcb_scene::{PcbPartHit, PcbPartPointerDown, PcbScene};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

pub(super) struct ToolbarInput {
    pub(super) command_label: String,
    pub(super) menu_owner_key: String,
    pub(super) open_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) transform: objects::LayoutTransformMenuMount,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub(super) has_selection_context: bool,
    pub(super) show_relationships: bool,
    pub(super) on_show_relationships: EventHandler<()>,
}

pub(super) struct CanvasInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Scope,
    pub(super) view_box: String,
    pub(super) selected_ids: Vec<String>,
    pub(super) generation: u64,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_empty_hit: EventHandler<PointerEvent>,
    pub(super) on_part_hit: EventHandler<PcbPartHit>,
    pub(super) on_part_pointer_down: EventHandler<PcbPartPointerDown>,
    pub(super) on_module_select: EventHandler<String>,
}

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) mirrored_pair: objects::MirroredPairMount,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "m1-pcb-objects-content",
        objects::Objects {
            selected_context: input.shared.selected_context,
            on_select: input.shared.on_select,
            on_navigate: input.shared.on_navigate,
            on_nudge: input.shared.on_nudge,
            on_open_geometry_scripts: input.shared.on_open_geometry_scripts,
            board_setup: Some(input.shared.board_setup),
            matrix_setup: Some(input.matrix_setup),
            mirrored_pair: Some(input.mirrored_pair),
            pair_created: None,
            on_place_component: Some(input.on_place_component),
            layout_target: Some(input.layout_target),
            parts_query: Some(input.parts_query),
            on_browse_parts: Some(input.on_browse_parts),
            placement_error: input.placement_error,
            matrix_inspector: None,
            add_object_menu_open: None,
        }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-layout-toolbar-region m1-pcb-toolbar-region",
        div { class: "m1-canvas-toolbar",
            objects::LayoutCommandPill {
                command_label: input.command_label,
                menu_owner_key: input.menu_owner_key,
                open_menu: input.open_menu,
                transform: input.transform,
                align: input.align,
                selection_kind: input.selection_kind,
                snap_settings: input.snap_settings,
                on_selection_kind: input.on_selection_kind,
                on_snap_intent: input.on_snap_intent,
                has_selection_context: input.has_selection_context,
                show_relationships: input.show_relationships,
                on_show_relationships: input.on_show_relationships,
            }
        }
        }
    }
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    let handlers = input.handlers;
    rsx! {
        section { class: "m1-pcb-workspace", "aria-label": "PCB workspace",
            svg {
                class: "m1-canvas m1-pcb-canvas",
                view_box: "{input.view_box}",
                preserve_aspect_ratio: "xMidYMid meet",
                tabindex: "0",
                role: "group",
                "aria-label": "PCB layout; select a part with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: handlers.mount,
                onpointerdown: move |event: PointerEvent| {
                    handlers.start_pan.call(event.clone());
                    input.on_empty_hit.call(event);
                },
                onpointermove: handlers.move_pointer,
                onpointerup: handlers.end_pointer,
                onpointercancel: handlers.cancel_pointer,
                onlostpointercapture: handlers.cancel_pointer,
                onkeydown: handlers.keyboard,
                onkeyup: handlers.key_up,
                onwheel: handlers.wheel,
                g { transform: "scale(1,-1)",
                    PcbScene {
                        snapshot: input.snapshot.clone(),
                        scope: input.scope.clone(),
                        selected_ids: input.selected_ids,
                        generation: input.generation,
                        on_part_hit: input.on_part_hit,
                        on_part_pointer_down: input.on_part_pointer_down,
                        on_module_select: input.on_module_select,
                    }
                }
            }

        }
    }
}

pub(super) fn inspector() -> Element {
    rsx! {}
}

/// Both panes stay mounted so changing the presentation mode retains numeric drafts.
#[component]
pub(super) fn PcbInspector(selected: bool, children: Element) -> Element {
    use boardstudio_web_ui_shared::ui::collapsible::{
        Collapsible, CollapsibleContent, CollapsibleTrigger,
    };
    use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
    let mut wiring = use_signal(|| true);
    let mut position_open = use_signal(|| true);
    rsx! {
        section { class: "m1-pcb-inspector", aria_label: "PCB inspector",
            ToggleGroup { class: "m1-inspector-mode", aria_label: "PCB inspector mode", horizontal: true,
                pressed: Some([if wiring() { 1usize } else { 0usize }].into_iter().collect()),
                on_pressed_change: move |pressed: std::collections::HashSet<usize>| {
                    if pressed.contains(&1) { wiring.set(true); }
                    else if pressed.contains(&0) && selected { wiring.set(false); }
                },
                ToggleItem { index: 0usize, disabled: !selected, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Selection" }
                ToggleItem { index: 1usize, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Routing & nets" }
            }
            div { hidden: wiring(), inert: wiring().then_some(""),
                Collapsible { open: position_open(), on_open_change: move |next| position_open.set(next), keep_mounted: true, class: "m1-inspector-section",
                    CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "Position" }
                    CollapsibleContent { hidden: !position_open(), inert: (!position_open()).then_some(""), super::inspector::Inspector {} }
                }
            }
            div { hidden: !wiring(), inert: (!wiring()).then_some(""), {children} }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use wasm_bindgen_test::*;

    fn fixture() -> Element {
        use_context_provider(|| crate::runtime::Runtime::new().unwrap());
        use_context_provider(|| Signal::new(0u64));
        rsx! { PcbInspector { selected: true, div { role: "status", aria_label: "Routing content", "Wiring remains mounted" } } }
    }

    #[wasm_bindgen_test]
    async fn pcb_inspector_mode_roves_and_preserves_mounted_panes() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("PCB inspector mounts", || {
            by_role_name(&root, "button", "Routing & nets").is_some()
        })
        .await;
        let routing = by_role_name(&root, "button", "Routing & nets").unwrap();
        let selection = by_role_name(&root, "button", "Selection").unwrap();
        assert_eq!(
            routing.get_attribute("aria-pressed").as_deref(),
            Some("true")
        );
        routing.focus().unwrap();
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("ArrowLeft");
        event.set_bubbles(true);
        event.set_cancelable(true);
        routing
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
        wait_until("selection receives roving focus", || {
            document
                .active_element()
                .is_some_and(|active| active == *selection)
        })
        .await;
        let event = web_sys::KeyboardEventInit::new();
        event.set_key(" ");
        event.set_bubbles(true);
        event.set_cancelable(true);
        selection
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
        selection.click();
        wait_until("selection pane is active", || {
            selection.get_attribute("aria-pressed").as_deref() == Some("true")
        })
        .await;
        assert!(
            by_role_name(&root, "status", "Routing content").is_some(),
            "hidden routing pane remains mounted"
        );
        let disclosure = by_role_name(&root, "button", "Position").unwrap();
        assert_eq!(
            disclosure.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        disclosure.click();
        wait_until("position collapses", || {
            disclosure.get_attribute("aria-expanded").as_deref() == Some("false")
        })
        .await;
        root.remove();
    }
}
