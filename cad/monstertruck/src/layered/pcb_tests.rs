use boardstudio_contracts::PreparedCaseIR;
use monstertruck_meshing::{prelude::*, tessellation::shell_to_polygon_strict};
use monstertruck_modeling::{InnerSpace, Point3, Solid, Surface};

#[test]
fn source_circle_metadata_is_authoritative_through_public_build_case() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    let mut polygon_only = input.clone();
    polygon_only.body.pcb_source = None;
    let legacy = crate::build_case(&polygon_only).expect("unchanged polygon-only input");
    let legacy_mesh = shell_to_polygon_strict(&legacy.solids[0].boundaries()[0], 1e-5).unwrap();
    assert!(
        (legacy_mesh.volume() - 16.).abs() < 1e-10,
        "diamond control"
    );

    let model = crate::build_case(&input).expect("authored circular PCB source");
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        step.matches("CYLINDRICAL_SURFACE(").count(),
        1,
        "source metadata must replace the diamond compatibility projection"
    );
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).expect("closed oriented native solid");
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), 3);
    assert_eq!(
        solid.boundaries()[0]
            .iter()
            .filter(|face| matches!(face.surface(), Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-5).unwrap();
    let expected = 8. * std::f64::consts::PI;
    assert!((mesh.volume() - expected).abs() < 0.002);
    let buffers = crate::mesh_profile(&model).expect("complete existing mesh path");
    assert!(!buffers.positions.is_empty());
    assert_eq!(buffers.positions.len(), buffers.normals.len());
    assert_eq!(buffers.positions.len() % 9, 0);
    assert!(
        buffers
            .positions
            .iter()
            .chain(&buffers.normals)
            .all(|x| x.is_finite())
    );

    native_cylinder_facts(&step, [10., 8.], 2., [0., 2.]);
    for (point, inside) in [
        ([11.4, 9.4, 1.], true),
        ([12.2, 8., 1.], false),
        ([10., 8., 2.2], false),
    ] {
        let point = Point3::from(point);
        let angle: f64 = mesh
            .faces()
            .triangle_iter()
            .map(|face| {
                let [a, b, c] = face.map(|v| mesh.positions()[v.pos] - point);
                let denominator = a.magnitude() * b.magnitude() * c.magnitude()
                    + a.dot(b) * c.magnitude()
                    + b.dot(c) * a.magnitude()
                    + c.dot(a) * b.magnitude();
                2. * a.dot(b.cross(c)).atan2(denominator)
            })
            .sum();
        let expected = if inside { 1. } else { 0. };
        assert!((angle / (4. * std::f64::consts::PI) - expected).abs() < 1e-8);
    }
    let certificate = crate::step_accuracy::certify_circular_extrusion(
        &step,
        crate::step_accuracy::CircularExtrusion {
            center: [10., 8.],
            radius: 2.,
            z: [0., 2.],
        },
    )
    .expect("independent raw STEP source-circle certificate");
    assert!(certificate.volume[0] <= expected && certificate.volume[1] >= expected);
    assert!(certificate.volume[1] - certificate.volume[0] < 0.002);
    assert_eq!(certificate.bounds, [[8., 6., 0.], [12., 10., 2.]]);
    assert_eq!(certificate.topology, [4, 4, 3]);
    assert!(certificate.bound_error.is_finite() && certificate.bound_error >= 0.);
    for (point, material) in [
        ([11.4, 9.4, 1.], true),
        ([12.2, 8., 1.], false),
        ([10., 8., 2.2], false),
    ] {
        assert_eq!(certificate.contains(point), Some(material));
    }
    assert_eq!(certificate.contains([12., 8., 1.]), None);
    assert_eq!(certificate.contains([10., 8., 0.]), None);
    // Generic inspect retains its planar scope; the circular certificate above proves this source.
    assert!(matches!(
        crate::step_accuracy::inspect(&step),
        Err(crate::step_accuracy::Error::Unsupported)
    ));
    if let Some(directory) = std::env::var_os("PCB_CONSUMER_EVIDENCE_OUTPUT") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("source-circle.step"), step.as_bytes()).unwrap();
        std::fs::write(
            directory.join("source-circle.json"),
            serde_json::to_vec_pretty(&input).unwrap(),
        )
        .unwrap();
    }
}

