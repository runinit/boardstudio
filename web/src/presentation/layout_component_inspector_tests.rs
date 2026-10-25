use super::inspector::{
    ComponentPositionAxis, LayoutComponentInspector, LayoutComponentInspectorAction,
    LayoutComponentInspectorLifetime, LayoutComponentInspectorProjection, MirroredHalfProjection,
};
use super::*;
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SessionEpoch};
use boardstudio_core::model::{
    Board, Constraint, ConstraintEndpoint, ConstraintRelation, Layout, LayoutMirrorLink, Matrix,
    MatrixCell, MatrixScene, MatrixSceneCell, MirroredCorrespondence, MirroredItemKind,
    PartDefinition, PartKind, PartOutline, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
};
use boardstudio_web_ui_shared::test_wait::wait_until;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// The document the fixture and the mounted runtime share: a board, two positioned
/// parts and the two mirrored layouts the projections read.
fn fixture_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
    let mut document = ProjectDoc::empty("inspector-doc", "Inspector fixture");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["selected-part".into(), "source-part".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: "component-definition".into(),
        name: "Fixture controller".into(),
        kind: PartKind::Controller,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: Some("Imported courtyard is approximate.".into()),
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    });
    for (id, reference, x) in [("selected-part", "U1", 4.0), ("source-part", "U2", 10.0)] {
        document.parts.push(Part {
            keycap: None,
            outline: (id == "selected-part").then_some(PartOutline {
                excluded: false,
                margin: Some(2.5),
                allow_body_overhang: true,
            }),
            id: id.into(),
            definition_id: "component-definition".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x, y: 3.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
    }
    document.layouts.push(Layout {
        id: "source-layout".into(),
        name: "Left half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["matrix".into()],
        part_ids: vec!["selected-part".into()],
        mirror_link: None,
    });
    document.layouts.push(Layout {
        id: "paired-layout".into(),
        name: "Right half".into(),
        board_id: "board".into(),
        matrix_ids: vec!["paired-matrix".into()],
        part_ids: vec![],
        mirror_link: Some(LayoutMirrorLink {
            correspondences: vec![MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "matrix".into(),
                mirror_id: "paired-matrix".into(),
            }],
            exceptions: vec![],
            source_id: "source-layout".into(),
            axis_x: 0.0,
            split_gap: None,
            spacing: None,
        }),
    });
    configure(&mut document);
    document
}

fn fixture() -> (ReadModel, objects::ScopedTreeContext) {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "inspector-doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    let document = fixture_document(|document| {
        document.revision = 9;
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "selected-part")
            .expect("the fixture selects a part")
            .locked = Some(true);
    });
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(13),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 9,
            transaction_id: "accepted-fixture".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: false,
            },
        }),
    };
    let mut model = ReadModel {
        accepted: Some(snapshot),
        active_board_id: "board".into(),
        selected_part_ids: vec!["selected-part".into()],
        ..ReadModel::default()
    };
    let context = objects::context_for_part(&model, "selected-part")
        .expect("fixture component has a current standalone tree context");
    let selected = objects::ScopedTreeContext { scope, context };
    model.selection_anchor_id = Some("selected-part".into());
    (model, selected)
}

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

    let projection = super::layout_component_inspector_projection(&model, Some(&selected), 1, 1)
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

#[derive(Clone)]
struct MountedProbe {
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected_part: std::rc::Rc<RefCell<Option<String>>>,
    context_override: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
    adapter_context: std::rc::Rc<RefCell<Option<objects::ScopedTreeContext>>>,
    selection_kind: std::rc::Rc<RefCell<Option<Signal<objects::LayoutSelectionKind>>>>,
    projection: std::rc::Rc<RefCell<Option<LayoutComponentInspectorProjection>>>,
    action: std::rc::Rc<RefCell<Option<EventHandler<LayoutComponentInspectorAction>>>>,
    pending_edits:
        std::rc::Rc<RefCell<Option<Signal<layout_component_edits::LayoutComponentInspectorEdits>>>>,
    render_generation: std::rc::Rc<RefCell<Option<Signal<u64>>>>,
    mirrored_half_override: std::rc::Rc<RefCell<Option<MirroredHalfProjection>>>,
    last_action: std::rc::Rc<RefCell<Option<LayoutComponentInspectorAction>>>,
    root_id: &'static str,
}

impl MountedProbe {
    fn select_parts(&self, part_ids: Vec<String>) {
        self.runtime
            .submit(boardstudio_application::Event::SelectParts {
                operation_id: self.runtime.operation(),
                part_ids,
                range_part_ids: Vec::new(),
                mode: boardstudio_application::SelectionMode::Replace,
            });
        crate::runtime::project_name_test_support::drive_pending(&self.runtime);
    }

    fn select(&self, part_id: Option<&str>) {
        *self.selected_part.borrow_mut() = part_id.map(str::to_owned);
        self.context_override.borrow_mut().take();
        self.select_parts(part_id.into_iter().map(str::to_owned).collect());
    }

    fn select_component_from_finding(&self, part_id: &str) {
        *self.selected_part.borrow_mut() = Some(part_id.to_owned());
        self.select_parts(vec![part_id.to_owned()]);
        let model = self.runtime.model();
        let scope = self.runtime.scope().expect("opened fixture scope");
        let context = objects::component_context_for_finding_part(&model, part_id)
            .expect("finding target remains a live Part");
        *self.context_override.borrow_mut() = Some(objects::ScopedTreeContext {
            scope,
            context: context.clone(),
        });
        let mut selection_kind = self
            .selection_kind
            .borrow()
            .expect("mounted selection-kind signal");
        if let Some(kind) = super::layout_workspace::selection_kind_for_tree_context(&context) {
            selection_kind.set(kind);
        }
    }

    /// Apply a real unrelated edit through the Session and CoreEngine: a project
    /// parameter plus a move of the selected part's Y, accepted and saved.
    async fn accept_unrelated_revision(&self) -> u64 {
        let accepted = self
            .runtime
            .model()
            .accepted
            .expect("fixture snapshot")
            .clone();
        let mut document = (*accepted.document).clone();
        document
            .parameters
            .insert("unrelated-inspector-edit".into(), serde_json::json!(42));
        document
            .parts
            .iter_mut()
            .find(|part| part.id == "selected-part")
            .expect("selected fixture part")
            .pose
            .at
            .y += 1.0;
        crate::runtime::project_name_test_support::submit_fixed_command(
            &self.runtime,
            boardstudio_core::model::EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "unrelated-inspector-edit".into(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec![document.id.clone()],
                operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            },
        );
        crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
        self.refresh();
        self.runtime
            .model()
            .accepted
            .expect("unrelated edit accepted")
            .document
            .revision
    }

    /// Force a rerender of the mounted host after pending effects ran.
    fn refresh(&self) {
        let mut generation = self
            .render_generation
            .borrow()
            .expect("mounted render-generation signal");
        generation += 1;
    }
}

#[component]
fn mounted_component_inspector_host() -> Element {
    let probe = use_context::<MountedProbe>();
    let render_generation = use_signal(|| 0u64);
    let _ = render_generation();
    *probe.render_generation.borrow_mut() = Some(render_generation);
    let workspace = use_signal(|| "Layout");
    let selection_kind = use_signal(objects::LayoutSelectionKind::default);
    let inspect_open = use_signal(|| true);
    let mut selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
    let mut show_component = use_signal(|| true);
    let anchor_scope = use_signal(|| None::<Scope>);
    let scope_generation = use_signal(|| 1u64);
    let adapter =
        use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
    let lifetime = use_hook(|| std::rc::Rc::new(LayoutComponentInspectorLifetime::default()));
    let pending_edits =
        use_signal(super::layout_component_edits::LayoutComponentInspectorEdits::default);
    *probe.pending_edits.borrow_mut() = Some(pending_edits);
    let x_draft = use_signal(String::new);
    let y_draft = use_signal(String::new);
    let margin_draft = use_signal(String::new);
    let x_failure = use_signal(|| None::<String>);
    let y_failure = use_signal(|| None::<String>);
    let margin_failure = use_signal(|| None::<String>);
    let error = use_signal(|| None::<String>);
    let layout_pending = use_signal(|| false);
    let constraint_pending = use_signal(|| false);
    let remove_pending = use_signal(|| false);
    let mirrored_half_pending = use_signal(|| false);
    let runtime = probe.runtime.clone();
    let model = runtime.model();
    let next_context = probe
        .context_override
        .borrow()
        .clone()
        .filter(|selected| {
            runtime.scope().as_ref() == Some(&selected.scope)
                && selection::context_is_current(&model, &selected.scope, &selected.context)
        })
        .or_else(|| {
            probe.selected_part.borrow().as_deref().and_then(|part_id| {
                let scope = runtime.scope()?;
                let context = objects::context_for_part(&model, part_id)?;
                Some(objects::ScopedTreeContext { scope, context })
            })
        });
    if selected_context.peek().clone() != next_context {
        selected_context.set(next_context.clone());
    }
    *probe.adapter_context.borrow_mut() = selected_context.peek().clone();
    let owner_key =
        layout_component_inspector_owner_key(&model, workspace(), next_context.as_ref());
    let generation = lifetime.update(owner_key);
    let mut projection = layout_component_inspector_projection(
        &model,
        next_context.as_ref(),
        generation,
        scope_generation(),
    );
    if let (Some(projection), Some(mirrored_half)) = (
        projection.as_mut(),
        probe.mirrored_half_override.borrow().clone(),
    ) {
        projection.mirrored_half = Some(mirrored_half);
    }
    let layout_owner = current_layout_owner(&runtime, workspace, &adapter);
    let action_runtime = runtime.clone();
    let action_adapter = adapter.clone();
    let action_lifetime = lifetime.clone();
    let action_workspace = workspace;
    let action_pending_edits = pending_edits;
    let last_action = probe.last_action.clone();
    let action_handler = EventHandler::new(move |action: LayoutComponentInspectorAction| {
        *last_action.borrow_mut() = Some(action.clone());
        dispatch_layout_component_inspector_action(
            &action_runtime,
            &action_adapter,
            &layout_owner,
            &action_lifetime,
            (action_workspace, inspect_open),
            action_pending_edits,
            action,
        )
    });
    *probe.projection.borrow_mut() = projection.clone();
    *probe.selection_kind.borrow_mut() = Some(selection_kind);
    *probe.action.borrow_mut() = Some(action_handler);
    let switch_probe = probe.clone();
    let restore_probe = probe.clone();
    let component_probe = probe.clone();
    let clear_probe = probe.clone();
    rsx! {
        style { {include_str!("../../assets/m1.css")} }
        button { id: "component-inspector-select-source", onclick: { let mut generation = render_generation; move |_| { switch_probe.select(Some("source-part")); generation += 1; } }, "Select source" }
        button { id: "component-inspector-select-original", onclick: { let mut generation = render_generation; move |_| { restore_probe.select(Some("selected-part")); generation += 1; } }, "Select original" }
        button { id: "component-inspector-select-component", onclick: { let mut generation = render_generation; move |_| { let part = component_probe.selected_part.borrow().clone().unwrap_or_else(|| "selected-part".into()); component_probe.select_component_from_finding(&part); generation += 1; } }, "Select component from finding" }
        button { id: "component-inspector-clear", onclick: { let mut generation = render_generation; move |_| { clear_probe.select(None); generation += 1; } }, "Clear selection" }
        button { id: "component-inspector-toggle-view", onclick: move |_| show_component.set(!show_component()), "Toggle Inspector view" }
        if let Some(projection) = projection {
            if show_component() {
                LayoutComponentInspector { projection, pending_edits, x: x_draft, y: y_draft, margin: margin_draft, x_failure, y_failure, margin_failure, error, layout_pending, constraint_pending, remove_pending, mirrored_half_pending, on_action: action_handler }
            }
        }
        output { class: "m1-layout-component-test-generation", "{render_generation()}" }
    }
}

