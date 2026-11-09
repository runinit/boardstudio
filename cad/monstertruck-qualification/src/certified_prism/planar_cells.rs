//! Bounded planar cells for valid source faces; no intersection curves are made.
//! Caller must prove the supplied material states throughout each cell.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Material {
    // Canonical plane normal is +Z; entries are material below and above it.
    pub(super) inside: [bool; 2],
    pub(super) outside: [bool; 2],
}

#[derive(Clone, Copy)]
#[allow(dead_code)] // All cell rules are tested; solid assembly currently uses difference.
pub(super) enum Operation {
    Difference,
    Union,
    Intersection,
}

impl Operation {
    fn apply(self, a: bool, b: bool) -> bool {
        match self {
            Self::Difference => a && !b,
            Self::Union => a || b,
            Self::Intersection => a && b,
        }
    }
}

fn bounds(face: &Face) -> Result<(f64, Vec<P>), Unsupported> {
    let Surface::Plane(plane) = face.surface() else {
        return Err(Unsupported::Geometry);
    };
    let z = plane.origin().z;
    if !z.is_finite()
        || plane.axis_u().z != 0.0
        || plane.axis_v().z != 0.0
        || !plane.normal().z.is_finite()
        || plane.normal().z == 0.0
    {
        return Err(Unsupported::Geometry);
    }
    let mut points = Vec::new();
    for edge in face
        .absolute_boundaries()
        .iter()
        .flat_map(|w| w.edge_iter())
    {
        if !edge.is_geometric_consistent() || !cap_within_existing_tolerance(&edge.curve(), z) {
            return Err(Unsupported::Geometry);
        }
        points.extend(exact_controls(&edge.curve()).ok_or(Unsupported::Geometry)?);
    }
    if points.len() < 3 {
        return Err(Unsupported::Geometry);
    }
    Ok((z, points))
}

// This conservative whole-control-box test proves separation of filled trims.
fn separated(a: &[P], b: &[P]) -> bool {
    let tol = Q::from_float(TOLERANCE).unwrap();
    for axis in 0..2 {
        let coordinate = |p: &P| if axis == 0 { p.0.clone() } else { p.1.clone() };
        let amin = a.iter().map(coordinate).min().unwrap();
        let amax = a.iter().map(coordinate).max().unwrap();
        let bmin = b.iter().map(coordinate).min().unwrap();
        let bmax = b.iter().map(coordinate).max().unwrap();
        if bmin - amax > tol || amin - bmax > tol {
            return true;
        }
    }
    false
}

fn same_trim(a: &Face, b: &Face) -> bool {
    if a.absolute_boundaries().len() != 1 || b.absolute_boundaries().len() != 1 {
        return false;
    }
    let a = &a.absolute_boundaries()[0];
    let b = &b.absolute_boundaries()[0];
    if a.len() != b.len() {
        return false;
    }
    let mut used = HashSet::new();
    a.edge_iter().all(|edge| {
        b.edge_iter().enumerate().any(|(i, other)| {
            if used.contains(&i) {
                return false;
            }
            let mut reversed = other.curve();
            reversed.invert();
            let same = (edge.absolute_front().point() == other.absolute_front().point()
                && edge.absolute_back().point() == other.absolute_back().point()
                && same_curve(&edge.curve(), &other.curve()))
                || (edge.absolute_front().point() == other.absolute_back().point()
                    && edge.absolute_back().point() == other.absolute_front().point()
                    && same_curve(&edge.curve(), &reversed));
            if same {
                used.insert(i);
            }
            same
        })
    })
}

fn hole(outer: &Face, inner: &Face) -> Result<Face, Unsupported> {
    if inner.absolute_boundaries().len() != 1 {
        return Err(Unsupported::Containment);
    }
    let Surface::Plane(plane) = outer.surface() else {
        unreachable!()
    };
    let Surface::Plane(inner_plane) = inner.surface() else {
        unreachable!()
    };
    let inner_positive = inner_plane.normal().z.is_sign_positive() == inner.orientation();
    let mut wire = inner.boundaries().remove(0);
    if inner_positive == plane.normal().z.is_sign_positive() {
        wire.invert();
    }
    let mut wires = outer.absolute_boundaries().clone();
    wires.push(wire);
    Face::try_new(wires, outer.surface()).map_err(|_| Unsupported::Topology)
}

