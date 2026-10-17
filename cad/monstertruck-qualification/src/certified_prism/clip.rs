//! Trim a certified source extrusion by horizontal sections, retaining wall supports.
use super::*;
use monstertruck_geometry::prelude::{
    TryIntoHomogeneousBsplineCurve, TryIntoHomogeneousBsplineSurface,
};
use std::collections::HashMap;

fn section_on_source(curve: &Curve, wall: &Surface, fraction: &Q, z: f64) -> bool {
    if exact_controls(curve).is_none() {
        return false;
    }
    let Some(net) = wall.try_into_homogeneous_bspline_surface() else {
        return false;
    };
    let Some(section) = curve.try_into_homogeneous_bspline_curve() else {
        return false;
    };
    if net.vdegree() != 1
        || net.knot_vector_v() != &KnotVector::bezier_knot(1)
        || net.knot_vector_u() != section.knot_vector()
        || net.control_points().len() != section.control_points().len()
        || !cap_within_existing_tolerance(curve, z)
    {
        return false;
    }
    let one = Q::from_integer(1.into());
    let tolerance = Q::from_float(TOLERANCE).unwrap();
    net.control_points()
        .iter()
        .zip(section.control_points())
        .all(|(row, q)| {
            if row.len() != 2 || row[0].w != row[1].w || row[0].w != q.w || q.w <= 0.0 {
                return false;
            }
            let Some(weight) = Q::from_float(q.w) else {
                return false;
            };
            let error: Option<Q> = (0..3)
                .map(|i| {
                    let exact = (&one - fraction) * Q::from_float(row[0][i])?
                        + fraction * Q::from_float(row[1][i])?;
                    let delta = Q::from_float(q[i])? - exact;
                    Some(&delta * &delta)
                })
                .sum();
            error.is_some_and(|e| e <= &tolerance * &tolerance * &weight * &weight)
        })
}