fn native_cylinder_facts(text: &str, center: [f64; 2], radius: f64, z: [f64; 2]) {
    use std::collections::BTreeMap;
    use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};
    fn list(value: &Parameter) -> &[Parameter] {
        match value {
            Parameter::List(values) => values,
            _ => panic!("Part21 list"),
        }
    }
    fn id(value: &Parameter) -> u64 {
        match value {
            Parameter::Ref(Name::Entity(id)) => *id,
            _ => panic!("Part21 reference"),
        }
    }
    fn number(value: &Parameter) -> f64 {
        match value {
            Parameter::Real(value) => *value,
            Parameter::Integer(value) => *value as f64,
            _ => panic!("Part21 number"),
        }
    }
    fn args<'a>(records: &'a BTreeMap<u64, Record>, entity: u64, name: &str) -> &'a [Parameter] {
        let record = &records[&entity];
        assert_eq!(record.name.as_str(), name);
        list(&record.parameter)
    }
    fn point(records: &BTreeMap<u64, Record>, entity: u64, name: &str) -> [f64; 3] {
        let coordinates = list(&args(records, entity, name)[1]);
        assert_eq!(coordinates.len(), 3);
        std::array::from_fn(|index| number(&coordinates[index]))
    }
    let exchange: Exchange = text.parse().unwrap();
    let records: BTreeMap<_, _> = exchange
        .data
        .into_iter()
        .flat_map(|data| data.entities)
        .filter_map(|entity| match entity {
            EntityInstance::Simple { id, record } => Some((id, record)),
            _ => None,
        })
        .collect();
    let cylinders: Vec<_> = records
        .iter()
        .filter(|(_, record)| record.name.as_str() == "CYLINDRICAL_SURFACE")
        .collect();
    assert_eq!(cylinders.len(), 1);
    let (&surface, cylinder) = cylinders[0];
    let cylinder = list(&cylinder.parameter);
    assert_eq!(
        number(&cylinder[2]).to_bits(),
        radius.to_bits(),
        "authored radius survives unchanged"
    );
    let placement = args(&records, id(&cylinder[1]), "AXIS2_PLACEMENT_3D");
    let origin = point(&records, id(&placement[1]), "CARTESIAN_POINT");
    assert_eq!([origin[0], origin[1]], center);
    let axis = point(&records, id(&placement[2]), "DIRECTION");
    assert_eq!(axis, [0., 0., 1.]);
    let faces: Vec<_> = records
        .values()
        .filter(|record| record.name.as_str() == "ADVANCED_FACE")
        .collect();
    assert_eq!(faces.len(), 3);
    let side = faces
        .iter()
        .filter(|record| id(&list(&record.parameter)[2]) == surface)
        .collect::<Vec<_>>();
    assert_eq!(side.len(), 1);
    assert_eq!(
        list(&list(&side[0].parameter)[1]).len(),
        2,
        "native periodic side has two circular boundaries"
    );
    let mut cap_z: Vec<_> = records
        .values()
        .filter(|record| record.name.as_str() == "PLANE")
        .map(|record| {
            let placement = args(
                &records,
                id(&list(&record.parameter)[1]),
                "AXIS2_PLACEMENT_3D",
            );
            point(&records, id(&placement[1]), "CARTESIAN_POINT")[2]
        })
        .collect();
    cap_z.sort_by(f64::total_cmp);
    assert_eq!(cap_z, z);
}

