//! Ticket 02: mechanical identities are monotonic within one Core document session,
//! including removal, Undo, owner lifecycle and exhausted namespaces.
use crate::support::fixture;
use crate::support::session::{
    Session, board_target,
    builders::{sparse_fit, sparse_hardware, sparse_mount, sparse_opening},
    error, instance_target, mechanical_ids, one_id, paired_instance, sparse_configuration,
    sparse_mechanical_doc,
};
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension,
    MechanicalHardwareTextField, MechanicalMountCollection,
};
use boardstudio_core::model::*;

fn unibody() -> ProjectDoc {
    sparse_mechanical_doc("identities", "Identities")
}

fn paired_owners() -> ProjectDoc {
    let mut document = ProjectDoc::empty("identities", "Identities");
    document.boards.push(fixture::board("board", "Board"));
    let owner = |id: &str| {
        paired_instance(
            id,
            "board",
            "primary",
            Some(sparse_configuration("board")),
            MechanicalConstructionOwnership::Local,
        )
    };
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Split,
        transport: HardwareTransport::None,
        instances: vec![owner("left"), owner("right")],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    document
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

type IdentityList = fn(&ProjectDoc) -> Vec<String>;

fn replace_identity(
    engine: &mut Session,
    label: &str,
    add: MechanicalChange,
    ids: IdentityList,
) -> String {
    engine.commit_mechanical(board_target(), add.clone());
    let first = ids(&engine.snapshot()).pop().unwrap();
    engine.commit_mechanical(board_target(), remove_identity(label, first.clone()));
    engine.commit_mechanical(board_target(), add);
    let second = ids(&engine.snapshot()).pop().unwrap();
    assert_ne!(second, first, "{label} reused a removed identity");
    first
}

#[test]
fn remove_and_add_assigns_a_fresh_identity_for_all_five_kinds() {
    // Opening.
    let mut engine = Session::default();
    assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
    let first = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    engine.commit_mechanical(
        board_target(),
        MechanicalChange::RemoveOpening {
            opening_id: first.clone(),
        },
    );
    let second = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(second, first);

    // Vertex.
    let vertex_first = one_id(
        &engine.commit_mechanical(
            board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: second.clone(),
            },
        ),
        "vertex-",
    );
    engine.commit_mechanical(
        board_target(),
        MechanicalChange::RemoveOpeningVertex {
            opening_id: second.clone(),
            vertex_id: vertex_first.clone(),
        },
    );
    let vertex_second = one_id(
        &engine.commit_mechanical(
            board_target(),
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
            (|document: &ProjectDoc| mechanical_ids(document, "fit")) as IdentityList,
        ),
        (
            "hardware",
            add_hardware(),
            (|document: &ProjectDoc| mechanical_ids(document, "hardware")) as IdentityList,
        ),
        (
            "mount",
            add_mount(),
            (|document: &ProjectDoc| mechanical_ids(document, "mount")) as IdentityList,
        ),
    ] {
        let mut engine = Session::default();
        assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
        replace_identity(&mut engine, label, add, ids);
    }
}

#[test]
fn add_undo_add_assigns_a_fresh_identity_for_all_five_kinds() {
    let cases: [(&str, MechanicalChange, IdentityList); 5] = [
        (
            "opening",
            MechanicalChange::AddOpening,
            (|document: &ProjectDoc| mechanical_ids(document, "opening")) as IdentityList,
        ),
        (
            "critical fit",
            add_critical_fit(),
            (|document: &ProjectDoc| mechanical_ids(document, "fit")) as IdentityList,
        ),
        (
            "hardware",
            add_hardware(),
            (|document: &ProjectDoc| mechanical_ids(document, "hardware")) as IdentityList,
        ),
        (
            "mount",
            add_mount(),
            (|document: &ProjectDoc| mechanical_ids(document, "mount")) as IdentityList,
        ),
        (
            "vertex",
            MechanicalChange::AddOpeningVertex {
                opening_id: "opening-1".into(),
            },
            (|document: &ProjectDoc| mechanical_ids(document, "vertex")) as IdentityList,
        ),
    ];
    for (label, add, ids) in cases {
        let mut engine = Session::default();
        assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
        if label == "vertex" {
            engine.commit_mechanical(board_target(), MechanicalChange::AddOpening);
        }
        engine.commit_mechanical(board_target(), add.clone());
        let first = ids(&engine.snapshot()).pop().unwrap();
        match engine.undo() {
            CoreReply::Scene { .. } => {}
            other => panic!("undo failed: {other:?}"),
        }
        engine.commit_mechanical(board_target(), add);
        let second = ids(&engine.snapshot()).pop().unwrap();
        assert_ne!(second, first, "{label} reused an undone identity");
    }
}