/// The fixture document the runtime opens: a board, two positioned parts, two empty
/// matrices for the mirrored layouts, and the outline/relationship metadata the
/// projection tests read. Core owns revisions from here on.
fn runtime_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
    fixture_document(|document| {
        for matrix_id in ["matrix", "paired-matrix"] {
            document.matrices.push(Matrix {
                id: matrix_id.into(),
                name: Some(matrix_id.into()),
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
                    enabled: false,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                }],
            });
        }
        configure(document);
    })
}

fn configure_mirrored_part_pair(document: &mut ProjectDoc) {
    document.boards[0].part_ids = vec!["selected-part".into()];
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
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "source-part")
        .unwrap()
        .pose
        .at
        .x = -4.0;
    document.layouts.clear();
    document.matrices.clear();
    document.hardware = Some(
        serde_json::from_value(serde_json::json!({
            "topology":"split", "transport":"wireless",
            "instances":[
                {"id":"source-instance","name":"Left half","boardId":"board","half":"left","role":"primary","flipped":false,"controllerPartId":null,"mechanical":null},
                {"id":"mirror-instance","name":"Right half","boardId":"mirror-board","half":"right","role":"secondary","flipped":true,"controllerPartId":null,"mechanical":null}
            ],
            "pairs":[{
                "id":"pair/A","members":["source-instance","mirror-instance"],
                "mapping":{"at":{"x":0.0,"y":0.0},"rotation":0.0},
                "construction":{"method":"printed","mount":"tray","integratedPlateFrame":false,"plateThickness":1.5,"plateFoamThickness":0.0,"bottomFoamThickness":0.0,"bottomThickness":1.5,"plateToPcb":5.0,"wallThickness":2.0,"clearance":0.2},
                "constructionSource":"source-instance",
                "halves":{"sourceBoardId":"board","mirrorBoardId":"mirror-board","correspondences":[
                    {"kind":"part","sourceId":"selected-part","mirrorId":"source-part"}
                ]}
            }]
        }))
        .expect("the mirrored board pair is valid"),
    );
}

/// A runtime whose Core requests run through the in-process engine and whose saves stay
/// in memory, opened on the fixture document with the given parts selected.
async fn mounted_runtime_probe(
    root_id: &'static str,
    configure: impl FnOnce(&mut ProjectDoc),
    selected_parts: &[&str],
) -> (MountedProbe, web_sys::Element) {
    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::install(
        &runtime,
        boardstudio_application::Session::new(),
        boardstudio_core::CoreEngine::new(),
    );
    crate::runtime::project_name_test_support::install_memory_persistence(&runtime);
    let document = runtime_document(configure);
    let open = runtime.operation();
    runtime.submit(boardstudio_application::Event::Open {
        operation_id: open,
        document,
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;
    runtime.model().accepted.expect(
        "the mounted Inspector fixture must be accepted by Session/Core Open before selection",
    );
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: selected_parts.iter().map(|id| (*id).into()).collect(),
        range_part_ids: Vec::new(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;
    let probe = MountedProbe {
        runtime,
        selected_part: std::rc::Rc::new(RefCell::new(Some(
            selected_parts
                .first()
                .map(|id| (*id).to_owned())
                .unwrap_or_default(),
        ))),
        context_override: std::rc::Rc::default(),
        adapter_context: std::rc::Rc::default(),
        selection_kind: std::rc::Rc::default(),
        projection: std::rc::Rc::default(),
        action: std::rc::Rc::default(),
        pending_edits: std::rc::Rc::default(),
        render_generation: std::rc::Rc::default(),
        mirrored_half_override: std::rc::Rc::default(),
        last_action: std::rc::Rc::default(),
        root_id,
    };
    let document_element = web_sys::window().unwrap().document().unwrap();
    let root = document_element.create_element("div").unwrap();
    root.set_id(probe.root_id);
    document_element
        .body()
        .unwrap()
        .append_child(&root)
        .unwrap();
    let dom = VirtualDom::new(mounted_component_inspector_host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    (probe, root)
}

async fn mounted_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(root_id, |_| {}, &["selected-part"]).await
}

async fn mounted_probe_configured(
    root_id: &'static str,
    configure: impl FnOnce(&mut ProjectDoc),
) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(root_id, configure, &["selected-part"]).await
}

async fn mounted_group_probe(root_id: &'static str) -> (MountedProbe, web_sys::Element) {
    mounted_runtime_probe(root_id, |_| {}, &["selected-part", "source-part"]).await
}

async fn settle_component_inspector(root_id: &str, mut generation: Signal<u64>) {
    generation += 1;
    let expected = generation.peek().to_string();
    wait_until(
        "the mounted component Inspector reflects its render generation",
        || {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!(
                    "#{} .m1-layout-component-test-generation",
                    root_id
                ))
                .ok()
                .flatten()
                .is_some_and(|label| label.text_content().as_deref() == Some(expected.as_str()))
        },
    )
    .await;
}

async fn settle_mounted_component_inspector(probe: &MountedProbe) {
    wait_until("the mounted component Inspector host renders", || {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} .m1-layout-component-test-generation",
                probe.root_id
            ))
            .unwrap()
            .is_some()
    })
    .await;
    let generation =
        { (*probe.render_generation.borrow()).expect("mounted render-generation signal") };
    settle_component_inspector(probe.root_id, generation).await;
}

fn click_component_inspector(root_id: &str, selector: &str) {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} {selector}"))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

