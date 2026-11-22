use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_model::model::{
    Board, BoardContours, Contour, Part, Pose2, Readiness, SceneDelta, Side, Vec2,
};

fn accepted(imported: bool) -> (AcceptedSnapshot, Scope) {
    let mut document = ProjectDoc::empty("doc", "Split board");
    document.revision = 7;
    document.boards.push(Board {
        id: "left".into(),
        name: "Left".into(),
        outline_ids: vec![],
        part_ids: vec!["part-1".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.push(Part {
        anchor: None,
        keycap: None,
        outline: None,
        id: "part-1".into(),
        definition_id: "mcu".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 2.0, y: 3.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    if imported {
        document.assets.push(Asset {
            id: "board-source".into(),
            name: "board.kicad_pcb".into(),
            media_type: "application/vnd.kicad.pcb".into(),
            sha256: "abc123".into(),
            license: None,
            source: None,
        });
        document.board_references.push(BoardReference {
            id: "left-reference".into(),
            board_id: "left".into(),
            asset_id: "board-source".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 {
                    x: 500.0,
                    y: -300.0,
                },
                rotation: 180.0,
            },
            elevation: 9.0,
            model_assets: BTreeMap::new(),
        });
    }
    let scope = Scope {
        session_epoch: SessionEpoch(2),
        document_id: "doc".into(),
        board_id: "left".into(),
        instance_id: Some("flipped-left".into()),
    };
    let snapshot = AcceptedSnapshot {
        token: SnapshotToken(11),
        session_epoch: scope.session_epoch,
        document: Arc::new(document),
        scene: Arc::new(SceneDelta {
            module_scenes: vec![],
            revision: 7,
            transaction_id: "accepted".into(),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "left".into(),
                contours: vec![Contour {
                    points: vec![
                        Vec2 { x: 1.0, y: 2.0 },
                        Vec2 { x: 4.0, y: 2.0 },
                        Vec2 { x: 4.0, y: 5.0 },
                    ],
                    hole: false,
                }],
            }],
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
    (snapshot, scope)
}

fn capture_source(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
    generation: u64,
    request_id: &str,
) -> Result<LayoutSourceCapture, String> {
    LayoutSourceCapture::capture(
        snapshot,
        scope,
        generation,
        request_id.into(),
        BTreeMap::new(),
    )
}

fn source_capture(snapshot: &AcceptedSnapshot, scope: &Scope) -> LayoutSourceCapture {
    capture_source(snapshot, scope, 3, "layout-preview-11").unwrap()
}

#[test]
fn authored_capture_uses_canonical_document_and_board_contours_independent_of_instance() {
    let (snapshot, scope) = accepted(false);
    let capture = source_capture(&snapshot, &scope);
    let mut another_instance = scope.clone();
    another_instance.instance_id = Some("other-physical-half".into());
    let other = source_capture(&snapshot, &another_instance);

    assert_eq!(capture.document.as_ref(), snapshot.document.as_ref());
    assert_eq!(capture.contours, snapshot.scene.board_contours[0].contours);
    assert_eq!(capture.contours, other.contours);
    assert_ne!(capture.owner, other.owner);
    let LayoutPreviewRequest::Authored(request) = capture.request else {
        panic!("authored board should use PreviewPcb");
    };
    assert_eq!(request.document.parts[0].pose.at.x, 2.0);
    assert_eq!(request.contours, snapshot.scene.board_contours[0].contours);
    assert_eq!(
        request.target,
        ExportTarget::Board {
            board_id: "left".into()
        }
    );
}
