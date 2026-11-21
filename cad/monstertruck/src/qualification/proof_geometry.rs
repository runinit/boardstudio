//! Test-only vendor geometry oracle and input for synthetic source mutations.
//! Generated production qualification uses native topology and final triangles.

use crate::layered::{
    brep::{EdgeUse, Region, Support},
    tessellation::FaceMesh,
};
use monstertruck_meshing::rexport_polymesh::{
    Faces, PolygonMesh, StandardAttributes, StandardVertex,
};
use monstertruck_modeling::{
    Curve, Edge, Face, Line, Matrix4, Plane, Point3, Processor, RevolutionSurface, Shell, Solid,
    Surface, Vector3, Vertex, builder,
};

pub(crate) fn polygon(mesh: FaceMesh) -> PolygonMesh {
    let faces = Faces::from_iter(mesh.triangles.into_iter().map(|triangle| {
        triangle.map(|pos| StandardVertex {
            pos,
            uv: Some(pos),
            nor: None,
        })
    }));
    PolygonMesh::new(
        StandardAttributes {
            positions: mesh.positions,
            uv_coords: mesh.uv,
            ..Default::default()
        },
        faces,
    )
}

pub(crate) fn solids(region: &Region) -> Result<Vec<Solid>, String> {
    let vertices = Vertex::from_points(&region.topology.vertices);
    let edges = region
        .topology
        .edges
        .iter()
        .map(|edge| {
            let [a, b] = edge.vertices.map(|index| &vertices[index]);
            if let Some(circle) = edge.circle {
                let radial = a.point() - circle.center;
                let tangent = Vector3::unit_z().cross(radial) * circle.sweep.signum();
                builder::try_circle_arc(a, b, tangent)
                    .map_err(|error| format!("generated proof circle: {error:?}"))
            } else {
                Ok(builder::line(a, b))
            }
        })
        .collect::<Result<Vec<Edge>, _>>()?;

    let edge_use = |edge: &EdgeUse| {
        let absolute = &edges[edge.index];
        if edge.forward {
            absolute.clone()
        } else {
            absolute.inverse()
        }
    };
    region
        .shells
        .iter()
        .map(|faces| {
            let shell = faces
                .iter()
                .map(|face| {
                    let bounds = face
                        .rings
                        .iter()
                        .map(|ring| ring.iter().map(edge_use).collect())
                        .collect();
                    let surface = match face.support {
                        Support::Plane { origin, u, v } => {
                            Surface::Plane(Plane::new(origin, origin + u, origin + v))
                        }
                        Support::Round {
                            center: [x, y],
                            lower: [z0, r0],
                            upper: [z1, r1],
                            outward,
                        } => {
                            let first = Point3::new(r0, 0., z0);
                            let last = Point3::new(r1, 0., z1);
                            let line = if outward {
                                Line(last, first)
                            } else {
                                Line(first, last)
                            };
                            Surface::RevolutionSurface(Processor::with_transform(
                                RevolutionSurface::by_revolution(
                                    Curve::Line(line),
                                    Point3::new(0., 0., 0.),
                                    Vector3::unit_z(),
                                ),
                                Matrix4::from_translation(Vector3::new(x, y, 0.)),
                            ))
                        }
                    };
                    let native = Face::try_new(bounds, surface)
                        .map_err(|error| format!("generated proof face: {error:?}"))?;
                    Ok(if face.forward {
                        native
                    } else {
                        native.inverse()
                    })
                })
                .collect::<Result<Shell, String>>()?;
            Solid::try_new(vec![shell]).map_err(|error| format!("generated proof solid: {error:?}"))
        })
        .collect()
}
