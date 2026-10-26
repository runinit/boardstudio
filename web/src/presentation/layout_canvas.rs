//! Read-only Layout canvas projection and scene rendering.
use super::{FootprintGraphics, layout_matrix_relations};
use boardstudio_core::model::{
    Matrix, MatrixScene, Part, PartDefinition, PartKind, Pose2, ProjectDoc, SceneDelta, Side,
    Transform, Vec2,
};
use boardstudio_web_layout::anchor_badge;
use boardstudio_web_layout::screen_scale::px_to_world;
use boardstudio_web_ui_model::split_focus::{self, SplitHalf};
use dioxus::prelude::*;
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

pub(super) fn transform_tools(
    context: &super::objects::TreeContext,
    active: Option<super::objects::LayoutTransformTool>,
    handles_visible: bool,
) -> Vec<super::objects::LayoutTransformTool> {
    use super::objects::{LayoutTransformTool as Tool, TreeContext};
    if !handles_visible {
        // View-only: every passive handle hides, but the active tool keeps
        // the handle its gesture requires. Origin and Splay expose their
        // origin through the active overlay itself, so no extra overlay is
        // needed. With no active tool nothing draws.
        return match (context, active) {
            (TreeContext::Column { .. }, Some(tool)) => vec![tool],
            (TreeContext::Row { .. }, Some(Tool::Offset)) => vec![Tool::Offset],
            (TreeContext::Matrix { .. }, Some(active @ (Tool::Move | Tool::Rotate))) => {
                vec![active]
            }
            _ => Vec::new(),
        };
    }
    match context {
        TreeContext::Column { .. } => match active {
            Some(Tool::Offset) => vec![Tool::Offset, Tool::Splay],
            Some(Tool::Origin) => vec![Tool::Stagger, Tool::Origin],
            Some(Tool::Spread) => vec![Tool::Spread, Tool::Splay],
            _ => vec![Tool::Stagger, Tool::Splay],
        },
        TreeContext::Row { .. } => vec![Tool::Offset],
        TreeContext::Matrix { .. } => vec![Tool::Move, Tool::Rotate],
        _ => vec![],
    }
}

pub(super) fn snap_origins(
    document: &ProjectDoc,
    board_id: &str,
    scene: &SceneDelta,
) -> Vec<super::outline_snapping::Origin> {
    let board_parts: HashSet<_> = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    let definitions: BTreeMap<_, _> = document
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();
    document
        .parts
        .iter()
        .filter(|part| board_parts.contains(part.id.as_str()))
        .map(|part| {
            let definition = definitions.get(part.definition_id.as_str()).copied();
            let keycap = definition
                .filter(|definition| definition.kind == PartKind::Switch)
                .and_then(|definition| part.keycap.or(definition.keycap));
            let local = if let Some(size) = keycap {
                vec![
                    Vec2 {
                        x: -size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: -size.y / 2.0,
                    },
                    Vec2 {
                        x: size.x / 2.0,
                        y: size.y / 2.0,
                    },
                    Vec2 {
                        x: -size.x / 2.0,
                        y: size.y / 2.0,
                    },
                ]
            } else {
                definition.map_or_else(Vec::new, |definition| definition.courtyard.clone())
            };
            // Ticket 12: snap half-frame parts at their evaluated board pose,
            // like the canvas draws them.
            let pose = scene
                .transforms
                .iter()
                .find(|transform| transform.id == part.id)
                .map(|transform| transform.pose)
                .unwrap_or_else(|| boardstudio_core::evaluated_part_pose(document, part));
            let (sin, cos) = pose.rotation.to_radians().sin_cos();
            let world = local
                .into_iter()
                .map(|point| Vec2 {
                    x: pose.at.x + point.x * cos - point.y * sin,
                    y: pose.at.y + point.x * sin + point.y * cos,
                })
                .collect();
            super::outline_snapping::Origin {
                reference: part.reference.clone(),
                center: pose.at,
                polygon: super::outline_snapping::convex_hull(world),
            }
        })
        .collect()
}

#[derive(Clone)]
pub(super) struct SceneSource {
    pub(super) document: Arc<ProjectDoc>,
    pub(super) scene: Arc<SceneDelta>,
    pub(super) board_id: String,
}

impl PartialEq for SceneSource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document, &other.document)
            && Arc::ptr_eq(&self.scene, &other.scene)
            && self.board_id == other.board_id
    }
}

/// Ticket 18: ghost of the previous spacing. After a committed edit changes a
/// split pair's evaluated spacing, this carries the pre-commit scene plus the
/// spacing delta and the pair axis so the canvas can draw the dashed ghost of
/// the keys at the previous spacing with its "+N mm" / "−N mm" axis note.
#[derive(Clone, Debug)]
pub(super) struct SpacingGhost {
    pub(super) scene: Arc<SceneDelta>,
    pub(super) delta_mm: f64,
    pub(super) axis_x: f64,
}

impl PartialEq for SpacingGhost {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.scene, &other.scene)
            && self.delta_mm == other.delta_mm
            && self.axis_x == other.axis_x
    }
}

#[derive(Props, Clone)]
pub(super) struct SceneProps {
    source: SceneSource,
    /// A transform drag is previewing: mark the effective poses amber until
    /// the preview clears. Part of PartialEq so toggling re-renders.
    preview_active: bool,
    /// The accepted scene behind the preview: keys whose accepted pose
    /// differs render a dashed ghost. Only `Some` while previewing.
    accepted_scene: Option<Arc<SceneDelta>>,
    /// Ticket 18: the previous spacing's ghost. Only `Some` while no preview
    /// is up; a drag preview takes precedence and hides the spacing ghost.
    #[props(default)]
    pub(super) spacing_ghost: Option<SpacingGhost>,
    selected_ids: Vec<String>,
    selected_context: Option<super::objects::ScopedTreeContext>,
    /// Ticket 14: view-only editing focus. `Some` dims the other half in
    /// Layout only; `None` (no split, or PCB/Case) draws everything solid.
    #[props(default)]
    focused_half: Option<SplitHalf>,
    /// Ticket 02: the current view scale for fixed screen-size chrome: the
    /// drawn view-box width in canvas units and the surface's CSS pixel
    /// width. Every fixed-size label and ring reads the screen-scale helper
    /// with these instead of a hard-coded world size.
    view_width_world: f64,
    surface_width_px: f64,
    /// Ticket 02: the anchor badge the readout names (computed once by the
    /// caller so the canvas ring and the readout share one projection).
    #[props(default)]
    anchor_badge: Option<anchor_badge::AnchorBadge>,
    keys_visible: bool,
    components_visible: bool,
    keycaps_visible: bool,
    footprints_visible: bool,
    on_matrix_pointer_down: EventHandler<MatrixPointerDown>,
    on_part_pointer_down: EventHandler<PartPointerDown>,
}

impl PartialEq for SceneProps {
    fn eq(&self, other: &Self) -> bool {
        // Props memoization refreshes callbacks even when the geometry does not rerender.
        self.source == other.source
            && self.preview_active == other.preview_active
            && match (&self.accepted_scene, &other.accepted_scene) {
                (Some(mine), Some(theirs)) => Arc::ptr_eq(mine, theirs),
                (None, None) => true,
                _ => false,
            }
            && self.spacing_ghost == other.spacing_ghost
            && self.selected_ids == other.selected_ids
            && self.selected_context == other.selected_context
            && self.focused_half == other.focused_half
            && self.view_width_world == other.view_width_world
            && self.surface_width_px == other.surface_width_px
            && self.anchor_badge == other.anchor_badge
            && self.keys_visible == other.keys_visible
            && self.components_visible == other.components_visible
            && self.keycaps_visible == other.keycaps_visible
            && self.footprints_visible == other.footprints_visible
    }
}

