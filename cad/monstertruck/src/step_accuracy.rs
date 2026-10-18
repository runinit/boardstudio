//! Independent planar B-rep measurements from Part21 records, without kernel conversion.
//! Unsupported curved geometry fails explicitly; it is never measured as vertex chords.
use std::collections::{BTreeMap, BTreeSet};
use step_p21::ast::{EntityInstance, Exchange, Name, Parameter, Record};

mod circular;

#[derive(Clone, Copy)]
pub(super) struct Countersink {
    pub bounds: [[f64; 3]; 2],
    pub center: [f64; 2],
    pub z: [f64; 2],
    pub radii: [f64; 2],
}

pub(super) struct CountersinkCertificate {
    pub bounds: [[f64; 3]; 2],
    pub bound_error: f64,
    pub volume: [f64; 2],
    pub topology: [usize; 3],
    shape: Countersink,
}

impl CountersinkCertificate {
    pub fn contains(&self, point: [f64; 3]) -> Option<bool> {
        if !point.into_iter().all(f64::is_finite) {
            return None;
        }
        let e = self.bound_error;
        if (0..3).any(|axis| {
            self.shape
                .bounds
                .iter()
                .any(|p| (p[axis] - point[axis]).abs() <= e)
        }) {
            return None;
        }
        let radial = (point[0] - self.shape.center[0]).hypot(point[1] - self.shape.center[1]);
        if (point[2] - self.shape.z[0]).abs() <= e && radial <= self.shape.radii[0] + e {
            return None;
        }
        let inside = (0..3)
            .all(|axis| point[axis] > self.bounds[0][axis] && point[axis] < self.bounds[1][axis]);
        if !inside || point[2] < self.shape.z[0] {
            return Some(inside);
        }
        let t = (point[2] - self.shape.z[0]) / (self.shape.z[1] - self.shape.z[0]);
        let radius = self.shape.radii[0] + (self.shape.radii[1] - self.shape.radii[0]) * t;
        if (radial - radius).abs() <= e {
            None
        } else {
            Some(radial > radius)
        }
    }
}

pub(super) fn certify_countersink(
    text: &str,
    expected: Countersink,
) -> Result<CountersinkCertificate> {
    circular::certify_countersink(text, expected)
}

#[derive(Clone, Copy)]
pub(super) struct CircularExtrusion {
    pub center: [f64; 2],
    pub radius: f64,
    pub z: [f64; 2],
}

pub(super) struct CircularCertificate {
    pub bounds: [[f64; 3]; 2],
    pub bound_error: f64,
    pub volume: [f64; 2],
    pub topology: [usize; 3],
    shape: CircularExtrusion,
    inner_radius: Option<f64>,
}

impl CircularCertificate {
    pub fn contains(&self, point: [f64; 3]) -> Option<bool> {
        if !point.into_iter().all(f64::is_finite) {
            return None;
        }
        let radius = (point[0] - self.shape.center[0]).hypot(point[1] - self.shape.center[1]);
        if (radius - self.shape.radius).abs() <= self.bound_error
            || self
                .inner_radius
                .is_some_and(|inner| (radius - inner).abs() <= self.bound_error)
            || self
                .shape
                .z
                .iter()
                .any(|z| (point[2] - z).abs() <= self.bound_error)
        {
            None
        } else {
            Some(
                radius < self.shape.radius
                    && self.inner_radius.is_none_or(|inner| radius > inner)
                    && point[2] > self.shape.z[0]
                    && point[2] < self.shape.z[1],
            )
        }
    }
}

pub(super) fn certify_annular_extrusion(
    text: &str,
    expected: CircularExtrusion,
    inner_radius: f64,
) -> Result<CircularCertificate> {
    circular::certify_annulus(text, expected, inner_radius)
}

pub(super) fn certify_circular_extrusion(
    text: &str,
    expected: CircularExtrusion,
) -> Result<CircularCertificate> {
    circular::certify(text, expected)
}

/// One horizontal-plane trim segment in traversal order: a line or a circular arc.
#[derive(Clone, Copy)]
pub(super) enum Segment {
    Line {
        from: [f64; 2],
        to: [f64; 2],
    },
    Arc {
        center: [f64; 2],
        radius: f64,
        from: [f64; 2],
        to: [f64; 2],
        turn: f64,
    },
}

