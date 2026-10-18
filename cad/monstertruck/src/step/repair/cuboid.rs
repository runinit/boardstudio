//! Complete zero-repair proof for one integer-authored native LINE/plane cuboid.
//! Dimensions and entity numbers are data; no fixture or file identity is recognized.
mod lexemes;

use super::*;
use crate::step::tessellation::StepMesh;
use monstertruck_io::step::load::step_geometry::{ElementarySurface, Surface};
use monstertruck_modeling::Vector3;
use std::collections::BTreeMap;

type Point = [f64; 3];
type FaceLoop = (u64, bool, Vec<(u64, bool)>);

struct Document<'a> {
    records: HashMap<u64, Vec<&'a Record>>,
    integers: HashMap<u64, Vec<f64>>,
}

impl<'a> Document<'a> {
    fn new(bytes: &[u8], original: &'a Exchange) -> Option<Self> {
        let [data] = original.data.as_slice() else {
            return None;
        };
        let mut records = HashMap::new();
        for instance in &data.entities {
            let (id, parts) = match instance {
                EntityInstance::Simple { id, record } => (*id, vec![record]),
                EntityInstance::Complex { id, subsuper } => (*id, subsuper.0.iter().collect()),
            };
            if records.insert(id, parts).is_some() {
                return None;
            }
        }
        Some(Self {
            records,
            integers: lexemes::integers(bytes)?,
        })
    }

    fn params(&self, id: u64, name: &str, count: usize) -> Option<&[Parameter]> {
        let [record] = self.records.get(&id)?.as_slice() else {
            return None;
        };
        if record.name != name {
            return None;
        }
        list(&record.parameter).filter(|params| params.len() == count)
    }

    fn triple(&self, id: u64, name: &str) -> Option<Point> {
        let params = self.params(id, name, 2)?;
        if list(&params[1])?.len() != 3 {
            return None;
        }
        self.integers.get(&id)?.as_slice().try_into().ok()
    }

    fn vertex(&self, id: u64) -> Option<Point> {
        self.triple(
            reference(&self.params(id, "VERTEX_POINT", 2)?[1])?,
            "CARTESIAN_POINT",
        )
    }

