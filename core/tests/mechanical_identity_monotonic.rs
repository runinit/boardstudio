//! Ticket 02: mechanical identities are monotonic within one Core document session,
//! including removal, Undo, owner lifecycle and exhausted namespaces.
use boardstudio_core::CoreEngine;
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension, MechanicalEdit,
    MechanicalHardwareTextField, MechanicalMountCollection, MechanicalTarget,
};
use boardstudio_core::model::*;
use serde_json::json;

fn configuration(board_id: &str) -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": board_id,
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
        "openings": [],
        "mounts": [],
        "closureMounts": [],
        "criticalFits": null,
        "hardware": null,
        "profiles": []
    }))
    .unwrap()
}

fn board() -> Board {
    Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

fn unibody() -> ProjectDoc {
    let mut document = ProjectDoc::empty("identities", "Identities");
    document.boards.push(board());
    document.mechanical = Some(configuration("board"));
    document
}

fn instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "primary".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(configuration(board_id)),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

fn paired_owners() -> ProjectDoc {
    let mut document = ProjectDoc::empty("identities", "Identities");
    document.boards.push(board());
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::None,
        instances: vec![instance("left", "board"), instance("right", "board")],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
}

fn template() -> PartDefinition {
    serde_json::from_value(json!({
        "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
        "kind":"custom", "pads":[{"id":"pad-0", "number": "", "at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
    }))
    .unwrap()
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn instance_target(id: &str) -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: id.into(),
    }
}

fn rev(engine: &mut CoreEngine) -> u64 {
    match engine.handle(CoreRequest::Snapshot { id: "rev".into() }) {
        CoreReply::Scene { scene, .. } => scene.revision,
        other => panic!("snapshot failed: {other:?}"),
    }
}

fn saved(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot { id: "saved".into() }) {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("snapshot failed: {other:?}"),
    }
}

fn open(engine: &mut CoreEngine, document: ProjectDoc) {
    match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }) {
        CoreReply::Scene { .. } => {}
        other => panic!("open failed: {other:?}"),
    }
}

fn commit(
    engine: &mut CoreEngine,
    target: &MechanicalTarget,
    change: MechanicalChange,
) -> CoreReply {
    let base_revision = rev(engine);
    engine.handle(CoreRequest::Edit {
        id: format!("{change:?}"),
        command: EditCommand {
            base_revision,
            transaction_id: format!("{change:?}"),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: EditOperation::ApplyMechanicalEdit {
                edit: Box::new(MechanicalEdit {
                    target: target.clone(),
                    change,
                    mounting_hole: Some(Box::new(template())),
                }),
            },
        },
    })
}

fn changed(reply: &CoreReply) -> Vec<String> {
    match reply {
        CoreReply::Scene { scene, .. } => scene.changed_ids.clone(),
        CoreReply::Error { message, .. } => panic!("edit failed: {message}"),
        other => panic!("unexpected reply: {other:?}"),
    }
}

fn error(reply: &CoreReply) -> String {
    match reply {
        CoreReply::Error { message, .. } => message.clone(),
        other => panic!("expected an error, got {other:?}"),
    }
}

fn one_id(reply: &CoreReply, prefix: &str) -> String {
    let ids: Vec<_> = changed(reply)
        .into_iter()
        .filter(|id| id.starts_with(prefix))
        .collect();
    assert_eq!(ids.len(), 1, "expected one {prefix} id in {ids:?}");
    ids[0].clone()
}

fn opening_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .mechanical
        .as_ref()
        .unwrap()
        .openings
        .iter()
        .flatten()
        .map(|opening| opening.id.clone())
        .collect()
}

fn fit_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .mechanical
        .as_ref()
        .unwrap()
        .critical_fits
        .iter()
        .flatten()
        .map(|fit| fit.id.clone())
        .collect()
}

fn hardware_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .mechanical
        .as_ref()
        .unwrap()
        .hardware
        .iter()
        .flatten()
        .map(|entry| entry.id.clone())
        .collect()
}

fn mount_ids(document: &ProjectDoc) -> Vec<String> {
    document
        .mechanical
        .as_ref()
        .unwrap()
        .mounts
        .iter()
        .map(|mount| mount.id.clone())
        .collect()
}

