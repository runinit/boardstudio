//! Ticket 12: parts in the half frame, split clamp, mirrored controller by default.
//! All behavior runs through public Core routes (`Open` / `Edit` / `Undo`).
use boardstudio_core::{CoreEngine, model::*};

fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

fn open(doc: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, doc) = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }));
    (engine, scene, doc)
}

fn edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    scene(engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn undone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]},
        {"id":"mcu","name":"Controller","kind":"controller",
         "courtyard":[{"x":-9.0,"y":-16.5},{"x":9.0,"y":-16.5},{"x":9.0,"y":16.5},{"x":-9.0,"y":16.5}],
         "pads":[]}
    ]))
    .unwrap()
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn key_part(id: &str, x: f64, y: f64) -> Part {
    Part {
        anchor: None,
        id: id.into(),
        definition_id: "switch".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn controller_part(id: &str, stored_x: f64, y: f64) -> Part {
    Part {
        anchor: None,
        id: id.into(),
        definition_id: "mcu".into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2 { x: stored_x, y },
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

fn matrix_1x1(id: &str, origin_x: f64, mirror: Option<Mirror>) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2 {
            x: origin_x,
            y: 0.0,
        },
        definition_id: "switch".into(),
        part_ids: vec![member(id, 0, 0)],
        board_id: Some("board".into()),
        mirror,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn mirror_kind(source_id: &str, mirror_id: &str) -> MirroredCorrespondence {
    MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source_id.into(),
        mirror_id: mirror_id.into(),
        companion_mode: None,
        companion_tweak: None,
    }
}

/// Stored-gap pair (gap 40 about axis 40): half offsets 20 (source) and 60
/// (mirror). Keys are 1x1 at board 11 and 69; no independent parts yet.
fn gapped_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("half-parts", "Half parts");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![member("main", 0, 0), member("main-mirror", 0, 0)],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(key_part(&member("main", 0, 0), 11.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 0), 69.0, 0.0));
    doc.matrices.push(matrix_1x1("main", -9.0, None));
    doc.matrices
        .push(matrix_1x1("main-mirror", 9.0, Some(Mirror::X)));
    doc.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main-mirror".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 40.0,
            split_gap: Some(40.0),
            spacing: None,
            correspondences: vec![mirror_kind("main", "main-mirror")],
            exceptions: vec![],
        }),
    });
    doc
}

fn link_of(doc: &ProjectDoc) -> LayoutMirrorLink {
    doc.layouts
        .iter()
        .find(|layout| layout.id == "right")
        .expect("mirror layout")
        .mirror_link
        .clone()
        .expect("mirror link")
}

fn part_of(doc: &ProjectDoc, id: &str) -> Part {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .clone()
}

fn transform_of(scene: &SceneDelta, id: &str) -> Pose2 {
    scene
        .transforms
        .iter()
        .find(|transform| transform.id == id)
        .unwrap_or_else(|| panic!("missing transform {id}"))
        .pose
}

fn part_correspondences(doc: &ProjectDoc) -> Vec<MirroredCorrespondence> {
    link_of(doc)
        .correspondences
        .into_iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .collect()
}

#[test]
fn adding_a_controller_mirrors_it_at_the_reflected_pose() {
    let (mut engine, _, doc) = open(gapped_doc());
    // Board-space drop point on the source half; Core stores it half-local.
    let (_, mirrored) = edit(
        &mut engine,
        &doc,
        EditOperation::AddPart {
            part: controller_part("u1", 25.0, 20.0),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    );
    // The source copy keeps half-frame storage: board 25 on offset 20.
    assert_eq!(part_of(&mirrored, "u1").pose.at, Vec2 { x: 5.0, y: 20.0 });
    // A linked partner exists on the mirror half at the reflected pose.
    let records = part_correspondences(&mirrored);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source_id, "u1");
    let mirror_id = records[0].mirror_id.clone();
    let partner = part_of(&mirrored, &mirror_id);
    assert_eq!(partner.pose.at, Vec2 { x: -5.0, y: 20.0 });
    // The scene shows both copies at their evaluated board poses.
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(transform_of(&current, "u1").at, Vec2 { x: 25.0, y: 20.0 });
    assert_eq!(
        transform_of(&current, &mirror_id).at,
        Vec2 { x: 55.0, y: 20.0 }
    );
}