    fn explicit_mm(&self, representation: u64, solid: u64) -> Option<()> {
        // Census every original record, including roots the importer does not
        // traverse. A suffix/unsupported-kind blacklist cannot establish source
        // completeness: presentation roots can expose geometry outside the solid.
        // This is the entire permitted grammar of this proof, not an importer
        // filter. Unknown records retain the asset's unknown movement result.
        // A rounded assembly/map identity is not original identity authority.
        let mut representations = 0;
        for (&id, parts) in &self.records {
            for part in parts {
                match part.name.as_str() {
                    "ADVANCED_BREP_SHAPE_REPRESENTATION" => {
                        if id != representation {
                            return None;
                        }
                        representations += 1;
                    }
                    // Native geometry; the sole root's complete finite graph is
                    // authenticated below and by Cuboid::from_original.
                    "CARTESIAN_POINT"
                    | "DIRECTION"
                    | "VECTOR"
                    | "LINE"
                    | "VERTEX_POINT"
                    | "EDGE_CURVE"
                    | "ORIENTED_EDGE"
                    | "EDGE_LOOP"
                    | "FACE_OUTER_BOUND"
                    | "AXIS2_PLACEMENT_3D"
                    | "PLANE"
                    | "ADVANCED_FACE"
                    | "CLOSED_SHELL"
                    | "MANIFOLD_SOLID_BREP" => {}
                    // Context/unit and product-definition records do not present
                    // geometry or transform occurrences. They may reference the
                    // sole geometry root, but cannot supply another permitted root.
                    "LENGTH_UNIT"
                    | "NAMED_UNIT"
                    | "SI_UNIT"
                    | "REPRESENTATION_CONTEXT"
                    | "GEOMETRIC_REPRESENTATION_CONTEXT"
                    | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                    | "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT"
                    | "UNCERTAINTY_MEASURE_WITH_UNIT"
                    | "APPLICATION_CONTEXT"
                    | "PRODUCT_CONTEXT"
                    | "PRODUCT"
                    | "PRODUCT_DEFINITION_FORMATION"
                    | "PRODUCT_DEFINITION_CONTEXT"
                    | "PRODUCT_DEFINITION"
                    | "PRODUCT_DEFINITION_SHAPE"
                    | "SHAPE_DEFINITION_REPRESENTATION" => {}
                    _ => return None,
                }
            }
        }
        if representations != 1 {
            return None;
        }
        let params = self.params(representation, "ADVANCED_BREP_SHAPE_REPRESENTATION", 3)?;
        let [item] = list(&params[1])? else {
            return None;
        };
        if reference(item)? != solid {
            return None;
        }
        let context = self.records.get(&reference(&params[2])?)?;
        let assigned: Vec<_> = context
            .iter()
            .filter(|r| r.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
            .collect();
        let [assigned] = assigned.as_slice() else {
            return None;
        };
        let [set] = list(&assigned.parameter)? else {
            return None;
        };
        let [unit] = list(set)? else {
            return None;
        };
        let parts = self.records.get(&reference(unit)?)?;
        if parts.len() != 3
            || !parts.iter().any(|r| r.name == "LENGTH_UNIT" && list(&r.parameter) == Some(&[][..]))
            || !parts.iter().any(|r| r.name == "NAMED_UNIT" && list(&r.parameter) == Some(&[Parameter::Omitted][..]))
            || !parts.iter().any(|r| r.name == "SI_UNIT" && list(&r.parameter).is_some_and(|p| matches!(p,
                [Parameter::Enumeration(prefix), Parameter::Enumeration(unit)] if prefix == "MILLI" && unit == "METRE"))) {
            return None;
        }
        Some(())
    }
}

fn list(p: &Parameter) -> Option<&[Parameter]> {
    if let Parameter::List(values) = p {
        Some(values)
    } else {
        None
    }
}
fn reference(p: &Parameter) -> Option<u64> {
    if let Parameter::Ref(Name::Entity(id)) = p {
        Some(*id)
    } else {
        None
    }
}
fn logical(p: &Parameter) -> Option<bool> {
    match p {
        Parameter::Enumeration(v) if v == "T" => Some(true),
        Parameter::Enumeration(v) if v == "F" => Some(false),
        _ => None,
    }
}
fn signed_axis(v: Point) -> Option<usize> {
    let indices: Vec<_> = (0..3).filter(|&i| v[i] != 0.0).collect();
    let [axis] = indices.as_slice() else {
        return None;
    };
    (v[*axis].abs() == 1.0).then_some(*axis)
}
fn point(p: Point3) -> Point {
    [p.x, p.y, p.z]
}
fn vector(v: Vector3) -> Point {
    [v.x, v.y, v.z]
}

struct Edge {
    vertices: [u64; 2],
    points: [Point; 2],
    axis: usize,
}

struct Face {
    origin: Point,
    u: Point,
    v: Point,
    normal: Point,
    axis: usize,
    sense: bool,
    edges: Vec<u64>,
}

pub(in crate::step) struct Cuboid<'a> {
    original: &'a OriginalGeometry,
    vertices: HashMap<u64, Point>,
    edges: HashMap<u64, Edge>,
    faces: HashMap<u64, Face>,
    bounds: [Point; 2],
}