#[wasm_bindgen_test]
async fn mounted_mirrored_half_actions_work_from_either_board_and_undo() {
    use boardstudio_core::model::{HalfExceptionState, MirroredExceptionState};
    use boardstudio_web_ui_shared::test_wait::wait_until_step;

    let (probe, root) = mounted_runtime_probe(
        "layout-mirrored-half-round-trip-test-root",
        configure_mirrored_part_pair,
        &["selected-part"],
    )
    .await;
    async fn drive_to_state(
        probe: &MountedProbe,
        expected: HalfExceptionState,
        previous_revision: u64,
    ) {
        wait_until_step("the mirrored-half state is accepted", async || {
            accept_pending(probe).await;
            let state = probe
                .runtime
                .model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .hardware
                .as_ref()
                .unwrap()
                .pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .exceptions
                .iter()
                .find(|exception| exception.mirror_id == "source-part")
                .map(|exception| match exception.state {
                    MirroredExceptionState::Local => HalfExceptionState::Local,
                    MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
                })
                .unwrap_or(HalfExceptionState::Linked);
            let model = probe.runtime.model();
            state == expected
                && model
                    .accepted
                    .as_ref()
                    .is_some_and(|accepted| accepted.document.revision > previous_revision)
        })
        .await;
        let accepted_state = probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .hardware
            .as_ref()
            .unwrap()
            .pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions
            .iter()
            .find(|exception| exception.mirror_id == "source-part")
            .map(|exception| match exception.state {
                MirroredExceptionState::Local => HalfExceptionState::Local,
                MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
            })
            .unwrap_or(HalfExceptionState::Linked);
        assert_eq!(
            accepted_state,
            expected,
            "runtime error: {:?}; revision: {} (expected after {})",
            probe.runtime.model().last_error,
            probe.runtime.model().accepted.unwrap().document.revision,
            previous_revision,
        );
        use boardstudio_web_ui_shared::test_wait::wait_until;
        wait_until(
            "the Inspector projects the accepted mirrored-half state",
            || {
                probe.refresh();
                probe
                    .projection
                    .borrow()
                    .as_ref()
                    .and_then(|projection| projection.mirrored_half.as_ref())
                    .is_some_and(|half| half.state == expected)
            },
        )
        .await;
    }

    use boardstudio_web_ui_shared::test_wait::wait_until;
    wait_until("the source part Inspector mounts", || {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.mirrored_half.as_ref())
            .is_some_and(|half| half.state == HalfExceptionState::Linked)
    })
    .await;
    let revision = accepted_revision(&probe);
    click_component_inspector(probe.root_id, ".m1-layout-component-mirror-state > summary");
    click_component_inspector(probe.root_id, "#component-inspector-mirrored-half-omit");
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetMirroredHalfState {
            state: HalfExceptionState::Omitted,
            ..
        })
    ));
    assert!(
        probe
            .pending_edits
            .borrow()
            .expect("mounted pending edits")
            .peek()
            .is_pending(&layout_component_edits::InspectorField::MirroredHalfState)
    );
    drive_to_state(&probe, HalfExceptionState::Omitted, revision).await;

    let revision = accepted_revision(&probe);
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    drive_to_state(&probe, HalfExceptionState::Linked, revision).await;

    probe
        .runtime
        .submit(boardstudio_application::Event::Navigate {
            operation_id: probe.runtime.operation(),
            board_id: "mirror-board".into(),
            instance_id: None,
        });
    crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.select(Some("source-part"));
    probe.refresh();
    wait_until("the counterpart Inspector mounts", || {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.mirrored_half.as_ref())
            .is_some_and(|half| half.state == HalfExceptionState::Linked)
    })
    .await;
    wait_until("the mirror-board selection owns the Inspector", || {
        probe
            .adapter_context
            .borrow()
            .as_ref()
            .is_some_and(|selected| {
                selected.scope.board_id == "mirror-board"
                    && matches!(
                        &selected.context,
                        objects::TreeContext::Component { part_id: Some(part_id), .. }
                            if part_id == "source-part"
                    )
            })
    })
    .await;
    let revision = accepted_revision(&probe);
    click_component_inspector(probe.root_id, ".m1-layout-component-mirror-state > summary");
    click_component_inspector(probe.root_id, "#component-inspector-mirrored-half-local");
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetMirroredHalfState {
            state: HalfExceptionState::Local,
            ..
        })
    ));
    assert!(
        probe
            .pending_edits
            .borrow()
            .expect("mounted pending edits")
            .peek()
            .is_pending(&layout_component_edits::InspectorField::MirroredHalfState)
    );
    drive_to_state(&probe, HalfExceptionState::Local, revision).await;

    let revision = accepted_revision(&probe);
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    drive_to_state(&probe, HalfExceptionState::Linked, revision).await;
    root.remove();
}

/// Ticket 12: the source layout and its mirror partner with a stored gap,
/// plus a mirrored controller pair. The partner pose is the half-frame
/// reflection of the source pose (stored x negates).
fn configure_gapped_part_mirror(document: &mut ProjectDoc) {
    let link = document
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "paired-layout")
        .expect("mirror layout")
        .mirror_link
        .as_mut()
        .expect("mirror link");
    link.axis_x = 40.0;
    link.split_gap = Some(40.0);
    link.correspondences.push(MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        source_id: "selected-part".into(),
        mirror_id: "selected-part-mirror".into(),
    });
    document.parts.push(Part {
        keycap: None,
        outline: None,
        id: "selected-part-mirror".into(),
        definition_id: "component-definition".into(),
        reference: "U3".into(),
        pose: Pose2 {
            at: Vec2 { x: -4.0, y: 3.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: Some(false),
        properties: None,
        generator_parameters: None,
    });
    document.boards[0]
        .part_ids
        .push("selected-part-mirror".into());
    document
        .layouts
        .iter_mut()
        .find(|layout| layout.id == "paired-layout")
        .expect("mirror layout")
        .part_ids
        .push("selected-part-mirror".into());
}

/// Ticket 12: the "Mirror on both halves" toggle dispatches SetMirroredPart
/// each way — off removes the partner, on recreates it — with one Undo entry
/// per flip.
#[wasm_bindgen_test]
async fn mounted_part_mirror_toggle_mirrors_and_unmirrors_with_undo() {
    use boardstudio_web_ui_shared::test_wait::wait_until_step;

    async fn settled_part_mirror(probe: &MountedProbe, expected: Option<bool>) -> bool {
        accept_pending(probe).await;
        accept_pending_ready(probe)
            && probe
                .projection
                .borrow()
                .as_ref()
                .and_then(|projection| projection.part_mirror)
                == expected
    }

    let (probe, root) = mounted_runtime_probe(
        "layout-part-mirror-toggle-test-root",
        configure_gapped_part_mirror,
        &["selected-part"],
    )
    .await;

    fn part_mirror(probe: &MountedProbe) -> Option<bool> {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.part_mirror)
    }

    fn accepted_part_records(probe: &MountedProbe) -> Vec<MirroredCorrespondence> {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .expect("fixture snapshot")
            .document
            .layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| link.correspondences.clone())
            .filter(|record| record.kind == MirroredItemKind::Part)
            .collect()
    }

    fn toggle_part_mirror(root_id: &str, mirrored: bool) {
        let checkbox = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{root_id} #component-inspector-part-mirror"))
            .unwrap()
            .expect("part mirror toggle")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        checkbox.set_checked(mirrored);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        checkbox
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
            .unwrap();
    }

    // The independent part shows the ticket-12 toggle instead of the member
    // exception buttons — checked while its Part correspondence is live.
    wait_until("the part mirror toggle mounts checked", || {
        probe.refresh();
        part_mirror(&probe) == Some(true)
    })
    .await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} #component-inspector-mirrored-half-local",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "independent parts use the toggle, not the member exception buttons"
    );

    // Off: the partner and its record go away together, in one Undo entry.
    let revision = accepted_revision(&probe);
    toggle_part_mirror(probe.root_id, false);
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetPartMirrored {
            part_id,
            mirrored: false,
            ..
        }) if part_id == "selected-part"
    ));
    wait_until_step("unmirroring is accepted", async || {
        accept_pending(&probe).await;
        accept_pending_ready(&probe)
            && accepted_part_records(&probe).is_empty()
            && part_mirror(&probe) == Some(false)
    })
    .await;
    assert!(
        accepted_revision(&probe) > revision,
        "the toggle off commits one Undo entry"
    );
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .expect("fixture snapshot")
            .document
            .parts
            .iter()
            .all(|part| part.id != "selected-part-mirror"),
        "the partner part is removed with its record"
    );

    // On: the partner returns at the reflected pose, in one Undo entry.
    let revision = accepted_revision(&probe);
    toggle_part_mirror(probe.root_id, true);
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetPartMirrored {
            part_id,
            mirrored: true,
            ..
        }) if part_id == "selected-part"
    ));
    wait_until_step("remirroring is accepted", async || {
        accept_pending(&probe).await;
        accept_pending_ready(&probe) && part_mirror(&probe) == Some(true)
    })
    .await;
    assert!(
        accepted_revision(&probe) > revision,
        "the toggle on commits one Undo entry"
    );
    let records = accepted_part_records(&probe);
    assert_eq!(records.len(), 1);
    let partner = probe
        .runtime
        .model()
        .accepted
        .as_ref()
        .expect("fixture snapshot")
        .document
        .parts
        .iter()
        .find(|part| part.id == records[0].mirror_id)
        .expect("the partner part returns with its record")
        .clone();
    assert_eq!(partner.pose.at, Vec2 { x: -4.0, y: 3.0 });

    // One Undo entry each way: undo unmirrors, undo again remirrors.
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    wait_until_step("undo unmirrors the part", async || {
        settled_part_mirror(&probe, Some(false)).await
    })
    .await;
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    wait_until_step("undo remirrors the part", async || {
        settled_part_mirror(&probe, Some(true)).await
    })
    .await;
    root.remove();
}

/// Whether the mounted Inspector has settled on the latest accepted revision.
fn accept_pending_ready(probe: &MountedProbe) -> bool {
    probe.pending_edits.borrow().as_ref().is_none_or(|edits| {
        use super::layout_component_edits::InspectorField;
        !edits.peek().is_pending(&InspectorField::PartMirror)
    })
}

/// The position of a fixture part in the accepted document.
fn accepted_position(probe: &MountedProbe, part_id: &str) -> Vec2 {
    probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .document
        .parts
        .iter()
        .find(|part| part.id == part_id)
        .unwrap_or_else(|| panic!("{part_id} is in the accepted document"))
        .pose
        .at
}

fn accepted_revision(probe: &MountedProbe) -> u64 {
    probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .document
        .revision
}

/// Drive any held effects to completion so accepted state reflects every submitted edit.
async fn accept_pending(probe: &MountedProbe) {
    crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.refresh();
    settle_mounted_component_inspector(probe).await;
}