#[test]
fn source_line_arc_loop_is_authoritative_through_public_build_case() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/line-arc-tracer.json"
    ))
    .unwrap();
    let model = crate::build_case(&input).expect("authored line/arc half disc");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).expect("closed oriented half disc");
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), 4);
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-5).unwrap();
    assert!((mesh.volume() - 4. * std::f64::consts::PI).abs() < 0.002);
    material(&mesh, [1.5, 0.5, 1.], true);
    material(&mesh, [-0.2, 0., 1.], false);
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 1);
    assert_eq!(step.matches("PLANE(").count(), 3);
    let buffers = crate::mesh_profile(&model).unwrap();
    assert!(!buffers.positions.is_empty());
    assert_eq!(buffers.positions.len(), buffers.normals.len());
    assert!(
        buffers
            .positions
            .iter()
            .chain(&buffers.normals)
            .all(|x| x.is_finite())
    );
}

#[test]
fn source_body_round_drills_do_not_use_polygon_region_filtering() {
    use boardstudio_contracts::{Mount, MountKind, Vec2};
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    input.body.mounts = Some(vec![Mount {
        id: "drill-outside-diamond".into(),
        at: Vec2 { x: 11.3, y: 9.3 },
        kind: MountKind::Hole,
        hole_diameter: 0.2,
        boss_diameter: None,
        height: None,
    }]);
    assert!(input.regions[0].mounts.is_empty());
    let model = crate::build_case(&input).expect("complete authored body drill inventory");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).unwrap();
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), 4);
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-5).unwrap();
    assert!((mesh.volume() - 7.98 * std::f64::consts::PI).abs() < 0.002);
    material(&mesh, [11.3, 9.3, 1.], false);
    material(&mesh, [11.15, 9.3, 1.], true);
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(
        step.matches("CYLINDRICAL_SURFACE(").count(),
        2,
        "one outer cylinder and one authored through-hole cylinder"
    );
    assert_eq!(step.matches("PLANE(").count(), 2);
}

