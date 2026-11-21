//! Complete zero-repair proof for one integer-authored native LINE/plane cuboid.
//! Dimensions and entity numbers are data; no fixture or file identity is recognized.
//! The original decimal spellings establish the box; the delivered asset's
//! actual triangles must then tile exactly that box. Mesh agreement alone never
//! establishes the proof, and neither does either route's repair accounting.
mod lexemes;

use super::*;

type Point = [f64; 3];
type FaceLoop = (u64, bool, [(u64, bool); 4]);

struct Document<'a> {
    records: Vec<&'a EntityInstance>,
    data: &'a [u8],
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
        // A REFERENCE entry naming a DATA ID makes that ID ambiguous.
        if original.reference.iter().any(|entry| {
            matches!(entry.name, Name::Entity(id) if records
                .binary_search_by_key(&id, |instance| parts(instance).0)
                .is_ok())
        }) {
            return None;
        }
        let data = lexemes::validate(bytes, &records)?;
        Some(Self { records, data })
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
        lexemes::values(self.data, id)?.as_slice().try_into().ok()
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
struct Edge {
    vertices: [u64; 2],
}

struct Face {
    plane: u64,
    origin: Point,
    normal: Point,
    axis: usize,
    sense: bool,
    edges: [u64; 4],
}

pub(super) struct Cuboid {
    solid: u64,
    representation: u64,
    vertices: [(u64, Point); 8],
    edges: [(u64, Edge); 12],
    faces: [(u64, Face); 6],
    bounds: [Point; 2],
}

/// The census admits no curve presentation, and an exact tiling of every face
/// is a closed outward mesh. Fallback meshing stays outside the class even when
/// its triangles happen to agree with the box.
pub(super) fn certify(model: &ImportedStep) -> bool {
    let [(solid, _, representation)] = model.occurrences.as_slice() else {
        return false;
    };
    model.source_faces.len() == 6
        && model.accuracy.fallback_meshed_faces == 0
        && Cuboid::from_original(
            &model.original_bytes,
            &model._original_source,
            *solid,
            *representation,
        )
        .is_some_and(|cuboid| cuboid.proves(model))
}

