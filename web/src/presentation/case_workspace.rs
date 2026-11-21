//! Case-owned tree and Inspector content. The page dispatcher supplies accepted
//! projections and callbacks; this module owns no Runtime/Session authority.
use super::{
    InstanceSelection, MechanicalSettings, MechanicalSettingsMount, MechanicalSettingsProps,
    case_controller::CaseBodyInspector,
    case_display::{is_inspector_visible, is_visible, preference_ids},
    case_viewer::{BodySelection, CaseSelection},
    objects::{ScopedTreeContext, TreeSelectRequest},
    selection::SelectionAdapter,
    shared_viewer::CaseDisplay,
};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Event, ReadModel, Scope, SelectionMode, SnapshotToken};
use boardstudio_core::model::{MechanicalGasketSupport, ProjectDoc};
pub(super) use boardstudio_web_case::case_selection::{SelectedPartSummary, selected_part_summary};
use boardstudio_web_ui_shared::choice_field::{ChoiceField, ChoiceFieldLayout};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    checkbox::{Checkbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{Field, FieldLabel},
    icons::ChevronDown,
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

/// State shared by Case's tree, Inspector, and viewer mount. The page supplies
/// current projections and routes actions; this handle owns Case selection.
#[derive(Clone, Copy)]
pub(super) struct CaseWorkspaceState {
    selection: CaseSelection,
    tree_expanded: Signal<BTreeSet<String>>,
}

pub(super) fn use_case_workspace_state() -> CaseWorkspaceState {
    let selection = CaseSelection {
        body: use_signal(|| None::<BodySelection>),
        layer: use_signal(|| None::<super::case_viewer::LayerSelection>),
        display: use_signal(std::collections::BTreeMap::new),
        body_edit_portal: super::case_viewer::CaseBodyEditPortal {
            dispatch: use_signal(|| None::<super::case_viewer::CaseBodyEditDispatch>),
            editable: use_signal(|| false),
        },
    };
    let state = CaseWorkspaceState {
        selection,
        tree_expanded: use_signal(BTreeSet::new),
    };
    use_context_provider(|| selection);
    state
}

impl CaseWorkspaceState {
    /// Selection interaction shared with the viewer and its Inspector.
    pub(super) fn selection(self) -> CaseSelection {
        self.selection
    }

    /// Tree expansion is edited by the mounted Case tree.
    pub(super) fn tree_expanded(self) -> Signal<BTreeSet<String>> {
        self.tree_expanded
    }

    /// Apply a tree action only while the captured Case owner is still current.
    pub(super) fn apply_tree_action(
        self,
        action: TreeAction,
        owner: &Admission,
        on_navigate: EventHandler<(Scope, String, Option<String>)>,
    ) {
        apply_tree_action(action, owner, on_navigate);
    }

    /// Persist a display change only after the captured Case owner is validated.
    pub(super) fn apply_display_request(self, request: DisplayRequest, owner: &Admission) {
        apply_display_request(request, owner);
    }
}

/// Case rows report intent to the page owner, which applies these changes to
/// the existing SelectionAdapter and CaseSelection signals.
#[derive(Clone)]
pub(super) enum TreeAction {
    SelectAssembly {
        scope: Scope,
        board_id: String,
        instance_id: Option<String>,
    },
    NavigateAssembly {
        scope: Scope,
        board_id: String,
        instance_id: Option<String>,
    },
    SelectBody {
        scope: Scope,
        body_id: String,
    },
    SelectLayer {
        scope: Scope,
        layer_id: String,
    },
    SelectGaskets {
        scope: Scope,
    },
    SelectPcb {
        scope: Scope,
    },
}

#[derive(Clone)]
pub(super) struct DisplayRequest {
    pub(super) target_scope: Scope,
    pub(super) id: String,
    pub(super) action: DisplayAction,
}

#[derive(Clone)]
pub(super) enum DisplayAction {
    ToggleVisibility,
    SetColor(String),
    ResetColor,
}

/// One owner identity shared by the tree and display callbacks. The request's
/// target Scope stays separate inside DisplayRequest for inactive-root rows.
#[derive(Clone)]
pub(super) struct Admission {
    pub(super) runtime: Rc<Runtime>,
    pub(super) adapter: SelectionAdapter,
    pub(super) case_state: CaseWorkspaceState,
    pub(super) instance_selection: InstanceSelection,
    pub(super) owner_scope: Scope,
    pub(super) owner_token: SnapshotToken,
    pub(super) owner_generation: u64,
}

pub(super) struct ObjectsInput<'a> {
    pub(super) model: &'a ReadModel,
    pub(super) scope: Option<Scope>,
    pub(super) instance_scope_pending: bool,
    /// Only the page owner may supply this scene for the current physical scope.
    /// This leaf keeps a completed same-scope scene visible across accepted edits,
    /// labeling it as previous geometry until current output replaces it.
    pub(super) scene: Option<Rc<CadScene>>,
    pub(super) selected_context: Signal<Option<ScopedTreeContext>>,
    pub(super) selected_body_id: Option<String>,
    pub(super) selected_layer_id: String,
    pub(super) case_selection: CaseSelection,
    pub(super) expanded: Signal<BTreeSet<String>>,
    pub(super) on_action: EventHandler<TreeAction>,
    pub(super) on_select: EventHandler<TreeSelectRequest>,
    pub(super) on_display: EventHandler<DisplayRequest>,
}

pub(super) struct CanvasInput {
    pub(super) generation_ready: bool,
    pub(super) instance_scope_pending: bool,
    pub(super) mechanical_settings: Option<MechanicalSettingsProps>,
}

