//! Complete zero-repair proof for one integer-authored native LINE/plane cuboid.
//! Dimensions and entity numbers are data; no fixture or file identity is recognized.
mod lexemes;

use super::*;
use crate::step::tessellation::StepMesh;
use monstertruck_io::step::load::step_geometry::{ElementarySurface, Surface};
use monstertruck_modeling::Vector3;

type Point = [f64; 3];
type FaceLoop = (u64, bool, [(u64, bool); 4]);

struct Document<'a> {
    records: Vec<&'a EntityInstance>,
    bytes: &'a [u8],
}

impl<'a> Document<'a> {
    fn new(bytes: &'a [u8], original: &'a Exchange) -> Option<Self> {
        let [data] = original.data.as_slice() else {
            return None;
        };
        let count = data.entities.len();
        if count > 1_000_000 || count.checked_mul(size_of::<&EntityInstance>())? > 256 * 1024 * 1024
        {
            return None;
        }
        let mut records = Vec::new();
        records.try_reserve_exact(count).ok()?;
        records.extend(&data.entities);
        if records.capacity() > count {
            return None;
        }
        records.sort_unstable_by_key(|instance| parts(instance).0);
        if records
            .windows(2)
            .any(|pair| parts(pair[0]).0 == parts(pair[1]).0)
        {
            return None;
        }
        lexemes::validate(bytes)?;
        Some(Self { records, bytes })
    }

    fn parts(&self, id: u64) -> Option<&[Record]> {
        let index = self
            .records
            .binary_search_by_key(&id, |instance| parts(instance).0)
            .ok()?;
        Some(parts(self.records[index]).1)
    }