pub(super) fn trim(cutter: &Solid, certificate: &Prism, z: [f64; 2]) -> Result<Solid, Unsupported> {
    if z == certificate.z {
        return Ok(cutter.clone());
    }
    if z[0] < certificate.z[0] || z[1] > certificate.z[1] || z[0] >= z[1] {
        return Err(Unsupported::Caps);
    }
    let shell = &cutter.boundaries()[0];
    let lower = &shell[certificate.caps[0]];
    let upper = &shell[certificate.caps[1]];
    let whole =
        Matrix4::from_translation(Vector3::unit_z() * (certificate.z[1] - certificate.z[0]));
    let mut vertices = HashMap::new();
    let mut edges = HashMap::new();
    for edge in lower
        .absolute_boundaries()
        .iter()
        .flat_map(|w| w.edge_iter())
    {
        let translated = edge.curve().transformed(whole);
        let top = upper
            .absolute_boundaries()
            .iter()
            .flat_map(|w| w.edge_iter())
            .find(|u| {
                same_curve(&translated, &u.curve())
                    && u.absolute_front().point()
                        == whole.transform_point(edge.absolute_front().point())
                    && u.absolute_back().point()
                        == whole.transform_point(edge.absolute_back().point())
            })
            .ok_or(Unsupported::Geometry)?;
        let wall = shell
            .iter()
            .enumerate()
            .find(|(i, f)| {
                !certificate.caps.contains(i)
                    && f.absolute_boundaries()
                        .iter()
                        .flatten()
                        .any(|e| e.id() == edge.id())
            })
            .ok_or(Unsupported::Topology)?
            .1;
        for (bottom_vertex, top_vertex) in [
            (edge.absolute_front(), top.absolute_front()),
            (edge.absolute_back(), top.absolute_back()),
        ] {
            vertices.entry(bottom_vertex.id()).or_insert_with(|| {
                if z[0] == certificate.z[0] {
                    bottom_vertex.clone()
                } else {
                    Vertex::new(
                        bottom_vertex.point() + Vector3::unit_z() * (z[0] - certificate.z[0]),
                    )
                }
            });
            vertices.entry(top_vertex.id()).or_insert_with(|| {
                if z[1] == certificate.z[1] {
                    top_vertex.clone()
                } else {
                    Vertex::new(
                        bottom_vertex.point() + Vector3::unit_z() * (z[1] - certificate.z[0]),
                    )
                }
            });
        }
        for (k, source) in [(0, edge), (1, top)] {
            let replacement = if z[k] == certificate.z[k] {
                source.absolute_clone()
            } else {
                let curve = edge.curve().transformed(Matrix4::from_translation(
                    Vector3::unit_z() * (z[k] - certificate.z[0]),
                ));
                let fraction = (Q::from_float(z[k]).unwrap()
                    - Q::from_float(certificate.z[0]).unwrap())
                    / (Q::from_float(certificate.z[1]).unwrap()
                        - Q::from_float(certificate.z[0]).unwrap());
                if !section_on_source(&curve, &wall.surface(), &fraction, z[k]) {
                    return Err(Unsupported::Geometry);
                }
                Edge::try_new(
                    &vertices[&source.absolute_front().id()],
                    &vertices[&source.absolute_back().id()],
                    curve,
                )
                .map_err(|_| Unsupported::Topology)?
            };
            if !replacement.is_geometric_consistent() {
                return Err(Unsupported::Geometry);
            }
            edges.insert(source.id(), replacement);
        }
    }
    // The source extrusion certificate proves these remaining edges are vertical
    // connectors; source incidence, not rounded coordinates, keys their children.
    for edge in shell.edge_iter() {
        if edges.contains_key(&edge.id()) {
            continue;
        }
        let front = vertices
            .get(&edge.absolute_front().id())
            .ok_or(Unsupported::Topology)?;
        let back = vertices
            .get(&edge.absolute_back().id())
            .ok_or(Unsupported::Topology)?;
        let Curve::Line(line) = edge.curve() else {
            return Err(Unsupported::Geometry);
        };
        if line.0.x != line.1.x
            || line.0.y != line.1.y
            || [front, back].into_iter().any(|v| {
                v.point().x != line.0.x
                    || v.point().y != line.0.y
                    || v.point().z < line.0.z.min(line.1.z)
                    || v.point().z > line.0.z.max(line.1.z)
            })
        {
            return Err(Unsupported::Geometry);
        }
        let replacement =
            if front.id() == edge.absolute_front().id() && back.id() == edge.absolute_back().id() {
                edge.absolute_clone()
            } else {
                Edge::try_new(front, back, Curve::Line(Line(front.point(), back.point())))
                    .map_err(|_| Unsupported::Topology)?
            };
        edges.insert(edge.id(), replacement);
    }
    let faces = shell
        .iter()
        .enumerate()
        .map(|(i, face)| {
            let wires: Vec<Wire> = face
                .absolute_boundaries()
                .iter()
                .map(|w| {
                    w.edge_iter()
                        .map(|edge| {
                            let mut child = edges[&edge.id()].clone();
                            if !edge.orientation() {
                                child.invert();
                            }
                            child
                        })
                        .collect()
                })
                .collect();
            let surface = if let Some(k) = certificate.caps.iter().position(|&j| j == i) {
                let Surface::Plane(plane) = face.surface() else {
                    return Err(Unsupported::Geometry);
                };
                let origin = Point3::new(plane.origin().x, plane.origin().y, z[k]);
                Surface::Plane(Plane::new(
                    origin,
                    origin + plane.axis_u(),
                    origin + plane.axis_v(),
                ))
            } else {
                face.surface()
            };
            let mut result = Face::try_new(wires, surface).map_err(|_| Unsupported::Topology)?;
            if !face.orientation() {
                result.invert();
            }
            Ok(result)
        })
        .collect::<Result<Vec<_>, Unsupported>>()?;
    Solid::try_new(vec![faces.into()]).map_err(|_| Unsupported::Topology)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn section_certificate_checks_entire_source_surface_not_just_plane_and_endpoints() {
        let profile = Curve::BsplineCurve(BsplineCurve::new(
            KnotVector::bezier_knot(2),
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
        ));
        let wall: Surface =
            ExtrusionSurface::by_extrusion(profile.clone(), Vector3::unit_z() * 4.0)
                .to_same_geometry();
        let section = profile.transformed(Matrix4::from_translation(Vector3::unit_z() * 2.0));
        let half = Q::new(1.into(), 2.into());
        assert!(section_on_source(&section, &wall, &half, 2.0));
        let Curve::BsplineCurve(mut bowed) = section else {
            unreachable!()
        };
        bowed.transform_control_points(|p| {
            if p.x == 1.0 {
                p.y += 1e-4
            }
        });
        let bowed = Curve::BsplineCurve(bowed);
        assert!(cap_within_existing_tolerance(&bowed, 2.0));
        assert!(!section_on_source(&bowed, &wall, &half, 2.0));
    }
}
