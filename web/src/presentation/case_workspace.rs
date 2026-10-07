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
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

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
    pub(super) case_selection: CaseSelection,
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
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "m1-object-tree",
                div { role: "tree", "aria-label": "Case assembly", class: "m1-component-list m1-object-tree-list",
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
                                    key: "{row_key}", class: "m1-tree-row",
                                    style: "padding-left: {8 + row_level * 14}px",
                                    role: "treeitem", "aria-labelledby": "{label_id}",
                                    "aria-level": "{row_level + 1}", "aria-selected": "{selected}",
                                    "aria-expanded": if row_is_disclosure { "{row_expanded == Some(true)}" },
                                    if row_is_disclosure {
                                        button {
                                            id: "{label_id}-disclosure", class: "m1-tree-disclosure",
                                            "aria-label": "{disclosure_label}", "aria-expanded": "{row_expanded == Some(true)}",
                                            onclick: move |_| {
                                                if let Some(action) = inactive_assembly_navigation(&row_action_for_disclosure) {
                                                    action_handler.call(action);
                                                } else {
                                                    toggle_tree(expanded, &row_key_for_disclosure);
                                                }
                                            },
                                            if row_expanded == Some(true) { "⌄" } else { "›" }
                                        }
                                    } else {
                                        span { class: "m1-tree-spacer", "aria-hidden": "true" }
                                    }
                                    button {
                                        id: "{label_id}",
                                        class: if selected { "m1-component selected" } else { "m1-component" },
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
                                        span { class: "m1-tree-glyph", {if row_level == 0 { "▰" } else if row_is_part { "◇" } else { "▱" }} }
                                        span { class: "m1-tree-label", "{row_label}" }
                                        if let Some(detail) = row.detail { span { class: "m1-object-kind", "{detail}" } }
                                    }
                                    if let Some(id) = visibility_id {
                                        button {
                                            class: "m1-object-visibility",
                                            r#type: "button",
                                            "aria-pressed": visible,
                                            "aria-label": if visible { "Hide {row_label}" } else { "Show {row_label}" },
                                            onclick: move |_| display_handler.call(DisplayRequest {
                                                target_scope: scope_for_visibility.clone(),
                                                id: id.clone(),
                                                action: DisplayAction::ToggleVisibility,
                                            }),
                                            svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                                path { d: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" }
                                                circle { cx: "12", cy: "12", r: "3" }
                                                if !visible { path { d: "m3 3 18 18" } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "m1-case-selection-footer", "aria-label": "Current selection",
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
    let mut case_selection = owner.case_selection;
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
    let case_selection = owner.case_selection;
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
        section { class: "m1-case-inspector", "aria-label": "Case Inspector",
            if input.instance_scope_pending {
                p { role: "status", "Selecting physical assembly…" }
            }
            if let Some(summary) = input.selected_part_summary.as_ref() {
                p { class: "m1-case-selection-breadcrumb", "aria-label": "Current selection",
                    "{summary.breadcrumb}"
                }
            }
            if !input.instance_scope_pending {
                section { class: "m1-case-physical-setup", "aria-label": "Physical assembly",
                    if input.selected_part_summary.is_none() {
                        p { "Mechanical settings and closure hardware apply to all case assemblies. Select an assembly in Objects." }
                    }
                    if let Some(instance) = input.physical_setup.projection.case_instance.clone() {
                        details {
                            summary { "Assembly setup" }
                            if input.physical_setup.projection.topology == boardstudio_core::model::HardwareTopology::Split {
                                label { "Half connection"
                                    select {
                                        "aria-label": "Half connection",
                                        value: match input.physical_setup.projection.transport { boardstudio_core::model::HardwareTransport::Wired => "wired", _ => "wireless" },
                                        disabled: input.physical_setup.projection.busy,
                                        onchange: {
                                            let mount = input.physical_setup.clone();
                                            move |event: FormEvent| {
                                                let next = if event.value() == "wired" { boardstudio_core::model::HardwareTransport::Wired } else { boardstudio_core::model::HardwareTransport::Wireless };
                                                mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseTransport(next));
                                            }
                                        },
                                        option { value: "wireless", "Wireless · local battery on each half" }
                                        option { value: "wired", "Wired serial · local power on each half" }
                                    }
                                }
                                if input.physical_setup.projection.transport == boardstudio_core::model::HardwareTransport::Wired {
                                    p { "Use a straight TRRS cable: tip and ring 2 carry crossed TX/RX, sleeve is ground, ring 1 is unused. Power both halves locally and unplug power before connecting." }
                                }
                            }
                            p { "{instance.name} · {instance.board_label} · {instance.role}" }
                            label { "PCB design"
                                select {
                                    "aria-label": "PCB design",
                                    value: "{instance.board_id}",
                                    disabled: input.physical_setup.projection.busy,
                                    onchange: {
                                        let mount = input.physical_setup.clone();
                                        move |event: FormEvent| mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CasePcbDesign(event.value()))
                                    },
                                    for board in input.physical_setup.projection.case_boards.iter() {
                                        option { value: "{board.id}", "{board.name}" }
                                    }
                                }
                            }
                            label {
                                input {
                                    r#type: "checkbox",
                                    "aria-label": "Turn PCB over for this half",
                                    checked: instance.flipped,
                                    disabled: input.physical_setup.projection.busy,
                                    onchange: {
                                        let mount = input.physical_setup.clone();
                                        move |event: FormEvent| mount.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseFlip(event.checked()))
                                    }
                                }
                                "Turn PCB over for this half"
                            }
                            if instance.has_board_reference {
                                p { "Imported routing is a reference. Review it after changing the assembly." }
                            }
                        }
                    }
                    if input.selected_part_summary.is_some() && input.physical_setup.projection.case_instance.is_none() {
                        p { "Select a physical assembly in Objects to edit its PCB design and flip state." }
                    }
                    if let Some(feedback) = input.physical_setup.projection.feedback.clone() {
                        p { role: "status", "{feedback}" }
                    }
                }
                if let Some(props) = input.mechanical_settings.props {
                    {MechanicalSettings(props)}
                }
                if generated {
                    if active_display_layer.is_some() {
                section { class: "m1-case-display", "aria-label": "Part appearance",
                    h3 { "Display" }
                    label { "Colour"
                        input {
                                r#type: "color", "aria-label": "Part colour", value: "{color}",
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
                    button {
                        r#type: "button",
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
                    label {
                        input {
                            r#type: "checkbox", checked: visible,
                                onchange: move |_| {
                                    if let (Some(scope), Some(id)) = (&scope_for_visibility, &display_id_for_visibility) {
                                        display_for_visibility.call(DisplayRequest {
                                            target_scope: scope.clone(),
                                            id: id.clone(),
                                            action: DisplayAction::ToggleVisibility,
                                        });
                                    }
                                }
                        }
                        "Visible"
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
mod mounted_live_scene_tests {
    use super::*;
    use crate::runtime::{firmware_export_test_support, project_name_test_support as support};
    use boardstudio_application::AcceptedSnapshot;
    use boardstudio_core::model::{
        Board, Finding, MechanicalAssembly, Scope as FindingScope, Severity,
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    async fn accepted_fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
        let runtime = support::new_runtime();
        let mut document = firmware_export_test_support::board_document();
        document.mechanical = Some(
            boardstudio_web_host::case_settings::initial_settings(&document, "main-board")
                .expect("test board has initial mechanical settings"),
        );
        support::open_document(&runtime, document).await;
        let accepted = runtime
            .model()
            .accepted
            .expect("the Case workspace project is accepted");
        let scope = runtime
            .scope()
            .expect("the accepted Case workspace project has a scope");
        let mechanical = serde_json::from_value::<MechanicalAssembly>(serde_json::json!({
            "suggestedMounts": [],
            "nominalPlateContours": [],
            "revision": accepted.document.revision,
            "plateContours": [],
            "case": {
                "revision": accepted.document.revision,
                "bodies": [{
                    "revision": accepted.document.revision,
                    "body": {
                        "id": "plate",
                        "name": "Plate body",
                        "boardId": scope.board_id,
                        "kind": "plate",
                        "thickness": 1.5,
                        "clearance": 0.2
                    },
                    "contours": []
                }]
            },
            "stack": [{ "id": "plate", "z": 0.0, "thickness": 1.5 }],
            "diagnostics": [{
                "id": "stale-case-finding",
                "severity": "error",
                "scope": "case",
                "message": "Stale diagnostic from previous geometry",
                "targetIds": ["plate"]
            }]
        }))
        .expect("minimal completed mechanical assembly fixture");
        runtime.set_cad_scene_test(Some(Rc::new(CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted.clone(),
            result: boardstudio_web_host::cad_jobs::CadResult {
                revision: accepted.document.revision,
                ..Default::default()
            },
            prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: accepted.document.revision,
                bodies: Vec::new(),
            },
            // Keep this cached scene stale after the accepted-token transition so the
            // display projection can retain it while the strict-current projection drops it.
            physical_fingerprint: Some([0xA5; 32]),
            mechanical: Some(mechanical),
            exact: true,
            contours: Vec::new(),
        })));
        (runtime, accepted, scope)
    }

    fn scene_with_current_finding(
        base: &Rc<CadScene>,
        accepted: AcceptedSnapshot,
        id: &str,
        message: &str,
    ) -> Rc<CadScene> {
        let mut scene = CadScene {
            scope: base.scope.clone(),
            token: base.token,
            snapshot: base.snapshot.clone(),
            result: base.result.clone(),
            prepared: base.prepared.clone(),
            physical_fingerprint: base.physical_fingerprint,
            mechanical: base.mechanical.clone(),
            exact: base.exact,
            contours: base.contours.clone(),
        };
        scene.token = accepted.token;
        scene.snapshot = accepted.clone();
        scene.result.revision = accepted.document.revision;
        scene.prepared.revision = accepted.document.revision;
        let assembly = scene
            .mechanical
            .as_mut()
            .expect("test scene has a mechanical assembly");
        assembly.diagnostics.clear();
        assembly.diagnostics.push(Finding {
            id: id.into(),
            severity: Severity::Error,
            scope: FindingScope::Case,
            message: message.into(),
            target_ids: vec!["plate".into()],
        });
        Rc::new(scene)
    }

    fn host() -> Element {
        let mut render_generation = use_signal(|| 0u64);
        let _ = render_generation();
        use_context_provider(|| render_generation);
        super::super::use_empty_test_instance_selection();
        let runtime = use_context::<Rc<Runtime>>();
        let instance_selection = use_context::<InstanceSelection>();
        let model = runtime.model();
        let scope = runtime.scope();
        let selected_body = use_signal(|| None::<BodySelection>);
        let selected_layer = use_signal(|| None);
        let display = use_signal(std::collections::BTreeMap::new);
        let body_edit_dispatch =
            use_signal(|| None::<super::super::case_viewer::CaseBodyEditDispatch>);
        let body_editable = use_signal(|| false);
        let case_selection = CaseSelection {
            body: selected_body,
            layer: selected_layer,
            display,
            body_edit_portal: super::super::case_viewer::CaseBodyEditPortal {
                dispatch: body_edit_dispatch,
                editable: body_editable,
            },
        };
        let initial_scope = scope.clone();
        let selection_for_hook = case_selection;
        use_hook({
            move || {
                if let Some(scope) = initial_scope {
                    selection_for_hook.select_layer(scope, "plate".into());
                }
            }
        });
        use_context_provider(|| case_selection);
        let selected_context = use_signal(|| None::<ScopedTreeContext>);
        let expanded = use_signal(std::collections::BTreeSet::new);
        let display_scene = scope
            .as_ref()
            .and_then(|scope| workspace_display_scene(runtime.cad_scene(), scope));
        let selected_layer_id = scope
            .as_ref()
            .map_or_else(String::new, |scope| case_selection.layer_id(scope));
        let tree = if model.accepted.is_some() {
            objects(ObjectsInput {
                model: &model,
                scope: scope.clone(),
                instance_scope_pending: false,
                scene: display_scene,
                selected_context,
                selected_body_id: None,
                selected_layer_id,
                case_selection,
                expanded,
                on_action: EventHandler::new(|_| {}),
                on_select: EventHandler::new(|_| {}),
                on_display: EventHandler::new(|_| {}),
            })
        } else {
            rsx! {}
        };
        let workspace = use_signal(|| "Case");
        let generation = use_signal(|| 1u64);
        let mut shown_finding = use_signal(String::new);
        let mechanical = super::super::mechanical_settings_mount::use_mechanical_settings_mount(
            runtime,
            generation,
            workspace,
            instance_selection,
            case_selection,
            EventHandler::new(move |request: super::super::mechanical_settings_mount::MechanicalFindingNavigation| shown_finding.set(request.finding_id)),
        );
        let inspector = mechanical.props.map(super::super::MechanicalSettings);
        rsx! {
            button {
                id: "case11-accepted-transition",
                onclick: move |_| render_generation += 1,
                "Advance accepted owner"
            }
            p { id: "case11-shown-finding", "{shown_finding}" }
            div { id: "case11-workspace-evidence", {tree} {inspector} }
        }
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(100).await;
    }

    async fn wait_for_current_resolution_inspector_text(document: &web_sys::Document) -> String {
        let current_resolution_finding =
            "Case body 'mechanical-frame-outline' requires at least one outer contour";
        let mut last_text = String::new();
        for _ in 0..50 {
            if let Some(inspector) = document
                .query_selector("#case11-workspace-evidence-root .m1-mechanical-settings")
                .unwrap()
            {
                last_text = inspector.text_content().unwrap_or_default();
                if last_text.contains(current_resolution_finding) {
                    return last_text;
                }
            }
            gloo_timers::future::TimeoutFuture::new(20).await;
        }
        panic!("current mechanical resolution did not reach the mounted Inspector: {last_text:?}");
    }

    #[wasm_bindgen_test]
    // The Objects tree retains and labels the stale same-scope scene. The Inspector
    // independently prefers an exact current mechanical resolution when it settles.
    async fn production_workspace_keeps_same_scope_layer_context_then_retires_it_on_owner_change() {
        let (runtime, original, scope) = accepted_fixture().await;
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("case11-workspace-evidence-root");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.into()),
        );
        settle().await;

        let mut edited = original.document.as_ref().clone();
        edited.name.push_str(" updated");
        let updated = support::replace_document(
            &runtime,
            "case-workspace-accepted-edit",
            &scope.board_id,
            edited,
        )
        .await;
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("case11-accepted-transition")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        // The opened project already resolved its settings once, so the Inspector text is
        // present before the accepted-owner render lands; let that render finish first.
        settle().await;

        let document = web_sys::window().unwrap().document().unwrap();
        let text = wait_for_current_resolution_inspector_text(&document).await;
        let root_text = document
            .get_element_by_id("case11-workspace-evidence-root")
            .unwrap()
            .text_content()
            .unwrap_or_default();
        let plate = document
            .get_element_by_id("m1-object-tree-case-generated:plate")
            .unwrap_or_else(|| {
                panic!("same-scope layer missing from production Objects tree: {root_text}")
            });
        assert!(
            plate
                .text_content()
                .unwrap_or_default()
                .contains("Previous geometry"),
            "the same-scope plate row is labelled previous; plate={:?}; root={root_text:?}",
            plate.text_content()
        );
        let row = plate.closest(".m1-tree-row").unwrap().unwrap();
        assert_eq!(row.get_attribute("aria-selected").as_deref(), Some("true"));
        assert!(
            !text.contains("previous generated geometry"),
            "the current resolved Inspector must not mark its layer previous; inspector={text:?}; root={root_text:?}"
        );
        assert!(text.contains("Resolved thickness 1.50 mm."));
        assert!(text.contains("Plate thickness"));
        assert!(!text.contains("Stale diagnostic from previous geometry"));
        assert_eq!(
            document
                .get_element_by_id("case11-shown-finding")
                .unwrap()
                .text_content()
                .as_deref(),
            Some("")
        );

        let (current_assembly, _) = runtime
            .resolve_mechanical_settings(
                updated.clone(),
                scope.clone(),
                updated.document.as_ref().clone(),
            )
            .await
            .expect("the accepted fixture has a current mechanical resolution");
        let current_finding = current_assembly
            .diagnostics
            .iter()
            .find(|finding| {
                finding.message.contains(
                    "Case body 'mechanical-frame-outline' requires at least one outer contour",
                )
            })
            .expect("the current resolution reports the fixture's missing outline");
        let stale_scene = runtime
            .cad_scene()
            .expect("same-scope completed geometry remains available for display");
        runtime.set_cad_scene_test(Some(scene_with_current_finding(
            &stale_scene,
            updated,
            "current-case-finding",
            "Current diagnostic for accepted geometry",
        )));
        document
            .get_element_by_id("case11-accepted-transition")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let inspector = document
            .query_selector("#case11-workspace-evidence-root .m1-mechanical-settings")
            .unwrap()
            .unwrap();
        let text = inspector.text_content().unwrap_or_default();
        assert!(text.contains(&current_finding.message));
        assert!(!text.contains("Current diagnostic for accepted geometry"));
        assert!(!text.contains("Stale diagnostic from previous geometry"));
        let rows = inspector.query_selector_all("li").unwrap();
        let finding_row = (0..rows.length())
            .filter_map(|index| rows.item(index))
            .find(|element| {
                element
                    .text_content()
                    .unwrap_or_default()
                    .contains(&current_finding.message)
            })
            .expect("current resolution finding is shown in the Inspector");
        let show_button = finding_row
            .dyn_into::<web_sys::Element>()
            .unwrap()
            .query_selector("button")
            .unwrap()
            .expect("current finding has a Show action");
        show_button
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert_eq!(
            document
                .get_element_by_id("case11-shown-finding")
                .unwrap()
                .text_content()
                .as_deref(),
            Some(current_finding.id.as_str())
        );

        // The accepted owner moves to another board: a real edit adds it and its mechanical
        // settings, then the Session navigates there. Navigating retires the cached geometry, so
        // the old board's scene is put back to show the display projection still ignores it.
        let mut with_other_board = runtime
            .model()
            .accepted
            .expect("the Case workspace project stays accepted")
            .document
            .as_ref()
            .clone();
        with_other_board.boards.push(Board {
            id: "other-board".into(),
            name: "Other board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        with_other_board.mechanical = Some(
            boardstudio_web_host::case_settings::initial_settings(&with_other_board, "other-board")
                .expect("replacement board has initial mechanical settings"),
        );
        support::replace_document(
            &runtime,
            "case-workspace-other-board",
            &scope.board_id,
            with_other_board,
        )
        .await;
        let previous_board_scene = runtime.cad_scene();
        support::navigate(&runtime, "other-board").await;
        assert_eq!(
            runtime.scope().map(|scope| scope.board_id),
            Some("other-board".to_owned())
        );
        runtime.set_cad_scene_test(previous_board_scene);
        document
            .get_element_by_id("case11-accepted-transition")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(
            document
                .get_element_by_id("m1-object-tree-case-generated:plate")
                .is_none()
        );
        let inspector = document
            .query_selector("#case11-workspace-evidence-root .m1-mechanical-settings")
            .unwrap()
            .unwrap();
        let text = inspector.text_content().unwrap_or_default();
        assert!(!text.contains("previous generated geometry"));
        assert!(!text.contains("Previous resolved stack"));
    }
}
