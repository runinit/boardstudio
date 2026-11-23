use super::*;
use crate::test_support::*;

#[test]
fn disabled_mechanical_configuration_has_no_stack() {
    let doc = ProjectDoc::empty("test", "test");
    assert!(
        serde_json::to_value(&doc)
            .unwrap()
            .get("mechanical")
            .is_none()
    );
    assert!(resolve(&doc, &[]).stack.is_empty());
}
#[test]
fn stack_is_explicit_and_revision_tagged() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.revision = 7;
    doc.boards.push(board("board", vec![], 1.6));
    doc.mechanical = Some(config());
    let result = resolve(&doc, &[]);
    assert_eq!(result.revision, 7);
    assert_eq!(
        result.stack.iter().find(|l| l.id == "bottom").unwrap().z,
        -4.1
    );
    assert_eq!(result.case.bodies[0].contours, result.plate_contours);
}
#[test]
fn custom_dimensions_and_disabled_foam_survive_normalization() {
    let mut c = config();
    c.plate_thickness = 1.8;
    c.plate_foam_thickness = 0.0;
    c.bottom_foam_thickness = 0.0;
    c.bottom_thickness = 4.2;
    c.clearance = 0.45;
    let processes = normalized_processes(&c);
    assert_eq!(c.plate_thickness, 1.8);
    assert_eq!(c.bottom_thickness, 4.2);
    assert_eq!(c.clearance, 0.45);
    assert_eq!(
        processes
            .iter()
            .find(|item| item.part_id == "plate-foam")
            .unwrap()
            .thickness,
        0.0
    );
    assert_eq!(
        processes
            .iter()
            .find(|item| item.part_id == "bottom-foam")
            .unwrap()
            .thickness,
        0.0
    );
}
#[test]
fn no_model_foam_fallbacks_transform_union_and_warn() {
    let mut doc = ProjectDoc::empty("foam", "foam");
    doc.boards.push(board(
        "board",
        vec!["front-a".into(), "front-b".into(), "back".into()],
        1.6,
    ));
    doc.parts.extend([
        serde_json::from_value(serde_json::json!({
            "id":"front-a", "definitionId":"courtyard-part", "reference":"U1",
            "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"front-b", "definitionId":"courtyard-part", "reference":"U2",
            "pose":{"at":{"x":23,"y":20},"rotation":0}, "side":"front"
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"back", "definitionId":"pad-only-part", "reference":"J1",
            "pose":{"at":{"x":70,"y":70},"rotation":0}, "side":"back"
        }))
        .unwrap(),
    ]);
    doc.definitions.extend([
        serde_json::from_value(serde_json::json!({
            "id":"courtyard-part", "name":"No model front component", "kind":"custom",
            "courtyard":[{"x":-4,"y":-3},{"x":4,"y":-3},{"x":4,"y":3},{"x":-4,"y":3}],
            "pads":[]
        }))
        .unwrap(),
        serde_json::from_value(serde_json::json!({
            "id":"pad-only-part", "name":"No model back component", "kind":"connector",
            "courtyard":[], "pads":[{"id":"1","number":"1","at":{"x":1,"y":0},
            "size":{"x":4,"y":2},"shape":"rect","rotation":90}]
        }))
        .unwrap(),
    ]);
    let mut configuration = config();
    configuration.clearance = 0.3;
    doc.mechanical = Some(configuration);
    let outline = [rectangle(0.0, 0.0, 100.0, 100.0)];

    let assembly = resolve(&doc, &outline);
    let plate_foam = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "plate-foam")
        .unwrap();
    let bottom_foam = assembly
        .case
        .bodies
        .iter()
        .find(|body| body.body.id == "bottom-foam")
        .unwrap();
    let hole_bounds = |body: &CaseIR| {
        body.contours
            .iter()
            .filter(|contour| contour.hole)
            .map(|contour| {
                let min_x = contour
                    .points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::INFINITY, f64::min);
                let max_x = contour
                    .points
                    .iter()
                    .map(|point| point.x)
                    .fold(f64::NEG_INFINITY, f64::max);
                (min_x, max_x)
            })
            .collect::<Vec<_>>()
    };
    let front_holes = hole_bounds(plate_foam);
    let back_holes = hole_bounds(bottom_foam);
    assert_eq!(
        front_holes.len(),
        1,
        "overlapping courtyard clearances are unioned"
    );
    assert!((front_holes[0].0 - 15.7).abs() < 0.01);
    assert!((front_holes[0].1 - 27.3).abs() < 0.01);
    assert_eq!(back_holes.len(), 1, "back-side pads clear only bottom foam");
    assert!((back_holes[0].0 - 67.7).abs() < 0.01);
    assert!((back_holes[0].1 - 70.3).abs() < 0.01);
    assert!(assembly.diagnostics.iter().any(|finding| {
        finding.id == "mechanical:foam-approximation:front-a"
            && finding.severity == Severity::Warning
    }));
    assert!(!assembly.generation_blocked);
}
#[test]
fn integrated_frame_uses_lid_and_separate_bottom() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(board("board", vec![], 1.6));
    let mut c = config();
    c.integrated_plate_frame = true;
    c.mount = MechanicalMount::Rigid;
    doc.mechanical = Some(c);
    let resolved = resolve(&doc, &[rectangle(0.0, 0.0, 50.0, 50.0)]);
    let plate = &resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "plate")
        .unwrap()
        .body;
    assert_eq!(plate.kind, CaseKind::Lid);
    assert!((plate.z.unwrap() + plate.wall_height.unwrap() - 3.5).abs() < 1e-9);
    assert_eq!(
        resolved
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "bottom")
            .unwrap()
            .body
            .kind,
        CaseKind::Plate
    );
}
#[test]
fn gasket_mode_places_discrete_supports_and_retaining_frame_from_defaults() {
    let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
    doc.boards.push(board("board", vec![], 1.6));
    let mut settings = config();
    settings.mount = MechanicalMount::Gasket;
    doc.mechanical = Some(settings);
    let contours = vec![rectangle(0., 0., 130., 90.)];
    let result = resolve(&doc, &contours);
    assert!(!result.generation_blocked, "{:?}", result.diagnostics);
    assert!(result.case.bodies.iter().any(|b| b.body.id == "retainer"));
    assert_eq!(
        result
            .case
            .bodies
            .iter()
            .filter(|b| b.body.id.starts_with("gasket:"))
            .count(),
        12
    );
}