    fn params(&self, id: u64, name: &str, count: usize) -> Option<&[Parameter]> {
        let [record] = self.parts(id)? else {
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
        lexemes::values(self.bytes, id)?.as_slice().try_into().ok()
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
        for instance in &self.records {
            let (id, parts) = parts(instance);
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
        let context = self.parts(reference(&params[2])?)?;
        let mut assigned = context
            .iter()
            .filter(|r| r.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT");
        let (Some(assigned), None) = (assigned.next(), assigned.next()) else {
            return None;
        };
        let [set] = list(&assigned.parameter)? else {
            return None;
        };
        let [unit] = list(set)? else {
            return None;
        };
        let parts = self.parts(reference(unit)?)?;
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

fn parts(instance: &EntityInstance) -> (u64, &[Record]) {
    match instance {
        EntityInstance::Simple { id, record } => (*id, std::slice::from_ref(record)),
        EntityInstance::Complex { id, subsuper } => (*id, &subsuper.0),
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
    let mut indices = (0..3).filter(|&i| v[i] != 0.0);
    let (Some(axis), None) = (indices.next(), indices.next()) else {
        return None;
    };
    (v[axis].abs() == 1.0).then_some(axis)
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
    edges: [u64; 4],
}

pub(in crate::step) struct Cuboid<'a> {
    original: &'a OriginalGeometry,
    vertices: [(u64, Point); 8],
    edges: [(u64, Edge); 12],
    faces: [(u64, Face); 6],
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
        let mut vertices = [None; 8];
        let mut edges: [Option<(u64, Edge)>; 12] = std::array::from_fn(|_| None);
        let mut faces: [Option<(u64, Face)>; 6] = std::array::from_fn(|_| None);
        let mut loops = [(0, false, [(0, false); 4]); 6];
        for (face_index, face_ref) in source_faces.iter().enumerate() {
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
            let mut oriented = [(0, false); 4];
            for (use_index, edge_use) in uses.iter().enumerate() {
                let params = doc.params(reference(edge_use)?, "ORIENTED_EDGE", 5)?;
                if params[1] != Parameter::Omitted || params[2] != Parameter::Omitted {
                    return None;
                }
                let id = reference(&params[3])?;
                if !edges.iter().flatten().any(|(previous, _)| *previous == id) {
                    let edge = doc.params(id, "EDGE_CURVE", 5)?;
                    let vertex_ids = [reference(&edge[1])?, reference(&edge[2])?];
                    let points = [doc.vertex(vertex_ids[0])?, doc.vertex(vertex_ids[1])?];
                    if vertex_ids[0] == vertex_ids[1] || points[0] == points[1] {
                        return None;
                    }
                    let mut axes = (0..3).filter(|&i| points[0][i] != points[1][i]);
                    let (Some(axis), None) = (axes.next(), axes.next()) else {
                        return None;
                    };
                    let line = doc.params(reference(&edge[3])?, "LINE", 3)?;
                    let origin = doc.triple(reference(&line[1])?, "CARTESIAN_POINT")?;
                    let vector_id = reference(&line[2])?;
                    let vec = doc.params(vector_id, "VECTOR", 3)?;
                    if lexemes::values(doc.bytes, vector_id)?.as_slice() != [1.0] {
                        return None;
                    }
                    let direction = doc.triple(reference(&vec[1])?, "DIRECTION")?;
                    if signed_axis(direction)? != axis
                        || (0..3).any(|i| {
                            i != axis && (origin[i] != points[0][i] || origin[i] != points[1][i])
                        })
                        || logical(&edge[4])?
                            != ((points[1][axis] > points[0][axis]) == (direction[axis] > 0.0))
                    {
                        return None;
                    }
                    for (id, p) in vertex_ids.into_iter().zip(points) {
                        let slot = vertices
                            .iter()
                            .position(|entry| entry.is_some_and(|(previous, _)| previous == id))
                            .or_else(|| vertices.iter().position(Option::is_none))?;
                        if vertices[slot].is_some_and(|(_, previous)| previous != p) {
                            return None;
                        }
                        vertices[slot] = Some((id, p));
                    }
                    *edges.iter_mut().find(|entry| entry.is_none())? = Some((
                        id,
                        Edge {
                            vertices: vertex_ids,
                            points,
                            axis,
                        },
                    ));
                }
                oriented[use_index] = (id, logical(&params[4])?);
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
                edges: oriented.map(|(id, _)| id),
            };
            if faces.iter().flatten().any(|(previous, _)| *previous == id) {
                return None;
            }
            faces[face_index] = Some((id, face));
            loops[face_index] = (id, logical(&bound[2])?, oriented);
        }
        if vertices.iter().any(Option::is_none) || edges.iter().any(Option::is_none) {
            return None;
        }
        let mut result = Self {
            original,
            vertices: vertices.map(Option::unwrap),
            edges: edges.map(Option::unwrap),
            faces: faces.map(Option::unwrap),
            bounds: [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]],
        };
        result.check_box(&loops)?;
        Some(result)
    }

    fn vertex(&self, id: u64) -> Option<&Point> {
        self.vertices
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, point)| point)
    }

    fn edge(&self, id: u64) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, edge)| edge)
    }

    fn face(&self, id: u64) -> Option<&Face> {
        self.faces
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, face)| face)
    }

    fn check_box(&mut self, loops: &[FaceLoop]) -> Option<()> {
        for (_, p) in &self.vertices {
            for (axis, value) in p.iter().enumerate() {
                self.bounds[0][axis] = self.bounds[0][axis].min(*value);
                self.bounds[1][axis] = self.bounds[1][axis].max(*value);
            }
        }
        let mut corners = 0_u8;
        for (_, p) in &self.vertices {
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
            if corners & (1 << corner) != 0 {
                return None;
            }
            corners |= 1 << corner;
        }
        for (index, (_, edge)) in self.edges.iter().enumerate() {
            let mut pair = edge.vertices;
            pair.sort_unstable();
            if self.edges[..index].iter().any(|(_, edge)| {
                let mut previous = edge.vertices;
                previous.sort_unstable();
                previous == pair
            }) {
                return None;
            }
        }
        let mut sides = 0_u8;
        let mut shared = [[false; 2]; 12];
        let mut use_counts = [0; 12];
        for (id, bound_orientation, uses) in loops {
            let face = self.face(*id)?;
            let mut ring = [[0; 2]; 4];
            let mut points = [[0.0; 3]; 4];
            for (index, &(id, orientation)) in uses.iter().enumerate() {
                let edge = self.edges.iter().position(|(key, _)| *key == id)?;
                let ends = self.edges[edge].1.vertices;
                ring[index] = if orientation {
                    ends
                } else {
                    [ends[1], ends[0]]
                };
                points[index] = *self.vertex(ring[index][0])?;
                *shared[edge].get_mut(use_counts[edge])? = orientation == *bound_orientation;
                use_counts[edge] += 1;
            }
            if (0..4).any(|i| ring[i][1] != ring[(i + 1) % 4][0]) {
                return None;
            }
            if points
                .iter()
                .enumerate()
                .any(|(index, point)| points[..index].contains(point))
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
            let side_bit = 1 << (face.axis * 2 + usize::from(side));
            if sides & side_bit != 0 || ((face.normal[face.axis] > 0.0) == face.sense) != side {
                return None;
            }
            sides |= side_bit;
            let cross = (Vector3::from(points[1]) - Vector3::from(points[0]))
                .cross(Vector3::from(points[2]) - Vector3::from(points[1]));
            if (cross[face.axis] > 0.0) != (side == *bound_orientation) {
                return None;
            }
        }
        if use_counts.iter().any(|count| *count != 2) || shared.iter().any(|[a, b]| a == b) {
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
        let mut vertices = [None; 8];
        for (index, id) in ids.iter().enumerate() {
            let raw = self.edge(*id)?;
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
                let slot = vertices.get_mut(index)?;
                if point(*shell.vertices.as_slice().get(index)?) != *self.vertex(id)?
                    || slot.is_some_and(|previous| previous != id)
                {
                    return None;
                }
                *slot = Some(id);
            }
            let emitted = &mesh.edges[index];
            if emitted.vertices != edge.vertices || !raw.proves_polyline(&emitted.curve) {
                return None;
            }
        }
        if vertices
            .iter()
            .enumerate()
            .any(|(index, id)| id.is_none() || vertices[..index].contains(id))
        {
            return None;
        }
        for (index, id) in source.source_face_ids().iter().enumerate() {
            let raw = self.face(*id)?;
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
        struct Segment {
            key: [[u64; 3]; 2],
            edge: Option<usize>,
            interval: [f64; 2],
        }
        let count = mesh.face_iter().try_fold(0_usize, |count, polygon| {
            (polygon.len() >= 3).then_some(())?;
            count.checked_add(polygon.len())
        })?;
        if count > 1_000_000 || count.checked_mul(size_of::<Segment>())? > 256 * 1024 * 1024 {
            return None;
        }
        let mut segments = Vec::new();
        segments.try_reserve_exact(count).ok()?;
        if segments.capacity() > count {
            return None;
        }
        for polygon in mesh.face_iter() {
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
                segments.push(Segment {
                    key,
                    edge: None,
                    interval: [0.0; 2],
                });
            }
        }
        segments.sort_unstable_by_key(|segment| segment.key);
        let mut index = 0;
        while index < segments.len() {
            let key = segments[index].key;
            let end = index
                + segments[index..]
                    .iter()
                    .take_while(|segment| segment.key == key)
                    .count();
            if end - index > 2 {
                return None;
            }
            if end - index == 1 {
                let [a, b] = key.map(|p| p.map(f64::from_bits));
                let (edge, raw) = face.edges.iter().enumerate().find_map(|(index, id)| {
                    self.edge(*id)
                        .filter(|edge| edge.contains_segment(a, b))
                        .map(|edge| (index, edge))
                })?;
                segments[index].edge = Some(edge);
                segments[index].interval =
                    [a[raw.axis].min(b[raw.axis]), a[raw.axis].max(b[raw.axis])];
            }
            index = end;
        }
        segments.sort_unstable_by(|a, b| {
            a.edge
                .cmp(&b.edge)
                .then_with(|| a.interval[0].total_cmp(&b.interval[0]))
        });
        for (index, id) in face.edges.iter().enumerate() {
            let raw = self.edge(*id)?;
            let mut end = raw.points[0][raw.axis].min(raw.points[1][raw.axis]);
            for segment in segments
                .iter()
                .filter(|segment| segment.edge == Some(index))
            {
                let [a, b] = segment.interval;
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn cuboid_census_borrows_large_unrelated_records() {
        let original = include_str!("../../../tests/fixtures/zero-repair-cuboid-mm.step");
        let label = "x".repeat(1024);
        let mut padding = String::new();
        for id in 10_000..12_048 {
            padding.push_str(&format!("#{id}=PRODUCT('{label}','','',());\n"));
        }
        let end = original.rfind("ENDSEC;").unwrap();
        let source = format!("{}{padding}{}", &original[..end], &original[end..]);
        let exchange = parser::parse(&source).unwrap();
        let (document, largest) = crate::allocation_tests::largest_request(|| {
            Document::new(source.as_bytes(), &exchange)
        });
        let document = document.unwrap();
        assert!(document.explicit_mm(304, 301).is_some());
        assert!(
            largest <= exchange.data[0].entities.len() * size_of::<&EntityInstance>(),
            "cuboid census allocated {largest} bytes"
        );

        let unsupported = source.replace("#10000=PRODUCT(", "#10000=UNKNOWN_GEOMETRY(");
        let exchange = parser::parse(&unsupported).unwrap();
        assert!(
            Document::new(unsupported.as_bytes(), &exchange)
                .unwrap()
                .explicit_mm(304, 301)
                .is_none()
        );

        let duplicate = source.replace("#12047=PRODUCT(", "#10000=PRODUCT(");
        let exchange = parser::parse(&duplicate).unwrap();
        assert!(Document::new(duplicate.as_bytes(), &exchange).is_none());
    }

    #[test]
    fn real_tessellation_requires_whole_boundaries_and_unchanged_normalization() {
        let bytes = include_bytes!("../../../tests/fixtures/zero-repair-cuboid-mm.step");
        let model = import_step(bytes.to_vec()).unwrap();
        let exchange = parser::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
        let original =
            OriginalGeometry::capture(&Table::from_data_section(&exchange.data[0]), bytes);
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
        midpoint[(candidate
            .edge(original.bind_edges(source).unwrap()[0])
            .unwrap()
            .axis
            + 1)
            % 3] += 0.001;
        points.insert(1, midpoint);
        assert!(
            !candidate.proves_output(source, &normalized, &changed, &[]),
            "unchanged endpoints do not certify the intermediate boundary set"
        );

        let mut changed = mesh.clone();
        let face = candidate.face(source.source_face_ids()[0]).unwrap();
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