fn sparse_opening(id: &str) -> MechanicalOpening {
    MechanicalOpening {
        id: id.into(),
        vertices: vec![
            MechanicalOpeningVertex {
                id: "vertex-1".into(),
                point: Vec2 { x: 0.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-3".into(),
                point: Vec2 { x: 5.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-4".into(),
                point: Vec2 { x: 5.0, y: 5.0 },
            },
        ],
        z: 0.0,
        height: 1.0,
        next_vertex_identity: 0,
    }
}

fn sparse_fit(id: &str) -> MechanicalCriticalFit {
    MechanicalCriticalFit {
        id: id.into(),
        part_id: "p".into(),
        label: "fit".into(),
        from: Vec2 { x: 0.0, y: 0.0 },
        to: Vec2 { x: 1.0, y: 0.0 },
        tolerance: "±0.2 mm".into(),
    }
}

fn sparse_hardware(id: &str) -> MechanicalHardwareSpecification {
    MechanicalHardwareSpecification {
        id: id.into(),
        part_id: "p".into(),
        feature_id: "f".into(),
        designation: "screw".into(),
        thread: "M2".into(),
        length: 6.0,
        quantity: 1,
        notes: None,
        tolerance: None,
    }
}

fn sparse_mount(id: &str) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x: 0.0, y: 0.0 },
        kind: MountKind::Hole,
        hole_diameter: 2.5,
        boss_diameter: None,
        height: None,
    }
}

fn add_critical_fit() -> MechanicalChange {
    MechanicalChange::AddCriticalFit {
        part_id: "case-closure/part".into(),
    }
}

fn add_hardware() -> MechanicalChange {
    MechanicalChange::AddHardware {
        part_id: "case-closure/part".into(),
        feature_id: "case-mechanical-mount-1".into(),
    }
}

fn add_mount() -> MechanicalChange {
    MechanicalChange::AddMount {
        collection: MechanicalMountCollection::Suspension,
    }
}

fn remove_identity(kind: &str, id: String) -> MechanicalChange {
    match kind {
        "critical fit" => MechanicalChange::RemoveCriticalFit { fit_id: id },
        "hardware" => MechanicalChange::RemoveHardware { hardware_id: id },
        "mount" => MechanicalChange::RemoveMount {
            collection: MechanicalMountCollection::Suspension,
            mount_id: id,
        },
        _ => unreachable!(),
    }
}

#[test]
fn remove_and_add_assigns_a_fresh_identity_for_all_five_kinds() {
    // Opening.
    let mut engine = CoreEngine::new();
    open(&mut engine, unibody());
    let first = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveOpening {
            opening_id: first.clone(),
        },
    );
    let second = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(second, first);

    // Vertex.
    let vertex_first = one_id(
        &commit(
            &mut engine,
            &board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: second.clone(),
            },
        ),
        "vertex-",
    );
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveOpeningVertex {
            opening_id: second.clone(),
            vertex_id: vertex_first.clone(),
        },
    );
    let vertex_second = one_id(
        &commit(
            &mut engine,
            &board_target(),
            MechanicalChange::AddOpeningVertex { opening_id: second },
        ),
        "vertex-",
    );
    assert_ne!(vertex_second, vertex_first);

    // Critical fit, hardware and mount.
    for (label, add, ids) in [
        (
            "critical fit",
            add_critical_fit(),
            fit_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "hardware",
            add_hardware(),
            hardware_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "mount",
            add_mount(),
            mount_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
    ] {
        let mut engine = CoreEngine::new();
        open(&mut engine, unibody());
        commit(&mut engine, &board_target(), add.clone());
        let first = ids(&saved(&mut engine)).pop().unwrap();
        commit(
            &mut engine,
            &board_target(),
            remove_identity(label, first.clone()),
        );
        commit(&mut engine, &board_target(), add);
        let second = ids(&saved(&mut engine)).pop().unwrap();
        assert_ne!(second, first, "{label} reused a removed identity");
    }
}

