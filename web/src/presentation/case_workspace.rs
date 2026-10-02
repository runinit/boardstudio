//! Case-owned tree and Inspector content. The page dispatcher supplies accepted
//! projections and callbacks; this module owns no Runtime/Session authority.
use super::{
    InstanceSelection, MechanicalSettings, MechanicalSettingsMount,
    case_controller::CaseBodyInspector,
    case_display::preference_ids,
    case_viewer::{BodySelection, CaseSelection},
    objects::{ScopedTreeContext, TreeSelectRequest},
    selection::SelectionAdapter,
    shared_viewer::CaseDisplay,
};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Event, ReadModel, Scope, SelectionMode, SnapshotToken};
use boardstudio_core::model::{MechanicalGasketSupport, ProjectDoc};
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

pub(super) struct ObjectsInput<'a> {
    pub(super) model: &'a ReadModel,
    pub(super) scope: Option<Scope>,
    /// Only the page owner may supply this scene, after its normal scope/token
    /// check. This leaf rechecks the identity before displaying generated rows.
    pub(super) scene: Option<Rc<CadScene>>,
    pub(super) selected_context: Signal<Option<ScopedTreeContext>>,
    pub(super) selected_body_id: Option<String>,
    pub(super) selected_layer_id: String,
    pub(super) case_selection: CaseSelection,
    pub(super) expanded: Signal<BTreeSet<String>>,
    pub(super) on_action: EventHandler<TreeAction>,
    pub(super) on_select: EventHandler<TreeSelectRequest>,
    pub(super) on_navigate: EventHandler<(Scope, String, Option<String>)>,
}

pub(super) struct CanvasInput {
    pub(super) generation_ready: bool,
}

