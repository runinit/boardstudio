use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SelectionMode};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

mod keycap_resize;
mod keycap_size;
mod keycap_size_controller;
mod layout_align;
mod layout_align_controller;
mod layout_align_geometry;
mod layout_toolbar;
mod layout_transform_toolbar;
mod matrix_inspector;
mod matrix_inspector_controller;
mod matrix_setup;
mod matrix_setup_controller;
mod matrix_transform_controller;
mod matrix_transform_inspector;
mod mirrored_pair;
mod mirrored_pair_controller;
mod tree;
pub(in crate::presentation) use keycap_size::KeySizeControls;
pub(in crate::presentation) use keycap_size_controller::{KeySizeMount, use_key_size};
pub(in crate::presentation) use layout_align::{
    AlignAction, AlignCommand, AlignFeedback, AlignReference, LayoutAlignMount,
};
pub(in crate::presentation) use layout_align_controller::use_canvas_align;
pub(in crate::presentation) use layout_toolbar::{
    LayoutCommandPill, LayoutSelectionKind, LayoutSelectionSnapStatus, LayoutSnapIntent,
    LayoutSnapSettings, TreeCellAnchor, context_for_selection_kind, gesture_snap_inputs,
};
pub(in crate::presentation) use layout_transform_toolbar::LayoutTransformMenuMount;
pub(in crate::presentation) use matrix_inspector::MatrixInspector;
pub(in crate::presentation) use matrix_inspector_controller::{
    MatrixInspectorMount, use_matrix_inspector,
};
pub(in crate::presentation) use matrix_setup::{MatrixSetup, MatrixSetupMount};
pub(in crate::presentation) use matrix_setup_controller::use_matrix_setup;
pub(in crate::presentation) use matrix_transform_controller::{
    MatrixTransformInspectorMount, use_workspace_matrix_transform,
};
pub(in crate::presentation) use matrix_transform_inspector::MatrixTransformInspector;
pub(in crate::presentation) use mirrored_pair::{
    MirroredPairCanvasOverlay, MirroredPairCreated, MirroredPairMount, MirroredPairMove,
};
pub(in crate::presentation) use mirrored_pair_controller::use_mirrored_pair;
pub(in crate::presentation) use tree::TreeContext;
use tree::{Grouping, TreeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopedTreeContext {
    pub scope: Scope,
    pub context: TreeContext,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TreeSelectRequest {
    pub scope: Scope,
    pub context: TreeContext,
    pub mode: SelectionMode,
    pub outline_action: Option<super::outline_lifecycle::OutlineAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TreeNudgeRequest {
    pub scope: Scope,
    pub part_id: String,
    pub dx: i8,
    pub dy: i8,
    pub large_step: bool,
}

pub(super) fn resolve_selection(model: &ReadModel, context: &TreeContext) -> Option<Vec<String>> {
    tree::resolve_selection(model, context)
}

pub(super) fn context_for_part(model: &ReadModel, part_id: &str) -> Option<TreeContext> {
    tree::context_for_part(model, part_id)
}

pub(super) fn component_context_for_finding_part(
    model: &ReadModel,
    part_id: &str,
) -> Option<TreeContext> {
    tree::component_context_for_finding_part(model, part_id)
}

pub(super) fn context_for_cell(
    model: &ReadModel,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<TreeContext> {
    tree::context_for_cell(model, matrix_id, row, column)
}

pub(super) fn matrix_visible_on_board(
    document: &boardstudio_core::model::ProjectDoc,
    board_id: &str,
    matrix_id: &str,
) -> bool {
    let board_parts = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.part_ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    tree::visible_matrices(document, board_id, &board_parts)
        .iter()
        .any(|matrix| matrix.id == matrix_id)
}

pub(super) fn context_label(model: &ReadModel, context: &TreeContext) -> Option<String> {
    tree::context_label(model, context)
}

#[component]
pub(super) fn Objects(
    selected_context: Signal<Option<ScopedTreeContext>>,
    on_select: EventHandler<TreeSelectRequest>,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
    on_nudge: EventHandler<TreeNudgeRequest>,
    matrix_setup: Option<MatrixSetupMount>,
    mirrored_pair: Option<MirroredPairMount>,
    pair_created: Option<Signal<Option<MirroredPairCreated>>>,
    on_place_component: Option<EventHandler<super::part_placement::ComponentPlacementAction>>,
    layout_target: Option<Signal<Option<String>>>,
    parts_query: Option<super::parts::PartsQuery>,
    on_browse_parts: Option<EventHandler<()>>,
    placement_error: Option<String>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let workspace = use_context::<super::WorkspaceState>().0;
    let case_workspace = workspace() == "Case";
    let pcb_workspace = workspace() == "PCB";
    let generation = (use_context::<super::SelectionAdapter>().generation)();
    let _ = use_context::<Signal<u64>>()();
    let local_pair_created = use_signal(|| None::<MirroredPairCreated>);
    let mut pair_created = pair_created.unwrap_or(local_pair_created);
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = &snapshot.document;
    let board_id = model.active_board_id.clone();
    let active_scope = runtime.scope();
    let board_scope = active_scope.clone();
    let instance_scope = active_scope.clone();
    let navigate_board = on_navigate;
    let navigate_instance = on_navigate;
    let defaults = tree::default_disclosures(document, &board_id);
    let mut expanded = use_signal(|| defaults.clone());
    let created_pair = pair_created();
    let disclosure_scope = (
        snapshot.session_epoch,
        document.id.clone(),
        board_id.clone(),
    );
    use_effect(use_reactive(
        (&disclosure_scope, &defaults, &created_pair),
        move |(_, defaults, created_pair)| {
            expanded.write().extend(defaults);
            if let Some(created) = created_pair {
                let mut tree = expanded.write();
                tree.insert(format!("half:{}", created.left_layout_id));
                tree.insert(format!("half:{}", created.right_layout_id));
                tree.insert(format!("matrix:{}", created.left_matrix_id));
                tree.insert(format!("matrix:{}", created.right_matrix_id));
                pair_created.set(None);
            }
        },
    ));
    let mut grouping = use_signal(|| Grouping::from_storage(read_tree_grouping()));
    let visible_count = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| {
            document
                .parts
                .iter()
                .filter(|part| board.part_ids.contains(&part.id))
                .count()
        })
        .unwrap_or(0);
    let instances: Vec<_> = document
        .hardware
        .as_ref()
        .map(|hardware| {
            hardware
                .instances
                .iter()
                .filter(|instance| instance.board_id == board_id)
                .collect()
        })
        .unwrap_or_default();
    let active_instance = model.active_instance_id.clone().unwrap_or_default();
    let instance_pending = !instances
        .iter()
        .any(|instance| instance.id == active_instance);
    let current_expanded = expanded.read().clone();
    let items = if pcb_workspace {
        tree::build_pcb_tree(&model, &current_expanded)
    } else {
        tree::build_tree(
            document,
            &board_id,
            grouping(),
            &current_expanded,
            snapshot.scene.matrix_scenes.as_slice(),
            snapshot.scene.board_outline_scenes.as_slice(),
        )
    };
    let current_context = selected_context.read().clone().filter(|selected| {
        active_scope.as_ref() == Some(&selected.scope)
            && tree::resolve_selection(&model, &selected.context).is_some()
    });
    let board_name = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map_or("Board", |board| board.name.as_str());
    let pcb_title = current_context
        .as_ref()
        .and_then(|selected| tree::context_label(&model, &selected.context))
        .unwrap_or_else(|| board_name.to_owned());
    let selection_count = model.selected_part_ids.len();
    rsx! {
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
            if (matrix_setup.is_some() || mirrored_pair.is_some())
                && let Some(on_place_component) = on_place_component
                && let Some(layout_target) = layout_target
                && let Some(parts_query) = parts_query
                && let Some(on_browse_parts) = on_browse_parts
            {
                LayoutAddObjectEntry {
                    matrix_setup,
                    mirrored_pair,
                    snapshot: snapshot.clone(),
                    scope: active_scope.clone(),
                    on_place_component,
                    layout_target,
                    parts_query,
                    on_browse_parts,
                }
            }
            if let Some(error) = placement_error {
                p { class: "m1-parts-placement-error", role: "alert", "{error}" }
            }
            div { class: "m1-object-navigation",
                label { "Board"
                    select { "aria-label": "Board", value: "{board_id}", onchange: move |event: FormEvent| {
                        if let Some(scope) = board_scope.clone() {
                            navigate_board.call((scope, event.value(), None));
                        }
                    },
                        for board in &document.boards { option { key: "{board.id}", value: "{board.id}", "{board.name}" } }
                    }
                }
                if !pcb_workspace && !instances.is_empty() && case_workspace {
                    div { class: "m1-instance-selection", role: "group", "aria-label": "Physical instance",
                        span { "Physical instance" }
                        div { class: "m1-instance-choices",
                            for instance in &instances {
                                {
                                    let id = instance.id.clone();
                                    let scope = instance_scope.clone();
                                    let selected = id == active_instance;
                                    rsx! {
                                        button {
                                            key: "{id}", r#type: "button", "aria-pressed": selected,
                                            onclick: move |_| {
                                                if let Some(scope) = scope.clone() {
                                                    navigate_instance.call((scope.clone(), scope.board_id, Some(id.clone())));
                                                }
                                            },
                                            "{instance.name}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if !pcb_workspace && !instances.is_empty() {
                    label { "Physical instance"
                        select { "aria-label": "Physical instance", value: "{active_instance}", onchange: move |event: FormEvent| {
                            if let Some(scope) = instance_scope.clone() {
                                let value = event.value();
                                if !value.is_empty() {
                                    navigate_instance.call((scope.clone(), scope.board_id, Some(value)));
                                }
                            }
                        },
                            if instance_pending {
                                option { value: "", disabled: true, "Selecting assembly…" }
                            }
                            for instance in &instances { option { key: "{instance.id}", value: "{instance.id}", "{instance.name}" } }
                        }
                    }
                }
                if !pcb_workspace {
                label { "Group objects"
                    select { "aria-label": "Group objects", value: if grouping() == Grouping::Row { "row" } else { "column" }, onchange: move |event: FormEvent| {
                        let next = Grouping::from_storage(Some(event.value()));
                        grouping.set(next);
                        write_tree_grouping(next);
                    },
                        option { value: "column", "Columns" }
                        option { value: "row", "Rows" }
                    }
                }
                }
            }
            div { class: "m1-object-tree",
                if !pcb_workspace { div { class: "m1-object-tree-heading", "{document.name}", span { "{visible_count} parts" } } }
                div { role: "tree", "aria-label": "CAD structure", class: "m1-component-list m1-object-tree-list",
                    for item in items.iter().cloned() {
                        {
                            let selected = item.context.as_ref().is_some_and(|context| {
                                current_context.as_ref().is_some_and(|current| current.context == *context)
                            });
                            let tree_item_id = format!("m1-object-tree-{}", item.id);
                            let toggle_item = item.clone();
                            let select_item = item.clone();
                            let keyboard_item = item.clone();
                            let disclosure_label = format!(
                                "{} {}",
                                if item.expanded == Some(true) {
                                    "Collapse"
                                } else {
                                    "Expand"
                                },
                                item.label
                            );
                            let select_on_click = on_select;
                            let select_on_key = on_select;
                            let nudge_on_key = on_nudge;
                            let click_scope = active_scope.clone();
                            let key_scope = active_scope.clone();
                            let outline_action = active_scope.as_ref().zip(item.context.as_ref()).and_then(|(scope, context)| super::outline_lifecycle::OutlineAction::for_tree(snapshot, scope, generation, context));
                            let click_outline_action = outline_action.clone();
                            let key_outline_action = outline_action;

                            rsx! {
                                div { key: "{item.id}", class: "m1-tree-row", style: "padding-left: {8 + item.level * 14}px",
                                    role: "treeitem",
                                    "aria-labelledby": "{tree_item_id}",
                                    "aria-level": "{item.level + 1}",
                                    "aria-selected": "{selected}",
                                    "aria-expanded": if item.expandable { "{item.expanded == Some(true)}" },
                                    if item.expandable {
                                        button {
                                            id: "{tree_item_id}-disclosure",
                                            class: "m1-tree-disclosure",
                                            "aria-label": "{disclosure_label}",
                                            "aria-expanded": "{item.expanded == Some(true)}",
                                            onclick: move |_| toggle_tree(expanded, &toggle_item.id),
                                            if item.expanded == Some(true) { "⌄" } else { "›" }
                                        }
                                    } else {
                                        span { class: "m1-tree-spacer", "aria-hidden": "true" }
                                    }
                                    button {
                                        id: "{tree_item_id}",
                                        class: if selected { "m1-component selected" } else { "m1-component" },
                                        onclick: move |_| {
                                            if let Some(context) = select_item.context.clone() {
                                                if let Some(scope) = click_scope.clone() {
                                                    select_on_click.call(TreeSelectRequest { scope, context, mode: SelectionMode::Replace, outline_action: click_outline_action.clone() });
                                                }
                                                if select_item.kind == TreeKind::Board && select_item.expanded != Some(true) {
                                                    toggle_tree(expanded, &select_item.id);
                                                }
                                            } else if select_item.expandable {
                                                toggle_tree(expanded, &select_item.id);
                                            }
                                        },
                                        onkeydown: move |event: KeyboardEvent| {
                                            let key = event.data().key().to_string();
                                            if key == "Enter" || key == " " {
                                                event.prevent_default();
                                                if let Some(context) = keyboard_item.context.clone() {
                                                    if let Some(scope) = key_scope.clone() {
                                                        select_on_key.call(TreeSelectRequest { scope, context, mode: SelectionMode::Replace, outline_action: key_outline_action.clone() });
                                                    }
                                                    if keyboard_item.kind == TreeKind::Board && keyboard_item.expanded != Some(true) {
                                                        toggle_tree(expanded, &keyboard_item.id);
                                                    }
                                                } else if keyboard_item.expandable {
                                                    toggle_tree(expanded, &keyboard_item.id);
                                                }
                                            } else if let Some((dx, dy)) = match key.as_str() {
                                                "ArrowLeft" => Some((-1, 0)),
                                                "ArrowRight" => Some((1, 0)),
                                                "ArrowUp" => Some((0, 1)),
                                                "ArrowDown" => Some((0, -1)),
                                                _ => None,
                                            } {
                                                let standalone = matches!(
                                                    keyboard_item.context.as_ref(),
                                                    Some(TreeContext::Component {
                                                        matrix_id: None,
                                                        ..
                                                    })
                                                );
                                                if (keyboard_item.kind == TreeKind::Key || standalone)
                                                    && let Some(part_id) = keyboard_item.primary_id.clone()
                                                    && let Some(scope) = key_scope.clone()
                                                {
                                                    event.prevent_default();
                                                    nudge_on_key.call(TreeNudgeRequest {
                                                        scope,
                                                        part_id,
                                                        dx,
                                                        dy,
                                                        large_step: event.data().modifiers().shift(),
                                                    });
                                                }
                                            }
                                        },
                                        span { class: "m1-tree-glyph", {tree_glyph(item.kind)} }
                                        span { class: "m1-tree-label", "{item.label}" }
                                        if let Some(detail) = item.detail { span { class: "m1-object-kind", "{detail}" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if pcb_workspace {
                footer { class: "m1-pcb-object-footer",
                    strong { "{pcb_title}" }
                    span { "{board_name} / PCB" }
                    span { if selection_count > 0 { "{selection_count} selected" } else { "Select an object to edit" } }
                }
            }
        }
    }
}

#[component]
fn LayoutAddObjectEntry(
    matrix_setup: Option<MatrixSetupMount>,
    mirrored_pair: Option<MirroredPairMount>,
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    layout_target: Signal<Option<String>>,
    parts_query: super::parts::PartsQuery,
    on_browse_parts: EventHandler<()>,
) -> Element {
    let mut menu_open = use_signal(|| false);
    let open_menu = menu_open();
    let mirrored_open = mirrored_pair
        .as_ref()
        .map(|mount| (mount.can_open, mount.on_open));
    let matrix_open = matrix_setup
        .as_ref()
        .map(|mount| (mount.can_open, mount.on_open));
    rsx! {
        div { class: "m1-layout-add-object",
            button {
                r#type: "button",
                class: "m1-layout-add-trigger",
                aria_expanded: open_menu,
                onclick: move |_| menu_open.set(!menu_open()),
                svg { class: "m1-add-icon", "aria-hidden": "true", view_box: "0 0 16 16",
                    path { d: "M8 3v10M3 8h10" }
                }
                "Add object"
            }
            if open_menu {
                div { role: "dialog", "aria-label": "Add", class: "m1-layout-add-menu",
                    if let Some(scope) = scope.as_ref() {
                        if let Some(board) = snapshot.document.boards.iter().find(|board| board.id == scope.board_id) {
                            strong { class: "m1-add-to-board-heading", "Add to {board.name}" }
                        }
                    }
                    section { "aria-label": "Layouts",
                        h3 { "Layouts" }
                        if let Some((can_open, on_open)) = mirrored_open {
                            button {
                                r#type: "button",
                                disabled: !can_open,
                                onclick: move |_| {
                                    menu_open.set(false);
                                    on_open.call(());
                                },
                                "Mirrored pair…"
                                small { "Create linked left and right halves" }
                            }
                        }
                        if let Some((can_open, on_open)) = matrix_open {
                            button {
                                r#type: "button",
                                disabled: !can_open,
                                onclick: move |_| {
                                    menu_open.set(false);
                                    on_open.call(());
                                },
                                "Matrix…"
                                small { "Rows, columns & key assemblies" }
                            }
                        }
                    }
                    section { "aria-label": "Parts",
                        super::parts::AddObjectComponentChooser {
                            snapshot,
                            scope,
                            layout_target,
                            query: parts_query,
                            on_browse: on_browse_parts,
                            on_place: EventHandler::new(move |action| {
                                menu_open.set(false);
                                on_place_component.call(action);
                            }),
                        }
                    }
                }
            }
        }
        if let Some(mount) = matrix_setup.as_ref() {
          if let Some(projection) = mount.projection.clone() {
            MatrixSetup {
                projection,
                on_cancel: mount.on_cancel,
                on_create: mount.on_create,
            }
          }
        }
    }
}

fn toggle_tree(mut expanded: Signal<BTreeSet<String>>, id: &str) {
    let mut state = expanded.write();
    if !state.remove(id) {
        state.insert(id.to_owned());
    }
}

fn read_tree_grouping() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| {
            storage
                .get_item("boardstudio:v2:tree-grouping")
                .ok()
                .flatten()
        })
}

fn write_tree_grouping(grouping: Grouping) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(
            "boardstudio:v2:tree-grouping",
            if grouping == Grouping::Row {
                "row"
            } else {
                "column"
            },
        );
    }
}

fn tree_glyph(kind: TreeKind) -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", fill: "none", stroke: "currentColor", stroke_width: "1.3", "aria-hidden": "true",
            if kind == TreeKind::Matrix {
                rect { x: "2", y: "2", width: "12", height: "12", rx: "1" }
                path { d: "M6 2v12M10 2v12M2 6h12M2 10h12" }
            } else if kind == TreeKind::Row {
                path { d: "M2 4h12M2 8h12M2 12h12" }
            } else if kind == TreeKind::Column {
                path { d: "M8 2v12M5 5l3-3 3 3M5 11l3 3 3-3" }
            } else if kind == TreeKind::Key {
                rect { x: "2", y: "2", width: "12", height: "12", rx: "2" }
            } else if kind == TreeKind::Component {
                path { d: "m8 1.8 6.2 6.2L8 14.2 1.8 8z" }
                circle { cx: "8", cy: "8", r: "1.4" }
            } else if kind == TreeKind::Components {
                rect { x: "2", y: "2", width: "8", height: "8", rx: "1" }
                rect { x: "6", y: "6", width: "8", height: "8", rx: "1" }
                path { d: "M4 4h4M8 8h4" }
            } else if kind == TreeKind::Outline || kind == TreeKind::OutlineVersion {
                path { d: "M3 3h10v10H3zM6 6h4M6 9h4" }
            } else if kind == TreeKind::Bridge {
                path { d: "M2 12c2-7 4-7 6 0s4 7 6 0" }
            } else {
                path { d: "M2 2h8v3h4v9H2z" }
            }
        }
    }
}
