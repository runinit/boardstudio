use crate::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
use crate::{MirrorDestination, MirroredPair, project_document};

fn split_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("project", "Project");
    for id in ["source", "destination"] {
        doc.boards.push(Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
    }
    doc.parts.push(Part {
        id: "authored".into(),
        definition_id: "switch".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 2.0, y: 3.0 },
            rotation: 15.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    doc.boards[0].part_ids.push("authored".into());
    doc.mirrored_pairs.push(MirroredPair {
        id: "split".into(),
        source: MirrorDestination::SecondBoard("source".into()),
        destination: MirrorDestination::SecondBoard("destination".into()),
        mapping: Pose2 {
            at: Vec2 { x: 10.0, y: 0.0 },
            rotation: 0.0,
        },
        overrides: vec![],
    });

    doc
}

#[test]
fn board_projection_is_repeatable_and_keeps_source_canonical() {
    let doc = split_document();
    let resolved = project_document(&doc);
    assert_eq!(resolved, project_document(&doc));
    let reflected = &resolved.geometry().parts[1];
    assert_eq!(reflected.pose.at, Vec2 { x: 8.0, y: 3.0 });
    assert_eq!(reflected.pose.rotation, -15.0);
    assert_eq!(
        resolved.geometry().boards[1].part_ids,
        [reflected.id.clone()]
    );
    assert_eq!(doc.parts.len(), 1);
    assert!(doc.boards[1].part_ids.is_empty());
}

#[test]
fn layout_projection_local_omission_rejoin_and_unlink_use_the_same_geometry() {
    use crate::model::{HalfExceptionState, Layout};
    use crate::{set_exception, unlink_pair};

    let mut doc = split_document();
    doc.boards.pop();
    for id in ["left", "right"] {
        doc.layouts.push(Layout {
            id: id.into(),
            name: id.into(),
            board_id: "source".into(),
            matrix_ids: vec![],
            part_ids: if id == "left" {
                vec!["authored".into()]
            } else {
                vec![]
            },
            mirror_link: None,
        });
    }
    let pair = &mut doc.mirrored_pairs[0];
    pair.source = MirrorDestination::SameBoardLayout("left".into());
    pair.destination = MirrorDestination::SameBoardLayout("right".into());
    let target = "5/split/part/authored";
    let expected = project_document(&doc).geometry().parts[1].clone();
    assert_eq!(expected.id, target);
    set_exception(&mut doc, "split", target, HalfExceptionState::Omitted).unwrap();
    assert_eq!(project_document(&doc).geometry().parts.len(), 1);
    set_exception(&mut doc, "split", target, HalfExceptionState::Linked).unwrap();
    assert_eq!(project_document(&doc).geometry().parts[1], expected);
    set_exception(&mut doc, "split", target, HalfExceptionState::Local).unwrap();
    doc.parts[0].pose.at.x = 40.0;
    assert_eq!(project_document(&doc).geometry().parts[1], expected);
    doc.parts.clear();
    doc.layouts[0].part_ids.clear();
    doc.boards[0].part_ids.clear();
    assert_eq!(project_document(&doc).geometry().parts[0], expected);
    unlink_pair(&mut doc, "split").unwrap();
    assert!(doc.mirrored_pairs.is_empty());
    assert_eq!(doc.parts, [expected]);
    assert_eq!(doc.layouts[1].part_ids, [target]);
}

#[test]
fn reversible_split_shares_its_board_without_projecting_extra_parts() {
    use crate::create_half;
    use crate::model::{HardwareConfiguration, MirroredHalfMode, PhysicalBoardInstance};

    let mut doc = split_document();
    doc.mirrored_pairs.clear();
    doc.boards.pop();
    doc.hardware = Some(HardwareConfiguration {
        instances: vec![PhysicalBoardInstance {
            id: "left".into(),
            name: "Left".into(),
            board_id: "source".into(),
            half: "left".into(),
            role: "primary".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: None,
            construction_ownership: Default::default(),
            mechanical_identity_reservation: None,
        }],
        ..Default::default()
    });
    create_half(
        &mut doc,
        "split",
        "left",
        &MirroredHalfMode::ReuseBoard {
            instance_id: "right".into(),
        },
        None,
        None,
        Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
    )
    .unwrap();

    let resolved = project_document(&doc);
    let geometry = resolved.geometry();
    assert_eq!(geometry.parts.len(), 1);
    assert_eq!(geometry.boards.len(), 1);
    let instances = &geometry.hardware.as_ref().unwrap().instances;
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].board_id, instances[1].board_id);
    assert_ne!(instances[0].flipped, instances[1].flipped);
}
