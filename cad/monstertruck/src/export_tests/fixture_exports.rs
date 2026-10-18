//! Reference faces constructed from authored geometry, never from the emitted STEP or a mesh.
use crate::step_accuracy::{
    FixtureEdge as Edge, FixtureFace as Face, FixtureSurface as Surface, compare_fixture,
    direct_product_solid_counts,
};
use crate::{StepBody, build_case, export_step, export_step_bodies};
use boardstudio_contracts::{PreparedCaseAssemblyIR, PreparedCaseIR};
use std::f64::consts::PI;

type P = [f64; 3];
fn line(from: P, to: P) -> Edge {
    Edge::Line { from, to }
}
fn ends(edge: &Edge) -> [P; 2] {
    match *edge {
        Edge::Line { from, to } | Edge::Arc { from, to, .. } => [from, to],
    }
}
fn reverse(edge: &Edge) -> Edge {
    match *edge {
        Edge::Line { from, to } => line(to, from),
        Edge::Arc {
            from,
            to,
            center,
            radius,
            turn,
        } => Edge::Arc {
            from: to,
            to: from,
            center,
            radius,
            turn: -turn,
        },
    }
}
fn reversed(loop_: &[Edge]) -> Vec<Edge> {
    loop_.iter().rev().map(reverse).collect()
}
fn at_z(edge: &Edge, z: f64) -> Edge {
    let [mut from, mut to] = ends(edge);
    from[2] = z;
    to[2] = z;
    match *edge {
        Edge::Line { .. } => line(from, to),
        Edge::Arc {
            center,
            radius,
            turn,
            ..
        } => Edge::Arc {
            from,
            to,
            center,
            radius,
            turn,
        },
    }
}
fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Edge> {
    let p = [[x0, y0, 0.], [x1, y0, 0.], [x1, y1, 0.], [x0, y1, 0.]];
    (0..4).map(|i| line(p[i], p[(i + 1) % 4])).collect()
}
fn arc(center: [f64; 2], radius: f64, angle: f64, turn: f64) -> Edge {
    let at = |a: f64| {
        [
            center[0] + radius * a.cos(),
            center[1] + radius * a.sin(),
            0.,
        ]
    };
    Edge::Arc {
        from: at(angle),
        to: at(angle + turn),
        center,
        radius,
        turn,
    }
}
fn circle(center: [f64; 2], radius: f64) -> Vec<Edge> {
    // A reference circle uses two directed semicircles; its two vertices are analytic ±X.
    vec![arc(center, radius, 0., PI), arc(center, radius, PI, PI)]
}
fn cap(z: f64, outward: f64, loops: &[Vec<Edge>]) -> Face {
    Face {
        surface: Surface::Plane {
            origin: [0., 0., z],
            outward: [0., 0., outward],
        },
        loops: loops
            .iter()
            .map(|l| {
                let l: Vec<_> = l.iter().map(|e| at_z(e, z)).collect();
                if outward > 0. { l } else { reversed(&l) }
            })
            .collect(),
    }
}
fn walls(loop_: &[Edge], z: [f64; 2]) -> Vec<Face> {
    // The references below have either a whole two-semicircle cylinder or individual walls.
    if let [
        Edge::Arc {
            center,
            radius,
            turn,
            ..
        },
        Edge::Arc { center: c, .. },
    ] = loop_
        && center == c
        && (turn.abs() - PI).abs() < 1e-12
    {
        return vec![Face {
            surface: Surface::Cylinder {
                center: *center,
                radius: *radius,
                outward: *turn > 0.,
                tolerance: 2e-14,
            },
            loops: vec![
                loop_.iter().map(|e| at_z(e, z[0])).collect(),
                reversed(&loop_.iter().map(|e| at_z(e, z[1])).collect::<Vec<_>>()),
            ],
        }];
    }
    loop_
        .iter()
        .map(|e| {
            let [from, to] = ends(e);
            let surface = match *e {
                Edge::Line { .. } => {
                    let d = [to[0] - from[0], to[1] - from[1]];
                    let n = d[0].hypot(d[1]);
                    Surface::Plane {
                        origin: from,
                        outward: [d[1] / n, -d[0] / n, 0.],
                    }
                }
                Edge::Arc {
                    center,
                    radius,
                    turn,
                    ..
                } => Surface::Cylinder {
                    center,
                    radius,
                    outward: turn > 0.,
                    tolerance: 2e-14,
                },
            };
            let low = at_z(e, z[0]);
            let high = at_z(e, z[1]);
            Face {
                surface,
                loops: vec![vec![
                    low.clone(),
                    line(ends(&low)[1], ends(&high)[1]),
                    reverse(&high),
                    line(ends(&high)[0], ends(&low)[0]),
                ]],
            }
        })
        .collect()
}
fn extrusion(loops: Vec<Vec<Edge>>, z: [f64; 2]) -> Vec<Face> {
    let mut faces = vec![cap(z[0], -1., &loops), cap(z[1], 1., &loops)];
    for loop_ in &loops {
        faces.extend(walls(loop_, z));
    }
    faces
}
fn decimal(v: &serde_json::Value) -> f64 {
    v.as_str().unwrap().parse().unwrap()
}
fn prepared(text: &str) -> PreparedCaseIR {
    serde_json::from_str(text).unwrap()
}
fn vik() -> PreparedCaseIR {
    prepared(include_str!(
        "../../tests/fixtures/pcb-source/vik-splitter-prepared.json"
    ))
}
fn slot() -> PreparedCaseIR {
    prepared(include_str!(
        "../../tests/fixtures/pcb-source/back-offset-slot-prepared.json"
    ))
}
fn vik_reference() -> Vec<Face> {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/pcb-source/vik-splitter-expectations.json"
    ))
    .unwrap();
    // The unchanged Decimal oracle was computed from authored KiCad three-point arcs. Reflect
    // KiCad Y about the original board origin; bridge the four authored arcs with straight lines.
    let arcs: Vec<_> = oracle["arc_supports"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            arc(
                [
                    decimal(&a["center_kicad"][0]) - 81.584194,
                    110.727271 - decimal(&a["center_kicad"][1]),
                ],
                decimal(&a["radius_mm"]),
                -decimal(&a["start_radians_kicad"]),
                -decimal(&a["sweep_radians_kicad"]),
            )
        })
        .collect();
    let mut outline = vec![];
    for i in 0..4 {
        outline.push(arcs[i].clone());
        outline.push(line(ends(&arcs[i])[1], ends(&arcs[(i + 1) % 4])[0]));
    }
    let mut loops = vec![reversed(&outline)];
    for d in oracle["round_drills"].as_array().unwrap() {
        loops.push(reversed(&circle(
            [
                decimal(&d["center_kicad_mm"][0]) - 81.584194,
                110.727271 - decimal(&d["center_kicad_mm"][1]),
            ],
            decimal(&d["radius_mm"]),
        )));
    }
    let mut faces = extrusion(loops, [0., decimal(&oracle["thickness_mm"])]);
    for face in &mut faces {
        if let Surface::Cylinder { tolerance, .. } = &mut face.surface {
            // Preserve the retained VIK Decimal-source parameter comparison band.
            *tolerance = 5e-12;
        }
    }
    faces
}
fn slot_reference() -> Vec<Face> {
    // Authored drill: at + R(14°)*(-.3,.2), major/minor diameters 1.2/.8.
    let theta = 14_f64.to_radians();
    let (sin, cos) = theta.sin_cos();
    let center = [
        2.5601853694216103 - 0.3 * cos - 0.2 * sin,
        -6.083269619744623 - 0.3 * sin + 0.2 * cos,
    ];
    let a = [center[0] + 0.2 * cos, center[1] + 0.2 * sin];
    let b = [center[0] - 0.2 * cos, center[1] - 0.2 * sin];
    let right = arc(a, 0.4, theta + PI / 2., -PI);
    let left = arc(b, 0.4, theta - PI / 2., -PI);
    let hole = vec![
        right.clone(),
        line(ends(&right)[1], ends(&left)[0]),
        left.clone(),
        line(ends(&left)[1], ends(&right)[0]),
    ];
    extrusion(vec![rectangle(-10., -10., 10., 10.), hole], [0., 1.6])
}
fn output(name: &str, text: &str) {
    if let Some(path) = std::env::var_os("STEP_FIXTURE_OUTPUT") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join(format!("{name}.step")), text).unwrap();
    }
}
fn step(input: &PreparedCaseIR) -> String {
    let text = String::from_utf8(export_step(&build_case(input).unwrap()).unwrap()).unwrap();
    output(&input.body.id, &text);
    text
}
#[test]
fn vik_splitter_emitted_faces_match_authored_decimal_trims() {
    compare_fixture(&step(&vik()), &[("body", vik_reference())]).unwrap();
}
#[test]
fn reflected_slot_emitted_faces_match_authored_stadium() {
    compare_fixture(&step(&slot()), &[("body", slot_reference())]).unwrap();
}
#[test]
fn circle_and_half_disc_emitted_faces_match_analytic_trims() {
    let circle_input = prepared(include_str!(
        "../../tests/fixtures/pcb-source/circle-tracer.json"
    ));
    compare_fixture(
        &step(&circle_input),
        &[("body", extrusion(vec![circle([10., 8.], 2.)], [0., 2.]))],
    )
    .unwrap();
    let half = prepared(include_str!(
        "../../tests/fixtures/pcb-source/line-arc-tracer.json"
    ));
    let semicircle = arc([0., 0.], 2., -PI / 2., PI);
    compare_fixture(
        &step(&half),
        &[(
            "body",
            extrusion(
                vec![vec![
                    semicircle.clone(),
                    line(ends(&semicircle)[1], ends(&semicircle)[0]),
                ]],
                [0., 2.],
            ),
        )],
    )
    .unwrap();
}
#[test]
fn curved_products_keep_identity_without_widening_planar_reader() {
    let a = build_case(&vik()).unwrap();
    let b = build_case(&slot()).unwrap();
    let text = String::from_utf8(
        export_step_bodies(&[
            StepBody {
                id: "vik",
                name: "Vik splitter",
                model: &a,
            },
            StepBody {
                id: "slot",
                name: "Slot",
                model: &b,
            },
        ])
        .unwrap(),
    )
    .unwrap();
    compare_fixture(
        &text,
        &[("vik", vik_reference()), ("slot", slot_reference())],
    )
    .unwrap();
    assert_eq!(
        direct_product_solid_counts(&text).unwrap(),
        [
            ("slot".into(), "Slot".into(), 1),
            ("vik".into(), "Vik splitter".into(), 1)
        ]
    );
    assert!(matches!(
        crate::step_accuracy::product_solid_counts(&text),
        Err(crate::step_accuracy::Error::Unsupported)
    ));
    // Reverse all numeric IDs, including the relative order of the two B-reps. Matching
    // follows product ownership and directed geometry, rather than serialization numbers.
    let mut renumbered = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        renumbered.push(c);
        if c == '#' {
            let mut digits = String::new();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.push(chars.next().unwrap());
            }
            if !digits.is_empty() {
                renumbered.push_str(&(1_000_000 - digits.parse::<u64>().unwrap()).to_string());
            }
        }
    }
    compare_fixture(
        &renumbered,
        &[("vik", vik_reference()), ("slot", slot_reference())],
    )
    .unwrap();
    assert_eq!(
        direct_product_solid_counts(&renumbered).unwrap(),
        direct_product_solid_counts(&text).unwrap()
    );
    let wrong_owners = text
        .replace("PRODUCT('vik',", "PRODUCT('temporary',")
        .replace("PRODUCT('slot',", "PRODUCT('vik',")
        .replace("PRODUCT('temporary',", "PRODUCT('slot',");
    assert_ne!(wrong_owners, text);
    assert!(
        compare_fixture(
            &wrong_owners,
            &[("vik", vik_reference()), ("slot", slot_reference())]
        )
        .is_err()
    );
    output("combined-pcb", &text);
}