/// A raw-Part21 measurement of one vertical-wall prism: two horizontal caps joined by
/// planar and Z-cylindrical walls, with every arc proved to lie on its declared cylinder.
pub(super) struct PrismCertificate {
    pub bounds: [[f64; 3]; 2],
    pub bound_error: f64,
    pub area: [f64; 2],
    pub volume: [f64; 2],
    /// Vertices, edges and faces of the shell.
    pub topology: [usize; 3],
    /// Through holes: inner loops on each cap.
    pub holes: usize,
    /// Declared (center, radius) of every cylindrical wall face.
    pub cylinders: Vec<([f64; 2], f64)>,
    /// Number of planar wall faces.
    pub planar_walls: usize,
    z: [f64; 2],
    /// Top-cap loops, outer counter-clockwise and holes clockwise about +Z.
    loops: Vec<Vec<Segment>>,
}

pub(super) fn certify_prisms(text: &str) -> Result<Vec<PrismCertificate>> {
    circular::certify_prisms(text)
}

type Point = [f64; 3];
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq)]
pub(super) enum Error {
    Syntax,
    Units,
    NoSolids,
    Topology,
    Unsupported,
    Geometry,
}

pub(super) struct SolidFacts {
    pub bounds: [Point; 2],
    pub volume: f64,
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    triangles: Vec<[Point; 3]>,
}

impl SolidFacts {
    pub fn contains(&self, point: Point) -> bool {
        let angle: f64 = self
            .triangles
            .iter()
            .map(|triangle| {
                let [a, b, c] = triangle.map(|p| sub(p, point));
                let lengths = [a, b, c].map(|v| dot(v, v).sqrt());
                let denominator = lengths.iter().product::<f64>()
                    + dot(a, b) * lengths[2]
                    + dot(b, c) * lengths[0]
                    + dot(c, a) * lengths[1];
                2.0 * dot(a, cross(b, c)).atan2(denominator)
            })
            .sum();
        angle.abs() > 2.0 * std::f64::consts::PI
    }
}

pub(super) fn inspect(text: &str) -> Result<Vec<SolidFacts>> {
    let doc = parse(text)?;
    let ids = doc.ids("MANIFOLD_SOLID_BREP");
    if ids.is_empty() {
        return Err(Error::NoSolids);
    }
    let scales = doc.solid_scales(&ids)?;
    doc.check_scope()?;
    let mut used_faces = BTreeSet::new();
    let solids = ids
        .into_iter()
        .map(|id| doc.solid(id, scales[&id], &mut used_faces))
        .collect::<Result<Vec<_>>>()?;
    if used_faces != doc.ids("ADVANCED_FACE").into_iter().collect() {
        return Err(Error::Topology);
    }
    Ok(solids)
}

pub(super) fn product_solid_counts(text: &str) -> Result<Vec<(String, String, usize)>> {
    let doc = parse(text)?;
    doc.check_scope()?;
    doc.ids("SHAPE_DEFINITION_REPRESENTATION")
        .into_iter()
        .map(|id| {
            let definition = doc.record(id, "SHAPE_DEFINITION_REPRESENTATION")?;
            let product = doc.product(definition)?;
            let representation = doc.record(reference(&definition[1])?, "SHAPE_REPRESENTATION")?;
            let shapes = refs(&representation[1])?;
            let count = shapes
                .iter()
                .filter(|&&id| doc.record(id, "MANIFOLD_SOLID_BREP").is_ok())
                .count();
            let name = |p: &Parameter| match p {
                Parameter::String(s) => Ok(s.clone()),
                _ => Err(Error::Syntax),
            };
            Ok((name(&product[0])?, name(&product[1])?, count))
        })
        .collect()
}