#[test]
fn source_vik_splitter_public_prepare_fixture_preserves_all_authored_geometry() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/vik-splitter-prepared.json"
    ))
    .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/vik-splitter-expectations.json"
    ))
    .unwrap();
    let decimal = |value: &serde_json::Value| value.as_str().unwrap().parse::<f64>().unwrap();
    prepared_source_bits(&input);
    let source = input.body.pcb_source.as_ref().unwrap();
    assert_eq!(source.contours.len(), 1);
    assert_eq!(source.contours[0].primitives.len(), 8);
    let drills = input.body.mounts.as_ref().unwrap();
    assert_eq!(drills.len(), 38);
    let mut expected_cylinders = Vec::new();
    for primitive in &source.contours[0].primitives {
        if let Some(support) = &primitive.curve_support {
            let expected = oracle["arc_supports"]
                .as_array()
                .unwrap()
                .iter()
                .find(|value| value["id"].as_str() == Some(primitive.source_id.as_str()))
                .unwrap();
            assert!((support.radius_mm - decimal(&expected["radius_mm"])).abs() < 5e-12);
            assert!(
                (support.center.x - (decimal(&expected["center_kicad"][0]) - 81.584194)).abs()
                    < 5e-12
            );
            assert!(
                (support.center.y - (110.727271 - decimal(&expected["center_kicad"][1]))).abs()
                    < 5e-12
            );
            expected_cylinders.push(([support.center.x, support.center.y], support.radius_mm));
        }
    }
    assert_eq!(expected_cylinders.len(), 4);
    for expected in oracle["round_drills"].as_array().unwrap() {
        let drill = drills
            .iter()
            .find(|drill| Some(drill.id.as_str()) == expected["source_id"].as_str())
            .unwrap();
        assert!(
            (drill.at.x - (decimal(&expected["center_kicad_mm"][0]) - 81.584194)).abs() < 5e-12
        );
        assert!(
            (drill.at.y - (110.727271 - decimal(&expected["center_kicad_mm"][1]))).abs() < 5e-12
        );
        assert_eq!(
            (drill.hole_diameter / 2.).to_bits(),
            decimal(&expected["radius_mm"]).to_bits()
        );
        expected_cylinders.push(([drill.at.x, drill.at.y], drill.hole_diameter / 2.));
    }
    let model = crate::build_case(&input).expect("actual Core PrepareCase VIK source fixture");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).expect("closed oriented drilled PCB");
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), 48); // 42 cylinders, four walls, two caps.
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).unwrap();
    assert!(
        (mesh.volume() - decimal(&oracle["volume_mm3"])).abs() < 0.02,
        "independent source volume: {}",
        mesh.volume()
    );
    for drill in drills {
        material(&mesh, [drill.at.x, drill.at.y, 0.8], false);
    }
    material(&mesh, [0., 0., 0.8], true);
    material(&mesh, [13., 0., 0.8], false);
    material(&mesh, [0., 0., 1.7], false);
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for point in mesh.positions() {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(point[axis]);
            bounds[1][axis] = bounds[1][axis].max(point[axis]);
        }
    }
    for (actual, expected) in bounds
        .into_iter()
        .flatten()
        .zip([-12.5, -12.515, 0., 12.5, 12.515, 1.6])
    {
        assert!(
            (actual - expected).abs() < 1e-8,
            "placed source bounds {actual} vs {expected}"
        );
    }
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
    assert_eq!(step.matches("CYLINDRICAL_SURFACE(").count(), 42);
    assert_eq!(step.matches("PLANE(").count(), 6);
    let actual_cylinders = raw_cylinders(&step);
    for expected in expected_cylinders {
        assert_eq!(
            actual_cylinders
                .iter()
                .filter(|actual| **actual == expected)
                .count(),
            1,
            "each authored support has exactly one native cylindrical face: {expected:?}"
        );
    }
    let buffers = crate::mesh_profile(&model).unwrap();
    assert!(!buffers.positions.is_empty());
    assert_eq!(buffers.positions.len(), buffers.normals.len());
    assert_eq!(buffers.positions.len() % 9, 0);
    assert!(
        buffers
            .positions
            .iter()
            .chain(&buffers.normals)
            .all(|value| value.is_finite())
    );
    if let Some(directory) = std::env::var_os("PCB_CONSUMER_EVIDENCE_OUTPUT") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("vik-splitter.step"), step.as_bytes()).unwrap();
        std::fs::write(
            directory.join("vik-splitter.json"),
            serde_json::to_vec_pretty(&input).unwrap(),
        )
        .unwrap();
    }
}

fn material(mesh: &monstertruck_meshing::prelude::PolygonMesh, point: [f64; 3], inside: bool) {
    let point = Point3::from(point);
    let angle: f64 = mesh
        .faces()
        .triangle_iter()
        .map(|face| {
            let [a, b, c] = face.map(|v| mesh.positions()[v.pos] - point);
            let denominator = a.magnitude() * b.magnitude() * c.magnitude()
                + a.dot(b) * c.magnitude()
                + b.dot(c) * a.magnitude()
                + c.dot(a) * b.magnitude();
            2. * a.dot(b.cross(c)).atan2(denominator)
        })
        .sum();
    let expected = if inside { 1. } else { 0. };
    assert!(
        (angle / (4. * std::f64::consts::PI) - expected).abs() < 1e-8,
        "material {point:?}"
    );
}

