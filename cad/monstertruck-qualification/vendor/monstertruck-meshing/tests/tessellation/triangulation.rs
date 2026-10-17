use super::*;

type CompressedSolid = monstertruck_topology::compress::CompressedSolid<Point3, Curve, Surface>;

#[test]
fn angular_unclamped_active_endpoint_retains_face_and_original_samples() {
    use monstertruck_meshing::tessellation::{
        TessellationOptions, robust_cshell_triangulation_with,
    };
    use monstertruck_topology::compress::{CompressedEdge, CompressedFace, CompressedShell};

    for weight in [1.0, std::f64::consts::FRAC_1_SQRT_2] {
        let arc = NurbsCurve::new(BsplineCurve::new(
            KnotVector::from(vec![-1.0, 0.0, 0.0, 1.0, 1.0, 2.0]),
            vec![
                Vector4::new(1.0, 0.0, 0.0, 1.0),
                Vector4::new(weight, weight, 0.0, weight),
                Vector4::new(0.0, 1.0, 0.0, 1.0),
            ],
        ));
        let points = [Point3::new(1.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)];
        let shell = CompressedShell {
            vertices: points.to_vec(),
            edges: vec![
                CompressedEdge {
                    vertices: (0, 1),
                    curve: Curve::NurbsCurve(arc),
                },
                CompressedEdge {
                    vertices: (1, 0),
                    curve: Curve::Line(Line(points[1], points[0])),
                },
            ],
            faces: vec![CompressedFace {
                boundaries: vec![vec![(0, true).into(), (1, true).into()]],
                orientation: true,
                surface: Surface::Plane(Plane::xy()),
            }],
            vertex_stable_ids: None,
            edge_stable_ids: None,
            face_stable_ids: None,
        };
        let options = TessellationOptions {
            tolerance: 0.0005,
            ..Default::default()
        };
        let linear = robust_cshell_triangulation_with(&shell, options);
        assert!(linear.faces[0].surface.is_some());
        let angular = robust_cshell_triangulation_with(
            &shell,
            TessellationOptions {
                edge_angular_tolerance: Some(0.5),
                ..options
            },
        );
        let face = angular.faces[0]
            .surface
            .as_ref()
            .expect("regular active endpoint must not drop its incident face");
        for (seed, refined) in linear.edges.iter().zip(&angular.edges) {
            assert!(seed.curve.iter().all(|point| refined.curve.contains(point)));
            assert!(
                refined
                    .curve
                    .iter()
                    .all(|point| face.positions().contains(point))
            );
        }
        for triple in angular.edges[0].curve.windows(3) {
            let a = (triple[1] - triple[0]).normalize();
            let b = (triple[2] - triple[1]).normalize();
            assert!(a.dot(b) >= 0.5_f64.cos() - 1e-12);
        }
    }
}

