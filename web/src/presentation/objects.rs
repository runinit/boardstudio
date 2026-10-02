use crate::runtime::Runtime;
use boardstudio_application::{ReadModel, Scope, SelectionMode};
use dioxus::prelude::*;
use std::{collections::BTreeSet, rc::Rc};

mod tree;
pub(in crate::presentation) use tree::TreeContext;
use tree::{Grouping, TreeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopedTreeContext {
    pub scope: Scope,
    pub context: TreeContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TreeSelectRequest {
    pub scope: Scope,
    pub context: TreeContext,
    pub mode: SelectionMode,
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

pub(super) fn context_for_cell(
    model: &ReadModel,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<TreeContext> {
    tree::context_for_cell(model, matrix_id, row, column)
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
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
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
    let disclosure_scope = (
        snapshot.session_epoch,
        document.id.clone(),
        board_id.clone(),
    );
    use_effect(use_reactive(
        (&disclosure_scope, &defaults),
        move |(_, defaults)| {
            expanded.write().extend(defaults);
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
    let current_expanded = expanded.read().clone();
    let items = tree::build_tree(
        document,
        &board_id,
        grouping(),
        &current_expanded,
        snapshot.scene.matrix_scenes.as_slice(),
    );
    let current_context = selected_context.read().clone().filter(|selected| {
        active_scope.as_ref() == Some(&selected.scope)
            && tree::resolve_selection(&model, &selected.context).is_some()
    });
    rsx! {
        aside { class: "m1-objects", "aria-label": "Objects",
            header { h2 { "Objects" } }
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
                if !instances.is_empty() {
                    label { "Physical instance"
                        select { "aria-label": "Physical instance", value: "{active_instance}", onchange: move |event: FormEvent| {
                            if let Some(scope) = instance_scope.clone() {
                                let value = event.value();
                                navigate_instance.call((scope.clone(), scope.board_id, (!value.is_empty()).then_some(value)));
                            }
                        },
                            option { value: "", "Canonical board" }
                            for instance in &instances { option { key: "{instance.id}", value: "{instance.id}", "{instance.name}" } }
                        }
                    }
                }
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
            div { class: "m1-object-tree",
                div { class: "m1-object-tree-heading", "{document.name}", span { "{visible_count} parts" } }
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
                            rsx! {
                                div { key: "{item.id}", class: "m1-tree-row", style: "padding-left: {8 + item.level * 14}px",
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
                                        role: "treeitem",
                                        "aria-level": "{item.level + 1}",
                                        "aria-selected": "{selected}",
                                        "aria-expanded": if item.expandable { "{item.expanded == Some(true)}" },
                                        onclick: move |_| {
                                            if let Some(context) = select_item.context.clone() {
                                                if let Some(scope) = click_scope.clone() {
                                                    select_on_click.call(TreeSelectRequest { scope, context, mode: SelectionMode::Replace });
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
                                                        select_on_key.call(TreeSelectRequest { scope, context, mode: SelectionMode::Replace });
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
            } else {
                path { d: "M2 2h8v3h4v9H2z" }
            }
        }
    }
}
