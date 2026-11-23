//! Table-driven coverage for the key-size planner.
//!
//! Overlap queries run as a case table over shared envelope fixtures; reflow
//! plans share one `plan_for` runner and one single-column fixture so the tall,
//! assembly, and history variants assert the same derivation without repeating
//! their document, scene, and placement setup.
use super::*;
use boardstudio_model::model::{
    Board, EditCommand, EditOperation, EditPhase, MatrixAssembly, MatrixCell, MatrixColumnBasis,
    MatrixSceneCell, Mirror, Part, Pose2, Side,
};

/// The diode/LED membership the assembly variants share. The history seam
/// replays rotated back-side assemblies; the planner variant is unrotated.
fn assembly_parts(history: bool) -> Vec<MatrixAssembly> {
    ["diode", "led"]
        .into_iter()
        .map(|id| MatrixAssembly {
            id: id.into(),
            definition_id: "switch".into(),
            offset: point(3.0, 4.0),
            rotation: history.then_some(17.0),
            side: history.then_some(Side::Back),
        })
        .collect()
}

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn part(id: &str, x: f64) -> Part {
    Part {
        keycap: Some(point(18.0, 18.0)),
        outline: None,
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: point(x, 0.0),
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn matrix() -> Matrix {
    Matrix {
        id: "m".into(),
        name: Some("Keys".into()),
        rows: 1,
        columns: 3,
        pitch: point(19.0, 19.0),
        origin: point(0.0, 0.0),
        definition_id: "switch".into(),
        part_ids: vec!["a".into(), "b".into(), "c".into()],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: Some(point(1.0, 1.0)),
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![0, 1, 2]
            .into_iter()
            .map(|column| MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            })
            .collect(),
    }
}

fn document(matrix: &Matrix) -> ProjectDoc {
    let mut document = ProjectDoc::empty("project", "Test");
    document.revision = 6;
    document.parts = vec![part("a", 0.0), part("b", 19.0), part("c", 38.0)];
    document.matrices = vec![matrix.clone()];
    document.boards = vec![Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: matrix.part_ids.clone(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }];
    document
}

fn scene_cell(row: u32, column: u32, member: &str, at: Vec2) -> MatrixSceneCell {
    MatrixSceneCell {
        shift_frame: Default::default(),
        offset_frame: Default::default(),
        row_shift: Default::default(),
        key_shift: Default::default(),
        row_bend: 0.0,
        key_bend: 0.0,
        cumulative_shift: Default::default(),
        cumulative_bend: 0.0,
        row,
        column,
        enabled: true,
        member_id: Some(member.into()),
        pose: Pose2 { at, rotation: 0.0 },
    }
}

fn column_basis(column: u32) -> MatrixColumnBasis {
    MatrixColumnBasis {
        base: Default::default(),
        rotation: 0.0,
        relative_origin: Default::default(),
        spread_axis: None,
        column,
        splay_origin: point(0.0, 0.0),
        splay_angle: 0.0,
        custom_origin: false,
        axis_x: point(1.0, 0.0),
        axis_y: point(0.0, 1.0),
        stagger_axis: point(0.0, 1.0),
    }
}

fn scene() -> MatrixScene {
    MatrixScene {
        matrix_id: "m".into(),
        cells: (0..3)
            .map(|column| {
                scene_cell(
                    0,
                    column,
                    ["a", "b", "c"][column as usize],
                    point(column as f64 * 19.0, 0.0),
                )
            })
            .collect(),
        columns: (0..3).map(column_basis).collect(),
        companions: vec![],
    }
}

fn placements_along(vertical: bool) -> Vec<KeycapPlacement> {
    ["a", "b", "c"]
        .into_iter()
        .enumerate()
        .map(|(index, id)| {
            let (row, column, at) = if vertical {
                (index as u32, 0, point(0.0, index as f64 * 19.0))
            } else {
                (0, index as u32, point(index as f64 * 19.0, 0.0))
            };
            KeycapPlacement {
                id: id.into(),
                matrix_id: "m".into(),
                row,
                column,
                at,
                rotation: 0.0,
                size: point(18.0, 18.0),
            }
        })
        .collect()
}

fn placements() -> Vec<KeycapPlacement> {
    placements_along(false)
}

fn overlap_placement(
    id: &str,
    matrix_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    rotation: f64,
) -> KeycapPlacement {
    KeycapPlacement {
        id: id.into(),
        matrix_id: matrix_id.into(),
        row: 0,
        column: 0,
        at: point(x, y),
        rotation,
        size: point(width, height),
    }
}

/// One runner for the single-matrix plan shape every reflow test shares.
fn plan_for(
    document: &ProjectDoc,
    matrix: &Matrix,
    scene: &MatrixScene,
    placements: &[KeycapPlacement],
    selected: &BTreeSet<String>,
    units: Vec2,
    axis: Option<ResizeAxis>,
) -> KeycapResizePlan {
    plan_resize(KeycapResizeInput {
        document,
        matrices: std::slice::from_ref(matrix),
        scenes: std::slice::from_ref(scene),
        layouts: &[],
        placements,
        selected_ids: selected,
        units,
        axis,
    })
    .expect("a supported accepted selection should make one plan")
}

/// Ticket 33: the planner exposes the same resize as typed-edit payloads, and
/// the payloads match the planned document exactly.
fn assert_typed_plan_matches_document(plan: &KeycapResizePlan, document: &ProjectDoc) {
    // Absolute poses match the planned document exactly, in sorted id order.
    let expected_moves: Vec<(String, Vec2)> = plan
        .document
        .parts
        .iter()
        .filter(|part| {
            document
                .parts
                .iter()
                .find(|before| before.id == part.id)
                .is_some_and(|before| before.pose.at != part.pose.at)
        })
        .map(|part| (part.id.clone(), part.pose.at))
        .collect();
    assert_eq!(
        plan.moves
            .iter()
            .map(|position| (position.id.clone(), position.at))
            .collect::<Vec<_>>(),
        expected_moves
    );
    // Every changed cell offset matches the planned matrix exactly.
    let planned: Vec<_> = plan
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "m")
        .unwrap()
        .cells
        .iter()
        .filter(|cell| cell.offset.is_some())
        .map(|cell| (cell.row, cell.column, cell.offset.unwrap()))
        .collect();
    assert_eq!(
        plan.offsets
            .iter()
            .map(|offset| (offset.row, offset.column, offset.offset.unwrap()))
            .collect::<Vec<_>>(),
        planned
    );
    assert!(plan.offsets.iter().all(|offset| offset.matrix_id == "m"));
}