impl Cuboid {
    fn from_original(
        bytes: &[u8],
        exchange: &Exchange,
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
                    if lexemes::values(doc.data, vector_id)?.as_slice() != [1.0] {
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
                        },
                    ));
                }
                oriented[use_index] = (id, logical(&params[4])?);
            }
            let plane_id = reference(&params[2])?;
            let plane = doc.params(plane_id, "PLANE", 2)?;
            let frame = doc.params(reference(&plane[1])?, "AXIS2_PLACEMENT_3D", 4)?;
            let origin = doc.triple(reference(&frame[1])?, "CARTESIAN_POINT")?;
            let normal = doc.triple(reference(&frame[2])?, "DIRECTION")?;
            let u = doc.triple(reference(&frame[3])?, "DIRECTION")?;
            let axis = signed_axis(normal)?;
            if axis == signed_axis(u)? {
                return None;
            }
            let face = Face {
                plane: plane_id,
                origin,
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
            solid,
            representation,
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

    /// Binds the source proof to the asset that was actually delivered: this
    /// one identity occurrence, the authored faces in shell order, the f32
    /// render image, and per face an exact outward tiling of its rectangle.
    fn proves(&self, asset: &ImportedStep) -> bool {
        self.check_asset(asset).is_some()
    }

    fn check_asset(&self, asset: &ImportedStep) -> Option<()> {
        let [(solid, placement, representation)] = asset.occurrences.as_slice() else {
            return None;
        };
        if asset.solid_count != 1
            || *solid != self.solid
            || *representation != self.representation
            || *placement != Matrix4::identity()
            || asset.bounds != self.bounds
            || asset.source_faces.len() != 6
            || asset.face_occurrences.len() != 6
            || Some(asset.preview_positions_f64.len()) != asset.triangle_faces.len().checked_mul(3)
            || Some(asset.mesh.positions.len()) != asset.preview_positions_f64.len().checked_mul(3)
            || asset.mesh.normals.len() != asset.mesh.positions.len()
        {
            return None;
        }
        for (index, ((id, face), (source, occurrence))) in self
            .faces
            .iter()
            .zip(asset.source_faces.iter().zip(&asset.face_occurrences))
            .enumerate()
        {
            if source.solid_id != self.solid
                || source.shell_index != 0
                || source.face_id != *id
                || source.support_id != face.plane
                || source.support_kind != "PLANE"
                || occurrence.source_face != index
                || occurrence.occurrence != 0
                || occurrence.representation_id != self.representation
                || occurrence.repair_displacement_mm != 0.0
            {
                return None;
            }
        }
        let mut faces: [Vec<[usize; 3]>; 6] = Default::default();
        for (index, (&tag, points)) in asset
            .triangle_faces
            .iter()
            .zip(asset.preview_positions_f64.as_chunks::<3>().0)
            .enumerate()
        {
            let face = &self.faces.get(tag)?.1;
            let normal = std::array::from_fn::<f64, 3, _>(|axis| {
                if axis == face.axis {
                    self.outward(face)
                } else {
                    0.0
                }
            });
            let rendered = <[f32]>::get(&asset.mesh.positions, index * 9..index * 9 + 9)?;
            let normals = <[f32]>::get(&asset.mesh.normals, index * 9..index * 9 + 9)?;
            let mut corners = [0; 3];
            for (k, p) in points.iter().enumerate() {
                if (0..3).any(|i| {
                    f64::from(rendered[k * 3 + i]) != p[i]
                        || f64::from(normals[k * 3 + i]) != normal[i]
                }) {
                    return None;
                }
                corners[k] = self.vertices.iter().position(|(_, q)| q == p)?;
            }
            faces[tag].push(corners);
        }
        for ((_, face), triangles) in self.faces.iter().zip(&faces) {
            self.check_tiling(face, triangles)?;
        }
        Some(())
    }

    fn outward(&self, face: &Face) -> f64 {
        // check_box placed every face on the low or high side of its axis.
        if face.origin[face.axis] == self.bounds[1][face.axis] {
            1.0
        } else {
            -1.0
        }
    }

    /// Every vertex is a proven corner with integer coordinates of magnitude at
    /// most 2^20, so each orientation product below is exact. The unpaired
    /// directed segments of positive triangles form a cycle; when it uses each
    /// rectangle side exactly once it is the outward rectangle boundary, so the
    /// triangles cover the rectangle once and nothing else. Over four corners
    /// no positive triangle traverses a side inward, so every side use counts;
    /// a repeated diagonal repeats its triangle and therefore that triangle's sides.
    fn check_tiling(&self, face: &Face, triangles: &[[usize; 3]]) -> Option<()> {
        let outward = self.outward(face);
        let (u, v) = ((face.axis + 1) % 3, (face.axis + 2) % 3);
        let p = |corner: usize| self.vertices[corner].1;
        let turn = |a: Point, b: Point, c: Point| {
            ((b[u] - a[u]) * (c[v] - a[v]) - (b[v] - a[v]) * (c[u] - a[u])) * outward
        };
        let mut segments = Vec::with_capacity(triangles.len() * 3);
        for &[a, b, c] in triangles {
            if [a, b, c]
                .iter()
                .any(|&q| p(q)[face.axis] != face.origin[face.axis])
                || turn(p(a), p(b), p(c)) <= 0.0
            {
                return None;
            }
            segments.extend([(a, b), (b, c), (c, a)]);
        }
        segments.sort_unstable();
        let mut boundary = [false; 4];
        for &(a, b) in &segments {
            if segments.binary_search(&(b, a)).is_ok() {
                continue;
            }
            let side = face.edges.iter().position(|id| {
                self.edge(*id).is_some_and(|edge| {
                    let ends = edge
                        .vertices
                        .map(|id| self.vertices.iter().position(|(key, _)| *key == id));
                    ends == [Some(a), Some(b)] || ends == [Some(b), Some(a)]
                })
            })?;
            if boundary[side] {
                return None;
            }
            boundary[side] = true;
        }
        boundary.iter().all(|side| *side).then_some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    const BOX: &[u8] = include_bytes!("../../../tests/fixtures/zero-repair-cuboid-mm.step");

    /// The same box outside the native exchange gate is delivered by legacy.
    fn ap242() -> Vec<u8> {
        std::str::from_utf8(BOX)
            .unwrap()
            .replace(
                "FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));",
                "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 1 1 4 }'));",
            )
            .into_bytes()
    }

    fn delivered(bytes: &[u8], options: StepImportOptions, native: bool) -> ImportedStep {
        let model = import_step_with_options(bytes.to_vec(), options).unwrap();
        assert_eq!(model.solids.is_empty(), native, "unexpected import route");
        model
    }

    fn set_point(asset: &mut ImportedStep, vertex: usize, p: Point) {
        asset.preview_positions_f64[vertex] = p;
        for (i, value) in p.into_iter().enumerate() {
            asset.mesh.positions[vertex * 3 + i] = value as f32;
        }
    }

    #[test]
    fn delivered_triangles_must_tile_each_proven_face_outward_on_either_route() {
        for (bytes, native) in [(BOX.to_vec(), true), (ap242(), false)] {
            delivered_tampering_is_refused(&bytes, native);
        }
    }

    fn delivered_tampering_is_refused(bytes: &[u8], native: bool) {
        let fresh = || delivered(bytes, StepImportOptions::default(), native);
        let asset = fresh();
        assert_eq!(asset.accuracy.max_repair_displacement_mm, Some(0.0));
        assert!(certify(&asset));
        let cuboid =
            Cuboid::from_original(&asset.original_bytes, &asset._original_source, 301, 304)
                .unwrap();
        assert!(cuboid.proves(&asset));
        assert_eq!(
            asset.triangle_faces,
            [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5],
            "two corner triangles per face"
        );
        let refuses = |asset: &ImportedStep, why: &str| {
            assert!(!cuboid.proves(asset), "{why}");
            assert!(!certify(asset), "{why}");
        };

        // The other diagonal is an equally exact tiling: no triangulation is recognized.
        let mut flipped = fresh();
        let [a, b, c] = [0, 1, 2].map(|i| asset.preview_positions_f64[i]);
        let [d, e, f] = [3, 4, 5].map(|i| asset.preview_positions_f64[i]);
        let quad: Vec<_> = [a, b, c, d, e, f]
            .into_iter()
            .fold(Vec::new(), |mut unique, p| {
                if !unique.contains(&p) {
                    unique.push(p);
                }
                unique
            });
        assert_eq!(quad.len(), 4);
        let shared: Vec<_> = [a, b, c]
            .into_iter()
            .filter(|p| [d, e, f].contains(p))
            .collect();
        let lone = [a, b, c].into_iter().find(|p| !shared.contains(p)).unwrap();
        let other = [d, e, f].into_iter().find(|p| !shared.contains(p)).unwrap();
        // The other diagonal splits the same rectangle; keep the authored winding.
        let turn = |[a, b, c]: [Point; 3]| {
            (Vector3::from(b) - Vector3::from(a)).cross(Vector3::from(c) - Vector3::from(a))
                [cuboid.faces[asset.triangle_faces[0]].1.axis]
        };
        let orient = |[p, q, r]: [Point; 3]| {
            if (turn([p, q, r]) > 0.0) == (turn([a, b, c]) > 0.0) {
                [p, q, r]
            } else {
                [p, r, q]
            }
        };
        assert_eq!(asset.triangle_faces[..2], [asset.triangle_faces[0]; 2]);
        let first = orient([lone, other, shared[0]]);
        let second = orient([lone, other, shared[1]]);
        for (vertex, p) in first.into_iter().chain(second).enumerate() {
            set_point(&mut flipped, vertex, p);
        }
        assert!(
            cuboid.proves(&flipped),
            "a different exact tiling still proves"
        );

        let mut reversed = fresh();
        let (one, two) = (
            reversed.preview_positions_f64[1],
            reversed.preview_positions_f64[2],
        );
        set_point(&mut reversed, 1, two);
        set_point(&mut reversed, 2, one);
        refuses(&reversed, "an inward triangle is not the outward face");

        let mut interior = fresh();
        let mut moved = interior.preview_positions_f64[0];
        let tag = interior.triangle_faces[0];
        let face = &cuboid.faces[tag].1;
        let axis = (face.axis + 1) % 3;
        moved[axis] = (cuboid.bounds[0][axis] + cuboid.bounds[1][axis]) * 0.5;
        set_point(&mut interior, 0, moved);
        refuses(
            &interior,
            "a vertex inside the rectangle is not a proven corner",
        );

        let mut missing = fresh();
        missing.preview_positions_f64.drain(0..3);
        missing.mesh.positions.drain(0..9);
        missing.mesh.normals.drain(0..9);
        missing.triangle_faces.remove(0);
        refuses(&missing, "half a face is not the face");

        let mut doubled = fresh();
        let copy: Vec<_> = doubled.preview_positions_f64[0..3].to_vec();
        doubled.preview_positions_f64.extend(copy);
        let render: Vec<_> = doubled.mesh.positions[0..9].to_vec();
        doubled.mesh.positions.extend(render);
        let normals: Vec<_> = doubled.mesh.normals[0..9].to_vec();
        doubled.mesh.normals.extend(normals);
        doubled.triangle_faces.push(doubled.triangle_faces[0]);
        refuses(&doubled, "covering part of a face twice is not a tiling");

        // Every boundary side is present and the diagonal pairs, but the
        // whole face points into the box. The render normals stay outward.
        let mut inward = fresh();
        for triangle in 0..2 {
            let (one, two) = (
                inward.preview_positions_f64[triangle * 3 + 1],
                inward.preview_positions_f64[triangle * 3 + 2],
            );
            set_point(&mut inward, triangle * 3 + 1, two);
            set_point(&mut inward, triangle * 3 + 2, one);
        }
        refuses(&inward, "an inward face is not the outward face");

        let mut empty = fresh();
        empty.preview_positions_f64.drain(0..6);
        empty.mesh.positions.drain(0..18);
        empty.mesh.normals.drain(0..18);
        empty.triangle_faces.drain(0..2);
        refuses(&empty, "a face with no triangles covers no side");

        let mut retagged = fresh();
        let other_face = retagged
            .triangle_faces
            .iter()
            .position(|tag| *tag != retagged.triangle_faces[0])
            .unwrap();
        retagged.triangle_faces.swap(0, other_face);
        refuses(&retagged, "triangles belong to their own source face");

        let mut rendered = fresh();
        rendered.mesh.positions[0] = f32::from_bits(rendered.mesh.positions[0].to_bits() + 1);
        refuses(
            &rendered,
            "the f32 render image is part of the emitted mesh",
        );

        let mut normal = fresh();
        normal.mesh.normals[0] = -normal.mesh.normals[0] + 0.5;
        refuses(&normal, "render normals are outward axis normals");

        let mut bounds = fresh();
        bounds.bounds[1][0] += 1.0;
        refuses(&bounds, "asset bounds are the proven box");

        let mut moved = fresh();
        moved.face_occurrences[0].repair_displacement_mm = 0.5;
        refuses(&moved, "a recorded face movement is not zero repair");

        let mut representation = fresh();
        representation.occurrences[0].2 = 303;
        refuses(
            &representation,
            "occurrence identity is the proven representation",
        );

        let mut support = fresh();
        support.source_faces[0].support_id = support.source_faces[1].support_id;
        refuses(&support, "source faces keep their authored supports");

        let mut placed = fresh();
        placed.occurrences[0].1 = Matrix4::from_nonuniform_scale(1.0, 1.0, 1.0 + f64::EPSILON);
        refuses(&placed, "only the identity occurrence is proven");

        let mut fallback = fresh();
        fallback.accuracy.fallback_meshed_faces = 1;
        assert!(cuboid.proves(&fallback));
        assert!(
            !certify(&fallback),
            "a fallback-meshed face is outside the class"
        );
    }

    #[test]
    fn refinement_and_budgets_keep_the_same_exact_asset_on_either_route() {
        for (bytes, native) in [(BOX.to_vec(), true), (ap242(), false)] {
            let coarse = delivered(&bytes, StepImportOptions::default(), native);
            for options in [
                StepImportOptions {
                    target_deflection_mm: Some(1e-6),
                    ..Default::default()
                },
                StepImportOptions {
                    preview_repair_budget_mm: Some(0.5),
                    target_deflection_mm: Some(0.01),
                },
            ] {
                let fine = delivered(&bytes, options, native);
                assert_eq!(fine.accuracy.max_repair_displacement_mm, Some(0.0));
                assert_eq!(fine.preview_positions_f64, coarse.preview_positions_f64);
                assert_eq!(fine.triangle_faces, coarse.triangle_faces);
                assert_eq!(fine.bounds, coarse.bounds);
            }
        }
    }
}