impl<'a> Cuboid<'a> {
    pub(in crate::step) fn from_original(
        bytes: &[u8],
        exchange: &Exchange,
        original: &'a OriginalGeometry,
        solid: u64,
        representation: u64,
    ) -> Option<Self> {
        let doc = Document::new(bytes, exchange)?;
        doc.explicit_mm(representation, solid)?;
        let solid_params = doc.params(solid, "MANIFOLD_SOLID_BREP", 2)?;
        let shell = doc.params(reference(&solid_params[1])?, "CLOSED_SHELL", 2)?;
        let source_faces = list(&shell[1])?;
        if source_faces.len() != 6 {
            return None;
        }
        let mut result = Self {
            original,
            vertices: HashMap::new(),
            edges: HashMap::new(),
            faces: HashMap::new(),
            bounds: [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]],
        };
        let mut loops = Vec::new();
        for face_ref in source_faces {
            let id = reference(face_ref)?;
            let params = doc.params(id, "ADVANCED_FACE", 4)?;
            let [bound] = list(&params[1])? else {
                return None;
            };
            let bound = doc.params(reference(bound)?, "FACE_OUTER_BOUND", 3)?;
            let edge_loop = doc.params(reference(&bound[1])?, "EDGE_LOOP", 2)?;
            let uses = list(&edge_loop[1])?;
            if uses.len() != 4 {
                return None;
            }
            let mut oriented = Vec::new();
            for edge_use in uses {
                let params = doc.params(reference(edge_use)?, "ORIENTED_EDGE", 5)?;
                if params[1] != Parameter::Omitted || params[2] != Parameter::Omitted {
                    return None;
                }
                let id = reference(&params[3])?;
                if !result.edges.contains_key(&id) {
                    let edge = doc.params(id, "EDGE_CURVE", 5)?;
                    let vertices = [reference(&edge[1])?, reference(&edge[2])?];
                    let points = [doc.vertex(vertices[0])?, doc.vertex(vertices[1])?];
                    if vertices[0] == vertices[1] || points[0] == points[1] {
                        return None;
                    }
                    let axes: Vec<_> = (0..3).filter(|&i| points[0][i] != points[1][i]).collect();
                    let [axis] = axes.as_slice() else {
                        return None;
                    };
                    let line = doc.params(reference(&edge[3])?, "LINE", 3)?;
                    let origin = doc.triple(reference(&line[1])?, "CARTESIAN_POINT")?;
                    let vector_id = reference(&line[2])?;
                    let vec = doc.params(vector_id, "VECTOR", 3)?;
                    if doc.integers.get(&vector_id)?.as_slice() != [1.0] {
                        return None;
                    }
                    let direction = doc.triple(reference(&vec[1])?, "DIRECTION")?;
                    if signed_axis(direction)? != *axis
                        || (0..3).any(|i| {
                            i != *axis && (origin[i] != points[0][i] || origin[i] != points[1][i])
                        })
                        || logical(&edge[4])?
                            != ((points[1][*axis] > points[0][*axis]) == (direction[*axis] > 0.0))
                    {
                        return None;
                    }
                    for (id, p) in vertices.into_iter().zip(points) {
                        if result.vertices.insert(id, p).is_some_and(|old| old != p) {
                            return None;
                        }
                    }
                    result.edges.insert(
                        id,
                        Edge {
                            vertices,
                            points,
                            axis: *axis,
                        },
                    );
                }
                oriented.push((id, logical(&params[4])?));
            }
            let plane = doc.params(reference(&params[2])?, "PLANE", 2)?;
            let frame = doc.params(reference(&plane[1])?, "AXIS2_PLACEMENT_3D", 4)?;
            let origin = doc.triple(reference(&frame[1])?, "CARTESIAN_POINT")?;
            let normal = doc.triple(reference(&frame[2])?, "DIRECTION")?;
            let u = doc.triple(reference(&frame[3])?, "DIRECTION")?;
            let axis = signed_axis(normal)?;
            if axis == signed_axis(u)? {
                return None;
            }
            let v = vector(Vector3::from(normal).cross(Vector3::from(u)));
            let face = Face {
                origin,
                u,
                v,
                normal,
                axis,
                sense: logical(&params[3])?,
                edges: oriented.iter().map(|(id, _)| *id).collect(),
            };
            if result.faces.insert(id, face).is_some() {
                return None;
            }
            loops.push((id, logical(&bound[2])?, oriented));
        }
        result.check_box(&loops)?;
        Some(result)
    }

    fn check_box(&mut self, loops: &[FaceLoop]) -> Option<()> {
        if self.vertices.len() != 8 || self.edges.len() != 12 || self.faces.len() != 6 {
            return None;
        }
        for p in self.vertices.values() {
            for (axis, value) in p.iter().enumerate() {
                self.bounds[0][axis] = self.bounds[0][axis].min(*value);
                self.bounds[1][axis] = self.bounds[1][axis].max(*value);
            }
        }
        let mut corners = HashSet::new();
        for p in self.vertices.values() {
            let mut corner = 0;
            for (axis, value) in p.iter().enumerate() {
                if self.bounds[0][axis] == self.bounds[1][axis] {
                    return None;
                }
                if *value == self.bounds[1][axis] {
                    corner |= 1 << axis;
                } else if *value != self.bounds[0][axis] {
                    return None;
                }
            }
            if !corners.insert(corner) {
                return None;
            }
        }
        let mut edge_pairs = HashSet::new();
        for edge in self.edges.values() {
            let mut pair = edge.vertices;
            pair.sort_unstable();
            if !edge_pairs.insert(pair) {
                return None;
            }
        }
        let mut sides = HashSet::new();
        let mut shared = HashMap::<u64, Vec<bool>>::new();
        for (id, bound_orientation, uses) in loops {
            let face = &self.faces[id];
            let mut ring = Vec::new();
            for &(id, orientation) in uses {
                let ends = self.edges[&id].vertices;
                ring.push(if orientation {
                    ends
                } else {
                    [ends[1], ends[0]]
                });
                shared
                    .entry(id)
                    .or_default()
                    .push(orientation == *bound_orientation);
            }
            if (0..4).any(|i| ring[i][1] != ring[(i + 1) % 4][0]) {
                return None;
            }
            let points: Vec<_> = ring.iter().map(|e| self.vertices[&e[0]]).collect();
            if points
                .iter()
                .map(|p| p.map(f64::to_bits))
                .collect::<HashSet<_>>()
                .len()
                != 4
                || !points.contains(&face.origin)
                || points
                    .iter()
                    .any(|p| p[face.axis] != face.origin[face.axis])
            {
                return None;
            }
            let side = if face.origin[face.axis] == self.bounds[0][face.axis] {
                false
            } else if face.origin[face.axis] == self.bounds[1][face.axis] {
                true
            } else {
                return None;
            };
            if !sides.insert((face.axis, side))
                || ((face.normal[face.axis] > 0.0) == face.sense) != side
            {
                return None;
            }
            let cross = (Vector3::from(points[1]) - Vector3::from(points[0]))
                .cross(Vector3::from(points[2]) - Vector3::from(points[1]));
            if (cross[face.axis] > 0.0) != (side == *bound_orientation) {
                return None;
            }
        }
        if shared.len() != 12
            || shared
                .values()
                .any(|uses| !matches!(uses.as_slice(), [a,b] if a != b))
        {
            return None;
        }
        Some(())
    }

    pub(in crate::step) fn proves_output(
        &self,
        source: &SourceBoundStepShell,
        normalized: &StepCompressedShell,
        mesh: &StepMesh,
        fallback: &[usize],
    ) -> bool {
        self.check_output(source, normalized, mesh, fallback)
            .is_some()
    }

    fn check_output(
        &self,
        source: &SourceBoundStepShell,
        normalized: &StepCompressedShell,
        mesh: &StepMesh,
        fallback: &[usize],
    ) -> Option<()> {
        let shell = source.shell();
        // Unit factor 1 and signed Cartesian frames must leave the actual normalized
        // shell unchanged. No re-created proxy is accepted as the normalized image.
        if normalized != shell
            || !fallback.is_empty()
            || shell.vertices.len() != 8
            || shell.edges.len() != 12
            || shell.faces.len() != 6
            || mesh.vertices != shell.vertices
            || mesh.edges.len() != 12
            || mesh.faces.len() != 6
        {
            return None;
        }
        let ids = self.original.bind_edges(source)?;
        let mut vertices = HashMap::new();
        for (index, id) in ids.iter().enumerate() {
            let raw = self.edges.get(id)?;
            let edge = &shell.edges[index];
            let Curve3D::Line(line) = &edge.curve else {
                return None;
            };
            if [point(line.0), point(line.1)] != raw.points {
                return None;
            }
            for (index, id) in [edge.vertices.0, edge.vertices.1]
                .into_iter()
                .zip(raw.vertices)
            {
                if point(*shell.vertices.as_slice().get(index)?) != self.vertices[&id]
                    || vertices
                        .insert(index, id)
                        .is_some_and(|previous| previous != id)
                {
                    return None;
                }
            }
            let emitted = &mesh.edges[index];
            if emitted.vertices != edge.vertices || !raw.proves_polyline(&emitted.curve) {
                return None;
            }
        }
        if vertices.values().copied().collect::<HashSet<_>>().len() != 8 {
            return None;
        }
        for (index, id) in source.source_face_ids().iter().enumerate() {
            let raw = self.faces.get(id)?;
            let face = &shell.faces[index];
            let Surface::ElementarySurface(ElementarySurface::Plane(plane)) = &face.surface else {
                return None;
            };
            if face.orientation != raw.sense
                || point(plane.origin()) != raw.origin
                || vector(plane.axis_u()) != raw.u
                || vector(plane.axis_v()) != raw.v
                || vector(plane.normal()) != raw.normal
            {
                return None;
            }
            let emitted = &mesh.faces[index];
            if emitted.orientation != face.orientation || emitted.boundaries != face.boundaries {
                return None;
            }
            self.check_face_mesh(raw, emitted.surface.as_ref()?)?;
        }
        Some(())
    }

    fn check_face_mesh(&self, face: &Face, mesh: &PolygonMesh) -> Option<()> {
        let positions = mesh.positions();
        if positions.iter().any(|p| {
            let p = point(*p);
            p[face.axis] != face.origin[face.axis]
                || (0..3).any(|i| {
                    !p[i].is_finite() || p[i] < self.bounds[0][i] || p[i] > self.bounds[1][i]
                })
        }) {
            return None;
        }
        let mut incidence = BTreeMap::new();
        for polygon in mesh.face_iter() {
            if polygon.len() < 3 {
                return None;
            }
            for i in 0..polygon.len() {
                let a = point(*positions.as_slice().get(polygon[i].pos)?);
                let b = point(
                    *positions
                        .as_slice()
                        .get(polygon[(i + 1) % polygon.len()].pos)?,
                );
                if a == b {
                    return None;
                }
                let mut key = [a.map(key_component), b.map(key_component)];
                key.sort_unstable();
                *incidence.entry(key).or_insert(0_usize) += 1;
            }
        }
        let mut intervals = HashMap::<u64, Vec<[f64; 2]>>::new();
        for (segment, count) in incidence {
            if count == 2 {
                continue;
            }
            if count != 1 {
                return None;
            }
            let [a, b] = segment.map(|p| p.map(f64::from_bits));
            let edge = face
                .edges
                .iter()
                .find(|id| self.edges[id].contains_segment(a, b))?;
            let axis = self.edges[edge].axis;
            intervals
                .entry(*edge)
                .or_default()
                .push([a[axis].min(b[axis]), a[axis].max(b[axis])]);
        }
        for id in &face.edges {
            let raw = &self.edges[id];
            let intervals = intervals.get_mut(id)?;
            intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
            let mut end = raw.points[0][raw.axis].min(raw.points[1][raw.axis]);
            for &[a, b] in intervals.iter() {
                if a != end || b <= a {
                    return None;
                }
                end = b;
            }
            if end != raw.points[0][raw.axis].max(raw.points[1][raw.axis]) {
                return None;
            }
        }
        Some(())
    }
}