/// One overlap scenario: input envelopes and the pairs that must overlap,
/// in stable order.
type OverlapCase = (Vec<KeycapPlacement>, Vec<(String, String)>);

#[test]
fn overlap_queries_use_oriented_rectangles_and_stable_equal_min_x_order() {
    let cases: Vec<OverlapCase> = vec![
        // Edge contact is not an overlap, but a 0.001 mm crossing is — even
        // across matrices.
        (
            vec![
                overlap_placement("a", "m1", 0.0, 0.0, 18.0, 18.0, 0.0),
                overlap_placement("b", "m2", 18.0, 0.0, 18.0, 18.0, 0.0),
            ],
            vec![],
        ),
        (
            vec![
                overlap_placement("a", "m1", 0.0, 0.0, 18.0, 18.0, 0.0),
                overlap_placement("b", "m2", 17.999, 0.0, 18.0, 18.0, 0.0),
            ],
            vec![("a".into(), "b".into())],
        ),
        // These thin rotated envelopes have overlapping axis-aligned bounds, but
        // their oriented rectangles are separated along the short axis.
        (
            vec![
                overlap_placement("upper", "m1", 0.0, 0.0, 18.0, 4.0, 45.0),
                overlap_placement("lower", "m2", -5.656854, 5.656854, 18.0, 4.0, 45.0),
            ],
            vec![],
        ),
        // Equal minimum-X bounds keep accepted document-parts order.
        (
            vec![
                overlap_placement("first", "m2", 0.0, 0.0, 18.0, 4.0, 45.0),
                overlap_placement("second", "m1", 0.0, 0.0, 18.0, 4.0, 45.0),
                overlap_placement("third", "m3", 0.0, 0.0, 18.0, 4.0, 45.0),
            ],
            vec![
                ("first".into(), "second".into()),
                ("first".into(), "third".into()),
                ("second".into(), "third".into()),
            ],
        ),
    ];
    for (placements, expected) in cases {
        assert_eq!(overlapping_pairs(&placements), expected);
    }
}

