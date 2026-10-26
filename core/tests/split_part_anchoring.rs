//! Ticket 13: parts anchored with the column vocabulary (anchor key + Follow,
//! Spread/Stagger/Splay carried, Offset/Rotation local). All behavior runs
//! through public Core routes (`Open` / `Edit` / `Undo`).
use boardstudio_core::{CoreEngine, evaluated_part_pose, model::*};

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

fn redone(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Redo { id: "redo".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene: {other:?}"),
    }
}

fn edit_error(engine: &mut CoreEngine, doc: &ProjectDoc, operation: EditOperation) -> String {
    match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: doc.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected error, got: {other:?}"),
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
        anchor: None,
    }
}

fn controller_part(id: &str, stored_x: f64, y: f64) -> Part {
    Part {
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
        anchor: None,
    }
}

fn matrix_3x6(id: &str, origin_x: f64, mirror: Option<Mirror>) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 3,
        columns: 6,
        pitch: Vec2 { x: 19.05, y: 19.05 },
        origin: Vec2 {
            x: origin_x,
            y: 0.0,
        },
        definition_id: "switch".into(),
        part_ids: (0..3)
            .flat_map(|row| (0..6).map(move |column| member(id, row, column)))
            .collect(),
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
/// (mirror). Main is 3x6; R3C6 sits at board (106.25, 38.1).
fn pair_doc() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("part-anchor", "Part anchor");
    doc.definitions = definitions();
    let mut board_part_ids = vec![];
    for matrix in ["main", "main-mirror"] {
        for row in 0..3 {
            for column in 0..6 {
                board_part_ids.push(member(matrix, row, column));
            }
        }
    }
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: board_part_ids,
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    // Member key parts ride the matrix refresh at board-space poses.
    for (matrix, offset) in [("main", 20.0), ("main-mirror", 60.0)] {
        let origin_x = if matrix == "main" { -9.0 } else { 9.0 };
        for row in 0..3 {
            for column in 0..6 {
                // Column base x steps by pitch; rows step by pitch; the mirror
                // half reflects x through its own frame.
                let half_x = origin_x + f64::from(column) * 19.05;
                let half_y = f64::from(row) * 19.05;
                let (board_x, board_y) = if matrix == "main" {
                    (half_x + offset, half_y)
                } else {
                    // Mirror::X negates the local x before the half offset.
                    (offset - half_x, half_y)
                };
                doc.parts
                    .push(key_part(&member(matrix, row, column), board_x, board_y));
            }
        }
    }
    doc.matrices.push(matrix_3x6("main", -9.0, None));
    doc.matrices
        .push(matrix_3x6("main-mirror", 9.0, Some(Mirror::X)));
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

fn part_of(doc: &ProjectDoc, id: &str) -> Part {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .clone()
}

/// Board-space pose of an independent part.
fn board_pose(doc: &ProjectDoc, id: &str) -> Pose2 {
    evaluated_part_pose(doc, &part_of(doc, id))
}

fn part_records(doc: &ProjectDoc) -> Vec<MirroredCorrespondence> {
    doc.layouts
        .iter()
        .find(|layout| layout.id == "right")
        .expect("mirror layout")
        .mirror_link
        .clone()
        .expect("mirror link")
        .correspondences
        .into_iter()
        .filter(|record| record.kind == MirroredItemKind::Part)
        .collect()
}

/// Drop near Main R3C6 (board 106.25, 38.1): 10 mm above the key.
fn drop_u1(engine: &mut CoreEngine, doc: &ProjectDoc) -> ProjectDoc {
    edit(
        engine,
        doc,
        EditOperation::AddPart {
            part: controller_part("u1", 106.25, 48.1),
            board_id: Some("board".into()),
            definition: None,
            layout_id: Some("left".into()),
        },
    )
    .1
}

#[test]
fn drop_anchors_to_the_nearest_key_and_keeps_the_drop_point_upright() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let part = part_of(&dropped, "u1");
    let anchor = part.anchor.clone().expect("dropped part anchors");
    assert_eq!(anchor.matrix_id, "main");
    assert_eq!((anchor.row, anchor.column), (2, 5));
    assert!(anchor.follow.x && anchor.follow.y && anchor.follow.rotation);
    // The evaluated board pose is the drop point, upright.
    let pose = board_pose(&dropped, "u1");
    assert_eq!(pose.at, Vec2 { x: 106.25, y: 48.1 });
    assert_eq!(pose.rotation, 0.0);
}

