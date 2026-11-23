use super::*;
use crate::test_support::*;

#[test]
fn duplicate_rings_ignore_winding_start_and_closure() {
    let a = Contour {
        hole: true,
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
        ],
    };
    let b = Contour {
        hole: true,
        points: vec![a.points[1], a.points[0], a.points[2], a.points[1]],
    };
    assert!(same_ring(&a, &b));
}
#[test]
fn profiles_follow_rotation_and_back_side_mirror() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(board("board", vec!["key".into()], 1.6));
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":10,"y":20},"rotation":90},"side":"back"})).unwrap());
    let mut configuration = config();
    configuration.profiles.push(MechanicalPartProfile {
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        switch_family: None,
        definition_id: "switch".into(),
        source: "custom".into(),
        cutouts: vec![vec![
            Vec2 { x: 1.0, y: 0.0 },
            Vec2 { x: 2.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
        ]],
        plate_to_pcb: 3.5,
        supported_thickness: None,
    });
    doc.mechanical = Some(configuration);
    for (side, rotation, x, y) in [
        (Side::Back, 90.0, 10.0, 19.0),
        (Side::Front, 90.0, 10.0, 21.0),
        (Side::Front, 0.0, 11.0, 20.0),
        (Side::Back, 0.0, 9.0, 20.0),
    ] {
        doc.parts[0].side = side;
        doc.parts[0].pose.rotation = rotation;
        let resolved = resolve(&doc, &[]);
        let point = resolved.plate_contours[0].points[0];
        assert!(
            (point.x - x).abs() < 1e-9 && (point.y - y).abs() < 1e-9,
            "{:?} at {rotation} degrees: {point:?}, expected ({x}, {y})",
            doc.parts[0].side
        );
    }
}
#[test]
fn combined_switch_stabilizer_deduplicates_and_regenerates_with_pose() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(board("board", vec!["key".into()], 1.6));
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":30,"y":30},"rotation":0},"side":"front"})).unwrap());
    let mut c = config();
    let mut profile =
        builtin_profile("switch".into(), MechanicalBuiltinProfile::MxSwitch, 3.5).unwrap();
    profile.cutouts.extend(
        builtin_profile("switch".into(), MechanicalBuiltinProfile::MxStab2u, 3.5)
            .unwrap()
            .cutouts,
    );
    let expected = profile.cutouts.len();
    c.profiles.push(profile);
    c.stabilizers = Some(vec![MechanicalStabilizerOverride {
        part_id: "key".into(),
        kind: MechanicalStabilizerKind::PcbMount,
        units: 2.0,
        rotation: None,
        profile: None,
    }]);
    doc.mechanical = Some(c);
    let first = resolve(&doc, &[]);
    assert_eq!(first.plate_contours.len(), expected);
    doc.parts[0].pose.at.x += 10.0;
    doc.revision += 1;
    let second = resolve(&doc, &[]);
    assert_eq!(second.plate_contours.len(), expected);
    assert!(
        (second.plate_contours[0].points[0].x - first.plate_contours[0].points[0].x - 10.0).abs()
            < 1e-9
    );
    assert_eq!(second.revision, 1);
}
#[test]
fn outer_expansion_preserves_nominal_holes() {
    let outer = rectangle(0.0, 0.0, 20.0, 20.0);
    let hole = Contour {
        hole: true,
        points: vec![
            Vec2 { x: 5.0, y: 5.0 },
            Vec2 { x: 5.0, y: 10.0 },
            Vec2 { x: 10.0, y: 10.0 },
            Vec2 { x: 10.0, y: 5.0 },
        ],
    };
    let expanded = expanded_outline(&[outer, hole.clone()], 2.0, 1).unwrap();
    assert!(expanded.contains(&hole));
    assert!(
        expanded
            .iter()
            .filter(|c| !c.hole)
            .flat_map(|c| c.points.iter())
            .any(|p| p.x < 0.0)
    );
}
