//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) mirrored_pair: objects::MirroredPairMount,
    pub(super) pair_created: Signal<Option<objects::MirroredPairCreated>>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) struct ToolbarInput {
    pub(super) save_failure: Option<String>,
    pub(super) recovery_required: bool,
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) assembly_3d: bool,
    pub(super) on_view_mode: EventHandler<bool>,
    pub(super) on_retry_save: EventHandler<()>,
    pub(super) on_recover_saved: EventHandler<()>,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) command_label: String,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) transform: objects::LayoutTransformMenuMount,
    pub(super) menu_owner_key: String,
    pub(super) open_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub(super) has_selection_context: bool,
    pub(super) show_relationships: bool,
    pub(super) on_show_relationships: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) geometry_scripts_open: bool,
    pub(super) on_close_geometry_scripts: EventHandler<()>,
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) component_inspector: Option<super::inspector::LayoutComponentInspectorProjection>,
    pub(super) on_component_inspector_action:
        EventHandler<super::inspector::LayoutComponentInspectorAction>,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) key_size: objects::KeySizeMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub(super) on_pick_splay_origin: EventHandler<()>,
    pub(super) splay_origin_pick_pending: bool,
    pub(super) on_cancel_splay_origin_pick: EventHandler<()>,
    pub(super) inspector_tab: Signal<LayoutInspectorTab>,
    pub(super) matrix_relationship_summary: Option<String>,
    pub(super) matrix_relationship_target: Option<objects::TreeSelectRequest>,
    pub(super) on_select_context: EventHandler<objects::TreeSelectRequest>,
    pub(super) outline_inspector: Option<Box<super::outline_lifecycle::OutlineInspectorProjection>>,
    pub(super) findings_return_available: bool,
    pub(super) on_findings_return: EventHandler<()>,
    pub(super) board_inspector: Option<super::board_inspector::BoardInspectorProjection>,
    pub(super) on_board_rename: EventHandler<super::board_inspector::BoardRenameAction>,
    pub(super) findings_page: Option<super::layout_findings::InspectorMount>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutInspectorTab {
    #[default]
    Properties,
    Relations,
}

/// Keep the shared tab choice scoped to the currently selected tree context.
/// Unrelated accepted revisions leave that context equal and retain the user's
/// tab, while selecting another object returns to its Properties.
pub(super) fn use_contextual_inspector_tab_reset(
    selected_context: Signal<Option<objects::ScopedTreeContext>>,
    inspector_tab: Signal<LayoutInspectorTab>,
) {
    let observed_context = selected_context();
    let previous_context = use_hook(|| Rc::new(RefCell::new(observed_context.clone())));
    use_effect(use_reactive!(|observed_context| {
        let mut inspector_tab = inspector_tab;
        if previous_context.borrow().as_ref() != observed_context.as_ref() {
            inspector_tab.set(LayoutInspectorTab::Properties);
        }
        *previous_context.borrow_mut() = observed_context.clone();
    }));
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "m1-layout-objects-content",
            objects::Objects {
                selected_context: input.shared.selected_context,
                on_select: input.shared.on_select,
                on_navigate: input.shared.on_navigate,
                on_nudge: input.shared.on_nudge,
                on_open_geometry_scripts: input.shared.on_open_geometry_scripts,
                board_setup: Some(input.shared.board_setup),
                matrix_setup: Some(input.matrix_setup),
                matrix_inspector: Some(input.matrix_inspector),
                mirrored_pair: Some(input.mirrored_pair),
                pair_created: Some(input.pair_created),
                on_place_component: Some(input.on_place_component),
                layout_target: Some(input.layout_target),
                parts_query: Some(input.parts_query),
                on_browse_parts: Some(input.on_browse_parts),
                placement_error: input.placement_error,
            }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "m1-canvas-toolbar",
            if input.assembly_3d {
                div { class: "m1-canvas-context",
                    strong { "Layout" }
                    span { "PCB assembly" }
                }
            } else {
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
            if let Some(reason) = input.save_failure.as_ref() {
                p { role: "alert", class: "m1-save-error", "Save failed: {reason}" }
                button { onclick: move |_| input.on_retry_save.call(()), "Retry save" }
            }
            if input.recovery_required {
                button { onclick: move |_| input.on_recover_saved.call(()), "Reopen last saved version (discard pending changes)" }
            }
        }
        div { class: "m1-layout-view-group", role: "group", aria_label: "Design view",
            button {
                r#type: "button",
                aria_pressed: "{!input.assembly_3d}",
                onclick: move |_| input.on_view_mode.call(false),
                "2D"
            }
            button {
                r#type: "button",
                aria_pressed: "{input.assembly_3d}",
                onclick: move |_| input.on_view_mode.call(true),
                "3D assembly"
            }
            if !input.assembly_3d {
                button {
                    r#type: "button",
                    aria_pressed: "{input.footprints_pressed}",
                    onclick: move |_| input.on_toggle_footprints.call(()),
                    "Footprints"
                }
            }
        }
    }
}