#[test]
fn widening_selected_key_reflows_row_neighbors_and_preserves_group_center() {
    let matrix = matrix();
    let document = document(&matrix);
    let placements = placements();
    let plan = plan_for(
        &document,
        &matrix,
        &scene(),
        &placements,
        &BTreeSet::from(["b".into()]),
        point(2.0, 1.0),
        None,
    );

    assert_eq!(plan.document.parts[1].keycap, Some(point(37.0, 18.0)));
    assert!((plan.document.parts[0].pose.at.x + 9.5).abs() < 1e-8);
    assert!((plan.document.parts[2].pose.at.x - 47.5).abs() < 1e-8);
    let matrix = plan
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "m")
        .unwrap();
    let changed_cells: Vec<_> = matrix
        .cells
        .iter()
        .filter(|cell| cell.offset.is_some())
        .collect();
    assert_eq!(changed_cells.len(), 2);
    assert_eq!(plan.document.revision, 6);
    assert_eq!(plan.target_ids, vec!["a", "b", "c", "m"]);
    assert_typed_plan_matches_document(&plan, &document);
}

/// Ticket 33: the planner exposes the same resize as typed-edit payloads,
/// so the controller submits SetKeycapSize instead of ReplaceDocument.
#[test]
fn resize_plan_exposes_typed_keycap_size_sizes_moves_and_offsets() {
    let matrix = matrix();
    let document = document(&matrix);
    let placements = placements();
    let plan = plan_for(
        &document,
        &matrix,
        &scene(),
        &placements,
        &BTreeSet::from(["b".into()]),
        point(2.0, 1.0),
        None,
    );

    assert_eq!(
        plan.sizes,
        BTreeMap::from([("b".to_owned(), point(37.0, 18.0))])
    );
    assert_typed_plan_matches_document(&plan, &document);
}

#[test]
fn axis_resize_preserves_other_axis_for_each_mixed_selected_key() {
    let matrix = matrix();
    let mut document = document(&matrix);
    document.parts[0].keycap = Some(point(18.0, 37.0));
    let mut placements = placements();
    placements[0].size = point(18.0, 37.0);
    let plan = plan_for(
        &document,
        &matrix,
        &scene(),
        &placements,
        &BTreeSet::from(["a".into(), "b".into()]),
        point(2.0, 4.0),
        Some(ResizeAxis::X),
    );
    assert_eq!(plan.document.parts[0].keycap, Some(point(37.0, 37.0)));
    assert_eq!(plan.document.parts[1].keycap, Some(point(37.0, 18.0)));
}

#[test]
fn invalid_units_and_unchanged_selection_create_no_plan() {
    let matrix = matrix();
    let document = document(&matrix);
    let placements = placements();
    for (units, selected) in [
        (point(f64::NAN, 1.0), BTreeSet::from(["b".into()])),
        (point(1.0, 1.0), BTreeSet::from(["b".into()])),
    ] {
        assert!(
            plan_resize(KeycapResizeInput {
                document: &document,
                matrices: std::slice::from_ref(&matrix),
                scenes: &[scene()],
                layouts: &[],
                placements: &placements,
                selected_ids: &selected,
                units,
                axis: None,
            })
            .is_none()
        );
    }
}

/// The single-column document, scene, and placements every vertical reflow
/// test shares. `assemblies` adds diode/LED membership plus companion parts
/// so the assembly-following variant needs no extra setup.
fn tall_column_matrix(assemblies: bool) -> Matrix {
    let mut matrix = matrix();
    matrix.rows = 3;
    matrix.columns = 1;
    matrix.part_ids = vec!["a".into(), "b".into(), "c".into()];
    matrix.cells = (0..3)
        .map(|row| MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 0,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: if assemblies {
                assembly_parts(false)
            } else {
                Vec::new()
            },
            assemblies_local: None,
        })
        .collect();
    matrix
}

