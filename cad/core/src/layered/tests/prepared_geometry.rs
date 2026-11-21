use super::*;

#[test]
fn pcb_polygon_outline_and_authored_round_drill_are_exact() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/pcb-simple.json"
    ))
    .unwrap();
    let (model, step) = verify(
        &input,
        512. - 1.6 * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 1.6]],
        &[
            ([10., 8., 0.8], false),
            ([11.2, 8., 0.8], true),
            ([1., 1., 0.8], true),
            ([10., 8., 1.7], false),
        ],
        1,
        0,
    );
    assert_eq!(
        model.test_solids()[0]
            .boundaries()
            .iter()
            .flatten()
            .filter(|f| matches!(f.surface(), Surface::RevolutionSurface(_)))
            .count(),
        1
    );
    assert_eq!(cylinder_senses(&step), vec![(1., false)]);
}
#[test]
fn pcb_nonconvex_polygon_with_diagonal_and_multiple_drills_is_exact() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/pcb-nonconvex.json"
    ))
    .unwrap();
    // Rectangle30x10 plus trapezoid(15+10)x10/2 gives425mm2; drills remove1.8125*pi.
    let (model, step) = verify(
        &input,
        680. - 2.9 * std::f64::consts::PI,
        [[0., 0., 0.], [30., 20., 1.6]],
        &[
            ([5., 5., 0.8], false),
            ([25., 5., 0.8], false),
            ([5., 15., 0.8], false),
            ([6.2, 5., 0.8], true),
            ([11., 11., 0.8], true),
            ([25., 15., 0.8], false),
        ],
        3,
        0,
    );
    let solids = model.test_solids();
    let shell = &solids[0].boundaries()[0];
    assert_eq!(
        shell
            .iter()
            .filter(|f| matches!(f.surface(), Surface::Plane(_)))
            .count(),
        8
    );
    assert_eq!(
        shell
            .iter()
            .filter(|f| matches!(f.surface(), Surface::RevolutionSurface(_)))
            .count(),
        3
    );
    assert_eq!(
        cylinder_senses(&step),
        vec![(0.5, false), (0.75, false), (1., false)]
    );
}
#[test]
fn unrepresentable_body_interval_is_rejected_before_layer_assembly() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.id = "collapsed-pcb-elevation".into();
    input.body.z = Some(1e17);
    input.body.thickness = 1.;
    let error = build_layered_case(&input)
        .err()
        .expect("body height cannot collapse at its elevation");
    assert!(
        error.contains("collapsed-pcb-elevation") && error.contains("height"),
        "{error}"
    );
}

#[test]
fn prepared_gasket_pair_retains_inner_material_for_tray_and_lid() {
    let assembly: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/gasketed-pair.prepared.json"
    ))
    .unwrap();
    for input in assembly.bodies {
        let z = input.body.z.unwrap();
        let groove_z = if input.body.kind == CaseKind::Tray {
            7.75
        } else {
            12.25
        };
        verify(
            &input,
            9122.,
            [[0., 0., z], [60., 40., z + 8.]],
            &[
                ([1.5, 20., groove_z], false),
                ([0.5, 20., groove_z], true),
                ([3., 20., groove_z], true),
                ([30., 20., z + 3.], false),
            ],
            0,
            0,
        );
    }
}

#[test]
fn prepared_wall_support_and_round_seat_preserve_clipped_circular_wall() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/wall-support-seat.prepared.json"
    ))
    .unwrap();
    let strip_area = 2. * ((1.6_f64 * 1.6 - 1.).sqrt() + 1.6 * 1.6 * (1. / 1.6_f64).asin());
    let (_, step) = verify(
        &input,
        2832. - 3. * strip_area,
        [[0., 0., 0.], [30., 20., 10.]],
        &[
            ([4., 10., 4.], true),
            ([7., 10., 4.], false),
            ([10., 1., 5.], false),
            ([10., 1., 3.], true),
            ([12., 1., 5.], true),
        ],
        2,
        0,
    ); // Two disconnected cylinder patches on opposite sides of the clipped circle.
    assert_eq!(
        cylinder_senses(&step),
        vec![(1.6, false), (1.6, false)],
        "authored radius must set the exact native cylinder, not a reconstructed endpoint radius"
    );
}