#[test]
fn add_undo_add_assigns_a_fresh_identity_for_all_five_kinds() {
    type IdentityList = fn(&ProjectDoc) -> Vec<String>;
    let cases: [(&str, MechanicalChange, IdentityList); 5] = [
        (
            "opening",
            MechanicalChange::AddOpening,
            opening_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        ("critical fit", add_critical_fit(), fit_ids),
        ("hardware", add_hardware(), hardware_ids),
        ("mount", add_mount(), mount_ids),
        (
            "vertex",
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-1".into(),
            },
            |document| {
                document
                    .mechanical
                    .as_ref()
                    .unwrap()
                    .openings
                    .iter()
                    .flatten()
                    .flat_map(|opening| opening.vertices.iter())
                    .map(|vertex| vertex.id.clone())
                    .collect()
            },
        ),
    ];
    for (label, add, ids) in cases {
        let mut engine = CoreEngine::new();
        open(&mut engine, unibody());
        if label == "vertex" {
            commit(&mut engine, &board_target(), MechanicalChange::AddOpening);
        }
        commit(&mut engine, &board_target(), add.clone());
        let first = ids(&saved(&mut engine)).pop().unwrap();
        match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
            CoreReply::Scene { .. } => {}
            other => panic!("undo failed: {other:?}"),
        }
        commit(&mut engine, &board_target(), add);
        let second = ids(&saved(&mut engine)).pop().unwrap();
        assert_ne!(second, first, "{label} reused an undone identity");
    }
}