fn parse(text: &str) -> Result<Document> {
    let exchange: Exchange = text.parse().map_err(|_| Error::Syntax)?;
    let mut records = BTreeMap::new();
    for entity in exchange.data.into_iter().flat_map(|data| data.entities) {
        let (id, list) = match entity {
            EntityInstance::Simple { id, record } => (id, vec![record]),
            EntityInstance::Complex { id, subsuper } => (id, subsuper.0),
        };
        if list
            .iter()
            .map(|record| &record.name)
            .collect::<BTreeSet<_>>()
            .len()
            != list.len()
        {
            return Err(Error::Topology);
        }
        if records.insert(id, list).is_some() {
            return Err(Error::Topology);
        }
    }
    Ok(Document(records))
}

struct Document(BTreeMap<u64, Vec<Record>>);

impl Document {
    fn check_complex_scope(&self) -> Result<()> {
        for records in self.0.values().filter(|records| records.len() > 1) {
            let names: BTreeSet<_> = records.iter().map(|record| record.name.as_str()).collect();
            let valid = if names.contains("B_SPLINE_CURVE") {
                names
                    == [
                        "BOUNDED_CURVE",
                        "B_SPLINE_CURVE",
                        "B_SPLINE_CURVE_WITH_KNOTS",
                        "CURVE",
                        "GEOMETRIC_REPRESENTATION_ITEM",
                        "RATIONAL_B_SPLINE_CURVE",
                        "REPRESENTATION_ITEM",
                    ]
                    .into_iter()
                    .collect()
            } else if names.contains("B_SPLINE_SURFACE") {
                names
                    == [
                        "BOUNDED_SURFACE",
                        "B_SPLINE_SURFACE",
                        "B_SPLINE_SURFACE_WITH_KNOTS",
                        "GEOMETRIC_REPRESENTATION_ITEM",
                        "RATIONAL_B_SPLINE_SURFACE",
                        "REPRESENTATION_ITEM",
                        "SURFACE",
                    ]
                    .into_iter()
                    .collect()
            } else if names.contains("GEOMETRIC_REPRESENTATION_CONTEXT") {
                names.contains("REPRESENTATION_CONTEXT")
                    && names.iter().all(|name| {
                        matches!(
                            *name,
                            "GEOMETRIC_REPRESENTATION_CONTEXT"
                                | "REPRESENTATION_CONTEXT"
                                | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
                                | "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT"
                                | "PARAMETRIC_REPRESENTATION_CONTEXT"
                        )
                    })
            } else if names.contains("NAMED_UNIT") {
                names.len() == 3
                    && names
                        .intersection(&["SI_UNIT", "CONVERSION_BASED_UNIT"].into_iter().collect())
                        .count()
                        == 1
                    && names
                        .intersection(
                            &["LENGTH_UNIT", "PLANE_ANGLE_UNIT", "SOLID_ANGLE_UNIT"]
                                .into_iter()
                                .collect(),
                        )
                        .count()
                        == 1
            } else {
                false
            };
            if !valid {
                return Err(Error::Unsupported);
            }
        }
        Ok(())
    }
    fn check_scope(&self) -> Result<()> {
        self.check_complex_scope()?;
        // Authenticate the bounded direct-solid scope before ignoring metadata: occurrence,
        // mapping and representation-relationship graphs require the separate placed oracle.
        if self
            .0
            .values()
            .flatten()
            .any(|record| !supported_record(record))
        {
            return Err(Error::Unsupported);
        }
        Ok(())
    }

    fn ids(&self, name: &str) -> Vec<u64> {
        self.0
            .iter()
            .filter(|(_, list)| list.iter().any(|r| r.name == name))
            .map(|(&id, _)| id)
            .collect()
    }

