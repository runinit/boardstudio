use super::*;

#[wasm_bindgen_test]
fn board_part_choices_mark_only_live_matrix_keys_as_matrix_keys() {
    let (mut model, selected) = fixture();
    let key = "matrix/component-matrix/r0c0";
    let companion = "matrix/component-matrix/r0c0/led";
    let disabled = "matrix/component-matrix/r0c1";
    let deleted = "matrix/component-matrix/r0c2";
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    document.boards[0]
        .part_ids
        .extend([key, companion, disabled, deleted].map(str::to_owned));
    for (id, reference) in [
        (key, "SW1"),
        (companion, "D1"),
        (disabled, "SW2"),
        (deleted, "SW3"),
    ] {
        let mut part = document.parts[0].clone();
        part.id = id.to_owned();
        part.reference = reference.to_owned();
        document.parts.push(part);
    }
    document.matrices.push(Matrix {
        id: "component-matrix".into(),
        name: None,
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "component-definition".into(),
        part_ids: [key, companion, disabled, deleted]
            .map(str::to_owned)
            .to_vec(),
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![
            MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            },
            MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 1,
                enabled: false,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            },
            MatrixCell {
                shift: None,
                bend: None,
                deleted: true,
                row: 0,
                column: 2,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            },
        ],
    });
    Arc::make_mut(&mut snapshot.scene)
        .matrix_scenes
        .push(MatrixScene {
            matrix_id: "component-matrix".into(),
            columns: vec![],
            companions: vec![],
            cells: vec![
                MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2::default(),
                    key_shift: Vec2::default(),
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2::default(),
                    cumulative_bend: 0.0,
                    row: 0,
                    column: 0,
                    enabled: true,
                    member_id: Some(key.into()),
                    pose: Pose2 {
                        at: Vec2::default(),
                        rotation: 0.0,
                    },
                },
                MatrixSceneCell {
                    shift_frame: Default::default(),
                    offset_frame: Default::default(),
                    row_shift: Vec2::default(),
                    key_shift: Vec2::default(),
                    row_bend: 0.0,
                    key_bend: 0.0,
                    cumulative_shift: Vec2::default(),
                    cumulative_bend: 0.0,
                    row: 0,
                    column: 1,
                    enabled: false,
                    member_id: None,
                    pose: Pose2 {
                        at: Vec2::default(),
                        rotation: 0.0,
                    },
                },
            ],
        });

    let projection =
        super::super::layout_component_inspector_projection(&model, Some(&selected), 1, 1)
            .expect("selected component projects");
    let choices: Vec<_> = projection
        .board_parts
        .iter()
        .filter(|part| [key, companion, disabled, deleted].contains(&part.id.as_str()))
        .map(|part| (part.id.as_str(), part.matrix_key))
        .collect();
    assert_eq!(
        choices,
        [
            (key, true),
            (companion, false),
            (disabled, false),
            (deleted, false)
        ]
    );
}

