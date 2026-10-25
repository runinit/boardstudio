//! Ticket 31: aggregate anchors through linked reflection. Aggregate part
//! targets map through the pair correspondence by identity; the
//! relative-negate rules apply to the resolved aggregate pose (mirror of the
//! whole candidate, ticket-20 transported-filter precedent), never per part.
//! All behavior runs through public Core routes (`Open` / `Edit`).
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

fn definitions() -> Vec<PartDefinition> {
    serde_json::from_value(serde_json::json!([
        {"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}
    ]))
    .unwrap()
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
    }
}

fn member(matrix: &str, row: u32, column: u32) -> String {
    format!("matrix/{matrix}/r{row}c{column}")
}

fn pose_of(doc: &ProjectDoc, id: &str) -> Pose2 {
    doc.parts
        .iter()
        .find(|part| part.id == id)
        .unwrap_or_else(|| panic!("missing part {id}"))
        .pose
}

fn anchor_of(doc: &ProjectDoc, id: &str) -> Option<MatrixAnchor> {
    doc.matrices
        .iter()
        .find(|matrix| matrix.id == id)
        .and_then(|matrix| matrix.anchor.clone())
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

fn all_follow() -> Follow {
    Follow {
        x: true,
        y: true,
        rotation: true,
    }
}

fn average_anchor(parts: Vec<(&str, u32, u32)>, follow: Follow) -> MatrixAnchor {
    MatrixAnchor {
        matrix_id: String::new(),
        row: 0,
        column: 0,
        follow,
        aggregate: Some(AggregateAnchor {
            method: AggregateMethod::Average,
            parts: parts
                .into_iter()
                .map(|(matrix_id, row, column)| AnchorPart {
                    matrix_id: matrix_id.into(),
                    row,
                    column,
                })
                .collect(),
        }),
    }
}

/// Linked pair without dependants: `main` on the source layout mirrored by
/// `main2` about x=10, with matrix correspondences only (part pairs are
/// maintained by the production sync, as in ticket 20).
fn linked_pair_fixture() -> (CoreEngine, ProjectDoc) {
    let mut doc = ProjectDoc::empty("aggregate-linked", "Aggregate linked");
    doc.definitions = definitions();
    let mut main = Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![
            member("main", 0, 0),
            member("main", 0, 1),
            member("main", 0, 2),
        ],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    };
    let mut main2 = main.clone();
    main2.id = "main2".into();
    main2.origin = Vec2 { x: 20.0, y: 0.0 };
    main2.mirror = Some(Mirror::X);
    main2.part_ids = vec![
        member("main2", 0, 0),
        member("main2", 0, 1),
        member("main2", 0, 2),
    ];
    main.part_ids = vec![
        member("main", 0, 0),
        member("main", 0, 1),
        member("main", 0, 2),
    ];
    doc.parts = vec![
        key_part(&member("main", 0, 0), 0.0, 0.0),
        key_part(&member("main", 0, 1), 19.0, 0.0),
        key_part(&member("main", 0, 2), 38.0, 0.0),
        key_part(&member("main2", 0, 0), 20.0, 0.0),
        key_part(&member("main2", 0, 1), 1.0, 0.0),
        key_part(&member("main2", 0, 2), -18.0, 0.0),
    ];
    doc.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: doc.parts.iter().map(|part| part.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(main);
    doc.matrices.push(main2);
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Layout".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.layouts.push(Layout {
        id: "mirror-layout".into(),
        name: "Mirror".into(),
        board_id: "board".into(),
        matrix_ids: vec!["main2".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            source_id: "layout".into(),
            axis_x: 10.0,
            split_gap: None,
            correspondences: vec![MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "main".into(),
                mirror_id: "main2".into(),
            }],
            exceptions: vec![],
        }),
    });
    let (engine, _, doc) = open(doc);
    (engine, doc)
}