#[wasm_bindgen_test]
async fn mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector() {
    let (probe, _root) = mounted_runtime_probe(
        "layout-matrix-part-inspector-test-root",
        |document| {
            // Core generates matrix members with its own ids; drop the standalone
            // selected part so the generated member takes over as the selection.
            document.parts.retain(|part| part.id == "source-part");
            document
                .boards
                .iter_mut()
                .for_each(|board| board.part_ids.retain(|id| id != "selected-part"));
            document.layouts.clear();
            document.layouts.push(Layout {
                id: "component-layout".into(),
                name: "Component half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["component-matrix".into()],
                part_ids: vec![],
                mirror_link: None,
            });
            document.layouts.push(Layout {
                id: "component-mirror-layout".into(),
                name: "Component mirror".into(),
                board_id: "board".into(),
                matrix_ids: vec!["component-paired-matrix".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    correspondences: vec![MirroredCorrespondence {
                        companion_mode: None,
                        companion_tweak: None,
                        kind: MirroredItemKind::Matrix,
                        source_id: "component-matrix".into(),
                        mirror_id: "component-paired-matrix".into(),
                    }],
                    exceptions: vec![],
                    source_id: "component-layout".into(),
                    axis_x: 0.0,
                    split_gap: None,
                    spacing: None,
                }),
            });
            document.matrices.clear();
            document.matrices.push(Matrix {
                id: "component-matrix".into(),
                name: Some("keys".into()),
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
            document.matrices.push(Matrix {
                id: "component-paired-matrix".into(),
                name: Some("component-paired-matrix".into()),
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
        },
        &["matrix/component-paired-matrix/r0c0"],
    )
    .await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Key,
        "ordinary matrix primary selection remains in Key mode"
    );
    assert!(probe.projection.borrow().is_none());
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} .m1-layout-component-inspector",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "ordinary matrix primary selection remains in Key mode"
    );

    click_component_inspector(probe.root_id, "#component-inspector-select-component");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .map(|projection| projection.reference.as_str()),
        Some("SW1"),
        "explicit finding navigation must project the selected matrix Part (Core numbers matrix members as switches)"
    );
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Part,
        "explicit finding navigation publishes the matching Part toolbar mode"
    );
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} .m1-layout-component-inspector",
                probe.root_id
            ))
            .unwrap()
            .is_some(),
        "the production component Inspector must mount for an explicit live Part context"
    );
    _root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_production_handler_rejects_selection_aba_and_unmount() {
    let (probe, root) = mounted_probe("layout-component-inspector-aba-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    let old_projection = probe.projection.borrow().clone().unwrap();
    let old_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "#component-inspector-select-source");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(*probe.selected_part.borrow(), Some("source-part".into()));
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().reference,
        "U2",
        "component selection changes must rerender the mounted Inspector"
    );
    click_component_inspector(probe.root_id, "#component-inspector-select-original");
    settle_mounted_component_inspector(&probe).await;
    assert_ne!(
        old_projection.owner.context_generation,
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        "the production owner lifetime must advance through the selection ABA"
    );
    let before = accepted_position(&probe, "selected-part");
    old_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: old_projection.owner.clone(),
        axis: ComponentPositionAxis::X,
        value: 42.0,
        submitted_draft: "42".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        before,
        "a stale owner action changes nothing"
    );

    let current = probe.projection.borrow().clone().unwrap();
    probe
        .action
        .borrow()
        .as_ref()
        .unwrap()
        .call(LayoutComponentInspectorAction::SetPosition {
            owner: current.owner,
            axis: ComponentPositionAxis::X,
            value: 42.0,
            submitted_draft: "42".into(),
        });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part").x,
        42.0,
        "the current owner's position edit lands"
    );

    let stale = probe.projection.borrow().clone().unwrap();
    let stale_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "#component-inspector-clear");
    settle_mounted_component_inspector(&probe).await;
    let cleared = accepted_position(&probe, "selected-part");
    stale_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: stale.owner,
        axis: ComponentPositionAxis::X,
        value: 43.0,
        submitted_draft: "43".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        cleared,
        "an action after selection clear changes nothing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_margin_enter_commits_and_escape_restores_the_accepted_value() {
    let (probe, root) = mounted_probe("layout-component-inspector-margin-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    settle_mounted_component_inspector(&probe).await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#{} .m1-layout-component-outline", probe.root_id))
            .unwrap()
            .unwrap()
            .has_attribute("open"),
        "the margin field must be in the expanded outline section"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let input = document
        .query_selector(&format!(
            "#{} input[aria-label='Part edge margin']",
            probe.root_id
        ))
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.focus().unwrap();
    assert_eq!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "the visible margin field should receive focus before keyboard input"
    );
    input.set_value("4.25");
    let input_init = web_sys::EventInit::new();
    input_init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    assert_ne!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "Enter must blur the margin field so the normal commit runs"
    );
    accept_pending(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "selected-part")
            .and_then(|part| part.outline.as_ref())
            .and_then(|outline| outline.margin),
        Some(4.25),
        "margin Enter commits the draft to the accepted document"
    );

    input.focus().unwrap();
    input.set_value("8.0");
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
    accept_pending(&probe).await;
    assert_eq!(
        input.value(),
        "4.25",
        "Escape restores the accepted value, which the Enter commit just moved"
    );
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .clone()
            .unwrap()
            .document
            .revision,
        accepted.document.revision,
        "Escape cancels the draft without another accepted edit"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_position_draft_survives_unrelated_acceptance_and_blur_uses_latest_owner()
{
    let (probe, root) = mounted_probe("layout-component-inspector-refresh-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let input = |label: &str| {
        document
            .query_selector(&format!("#{} input[aria-label='{label}']", probe.root_id))
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    };
    let type_value = |field: &web_sys::HtmlInputElement, value: &str| {
        field.focus().unwrap();
        field.set_value(value);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        field
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
    };

    click_component_inspector(probe.root_id, ".m1-layout-component-outline summary");
    click_component_inspector(probe.root_id, ".m1-layout-component-constraint summary");
    settle_mounted_component_inspector(&probe).await;
    let x = input("X mm");
    type_value(&x, "7.25");

    let initial_revision = accepted_revision(&probe);
    let initial_context_generation = probe
        .projection
        .borrow()
        .as_ref()
        .unwrap()
        .owner
        .context_generation;
    // Advance accepted state without changing focus. Driving a control here would blur and
    // submit the draft before the unrelated acceptance being tested.
    let accepted_revision = probe.accept_unrelated_revision().await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        accepted_revision
    );
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        initial_context_generation,
        "accepted revisions refresh action capture without ending component selection"
    );
    assert_eq!(
        x.value(),
        "7.25",
        "dirty focused X draft survives the refresh"
    );
    assert_eq!(
        input("Y mm").value(),
        "4.00",
        "clean Y follows the accepted position"
    );
    for selector in [
        ".m1-layout-component-outline",
        ".m1-layout-component-constraint",
    ] {
        assert!(
            document
                .query_selector(&format!("#{} {selector}", probe.root_id))
                .unwrap()
                .unwrap()
                .has_attribute("open"),
            "disclosure {selector} survives unrelated acceptance"
        );
    }

    let _ = x.blur();
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 7.25, y: 4.0 },
        "blur lands the retained X draft with the accepted Y against the refreshed owner"
    );
    assert!(accepted_revision > initial_revision);

    // The Relationships section beside the fields survives a further unrelated
    // acceptance on the same mounted Inspector these draft checks use.
    let relations_revision = probe.accept_unrelated_revision().await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        relations_revision
    );
    assert!(
        document
            .query_selector(&format!(
                "#{} #m1-component-relationships-heading",
                probe.root_id
            ))
            .unwrap()
            .is_some(),
        "Relationships section survives an unrelated accepted revision"
    );
    root.remove();
}

fn position_input(root_id: &str, label: &str) -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} input[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("{label} input is rendered"))
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
}

/// Type `value` into a position field and try every commit boundary the
/// Inspector listens to (Enter, blur, focusout) without relying on focus, so a
/// disabled field is exercised through the same handlers.
fn try_commit_position(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    for name in ["blur", "focusout"] {
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict(name, &bubbling).unwrap())
            .unwrap();
    }
}