#[test]
fn moving_toward_the_axis_clamps_widens_and_keeps_half_frame_values() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, mirrored) = edit(
        &mut engine,
        &doc,
        EditOperation::AddPart {
            part: controller_part("u1", 25.0, 20.0),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    );
    // Dragging past the axis stops at it: the 18 mm wide controller touches
    // x = 40 with its board center at 31 (half-frame 11).
    let (moved_scene, moved) = edit(
        &mut engine,
        &mirrored,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "u1".into(),
                at: Vec2 { x: 38.0, y: 20.0 },
            }],
        },
    );
    assert_eq!(
        part_of(&moved, "u1").pose.at,
        Vec2 { x: 11.0, y: 20.0 },
        "the drag stops at the split axis"
    );
    // The touching outline needs the other half out: deficit 12 - 0, so the
    // spacing widens 40 -> 52 in the same Undo entry.
    assert_eq!(link_of(&moved).split_gap, Some(40.0));
    assert_eq!(link_of(&moved).spacing, Some(52.0));
    // Both copies keep their half-frame values; the evaluated poses stay
    // reflected about the axis.
    let mirror_id = part_correspondences(&moved)[0].mirror_id.clone();
    assert_eq!(
        part_of(&moved, &mirror_id).pose.at,
        Vec2 { x: -11.0, y: 20.0 }
    );
    assert_eq!(
        transform_of(&moved_scene, "u1").at,
        Vec2 { x: 25.0, y: 20.0 }
    );
    assert_eq!(
        transform_of(&moved_scene, &mirror_id).at,
        Vec2 { x: 55.0, y: 20.0 }
    );
    // One Undo entry restores the move and the spacing together.
    let restored = undone(&mut engine);
    assert_eq!(part_of(&restored, "u1").pose.at, Vec2 { x: 5.0, y: 20.0 });
    assert_eq!(link_of(&restored).spacing, None);
}

#[test]
fn dragging_the_mirror_copy_edits_both_without_crossing() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, mirrored) = edit(
        &mut engine,
        &doc,
        EditOperation::AddPart {
            part: controller_part("u1", 25.0, 20.0),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    );
    let mirror_id = part_correspondences(&mirrored)[0].mirror_id.clone();
    // Dragging the mirror copy deep across the axis clamps it to a touch
    // from its own side and writes the source copy through.
    let (_, moved) = edit(
        &mut engine,
        &mirrored,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: mirror_id.clone(),
                at: Vec2 { x: -30.0, y: 20.0 },
            }],
        },
    );
    assert_eq!(
        part_of(&moved, &mirror_id).pose.at,
        Vec2 { x: -11.0, y: 20.0 }
    );
    assert_eq!(part_of(&moved, "u1").pose.at, Vec2 { x: 11.0, y: 20.0 });
    assert_eq!(link_of(&moved).spacing, Some(52.0));
}

#[test]
fn mirror_toggle_removes_and_recreates_the_partner_in_one_undo_each_way() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, mirrored) = edit(
        &mut engine,
        &doc,
        EditOperation::AddPart {
            part: controller_part("u1", 25.0, 20.0),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    );
    let mirror_id = part_correspondences(&mirrored)[0].mirror_id.clone();
    // Making the part local removes the partner; the source copy is untouched.
    let (_, local) = edit(
        &mut engine,
        &mirrored,
        EditOperation::SetMirroredPart {
            part_id: "u1".into(),
            mirrored: false,
        },
    );
    assert!(part_correspondences(&local).is_empty());
    assert!(local.parts.iter().all(|part| part.id != mirror_id));
    assert_eq!(part_of(&local, "u1").pose.at, Vec2 { x: 5.0, y: 20.0 });
    // Undo restores the partner and the link together.
    let restored = undone(&mut engine);
    assert_eq!(part_correspondences(&restored).len(), 1);
    assert_eq!(
        part_of(&restored, &mirror_id).pose.at,
        Vec2 { x: -5.0, y: 20.0 }
    );
    // Re-mirroring recreates the partner at the reflected pose.
    let (_, local_again) = edit(
        &mut engine,
        &restored,
        EditOperation::SetMirroredPart {
            part_id: "u1".into(),
            mirrored: false,
        },
    );
    assert!(part_correspondences(&local_again).is_empty());
    let (remirrored_scene, remirrored) = edit(
        &mut engine,
        &local_again,
        EditOperation::SetMirroredPart {
            part_id: "u1".into(),
            mirrored: true,
        },
    );
    let records = part_correspondences(&remirrored);
    assert_eq!(records.len(), 1);
    let partner = part_of(&remirrored, &records[0].mirror_id);
    assert_eq!(partner.pose.at, Vec2 { x: -5.0, y: 20.0 });
    assert_eq!(
        transform_of(&remirrored_scene, &records[0].mirror_id).at,
        Vec2 { x: 55.0, y: 20.0 }
    );
    let restored = undone(&mut engine);
    assert!(part_correspondences(&restored).is_empty());
}

