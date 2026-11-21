use super::{List, Ref, Writer, geometry::Real, logical};
use crate::layered::brep::{Face, Point3, Support, Topology, Vector3};
use cgmath::InnerSpace;
use std::collections::BTreeMap;

impl Writer {
    pub(super) fn generated_solid(&mut self, topology: &Topology, faces: &[Face]) -> Ref {
        let mut vertices = BTreeMap::new();
        let mut supports = BTreeMap::<usize, Vec<Ref>>::new();
        let surfaces: Vec<_> = faces
            .iter()
            .map(|face| self.generated_support(face.support))
            .collect();
        for (face, &(surface, _)) in faces.iter().zip(&surfaces) {
            for edge in face.rings.iter().flatten() {
                supports.entry(edge.index).or_default().push(surface);
            }
        }
        let mut edges = BTreeMap::new();
        for (index, supports) in supports {
            let edge = topology.edges[index];
            let ends = edge.vertices.map(|index| {
                *vertices.entry(index).or_insert_with(|| {
                    let point = self.point(topology.vertices[index]);
                    self.entity(format_args!("VERTEX_POINT('', {point})"))
                })
            });
            let [a, b] = edge.vertices.map(|index| topology.vertices[index]);
            let carrier = if let Some(circle) = edge.circle {
                self.circle(circle.center, circle.radius, circle.sweep.signum(), a)
            } else {
                let line = self.line(a, b);
                self.entity(format_args!(
                    "SURFACE_CURVE('', {line}, {}, .CURVE_3D.)",
                    List(&supports)
                ))
            };
            let [a, b] = ends;
            edges.insert(
                index,
                self.entity(format_args!("EDGE_CURVE('', {a}, {b}, {carrier}, .T.)")),
            );
        }
        let faces = faces
            .iter()
            .zip(surfaces)
            .map(|(face, (surface, same_sense))| {
                let bounds: Vec<_> = face
                    .rings
                    .iter()
                    .map(|ring| {
                        let uses: Vec<_> = ring
                            .iter()
                            .map(|edge| {
                                let curve = edges[&edge.index];
                                let sense = logical(edge.forward);
                                self.entity(format_args!(
                                    "ORIENTED_EDGE('', *, *, {curve}, {sense})"
                                ))
                            })
                            .collect();
                        let wire = self.entity(format_args!("EDGE_LOOP('', {})", List(&uses)));
                        let sense = logical(face.forward);
                        self.entity(format_args!("FACE_BOUND('', {wire}, {sense})"))
                    })
                    .collect();
                let sense = logical(face.forward == same_sense);
                self.entity(format_args!(
                    "ADVANCED_FACE('', {}, {surface}, {sense})",
                    List(&bounds)
                ))
            })
            .collect::<Vec<_>>();
        let shell = self.entity(format_args!("CLOSED_SHELL('', {})", List(&faces)));
        self.entity(format_args!("MANIFOLD_SOLID_BREP('', {shell})"))
    }

    fn generated_support(&mut self, support: Support) -> (Ref, bool) {
        match support {
            Support::Plane { origin, u, v } => {
                let placement = self.placement(origin, u.cross(v).normalize(), u.normalize());
                (self.entity(format_args!("PLANE('', {placement})")), true)
            }
            Support::Round {
                center: [x, y],
                lower: [z0, r0],
                upper: [z1, r1],
                outward,
            } => {
                if r0 == r1 {
                    let origin = Point3::new(x, y, z1);
                    let placement = self.placement(origin, Vector3::unit_z(), Vector3::unit_x());
                    return (
                        self.entity(format_args!(
                            "CYLINDRICAL_SURFACE('', {placement}, {})",
                            Real(r0)
                        )),
                        outward,
                    );
                }
                // A positive STEP semi-angle grows in the direction of the placement axis.
                let (z, radius, axis) = if r1 > r0 { (z0, r0, 1.) } else { (z1, r1, -1.) };
                let placement = self.placement(
                    Point3::new(x, y, z),
                    axis * Vector3::unit_z(),
                    Vector3::unit_x(),
                );
                let angle = (r1 - r0).abs().atan2(z1 - z0);
                let cone = self.entity(format_args!(
                    "CONICAL_SURFACE('', {placement}, {}, {})",
                    Real(radius),
                    Real(angle)
                ));
                (cone, outward)
            }
        }
    }
}
