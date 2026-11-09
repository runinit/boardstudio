use super::*;
use num_traits::One;
use query::{ContactRegion, Containment, Limits};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::console_log as println;

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../wasm/tests/fixtures/generated-analysis-contract.json"
    ))
    .unwrap()
}
fn capacitor() -> ImportedStep {
    crate::import_step(
        include_bytes!(
            "../../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step"
        )
        .to_vec(),
    )
    .unwrap()
}

#[test]
fn complete_nonmanifold_surface_preview_has_no_private_source_qualification() {
    let model = crate::import_step(
        include_bytes!("../../tests/fixtures/touching-unit-boxes.step").to_vec(),
    )
    .unwrap();
    assert_eq!(
        qualify(
            &model,
            model.preview_positions_f64(),
            model.triangle_faces(),
            IDENTITY
        )
        .err(),
        Some("unsupported nonmanifold or unoriented source surface")
    );
}
fn qualified(m: &ImportedStep, row: [f64; 16]) -> QualifiedSurface {
    qualify(m, m.preview_positions_f64(), m.triangle_faces(), row).unwrap()
}
fn offset(x: f64, y: f64, z: f64) -> [f64; 16] {
    let mut m = IDENTITY;
    m[12] = x;
    m[13] = y;
    m[14] = z;
    m
}
fn native_offset() -> [f64; 16] {
    let f = fixture();
    let t = &f["qualificationV2"]["sourcePlacement"]["privateSuppliedWorldTranslationMm"];
    offset(
        t[0].as_f64().unwrap(),
        t[1].as_f64().unwrap(),
        t[2].as_f64().unwrap(),
    )
}
fn pair_contains(boxes: &[query::BoxPair], a: FPoint, b: FPoint) -> bool {
    boxes.iter().any(|pair| {
        [a, b]
            .into_iter()
            .enumerate()
            .all(|(side, p)| (0..3).all(|i| pair[side][0][i] <= p[i] && p[i] <= pair[side][1][i]))
    })
}
fn triangle(z: f64) -> FTriangle {
    [[0., 0., z], [2., 0., z], [0., 2., z]]
}
fn cube(lo: f64, hi: f64) -> Vec<FTriangle> {
    let p = [
        [lo, lo, lo],
        [hi, lo, lo],
        [hi, hi, lo],
        [lo, hi, lo],
        [lo, lo, hi],
        [hi, lo, hi],
        [hi, hi, hi],
        [lo, hi, hi],
    ];
    [
        [0, 2, 1],
        [0, 3, 2],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [1, 2, 6],
        [1, 6, 5],
        [2, 3, 7],
        [2, 7, 6],
        [3, 0, 4],
        [3, 4, 7],
    ]
    .map(|t| t.map(|i| p[i]))
    .to_vec()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn complete_capacitor_final_mesh_has_a_symmetric_source_bound() {
    let model = capacitor();
    let q = qualified(&model, IDENTITY);
    let f = fixture();
    let expected = &f["qualificationV2"];
    assert_eq!(
        model.solid_count(),
        expected["expectedCensus"]["solids"].as_u64().unwrap() as usize
    );
    assert_eq!(model.source_faces().len(), 30);
    assert_eq!(model.face_occurrences().len(), 30);
    assert_eq!(q.tags.iter().copied().collect::<BTreeSet<_>>().len(), 30);
    assert!(q.bound.direct_mm <= expected["maximumDirectBoundMm"].as_f64().unwrap());
    assert!(Q::from_float(q.bound.precision_mm).unwrap() >= decimal("0.01").unwrap());
    assert!(q.bound.total_mm >= q.bound.precision_mm);
    assert_eq!(q.repair_displacement_mm, None);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert_eq!(
        model.accuracy().analysis_error_bound_mm,
        None,
        "private evidence never activates a production certificate"
    );
    assert_eq!(q.transform_rounding_mm, 0.0);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn capacitor_pair_uses_slab_and_keeps_broad_nonunique_contacts() {
    let m = capacitor();
    let b = qualified(&m, IDENTITY);
    let a = qualified(&m, native_offset());
    let f = fixture();
    let expected = &f["qualificationV2"]["pair"];
    let result = query::distance(
        &a.triangles,
        &b.triangles,
        [0, 0],
        Some([&a, &b]),
        Limits {
            max_contact_boxes: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(result.mm[0] >= 1.0 - 1e-10 && result.mm[1] <= 1.0 + 1e-10);
    assert!(result.mm[1] - result.mm[0] <= 1e-10);
    assert!(result.witness[0] < a.triangles.len() && result.witness[1] < b.triangles.len());
    assert!(
        result.exact_pairs <= result.limits.max_exact_pairs
            && result.node_pairs <= result.limits.max_node_pairs
    );
    assert!(
        matches!(query::slab(&a,&b).unwrap(),Containment::RuledOutSlab{axis:1,left_upper,right_lower} if left_upper<right_lower)
    );
    assert!(
        matches!(query::slab(&a, &a).unwrap(), Containment::Unknown(_)),
        "touching capacitor constituents never imply one-shell parity"
    );
    let ContactRegion::Boxes(boxes) = result.contact else {
        panic!("expected bounded contact region")
    };
    assert_eq!(boxes.len(), 1);
    assert!(pair_contains(&boxes, [-0.625, -2., 0.], [-0.625, -1., 0.]));
    assert!(pair_contains(
        &boxes,
        [0.625, -2., 1.35],
        [0.625, -1., 1.35]
    ));
    assert!(
        query::distance(
            &a.triangles,
            &b.triangles,
            [0, 0],
            Some([&b, &a]),
            Limits::default(),
        )
        .is_err()
    );
    let half_width = (result.mm[1] - result.mm[0]) / 2.0 + a.bound.total_mm + b.bound.total_mm;
    assert!(
        result.mm[0] - a.bound.total_mm - b.bound.total_mm <= 1.0
            && result.mm[1] + a.bound.total_mm + b.bound.total_mm >= 1.0,
        "nominal source gap is enclosed after normalization and floating transform effects"
    );
    println!(
        "capacitor direct bounds {:?}/{:?}, mesh interval {:?}, source half-width {}, exact/node pairs {}/{}",
        a.bound, b.bound, result.mm, half_width, result.exact_pairs, result.node_pairs
    );
    assert!(
        half_width > expected["tightRequestedAccuracyMm"].as_f64().unwrap(),
        "tight source accuracy must remain uncertain"
    );
    assert!(
        result.mm[0] - a.bound.total_mm - b.bound.total_mm
            > expected["clearanceThresholdMm"].as_f64().unwrap()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn thirty_degree_affine_control_counts_exact_rounding_once() {
    let m = capacitor();
    let mut row = IDENTITY;
    let c = fixture()["qualificationV2"]["rotatedControl"]["cos30F64"]
        .as_f64()
        .unwrap();
    row[0] = c;
    row[1] = 0.5;
    row[4] = -0.5;
    row[5] = c;
    let b = qualified(&m, row);
    let t = native_offset();
    row[12] = c * t[12] - 0.5 * t[13];
    row[13] = 0.5 * t[12] + c * t[13];
    let a = qualified(&m, row);
    assert!(a.transform_rounding_mm > 0.0 && a.transform_rounding_mm < 1e-12);
    assert!(a.bound.direct_mm < 1e-10 && b.bound.direct_mm < 1e-10);
    let result = query::distance(
        &a.triangles,
        &b.triangles,
        [0, 0],
        Some([&a, &b]),
        Limits::default(),
    )
    .unwrap();
    assert!(result.mm[0] >= 1. - 1e-10 && result.mm[1] <= 1. + 1e-10);
    assert!(matches!(
        query::slab(&a, &b).unwrap(),
        Containment::RuledOutSlab { axis: 1, .. }
    ));
    let source_errors = a.bound.total_mm + b.bound.total_mm;
    assert!(result.mm[0] - source_errors < 1.0 && result.mm[1] + source_errors > 1.0);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn finite_source_band_does_not_invent_a_world_slab_after_rotation() {
    let m = capacitor();
    let mut row = IDENTITY;
    let c = 0.5f64.sqrt();
    row[0] = c;
    row[1] = c;
    row[4] = -c;
    row[5] = c;
    let b = qualified(&m, row);
    let t = native_offset();
    row[12] = c * (t[12] - t[13]);
    row[13] = c * (t[12] + t[13]);
    let a = qualified(&m, row);
    assert!(a.bound.total_mm.is_finite() && b.bound.total_mm.is_finite());
    assert!(matches!(
        query::slab(&a, &b).unwrap(),
        Containment::Unknown("no separating slab; parity is unqualified")
    ));
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn original_frame_order_normalization_and_origins_are_independently_checked() {
    let m = capacitor();
    let text = std::str::from_utf8(m.original_bytes()).unwrap();
    let source = source::read(m.original_bytes()).unwrap();
    assert!(source.placement[0] > Q::zero() && source.placement[1] < Q::zero());
    assert!(source.placement[4] > Q::zero());
    assert!(source.placement_error_per_radius >= Q::zero());
    let reversed = text.replace(
        "ITEM_DEFINED_TRANSFORMATION($,$,#552,#583)",
        "ITEM_DEFINED_TRANSFORMATION($,$,#583,#552)",
    );
    assert!(
        source::read(reversed.as_bytes()).is_err(),
        "frame representation order authenticates composition"
    );
    let bad_axis = text.replace(
        "#712=DIRECTION('',(0.,0.,1.))",
        "#712=DIRECTION('',(0.,1.,0.))",
    );
    assert!(source::read(bad_axis.as_bytes()).is_err());
    let a = decimal("2.4980018054066e-16").unwrap();
    let n = Q::one() + &a * &a;
    let f = fixture();
    let intervals = &f["qualificationV2"]["sourcePlacement"]["expectedWorldNativeXTranslationMm"];
    let lo = Q::from_float(intervals[0][0].as_f64().unwrap()).unwrap();
    let hi = Q::from_float(intervals[0][1].as_f64().unwrap()).unwrap();
    let numerator = decimal("9").unwrap() * &a * &a;
    assert!(&lo * &lo * &n <= numerator && &hi * &hi * &n >= numerator);
    let moved = text
        .replace(
            "#715=CARTESIAN_POINT('',(0.,0.,0.))",
            "#715=CARTESIAN_POINT('',(1.,0.,0.))",
        )
        .replace(
            "#850=CARTESIAN_POINT('',(0.,0.,0.))",
            "#850=CARTESIAN_POINT('',(3.,4.,0.))",
        );
    let moved = crate::import_step(moved.into_bytes()).unwrap();
    let q = qualified(&moved, IDENTITY);
    assert!(q.bound.direct_mm < 1e-10);
    assert!(q.source_hull[0][1] > decimal("3.9").unwrap());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn final_triangle_loss_tags_and_support_edits_refuse_qualification() {
    let m = capacitor();
    let positions = m.preview_positions_f64();
    let tags = m.triangle_faces();
    for removed in 0..30 {
        let p = positions
            .as_chunks::<3>()
            .0
            .iter()
            .zip(tags)
            .filter(|(_, tag)| **tag != removed)
            .flat_map(|(p, _)| p.iter().copied())
            .collect::<Vec<_>>();
        let t = tags
            .iter()
            .copied()
            .filter(|tag| *tag != removed)
            .collect::<Vec<_>>();
        assert!(
            qualify(&m, &p, &t, IDENTITY).is_err(),
            "face {removed} was lost"
        );
    }
    assert!(
        qualify(&m, &positions[3..], &tags[1..], IDENTITY).is_err(),
        "missing triangle strip cannot be certified"
    );
    let mut t = tags.to_vec();
    t[0] = usize::MAX;
    assert!(qualify(&m, positions, &t, IDENTITY).is_err());
    let mut p = positions.to_vec();
    p[0][0] += 0.02;
    assert!(qualify(&m, &p, tags, IDENTITY).is_err());
    p[0][0] = f64::NAN;
    assert!(qualify(&m, &p, tags, IDENTITY).is_err());
    let text = String::from_utf8(m.original_bytes().to_vec()).unwrap();
    assert!(source::read(text.replace("PLANE(", "SPHERICAL_SURFACE(").as_bytes()).is_err());
    assert!(
        source::read(
            text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)")
                .as_bytes()
        )
        .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn original_decimal_authority_and_sqrt_bounds_are_exact() {
    let q = decimal("1.000000000000000000000000000001").unwrap();
    assert!(q > Q::one());
    assert_ne!(
        q,
        Q::from_float("1.000000000000000000000000000001".parse::<f64>().unwrap()).unwrap()
    );
    for q in [
        Q::zero(),
        Q::one(),
        decimal("2").unwrap(),
        decimal("1e-64").unwrap(),
        decimal("1e64").unwrap(),
    ] {
        let b = sqrt_bounds(&q).unwrap();
        let lo = Q::from_float(b[0]).unwrap();
        let hi = Q::from_float(b[1]).unwrap();
        assert!(&lo * &lo <= q && &hi * &hi >= q);
        assert!(b[0] == b[1] || b[0].next_up() == b[1]);
    }
    assert!(decimal("1e1000").is_err());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn mesh_distance_checks_separated_touching_crossing_coplanar_and_nested() {
    let a = triangle(0.0);
    let crossing = [[0.5, 0.5, -1.], [0.5, 0.5, 1.], [1.5, 0.5, 0.]];
    for (b, expected) in [
        (triangle(1.), 1.),
        (triangle(0.), 0.),
        (crossing, 0.),
        ([[0.25, 0.25, 0.], [1.25, 0.25, 0.], [0.25, 1.25, 0.]], 0.),
    ] {
        let d = query::distance(&[a], &[b], [0, 0], None, Limits::default()).unwrap();
        assert!(d.mm[0] <= expected && d.mm[1] >= expected);
        assert!(matches!(
            d.contact,
            ContactRegion::Unknown("source certificates unavailable")
        ));
    }
    let nested = query::distance(
        &cube(0., 10.),
        &cube(3., 7.),
        [0, 0],
        None,
        Limits::default(),
    )
    .unwrap();
    assert!(
        nested.mm[0] <= 3. && nested.mm[1] >= 3.,
        "surface separation is not absence of containment"
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn exact_features_cover_oblique_parallel_edges_and_square_root_two() {
    let a = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let b = [[2., 1., 0.], [3., 1., 0.], [2., 2., 0.]];
    assert_eq!(
        distance2(&a.map(|p| point(p).unwrap()), &b.map(|p| point(p).unwrap())).unwrap(),
        decimal("2").unwrap()
    );
    let d = query::distance(&[a], &[b], [0, 0], None, Limits::default()).unwrap();
    let lo = Q::from_float(d.mm[0]).unwrap();
    let hi = Q::from_float(d.mm[1]).unwrap();
    assert!(&lo * &lo <= decimal("2").unwrap() && &hi * &hi >= decimal("2").unwrap());
    let near = [[0., 0., 1e-12], [1., 0., 1.00001e-12], [0., 1., 1e-12]];
    let d = query::distance(&[a], &[near], [0, 0], None, Limits::default()).unwrap();
    assert!(d.mm[0] > 0. && d.mm[1] >= 1e-12);
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_seeds_nonfinite_degenerates_caps_and_tighter_numeric_requests_refuse() {
    let a = triangle(0.);
    let b = triangle(1.);
    assert!(query::distance(&[a], &[b], [1, 0], None, Limits::default()).is_err());
    for t in [
        [[f64::INFINITY, 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        [[0.; 3]; 3],
        [[2f64.powi(129), 0., 0.], [1., 0., 0.], [0., 1., 0.]],
    ] {
        assert!(query::distance(&[t], &[b], [0, 0], None, Limits::default()).is_err());
    }
    assert!(
        query::distance(
            &[a],
            &[b],
            [0, 0],
            None,
            Limits {
                max_triangles: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let large = triangle(0.).map(|p| [p[0] + 1e15, p[1] + 1e15, p[2]]);
    let shifted = large.map(|p| [p[0], p[1], 1.]);
    let d = query::distance(&[large], &[shifted], [0, 0], None, Limits::default()).unwrap();
    assert!(d.mm[0] <= 1. && d.mm[1] >= 1.);
    let x = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    let y = [[2., 1., 0.], [3., 1., 0.], [2., 2., 0.]];
    assert!(
        query::distance(
            &[x],
            &[y],
            [0, 0],
            None,
            Limits {
                numerical_width_mm: 1e-30,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        query::distance(
            &cube(0., 10.),
            &cube(3., 7.),
            [0, 0],
            None,
            Limits {
                max_node_pairs: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        query::distance(
            &[triangle(0.), triangle(10.)],
            &[triangle(10.)],
            [0, 0],
            None,
            Limits {
                max_exact_pairs: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn filled_hole_and_duplicate_face_triangles_do_not_gain_coverage_or_containment() {
    let p = |x, y| point([x, y, 0.]).unwrap();
    let loops = vec![
        vec![p(0., 0.), p(4., 0.), p(4., 4.), p(0., 4.)],
        vec![p(1., 1.), p(1., 3.), p(3., 3.), p(3., 1.)],
    ];
    let filled = [
        [[0., 0., 0.], [4., 0., 0.], [4., 4., 0.]],
        [[0., 0., 0.], [4., 4., 0.], [0., 4., 0.]],
    ]
    .map(|t| t.map(|p| point(p).unwrap()));
    assert!(source::face_bound(&loops, &filled).is_err());
    assert!(
        source::face_bound(
            &loops[..1],
            &[filled[0].clone(), filled[1].clone(), filled[0].clone()]
        )
        .is_err()
    );
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn missing_source_precision_and_extreme_decimal_exponents_refuse() {
    let bytes = include_str!(
        "../../../../ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step"
    );
    assert_eq!(
        source::read(
            bytes
                .replace(
                    "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT",
                    "UNQUALIFIED_UNCERTAINTY_CONTEXT"
                )
                .as_bytes()
        )
        .err(),
        Some("missing source precision")
    );
    for (replacement, expected) in [
        ("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT()", "source arity"),
        ("GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT(#852)", "source list"),
        (
            "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#999999))",
            "missing source record",
        ),
    ] {
        let original = "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#852))";
        assert_eq!(bytes.matches(original).count(), 1);
        assert_eq!(
            source::read(bytes.replace(original, replacement).as_bytes()).err(),
            Some(expected)
        );
    }
    assert!(
        source::read(
            bytes
                .replace("GLOBAL_UNIT_ASSIGNED_CONTEXT", "UNQUALIFIED_UNIT_CONTEXT")
                .as_bytes()
        )
        .is_err()
    );
    assert!(decimal("1e-2147483648").is_err());
}

fn box_surface(hull: [FPoint; 2]) -> QualifiedSurface {
    let precision = 1e-30;
    QualifiedSurface {
        triangles: cube(0., 1.)
            .into_iter()
            .map(|t| t.map(|p| std::array::from_fn(|i| hull[usize::from(p[i] == 1.)][i])))
            .collect(),
        tags: vec![0; 12],
        bound: SurfaceBound {
            direct_mm: 0.,
            precision_mm: precision,
            total_mm: precision,
        },
        source_hull: hull.map(|p| point(p).unwrap()),
        transform_rounding_mm: 0.,
        repair_displacement_mm: None,
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn slab_rejects_rounded_endpoints_that_reverse_or_collapse() {
    let a = box_surface([[0.; 3], [1.; 3]]);
    for (minimum, expected_right) in [
        (1f64.next_up(), 1.),
        (1f64.next_up().next_up(), 1f64.next_up()),
    ] {
        let b = box_surface([[minimum, 0., 0.], [2., 1., 1.]]);
        let left = &a.source_hull[1][0] + Q::from_float(a.bound.total_mm).unwrap();
        let right = &b.source_hull[0][0] - Q::from_float(b.bound.total_mm).unwrap();
        assert!(left < right, "exact inflated hulls are strictly separated");
        assert_eq!(upper(&left).unwrap(), 1f64.next_up());
        assert_eq!(-upper(&(-right)).unwrap(), expected_right);
        assert!(matches!(
            query::slab(&a, &b).unwrap(),
            Containment::Unknown(_)
        ));
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn slab_requires_strict_emitted_separation_and_tries_other_axes() {
    let a = box_surface([[0.; 3], [1.; 3]]);
    let minimum = 1f64.next_up().next_up().next_up();
    let b = box_surface([[minimum, 0., 0.], [2., 1., 1.]]);
    assert_eq!(
        query::slab(&a, &b).unwrap(),
        Containment::RuledOutSlab {
            axis: 0,
            left_upper: 1f64.next_up(),
            right_lower: minimum.next_down(),
        }
    );
    let b = box_surface([[1f64.next_up(), 2., 0.], [2., 3., 1.]]);
    let proof = query::slab(&a, &b).unwrap();
    assert_eq!(
        proof,
        Containment::RuledOutSlab {
            axis: 1,
            left_upper: 1f64.next_up(),
            right_lower: 2f64.next_down(),
        }
    );
    if let Containment::RuledOutSlab {
        left_upper,
        right_lower,
        ..
    } = proof
    {
        assert!(left_upper < right_lower);
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn slab_refuses_touching_and_overlapping_source_hulls() {
    let a = box_surface([[0.; 3], [1.; 3]]);
    for minimum in [0.5, 1.] {
        let b = box_surface([[minimum, 0., 0.], [2., 1., 1.]]);
        assert!(matches!(
            query::slab(&a, &b).unwrap(),
            Containment::Unknown(_)
        ));
    }
}
