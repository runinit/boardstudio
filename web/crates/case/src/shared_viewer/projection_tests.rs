use super::*;
use boardstudio_application::SessionEpoch;

fn identity() -> ViewerIdentity {
    ViewerIdentity {
        scope: Scope {
            session_epoch: SessionEpoch(3),
            document_id: "project-1".to_owned(),
            board_id: "board-1".to_owned(),
            instance_id: Some("case-instance-1".to_owned()),
        },
        snapshot_token: SnapshotToken(8),
        revision: 9,
        viewer_instance: 2,
        projection_generation: 4,
        renderer_sequence: 6,
    }
}

fn imported_preview_fixture(
    reference: boardstudio_model::model::BoardReference,
) -> (boardstudio_application::AcceptedSnapshot, ViewerIdentity) {
    use boardstudio_model::model::{
        Asset, Board, BoardContours, ProjectDoc, Readiness, SceneDelta,
    };
    use std::sync::Arc;

    let viewer = identity();
    let mut document = ProjectDoc::empty("project-1", "Routed board");
    document.revision = viewer.revision;
    document.boards.push(Board {
        id: "board-1".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.assets.push(Asset {
        id: "routed-source".into(),
        name: "board.kicad_pcb".into(),
        media_type: "application/vnd.kicad.pcb".into(),
        sha256: "ab".repeat(32),
        license: None,
        source: None,
    });
    document.board_references.push(reference);
    document.hardware = Some(boardstudio_model::model::HardwareConfiguration {
        instances: vec![boardstudio_model::model::PhysicalBoardInstance {
            id: "case-instance-1".into(),
            name: "Case instance".into(),
            board_id: "board-1".into(),
            half: "left".into(),
            role: "central".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: None,
            construction_ownership:
                boardstudio_model::model::MechanicalConstructionOwnership::Local,
            mechanical_identity_reservation: None,
        }],
        ..Default::default()
    });
    let accepted = boardstudio_application::AcceptedSnapshot {
        token: viewer.snapshot_token,
        session_epoch: viewer.scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: viewer.revision,
            transaction_id: "test".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "board-1".into(),
                contours: vec![],
            }],
            board_readiness: vec![],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: false,
                outline: false,
                pcb: false,
                case_ready: false,
            },
        }),
    };
    (accepted, viewer)
}