pub(super) fn inspector(mut input: InspectorInput) -> Element {
    let mut matrix_relationship_escape_target = use_signal(|| None::<objects::TreeSelectRequest>);
    let component_owner = input
        .component_inspector
        .as_ref()
        .map(|projection| projection.owner.clone());
    let board_inspector_visible = input.board_inspector.is_some();
    let observed_relationship_target = matrix_relationship_escape_target();
    let observed_component_owner = component_owner.clone();
    let observed_board_visible = board_inspector_visible;
    use_effect(use_reactive!(
        |observed_relationship_target, observed_component_owner, observed_board_visible| {
            let Some(target_value) = observed_relationship_target.as_ref() else {
                return;
            };
            if observed_board_visible
                || observed_component_owner.as_ref().is_some_and(|owner| {
                    !matrix_relationship_escape_target_is_current(target_value, owner)
                })
            {
                matrix_relationship_escape_target.set(None);
            }
        }
    ));
    let mut inspector_tab = input.inspector_tab;
    let mut return_target = matrix_relationship_escape_target;
    let on_matrix_relationship_escape =
        EventHandler::new(move |request: objects::TreeSelectRequest| {
            return_target.set(None);
            inspector_tab.set(LayoutInspectorTab::Properties);
            input.on_select_context.call(request);
        });
    if let Some(page) = input.findings_page.filter(|page| page.open) {
        return rsx! {
            PendingSplayOriginPickEscape {
                pending: input.splay_origin_pick_pending,
                on_cancel: input.on_cancel_splay_origin_pick,
            }
            super::layout_findings::LayoutFindingsInspector {
                open: page.open,
                document: page.document,
                findings: page.findings,
                source: page.source,
                on_close: page.on_close,
                on_navigate: page.on_navigate,
            }
        };
    }
    if input.geometry_scripts_open {
        return rsx! {
            PendingSplayOriginPickEscape {
                pending: input.splay_origin_pick_pending,
                on_cancel: input.on_cancel_splay_origin_pick,
            }
            super::geometry_scripts::GeometryScriptsEditor {
                on_back: input.on_close_geometry_scripts,
            }
        };
    }
    let board_context_header = input.board_inspector.is_some();
    let matrix_context_tabs = input.component_inspector.is_none()
        && input.outline_inspector.is_none()
        && input.board_inspector.is_none()
        && input.matrix_transform_inspector.projection.is_some();
    let show_matrix_relations =
        matrix_context_tabs && (input.inspector_tab)() == LayoutInspectorTab::Relations;
    rsx! {
        PendingSplayOriginPickEscape {
            pending: input.splay_origin_pick_pending,
            on_cancel: input.on_cancel_splay_origin_pick,
        }
        MatrixRelationshipEscape {
            target: matrix_relationship_escape_target(),
            component_owner,
            on_escape: on_matrix_relationship_escape,
        }
        if input.findings_return_available {
            div { class: "m1-layout-findings-return",
                button {
                    class: "m1-layout-findings-back",
                    id: "m1-layout-findings-back-to-selection",
                    r#type: "button",
                    onclick: move |_| input.on_findings_return.call(()),
                    "Back to selection"
                }
            }
        }
        if input.outline_inspector.is_none() && input.component_inspector.is_none() {
        if let Some(title) = input.context_title.as_ref() {
            section { class: if board_context_header { "m1-selected-context m1-board-context" } else { "m1-selected-context" }, "aria-label": "Selected context",
                h2 { "{title}" }
                if let Some(detail) = input.context_detail.as_ref() { p { "{detail}" } }
            }
        }
        }
        if matrix_context_tabs {
            div { class: "m1-layout-component-tabs", role: "tablist", aria_label: "Inspector details",
                button {
                    role: "tab",
                    aria_selected: "{(input.inspector_tab)() == LayoutInspectorTab::Properties}",
                    onclick: move |_| input.inspector_tab.set(LayoutInspectorTab::Properties),
                    "Properties"
                }
                button {
                    role: "tab",
                    aria_selected: "{(input.inspector_tab)() == LayoutInspectorTab::Relations}",
                    onclick: move |_| input.inspector_tab.set(LayoutInspectorTab::Relations),
                    "Relations"
                }
            }
        }
        if show_matrix_relations {
            section { class: "m1-layout-component-inspector", aria_label: "Relationships",
                h2 { "Relationships" }
                p { class: "m1-layout-component-relation-summary", "{input.matrix_relationship_summary.as_deref().unwrap_or(\"No saved placement relationship on this selection.\")}" }
                if let Some(target) = input.matrix_relationship_target.clone() {
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            matrix_relationship_escape_target.set(Some(target.clone()));
                            input.inspector_tab.set(LayoutInspectorTab::Properties);
                            input.on_select_context.call(target.clone());
                        },
                        "Edit placement relationship"
                    }
                }
                p { class: "m1-layout-component-matrix-note", "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties." }
            }
        } else {
            if input.outline_inspector.is_none() {
                if let Some(projection) = input.component_inspector {
                    super::inspector::LayoutComponentInspector {
                        projection,
                        inspector_tab: input.inspector_tab,
                        on_action: input.on_component_inspector_action,
                    }
                } else if input.show_position_inspector { super::inspector::Inspector {} }
            }
            if let Some(projection) = input.matrix_inspector.projection.clone() {
                objects::MatrixInspector {
                    projection,
                    request_sequence: input.matrix_inspector.request_sequence,
                    editable: input.matrix_inspector.editable,
                    busy: input.matrix_inspector.busy,
                    feedback: input.matrix_inspector.feedback.clone(),
                    on_edit: input.matrix_inspector.on_edit,
                    on_apply_preset: input.matrix_inspector.on_apply_preset,
                    on_delete: input.matrix_inspector.on_delete,
                    on_unlink: input.matrix_inspector.on_unlink,
                    on_duplicate: input.matrix_inspector.on_duplicate,
                }
            }
            if input.key_size.projection.is_some() {
                objects::KeySizeControls { mount: input.key_size }
            }
            if input.matrix_transform_inspector.projection.is_some() {
                objects::MatrixTransformInspector {
                    mount: input.matrix_transform_inspector,
                    on_pick_splay_origin: input.on_pick_splay_origin,
                }
            }
            if let Some(projection) = input.outline_inspector {
                super::outline_lifecycle::OutlineVersionInspector { projection: *projection }
            }
            if let Some(projection) = input.board_inspector {
                super::board_inspector::BoardInspector {
                    projection,
                    on_rename: input.on_board_rename,
                }
            }
        }
    }
}