#[wasm_bindgen_test]
async fn mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y() {
    let (probe, root) = mounted_probe_configured(
        "layout-component-position-queued-xy-test-root",
        |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        },
    )
    .await;
    settle_mounted_component_inspector(&probe).await;
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);

    // Hold the first Core reply so the Y edit queues behind the X edit, exactly as
    // rapid typing does when the engine is busy.
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);

    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("the X edit reached the in-process Core");

    y.focus().unwrap();
    y.set_value("-40");
    y.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    y.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    release.send(()).expect("release the held X reply");
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 60.0, y: -40.0 },
        "queued X then Y edits must both survive: the Y edit resolves against the document the X edit produced"
    );

    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 {
            x: 60.0,
            y: -47.625
        },
        "one Undo removes only the Y change"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_view_unmount_retires_held_field_observation_and_remount_refreshes()
 {
    let (probe, root) = mounted_probe("layout-component-position-unmount-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let x = position_input(probe.root_id, "X mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.focus().unwrap();
    x.set_value("60");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered.await.expect("the position edit reached Core");

    click_component_inspector(probe.root_id, "#component-inspector-toggle-view");
    settle_mounted_component_inspector(&probe).await;
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} [aria-label='Component inspector']",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "the view-only toggle unmounts the Inspector"
    );
    assert_eq!(*probe.selected_part.borrow(), Some("selected-part".into()));
    click_component_inspector(probe.root_id, "#component-inspector-toggle-view");
    settle_mounted_component_inspector(&probe).await;
    release
        .send(())
        .expect("release the held position result after remount");
    accept_pending(&probe).await;
    assert_eq!(accepted_position(&probe, "selected-part").x, 60.0);
    assert_eq!(position_input(probe.root_id, "X mm").value(), "60.00");
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_edit_retires_when_the_part_becomes_locked_before_execution() {
    let (probe, root) = mounted_probe("layout-component-position-retire-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    // Hold the lock edit's Core reply so the position edit queues behind it.
    let (entered, release) =
        crate::runtime::project_name_test_support::gate_next_core_reply(&probe.runtime);
    let accepted = probe
        .runtime
        .model()
        .accepted
        .expect("fixture snapshot")
        .clone();
    let mut replacement = (*accepted.document).clone();
    replacement
        .parts
        .iter_mut()
        .find(|part| part.id == "selected-part")
        .unwrap()
        .locked = Some(true);
    crate::runtime::project_name_test_support::submit_fixed_command(
        &probe.runtime,
        boardstudio_core::model::EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "lock-before-position".into(),
            phase: boardstudio_core::model::EditPhase::Commit,
            target_ids: vec!["selected-part".into()],
            operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        },
    );
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    entered
        .await
        .expect("the lock edit reached the in-process Core");

    let x = position_input(probe.root_id, "X mm");
    x.focus().unwrap();
    x.set_value("7.5");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
    )
    .unwrap();
    crate::runtime::project_name_test_support::drive_pending(&probe.runtime);
    release.send(()).expect("release the held lock edit");
    // The parked task drives the lock edit and the queued position edit to completion
    // on its own; only afterwards does a forced rerender settle the ticket.
    settle_mounted_component_inspector(&probe).await;
    probe.refresh();
    settle_mounted_component_inspector(&probe).await;
    assert!(
        accepted_position(&probe, "selected-part") == Vec2 { x: 4.0, y: 3.0 },
        "a retired edit moves nothing"
    );
    let alert = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{} [role='alert']", probe.root_id))
        .unwrap()
        .expect("the retirement is explained inline")
        .text_content()
        .unwrap();
    assert!(
        alert.contains("locked"),
        "the inline message explains why the edit retired: {alert}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_position_untouched_blur_and_escape_cancel_submit_no_edit() {
    let (probe, root) = mounted_probe_configured(
        "layout-component-position-untouched-blur-test-root",
        |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        },
    )
    .await;
    settle_mounted_component_inspector(&probe).await;
    let initial_revision = accepted_revision(&probe);
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);

    // Merely moving through both fields must not turn their two-decimal
    // presentation into a coordinate edit on blur.
    x.focus().unwrap();
    y.focus().unwrap();
    x.focus().unwrap();

    // Escape cancels a real draft. The ensuing blur must not submit the
    // restored, formatted display value as a replacement for raw geometry.
    x.set_value("71.125");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
    )
    .unwrap();
    let _ = x.blur();
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 {
            x: 66.675,
            y: -47.625
        },
        "untouched focus/blur and Escape-cancel must not move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "untouched focus/blur and Escape-cancel must accept no edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_group_position_blur_moves_all_selected_parts_from_first_anchor() {
    let (probe, root) = mounted_group_probe("layout-component-group-position-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    let document = web_sys::window().unwrap().document().unwrap();
    assert_eq!(
        document
            .query_selector(&format!(
                "#{} .m1-layout-component-selection-note",
                probe.root_id
            ))
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref(),
        Some("2 parts selected. Position edits apply to the selection.")
    );
    let x = position_input(probe.root_id, "X mm");
    assert!(!x.disabled());
    try_commit_position(&x, "5.5");
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 5.5, y: 3.0 },
        "the group position edit moves the anchor part"
    );
    assert_eq!(
        accepted_position(&probe, "source-part"),
        Vec2 { x: 11.5, y: 3.0 },
        "the group position edit translates every selected part by the anchor delta"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_locked_component_presents_xy_unavailable_and_suppresses_commit() {
    let (probe, root) =
        mounted_probe_configured("layout-component-position-locked-test-root", |document| {
            document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap()
                .locked = Some(true);
        })
        .await;
    settle_mounted_component_inspector(&probe).await;
    assert!(probe.projection.borrow().as_ref().unwrap().locked);
    let initial_revision = accepted_revision(&probe);
    for (label, value) in [("X mm", "50"), ("Y mm", "60")] {
        let input = position_input(probe.root_id, label);
        assert!(
            input.disabled(),
            "{label} must be unavailable for a locked part"
        );
        try_commit_position(&input, value);
    }
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 3.0 },
        "locked position commits never move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "locked position commits accept no edits"
    );
    root.remove();
}

fn offset_form_select(root_id: &str, label: &str) -> web_sys::HtmlSelectElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#{root_id} select[aria-label='{label}']"))
        .unwrap()
        .unwrap_or_else(|| panic!("the {label} choice mounts"))
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap()
}

/// The Offset X draft, the Offset axes choice and the Source part choice the form shows.
fn offset_form(root_id: &str) -> (String, String, String) {
    (
        position_input(root_id, "Offset X (mm)").value(),
        offset_form_select(root_id, "Offset axes").value(),
        offset_form_select(root_id, "Constraint source part").value(),
    )
}

fn choose_offset_axes_in_form(root_id: &str, value: &str) {
    let select = offset_form_select(root_id, "Offset axes");
    select.set_value(value);
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    select
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &bubbling).unwrap())
        .unwrap();
}

/// The Properties form takes its source, axes and numbers from `offset_drafts` on every
/// path: the first render, a change of selection, an accepted change of the saved constraint
/// (a conversion and its undo) and the reset when Properties is shown again.
#[wasm_bindgen_test]
async fn mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_the_reset() {
    const KEY: &str = "matrix/component-matrix/r0c0";
    let (probe, root) = mounted_probe_configured("layout-offset-form-test-root", |document| {
        // The key sits between the two standalone parts, so the first part that is not
        // the selected one is a key for one selection and not for the other.
        document.boards[0].part_ids =
            vec!["selected-part".into(), KEY.into(), "source-part".into()];
        let mut key = document.parts[0].clone();
        key.id = KEY.into();
        key.reference = "SW1".into();
        key.outline = None;
        key.pose.rotation = 30.0;
        document.parts.push(key);
        document.matrices.push(Matrix {
            id: "component-matrix".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "component-definition".into(),
            part_ids: vec![KEY.into()],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![boardstudio_core::model::ColumnLayout::default()],
            anchor: None,
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: Some(30.0),
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        document.constraints.push(Constraint::PartOffset {
            id: "held".into(),
            source_part_id: KEY.into(),
            target_part_id: "selected-part".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        });
    })
    .await;
    settle_mounted_component_inspector(&probe).await;
    let order: Vec<(String, bool)> = probe
        .projection
        .borrow()
        .as_ref()
        .expect("the selected part projects")
        .board_parts
        .iter()
        .map(|part| (part.id.clone(), part.matrix_key))
        .collect();
    assert_eq!(
        order[..3],
        [
            ("selected-part".to_owned(), false),
            (KEY.to_owned(), true),
            ("source-part".to_owned(), false),
        ],
        "the fixture's board order and key flags"
    );
    let root_id = probe.root_id;

    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .expect("fixture snapshot")
            .document
            .parts
            .iter()
            .find(|part| part.id == KEY)
            .expect("the source key is accepted")
            .pose
            .rotation,
        30.0,
        "the accepted source key must retain its authored 30 degree heading before conversion"
    );

    // First render: the saved part-offset constraint.
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "first render"
    );

    // A selection change to a part without a constraint: its first other part is the
    // selected-part, which is not a key, so the default is board axes and zeros.
    click_component_inspector(root_id, "#component-inspector-select-source");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "board".into(), "selected-part".into()),
        "after selecting an unconstrained part"
    );

    // Back to the constrained part: its saved kind, source and numbers again.
    click_component_inspector(root_id, "#component-inspector-select-original");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "after selecting the constrained part again"
    );

    // A conversion re-expresses the numbers on the board axes at the key's 30 degree turn.
    choose_offset_axes_in_form(root_id, "board");
    accept_pending(&probe).await;
    let (x, axes, source) = offset_form(root_id);
    assert!(
        (x.parse::<f64>().unwrap() - 2.866_025_403_784_439).abs() < 1e-6,
        "converted X draft {x}"
    );
    assert_eq!((axes.as_str(), source.as_str()), ("board", KEY));

    // Undoing the conversion restores the saved part-offset constraint and its numbers.
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "part".into(), KEY.into()),
        "after undoing the conversion"
    );

    // The reset: an unconstrained part's unsaved axes choice goes back to the default when
    // another owner is shown again.
    click_component_inspector(root_id, "#component-inspector-select-source");
    settle_mounted_component_inspector(&probe).await;
    choose_offset_axes_in_form(root_id, "part");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(offset_form(root_id).1, "part", "the unsaved choice shows");
    click_component_inspector(root_id, "#component-inspector-select-original");
    settle_mounted_component_inspector(&probe).await;
    click_component_inspector(root_id, "#component-inspector-select-source");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "board".into(), "selected-part".into()),
        "after the reset"
    );
    root.remove();
}

// ---- Matrix Relationships section: the real Layout Inspector over a real Session ----

mod matrix_relations_panel {
    use super::super::layout_workspace;
    use super::super::*;
    use super::{click_component_inspector, settle_component_inspector, wait_until};
    use boardstudio_application::{Event, Session};
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{
        CoreReply, CoreRequest, EditCommand, EditPhase, ProjectDoc, Vec2,
    };
    use serde_json::json;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen_test::wasm_bindgen_test;

    const MATRIX: &str = "main";

    #[derive(Clone)]
    struct RelationsProbe {
        runtime: Rc<crate::runtime::Runtime>,
        root_id: &'static str,
        render_generation: Rc<RefCell<Option<Signal<u64>>>>,
        version: Rc<RefCell<Option<Signal<u64>>>>,
        selected: Rc<RefCell<Option<Signal<Option<objects::ScopedTreeContext>>>>>,
        adapter: Rc<RefCell<Option<SelectionAdapter>>>,
    }

    impl RelationsProbe {
        fn scope(&self) -> Scope {
            self.runtime.scope().expect("opened project scope")
        }

        fn select(&self, context: objects::TreeContext) {
            let adapter = self.adapter.borrow().clone().expect("mounted adapter");
            selection::submit_context(
                &self.runtime,
                &adapter,
                selection::ContextRequest {
                    scope: self.scope(),
                    context,
                    mode: boardstudio_application::SelectionMode::Replace,
                },
            );
        }