#[test]
fn manual_gasket_anchors_are_exact_persistent_and_invalidated_by_outline_changes() {
    let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
    doc.boards.push(board("board", vec![], 1.6));
    let mut settings = config();
    settings.mount = MechanicalMount::Gasket;
    doc.mechanical = Some(settings);
    let mut contours = vec![rectangle(0., 0., 130., 90.)];
    let first = resolve(&doc, &contours);
    let support = &first.gasket_supports[0];
    let anchor = support.anchor + 0.0007;
    doc.mechanical.as_mut().unwrap().gasket_layout = Some(MechanicalGasketLayout {
        auto_size: None,
        adhesive_thickness: None,
        minimum_foam_thickness: None,
        preset_id: None,
        material: None,
        length: 12.,
        width: 3.,
        thickness: 2.,
        compression: 0.15,
        supports: vec![MechanicalGasketAnchor {
            length: None,
            width: None,
            placement: None,
            id: support.id.clone(),
            region_id: support.region_id.clone(),
            outline_key: support.outline_key.clone(),
            anchor,
            unlinked: false,
        }],
    });
    let moved = resolve(&doc, &contours);
    assert!(!moved.generation_blocked, "{:?}", moved.diagnostics);
    assert!((moved.gasket_supports[0].anchor - anchor).abs() < 1e-9);
    let saved = serde_json::to_string(&doc).unwrap();
    let reopened: ProjectDoc = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        moved.gasket_supports,
        resolve(&reopened, &contours).gasket_supports
    );
    contours[0].points[1].x += 1.;
    assert!(resolve(&doc, &contours).generation_blocked);
}