/// A legacy board-space pair adopts the half frame on its first edit: poses
/// convert pose-preservingly, the legacy mirror link gains a Part
/// correspondence, and the evaluated geometry does not move.
#[test]
fn legacy_pair_parts_adopt_the_half_frame_on_first_edit() {
    let mut doc = ProjectDoc::empty("legacy-parts", "Legacy parts");
    doc.definitions = definitions();
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![
            member("main", 0, 0),
            member("main-mirror", 0, 0),
            "u1".into(),
            "right::mirror::u1".into(),
        ],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.parts.push(key_part(&member("main", 0, 0), 16.0, 0.0));
    doc.parts
        .push(key_part(&member("main-mirror", 0, 0), 64.0, 0.0));
    let source = controller_part("u1", 25.0, 20.0);
    let mut mirror = controller_part("right::mirror::u1", 55.0, 20.0);
    mirror.reference = "U1_M".into();
    mirror
        .properties
        .get_or_insert_with(Default::default)
        .insert(
            "boardstudio.mirroredComponentSource".into(),
            serde_json::json!("u1"),
        );
    doc.parts.push(source);
    doc.parts.push(mirror);
    doc.matrices.push(matrix_1x1("main", 16.0, None));
    doc.matrices
        .push(matrix_1x1("main-mirror", 64.0, Some(Mirror::X)));
    doc.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec!["u1".into()],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main-mirror".into()],
        part_ids: vec!["right::mirror::u1".into()],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 40.0,
            split_gap: None,
            spacing: None,
            correspondences: vec![mirror_kind("main", "main-mirror")],
            exceptions: vec![],
        }),
    });

    let (mut engine, _, doc) = open(doc);
    // Nudging a key is the first committed edit: the pair adopts.
    let (_, adopted) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 0),
                at: Vec2 { x: 17.0, y: 0.0 },
            }],
        },
    );
    // Implied gap 2 * (40 - 16) - 18 = 30; halves at 25 and 55.
    assert_eq!(link_of(&adopted).split_gap, Some(30.0));
    assert_eq!(link_of(&adopted).spacing, None);
    // Poses convert board -> half-local, preserving the evaluated geometry.
    assert_eq!(part_of(&adopted, "u1").pose.at, Vec2 { x: 0.0, y: 20.0 });
    assert_eq!(
        part_of(&adopted, "right::mirror::u1").pose.at,
        Vec2 { x: 0.0, y: 20.0 }
    );
    // The legacy mirror link is now a recorded Part correspondence.
    let records = part_correspondences(&adopted);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source_id, "u1");
    assert_eq!(records[0].mirror_id, "right::mirror::u1");
    let current = scene(engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }))
    .0;
    assert_eq!(transform_of(&current, "u1").at, Vec2 { x: 25.0, y: 20.0 });
    assert_eq!(
        transform_of(&current, "right::mirror::u1").at,
        Vec2 { x: 55.0, y: 20.0 }
    );
    // One Undo restores the legacy board-space pair.
    let restored = undone(&mut engine);
    assert_eq!(link_of(&restored).split_gap, None);
    assert_eq!(part_of(&restored, "u1").pose.at, Vec2 { x: 25.0, y: 20.0 });
    assert!(part_correspondences(&restored).is_empty());
}

#[test]
fn a_local_part_at_the_axis_pushes_the_other_half_out() {
    let (mut engine, _, doc) = open(gapped_doc());
    let (_, mirrored) = edit(
        &mut engine,
        &doc,
        EditOperation::AddPart {
            part: controller_part("u1", 25.0, 20.0),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    );
    let (_, local) = edit(
        &mut engine,
        &mirrored,
        EditOperation::SetMirroredPart {
            part_id: "u1".into(),
            mirrored: false,
        },
    );
    // The local copy drags to the axis; its overhang still counts, so the
    // mirror half moves out and the stored value never crosses. The mirror
    // half carries keys only, so the touching outline needs the deficit
    // 2 * (40 + 3 - 40) = 6: spacing 40 -> 46 in the same Undo entry.
    let (moved_scene, moved) = edit(
        &mut engine,
        &local,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "u1".into(),
                at: Vec2 { x: 38.0, y: 20.0 },
            }],
        },
    );
    assert_eq!(part_of(&moved, "u1").pose.at, Vec2 { x: 11.0, y: 20.0 });
    assert_eq!(link_of(&moved).spacing, Some(46.0));
    assert_eq!(
        transform_of(&moved_scene, "u1").at,
        Vec2 { x: 28.0, y: 20.0 }
    );
}