#[wasm_bindgen_test]
fn companion_placement_projection_normalizes_layout_selections() {
    use serde_json::json;

    let (mut model, selected) = fixture();
    let document = Arc::make_mut(&mut model.accepted.as_mut().expect("fixture snapshot").document);
    let source_key_id = "matrix/component-matrix/r0c0";
    let source_companion_id = format!("{source_key_id}/led");
    let mirror_companion_id = "matrix/paired-matrix/r0c0/led";
    document.matrices.push(Matrix {
        id: "component-matrix".into(),
        name: Some("Keys".into()),
        rows: 1,
        columns: 1,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "component-definition".into(),
        part_ids: vec![],
        board_id: Some("board".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row: 0,
            column: 0,
            enabled: true,
            definition_id: Some("component-definition".into()),
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![],
            assemblies_local: None,
        }],
    });
    document.boards[0].part_ids.push(source_key_id.into());
    document.boards[0]
        .part_ids
        .push(source_companion_id.clone());
    document.matrices[0].part_ids = vec![source_key_id.into(), source_companion_id.clone()];
    document.matrices[0].cells[0].assemblies.push(
        serde_json::from_value(json!({
            "id": "led", "definitionId": "component-definition",
            "offset": {"x": 1.0, "y": 2.0}
        }))
        .expect("fixture companion assembly"),
    );
    let mut source_key = document.parts[0].clone();
    source_key.id = source_key_id.into();
    document.parts.push(source_key.clone());
    let mut source_companion = source_key;
    source_companion.id = source_companion_id.clone();
    document.parts.push(source_companion);
    document.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                {"id":"source-instance","name":"Source","boardId":"board","half":"left","role":"primary","flipped":false,"controllerPartId":null,"mechanical":null},
                {"id":"mirror-instance","name":"Mirror","boardId":"mirror-board","half":"right","role":"secondary","flipped":true,"controllerPartId":null,"mechanical":null}
            ],
            "pairs": [{
                "id":"pair/A","members":["source-instance","mirror-instance"],
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "construction":{"method":"printed","mount":"tray","integratedPlateFrame":false,"plateThickness":1.5,"plateFoamThickness":0.0,"bottomFoamThickness":0.0,"bottomThickness":1.5,"plateToPcb":5.0,"wallThickness":2.0,"clearance":0.2},
                "constructionSource":"source-instance",
                "halves":{"sourceBoardId":"board","mirrorBoardId":"mirror-board","correspondences":[
                    {"kind":"matrix","sourceId":"component-matrix","mirrorId":"paired-matrix"},
                    {"kind":"part","sourceId":source_companion_id,"mirrorId":mirror_companion_id}
                ]}
            }]
        }))
        .expect("fixture linked pair"),
    );
    let second_source_id = format!("{source_key_id}/led2");
    document.matrices[0].part_ids.push(second_source_id.clone());
    document.matrices[0].cells[0].assemblies.push(
        serde_json::from_value(json!({
            "id": "led2", "definitionId": "component-definition",
            "offset": {"x": -1.0, "y": 2.0}
        }))
        .expect("second fixture companion assembly"),
    );
    document.boards[0].part_ids.push(second_source_id.clone());
    let mut second_source = document
        .parts
        .iter()
        .find(|part| part.id == source_companion_id)
        .expect("first source companion")
        .clone();
    second_source.id = second_source_id.clone();
    document.parts.push(second_source);
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences
        .push(boardstudio_core::model::MirroredCorrespondence {
            companion_mode: Some(boardstudio_core::model::MirroredCompanionMode::DuplicatePosition),
            companion_tweak: None,
            kind: boardstudio_core::model::MirroredItemKind::Part,
            source_id: second_source_id.clone(),
            mirror_id: "matrix/paired-matrix/r0c0/led2".into(),
        });
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let scene = Arc::make_mut(&mut snapshot.scene);
    scene.matrix_scenes.push(MatrixScene {
        matrix_id: "component-matrix".into(),
        cells: vec![MatrixSceneCell {
            shift_frame: Default::default(),
            offset_frame: Default::default(),
            row_shift: Vec2::default(),
            key_shift: Vec2::default(),
            row_bend: 0.0,
            key_bend: 0.0,
            cumulative_shift: Vec2::default(),
            cumulative_bend: 0.0,
            row: 0,
            column: 0,
            enabled: true,
            member_id: Some(source_key_id.into()),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
        }],
        columns: vec![],
        companions: vec![],
    });
    scene.matrix_scenes[0].companions.push(
        serde_json::from_value(json!({
            "assemblyId":"led", "partId":source_companion_id,
            "keyId":source_key_id, "mode":"follows",
            "pose":{"at":{"x":1.0,"y":2.0},"rotation":0.0}
        }))
        .expect("fixture companion scene"),
    );

    let cases = [
        (
            objects::TreeContext::Matrix {
                matrix_id: "component-matrix".into(),
            },
            vec![boardstudio_core::model::CompanionPlacementTarget::Matrix {
                matrix_id: "component-matrix".into(),
            }],
        ),
        (
            objects::TreeContext::Row {
                matrix_id: "component-matrix".into(),
                row: 0,
            },
            vec![boardstudio_core::model::CompanionPlacementTarget::Row {
                matrix_id: "component-matrix".into(),
                row: 0,
            }],
        ),
        (
            objects::TreeContext::Column {
                matrix_id: "component-matrix".into(),
                column: 0,
            },
            vec![boardstudio_core::model::CompanionPlacementTarget::Column {
                matrix_id: "component-matrix".into(),
                column: 0,
            }],
        ),
        (
            objects::TreeContext::Key {
                matrix_id: "component-matrix".into(),
                row: 0,
                column: 0,
            },
            vec![
                boardstudio_core::model::CompanionPlacementTarget::Item {
                    item_kind: boardstudio_core::model::MirroredItemKind::Part,
                    mirror_id: mirror_companion_id.into(),
                },
                boardstudio_core::model::CompanionPlacementTarget::Item {
                    item_kind: boardstudio_core::model::MirroredItemKind::Part,
                    mirror_id: "matrix/paired-matrix/r0c0/led2".into(),
                },
            ],
        ),
        (
            objects::TreeContext::Component {
                part_id: Some(source_companion_id.clone()),
                matrix_id: Some("component-matrix".into()),
                row: Some(0),
                column: Some(0),
                assembly_id: Some("led".into()),
            },
            vec![
                boardstudio_core::model::CompanionPlacementTarget::Item {
                    item_kind: boardstudio_core::model::MirroredItemKind::Part,
                    mirror_id: mirror_companion_id.into(),
                },
                boardstudio_core::model::CompanionPlacementTarget::Item {
                    item_kind: boardstudio_core::model::MirroredItemKind::Part,
                    mirror_id: "matrix/paired-matrix/r0c0/led2".into(),
                },
            ],
        ),
    ];
    for (context, expected) in cases {
        let scoped = objects::ScopedTreeContext {
            scope: selected.scope.clone(),
            context,
        };
        let projection = companion_placement_projection(&model, Some(&scoped), 7, 3)
            .expect("eligible Layout selection projects companion placement");
        assert_eq!(projection.pair_id, "pair/A");
        assert_eq!(projection.targets, expected);
        assert!(!projection.targets.iter().any(|target| matches!(
            target,
            boardstudio_core::model::CompanionPlacementTarget::Item { mirror_id, .. }
                if mirror_id.contains("controller")
        )));
    }

    model.selected_part_ids = vec![
        source_companion_id.clone(),
        second_source_id,
        "selected-part".into(),
    ];
    let scoped = objects::ScopedTreeContext {
        scope: selected.scope,
        context: objects::TreeContext::Component {
            part_id: Some("matrix/component-matrix/r0c0/led".into()),
            matrix_id: Some("component-matrix".into()),
            row: Some(0),
            column: Some(0),
            assembly_id: Some("led".into()),
        },
    };
    let projection = companion_placement_projection(&model, Some(&scoped), 7, 3)
        .expect("selected companion component projects");
    assert_eq!(
        projection.targets.len(),
        2,
        "only selected linked companions project"
    );
    assert!(projection.mixed_mode);
    assert!(projection.mode.is_none());
    assert!(projection.targets.iter().any(|target| matches!(
        target,
        boardstudio_core::model::CompanionPlacementTarget::Item { mirror_id, .. }
            if mirror_id == mirror_companion_id
    )));
    assert!(projection.targets.iter().any(|target| matches!(
        target,
        boardstudio_core::model::CompanionPlacementTarget::Item { mirror_id, .. }
            if mirror_id == "matrix/paired-matrix/r0c0/led2"
    )));
}