    fn record(&self, id: u64, name: &str) -> Result<&[Parameter]> {
        let record = self
            .0
            .get(&id)
            .ok_or(Error::Topology)?
            .iter()
            .find(|r| r.name == name)
            .ok_or(Error::Unsupported)?;
        let values = list(&record.parameter)?;
        let arity = match name {
            "GEOMETRIC_REPRESENTATION_CONTEXT"
            | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
            | "RATIONAL_B_SPLINE_CURVE"
            | "RATIONAL_B_SPLINE_SURFACE" => Some(1),
            "REPRESENTATION_CONTEXT"
            | "SI_UNIT"
            | "CONVERSION_BASED_UNIT"
            | "LENGTH_MEASURE_WITH_UNIT"
            | "SHAPE_DEFINITION_REPRESENTATION"
            | "CARTESIAN_POINT"
            | "DIRECTION"
            | "VERTEX_POINT"
            | "PLANE"
            | "EDGE_LOOP"
            | "CLOSED_SHELL"
            | "MANIFOLD_SOLID_BREP" => Some(2),
            "PRODUCT_DEFINITION_FORMATION"
            | "PRODUCT_DEFINITION_SHAPE"
            | "SHAPE_REPRESENTATION"
            | "ADVANCED_BREP_SHAPE_REPRESENTATION"
            | "AXIS2_PLACEMENT_2D"
            | "LINE"
            | "VECTOR"
            | "CIRCLE"
            | "CYLINDRICAL_SURFACE"
            | "PCURVE"
            | "DEFINITIONAL_REPRESENTATION"
            | "FACE_BOUND"
            | "FACE_OUTER_BOUND"
            | "B_SPLINE_CURVE_WITH_KNOTS" => Some(3),
            "PRODUCT" | "PRODUCT_DEFINITION" | "AXIS2_PLACEMENT_3D" | "SURFACE_CURVE"
            | "SEAM_CURVE" | "ADVANCED_FACE" => Some(4),
            "EDGE_CURVE" | "ORIENTED_EDGE" | "B_SPLINE_CURVE" | "B_SPLINE_SURFACE_WITH_KNOTS" => {
                Some(5)
            }
            "B_SPLINE_SURFACE" => Some(7),
            _ => None,
        };
        if arity.is_some_and(|arity| values.len() != arity) {
            return Err(Error::Syntax);
        }
        Ok(values)
    }

    fn solid_scales(&self, solid_ids: &[u64]) -> Result<BTreeMap<u64, f64>> {
        let mut scales = BTreeMap::new();
        for id in self.ids("SHAPE_DEFINITION_REPRESENTATION") {
            let definition = self.record(id, "SHAPE_DEFINITION_REPRESENTATION")?;
            self.product(definition)?;
            let representation_id = reference(definition.get(1).ok_or(Error::Topology)?)?;
            let representation = self
                .record(representation_id, "SHAPE_REPRESENTATION")
                .or_else(|_| {
                    self.record(representation_id, "ADVANCED_BREP_SHAPE_REPRESENTATION")
                })?;
            let context = reference(representation.get(2).ok_or(Error::Units)?)?;
            let scale = self.context_scale(context)?;
            for item in refs(representation.get(1).ok_or(Error::Topology)?)? {
                if !self.0.contains_key(&item) {
                    return Err(Error::Topology);
                }
                if solid_ids.contains(&item) {
                    if scales.insert(item, scale).is_some() {
                        return Err(Error::Units);
                    }
                } else if self.record(item, "AXIS2_PLACEMENT_3D").is_err() {
                    // Mapped/nested occurrences require their own transform proof.
                    return Err(Error::Unsupported);
                }
            }
        }
        if scales.len() != solid_ids.len() {
            return Err(Error::Topology);
        }
        Ok(scales)
    }

    fn product(&self, definition: &[Parameter]) -> Result<&[Parameter]> {
        let shape = self.record(
            reference(definition.first().ok_or(Error::Topology)?)?,
            "PRODUCT_DEFINITION_SHAPE",
        )?;
        let product_definition = self.record(
            reference(shape.get(2).ok_or(Error::Topology)?)?,
            "PRODUCT_DEFINITION",
        )?;
        let formation = self.record(
            reference(product_definition.get(2).ok_or(Error::Topology)?)?,
            "PRODUCT_DEFINITION_FORMATION",
        )?;
        self.record(
            reference(formation.get(2).ok_or(Error::Topology)?)?,
            "PRODUCT",
        )
    }