/// Partition two same-support face regions and retain each material transition
/// once. Curved nested trims need a straight containing face; crossed/contacting
/// trims and overlapping control boxes without a containment proof refuse.
pub(super) fn arrange(
    a: &Face,
    b: &Face,
    materials: [Material; 2],
    operation: Operation,
) -> Result<Vec<Face>, Unsupported> {
    let (za, pa) = bounds(a)?;
    let (zb, pb) = bounds(b)?;
    if za != zb {
        return Err(Unsupported::Caps);
    }
    let state = |membership: [bool; 2]| {
        let sides = [0, 1].map(|i| {
            if membership[i] {
                materials[i].inside
            } else {
                materials[i].outside
            }
        });
        [
            operation.apply(sides[0][0], sides[1][0]),
            operation.apply(sides[0][1], sides[1][1]),
        ]
    };
    // The finite source trims cannot represent an unbounded exterior sheet.
    let outside = state([false, false]);
    if outside[0] != outside[1] {
        return Err(Unsupported::Containment);
    }
    let cells = if same_trim(a, b) {
        vec![(a.clone(), [true, true])]
    } else if separated(&pa, &pb) {
        vec![(a.clone(), [true, false]), (b.clone(), [false, true])]
    } else if b.absolute_boundaries().len() == 1 && contained_hull(&pb, a) {
        vec![(hole(a, b)?, [true, false]), (b.clone(), [true, true])]
    } else if a.absolute_boundaries().len() == 1 && contained_hull(&pa, b) {
        vec![(hole(b, a)?, [false, true]), (a.clone(), [true, true])]
    } else {
        return Err(Unsupported::Containment);
    };
    let mut result = Vec::new();
    for (mut face, membership) in cells {
        let sides = state(membership);
        if sides[0] == sides[1] {
            continue;
        }
        let Surface::Plane(plane) = face.surface() else {
            unreachable!()
        };
        let positive = plane.normal().z.is_sign_positive() == face.orientation();
        if positive != sides[0] {
            face.invert();
        }
        result.push(face);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(x: f64, size: f64) -> Face {
        builder::try_attach_plane(vec![primitive::rect(
            BoundingBox::from_iter([Point2::new(x, 0.0), Point2::new(x + size, size)]),
            Plane::xy(),
        )])
        .unwrap()
    }
    fn circle(x: f64, y: f64, radius: f64) -> Face {
        let right = builder::vertex(Point3::new(x + radius, y, 0.0));
        let left = builder::vertex(Point3::new(x - radius, y, 0.0));
        builder::try_attach_plane(vec![
            vec![
                builder::circle_arc(&right, &left, Point3::new(x, y + radius, 0.0)),
                builder::circle_arc(&left, &right, Point3::new(x, y - radius, 0.0)),
            ]
            .into(),
        ])
        .unwrap()
    }
    fn cap() -> Material {
        Material {
            inside: [true, false],
            outside: [false, false],
        }
    }
    fn signed_area(faces: &[Face]) -> f64 {
        use monstertruck_meshing::tessellation::shell_to_polygon_strict;
        let mesh = shell_to_polygon_strict(&faces.to_vec().into(), 0.01).unwrap();
        mesh.faces()
            .triangle_iter()
            .map(|t| {
                let [a, b, c] = t.map(|v| mesh.positions()[v.pos]);
                (b - a).cross(c - a).z / 2.0
            })
            .sum()
    }
    #[test]
    fn nested_curved_difference_keeps_annulus_and_source_edges() {
        let a = rectangle(0.0, 10.0);
        let b = circle(5.0, 5.0, 1.0);
        let result = arrange(&a, &b, [cap(), cap()], Operation::Difference).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].absolute_boundaries().len(), 2);
        let actual: HashSet<_> = result[0]
            .absolute_boundaries()
            .iter()
            .flatten()
            .map(|e| e.id())
            .collect();
        let expected: HashSet<_> = [a, b]
            .iter()
            .flat_map(|f| f.absolute_boundaries().iter().flatten())
            .map(|e| e.id())
            .collect();
        assert_eq!(actual, expected);
        assert!((signed_area(&result) - (100.0 - std::f64::consts::PI)).abs() < 0.1);
    }
    #[test]
    fn coincident_curved_caps_have_one_owner_or_cancel() {
        let a = circle(0.0, 0.0, 1.0);
        let b = circle(0.0, 0.0, 1.0);
        let union = arrange(&a, &b, [cap(), cap()], Operation::Union).unwrap();
        assert_eq!(union.len(), 1);
        assert_eq!(
            union[0].absolute_boundaries()[0][0].id(),
            a.absolute_boundaries()[0][0].id()
        );
        assert!(
            arrange(&a, &b, [cap(), cap()], Operation::Difference)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            arrange(&a, &b, [cap(), cap()], Operation::Intersection)
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn disjoint_curved_caps_do_not_get_omitted() {
        let a = circle(0.0, 0.0, 1.0);
        let b = circle(4.0, 0.0, 1.0);
        let union = arrange(&a, &b, [cap(), cap()], Operation::Union).unwrap();
        assert_eq!(union.len(), 2);
        assert!((signed_area(&union) - 2.0 * std::f64::consts::PI).abs() < 0.1);
        assert!(
            arrange(&a, &b, [cap(), cap()], Operation::Intersection)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn cavity_floor_uses_material_on_both_sides() {
        let a = rectangle(0.0, 10.0);
        let b = circle(5.0, 5.0, 1.0);
        let material = Material {
            inside: [true, true],
            outside: [true, true],
        };
        let cutter = Material {
            inside: [false, true],
            outside: [false, false],
        };
        let floor = arrange(&a, &b, [material, cutter], Operation::Difference).unwrap();
        assert_eq!(floor.len(), 1);
        assert!((signed_area(&floor) - std::f64::consts::PI).abs() < 0.1);
        let roof = arrange(&a, &b, [material, cap()], Operation::Difference).unwrap();
        assert!((signed_area(&roof) + std::f64::consts::PI).abs() < 0.1);
    }
    #[test]
    fn crossing_contact_and_uncertified_nested_curves_refuse() {
        let a = rectangle(0.0, 10.0);
        for b in [circle(10.0, 5.0, 1.0), circle(9.0, 5.0, 1.0)] {
            assert!(matches!(
                arrange(&a, &b, [cap(), cap()], Operation::Difference),
                Err(Unsupported::Containment)
            ));
        }
        assert!(
            arrange(
                &circle(0.0, 0.0, 3.0),
                &circle(0.0, 0.0, 1.0),
                [cap(), cap()],
                Operation::Difference
            )
            .is_err()
        );
    }
    #[test]
    fn nested_union_and_intersection_have_no_missing_or_duplicate_material() {
        let a = rectangle(0.0, 10.0);
        let b = circle(5.0, 5.0, 1.0);
        let union = arrange(&a, &b, [cap(), cap()], Operation::Union).unwrap();
        assert!((signed_area(&union) - 100.0).abs() < 0.1);
        let mut uses = std::collections::HashMap::new();
        for face in &union {
            for edge in face.boundaries().iter().flatten() {
                let entry = uses.entry(edge.id()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += if edge.orientation() { 1 } else { -1 };
            }
        }
        for edge in b.absolute_boundaries().iter().flatten() {
            assert_eq!(uses[&edge.id()], (2, 0));
        }
        let intersection = arrange(&b, &a, [cap(), cap()], Operation::Intersection).unwrap();
        assert!((signed_area(&intersection) - std::f64::consts::PI).abs() < 0.1);
        assert!(
            arrange(&b, &a, [cap(), cap()], Operation::Difference)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn unsupported_projected_leader_is_not_a_whole_curve_certificate() {
        let a = rectangle(0.0, 10.0);
        let b = rectangle(2.0, 1.0);
        let edge = b.absolute_boundaries()[0][0].clone();
        let leader = edge.curve();
        edge.set_curve(Curve::IntersectionCurve(SurfaceCurve::with_boundaries(
            Box::new(b.surface()),
            Box::new(b.surface()),
            Box::new(leader),
            None,
            None,
        )));
        assert!(matches!(
            arrange(&a, &b, [cap(), cap()], Operation::Difference),
            Err(Unsupported::Geometry)
        ));
    }
    #[test]
    fn unbounded_material_transition_and_tolerance_near_gap_refuse() {
        let a = rectangle(0.0, 1.0);
        let b = rectangle(1.0 + 0.5 * TOLERANCE, 1.0);
        assert!(arrange(&a, &b, [cap(), cap()], Operation::Union).is_err());
        let b = circle(0.5, 0.5, 0.1);
        let unbounded = Material {
            inside: [true, false],
            outside: [true, false],
        };
        assert!(arrange(&a, &b, [unbounded, cap()], Operation::Union).is_err());
    }
    #[test]
    fn distinct_support_planes_are_not_merged_within_tolerance() {
        let a = rectangle(0.0, 10.0);
        let b = builder::translated(&circle(5.0, 5.0, 1.0), Vector3::unit_z() * 1e-10);
        assert!(matches!(
            arrange(&a, &b, [cap(), cap()], Operation::Difference),
            Err(Unsupported::Caps)
        ));
    }
}