#[wasm_bindgen_test]
fn standalone_component_projection_preserves_accepted_owner_and_metadata() {
    let (model, selected) = fixture();
    let projection = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("selected standalone component should have an Inspector");
    assert_eq!(projection.reference, "U1");
    assert_eq!(projection.definition_name, "Fixture controller");
    assert_eq!(projection.definition_kind, "controller");
    assert_eq!(
        projection.envelope_notice.as_deref(),
        Some("Imported courtyard is approximate.")
    );
    assert!(projection.locked);
    assert_eq!(projection.layout_id.as_deref(), Some("source-layout"));
    assert_eq!(projection.owner.scope, selected.scope);
    assert_eq!(projection.owner.snapshot_token, SnapshotToken(13));
    assert_eq!(projection.owner.revision, 9);
    assert_eq!(projection.owner.context_generation, 7);
    assert_eq!(projection.owner.scope_generation, 3);
    assert_eq!(projection.outline.margin, Some(2.5));
    assert!(projection.outline.allow_body_overhang);
    assert!(projection.relationship_summary.contains("Right half"));
}

#[wasm_bindgen_test]
fn component_projection_supports_group_position_selection_and_rejects_invalid_context() {
    let (mut model, selected) = fixture();
    model.selected_part_ids.push("source-part".into());
    let group = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("a live mixed component selection has a group-position projection");
    assert_eq!(group.selection_count, 2);
    assert_eq!(group.owner.selected_part_ids, model.selected_part_ids);
    assert_eq!(
        group.position.x, 4.0,
        "the first selected part anchors the group"
    );

    let mut reordered = model.clone();
    reordered.selected_part_ids.reverse();
    assert!(
        layout_component_inspector_projection(&reordered, Some(&selected), 7, 3).is_none(),
        "a component context cannot act for a group whose first selected part changed"
    );

    let (model, mut selected) = fixture();
    selected.context = objects::TreeContext::Component {
        part_id: Some("selected-part".into()),
        matrix_id: Some("matrix".into()),
        row: None,
        column: None,
        assembly_id: None,
    };
    assert!(layout_component_inspector_projection(&model, Some(&selected), 7, 3).is_none());
}

