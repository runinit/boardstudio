//! Layout-owned Objects, toolbar and Inspector composition.
use super::objects;
use super::workspace_composition::SharedObjectsInput;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use boardstudio_web_ui_shared::ui::tabs::{Tabs, TabsContent, TabsList, TabsTrigger};
use boardstudio_web_ui_shared::ui::{
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
pub(super) use objects::{LayoutInspectorTab, LayoutWorkspaceState, use_layout_workspace_state};
use std::rc::Rc;

pub(in crate::presentation) mod inspector_wiring;
pub(in crate::presentation) mod pointer;
pub(in crate::presentation) mod scene;
pub(in crate::presentation) mod toolbar_wiring;

#[derive(Clone)]
pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) pair_created: Signal<Option<objects::MirroredPairCreated>>,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
    pub(super) add_object_menu_open: Signal<bool>,
}

#[derive(Clone)]
pub(super) struct ToolbarInput {
    pub(super) footprints_pressed: bool,
    pub(super) on_toggle_footprints: EventHandler<()>,
    pub(super) assembly_3d: bool,
    pub(super) on_view_mode: EventHandler<bool>,
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

#[derive(Clone)]
pub(super) struct InspectorInput {
    pub(super) context_title: Option<String>,
    pub(super) context_detail: Option<String>,
    pub(super) show_position_inspector: bool,
    pub(super) component_inspector: Option<super::inspector::LayoutComponentInspectorProjection>,
    pub(super) component_inspector_x: Signal<String>,
    pub(super) component_inspector_y: Signal<String>,
    pub(super) component_inspector_margin: Signal<String>,
    pub(super) component_inspector_error: Signal<Option<String>>,
    pub(super) component_inspector_x_failure: Signal<Option<String>>,
    pub(super) component_inspector_y_failure: Signal<Option<String>>,
    pub(super) component_inspector_margin_failure: Signal<Option<String>>,
    pub(super) companion_placement: Option<super::inspector::CompanionPlacementProjection>,
    pub(super) companion_placement_error: Signal<Option<String>>,
    pub(super) on_companion_placement_action:
        EventHandler<super::inspector::CompanionPlacementAction>,
    pub(super) on_component_inspector_action:
        EventHandler<super::inspector::LayoutComponentInspectorAction>,
    pub(super) matrix_inspector: objects::MatrixInspectorMount,
    pub(super) key_size: objects::KeySizeMount,
    pub(super) matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub(super) on_pick_splay_origin: EventHandler<()>,
    pub(super) on_pick_anchor: EventHandler<()>,
    pub(super) anchor_pick_pending: bool,
    pub(super) on_cancel_anchor_pick: EventHandler<()>,
    /// Select an aggregate anchor part's key without writing.
    pub(super) on_navigate_anchor_part: EventHandler<objects::TreeContext>,
    pub(super) inspector_tab: Signal<LayoutInspectorTab>,
    pub(super) matrix_relations: Option<boardstudio_web_layout::matrix_relations::MatrixRelations>,
    /// Resolves a Relations reference against the live accepted document at click time:
    /// the request to submit (a component opens its Inspector context), or `None` when the
    /// target has gone.
    pub(super) relation_target:
        Rc<dyn Fn(objects::TreeSelectRequest) -> Option<objects::TreeSelectRequest>>,
    /// The scope of the selected tree context, which Relations navigation requests carry.
    pub(super) relation_scope: Option<boardstudio_application::Scope>,
    pub(super) on_select_context: EventHandler<objects::TreeSelectRequest>,
    pub(super) outline_inspector: Option<Box<super::outline_lifecycle::OutlineInspectorProjection>>,
    pub(super) findings_return_available: bool,
    pub(super) on_findings_return: EventHandler<()>,
    pub(super) board_inspector_mount: super::board_inspector::BoardInspectorMount,
    pub(super) findings_page: Option<super::layout_findings::InspectorMount>,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        div { class: "block h-auto overflow-visible",
            objects::Objects {
                selected_context: input.shared.selected_context,
                on_select: input.shared.on_select,
                on_navigate: input.shared.on_navigate,
                on_nudge: input.shared.on_nudge,
                board_setup: Some(input.shared.board_setup),
                matrix_setup: Some(input.matrix_setup),
                matrix_inspector: Some(input.matrix_inspector),
                pair_created: Some(input.pair_created),
                on_place_component: Some(input.on_place_component),
                layout_target: Some(input.layout_target),
                parts_query: Some(input.parts_query),
                on_browse_parts: Some(input.on_browse_parts),
                placement_error: input.placement_error,
                add_object_menu_open: Some(input.add_object_menu_open),
            }
        }
    }
}