    fn context_scale(&self, id: u64) -> Result<f64> {
        let context = self.record(id, "GEOMETRIC_REPRESENTATION_CONTEXT")?;
        if context.first().map(number).transpose()? != Some(3.0) {
            return Err(Error::Units);
        }
        self.record(id, "REPRESENTATION_CONTEXT")?;
        let units = self.record(id, "GLOBAL_UNIT_ASSIGNED_CONTEXT")?;
        let mut lengths = Vec::new();
        for unit in refs(units.first().ok_or(Error::Units)?)? {
            if !self.0.contains_key(&unit) {
                return Err(Error::Units);
            }
            if self.record(unit, "LENGTH_UNIT").is_ok() {
                lengths.push(unit);
            }
        }
        if lengths.len() != 1 {
            return Err(Error::Units);
        }
        self.unit_scale(lengths[0], &mut BTreeSet::new())
    }

    fn unit_scale(&self, id: u64, visited: &mut BTreeSet<u64>) -> Result<f64> {
        if !visited.insert(id) {
            return Err(Error::Units);
        }
        if let Ok(si) = self.record(id, "SI_UNIT") {
            if si.get(1) != Some(&Parameter::Enumeration("METRE".into())) {
                return Err(Error::Units);
            }
            return match &si[0] {
                Parameter::Enumeration(prefix) if prefix == "MILLI" => Ok(1.0),
                Parameter::NotProvided => Ok(1000.0),
                _ => Err(Error::Units),
            };
        }
        let unit = self.record(id, "CONVERSION_BASED_UNIT")?;
        let measure = self.record(
            reference(unit.get(1).ok_or(Error::Units)?)?,
            "LENGTH_MEASURE_WITH_UNIT",
        )?;
        let factor = number(measure.first().ok_or(Error::Units)?)?
            * self.unit_scale(reference(measure.get(1).ok_or(Error::Units)?)?, visited)?;
        if factor.is_finite() && factor > 0.0 {
            Ok(factor)
        } else {
            Err(Error::Units)
        }
    }

    fn point(&self, id: u64) -> Result<Point> {
        vector(&self.record(id, "CARTESIAN_POINT")?[1])
    }

    fn vertex(&self, id: u64) -> Result<Point> {
        self.point(reference(&self.record(id, "VERTEX_POINT")?[1])?)
    }

    fn plane(&self, id: u64) -> Result<(Point, Point)> {
        let place = self.record(
            reference(&self.record(id, "PLANE")?[1])?,
            "AXIS2_PLACEMENT_3D",
        )?;
        let origin = self.point(reference(&place[1])?)?;
        let normal = match place.get(2) {
            Some(Parameter::NotProvided) => [0.0, 0.0, 1.0],
            Some(axis) => vector(&self.record(reference(axis)?, "DIRECTION")?[1])?,
            None => return Err(Error::Geometry),
        };
        if dot(normal, normal) == 0.0 {
            return Err(Error::Geometry);
        }
        Ok((origin, normal))
    }

    fn check_line(&self, id: u64, start: Point, end: Point) -> Result<()> {
        let line_id = if self.record(id, "LINE").is_ok() {
            id
        } else {
            let record = self
                .record(id, "SURFACE_CURVE")
                .or_else(|_| self.record(id, "SEAM_CURVE"))?;
            let associated = refs(record.get(2).ok_or(Error::Geometry)?)?;
            if associated.is_empty() || associated.len() > 2 {
                return Err(Error::Geometry);
            }
            let selected = match record.get(3) {
                Some(Parameter::Enumeration(value)) if value == "CURVE_3D" => None,
                Some(Parameter::Enumeration(value)) if value == "PCURVE_S1" => Some(0),
                Some(Parameter::Enumeration(value)) if value == "PCURVE_S2" => Some(1),
                _ => return Err(Error::Unsupported),
            };
            if let Some(index) = selected {
                self.record(*associated.get(index).ok_or(Error::Geometry)?, "PCURVE")?;
            }
            for association in associated {
                if self.record(association, "PCURVE").is_ok() {
                    let (origin, direction) = self.plane_pcurve_line(association)?;
                    check_supporting_line(origin, direction, start, end)?;
                } else {
                    let (origin, normal) = self.plane(association)?;
                    for point in [start, end] {
                        if dot(sub(point, origin), normal).abs()
                            > 1.0e-7 * dot(normal, normal).sqrt()
                        {
                            return Err(Error::Geometry);
                        }
                    }
                }
            }
            reference(&record[1])?
        };
        let line = self.record(line_id, "LINE")?;
        let origin = self.point(reference(&line[1])?)?;
        let v = self.record(reference(&line[2])?, "VECTOR")?;
        let direction = vector(&self.record(reference(&v[1])?, "DIRECTION")?[1])?;
        if number(&v[2])? <= 0.0 {
            return Err(Error::Geometry);
        }
        check_supporting_line(origin, direction, start, end)
    }