fn gasket_reference() -> Vec<Face> {
    // gasketed-pair.prepared.json: 60×40 exterior, 52×32 cavity, groove between
    // inset rectangles 1 and 2, depth .5 at the tray's z=8 rim. No output-derived data.
    let outer = rectangle(0., 0., 60., 40.);
    let cavity = rectangle(4., 4., 56., 36.);
    let groove_outer = rectangle(1., 1., 59., 39.);
    let groove_inner = rectangle(2., 2., 58., 38.);
    let mut faces = vec![
        cap(0., -1., std::slice::from_ref(&outer)),
        cap(2., 1., std::slice::from_ref(&cavity)),
        cap(7.5, 1., &[groove_outer.clone(), reversed(&groove_inner)]),
        cap(8., 1., &[outer.clone(), reversed(&groove_outer)]),
        cap(8., 1., &[groove_inner.clone(), reversed(&cavity)]),
    ];
    for (loop_, z) in [
        (outer, [0., 8.]),
        (reversed(&cavity), [2., 8.]),
        (reversed(&groove_outer), [7.5, 8.]),
        (groove_inner, [7.5, 8.]),
    ] {
        faces.extend(walls(&loop_, z));
    }
    faces
}

fn gasket_lid_reference() -> Vec<Face> {
    // The authored lid is the tray reflected across z=10: its rim is z=12 and roof z=20.
    gasket_reference()
        .into_iter()
        .map(|mut face| {
            let Surface::Plane { origin, outward } = &mut face.surface else {
                unreachable!()
            };
            origin[2] = 20. - origin[2];
            outward[2] = -outward[2];
            for loop_ in &mut face.loops {
                for e in loop_.iter_mut() {
                    let Edge::Line { from, to } = e else {
                        unreachable!()
                    };
                    from[2] = 20. - from[2];
                    to[2] = 20. - to[2];
                }
                *loop_ = reversed(loop_);
            }
            face
        })
        .collect()
}

