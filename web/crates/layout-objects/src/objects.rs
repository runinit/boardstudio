use crate::runtime::Runtime;
use boardstudio_application::{ReadModel, Scope, SelectionMode};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
use boardstudio_web_ui_shared::ui::{
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::Item,
};
use boardstudio_web_ui_shared::ui::{
    icons::ChevronDown,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};

use boardstudio_web_ui_shared::inspector_fields::SelectField;
use boardstudio_web_ui_shared::ui::button::{Button, ButtonSize, ButtonVariant};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use existing_half::use_existing_half;
use std::{collections::BTreeSet, rc::Rc};

mod add_matrix_surface;
mod board_setup_controller;
mod existing_half;
mod keycap_resize;
mod keycap_size;
mod keycap_size_controller;
mod keycap_size_logic;
mod layout_align;
mod layout_align_controller;
mod layout_align_geometry;
mod layout_align_logic;
mod layout_anchor_pick;
mod layout_canvas_controls;
mod layout_rail;
mod layout_rail_chrome;
mod layout_rail_menu;
mod layout_rail_tool;
mod workspace_state;
pub use workspace_state::{LayoutInspectorTab, LayoutWorkspaceState, use_layout_workspace_state};
mod layout_selection_kind;
mod layout_toolbar;
mod layout_toolbar_logic;
mod matrix_inspector;
mod matrix_placement_controller;
mod matrix_preparation;
mod matrix_preset;
mod matrix_preset_logic;
mod matrix_setup;
mod matrix_setup_controller;
mod matrix_setup_fields;
mod matrix_setup_logic;
mod matrix_transform;
mod mirror_links;
mod mirrored_pair;
mod mirrored_pair_controller;
mod tree_projection_logic;
pub use add_matrix_surface::{AddMatrixMode, AddMatrixSurface};
pub use board_setup_controller::{BoardSetupMount, use_board_setup};
use boardstudio_web_ui_model::tree;
pub use existing_half::{ExistingHalfMount, ExistingHalfSetup};
pub use keycap_size::KeySizeControls;
pub use keycap_size_controller::{KeySizeMount, use_key_size};
pub use layout_align::{
    AlignAction, AlignCommand, AlignFeedback, AlignReference, LayoutAlignMount,
};
pub use layout_align_controller::use_canvas_align;
pub use layout_anchor_pick::{AnchorPickOwner, LayoutAnchorPickOverlay};
pub use layout_canvas_controls::LayoutCanvasControls;
pub use layout_rail::{
    LayoutRailOverlay, LayoutTransformRuntime, LayoutTransformTool, RailTarget, ToolActivation,
    activate_transform_tool,
};
pub use layout_rail_menu::{
    LayoutContextActionToolbar, LayoutTransformMenuMount, LayoutTransformToolbar,
};
pub use layout_rail_tool::transform_tools;
pub use layout_toolbar::{
    GestureSnapInputs, LayoutCanvasGridStyle, LayoutCommandMenu, LayoutCommandPill,
    LayoutSelectionKind, LayoutSnapDock, LayoutSnapIntent, LayoutSnapSettings, TreeCellAnchor,
    context_for_selection_kind, gesture_snap_inputs, layout_canvas_grid_style,
    snap_label as layout_snap_label,
};
pub use matrix_inspector::{MatrixInspector, MatrixInspectorMount, use_matrix_inspector};
pub use matrix_placement_controller::use_matrix_placement;
pub use matrix_preset::{
    MatrixPreset, SwitchOrientation as MatrixSwitchOrientation, matrix_with_preset,
};
pub use matrix_setup::{
    MatrixPlacementInput, MatrixPlacementMount, MatrixPlacementMove, MatrixPlacementSource,
    MatrixSetup, MatrixSetupMount,
};
pub use matrix_setup_controller::use_matrix_setup;
pub use matrix_transform::{
    DepartureControl, MatrixTransformInspector, MatrixTransformInspectorMount,
    MatrixTransformInspectorOwner, MatrixTransformProjection, SelectionDeleteControl,
    use_workspace_matrix_transform,
};
pub use mirrored_pair::{
    MirroredPairCanvasOverlay, MirroredPairCreated, MirroredPairMount, MirroredPairMove,
};
pub use mirrored_pair_controller::use_mirrored_pair;
use tree::TreeKind;
pub use tree::{ScopedTreeContext, TreeContext};

