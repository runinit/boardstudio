//! Read-only Layout canvas projection and scene rendering.
use super::{FootprintGraphics, layout_matrix_relations};
use boardstudio_core::model::{
    Matrix, MatrixScene, Part, PartDefinition, PartKind, Pose2, ProjectDoc, SceneDelta, Side,
    Transform, Vec2,
};
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
        _ => vec![],
    }
}

pub(super) fn snap_origins(
    document: &ProjectDoc,
    board_id: &str,
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
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            let world = local
                .into_iter()
                .map(|point| Vec2 {
                    x: part.pose.at.x + point.x * cos - point.y * sin,
                    y: part.pose.at.y + point.x * sin + point.y * cos,
                })
                .collect();
            super::outline_snapping::Origin {
                reference: part.reference.clone(),
                center: part.pose.at,
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

#[derive(Props, Clone)]
pub(super) struct SceneProps {
    source: SceneSource,
    /// A transform drag is previewing: mark the effective poses amber until
    /// the preview clears. Part of PartialEq so toggling re-renders.
    preview_active: bool,
    /// The accepted scene behind the preview: keys whose accepted pose
    /// differs render a dashed ghost. Only `Some` while previewing.
    accepted_scene: Option<Arc<SceneDelta>>,
    selected_ids: Vec<String>,
    selected_context: Option<super::objects::ScopedTreeContext>,
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
            && self.selected_ids == other.selected_ids
            && self.selected_context == other.selected_context
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
    let accepted_matrix_scenes: Option<BTreeMap<&str, &MatrixScene>> =
        props.accepted_scene.as_ref().map(|accepted| {
            accepted
                .matrix_scenes
                .iter()
                .map(|matrix| (matrix.matrix_id.as_str(), matrix))
                .collect()
        });
    let input = LayoutCanvasInput {
        document,
        visible_indices: &visible_indices,
        definition_indices: &definition_indices,
        visible: &visible,
        definitions: &definitions,
        matrices: &matrices,
        matrix_scenes: &matrix_scenes,
        accepted_matrix_scenes: accepted_matrix_scenes.as_ref(),
        transforms: &scene_data.transforms,
        selected_ids: &props.selected_ids,
        selected_context: props.selected_context.as_ref(),
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
    pub(super) transforms: &'a [Transform],
    pub(super) selected_ids: &'a [String],
    pub(super) selected_context: Option<&'a super::objects::ScopedTreeContext>,
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
    // Dashed ghosts of accepted (pre-drag) key poses, drawn only for matrix
    // keys whose preview pose moved. Computed up front: rsx flow takes no
    // bare `let` bindings.
    let mut ghosts: Vec<(String, u32, u32, Pose2, Vec2)> = Vec::new();
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
                            let on_pointer_down = input.on_matrix_pointer_down;
                            let matrix_id = matrix.id.clone();
                            let row = cell.row;
                            let column = cell.column;
                            let part_id = cell.member_id.clone();
                            rsx! {
                                layout_matrix_relations::RelationMatrixKey {
                                    key: "{matrix.id}-{cell.row}-{cell.column}",
                                    part_id: cell.member_id.clone(),
                                    selected,
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
                class: "m1-scene-ghost",
                x: "{-size.x / 2.0}",
                y: "{-size.y / 2.0}",
                width: "{size.x}",
                height: "{size.y}",
                transform: "translate({pose.at.x} {pose.at.y}) rotate({pose.rotation})",
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
                            PartContents {
                                source: PartSource {
                                    document: input.document.clone(), part_index: *part_index,
                                    definition_index: input.definition_indices.get(part.definition_id.as_str()).copied(),
                                },
                                selected, layer_visible, footprints_on, show_keycap,
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
    layer_visible: bool,
    footprints_on: bool,
    show_keycap: Option<Vec2>,
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
    rsx! {
        if layer_visible {
            layout_matrix_relations::RelationPartOutline {
                key: "{part.id}", part_id: part.id.clone(), selected, points: courtyard
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
                    g { class: if selected { "m1-keycap-overlay is-selected" } else { "m1-keycap-overlay" }, "aria-hidden": "true",
                        rect { x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}", rx: "0.9" }
                        rect { class: "m1-keycap-top", x: "{-size.x / 2.0 + inset}", y: "{-size.y / 2.0 + inset}", width: "{size.x - inset * 2.0}", height: "{size.y - inset * 2.0}", rx: "0.7" }
                    }
                    rect { class: "m1-part-hit-area", x: "{-size.x / 2.0}", y: "{-size.y / 2.0}", width: "{size.x}", height: "{size.y}" }
                }
            }
        }
        if layer_visible && show_keycap.is_none() {
            text { transform: "scale(1,-1)", text_anchor: "middle", class: "m1-part-label", x: "0", y: "-5.2", "{part.reference}" }
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