#[test]
fn current_assembled_gasket_pair_keeps_placed_faces_and_material() {
    let assembly: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../tests/fixtures/layered/gasketed-pair.prepared.json"
    ))
    .unwrap();
    let models: Vec<_> = assembly
        .bodies
        .iter()
        .map(|input| build_case(input).unwrap())
        .collect();
    let bodies: Vec<_> = assembly
        .bodies
        .iter()
        .zip(&models)
        .map(|(input, model)| StepBody {
            id: &input.body.id,
            name: &input.body.name,
            model,
        })
        .collect();
    let text = String::from_utf8(export_step_bodies(&bodies).unwrap()).unwrap();
    compare_fixture(
        &text,
        &[
            ("gasket-tray", gasket_reference()),
            ("gasket-lid", gasket_lid_reference()),
        ],
    )
    .unwrap();
    assert_eq!(
        direct_product_solid_counts(&text).unwrap(),
        [
            ("gasket-lid".into(), "gasket-lid".into(), 1),
            ("gasket-tray".into(), "gasket-tray".into(), 1),
        ]
    );
    // Independent planar reader measures these actual bytes, rather than a kernel mesh.
    let solids = crate::step_accuracy::inspect(&text).unwrap();
    assert_eq!(solids.len(), 2);
    for (solid, z) in solids.iter().zip([0., 12.]) {
        // 60*40*8 - 52*32*6 - (58*38 - 56*36)*.5 = 9122 mm³.
        assert!((solid.volume - 9122.).abs() < 1e-8);
        assert_eq!(solid.bounds, [[0., 0., z], [60., 40., z + 8.]]);
        let groove_z = if z == 0. { 7.75 } else { 12.25 };
        for (point, material) in [
            ([1.5, 20., groove_z], false),
            ([0.5, 20., groove_z], true),
            ([3., 20., groove_z], true),
            ([30., 20., z + 3.], false),
        ] {
            assert_eq!(solid.contains(point), material);
        }
    }
    let shifted = super::map_points(&text, |mut p| {
        p[2] += 1.;
        p
    });
    assert!(
        compare_fixture(
            &shifted,
            &[
                ("gasket-tray", gasket_reference()),
                ("gasket-lid", gasket_lid_reference())
            ]
        )
        .is_err()
    );
    let wrong_depth = super::map_points(&text, |mut p| {
        if p[2] == 7.5 {
            p[2] = 7.25;
        }
        p
    });
    assert!(
        compare_fixture(
            &wrong_depth,
            &[
                ("gasket-tray", gasket_reference()),
                ("gasket-lid", gasket_lid_reference())
            ]
        )
        .is_err()
    );
    output("gasket-pair", &text);
}