fn raw_cylinders(text: &str) -> Vec<([f64; 2], f64)> {
    use std::collections::BTreeMap;
    use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};
    fn list(value: &Parameter) -> &[Parameter] {
        match value {
            Parameter::List(values) => values,
            _ => panic!("Part21 list"),
        }
    }
    fn id(value: &Parameter) -> u64 {
        match value {
            Parameter::Ref(Name::Entity(id)) => *id,
            _ => panic!("Part21 ref"),
        }
    }
    fn number(value: &Parameter) -> f64 {
        match value {
            Parameter::Real(value) => *value,
            Parameter::Integer(value) => *value as f64,
            _ => panic!("Part21 number"),
        }
    }
    let exchange: Exchange = text.parse().unwrap();
    let records: BTreeMap<u64, Record> = exchange
        .data
        .into_iter()
        .flat_map(|data| data.entities)
        .filter_map(|entity| match entity {
            EntityInstance::Simple { id, record } => Some((id, record)),
            _ => None,
        })
        .collect();
    records
        .iter()
        .filter(|(_, record)| record.name.as_str() == "CYLINDRICAL_SURFACE")
        .map(|(surface, cylinder)| {
            assert_eq!(
                records
                    .values()
                    .filter(|record| record.name.as_str() == "ADVANCED_FACE"
                        && id(&list(&record.parameter)[2]) == *surface)
                    .count(),
                1
            );
            let cylinder = list(&cylinder.parameter);
            let placement = list(&records[&id(&cylinder[1])].parameter);
            let axis = list(&records[&id(&placement[2])].parameter);
            assert_eq!(
                list(&axis[1]).iter().map(number).collect::<Vec<_>>(),
                [0., 0., 1.]
            );
            let origin = list(&records[&id(&placement[1])].parameter);
            let coordinates = list(&origin[1]);
            (
                [number(&coordinates[0]), number(&coordinates[1])],
                number(&cylinder[2]),
            )
        })
        .collect()
}

#[test]
fn source_contour_hole_ownership_is_preserved() {
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    let source = input.body.pcb_source.as_mut().unwrap();
    let mut hole = source.contours[0].clone();
    hole.id = "owned-hole".into();
    hole.outer_id = Some("outline".into());
    hole.primitives[0].source_id = "hole-circle".into();
    hole.primitives[0].curve_support.as_mut().unwrap().radius_mm = 0.5;
    if let boardstudio_contracts::MechanicalShape::Circle { end, .. } =
        &mut hole.primitives[0].geometry
    {
        end.x = 10.5;
    }
    source.contours.push(hole);
    let model = crate::build_case(&input).expect("source-declared owned hole");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).unwrap();
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), 4);
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-5).unwrap();
    assert!((mesh.volume() - 7.5 * std::f64::consts::PI).abs() < 0.002);
    material(&mesh, [10., 8., 1.], false);
    material(&mesh, [11., 8., 1.], true);
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    assert_eq!(raw_cylinders(&step).len(), 2);
    for owner in ["missing-owner", "owned-hole"] {
        let mut invalid = input.clone();
        invalid.body.pcb_source.as_mut().unwrap().contours[1].outer_id = Some(owner.into());
        let error = crate::build_case(&invalid)
            .err()
            .expect("invalid ownership must reject");
        assert!(error.contains("owned-hole"), "{error}");
        assert!(error.contains(owner), "{error}");
    }
}

#[test]
fn source_reflected_rotated_slot_resolves_nonzero_offset_once() {
    use boardstudio_contracts::{CasePcbSlot, MechanicalDrillShape, MechanicalShape, Vec2};
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    let source = input.body.pcb_source.as_mut().unwrap();
    let outer = &mut source.contours[0].primitives[0];
    outer.curve_support.as_mut().unwrap().center = Vec2 { x: 0., y: 0. };
    outer.curve_support.as_mut().unwrap().radius_mm = 10.;
    outer.geometry = MechanicalShape::Circle {
        center: Vec2 { x: 0., y: 0. },
        end: Vec2 { x: 10., y: 0. },
        width: None,
    };
    source.slots.push(CasePcbSlot {
        id: "reflected-oval".into(),
        geometry: MechanicalShape::Drill {
            at: Vec2 {
                x: 2.56018536942161,
                y: -6.083269619744623,
            },
            size: Vec2 { x: 1.2, y: 0.8 },
            offset: Vec2 { x: -0.3, y: 0.2 },
            rotation_degrees: 14.,
            shape: MechanicalDrillShape::Oval,
            plated: Some(false),
        },
    });
    slot_geometry(&input, 100. * std::f64::consts::PI, 7, 3);
}