impl From<TreeSelectRequest> for super::selection::ContextRequest {
    fn from(request: TreeSelectRequest) -> Self {
        Self {
            scope: request.scope,
            context: request.context,
            mode: request.mode,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeSelectRequest {
    pub scope: Scope,
    pub context: TreeContext,
    pub mode: SelectionMode,
    pub outline_action: Option<super::outline_lifecycle::OutlineAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNudgeRequest {
    pub scope: Scope,
    pub part_id: String,
    pub dx: i8,
    pub dy: i8,
    pub large_step: bool,
}

pub fn resolve_selection(model: &ReadModel, context: &TreeContext) -> Option<Vec<String>> {
    tree::resolve_selection(model, context)
}

pub fn context_for_part(model: &ReadModel, part_id: &str) -> Option<TreeContext> {
    tree::context_for_part(model, part_id)
}

pub fn component_context_for_finding_part(model: &ReadModel, part_id: &str) -> Option<TreeContext> {
    tree::component_context_for_finding_part(model, part_id)
}

pub fn context_for_cell(
    model: &ReadModel,
    matrix_id: &str,
    row: u32,
    column: u32,
) -> Option<TreeContext> {
    tree::context_for_cell(model, matrix_id, row, column)
}

/// The matrix a tree selection belongs to, when it names one. Matrix, row,
/// column, key and component selections all carry their matrix identity.
pub fn matrix_id_for_context(context: &TreeContext) -> Option<&str> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id),
        TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => Some(matrix_id),
        _ => None,
    }
}

pub use boardstudio_web_ui_model::tree::matrix_visible_on_board;

pub fn context_label(model: &ReadModel, context: &TreeContext) -> Option<String> {
    tree::context_label(model, context)
}

#[component]
pub fn Objects(
    selected_context: Signal<Option<ScopedTreeContext>>,
    on_select: EventHandler<TreeSelectRequest>,
    on_navigate: EventHandler<(Scope, String, Option<String>)>,
    on_nudge: EventHandler<TreeNudgeRequest>,
    board_setup: Option<BoardSetupMount>,
    matrix_setup: Option<MatrixSetupMount>,
    pair_created: Option<Signal<Option<MirroredPairCreated>>>,
    on_place_component: Option<
        EventHandler<boardstudio_web_ui_model::state::ComponentPlacementAction>,
    >,
    layout_target: Option<Signal<Option<String>>>,
    parts_query: Option<super::parts::PartsQuery>,
    on_browse_parts: Option<EventHandler<()>>,
    placement_error: Option<String>,
    matrix_inspector: Option<MatrixInspectorMount>,
    add_object_menu_open: Option<Signal<bool>>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let workspace = use_context::<super::WorkspaceState>().0;
    let case_workspace = workspace() == Workspace::Case;
    let pcb_workspace = workspace() == Workspace::Pcb;
    let scope_generation = use_context::<super::SelectionAdapter>().generation;
    let generation = scope_generation();
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let existing_half = use_existing_half(runtime.clone(), version, workspace, scope_generation);
    let existing_half_active = existing_half.projection.is_some();
    let local_pair_created = use_signal(|| None::<MirroredPairCreated>);
    let mut pair_created = pair_created.unwrap_or(local_pair_created);
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let resolved = boardstudio_application::resolved_document(&snapshot.document);
    let document = resolved.as_ref();
    let board_id = model.active_board_id.clone();
    let active_scope = runtime.scope();
    let instance_scope = active_scope.clone();
    let navigate_instance = on_navigate;
    let mut defaults = tree::default_disclosures(document, &board_id);
    defaults.extend(
        [
            "Switches",
            "MCUs",
            "Diodes",
            "LEDs",
            "Encoders",
            "Connectors",
            "Other",
        ]
        .map(|name| format!("category:{name}")),
    );
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
    let mut by_part = use_signal(|| {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("boardstudio:v2:object-view").ok().flatten())
            .as_deref()
            == Some("part")
    });
    let source = AcceptedSource(snapshot.clone());
    let grouping_choice = use_memo(move || if by_part() { "part" } else { "layout" }.to_owned());
    let group_by_part = by_part();
    let items = use_memo(use_reactive(
        (&source, &board_id, &current_expanded, &group_by_part),
        |(source, board_id, expanded, by_part)| {
            let snapshot = &source.0;
            let resolved = boardstudio_application::resolved_document(&snapshot.document);
            let document = resolved.as_ref();
            let mut items = if by_part {
                grouped_part_tree(
                    &ReadModel {
                        accepted: Some(snapshot.clone()),
                        active_board_id: board_id.clone(),
                        ..ReadModel::default()
                    },
                    &expanded,
                )
            } else {
                tree::build_tree(
                    document,
                    &board_id,
                    tree::Grouping::Column,
                    &expanded,
                    &snapshot.scene.matrix_scenes,
                    &snapshot.scene.board_outline_scenes,
                )
            };
            for board in document.boards.iter().filter(|board| board.id != board_id) {
                items.push(tree::TreeItem {
                    id: format!("board:{}", board.id),
                    label: board.name.clone(),
                    detail: Some(format!("{} parts", board.part_ids.len())),
                    level: 0,
                    kind: TreeKind::Board,
                    context: Some(TreeContext::Board {
                        board_id: board.id.clone(),
                    }),
                    expanded: None,
                    primary_id: None,
                    expandable: false,
                });
            }
            items
        },
    ));
    let items = items.read();