fn key_component(value: f64) -> u64 {
    if value == 0.0 { 0 } else { value.to_bits() }
}

impl Edge {
    fn contains_segment(&self, a: Point, b: Point) -> bool {
        let low = self.points[0][self.axis].min(self.points[1][self.axis]);
        let high = self.points[0][self.axis].max(self.points[1][self.axis]);
        [a, b].iter().all(|p| {
            (0..3).all(|i| {
                p[i].is_finite()
                    && if i == self.axis {
                        low <= p[i] && p[i] <= high
                    } else {
                        p[i] == self.points[0][i]
                    }
            })
        })
    }

    fn proves_polyline(&self, points: &[Point3]) -> bool {
        if points.len() < 2
            || point(points[0]) != self.points[0]
            || point(points[points.len() - 1]) != self.points[1]
        {
            return false;
        }
        let forward = self.points[1][self.axis] > self.points[0][self.axis];
        points.windows(2).all(|pair| {
            let [a, b] = [point(pair[0]), point(pair[1])];
            self.contains_segment(a, b)
                && if forward {
                    a[self.axis] <= b[self.axis]
                } else {
                    a[self.axis] >= b[self.axis]
                }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use monstertruck_meshing::tessellation::TessellationOptions;

    #[test]
    fn real_tessellation_requires_whole_boundaries_and_unchanged_normalization() {
        let bytes = include_bytes!("../../../tests/fixtures/zero-repair-cuboid-mm.step");
        let model = import_step(bytes.to_vec()).unwrap();
        let exchange = parser::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
        let original = OriginalGeometry::capture(&Table::from_data_section(&exchange.data[0]));
        let candidate = Cuboid::from_original(bytes, &exchange, &original, 301, 304).unwrap();
        let source = &model.solids[&301].source[&304].shells()[0];
        let mut normalized = source.shell().clone();
        units::normalize_shell_to_mm(&mut normalized, 1.0).unwrap();
        let (mesh, fallback) = tessellation::triangulate_source_bound_shell_report(
            source,
            1.0,
            TessellationOptions {
                tolerance: 0.1,
                preview_fallback: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(candidate.proves_output(source, &normalized, &mesh, &fallback));
        assert!(
            !candidate.proves_output(source, &normalized, &mesh, &[0]),
            "fallback execution is outside this class even if its boundary happens to agree"
        );

        let mut changed = mesh.clone();
        let points = &mut changed.edges[0].curve.0;
        let mut midpoint = points[0] + (points[points.len() - 1] - points[0]) * 0.5;
        midpoint[(candidate.edges[&original.bind_edges(source).unwrap()[0]].axis + 1) % 3] += 0.001;
        points.insert(1, midpoint);
        assert!(
            !candidate.proves_output(source, &normalized, &changed, &[]),
            "unchanged endpoints do not certify the intermediate boundary set"
        );

        let mut changed = mesh.clone();
        let face = &candidate.faces[&source.source_face_ids()[0]];
        let vertex = changed.faces[0]
            .surface
            .as_mut()
            .unwrap()
            .positions_mut()
            .iter_mut()
            .find(|p| point(**p) == face.origin)
            .unwrap();
        let axis = (face.axis + 1) % 3;
        vertex[axis] = (candidate.bounds[0][axis] + candidate.bounds[1][axis]) * 0.5;
        assert!(
            !candidate.proves_output(source, &normalized, &changed, &[]),
            "actual face boundary reconciliation is checked even when shared edge polylines and counters are unchanged"
        );

        normalized.vertices[0].x += 1.0;
        assert!(!candidate.proves_output(source, &normalized, &mesh, &[]));
    }
}