pub(super) struct InspectorInput {
    pub(super) physical_setup: super::pcb_physical_setup::PhysicalSetupMount,
    pub(super) mechanical_settings: MechanicalSettingsMount,
    pub(super) instance_scope_pending: bool,
    pub(super) scope: Option<Scope>,
    pub(super) scene: Option<Rc<CadScene>>,
    pub(super) case_selection: CaseSelection,
    pub(super) selected_layer_id: String,
    pub(super) selected_context: Option<ScopedTreeContext>,
    pub(super) selected_part_summary: Option<SelectedPartSummary>,
    pub(super) on_show_configured_board: EventHandler<String>,
    pub(super) on_display: EventHandler<DisplayRequest>,
}

/// The page dispatcher may pass a completed same-scope result to the Case
/// Objects tree and Inspector after an accepted edit so they can retain
/// contextual selection. Mutation callbacks still validate the current token.
pub(super) fn workspace_display_scene(
    scene: Option<Rc<CadScene>>,
    scope: &Scope,
) -> Option<Rc<CadScene>> {
    scene.filter(|scene| {
        crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
            scene.exact,
            &scene.scope,
            scope,
        )
    })
}

#[derive(Clone)]
struct Row {
    id: String,
    label: String,
    detail: Option<String>,
    level: usize,
    expanded: Option<bool>,
    selectable: bool,
    selected: bool,
    action: Option<TreeAction>,
    part_context: Option<TreeSelectRequest>,
    visibility_id: Option<String>,
    visible: bool,
}

