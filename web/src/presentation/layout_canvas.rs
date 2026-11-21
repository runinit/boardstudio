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

/// Ticket 07: `part` carries the mount site's part-rail facts for an
/// independent part selection (a `Component` with a part id and no matrix).
/// Matrix members and companions name themselves in the facts and keep their
/// matrix's tools.
pub(super) fn transform_tools(
    context: &super::objects::TreeContext,
    active: Option<super::objects::LayoutTransformTool>,
    handles_visible: bool,
    part: Option<boardstudio_web_layout::part_rail::PartRailFacts>,
) -> Vec<super::objects::LayoutTransformTool> {
    use super::objects::{LayoutTransformTool as Tool, TreeContext};
    use boardstudio_web_layout::part_rail::part_rail_tools;
    let part_tools = |facts: boardstudio_web_layout::part_rail::PartRailFacts| {
        part_rail_tools(facts)
            .into_iter()
            .map(Tool::from)
            .collect::<Vec<_>>()
    };
    if !handles_visible {
        // View-only: every passive handle hides, but the active tool keeps
        // the handle its gesture requires. Origin and Splay expose their
        // origin through the active overlay itself, so no extra overlay is
        // needed. With no active tool nothing draws.
        return match (context, active) {
            (TreeContext::Column { .. }, Some(tool)) => vec![tool],
            (TreeContext::Row { .. }, Some(Tool::Offset)) => vec![Tool::Offset],
            (TreeContext::Row { .. }, Some(Tool::Shift)) => vec![Tool::Shift],
            (TreeContext::Row { .. }, Some(Tool::Bend)) => vec![Tool::Bend],
            (TreeContext::Key { .. }, Some(Tool::Shift)) => vec![Tool::Shift],
            (TreeContext::Key { .. }, Some(Tool::Bend)) => vec![Tool::Bend],
            (TreeContext::Matrix { .. }, Some(active @ (Tool::Move | Tool::Rotate))) => {
                vec![active]
            }
            // Ticket 07: a hidden part rail keeps only its active tool.
            (TreeContext::Component { .. }, Some(tool)) => part.map_or_else(Vec::new, |facts| {
                let tools = part_tools(facts);
                if tools.contains(&tool) {
                    vec![tool]
                } else {
                    Vec::new()
                }
            }),
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
        TreeContext::Row { .. } => vec![Tool::Offset, Tool::Shift, Tool::Bend],
        TreeContext::Key { .. } => vec![Tool::Shift, Tool::Bend],
        // Ticket 08: the rail of a matrix ends in the Anchor token. The
        // mount site only mounts it while the matrix is anchored (the
        // projection arms no pick without an anchor to retarget).
        TreeContext::Matrix { .. } => vec![Tool::Move, Tool::Rotate, Tool::Anchor],
        // Ticket 07: an independent part gets its own rail; without facts
        // (a member or companion) nothing draws.
        TreeContext::Component { .. } => part.map_or_else(Vec::new, part_tools),
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
            class: if props.preview_active { "[&_[data-matrix-id]]:stroke-warning [&_[data-geometry=keycap]_rect]:stroke-warning" } else { "" },
            "data-scene-preview": "{props.preview_active}",
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
        "pointer-events-none fill-none stroke-border-strong stroke-[0.3] [stroke-dasharray:0.8_0.8]"
    } else {
        "pointer-events-none fill-none stroke-foreground stroke-[0.3] [stroke-dasharray:3_2] opacity-60"
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
                "data-ghost": if input.spacing_ghost_note.is_some() { "spacing" } else { "scene" },
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
                class: "pointer-events-none fill-warning font-mono",
                "data-ghost-note": "spacing",
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
                class: "pointer-events-none fill-muted-foreground font-mono",
                "data-focus-label": "split",
                style: "font-size:{label_font}px",
                "data-dimmed-half": "{dimmed.label()}",
                "{dimmed.dim_label()}"
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
                            class: "cursor-grab",
                            transform: "translate({pose.at.x},{pose.at.y}) rotate({pose.rotation}) {side_transform}",
                            "data-geometry": "scene-part",
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
        // Ticket 02 anchor badge (prototype variant D): a 7 px violet ring
        // at the anchor point; the badge label moved out of the canvas into
        // the "Placed from" readout. A dashed tie still runs to the selection
        // base. Read-only chrome: it never takes pointer events, so canvas
        // clicks (including dimmed-half focus) pass through. It paints after
        // the parts so the ring stays visible when the anchor sits under a
        // key, matching the prototype's ring-on-top treatment.
        if let Some(badge) = badge.as_ref() {
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
                            class: "pointer-events-none stroke-geometry-part stroke-[0.4] [stroke-dasharray:1.2_0.8]",
                            "data-anchor-chrome": "tie",
                            x1: "{anchor_x}", y1: "{anchor_y}",
                            x2: "{selection_x}", y2: "{selection_y}",
                        }
                        circle {
                            class: "pointer-events-none fill-geometry-part",
                            "data-anchor-chrome": "tie-dot",
                            cx: "{selection_x}", cy: "{selection_y}", r: "0.9",
                        }
                    }
                    g {
                        class: "pointer-events-none",
                        "data-anchor-chrome": "badge",
                        transform: "translate({anchor_x} {anchor_y})",
                        "aria-hidden": "true",
                        circle { class: "pointer-events-none fill-card stroke-geometry-part", "data-anchor-chrome": "ring", r: "{ring_r}", stroke_width: "{ring_w}" }
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
    let overlay_class = if selected {
        "pointer-events-none [&_rect]:fill-selected [&_rect]:stroke-primary [&_rect]:[stroke-width:0.22] [&_rect]:[fill-opacity:0.75]"
    } else if hovered && dimmed {
        "pointer-events-none [&_rect]:fill-muted-foreground [&_rect]:stroke-muted-foreground [&_rect]:[stroke-width:1.5px] [&_rect]:[vector-effect:non-scaling-stroke]"
    } else if hovered {
        "pointer-events-none [&_rect]:fill-selected/60 [&_rect]:stroke-primary [&_rect]:[stroke-width:1.5px] [&_rect]:[vector-effect:non-scaling-stroke]"
    } else {
        "pointer-events-none [&_rect]:fill-geometry-body [&_rect]:stroke-geometry-outline [&_rect]:[stroke-width:0.22] [&_rect]:[fill-opacity:0.75]"
    };
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
                                    rect { class: "fill-geometry-part stroke-none", x: "{-pad.size.x / 2.0}", y: "{-pad.size.y / 2.0}", width: "{pad.size.x}", height: "{pad.size.y}", rx: "{rx}" }
                                }
                                if let Some(drill) = pad.drill { circle { class: "fill-canvas stroke-geometry-outline stroke-[0.1]", r: "{drill / 2.0}" } }
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
                    g { class: "{overlay_class}", "aria-hidden": "true", "data-geometry": "keycap", "data-selected": "{selected}", "data-dimmed": "{dimmed}", "data-hovered": "{hovered}",
                        rect { x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9" }
                        rect { class: "[fill-opacity:0.25]! [stroke-width:0.14]!", x: "{-size.x / 2.0 + inset}", y: "{-size.y / 2.0 + inset}", width: "{size.x - inset * 2.0}", height: "{size.y - inset * 2.0}", rx: "0.7" }
                    }
                    rect { class: "cursor-grab fill-transparent stroke-none [pointer-events:all]", "data-geometry": "part-hit-area", x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}" }
                }
            }
        }
        if layer_visible && show_keycap.is_none() {
            // Ticket 02: part names sit at the top of the part, clear of the
            // tie dot, at a fixed 11 px (prototype variant D).
            text { transform: "scale(1,-1)", text_anchor: "middle", class: "pointer-events-none fill-geometry-label font-mono", style: "font-size:{label_font}px", "data-part-label": "", x: "0", y: "{part_label_y}", "{part.reference}" }
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
mod tests;

#[cfg(test)]
mod ghost_tests;

/// Ticket 18: the spacing ghost renders the previous spacing's keys grey
/// dashed with a "+N mm" / "−N mm" note on the axis, and only while no drag
/// preview is up.
#[cfg(test)]
mod spacing_ghost_render_tests;

/// Ticket 02: fixed-size canvas chrome (prototype variant D). Technical
/// labels draw at 11 px mono and the anchor ring at 7 px violet at every
/// zoom; the canvas badge label moved into the "Placed from" readout.
#[cfg(test)]
mod screen_chrome_render_tests;
