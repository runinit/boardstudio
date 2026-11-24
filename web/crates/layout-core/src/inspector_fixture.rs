#[cfg(not(target_arch = "wasm32"))]
use boardstudio_application::{AcceptedSnapshot, ReadModel, Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{
    Board, Layout, LayoutLinkPair, LayoutMirrorLink, MirroredItemKind, Part, PartDefinition,
    PartKind, PartOutline, Pose2, ProjectDoc, Side, Vec2,
};
#[cfg(not(target_arch = "wasm32"))]
use boardstudio_core::model::{Readiness, SceneDelta};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// The document the fixture and the mounted runtime share: a board, two positioned
/// parts and the two mirrored layouts the projections read.
pub(super) fn fixture_document(configure: impl FnOnce(&mut ProjectDoc)) -> ProjectDoc {
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
            anchor: None,
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
            correspondences: vec![LayoutLinkPair {
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

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn fixture() -> (ReadModel, boardstudio_web_ui_model::tree::ScopedTreeContext) {
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
    let context = boardstudio_web_ui_model::tree::context_for_part(&model, "selected-part")
        .expect("fixture component has a current standalone tree context");
    let selected = boardstudio_web_ui_model::tree::ScopedTreeContext { scope, context };
    model.selection_anchor_id = Some("selected-part".into());
    (model, selected)
}