#[component]
pub(super) fn Scene(props: SceneProps) -> Element {
    let document = &props.source.document;
    let scene_data = &props.source.scene;
    let board_part_ids: HashSet<_> = document
        .boards
        .iter()
        .find(|board| board.id == props.source.board_id)
        .into_iter()
        .flat_map(|board| board.part_ids.iter().map(String::as_str))
        .collect();
    let visible_indices: Vec<_> = document
        .parts
        .iter()
        .enumerate()
        .filter(|(_, part)| board_part_ids.contains(part.id.as_str()))
        .map(|(index, _)| index)
        .collect();
    let visible: Vec<_> = visible_indices
        .iter()
        .map(|index| &document.parts[*index])
        .collect();
    let definition_indices = document
        .definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| (definition.id.as_str(), index))
        .collect();
    let definitions = document
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();
    let matrices: Vec<_> = document
        .matrices
        .iter()
        .filter(|matrix| {
            matrix
                .board_id
                .as_deref()
                .is_some_and(|id| id == props.source.board_id)
                || (matrix.board_id.is_none()
                    && (matrix
                        .part_ids
                        .iter()
                        .any(|id| board_part_ids.contains(id.as_str()))
                        || (matrix.part_ids.is_empty() && document.boards.len() <= 1)))
        })
        .collect();
    let matrix_scenes = scene_data
        .matrix_scenes
        .iter()
        .map(|matrix| (matrix.matrix_id.as_str(), matrix))
        .collect();
    // Ticket 18: the ghost source is the accepted scene behind a drag preview,
    // otherwise the pre-commit scene of the previous spacing. The accepted
    // scene always wins when present (a preview takes precedence and hides
    // the spacing ghost while it is up); the caller only passes the spacing
    // ghost while no preview is up.
    let spacing_source: Option<&Arc<SceneDelta>> = (!props.preview_active)
        .then(|| props.spacing_ghost.as_ref().map(|ghost| &ghost.scene))
        .flatten();
    let ghost_source: Option<&Arc<SceneDelta>> = props.accepted_scene.as_ref().or(spacing_source);
    let ghost_matrix_scenes: Option<BTreeMap<&str, &MatrixScene>> = ghost_source.map(|source| {
        source
            .matrix_scenes
            .iter()
            .map(|matrix| (matrix.matrix_id.as_str(), matrix))
            .collect()
    });
    // The axis note only reads while the spacing ghost does: never on a drag
    // preview, where the amber preview ghosts own the canvas.
    let spacing_note: Option<(f64, f64)> = if props.preview_active {
        None
    } else {
        props
            .spacing_ghost
            .as_ref()
            .map(|ghost| (ghost.axis_x, ghost.delta_mm))
    };
    let input = LayoutCanvasInput {
        document,
        visible_indices: &visible_indices,
        definition_indices: &definition_indices,
        visible: &visible,
        definitions: &definitions,
        matrices: &matrices,
        matrix_scenes: &matrix_scenes,
        accepted_matrix_scenes: ghost_matrix_scenes.as_ref(),
        spacing_ghost_note: spacing_note,
        transforms: &scene_data.transforms,
        selected_ids: &props.selected_ids,
        selected_context: props.selected_context.as_ref(),
        focused_half: props.focused_half,
        view_width_world: props.view_width_world,
        surface_width_px: props.surface_width_px,
        anchor_badge: props.anchor_badge.clone(),
        // Ticket 02: key hover (prototype variant D) is view-only chrome: the
        // pointer's part id lives here so entering or leaving a part
        // re-renders only this scene. Amber previews win ties, so the scene
        // only marks a part hovered while no preview claims it.
        hovered_part: use_signal(|| None::<String>),
        keys_visible: props.keys_visible,
        components_visible: props.components_visible,
        keycaps_visible: props.keycaps_visible,
        footprints_visible: props.footprints_visible,
        on_matrix_pointer_down: props.on_matrix_pointer_down,
        on_part_pointer_down: props.on_part_pointer_down,
    };
    // The wrapper stays mounted whether or not a preview is up so toggling
    // the amber mark never rebuilds memoized part graphics.
    rsx! {
        g {
            class: if props.preview_active { "m1-scene-preview" } else { "m1-scene" },
            {scene(input)}
        }
    }
}

pub(super) struct MatrixPointerDown {
    pub(super) event: PointerEvent,
    pub(super) matrix_id: String,
    pub(super) row: u32,
    pub(super) column: u32,
    pub(super) part_id: Option<String>,
}

pub(super) struct PartPointerDown {
    pub(super) event: PointerEvent,
    pub(super) part_id: String,
}

pub(super) struct LayoutCanvasInput<'a> {
    document: &'a Arc<ProjectDoc>,
    visible_indices: &'a [usize],
    definition_indices: &'a BTreeMap<&'a str, usize>,
    pub(super) visible: &'a [&'a Part],
    pub(super) definitions: &'a BTreeMap<&'a str, &'a PartDefinition>,
    pub(super) matrices: &'a [&'a Matrix],
    pub(super) matrix_scenes: &'a BTreeMap<&'a str, &'a MatrixScene>,
    pub(super) accepted_matrix_scenes: Option<&'a BTreeMap<&'a str, &'a MatrixScene>>,
    /// Ticket 18: `(axis_x, delta_mm)` while the spacing ghost reads.
    /// `None` on drag previews and settled scenes, where the ghost rects
    /// above (if any) keep the preview styling.
    pub(super) spacing_ghost_note: Option<(f64, f64)>,
    pub(super) transforms: &'a [Transform],
    pub(super) selected_ids: &'a [String],
    pub(super) selected_context: Option<&'a super::objects::ScopedTreeContext>,
    pub(super) focused_half: Option<SplitHalf>,
    pub(super) view_width_world: f64,
    pub(super) surface_width_px: f64,
    pub(super) anchor_badge: Option<anchor_badge::AnchorBadge>,
    pub(super) hovered_part: Signal<Option<String>>,
    pub(super) keys_visible: bool,
    pub(super) components_visible: bool,
    pub(super) keycaps_visible: bool,
    pub(super) footprints_visible: bool,
    pub(super) on_matrix_pointer_down: EventHandler<MatrixPointerDown>,
    pub(super) on_part_pointer_down: EventHandler<PartPointerDown>,
}

struct Projection<'a> {
    visible_by_id: BTreeMap<&'a str, &'a Part>,
    matrix_keycaps: BTreeMap<&'a str, Vec2>,
    part_transforms: BTreeMap<&'a str, Pose2>,
}

fn project<'a>(
    visible: &'a [&'a Part],
    matrices: &'a [&'a Matrix],
    matrix_scenes: &'a BTreeMap<&'a str, &'a MatrixScene>,
    transforms: &'a [Transform],
) -> Projection<'a> {
    let visible_by_id = visible
        .iter()
        .map(|part| (part.id.as_str(), *part))
        .collect();
    let mut matrix_keycaps = BTreeMap::new();
    for matrix in matrices {
        if let Some(projected) = matrix_scenes.get(matrix.id.as_str()) {
            for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                if let Some(part_id) = cell.member_id.as_deref() {
                    matrix_keycaps
                        .entry(part_id)
                        .or_insert_with(|| resolved_matrix_keycap(None, None, matrix));
                }
            }
        }
    }
    let mut part_transforms = BTreeMap::new();
    for transform in transforms {
        part_transforms
            .entry(transform.id.as_str())
            .or_insert(transform.pose);
    }
    Projection {
        visible_by_id,
        matrix_keycaps,
        part_transforms,
    }
}

