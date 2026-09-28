use super::*;

#[test]
#[ignore = "Prepared live fixture construction experiment; run on a quiet host"]
fn benchmark_opening_groups() {
    let input = std::fs::read_to_string(std::env::var("CAD_BENCH_INPUT").unwrap()).unwrap();
    let mut ir: PreparedAssembly = serde_json::from_str(&input).unwrap();
    let body = ir
        .bodies
        .iter_mut()
        .find(|body| body.body.id == "bottom")
        .unwrap();
    for sample in 0..6 {
        for offset in 0..7 {
            let strategy = (sample + offset) % 7;
            let mut elapsed = 0.;
            for region in &mut body.regions {
                let start = std::time::Instant::now();
                let mounts = if strategy == 3 || strategy == 6 {
                    std::mem::take(&mut region.mounts)
                } else {
                    vec![]
                };
                let mut solids = build_region_upstream(
                    &body.body,
                    region,
                    BuildMode {
                        mount_holes: true,
                        gasket_profile: true,
                        boss_unions: true,
                        ..BuildMode::default()
                    },
                )
                .unwrap();
                if strategy == 3 || strategy == 6 {
                    region.mounts = mounts;
                }
                let openings: Vec<_> = body
                    .body
                    .openings
                    .as_ref()
                    .unwrap()
                    .iter()
                    .filter(|opening| opening_intersects_region(&body.body, region, opening))
                    .collect();
                let mut groups: Vec<Vec<&CaseOpening>> = Vec::new();
                for opening in openings {
                    let index = match strategy {
                        0 | 3 | 6 => (!groups.is_empty()).then_some(0),
                        1 => None,
                        4 | 5 => groups.iter().position(|group| {
                            (group[0].z < body.body.z.unwrap_or(0.))
                                == (opening.z < body.body.z.unwrap_or(0.))
                        }),
                        _ => groups.iter().position(|group| {
                            group[0].z == opening.z && group[0].height == opening.height
                        }),
                    };
                    if let Some(index) = index {
                        groups[index].push(opening);
                    } else {
                        groups.push(vec![opening]);
                    }
                }
                if strategy == 5 {
                    groups.reverse();
                }
                for group in groups {
                    let mut cutters = group
                        .iter()
                        .map(|opening| make_prism(&opening.points, opening.z, opening.height))
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    if strategy == 3 {
                        for mount in &region.mounts {
                            cutters.push(
                                make_cylinder(
                                    &mount.at,
                                    body.body.z.unwrap_or(0.),
                                    mount.hole_diameter,
                                    body.body.thickness,
                                )
                                .unwrap(),
                            );
                        }
                    }
                    subtract_many(&mut solids, &cutters).unwrap();
                }
                if strategy == 6 {
                    let cutters = region
                        .mounts
                        .iter()
                        .map(|mount| {
                            make_cylinder(
                                &mount.at,
                                body.body.z.unwrap_or(0.),
                                mount.hole_diameter,
                                body.body.thickness,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap();
                    subtract_many(&mut solids, &cutters).unwrap();
                }
                elapsed += start.elapsed().as_secs_f64() * 1000.;
                assert!(solids.iter().all(|solid| solid.volume() > 0.));
            }
            if sample > 0 {
                println!("OPENING_STRATEGY {strategy} {elapsed}");
            }
        }
    }
}

fn square(x: f64, y: f64, size: f64) -> Vec<Vec2> {
    vec![
        Vec2 { x, y },
        Vec2 { x: x + size, y },
        Vec2 {
            x: x + size,
            y: y + size,
        },
        Vec2 { x, y: y + size },
    ]
}

#[test]
fn overlapping_opening_profiles_preserve_exact_material_and_export() {
    let ir: PreparedCase = serde_json::from_str(include_str!(
        "../../../../bench/fixtures/live-gasket-bottom-regression.json"
    ))
    .unwrap();
    for region in &ir.regions {
        let openings: Vec<_> = ir
            .body
            .openings
            .as_ref()
            .unwrap()
            .iter()
            .filter(|opening| opening_intersects_region(&ir.body, region, opening))
            .collect();
        let tabs = opening_remainder(openings[1], &openings[..1])
            .expect("upper pocket has a covered rectangular center");
        assert_eq!(tabs.len(), 6);
        let reference = reference_region(&ir.body, region).unwrap();
        let actual = build_region(&ir.body, region).unwrap();
        equivalent(&reference, &actual);
        for (before, after) in reference.iter().zip(&actual) {
            for difference in [
                (Boolean::from(before) - after),
                (Boolean::from(after) - before),
            ] {
                assert!(
                    difference
                        .build_vec()
                        .unwrap()
                        .iter()
                        .map(Solid::volume)
                        .sum::<f64>()
                        .abs()
                        < 0.001
                );
            }
        }
    }
}

#[test]
fn opening_remainder_falls_back_for_unsupported_or_uncovered_profiles() {
    let covered = CaseOpening {
        points: square(2., 2., 10.),
        z: 0.,
        height: 5.,
    };
    let mut opening = CaseOpening {
        points: square(0., 0., 20.),
        z: 1.,
        height: 2.,
    };
    assert!(opening_remainder(&opening, &[&covered]).is_some());
    opening.z = -1.;
    assert!(opening_remainder(&opening, &[&covered]).is_none());
    opening.z = 4.;
    assert!(opening_remainder(&opening, &[&covered]).is_none());
    opening.z = 1.;
    opening.points[1].y = 1.;
    assert!(opening_remainder(&opening, &[&covered]).is_none());
    opening.points = vec![
        Vec2 { x: 0., y: 0. },
        Vec2 { x: 20., y: 0. },
        Vec2 { x: 20., y: 4. },
        Vec2 { x: 0., y: 4. },
    ];
    assert!(opening_remainder(&opening, &[&covered]).is_none());
    opening.points[0].x = f64::NAN;
    assert!(opening_remainder(&opening, &[&covered]).is_none());
}

#[test]
fn opening_remainders_preserve_partial_depth_and_winding() {
    for reversed in [false, true] {
        let mut b = body(CaseKind::Plate);
        b.thickness = 6.;
        let mut center = square(5., 5., 10.);
        let mut surrounding = square(2., 2., 20.);
        if reversed {
            center.reverse();
            surrounding.reverse();
        }
        b.openings = Some(vec![
            CaseOpening {
                points: center,
                z: 1.,
                height: 6.,
            },
            CaseOpening {
                points: surrounding,
                z: 3.,
                height: 4.,
            },
        ]);
        let r = region();
        equivalent(
            &reference_region(&b, &r).unwrap(),
            &build_region(&b, &r).unwrap(),
        );
        let before = build_region(&b, &r).unwrap();
        b.openings.as_mut().unwrap()[1].height = 1.;
        let after = build_region(&b, &r).unwrap();
        assert!(after[0].volume() > before[0].volume());
        equivalent(&reference_region(&b, &r).unwrap(), &after);
    }
}
fn body(kind: CaseKind) -> CaseBody {
    CaseBody {
        id: "test".into(),
        name: "Test".into(),
        kind,
        thickness: 2.,
        z: None,
        wall_height: Some(5.),
        openings: None,
        gasket: Some(Gasket { depth: 0.5 }),
    }
}
fn region() -> PreparedRegion {
    PreparedRegion {
        outer: square(0., 0., 30.),
        holes: vec![],
        cavities: vec![square(2., 2., 26.)],
        gaskets: vec![],
        mounts: vec![],
    }
}
fn boss(x: f64, y: f64, hole: f64) -> Mount {
    Mount {
        at: Vec2 { x, y },
        kind: MountKind::Boss,
        hole_diameter: hole,
        boss_diameter: Some(6.),
        height: Some(4.),
    }
}
fn equivalent(a: &[Solid], b: &[Solid]) {
    assert_eq!(a.len(), b.len());
    let volume = |s: &[Solid]| s.iter().map(Solid::volume).sum::<f64>();
    assert!(
        (volume(a) - volume(b)).abs() < 0.01,
        "volumes {} {}",
        volume(a),
        volume(b)
    );
    for solids in [a, b] {
        assert!(solids.iter().all(|s| s.volume() > 0.));
        let bytes = export_case(solids.to_vec(), 1, None).unwrap().step;
        let imported = Solid::read_step(&mut std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(imported.len(), solids.len());
        assert!((volume(&imported) - volume(solids)).abs() < 0.01);
    }
    let bounds = |s: &[Solid]| {
        s.iter().fold(
            [DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)],
            |a, s| {
                let b = s.bounding_box();
                [a[0].min(b[0]), a[1].max(b[1])]
            },
        )
    };
    let ab = bounds(a);
    let bb = bounds(b);
    assert!((ab[0] - bb[0]).length() < 0.01 && (ab[1] - bb[1]).length() < 0.01);
    // Compare material occupancy through boss/hole planes as well as aggregate volume.
    for z in [0.5, 2.5, 5.5] {
        for x in 0..16 {
            for y in 0..16 {
                let p = DVec3::new(x as f64 * 2. + 0.13, y as f64 * 2. + 0.17, z);
                assert_eq!(
                    a.iter().any(|s| s.contains(p)),
                    b.iter().any(|s| s.contains(p)),
                    "material at {p:?}"
                );
            }
        }
    }
}

#[test]
fn construction_preserves_edge_cases_and_step_material() {
    for kind in [CaseKind::Tray, CaseKind::Lid] {
        let b = body(kind);
        for variant in 0..6 {
            let mut r = region();
            match variant {
                0 => r.mounts = vec![boss(10., 10., 2.), boss(20., 20., 2.)],
                1 => r.mounts = vec![boss(10., 10., 2.), boss(15., 10., 2.)],
                2 => r.mounts = vec![boss(10., 10., 5.), boss(12., 10., 2.)],
                3 => r.cavities = vec![square(2., 2., 13.), square(15., 2., 13.)],
                4 => r.cavities = vec![square(0.1, 0.1, 29.8)],
                _ => {
                    r.gaskets = vec![PreparedGasket {
                        outer: square(0.3, 0.3, 29.4),
                        holes: vec![square(0.8, 0.8, 28.4)],
                    }]
                }
            }
            for mode in [
                BuildMode {
                    cavities: true,
                    ..BuildMode::default()
                },
                BuildMode {
                    mount_holes: true,
                    ..BuildMode::default()
                },
                BuildMode {
                    gasket_profile: true,
                    ..BuildMode::default()
                },
                BuildMode {
                    boss_unions: true,
                    ..BuildMode::default()
                },
                BuildMode {
                    cavities: true,
                    mount_holes: true,
                    gasket_profile: true,
                    boss_unions: true,
                },
            ] {
                equivalent(
                    &reference_region(&b, &r).unwrap(),
                    &build_region_mode(&b, &r, mode).unwrap(),
                );
            }
        }
    }
    let mut b = body(CaseKind::Plate);
    let r = region();
    b.openings = Some(vec![CaseOpening {
        points: vec![
            Vec2 { x: 14., y: -1. },
            Vec2 { x: 16., y: -1. },
            Vec2 { x: 16., y: 31. },
            Vec2 { x: 14., y: 31. },
        ],
        z: -1.,
        height: 4.,
    }]);
    let split = build_region(&b, &r).unwrap();
    assert_eq!(split.len(), 2);
    equivalent(&reference_region(&b, &r).unwrap(), &split);
    b.openings = Some(vec![CaseOpening {
        points: square(-1., -1., 32.),
        z: -1.,
        height: 4.,
    }]);
    assert!(reference_region(&b, &r).is_err());
    assert!(build_region(&b, &r).is_err());
    b.thickness = -1.;
    assert!(build_region(&b, &r).is_err());
    assert!(subtract_many(&mut vec![], &[Solid::cube(DVec3::ZERO, DVec3::ONE)]).is_err());
}

fn reference_region(body: &CaseBody, region: &PreparedRegion) -> Result<Vec<Solid>, String> {
    let base_z = body.z.unwrap_or(0.0);
    let wall_height = if body.kind == CaseKind::Plate {
        0.0
    } else {
        body.wall_height.unwrap_or(0.0)
    };
    let total_height = body.thickness + wall_height;
    if !base_z.is_finite() || !total_height.is_finite() || total_height <= 0.0 {
        return Err("Case body has invalid height or elevation".into());
    }

    let mut edges = polygon_edges(&region.outer, base_z)?;
    for hole in &region.holes {
        edges.extend(polygon_edges(hole, base_z)?);
    }
    // Cadrum extrudes the first loop with all following loops as holes in one operation.
    let mut solids = {
        let _stage = Stage::new("baseExtrusion");
        metrics::count("extrusions", 1);
        vec![Solid::extrude(&edges, DVec3::Z * total_height).map_err(cadrum_error)?]
    };

    if body.kind != CaseKind::Plate {
        let cavity_z = if body.kind == CaseKind::Tray {
            base_z + body.thickness
        } else {
            base_z
        };
        for cavity in &region.cavities {
            let _stage = Stage::new("cavityCuts");
            subtract(&mut solids, make_prism(cavity, cavity_z, wall_height)?)?;
        }

        for gasket in &region.gaskets {
            let _stage = Stage::new("gasketCuts");
            let depth = body
                .gasket
                .as_ref()
                .map(|gasket| gasket.depth)
                .ok_or_else(|| "Prepared gasket has no body gasket dimensions".to_string())?;
            let groove_z = if body.kind == CaseKind::Tray {
                base_z + total_height - depth
            } else {
                base_z
            };
            let mut groove = vec![make_prism(&gasket.outer, groove_z, depth)?];
            for hole in &gasket.holes {
                subtract(&mut groove, make_prism(hole, groove_z, depth)?)?;
            }
            for cutter in groove {
                subtract(&mut solids, cutter)?;
            }
        }
    }

    for mount in &region.mounts {
        if mount.kind == MountKind::Boss {
            let _stage = Stage::new("bossUnions");
            let boss_height = mount
                .height
                .ok_or_else(|| "Boss mount has no height".to_string())?;
            let boss_diameter = mount
                .boss_diameter
                .ok_or_else(|| "Boss mount has no diameter".to_string())?;
            let boss_z = if body.kind == CaseKind::Lid {
                base_z + wall_height - boss_height
            } else {
                base_z + body.thickness
            };
            fuse(
                &mut solids,
                make_cylinder(&mount.at, boss_z, boss_diameter, boss_height)?,
            )?;
        }
        let _stage = Stage::new("mountHoleCuts");
        subtract(
            &mut solids,
            make_cylinder(&mount.at, base_z, mount.hole_diameter, total_height)?,
        )?;
    }

    if let Some(openings) = &body.openings {
        let _stage = Stage::new("openingCuts");
        let cutters = openings
            .iter()
            .filter(|opening| opening_intersects_region(body, region, opening))
            .map(|opening| make_prism(&opening.points, opening.z, opening.height))
            .collect::<Result<Vec<_>, _>>()?;
        subtract_many(&mut solids, &cutters)?;
    }

    Ok(solids)
}

#[test]
#[ignore = "Run explicitly on a quiet host with CAD_BENCH_INPUT pointing to prepared fixture JSON"]
fn benchmark_construction_strategies() {
    let input = std::fs::read_to_string(std::env::var("CAD_BENCH_INPUT").unwrap()).unwrap();
    let ir: PreparedAssembly = serde_json::from_str(&input).unwrap();
    let modes = [
        ("sequential", BuildMode::default()),
        (
            "cavities",
            BuildMode {
                cavities: true,
                ..BuildMode::default()
            },
        ),
        (
            "mount-holes",
            BuildMode {
                mount_holes: true,
                ..BuildMode::default()
            },
        ),
        (
            "gasket-profile",
            BuildMode {
                gasket_profile: true,
                ..BuildMode::default()
            },
        ),
        (
            "boss-unions",
            BuildMode {
                boss_unions: true,
                ..BuildMode::default()
            },
        ),
        (
            "combined",
            BuildMode {
                cavities: true,
                mount_holes: true,
                gasket_profile: true,
                boss_unions: true,
            },
        ),
    ];
    for sample in 0..11 {
        // Rotate order to reduce warmup and thermal bias between candidates.
        for offset in 0..modes.len() {
            let (name, mode) = modes[(sample + offset) % modes.len()];
            let start = std::time::Instant::now();
            let solids = ir
                .bodies
                .iter()
                .flat_map(|body| {
                    body.regions
                        .iter()
                        .map(|region| build_region_mode(&body.body, region, mode).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            assert!(solids.iter().flatten().all(|solid| solid.volume() > 0.));
            if sample > 0 {
                println!("CAD_STRATEGY {name} {elapsed:.6}");
            }
        }
    }
}

#[test]
fn unusual_gasket_profiles_keep_sequential_semantics() {
    let mut b = body(CaseKind::Tray);
    for holes in [
        vec![square(40., 40., 2.)],
        vec![square(1., 1., 4.), square(20., 20., 4.)],
    ] {
        let mut r = region();
        r.gaskets = vec![PreparedGasket {
            outer: square(0.3, 0.3, 29.4),
            holes,
        }];
        assert!(!can_extrude_gasket(&r.gaskets[0]));
        equivalent(
            &reference_region(&b, &r).unwrap(),
            &build_region(&b, &r).unwrap(),
        );
    }
    let mut r = region();
    r.gaskets = vec![PreparedGasket {
        outer: square(0.3, 0.3, 29.4),
        holes: vec![square(0.8, 0.8, 28.4)],
    }];
    b.gasket = Some(Gasket { depth: -1. });
    assert!(reference_region(&b, &r).is_err());
    assert!(build_region(&b, &r).is_err());
    r.mounts = vec![boss(f64::NAN, 10., 2.)];
    r.gaskets.clear();
    assert!(reference_region(&b, &r).is_err());
    assert!(build_region(&b, &r).is_err());
}

#[test]
fn touching_and_overlapping_mount_cutters_match_sequential_cuts() {
    let b = body(CaseKind::Plate);
    for distance in [2., 1.5] {
        let mut r = region();
        r.mounts = [10., 10. + distance]
            .iter()
            .map(|x| Mount {
                at: Vec2 { x: *x, y: 10. },
                kind: MountKind::Hole,
                hole_diameter: 2.,
                boss_diameter: None,
                height: None,
            })
            .collect();
        equivalent(
            &reference_region(&b, &r).unwrap(),
            &build_region(&b, &r).unwrap(),
        );
    }
}

#[test]
#[ignore = "diagnostic native opening profile; run explicitly on a quiet host"]
fn benchmark_opening_stages() {
    let input = std::fs::read_to_string(std::env::var("CAD_BENCH_INPUT").unwrap()).unwrap();
    let ir: PreparedAssembly = serde_json::from_str(&input).unwrap();
    let mode = BuildMode {
        cavities: false,
        mount_holes: true,
        gasket_profile: true,
        boss_unions: true,
    };
    for sample in 0..11 {
        let mut filtering = 0.;
        let mut tools = 0.;
        let mut evaluation = 0.;
        for body in &ir.bodies {
            for region in &body.regions {
                let mut solids = build_region_upstream(&body.body, region, mode).unwrap();
                let start = std::time::Instant::now();
                let relevant = body
                    .body
                    .openings
                    .iter()
                    .flatten()
                    .filter(|opening| opening_intersects_region(&body.body, region, opening))
                    .collect::<Vec<_>>();
                filtering += start.elapsed().as_secs_f64() * 1000.;
                let start = std::time::Instant::now();
                let cutters = relevant
                    .iter()
                    .map(|opening| make_prism(&opening.points, opening.z, opening.height).unwrap())
                    .collect::<Vec<_>>();
                tools += start.elapsed().as_secs_f64() * 1000.;
                let start = std::time::Instant::now();
                subtract_many(&mut solids, &cutters).unwrap();
                evaluation += start.elapsed().as_secs_f64() * 1000.;
                assert!(solids.iter().all(|solid| solid.volume() > 0.));
            }
        }
        if sample > 0 {
            println!(
                "CAD_OPENINGS filter={filtering:.6} tools={tools:.6} evaluate={evaluation:.6}"
            );
        }
    }
}

#[test]
#[ignore = "diagnostic native import profile; run explicitly with CAD_BENCH_STEP"]
fn benchmark_import_stages() {
    let bytes = std::fs::read(std::env::var("CAD_BENCH_STEP").unwrap()).unwrap();
    for sample in 0..11 {
        let mut reader = std::io::Cursor::new(&bytes);
        let start = std::time::Instant::now();
        let solids = Solid::read_step(&mut reader).unwrap();
        let parsing = start.elapsed().as_secs_f64() * 1000.;
        let start = std::time::Instant::now();
        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        for solid in &solids {
            let [lo, hi] = solid.bounding_box();
            min = min.min(lo);
            max = max.max(hi);
        }
        let bounds = start.elapsed().as_secs_f64() * 1000.;
        let start = std::time::Instant::now();
        let mesh = mesh_data(&solids).unwrap();
        let tessellation = start.elapsed().as_secs_f64() * 1000.;
        assert!(min.is_finite() && max.is_finite() && !mesh.positions.is_empty());
        if sample > 0 {
            println!("CAD_IMPORT parse_and_shape={parsing:.6} bounds={bounds:.6} tessellation_and_expansion={tessellation:.6}");
        }
    }
}