#[test]
fn owners_keep_separate_counters_and_survive_disable_reenable() {
    let mut engine = CoreEngine::new();
    open(&mut engine, paired_owners());
    let left_first = one_id(
        &commit(
            &mut engine,
            &instance_target("left"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    let right_first = one_id(
        &commit(
            &mut engine,
            &instance_target("right"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    assert_eq!(left_first, "opening-1");
    assert_eq!(right_first, "opening-1", "owners must not share a counter");

    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Disable,
    );
    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Enable,
    );
    let left_next = one_id(
        &commit(
            &mut engine,
            &instance_target("left"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    assert_ne!(
        left_next, left_first,
        "disable/re-enable reused an identity"
    );
    let right_next = one_id(
        &commit(
            &mut engine,
            &instance_target("right"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    assert_ne!(right_next, right_first);
}

#[test]
fn sparse_imported_ids_seed_allocation_and_max_identities_exhaust_cleanly() {
    // Sparse live identities seed the high-water mark, so the hole is not reused.
    for (label, add, mutate, expected, ids) in [
        (
            "opening",
            MechanicalChange::AddOpening,
            (|configuration: &mut MechanicalConfiguration| {
                configuration.openings = Some(vec![
                    sparse_opening("opening-1"),
                    sparse_opening("opening-3"),
                ]);
            }) as fn(&mut MechanicalConfiguration),
            "opening-4",
            opening_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "vertex",
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-1".into(),
            },
            (|configuration: &mut MechanicalConfiguration| {
                configuration.openings = Some(vec![sparse_opening("opening-1")]);
            }) as fn(&mut MechanicalConfiguration),
            "vertex-5",
            |document: &ProjectDoc| {
                document
                    .mechanical
                    .as_ref()
                    .unwrap()
                    .openings
                    .iter()
                    .flatten()
                    .flat_map(|opening| opening.vertices.iter())
                    .map(|vertex| vertex.id.clone())
                    .collect()
            },
        ),
        (
            "critical fit",
            add_critical_fit(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.critical_fits = Some(vec![
                    sparse_fit("case-critical-fit-1"),
                    sparse_fit("case-critical-fit-3"),
                ]);
            }) as fn(&mut MechanicalConfiguration),
            "case-critical-fit-4",
            fit_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "hardware",
            add_hardware(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.hardware = Some(vec![
                    sparse_hardware("case-mechanical-hardware-1"),
                    sparse_hardware("case-mechanical-hardware-3"),
                ]);
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-hardware-4",
            hardware_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "mount",
            add_mount(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.mounts = vec![
                    sparse_mount("case-mechanical-mount-1"),
                    sparse_mount("case-mechanical-mount-3"),
                ];
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-mount-4",
            mount_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
    ] {
        let mut document = unibody();
        mutate(document.mechanical.as_mut().unwrap());
        let mut engine = CoreEngine::new();
        open(&mut engine, document);
        commit(&mut engine, &board_target(), add);
        let added = ids(&saved(&mut engine)).pop().unwrap();
        assert_eq!(added, expected, "{label}: {added}");
    }

    // MAX-1 -> MAX -> clean exhaustion for every namespace.
    for (label, add, mutate, prefix, ids) in [
        (
            "opening suffix",
            MechanicalChange::AddOpening,
            (|configuration: &mut MechanicalConfiguration| {
                configuration.openings = Some(vec![MechanicalOpening {
                    id: format!("opening-{}", u64::MAX - 1),
                    vertices: vec![],
                    z: 0.0,
                    height: 1.0,
                    next_vertex_identity: 0,
                }]);
            }) as fn(&mut MechanicalConfiguration),
            "opening-",
            opening_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "critical fit suffix",
            add_critical_fit(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.critical_fits = Some(vec![MechanicalCriticalFit {
                    id: format!("case-critical-fit-{}", u64::MAX - 1),
                    part_id: "p".into(),
                    label: "fit".into(),
                    from: Vec2 { x: 0.0, y: 0.0 },
                    to: Vec2 { x: 1.0, y: 0.0 },
                    tolerance: "±0.2 mm".into(),
                }]);
            }) as fn(&mut MechanicalConfiguration),
            "case-critical-fit-",
            fit_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "hardware suffix",
            add_hardware(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.hardware = Some(vec![MechanicalHardwareSpecification {
                    id: format!("case-mechanical-hardware-{}", u64::MAX - 1),
                    part_id: "p".into(),
                    feature_id: "f".into(),
                    designation: "screw".into(),
                    thread: "M2".into(),
                    length: 6.0,
                    quantity: 1,
                    notes: None,
                    tolerance: None,
                }]);
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-hardware-",
            hardware_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
        (
            "mount suffix",
            add_mount(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.mounts = vec![Mount {
                    id: format!("case-mechanical-mount-{}", u64::MAX - 1),
                    at: Vec2 { x: 0.0, y: 0.0 },
                    kind: MountKind::Hole,
                    hole_diameter: 2.5,
                    boss_diameter: None,
                    height: None,
                }];
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-mount-",
            mount_ids as fn(&ProjectDoc) -> Vec<String>,
        ),
    ] {
        let mut document = unibody();
        mutate(document.mechanical.as_mut().unwrap());
        let mut engine = CoreEngine::new();
        open(&mut engine, document);
        commit(&mut engine, &board_target(), add.clone());
        let max = ids(&saved(&mut engine)).pop().unwrap();
        assert_eq!(max, format!("{prefix}{}", u64::MAX), "{label}");
        let accepted = saved(&mut engine);
        let refused = commit(&mut engine, &board_target(), add);
        assert!(
            error(&refused).contains("exhausted"),
            "{label}: {refused:?}"
        );
        assert_eq!(saved(&mut engine), accepted, "{label} changed state");
    }

    // An imported counter advances MAX-1 -> MAX -> exhaustion.
    let mut document = unibody();
    document.mechanical.as_mut().unwrap().next_opening_identity = u64::MAX - 1;
    let mut engine = CoreEngine::new();
    open(&mut engine, document);
    commit(&mut engine, &board_target(), MechanicalChange::AddOpening);
    assert_eq!(
        opening_ids(&saved(&mut engine)),
        [format!("opening-{}", u64::MAX - 1)]
    );
    let accepted = saved(&mut engine);
    let refused = commit(&mut engine, &board_target(), MechanicalChange::AddOpening);
    assert!(error(&refused).contains("exhausted"), "{refused:?}");
    assert_eq!(saved(&mut engine), accepted);

    // A live MAX suffix is refused rather than saturated.
    let mut document = unibody();
    document.mechanical.as_mut().unwrap().openings = Some(vec![MechanicalOpening {
        id: format!("opening-{}", u64::MAX),
        vertices: vec![],
        z: 0.0,
        height: 1.0,
        next_vertex_identity: 0,
    }]);
    let mut engine = CoreEngine::new();
    open(&mut engine, document);
    let refused = commit(&mut engine, &board_target(), MechanicalChange::AddOpening);
    assert!(error(&refused).contains("exhausted"), "{refused:?}");
}

#[test]
fn exhausted_reservations_do_not_reopen_after_disable_reenable() {
    let mut document = paired_owners();
    let left = document
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "left")
        .unwrap();
    left.mechanical.as_mut().unwrap().openings = Some(vec![MechanicalOpening {
        id: format!("opening-{}", u64::MAX),
        vertices: vec![
            MechanicalOpeningVertex {
                id: "vertex-1".into(),
                point: Vec2 { x: 0.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-2".into(),
                point: Vec2 { x: 5.0, y: 0.0 },
            },
            MechanicalOpeningVertex {
                id: "vertex-3".into(),
                point: Vec2 { x: 5.0, y: 5.0 },
            },
        ],
        z: 0.0,
        height: 1.0,
        next_vertex_identity: 4,
    }]);
    left.mechanical.as_mut().unwrap().next_opening_identity = u64::MAX;
    let mut engine = CoreEngine::new();
    open(&mut engine, document);
    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Disable,
    );
    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Enable,
    );
    let accepted = saved(&mut engine);
    let refused = commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::AddOpening,
    );
    assert!(error(&refused).contains("exhausted"), "{refused:?}");
    assert_eq!(saved(&mut engine), accepted);
}

#[test]
fn failed_open_keeps_reservations_and_successful_open_starts_a_new_session() {
    let mut engine = CoreEngine::new();
    open(&mut engine, unibody());
    let first = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );

    // A failed explicit Open must not reset the document session.
    let mut duplicate = unibody();
    let part = Part {
        anchor: None,
        keycap: None,
        outline: None,
        id: "dup".into(),
        definition_id: "missing".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    duplicate.parts.push(part.clone());
    duplicate.parts.push(part);
    let reply = engine.handle(CoreRequest::Open {
        id: "failed".into(),
        document: duplicate,
    });
    assert!(matches!(reply, CoreReply::Error { .. }));

    let second = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(second, first, "a failed Open reset the reservation");

    // A successful explicit Open is a new session: imported/live ids seed it.
    open(&mut engine, unibody());
    let reset = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_eq!(reset, "opening-1");
}

#[test]
fn stored_instance_reservations_seed_re_enabled_allocation() {
    let mut document = paired_owners();
    let left = document
        .hardware
        .as_mut()
        .unwrap()
        .instances
        .iter_mut()
        .find(|instance| instance.id == "left")
        .unwrap();
    left.mechanical = None;
    left.mechanical_identity_reservation = Some(MechanicalIdentityReservation {
        next_opening_identity: 9,
        next_vertex_identity: 1,
        next_critical_fit_identity: 1,
        next_hardware_identity: 1,
        next_mount_identity: 1,
    });
    let mut engine = CoreEngine::new();
    open(&mut engine, document);
    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Enable,
    );
    let opening = one_id(
        &commit(
            &mut engine,
            &instance_target("left"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    assert_eq!(opening, "opening-9");
}

#[test]
fn stale_setters_cannot_change_a_replacement_for_all_five_kinds() {
    let mut engine = CoreEngine::new();
    open(&mut engine, unibody());
    let opening_first = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    let vertex_first = one_id(
        &commit(
            &mut engine,
            &board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: opening_first.clone(),
            },
        ),
        "vertex-",
    );
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveOpeningVertex {
            opening_id: opening_first.clone(),
            vertex_id: vertex_first.clone(),
        },
    );
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveOpening {
            opening_id: opening_first.clone(),
        },
    );
    let opening_second = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    let vertex_second = one_id(
        &commit(
            &mut engine,
            &board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: opening_second.clone(),
            },
        ),
        "vertex-",
    );
    assert_ne!(opening_second, opening_first);
    assert_ne!(vertex_second, vertex_first);

    let fit_first = {
        commit(&mut engine, &board_target(), add_critical_fit());
        fit_ids(&saved(&mut engine)).pop().unwrap()
    };
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveCriticalFit {
            fit_id: fit_first.clone(),
        },
    );
    commit(&mut engine, &board_target(), add_critical_fit());
    let fit_second = fit_ids(&saved(&mut engine)).pop().unwrap();
    assert_ne!(fit_second, fit_first);

    let hardware_first = {
        commit(&mut engine, &board_target(), add_hardware());
        hardware_ids(&saved(&mut engine)).pop().unwrap()
    };
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveHardware {
            hardware_id: hardware_first.clone(),
        },
    );
    commit(&mut engine, &board_target(), add_hardware());
    let hardware_second = hardware_ids(&saved(&mut engine)).pop().unwrap();
    assert_ne!(hardware_second, hardware_first);

    let mount_first = {
        commit(&mut engine, &board_target(), add_mount());
        mount_ids(&saved(&mut engine)).pop().unwrap()
    };
    commit(
        &mut engine,
        &board_target(),
        MechanicalChange::RemoveMount {
            collection: MechanicalMountCollection::Suspension,
            mount_id: mount_first.clone(),
        },
    );
    commit(&mut engine, &board_target(), add_mount());
    let mount_second = mount_ids(&saved(&mut engine)).pop().unwrap();
    assert_ne!(mount_second, mount_first);

    // Setters authored against the removed identities must not reach the replacements.
    for (label, change) in [
        (
            "opening",
            MechanicalChange::SetOpeningDimension {
                opening_id: opening_first,
                vertex_id: None,
                field: MechanicalDimension::OpeningHeight,
                value: 99.0,
            },
        ),
        (
            "vertex",
            MechanicalChange::SetOpeningDimension {
                opening_id: opening_second,
                vertex_id: Some(vertex_first),
                field: MechanicalDimension::OpeningPointX,
                value: 1.0,
            },
        ),
        (
            "critical fit",
            MechanicalChange::SetCriticalFitText {
                fit_id: fit_first,
                field: MechanicalCriticalFitTextField::Label,
                value: "stale".into(),
            },
        ),
        (
            "hardware",
            MechanicalChange::SetHardwareText {
                hardware_id: hardware_first,
                field: MechanicalHardwareTextField::Designation,
                value: "stale".into(),
            },
        ),
        (
            "mount",
            MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: mount_first,
                at: Vec2 { x: 1.0, y: 1.0 },
            },
        ),
    ] {
        let accepted = saved(&mut engine);
        let refused = commit(&mut engine, &board_target(), change);
        assert!(
            matches!(refused, CoreReply::Error { .. }),
            "{label}: {refused:?}"
        );
        assert_eq!(saved(&mut engine), accepted, "{label} changed state");
    }
}

#[test]
fn redo_and_undo_of_configuration_creation_keep_high_water_marks() {
    let mut engine = CoreEngine::new();
    open(&mut engine, unibody());
    let first = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { .. } => {}
        other => panic!("undo failed: {other:?}"),
    }
    match engine.handle(CoreRequest::Redo { id: "redo".into() }) {
        CoreReply::Scene { .. } => {}
        other => panic!("redo failed: {other:?}"),
    }
    let after_redo = one_id(
        &commit(&mut engine, &board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(after_redo, first, "redo reused an identity");

    let mut engine = CoreEngine::new();
    open(&mut engine, paired_owners());
    let left_first = one_id(
        &commit(
            &mut engine,
            &instance_target("left"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    commit(
        &mut engine,
        &instance_target("left"),
        MechanicalChange::Disable,
    );
    match engine.handle(CoreRequest::Undo { id: "undo".into() }) {
        CoreReply::Scene { .. } => {}
        other => panic!("undo failed: {other:?}"),
    }
    let after_undo = one_id(
        &commit(
            &mut engine,
            &instance_target("left"),
            MechanicalChange::AddOpening,
        ),
        "opening-",
    );
    assert_ne!(
        after_undo, left_first,
        "undo of creation reused an identity"
    );
}

#[test]
fn max_counter_sentinels_are_refused_for_all_five_namespaces() {
    for (label, add, mutate) in [
        (
            "opening",
            MechanicalChange::AddOpening,
            (|configuration: &mut MechanicalConfiguration| {
                configuration.next_opening_identity = u64::MAX;
            }) as fn(&mut MechanicalConfiguration),
        ),
        (
            "vertex",
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-1".into(),
            },
            (|configuration: &mut MechanicalConfiguration| {
                let mut opening = sparse_opening("opening-1");
                opening.next_vertex_identity = u64::MAX;
                configuration.openings = Some(vec![opening]);
            }) as fn(&mut MechanicalConfiguration),
        ),
        (
            "critical fit",
            add_critical_fit(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.next_critical_fit_identity = u64::MAX;
            }) as fn(&mut MechanicalConfiguration),
        ),
        (
            "hardware",
            add_hardware(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.next_hardware_identity = u64::MAX;
            }) as fn(&mut MechanicalConfiguration),
        ),
        (
            "mount",
            add_mount(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.next_mount_identity = u64::MAX;
            }) as fn(&mut MechanicalConfiguration),
        ),
    ] {
        let mut document = unibody();
        mutate(document.mechanical.as_mut().unwrap());
        let mut engine = CoreEngine::new();
        open(&mut engine, document);
        let accepted = saved(&mut engine);
        let refused = commit(&mut engine, &board_target(), add);
        assert!(
            error(&refused).contains("exhausted"),
            "{label}: {refused:?}"
        );
        assert_eq!(saved(&mut engine), accepted, "{label} changed state");
    }
}