pub(super) fn objects(input: ObjectsInput<'_>) -> Element {
    if input.instance_scope_pending {
        return rsx! { p { role: "status", "Selecting physical assembly…" } };
    }
    let Some(snapshot) = input.model.accepted.as_ref() else {
        return rsx! { p { role: "status", "Open a saved keyboard to inspect its Case assembly." } };
    };
    let Some(scope) = input.scope.clone().filter(|scope| {
        scope.document_id == snapshot.document.id
            && scope.session_epoch == snapshot.session_epoch
            && scope.board_id == input.model.active_board_id
    }) else {
        return rsx! { p { role: "status", "The current Case scope is not ready." } };
    };
    let document = snapshot.document.as_ref();
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id);
    let Some(board) = board else {
        return rsx! { p { role: "alert", "The selected Case board is unavailable." } };
    };
    let selected_part_summary = input
        .selected_context
        .read()
        .as_ref()
        .filter(|selected| selected.scope == scope)
        .and_then(|selected| selected_part_summary(input.model, Some(selected)));
    let selection_title = selected_part_summary
        .as_ref()
        .map(|summary| summary.title.as_str())
        .unwrap_or(&board.name);
    let selection_breadcrumb = selected_part_summary
        .as_ref()
        .map(|summary| summary.breadcrumb.as_str())
        .unwrap_or("Case");
    let selection_count = selected_part_summary
        .as_ref()
        .map(|summary| format!("{} selected", summary.selected_count))
        .unwrap_or_else(|| "Select an object to edit".to_owned());
    let active_instance_id = scope.instance_id.clone();
    let active_root_id = active_instance_id
        .clone()
        .unwrap_or_else(|| scope.board_id.clone());
    let live_scene = input.scene.as_ref().filter(|scene| {
        crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
            scene.exact,
            &scene.scope,
            &scope,
        )
    });
    let showing_previous_geometry = live_scene.is_some_and(|scene| scene.token != snapshot.token);
    let mechanical = live_scene.and_then(|scene| scene.mechanical.as_ref());
    let active_body = input.selected_body_id.as_deref().filter(|id| {
        document
            .case_bodies
            .iter()
            .any(|body| body.id.as_str() == *id && body.board_id == scope.board_id)
    });
    let active_layer = if input.selected_layer_id == "pcb" {
        "pcb"
    } else if let Some(assembly) = mechanical {
        if assembly
            .stack
            .iter()
            .any(|layer| layer.id == input.selected_layer_id)
        {
            input.selected_layer_id.as_str()
        } else if input.selected_layer_id == "gaskets" && !assembly.gasket_supports.is_empty() {
            "gaskets"
        } else if input.selected_layer_id.starts_with("gasket:")
            && live_scene.is_some_and(|scene| {
                scene
                    .result
                    .bodies
                    .iter()
                    .any(|body| body.id == input.selected_layer_id)
            })
        {
            input.selected_layer_id.as_str()
        } else {
            ""
        }
    } else {
        ""
    };
    let current_context = input
        .selected_context
        .read()
        .clone()
        .filter(|selected| selected.scope == scope);
    let current_expanded = input.expanded.read().clone();
    let display = input.case_selection.display_value(&scope);
    let instances = case_roots(document);
    let board_part_ids: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let parts: Vec<_> = document
        .parts
        .iter()
        .filter(|part| board_part_ids.contains(part.id.as_str()))
        .collect();

    let mut rows = Vec::new();
    for instance in &instances {
        let root_scope =
            assembly_display_scope(&scope, &instance.board_id, instance.instance_id.clone());
        let root_display = input.case_selection.display_value(&root_scope);
        let root_key = format!("case-assembly:{}", instance.id);
        let active = instance.id == active_root_id && instance.board_id == scope.board_id;
        let collapsed = current_expanded.contains(&root_key);
        rows.push(Row {
            id: root_key.clone(),
            label: format!("{} case assembly", assembly_label(&instance.name)),
            detail: showing_previous_geometry.then(|| "Previous geometry".into()),
            level: 0,
            expanded: Some(active && !collapsed),
            selectable: true,
            selected: active && active_layer.is_empty(),
            action: Some(TreeAction::SelectAssembly {
                scope: scope.clone(),
                board_id: instance.board_id.clone(),
                instance_id: instance.instance_id.clone(),
            }),
            part_context: None,
            visibility_id: Some("Assembly".into()),
            visible: is_visible(&root_display, "Assembly"),
        });
        if !active || collapsed {
            continue;
        }

        if let Some(assembly) = mechanical {
            for layer in &assembly.stack {
                if layer.id == "pcb" || layer.id.starts_with("gasket:") {
                    continue;
                }
                rows.push(Row {
                    id: format!("case-generated:{}", layer.id),
                    label: stack_label(&layer.id),
                    detail: showing_previous_geometry.then(|| "Previous geometry".into()),
                    level: 1,
                    expanded: None,
                    selectable: true,
                    selected: active_layer == layer.id.as_str(),
                    action: Some(TreeAction::SelectLayer {
                        scope: scope.clone(),
                        layer_id: layer.id.clone(),
                    }),
                    part_context: None,
                    visibility_id: Some(layer.id.clone()),
                    visible: is_visible(&display, &layer.id),
                });
            }
            if !assembly.gasket_supports.is_empty() {
                let group_key = format!("case-gaskets:{}", scope.board_id);
                let open = current_expanded.contains(&group_key);
                rows.push(Row {
                    id: group_key.clone(),
                    label: "Gaskets".into(),
                    detail: Some(if showing_previous_geometry {
                        format!(
                            "{} pairs · Previous geometry",
                            assembly.gasket_supports.len()
                        )
                    } else {
                        format!("{} pairs", assembly.gasket_supports.len())
                    }),
                    level: 1,
                    expanded: Some(open),
                    selectable: true,
                    selected: active_layer == "gaskets",
                    action: Some(TreeAction::SelectGaskets {
                        scope: scope.clone(),
                    }),
                    part_context: None,
                    visibility_id: Some("gaskets".into()),
                    visible: is_visible(&display, "gaskets"),
                });
                if open {
                    for (index, support) in assembly.gasket_supports.iter().enumerate() {
                        let id = format!("gasket:{}:lower", support.id);
                        rows.push(Row {
                            id: format!("case-generated:{id}"),
                            label: format!("Gasket {} · {}", index + 1, gasket_side(support)),
                            detail: Some(format!(
                                "{} × {} mm{}",
                                support.length,
                                support.width,
                                if support.fit_error.is_some() {
                                    " · Does not fit"
                                } else {
                                    ""
                                }
                            )),
                            level: 2,
                            expanded: None,
                            selectable: live_scene.is_some_and(|scene| {
                                scene.result.bodies.iter().any(|body| body.id == id)
                            }),
                            selected: active_layer == id
                                || active_layer == format!("gasket:{}:upper", support.id),
                            action: Some(TreeAction::SelectLayer {
                                scope: scope.clone(),
                                layer_id: id.clone(),
                            }),
                            part_context: None,
                            visibility_id: Some(id.clone()),
                            visible: is_visible(&display, &id),
                        });
                    }
                }
            }
        } else {
            for body in document
                .case_bodies
                .iter()
                .filter(|body| body.board_id == scope.board_id)
            {
                rows.push(Row {
                    id: format!("case:{}", body.id),
                    label: body.name.clone(),
                    detail: Some(case_kind_label(&body.kind).into()),
                    level: 1,
                    expanded: None,
                    selectable: true,
                    selected: active_body == Some(body.id.as_str()),
                    action: Some(TreeAction::SelectBody {
                        scope: scope.clone(),
                        body_id: body.id.clone(),
                    }),
                    part_context: None,
                    visibility_id: Some(body.id.clone()),
                    visible: is_visible(&display, &body.id),
                });
            }
        }

        let pcb_key = format!("case-pcb:{}", instance.id);
        let pcb_open = current_expanded.contains(&pcb_key);
        rows.push(Row {
            id: pcb_key.clone(),
            label: "PCB".into(),
            detail: Some(format!("{} components", parts.len())),
            level: 1,
            expanded: Some(pcb_open),
            selectable: true,
            selected: active_layer == "pcb",
            action: Some(TreeAction::SelectPcb {
                scope: scope.clone(),
            }),
            part_context: None,
            visibility_id: Some("pcb".into()),
            visible: is_visible(&display, "pcb"),
        });
        if pcb_open {
            for part in &parts {
                let context = super::objects::context_for_part(input.model, &part.id);
                let request = context.map(|context| TreeSelectRequest {
                    scope: scope.clone(),
                    context,
                    mode: SelectionMode::Replace,
                    outline_action: None,
                });
                let selected = request.as_ref().is_some_and(|request| {
                    current_context
                        .as_ref()
                        .is_some_and(|current| current.context == request.context)
                });
                let detail = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .map(|definition| definition.name.clone())
                    .unwrap_or_else(|| "Component".into());
                rows.push(Row {
                    id: format!("Case:{}", part.id),
                    label: part.reference.clone(),
                    detail: Some(detail),
                    level: 2,
                    expanded: None,
                    selectable: request.is_some(),
                    selected,
                    action: None,
                    part_context: request,
                    visibility_id: Some(part.reference.clone()),
                    visible: is_visible(&display, &part.reference),
                });
            }
        }
    }

    let toggle_scope = input.expanded;
    let on_select = input.on_select;
    let on_action = input.on_action;
    let rows_for_render = rows;
    rsx! {
        aside { class: "flex min-h-0 min-w-0 flex-col border-r border-border bg-card max-[760px]:border-0", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "flex min-h-0 flex-1 flex-col",
                div { role: "tree", "aria-label": "Case assembly", class: "grid min-h-0 flex-1 content-start gap-0 overflow-auto overscroll-contain px-0 py-1",
                    for row in rows_for_render.iter().cloned() {
                        {
                            let row_key = row.id.clone();
                            let row_action = row.action.clone();
                            let part_request = row.part_context.clone();
                            let row_key_for_disclosure = row_key.clone();
                            let row_action_for_disclosure = row_action.clone();
                            let row_key_for_click = row_key.clone();
                            let row_action_for_click = row_action.clone();
                            let part_request_for_click = part_request.clone();
                            let row_key_for_key = row_key.clone();
                            let row_action_for_key = row_action.clone();
                            let part_request_for_key = part_request.clone();
                            let visibility_id = row.visibility_id.clone();
                            let visible = row.visible;
                            let selected = row.selected;
                            let row_level = row.level;
                            let row_expanded = row.expanded;
                            let row_is_disclosure = row.expanded.is_some();
                            let row_is_part = row.part_context.is_some();
                            let row_label = row.label.clone();
                            let label_id = format!("m1-object-tree-{}", row.id);
                            let expanded = toggle_scope;
                            let expanded_for_click = toggle_scope;
                            let expanded_for_key = toggle_scope;
                            let scope_for_click = scope.clone();
                            let scope_for_key = scope.clone();
                            let scope_for_visibility = row_action
                                .as_ref()
                                .and_then(|action| match action {
                                    TreeAction::SelectAssembly {
                                        scope,
                                        board_id,
                                        instance_id,
                                    } => Some(assembly_display_scope(
                                        scope,
                                        board_id,
                                        instance_id.clone(),
                                    )),
                                    _ => None,
                                })
                                .unwrap_or_else(|| scope.clone());
                            let display_handler = input.on_display;
                            let action_handler = on_action;
                            let select_handler = on_select;
                            let row_selectable = row.selectable;
                            let disclosure_label = format!("{} {}", if row.expanded == Some(true) { "Collapse" } else { "Expand" }, row.label);
                            rsx! {
                                div {
                                    key: "{row_key}", class: "flex min-h-10 min-w-0 items-center gap-0.5 pr-2",
                                    style: "padding-left: {8 + row_level * 14}px",
                                    role: "treeitem", "aria-labelledby": "{label_id}",
                                    "aria-level": "{row_level + 1}", "aria-selected": "{selected}",
                                    "aria-expanded": if row_is_disclosure { "{row_expanded == Some(true)}" },
                                    if row_is_disclosure {
                                        {
                                            let trigger_label = disclosure_label.clone();
                                            let disclosure_id = format!("{label_id}-disclosure");
                                            rsx! { Tooltip {
                                            TooltipTrigger {
                                                r#as: move |attributes: Vec<Attribute>| {
                                                    let row_action = row_action_for_disclosure.clone();
                                                    let row_key = row_key_for_disclosure.clone();
                                                    rsx! {
                                                        Button {
                                                            attributes,
                                                            variant: ButtonVariant::Ghost,
                                                            size: ButtonSize::IconSm,
                                                            id: disclosure_id.clone(),
                                                            class: "min-h-11 min-w-11",
                                                            aria_label: "{trigger_label}",
                                                            aria_expanded: row_expanded == Some(true),
                                                            onclick: move |_| {
                                                                if let Some(action) = inactive_assembly_navigation(&row_action) {
                                                                    action_handler.call(action);
                                                                } else {
                                                                    toggle_tree(expanded, &row_key);
                                                                }
                                                            },
                                                            ChevronDown { class: if row_expanded == Some(true) { "size-4" } else { "size-4 -rotate-90" } }
                                                        }
                                                    }
                                                },
                                            }
                                            TooltipContent { "{disclosure_label}" }
                                            } }
                                        }
                                    } else {
                                        span { class: "grid h-6 min-h-6 w-5 shrink-0 place-items-center border-0 bg-transparent p-0 text-muted-foreground", "aria-hidden": "true" }
                                    }
                                    Button { variant: ButtonVariant::Secondary,
                                        id: "{label_id}",
                                        class: if selected { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-[9px] rounded-[3px] border-primary bg-selected px-[5px] text-left text-selected-foreground [&_span]:text-selected-foreground" } else { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-[9px] rounded-[3px] border-transparent bg-transparent px-[5px] text-left hover:bg-accent" },
                                        disabled: !row_selectable,
                                        onclick: move |_| {
                                            if let Some(action) = row_action_for_click.clone() {
                                                action_handler.call(action);
                                            }
                                            if let Some(request) = part_request_for_click.clone() {
                                                action_handler.call(TreeAction::SelectPcb { scope: scope_for_click.clone() });
                                                select_handler.call(request);
                                            }
                                            if matches!(row_action_for_click.as_ref(), Some(TreeAction::SelectAssembly { .. }))
                                                && expanded_for_click.read().contains(&row_key_for_click)
                                            {
                                                toggle_tree(expanded_for_click, &row_key_for_click);
                                            }
                                        },
                                        onkeydown: move |event: KeyboardEvent| {
                                            let key = event.data().key().to_string();
                                            if (key == "Enter" || key == " ") && row_selectable {
                                                event.prevent_default();
                                                if let Some(action) = row_action_for_key.clone() { action_handler.call(action); }
                                                if let Some(request) = part_request_for_key.clone() {
                                                    action_handler.call(TreeAction::SelectPcb { scope: scope_for_key.clone() });
                                                    select_handler.call(request);
                                                }
                                                if matches!(row_action_for_key.as_ref(), Some(TreeAction::SelectAssembly { .. }))
                                                    && expanded_for_key.read().contains(&row_key_for_key)
                                                {
                                                    toggle_tree(expanded_for_key, &row_key_for_key);
                                                }
                                            }
                                        },
                                        span { class: "block size-[17px] shrink-0 text-muted-foreground", {if row_level == 0 { "▰" } else if row_is_part { "◇" } else { "▱" }} }
                                        span { class: "min-w-0 truncate text-base leading-[1.45] text-foreground", "{row_label}" }
                                        if let Some(detail) = row.detail { span { class: "ml-auto min-w-0 flex-[0_1_auto] truncate text-sm leading-[1.45] text-muted-foreground", "{detail}" } }
                                    }
                                    if let Some(id) = visibility_id {
                                        {
                                            let item_label = row_label.clone();
                                            rsx! { ToggleGroup {
                                            aria_label: "{row_label} visibility",
                                            pressed: Some(if visible { [0].into_iter().collect() } else { Default::default() }),
                                            on_pressed_change: move |next: std::collections::HashSet<usize>| {
                                                if next.contains(&0) != visible {
                                                    display_handler.call(DisplayRequest {
                                                        target_scope: scope_for_visibility.clone(),
                                                        id: id.clone(),
                                                        action: DisplayAction::ToggleVisibility,
                                                    });
                                                }
                                            },
                                            Tooltip {
                                                TooltipTrigger {
                                                    r#as: move |attributes: Vec<Attribute>| {
                                                        let label = item_label.clone();
                                                        rsx! {
                                                        ToggleItem { index: 0usize,
                                                            aria_label: if visible { "Hide {label}" } else { "Show {label}" },
                                                            class: "min-h-11 min-w-11",
                                                            attributes,
                                                            svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                                                path { d: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" }
                                                                circle { cx: "12", cy: "12", r: "3" }
                                                                if !visible { path { d: "m3 3 18 18" } }
                                                            }
                                                        }
                                                        }
                                                    },
                                                }
                                                TooltipContent { if visible { "Hide {row_label}" } else { "Show {row_label}" } }
                                            }
                                        } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "grid gap-1.5 border-t border-border p-2.5 text-sm [&_strong]:text-foreground [&_strong]:[overflow-wrap:anywhere] [&_span]:text-muted-foreground [&_span]:[overflow-wrap:anywhere]", "aria-label": "Current selection",
                strong { "{selection_title}" }
                span { "{selection_breadcrumb}" }
                span { "{selection_count}" }
            }
        }
    }
}

/// Apply a row intent through the existing root-owned Session and selection
/// authorities. The captured identity makes a late click from an old tree a
/// no-op after navigation, acceptance, or editor-generation changes.
pub(super) fn apply_tree_action(
    action: TreeAction,
    owner: &Admission,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
) {
    let runtime = &owner.runtime;
    let adapter = &owner.adapter;
    let mut case_selection = owner.case_state.selection();
    let instance_selection = owner.instance_selection;
    let expected_scope = &owner.owner_scope;
    let expected_token = owner.owner_token;
    let expected_generation = owner.owner_generation;
    let action_scope = match &action {
        TreeAction::SelectAssembly { scope, .. }
        | TreeAction::NavigateAssembly { scope, .. }
        | TreeAction::SelectBody { scope, .. }
        | TreeAction::SelectLayer { scope, .. }
        | TreeAction::SelectGaskets { scope }
        | TreeAction::SelectPcb { scope } => scope,
    };
    if action_scope != expected_scope
        || runtime.scope().as_ref() != Some(expected_scope)
        || (adapter.generation)() != expected_generation
    {
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref().filter(|snapshot| {
        snapshot.token == expected_token
            && snapshot.session_epoch == expected_scope.session_epoch
            && snapshot.document.id.as_str() == expected_scope.document_id.as_str()
            && model.active_board_id == expected_scope.board_id
            && model.active_instance_id.as_deref() == expected_scope.instance_id.as_deref()
    }) else {
        return;
    };
    if !instance_selection.is_current(&model) {
        return;
    }

    match action {
        TreeAction::SelectAssembly {
            scope,
            board_id,
            instance_id,
        } => {
            let root_exists = if let Some(instance_id) = instance_id.as_deref() {
                snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .any(|instance| instance.id == instance_id && instance.board_id == board_id)
                })
            } else {
                snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == board_id)
            };
            if !root_exists {
                return;
            }
            case_selection.layer.set(None);
            clear_tree_part_selection(runtime, adapter);
            on_navigate.call((scope, board_id, instance_id));
        }
        TreeAction::NavigateAssembly {
            scope,
            board_id,
            instance_id,
        } => {
            let root_exists = if let Some(instance_id) = instance_id.as_deref() {
                snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                    hardware
                        .instances
                        .iter()
                        .any(|instance| instance.id == instance_id && instance.board_id == board_id)
                })
            } else {
                snapshot
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == board_id)
            };
            if root_exists {
                on_navigate.call((scope, board_id, instance_id));
            }
        }
        TreeAction::SelectBody { scope, body_id } => {
            if !snapshot
                .document
                .case_bodies
                .iter()
                .any(|body| body.id == body_id && body.board_id == scope.board_id)
            {
                return;
            }
            case_selection
                .body
                .set(Some(BodySelection { scope, body_id }));
        }
        TreeAction::SelectLayer { scope, layer_id } => {
            let Some(scene) = runtime.cad_scene().filter(|scene| {
                crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                    scene.exact,
                    &scene.scope,
                    &scope,
                )
            }) else {
                return;
            };
            let Some(assembly) = scene.mechanical.as_ref() else {
                return;
            };
            let represented = if layer_id.starts_with("gasket:") {
                scene.result.bodies.iter().any(|body| body.id == layer_id)
            } else {
                assembly.stack.iter().any(|layer| layer.id == layer_id)
            };
            if !represented {
                return;
            }
            case_selection.select_layer(scope, layer_id);
            clear_tree_part_selection(runtime, adapter);
        }
        TreeAction::SelectGaskets { scope } => {
            let Some(scene) = runtime.cad_scene().filter(|scene| {
                crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                    scene.exact,
                    &scene.scope,
                    &scope,
                )
            }) else {
                return;
            };
            if !scene
                .mechanical
                .as_ref()
                .is_some_and(|assembly| !assembly.gasket_supports.is_empty())
            {
                return;
            }
            case_selection.select_layer(scope, "gaskets".into());
            clear_tree_part_selection(runtime, adapter);
        }
        TreeAction::SelectPcb { scope } => {
            case_selection.select_layer(scope, "pcb".into());
            clear_tree_part_selection(runtime, adapter);
        }
    }
}