#[test]
fn source_core_prepared_reflected_slot_is_exact() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/back-offset-slot-prepared.json"
    ))
    .unwrap();
    let step = slot_geometry(&input, 400., 10, 2);
    if let Some(directory) = std::env::var_os("PCB_CONSUMER_EVIDENCE_OUTPUT") {
        let directory = std::path::Path::new(&directory);
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("back-offset-slot.step"), step.as_bytes()).unwrap();
        std::fs::write(
            directory.join("back-offset-slot.json"),
            serde_json::to_vec_pretty(&input).unwrap(),
        )
        .unwrap();
    }
}

fn slot_geometry(
    input: &PreparedCaseIR,
    outer_area: f64,
    faces: usize,
    cylinder_count: usize,
) -> String {
    let model = crate::build_case(input).expect("reflected rotated authored oval slot");
    assert_eq!(model.solids.len(), 1);
    let solid = &model.solids[0];
    Solid::try_new(solid.boundaries().clone()).unwrap();
    assert!(solid.is_geometric_consistent());
    assert_eq!(solid.boundaries()[0].len(), faces);
    let mesh = shell_to_polygon_strict(&solid.boundaries()[0], 1e-4).unwrap();
    let expected = input.body.thickness * (outer_area - (0.32 + 0.16 * std::f64::consts::PI));
    assert!((mesh.volume() - expected).abs() < 0.02);
    let center = [2.2207122724188775, -5.961787043169324];
    let (sin, cos) = 14_f64.to_radians().sin_cos();
    material(&mesh, [center[0], center[1], 1.], false);
    material(
        &mesh,
        [center[0] + sin * 0.38, center[1] - cos * 0.38, 1.],
        false,
    );
    material(
        &mesh,
        [center[0] - sin * 0.43, center[1] + cos * 0.43, 1.],
        true,
    );
    let step = String::from_utf8(crate::export_step(&model).unwrap()).unwrap();
    let cylinders = raw_cylinders(&step);
    assert_eq!(cylinders.len(), cylinder_count);
    for sign in [-1., 1.] {
        let expected = [center[0] + sign * cos * 0.2, center[1] + sign * sin * 0.2];
        assert_eq!(
            cylinders
                .iter()
                .filter(|(actual, radius)| radius.to_bits() == 0.4_f64.to_bits()
                    && actual
                        .iter()
                        .zip(expected)
                        .all(|(a, b)| (a - b).abs() < 2e-14))
                .count(),
            1
        );
    }
    let buffers = crate::mesh_profile(&model).unwrap();
    assert!(!buffers.positions.is_empty());
    assert_eq!(buffers.positions.len(), buffers.normals.len());
    assert!(
        buffers
            .positions
            .iter()
            .chain(&buffers.normals)
            .all(|value| value.is_finite())
    );
    step
}