#[test]
fn linked_add_with_average_aggregate_mirrors_the_whole_candidate() {
    // Adding a matrix with an average aggregate anchor to a linked layout
    // creates the mirrored counterpart with every part mapped by identity;
    // the counterpart resolves the mirrored whole-candidate pose, not a
    // per-part reflection.
    let (mut engine, doc) = linked_pair_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "claw".into(),
            anchor: average_anchor(vec![("main", 0, 0), ("main", 0, 2)], all_follow()),
        },
    );
    let claw = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw")
        .expect("claw matrix");
    let mirror = doc
        .matrices
        .iter()
        .find(|matrix| matrix.id == "claw-mirror")
        .expect("claw mirror");
    let claw_anchor = claw.anchor.clone().expect("claw anchor");
    assert!(claw_anchor.is_aggregate());
    assert_eq!(
        claw_anchor.targets(),
        vec![
            AnchorPart {
                matrix_id: "main".into(),
                row: 0,
                column: 0,
            },
            AnchorPart {
                matrix_id: "main".into(),
                row: 0,
                column: 2,
            },
        ]
    );
    // Every part maps through the pair correspondence by identity; the
    // physical key (row/column), method and Follow subset stay unchanged.
    let mirror_anchor = mirror.anchor.clone().expect("mirror anchor");
    assert!(mirror_anchor.is_aggregate());
    assert_eq!(
        mirror_anchor.aggregate.as_ref().expect("method").method,
        AggregateMethod::Average
    );
    assert_eq!(
        mirror_anchor.targets(),
        vec![
            AnchorPart {
                matrix_id: "main2".into(),
                row: 0,
                column: 0,
            },
            AnchorPart {
                matrix_id: "main2".into(),
                row: 0,
                column: 2,
            },
        ]
    );
    assert_eq!(mirror_anchor.follow, all_follow());
    // The aggregate mean of (0,0) and (38,0) is (19,0); the (0,-19) relative
    // origin completes the (19,-19) candidate, mirrored whole to (1,-19).
    assert_eq!(
        pose_of(&doc, &member("claw", 0, 0)).at,
        Vec2 { x: 19.0, y: -19.0 }
    );
    assert_eq!(
        pose_of(&doc, &member("claw-mirror", 0, 0)).at,
        Vec2 { x: 1.0, y: -19.0 }
    );
    // One Undo removes both, Redo restores both.
    let undone_doc = undone(&mut engine);
    assert!(undone_doc.matrices.iter().all(|matrix| matrix.id != "claw"));
    assert!(
        undone_doc
            .matrices
            .iter()
            .all(|matrix| matrix.id != "claw-mirror")
    );
    let redone_doc = redone(&mut engine);
    assert!(redone_doc.matrices.iter().any(|matrix| matrix.id == "claw"));
    assert!(
        redone_doc
            .matrices
            .iter()
            .any(|matrix| matrix.id == "claw-mirror")
    );
}

#[test]
fn linked_aggregate_follow_transports_source_basis_with_non_origin_axis() {
    // Axis x=10 is not the origin: with Follow X off, the source keeps
    // world-zero X while the partner keeps the transported X=20, exactly as
    // ticket 20 proves for single-key anchors.
    let (mut engine, doc) = linked_pair_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(
                vec![("main", 0, 0), ("main", 0, 2)],
                Follow {
                    x: false,
                    y: true,
                    rotation: true,
                },
            ),
        },
    );
    assert!(
        !anchor_of(&doc, "thumb-mirror")
            .expect("thumb-mirror anchor")
            .follow
            .x
    );
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main", 0, 2),
                at: Vec2 { x: 43.0, y: 7.0 },
            }],
        },
    );
    // The moved key shifts the mean to (21.5,3.5); the (0,-19) origin
    // completes (21.5,-15.5), and Follow X off keeps world-zero X on the
    // source while the partner keeps the transported axis image.
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 0.0, y: -15.5 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb-mirror", 0, 0)).at,
        Vec2 { x: 20.0, y: -15.5 }
    );
}

#[test]
fn linked_aggregate_dependent_follows_actual_local_part() {
    // The mirror dependent follows the actual Local part pose through the
    // aggregate mean, not a substitute invisible pose; its own layout values
    // stay linked.
    let (mut engine, doc) = linked_pair_fixture();
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::AddAnchoredMatrix {
            id: "thumb".into(),
            anchor: average_anchor(vec![("main", 0, 0), ("main", 0, 2)], all_follow()),
        },
    );
    // Make the mirror part key Local via its Layout link.
    let mut mirror_layout = doc
        .layouts
        .iter()
        .find(|layout| layout.id == "mirror-layout")
        .expect("mirror layout")
        .clone();
    let link = mirror_layout.mirror_link.as_mut().expect("mirror link");
    link.exceptions.push(MirroredException {
        kind: MirroredItemKind::Part,
        mirror_id: member("main2", 0, 2),
        state: MirroredExceptionState::Local,
        companion_mode: None,
        companion_tweak: None,
        removed_copper: None,
    });
    let (_, doc) = edit(
        &mut engine,
        &doc,
        EditOperation::SetLayout {
            layout: mirror_layout,
        },
    );
    // Move the actual Local part key independently from the mirror side.
    let (_, moved) = edit(
        &mut engine,
        &doc,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: member("main2", 0, 2),
                at: Vec2 { x: -10.0, y: 5.0 },
            }],
        },
    );
    // Source stays where it was; the mirror part moved locally.
    assert_eq!(
        pose_of(&moved, &member("main", 0, 2)).at,
        Vec2 { x: 38.0, y: 0.0 }
    );
    assert_eq!(
        pose_of(&moved, &member("main2", 0, 2)).at,
        Vec2 { x: -10.0, y: 5.0 }
    );
    // The mirror dependent follows the mean of the actual poses
    // (((20,0) + (-10,5)) / 2 = (5,2.5)) completed with the (0,-19) origin.
    assert_eq!(
        pose_of(&moved, &member("thumb-mirror", 0, 0)).at,
        Vec2 { x: 5.0, y: -16.5 }
    );
    assert_eq!(
        pose_of(&moved, &member("thumb", 0, 0)).at,
        Vec2 { x: 19.0, y: -19.0 }
    );
    let thumb = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb")
        .unwrap();
    let thumb_mirror = moved
        .matrices
        .iter()
        .find(|matrix| matrix.id == "thumb-mirror")
        .unwrap();
    assert_eq!(thumb.row_layout, thumb_mirror.row_layout);
    assert_eq!(thumb.column_layout, thumb_mirror.column_layout);
}
