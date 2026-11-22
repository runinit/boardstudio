use crate::model::{Board, Part, Pose2, ProjectDoc, Side, Vec2};
use crate::{MirrorDestination, MirroredPair, project_document};

#[test]
fn board_projection_is_repeatable_and_keeps_source_canonical() {
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
