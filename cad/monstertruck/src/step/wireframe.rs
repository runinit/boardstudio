//! Finite source LINE presentation, kept separate from surface triangles.
use super::*;
use monstertruck_io::step::load::step_p21::tables::IntoOwned;
use monstertruck_modeling::Vector3;

// The original typed point/vector frame and scalar trim are retained. An
// endpoint-pair line would lose that frame when q - p cancels at a large origin.
struct SourceLine {
    _basis_id: u64,
    _source_line: monstertruck_io::step::load::LineHolder,
    _interval: (f64, f64),
    _sense_agreement: bool,
    endpoints: [Point3; 2],
}

struct CurveOccurrence {
    _curve_id: u64,
    _set_id: u64,
    _representation_id: u64,
    _mm_per_unit: f64,
    _placement: Matrix4,
    _declared_uncertainty_mm: Option<f64>,
}

/// Internal until the line-capable worker/renderer delivery seam is approved.
#[derive(Default)]
pub(super) struct Wireframe {
    _definitions: HashMap<u64, SourceLine>,
    _sets: HashMap<u64, Vec<u64>>,
    occurrences: Vec<CurveOccurrence>,
    segments: Vec<[Point3; 2]>,
    render_positions: Vec<[f32; 3]>,
    _segment_occurrences: Vec<usize>,
}

impl Wireframe {
    pub(super) fn build(
        exchange: &Exchange,
        table: &Table,
        occurrences: &[(u64, Matrix4, u64)],
        units: &HashMap<u64, f64>,
        precision: &[StepPrecision],
    ) -> Result<Self, String> {
        if table.geometric_curve_set.is_empty() && occurrences.is_empty() {
            return Ok(Self::default());
        }
        let records = source_records(exchange);
        let mut result = Self::default();
        for (set_id, placement, representation_id) in occurrences {
            super::units::validate_placement_matrix(*placement)?;
            let factor = *units
                .get(representation_id)
                .ok_or("wireframe representation lacks units")?;
            if !factor.is_finite() || factor <= 0. {
                return Err("invalid wireframe representation unit".into());
            }
            if !result._sets.contains_key(set_id) {
                let parameters =
                    parameters(named_record(&records, *set_id, "GEOMETRIC_CURVE_SET")?, 2)?;
                let Parameter::List(elements) = &parameters[1] else {
                    return Err("source curve set lacks its member SET".into());
                };
                if elements.is_empty() {
                    return Err("source curve set is empty".into());
                }
                let mut ids = Vec::new();
                let mut seen = HashSet::new();
                for element in elements {
                    let curve_id = reference(element)?;
                    if !seen.insert(curve_id) {
                        return Err("source curve set repeats a member".into());
                    }
                    if let std::collections::hash_map::Entry::Vacant(entry) =
                        result._definitions.entry(curve_id)
                    {
                        let line = source_line(&records, table, curve_id)
                            .map_err(|error| format!("source curve #{curve_id}: {error}"))?;
                        entry.insert(line);
                    }
                    ids.push(curve_id);
                }
                result._sets.insert(*set_id, ids);
            }
            let world = *placement * Matrix4::from_scale(factor);
            let owner = precision
                .iter()
                .find(|owner| owner.representation_id == *representation_id)
                .ok_or("wireframe representation precision is missing")?;
            let scale = placement.x.truncate().magnitude();
            let uncertainty = owner.uncertainty_mm().map(|bound| bound * scale);
            if uncertainty.is_some_and(|bound| !bound.is_finite()) {
                return Err("nonfinite placed wireframe declaration".into());
            }
            for curve_id in &result._sets[set_id] {
                let endpoints = result._definitions[curve_id]
                    .endpoints
                    .map(|p| world.transform_point(p));
                let rendered = render_segment(endpoints)?;
                let index = result.occurrences.len();
                result.occurrences.push(CurveOccurrence {
                    _curve_id: *curve_id,
                    _set_id: *set_id,
                    _representation_id: *representation_id,
                    _mm_per_unit: factor,
                    _placement: *placement,
                    _declared_uncertainty_mm: uncertainty,
                });
                result.segments.push(endpoints);
                result.render_positions.extend(rendered);
                result._segment_occurrences.push(index);
            }
        }
        if table
            .geometric_curve_set
            .keys()
            .any(|id| !result._sets.contains_key(id))
        {
            return Err("unreferenced source curve set was not traversed".into());
        }
        Ok(result)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.occurrences.is_empty()
    }