/// Admit display changes against the live Editor owner before writing either
/// the CaseSelection signal or its existing local-storage preference. The
/// target may be another real assembly, but must belong to this accepted doc.
pub(super) fn apply_display_request(request: DisplayRequest, owner: &Admission) {
    let runtime = &owner.runtime;
    let adapter = &owner.adapter;
    let case_selection = owner.case_state.selection();
    let instance_selection = owner.instance_selection;
    let owner_scope = &owner.owner_scope;
    let owner_token = owner.owner_token;
    let owner_generation = owner.owner_generation;
    if runtime.scope().as_ref() != Some(owner_scope) || (adapter.generation)() != owner_generation {
        return;
    }
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref().filter(|snapshot| {
        snapshot.token == owner_token
            && snapshot.session_epoch == owner_scope.session_epoch
            && snapshot.document.id == owner_scope.document_id
            && model.active_board_id == owner_scope.board_id
            && model.active_instance_id.as_deref() == owner_scope.instance_id.as_deref()
    }) else {
        return;
    };
    if !instance_selection.is_current(&model)
        || !target_display_is_current(runtime, snapshot, &request.target_scope, &request.id)
    {
        return;
    }

    let mut display = case_selection.display_value(&request.target_scope);
    match request.action {
        DisplayAction::ToggleVisibility => {
            toggle_visibility_value(&mut display, &request.id);
        }
        DisplayAction::SetColor(color) => display.set_color(&request.id, &color),
        DisplayAction::ResetColor => display.set_color(&request.id, ""),
    }
    case_selection.save_display(&request.target_scope, display);
}

