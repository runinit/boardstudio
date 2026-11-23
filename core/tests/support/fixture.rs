//! Small builders used by more than one integration test.

use boardstudio_core::CoreEngine;
use boardstudio_core::model::{
    Constraint, EditOperation, MatrixAssembly, MatrixCell, MechanicalCriticalFit,
    MechanicalHardwareSpecification, MechanicalOpening, MechanicalOpeningVertex, Mount, MountKind,
    Operation, OutlineFeature, Position, ProjectDoc, Vec2,
};
use serde_json::{Value, json};

pub(crate) fn rect(id: &str, center: Vec2, size: Vec2, operation: Operation) -> OutlineFeature {
    serde_json::from_value(json!({
        "id": id, "kind": "rect", "center": center, "size": size,
        "radius": 0, "operation": operation
    }))
    .unwrap()
}

/// One enabled key cell carrying the diode/led companion pair.
pub(crate) fn companion_cell(row: u32, column: u32) -> MatrixCell {
    MatrixCell {
        row,
        column,
        assemblies: vec![
            MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 5.0 },
                rotation: None,
                side: None,
            },
            MatrixAssembly {
                id: "led".into(),
                definition_id: "led".into(),
                offset: Vec2 { x: -3.0, y: 6.0 },
                rotation: None,
                side: None,
            },
        ],
        ..crate::support::fixture::cell(row, column)
    }
}

/// Move one part to an absolute position.
pub(crate) fn move_key(id: &str, x: f64, y: f64) -> EditOperation {
    EditOperation::MoveParts {
        positions: vec![Position {
            id: id.into(),
            at: Vec2 { x, y },
        }],
    }
}

/// Move several parts to absolute positions.
pub(crate) fn move_parts(moves: &[(&str, f64, f64)]) -> EditOperation {
    EditOperation::MoveParts {
        positions: moves
            .iter()
            .map(|(id, x, y)| Position {
                id: id.to_string(),
                at: Vec2 { x: *x, y: *y },
            })
            .collect(),
    }
}

/// An offset constraint between two parts at rest.
pub(crate) fn offset_constraint(
    id: &str,
    source_part_id: &str,
    target_part_id: &str,
    x: f64,
    y: f64,
) -> Constraint {
    Constraint::Offset {
        id: id.into(),
        source_part_id: source_part_id.into(),
        target_part_id: target_part_id.into(),
        offset: Vec2 { x, y },
        rotation: 0.0,
    }
}

/// Remove parts by id.
pub(crate) fn remove_parts(ids: &[&str]) -> EditOperation {
    EditOperation::RemoveParts {
        ids: ids.iter().map(|id| id.to_string()).collect(),
    }
}

/// Replace the whole document.
pub(crate) fn replace_document(document: ProjectDoc) -> EditOperation {
    EditOperation::ReplaceDocument {
        document: Box::new(document),
    }
}

/// A typed part-envelope outline feature.
pub(crate) fn part_envelope(
    id: &str,
    part_ids: &[&str],
    margin: f64,
    operation: Operation,
) -> OutlineFeature {
    OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: id.into(),
        part_ids: part_ids.iter().map(|id| id.to_string()).collect(),
        margin,
        operation,
    }
}

/// A typed polygonal outline feature anchored to a part or matrix key.
pub(crate) fn anchored_polygon(
    id: &str,
    anchor_part_id: &str,
    points: &[(f64, f64)],
    operation: Operation,
) -> OutlineFeature {
    OutlineFeature::Polygon {
        anchor_part_id: Some(anchor_part_id.into()),
        id: id.into(),
        points: points.iter().map(|(x, y)| Vec2 { x: *x, y: *y }).collect(),
        operation,
    }
}

/// Sparse identity seeds shared by the mechanical monotonicity checks: the
/// stored records carry only an id so allocation must assign fresh ones.
pub(crate) fn sparse_opening(id: &str) -> MechanicalOpening {
    MechanicalOpening {
        id: id.into(),
        vertices: vec![
            MechanicalOpeningVertex {
                id: "vertex-1".into(),
                point: Vec2 { x: 0.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-3".into(),
                point: Vec2 { x: 5.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-4".into(),
                point: Vec2 { x: 5.0, y: 5.0 },
            },
        ],
        z: 0.0,
        height: 1.0,
        next_vertex_identity: 0,
    }
}

pub(crate) fn sparse_fit(id: &str) -> MechanicalCriticalFit {
    MechanicalCriticalFit {
        id: id.into(),
        part_id: "p".into(),
        label: "fit".into(),
        from: Vec2 { x: 0.0, y: 0.0 },
        to: Vec2 { x: 1.0, y: 0.0 },
        tolerance: "±0.2 mm".into(),
    }
}

pub(crate) fn sparse_hardware(id: &str) -> MechanicalHardwareSpecification {
    MechanicalHardwareSpecification {
        id: id.into(),
        part_id: "p".into(),
        feature_id: "f".into(),
        designation: "screw".into(),
        thread: "M2".into(),
        length: 6.0,
        quantity: 1,
        notes: None,
        tolerance: None,
    }
}

pub(crate) fn sparse_mount(id: &str) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x: 0.0, y: 0.0 },
        kind: MountKind::Hole,
        hole_diameter: 2.5,
        boss_diameter: None,
        height: None,
    }
}