#[test]
fn source_declared_owner_is_not_silently_reassigned_to_nested_island() {
    let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    let template = input.body.pcb_source.as_ref().unwrap().contours[0].clone();
    let make = |id: &str, owner: Option<&str>, radius: f64| {
        let mut contour = template.clone();
        contour.id = id.into();
        contour.outer_id = owner.map(str::to_string);
        contour.primitives[0].source_id = format!("{id}-circle");
        contour.primitives[0]
            .curve_support
            .as_mut()
            .unwrap()
            .radius_mm = radius;
        if let boardstudio_contracts::MechanicalShape::Circle { end, .. } =
            &mut contour.primitives[0].geometry
        {
            end.x = 10. + radius;
        }
        contour
    };
    input.body.pcb_source.as_mut().unwrap().contours = vec![
        make("outer", None, 2.),
        make("void", Some("outer"), 1.5),
        make("island", None, 1.),
        make("island-hole", Some("outer"), 0.25),
    ];
    let error = crate::build_case(&input)
        .err()
        .expect("contradictory explicit hole ownership");
    assert!(
        error.contains("island-hole") && error.contains("outer"),
        "{error}"
    );
    input.body.pcb_source.as_mut().unwrap().contours[3].outer_id = Some("island".into());
    let model = crate::build_case(&input).expect("correctly owned nested island and hole");
    assert_eq!(model.solids.len(), 2);
    let mut volume = 0.;
    for solid in &model.solids {
        Solid::try_new(solid.boundaries().clone()).unwrap();
        assert!(solid.is_geometric_consistent());
        volume += shell_to_polygon_strict(&solid.boundaries()[0], 1e-5)
            .unwrap()
            .volume();
    }
    assert!((volume - 5.375 * std::f64::consts::PI).abs() < 0.004);
}

#[test]
fn source_crossing_and_tangent_drills_are_named_refusals() {
    use boardstudio_contracts::{Mount, MountKind, Vec2};
    for x in [11.5, 11.6] {
        let mut input: PreparedCaseIR = serde_json::from_str(include_str!(
            "../../tests/fixtures/pcb-source/circle-tracer.json"
        ))
        .unwrap();
        input.body.mounts = Some(vec![Mount {
            id: "boundary-drill".into(),
            at: Vec2 { x, y: 8. },
            kind: MountKind::Hole,
            hole_diameter: 1.,
            boss_diameter: None,
            height: None,
        }]);
        let error = crate::build_case(&input)
            .err()
            .expect("crossing or tangent source boundary must refuse");
        assert!(error.contains("boundary-drill"), "{error}");
    }
}

fn prepared_source_bits(input: &PreparedCaseIR) {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/vik-splitter-prepared-bits.json"
    ))
    .unwrap();
    let check = |id: &str, actual: &[f64], expected: &serde_json::Value| {
        let values = expected["values"].as_array().unwrap();
        assert_eq!(actual.len(), values.len());
        for (index, (actual, value)) in actual.iter().zip(values).enumerate() {
            let bits = u64::from_str_radix(value["binary64_hex"].as_str().unwrap(), 16).unwrap();
            assert_eq!(
                value["decimal"]
                    .as_str()
                    .unwrap()
                    .parse::<f64>()
                    .unwrap()
                    .to_bits(),
                bits,
                "independent literal/bit agreement {id}/{index}"
            );
            assert_eq!(
                actual.to_bits(),
                bits,
                "lossless prepared source field {id}/{}",
                expected["fields"][index]
            );
        }
    };
    let source = input.body.pcb_source.as_ref().unwrap();
    assert_eq!(expected["arc_supports"].as_array().unwrap().len(), 4);
    for record in expected["arc_supports"].as_array().unwrap() {
        let id = record["id"].as_str().unwrap();
        let primitive = source
            .contours
            .iter()
            .flat_map(|contour| &contour.primitives)
            .find(|primitive| primitive.source_id == id)
            .unwrap();
        let support = primitive.curve_support.as_ref().unwrap();
        check(
            id,
            &[
                support.center.x,
                support.center.y,
                support.radius_mm,
                support.start_direction.x,
                support.start_direction.y,
                support.sweep_radians,
            ],
            record,
        );
    }
    assert_eq!(expected["round_drills"].as_array().unwrap().len(), 38);
    for record in expected["round_drills"].as_array().unwrap() {
        let id = record["id"].as_str().unwrap();
        let drill = input
            .body
            .mounts
            .as_ref()
            .unwrap()
            .iter()
            .find(|drill| drill.id == id)
            .unwrap();
        check(id, &[drill.at.x, drill.at.y, drill.hole_diameter], record);
    }
}