    let current_context = selected_context.read().clone().filter(|selected| {
        active_scope.as_ref() == Some(&selected.scope)
            && super::selection::context_is_current(&model, &selected.scope, &selected.context)
    });
    let selected_matrix_id = current_context
        .as_ref()
        .and_then(|selected| matrix_id_for_context(&selected.context));
    let selected_matrix_action = current_context.as_ref().and_then(|selected| {
        let matrix_id = selected_matrix_id?;
        let matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)?;
        let matrix_context = TreeContext::Matrix {
            matrix_id: matrix.id.clone(),
        };
        Some((
            selected.scope.clone(),
            matrix.id.clone(),
            tree::context_label(&model, &matrix_context).unwrap_or_else(|| {
                matrix
                    .name
                    .clone()
                    .unwrap_or_else(|| "Selected matrix".into())
            }),
        ))
    });
    let selected_layout_target = selected_matrix_id.and_then(|matrix_id| {
        document
            .layouts
            .iter()
            .find(|layout| {
                layout.board_id == board_id && layout.matrix_ids.iter().any(|id| *id == matrix_id)
            })
            .map(|layout| layout.id.clone())
    });
    let layout_target_for_effect = layout_target;
    use_effect(use_reactive(
        (&selected_layout_target,),
        move |(selected_layout_target,)| {
            if let Some(mut layout_target) = layout_target_for_effect {
                layout_target.set(selected_layout_target);
            }
        },
    ));
    let local_add_menu_open = use_signal(|| false);
    let use_external_add_action = add_object_menu_open.is_some();
    let mut add_menu_open = add_object_menu_open.unwrap_or(local_add_menu_open);
    let add_menu_owner = (
        snapshot.session_epoch,
        document.id.clone(),
        board_id.clone(),
        workspace(),
    );
    use_effect(use_reactive((&add_menu_owner,), move |_| {
        add_menu_open.set(false)
    }));
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
                aside { class: "relative flex min-h-0 min-w-0 flex-1 flex-col bg-card [&>header]:hidden", "aria-label": "Objects",
                    onkeydown: stop_space_pan,
                    header {
                        h2 { "Objects" }
                    }
                    if let Some(on_place_component) = on_place_component
                        && let Some(layout_target) = layout_target && let Some(parts_query) = parts_query
                        && let Some(on_browse_parts) = on_browse_parts
                    {
                        AddObjectEntry {
                            menu_open: add_menu_open,
                            show_trigger: !use_external_add_action,
                            on_select,
                            matrix_setup,
                            matrix_inspector,
                            existing_half,
                            snapshot: AcceptedSource(snapshot.clone()),
                            scope: active_scope.clone(),
                            selected_matrix_action: selected_matrix_action.clone(),
                            on_place_component,
                            layout_target,
                            parts_query,
                            on_browse_parts,
                        }
                    }
                    if let Some(error) = placement_error {
                        Alert {
        variant: AlertVariant::Destructive,AlertDescription {  "{error}"  } }
                    }
                    if !add_menu_open() && !existing_half_active {
                        SelectField {
                            label: "Group".to_owned(),
                            value: grouping_choice(),
                            options: vec![
                                ("layout".to_owned(), "By layout".to_owned()),
                                ("part".to_owned(), "By part".to_owned()),
                            ],
                            aria_label: "Group objects by",
                            on_commit: move |choice: String| {
                                let next = choice == "part";
                                by_part.set(next);
                                if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
                                    let _ = storage.set_item(
                                        "boardstudio:v2:object-view",
                                        if next { "part" } else { "layout" },
                                    );
                                }
                            },
                        }
                        div { class: "grid gap-[7px] border-b border-border px-2.5 py-[9px] empty:hidden",
                            if !pcb_workspace && !instances.is_empty() && case_workspace {
                                div {
                                    class: "grid gap-1 text-muted-foreground text-xs",
                                    role: "group",
                                    "aria-label": "Physical instance",
                                    span { "Physical instance" }
                                    div { class: "flex flex-wrap gap-1 [&_button]:min-h-11 [&_button]:text-left [&_button]:break-words",
                                        for instance in &instances {
                                            {
                                                let id = instance.id.clone();
                                                let navigation_id = id.clone();
                                                let scope = instance_scope.clone();
                                                let selected = id == active_instance;
                                                rsx! {
                                                    ToggleGroup {
                                                        aria_label: format!("{} selection", instance.name),
                                                        pressed: Some(
                                                            if selected {
                                                                std::collections::HashSet::from([0usize])
                                                            } else {
                                                                std::collections::HashSet::new()
                                                            },
                                                        ),
                                                        on_pressed_change: move |_| {
                                                            if let Some(scope) = scope.clone() {
                                                                navigate_instance
                                                                    .call((scope.clone(), scope.board_id, Some(navigation_id.clone())));
                                                            }
                                                        },
                                                        ToggleItem { index: 0usize, key: "{id}", "{instance.name}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex min-h-0 flex-1 flex-col",
                            div {
                                role: "tree",
                                "aria-label": "CAD structure",
                                class: "grid min-h-0 flex-1 content-start gap-0 overflow-auto overscroll-contain px-0 py-1",
                                for item in items.iter().cloned() {
                                    {
                                        let selected = item
                                            .context
                                            .as_ref()
                                            .is_some_and(|context| {
                                                current_context
                                                    .as_ref()
                                                    .is_some_and(|current| current.context == *context)
                                            });
                                        let tree_item_id = format!("m1-object-tree-{}", item.id);
                                        let toggle_item = item.clone();
                                        let select_item = item.clone();
                                        let keyboard_item = item.clone();
                                        let disclosure_label = format!(
                                            "{} {}",
                                            if item.expanded == Some(true) { "Collapse" } else { "Expand" },
                                            item.label,
                                        );
                                        let disclosure_id = format!("{tree_item_id}-disclosure");
                                        let tooltip_label = disclosure_label.clone();
                                        let is_expanded = item.expanded == Some(true);
                                        let select_on_click = on_select;
                                        let navigate_on_click = on_navigate;
                                        let select_on_key = on_select;
                                        let navigate_on_key = on_navigate;
                                        let nudge_on_key = on_nudge;
                                        let click_scope = active_scope.clone();
                                        let key_scope = active_scope.clone();
                                        let outline_action = active_scope
                                            .as_ref()
                                            .zip(item.context.as_ref())
                                            .and_then(|(scope, context)| super::outline_lifecycle::OutlineAction::for_tree(
                                                snapshot,
                                                scope,
                                                generation,
                                                context,
                                            ));
                                        let click_outline_action = outline_action.clone();
                                        let key_outline_action = outline_action;
                                        rsx! {
                                            div {
                                                key: "{item.id}",
                                                class: "flex min-h-10 min-w-0 items-center gap-0.5 pr-2",
                                                style: "padding-left: {8 + item.level * 14}px",
                                                role: "treeitem",
                                                "aria-labelledby": "{tree_item_id}",
                                                "aria-level": "{item.level + 1}",
                                                "aria-selected": "{selected}",
                                                "aria-expanded": if item.expandable { "{item.expanded == Some(true)}" },
                                                if item.expandable {
                                                    Tooltip {
                                                        TooltipTrigger {
                                                            r#as: move |attributes: Vec<Attribute>| {
                                                                let toggle_item = toggle_item.clone();
                                                                rsx! {
                                                                    Button {
                                                                        attributes,
                                                                        variant: ButtonVariant::Ghost,
                                                                        id: disclosure_id.clone(),
                                                                        class: "min-h-11 min-w-11",
                                                                        aria_label: disclosure_label.clone(),
                                                                        aria_expanded: is_expanded,
                                                                        onclick: move |_| toggle_tree(expanded, &toggle_item.id),
                                                                        size: ButtonSize::IconSm,
                                                                        ChevronDown { class: if is_expanded { "size-4" } else { "size-4 -rotate-90" } }
                                                                    }
                                                                }
                                                            },
                                                        }
                                                        TooltipContent { "{tooltip_label}" }
                                                    }
                                                } else {
                                                    span { class: "grid h-6 min-h-6 w-5 shrink-0 place-items-center border-0 bg-transparent p-0 text-muted-foreground", "aria-hidden": "true" }

                                                }
                                                Button { variant: ButtonVariant::Secondary,
    id: "{tree_item_id}",
    class: if selected { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-[9px] rounded-[3px] border-primary bg-selected px-[5px] text-left text-selected-foreground [&_span]:text-selected-foreground" } else { "flex min-h-8 min-w-0 flex-1 items-center justify-start gap-[9px] rounded-[3px] border-transparent bg-transparent px-[5px] text-left hover:bg-accent" },
    onclick: move |_| {
                                                    if let Some(context) = select_item.context.clone() {
                                                        if let Some(scope) = click_scope.clone() {
                                                            if let TreeContext::Board { board_id } = &context
                                                                && *board_id != scope.board_id
                                                            {
                                                                navigate_on_click.call((scope, board_id.clone(), None));
                                                                return;
                                                            }
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
                                                                if let TreeContext::Board { board_id } = &context
                                                                    && *board_id != scope.board_id
                                                                {
                                                                    navigate_on_key.call((scope, board_id.clone(), None));
                                                                    return;
                                                                }
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

                                                    span { class: "block size-[17px] shrink-0 text-muted-foreground [&_svg]:block [&_svg]:size-[17px]", {tree_glyph(item.kind)} }
                                                    span { class: if pcb_workspace { "min-w-0 shrink-0 overflow-visible text-sm font-[650] leading-normal text-foreground" } else { "min-w-0 overflow-hidden text-ellipsis whitespace-nowrap text-sm leading-normal text-foreground" },
                                                        "{item.label}"
                                                    }
                                                    if let Some(detail) = item.detail {
                                                        span { class: "ml-auto min-w-0 overflow-hidden text-ellipsis whitespace-nowrap text-xs text-muted-foreground", "{detail}" }
                                                    }

                                                 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if pcb_workspace {
                            footer { class: "flex shrink-0 flex-col gap-[5px] border-t border-border px-5 py-4 text-xs [&_span]:text-muted-foreground",
                                strong { "{pcb_title}" }
                                span { "{board_name} / PCB" }
                                span {
                                    if selection_count > 0 {
                                        "{selection_count} selected"
                                    } else {
                                        "Select an object to edit"
                                    }
                                }
                            }
                        }
                    }
                }
            }
}

#[component]
fn AddObjectEntry(
    mut menu_open: Signal<bool>,
    show_trigger: bool,
    on_select: EventHandler<TreeSelectRequest>,
    matrix_setup: Option<MatrixSetupMount>,
    matrix_inspector: Option<MatrixInspectorMount>,
    existing_half: ExistingHalfMount,
    snapshot: AcceptedSource,
    scope: Option<Scope>,
    selected_matrix_action: Option<(Scope, String, String)>,
    on_place_component: EventHandler<boardstudio_web_ui_model::state::ComponentPlacementAction>,
    layout_target: Signal<Option<String>>,
    parts_query: super::parts::PartsQuery,
    on_browse_parts: EventHandler<()>,
) -> Element {
    let mut layouts_open = use_signal(|| true);
    let mut matrix_actions_open = use_signal(|| true);
    let mut geometry_open = use_signal(|| true);
    let snapshot = snapshot.0;
    let open_menu = menu_open();
    let matrix_open = matrix_setup
        .as_ref()
        .map(|mount| (mount.can_open, mount.on_open));
    let existing_half_open = (existing_half.can_open, existing_half.on_open);
    // The selected matrix's label and id, plus the scope to select first when the matrix is only named.
    let matrix_actions = matrix_inspector
        .as_ref()
        .and_then(|mount| {
            let id = mount.selected_matrix_id().filter(|_| mount.is_active())?;
            Some((format!("Matrix {id}"), id, None, mount.clone()))
        })
        .or_else(|| {
            let (scope, matrix_id, label) = selected_matrix_action?;
            Some((
                label,
                matrix_id.clone(),
                Some((scope, matrix_id)),
                matrix_inspector.clone()?,
            ))
        });
    rsx! {
            if show_trigger || open_menu {
                div { class: if show_trigger { "relative shrink-0 border-b border-border px-2.5 py-2 [&>button]:min-h-[38px] [&>button]:w-full" } else { "relative shrink-0" },
                    if show_trigger {
                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            id: "m1-add-object-trigger",
                            aria_label: "Add object",
                            class: "max-[760px]:min-h-control-touch!",
                            aria_expanded: open_menu,
                            onclick: move |_| menu_open.set(!menu_open()),

                            svg {
                                class: "size-4 fill-none stroke-current stroke-[1.8] [stroke-linecap:round]",
                                "aria-hidden": "true",
                                view_box: "0 0 16 16",
                                path { d: "M8 3v10M3 8h10" }
                            }
                            "Add object"

                        }
                    }
                    if open_menu {
                        section {
                            role: "region",
                            aria_label: "Add",
                            class: "relative mt-3 grid max-h-[calc(100dvh-168px)] gap-1.5 overflow-y-auto overscroll-contain bg-card [&_h3]:mb-[5px] [&_h3]:text-xs [&_button]:min-h-9 [&_button]:w-full [&_button]:text-left [&_small]:text-muted-foreground",
                            onkeydown: move |event: KeyboardEvent| {
                                if event.key() == Key::Escape {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    menu_open.set(false);
                                    focus_add_object();
                                }
                            },

                            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    onclick: move |_| { menu_open.set(false); focus_add_object(); },
                                "← Back to objects"
                             }
                            if let Some(scope) = scope.as_ref() {
                                if let Some(board) = snapshot
                                    .document
                                    .boards
                                    .iter()
                                    .find(|board| board.id == scope.board_id)
                                {
                                    strong { class: "text-muted-foreground text-sm font-medium", "Add to {board.name}" }
                                }
                            }
                            AddMenuSection {
                                heading: "Layouts".to_owned(),
                                open: layouts_open(),
                                on_open_change: move |next| layouts_open.set(next),

                                    if existing_half.visible {
                                        AddMenuEntry {
                                            label: "Mirror existing half…",
                                            hint: "Create a linked half from an existing layout",
                                            enabled: existing_half_open.0,
                                            on_press: move |_| {
                                                menu_open.set(false);
                                                existing_half_open.1.call(());
                                            },
                                        }
                                    }
                                    if let Some((can_open, on_open)) = matrix_open {
                                        AddMenuEntry {
                                            label: "Matrix…",
                                            hint: "Single or mirrored-pair rows, columns & key assemblies",
                                            enabled: can_open,
                                            on_press: move |_| {
                                                menu_open.set(false);
                                                on_open.call(());
                                            },
                                        }
                                    }
                            }
                            if let Some((matrix_label, matrix_id, selection, mount)) = matrix_actions
                            {
                                {
                                    rsx! {
                                        AddMenuSection {
                                            heading: matrix_label,
                                            open: matrix_actions_open(),
                                            on_open_change: move |next| matrix_actions_open.set(next),
                                            for (label, rows) in [("Add row", true), ("Add column", false)] {
                                                {
                                                    let selection = selection.clone();
                                                    let mount = mount.clone();
                                                    let matrix_id = matrix_id.clone();
                                                    rsx! {
                                                        AddMenuEntry {
                                                            label: label,
                                                            on_press: move |_| {
                                                                menu_open.set(false);
                                                                if let Some((scope, matrix_id)) = selection.clone() {
                                                                    on_select.call(TreeSelectRequest {
                                                                        scope,
                                                                        context: TreeContext::Matrix { matrix_id: matrix_id.clone() },
                                                                        mode: SelectionMode::Replace,
                                                                        outline_action: None,
                                                                    });
                                                                }
                                                                report_grow(&mount, mount.grow(&matrix_id, rows));
                                                            },
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            section { "aria-label": "Parts",
                                super::parts::AddObjectComponentChooser {
                                    snapshot,
                                    scope: scope.clone(),
                                    layout_target,
                                    query: parts_query,
                                    on_browse: on_browse_parts,
                                    on_place: EventHandler::new(move |action| {
                                        menu_open.set(false);
                                        on_place_component.call(action);
                                    }),
                                }
                            }
                            if let Some(scope) = scope.as_ref() {
                                AddMenuSection {
                                    heading: "Board geometry".to_owned(),
                                    open: geometry_open(),
                                    on_open_change: move |next| geometry_open.set(next),
                                    AddMenuEntry {
                                        label: "Board outline…",
                                        on_press: {
                                            let scope = scope.clone();
                                            move |_| {
                                                menu_open.set(false);
                                                on_select.call(TreeSelectRequest {
                                                    scope: scope.clone(),
                                                    context: TreeContext::Outline {
                                                        board_id: scope.board_id.clone(),
                                                    },
                                                    mode: SelectionMode::Replace,
                                                    outline_action: None,
                                                });
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Some(projection) = existing_half.projection.clone() {
                ExistingHalfSetup {
                    projection,
                    on_cancel: existing_half.on_cancel,
                    on_create: existing_half.on_create,
                }
            }
        }
}

/// One Add-menu collapsible section: identical trigger/content chrome; only the
/// title, open state and entries vary.
#[component]
fn AddMenuSection(
    heading: String,
    open: bool,
    on_open_change: EventHandler<bool>,
    children: Element,
) -> Element {
    rsx! {
        Collapsible {
            keep_mounted: true,
            open: Some(open),
            on_open_change: move |next| on_open_change.call(next),
            CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", "{heading}"  ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
            CollapsibleContent {
                hidden: !open,
                inert: (!open).then_some(""),
                {children}
            }
        }
    }
}

/// One Add-menu entry: identical Secondary button chrome; only the label, hint,
/// availability and press action vary.
#[component]
fn AddMenuEntry(
    label: String,
    #[props(into, optional)] hint: Option<String>,
    #[props(default = true)] enabled: bool,
    on_press: EventHandler<()>,
) -> Element {
    rsx! {
        Item { class: "p-0",
            Button { variant: ButtonVariant::Secondary,
                button_type: "button",
                disabled: !enabled,
                onclick: move |_| on_press.call(()),
                class: "w-full justify-start whitespace-normal h-auto min-h-control",
                "{label}"
                if let Some(hint) = hint {
                    small { "{hint}" }
                }
            }
        }
    }
}

/// Keep Space activation on controls from starting canvas pan: native control
/// activation must not become the canvas's Space-pan shortcut.
pub fn stop_space_pan(event: KeyboardEvent) {
    if event.key().to_string() == " " {
        event.stop_propagation();
        if let Some(raw) = event.data().try_as_web_event() {
            raw.stop_propagation();
        }
    }
}

fn focus_add_object() {
    use wasm_bindgen::JsCast;
    if let Some(element) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("m1-add-object-trigger"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

fn toggle_tree(mut expanded: Signal<BTreeSet<String>>, id: &str) {
    let mut state = expanded.write();
    if !state.remove(id) {
        state.insert(id.to_owned());
    }
}

fn tree_glyph(kind: TreeKind) -> Element {
    rsx! {
        svg {
            view_box: "0 0 16 16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.3",
            "aria-hidden": "true",
            if kind == TreeKind::Matrix {
                rect {
                    x: "2",
                    y: "2",
                    width: "12",
                    height: "12",
                    rx: "1",
                }
                path { d: "M6 2v12M10 2v12M2 6h12M2 10h12" }
            } else if kind == TreeKind::Row {
                path { d: "M2 4h12M2 8h12M2 12h12" }
            } else if kind == TreeKind::Column {
                path { d: "M8 2v12M5 5l3-3 3 3M5 11l3 3 3-3" }
            } else if kind == TreeKind::Key {
                rect {
                    x: "2",
                    y: "2",
                    width: "12",
                    height: "12",
                    rx: "2",
                }
            } else if kind == TreeKind::Component {
                path { d: "m8 1.8 6.2 6.2L8 14.2 1.8 8z" }
                circle { cx: "8", cy: "8", r: "1.4" }
            } else if kind == TreeKind::Components {
                rect {
                    x: "2",
                    y: "2",
                    width: "8",
                    height: "8",
                    rx: "1",
                }
                rect {
                    x: "6",
                    y: "6",
                    width: "8",
                    height: "8",
                    rx: "1",
                }
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

use tree_projection_logic::AcceptedSource;
pub(crate) use tree_projection_logic::grouped_part_tree;

fn report_grow(mount: &MatrixInspectorMount, result: Result<(), String>) {
    if let Err(message) = result {
        mount.runtime.0.report(message);
    }
}