        fn selected_context(&self) -> Option<objects::TreeContext> {
            let signal = (*self.selected.borrow())?;
            signal
                .peek()
                .as_ref()
                .map(|selected| selected.context.clone())
        }

        fn bump(&self) {
            let mut version = self.version.borrow().expect("mounted version signal");
            version += 1;
        }

        async fn settle(&self) {
            wait_until("the mounted relations Inspector host renders", || {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .query_selector(&format!(
                        "#{} .m1-layout-component-test-generation",
                        self.root_id
                    ))
                    .unwrap()
                    .is_some()
            })
            .await;
            let generation =
                { (*self.render_generation.borrow()).expect("mounted render-generation signal") };
            settle_component_inspector(self.root_id, generation).await;
        }

        /// Drive every pending effect, re-render the host and let effects settle.
        async fn land(&self) {
            crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
            self.settle().await;
            crate::runtime::project_name_test_support::run_pending(&self.runtime).await;
            self.bump();
            self.settle().await;
        }

        fn text(&self) -> String {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id(self.root_id)
                .unwrap()
                .text_content()
                .unwrap_or_default()
        }

        fn click(&self, selector: &str) {
            click_component_inspector(self.root_id, selector);
        }

        fn exists(&self, selector: &str) -> bool {
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector(&format!("#{} {selector}", self.root_id))
                .unwrap()
                .is_some()
        }

        fn matrix(&self) -> boardstudio_core::model::Matrix {
            self.runtime
                .model()
                .accepted
                .expect("accepted document")
                .document
                .matrices
                .iter()
                .find(|matrix| matrix.id == MATRIX)
                .cloned()
                .expect("the matrix exists")
        }
    }

    #[component]
    fn relations_host() -> Element {
        let probe = use_context::<RelationsProbe>();
        let render_generation = use_signal(|| 0u64);
        let _ = render_generation();
        *probe.render_generation.borrow_mut() = Some(render_generation);
        let version = use_signal(|| 0u64);
        *probe.version.borrow_mut() = Some(version);
        let _ = version();
        let workspace = use_signal(|| "Layout");
        let scope_generation = use_signal(|| 1u64);
        let selected_context = use_signal(|| None::<objects::ScopedTreeContext>);
        *probe.selected.borrow_mut() = Some(selected_context);
        let anchor_scope = use_signal(|| None::<Scope>);
        let adapter =
            use_hook(|| SelectionAdapter::new(selected_context, anchor_scope, scope_generation));
        *probe.adapter.borrow_mut() = Some(adapter.clone());
        // The key actions mounted by the Inspector read the Editor's shared contexts.
        use_context_provider(|| probe.runtime.clone());
        use_context_provider(|| adapter.clone());
        use_context_provider(|| version);
        use_context_provider(|| WorkspaceState(workspace));
        let relation_highlight = use_signal(|| None::<Vec<String>>);
        use_context_provider(|| RelationHighlight(relation_highlight));
        let state = layout_workspace::use_layout_workspace_state(selected_context, workspace);
        let runtime = probe.runtime.clone();
        let lifetime = use_hook(|| Rc::new(inspector::LayoutComponentInspectorLifetime::default()));
        let model = runtime.model();
        let selected = selected_context();
        let owner_key =
            layout_component_inspector_owner_key(&model, workspace(), selected.as_ref());
        let generation = lifetime.update(owner_key);
        let component_inspector = layout_component_inspector_projection(
            &model,
            selected.as_ref(),
            generation,
            scope_generation(),
        );
        let matrix_inspector = objects::use_matrix_inspector(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
        );
        let board_inspector = board_inspector::use_board_inspector(
            runtime.clone(),
            selected_context,
            workspace,
            adapter.generation,
        );
        let key_size = objects::use_key_size(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
        );
        let transform = objects::use_workspace_matrix_transform(
            runtime.clone(),
            version,
            selected_context,
            workspace,
            adapter.generation,
            state.splay_affect(),
            "Layout",
        );
        let (x, y, margin, error) = state.component_inspector_drafts();
        let (layout_pending, constraint_pending, remove_pending, mirrored_half_pending) =
            state.component_inspector_pending_controls();
        let (companion_pending, companion_error, companion_edits) =
            state.companion_placement_controls();
        let (x_failure, y_failure, margin_failure) = state.component_inspector_field_failures();
        let select_runtime = runtime.clone();
        let select_adapter = adapter.clone();
        rsx! {
            {layout_workspace::inspector(layout_workspace::InspectorInput {
                geometry_scripts_open: false,
                on_close_geometry_scripts: EventHandler::new(|()| {}),
                context_title: None,
                context_detail: None,
                show_position_inspector: false,
                component_inspector,
                component_inspector_pending_edits: state.component_inspector_edits(),
                component_inspector_x: x,
                component_inspector_y: y,
                component_inspector_margin: margin,
                component_inspector_error: error,
                component_inspector_x_failure: x_failure,
                component_inspector_y_failure: y_failure,
                component_inspector_margin_failure: margin_failure,
                component_inspector_layout_pending: layout_pending,
                component_inspector_constraint_pending: constraint_pending,
                component_inspector_remove_pending: remove_pending,
                component_inspector_mirrored_half_pending: mirrored_half_pending,
                companion_placement: None,
                companion_placement_pending: companion_pending,
                companion_placement_error: companion_error,
                companion_placement_edits: companion_edits,
                on_companion_placement_action: EventHandler::new(|_| {}),
                on_component_inspector_action: EventHandler::new(|_| {}),
                matrix_inspector,
                key_size,
                matrix_transform_inspector: transform,
                on_pick_splay_origin: EventHandler::new(|()| {}),
                splay_origin_pick_pending: false,
                on_cancel_splay_origin_pick: EventHandler::new(|()| {}),
                on_pick_anchor: EventHandler::new(|()| {}),
                anchor_pick_pending: false,
                on_cancel_anchor_pick: EventHandler::new(|()| {}),
                matrix_relations: matrix_context_relations(&model, selected.as_ref()),
                relation_target: {
                    let runtime = runtime.clone();
                    Rc::new(move |request| relation_target_request(&runtime, request))
                },
                relation_scope: selected.as_ref().map(|selected| selected.scope.clone()),
                on_select_context: EventHandler::new(move |request: objects::TreeSelectRequest| {
                    selection::submit_context(&select_runtime, &select_adapter, request.into());
                }),
                on_navigate_anchor_part: EventHandler::new(|_| {}),
                outline_inspector: None,
                findings_return_available: false,
                on_findings_return: EventHandler::new(|()| {}),
                board_inspector_mount: board_inspector,
                findings_page: None,
            })}
            output { class: "m1-layout-component-test-generation", "{render_generation()}" }
        }
    }