fn target_display_is_current(
    runtime: &Runtime,
    snapshot: &boardstudio_application::AcceptedSnapshot,
    target_scope: &Scope,
    id: &str,
) -> bool {
    if target_scope.session_epoch != snapshot.session_epoch
        || target_scope.document_id != snapshot.document.id
        || !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == target_scope.board_id)
    {
        return false;
    }
    let root_exists = target_scope
        .instance_id
        .as_deref()
        .is_none_or(|instance_id| {
            snapshot.document.hardware.as_ref().is_some_and(|hardware| {
                hardware.instances.iter().any(|instance| {
                    instance.id == instance_id && instance.board_id == target_scope.board_id
                })
            })
        });
    if !root_exists {
        return false;
    }
    if id == "Assembly" || id == "pcb" {
        return true;
    }
    if snapshot
        .document
        .case_bodies
        .iter()
        .any(|body| body.board_id == target_scope.board_id && body.id == id)
    {
        return true;
    }
    let board_parts: BTreeSet<&str> = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == target_scope.board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    if snapshot
        .document
        .parts
        .iter()
        .any(|part| board_parts.contains(part.id.as_str()) && part.reference == id)
    {
        return true;
    }
    runtime.cad_scene().is_some_and(|scene| {
        crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
            scene.exact,
            &scene.scope,
            target_scope,
        ) && (scene.mechanical.as_ref().is_some_and(|assembly| {
            assembly.stack.iter().any(|layer| layer.id == id)
                || (id == "gaskets" && !assembly.gasket_supports.is_empty())
        }) || scene.result.bodies.iter().any(|body| body.id == id))
    })
}