macro_rules! dir ( () => { concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/shape/") });

const SHAPE_JSONS: &[&str] = &[
    concat!(dir!(), "bottle.json"),
    concat!(dir!(), "punched-cube.json"),
    concat!(dir!(), "torus-punched-cube.json"),
    concat!(dir!(), "sphere.json"),
    concat!(dir!(), "torus.json"),
];

fn read_jsons() -> Vec<Vec<u8>> {
    let closure = |json| std::fs::read(json).unwrap();
    SHAPE_JSONS.iter().map(closure).collect()
}

#[test]
fn solid_is_closed() {
    for (i, json) in read_jsons().into_iter().enumerate() {
        let solid: Solid = serde_json::from_reader(json.as_slice()).unwrap();
        let mut poly = solid.triangulation(0.01).to_polygon();
        poly.put_together_same_attrs(TOLERANCE * 2.0)
            .remove_degenerate_faces()
            .remove_unused_attrs();
        assert_eq!(
            poly.shell_condition(),
            ShellCondition::Closed,
            "not closed: file no. {i}"
        );
    }
}

#[test]
fn csolid_is_closed() {
    for (i, json) in read_jsons().into_iter().enumerate() {
        let solid: CompressedSolid = serde_json::from_reader(json.as_slice()).unwrap();
        let mut poly = solid.triangulation(0.01).to_polygon();
        poly.put_together_same_attrs(TOLERANCE * 2.0)
            .remove_degenerate_faces()
            .remove_unused_attrs();
        assert_eq!(
            poly.shell_condition(),
            ShellCondition::Closed,
            "not closed: file no. {i}"
        );
    }
}

#[test]
fn compare_occt_mesh() {
    let jsons = read_jsons();
    let solid: Solid = serde_json::from_slice(jsons[2].as_slice()).unwrap();
    let res = solid.triangulation(0.005).to_polygon();
    let path = concat!(dir!(), "../obj/by_occt.obj");
    let ans = obj::read(std::fs::read(path).unwrap().as_slice()).unwrap();
    // Compare geometric coverage independently of either mesh's triangle density.
    assert!(res.neighborhood_include(ans.positions(), 0.05));
    assert!(ans.neighborhood_include(res.positions(), 0.05));
}

#[test]
fn compare_occt_mesh_csolid() {
    let jsons = read_jsons();
    let solid: CompressedSolid = serde_json::from_slice(jsons[2].as_slice()).unwrap();
    let res = solid.triangulation(0.005).to_polygon();
    let path = concat!(dir!(), "../obj/by_occt.obj");
    let ans = obj::read(std::fs::read(path).unwrap().as_slice()).unwrap();
    // Compare geometric coverage independently of either mesh's triangle density.
    assert!(res.neighborhood_include(ans.positions(), 0.05));
    assert!(ans.neighborhood_include(res.positions(), 0.05));
}

#[test]
fn large_number_meshing() {
    let json = std::fs::read(concat!(dir!(), "large-torus.json")).unwrap();
    let torus: Solid = serde_json::from_slice(json.as_slice()).unwrap();
    let _ = torus.triangulation(1.0).to_polygon();
}

fn special_cylinder_model() -> Shell {
    let v0 = builder::vertex(Point3::new(0.0, 1.0, 0.0));
    let v1 = builder::vertex(Point3::new(0.0, 1.0, 1.0));
    let v2 = builder::vertex(Point3::new(0.0, -1.0, 0.0));
    let v3 = builder::vertex(Point3::new(0.0, -1.0, 1.0));

    let edge0 = builder::line(&v0, &v1);
    let edge1 = builder::line(&v2, &v3);
    let edge2 = builder::circle_arc(&v0, &v2, Point3::new(-1.0, 0.0, 0.0));
    let edge3 = builder::circle_arc(&v2, &v0, Point3::new(1.0, 0.0, 0.0));
    let edge4 = builder::circle_arc(&v1, &v3, Point3::new(-1.0, 0.0, 1.0));
    let edge5 = builder::circle_arc(&v3, &v1, Point3::new(1.0, 0.0, 1.0));

    let face0 =
        builder::try_attach_plane(&[vec![edge2.inverse(), edge3.inverse()].into()]).unwrap();
    let face1 = builder::try_attach_plane(&[vec![edge4.clone(), edge5.clone()].into()]).unwrap();

    let surface_row = RevolutionSurface::<Curve>::by_revolution(
        Line(Point3::new(1.0, 0.0, 1.0), Point3::new(1.0, 0.0, 0.0)).into(),
        Point3::origin(),
        Vector3::unit_z(),
    );
    let surface: Surface = Processor::new(surface_row).into();

    let face2 = Face::try_new(
        vec![vec![edge2, edge1.clone(), edge4.inverse(), edge0.inverse()].into()],
        surface.clone(),
    )
    .unwrap();
    let face3 = Face::try_new(
        vec![vec![edge3, edge0, edge5.inverse(), edge1.inverse()].into()],
        surface,
    )
    .unwrap();

    vec![face0, face1, face2, face3].into()
}

#[test]
fn special_cylinder() {
    let shell = special_cylinder_model();
    let mut mesh = shell.triangulation(0.01).to_polygon();
    mesh.put_together_same_attrs(TOLERANCE)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    assert_eq!(mesh.shell_condition(), ShellCondition::Closed);
}

#[test]
fn special_cylinder_csolid() {
    let shell = special_cylinder_model().compress();
    let mut mesh = shell.triangulation(0.01).to_polygon();
    mesh.put_together_same_attrs(TOLERANCE)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    assert_eq!(mesh.shell_condition(), ShellCondition::Closed);
}

#[test]
fn robust_closed() {
    let cube: Solid = {
        let v = builder::vertex(Point3::origin());
        let e = builder::extrude(&v, Vector3::unit_x());
        let f = builder::extrude(&e, Vector3::unit_y());
        builder::extrude(&f, Vector3::unit_z())
    };

    let o = Point3::new(0.5, 0.5, 0.5);
    cube.edge_iter().for_each(|edge| {
        let curve = edge.curve();

        if let Curve::Line(line) = curve {
            let m = line.subs(0.5);
            let p = m + 0.2 * (o - m);
            let bsp = BsplineCurve::new(KnotVector::bezier_knot(2), vec![line.0, p, line.1]);
            edge.set_curve(bsp.into());
        }
    });

    assert!(
        cube.triangulation(0.01)
            .face_iter()
            .all(|face| face.surface().is_none())
    );

    let mut mesh = cube.robust_triangulation(0.01).to_polygon();
    mesh.put_together_same_attrs(TOLERANCE2)
        .remove_unused_attrs();
    assert_eq!(mesh.shell_condition(), ShellCondition::Closed);
}

/// Verifies that every face mesh from a tessellated solid has consistent
/// winding — all triangles within a face agree on the normal direction.
#[test]
fn per_face_winding_consistency() {
    let shapes: Vec<(&str, Shell)> = vec![
        ("cube", {
            let v = builder::vertex(Point3::origin());
            let e = builder::extrude(&v, Vector3::unit_x());
            let f = builder::extrude(&e, Vector3::unit_y());
            let solid: Solid = builder::extrude(&f, Vector3::unit_z());
            solid.into_boundaries().pop().unwrap()
        }),
        ("cylinder", special_cylinder_model()),
    ];
    for (name, shell) in &shapes {
        let meshed = shell.triangulation(0.01);
        for (fi, face) in meshed.face_iter().enumerate() {
            if let Some(mut poly) = face.surface() {
                if !face.orientation() {
                    poly.invert();
                }
                let positions = poly.positions();
                let normals = poly.normals();
                let (mut pos, mut neg) = (0usize, 0usize);
                for tri in poly.tri_faces() {
                    let e1 = positions[tri[1].pos] - positions[tri[0].pos];
                    let e2 = positions[tri[2].pos] - positions[tri[0].pos];
                    let geo = e1.cross(e2);
                    let vtx: Vector3 = tri.iter().fold(Vector3::zero(), |s, v| {
                        s + v.nor.map(|idx| normals[idx]).unwrap_or(Vector3::zero())
                    });
                    let dot = geo.dot(vtx);
                    if dot > 0.0 {
                        pos += 1;
                    } else if dot < 0.0 {
                        neg += 1;
                    }
                }
                assert!(
                    pos == 0 || neg == 0,
                    "{name} face {fi}: mixed winding ({pos} pos, {neg} neg)"
                );
            }
        }
    }
}

#[test]
fn tilted_cylindrical_cap_rejects_numerical_boundary_ears() {
    let radius = (6.6_f64.powi(2) + 0.65_f64.powi(2)) / (2.0 * 0.65);
    // Exact accepted bounded-cutter height: literal 0.66 misses this roundoff ear.
    let height = f64::from_bits(4604119971053405483);
    let radial = (height * (2.0 * radius - height)).sqrt();
    let left = builder::vertex(Point3::new(0.0, -radial, height));
    let right = builder::vertex(Point3::new(0.0, radial, height));
    let arc = builder::try_circle_arc(&left, &right, Point3::origin()).unwrap();
    let wire: Wire = vec![arc, builder::line(&right, &left)].into();
    let disk = builder::try_attach_plane(vec![wire]).unwrap();
    let cylinder: Solid = builder::extrude(&disk, Vector3::new(36.4, 0.0, 0.0));
    let cylinder = builder::translated(&cylinder, Vector3::new(-18.2, 0.0, 6.85));
    let cylinder = builder::rotated(
        &cylinder,
        Point3::new(0.0, 0.0, 7.5),
        Vector3::unit_x(),
        Rad(12_f64.to_radians()),
    );
    assert!(cylinder.is_geometric_consistent());
    let mut mesh = cylinder.triangulation(0.005).to_polygon();
    mesh.put_together_same_attrs(TOLERANCE * 2.0)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    assert_eq!(
        mesh.shell_condition(),
        ShellCondition::Closed,
        "curved boundary chords must not become overlapping cap ears"
    );
}