    fn plane_pcurve_line(&self, id: u64) -> Result<(Point, Point)> {
        let pcurve = self.record(id, "PCURVE")?;
        let plane_id = reference(pcurve.get(1).ok_or(Error::Geometry)?)?;
        let (origin, normal) = self.plane(plane_id)?;
        let place = self.record(
            reference(&self.record(plane_id, "PLANE")?[1])?,
            "AXIS2_PLACEMENT_3D",
        )?;
        let z = unit(normal)?;
        let ref_direction = match place.get(3) {
            Some(Parameter::NotProvided) if z == [1.0, 0.0, 0.0] || z == [-1.0, 0.0, 0.0] => {
                [0.0, 1.0, 0.0]
            }
            Some(Parameter::NotProvided) => [1.0, 0.0, 0.0],
            Some(axis) => vector(&self.record(reference(axis)?, "DIRECTION")?[1])?,
            None => return Err(Error::Geometry),
        };
        let x = unit(sub(ref_direction, z.map(|v| v * dot(ref_direction, z))))?;
        let y = cross(z, x);
        let definition = self.record(
            reference(pcurve.get(2).ok_or(Error::Geometry)?)?,
            "DEFINITIONAL_REPRESENTATION",
        )?;
        let curves = refs(definition.get(1).ok_or(Error::Geometry)?)?;
        if curves.len() != 1 {
            return Err(Error::Unsupported);
        }
        let context = reference(definition.get(2).ok_or(Error::Geometry)?)?;
        self.record(context, "PARAMETRIC_REPRESENTATION_CONTEXT")?;
        if self
            .record(context, "GEOMETRIC_REPRESENTATION_CONTEXT")?
            .first()
            .map(number)
            .transpose()?
            != Some(2.0)
        {
            return Err(Error::Geometry);
        }
        let line = self.record(curves[0], "LINE")?;
        let uv = list(&self.record(reference(&line[1])?, "CARTESIAN_POINT")?[1])?;
        let v = self.record(reference(&line[2])?, "VECTOR")?;
        let direction = list(&self.record(reference(&v[1])?, "DIRECTION")?[1])?;
        if uv.len() != 2 || direction.len() != 2 || number(&v[2])? <= 0.0 {
            return Err(Error::Geometry);
        }
        let (u, v) = (number(&uv[0])?, number(&uv[1])?);
        let (du, dv) = (number(&direction[0])?, number(&direction[1])?);
        Ok((
            std::array::from_fn(|i| origin[i] + u * x[i] + v * y[i]),
            std::array::from_fn(|i| du * x[i] + dv * y[i]),
        ))
    }