#[wasm_bindgen_test]
fn relations_summary_uses_layout_membership_before_constraint_source() {
    let (mut model, selected) = fixture();
    let framed = Constraint::Framed {
        id: "constraint".into(),
        source: ConstraintEndpoint::Part {
            part_id: "source-part".into(),
        },
        target: ConstraintEndpoint::Part {
            part_id: "selected-part".into(),
        },
        relation: ConstraintRelation::PartOffset {
            offset: Vec2 { x: 2.0, y: -3.0 },
            rotation: 17.0,
        },
    };
    {
        let snapshot = model.accepted.as_mut().expect("fixture snapshot");
        Arc::make_mut(&mut snapshot.document)
            .constraints
            .push(framed.clone());
    }
    let assigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("assigned component should project");
    assert_eq!(assigned.active_constraint, Some(framed.clone()));
    assert!(assigned.relationship_summary.contains("Right half"));

    Arc::make_mut(&mut model.accepted.as_mut().expect("fixture snapshot").document).layouts[0]
        .part_ids
        .clear();
    let unassigned = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("unassigned component should project");
    assert!(unassigned.relationship_summary.contains("U2 drives U1"));
    assert_eq!(unassigned.active_constraint, Some(framed));
}

#[wasm_bindgen_test]
fn driving_constraint_lookup_finds_the_same_record_from_both_halves_and_rejects_stale_binding() {
    use serde_json::json;

    let mut document = ProjectDoc::empty("framed-query", "Framed query");
    document.parts = serde_json::from_value(json!([
        { "id": "alpha/source", "definitionId": "part", "reference": "U1",
          "side": "front", "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "alpha/target", "definitionId": "part", "reference": "U2",
          "side": "front", "pose": { "at": { "x": 1.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "beta/source", "definitionId": "part", "reference": "U3",
          "side": "front", "pose": { "at": { "x": 2.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "beta/target", "definitionId": "part", "reference": "U4",
          "side": "front", "pose": { "at": { "x": 3.0, "y": 0.0 }, "rotation": 0.0 } }
    ]))
    .expect("the query fixture parts deserialize");
    document.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                { "id": "alpha-instance", "name": "A", "boardId": "alpha", "half": "left",
                  "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null },
                { "id": "beta-instance", "name": "B", "boardId": "beta", "half": "right",
                  "role": "secondary", "flipped": true, "controllerPartId": null, "mechanical": null }
            ],
            "pairs": [{
                "id": "pair/A", "members": ["alpha-instance", "beta-instance"],
                "mapping": { "at": { "x": 20.0, "y": -7.0 }, "rotation": 36.86989764584402 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 1.5, "plateToPcb": 5.0, "wallThickness": 2.0, "clearance": 0.2
                },
                "constructionSource": "alpha-instance",
                "halves": {
                    "sourceBoardId": "alpha", "mirrorBoardId": "beta",
                    "correspondences": [
                        { "kind": "part", "sourceId": "alpha/source", "mirrorId": "beta/source" },
                        { "kind": "part", "sourceId": "alpha/target", "mirrorId": "beta/target" }
                    ]
                }
            }]
        }))
        .expect("the query fixture pair deserializes"),
    );
    let endpoint = |mirror_part_id: &str, source_part_id: &str| ConstraintEndpoint::LinkedMirror {
        pair_id: "pair/A".into(),
        source_part_id: source_part_id.into(),
        mirror_part_id: mirror_part_id.into(),
    };
    let constraint = Constraint::Framed {
        id: "durable/constraint-7".into(),
        source: ConstraintEndpoint::Part {
            part_id: "alpha/source".into(),
        },
        target: endpoint("beta/target", "alpha/target"),
        relation: ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    };
    document.constraints.push(constraint.clone());

    for part_id in ["alpha/target", "beta/target"] {
        assert_eq!(
            document
                .constraint_driving_part(part_id)
                .expect("both endpoint views resolve"),
            Some(&constraint),
            "selecting {part_id} keeps the one stable record"
        );
    }

    document.constraints[0] = Constraint::Framed {
        id: "durable/constraint-7".into(),
        source: ConstraintEndpoint::Part {
            part_id: "alpha/source".into(),
        },
        target: endpoint("beta/target", "alpha/not-target"),
        relation: ConstraintRelation::Offset {
            offset: Vec2 { x: 5.0, y: -2.0 },
            rotation: 15.0,
        },
    };
    assert!(
        document.constraint_driving_part("beta/target").is_err(),
        "stale explicit endpoint bindings must refuse the lookup"
    );
}