#[test]
fn owners_keep_separate_counters_and_survive_disable_reenable() {
    let mut engine = Session::default();
    assert!(matches!(
        engine.open(paired_owners()),
        CoreReply::Scene { .. }
    ));
    let left_first = one_id(
        &engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening),
        "opening-",
    );
    let right_first = one_id(
        &engine.commit_mechanical(instance_target("right"), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_eq!(left_first, "opening-1");
    assert_eq!(right_first, "opening-1", "owners must not share a counter");

    engine.commit_mechanical(instance_target("left"), MechanicalChange::Disable);
    engine.commit_mechanical(instance_target("left"), MechanicalChange::Enable);
    let left_next = one_id(
        &engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(
        left_next, left_first,
        "disable/re-enable reused an identity"
    );
    let right_next = one_id(
        &engine.commit_mechanical(instance_target("right"), MechanicalChange::AddOpening),
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
            (|document: &ProjectDoc| mechanical_ids(document, "opening")) as IdentityList,
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
            (|document: &ProjectDoc| mechanical_ids(document, "vertex")) as IdentityList,
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
            (|document: &ProjectDoc| mechanical_ids(document, "fit")) as IdentityList,
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
            (|document: &ProjectDoc| mechanical_ids(document, "hardware")) as IdentityList,
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
            (|document: &ProjectDoc| mechanical_ids(document, "mount")) as IdentityList,
        ),
    ] {
        let mut document = unibody();
        mutate(document.mechanical.as_mut().unwrap());
        let mut engine = Session::default();
        assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
        engine.commit_mechanical(board_target(), add);
        let added = ids(&engine.snapshot()).pop().unwrap();
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
            (|document: &ProjectDoc| mechanical_ids(document, "opening")) as IdentityList,
        ),
        (
            "critical fit suffix",
            add_critical_fit(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.critical_fits = Some(vec![sparse_fit(&format!(
                    "case-critical-fit-{}",
                    u64::MAX - 1
                ))]);
            }) as fn(&mut MechanicalConfiguration),
            "case-critical-fit-",
            (|document: &ProjectDoc| mechanical_ids(document, "fit")) as IdentityList,
        ),
        (
            "hardware suffix",
            add_hardware(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.hardware = Some(vec![sparse_hardware(&format!(
                    "case-mechanical-hardware-{}",
                    u64::MAX - 1
                ))]);
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-hardware-",
            (|document: &ProjectDoc| mechanical_ids(document, "hardware")) as IdentityList,
        ),
        (
            "mount suffix",
            add_mount(),
            (|configuration: &mut MechanicalConfiguration| {
                configuration.mounts = vec![sparse_mount(&format!(
                    "case-mechanical-mount-{}",
                    u64::MAX - 1
                ))];
            }) as fn(&mut MechanicalConfiguration),
            "case-mechanical-mount-",
            (|document: &ProjectDoc| mechanical_ids(document, "mount")) as IdentityList,
        ),
    ] {
        let mut document = unibody();
        mutate(document.mechanical.as_mut().unwrap());
        let mut engine = Session::default();
        assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
        engine.commit_mechanical(board_target(), add.clone());
        let max = ids(&engine.snapshot()).pop().unwrap();
        assert_eq!(max, format!("{prefix}{}", u64::MAX), "{label}");
        let accepted = engine.snapshot();
        let refused = engine.commit_mechanical(board_target(), add);
        assert!(
            error(&refused).contains("exhausted"),
            "{label}: {refused:?}"
        );
        assert_eq!(engine.snapshot(), accepted, "{label} changed state");
    }

    // An imported counter advances MAX-1 -> MAX -> exhaustion.
    let mut document = unibody();
    document.mechanical.as_mut().unwrap().next_opening_identity = u64::MAX - 1;
    let mut engine = Session::default();
    assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
    engine.commit_mechanical(board_target(), MechanicalChange::AddOpening);
    assert_eq!(
        mechanical_ids(&engine.snapshot(), "opening"),
        [format!("opening-{}", u64::MAX - 1)]
    );
    let accepted = engine.snapshot();
    let refused = engine.commit_mechanical(board_target(), MechanicalChange::AddOpening);
    assert!(error(&refused).contains("exhausted"), "{refused:?}");
    assert_eq!(engine.snapshot(), accepted);

    // A live MAX suffix is refused rather than saturated.
    let mut document = unibody();
    document.mechanical.as_mut().unwrap().openings = Some(vec![MechanicalOpening {
        id: format!("opening-{}", u64::MAX),
        vertices: vec![],
        z: 0.0,
        height: 1.0,
        next_vertex_identity: 0,
    }]);
    let mut engine = Session::default();
    assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
    let refused = engine.commit_mechanical(board_target(), MechanicalChange::AddOpening);
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
    let mut engine = Session::default();
    assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
    engine.commit_mechanical(instance_target("left"), MechanicalChange::Disable);
    engine.commit_mechanical(instance_target("left"), MechanicalChange::Enable);
    let accepted = engine.snapshot();
    let refused = engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening);
    assert!(error(&refused).contains("exhausted"), "{refused:?}");
    assert_eq!(engine.snapshot(), accepted);
}