fn pick_r2c5(engine: &mut CoreEngine, doc: &ProjectDoc) -> ProjectDoc {
    edit(
        engine,
        doc,
        EditOperation::SetPartAnchor {
            part_id: "u1".into(),
            anchor: PartAnchor {
                matrix_id: "main".into(),
                row: 1,
                column: 4,
                follow: Follow {
                    x: true,
                    y: true,
                    rotation: true,
                },
                spread: 0.0,
                stagger: 0.0,
                splay: 0.0,
                offset: Vec2::default(),
                rotation: 0.0,
            },
        },
    )
    .1
}

#[test]
fn pick_moves_the_part_from_the_new_anchor_and_is_one_undo_entry() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    // Give the part carried Spread so the Pick has stored values to keep.
    let spread = edit(
        &mut engine,
        &dropped,
        EditOperation::SetPartAnchorValues {
            part_id: "u1".into(),
            change: PartAnchorValueChange::Spread(4.0),
        },
    )
    .1;
    let before = board_pose(&spread, "u1");
    let stored = part_of(&spread, "u1").anchor.expect("spread anchor");
    assert_eq!((stored.row, stored.column), (2, 5));
    assert_eq!(stored.spread, 4.0);
    // Pick R2C5 with zeroed values: the stored carried/local values win and
    // the part moves to the fresh placement from the new anchor key.
    let picked = pick_r2c5(&mut engine, &spread);
    let anchor = part_of(&picked, "u1").anchor.expect("picked anchor");
    assert_eq!(anchor.matrix_id, "main");
    assert_eq!((anchor.row, anchor.column), (1, 4));
    assert!(anchor.follow.x && anchor.follow.y && anchor.follow.rotation);
    assert_eq!(anchor.spread, 4.0);
    assert_eq!(anchor.stagger, stored.stagger);
    assert_eq!(anchor.splay, 0.0);
    assert_eq!(anchor.offset, Vec2::default());
    assert_eq!(anchor.rotation, 0.0);
    // ADR-0038 placement from R2C5's key pose with the kept values.
    let key = board_pose(&picked, &member("main", 1, 4));
    assert_eq!(
        board_pose(&picked, "u1"),
        Pose2 {
            at: Vec2 {
                x: key.at.x + 4.0,
                y: key.at.y + stored.stagger,
            },
            rotation: key.rotation,
        }
    );
    assert_ne!(board_pose(&picked, "u1"), before);
    // One Undo entry restores the old anchor and pose.
    let undone_doc = undone(&mut engine);
    let restored = part_of(&undone_doc, "u1")
        .anchor
        .expect("undo restores the drop anchor");
    assert_eq!((restored.row, restored.column), (2, 5));
    assert_eq!(restored.spread, 4.0);
    assert_eq!(board_pose(&undone_doc, "u1"), before);
}

#[test]
fn pick_moves_the_mirror_partner_by_the_mirrored_amount() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let partner_id = part_records(&dropped)[0].mirror_id.clone();
    let spread = edit(
        &mut engine,
        &dropped,
        EditOperation::SetPartAnchorValues {
            part_id: "u1".into(),
            change: PartAnchorValueChange::Spread(4.0),
        },
    )
    .1;
    let stored = part_of(&spread, "u1").anchor.expect("spread anchor");
    let picked = pick_r2c5(&mut engine, &spread);
    let source_pose = board_pose(&picked, "u1");
    let anchor = part_of(&picked, &partner_id)
        .anchor
        .expect("partner anchor");
    // The partner maps to the counterpart key with mirrored values.
    assert_eq!(anchor.matrix_id, "main-mirror");
    assert_eq!((anchor.row, anchor.column), (1, 4));
    assert_eq!(anchor.spread, -4.0);
    assert_eq!(anchor.stagger, stored.stagger);
    // The partner pose mirrors across the split axis.
    let partner_pose = board_pose(&picked, &partner_id);
    assert_eq!(partner_pose.at.x, 2.0 * 40.0 - source_pose.at.x);
    assert_eq!(partner_pose.at.y, source_pose.at.y);
}

#[test]
fn carried_stagger_moves_the_part_by_the_transported_amount() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let before = board_pose(&dropped, "u1");
    let moved = edit(
        &mut engine,
        &dropped,
        EditOperation::SetMatrixPlacement {
            matrix_id: "main".into(),
            change: MatrixPlacementChange::ColumnStagger {
                column: 5,
                value: 3.0,
            },
        },
    )
    .1;
    let after = board_pose(&moved, "u1");
    assert_eq!(after.at.x, before.at.x);
    assert_eq!(after.at.y, before.at.y + 3.0);
    assert_eq!(after.rotation, before.rotation);
}