#[test]
fn direct_identity_reader_refuses_occurrences_and_duplicate_ownership() {
    use crate::step_accuracy::Error;
    assert!(matches!(
        direct_product_solid_counts(include_str!(
            "../../tests/fixtures/placed-tetra-occurrences.step"
        )),
        Err(Error::Unsupported)
    ));
    let text = step(&slot());
    let extra = super::append_step_records(
        &text,
        "#9000 = REPRESENTATION_RELATIONSHIP('', '', #18, #18);",
    );
    assert!(matches!(
        direct_product_solid_counts(&extra),
        Err(Error::Unsupported)
    ));
    let representation = text
        .lines()
        .find(|l| l.contains(" = SHAPE_REPRESENTATION("))
        .unwrap();
    let solid = text
        .lines()
        .find(|l| l.contains(" = MANIFOLD_SOLID_BREP("))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap();
    let duplicate = representation.replace(&format!("{solid})"), &format!("{solid}, {solid})"));
    assert_ne!(duplicate, representation);
    assert!(direct_product_solid_counts(&text.replacen(representation, &duplicate, 1)).is_err());
}

#[test]
fn fixture_comparison_refuses_surface_units_and_complete_trim_loss() {
    let text = step(&slot());
    let cylinder = text
        .lines()
        .find(|l| l.contains("CYLINDRICAL_SURFACE"))
        .unwrap();
    let wrong = cylinder.replace("0.4)", "0.41)");
    assert_ne!(wrong, cylinder);
    assert!(
        compare_fixture(
            &text.replacen(cylinder, &wrong, 1),
            &[("body", slot_reference())]
        )
        .is_err()
    );
    assert!(
        compare_fixture(
            &text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)"),
            &[("body", slot_reference())]
        )
        .is_err()
    );
    let curve = text.lines().find(|l| l.contains("SURFACE_CURVE")).unwrap();
    let wrong = curve.replace(".CURVE_3D.", ".PCURVE_S1.");
    assert!(
        compare_fixture(
            &text.replacen(curve, &wrong, 1),
            &[("body", slot_reference())]
        )
        .is_err()
    );
    let loop_ = text.lines().find(|l| l.contains("EDGE_LOOP")).unwrap();
    let comma = loop_.rfind(", ").unwrap();
    let shortened = format!("{}));", &loop_[..comma]);
    assert!(
        compare_fixture(
            &text.replacen(loop_, &shortened, 1),
            &[("body", slot_reference())]
        )
        .is_err()
    );
}
#[test]
fn fixture_comparison_refuses_disconnected_wall_trim_order() {
    let text = step(&slot());
    assert!(
        compare_fixture(&text, &[("body", slot_reference())]).is_ok(),
        "unmodified source export"
    );
    // A wall loop has four edges. Swapping adjacent uses leaves all edge counts, senses,
    // surfaces and coordinates unchanged, but disconnects the actual face boundary.
    let wall = text
        .lines()
        .filter(|line| line.contains("EDGE_LOOP"))
        .nth(2)
        .unwrap();
    let (prefix, edges) = wall.split_once("('', (").unwrap();
    let edges = edges.strip_suffix("));").unwrap();
    let mut edges: Vec<_> = edges.split(", ").collect();
    assert_eq!(edges.len(), 4);
    edges.swap(0, 1);
    let disconnected = format!("{prefix}('', ({}));", edges.join(", "));
    let result = compare_fixture(
        &text.replacen(wall, &disconnected, 1),
        &[("body", slot_reference())],
    );
    assert!(result.is_err(), "{result:?}");
}