fn imported_layout_projection(
    reference: boardstudio_model::model::BoardReference,
    model_rows: Option<ModelDeliveryRows>,
) -> RendererSceneProjection {
    use boardstudio_model::model::PcbPreview;
    use std::collections::BTreeMap;
    let (accepted, viewer) = imported_preview_fixture(reference);
    let capture = crate::layout_viewer_source::LayoutSourceCapture::capture(
        &accepted,
        &viewer.scope,
        1,
        "layout-preview-test".into(),
        BTreeMap::new(),
    )
    .unwrap();
    let preview = capture
        .accept_preview(PcbPreview {
            revision: viewer.revision,
            thickness: 1.6,
            contours: vec![],
            surfaces: vec![],
            holes: vec![],
            models: vec![],
            diagnostics: vec![],
        })
        .unwrap();
    project_source(
        &ViewerSource::Layout(Rc::new(preview)),
        viewer,
        "light",
        None,
        model_rows.as_ref(),
        None,
    )
    .unwrap()
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn imported_layout_projection_preserves_nonidentity_board_reference_pose_and_elevation() {
    use boardstudio_model::model::{BoardReference, Pose2, Vec2};
    use std::collections::BTreeMap;

    let projection = imported_layout_projection(
        BoardReference {
            id: "routed-board-1".into(),
            board_id: "board-1".into(),
            asset_id: "routed-source".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 17.25, y: -8.5 },
                rotation: 31.0,
            },
            elevation: 4.75,
            model_assets: BTreeMap::new(),
        },
        None,
    );
    let packet = packet(&projection);

    assert_eq!(
        packet["reference"],
        serde_json::json!({
            "pose": {
                "at": { "x": 17.25, "y": -8.5 },
                "rotation": 31
            },
            "elevation": 4.75
        })
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn imported_case_projection_keeps_routed_contours_and_pose_in_pcb_and_case_scene_packets() {
    use boardstudio_model::model::{
        BoardReference, Contour, PcbPreview, Pose2, PreparedCaseAssemblyIR, Vec2,
    };
    let reference = BoardReference {
        id: "routed-case".into(),
        board_id: "board-1".into(),
        asset_id: "routed-source".into(),
        enabled: true,
        pose: Pose2 {
            at: Vec2 { x: 4.2, y: -3.1 },
            rotation: 15.0,
        },
        elevation: 2.5,
        model_assets: Default::default(),
    };
    let (accepted, viewer) = imported_preview_fixture(reference.clone());
    let capture = crate::case_preview::capture_native_preview(
        &accepted,
        &viewer.scope,
        1,
        1,
        "case-routed-projection".into(),
    )
    .unwrap();
    let contours = vec![Contour {
        points: vec![
            Vec2 { x: 10.0, y: 20.0 },
            Vec2 { x: 30.0, y: 20.0 },
            Vec2 { x: 30.0, y: 40.0 },
        ],
        hole: false,
    }];
    let preview = crate::case_preview::accept_native_preview(
        capture,
        PcbPreview {
            revision: viewer.revision,
            thickness: 1.6,
            contours: contours.clone(),
            surfaces: vec![],
            holes: vec![],
            models: vec![],
            diagnostics: vec![],
        },
    )
    .unwrap();
    let scene = Rc::new(CadScene {
        scope: viewer.scope.clone(),
        token: accepted.token,
        snapshot: accepted,
        result: boardstudio_web_host::cad_jobs::CadResult {
            revision: viewer.revision,
            ..Default::default()
        },
        prepared: PreparedCaseAssemblyIR {
            revision: viewer.revision,
            bodies: vec![],
        },
        physical_fingerprint: None,
        mechanical: None,
        exact: true,
        contours: vec![],
    });
    let preview = Rc::new(preview);
    for source in [
        ViewerSource::Native(preview.clone()),
        ViewerSource::Cad(scene),
    ] {
        let projection =
            project_source(&source, viewer.clone(), "light", Some(&preview), None, None).unwrap();
        let packet = packet(&projection);
        let projected_pose: Pose2 =
            serde_json::from_value(packet["reference"]["pose"].clone()).unwrap();
        assert_eq!(projected_pose, reference.pose);
        assert_eq!(
            packet["reference"]["elevation"].as_f64(),
            Some(reference.elevation)
        );
        let projected_contours: Vec<Contour> =
            serde_json::from_value(packet["board"]["contours"].clone()).unwrap();
        assert_eq!(projected_contours, contours);
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn layout_packet_includes_module_model_mesh_at_core_stable_id_and_transform() {
    use crate::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
    use boardstudio_model::model::{BoardReference, Pose2, Vec2};
    use std::collections::BTreeMap;

    let placement_id = "module-model/placement-a/0".to_owned();
    let matrix = [
        1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 12.0, 13.0, 14.0, 1.0,
    ];
    let rows = ModelDeliveryRows {
        delivered: vec![DeliveredModel {
            id: placement_id.clone(),
            mesh: Rc::new(ValidatedMesh {
                positions: Rc::from([1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
                normals: Rc::from([1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                colors: None,
                step_bounds: None,
                vendor_step: None,
            }),
            matrix: Some(matrix),
        }],
        pending: Vec::new(),
        failures: Vec::new(),
    };
    let projection = imported_layout_projection(
        BoardReference {
            id: "routed-board-1".into(),
            board_id: "board-1".into(),
            asset_id: "routed-source".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: BTreeMap::new(),
        },
        Some(rows),
    );
    let packet = packet(&projection);

    assert_eq!(packet["bodies"][0]["id"], placement_id);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][0], 13.0);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][1], 13.0);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][2], 14.0);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn vendor_step_sibling_lines_attach_to_pcb_and_module_models_with_single_row_transform() {
    use crate::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
    use boardstudio_model::model::{BoardReference, Pose2, Vec2};
    use boardstudio_web_host::cad_jobs::CadVendorStep;
    use std::collections::BTreeMap;

    let vendor_step = Rc::new(CadVendorStep {
        line_render_positions: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        ..CadVendorStep::default()
    });
    let mesh = Rc::new(ValidatedMesh {
        positions: Rc::from([1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        normals: Rc::from([0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0]),
        colors: None,
        step_bounds: None,
        vendor_step: Some(vendor_step),
    });
    let matrix = [
        2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 10.0, 20.0, 30.0, 1.0,
    ];
    let rows = ModelDeliveryRows {
        delivered: vec![
            DeliveredModel {
                id: "pcb-step-row".into(),
                mesh: mesh.clone(),
                matrix: None,
            },
            DeliveredModel {
                id: "module-model/placement-a/0".into(),
                mesh,
                matrix: Some(matrix),
            },
        ],
        pending: Vec::new(),
        failures: Vec::new(),
    };
    let loaded = loaded_model_inputs(Some(&rows));
    assert_eq!(loaded.length(), 1);
    let pcb_item = loaded.get(0);
    let pcb_lines =
        js_sys::Float32Array::from(Reflect::get(&pcb_item, &"lines".into()).unwrap()).to_vec();
    assert_eq!(pcb_lines, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

    let projection = imported_layout_projection(
        BoardReference {
            id: "routed-board-1".into(),
            board_id: "board-1".into(),
            asset_id: "routed-source".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: BTreeMap::new(),
        },
        Some(rows),
    );
    let packet = packet(&projection);
    let lines: Vec<f32> = serde_json::from_value(packet["bodies"][0]["lines"].clone()).unwrap();
    assert_eq!(lines, [12.0, 26.0, 42.0, 18.0, 35.0, 54.0]);
}

fn packet(projection: &RendererSceneProjection) -> serde_json::Value {
    let serialized = js_sys::JSON::stringify(&projection.input)
        .unwrap()
        .as_string()
        .unwrap();
    serde_json::from_str(&serialized).unwrap()
}