fn clear_tree_part_selection(runtime: &Rc<Runtime>, adapter: &SelectionAdapter) {
    let mut selected_context = adapter.selected_context;
    selected_context.set(None);
    let mut anchor_scope = adapter.anchor_scope;
    anchor_scope.set(None);
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: Vec::new(),
        range_part_ids: Vec::new(),
        mode: SelectionMode::Replace,
    });
}

fn inactive_assembly_navigation(action: &Option<TreeAction>) -> Option<TreeAction> {
    let Some(TreeAction::SelectAssembly {
        scope,
        board_id,
        instance_id,
    }) = action
    else {
        return None;
    };
    (scope.board_id.as_str() != board_id.as_str()
        || scope.instance_id.as_deref() != instance_id.as_deref())
    .then(|| TreeAction::NavigateAssembly {
        scope: scope.clone(),
        board_id: board_id.clone(),
        instance_id: instance_id.clone(),
    })
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    if input.instance_scope_pending {
        rsx! { p { role: "status", "Selecting physical assembly…" } }
    } else {
        rsx! { crate::cad_presentation::CasePanel {
            generation_ready: input.generation_ready,
            mechanical_settings: input.mechanical_settings,
        } }
    }
}

/// Presentation-only select; the caller owns the accepted value and domain request.
/// Short fixed lists take Select, long or filterable ones Combobox (mapping rule).
#[component]
fn CaseInspectorSelect(
    label: String,
    value: String,
    options: Vec<(String, String)>,
    #[props(default)] disabled: bool,
    on_change: EventHandler<Option<String>>,
) -> Element {
    rsx! {
        ChoiceField { label, value, options, disabled, layout: ChoiceFieldLayout::StackedNarrow,
            on_value_change: on_change,
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    let body_inspector_layer_id = input.selected_layer_id.clone();
    let generated = input
        .mechanical_settings
        .props
        .as_ref()
        .and_then(|props| {
            props
                .values
                .as_ref()
                .map(|values| values.board_id == props.identity.active_board_id)
        })
        .unwrap_or(false);
    let pcb_context_selected = input.selected_context.as_ref().is_some_and(|selected| {
        matches!(
            &selected.context,
            super::objects::TreeContext::Key { .. } | super::objects::TreeContext::Component { .. }
        )
    });
    let active_display_layer = input.scope.as_ref().and_then(|scope| {
        let id = body_inspector_layer_id.as_str();
        let valid = if id == "pcb" {
            true
        } else {
            input.scene.as_ref().is_some_and(|scene| {
                scene.exact
                    && scene.scope == *scope
                    && (scene.mechanical.as_ref().is_some_and(|assembly| {
                        assembly.stack.iter().any(|layer| layer.id == id)
                            || (id == "gaskets" && !assembly.gasket_supports.is_empty())
                    }) || (id.starts_with("gasket:")
                        && scene.result.bodies.iter().any(|body| body.id == id)))
            })
        };
        (generated && !id.is_empty() && valid).then(|| (scope.clone(), id.to_owned()))
    });
    let show_generated_note = active_display_layer.is_none() && !pcb_context_selected;
    let display = input
        .scope
        .as_ref()
        .map(|scope| input.case_selection.display_value(scope))
        .unwrap_or_default();
    let display_for_color = input.on_display;
    let display_for_reset = input.on_display;
    let display_for_visibility = input.on_display;
    let scope_for_color = active_display_layer
        .as_ref()
        .map(|(scope, _)| scope.clone());
    let scope_for_reset = scope_for_color.clone();
    let scope_for_visibility = scope_for_color.clone();
    let display_id_for_color = active_display_layer.as_ref().map(|(_, id)| id.clone());
    let display_id_for_reset = display_id_for_color.clone();
    let display_id_for_visibility = display_id_for_color.clone();
    let visible = active_display_layer
        .as_ref()
        .is_some_and(|(_, id)| is_inspector_visible(&display, id));
    let color = active_display_layer
        .as_ref()
        .and_then(|(_, id)| display.color(id))
        .unwrap_or("#b4bac2")
        .to_owned();
    rsx! {
        section { "aria-label": "Case Inspector",
            if input.instance_scope_pending {
                p { role: "status", "Selecting physical assembly…" }
            }
            if let Some(summary) = input.selected_part_summary.as_ref() {
                p { class: "m-0 px-3.5 pt-2.5 text-sm leading-normal text-muted-foreground", "aria-label": "Current selection",
                    "{summary.breadcrumb}"
                }
            }
            if !input.instance_scope_pending {
                section { class: "grid gap-2 border-b border-border py-2.5 [&>p]:m-0 [&>p]:text-sm [&>p]:leading-normal [&>p]:text-muted-foreground", "aria-label": "Physical assembly",
                    if let Some(instance) = input.physical_setup.projection.case_instance.clone() {
                        Collapsible { keep_mounted: true, default_open: false,
                            CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Assembly setup", "Assembly setup" }
                            CollapsibleContent { class: "data-[open=false]:hidden",
                            if input.physical_setup.projection.topology == boardstudio_core::model::HardwareTopology::Split {
                                CaseInspectorSelect {
                                    label: "Half connection",
                                    value: (match input.physical_setup.projection.transport { boardstudio_core::model::HardwareTransport::Wired => "wired", _ => "wireless" }).to_string(),
                                    options: vec![("wireless".to_owned(), "Wireless · local battery on each half".to_owned()), ("wired".to_owned(), "Wired serial · local power on each half".to_owned())],
                                    disabled: input.physical_setup.projection.busy,
                                    on_change: {
                                            let mount = input.physical_setup.clone();
                                            move |value: Option<String>| {
                                                let next = if value.as_deref() == Some("wired") { boardstudio_core::model::HardwareTransport::Wired } else { boardstudio_core::model::HardwareTransport::Wireless };
                                                mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseTransport(next));
                                            }
                                        },
                                }
                                if input.physical_setup.projection.transport == boardstudio_core::model::HardwareTransport::Wired {
                                    Callout { "Use a straight TRRS cable: tip and ring 2 carry crossed TX/RX, sleeve is ground, ring 1 is unused. Power both halves locally and unplug power before connecting." }
                                }
                            }
                            p { "{instance.name} · {instance.board_label} · {instance.role}" }
                            CaseInspectorSelect {
                                label: "PCB design",
                                value: instance.board_id.clone(),
                                options: input.physical_setup.projection.case_boards.iter().map(|board| (board.id.clone(), board.name.clone())).collect(),
                                disabled: input.physical_setup.projection.busy,
                                on_change: {
                                        let mount = input.physical_setup.clone();
                                        move |value: Option<String>| {
                                            if let Some(id) = value {
                                                mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CasePcbDesign(id))
                                            }
                                        }
                                    },
                            }
                            FieldLabel { class: "min-h-control-touch", html_for: format!("case-flip-{}", dioxus::core::current_scope_id().0),
                                Checkbox {
                                    id: format!("case-flip-{}", dioxus::core::current_scope_id().0),
                                    aria_label: "Turn PCB over for this half",
                                    disabled: input.physical_setup.projection.busy,
                                    checked: Some(if instance.flipped { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                                    on_checked_change: {
                                        let mount = input.physical_setup.clone();
                                        move |state: CheckboxState| mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseFlip(bool::from(state)))
                                    },
                                }
                                span { "Turn PCB over for this half" }
                            }
                            if instance.has_board_reference {
                                Callout { "Imported routing is a reference. Review it after changing the assembly." }
                            }
                            }
                        }
                    }
                    if input.selected_part_summary.is_some() && input.physical_setup.projection.case_instance.is_none() {
                        Callout { "Select a physical assembly in Objects to edit its PCB design and flip state." }
                    }
                    if let Some(feedback) = input.physical_setup.projection.feedback.clone() {
                        p { role: "status", "{feedback}" }
                    }
                }
                if let Some(props) = input.mechanical_settings.props {
                    MechanicalSettings { ..props }
                }
                if generated {
                    if active_display_layer.is_some() {
                section { "aria-label": "Part appearance",
                    h3 { "Display" }
                    Field {
                        Label { html_for: format!("case-part-colour-{}", dioxus::core::current_scope_id().0), "Colour" }
                        InputGroup {
                            InputGroupInput {
                                id: format!("case-part-colour-{}", dioxus::core::current_scope_id().0),
                                r#type: InputType::Color, aria_label: "Part colour", value: "{color}",
                                oninput: move |event: FormEvent| {
                                    if let (Some(scope), Some(id)) = (&scope_for_color, &display_id_for_color) {
                                        display_for_color.call(DisplayRequest {
                                            target_scope: scope.clone(),
                                            id: id.clone(),
                                            action: DisplayAction::SetColor(event.value()),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Button {
                        variant: ButtonVariant::Secondary, button_type: "button",
                        class: "max-[760px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                            onclick: move |_| {
                                if let (Some(scope), Some(id)) = (&scope_for_reset, &display_id_for_reset) {
                                    display_for_reset.call(DisplayRequest {
                                        target_scope: scope.clone(),
                                        id: id.clone(),
                                        action: DisplayAction::ResetColor,
                                    });
                                }
                        },
                        "Reset colour"
                    }
                    FieldLabel { class: "min-h-control-touch", html_for: format!("case-part-visible-{}", dioxus::core::current_scope_id().0),
                        Checkbox {
                            id: format!("case-part-visible-{}", dioxus::core::current_scope_id().0),
                            aria_label: "Visible",
                            checked: Some(if visible { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                            on_checked_change: move |_| {
                                if let (Some(scope), Some(id)) = (&scope_for_visibility, &display_id_for_visibility) {
                                    display_for_visibility.call(DisplayRequest {
                                        target_scope: scope.clone(),
                                        id: id.clone(),
                                        action: DisplayAction::ToggleVisibility,
                                    });
                                }
                            },
                        }
                        span { "Visible" }
                    }
                    }
                    }
                }
            }
            div { hidden: input.instance_scope_pending || (generated && !show_generated_note),
                CaseBodyInspector { on_show_configured_board: input.on_show_configured_board }
            }
        }
    }
}

fn case_roots(document: &ProjectDoc) -> Vec<CaseRoot> {
    let instances = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .map(|instance| CaseRoot {
                    id: instance.id.clone(),
                    instance_id: Some(instance.id.clone()),
                    board_id: instance.board_id.clone(),
                    name: instance.name.clone(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if instances.is_empty() {
        document
            .boards
            .iter()
            .map(|board| CaseRoot {
                id: board.id.clone(),
                instance_id: None,
                board_id: board.id.clone(),
                name: board.name.clone(),
            })
            .collect()
    } else {
        instances
    }
}

fn assembly_display_scope(scope: &Scope, board_id: &str, instance_id: Option<String>) -> Scope {
    Scope {
        session_epoch: scope.session_epoch,
        document_id: scope.document_id.clone(),
        board_id: board_id.to_owned(),
        instance_id,
    }
}

fn toggle_visibility_value(display: &mut CaseDisplay, id: &str) {
    let aliases = preference_ids(id);
    if aliases.iter().all(|alias| display.hidden.contains(alias)) {
        display.hidden.retain(|hidden| !aliases.contains(hidden));
    } else {
        for alias in aliases {
            if !display.hidden.contains(&alias) {
                display.hidden.push(alias);
            }
        }
    }
}

struct CaseRoot {
    id: String,
    instance_id: Option<String>,
    board_id: String,
    name: String,
}

fn assembly_label(name: &str) -> String {
    let trimmed = [" half", " PCB"]
        .into_iter()
        .find_map(|suffix| {
            name.get(name.len().checked_sub(suffix.len())?..)
                .filter(|tail| tail.eq_ignore_ascii_case(suffix))
                .map(|_| &name[..name.len() - suffix.len()])
        })
        .unwrap_or(name);
    let mut characters = trimmed.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}

fn stack_label(id: &str) -> String {
    match id {
        "plate" => "Plate".into(),
        "bottom" => "Bottom case".into(),
        "retainer" => "Top case".into(),
        "plate-foam" => "Plate foam".into(),
        "bottom-foam" => "Bottom foam".into(),
        "middle-frame" => "Middle frame".into(),
        "battery" => "Battery".into(),
        other => other.to_owned(),
    }
}

fn case_kind_label(kind: &boardstudio_core::model::CaseKind) -> &'static str {
    match kind {
        boardstudio_core::model::CaseKind::Plate => "Plate",
        boardstudio_core::model::CaseKind::Tray => "Tray",
        boardstudio_core::model::CaseKind::Lid => "Lid",
    }
}

fn gasket_side(support: &MechanicalGasketSupport) -> &'static str {
    if support.normal.x.abs() > support.normal.y.abs() {
        if support.normal.x > 0.0 {
            "Right"
        } else {
            "Left"
        }
    } else if support.normal.y > 0.0 {
        "Top"
    } else {
        "Bottom"
    }
}

fn toggle_tree(mut expanded: Signal<BTreeSet<String>>, id: &str) {
    let mut state = expanded.write();
    if !state.remove(id) {
        state.insert(id.to_owned());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_live_scene_tests;