#[test]
fn fixture_comparison_refuses_a_slot_outside_the_outer_cap() {
    let text = step(&slot());
    assert!(
        compare_fixture(&text, &[("body", slot_reference())]).is_ok(),
        "unmodified source export"
    );
    let mut moved = 0;
    let displaced = text
        .lines()
        .map(|line| {
            let Some((prefix, coordinates)) = line.split_once("CARTESIAN_POINT('', (") else {
                return line.to_string();
            };
            let coordinates = coordinates.strip_suffix("));").unwrap();
            let values: Vec<f64> = coordinates
                .split(", ")
                .map(|v| v.parse().unwrap())
                .collect();
            assert_eq!(values.len(), 3);
            // Move every slot point (vertices, curve controls and surface origins), leaving
            // the square at [-10,10] unchanged. The alleged hole now lies outside its cap.
            if (1.0..3.0).contains(&values[0]) && (-7.0..-5.0).contains(&values[1]) {
                moved += 1;
                format!(
                    "{prefix}CARTESIAN_POINT('', ({}, {}, {}));",
                    values[0] + 30.0,
                    values[1],
                    values[2]
                )
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        moved > 20,
        "all slot curve and surface points must be moved"
    );
    let result = compare_fixture(&displaced, &[("body", slot_reference())]);
    assert!(result.is_err(), "{result:?}");
}