    pub(super) fn include_bounds(&self, bounds: &mut [[f64; 3]; 2]) {
        for point in self
            .segments
            .iter()
            .flatten()
            .map(|p| [p.x, p.y, p.z])
            .chain(self.render_positions.iter().map(|p| p.map(f64::from)))
        {
            for axis in 0..3 {
                bounds[0][axis] = bounds[0][axis].min(point[axis]);
                bounds[1][axis] = bounds[1][axis].max(point[axis]);
            }
        }
    }
}

fn source_line(records: &SourceRecords<'_>, table: &Table, id: u64) -> Result<SourceLine, String> {
    let parameters = parameters(named_record(records, id, "TRIMMED_CURVE")?, 6)?;
    if !matches!(&parameters[0], Parameter::String(_))
        || !matches!(&parameters[5], Parameter::Enumeration(kind) if kind == "PARAMETER")
    {
        return Err("only authored finite PARAMETER line trims are supported".into());
    }
    let basis_id = reference(&parameters[1])?;
    named_record(records, basis_id, "LINE")?;
    let holder = table
        .line
        .get(&basis_id)
        .ok_or("typed LINE basis is missing")?
        .clone();
    let line = holder
        .clone()
        .into_owned(table)
        .map_err(|error| error.to_string())?;
    if line.pnt.coordinates.len() != 3
        || line.dir.orientation.direction_ratios.len() != 3
        || !line
            .pnt
            .coordinates
            .iter()
            .chain(&line.dir.orientation.direction_ratios)
            .all(|x| x.is_finite())
        || !line.dir.magnitude.is_finite()
        || line.dir.magnitude <= 0.
    {
        return Err("invalid finite three-dimensional source LINE frame".into());
    }
    let direction = Vector3::from(&line.dir.orientation);
    if !direction.magnitude2().is_finite() || direction.magnitude2() == 0. {
        return Err("invalid source LINE direction".into());
    }
    let vector = direction.normalize() * line.dir.magnitude;
    let interval = (
        trim_parameter(&parameters[2])?,
        trim_parameter(&parameters[3])?,
    );
    if interval.0 >= interval.1 {
        return Err("source LINE trim interval must be finite and ordered".into());
    }
    let sense = match &parameters[4] {
        Parameter::Enumeration(value) if value == "T" => true,
        Parameter::Enumeration(value) if value == "F" => false,
        _ => return Err("invalid source curve sense agreement".into()),
    };
    let origin = Point3::from(&line.pnt);
    let mut endpoints = [origin + vector * interval.0, origin + vector * interval.1];
    if !sense {
        endpoints.swap(0, 1);
    }
    if endpoints
        .iter()
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite())
        || endpoints[0] == endpoints[1]
    {
        return Err("nonfinite or zero-extent source LINE trim".into());
    }
    Ok(SourceLine {
        _basis_id: basis_id,
        _source_line: holder,
        _interval: interval,
        _sense_agreement: sense,
        endpoints,
    })
}

fn render_segment(endpoints: [Point3; 2]) -> Result<[[f32; 3]; 2], String> {
    if endpoints.iter().any(|p| {
        [p.x, p.y, p.z].iter().any(|v| {
            !v.is_finite() || !(*v as f32).is_finite() || (v * 1e7).abs() >= i64::MAX as f64
        })
    }) {
        return Err("nonfinite or unsupported source line coordinate range".into());
    }
    let rendered = endpoints.map(|p| [p.x as f32, p.y as f32, p.z as f32]);
    let rendered_direction =
        Point3::from(rendered[1].map(f64::from)) - Point3::from(rendered[0].map(f64::from));
    if rendered_direction.dot(endpoints[1] - endpoints[0]) <= 0. {
        return Err("source line endpoints collapse or reverse at f32 render precision".into());
    }
    Ok(rendered)
}

fn trim_parameter(parameter: &Parameter) -> Result<f64, String> {
    let Parameter::List(selectors) = parameter else {
        return Err("source curve trim lacks a SET".into());
    };
    let [Parameter::Typed { keyword, parameter }] = selectors.as_slice() else {
        return Err("source curve requires one parameter trim selector".into());
    };
    if keyword != "PARAMETER_VALUE" {
        return Err("source curve trim is not PARAMETER_VALUE".into());
    }
    let value = match parameter.as_ref() {
        Parameter::Real(value) => *value,
        Parameter::Integer(value) => *value as f64,
        _ => return Err("source curve parameter is not numeric".into()),
    };
    if !value.is_finite() {
        return Err("nonfinite source curve parameter".into());
    }
    Ok(value)
}
