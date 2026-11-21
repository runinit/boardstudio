//! Exact boundary of generated layer solids. Edge indices belong to one region.

pub(crate) type Point3 = cgmath::Point3<f64>;
pub(crate) type Vector3 = cgmath::Vector3<f64>;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Region {
    pub topology: Topology,
    pub shells: Vec<Vec<Face>>,
}

#[derive(Default)]
pub(crate) struct Topology {
    pub vertices: Vec<Point3>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Copy)]
pub(crate) struct Circle {
    pub center: Point3,
    pub radius: f64,
    pub sweep: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct Edge {
    pub vertices: [usize; 2],
    pub circle: Option<Circle>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EdgeUse {
    pub index: usize,
    pub forward: bool,
}

impl EdgeUse {
    pub fn inverse(self) -> Self {
        Self {
            forward: !self.forward,
            ..self
        }
    }

    pub fn vertices(self, topology: &Topology) -> [usize; 2] {
        let [a, b] = topology.edges[self.index].vertices;
        if self.forward { [a, b] } else { [b, a] }
    }
}

pub(crate) type Ring = Vec<EdgeUse>;

pub(crate) fn reverse(ring: &mut Ring) {
    ring.reverse();
    for edge in ring {
        *edge = edge.inverse();
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Support {
    Plane {
        origin: Point3,
        u: Vector3,
        v: Vector3,
    },
    Round {
        center: [f64; 2],
        lower: [f64; 2], // elevation, radius
        upper: [f64; 2],
        outward: bool,
    },
}

pub(crate) struct Face {
    // In support orientation. Reverse the rings for an inward use of the support.
    pub rings: Vec<Ring>,
    pub support: Support,
    pub forward: bool,
    pub strips: Vec<Strip>,
}

#[derive(Clone, Copy)]
pub(crate) struct Strip {
    pub lower: EdgeUse,
    pub upper: EdgeUse,
}

type Result<T> = std::result::Result<T, String>;

impl Region {
    pub(super) fn new(topology: Topology, faces: Vec<Face>) -> Result<Self> {
        let mut uses = vec![[None; 2]; topology.edges.len()];
        for (face_index, face) in faces.iter().enumerate() {
            let mut vertices = BTreeSet::new();
            for ring in &face.rings {
                if ring.len() < 2 {
                    return Err("layered face ring has fewer than two edges".into());
                }
                for (edge, next) in ring.iter().zip(ring.iter().cycle().skip(1)) {
                    let [start, end] = edge.vertices(&topology);
                    if end != next.vertices(&topology)[0] || !vertices.insert(start) {
                        return Err("layered face rings are open or touch at a vertex".into());
                    }
                    let use_slot = uses[edge.index]
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .ok_or("layered edge has more than two face uses")?;
                    *use_slot = Some((face_index, edge.forward == face.forward));
                }
            }
        }
        let mut adjacency = vec![Vec::new(); faces.len()];
        let mut seen = vec![false; uses.len()];
        // Traverse first edge uses in face order to keep face occurrences stable.
        for edge in faces.iter().flat_map(|face| face.rings.iter().flatten()) {
            if std::mem::replace(&mut seen[edge.index], true) {
                continue;
            }
            match uses[edge.index] {
                [Some((a, fa)), Some((b, fb))] if a != b && fa != fb => {
                    if !adjacency[a].contains(&b) {
                        adjacency[a].push(b);
                        adjacency[b].push(a);
                    }
                }
                _ => return Err("layered exact solid is not a closed oriented shell".into()),
            }
        }

        let mut faces: Vec<_> = faces.into_iter().map(Some).collect();
        let mut shells = Vec::new();
        for seed in 0..faces.len() {
            if faces[seed].is_none() {
                continue;
            }
            let mut stack = vec![seed];
            let mut shell = Vec::new();
            while let Some(index) = stack.pop() {
                if let Some(face) = faces[index].take() {
                    shell.push(face);
                    stack.extend(adjacency[index].iter().copied());
                }
            }
            check_shell(&topology, &shell)?;
            shells.push(shell);
        }
        Ok(Self { topology, shells })
    }
}

fn check_shell(topology: &Topology, faces: &[Face]) -> Result<()> {
    let mut links = BTreeMap::<usize, BTreeMap<usize, Vec<usize>>>::new();
    for ring in faces.iter().flat_map(|face| &face.rings) {
        for (edge, next) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let vertex = edge.vertices(topology)[1];
            let link = links.entry(vertex).or_default();
            link.entry(edge.index).or_default().push(next.index);
            link.entry(next.index).or_default().push(edge.index);
        }
    }
    let top = links
        .keys()
        .map(|&index| topology.vertices[index].z)
        .fold(f64::NEG_INFINITY, f64::max);
    if !faces.iter().any(|face| {
        matches!(face.support, Support::Plane { origin, u, v }
            if origin.z == top && u.z == 0. && v.z == 0. && face.forward)
    }) {
        return Err("disconnected sealed void shell has unsupported ownership".into());
    }

    // Every vertex link must be one connected cycle within this material shell.
    for mut link in links.into_values() {
        let mut stack = vec![*link.keys().next().expect("vertex link has an edge")];
        while let Some(edge) = stack.pop() {
            if let Some(neighbors) = link.remove(&edge) {
                stack.extend(neighbors);
            }
        }
        if !link.is_empty() {
            return Err("layered exact solid has a non-manifold vertex".into());
        }
    }
    Ok(())
}