fn tall_column_document(matrix: &Matrix, companions: bool) -> ProjectDoc {
    let mut document = document(matrix);
    for (index, part) in document.parts.iter_mut().enumerate() {
        part.pose.at = point(0.0, index as f64 * 19.0);
    }
    if companions {
        for (index, id) in ["a", "b", "c"].into_iter().enumerate() {
            for suffix in ["diode", "led"] {
                let mut companion = part(&format!("{id}/{suffix}"), 3.0);
                companion.pose.at.y = index as f64 * 19.0 + 4.0;
                companion.keycap = None;
                document.parts.push(companion);
            }
        }
        document.boards[0].part_ids = document.parts.iter().map(|part| part.id.clone()).collect();
    } else {
        document.boards[0].part_ids = matrix.part_ids.clone();
    }
    document
}

fn tall_column_scene() -> MatrixScene {
    MatrixScene {
        matrix_id: "m".into(),
        cells: (0..3)
            .map(|row| {
                scene_cell(
                    row,
                    0,
                    ["a", "b", "c"][row as usize],
                    point(0.0, row as f64 * 19.0),
                )
            })
            .collect(),
        columns: vec![column_basis(0)],
        companions: vec![],
    }
}

fn tall_column_placements() -> Vec<KeycapPlacement> {
    placements_along(true)
}

#[test]
fn tall_resize_reflows_same_column_on_projected_y_axis() {
    let matrix = tall_column_matrix(false);
    let document = tall_column_document(&matrix, false);
    let plan = plan_for(
        &document,
        &matrix,
        &tall_column_scene(),
        &tall_column_placements(),
        &BTreeSet::from(["b".into()]),
        point(1.0, 2.0),
        None,
    );
    assert_eq!(plan.document.parts[1].keycap, Some(point(18.0, 37.0)));
    assert!((plan.document.parts[0].pose.at.y + 9.5).abs() < 1e-8);
    assert!((plan.document.parts[2].pose.at.y - 47.5).abs() < 1e-8);
}

#[test]
fn tall_resize_moves_neighbor_assembly_parts_with_their_switch() {
    let matrix = tall_column_matrix(true);
    let document = tall_column_document(&matrix, true);
    let plan = plan_for(
        &document,
        &matrix,
        &tall_column_scene(),
        &tall_column_placements(),
        &BTreeSet::from(["b".into()]),
        point(1.0, 2.0),
        None,
    );
    for id in ["a", "b", "c"] {
        let host = plan
            .document
            .parts
            .iter()
            .find(|part| part.id == id)
            .unwrap();
        for suffix in ["diode", "led"] {
            let companion = plan
                .document
                .parts
                .iter()
                .find(|part| part.id == format!("{id}/{suffix}"))
                .unwrap();
            assert!((companion.pose.at.x - host.pose.at.x - 3.0).abs() < 1e-8);
            assert!(
                (companion.pose.at.y - host.pose.at.y - 4.0).abs() < 1e-8,
                "{id}/{suffix} must follow its switch during key-size reflow"
            );
        }
    }
    assert_eq!(plan.document.parts[1].keycap, Some(point(18.0, 37.0)));
    assert!((plan.document.parts[0].pose.at.y + 9.5).abs() < 1e-8);
    assert!((plan.document.parts[2].pose.at.y - 47.5).abs() < 1e-8);
}

/// The rotated single-column matrix the accepted-history seam replays, with
/// and without a mirror.
fn assembly_history_matrix(mirror: Option<Mirror>) -> Matrix {
    let mut matrix = tall_column_matrix(false);
    matrix.rotation = Some(23.0);
    matrix.mirror = mirror;
    matrix.part_ids.clear();
    for cell in &mut matrix.cells {
        cell.assemblies = assembly_parts(true);
    }
    matrix
}