#[wasm_bindgen_test]
fn inspector_projection_finds_a_driver_authored_against_the_selected_parts_linked_mirror() {
    use serde_json::json;

    let (mut model, selected) = fixture();
    let snapshot = model.accepted.as_mut().expect("fixture snapshot");
    let document = Arc::make_mut(&mut snapshot.document);
    document.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    document.boards[0].part_ids.push("driver-part".into());
    document.boards.push(Board {
        id: "mirror-board".into(),
        name: "Mirror board".into(),
        outline_ids: vec![],
        part_ids: vec!["source-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut driver = document.parts[0].clone();
    driver.id = "driver-part".into();
    driver.reference = "U3".into();
    document.parts.push(driver);
    document.hardware = Some(
        serde_json::from_value(json!({
            "instances": [
                { "id": "alpha-instance", "name": "A", "boardId": "board", "half": "left",
                  "role": "primary", "flipped": false, "controllerPartId": null, "mechanical": null },
                { "id": "beta-instance", "name": "B", "boardId": "mirror-board", "half": "right",
                  "role": "secondary", "flipped": true, "controllerPartId": null, "mechanical": null }
            ],
            "pairs": [{
                "id": "pair/A", "members": ["alpha-instance", "beta-instance"],
                "mapping": { "at": { "x": 20.0, "y": -7.0 }, "rotation": 36.86989764584402 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                    "bottomThickness": 1.5, "plateToPcb": 5.0, "wallThickness": 2.0, "clearance": 0.2
                },
                "constructionSource": "alpha-instance",
                "halves": {
                    "sourceBoardId": "board", "mirrorBoardId": "mirror-board",
                    "correspondences": [
                        { "kind": "part", "sourceId": "selected-part", "mirrorId": "source-part" }
                    ]
                }
            }]
        }))
        .expect("the linked pair fixture deserializes"),
    );
    document.constraints.push(Constraint::Framed {
        id: "stable-driver-id".into(),
        source: ConstraintEndpoint::Part {
            part_id: "driver-part".into(),
        },
        target: ConstraintEndpoint::LinkedMirror {
            pair_id: "pair/A".into(),
            source_part_id: "selected-part".into(),
            mirror_part_id: "source-part".into(),
        },
        relation: ConstraintRelation::MirrorLine {
            at: Vec2 { x: 12.0, y: -9.0 },
            angle: 31.0,
        },
    });
    let expected_constraint = document.constraints[0].clone();

    let projection = layout_component_inspector_projection(&model, Some(&selected), 7, 3)
        .expect("the selected authoritative part projects");
    let half = projection
        .mirrored_half
        .as_ref()
        .expect("linked half projects");
    assert_eq!(half.pair_id, "pair/A");
    assert_eq!(half.mirror_id, "source-part");
    assert_eq!(
        half.state,
        boardstudio_core::model::HalfExceptionState::Linked,
        "the source-side Inspector projects the linked mirror state"
    );
    assert_eq!(
        projection.active_constraint,
        Some(expected_constraint.clone()),
        "inspector projection follows authority through the linked-mirror target"
    );
    let local_edit = layout_component_edits::mirrored_half_state_resolver(
        "pair/A".into(),
        "source-part".into(),
        boardstudio_core::model::HalfExceptionState::Local,
    )
    .resolve(model.accepted.as_ref().unwrap());
    let boardstudio_application::Resolution::Submit(local_command) = local_edit else {
        panic!("a linked item resolves to one typed Local edit");
    };
    assert_eq!(local_command.target_ids, ["pair/A", "source-part"]);
    assert!(matches!(
        local_command.operation,
        boardstudio_core::model::EditOperation::SetHalfException {
            pair_id,
            kind: boardstudio_core::model::MirroredItemKind::Part,
            mirror_id,
            state: boardstudio_core::model::HalfExceptionState::Local,
        } if pair_id == "pair/A" && mirror_id == "source-part"
    ));

    let mut mirror_model = model.clone();
    mirror_model.active_board_id = "mirror-board".into();
    mirror_model.selected_part_ids = vec!["source-part".into()];
    mirror_model.selection_anchor_id = Some("source-part".into());
    let mut mirror_selected = selected.clone();
    mirror_selected.scope.board_id = "mirror-board".into();
    mirror_selected.context = objects::context_for_part(&mirror_model, "source-part")
        .expect("the linked counterpart has a current mirror-board context");
    let mirror_projection =
        layout_component_inspector_projection(&mirror_model, Some(&mirror_selected), 8, 3)
            .expect("the linked counterpart projects");
    let mirror_half = mirror_projection
        .mirrored_half
        .as_ref()
        .expect("the linked counterpart projects its half state");
    assert_eq!(mirror_half.pair_id, "pair/A");
    assert_eq!(mirror_half.mirror_id, "source-part");
    assert_eq!(
        mirror_half.state,
        boardstudio_core::model::HalfExceptionState::Linked,
        "selecting either half presents the same normalized mirror-item state"
    );
    assert_eq!(
        mirror_projection.active_constraint,
        Some(expected_constraint.clone()),
        "selecting the mirror half resolves the same durable relation"
    );

    let mut local_model = model.clone();
    Arc::make_mut(
        &mut local_model
            .accepted
            .as_mut()
            .expect("fixture snapshot")
            .document,
    )
    .hardware
    .as_mut()
    .unwrap()
    .pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .push(boardstudio_core::model::MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: boardstudio_core::model::MirroredItemKind::Part,
            mirror_id: "source-part".into(),
            state: boardstudio_core::model::MirroredExceptionState::Local,
            removed_copper: None,
        });
    assert_eq!(
        mirrored_half_projection(
            &local_model.accepted.as_ref().unwrap().document,
            "selected-part"
        )
        .unwrap()
        .state,
        boardstudio_core::model::HalfExceptionState::Local,
        "local state projects independently of linked constraint resolution"
    );

    let mut stale_model = model.clone();
    let stale_document = Arc::make_mut(
        &mut stale_model
            .accepted
            .as_mut()
            .expect("fixture snapshot")
            .document,
    );
    stale_document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .correspondences[0]
        .source_id = "removed-authority".into();
    assert!(
        layout_component_inspector_projection(&stale_model, Some(&selected), 9, 3).is_none(),
        "an invalid stored binding fails closed at the inspector projection"
    );
}