#[component]
fn MatrixRelationshipEscape(
    target: Option<objects::TreeSelectRequest>,
    component_owner: Option<super::inspector::LayoutComponentInspectorOwner>,
    on_escape: EventHandler<objects::TreeSelectRequest>,
) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use std::{cell::RefCell, rc::Rc};
        use wasm_bindgen::{JsCast, closure::Closure};

        type Listener = Rc<
            RefCell<
                Option<(
                    web_sys::Document,
                    Closure<dyn FnMut(web_sys::KeyboardEvent)>,
                )>,
            >,
        >;

        let listener = use_hook(Listener::default);
        let effect_listener = listener.clone();
        use_effect(use_reactive!(|target, component_owner| {
            let listener = effect_listener.clone();
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
            let (Some(target), Some(component_owner)) = (target.as_ref(), component_owner.as_ref())
            else {
                return;
            };
            if !matrix_relationship_escape_target_is_current(target, component_owner) {
                return;
            }
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let target = target.clone();
            let component_owner = component_owner.clone();
            let on_escape = on_escape;
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|target| {
                        target
                            .closest("input, textarea, select, [contenteditable='true']")
                            .ok()
                            .flatten()
                            .is_some()
                    })
                {
                    return;
                }
                let Some(request) = matrix_relationship_escape_return_request(
                    &target,
                    &component_owner,
                    &event.key(),
                    event.default_prevented(),
                ) else {
                    return;
                };
                event.prevent_default();
                event.stop_propagation();
                on_escape.call(request);
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }));
        use_drop({
            let listener = listener.clone();
            move || {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (target, component_owner, on_escape);
    rsx! {}
}

fn matrix_relationship_escape_target_is_current(
    target: &objects::TreeSelectRequest,
    component_owner: &super::inspector::LayoutComponentInspectorOwner,
) -> bool {
    target.scope == component_owner.scope
        && matches!(
            &target.context,
            objects::TreeContext::Component {
                part_id: Some(part_id),
                ..
            } if part_id == &component_owner.part_id
        )
}

fn matrix_relationship_escape_return_request(
    target: &objects::TreeSelectRequest,
    component_owner: &super::inspector::LayoutComponentInspectorOwner,
    key: &str,
    default_prevented: bool,
) -> Option<objects::TreeSelectRequest> {
    if key != "Escape"
        || default_prevented
        || !matrix_relationship_escape_target_is_current(target, component_owner)
    {
        return None;
    }
    Some(objects::TreeSelectRequest {
        scope: target.scope.clone(),
        context: objects::TreeContext::Board {
            board_id: target.scope.board_id.clone(),
        },
        mode: target.mode.clone(),
        outline_action: None,
    })
}

#[component]
fn PendingSplayOriginPickEscape(pending: bool, on_cancel: EventHandler<()>) -> Element {
    #[cfg(target_arch = "wasm32")]
    {
        use std::{cell::RefCell, rc::Rc};
        use wasm_bindgen::{JsCast, closure::Closure};

        type Listener = Rc<
            RefCell<
                Option<(
                    web_sys::Document,
                    Closure<dyn FnMut(web_sys::KeyboardEvent)>,
                )>,
            >,
        >;

        let listener = use_hook(Listener::default);
        use_effect(use_reactive((&pending,), {
            let listener = listener.clone();
            move |(pending,)| {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
                if !pending {
                    return;
                }
                let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                    return;
                };
                let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if event.key() != "Escape" || event.default_prevented() {
                        return;
                    }
                    if event
                        .target()
                        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|target| {
                            target
                                .closest("input, textarea, select, [contenteditable='true']")
                                .ok()
                                .flatten()
                                .is_some()
                        })
                    {
                        return;
                    }
                    event.prevent_default();
                    event.stop_propagation();
                    on_cancel.call(());
                }) as Box<dyn FnMut(_)>);
                let _ = document
                    .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
                *listener.borrow_mut() = Some((document, callback));
            }
        }));
        use_drop({
            let listener = listener.clone();
            move || {
                if let Some((document, callback)) = listener.borrow_mut().take() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        callback.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (pending, on_cancel);

    rsx! {}
}

#[cfg(all(test, target_arch = "wasm32"))]
mod origin_pick_escape_regression_tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn pending_pick_host() -> Element {
        let mut pending = use_signal(|| false);
        rsx! {
            PendingSplayOriginPickEscape {
                pending: pending(),
                on_cancel: EventHandler::new(move |()| pending.set(false)),
            }
            button {
                id: "arm-origin-pick",
                onclick: move |_| pending.set(true),
                "Pick origin"
            }
            output { id: "pending-origin-pick", "{pending()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_inspector_cancels_pending_origin_pick() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(pending_pick_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        root.query_selector("#arm-origin-pick")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        document
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;

        assert_eq!(
            root.query_selector("#pending-origin-pick")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false"),
            "Escape from the Inspector must clear the pending canvas pick"
        );
        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod relation_escape_regression_tests {
    use super::*;
    use boardstudio_application::{Scope, SelectionMode, SessionEpoch};
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn route() -> (
        objects::TreeSelectRequest,
        super::super::inspector::LayoutComponentInspectorOwner,
    ) {
        let scope = Scope {
            session_epoch: SessionEpoch(5),
            document_id: "layout-escape".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let part_id = "left-keys-SW1".to_owned();
        (
            objects::TreeSelectRequest {
                scope: scope.clone(),
                context: objects::TreeContext::Component {
                    part_id: Some(part_id.clone()),
                    matrix_id: Some("left-keys".into()),
                    row: Some(0),
                    column: Some(0),
                    assembly_id: None,
                },
                mode: SelectionMode::Replace,
                outline_action: None,
            },
            super::super::inspector::LayoutComponentInspectorOwner {
                scope,
                snapshot_token: boardstudio_application::SnapshotToken(13),
                revision: 9,
                context_generation: 2,
                scope_generation: 3,
                part_id,
                selected_part_ids: vec!["left-keys-SW1".to_owned()],
            },
        )
    }

    #[component]
    fn relation_escape_host() -> Element {
        let (target, owner) = route();
        let mut context = use_signal(|| "component");
        rsx! {
            MatrixRelationshipEscape {
                target: Some(target),
                component_owner: Some(owner),
                on_escape: EventHandler::new(move |request: objects::TreeSelectRequest| {
                    context.set(if matches!(request.context, objects::TreeContext::Board { .. }) {
                        "board"
                    } else {
                        "unexpected"
                    });
                }),
            }
            output { id: "selection-context", "{context()}" }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_matrix_relationship_component_returns_to_board_context() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(relation_escape_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        document
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;

        assert_eq!(
            root.query_selector("#selection-context")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("board"),
            "Escape after Edit placement relationship returns to the Board Inspector context"
        );
        root.remove();
    }
}