#[test]
fn detach_keeps_the_pose_from_the_half_origin() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let before = board_pose(&dropped, "u1");
    let detached = edit(
        &mut engine,
        &dropped,
        EditOperation::DetachPartAnchor {
            part_id: "u1".into(),
        },
    )
    .1;
    assert!(part_of(&detached, "u1").anchor.is_none());
    assert_eq!(board_pose(&detached, "u1"), before);
    let undone_doc = undone(&mut engine);
    assert!(part_of(&undone_doc, "u1").anchor.is_some());
    assert_eq!(board_pose(&undone_doc, "u1"), before);
}

#[test]
fn follow_subset_change_keeps_the_pose() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let before = board_pose(&dropped, "u1");
    let changed = edit(
        &mut engine,
        &dropped,
        EditOperation::SetPartAnchorFollow {
            part_id: "u1".into(),
            follow: Follow {
                x: false,
                y: true,
                rotation: true,
            },
        },
    )
    .1;
    assert_eq!(board_pose(&changed, "u1"), before);
    assert!(
        !part_of(&changed, "u1")
            .anchor
            .as_ref()
            .expect("anchor")
            .follow
            .x
    );
}

#[test]
fn dragging_an_anchored_part_edits_spread_and_stagger() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let before = part_of(&dropped, "u1").anchor.expect("anchor");
    let dragged = edit(
        &mut engine,
        &dropped,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "u1".into(),
                at: Vec2 { x: 111.25, y: 50.1 },
            }],
        },
    )
    .1;
    let after = part_of(&dragged, "u1").anchor.expect("anchor");
    assert_eq!(after.spread, before.spread + 5.0);
    assert_eq!(after.stagger, before.stagger + 2.0);
    assert_eq!(after.rotation, before.rotation);
    assert_eq!(board_pose(&dragged, "u1").at, Vec2 { x: 111.25, y: 50.1 });
}

#[test]
fn mirror_partners_share_half_frame_values_while_splay_mirrors() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let records = part_records(&dropped);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].source_id, "u1");
    let partner_id = records[0].mirror_id.clone();
    let source = part_of(&dropped, "u1");
    let partner = part_of(&dropped, &partner_id);
    let source_anchor = source.anchor.expect("source anchor");
    let partner_anchor = partner.anchor.expect("partner anchor");
    // The partner anchors to the counterpart key with mirrored values.
    assert_eq!(partner_anchor.matrix_id, "main-mirror");
    assert_eq!(
        (partner_anchor.row, partner_anchor.column),
        (source_anchor.row, source_anchor.column)
    );
    assert_eq!(partner_anchor.spread, -source_anchor.spread);
    assert_eq!(partner_anchor.stagger, source_anchor.stagger);
    // Board poses mirror across the split axis.
    let source_pose = board_pose(&dropped, "u1");
    let partner_pose = board_pose(&dropped, &partner_id);
    assert_eq!(partner_pose.at.x, 2.0 * 40.0 - source_pose.at.x);
    assert_eq!(partner_pose.at.y, source_pose.at.y);
    // Splay mirrors: +20 on the source reads -20 on the partner.
    let splayed = edit(
        &mut engine,
        &dropped,
        EditOperation::SetPartAnchorValues {
            part_id: "u1".into(),
            change: PartAnchorValueChange::Splay(20.0),
        },
    )
    .1;
    assert_eq!(board_pose(&splayed, "u1").rotation, 20.0);
    assert_eq!(board_pose(&splayed, &partner_id).rotation, -20.0);
    let partner_anchor = part_of(&splayed, &partner_id)
        .anchor
        .expect("partner anchor");
    assert_eq!(partner_anchor.splay, -20.0);
}

/// Ticket 07: a detached Rotate commits the exact board rotation through the
/// public routes, with one Undo/Redo and a reload keeping it. The position
/// stays exactly where the part was.
#[test]
fn detached_rotation_commits_exact_pose_with_undo_redo_and_reload() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let before = board_pose(&dropped, "u1");
    let detached = edit(
        &mut engine,
        &dropped,
        EditOperation::DetachPartAnchor {
            part_id: "u1".into(),
        },
    )
    .1;
    assert!(part_of(&detached, "u1").anchor.is_none());
    let rotated = edit(
        &mut engine,
        &detached,
        EditOperation::SetPartRotation {
            part_id: "u1".into(),
            rotation: 45.0,
        },
    )
    .1;
    let after = board_pose(&rotated, "u1");
    assert_eq!(after.at, before.at);
    assert!((after.rotation - 45.0).abs() < 1e-9);
    assert_eq!(part_of(&rotated, "u1").pose.rotation, 45.0);
    let restored = undone(&mut engine);
    assert_eq!(board_pose(&restored, "u1"), board_pose(&detached, "u1"));
    let again = redone(&mut engine);
    assert_eq!(board_pose(&again, "u1"), after);
    // Reload keeps the exact rotation.
    let saved = serde_json::to_string(&again).expect("serialize");
    let reloaded: ProjectDoc = serde_json::from_str(&saved).expect("deserialize");
    let (_, _, reopened) = open(reloaded);
    assert_eq!(board_pose(&reopened, "u1"), after);
}