    fn definition(id: &str, kind: &str) -> serde_json::Value {
        json!({
            "id": id, "name": id, "kind": kind,
            "courtyard": [
                { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
                { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
            ],
            "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                       "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
        })
    }

    fn base() -> ProjectDoc {
        let mut document =
            serde_json::to_value(ProjectDoc::empty("relations-doc", "Relations")).unwrap();
        document["definitions"] = json!([
            definition("switch", "switch"),
            definition("diode", "passive")
        ]);
        document["boards"] = json!([{ "id": "board", "name": "Board", "outlineIds": [],
            "partIds": [], "netIds": [], "thickness": 1.6 }]);
        serde_json::from_value(document).unwrap()
    }

    fn matrix_json(rows: u32, columns: u32, extra: serde_json::Value) -> serde_json::Value {
        let mut matrix = json!({
            "id": MATRIX, "rows": rows, "columns": columns,
            "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "switch", "boardId": "board", "partIds": []
        });
        for (key, value) in extra.as_object().into_iter().flatten() {
            matrix[key] = value.clone();
        }
        matrix
    }

    fn set_matrix_command(base_revision: u64, matrix: serde_json::Value) -> EditCommand {
        EditCommand {
            base_revision,
            transaction_id: "set-matrix".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: serde_json::from_value(json!({ "kind": "set-matrix", "matrix": matrix }))
                .expect("the operation parses"),
        }
    }

    /// Open `document`, then create the matrix through the Session and Core.
    async fn mount(
        root_id: &'static str,
        document: ProjectDoc,
        matrix: Option<serde_json::Value>,
    ) -> (RelationsProbe, web_sys::Element) {
        let support = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(
            &support,
            Session::new(),
            CoreEngine::new(),
        );
        crate::runtime::project_name_test_support::install_memory_persistence(&support);
        support.submit(Event::Open {
            operation_id: support.operation(),
            document,
        });
        crate::runtime::project_name_test_support::run_pending(&support).await;
        if let Some(matrix) = matrix {
            let revision = support.model().accepted.unwrap().document.revision;
            crate::runtime::project_name_test_support::submit_fixed_command(
                &support,
                set_matrix_command(revision, matrix),
            );
            crate::runtime::project_name_test_support::run_pending(&support).await;
        }
        let probe = RelationsProbe {
            runtime: support,
            root_id,
            render_generation: Rc::default(),
            version: Rc::default(),
            selected: Rc::default(),
            adapter: Rc::default(),
        };
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        root.set_id(root_id);
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(relations_host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        probe.settle().await;
        (probe, root)
    }

    fn matrix_ctx() -> objects::TreeContext {
        objects::TreeContext::Matrix {
            matrix_id: MATRIX.into(),
        }
    }
    fn column_ctx(column: u32) -> objects::TreeContext {
        objects::TreeContext::Column {
            matrix_id: MATRIX.into(),
            column,
        }
    }
    fn key_ctx(row: u32, column: u32) -> objects::TreeContext {
        objects::TreeContext::Key {
            matrix_id: MATRIX.into(),
            row,
            column,
        }
    }

    #[wasm_bindgen_test]
    async fn a_column_offset_departure_resets_with_undo_and_converts_to_a_stagger() {
        let (probe, root) = mount(
            "relations-departure-test",
            base(),
            Some(matrix_json(
                2,
                3,
                json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 2.0}}]
}),
            )),
        )
        .await;
        probe.select(column_ctx(1));
        probe.land().await;
        assert!(probe.exists("[data-relation-departure='Column offset']"));

        probe.click("[data-relation-action='reset']");
        probe.land().await;
        assert_eq!(
            probe.matrix().column_layout[1].offset,
            Vec2 { x: 0.0, y: 0.0 },
            "Reset lands through SetMatrixPlacement"
        );
        assert!(
            !probe.exists("[data-relation-departure='Column offset']"),
            "the departure disappears: {}",
            probe.text()
        );

        probe.runtime.submit(Event::Undo {
            operation_id: probe.runtime.operation(),
        });
        probe.land().await;
        assert_eq!(
            probe.matrix().column_layout[1].offset,
            Vec2 { x: 1.5, y: 2.0 }
        );
        assert!(
            probe.exists("[data-relation-departure='Column offset']"),
            "Undo restores the departure: {}",
            probe.text()
        );

        probe.click("[data-relation-action='convert']");
        probe.land().await;
        let matrix = probe.matrix();
        assert_eq!(matrix.column_layout[1].stagger, 2.0);
        assert_eq!(matrix.column_layout[1].offset, Vec2 { x: 1.5, y: 0.0 });
        assert!(
            probe.text().contains("2 mm from Column 1, cumulative 2 mm"),
            "Structure shows the new stagger: {}",
            probe.text()
        );
        root.remove();
    }

    /// A splayed column keeps its Column offset departure, but Convert is
    /// disabled with the Core-owned reason wired through `aria-describedby`.
    /// Rendering the refusal writes nothing.
    #[wasm_bindgen_test]
    async fn a_blocked_convert_in_relations_names_the_core_reason() {
        let (probe, root) = mount(
            "relations-blocked-convert-test",
            base(),
            Some(matrix_json(
                2,
                3,
                json!({ "columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 1.5, "y": 2.0}, "splay": 10.0}]
}),
            )),
        )
        .await;
        let before = probe
            .runtime
            .model()
            .accepted
            .expect("accepted document")
            .document
            .revision;
        probe.select(column_ctx(1));
        probe.land().await;
        assert!(
            probe.exists("[data-relation-departure='Column offset']"),
            "the departure is listed: {}",
            probe.text()
        );
        let button = root
            .query_selector("[data-relation-action='convert']")
            .unwrap()
            .expect("Convert is mounted");
        assert!(
            button.has_attribute("disabled"),
            "splay blocks Convert in Relations too"
        );
        let reason = root
            .query_selector(".m1-matrix-transform-unavailable")
            .unwrap()
            .expect("the Core reason is shown");
        assert_eq!(
            reason.text_content().as_deref(),
            Some("Converting requires zero Splay, Column rotation and carried Bend")
        );
        assert_eq!(
            button.get_attribute("aria-describedby").as_deref(),
            reason.get_attribute("id").as_deref(),
            "the refusal is programmatically named"
        );
        assert!(
            button
                .get_attribute("aria-describedby")
                .as_deref()
                .is_some_and(|id| !id.is_empty()),
            "the description reference is not empty"
        );
        assert_eq!(
            probe
                .runtime
                .model()
                .accepted
                .expect("accepted document")
                .document
                .revision,
            before,
            "rendering a refusal writes nothing, so no Undo entry"
        );
        root.remove();
    }

    /// The `tie` offset constraint wins over its companion's attachment to its
    /// key: Core derives the Constrained mode, and Relations names that
    /// precedence in text.
    #[wasm_bindgen_test]
    async fn a_won_constraint_names_its_priority_over_the_key_attachment() {
        let (document, _) = constrained_companion_document();
        let (probe, root) = mount("relations-precedence-test", document, None).await;
        probe.select(matrix_ctx());
        probe.land().await;
        let row = root
            .query_selector("[data-relation-constraint='tie']")
            .unwrap()
            .expect("the constraint is listed");
        assert!(
            row.text_content()
                .unwrap_or_default()
                .contains("Takes priority over the attachment to its key."),
            "Core-owned precedence is explained: {}",
            probe.text()
        );
        root.remove();
    }

    /// Two keys' companions, one tied to the first key by an explicit constraint. Core
    /// generates the part ids, so a scratch engine does the work and the document that
    /// results opens in the Session.
    fn constrained_companion_document() -> (ProjectDoc, String) {
        let mut engine = CoreEngine::new();
        let opened = match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: base(),
        }) {
            CoreReply::Scene { scene, .. } => scene,
            other => panic!("{other:?}"),
        };
        let assemblies: Vec<serde_json::Value> = ["a", "b"]
            .iter()
            .enumerate()
            .map(|(index, id)| {
                json!({ "id": id, "definitionId": "diode",
                        "offset": { "x": 0.0, "y": 8.0 + index as f64 * 3.0 } })
            })
            .collect();
        let matrix = matrix_json(
            1,
            2,
            json!({ "cells": [{ "row": 0, "column": 0, "enabled": true, "assemblies": assemblies }] }),
        );
        let (scene, mut document) = match engine.handle(CoreRequest::Edit {
            id: "edit".into(),
            command: set_matrix_command(opened.revision, matrix),
        }) {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            other => panic!("{other:?}"),
        };
        let companion = scene.matrix_scenes[0].companions[1].part_id.clone();
        document
            .constraints
            .push(boardstudio_core::model::Constraint::Offset {
                id: "tie".into(),
                source_part_id: "matrix/main/r0c0".into(),
                target_part_id: companion.clone(),
                offset: Vec2 { x: 0.0, y: 12.0 },
                rotation: 0.0,
            });
        (document, companion)
    }

    #[wasm_bindgen_test]
    async fn a_constraint_reference_selects_its_object_and_escape_returns() {
        let (document, companion) = constrained_companion_document();
        let (probe, root) = mount("relations-constraint-test", document, None).await;
        probe.select(matrix_ctx());
        probe.land().await;
        assert!(
            probe.exists("[data-relation-constraint='tie']"),
            "the constraint is listed: {}",
            probe.text()
        );

        probe.click("[data-relation-constraint='tie'] [data-relation-role='source']");
        probe.land().await;
        assert_eq!(probe.selected_context(), Some(key_ctx(0, 0)));

        probe.select(matrix_ctx());
        probe.land().await;
        probe.click("[data-relation-constraint='tie'] [data-relation-role='target']");
        probe.land().await;
        assert!(
            matches!(
                probe.selected_context(),
                Some(objects::TreeContext::Component { part_id: Some(ref id), .. }) if *id == companion
            ),
            "the companion is selected: {:?}",
            probe.selected_context()
        );

        let options = web_sys::KeyboardEventInit::new();
        options.set_key("Escape");
        options.set_bubbles(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options).unwrap();
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .dispatch_event(&event)
            .unwrap();
        probe.land().await;
        assert!(
            matches!(
                probe.selected_context(),
                Some(objects::TreeContext::Board { .. })
            ),
            "Escape returns: {:?} text={}",
            probe.selected_context(),
            probe.text()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn a_reference_to_a_removed_key_disappears_and_a_queued_click_does_nothing() {
        let (document, _) = constrained_companion_document();
        let (probe, root) = mount("relations-removed-test", document, None).await;
        probe.select(matrix_ctx());
        probe.land().await;
        let source = "[data-relation-constraint='tie'] [data-relation-role='source']";
        assert!(probe.exists(source), "{}", probe.text());

        // Remove the referenced key, letting the edit land without re-rendering the panel:
        // the button is still in the DOM, as when a click was queued before the removal.
        let accepted = probe.runtime.model().accepted.unwrap();
        crate::runtime::project_name_test_support::submit_fixed_command(
            &probe.runtime,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "remove-key".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation: boardstudio_core::model::EditOperation::RemoveMatrixCells {
                    matrix_id: MATRIX.into(),
                    cells: vec![(0, 0)],
                },
            },
        );
        crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
        let landed = probe.runtime.model().accepted.unwrap();
        assert!(
            landed.document.revision > accepted.document.revision,
            "the removal landed"
        );
        let before = probe.selected_context();
        probe.click(source);
        crate::runtime::project_name_test_support::run_pending(&probe.runtime).await;
        assert_eq!(
            probe.selected_context(),
            before,
            "the stale click did nothing"
        );

        probe.bump();
        probe.settle().await;
        assert!(
            !probe.exists(source),
            "the removed key's row is gone: {}",
            probe.text()
        );
        root.remove();
    }
}

fn companion_click(root: &web_sys::Element, selector: &str) {
    root.query_selector(selector)
        .unwrap()
        .expect(selector)
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
}

fn companion_placement_component_host() -> Element {
    let projection = use_context::<inspector::CompanionPlacementProjection>();
    let actions = use_context::<Rc<RefCell<Option<inspector::CompanionPlacementAction>>>>();
    let pending = use_signal(|| false);
    let error = use_signal(|| Some("Core refused this edit.".to_owned()));
    let pending_edits = use_signal(layout_component_edits::LayoutComponentInspectorEdits::default);
    rsx! {
        inspector::CompanionPlacementInspector {
            projection,
            pending,
            error,
            pending_edits,
            on_apply: EventHandler::new(move |action| *actions.borrow_mut() = Some(action)),
        }
    }
}

