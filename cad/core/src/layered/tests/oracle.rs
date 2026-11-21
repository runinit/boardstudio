//! Test views of the native exact solids, meshed by the production tessellator.
//!
//! These replace the former proof-solid oracle with the same small
//! surface the layered tests read: shells, faces, supports, vertices and meshes.
use super::super::brep::{self, Support};
use super::super::tessellation;
use super::{Point3, Vector3};
use crate::ExactModel;
use cgmath::prelude::*;
use std::collections::BTreeSet;

/// One material solid: a single closed shell of the native model.
#[derive(Clone, Copy)]
pub(in crate::layered) struct Solid<'a> {
    shells: [Shell<'a>; 1],
}

#[derive(Clone, Copy)]
pub(in crate::layered) struct Shell<'a> {
    topology: &'a brep::Topology,
    faces: &'a [brep::Face],
}

#[derive(Clone, Copy)]
pub(in crate::layered) struct Face<'a>(&'a brep::Face);

pub(in crate::layered) struct Plane {
    normal: Vector3,
}

pub(in crate::layered) enum Surface {
    Plane(Plane),
    RevolutionSurface(()),
}

pub(in crate::layered) struct Vertex(Point3);

#[derive(Clone, Copy)]
pub(in crate::layered) struct MeshVertex {
    pub pos: usize,
}

/// Triangle soup with the read-only queries the tests make of a face mesh.
#[derive(Clone, Debug, Default)]
pub(in crate::layered) struct PolygonMesh {
    positions: Vec<Point3>,
}

pub(in crate::layered) trait TestSolids {
    fn test_solids(&self) -> Vec<Solid<'_>>;
}

impl TestSolids for ExactModel {
    fn test_solids(&self) -> Vec<Solid<'_>> {
        self.shells()
            .map(|(topology, faces)| Solid {
                shells: [Shell { topology, faces }],
            })
            .collect()
    }
}

impl<'a> Solid<'a> {
    pub fn boundaries(&self) -> &[Shell<'a>] {
        &self.shells
    }

    /// The shell must mesh closed and outward oriented.
    pub fn try_new(shells: &[Shell<'a>]) -> Result<Self, String> {
        let [shell] = shells else {
            return Err("one material shell per solid".into());
        };
        let faces = tessellation::tessellate(
            shell.topology,
            shell.faces,
            0.1,
            crate::profile::PROFILE_EDGE_ANGULAR_TOLERANCE_RAD,
        )?;
        let mesh = PolygonMesh::from_triangles(
            faces
                .iter()
                .flat_map(tessellation::FaceMesh::triangle_points),
        );
        let triangles: Vec<_> = mesh.triangle_points().collect();
        if !crate::profile::closed_oriented_shell_triangles(&triangles) || mesh.volume() <= 0. {
            return Err("shell is not closed and outward oriented".into());
        }
        Ok(Self { shells: [*shell] })
    }

    pub fn is_geometric_consistent(&self) -> bool {
        Self::try_new(self.boundaries()).is_ok()
    }
}

impl<'a> Shell<'a> {
    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = Face<'a>> + 'a {
        self.faces.iter().map(Face)
    }

    pub fn face_iter(&self) -> impl Iterator<Item = Face<'a>> + 'a {
        self.iter()
    }

    pub fn vertex_iter(&self) -> impl Iterator<Item = Vertex> + 'a {
        let topology = self.topology;
        let used: BTreeSet<_> = self
            .faces
            .iter()
            .flat_map(|face| face.rings.iter().flatten())
            .flat_map(|edge| topology.edges[edge.index].vertices)
            .collect();
        used.into_iter()
            .map(move |index| Vertex(topology.vertices[index]))
    }
}

impl<'a> IntoIterator for Shell<'a> {
    type Item = Face<'a>;
    type IntoIter =
        std::iter::Map<std::slice::Iter<'a, brep::Face>, fn(&'a brep::Face) -> Face<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        self.faces
            .iter()
            .map(Face as fn(&'a brep::Face) -> Face<'a>)
    }
}

impl<'a> IntoIterator for &Shell<'a> {
    type Item = Face<'a>;
    type IntoIter =
        std::iter::Map<std::slice::Iter<'a, brep::Face>, fn(&'a brep::Face) -> Face<'a>>;
    fn into_iter(self) -> Self::IntoIter {
        (*self).into_iter()
    }
}

impl Face<'_> {
    pub fn surface(&self) -> Surface {
        match self.0.support {
            Support::Plane { u, v, .. } => Surface::Plane(Plane { normal: u.cross(v) }),
            Support::Round { .. } => Surface::RevolutionSurface(()),
        }
    }

    /// The support as used by the face, flipped for an inward use.
    pub fn oriented_surface(&self) -> Surface {
        match self.surface() {
            Surface::Plane(Plane { normal }) if !self.0.forward => {
                Surface::Plane(Plane { normal: -normal })
            }
            surface => surface,
        }
    }
}

impl Plane {
    pub fn normal(&self) -> Vector3 {
        self.normal
    }
}

impl Vertex {
    pub fn point(&self) -> Point3 {
        self.0
    }
}

impl PolygonMesh {
    pub fn from_triangles(triangles: impl IntoIterator<Item = [Point3; 3]>) -> Self {
        Self {
            positions: triangles.into_iter().flatten().collect(),
        }
    }

    pub fn positions(&self) -> &[Point3] {
        &self.positions
    }

    pub fn faces(&self) -> &Self {
        self
    }

    pub fn triangle_iter(&self) -> impl Iterator<Item = [MeshVertex; 3]> + '_ {
        (0..self.positions.len() / 3).map(|t| [0, 1, 2].map(|k| MeshVertex { pos: 3 * t + k }))
    }

    pub fn triangle_points(&self) -> impl Iterator<Item = [Point3; 3]> + '_ {
        self.positions.chunks_exact(3).map(|t| [t[0], t[1], t[2]])
    }

    pub fn volume(&self) -> f64 {
        self.triangle_points()
            .map(|[a, b, c]| a.to_vec().dot(b.to_vec().cross(c.to_vec())) / 6.)
            .sum()
    }
}

/// The whole shell meshed by the production tessellator at `deflection`.
pub(in crate::layered) fn shell_to_polygon_strict<'a>(
    shell: impl std::borrow::Borrow<Shell<'a>>,
    deflection: f64,
) -> Result<PolygonMesh, String> {
    let shell = shell.borrow();
    let faces = tessellation::tessellate(
        shell.topology,
        shell.faces,
        deflection * 0.1,
        crate::profile::PROFILE_EDGE_ANGULAR_TOLERANCE_RAD.min(0.05),
    )?;
    Ok(PolygonMesh::from_triangles(
        faces
            .iter()
            .flat_map(tessellation::FaceMesh::triangle_points),
    ))
}