/// Ticket 07: detached rotation keeps the locked/member/anchored guards and
/// refuses non-finite and unknown parts. Anchored parts keep using
/// `SetPartAnchorValues::Rotation`.
#[test]
fn detached_rotation_keeps_guards_and_refuses_bad_input() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    // Anchored refuses: use SetPartAnchorValues instead.
    let message = edit_error(
        &mut engine,
        &dropped,
        EditOperation::SetPartRotation {
            part_id: "u1".into(),
            rotation: 10.0,
        },
    );
    assert!(message.contains("anchored"), "anchored refuses: {message}");
    let detached = edit(
        &mut engine,
        &dropped,
        EditOperation::DetachPartAnchor {
            part_id: "u1".into(),
        },
    )
    .1;
    // Non-finite refuses.
    let message = edit_error(
        &mut engine,
        &detached,
        EditOperation::SetPartRotation {
            part_id: "u1".into(),
            rotation: f64::NAN,
        },
    );
    assert!(message.contains("finite"), "non-finite refuses: {message}");
    // Unknown refuses.
    let message = edit_error(
        &mut engine,
        &detached,
        EditOperation::SetPartRotation {
            part_id: "missing".into(),
            rotation: 10.0,
        },
    );
    assert!(message.contains("Unknown"), "unknown refuses: {message}");
    // A matrix member keeps its matrix's tools.
    let message = edit_error(
        &mut engine,
        &detached,
        EditOperation::SetPartRotation {
            part_id: member("main", 0, 0),
            rotation: 10.0,
        },
    );
    assert!(
        message.contains("follows its matrix"),
        "member refuses: {message}"
    );
}

/// Ticket 07: a locked detached part cannot rotate.
#[test]
fn detached_rotation_refuses_locked_parts() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let detached = edit(
        &mut engine,
        &dropped,
        EditOperation::DetachPartAnchor {
            part_id: "u1".into(),
        },
    )
    .1;
    let mut locked_doc = detached.clone();
    if let Some(part) = locked_doc.parts.iter_mut().find(|part| part.id == "u1") {
        part.locked = Some(true);
    }
    // Lock through MoveParts would refuse; rotate through the new op too.
    // Apply the lock first via a fresh open so the revision matches.
    let (mut locked_engine, _, locked_accepted) = open(locked_doc);
    let message = edit_error(
        &mut locked_engine,
        &locked_accepted,
        EditOperation::SetPartRotation {
            part_id: "u1".into(),
            rotation: 10.0,
        },
    );
    assert!(message.contains("locked"), "locked refuses: {message}");
}

/// Ticket 07: a detached rotation on one half mirrors the partner heading in
/// the same Undo entry (controller keeps the world rotation).
#[test]
fn detached_rotation_mirrors_the_partner_in_one_entry() {
    let (mut engine, _, doc) = open(pair_doc());
    let dropped = drop_u1(&mut engine, &doc);
    let partner_id = part_records(&dropped)[0].mirror_id.clone();
    let detached = edit(
        &mut engine,
        &dropped,
        EditOperation::DetachPartAnchor {
            part_id: "u1".into(),
        },
    )
    .1;
    let both = edit(
        &mut engine,
        &detached,
        EditOperation::DetachPartAnchor {
            part_id: partner_id.clone(),
        },
    )
    .1;
    let rotated = edit(
        &mut engine,
        &both,
        EditOperation::SetPartRotation {
            part_id: "u1".into(),
            rotation: 30.0,
        },
    )
    .1;
    assert!((board_pose(&rotated, "u1").rotation - 30.0).abs() < 1e-9);
    // The MCU is a controller: the mirror takes the plain reflected heading
    // (-30, normalized to 330), no extra half turn.
    let partner_rotation = board_pose(&rotated, &partner_id).rotation;
    assert!(
        (partner_rotation - 330.0).abs() < 1e-9 || (partner_rotation + 30.0).abs() < 1e-9,
        "partner mirrors: {partner_rotation}"
    );
    let undone_doc = undone(&mut engine);
    assert_eq!(board_pose(&undone_doc, "u1"), board_pose(&both, "u1"));
    assert_eq!(
        board_pose(&undone_doc, &partner_id),
        board_pose(&both, &partner_id)
    );
}