fn companion_placement_runtime_host() -> Element {
    let projection = use_context::<inspector::CompanionPlacementProjection>();
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let refuse_as_invalid_pair_tweak = use_context::<Rc<std::cell::Cell<bool>>>();
    let generation = use_signal(|| 0u64);
    let _ = generation();
    use_hook(|| {
        runtime.subscribe(Rc::new(move || {
            let mut generation = generation;
            generation += 1;
        }));
    });
    let pending = use_signal(|| false);
    let error = use_signal(|| None::<String>);
    let pending_edits = use_signal(layout_component_edits::LayoutComponentInspectorEdits::default);
    let action_runtime = runtime.clone();
    let action_edits = pending_edits;
    rsx! {
        inspector::CompanionPlacementInspector {
            projection,
            pending,
            error,
            pending_edits,
            on_apply: EventHandler::new(move |mut action: inspector::CompanionPlacementAction| {
                if refuse_as_invalid_pair_tweak.get() {
                    action.targets.push(boardstudio_core::model::CompanionPlacementTarget::Pair);
                    action.tweak = Some(boardstudio_core::model::MirroredCompanionTweak {
                        x: 0.5,
                        y: 0.25,
                        rotation: 2.0,
                    });
                }
                submit_layout_component_edit(
                    &action_runtime,
                    action_edits,
                    layout_component_edits::InspectorField::CompanionPlacement,
                    layout_component_edits::companion_placement_resolver(
                        action.pair_id,
                        action.targets,
                        action.mode,
                        action.tweak,
                    ),
                    None,
                );
            }),
        }
    }
}

fn companion_placement_component_projection(
    disabled_reason: Option<&'static str>,
) -> inspector::CompanionPlacementProjection {
    let scope = Scope {
        session_epoch: SessionEpoch(5),
        document_id: "companion-placement-doc".into(),
        board_id: "board".into(),
        instance_id: None,
    };
    inspector::CompanionPlacementProjection {
        owner: inspector::CompanionPlacementOwner {
            scope: scope.clone(),
            context: objects::ScopedTreeContext {
                scope,
                context: objects::TreeContext::Component {
                    part_id: Some("matrix/source/r0c0/led".into()),
                    matrix_id: Some("source".into()),
                    row: Some(0),
                    column: Some(0),
                    assembly_id: Some("led".into()),
                },
            },
            selected_part_ids: vec!["matrix/source/r0c0/led".into()],
            snapshot_token: SnapshotToken(13),
            revision: 9,
            context_generation: 7,
            scope_generation: 7,
        },
        pair_id: "pair/A".into(),
        targets: if disabled_reason.is_some() {
            vec![]
        } else {
            vec![boardstudio_core::model::CompanionPlacementTarget::Item {
                item_kind: boardstudio_core::model::MirroredItemKind::Part,
                mirror_id: "matrix/mirror/r0c0/led".into(),
            }]
        },
        mode: Some(boardstudio_core::model::MirroredCompanionMode::LiteralMirror),
        mixed_mode: false,
        tweak: None,
        mixed_tweak: false,
        disabled_reason,
    }
}

#[wasm_bindgen_test]
async fn companion_placement_mounted_edit_undo_and_core_refusal() {
    use boardstudio_core::model::{CompanionPlacementTarget, MirroredCompanionMode};
    use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_step};
    use wasm_bindgen::JsCast;

    let runtime = crate::runtime::project_name_test_support::new_runtime();
    crate::runtime::project_name_test_support::install(
        &runtime,
        boardstudio_application::Session::new(),
        boardstudio_core::CoreEngine::new(),
    );
    crate::runtime::project_name_test_support::install_memory_persistence(&runtime);
    runtime.submit(boardstudio_application::Event::Open {
        operation_id: runtime.operation(),
        document: runtime_document(configure_mirrored_part_pair),
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;
    runtime.submit(boardstudio_application::Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["selected-part".into()],
        range_part_ids: Vec::new(),
        mode: boardstudio_application::SelectionMode::Replace,
    });
    crate::runtime::project_name_test_support::run_pending(&runtime).await;

    let model = runtime.model();
    let snapshot = model.accepted.as_ref().expect("fixture opened");
    let scope = runtime.scope().expect("fixture scope");
    let mut projection = companion_placement_component_projection(None);
    projection.owner.scope = scope.clone();
    projection.owner.context = objects::ScopedTreeContext {
        scope,
        context: objects::context_for_part(&model, "selected-part").expect("selected part context"),
    };
    projection.owner.selected_part_ids = model.selected_part_ids.clone();
    projection.owner.snapshot_token = snapshot.token;
    projection.owner.revision = snapshot.document.revision;
    projection.targets = vec![CompanionPlacementTarget::Item {
        item_kind: boardstudio_core::model::MirroredItemKind::Part,
        mirror_id: "source-part".into(),
    }];
    let reject_pair_tweak = Rc::new(std::cell::Cell::new(false));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(companion_placement_runtime_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(projection);
    dom.provide_root_context(reject_pair_tweak.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the live companion placement control mounts", || {
        root.query_selector("details.m1-layout-companion-placement summary")
            .unwrap()
            .is_some()
    })
    .await;
    companion_click(&root, "details.m1-layout-companion-placement summary");

    let choose_mode = |value: &str| {
        let select = root
            .query_selector("select")
            .unwrap()
            .expect("mode selector")
            .dyn_into::<web_sys::HtmlSelectElement>()
            .unwrap();
        select.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        select
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &event).unwrap())
            .unwrap();
    };
    let pair_mode = |runtime: &Rc<crate::runtime::Runtime>| {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .hardware
            .as_ref()
            .unwrap()
            .pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .companion_mode
    };

    let before = runtime.model().accepted.unwrap().document.revision;
    choose_mode("duplicate-position");
    companion_click(&root, ".m1-layout-companion-placement-default input");
    companion_click(&root, "button");
    wait_until_step("Core accepts the companion default edit", async || {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        pair_mode(&runtime) == MirroredCompanionMode::DuplicatePosition
            && runtime.model().accepted.unwrap().document.revision > before
    })
    .await;
    let accepted_revision = runtime.model().accepted.unwrap().document.revision;
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step("Undo restores the companion default", async || {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        pair_mode(&runtime) == MirroredCompanionMode::LiteralMirror
            && runtime.model().accepted.unwrap().document.revision > accepted_revision
    })
    .await;

    companion_click(&root, ".m1-layout-companion-placement-default input");
    choose_mode("literal-mirror-with-tweak");
    wait_until("the tweak fields mount", || {
        root.query_selector_all("input[type='number']")
            .unwrap()
            .length()
            == 3
    })
    .await;
    let tweak_inputs = root.query_selector_all("input[type='number']").unwrap();
    for (index, value) in ["0.5", "0.25", "2"].into_iter().enumerate() {
        let input = tweak_inputs
            .item(index as u32)
            .expect("tweak input")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    }
    reject_pair_tweak.set(true);
    companion_click(&root, "button");
    wait_until_step("Core refusal reaches the Inspector", async || {
        crate::runtime::project_name_test_support::run_pending(&runtime).await;
        root.query_selector("[role='alert']")
            .unwrap()
            .is_some_and(|alert| {
                alert
                    .text_content()
                    .unwrap_or_default()
                    .contains("cannot carry a companion tweak")
            })
    })
    .await;
    assert_eq!(pair_mode(&runtime), MirroredCompanionMode::LiteralMirror);
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        accepted_revision + 1,
        "the refused request leaves accepted state unchanged after Undo"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn companion_placement_shows_mixed_modes_and_tweaks() {
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;

    let mut projection = companion_placement_component_projection(None);
    projection.mode = None;
    projection.mixed_mode = true;
    projection.tweak = None;
    projection.mixed_tweak = true;
    let actions = Rc::new(RefCell::new(None::<inspector::CompanionPlacementAction>));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(companion_placement_component_host);
    dom.provide_root_context(projection);
    dom.provide_root_context(actions);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the mixed companion placement section mounts", || {
        root.query_selector("details.m1-layout-companion-placement summary")
            .unwrap()
            .is_some()
    })
    .await;
    let summary = root
        .query_selector("details.m1-layout-companion-placement summary small")
        .unwrap()
        .unwrap();
    assert_eq!(summary.text_content().as_deref(), Some("Mixed"));
    companion_click(&root, "details.m1-layout-companion-placement summary");
    let mode = root
        .query_selector("select")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap();
    assert_eq!(mode.value(), "");
    mode.set_value("literal-mirror-with-tweak");
    let change = web_sys::EventInit::new();
    change.set_bubbles(true);
    mode.dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &change).unwrap())
        .unwrap();
    wait_until("mixed tweak fields mount", || {
        root.query_selector_all("input[type='number']")
            .unwrap()
            .length()
            == 3
    })
    .await;
    let inputs = root.query_selector_all("input[type='number']").unwrap();
    for index in 0..inputs.length() {
        assert_eq!(
            inputs
                .item(index)
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .placeholder(),
            "Mixed values"
        );
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn stabiliser_companion_placement_shows_a_disabled_mode_and_core_reason() {
    let projection = companion_placement_component_projection(Some(
        "Stabilisers keep their mirrored-X position and unchanged heading in every mode.",
    ));
    let actions = Rc::new(RefCell::new(None::<inspector::CompanionPlacementAction>));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(companion_placement_component_host);
    dom.provide_root_context(projection);
    dom.provide_root_context(actions);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    use boardstudio_web_ui_shared::test_wait::wait_until;
    wait_until("the stabiliser placement reason mounts", || {
        root.query_selector("select").unwrap().is_some()
    })
    .await;
    let mode = root
        .query_selector("select")
        .unwrap()
        .expect("disabled mode control");
    assert!(mode.has_attribute("disabled"));
    assert!(
        root.text_content()
            .unwrap()
            .contains("Stabilisers keep their mirrored-X position")
    );
    assert_eq!(root.query_selector_all("button").unwrap().length(), 0);
    root.remove();
}