#[test]
fn accepted_rotated_and_mirrored_resize_preserves_assemblies_through_history() {
    let unpack = |result: Result<boardstudio_core::CommittedScene, String>| {
        let committed = result.expect("Core accepts the fixture edit");
        (committed.document, committed.scene)
    };
    for mirror in [None, Some(Mirror::X)] {
        let mut engine = boardstudio_core::CoreEngine::new();
        let mut initial = ProjectDoc::empty("project", "Assembly resize");
        initial.boards = document(&matrix()).boards;
        initial.boards[0].part_ids.clear();
        initial.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch", "name": "Switch", "kind": "switch",
                "courtyard": [{"x": -3.0, "y": -3.0}, {"x": 3.0, "y": -3.0}, {"x": 3.0, "y": 3.0}],
                "pads": []
            }))
            .unwrap(),
        );
        engine.open(initial).expect("Core opens the fixture");
        let matrix = assembly_history_matrix(mirror);
        let (accepted, scene) = unpack(engine.commit_edit(EditCommand {
            base_revision: 0,
            transaction_id: "create".into(),
            phase: EditPhase::Commit,
            target_ids: vec![matrix.id.clone()],
            operation: EditOperation::SetMatrix {
                matrix,
                definitions: None,
            },
        }));
        let placements: Vec<_> = scene.matrix_scenes[0]
            .cells
            .iter()
            .map(|cell| KeycapPlacement {
                id: cell.member_id.clone().unwrap(),
                matrix_id: "m".into(),
                row: cell.row,
                column: cell.column,
                at: cell.pose.at,
                rotation: cell.pose.rotation,
                size: point(18.0, 18.0),
            })
            .collect();
        let selected = BTreeSet::from([placements[1].id.clone()]);
        let plan = plan_resize(KeycapResizeInput {
            document: &accepted,
            matrices: &accepted.matrices,
            scenes: &scene.matrix_scenes,
            layouts: &accepted.layouts,
            placements: &placements,
            selected_ids: &selected,
            units: point(1.0, 2.0),
            axis: Some(ResizeAxis::Y),
        })
        .unwrap();
        let (saved, saved_scene) = unpack(engine.commit_edit(EditCommand {
            base_revision: accepted.revision,
            transaction_id: "resize".into(),
            phase: EditPhase::Commit,
            // Ticket 33: the planner output commits as the typed edit.
            target_ids: plan.target_ids.clone(),
            operation: EditOperation::SetKeycapSize {
                sizes: plan.sizes.clone(),
                moves: plan.moves.clone(),
                offsets: plan.offsets.clone(),
            },
        }));
        for placement in &placements {
            let original_host = accepted
                .parts
                .iter()
                .find(|part| part.id == placement.id)
                .unwrap();
            let host = saved
                .parts
                .iter()
                .find(|part| part.id == placement.id)
                .unwrap();
            let projected = saved_scene.matrix_scenes[0]
                .cells
                .iter()
                .find(|cell| cell.row == placement.row)
                .unwrap();
            assert!((projected.pose.at.x - host.pose.at.x).abs() < 1e-8);
            assert!((projected.pose.at.y - host.pose.at.y).abs() < 1e-8);
            for suffix in ["diode", "led"] {
                let id = format!("{}/{suffix}", placement.id);
                let before = accepted.parts.iter().find(|part| part.id == id).unwrap();
                let after = saved.parts.iter().find(|part| part.id == id).unwrap();
                assert!(
                    (after.pose.at.x - host.pose.at.x - before.pose.at.x + original_host.pose.at.x)
                        .abs()
                        < 1e-8
                );
                assert!(
                    (after.pose.at.y - host.pose.at.y - before.pose.at.y + original_host.pose.at.y)
                        .abs()
                        < 1e-8,
                    "{id} must retain its assembly offset in the accepted document"
                );
                assert_eq!(after.pose.rotation, before.pose.rotation);
                assert_eq!(after.side, before.side);
            }
        }
        let (undone, _) = unpack(engine.undo());
        assert_eq!(undone.parts, accepted.parts);
        assert_eq!(undone.matrices, accepted.matrices);
        let (redone, _) = unpack(engine.redo());
        assert_eq!(redone.parts, saved.parts);
        assert_eq!(redone.matrices, saved.matrices);
    }
}