pub(super) fn scene(input: LayoutCanvasInput<'_>) -> Element {
    #[cfg(test)]
    SCENE_RENDERS.with(|count| count.set(count.get() + 1));
    let projection = project(
        input.visible,
        input.matrices,
        input.matrix_scenes,
        input.transforms,
    );
    // Ticket 02: one screen-pixel length in canvas units for this view.
    // Every fixed-size label and ring below reads this instead of a
    // hard-coded world size, so chrome stays fixed on screen at every zoom.
    let px = |pixels: f64| px_to_world(pixels, input.view_width_world, input.surface_width_px);
    // Ticket 02: the pointer's part id, or `None` while no amber preview
    // claims the part, so the violet hover below never overrides amber.
    let mut hovered_part = input.hovered_part;
    let hovered_id = hovered_part().filter(|id| !layout_matrix_relations::field_previews(Some(id)));
    // Ticket 14: the half without editing focus dims. Focus is Layout-only
    // workspace state, so a `None` focus (no split, PCB/Case) draws solid.
    let dimmed_half = input
        .focused_half
        .map(|focused| focused.other())
        .filter(|_| split_focus::has_split(input.document));
    // Inline click-to-edit label anchor: the centroid of the dimmed half's
    // enabled cells, drawn a fixed screen gap above its topmost key edge
    // (ticket 02: 10 px, prototype variant D).
    let dim_label_anchor = dimmed_half.and_then(|dimmed| {
        let mut count = 0_u32;
        let (mut sum_x, mut top_y) = (0.0, f64::NEG_INFINITY);
        for matrix in input.matrices {
            if split_focus::matrix_half(input.document, &matrix.id) != Some(dimmed) {
                continue;
            }
            let size = resolved_matrix_keycap(
                None,
                input
                    .definitions
                    .get(matrix.definition_id.as_str())
                    .copied(),
                matrix,
            );
            if let Some(projected) = input.matrix_scenes.get(matrix.id.as_str()) {
                for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                    count += 1;
                    sum_x += cell.pose.at.x;
                    top_y = top_y.max(cell.pose.at.y + size.y / 2.0);
                }
            }
        }
        (count > 0 && top_y.is_finite()).then(|| (dimmed, sum_x / f64::from(count), top_y))
    });
    // Dashed ghosts of the ghost-source key poses, drawn only for matrix keys
    // whose displayed pose moved: the pre-drag pose behind a preview, or the
    // previous spacing behind a spacing change (ticket 18). Computed up
    // front: rsx flow takes no bare `let` bindings.
    let mut ghosts: Vec<(String, u32, u32, Pose2, Vec2)> = Vec::new();
    // Ticket 18: spacing ghosts read grey dashed (prototype variant B), like
    // the dimmed half; drag previews keep the existing ghost styling.
    let ghost_class = if input.spacing_ghost_note.is_some() {
        "m1-spacing-ghost"
    } else {
        "m1-scene-ghost"
    };
    if let Some(accepted_scenes) = input.accepted_matrix_scenes {
        for matrix in input.matrices {
            if let (Some(displayed), Some(accepted)) = (
                input.matrix_scenes.get(matrix.id.as_str()),
                accepted_scenes.get(matrix.id.as_str()),
            ) {
                for cell in accepted.cells.iter().filter(|cell| cell.enabled) {
                    let unchanged = displayed.cells.iter().any(|shown| {
                        shown.enabled
                            && shown.row == cell.row
                            && shown.column == cell.column
                            && shown.pose == cell.pose
                    });
                    if unchanged {
                        continue;
                    }
                    let member = cell
                        .member_id
                        .as_deref()
                        .and_then(|id| projection.visible_by_id.get(id).copied());
                    let definition = match member {
                        Some(part) => input.definitions.get(part.definition_id.as_str()).copied(),
                        None => input
                            .definitions
                            .get(matrix.definition_id.as_str())
                            .copied(),
                    };
                    ghosts.push((
                        matrix.id.clone(),
                        cell.row,
                        cell.column,
                        cell.pose,
                        resolved_matrix_keycap(member, definition, matrix),
                    ));
                }
            }
        }
    }
    // Ticket 02: the anchor badge the caller computed (shared with the
    // "Placed from" readout): a 7 px violet ring at what the selection is
    // placed from, with a violet dashed target and a dashed tie to the
    // selection. It reads the selected half, dimmed or not, and clears with
    // the selection.
    let badge = input.anchor_badge.clone();
    let badge_highlight: Vec<String> = badge
        .as_ref()
        .map(|badge| badge.highlight.clone())
        .unwrap_or_default();
    // Ticket 18: the axis note sits above the displayed keys, so the refit
    // view (which pads the content by 12 mm on every side) always shows it.
    // Ticket 02 draws the note at a fixed 11 px (prototype variant D) a
    // fixed screen gap above the topmost key edge.
    let spacing_note_anchor: Option<(f64, f64, String)> =
        input.spacing_ghost_note.and_then(|(axis_x, delta_mm)| {
            let mut top_y = f64::NEG_INFINITY;
            for matrix in input.matrices {
                let size = resolved_matrix_keycap(
                    None,
                    input
                        .definitions
                        .get(matrix.definition_id.as_str())
                        .copied(),
                    matrix,
                );
                if let Some(projected) = input.matrix_scenes.get(matrix.id.as_str()) {
                    for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                        top_y = top_y.max(cell.pose.at.y + size.y / 2.0);
                    }
                }
            }
            top_y.is_finite().then(|| {
                (
                    axis_x,
                    top_y + px(14.0),
                    boardstudio_web_ui_model::spacing_ghost::spacing_note(delta_mm),
                )
            })
        });
    // Ticket 02: canvas technical labels draw at a fixed 11 px mono at every
    // zoom (prototype variant D), set inline so the size beats the
    // world-unit stylesheet fallback.
    let label_font = px(11.0);
    rsx! {
        if input.keys_visible {
            for matrix in input.matrices {
                if let Some(projected) = input.matrix_scenes.get(matrix.id.as_str()) {
                    for cell in projected.cells.iter().filter(|cell| cell.enabled) {
                        {
                            let member = cell
                                .member_id
                                .as_deref()
                                .and_then(|id| projection.visible_by_id.get(id).copied());
                            let definition = match member {
                                Some(part) => input.definitions.get(part.definition_id.as_str()).copied(),
                                None => input.definitions.get(matrix.definition_id.as_str()).copied(),
                            };
                            let size = resolved_matrix_keycap(member, definition, matrix);
                            let selected = cell
                                .member_id
                                .as_ref()
                                .is_some_and(|id| input.selected_ids.contains(id))
                                || input.selected_context.is_some_and(|selected| {
                                    matches!(
                                        &selected.context,
                                        super::objects::TreeContext::Key { matrix_id, row, column }
                                            if matrix_id == &matrix.id && *row == cell.row && *column == cell.column
                                    )
                                });
                            // Ticket 17: the badge target keeps the violet
                            // dashed reference while the selection stands,
                            // distinct from the transient hover highlight.
                            let anchor_target = cell
                                .member_id
                                .as_ref()
                                .is_some_and(|id| badge_highlight.contains(id));
                            let on_pointer_down = input.on_matrix_pointer_down;
                            let matrix_id = matrix.id.clone();
                            let row = cell.row;
                            let column = cell.column;
                            let part_id = cell.member_id.clone();
                            let dimmed = dimmed_half.is_some_and(|dimmed| {
                                split_focus::matrix_half(input.document, &matrix.id) == Some(dimmed)
                            });
                            // Ticket 02: key hover (prototype variant D). The
                            // scene only marks the key while no amber preview
                            // claims its part, so amber keeps winning ties.
                            let hovered = cell.member_id.as_ref().is_some_and(|id| {
                                hovered_id.as_deref() == Some(id.as_str())
                            });
                            rsx! {
                                layout_matrix_relations::RelationMatrixKey {
                                    key: "{matrix.id}-{cell.row}-{cell.column}",
                                    part_id: cell.member_id.clone(),
                                    selected,
                                    anchor_target,
                                    dimmed,
                                    hovered,
                                    x: -size.x / 2.0,
                                    y: -size.y / 2.0,
                                    width: size.x,
                                    height: size.y,
                                    transform: format!("translate({} {}) rotate({})", cell.pose.at.x, cell.pose.at.y, cell.pose.rotation),
                                    matrix_id: matrix.id.clone(),
                                    row: cell.row,
                                    column: cell.column,
                                    on_pointer_down: move |event: PointerEvent| {
                                        on_pointer_down.call(MatrixPointerDown {
                                            event,
                                            matrix_id: matrix_id.clone(),
                                            row,
                                            column,
                                            part_id: part_id.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for (matrix_id, row, column, pose, size) in &ghosts {
            rect {
                key: "ghost-{matrix_id}-{row}-{column}",
                class: "{ghost_class}",
                x: "{-size.x / 2.0}",
                y: "{-size.y / 2.0}",
                width: "{size.x}",
                height: "{size.y}",
                transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})",
            }
        }
        if let Some((axis_x, note_y, note)) = spacing_note_anchor {
            // Short delta note on the axis above the keys, fixed at 11 px
            // (ticket 02, prototype variant D). The canvas parent flips y, so
            // the text counter-flips to read upright like the split-focus
            // label.
            text {
                key: "spacing-ghost-note-{axis_x}",
                transform: "translate({axis_x} {note_y}) scale(1,-1)",
                text_anchor: "middle",
                class: "m1-spacing-ghost-note",
                style: "font-size:{label_font}px",
                "{note}"
            }
        }
        if let Some((dimmed, label_x, label_top)) = dim_label_anchor {
            // Inline click-to-edit label, fixed at 11 px (ticket 02,
            // prototype variant D). The canvas parent flips y, so the text
            // counter-flips to read upright like part labels.
            text {
                key: "{dimmed.label()}-focus-label",
                transform: "translate({label_x} {label_top + px(10.0)}) scale(1,-1)",
                text_anchor: "middle",
                class: "m1-split-focus-label",
                style: "font-size:{label_font}px",
                "data-dimmed-half": "{dimmed.label()}",
                "{dimmed.dim_label()}"
            }
        }
        if let Some(badge) = badge.as_ref() {
            // Ticket 02 anchor badge (prototype variant D): a 7 px violet
            // ring at the anchor point; the badge label moved out of the
            // canvas into the "Placed from" readout. A dashed tie still runs
            // to the selection base. Read-only chrome: it never takes pointer
            // events, so canvas clicks (including dimmed-half focus) pass
            // through.
            {
                let anchor_x = badge.anchor.x;
                let anchor_y = badge.anchor.y;
                let selection_x = badge.selection.x;
                let selection_y = badge.selection.y;
                let ring_r = px(3.5);
                let ring_w = px(1.5);
                rsx! {
                    if badge.tied {
                        line {
                                                        class: "m1-anchor-tie",
                            x1: "{anchor_x}", y1: "{anchor_y}",
                            x2: "{selection_x}", y2: "{selection_y}",
                        }
                        circle {
                                                        class: "m1-anchor-tie-dot",
                            cx: "{selection_x}", cy: "{selection_y}", r: "0.9",
                        }
                    }
                    g {
                                                class: "m1-anchor-badge",
                        transform: "translate({anchor_x} {anchor_y})",
                        "aria-hidden": "true",
                        circle { class: "m1-anchor-badge-ring", r: "{ring_r}", stroke_width: "{ring_w}" }
                    }
                }
            }
        }
        for (part_index, part) in input.visible_indices.iter().zip(input.visible) {
            {
                let pose = projection
                    .part_transforms
                    .get(part.id.as_str())
                    .copied()
                    .unwrap_or(part.pose);
                let definition = input.definitions.get(part.definition_id.as_str()).copied();
                let selected = input.selected_ids.contains(&part.id);
                let side_transform = if matches!(part.side, Side::Back) { "scale(-1 1)" } else { "" };
                let is_matrix_key = projection.matrix_keycaps.contains_key(part.id.as_str());
                let is_switch = is_matrix_key || definition.is_some_and(|definition| matches!(definition.kind, PartKind::Switch));
                let layer_visible = if is_switch { input.keys_visible } else { input.components_visible };
                let is_encoder = definition.is_some_and(|definition| matches!(definition.kind, PartKind::Encoder) || definition.input_profile.as_ref().is_some_and(|profile| profile.rotary.is_some()));
                let standalone_switch = definition.is_some_and(|definition| matches!(definition.kind, PartKind::Switch));
                let keycap = if is_encoder { None } else if is_matrix_key || standalone_switch {
                    part.keycap
                        .or_else(|| definition.and_then(|definition| definition.keycap))
                        .or_else(|| projection.matrix_keycaps.get(part.id.as_str()).copied())
                } else {
                    None
                };
                let show_keycap = keycap.filter(|_| input.keycaps_visible);
                let footprints_on = input.footprints_visible;
                let on_pointer_down = input.on_part_pointer_down;
                let part_id = part.id.clone();
                let hover_id = part.id.clone();
                let unhover_id = part.id.clone();
                let dimmed = dimmed_half.is_some_and(|dimmed| {
                    split_focus::part_half(input.document, &part.id) == Some(dimmed)
                });
                // Ticket 17: badge targets keep the violet dashed reference on
                // the part outline too, matching the key rect above.
                let anchor_target = badge_highlight.contains(&part.id);
                // Ticket 02: key hover (prototype variant D), set while no
                // amber preview claims the part so amber keeps winning ties.
                let hovered = hovered_id.as_deref() == Some(part.id.as_str());
                // Ticket 02: part names sit at the top of the part, clear of
                // the tie dot: a fixed 12 px screen gap below the courtyard's
                // top edge, at a fixed 11 px.
                let label_top = definition
                    .map(|definition| {
                        definition
                            .courtyard
                            .iter()
                            .map(|point| point.y)
                            .fold(f64::NEG_INFINITY, f64::max)
                    })
                    .filter(|top| top.is_finite())
                    .unwrap_or(0.0);
                let part_label_y = label_top - px(12.0);
                rsx! {
                    if layer_visible || show_keycap.is_some() {
                        g {
                            key: "{part.id}",
                            class: "m1-scene-part",
                            transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}",
                            "data-part-id": "{part.id}",
                            onpointerdown: move |event: PointerEvent| {
                                on_pointer_down.call(PartPointerDown { event, part_id: part_id.clone() });
                            },
                            onpointerenter: move |_| {
                                hovered_part.set(Some(hover_id.clone()));
                            },
                            onpointerleave: move |_| {
                                if hovered_part().as_deref() == Some(unhover_id.as_str()) {
                                    hovered_part.set(None);
                                }
                            },
                            PartContents {
                                source: PartSource {
                                    document: input.document.clone(), part_index: *part_index,
                                    definition_index: input.definition_indices.get(part.definition_id.as_str()).copied(),
                                },
                                selected, dimmed, layer_visible, footprints_on, show_keycap, anchor_target,
                                hovered, label_font, part_label_y,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
struct PartSource {
    document: Arc<ProjectDoc>,
    part_index: usize,
    definition_index: Option<usize>,
}

impl PartialEq for PartSource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document, &other.document)
            && self.part_index == other.part_index
            && self.definition_index == other.definition_index
    }
}

#[component]
fn PartContents(
    source: PartSource,
    selected: bool,
    dimmed: bool,
    layer_visible: bool,
    footprints_on: bool,
    show_keycap: Option<Vec2>,
    #[props(default)] anchor_target: bool,
    /// Ticket 02: the pointer rests on this part (prototype variant D). The
    /// caller only sets it while no amber preview marks the part.
    #[props(default)]
    hovered: bool,
    /// Ticket 02: fixed screen-size part label: 11 px font world size and the
    /// label baseline below the courtyard's top edge.
    label_font: f64,
    part_label_y: f64,
) -> Element {
    #[cfg(test)]
    PART_CONTENT_RENDERS.with(|count| count.set(count.get() + 1));
    let part = &source.document.parts[source.part_index];
    let definition = source
        .definition_index
        .map(|index| &source.document.definitions[index]);
    let courtyard = definition
        .map(|definition| polygon_points(&definition.courtyard))
        .unwrap_or_default();
    // Ticket 02: key hover (prototype variant D) also tints the keycap face;
    // the dimmed half hovers solid muted. Amber previews win ties: the
    // caller only sets `hovered` while no preview marks this part.
    let mut overlay_class = String::from("m1-keycap-overlay");
    if selected {
        overlay_class.push_str(" is-selected");
    }
    if dimmed {
        overlay_class.push_str(" is-dimmed");
    }
    if hovered {
        overlay_class.push_str(" is-hover");
    }
    rsx! {
        if layer_visible {
            layout_matrix_relations::RelationPartOutline {
                key: "{part.id}", part_id: part.id.clone(), selected, dimmed, anchor_target, hovered, points: courtyard
            }
        }
        if layer_visible && footprints_on {
            if let Some(definition) = definition {
                FootprintGraphics { definition: definition.clone(), parameters: part.generator_parameters.clone() }
                for pad in &definition.pads {
                    {
                        let rx = match &pad.shape {
                            boardstudio_core::model::PadShape::Circle | boardstudio_core::model::PadShape::Oval => pad.size.x.min(pad.size.y) / 2.0,
                            boardstudio_core::model::PadShape::Roundrect => pad.size.x.min(pad.size.y) / 4.0,
                            boardstudio_core::model::PadShape::Rect => 0.0,
                        };
                        rsx! {
                            g { transform: "translate({pad.at.x} {pad.at.y}) rotate({pad.rotation.unwrap_or(0.0)})",
                                if pad.plated != Some(false) {
                                    rect { class: "m1-part-pad", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" }
                                }
                                if let Some(drill) = pad.drill { circle { class: "m1-part-drill", r: "{drill / 2.0}" } }
                            }
                        }
                    }
                }
            }
        }
        if let Some(size) = show_keycap {
            {
                let inset = 1.5_f64.min(size.x / 6.0).min(size.y / 6.0);
                rsx! {
                    g { class: "{overlay_class}", "aria-hidden": "true",
                        rect { x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9" }
                        rect { class: "m1-keycap-top", x: "{-size.x / 2.0 + inset}", y: "{-size.y / 2.0 + inset}", width: "{size.x - inset * 2.0}", height: "{size.y - inset * 2.0}", rx: "0.7" }
                    }
                    rect { class: "m1-part-hit-area", x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}" }
                }
            }
        }
        if layer_visible && show_keycap.is_none() {
            // Ticket 02: part names sit at the top of the part, clear of the
            // tie dot, at a fixed 11 px (prototype variant D).
            text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", style: "font-size:{label_font}px", x: "0", y: "{part_label_y}", "{part.reference}" }
        }
    }
}

fn resolved_matrix_keycap(
    part: Option<&Part>,
    definition: Option<&PartDefinition>,
    matrix: &Matrix,
) -> Vec2 {
    part.and_then(|part| part.keycap)
        .or_else(|| definition.and_then(|definition| definition.keycap))
        .unwrap_or(Vec2 {
            x: (matrix.pitch.x - matrix.edge_gap.map(|gap| gap.x).unwrap_or(1.0)).max(1.0),
            y: (matrix.pitch.y - matrix.edge_gap.map(|gap| gap.y).unwrap_or(1.0)).max(1.0),
        })
}

fn polygon_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
thread_local! {
    static SCENE_RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PART_CONTENT_RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{MatrixSceneCell, Transform};
    use boardstudio_core::model::{ProjectDoc, SceneDelta};
    use serde_json::json;
    use std::sync::Arc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn explicit_offset_and_origin_activate_the_named_canvas_handle() {
        use super::super::objects::{LayoutTransformTool as Tool, TreeContext};
        let column = TreeContext::Column {
            matrix_id: "matrix".into(),
            column: 0,
        };
        assert_eq!(
            transform_tools(&column, Some(Tool::Offset), true),
            vec![Tool::Offset, Tool::Splay]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Origin), true),
            vec![Tool::Stagger, Tool::Origin]
        );
        assert_eq!(
            transform_tools(&column, None, true),
            vec![Tool::Stagger, Tool::Splay]
        );
        assert_eq!(
            transform_tools(
                &TreeContext::Row {
                    matrix_id: "matrix".into(),
                    row: 0
                },
                Some(Tool::Stagger),
                true,
            ),
            vec![Tool::Offset]
        );
        assert_eq!(
            transform_tools(
                &TreeContext::Matrix {
                    matrix_id: "matrix".into()
                },
                None,
                true,
            ),
            vec![Tool::Move, Tool::Rotate]
        );
        assert_eq!(
            transform_tools(
                &TreeContext::Matrix {
                    matrix_id: "matrix".into()
                },
                Some(Tool::Move),
                true,
            ),
            vec![Tool::Move, Tool::Rotate]
        );
        assert_eq!(
            transform_tools(
                &TreeContext::Matrix {
                    matrix_id: "matrix".into()
                },
                Some(Tool::Move),
                false,
            ),
            vec![Tool::Move]
        );
    }

    /// Ticket 23: the Handles toggle is view-only. Hidden handles drop every
    /// passive overlay but keep the active tool's required handle; with no
    /// active tool nothing draws.
    #[wasm_bindgen_test]
    fn hidden_handles_keep_only_the_active_tool() {
        use super::super::objects::{LayoutTransformTool as Tool, TreeContext};
        let column = TreeContext::Column {
            matrix_id: "matrix".into(),
            column: 1,
        };
        let row = TreeContext::Row {
            matrix_id: "matrix".into(),
            row: 0,
        };
        let key = TreeContext::Key {
            matrix_id: "matrix".into(),
            row: 0,
            column: 1,
        };
        assert_eq!(transform_tools(&column, None, false), Vec::new());
        assert_eq!(transform_tools(&row, None, false), Vec::new());
        assert_eq!(transform_tools(&key, None, false), Vec::new());
        assert_eq!(
            transform_tools(&column, Some(Tool::Stagger), false),
            vec![Tool::Stagger]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Spread), false),
            vec![Tool::Spread]
        );
        assert_eq!(
            transform_tools(&column, Some(Tool::Origin), false),
            vec![Tool::Origin]
        );
        assert_eq!(
            transform_tools(&row, Some(Tool::Offset), false),
            vec![Tool::Offset]
        );
        // A Row selection never draws a Stagger handle, even hidden.
        assert_eq!(
            transform_tools(&row, Some(Tool::Stagger), false),
            Vec::new()
        );
    }

    fn fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>) {
        let mut document = ProjectDoc::empty("canvas", "Canvas");
        document.parts.push(
            serde_json::from_value(json!({
                "id": "key", "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                "keycap": {"x": 18.0, "y": 18.0}
            }))
            .unwrap(),
        );
        document.definitions.push(
            serde_json::from_value(json!({
                "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        let mut stationary = document.parts[0].clone();
        stationary.id = "stationary".into();
        stationary.pose.at.x = 30.0;
        document.parts.push(stationary);
        document.boards.push(serde_json::from_value(json!({
            "id": "board", "name": "Board", "partIds": ["key", "stationary"], "outlineIds": [], "netIds": [], "thickness": 1.6
        })).unwrap());
        let scene = serde_json::from_value(json!({
            "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
            "matrixScenes": [], "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
            "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
        })).unwrap();
        (Arc::new(document), Arc::new(scene))
    }

    fn render_fixture() -> Element {
        let mut highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| super::super::RelationHighlight(highlight));
        let (document, initial_scene) = use_hook(fixture);
        let mut accepted_document = use_signal(|| document.clone());
        let mut displayed = use_signal(|| initial_scene.clone());
        let mut preview = use_signal(|| false);
        let mut generation = use_signal(|| 0_usize);
        let mut observed = use_signal(|| 0_usize);
        let mut selected = use_signal(Vec::<String>::new);
        let mut keys = use_signal(|| true);
        let captured_generation = generation();
        rsx! {
            button { id: "canvas-unrelated", onclick: move |_| generation += 1, "Unrelated update" }
            button { id: "canvas-highlight", onclick: move |_| highlight.set(Some(vec!["key".into()])), "Highlight relation" }
            button { id: "canvas-preview", onclick: move |_| {
                let mut next = (*displayed()).clone();
                next.transforms = vec![Transform { id: "key".into(), pose: Pose2 { at: Vec2 { x: 9.0, y: 4.0 }, rotation: 0.0 } }];
                displayed.set(Arc::new(next));
                preview.set(true);
            }, "Preview" }
            button { id: "canvas-preview-off", onclick: move |_| {
                displayed.set(initial_scene.clone());
                preview.set(false);
            }, "Clear preview" }
            button { id: "canvas-select", onclick: move |_| selected.set(vec!["key".into()]), "Select" }
            button { id: "canvas-accepted", onclick: move |_| {
                let mut next = accepted_document();
                Arc::make_mut(&mut next).parts[0].keycap = Some(Vec2 { x: 30.0, y: 18.0 });
                accepted_document.set(next);
            }, "Accepted geometry" }
            button { id: "canvas-hide", onclick: move |_| keys.set(false), "Hide" }
            output { id: "canvas-observed", "{observed()}" }
            svg {
                Scene {
                    source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                    preview_active: preview(),
                    accepted_scene: None,
                    selected_ids: selected(), selected_context: None,
                    view_width_world: 200.0, surface_width_px: 800.0,
                    keys_visible: keys(), components_visible: true, keycaps_visible: keys(), footprints_visible: false,
                    on_matrix_pointer_down: |_| {},
                    on_part_pointer_down: move |_| observed.set(captured_generation),
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn canvas_skips_unrelated_updates_but_refreshes_callbacks_preview_selection_and_layers() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(render_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(5).await;
        let renders = SCENE_RENDERS.with(|count| count.get());
        let click = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click()
        };
        click("#canvas-unrelated");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert_eq!(
            SCENE_RENDERS.with(|count| count.get()),
            renders,
            "unchanged canvas inputs must not rebuild the scene"
        );
        let pointer = js_sys::eval("new PointerEvent('pointerdown', {bubbles: true})")
            .unwrap()
            .dyn_into::<web_sys::Event>()
            .unwrap();
        root.query_selector(".m1-part-hit-area")
            .unwrap()
            .unwrap()
            .dispatch_event(&pointer)
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert_eq!(
            root.query_selector("#canvas-observed")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("1"),
            "memoized canvas callbacks must use the latest parent capture"
        );
        click("#canvas-highlight");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            root.query_selector(".m1-part.is-relation-highlight")
                .unwrap()
                .is_some()
        );
        assert_eq!(
            SCENE_RENDERS.with(|count| count.get()),
            renders,
            "relation children remain reactive without rebuilding the canvas"
        );
        let part_renders = PART_CONTENT_RENDERS.with(|count| count.get());
        click("#canvas-preview");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert_eq!(
            PART_CONTENT_RENDERS.with(|count| count.get()),
            part_renders,
            "pose-only previews must not rebuild unchanged part graphics"
        );
        assert_eq!(
            root.query_selector(".m1-scene-part")
                .unwrap()
                .unwrap()
                .get_attribute("transform")
                .as_deref(),
            Some("translate(9,4) rotate(0) ")
        );
        click("#canvas-select");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            root.query_selector(".m1-keycap-overlay.is-selected")
                .unwrap()
                .is_some()
        );
        click("#canvas-accepted");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert_eq!(
            root.query_selector(".m1-part-hit-area")
                .unwrap()
                .unwrap()
                .get_attribute("width")
                .as_deref(),
            Some("30"),
            "accepted geometry must refresh cached graphics"
        );
        click("#canvas-hide");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(root.query_selector(".m1-scene-part").unwrap().is_none());
        root.remove();
    }

    /// Ticket 23 slice 9, RED: a transform-drag preview marks the effective
    /// poses amber until the preview clears.
    #[wasm_bindgen_test]
    async fn transform_preview_marks_the_scene_amber_until_it_clears() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(render_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(5).await;
        let click = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click()
        };
        assert!(
            root.query_selector(".m1-scene-preview").unwrap().is_none(),
            "the settled scene carries no preview mark"
        );
        click("#canvas-preview");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            root.query_selector(".m1-scene-preview").unwrap().is_some(),
            "the preview marks its effective poses amber"
        );
        click("#canvas-preview-off");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            root.query_selector(".m1-scene-preview").unwrap().is_none(),
            "clearing the preview restores the scene"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    fn projection_indexes_visible_parts_enabled_matrix_keycaps_and_first_transform() {
        let visible: Vec<Part> = vec![
            serde_json::from_value(json!({
                "id": "key-1", "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front"
            }))
            .unwrap(),
        ];
        let matrix: Matrix = serde_json::from_value(json!({
            "id": "matrix", "rows": 1, "columns": 1, "pitch": {"x": 19.0, "y": 20.0},
            "origin": {"x": 0.0, "y": 0.0}, "definitionId": "switch", "partIds": ["key-1"],
            "boardId": "board", "edgeGap": {"x": 2.0, "y": 3.0}, "cells": []
        }))
        .unwrap();
        let matrix_scene = MatrixScene {
            matrix_id: "matrix".into(),
            cells: vec![
                MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2 { x: 0.0, y: 0.0 },
                    key_shift: Vec2 { x: 0.0, y: 0.0 },
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                    cumulative_bend: 0.0,
                    row: 0,
                    column: 0,
                    enabled: true,
                    member_id: Some("key-1".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0,
                    },
                },
                MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2 { x: 0.0, y: 0.0 },
                    key_shift: Vec2 { x: 0.0, y: 0.0 },
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                    cumulative_bend: 0.0,
                    row: 0,
                    column: 1,
                    enabled: false,
                    member_id: Some("disabled".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0,
                    },
                },
            ],
            columns: vec![],
            companions: vec![],
        };
        let matrix_scenes = BTreeMap::from([("matrix", &matrix_scene)]);
        let matrices = [&matrix];
        let transforms = [
            Transform {
                id: "key-1".into(),
                pose: Pose2 {
                    at: Vec2 { x: 5.0, y: 6.0 },
                    rotation: 7.0,
                },
            },
            Transform {
                id: "key-1".into(),
                pose: Pose2 {
                    at: Vec2 { x: 8.0, y: 9.0 },
                    rotation: 10.0,
                },
            },
        ];
        let visible_refs: Vec<_> = visible.iter().collect();
        let projection = project(&visible_refs, &matrices, &matrix_scenes, &transforms);

        assert!(projection.visible_by_id.contains_key("key-1"));
        assert_eq!(
            projection.matrix_keycaps.get("key-1"),
            Some(&Vec2 { x: 17.0, y: 17.0 })
        );
        assert!(!projection.matrix_keycaps.contains_key("disabled"));
        assert_eq!(
            projection.part_transforms.get("key-1").unwrap().at,
            Vec2 { x: 5.0, y: 6.0 }
        );
    }
}

#[cfg(test)]
mod ghost_tests {
    use super::*;
    use boardstudio_core::model::SceneDelta;
    use serde_json::json;
    use std::sync::Arc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn ghost_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
        let mut document = ProjectDoc::empty("canvas", "Canvas");
        document.parts.push(
            serde_json::from_value(json!({
                "id": "matrix/m/r0c0", "definitionId": "switch", "reference": "SW1",
                "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                "keycap": {"x": 18.0, "y": 18.0}
            }))
            .unwrap(),
        );
        document.definitions.push(
            serde_json::from_value(json!({
                "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        document.boards.push(
            serde_json::from_value(json!({
                "id": "board", "name": "Board", "partIds": ["matrix/m/r0c0"],
                "outlineIds": [], "netIds": [], "thickness": 1.6
            }))
            .unwrap(),
        );
        document.matrices.push(
            serde_json::from_value(json!({
                "id": "m", "rows": 1, "columns": 1,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 0.0, "y": 0.0},
                "definitionId": "switch", "partIds": ["matrix/m/r0c0"], "boardId": "board",
                "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
            }))
            .unwrap(),
        );
        let document = Arc::new(document);
        let cell = |x: f64, y: f64| {
            json!({
                "row": 0, "column": 0, "enabled": true, "memberId": "matrix/m/r0c0",
                "pose": {"at": {"x": x, "y": y}, "rotation": 0.0},
                "shiftFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}},
                "offsetFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}}
            })
        };
        let scene_with = |x: f64, y: f64| -> Arc<SceneDelta> {
            Arc::new(
                serde_json::from_value(json!({
                    "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
                    "matrixScenes": [{
                        "matrixId": "m", "cells": [cell(x, y)],
                        "columns": [{
                            "column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                            "relativeOrigin": {"x": 0.0, "y": 0.0},
                            "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                            "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                            "staggerAxis": {"x": 0.0, "y": 1.0}
                        }]
                    }],
                    "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
                    "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
                }))
                .unwrap(),
            )
        };
        (document, scene_with(0.0, 0.0), scene_with(9.0, 4.0))
    }

    fn render_ghost_fixture() -> Element {
        let highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| super::super::RelationHighlight(highlight));
        let (document, initial_scene, preview_scene) = use_hook(ghost_fixture);
        let accepted_document = use_signal(|| document.clone());
        let mut displayed = use_signal(|| initial_scene.clone());
        rsx! {
            button { id: "canvas-ghost-preview", onclick: move |_| {
                displayed.set(preview_scene.clone());
            }, "Preview" }
            button { id: "canvas-ghost-off", onclick: move |_| {
                displayed.set(initial_scene.clone());
            }, "Clear preview" }
            svg {
                Scene {
                    source: SceneSource { document: accepted_document(), scene: displayed(), board_id: "board".into() },
                    preview_active: false,
                    accepted_scene: Some(initial_scene.clone()),
                    selected_ids: Vec::new(), selected_context: None,
                    view_width_world: 200.0, surface_width_px: 800.0,
                    keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                    on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
                }
            }
        }
    }

    /// Ticket 23 slice 10, RED: a transform preview ghosts the pre-drag pose
    /// of every moved matrix key until the preview clears.
    #[wasm_bindgen_test]
    async fn transform_preview_ghosts_the_pre_drag_pose() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(render_ghost_fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(5).await;
        let click = |selector: &str| {
            root.query_selector(selector)
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click()
        };
        assert!(
            root.query_selector(".m1-scene-ghost").unwrap().is_none(),
            "the settled scene carries no ghosts"
        );
        click("#canvas-ghost-preview");
        gloo_timers::future::TimeoutFuture::new(5).await;
        let ghost = root
            .query_selector(".m1-scene-ghost")
            .unwrap()
            .expect("the preview ghosts the pre-drag pose");
        assert!(
            ghost
                .get_attribute("transform")
                .as_deref()
                .unwrap_or_default()
                .contains("translate(0 0)"),
            "the ghost sits on the pre-drag pose"
        );
        click("#canvas-ghost-off");
        gloo_timers::future::TimeoutFuture::new(5).await;
        assert!(
            root.query_selector(".m1-scene-ghost").unwrap().is_none(),
            "clearing the preview retires its ghosts"
        );
        root.remove();
    }
}

/// Ticket 18: the spacing ghost renders the previous spacing's keys grey
/// dashed with a "+N mm" / "−N mm" note on the axis, and only while no drag
/// preview is up.
#[cfg(test)]
mod spacing_ghost_render_tests {
    use super::*;
    use boardstudio_core::model::SceneDelta;
    use serde_json::json;
    use std::sync::Arc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn spacing_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>, Arc<SceneDelta>) {
        let mut document = ProjectDoc::empty("canvas", "Canvas");
        for (half, part_id) in [("left", "matrix/left/r0c0"), ("right", "matrix/right/r0c0")] {
            document.parts.push(
                serde_json::from_value(json!({
                    "id": part_id, "definitionId": "switch", "reference": "SW1",
                    "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                    "keycap": {"x": 18.0, "y": 18.0}
                }))
                .unwrap(),
            );
            document.matrices.push(
                serde_json::from_value(json!({
                    "id": half, "rows": 1, "columns": 1,
                    "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 0.0, "y": 0.0},
                    "definitionId": "switch", "partIds": [part_id], "boardId": "board",
                    "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
                }))
                .unwrap(),
            );
        }
        document.definitions.push(
            serde_json::from_value(json!({
                "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        document.boards.push(
            serde_json::from_value(json!({
                "id": "board", "name": "Board",
                "partIds": ["matrix/left/r0c0", "matrix/right/r0c0"],
                "outlineIds": [], "netIds": [], "thickness": 1.6
            }))
            .unwrap(),
        );
        let document = Arc::new(document);
        let cell = |matrix: &str, x: f64| {
            json!({
                "row": 0, "column": 0, "enabled": true,
                "memberId": format!("matrix/{matrix}/r0c0"),
                "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0.0},
                "shiftFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}},
                "offsetFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}}
            })
        };
        let scene_with = |half_gap: f64| -> Arc<SceneDelta> {
            Arc::new(
                serde_json::from_value(json!({
                    "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
                    "matrixScenes": [
                        {"matrixId": "left", "cells": [cell("left", -half_gap)],
                         "columns": [{"column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                                      "relativeOrigin": {"x": 0.0, "y": 0.0},
                                      "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                                      "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                                      "staggerAxis": {"x": 0.0, "y": 1.0}}]},
                        {"matrixId": "right", "cells": [cell("right", half_gap)],
                         "columns": [{"column": 0, "base": {"x": 0.0, "y": 0.0}, "rotation": 0.0,
                                      "relativeOrigin": {"x": 0.0, "y": 0.0},
                                      "splayOrigin": {"x": 0.0, "y": 0.0}, "splayAngle": 0.0, "customOrigin": false,
                                      "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0},
                                      "staggerAxis": {"x": 0.0, "y": 1.0}}]}
                    ],
                    "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
                    "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
                }))
                .unwrap(),
            )
        };
        // Narrow spacing (keys at ±30) and the widened spacing (keys at ±35).
        (document, scene_with(30.0), scene_with(35.0))
    }

    #[derive(Props, Clone, PartialEq)]
    struct SpacingCaseProps {
        document: Arc<ProjectDoc>,
        displayed: Arc<SceneDelta>,
        accepted: Option<Arc<SceneDelta>>,
        ghost: Option<SpacingGhost>,
        preview: bool,
    }

    #[component]
    fn RenderSpacingCase(props: SpacingCaseProps) -> Element {
        let highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| super::super::RelationHighlight(highlight));
        rsx! {
            svg {
                Scene {
                    source: SceneSource { document: props.document, scene: props.displayed, board_id: "board".into() },
                    preview_active: props.preview,
                    accepted_scene: props.accepted,
                    spacing_ghost: props.ghost,
                    selected_ids: Vec::new(), selected_context: None,
                    view_width_world: 200.0, surface_width_px: 800.0,
                    keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                    on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
                }
            }
        }
    }

    async fn mount_case(
        displayed: Arc<SceneDelta>,
        accepted: Option<Arc<SceneDelta>>,
        ghost: Option<SpacingGhost>,
        preview: bool,
    ) -> web_sys::Element {
        let (document, _, _) = spacing_fixture();
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                RenderSpacingCase,
                SpacingCaseProps {
                    document,
                    displayed,
                    accepted,
                    ghost,
                    preview,
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(5).await;
        root
    }

    fn ghost_transforms(root: &web_sys::Element) -> Vec<String> {
        let nodes = root.query_selector_all(".m1-spacing-ghost").unwrap();
        (0..nodes.length())
            .filter_map(|index| {
                nodes
                    .item(index)?
                    .dyn_into::<web_sys::SvgElement>()
                    .ok()?
                    .get_attribute("transform")
            })
            .collect()
    }

    /// RED: widening draws the previous (narrow) keys dashed with a +N note.
    #[wasm_bindgen_test]
    async fn widening_ghosts_the_narrow_keys_with_a_plus_note() {
        let (_, narrow, wide) = spacing_fixture();
        let root = mount_case(
            wide,
            None,
            Some(SpacingGhost {
                scene: narrow,
                delta_mm: 10.0,
                axis_x: 0.0,
            }),
            false,
        )
        .await;
        let transforms = ghost_transforms(&root);
        assert_eq!(
            transforms.len(),
            2,
            "both halves ghost at the previous spacing"
        );
        assert!(
            transforms
                .iter()
                .any(|transform| transform.contains("translate(-30 0)")),
            "the left ghost sits on its narrow pose, got {transforms:?}"
        );
        assert!(
            transforms
                .iter()
                .any(|transform| transform.contains("translate(30 0)")),
            "the right ghost sits on its narrow pose, got {transforms:?}"
        );
        let note = root
            .query_selector(".m1-spacing-ghost-note")
            .unwrap()
            .expect("the axis carries the widening delta");
        assert_eq!(note.text_content().as_deref(), Some("+10.0 mm"));
        // Ticket 02: the note sits a fixed 14 px screen gap above the top
        // key edge: 9.025 half key plus 3.5 world units at this zoom.
        assert!(
            note.get_attribute("transform")
                .as_deref()
                .unwrap_or_default()
                .contains("translate(0 12.5"),
            "the note sits on the axis above the keys"
        );
        root.remove();
    }

    /// RED: narrowing draws the previous (wide) keys dashed with a −N note.
    #[wasm_bindgen_test]
    async fn narrowing_ghosts_the_wide_keys_with_a_minus_note() {
        let (_, narrow, wide) = spacing_fixture();
        let root = mount_case(
            narrow,
            None,
            Some(SpacingGhost {
                scene: wide,
                delta_mm: -6.0,
                axis_x: 0.0,
            }),
            false,
        )
        .await;
        let transforms = ghost_transforms(&root);
        assert_eq!(transforms.len(), 2, "both halves ghost at the wide poses");
        assert!(
            transforms
                .iter()
                .any(|transform| transform.contains("translate(-35 0)")),
            "the left ghost sits on its wide pose, got {transforms:?}"
        );
        let note = root
            .query_selector(".m1-spacing-ghost-note")
            .unwrap()
            .expect("the axis carries the narrowing delta");
        assert_eq!(note.text_content().as_deref(), Some("−6.0 mm"));
        root.remove();
    }

    /// RED: a settled scene with no ghost prop draws no ghost elements.
    #[wasm_bindgen_test]
    async fn settled_scene_without_ghost_draws_nothing() {
        let (_, _, wide) = spacing_fixture();
        let root = mount_case(wide, None, None, false).await;
        assert!(
            root.query_selector_all(".m1-spacing-ghost")
                .unwrap()
                .length()
                == 0,
            "no ghost prop means no ghost rects"
        );
        assert!(
            root.query_selector(".m1-spacing-ghost-note")
                .unwrap()
                .is_none(),
            "no ghost prop means no axis note"
        );
        root.remove();
    }

    /// RED: a drag preview keeps the amber preview ghosts and hides the
    /// spacing ghost and its note.
    #[wasm_bindgen_test]
    async fn drag_preview_preempts_the_spacing_ghost() {
        let (_, narrow, wide) = spacing_fixture();
        let root = mount_case(
            wide,
            Some(narrow.clone()),
            Some(SpacingGhost {
                scene: narrow,
                delta_mm: 10.0,
                axis_x: 0.0,
            }),
            true,
        )
        .await;
        assert!(
            root.query_selector(".m1-scene-ghost").unwrap().is_some(),
            "the preview still ghosts the pre-drag pose"
        );
        assert!(
            root.query_selector_all(".m1-spacing-ghost")
                .unwrap()
                .length()
                == 0,
            "the spacing ghost hides while a preview is up"
        );
        assert!(
            root.query_selector(".m1-spacing-ghost-note")
                .unwrap()
                .is_none(),
            "the axis note hides while a preview is up"
        );
        root.remove();
    }
}

/// Ticket 02: fixed-size canvas chrome (prototype variant D). Technical
/// labels draw at 11 px mono and the anchor ring at 7 px violet at every
/// zoom; the canvas badge label moved into the "Placed from" readout.
#[cfg(test)]
mod screen_chrome_render_tests {
    use super::super::{
        CompactPanelState, Editor, ExportReturnWorkspace, LayerVisibility,
        PreferenceStorageWarning, ResolvedTheme, SelectionAdapter, SetupGuideRequest, ThemeState,
        WorkspaceState, objects, selection,
    };
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::{Event, Session};
    use boardstudio_core::CoreEngine;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use serde_json::json;
    use std::cell::RefCell;
    use std::collections::BTreeSet;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    /// A split pair with a detached diode part: left/right 1x1 switch
    /// matrices under a mirror link, plus `d1` placed from the left half
    /// origin so the canvas draws the dim label, the axis note, the ring
    /// and a part name together.
    fn chrome_fixture() -> (Arc<ProjectDoc>, Arc<SceneDelta>) {
        let mut document = ProjectDoc::empty("chrome", "Chrome");
        for (half, part_id, origin_x) in [
            ("left", "matrix/left/r0c0", -30.0),
            ("right", "matrix/right/r0c0", 30.0),
        ] {
            document.parts.push(
                serde_json::from_value(json!({
                    "id": part_id, "definitionId": "switch", "reference": "SW1",
                    "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0}, "side": "front",
                    "keycap": {"x": 18.0, "y": 18.0}
                }))
                .unwrap(),
            );
            document.matrices.push(
                serde_json::from_value(json!({
                    "id": half, "rows": 1, "columns": 1,
                    "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": origin_x, "y": 0.0},
                    "definitionId": "switch", "partIds": [part_id], "boardId": "board",
                    "cells": [{"row": 0, "column": 0, "enabled": true, "definitionId": "switch", "assemblies": []}]
                }))
                .unwrap(),
            );
        }
        document.parts.push(
            serde_json::from_value(json!({
                "id": "d1", "definitionId": "diode", "reference": "D1",
                "pose": {"at": {"x": -20.0, "y": 10.0}, "rotation": 0.0}, "side": "front",
                "keycap": null
            }))
            .unwrap(),
        );
        document.definitions.push(
            serde_json::from_value(json!({
                "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        document.definitions.push(
            serde_json::from_value(json!({
                "id": "diode", "name": "Diode", "kind": "passive",
                "courtyard": [
                    {"x": -7.0, "y": -7.0}, {"x": 7.0, "y": -7.0},
                    {"x": 7.0, "y": 7.0}, {"x": -7.0, "y": 7.0}
                ],
                "pads": []
            }))
            .unwrap(),
        );
        document.boards.push(
            serde_json::from_value(json!({
                "id": "board", "name": "Board",
                "partIds": ["matrix/left/r0c0", "matrix/right/r0c0", "d1"],
                "outlineIds": [], "netIds": [], "thickness": 1.6
            }))
            .unwrap(),
        );
        document.layouts.push(
            serde_json::from_value(json!({
                "id": "left", "name": "Left half", "boardId": "board",
                "matrixIds": ["left"], "partIds": ["d1"]
            }))
            .unwrap(),
        );
        document.layouts.push(
            serde_json::from_value(json!({
                "id": "right", "name": "Right half", "boardId": "board",
                "matrixIds": ["right"], "partIds": [],
                "mirrorLink": { "sourceId": "left", "axisX": 0.0,
                    "correspondences": [{ "kind": "matrix", "sourceId": "left", "mirrorId": "right" }],
                    "exceptions": [] }
            }))
            .unwrap(),
        );
        let document = Arc::new(document);
        let cell = |matrix: &str, x: f64| {
            json!({
                "row": 0, "column": 0, "enabled": true,
                "memberId": format!("matrix/{matrix}/r0c0"),
                "pose": {"at": {"x": x, "y": 0.0}, "rotation": 0.0},
                "shiftFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}},
                "offsetFrame": {"origin": {"x": 0.0, "y": 0.0}, "rotation": 0.0, "axisX": {"x": 1.0, "y": 0.0}, "axisY": {"x": 0.0, "y": 1.0}}
            })
        };
        let scene = Arc::new(
            serde_json::from_value(json!({
                "revision": 0, "transactionId": "fixture", "changedIds": [], "transforms": [],
                "matrixScenes": [
                    {"matrixId": "left", "cells": [cell("left", -30.0)], "columns": []},
                    {"matrixId": "right", "cells": [cell("right", 30.0)], "columns": []}
                ],
                "contours": [], "boardContours": [], "boardReadiness": [], "findings": [],
                "readiness": {"layout": true, "outline": false, "pcb": false, "case": false}
            }))
            .unwrap(),
        );
        (document, scene)
    }

    fn chrome_badge() -> anchor_badge::AnchorBadge {
        anchor_badge::AnchorBadge {
            anchor: Vec2 { x: -30.0, y: 0.0 },
            selection: Vec2 { x: -11.0, y: 0.0 },
            highlight: Vec::new(),
            label: "Column 1 · spread, stagger, splay carry".to_owned(),
            tied: true,
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct ChromeCaseProps {
        document: Arc<ProjectDoc>,
        scene: Arc<SceneDelta>,
        ghost: SpacingGhost,
        badge: anchor_badge::AnchorBadge,
        view_width: f64,
    }

    #[component]
    fn RenderChromeCase(props: ChromeCaseProps) -> Element {
        let highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| super::super::RelationHighlight(highlight));
        // A real viewport mapping: 800x600 CSS px over the view box, so the
        // browser maps world units to screen pixels and getBBox reads true
        // on-screen sizes.
        let view_box = format!("0 0 {} {}", props.view_width, props.view_width * 0.75);
        rsx! {
            svg { width: "800", height: "600", view_box: "{view_box}",
                Scene {
                    source: SceneSource { document: props.document, scene: props.scene, board_id: "board".into() },
                    preview_active: false,
                    accepted_scene: None,
                    spacing_ghost: Some(props.ghost),
                    selected_ids: Vec::new(), selected_context: None,
                    view_width_world: props.view_width, surface_width_px: 800.0,
                    anchor_badge: Some(props.badge),
                    focused_half: Some(SplitHalf::Left),
                    keys_visible: true, components_visible: true, keycaps_visible: true, footprints_visible: false,
                    on_matrix_pointer_down: |_| {}, on_part_pointer_down: |_| {},
                }
            }
        }
    }

    async fn mount_chrome(view_width: f64) -> web_sys::Element {
        let (document, scene) = chrome_fixture();
        let ghost = SpacingGhost {
            scene: scene.clone(),
            delta_mm: 10.0,
            axis_x: 0.0,
        };
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                RenderChromeCase,
                ChromeCaseProps {
                    document,
                    scene,
                    ghost,
                    badge: chrome_badge(),
                    view_width,
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the fixed-size chrome renders", || {
            root.query_selector(".m1-split-focus-label")
                .unwrap()
                .is_some()
        })
        .await;
        root
    }

    /// World units behind an inline `font-size:…px` style. Browsers may
    /// serialize the declaration with whitespace or a trailing semicolon.
    fn style_world_size(root: &web_sys::Element, selector: &str) -> f64 {
        let node = root
            .query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {selector}"));
        let style = node
            .dyn_into::<web_sys::SvgElement>()
            .unwrap()
            .get_attribute("style")
            .unwrap_or_default();
        style
            .split(';')
            .find_map(|declaration| {
                let (name, value) = declaration.split_once(':')?;
                (name.trim() == "font-size").then(|| value.trim().to_owned())
            })
            .and_then(|value| value.trim_end_matches("px").parse().ok())
            .unwrap_or_else(|| panic!("no font-size in {selector} style {style:?}"))
    }

    /// World units per screen pixel for the mounted surface width.
    fn on_screen(world: f64, view_width: f64) -> f64 {
        world / (view_width / 800.0)
    }

    /// True on-screen height in CSS px through the browser's own view-box
    /// mapping (all transforms included), independent of the helper under
    /// test.
    fn bbox_height(root: &web_sys::Element, selector: &str) -> f64 {
        root.query_selector(selector)
            .unwrap()
            .unwrap_or_else(|| panic!("missing {selector}"))
            .get_bounding_client_rect()
            .height()
    }

    /// Ticket 02 RED: one scene at two zoom levels keeps every technical
    /// label at 11 px on screen. It fails while labels use world units.
    #[wasm_bindgen_test]
    async fn labels_hold_eleven_px_across_zooms() {
        let mut real_heights: Vec<Vec<f64>> = Vec::new();
        for view_width in [200.0, 100.0] {
            let root = mount_chrome(view_width).await;
            for selector in [
                ".m1-split-focus-label",
                ".m1-spacing-ghost-note",
                ".m1-part-label",
            ] {
                let px = on_screen(style_world_size(&root, selector), view_width);
                assert!(
                    (px - 11.0).abs() < 0.5,
                    "{selector} reads {px:.2} px on screen at view width {view_width}, want 11"
                );
            }
            real_heights.push(
                [
                    ".m1-split-focus-label",
                    ".m1-spacing-ghost-note",
                    ".m1-part-label",
                ]
                .iter()
                .map(|selector| bbox_height(&root, selector))
                .collect(),
            );
            assert!(
                root.query_selector(".m1-anchor-badge-label")
                    .unwrap()
                    .is_none(),
                "the canvas carries no badge label at view width {view_width}; it lives in the readout"
            );
            root.remove();
        }
        // True pixels through the browser mapping: the same label measures
        // the same height at both zooms.
        for (wide, close) in real_heights[0].iter().zip(real_heights[1].iter()) {
            assert!(
                (wide - close).abs() < 0.5,
                "label measures {wide:.2} px at one zoom and {close:.2} px at the other"
            );
        }
    }

    /// Ticket 02: the anchor ring holds 7 px on screen at both zooms.
    #[wasm_bindgen_test]
    async fn anchor_ring_holds_seven_px_across_zooms() {
        for view_width in [200.0, 100.0] {
            let root = mount_chrome(view_width).await;
            let ring = root
                .query_selector(".m1-anchor-badge-ring")
                .unwrap()
                .expect("the anchor ring renders");
            let radius: f64 = ring
                .get_attribute("r")
                .unwrap_or_default()
                .parse()
                .expect("the ring radius parses");
            let diameter = on_screen(2.0 * radius, view_width);
            assert!(
                (diameter - 7.0).abs() < 0.5,
                "the ring reads {diameter:.2} px across at view width {view_width}, want 7"
            );
            // True pixels through the browser mapping.
            let real = bbox_height(&root, ".m1-anchor-badge-ring");
            assert!(
                (real - 7.0).abs() < 0.5,
                "the ring measures {real:.2} px across at view width {view_width}, want 7"
            );
            root.remove();
        }
    }

    #[derive(Clone)]
    struct ChromeProbe {
        runtime: Rc<crate::runtime::Runtime>,
        adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    }

    impl ChromeProbe {
        fn select(&self, context: objects::TreeContext) {
            selection::submit_context(
                &self.runtime,
                &self.adapter.borrow().clone().expect("mounted adapter"),
                selection::ContextRequest {
                    scope: self.runtime.scope().expect("opened project scope"),
                    context,
                    mode: boardstudio_application::SelectionMode::Replace,
                },
            );
        }
    }

    fn chrome_host() -> Element {
        let probe = use_context::<ChromeProbe>();
        let workspace = use_signal(|| "Layout");
        use_context_provider(|| WorkspaceState(workspace));
        use_context_provider(|| ExportReturnWorkspace(workspace));
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        let created = use_signal(|| None::<SetupGuideRequest>);
        use_context_provider(|| created);
        let objects_open = use_signal(|| true);
        let inspector_open = use_signal(|| true);
        use_context_provider(|| CompactPanelState {
            objects_open,
            inspector_open,
        });
        let warning = use_signal(|| false);
        use_context_provider(|| PreferenceStorageWarning(warning));
        let theme = use_signal(|| "light");
        use_context_provider(|| ThemeState(theme));
        let resolved = use_memo(|| "light");
        use_context_provider(|| ResolvedTheme(resolved));
        let selected = use_signal(|| None);
        let anchor = use_signal(|| None);
        let generation = use_signal(|| 0u64);
        let adapter = SelectionAdapter::new(selected, anchor, generation);
        *probe.adapter.borrow_mut() = Some(adapter.clone());
        use_context_provider(|| adapter);
        let hidden = use_signal(BTreeSet::new);
        let modules_hidden = use_signal(BTreeSet::new);
        let footprints = use_signal(|| false);
        use_context_provider(|| LayerVisibility {
            hidden,
            modules_hidden,
            footprints,
        });
        boardstudio_web_ui_model::state::use_test_case_generation_state();
        rsx! { Editor {} }
    }

    /// Mount the real Layout editor on the chrome document (empty matrix
    /// scenes: only the detached diode part draws).
    async fn mount_chrome_editor(root_id: &'static str) -> ChromeProbe {
        let (document, _) = chrome_fixture();
        let runtime = support::new_runtime();
        support::install(&runtime, Session::new(), CoreEngine::new());
        support::install_memory_persistence(&runtime);
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document: (*document).clone(),
        });
        support::run_pending(&runtime).await;
        let probe = ChromeProbe {
            runtime: runtime.clone(),
            adapter: Rc::default(),
        };
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        root.set_id(root_id);
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(chrome_host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("the Layout canvas draws the diode part", || {
            dom_document
                .query_selector(&format!("#{root_id} .m1-scene-part"))
                .unwrap()
                .is_some()
        })
        .await;
        probe
    }

    fn readout(root_id: &'static str) -> String {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{root_id} .m1-canvas-anchor-readout"))
            .unwrap()
            .map(|node| node.text_content().unwrap_or_default())
            .unwrap_or_default()
    }

    /// Ticket 02: a part selection names the half origin in the readout, and
    /// clearing the selection clears the readout with the ring.
    #[wasm_bindgen_test]
    async fn part_selection_reads_placed_from_and_clears() {
        let root_id = "chrome-part-readout";
        let probe = mount_chrome_editor(root_id).await;
        probe.select(objects::TreeContext::Component {
            part_id: Some("d1".into()),
            matrix_id: None,
            row: None,
            column: None,
            assembly_id: None,
        });
        wait_until("the readout names the part anchor", || {
            readout(root_id).contains("Left half origin · default")
        })
        .await;
        assert_eq!(
            readout(root_id),
            "Placed from Left half origin · default",
            "the readout carries the full Placed from text"
        );
        let dom_document = web_sys::window().unwrap().document().unwrap();
        assert!(
            dom_document
                .query_selector(&format!("#{root_id} .m1-anchor-badge-ring"))
                .unwrap()
                .is_some(),
            "the 7 px ring marks the anchor while the readout stands"
        );
        probe.select(objects::TreeContext::Board {
            board_id: "board".into(),
        });
        wait_until("clearing the selection clears the readout", || {
            readout(root_id).is_empty()
        })
        .await;
        assert!(
            dom_document
                .query_selector(&format!("#{root_id} .m1-anchor-badge"))
                .unwrap()
                .is_none(),
            "clearing the selection clears the ring with the readout"
        );
    }
}