/// One scene cell of a typed matrix projection.
#[track_caller]
pub(crate) fn typed_scene_cell<'a>(
    scene: &'a boardstudio_core::model::SceneDelta,
    matrix: &str,
    row: u32,
    column: u32,
) -> &'a boardstudio_core::model::MatrixSceneCell {
    crate::support::fixture::matrix_scene(scene, matrix)
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .unwrap_or_else(|| panic!("missing cell ({row},{column}) in {matrix}"))
}

/// Assert a typed scene cell pose within 1e-9, pinning the carried geometry.
#[track_caller]
pub(crate) fn assert_typed_cell(
    scene: &boardstudio_core::model::SceneDelta,
    matrix: &str,
    row: u32,
    column: u32,
    x: f64,
    y: f64,
    rotation: f64,
) {
    let cell = typed_scene_cell(scene, matrix, row, column);
    let (ax, ay, ar) = (cell.pose.at.x, cell.pose.at.y, cell.pose.rotation);
    assert!(
        (ax - x).abs() < 1e-9 && (ay - y).abs() < 1e-9 && (ar - rotation).abs() < 1e-9,
        "cell {matrix}({row},{column}): expected ({x}, {y}, {rotation}), got ({ax}, {ay}, {ar})"
    );
}

/// The scene pose of one matrix key inside wire scene projections.
pub(crate) fn scene_pose(scenes: &Value, matrix: &str, row: u64, column: u64) -> (f64, f64, f64) {
    let scene = scenes
        .as_array()
        .unwrap()
        .iter()
        .find(|scene| scene["matrixId"] == matrix)
        .unwrap_or_else(|| panic!("missing scene {matrix}"));
    let cell = scene["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == row && cell["column"] == column)
        .unwrap_or_else(|| panic!("missing cell ({row},{column}) in {scene}"));
    (
        cell["pose"]["at"]["x"].as_f64().unwrap(),
        cell["pose"]["at"]["y"].as_f64().unwrap(),
        cell["pose"]["rotation"].as_f64().unwrap(),
    )
}

/// Open a wire document on a fresh engine, asserting the scene reply.
#[track_caller]
pub(crate) fn open_wire_engine(document: Value) -> (CoreEngine, Value) {
    let mut engine = CoreEngine::new();
    let reply: Value = serde_json::from_str(
        &engine.request(&json!({"kind": "open", "id": "open", "document": document}).to_string()),
    )
    .unwrap();
    assert_eq!(reply["kind"], "scene", "{reply}");
    (engine, reply)
}

/// Request the draft matrix projection for already-opened wire matrices.
#[track_caller]
pub(crate) fn draft_projection(engine: &mut CoreEngine, matrices: &Value) -> Value {
    let reply: Value = serde_json::from_str(
        &engine
            .request(
                &json!({"kind": "project-matrices", "id": "draft", "baseRevision": 0, "matrices": matrices})
                    .to_string(),
            ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "matrix-projections", "{reply}");
    reply
}

/// Resolve a modules document against the shared host board, asserting success.
#[track_caller]
pub(crate) fn resolve_modules(engine: &mut CoreEngine, id: &str, document: &Value) -> Value {
    let reply: Value = serde_json::from_str(
        &engine.request(
            &json!({"id": id, "kind": "resolve-modules", "document": document, "boardId": "host"})
                .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(reply["kind"], "modules-resolved", "{reply}");
    reply
}

/// Run an `import-module-board` artifact request, asserting success.
#[track_caller]
pub(crate) fn import_module_board(
    definition_id: &str,
    name: &str,
    source: &str,
    provenance: &Value,
    family: &str,
    variant: &str,
    repair: Option<&str>,
) -> Value {
    let mut request = json!({
        "id": "import", "kind": "import-module-board", "definitionId": definition_id,
        "name": name, "source": source, "provenance": provenance,
        "family": family, "variant": variant,
    });
    if let Some(repair) = repair {
        request["repair"] = repair.into();
    }
    let reply: Value =
        serde_json::from_str(&boardstudio_core::artifact::request(&request.to_string())).unwrap();
    assert_eq!(reply["kind"], "import-module-board", "{reply}");
    reply["result"].clone()
}

/// A document layout record on the shared host board with no loose parts.
pub(crate) fn layout_record(matrix_ids: &[&str]) -> Value {
    json!({"id": "layout", "name": "Layout", "boardId": "board",
        "matrixIds": matrix_ids, "partIds": []})
}

/// A copy of a wire document with its constraint list cleared.
pub(crate) fn unconstrained(mut document: Value) -> Value {
    document["constraints"] = json!([]);
    document
}