    fn solid(&self, id: u64, scale: f64, used_faces: &mut BTreeSet<u64>) -> Result<SolidFacts> {
        let shell = reference(&self.record(id, "MANIFOLD_SOLID_BREP")?[1])?;
        let faces = refs(&self.record(shell, "CLOSED_SHELL")?[1])?;
        let mut vertices = BTreeSet::new();
        let mut edge_uses: BTreeMap<u64, Vec<bool>> = BTreeMap::new();
        let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
        let mut triangles = Vec::new();
        for &face_id in &faces {
            if !used_faces.insert(face_id) {
                return Err(Error::Topology);
            }
            let face = self.record(face_id, "ADVANCED_FACE")?;
            let (origin, normal) = self.plane(reference(&face[2])?)?;
            let sense = boolean(&face[3])?;
            let face_normal = normal.map(|v| v * if sense { 1.0 } else { -1.0 });
            let mut areas = Vec::new();
            for bound_id in refs(&face[1])? {
                let bound = self
                    .record(bound_id, "FACE_BOUND")
                    .or_else(|_| self.record(bound_id, "FACE_OUTER_BOUND"))?;
                let loop_id = reference(&bound[1])?;
                let mut segments = Vec::new();
                for oriented in refs(&self.record(loop_id, "EDGE_LOOP")?[1])? {
                    let edge = self.record(oriented, "ORIENTED_EDGE")?;
                    let edge_id = reference(&edge[3])?;
                    let curve = self.record(edge_id, "EDGE_CURVE")?;
                    let (mut start, mut end) = (reference(&curve[1])?, reference(&curve[2])?);
                    let forward = boolean(&edge[4])?;
                    if !forward {
                        std::mem::swap(&mut start, &mut end);
                    }
                    self.check_line(
                        reference(&curve[3])?,
                        self.vertex(start)?,
                        self.vertex(end)?,
                    )?;
                    segments.push((edge_id, forward, start, end));
                }
                if !boolean(&bound[2])? {
                    segments.reverse();
                    for (_, forward, start, end) in &mut segments {
                        *forward = !*forward;
                        std::mem::swap(start, end);
                    }
                }
                if segments.len() < 3
                    || segments
                        .iter()
                        .zip(segments.iter().cycle().skip(1))
                        .any(|(a, b)| a.3 != b.2)
                {
                    return Err(Error::Topology);
                }
                let mut points = Vec::new();
                for (edge_id, forward, start, _) in segments {
                    edge_uses.entry(edge_id).or_default().push(forward);
                    vertices.insert(start);
                    let point = self.vertex(start)?;
                    if dot(sub(point, origin), normal).abs() > 1.0e-7 * dot(normal, normal).sqrt() {
                        return Err(Error::Geometry);
                    }
                    let point = point.map(|x| x * scale);
                    for axis in 0..3 {
                        bounds[0][axis] = bounds[0][axis].min(point[axis]);
                        bounds[1][axis] = bounds[1][axis].max(point[axis]);
                    }
                    points.push(point);
                }
                for pair in points[1..].windows(2) {
                    triangles.push([points[0], pair[0], pair[1]]);
                }
                let area: f64 = points
                    .iter()
                    .zip(points.iter().cycle().skip(1))
                    .take(points.len())
                    .map(|(a, b)| dot(cross(sub(*a, points[0]), sub(*b, points[0])), face_normal))
                    .sum();
                areas.push(area);
            }
            if areas.iter().filter(|&&area| area > 0.0).count() != 1 || areas.contains(&0.0) {
                return Err(Error::Geometry);
            }
        }
        if faces.is_empty()
            || edge_uses
                .values()
                .any(|uses| uses.len() != 2 || uses[0] == uses[1])
        {
            return Err(Error::Topology);
        }
        let origin = triangles.first().ok_or(Error::Geometry)?[0];
        let volume: f64 = triangles
            .iter()
            .map(|[a, b, c]| dot(sub(*a, origin), cross(sub(*b, origin), sub(*c, origin))))
            .sum::<f64>()
            / 6.0;
        if !volume.is_finite() || volume <= 0.0 {
            return Err(Error::Geometry);
        }
        Ok(SolidFacts {
            bounds,
            volume,
            vertices: vertices.len(),
            edges: edge_uses.len(),
            faces: faces.len(),
            triangles,
        })
    }
}

fn list(value: &Parameter) -> Result<&[Parameter]> {
    if let Parameter::List(values) = value {
        Ok(values)
    } else {
        Err(Error::Syntax)
    }
}
fn reference(value: &Parameter) -> Result<u64> {
    if let Parameter::Ref(Name::Entity(id)) = value {
        Ok(*id)
    } else {
        Err(Error::Syntax)
    }
}
fn refs(value: &Parameter) -> Result<Vec<u64>> {
    list(value)?.iter().map(reference).collect()
}
fn number(value: &Parameter) -> Result<f64> {
    let value = match value {
        Parameter::Real(value) => *value,
        Parameter::Integer(value) => *value as f64,
        Parameter::Typed { parameter, .. } => return number(parameter),
        _ => return Err(Error::Geometry),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Geometry)
    }
}
fn boolean(value: &Parameter) -> Result<bool> {
    match value {
        Parameter::Enumeration(value) if value == "T" => Ok(true),
        Parameter::Enumeration(value) if value == "F" => Ok(false),
        _ => Err(Error::Syntax),
    }
}
fn vector(value: &Parameter) -> Result<Point> {
    let values = list(value)?;
    if values.len() != 3 {
        return Err(Error::Geometry);
    }
    Ok([
        number(&values[0])?,
        number(&values[1])?,
        number(&values[2])?,
    ])
}
fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: Point, b: Point) -> f64 {
    a.into_iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(v: Point) -> Result<Point> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    if !length.is_finite() || length == 0.0 {
        return Err(Error::Geometry);
    }
    Ok(v.map(|v| v / length))
}