#[test]
fn gasket_support_reaches_plate_without_rigid_contact() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(board("board", vec![], 1.6));
    let mut c = config();
    c.mount = MechanicalMount::Gasket;
    c.gasket_travel = Some(0.5);
    c.gasket = Some(Gasket {
        inset: 0.2,
        width: 1.0,
        depth: 0.4,
    });
    doc.mechanical = Some(c);
    let resolved = resolve(&doc, &[rectangle(0.0, 0.0, 50.0, 50.0)]);
    let bottom = &resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "bottom")
        .unwrap()
        .body;
    let strips = resolved
        .case
        .bodies
        .iter()
        .filter(|b| b.body.id.starts_with("gasket:"))
        .collect::<Vec<_>>();
    assert_eq!(strips.len(), 12);
    for strip in strips {
        if strip.body.id.ends_with(":lower") {
            assert!((strip.body.z.unwrap() + strip.body.thickness - 3.5).abs() < 1e-9);
        } else {
            assert!((strip.body.z.unwrap() - 5.0).abs() < 1e-9);
        }
    }
    let retainer = resolved
        .case
        .bodies
        .iter()
        .find(|b| b.body.id == "retainer")
        .unwrap();
    assert!((bottom.z.unwrap() + bottom.thickness - retainer.body.z.unwrap()).abs() < 1e-9);
    assert!(
        bottom
            .mounts
            .as_ref()
            .unwrap()
            .iter()
            .all(|m| m.id.starts_with("closure:"))
    );
    assert!(
        bottom
            .openings
            .as_ref()
            .unwrap()
            .iter()
            .any(|o| (o.z - 1.8).abs() < 1e-9)
    );
}
#[test]
fn cable_exit_regenerates_single_foam_envelope_without_mutation() {
    let mut doc = ProjectDoc::empty("test", "test");
    doc.boards.push(board("board", vec![], 1.6));
    let mut c = config();
    c.battery = Some(MechanicalBattery {
        size: Vec3 {
            x: 8.0,
            y: 8.0,
            z: 3.0,
        },
        at: Vec2 { x: 20.0, y: 20.0 },
        cable_exit: Vec2 { x: 35.0, y: 20.0 },
        cable_width: Some(2.0),
    });
    doc.mechanical = Some(c);
    let outline = [rectangle(0.0, 0.0, 50.0, 50.0)];
    let saved = doc.clone();
    let first = resolve(&doc, &outline);
    assert_eq!(doc, saved);
    doc.mechanical
        .as_mut()
        .unwrap()
        .battery
        .as_mut()
        .unwrap()
        .cable_exit
        .y = 35.0;
    let second = resolve(&doc, &outline);
    let foam = |assembly: MechanicalAssembly| {
        assembly
            .case
            .bodies
            .into_iter()
            .find(|body| body.body.id == "bottom-foam")
            .unwrap()
            .contours
    };
    let a = foam(first);
    let b = foam(second);
    assert_ne!(a, b);
    assert_eq!(a.iter().filter(|contour| contour.hole).count(), 1);
    assert_eq!(b.iter().filter(|contour| contour.hole).count(), 1);
}
#[test]
fn rejects_impossible_foam_and_rigid_gasket() {
    let foam = "mechanical:foam-engagement";
    let frame = "mechanical:gasket-rigid-frame";
    for (thickness, integrated, expected) in [
        (4.0, false, &[foam][..]),
        (1.0, true, &[frame][..]),
        (4.0, true, &[foam, frame][..]),
    ] {
        let mut doc = ProjectDoc::empty("test", "test");
        let mut c = config();
        c.plate_foam_thickness = thickness;
        c.mount = MechanicalMount::Gasket;
        c.integrated_plate_frame = integrated;
        doc.mechanical = Some(c);
        let result = resolve(&doc, &[]);
        for id in [foam, frame] {
            assert_eq!(
                result.diagnostics.iter().any(|d| d.id == id),
                expected.contains(&id),
                "foam {thickness}, integrated {integrated}: {id}: {:?}",
                result.diagnostics
            );
        }
    }
}

#[test]
fn mechanical_defaults_have_their_reference_values() {
    use MechanicalSwitchFamily::{ChocV1, ChocV2, Mx};
    for (family, datum, thickness) in [(Mx, 5.0, 1.5), (ChocV1, 3.5, 1.3), (ChocV2, 5.0, 1.5)] {
        assert_eq!(mounting_datum(family), datum, "{family:?}");
        assert_eq!(default_plate_thickness(family), thickness, "{family:?}");
    }
    assert_eq!(default_plate_foam_thickness(3.5), 3.0);
    assert_eq!(default_plate_foam_thickness(0.0), 0.0);
    assert_eq!(default_material("plate", &PlateMethod::Printed), "PLA");
    assert_eq!(default_material("plate-foam", &PlateMethod::Printed), "EVA");
    let process = default_layer_process("plate-foam", PlateMethod::Printed, 2.0);
    assert_eq!(process.method, PlateMethod::CutSheet);
    assert_eq!(process.material, "EVA");
    assert_eq!(process.thickness, 2.0);
    let layout = default_gasket_layout();
    assert_eq!(layout.length, 80.0);
    assert_eq!(layout.width, 3.0);
    assert_eq!(layout.compression, 0.15);
    assert!(layout.supports.is_empty());
    let gasket = default_internal_gasket();
    assert_eq!(gasket.version, GasketConstructionVersion::InternalV1);
    assert_eq!(gasket.hardware.id, "custom-m2");
    assert_eq!(
        gasket.hardware.screw_lengths,
        vec![8.0, 10.0, 12.0, 14.0, 15.0, 16.0]
    );
    assert_eq!(gasket.hardware.drive, ScrewDrive::Hex);
    assert_eq!(gasket.hardware.installation, InsertInstallation::HeatSet);
    assert_eq!(gasket.hardware.head_profile, ScrewHeadProfile::Flat);
    assert_eq!(gasket.hardware.length_datum, ScrewLengthDatum::UnderHead);
}
