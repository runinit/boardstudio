use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_web_ui_shared::test_mount::MountedFixture;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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

#[wasm_bindgen_test::wasm_bindgen_test]
fn generic_layout_module_projection_preserves_board_holes_and_omits_opening_voids() {
    use boardstudio_model::model::{
        CaseOpening, ModuleAttachment, ModuleSupportGeometry, MountedModule, ProjectDoc,
        ResolvedModule, Side, Vec2,
    };

    let mut document = ProjectDoc::empty("project-1", "Generic module");
    document.modules.push(MountedModule {
        id: "placement-generic".into(),
        definition_id: "definition-generic".into(),
        host_board_id: "board-1".into(),
        host_instance_id: None,
        host_face: Side::Front,
        facing_face: Side::Front,
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        gap: 0.0,
        attachment: ModuleAttachment::Board,
        detached: false,
        connection: None,
        service_clearance: 0.0,
        mount_supports: vec![],
    });
    let polygon = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 6.0, y: 0.0 },
        Vec2 { x: 6.0, y: 2.0 },
        Vec2 { x: 2.0, y: 2.0 },
        Vec2 { x: 2.0, y: 6.0 },
        Vec2 { x: 0.0, y: 6.0 },
    ];
    let module = ResolvedModule {
        id: "placement-generic".into(),
        definition_id: "definition-generic".into(),
        at: Vec2 { x: 0.0, y: 0.0 },
        rotation: 0.0,
        midplane_z: 1.6,
        flipped: false,
        board: vec![CaseOpening {
            points: polygon.clone(),
            z: 0.0,
            height: 1.2,
        }],
        pcb_source: None,
        round_drills: vec![],
        board_holes: vec![boardstudio_model::model::Contour {
            points: vec![
                Vec2 { x: 0.5, y: 0.5 },
                Vec2 { x: 0.5, y: 1.5 },
                Vec2 { x: 1.5, y: 1.5 },
                Vec2 { x: 1.5, y: 0.5 },
            ],
            hole: true,
        }],
        volumes: vec![boardstudio_model::model::ModuleVolume {
            id: "volume-1".into(),
            geometry: CaseOpening {
                points: polygon,
                z: 1.2,
                height: 0.4,
            },
            purpose: "assembly".into(),
            source: "fixture".into(),
            qualified: true,
        }],
        openings: vec![boardstudio_model::model::ModuleVolume {
            id: "clearance-void".into(),
            geometry: CaseOpening {
                points: vec![
                    Vec2 { x: 2.5, y: 2.5 },
                    Vec2 { x: 3.5, y: 2.5 },
                    Vec2 { x: 3.5, y: 3.5 },
                ],
                z: 1.0,
                height: 3.0,
            },
            purpose: "connector-clearance".into(),
            source: "fixture".into(),
            qualified: true,
        }],
        mounts: vec![],
        mount_supports: vec![ModuleSupportGeometry {
            mount_id: "support-1".into(),
            at: Vec2 { x: 4.0, y: 1.0 },
            outer_diameter: 2.0,
            hole_diameter: 0.8,
            z: 0.0,
            height: 1.0,
        }],
        footprints: vec![],
        models: vec![],
        gates: vec![],
    };

    let bodies = layout_module_bodies(&[module], &document, "board-1", 1.6);
    assert_eq!(bodies.len(), 3, "PCB, assembly volume and standoff only");
    assert_eq!(bodies[0]["id"], "module:placement-generic:pcb:0");
    assert_eq!(bodies[0]["contours"].as_array().unwrap().len(), 2);
    assert_eq!(bodies[0]["contours"][1]["hole"], true);
    assert_eq!(
        bodies[0]["contours"][0]["points"].as_array().unwrap().len(),
        6
    );
    assert_eq!(bodies[1]["id"], "module:placement-generic:volume:0");
    assert_eq!(
        bodies[2]["id"],
        "module:placement-generic:standoff:support-1"
    );
    assert_eq!(bodies[2]["contours"][1]["hole"], true);
    assert!(
        bodies
            .iter()
            .all(|body| !body["id"].as_str().unwrap().contains("clearance-void"))
    );
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
    let capture = super::super::layout_viewer_source::LayoutSourceCapture::capture(
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
    project_layout_preview(&preview, viewer, "light", model_rows.as_ref(), None).unwrap()
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
    let serialized = js_sys::JSON::stringify(&projection.input)
        .unwrap()
        .as_string()
        .unwrap();
    let packet: serde_json::Value = serde_json::from_str(&serialized).unwrap();

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
    for projection in [
        project_native_preview(&preview, viewer.clone(), "light", None, None).unwrap(),
        project_case_scene(scene, viewer, "light", Some(&preview), None, None).unwrap(),
    ] {
        let encoded = js_sys::JSON::stringify(&projection.input)
            .unwrap()
            .as_string()
            .unwrap();
        let packet: serde_json::Value = serde_json::from_str(&encoded).unwrap();
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
    use super::super::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
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
    let serialized = js_sys::JSON::stringify(&projection.input)
        .unwrap()
        .as_string()
        .unwrap();
    let packet: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    assert_eq!(packet["bodies"][0]["id"], placement_id);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][0], 13.0);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][1], 13.0);
    assert_eq!(packet["bodies"][0]["mesh"]["positions"][2], 14.0);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn vendor_step_sibling_lines_attach_to_pcb_and_module_models_with_single_row_transform() {
    use super::super::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
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
    let serialized = js_sys::JSON::stringify(&projection.input)
        .unwrap()
        .as_string()
        .unwrap();
    let packet: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    let lines: Vec<f32> = serde_json::from_value(packet["bodies"][0]["lines"].clone()).unwrap();
    assert_eq!(lines, [12.0, 26.0, 42.0, 18.0, 35.0, 54.0]);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn layout_module_model_normals_follow_nonuniform_core_scale() {
    let diagonal = std::f32::consts::FRAC_1_SQRT_2;
    let matrix = [
        2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let (_, normals) =
        transformed_module_mesh(&[0.0, 0.0, 0.0], &[diagonal, diagonal, 0.0], &matrix);
    assert!((normals[0] - 1.0 / 5.0_f32.sqrt()).abs() < 1e-5);
    assert!((normals[1] - 2.0 / 5.0_f32.sqrt()).abs() < 1e-5);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn finding_focus_retriggers_repeats_and_rejects_stale_project_board_token_and_revision() {
    let identity = identity();
    let request = ViewerFocusRequest {
        scope: identity.scope.clone(),
        snapshot_token: identity.snapshot_token,
        revision: identity.revision,
        navigation_id: 1,
        target_ids: vec!["plate".into()],
    };
    assert!(focus_request_matches(&request, &identity));

    let focused = super::super::keycaps_finding_marker::FocusedFinding {
        scope: identity.scope.clone(),
        token: identity.snapshot_token,
        revision: identity.revision,
        finding_id: "finding-1".into(),
        navigation_id: request.navigation_id,
    };
    let mut repeated = request.clone();
    repeated.navigation_id =
        super::super::keycaps_finding_marker::next_navigation_id(Some(&focused));
    assert_ne!(repeated, request);
    assert!(focus_request_matches(&repeated, &identity));

    let mut stale = request.clone();
    stale.scope.document_id.push_str("-other");
    assert!(!focus_request_matches(&stale, &identity));
    let mut stale = request.clone();
    stale.scope.board_id.push_str("-other");
    assert!(!focus_request_matches(&stale, &identity));
    let mut stale = request.clone();
    stale.snapshot_token = SnapshotToken(identity.snapshot_token.0 + 1);
    assert!(!focus_request_matches(&stale, &identity));
    let mut stale = request;
    stale.revision += 1;
    assert!(!focus_request_matches(&stale, &identity));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn physical_preview_rows_are_qualified_by_source_scope_and_token() {
    let viewer = identity();
    let owner = crate::case_preview::CasePreviewOwnerIdentity {
        scope: viewer.scope.clone(),
        snapshot_token: viewer.snapshot_token,
        accepted_revision: 11,
        accepted_scene_identity: 12,
        viewer_instance: 13,
        projection_generation: 14,
        batch_generation: 15,
        request_token: "case-preview-1".to_owned(),
    };

    assert!(preview_matches_source(
        &owner,
        &viewer.scope,
        viewer.snapshot_token
    ));
    assert!(!preview_matches_source(
        &owner,
        &viewer.scope,
        SnapshotToken(viewer.snapshot_token.0 + 1)
    ));
    let mut other_scope = viewer.scope.clone();
    other_scope.board_id = "board-2".to_owned();
    assert!(!preview_matches_source(
        &owner,
        &other_scope,
        viewer.snapshot_token
    ));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn viewer_events_reject_changed_owner_identity_and_unmount() {
    let identity = identity();
    let owner = Rc::new(ViewerOwner {
        active: Cell::new(true),
        identity: RefCell::new(Some(identity.clone())),
        last_inputs: RefCell::new(None),
        projection_generation: Cell::new(identity.projection_generation),
        renderer_sequence: Cell::new(identity.renderer_sequence),
        viewer_instance: identity.viewer_instance,
    });
    let signal = ScopedViewerSignal {
        identity: identity.clone(),
        kind: ViewerSignalKind::Picked("body-1".to_owned()),
        owner: Rc::downgrade(&owner),
    };
    assert!(signal.is_current());

    let mut changed = identity.clone();
    changed.renderer_sequence += 1;
    *owner.identity.borrow_mut() = Some(changed);
    assert!(!signal.is_current());

    *owner.identity.borrow_mut() = Some(identity);
    assert!(signal.is_current());
    owner.active.set(false);
    assert!(!signal.is_current());
}

fn gesture_scene(identity: &ViewerIdentity) -> Rc<CadScene> {
    use boardstudio_model::model::{PreparedCaseAssemblyIR, ProjectDoc, Readiness, SceneDelta};
    use std::sync::Arc;
    let mut document = ProjectDoc::empty(&identity.scope.document_id, "Gesture source");
    document.revision = identity.revision;
    Rc::new(CadScene {
        scope: identity.scope.clone(),
        token: identity.snapshot_token,
        snapshot: boardstudio_application::AcceptedSnapshot {
            token: identity.snapshot_token,
            session_epoch: identity.scope.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: identity.revision,
                transaction_id: String::new(),
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
                    case_ready: true,
                },
            }),
        },
        result: boardstudio_web_host::cad_jobs::CadResult::default(),
        prepared: PreparedCaseAssemblyIR {
            revision: identity.revision,
            bodies: vec![],
        },
        physical_fingerprint: None,
        mechanical: None,
        exact: true,
        contours: vec![],
    })
}

fn start_gesture(
    owner: &ViewerOwner,
    accepted: &Rc<CadScene>,
) -> (ProjectionInputs, ViewerIdentity) {
    let inputs = ProjectionInputs {
        source: Rc::as_ptr(accepted) as usize,
        preview: 0,
        models: vec![],
        theme: "light".into(),
    };
    let started = ViewerSource::Cad(accepted.clone())
        .identity(owner, &inputs)
        .unwrap();
    (inputs, started)
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn case_handle_survives_published_provisional_scene_until_pointer_up() {
    let source_identity = identity();
    let accepted = gesture_scene(&source_identity);
    let owner = ViewerOwner::new().unwrap();
    let (mut inputs, started) = start_gesture(&owner, &accepted);
    let pointer = Rc::new(RefCell::new(Some(PointerOwner::Handle {
        identity: started.clone(),
        pointer_id: 9,
        z: 0.0,
        id: "mount-1".into(),
        projection: Some(CaseHandleProjection {
            source: accepted.clone(),
            inputs: inputs.clone(),
        }),
    })));
    let mut preview = crate::case_gesture_preview::CaseGesturePreviewState::default();
    let preview_owner = preview
        .begin(
            started.scope.clone(),
            started.snapshot_token,
            started.revision,
        )
        .unwrap();
    assert!(preview.publish(&preview_owner, gesture_scene(&source_identity)));
    let display_scene = preview
        .scene(
            &accepted.scope,
            accepted.token,
            accepted.snapshot.document.revision,
        )
        .unwrap();
    assert!(!Rc::ptr_eq(&accepted, &display_scene));
    inputs.source = Rc::as_ptr(&display_scene) as usize;
    let current = ViewerSource::Cad(display_scene)
        .identity(&owner, &inputs)
        .unwrap();
    assert!(current.projection_generation > started.projection_generation);
    assert!(current.renderer_sequence > started.renderer_sequence);

    let handle_projection = CaseHandleProjection {
        source: accepted.clone(),
        inputs,
    };
    cancel_superseded_pointer(
        &pointer,
        &Rc::new(RefCell::new(None)),
        &owner,
        Some(&handle_projection),
        true,
    );
    assert!(
        pointer.borrow().is_some(),
        "Publishing this gesture's preview must retain its handle capture"
    );
    assert!(
        take_pointer_for_event(&pointer, 9, &started).is_none(),
        "Old callbacks must stay stale"
    );
    let dom = VirtualDom::new(|| rsx! { div {} });
    let delivered = Rc::new(RefCell::new(Vec::new()));
    let on_signal = dom.in_scope(ScopeId::ROOT, {
        let delivered = delivered.clone();
        move || {
            EventHandler::new(move |event: ScopedViewerSignal| {
                assert!(
                    event.is_current(),
                    "Case must receive a current owner stamp"
                );
                if let ViewerSignalKind::HandleGesture { phase, .. } = event.kind {
                    delivered.borrow_mut().push(phase);
                }
            })
        }
    });
    // The same admission used by Move/Up must see the current full identity,
    // including the renderer sequence, while old emissions remain rejected.
    let moved =
        take_pointer_for_event(&pointer, 9, &current).expect("Move must still own the handle");
    let applied_identity = current.clone();
    assert_eq!(moved.identity(), &applied_identity);
    emit_signal(
        &owner,
        on_signal,
        &started,
        ViewerSignalKind::HandleGesture {
            phase: HandleGesturePhase::End,
            handle_id: "mount-1".into(),
            point: None,
        },
    );
    assert!(delivered.borrow().is_empty());
    emit_signal(
        &owner,
        on_signal,
        moved.identity(),
        ViewerSignalKind::HandleGesture {
            phase: HandleGesturePhase::Move,
            handle_id: "mount-1".into(),
            point: Some([1.0, 2.0, 0.0]),
        },
    );
    *pointer.borrow_mut() = Some(moved);
    let completed = take_pointer_for_event(&pointer, 9, &current)
        .expect("Pointer-up must still own the handle");
    emit_signal(
        &owner,
        on_signal,
        completed.identity(),
        ViewerSignalKind::HandleGesture {
            phase: HandleGesturePhase::End,
            handle_id: "mount-1".into(),
            point: Some([1.0, 2.0, 0.0]),
        },
    );
    assert_eq!(
        *delivered.borrow(),
        [HandleGesturePhase::Move, HandleGesturePhase::End]
    );
    assert!(
        preview.cancel(&preview_owner),
        "End must retire the provisional preview"
    );
    assert!(
        preview
            .scene(
                &accepted.scope,
                accepted.token,
                accepted.snapshot.document.revision
            )
            .is_none()
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn case_handle_preview_transfer_rejects_replaced_source_scope_revision_and_owner() {
    for change in [
        "source", "session", "document", "board", "instance", "token", "revision", "viewer",
        "unmount", "stale", "models", "theme", "preview", "missing", "orbit",
    ] {
        let accepted = gesture_scene(&identity());
        let owner = ViewerOwner::new().unwrap();
        let (inputs, started) = start_gesture(&owner, &accepted);
        let mut next = CaseHandleProjection {
            source: accepted.clone(),
            inputs: inputs.clone(),
        };
        let display_scene = gesture_scene(&started);
        next.inputs.source = Rc::as_ptr(&display_scene) as usize;
        let mut current = ViewerSource::Cad(display_scene)
            .identity(&owner, &next.inputs)
            .unwrap();
        match change {
            "source" => next.source = gesture_scene(&started),
            "session" => current.scope.session_epoch = SessionEpoch(99),
            "document" => current.scope.document_id.push_str("-other"),
            "board" => current.scope.board_id.push_str("-other"),
            "instance" => current.scope.instance_id = None,
            "token" => current.snapshot_token = SnapshotToken(99),
            "revision" => current.revision += 1,
            "viewer" => current.viewer_instance += 1,
            "unmount" => owner.active.set(false),
            "models" => next.inputs.models.push(("model".into(), 1)),
            "theme" => next.inputs.theme = "dark".into(),
            "preview" => next.inputs.preview += 1,
            _ => {}
        }
        *owner.identity.borrow_mut() = Some(current);
        let active = if change == "orbit" {
            PointerOwner::Orbit {
                identity: started,
                pointer_id: 9,
                last_x: 0.0,
                last_y: 0.0,
                start_x: 0.0,
                start_y: 0.0,
                moved: false,
                picked: None,
            }
        } else {
            PointerOwner::Handle {
                identity: started,
                projection: Some(CaseHandleProjection {
                    source: accepted,
                    inputs,
                }),
                pointer_id: 9,
                z: 0.0,
                id: "mount-1".into(),
            }
        };
        let pointer = Rc::new(RefCell::new(Some(active)));
        cancel_superseded_pointer(
            &pointer,
            &Rc::new(RefCell::new(None)),
            &owner,
            (change != "missing").then_some(&next),
            change != "stale",
        );
        assert!(
            pointer.borrow().is_none(),
            "{change} must cancel the old capture"
        );
    }
}

fn orbit_pointer(identity: &ViewerIdentity) -> PointerOwner {
    PointerOwner::Orbit {
        identity: identity.clone(),
        pointer_id: 9,
        last_x: 10.0,
        last_y: 20.0,
        start_x: 10.0,
        start_y: 20.0,
        moved: false,
        picked: Some("body-1".to_owned()),
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_completion_keeps_its_starting_projection_identity() {
    let started = identity();
    let mut current = started.clone();
    current.projection_generation += 1;
    current.renderer_sequence += 1;
    let pointer = orbit_pointer(&started);
    assert_eq!(pointer.identity(), &started);
    assert_ne!(pointer.identity(), &current);
    let owner = ViewerOwner {
        active: Cell::new(true),
        identity: RefCell::new(Some(current)),
        last_inputs: RefCell::new(None),
        projection_generation: Cell::new(5),
        renderer_sequence: Cell::new(7),
        viewer_instance: started.viewer_instance,
    };
    assert!(!owner_is_current(&owner, pointer.identity()));
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn pointer_event_cannot_take_reused_id_from_another_projection() {
    let started = identity();
    let mut current = started.clone();
    current.projection_generation += 1;
    current.renderer_sequence += 1;
    let pointer = Rc::new(RefCell::new(Some(orbit_pointer(&current))));

    assert!(take_pointer_for_event(&pointer, 9, &started).is_none());
    assert_eq!(
        pointer.borrow().as_ref().map(PointerOwner::identity),
        Some(&current)
    );
    assert!(take_pointer_for_event(&pointer, 9, &current).is_some());
    assert!(pointer.borrow().is_none());
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn display_preference_aliases_match_case_display_groups() {
    assert_eq!(
        preference_ids("pcb"),
        ["PCB", "Models", "Keycaps", "Copper", "Mask", "Silkscreen"].map(str::to_owned)
    );
    assert_eq!(preference_ids("gaskets"), ["Gaskets"]);
    assert_eq!(
        preference_ids("gasket:plate:lower"),
        ["gasket:plate:lower", "gasket:plate:upper"]
    );
    assert_eq!(preference_ids("case-body-1"), ["case-body-1"]);
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn canvas_accessible_names_match_the_active_workflow_context() {
    assert!(
        ViewerCanvasContext::Layout
            .accessible_name()
            .contains("Layout PCB assembly")
    );
    assert!(
        ViewerCanvasContext::Layout
            .accessible_name()
            .contains("current Layout part")
    );
    assert!(
        ViewerCanvasContext::Case
            .accessible_name()
            .contains("3D Case preview")
    );
    assert!(
        !ViewerCanvasContext::Layout
            .accessible_name()
            .contains("Case preview")
    );
    assert!(
        ViewerCanvasContext::Keymap
            .accessible_name()
            .contains("3D Keymap board preview")
    );
    assert!(
        ViewerCanvasContext::Keycaps
            .accessible_name()
            .contains("3D Keycaps board preview")
    );
}

#[derive(Clone, Default)]
struct FeedbackLayoutProbe(Rc<RefCell<Option<Signal<Option<String>>>>>);

fn feedback_layout_fixture() -> Element {
    let probe = use_context::<FeedbackLayoutProbe>();
    let feedback = use_signal(|| None::<String>);
    *probe.0.borrow_mut() = Some(feedback);
    let owner = use_hook(|| {
        let owner = ViewerOwner::new().unwrap();
        // Exercise SharedViewer's actual DOM and maintained stylesheet.
        // GPU/provider work is outside this layout regression's boundary.
        owner.active.set(false);
        owner
    });
    let projection = use_hook(|| {
        Rc::new(RendererSceneProjection {
            identity: identity(),
            input: JsValue::NULL,
            layers: vec![],
        })
    });
    rsx! {
        style { {include_str!("../../../../assets/workbench-base.css")} }
        style { {include_str!("../../../../assets/tailwind.css")} }
        section { class: "relative flex min-h-0 flex-col overflow-hidden bg-canvas", style: "width:725px;height:560px",
            SharedViewer {
                projection,
                assembly_layers: vec![],
                component_layers: vec![],
                selectable_layers: vec![],
                canvas_context: ViewerCanvasContext::Case,
                selected_layer: String::new(),
                selected_reference: None,
                display: CaseDisplay::default(),
                theme: "light".to_owned(),
                owner,
                current_source: SourceGuard(Rc::new(|| false)),
                on_signal: |_| {},
                on_display_change: |_| {},
                mechanical_settings: None,
                inline_case_controls: true,
                handle_projection: None,
                handles: vec![],
                handle_preview: None,
                gesture_message: feedback(),
                focus_request: None,
            }
        }
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn mounted_case_gesture_feedback_preserves_canvas_geometry() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-feedback-layout-test");
    document.body().unwrap().append_child(&root).unwrap();
    let probe = FeedbackLayoutProbe::default();
    let dom = VirtualDom::new(feedback_layout_fixture);
    dom.provide_root_context(probe.clone());
    dom.provide_root_context(
        boardstudio_web_test_runtime::project_name_test_support::new_runtime(),
    );
    let _mounted = MountedFixture::new(root.clone(), dom);
    for _ in 0..50 {
        if root.query_selector("canvas").unwrap().is_some() {
            break;
        }
        gloo_timers::future::TimeoutFuture::new(10).await;
    }
    let canvas = root
        .query_selector("canvas")
        .unwrap()
        .expect("real SharedViewer canvas mounts");
    let before = canvas.get_bounding_client_rect();
    assert!(before.width() > 600.0 && before.height() > 200.0);
    let mut feedback = probe
        .0
        .borrow()
        .expect("fixture exposes its viewer feedback input");
    for message in [
        Some("Drag the mount; release to save its case position."),
        Some("Release to save mount position."),
        Some("Placement blocked by the case edge, openings, or another mount."),
        Some("Blocked move was not saved."),
        None,
    ] {
        feedback.set(message.map(str::to_owned));
        let mut rendered = false;
        for _ in 0..50 {
            let hint = root
                .query_selector("[role='status'][aria-label='Case edit guidance']")
                .unwrap();
            if hint
                .as_ref()
                .and_then(|node| node.text_content())
                .as_deref()
                == message
            {
                rendered = true;
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            rendered,
            "feedback must actually render before measuring: {message:?}"
        );
        let after = root
            .query_selector("canvas")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        for (name, old, new) in [
            ("x", before.x(), after.x()),
            ("y", before.y(), after.y()),
            ("width", before.width(), after.width()),
            ("height", before.height(), after.height()),
        ] {
            assert!(
                (old - new).abs() < 0.01,
                "gesture feedback must not change canvas {name}: {old} -> {new}, message {message:?}"
            );
        }
    }
    root.remove();
}