fn check_supporting_line(origin: Point, direction: Point, start: Point, end: Point) -> Result<()> {
    if !origin
        .into_iter()
        .chain(direction)
        .chain(start)
        .chain(end)
        .all(f64::is_finite)
    {
        return Err(Error::Geometry);
    }
    let direction = unit(direction)?;
    for point in [start, end] {
        let offset = cross(sub(point, origin), direction);
        let distance = offset[0].hypot(offset[1]).hypot(offset[2]);
        if !distance.is_finite() || distance > 1.0e-7 {
            return Err(Error::Geometry);
        }
    }
    Ok(())
}

fn supported_record(record: &Record) -> bool {
    matches!(
        record.name.as_str(),
        "ADVANCED_BREP_SHAPE_REPRESENTATION"
            | "ADVANCED_FACE"
            | "APPLICATION_CONTEXT"
            | "APPLICATION_PROTOCOL_DEFINITION"
            | "AXIS2_PLACEMENT_3D"
            | "CARTESIAN_POINT"
            | "CLOSED_SHELL"
            | "COLOUR_RGB"
            | "CONVERSION_BASED_UNIT"
            | "CURVE_STYLE"
            | "DEFINITIONAL_REPRESENTATION"
            | "DESIGN_CONTEXT"
            | "DIRECTION"
            | "DRAUGHTING_PRE_DEFINED_CURVE_FONT"
            | "EDGE_CURVE"
            | "EDGE_LOOP"
            | "FACE_BOUND"
            | "FACE_OUTER_BOUND"
            | "FILL_AREA_STYLE"
            | "FILL_AREA_STYLE_COLOUR"
            | "GEOMETRIC_REPRESENTATION_CONTEXT"
            | "GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT"
            | "GLOBAL_UNIT_ASSIGNED_CONTEXT"
            | "LENGTH_MEASURE_WITH_UNIT"
            | "LENGTH_UNIT"
            | "LINE"
            | "MANIFOLD_SOLID_BREP"
            | "MECHANICAL_CONTEXT"
            | "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION"
            | "NAMED_UNIT"
            | "ORIENTED_EDGE"
            | "PARAMETRIC_REPRESENTATION_CONTEXT"
            | "PCURVE"
            | "PLANE"
            | "PLANE_ANGLE_UNIT"
            | "PRESENTATION_STYLE_ASSIGNMENT"
            | "PRODUCT"
            | "PRODUCT_CONTEXT"
            | "PRODUCT_DEFINITION"
            | "PRODUCT_DEFINITION_CONTEXT"
            | "PRODUCT_DEFINITION_FORMATION"
            | "PRODUCT_DEFINITION_SHAPE"
            | "PRODUCT_RELATED_PRODUCT_CATEGORY"
            | "REPRESENTATION_CONTEXT"
            | "SEAM_CURVE"
            | "SHAPE_DEFINITION_REPRESENTATION"
            | "SHAPE_REPRESENTATION"
            | "SI_UNIT"
            | "SOLID_ANGLE_UNIT"
            | "STYLED_ITEM"
            | "SURFACE_CURVE"
            | "SURFACE_SIDE_STYLE"
            | "SURFACE_STYLE_FILL_AREA"
            | "SURFACE_STYLE_USAGE"
            | "UNCERTAINTY_MEASURE_WITH_UNIT"
            | "VECTOR"
            | "VERTEX_POINT"
    )
}