#[test]
fn prepared_concave_gasket_pair_keeps_nested_material_and_notch() {
    let assembly: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/concave-gasket.prepared.json"
    ))
    .unwrap();
    // L-shaped area3900; cavity72*52-30*30=2844; groove3624-3356=268.
    for input in assembly.bodies {
        let z = input.body.z.unwrap();
        let groove_z = if input.body.kind == CaseKind::Tray {
            7.75
        } else {
            12.25
        };
        verify(
            &input,
            14002.,
            [[0., 0., z], [80., 60., z + 8.]],
            &[
                ([1.5, 20., groove_z], false),
                ([0.5, 20., groove_z], true),
                ([3., 20., groove_z], true),
                ([20., 20., z + 3.], false),
                ([60., 40., z + 1.], false),
            ],
            0,
            0,
        );
    }
}

#[test]
fn prepared_internal_gasket_inventory() {
    let expectations: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/internal-independent-expectations.json"
    ))
    .unwrap();
    let mut work_items = vec![];
    for name in [
        "rectangle",
        "countersunk",
        "downward-boss",
        "rotated-concave",
        "split",
    ] {
        let path = format!(
            "{}/tests/fixtures/layered/internal-gasket-v1/{name}.prepared.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let assembly: PreparedCaseAssemblyIR =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for input in assembly.bodies {
            work_items.push((name, input));
        }
    }
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = work_items
            .into_iter()
            .map(|(name, input)| {
                let expectations = &expectations;
                scope.spawn(move || -> Option<String> {
                    match build_layered_case(&input) {
                        Ok(model) => {
                            let expected = &expectations["fixtures"][name][&input.body.id];
                            assert_eq!(model.test_solids().len(), input.regions.len());
                            let mut meshes = vec![];
                            for solid in model.test_solids() {
                                check_solid(solid);
                                for shell in solid.boundaries() {
                                    meshes.push(
                                        shell_to_polygon_strict(shell, 1e-3)
                                            .expect("every internal face tessellates"),
                                    );
                                }
                            }
                            let volume: f64 = meshes.iter().map(|mesh| mesh.volume()).sum();
                            let target = expected["volume"].as_f64().unwrap();
                            assert!(
                                (volume - target).abs() < 0.2,
                                "{name}/{} volume {volume} != independent {target}",
                                input.body.id
                            );
                            let solids = model.test_solids();
                            let points = solids
                                .iter()
                                .flat_map(Solid::boundaries)
                                .flat_map(|shell| shell.vertex_iter())
                                .map(|vertex| vertex.point());
                            for (side, row) in point_bounds(points).into_iter().enumerate() {
                                for (axis, value) in row.into_iter().enumerate() {
                                    let target = expected["bounds"][side][axis].as_f64().unwrap();
                                    assert!((value - target).abs() < 1e-10);
                                }
                            }
                            for probe in expected["probes"].as_array().unwrap() {
                                let point = Point3::new(
                                    probe[0][0].as_f64().unwrap(),
                                    probe[0][1].as_f64().unwrap(),
                                    probe[0][2].as_f64().unwrap(),
                                );
                                let target = if probe[1].as_bool().unwrap() { 1. } else { 0. };
                                assert!(
                                    (mesh_winding(&meshes, point) - target).abs() < 1e-8,
                                    "{name}/{} probe {point:?}",
                                    input.body.id
                                );
                            }
                            crate::mesh_profile(&model).expect("existing mesh path");
                            let step =
                                crate::export_step(&model).expect("existing exact STEP path");
                            retain_evidence(
                                &format!(
                                    "internal-step/{name}-{}.step",
                                    input.body.id.replace(':', "_")
                                ),
                                &step,
                            );
                            None
                        }
                        Err(error) => Some(format!("{name}/{}: {error}", input.body.id)),
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap())
            .collect()
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn near_concentric_distinct_sources_are_not_declared_coincident() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![
        CaseFeature::RoundSeat {
            id: "first-circle".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
        CaseFeature::RoundSeat {
            id: "distinct-circle".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2. + 1e-11,
        },
    ]);
    let error = build_layered_case(&input)
        .err()
        .expect("distinct authored circles must not be silently coincident");
    assert!(error.contains("unproved coincidence"), "{error}");
    if let Some(features) = &mut input.body.features
        && let CaseFeature::RoundSeat { at, diameter, .. } = &mut features[1]
    {
        at.x += 1e-11;
        *diameter = 2.;
    }
    let error = build_layered_case(&input)
        .err()
        .expect("nearby authored axes must not be declared coincident");
    assert!(error.contains("unproved coincidence"), "{error}");
}

#[test]
fn exactly_coincident_authored_seats_keep_one_native_cylinder() {
    let mut input = plate();
    input.regions[0].mounts.clear();
    input.body.features = Some(vec![
        CaseFeature::RoundSeat {
            id: "first".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
        CaseFeature::RoundSeat {
            id: "second".into(),
            at: Vec2 { x: 10., y: 8. },
            z: 0.,
            height: 4.,
            diameter: 2.,
        },
    ]);
    verify(
        &input,
        1280. - 4. * std::f64::consts::PI,
        [[0., 0., 0.], [20., 16., 4.]],
        &[([10., 8., 2.], false), ([12., 8., 2.], true)],
        1,
        0,
    );
}

#[test]
fn rotated_notched_wall_is_one_authored_plane() {
    let angle = 0.3_f64;
    let (sine, cosine) = angle.sin_cos();
    let rotate = |point: &mut Vec2| {
        let (x, y) = (point.x, point.y);
        point.x = x * cosine - y * sine;
        point.y = x * sine + y * cosine;
    };
    let mut input = plate();
    input.regions[0].mounts.clear();
    for point in &mut input.regions[0].outer {
        rotate(point);
    }
    let mut at = Vec2 { x: 10., y: 0. };
    rotate(&mut at);
    input.body.features = Some(vec![CaseFeature::RoundSeat {
        id: "rotated-wall-notch".into(),
        at,
        z: 1.,
        height: 2.,
        diameter: 2.,
    }]);
    let retained: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../tests/fixtures/layered/rotated-wall-notch.json"
    ))
    .unwrap();
    // Cover the exact typed RED and the normal JSON seam without changing decoder features.
    assert_eq!(input, retained);
    let model = build_layered_case(&input).expect("rotated wall with finite-height notch");
    let outward = Vector3::new(sine, -cosine, 0.);
    let count=model.test_solids()[0].boundaries()[0].iter().filter(|face| matches!(face.oriented_surface(),Surface::Plane(plane) if plane.normal().normalize().dot(outward)>1.-1e-12)).count();
    assert_eq!(
        count, 1,
        "one connected authored wall plane, including the boundary around its notch"
    );
    let sample = |x: f64, y: f64, z: f64| [x * cosine - y * sine, x * sine + y * cosine, z];
    verify(
        &input,
        1280. - std::f64::consts::PI,
        [
            [-16. * sine, 0., 0.],
            [20. * cosine, 20. * sine + 16. * cosine, 4.],
        ],
        &[
            (sample(10., 0.5, 2.), false),
            (sample(10., 1.5, 2.), true),
            (sample(10., 0.5, 0.5), true),
            (sample(10., -0.5, 2.), false),
        ],
        1,
        0,
    );
    let exact = crate::export_step(&model).unwrap();
    retain_evidence("rotated-step.step", &exact);
}