/// End-to-end Core seam for linked key resize. Set `KEYCAPS_MIRROR_PROJECT_JSON` to the
/// extracted `project.json` from the pinned Sofle archive to run this opt-in fixture test.
#[test]
#[ignore = "requires KEYCAPS_MIRROR_PROJECT_JSON to point at the pinned archive fixture"]
fn replace_document_reflows_source_and_core_syncs_linked_partner_through_history() {
    let source = std::env::var("KEYCAPS_MIRROR_PROJECT_JSON").unwrap();
    let mut document: ProjectDoc = serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap();
    let left = document
        .layouts
        .iter()
        .find(|layout| layout.id == "left-keys-layout")
        .unwrap()
        .clone();
    let right_index = document
        .layouts
        .iter()
        .position(|layout| layout.id == "right-keys-layout")
        .unwrap();
    let right_matrix_id = document.layouts[right_index].matrix_ids[0].clone();
    document.layouts[right_index].board_id = left.board_id.clone();
    document.layouts[right_index].mirror_link = Some(boardstudio_model::model::LayoutMirrorLink {
        correspondences: vec![],
        exceptions: vec![],
        source_id: left.id.clone(),
        axis_x: 0.0,
        split_gap: None,
        spacing: None,
    });
    let right_matrix_index = document
        .matrices
        .iter()
        .position(|matrix| matrix.id == right_matrix_id)
        .unwrap();
    document.matrices[right_matrix_index].board_id = Some(left.board_id.clone());
    let right_parts: BTreeSet<_> = document
        .boards
        .iter()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .iter()
        .cloned()
        .collect();
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "right")
        .unwrap()
        .part_ids
        .retain(|id| !right_parts.contains(id));
    document
        .boards
        .iter_mut()
        .find(|board| board.id == left.board_id)
        .unwrap()
        .part_ids
        .extend(right_parts);

    let mut engine = boardstudio_core::CoreEngine::new();
    let opened = engine.open(document).expect("linked fixture opens");
    let (accepted, scene) = (opened.document, opened.scene);
    let source_matrix_id = accepted
        .layouts
        .iter()
        .find(|layout| layout.id == "left-keys-layout")
        .unwrap()
        .matrix_ids[0]
        .clone();
    let source_matrix = accepted
        .matrices
        .iter()
        .find(|matrix| matrix.id == source_matrix_id)
        .unwrap();
    let source_scene = scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == source_matrix_id)
        .unwrap();
    let cell = source_scene
        .cells
        .iter()
        .find(|cell| cell.enabled && cell.row == 1 && cell.column == 2)
        .unwrap();
    let selected_id = cell.member_id.clone().unwrap();
    let mut accepted_placements = Vec::new();
    for matrix in &accepted.matrices {
        let Some(matrix_scene) = scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
        else {
            continue;
        };
        for cell in matrix_scene.cells.iter().filter(|cell| cell.enabled) {
            let Some(id) = cell.member_id.as_deref() else {
                continue;
            };
            let Some(part) = accepted.parts.iter().find(|part| part.id == id) else {
                continue;
            };
            if !matrix.part_ids.contains(&part.id) {
                continue;
            }
            let definition = accepted
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let gap = matrix.edge_gap.unwrap_or(point(1.0, 1.0));
            let size = part
                .keycap
                .or_else(|| definition.and_then(|definition| definition.keycap))
                .unwrap_or(point(
                    (matrix.pitch.x - gap.x).max(1.0),
                    (matrix.pitch.y - gap.y).max(1.0),
                ));
            accepted_placements.push(KeycapPlacement {
                id: id.into(),
                matrix_id: matrix.id.clone(),
                row: cell.row,
                column: cell.column,
                at: cell.pose.at,
                rotation: cell.pose.rotation,
                size,
            });
        }
    }
    let units = point(3.0, 1.0);
    let selected_ids = BTreeSet::from([selected_id.clone()]);
    let plan = plan_resize(KeycapResizeInput {
        document: &accepted,
        matrices: &accepted.matrices,
        scenes: &scene.matrix_scenes,
        layouts: &accepted.layouts,
        placements: &accepted_placements,
        selected_ids: &selected_ids,
        units,
        axis: None,
    })
    .expect("linked source selection should make a plan");
    let command = EditCommand {
        base_revision: accepted.revision,
        transaction_id: "keycaps-typed-mirror".into(),
        phase: EditPhase::Commit,
        target_ids: plan.target_ids.clone(),
        // Ticket 33: the linked planner output commits as the typed edit.
        operation: EditOperation::SetKeycapSize {
            sizes: plan.sizes.clone(),
            moves: plan.moves.clone(),
            offsets: plan.offsets.clone(),
        },
    };
    let committed = engine
        .commit_edit(command)
        .expect("typed keycap-size resize commits");
    let (saved, saved_scene) = (committed.document, committed.scene);
    assert_eq!(saved.revision, accepted.revision + 1);
    let partner_layout = saved
        .layouts
        .iter()
        .find(|layout| layout.id == "right-keys-layout")
        .unwrap();
    let original_partner_scene = scene
        .matrix_scenes
        .iter()
        .find(|scene| partner_layout.matrix_ids.contains(&scene.matrix_id))
        .unwrap();
    let partner_scene = saved_scene
        .matrix_scenes
        .iter()
        .find(|scene| partner_layout.matrix_ids.contains(&scene.matrix_id))
        .unwrap();
    let partner_id = partner_scene
        .cells
        .iter()
        .find(|cell| cell.row == 1 && cell.column == 2)
        .unwrap()
        .member_id
        .as_ref()
        .unwrap();
    let source_part = saved
        .parts
        .iter()
        .find(|part| part.id == selected_id)
        .unwrap();
    let partner_part = saved
        .parts
        .iter()
        .find(|part| &part.id == partner_id)
        .unwrap();
    assert_eq!(
        source_part.keycap,
        Some(point(
            3.0 * source_matrix.pitch.x - source_matrix.edge_gap.unwrap_or(point(1.0, 1.0)).x,
            source_matrix.pitch.y - source_matrix.edge_gap.unwrap_or(point(1.0, 1.0)).y
        ))
    );
    assert_eq!(partner_part.keycap, source_part.keycap);
    let axis_x = partner_layout.mirror_link.as_ref().unwrap().axis_x;
    assert!((source_part.pose.at.x + partner_part.pose.at.x - 2.0 * axis_x).abs() < 1e-6);
    assert!((source_part.pose.at.y - partner_part.pose.at.y).abs() < 1e-6);
    let original_selected_pose = cell.pose.at;
    assert!((source_part.pose.at.x - original_selected_pose.x).abs() < 1e-6);
    assert!((source_part.pose.at.y - original_selected_pose.y).abs() < 1e-6);
    for column in [1, 3] {
        let original_cell = source_scene
            .cells
            .iter()
            .find(|cell| cell.enabled && cell.row == 1 && cell.column == column)
            .unwrap();
        let neighbor_id = original_cell.member_id.as_ref().unwrap();
        let original_neighbor = accepted
            .parts
            .iter()
            .find(|part| &part.id == neighbor_id)
            .unwrap();
        let saved_neighbor = saved
            .parts
            .iter()
            .find(|part| &part.id == neighbor_id)
            .unwrap();
        assert!((saved_neighbor.pose.at.x - original_neighbor.pose.at.x).abs() > 1e-6);
        let original_partner_id = original_partner_scene
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == column)
            .unwrap()
            .member_id
            .as_ref()
            .unwrap();
        let saved_partner_id = partner_scene
            .cells
            .iter()
            .find(|cell| cell.row == 1 && cell.column == column)
            .unwrap()
            .member_id
            .as_ref()
            .unwrap();
        let saved_partner = saved
            .parts
            .iter()
            .find(|part| &part.id == saved_partner_id)
            .unwrap();
        assert_eq!(
            saved
                .parts
                .iter()
                .find(|part| part.id == *original_partner_id)
                .unwrap()
                .id,
            *saved_partner_id
        );
        assert!((saved_neighbor.pose.at.x + saved_partner.pose.at.x - 2.0 * axis_x).abs() < 1e-6);
        assert!((saved_neighbor.pose.at.y - saved_partner.pose.at.y).abs() < 1e-6);
    }

    let undone = engine.undo().expect("undo succeeds");
    let undone = undone.document;
    assert_eq!(
        undone
            .parts
            .iter()
            .find(|part| part.id == selected_id)
            .unwrap()
            .keycap,
        accepted
            .parts
            .iter()
            .find(|part| part.id == selected_id)
            .unwrap()
            .keycap
    );
    let redone = engine.redo().expect("redo succeeds");
    let redone = redone.document;
    assert_eq!(
        redone
            .parts
            .iter()
            .find(|part| part.id == selected_id)
            .unwrap()
            .keycap,
        source_part.keycap
    );
}
