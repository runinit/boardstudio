//! Case-owned tree and Inspector content.
use super::{
    InstanceSelection, MechanicalSettings, MechanicalSettingsProps, MechanicalSettingsState,
    case_controller::CaseBodyInspector,
    case_display::{is_inspector_visible, is_visible, preference_ids},
    objects::{ScopedTreeContext, TreeSelectRequest},
    selection::SelectionAdapter,
    shared_viewer::CaseDisplay,
    shared_viewer::{BodySelection, CaseSelection},
};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{Event, ReadModel, Scope, SelectionMode, SnapshotToken};
use boardstudio_core::model::{
    HardwareTopology, HardwareTransport, MechanicalGasketSupport, ProjectDoc,
};
pub(super) use boardstudio_web_case::case_selection::{SelectedPartSummary, selected_part_summary};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, SelectField, TextField, ToggleField,
};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    icons::ChevronDown,
    input::InputType,
    toggle_group::{ToggleGroup, ToggleItem},
};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

#[derive(Clone, Copy)]
pub(super) struct CaseWorkspaceState {
    selection: CaseSelection,
    tree_expanded: Signal<BTreeSet<String>>,
}

pub(super) fn use_case_workspace_state() -> CaseWorkspaceState {
    let selection = CaseSelection {
        body: use_signal(|| None::<BodySelection>),
        layer: use_signal(|| None::<super::shared_viewer::LayerSelection>),
        display: use_signal(std::collections::BTreeMap::new),
        body_edit_portal: super::shared_viewer::CaseBodyEditPortal {
            dispatch: use_signal(|| None::<super::shared_viewer::CaseBodyEditDispatch>),
            editable: use_signal(|| false),
        },
    };
    use_context_provider(|| selection);
    CaseWorkspaceState {
        selection,
        tree_expanded: use_signal(BTreeSet::new),
    }
}

impl CaseWorkspaceState {
    pub(super) fn selection(self) -> CaseSelection {
        self.selection
    }
    pub(super) fn tree_expanded(self) -> Signal<BTreeSet<String>> {
        self.tree_expanded
    }
    pub(super) fn apply_tree_action(
        self,
        action: TreeAction,
        owner: &Admission,
        on_navigate: EventHandler<(Scope, String, Option<String>)>,
    ) {
        apply_tree_action(action, owner, on_navigate);
    }
    pub(super) fn apply_display_request(self, request: DisplayRequest, owner: &Admission) {
        apply_display_request(request, owner);
    }
}

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
    pub(super) mechanical_settings: MechanicalSettingsState,
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