pub(super) fn add_object_header_action(mut menu_open: Signal<bool>) -> Element {
    let open = menu_open();
    rsx! {
        div { class: "relative flex-none border-b border-border px-2.5 py-2 [&>button]:min-h-[38px] [&>button]:w-full",
            Button {
                variant: ButtonVariant::Secondary,
                button_type: "button",
                id: "m1-add-object-trigger",
                aria_label: "Add object",
                class: "max-[760px]:min-h-control-touch!",
                onkeydown: super::view_helpers::stop_space_propagation,
                aria_expanded: open,
                onclick: move |_| menu_open.set(!menu_open()),

                svg {
                    class: "size-4 fill-none stroke-current [stroke-linecap:round] [stroke-width:1.8]",
                    "aria-hidden": "true",
                    view_box: "0 0 16 16",
                    path { d: "M8 3v10M3 8h10" }
                }
                "Add object"

            }
        }
    }
}

pub(super) fn toolbar(input: ToolbarInput) -> Element {
    rsx! {
        div { class: "pointer-events-none absolute inset-0 z-[12] [container:layout-toolbar/inline-size] [&>*]:pointer-events-auto", "data-workspace-toolbar": "layout",
        onkeydown: super::view_helpers::stop_space_propagation,
            div { class: "pointer-events-none! absolute inset-x-3 top-3 max-[760px]:inset-x-2 max-[760px]:top-2",
                if !input.assembly_3d {
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
                        snap_in_dock: true,
                    }
                }
            }
            div { class: "absolute bottom-2 left-3 z-[11] flex gap-0.5 rounded-[8px] bg-toolbar p-[3px]",
                ToggleGroup {
                    aria_label: "Design view",
                    horizontal: true,
                    pressed: Some(
                        std::collections::HashSet::from([
                            if input.assembly_3d { 1usize } else { 0usize },
                        ]),
                    ),
                    on_pressed_change: move |next: std::collections::HashSet<usize>| {
                        if next.contains(&0) {
                            input.on_view_mode.call(false);
                        } else if next.contains(&1) {
                            input.on_view_mode.call(true);
                        }
                    },
                    Tooltip {
                        TooltipTrigger {
                            r#as: move |attributes: Vec<Attribute>| rsx! {
                                ToggleItem {
                                    index: 0usize,
                                    aria_label: "2D",
                                    class: "min-h-11 min-w-11",
                                    attributes,
                                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                        rect {
                                            x: "3",
                                            y: "3",
                                            width: "14",
                                            height: "14",
                                            rx: "2",
                                        }
                                        path { d: "M3 8h14M8 3v14" }
                                    }
                                    span { class: "sr-only", "2D" }
                                }
                            },
                        }
                        TooltipContent { "2D layout" }
                    }
                    Tooltip {
                        TooltipTrigger {
                            r#as: move |attributes: Vec<Attribute>| rsx! {
                                ToggleItem {
                                    index: 1usize,
                                    aria_label: "3D assembly",
                                    class: "min-h-11 min-w-11",
                                    attributes,
                                    svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                        path { d: "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8" }
                                    }
                                    span { class: "sr-only", "3D assembly" }
                                }
                            },
                        }
                        TooltipContent { "3D assembly" }
                    }
                }
                if !input.assembly_3d {
                    ToggleGroup {
                        aria_label: "Footprints visibility",
                        pressed: Some(
                            if input.footprints_pressed {
                                std::collections::HashSet::from([0usize])
                            } else {
                                std::collections::HashSet::new()
                            },
                        ),
                        on_pressed_change: move |_| input.on_toggle_footprints.call(()),
                        Tooltip {
                            TooltipTrigger {
                                r#as: move |attributes: Vec<Attribute>| rsx! {
                                    ToggleItem {
                                        index: 0usize,
                                        aria_label: "Footprints",
                                        class: "min-h-11 min-w-11",
                                        attributes,
                                        svg { view_box: "0 0 20 20", "aria-hidden": "true",
                                            rect {
                                                x: "5",
                                                y: "4",
                                                width: "10",
                                                height: "12",
                                                rx: "1",
                                            }
                                            path { d: "M2 7h3M2 13h3M15 7h3M15 13h3" }
                                        }
                                        span { class: "sr-only", "Footprints" }
                                    }
                                },
                            }
                            TooltipContent { "Footprints" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn LayoutInspector(input: ReadSignal<InspectorInput>) -> Element {
    let mut input = input();
    let mut key_size_open = use_signal(|| true);
    let mut matrix_relationship_escape_target = use_signal(|| None::<objects::TreeSelectRequest>);
    let component_owner = input
        .component_inspector
        .as_ref()
        .map(|projection| projection.owner.clone());
    let board_inspector_visible = input.board_inspector_mount.projection.is_some();
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
    let relation_scope = input.relation_scope.clone();
    let relation_target = input.relation_target.clone();
    let relation_select = input.on_select_context;
    let mut relation_escape_target = matrix_relationship_escape_target;
    let mut relation_tab = input.inspector_tab;
    let on_navigate_relation = EventHandler::new(move |context: objects::TreeContext| {
        let Some(scope) = relation_scope.clone() else {
            return;
        };
        let request = objects::TreeSelectRequest {
            scope,
            context,
            mode: boardstudio_application::SelectionMode::Replace,
            outline_action: None,
        };
        // A target removed since the row was projected: do nothing; the next accepted
        // revision re-projects without it.
        let Some(request) = relation_target(request) else {
            return;
        };
        if matches!(request.context, objects::TreeContext::Component { .. }) {
            relation_escape_target.set(Some(request.clone()));
            relation_tab.set(LayoutInspectorTab::Properties);
        }
        relation_select.call(request);
    });
    // Parameter entries select their owner and open the matching tool/editor
    // beside the selection. The same currency guard applies: a target removed
    // since the row was projected does nothing.
    let open_scope = input.relation_scope.clone();
    let open_target = input.relation_target.clone();
    let open_select = input.on_select_context;
    let on_open_owner_relation = EventHandler::new(move |context: objects::TreeContext| {
        let Some(scope) = open_scope.clone() else {
            return;
        };
        let request = objects::TreeSelectRequest {
            scope,
            context,
            mode: boardstudio_application::SelectionMode::Replace,
            outline_action: None,
        };
        let Some(request) = open_target(request) else {
            return;
        };
        open_select.call(request);
    });
    if let Some(page) = input.findings_page.filter(|page| page.open) {
        return rsx! {
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
    let board_context_header = input.board_inspector_mount.projection.is_some();
    let show_context_header =
        input.outline_inspector.is_none() && input.component_inspector.is_none();
    let matrix_context_tabs = input.component_inspector.is_none()
        && input.outline_inspector.is_none()
        && input.board_inspector_mount.projection.is_none()
        && input.matrix_transform_inspector.is_active();
    let properties = rsx! {
        if input.outline_inspector.is_none() {
            if let Some(projection) = input.component_inspector {
                super::inspector::LayoutComponentInspector {
                    projection,
                    inspector_tab: input.inspector_tab,
                    x: input.component_inspector_x,
                    y: input.component_inspector_y,
                    margin: input.component_inspector_margin,
                    error: input.component_inspector_error,
                    x_failure: input.component_inspector_x_failure,
                    y_failure: input.component_inspector_y_failure,
                    margin_failure: input.component_inspector_margin_failure,
                    on_action: input.on_component_inspector_action,
                }
            } else if input.show_position_inspector {
                super::inspector::Inspector {}
            }
        }
        if let Some(projection) = input.companion_placement {
            super::inspector::CompanionPlacementInspector {
                projection,
                error: input.companion_placement_error,
                on_apply: input.on_companion_placement_action,
            }
        }
        if input.matrix_inspector.is_active() {
            objects::MatrixInspector { mount: input.matrix_inspector.clone() }
        }
        if input.key_size.projection.is_some() {
            Collapsible {
                class: "mx-4 mb-4 border-0",
                keep_mounted: true,
                open: Some(key_size_open()),
                on_open_change: move |next| key_size_open.set(next),
                CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold",
                    "Key size"
                    ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" }
                }
                CollapsibleContent {
                    class: "data-[open=true]:overflow-visible",
                    hidden: !key_size_open(),
                    inert: (!key_size_open()).then_some(""),
                    objects::KeySizeControls { mount: input.key_size }
                }
            }
        }
        if input.matrix_transform_inspector.is_active() {
            objects::MatrixTransformInspector {
                mount: input.matrix_transform_inspector.clone(),
                on_pick_splay_origin: input.on_pick_splay_origin,
                on_pick_anchor: input.on_pick_anchor,
                anchor_pick_pending: input.anchor_pick_pending,
                on_cancel_anchor_pick: input.on_cancel_anchor_pick,
                on_navigate_anchor_part: input.on_navigate_anchor_part,
            }
        }
        if let Some(projection) = input.outline_inspector {
            super::outline_lifecycle::OutlineVersionInspector { projection: *projection }
        }
        if input.board_inspector_mount.projection.is_some() {
            super::board_inspector::BoardInspector { mount: input.board_inspector_mount }
        }
    };
    rsx! {
            super::layout_key_selection_actions::SelectedKeysActions {}
            MatrixRelationshipEscape {
                target: matrix_relationship_escape_target(),
                component_owner,
                on_escape: on_matrix_relationship_escape,
            }
            if input.findings_return_available {
                div { class: "flex items-center gap-2 text-sm text-muted-foreground",
                onkeydown: super::view_helpers::stop_space_propagation,
                    Button {
                        variant: ButtonVariant::Secondary,
                        class: "max-[760px]:min-h-control-touch",
                        id: "m1-layout-findings-back-to-selection",
                        button_type: "button",
                        onclick: move |_| input.on_findings_return.call(()),

                        "Back to selection"

                    }
                }
            }
            if show_context_header {
                if let Some(title) = input.context_title.as_ref() {
                    section {
                        class: if board_context_header { "border-b border-border bg-card px-4 pt-6 pb-4 [&_h2]:m-0 [&_h2]:text-2xl [&_h2]:leading-tight [&_p]:mt-1.5 [&_p]:text-muted-foreground" } else { "border-b border-border bg-card px-4 pt-6 pb-4 [&_h2]:m-0 [&_h2]:text-xl [&_h2]:leading-tight [&_p]:mt-1.5 [&_p]:text-muted-foreground" },
                        "aria-label": "Selected context",
                        h2 { "{title}" }
                        if let Some(detail) = input.context_detail.as_ref() {
                            p { "{detail}" }
                        }
                    }
                }
            }

            if matrix_context_tabs {
                if let Some(relations) = input.matrix_relations.clone() {
                    super::layout_matrix_relations::PlacementBreadcrumb {
                        relations, on_navigate: on_navigate_relation, on_open_owner: on_open_owner_relation,
                    }
                }
                Tabs { horizontal: true,
                attributes: vec![dioxus::events::onkeydown(super::view_helpers::stop_space_propagation)],
    value: Some(
                        if (input.inspector_tab)() == LayoutInspectorTab::Properties {
                            "properties".to_owned()
                        } else {
                            "relations".to_owned()
                        },
                    ),
    on_value_change: move |value: String| input.inspector_tab.set(if value == "relations" { LayoutInspectorTab::Relations } else { LayoutInspectorTab::Properties }),
                    TabsList {
                        aria_label: "Inspector details",
                        class: "layout-inspector-tabs",
                        TabsTrigger { index: 0usize, value: "properties", "Properties" }
                        TabsTrigger { index: 1usize, value: "relations", "Relations" }
                    }
                    TabsContent { index: 0usize, value: "properties", {properties.clone()} }
                    TabsContent { index: 1usize, value: "relations",
                        super::layout_matrix_relations::MatrixRelationsPanel {
                            relations: input.matrix_relations.clone(),
                            mount: input.matrix_transform_inspector.clone(),
                            on_navigate: on_navigate_relation,
                            on_open_owner: on_open_owner_relation,
                        }
                    }
                 }
            } else {
                {properties}
            }
        }
}

#[component]
fn MatrixRelationshipEscape(
    target: Option<objects::TreeSelectRequest>,
    component_owner: Option<super::inspector::LayoutComponentInspectorOwner>,
    on_escape: EventHandler<objects::TreeSelectRequest>,
) -> Element {
    let mut input = use_context::<super::layout_input::LayoutInput>();
    let request = target
        .as_ref()
        .filter(|target| {
            component_owner
                .as_ref()
                .is_some_and(|owner| matrix_relationship_escape_target_is_current(target, owner))
        })
        .map(|target| objects::TreeSelectRequest {
            scope: target.scope.clone(),
            context: objects::TreeContext::Board {
                board_id: target.scope.board_id.clone(),
            },
            mode: target.mode,
            outline_action: None,
        });
    let active = request.is_some();
    let escape = use_callback(move |()| {
        if let Some(request) = request.clone() {
            on_escape.call(request);
        }
    });
    use_effect(use_reactive((&active,), move |(active,)| {
        input.relation_escape.set(active.then_some(escape));
    }));
    use_drop(move || {
        if *input.relation_escape.peek() == Some(escape) {
            input.relation_escape.set(None);
        }
    });
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

#[cfg(all(test, target_arch = "wasm32"))]
mod origin_pick_escape_regression_tests {
    use super::*;
    use crate::presentation::test_mount_helpers;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn pending_pick_host() -> Element {
        let input = super::super::layout_input::use_layout_input();
        let interaction = use_context::<super::super::LayoutInteraction>();
        let mut pending = use_signal(|| false);
        rsx! {
            main { id: "m1-workbench", tabindex: "-1",
                onkeydown: move |event| input.keydown(&event, true),
                Button {
                    variant: ButtonVariant::Secondary,
                    id: "arm-origin-pick",
                    onclick: move |_| {
                        if interaction.begin(super::super::CanvasMode::PickingSplayOrigin) {
                            pending.set(true);
                            interaction.on_cancel(move || { let mut pending = pending; pending.set(false); });
                        }
                    },
                    "Pick origin"
                }
                output { id: "pending-origin-pick", "{pending()}" }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_inspector_cancels_pending_origin_pick() {
        let root = test_mount_helpers::mount(VirtualDom::new(pending_pick_host), None);
        wait_until("the origin-pick control renders", || {
            root.query_selector("#arm-origin-pick")
                .is_ok_and(|button| button.is_some())
        })
        .await;

        root.query_selector("#arm-origin-pick")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        wait_until("the origin pick becomes pending", || {
            root.query_selector("#pending-origin-pick")
                .ok()
                .flatten()
                .is_some_and(|state| state.text_content().as_deref() == Some("true"))
        })
        .await;
        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        options.set_cancelable(true);
        root.query_selector("#arm-origin-pick")
            .unwrap()
            .unwrap()
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        wait_until("Escape cancels the pending origin pick", || {
            root.query_selector("#pending-origin-pick")
                .ok()
                .flatten()
                .is_some_and(|state| state.text_content().as_deref() == Some("false"))
        })
        .await;

        assert_eq!(
            root.query_selector("#pending-origin-pick")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("false"),
            "Escape from the Inspector must clear the pending canvas pick"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod relation_escape_regression_tests {
    use super::*;
    use crate::presentation::test_mount_helpers;
    use boardstudio_application::{Scope, SelectionMode, SessionEpoch};
    use boardstudio_web_ui_shared::test_wait::wait_until;
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
        let input = super::super::layout_input::use_layout_input();
        let mut context = use_signal(|| "component");
        rsx! { main { id: "m1-workbench", tabindex: "-1", onkeydown: move |event| input.keydown(&event, true),
            MatrixRelationshipEscape {
                target: Some(target),
                component_owner: Some(owner),
                on_escape: EventHandler::new(move |request: objects::TreeSelectRequest| {
                    context
                        .set(
                            if matches!(request.context, objects::TreeContext::Board { .. }) {
                                "board"
                            } else {
                                "unexpected"
                            },
                        );
                }),
            }
            output { id: "selection-context", "{context()}" }
        }
        }
    }

    #[wasm_bindgen_test]
    async fn escape_from_matrix_relationship_component_returns_to_board_context() {
        let root = test_mount_helpers::mount(VirtualDom::new(relation_escape_host), None);
        wait_until("the matrix relationship renders", || {
            root.query_selector("#selection-context")
                .ok()
                .flatten()
                .is_some_and(|context| context.text_content().as_deref() == Some("component"))
        })
        .await;

        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        options.set_cancelable(true);
        root.query_selector("#selection-context")
            .unwrap()
            .unwrap()
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)
                    .unwrap(),
            )
            .unwrap();
        wait_until("Escape returns to the board Inspector context", || {
            root.query_selector("#selection-context")
                .ok()
                .flatten()
                .is_some_and(|context| context.text_content().as_deref() == Some("board"))
        })
        .await;

        assert_eq!(
            root.query_selector("#selection-context")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("board"),
            "Escape after Edit placement relationship returns to the Board Inspector context"
        );
    }
}

/// Layout-only mounts. Created inside `LayoutWorkspace` so the session never
/// hoards them; the Align reference persists on `LayoutWorkspaceState` across
/// workspace switches while feedback and projection stay per-mount.
pub(in crate::presentation) struct LayoutMounts {
    pub board_inspector: super::board_inspector::BoardInspectorMount,
    pub key_size: objects::KeySizeMount,
    pub layout_align: objects::LayoutAlignMount,
}

pub(in crate::presentation) fn use_layout_mounts() -> LayoutMounts {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let adapter = use_context::<super::SelectionAdapter>();
    let runtime = editor.runtime();
    let boardstudio_web_ui_model::editor::EditorContext {
        version, workspace, ..
    } = editor;
    let board_inspector = super::board_inspector::use_board_inspector(
        runtime.clone(),
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let key_size = objects::use_key_size(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
    );
    let layout_align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        adapter.generation,
        Workspace::Layout,
    );
    LayoutMounts {
        board_inspector,
        key_size,
        layout_align,
    }
}

pub(in crate::presentation) struct LayoutBundle {
    pub objects: ObjectsInput,
    pub inspector: InspectorInput,
    pub toolbar: ToolbarInput,
    pub canvas: Element,
    pub commands: Vec<boardstudio_web_ui_shared::ui::command::Command>,
}

/// Assemble every Layout surface from the frame and contexts. All Layout-only
/// hooks (`use_canvas_bundle`, `use_layout_toolbar`, the pointer builders, the
/// outline overlay, the selection anchor) run here at Layout lifetime; the
/// session only supplies shared services through the frame.
pub(in crate::presentation) fn use_layout_bundle(
    frame: &super::workspace_composition::WorkspaceFrame,
) -> LayoutBundle {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let layout_state = use_context::<LayoutWorkspaceState>();
    let services = frame.layout_services();
    let mounts = use_layout_mounts();
    let lifetime =
        use_hook(|| Rc::new(super::inspector::LayoutComponentInspectorLifetime::default()));
    let canvas = inspector_wiring::use_canvas_bundle(frame, &mounts, lifetime.clone());
    let toolbar_wiring = toolbar_wiring::use_layout_toolbar(frame, &canvas, &mounts, lifetime);
    let pointer = pointer::build_canvas_pointer(frame);
    let overlay = scene::use_outline_overlay(frame);
    let selected_tree_context = canvas.selection.selected_tree_context.clone();
    let bridge = scene::selected_bridge(&frame.snapshot, selected_tree_context.as_ref());
    let focused_half = scene::split_focused_half(&frame.document, layout_state.split_focus());
    let visible: Vec<_> = super::workspace_composition::board_visible_parts(
        &frame.document,
        &frame.model.active_board_id,
    );
    let anchor = (!layout_state.assembly_3d()()).then(|| {
        scene::selection_geometry_anchor(
            &visible,
            &frame.scene,
            &frame.model.selected_part_ids,
            frame.svg.borrow().is_some(),
            frame.surface_size,
            frame.camera.width,
            frame.camera.height,
            frame.camera.view_x,
        )
    });
    let anchor = anchor.flatten();
    let objects_input = ObjectsInput {
        shared: frame.objects.clone(),
        matrix_setup: frame.matrix_setup.clone(),
        matrix_inspector: services.matrix_inspector.clone(),
        pair_created: layout_state.pair_created_selection(),
        on_place_component: frame.part_placement.on_place_component,
        layout_target: frame.layout_target,
        parts_query: frame.parts_query,
        on_browse_parts: frame.browse_parts,
        placement_error: frame.placement_error.clone(),
        add_object_menu_open: layout_state.add_object_menu_open(),
    };
    let inspector_input = inspector_wiring::layout_inspector_input(
        frame,
        &canvas,
        &mounts,
        &toolbar_wiring,
        editor.runtime(),
    );
    let canvas_element = scene::use_layout_canvas(
        frame,
        &canvas,
        &toolbar_wiring.toolbar_input.transform,
        &pointer,
        &overlay,
        scene::LayoutCanvasExtras {
            bridge,
            focused_half,
            anchor,
        },
    );
    let pills = canvas_status_pills(CanvasStatusPillsInput {
        active: true,
        part_placement: &frame.part_placement,
        document: frame.document.clone(),
        pending_splay_origin_pick: layout_state.pending_splay_origin_pick(),
        pending_anchor_pick: layout_state.pending_anchor_pick(),
        field_preview: use_context::<super::FieldPreview>().0,
        layout_assembly_3d: layout_state.assembly_3d(),
        canvas_anchor_badge: &canvas.selection.canvas_anchor_badge,
    });
    let commands = view_commands(
        toolbar_wiring.on_layout_view_mode,
        toolbar_wiring.toolbar_input.on_toggle_footprints,
        layout_state,
        editor.panels.objects_open,
        editor.panels.inspector_open,
        services.objects_panel_settings,
    );
    LayoutBundle {
        objects: objects_input,
        inspector: inspector_input,
        toolbar: toolbar_wiring.toolbar_input.clone(),
        canvas: rsx! {
            {canvas_element}
            {pills}
        },
        commands,
    }
}

#[component]
pub(super) fn LayoutWorkspace() -> Element {
    use super::workspace_composition::{WorkspaceSurfaces, use_frame, use_publish_surfaces};

    let frame = use_frame();
    let bundle = use_layout_bundle(&frame);
    let input = bundle.inspector.clone();
    let surfaces = WorkspaceSurfaces {
        objects: objects(bundle.objects.clone()),
        inspector: rsx! { LayoutInspector { input } },
        toolbar: toolbar(bundle.toolbar.clone()),
        canvas: bundle.canvas.clone(),
        status: rsx! {},
        layers: rsx! {},
    };
    boardstudio_web_ui_shared::ui::command::use_commands(bundle.commands.clone());
    use_publish_surfaces(Workspace::Layout, surfaces);
    rsx! {}
}

// --- Layout session hooks (run at editor-session lifetime) ---
//
// Only mounts with cross-workspace lifetime stay here: the Export objects dock
// reads `matrix_inspector`, session keyboard clears the aggregate pick on
// `matrix_transform_inspector`, and Parts/PCB share `matrix_setup`. Everything
// Layout-only (spacing ghost, board/key-size inspectors, align) runs inside
// `LayoutWorkspace` via `use_layout_mounts` and the canvas bundle.

pub(in crate::presentation) struct LayoutViewState {
    pub tree_cell_anchor: Rc<std::cell::RefCell<Option<super::OwnedTreeCellAnchor>>>,
    pub matrix_inspector: objects::MatrixInspectorMount,
    pub matrix_transform_inspector: objects::MatrixTransformInspectorMount,
    pub matrix_setup: objects::MatrixSetupMount,
    pub parts_state: super::parts_workspace::PartsWorkspaceState,
    pub parts_assembly_orientation: Signal<super::parts::SwitchOrientation>,
}

pub(in crate::presentation) fn use_layout_view_state(
    editor: boardstudio_web_ui_model::editor::EditorContext,
    adapter: &super::SelectionAdapter,
    layout_state: LayoutWorkspaceState,
) -> LayoutViewState {
    let runtime = editor.runtime();
    let boardstudio_web_ui_model::editor::EditorContext {
        version, workspace, ..
    } = editor;
    let tree_cell_anchor =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<super::OwnedTreeCellAnchor>)));
    let matrix_inspector = objects::use_matrix_inspector(runtime.clone(), adapter.selected_context);
    let matrix_transform_inspector = objects::use_workspace_matrix_transform(
        runtime.clone(),
        adapter.selected_context,
        layout_state.splay_affect(),
    );
    let matrix_setup = objects::use_matrix_setup(
        runtime.clone(),
        version,
        adapter.selected_context,
        adapter.anchor_scope,
        workspace,
        adapter.generation,
    );
    let parts_state = super::parts_workspace::use_parts_workspace_state();
    use_context_provider(|| parts_state);
    let parts_assembly_orientation = parts_state.assembly_orientation();
    LayoutViewState {
        tree_cell_anchor,
        matrix_inspector,
        matrix_transform_inspector,
        matrix_setup,
        parts_state,
        parts_assembly_orientation,
    }
}

/// Relations highlight and Inspector field-preview context providers. Both
/// are transient Layout view state, cleared by selection/workspace changes.
pub(in crate::presentation) fn use_layout_transient_highlights(
    adapter: &super::SelectionAdapter,
    model: &super::ReadModel,
    active_workspace: Workspace,
    resolved_document: Option<std::sync::Arc<boardstudio_core::model::ProjectDoc>>,
) -> Signal<Option<super::FieldPreviewState>> {
    // The Relations highlight is transient view state. A selection, workspace or board change
    // clears it; an accepted revision that drops its parts is handled by the canvas filter.
    let relation_highlight = use_signal(|| None::<Vec<String>>);
    use_context_provider(|| super::RelationHighlight(relation_highlight));
    let highlight_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
    );
    use_effect(use_reactive((&highlight_context,), move |(_,)| {
        super::RelationHighlight(relation_highlight).clear();
    }));
    // Hovering or focusing an Inspector number field shows its accepted geometry.
    // Leave, blur, unmount, selection changes, and accepted revisions clear it.
    let field_preview = use_signal(|| None::<super::FieldPreviewState>);
    use_context_provider(|| super::FieldPreview(field_preview));
    let field_preview_source =
        model
            .accepted
            .as_ref()
            .zip(resolved_document.as_ref())
            .map(|(snapshot, document)| super::FieldPreviewSource {
                document: document.clone(),
                scene: snapshot.scene.clone(),
            });
    use_context_provider(|| field_preview_source.clone());
    let field_preview_context = (
        adapter.selected_context.read().clone(),
        model.selected_part_ids.clone(),
        model.active_board_id.clone(),
        active_workspace,
        model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
    );
    use_effect(use_reactive((&field_preview_context,), move |(_,)| {
        super::FieldPreview(field_preview).clear();
    }));
    field_preview
}

/// Layout-only command-palette entries (2D / 3D assembly, Footprints,
/// Add object), registered by the session while Layout is active.
pub(in crate::presentation) fn view_commands(
    on_layout_view_mode: EventHandler<bool>,
    toggle_footprints: EventHandler<()>,
    layout_state: LayoutWorkspaceState,
    mut objects_open: Signal<bool>,
    mut inspect_open: Signal<bool>,
    mut objects_panel_settings: Signal<super::PanelSettings>,
) -> Vec<boardstudio_web_ui_shared::ui::command::Command> {
    let mut commands: Vec<_> = [("2D", false), ("3D assembly", true)]
        .into_iter()
        .map(
            |(label, mode)| boardstudio_web_ui_shared::ui::command::Command {
                group: "View",
                label: label.into(),
                reason: None,
                hint: None,
                on_select: EventHandler::new(move |()| on_layout_view_mode.call(mode)),
            },
        )
        .collect();
    commands.push(boardstudio_web_ui_shared::ui::command::Command {
        group: "View",
        label: "Footprints".into(),
        reason: None,
        hint: None,
        on_select: toggle_footprints,
    });
    let mut add_open = layout_state.add_object_menu_open();
    commands.push(boardstudio_web_ui_shared::ui::command::Command {
        group: "Current Workspace",
        label: "Add object".into(),
        reason: None,
        hint: None,
        on_select: EventHandler::new(move |()| {
            if super::view_helpers::is_compact_viewport() {
                inspect_open.set(false);
                objects_open.set(true);
            } else {
                let mut current = objects_panel_settings();
                current.mode = super::PanelMode::Pinned;
                objects_panel_settings.set(current);
            }
            add_open.set(true);
        }),
    });
    commands
}

/// Layout canvas-visibility layers dock, hosted at the foot of the Objects
/// panel while Layout owns the canvas.
pub(in crate::presentation) fn canvas_layers() -> Element {
    rsx! {
        super::CanvasLayers {
            initially_open: false,
            trigger_label: "Canvas visibility".to_owned(),
            trigger_id: String::from("m1-layers-trigger"),
            list_id: String::from("m1-layers-list"),
            groups: super::canvas_layers::layout_groups(),
        }
    }
}

/// Layout snap settings dock under the Objects panel.
pub(in crate::presentation) fn snap_dock(
    layout_state: LayoutWorkspaceState,
    layout_snap_settings: Signal<objects::LayoutSnapSettings>,
    layout_command_menu: Signal<Option<objects::LayoutCommandMenu>>,
    layout_snap_intent: EventHandler<objects::LayoutSnapIntent>,
) -> Element {
    rsx! {
        objects::LayoutSnapDock {
            settings: layout_snap_settings.read().clone(),
            open_menu: layout_command_menu,
            on_snap_intent: layout_snap_intent,
            handles_visible: layout_state.transform_handles()(),
            on_toggle_handles: EventHandler::new(move |visible| layout_state.transform_handles().set(visible)),
        }
    }
}

/// The Add-matrix surface floating over the Layout canvas.
pub(in crate::presentation) fn add_matrix_surface(
    matrix_setup: objects::MatrixSetupMount,
    mirrored_pair: objects::MirroredPairMount,
    guide_covering_single: bool,
) -> Element {
    rsx! {
        objects::AddMatrixSurface { matrix_setup, mirrored_pair, guide_covering_single }
    }
}

pub(in crate::presentation) struct CanvasStatusPillsInput<'a> {
    pub active: bool,
    pub part_placement: &'a super::part_placement::PartPlacementMount,
    pub document: std::sync::Arc<boardstudio_core::model::ProjectDoc>,
    pub pending_splay_origin_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub pending_anchor_pick: Signal<Option<objects::MatrixTransformInspectorOwner>>,
    pub field_preview: Signal<Option<super::FieldPreviewState>>,
    pub layout_assembly_3d: Signal<bool>,
    pub canvas_anchor_badge: &'a Option<super::anchor_badge::AnchorBadge>,
}

/// Transient Layout canvas status pills: placement, pending picks, the
/// field-preview hint and the "Placed from" anchor readout.
pub(in crate::presentation) fn canvas_status_pills(input: CanvasStatusPillsInput<'_>) -> Element {
    if !input.active {
        return rsx! {};
    }
    let placement_pill = input.part_placement.projection.as_ref().map(|placement| {
        let document = &input.document;
        rsx! {
            div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                {format!("Place {}", super::parts::placement_label(&placement.pending.definition))}
                if let Some(layout_name) = placement.owner.layout_id.as_ref()
                    .and_then(|id| document.layouts.iter().find(|layout| &layout.id == id))
                    .map(|layout| layout.name.as_str())
                {
                    " in {layout_name}"
                }
                " · Click or Enter to place · Esc cancels"
            }
        }
    });
    rsx! {
        {placement_pill}
        if (input.pending_splay_origin_pick)().is_some() {
            div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded-sm border border-border bg-card px-[9px] py-[7px] text-[10px] text-muted-foreground shadow-[0_4px_14px_#24373312]", role: "status",
                "Pick splay origin · Click the canvas · Esc cancels"
            }
        }
        if (input.pending_anchor_pick)().is_some() {
            div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] text-muted-foreground shadow-sm", role: "status",
                "Pick anchor key · Click a key · Esc cancels"
            }
        }
        // Ticket 16: the at-rest field preview names the keys the
        // hovered or focused Inspector number field would move.
        if let Some(state) = (input.field_preview)() {
            div { class: "pointer-events-none absolute right-[17px] bottom-[62px] flex items-center gap-2 rounded border border-border bg-card px-2 py-1.5 text-[10px] font-semibold text-warning shadow-sm", role: "status", "data-field-preview-hint": "",
                "{state.hint}"
            }
        }
        // Ticket 02: the "Placed from" readout (prototype variant
        // D) names what the selection is placed from at the
        // canvas's bottom left. It shares the canvas ring's badge
        // projection and clears with the selection.
        if !(input.layout_assembly_3d)()
            && let Some(badge) = input.canvas_anchor_badge.as_ref()
        {
            div { class: "pointer-events-none absolute bottom-[62px] left-3 flex items-center gap-1.5 rounded border border-border bg-card px-2 py-1.5 font-mono text-[11px] text-geometry-part shadow-sm", "data-canvas-anchor-readout": "",
                "Placed from {badge.label}"
            }
        }
    }
}