#[test]
fn failed_open_keeps_reservations_and_successful_open_starts_a_new_session() {
    let mut engine = Session::default();
    assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
    let first = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );

    // A failed explicit Open must not reset the document session.
    let mut duplicate = unibody();
    let part = fixture::part("dup", "missing", "SW1");
    duplicate.parts.push(part.clone());
    duplicate.parts.push(part);
    let reply = engine.send(CoreRequest::Open {
        id: "failed".into(),
        document: duplicate,
    });
    assert!(matches!(reply, CoreReply::Error { .. }));

    let second = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(second, first, "a failed Open reset the reservation");

    // A successful explicit Open is a new session: imported/live ids seed it.
    assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
    let reset = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
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
    let mut engine = Session::default();
    assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
    engine.commit_mechanical(instance_target("left"), MechanicalChange::Enable);
    let opening = one_id(
        &engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_eq!(opening, "opening-9");
}

#[test]
fn stale_setters_cannot_change_a_replacement_for_all_five_kinds() {
    let mut engine = Session::default();
    assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
    let opening_first = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    let vertex_first = one_id(
        &engine.commit_mechanical(
            board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: opening_first.clone(),
            },
        ),
        "vertex-",
    );
    engine.commit_mechanical(
        board_target(),
        MechanicalChange::RemoveOpeningVertex {
            opening_id: opening_first.clone(),
            vertex_id: vertex_first.clone(),
        },
    );
    engine.commit_mechanical(
        board_target(),
        MechanicalChange::RemoveOpening {
            opening_id: opening_first.clone(),
        },
    );
    let opening_second = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    let vertex_second = one_id(
        &engine.commit_mechanical(
            board_target(),
            MechanicalChange::AddOpeningVertex {
                opening_id: opening_second.clone(),
            },
        ),
        "vertex-",
    );
    assert_ne!(opening_second, opening_first);
    assert_ne!(vertex_second, vertex_first);

    let fit_first = replace_identity(
        &mut engine,
        "critical fit",
        add_critical_fit(),
        |document| mechanical_ids(document, "fit"),
    );
    let hardware_first = replace_identity(&mut engine, "hardware", add_hardware(), |document| {
        mechanical_ids(document, "hardware")
    });
    let mount_first = replace_identity(&mut engine, "mount", add_mount(), |document| {
        mechanical_ids(document, "mount")
    });

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
        let accepted = engine.snapshot();
        let refused = engine.commit_mechanical(board_target(), change);
        assert!(
            matches!(refused, CoreReply::Error { .. }),
            "{label}: {refused:?}"
        );
        assert_eq!(engine.snapshot(), accepted, "{label} changed state");
    }
}

#[test]
fn redo_and_undo_of_configuration_creation_keep_high_water_marks() {
    let mut engine = Session::default();
    assert!(matches!(engine.open(unibody()), CoreReply::Scene { .. }));
    let first = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    match engine.undo() {
        CoreReply::Scene { .. } => {}
        other => panic!("undo failed: {other:?}"),
    }
    match engine.redo() {
        CoreReply::Scene { .. } => {}
        other => panic!("redo failed: {other:?}"),
    }
    let after_redo = one_id(
        &engine.commit_mechanical(board_target(), MechanicalChange::AddOpening),
        "opening-",
    );
    assert_ne!(after_redo, first, "redo reused an identity");

    let mut engine = Session::default();
    assert!(matches!(
        engine.open(paired_owners()),
        CoreReply::Scene { .. }
    ));
    let left_first = one_id(
        &engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening),
        "opening-",
    );
    engine.commit_mechanical(instance_target("left"), MechanicalChange::Disable);
    match engine.undo() {
        CoreReply::Scene { .. } => {}
        other => panic!("undo failed: {other:?}"),
    }
    let after_undo = one_id(
        &engine.commit_mechanical(instance_target("left"), MechanicalChange::AddOpening),
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
        let mut engine = Session::default();
        assert!(matches!(engine.open(document), CoreReply::Scene { .. }));
        let accepted = engine.snapshot();
        let refused = engine.commit_mechanical(board_target(), add);
        assert!(
            error(&refused).contains("exhausted"),
            "{label}: {refused:?}"
        );
        assert_eq!(engine.snapshot(), accepted, "{label} changed state");
    }
}