pub(super) fn workspace_display_scene(
    scene: Option<Rc<CadScene>>,
    scope: &Scope,
) -> Option<Rc<CadScene>> {
    scene.filter(|s| {
        crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
            s.exact, &s.scope, scope,
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
    let Some(snap) = input.model.accepted.as_ref() else {
        return rsx! { p { role: "status", "Open a saved keyboard to inspect its Case assembly." } };
    };
    let Some(scope) = input.scope.clone().filter(|s| {
        s.document_id == snap.document.id
            && s.session_epoch == snap.session_epoch
            && s.board_id == input.model.active_board_id
    }) else {
        return rsx! { p { role: "status", "The current Case scope is not ready." } };
    };
    let doc = snap.document.as_ref();
    let Some(board) = doc.boards.iter().find(|b| b.id == scope.board_id) else {
        return rsx! { p { role: "alert", "The selected Case board is unavailable." } };
    };
    let sel_summary = input
        .selected_context
        .read()
        .as_ref()
        .filter(|s| s.scope == scope)
        .and_then(|s| selected_part_summary(input.model, Some(s)));
    let active_root_id = scope
        .instance_id
        .clone()
        .unwrap_or_else(|| scope.board_id.clone());
    let live_scene = input.scene.as_ref().filter(|s| {
        crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
            s.exact, &s.scope, &scope,
        )
    });
    let prev_geom = live_scene.is_some_and(|s| s.token != snap.token);
    let mechanical = live_scene.and_then(|s| s.mechanical.as_ref());
    let active_body = input.selected_body_id.as_deref().filter(|id| {
        doc.case_bodies
            .iter()
            .any(|b| b.id == *id && b.board_id == scope.board_id)
    });
    let active_layer = if input.selected_layer_id == "pcb" {
        "pcb"
    } else if let Some(a) = mechanical {
        if a.stack.iter().any(|l| l.id == input.selected_layer_id)
            || (input.selected_layer_id == "gaskets" && !a.gasket_supports.is_empty())
            || (input.selected_layer_id.starts_with("gasket:")
                && live_scene.is_some_and(|s| {
                    s.result
                        .bodies
                        .iter()
                        .any(|b| b.id == input.selected_layer_id)
                }))
        {
            input.selected_layer_id.as_str()
        } else {
            ""
        }
    } else {
        ""
    };
    let cur_ctx = input
        .selected_context
        .read()
        .clone()
        .filter(|s| s.scope == scope);
    let expanded_set = input.expanded.read().clone();
    let display = input.case_selection.display_value(&scope);
    let board_parts: BTreeSet<&str> = board.part_ids.iter().map(String::as_str).collect();
    let parts: Vec<_> = doc
        .parts
        .iter()
        .filter(|p| board_parts.contains(p.id.as_str()))
        .collect();

    let mut rows = Vec::new();
    for inst in case_roots(doc) {
        let root_scope = assembly_display_scope(&scope, &inst.board_id, inst.instance_id.clone());
        let root_disp = input.case_selection.display_value(&root_scope);
        let root_key = format!("case-assembly:{}", inst.id);
        let active = inst.id == active_root_id && inst.board_id == scope.board_id;
        let collapsed = expanded_set.contains(&root_key);
        rows.push(Row {
            id: root_key.clone(),
            label: format!("{} case assembly", assembly_label(&inst.name)),
            detail: prev_geom.then(|| "Previous geometry".into()),
            level: 0,
            expanded: Some(active && !collapsed),
            selectable: true,
            selected: active && active_layer.is_empty(),
            action: Some(TreeAction::SelectAssembly {
                scope: scope.clone(),
                board_id: inst.board_id.clone(),
                instance_id: inst.instance_id.clone(),
            }),
            part_context: None,
            visibility_id: Some("Assembly".into()),
            visible: is_visible(&root_disp, "Assembly"),
        });
        if !active || collapsed {
            continue;
        }
        if let Some(a) = mechanical {
            for l in a
                .stack
                .iter()
                .filter(|l| l.id != "pcb" && !l.id.starts_with("gasket:"))
            {
                rows.push(Row {
                    id: format!("case-generated:{}", l.id),
                    label: stack_label(&l.id),
                    detail: prev_geom.then(|| "Previous geometry".into()),
                    level: 1,
                    expanded: None,
                    selectable: true,
                    selected: active_layer == l.id,
                    action: Some(TreeAction::SelectLayer {
                        scope: scope.clone(),
                        layer_id: l.id.clone(),
                    }),
                    part_context: None,
                    visibility_id: Some(l.id.clone()),
                    visible: is_visible(&display, &l.id),
                });
            }
            if !a.gasket_supports.is_empty() {
                let gk = format!("case-gaskets:{}", scope.board_id);
                let open = expanded_set.contains(&gk);
                rows.push(Row {
                    id: gk,
                    label: "Gaskets".into(),
                    detail: Some(format!("{} pairs", a.gasket_supports.len())),
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
                    for (idx, g) in a.gasket_supports.iter().enumerate() {
                        let gid = format!("gasket:{}:lower", g.id);
                        rows.push(Row {
                            id: format!("case-generated:{gid}"),
                            label: format!("Gasket {} · {}", idx + 1, gasket_side(g)),
                            detail: Some(format!("{} × {} mm", g.length, g.width)),
                            level: 2,
                            expanded: None,
                            selectable: live_scene
                                .is_some_and(|s| s.result.bodies.iter().any(|b| b.id == gid)),
                            selected: active_layer == gid
                                || active_layer == format!("gasket:{}:upper", g.id),
                            action: Some(TreeAction::SelectLayer {
                                scope: scope.clone(),
                                layer_id: gid.clone(),
                            }),
                            part_context: None,
                            visibility_id: Some(gid.clone()),
                            visible: is_visible(&display, &gid),
                        });
                    }
                }
            }
        } else {
            for b in doc
                .case_bodies
                .iter()
                .filter(|b| b.board_id == scope.board_id)
            {
                rows.push(Row {
                    id: format!("case:{}", b.id),
                    label: b.name.clone(),
                    detail: Some(case_kind_label(&b.kind).into()),
                    level: 1,
                    expanded: None,
                    selectable: true,
                    selected: active_body == Some(b.id.as_str()),
                    action: Some(TreeAction::SelectBody {
                        scope: scope.clone(),
                        body_id: b.id.clone(),
                    }),
                    part_context: None,
                    visibility_id: Some(b.id.clone()),
                    visible: is_visible(&display, &b.id),
                });
            }
        }
        let pcb_key = format!("case-pcb:{}", inst.id);
        let pcb_open = expanded_set.contains(&pcb_key);
        rows.push(Row {
            id: pcb_key,
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
            for p in &parts {
                let req = super::objects::context_for_part(input.model, &p.id).map(|context| {
                    TreeSelectRequest {
                        scope: scope.clone(),
                        context,
                        mode: SelectionMode::Replace,
                        outline_action: None,
                    }
                });
                let sel = req
                    .as_ref()
                    .is_some_and(|r| cur_ctx.as_ref().is_some_and(|c| c.context == r.context));
                let detail = doc
                    .definitions
                    .iter()
                    .find(|d| d.id == p.definition_id)
                    .map_or("Component", |d| d.name.as_str())
                    .to_owned();
                rows.push(Row {
                    id: format!("Case:{}", p.id),
                    label: p.reference.clone(),
                    detail: Some(detail),
                    level: 2,
                    expanded: None,
                    selectable: req.is_some(),
                    selected: sel,
                    action: None,
                    part_context: req,
                    visibility_id: Some(p.reference.clone()),
                    visible: is_visible(&display, &p.reference),
                });
            }
        }
    }

    let (toggle_scope, on_select, on_action, on_display) = (
        input.expanded,
        input.on_select,
        input.on_action,
        input.on_display,
    );
    rsx! {
        aside { class: "flex min-h-0 min-w-0 flex-col border-r border-border bg-card max-[760px]:border-0", "aria-label": "Objects",
            header { h2 { "Objects" } }
            div { class: "flex min-h-0 flex-1 flex-col",
                div { role: "tree", "aria-label": "Case assembly", class: "grid min-h-0 flex-1 content-start gap-0 overflow-auto overscroll-contain px-0 py-1",
                    for row in rows {
                        {
                            let (rk, ra, pr, vis_id, vis, sel) = (row.id.clone(), row.action.clone(), row.part_context.clone(), row.visibility_id.clone(), row.visible, row.selected);
                            let label_id = format!("m1-object-tree-{rk}");
                            let vis_scope = ra.as_ref().and_then(|a| match a {
                                TreeAction::SelectAssembly { scope, board_id, instance_id } => Some(assembly_display_scope(scope, board_id, instance_id.clone())),
                                _ => None,
                            }).unwrap_or_else(|| scope.clone());
                            let (rk_d, ra_d, rk_c, ra_c, pr_c, sc_c) = (rk.clone(), ra.clone(), rk.clone(), ra, pr, scope.clone());
                            rsx! {
                                div { key: "{rk}", class: "flex min-h-10 min-w-0 items-center gap-0.5 pr-2", style: "padding-left: {8 + row.level * 14}px",
                                    role: "treeitem", "aria-labelledby": "{label_id}", "aria-level": "{row.level + 1}", "aria-selected": "{sel}",
                                    "aria-expanded": if let Some(e) = row.expanded { "{e}" },
                                    if let Some(exp) = row.expanded {
                                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::IconSm, id: format!("{label_id}-disclosure"), class: "min-h-11 min-w-11",
                                            aria_label: format!("{} {}", if exp { "Collapse" } else { "Expand" }, row.label), aria_expanded: exp,
                                            onclick: move |_| {
                                                if let Some(a) = inactive_assembly_navigation(&ra_d) { on_action.call(a); }
                                                else { toggle_tree(toggle_scope, &rk_d); }
                                            },
                                            ChevronDown { class: if exp { "size-4" } else { "size-4 -rotate-90" } }
                                        }
                                    } else {
                                        span { class: "grid h-6 w-5 shrink-0", "aria-hidden": "true" }
                                    }
                                    Button { variant: ButtonVariant::Secondary, id: "{label_id}", disabled: !row.selectable,
                                        class: if sel { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-2 rounded-[3px] border-primary bg-selected px-1.5 text-left text-selected-foreground" } else { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-2 rounded-[3px] border-transparent bg-transparent px-1.5 text-left hover:bg-accent" },
                                        onclick: move |_| {
                                            if let Some(a) = ra_c.clone() { on_action.call(a); }
                                            if let Some(r) = pr_c.clone() { on_action.call(TreeAction::SelectPcb { scope: sc_c.clone() }); on_select.call(r); }
                                            if matches!(ra_c.as_ref(), Some(TreeAction::SelectAssembly { .. })) && toggle_scope.read().contains(&rk_c) { toggle_tree(toggle_scope, &rk_c); }
                                        },
                                        span { class: "min-w-0 truncate text-base text-foreground", "{row.label}" }
                                        if let Some(d) = row.detail { span { class: "ml-auto truncate text-sm text-muted-foreground", "{d}" } }
                                    }
                                    if let Some(id) = vis_id {
                                        ToggleGroup { aria_label: "{row.label} visibility",
                                            pressed: Some(if vis { [0].into_iter().collect() } else { Default::default() }),
                                            on_pressed_change: move |next: std::collections::HashSet<usize>| if next.contains(&0) != vis {
                                                on_display.call(DisplayRequest { target_scope: vis_scope.clone(), id: id.clone(), action: DisplayAction::ToggleVisibility });
                                            },
                                            ToggleItem { index: 0usize, aria_label: if vis { "Hide {row.label}" } else { "Show {row.label}" }, class: "min-h-11 min-w-11",
                                                svg { view_box: "0 0 24 24", "aria-hidden": "true", path { d: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" } circle { cx: "12", cy: "12", r: "3" } if !vis { path { d: "m3 3 18 18" } } }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "grid gap-1 border-t border-border p-2.5 text-sm", "aria-label": "Current selection",
                strong { {sel_summary.as_ref().map_or(board.name.as_str(), |s| s.title.as_str())} }
                span { class: "text-muted-foreground", {sel_summary.as_ref().map_or("Case", |s| s.breadcrumb.as_str())} }
            }
        }
    }
}

fn validate_owner(owner: &Admission) -> Option<boardstudio_application::AcceptedSnapshot> {
    if owner.runtime.scope().as_ref() != Some(&owner.owner_scope)
        || (owner.adapter.generation)() != owner.owner_generation
    {
        return None;
    }
    let model = owner.runtime.model();
    if !owner.instance_selection.is_current(&model) {
        return None;
    }
    model.accepted.filter(|s| {
        s.token == owner.owner_token
            && s.session_epoch == owner.owner_scope.session_epoch
            && s.document.id == owner.owner_scope.document_id
            && model.active_board_id == owner.owner_scope.board_id
            && model.active_instance_id.as_deref() == owner.owner_scope.instance_id.as_deref()
    })
}

fn root_exists(doc: &ProjectDoc, board_id: &str, instance_id: Option<&str>) -> bool {
    if let Some(iid) = instance_id {
        doc.hardware.as_ref().is_some_and(|h| {
            h.instances
                .iter()
                .any(|i| i.id == iid && i.board_id == board_id)
        })
    } else {
        doc.boards.iter().any(|b| b.id == board_id)
    }
}

pub(super) fn apply_tree_action(
    action: TreeAction,
    owner: &Admission,
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
    if action_scope != &owner.owner_scope {
        return;
    }
    let Some(snap) = validate_owner(owner) else {
        return;
    };
    let mut cs = owner.case_state.selection();
    match action {
        TreeAction::SelectAssembly {
            scope,
            board_id,
            instance_id,
        } if root_exists(&snap.document, &board_id, instance_id.as_deref()) => {
            cs.layer.set(None);
            clear_tree_part_selection(&owner.runtime, &owner.adapter);
            on_navigate.call((scope, board_id, instance_id));
        }
        TreeAction::NavigateAssembly {
            scope,
            board_id,
            instance_id,
        } if root_exists(&snap.document, &board_id, instance_id.as_deref()) => {
            on_navigate.call((scope, board_id, instance_id));
        }
        TreeAction::SelectBody { scope, body_id }
            if snap
                .document
                .case_bodies
                .iter()
                .any(|b| b.id == body_id && b.board_id == scope.board_id) =>
        {
            cs.body.set(Some(BodySelection { scope, body_id }));
        }
        TreeAction::SelectLayer { scope, layer_id } => {
            let ok = owner.runtime.cad_scene().is_some_and(|s| {
                crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                    s.exact, &s.scope, &scope,
                ) && if layer_id.starts_with("gasket:") {
                    s.result.bodies.iter().any(|b| b.id == layer_id)
                } else {
                    s.mechanical
                        .as_ref()
                        .is_some_and(|a| a.stack.iter().any(|l| l.id == layer_id))
                }
            });
            if ok {
                cs.select_layer(scope, layer_id);
                clear_tree_part_selection(&owner.runtime, &owner.adapter);
            }
        }
        TreeAction::SelectGaskets { scope } => {
            if owner.runtime.cad_scene().is_some_and(|s| {
                crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                    s.exact, &s.scope, &scope,
                ) && s
                    .mechanical
                    .as_ref()
                    .is_some_and(|a| !a.gasket_supports.is_empty())
            }) {
                cs.select_layer(scope, "gaskets".into());
                clear_tree_part_selection(&owner.runtime, &owner.adapter);
            }
        }
        TreeAction::SelectPcb { scope } => {
            cs.select_layer(scope, "pcb".into());
            clear_tree_part_selection(&owner.runtime, &owner.adapter);
        }
        _ => {}
    }
}

pub(super) fn apply_display_request(request: DisplayRequest, owner: &Admission) {
    let Some(snap) = validate_owner(owner) else {
        return;
    };
    if request.target_scope.session_epoch != snap.session_epoch
        || request.target_scope.document_id != snap.document.id
        || !root_exists(
            &snap.document,
            &request.target_scope.board_id,
            request.target_scope.instance_id.as_deref(),
        )
    {
        return;
    }
    let cs = owner.case_state.selection();
    let mut display = cs.display_value(&request.target_scope);
    match request.action {
        DisplayAction::ToggleVisibility => toggle_visibility_value(&mut display, &request.id),
        DisplayAction::SetColor(color) => display.set_color(&request.id, &color),
        DisplayAction::ResetColor => display.set_color(&request.id, ""),
    }
    cs.save_display(&request.target_scope, display);
}

fn clear_tree_part_selection(runtime: &Rc<Runtime>, adapter: &SelectionAdapter) {
    let (mut sc, mut as_) = (adapter.selected_context, adapter.anchor_scope);
    sc.set(None);
    as_.set(None);
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec![],
        range_part_ids: vec![],
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
    (scope.board_id != *board_id || scope.instance_id != *instance_id).then(|| {
        TreeAction::NavigateAssembly {
            scope: scope.clone(),
            board_id: board_id.clone(),
            instance_id: instance_id.clone(),
        }
    })
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(input: CanvasInput) -> Element {
    if input.instance_scope_pending {
        rsx! { p { role: "status", "Selecting physical assembly…" } }
    } else {
        rsx! { crate::cad_presentation::CasePanel { generation_ready: input.generation_ready, mechanical_settings: input.mechanical_settings } }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    let generated = input
        .mechanical_settings
        .props
        .as_ref()
        .is_some_and(|p| p.configuration.is_some() && p.mismatch.is_none());
    let pcb_sel = input.selected_context.as_ref().is_some_and(|s| {
        matches!(
            &s.context,
            super::objects::TreeContext::Key { .. } | super::objects::TreeContext::Component { .. }
        )
    });
    let active_display_layer = input.scope.as_ref().and_then(|scope| {
        let id = input.selected_layer_id.as_str();
        let valid = id == "pcb"
            || input.scene.as_ref().is_some_and(|s| {
                s.exact
                    && s.scope == *scope
                    && (s.mechanical.as_ref().is_some_and(|a| {
                        a.stack.iter().any(|l| l.id == id)
                            || (id == "gaskets" && !a.gasket_supports.is_empty())
                    }) || (id.starts_with("gasket:")
                        && s.result.bodies.iter().any(|b| b.id == id)))
            });
        (generated && !id.is_empty() && valid).then(|| (scope.clone(), id.to_owned()))
    });
    let show_generated_note = active_display_layer.is_none() && !pcb_sel;
    let display = input
        .scope
        .as_ref()
        .map(|s| input.case_selection.display_value(s))
        .unwrap_or_default();
    let visible = active_display_layer
        .as_ref()
        .is_some_and(|(_, id)| is_inspector_visible(&display, id));
    let color = active_display_layer
        .as_ref()
        .and_then(|(_, id)| display.color(id))
        .unwrap_or("#b4bac2")
        .to_owned();
    let (on_disp, adl_c, adl_r, adl_v) = (
        input.on_display,
        active_display_layer.clone(),
        active_display_layer.clone(),
        active_display_layer.clone(),
    );
    rsx! {
        section { "aria-label": "Case Inspector",
            if input.instance_scope_pending { p { role: "status", "Selecting physical assembly…" } }
            if let Some(summary) = input.selected_part_summary.as_ref() {
                p { class: "m-0 px-3.5 pt-2.5 text-sm text-muted-foreground", "aria-label": "Current selection", "{summary.breadcrumb}" }
            }
            if !input.instance_scope_pending {
                section { class: "grid gap-2 border-b border-border py-2.5", "aria-label": "Physical assembly",
                    if let Some(inst) = input.physical_setup.projection.case_instance.clone() {
                        InspectorSection { title: "Assembly setup", default_open: false,
                            if input.physical_setup.projection.topology == HardwareTopology::Split {
                                SelectField::<String> {
                                    label: "Half connection".to_owned(),
                                    value: (if input.physical_setup.projection.transport == HardwareTransport::Wired { "wired" } else { "wireless" }).to_owned(),
                                    options: vec![("wireless".into(), "Wireless · local battery on each half".into()), ("wired".into(), "Wired serial · local power on each half".into())],
                                    disabled: input.physical_setup.projection.busy,
                                    on_commit: { let m = input.physical_setup.clone(); move |v: String| m.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseTransport(if v == "wired" { HardwareTransport::Wired } else { HardwareTransport::Wireless })) },
                                }
                                if input.physical_setup.projection.transport == HardwareTransport::Wired {
                                    Callout { "Use a straight TRRS cable: tip and ring 2 carry crossed TX/RX, sleeve is ground, ring 1 is unused." }
                                }
                            }
                            p { "{inst.name} · {inst.board_label} · {inst.role}" }
                            SelectField::<String> {
                                label: "PCB design".to_owned(), value: inst.board_id.clone(),
                                options: input.physical_setup.projection.case_boards.iter().map(|b| (b.id.clone(), b.name.clone())).collect(),
                                disabled: input.physical_setup.projection.busy,
                                on_commit: { let m = input.physical_setup.clone(); move |id: String| m.submit(super::pcb_physical_setup::PhysicalSetupIntent::CasePcbDesign(id)) },
                            }
                            ToggleField {
                                label: "Turn PCB over for this half".to_owned(), checked: inst.flipped,
                                disabled_reason: input.physical_setup.projection.busy.then(|| "Updating…".into()),
                                on_commit: { let m = input.physical_setup.clone(); move |v: bool| m.submit(super::pcb_physical_setup::PhysicalSetupIntent::CaseFlip(v)) },
                            }
                            if inst.has_board_reference { Callout { "Imported routing is a reference. Review it after changing the assembly." } }
                        }
                    }
                    if let Some(fb) = input.physical_setup.projection.feedback.clone() { p { role: "status", "{fb}" } }
                }
                if let Some(props) = input.mechanical_settings.props { MechanicalSettings { ..props } }
                if generated && active_display_layer.is_some() {
                    section { "aria-label": "Part appearance",
                        h3 { "Display" }
                        TextField { id: format!("case-part-colour-{}", dioxus::core::current_scope_id().0), label: "Colour".to_owned(), r#type: InputType::Color, aria_label: "Part colour", value: color,
                            oninput: move |e: FormEvent| if let Some((s, id)) = &adl_c { on_disp.call(DisplayRequest { target_scope: s.clone(), id: id.clone(), action: DisplayAction::SetColor(e.value()) }); } }
                        Button { variant: ButtonVariant::Secondary, button_type: "button",
                            onclick: move |_| if let Some((s, id)) = &adl_r { on_disp.call(DisplayRequest { target_scope: s.clone(), id: id.clone(), action: DisplayAction::ResetColor }); }, "Reset colour" }
                        ToggleField { label: "Visible".to_owned(), checked: visible,
                            on_commit: move |_| if let Some((s, id)) = &adl_v { on_disp.call(DisplayRequest { target_scope: s.clone(), id: id.clone(), action: DisplayAction::ToggleVisibility }); } }
                    }
                }
            }
            div { hidden: input.instance_scope_pending || (generated && !show_generated_note),
                CaseBodyInspector { on_show_configured_board: input.on_show_configured_board }
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

fn case_roots(doc: &ProjectDoc) -> Vec<CaseRoot> {
    let insts: Vec<_> = doc
        .hardware
        .as_ref()
        .map(|h| {
            h.instances
                .iter()
                .map(|i| CaseRoot {
                    id: i.id.clone(),
                    instance_id: Some(i.id.clone()),
                    board_id: i.board_id.clone(),
                    name: i.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    if !insts.is_empty() {
        return insts;
    }
    doc.boards
        .iter()
        .map(|b| CaseRoot {
            id: b.id.clone(),
            instance_id: None,
            board_id: b.id.clone(),
            name: b.name.clone(),
        })
        .collect()
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
    if aliases.iter().all(|a| display.hidden.contains(a)) {
        display.hidden.retain(|h| !aliases.contains(h));
    } else {
        for a in aliases {
            if !display.hidden.contains(&a) {
                display.hidden.push(a);
            }
        }
    }
}

fn assembly_label(name: &str) -> String {
    let trimmed = [" half", " PCB"]
        .into_iter()
        .find_map(|s| {
            name.get(name.len().checked_sub(s.len())?..)
                .filter(|t| t.eq_ignore_ascii_case(s))
                .map(|_| &name[..name.len() - s.len()])
        })
        .unwrap_or(name);
    let mut c = trimmed.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn stack_label(id: &str) -> String {
    match id {
        "plate" => "Plate",
        "bottom" => "Bottom case",
        "retainer" => "Top case",
        "plate-foam" => "Plate foam",
        "bottom-foam" => "Bottom foam",
        "middle-frame" => "Middle frame",
        "battery" => "Battery",
        other => other,
    }
    .to_owned()
}

fn case_kind_label(kind: &boardstudio_core::model::CaseKind) -> &'static str {
    match kind {
        boardstudio_core::model::CaseKind::Plate => "Plate",
        boardstudio_core::model::CaseKind::Tray => "Tray",
        boardstudio_core::model::CaseKind::Lid => "Lid",
    }
}

fn gasket_side(g: &MechanicalGasketSupport) -> &'static str {
    if g.normal.x.abs() > g.normal.y.abs() {
        if g.normal.x > 0.0 { "Right" } else { "Left" }
    } else if g.normal.y > 0.0 {
        "Top"
    } else {
        "Bottom"
    }
}

fn toggle_tree(mut expanded: Signal<BTreeSet<String>>, id: &str) {
    let mut s = expanded.write();
    if !s.remove(id) {
        s.insert(id.to_owned());
    }
}
