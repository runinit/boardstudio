//! Mechanical tests that drive the real `CoreEngine`.
//!
//! They live here, one layer above `boardstudio-core-mechanical`, so that crate's own tests do
//! not depend on the engine (a dev-dependency cycle would hold its test binary back until the
//! whole engine graph is built).

use boardstudio_core_engine::CoreEngine;
use boardstudio_core_engine::mechanical::change::MechanicalFeatureCorrespondence;
use boardstudio_core_engine::mechanical::clearance::project_owned_closure_clearance;
use boardstudio_core_engine::mechanical::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use boardstudio_core_engine::model::{
    Board, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, HardwareConfiguration,
    HardwareTopology, HardwareTransport, MechanicalConfiguration, MechanicalConstructionOwnership,
    MechanicalFeatureKind, Mount, MountKind, Part, PartDefinition, PhysicalBoardInstance, Pose2,
    ProjectDoc, Side, Vec2,
};
use serde_json::json;

/// The generated closure-clearance part prefix of `mechanical::clearance`.
const PART_PREFIX: &str = "case-closure/";

fn configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": "board",
        "method": "printed",
        "mount": "rigid",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4,
        "batteryHeight": 4.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "bottomStyle": "shell",
        "middleFrame": false,
        "openingAllowance": 0.25,
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .unwrap()
}

/// The normalized bundled mounting-hole catalogue entry projection expects.
fn hole_template() -> PartDefinition {
    serde_json::from_value(json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

fn board(id: &str) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

fn mount(id: &str, x: f64, y: f64) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x, y },
        kind: MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    }
}

// ---- closure clearance ---------------------------------------------------------------------

fn clearance_configuration(clearance: f64, mounts: Vec<Mount>) -> MechanicalConfiguration {
    let mut configuration = serde_json::from_value::<MechanicalConfiguration>(json!({
        "boardId": "board", "method": "printed", "mount": "rigid",
        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
        "bottomFoamThickness": 0.5, "batteryHeight": 0.0, "bottomThickness": 2.0,
        "plateToPcb": 3.0, "wallThickness": 2.0, "clearance": clearance,
        "profiles": [], "mounts": [], "closureMounts": mounts
    }))
    .unwrap();
    configuration.clearance = clearance;
    configuration
}

fn document_with_reference(reference: &str, mount_count: usize) -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Doc");
    let mut host = board("board");
    host.part_ids = vec!["authored".into()];
    document.boards.push(host);
    let definition = hole_template();
    document.definitions.push(definition.clone());
    document.parts.push(Part {
        anchor: None,
        id: "authored".into(),
        definition_id: definition.id,
        reference: reference.into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        outline: None,
        keycap: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    let mounts = (0..mount_count)
        .map(|index| Mount {
            id: "mount".into(),
            at: Vec2 {
                x: index as f64 + 10.0,
                y: 20.0,
            },
            kind: MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        })
        .collect();
    document.mechanical = Some(clearance_configuration(0.0, mounts));
    document
}

fn open_core_document(document: ProjectDoc) -> (CoreEngine, ProjectDoc) {
    let mut engine = CoreEngine::new();
    match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }) {
        CoreReply::Scene { document, .. } => (engine, *document),
        reply => panic!("core rejected accepted-document fixture: {reply:?}"),
    }
}

#[test]
fn unsafe_or_exhausted_mh_references_fail_before_any_core_edit() {
    for (reference, mount_count) in [
        ("MH9007199254740992", 1),
        ("MH18446744073709551616", 1),
        ("MH9007199254740990", 2),
    ] {
        let (mut core, accepted) =
            open_core_document(document_with_reference(reference, mount_count));
        let error =
            project_owned_closure_clearance(accepted.clone(), &hole_template()).unwrap_err();
        assert!(error.to_ascii_lowercase().contains("reference"), "{error}");
        assert_eq!(accepted.parts[0].reference, reference);
        match core.handle(CoreRequest::Snapshot {
            id: "snapshot".into(),
        }) {
            CoreReply::Scene { document, .. } => assert_eq!(*document, accepted),
            reply => panic!("core snapshot failed after rejected planner result: {reply:?}"),
        }
    }
}

#[test]
fn safe_mh_reference_and_output_count_boundaries_remain_allocatable() {
    let (mut core, accepted) = open_core_document(document_with_reference("MH9007199254740989", 2));
    let projected = project_owned_closure_clearance(accepted.clone(), &hole_template()).unwrap();
    let generated = projected
        .parts
        .iter()
        .filter(|part| part.id.starts_with(PART_PREFIX))
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 2);
    assert_eq!(generated[0].reference, "MH9007199254740990");
    assert_eq!(generated[1].reference, "MH9007199254740991");
    match core.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => assert_eq!(*document, accepted),
        reply => panic!("core snapshot failed after pure projection: {reply:?}"),
    }
}

// ---- mirror-pair link and undo ---------------------------------------------------------------

fn mirror_instance(id: &str, mounts: Vec<Mount>) -> PhysicalBoardInstance {
    let mut mechanical = configuration();
    mechanical.mounts = mounts;
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(mechanical),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

/// Two physical halves on one board, each with its own local mount geometry.
fn mirror_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("doc", "Fixture");
    doc.boards.push(board("board"));
    doc.mechanical = Some(configuration());
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            mirror_instance("left", vec![mount("mount-1", 2.0, 3.0)]),
            mirror_instance("right", vec![mount("mount-1", 7.0, 3.0)]),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    doc
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn correspondence() -> MechanicalFeatureCorrespondence {
    MechanicalFeatureCorrespondence {
        kind: MechanicalFeatureKind::Mount,
        first_feature_id: "mount-1".into(),
        second_feature_id: "mount-1".into(),
        source_instance_id: None,
    }
}

fn link_edit(target: MechanicalTarget, pair_id: &str, source: &str) -> MechanicalEdit {
    MechanicalEdit {
        target,
        change: MechanicalChange::LinkMirrorPair {
            pair_id: pair_id.into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            construction_source: source.into(),
            correspondences: vec![correspondence()],
        },
        mounting_hole: Some(Box::new(hole_template())),
    }
}

#[test]
fn an_end_to_end_link_commits_and_one_undo_restores_the_document() {
    let mut engine = CoreEngine::new();
    let before = mirror_document();
    let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: before.clone(),
    }) else {
        panic!("open must succeed");
    };
    let revision = document.revision;
    let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Edit {
        id: "link".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "link".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::ApplyMechanicalEdit {
                edit: Box::new(link_edit(board_target(), "pair-left-right", "left")),
            },
        },
    }) else {
        panic!("the link must commit");
    };
    assert_eq!(document.hardware.as_ref().unwrap().pairs.len(), 1);
    let CoreReply::Scene { document, .. } = engine.handle(CoreRequest::Undo { id: "undo".into() })
    else {
        panic!("undo must succeed");
    };
    assert_eq!(document.hardware, before.hardware);
    assert_eq!(document.mechanical, before.mechanical);
}