pub(super) struct InspectorInput {
    pub(super) mechanical_settings: MechanicalSettingsMount,
    pub(super) scope: Option<Scope>,
    pub(super) scene: Option<Rc<CadScene>>,
    pub(super) case_selection: CaseSelection,
    pub(super) selected_layer_id: String,
    pub(super) selected_context: Option<ScopedTreeContext>,
    pub(super) on_show_configured_board: EventHandler<String>,
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
    let active_instance_id = scope.instance_id.clone();
    let active_root_id = active_instance_id
        .clone()
        .unwrap_or_else(|| scope.board_id.clone());
    let live_scene = input
        .scene
        .as_ref()
        .filter(|scene| scene.exact && scene.scope == scope && scene.token == snapshot.token);
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
        let root_key = format!("case-assembly:{}", instance.id);
        let active = instance.id == active_root_id && instance.board_id == scope.board_id;
        let collapsed = current_expanded.contains(&root_key);
        rows.push(Row {
            id: root_key.clone(),
            label: format!("{} case assembly", assembly_label(&instance.name)),
            detail: None,
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
            visible: is_visible(&display, "Assembly"),
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
                    detail: None,
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
                    detail: Some(format!("{} pairs", assembly.gasket_supports.len())),
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

    let board_scope = scope.clone();
    let instance_scope = scope.clone();
    let toggle_scope = input.expanded;
    let on_select = input.on_select;
    let on_navigate = input.on_navigate;
    let on_action = input.on_action;
    let rows_for_render = rows;
    rsx! {
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "m1-object-navigation",
                label { "Board"
                    select { "aria-label": "Board", value: "{scope.board_id}", onchange: move |event: FormEvent| {
                        on_navigate.call((board_scope.clone(), event.value(), None));
                    },
                        for board in &document.boards {
                            option { key: "{board.id}", value: "{board.id}", "{board.name}" }
                        }
                    }
                }
                div { class: "m1-instance-selection", role: "group", "aria-label": "Physical instance",
                    span { "Physical instance" }
                    div { class: "m1-instance-choices",
                        for instance in instances.iter().filter(|instance| instance.board_id == scope.board_id) {
                            {
                                let id = instance.instance_id.clone().unwrap_or_else(|| instance.board_id.clone());
                                let selected = id == active_root_id;
                                let instance_id = instance.instance_id.clone();
                                let board_id = instance.board_id.clone();
                                let name = instance.name.clone();
                                let navigate = on_navigate;
                                let scope = instance_scope.clone();
                                rsx! {
                                    button {
                                        key: "{id}", r#type: "button", "aria-pressed": selected,
                                        onclick: move |_| navigate.call((scope.clone(), board_id.clone(), instance_id.clone())),
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "m1-object-tree",
                div { class: "m1-object-tree-heading", "{document.name}", span { "{parts.len()} parts" } }
                div { role: "tree", "aria-label": "Case assembly", class: "m1-component-list m1-object-tree-list",
                    for row in rows_for_render.iter().cloned() {
                        {
                            let row_key = row.id.clone();
                            let row_action = row.action.clone();
                            let part_request = row.part_context.clone();
                            let visibility_id = row.visibility_id.clone();
                            let visible = row.visible;
                            let case_selection = input.case_selection;
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
                            let scope_for_visibility = scope.clone();
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
                                                if let Some(action) = inactive_assembly_navigation(&row_action) {
                                                    action_handler.call(action);
                                                } else {
                                                    toggle_tree(expanded, &row_key);
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
                                            if let Some(action) = row_action.clone() {
                                                action_handler.call(action);
                                            }
                                            if let Some(request) = part_request.clone() {
                                                action_handler.call(TreeAction::SelectPcb { scope: scope_for_click.clone() });
                                                select_handler.call(request);
                                            }
                                            if matches!(row_action.as_ref(), Some(TreeAction::SelectAssembly { .. }))
                                                && expanded_for_click.read().contains(&row_key)
                                            {
                                                toggle_tree(expanded_for_click, &row_key);
                                            }
                                        },
                                        onkeydown: move |event: KeyboardEvent| {
                                            let key = event.data().key().to_string();
                                            if (key == "Enter" || key == " ") && row_selectable {
                                                event.prevent_default();
                                                if let Some(action) = row_action.clone() { action_handler.call(action); }
                                                if let Some(request) = part_request.clone() {
                                                    action_handler.call(TreeAction::SelectPcb { scope: scope_for_key.clone() });
                                                    select_handler.call(request);
                                                }
                                                if matches!(row_action.as_ref(), Some(TreeAction::SelectAssembly { .. }))
                                                    && expanded_for_key.read().contains(&row_key)
                                                {
                                                    toggle_tree(expanded_for_key, &row_key);
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
                                            "aria-label": "Toggle visibility for {row_label}",
                                            onclick: move |_| toggle_visibility(case_selection, &scope_for_visibility, &id),
                                            if visible { "Visible" } else { "Hidden" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Apply a row intent through the existing root-owned Session and selection
/// authorities. The captured identity makes a late click from an old tree a
/// no-op after navigation, acceptance, or editor-generation changes.
pub(super) fn apply_tree_action(
    action: TreeAction,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    case_selection: CaseSelection,
    instance_selection: InstanceSelection,
    expected_scope: &Scope,
    expected_token: SnapshotToken,
    expected_generation: u64,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
) {
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
                scene.exact && scene.scope == scope && scene.token == expected_token
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
                scene.exact && scene.scope == scope && scene.token == expected_token
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

fn clear_tree_part_selection(runtime: &Rc<Runtime>, adapter: &SelectionAdapter) {
    adapter.selected_context.set(None);
    adapter.anchor_scope.set(None);
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
    rsx! { crate::cad_presentation::CasePanel { generation_ready: input.generation_ready } }
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
    let display_for_color = input.case_selection;
    let display_for_reset = input.case_selection;
    let display_for_visibility = input.case_selection;
    let scope_for_color = active_display_layer
        .as_ref()
        .map(|(scope, _)| scope.clone());
    let scope_for_reset = scope_for_color.clone();
    let scope_for_visibility = scope_for_color;
    let display_id_for_color = active_display_layer.as_ref().map(|(_, id)| id.clone());
    let display_id_for_reset = display_id_for_color.clone();
    let display_id_for_visibility = display_id_for_color.clone();
    let visible = active_display_layer
        .as_ref()
        .is_some_and(|(_, id)| is_visible(&display, id));
    let color = active_display_layer
        .as_ref()
        .and_then(|(_, id)| display.color(id))
        .unwrap_or("#b4bac2")
        .to_owned();
    rsx! {
        section { class: "m1-case-inspector", "aria-label": "Case Inspector",
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
                                    let mut next = display_for_color.display_value(scope);
                                    next.set_color(id, &event.value());
                                    display_for_color.save_display(scope, next);
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            if let (Some(scope), Some(id)) = (&scope_for_reset, &display_id_for_reset) {
                                let mut next = display_for_reset.display_value(scope);
                                next.set_color(id, "");
                                display_for_reset.save_display(scope, next);
                            }
                        },
                        "Reset colour"
                    }
                    label {
                        input {
                            r#type: "checkbox", checked: visible,
                            onchange: move |_| {
                                if let (Some(scope), Some(id)) = (&scope_for_visibility, &display_id_for_visibility) {
                                    toggle_visibility(display_for_visibility, scope, id);
                                }
                            }
                        }
                        "Visible"
                    }
                }
                }
            }
            div { hidden: generated && !show_generated_note,
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

fn is_visible(display: &CaseDisplay, id: &str) -> bool {
    !preference_ids(id)
        .iter()
        .all(|alias| display.hidden.contains(alias))
}

fn toggle_visibility(selection: CaseSelection, scope: &Scope, id: &str) {
    let mut display = selection.display_value(scope);
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
    selection.save_display(scope, display);
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
