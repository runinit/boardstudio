//! Exact source placement for repaired occurrences. The render path keeps its
//! existing matrix and operation order.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use repair::finite_line::{ExactPlacement, NativeSources};
use scene::{PlacementPath, PlacementStep};

#[derive(Default)]
pub(super) struct Inputs {
    frames: HashMap<(u64, bool), Frame>,
    units: HashMap<u64, i32>,
    pub values: HashSet<u64>,
}

struct Frame {
    origin: u64,
    x: Option<u64>,
    y: Option<u64>,
    z: Option<u64>,
    scale: Option<u64>,
    mapped: bool,
}

impl Inputs {
    pub fn add(
        &mut self,
        records: &SourceRecords<'_>,
        representation: u64,
        path: &PlacementPath,
    ) -> Option<()> {
        self.add_unit(records, representation)?;
        for step in path.steps() {
            self.add_unit(records, step.from_rep)?;
            self.add_unit(records, step.to_rep)?;
            if step.mapped && self.units[&step.from_rep] != self.units[&step.to_rep] {
                return None;
            }
            self.add_frame(records, step.from_frame, false)?;
            self.add_frame(records, step.to_frame, step.mapped)?;
        }
        Some(())
    }

    fn add_unit(&mut self, records: &SourceRecords<'_>, representation: u64) -> Option<()> {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.units.entry(representation) {
            entry.insert(units::exact_si_mm_exponent(records, representation)?);
        }
        Some(())
    }

    fn add_frame(&mut self, records: &SourceRecords<'_>, id: u64, mapped: bool) -> Option<()> {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.frames.entry((id, mapped)) {
            let direction = |param: &Parameter| {
                if matches!(param, Parameter::NotProvided) {
                    return Some(None);
                }
                let id = reference(param).ok()?;
                named_record(records, id, "DIRECTION").ok()?;
                Some(Some(id))
            };
            let frame = if mapped {
                let params = parameters(
                    named_record(records, id, "CARTESIAN_TRANSFORMATION_OPERATOR_3D").ok()?,
                    7,
                )
                .ok()?;
                Frame {
                    mapped,
                    origin: reference(&params[4]).ok()?,
                    x: direction(&params[2])?,
                    y: direction(&params[3])?,
                    z: direction(&params[6])?,
                    scale: (!matches!(params[5], Parameter::NotProvided)).then_some(id),
                }
            } else {
                let params = named_parameters(records, id, "AXIS2_PLACEMENT_3D", 4).ok()?;
                Frame {
                    mapped,
                    origin: reference(&params[1]).ok()?,
                    x: direction(&params[3])?,
                    y: None,
                    z: direction(&params[2])?,
                    scale: None,
                }
            };
            named_record(records, frame.origin, "CARTESIAN_POINT").ok()?;
            self.values.insert(frame.origin);
            self.values.extend(
                [frame.x, frame.y, frame.z, frame.scale]
                    .into_iter()
                    .flatten(),
            );
            entry.insert(frame);
        }
        Some(())
    }

    pub fn resolve(
        &self,
        source: &NativeSources,
        representation: u64,
        path: &PlacementPath,
        steps: &mut HashMap<PlacementStep, ExactPlacement>,
    ) -> Option<ExactPlacement> {
        let scale = power_of_ten(self.units[&representation]);
        let mut world = ExactPlacement::from_coefficients(
            std::array::from_fn(|col| {
                std::array::from_fn(|row| if col == row { scale.clone() } else { Q::zero() })
            }),
            std::array::from_fn(|_| Q::zero()),
        )?;
        for step in path.steps() {
            if let std::collections::hash_map::Entry::Vacant(entry) = steps.entry(step) {
                let from = self.frames[&(step.from_frame, false)]
                    .resolve(source, self.units[&step.from_rep])?;
                let to = self.frames[&(step.to_frame, step.mapped)]
                    .resolve(source, self.units[&step.to_rep])?;
                entry.insert(to.compose(&from.inverse()?)?);
            }
            world = steps[&step].compose(&world)?;
        }
        Some(world)
    }
}

impl Frame {
    fn resolve(&self, source: &NativeSources, mm_exponent: i32) -> Option<ExactPlacement> {
        // Accept only source directions whose normalization is exactly a signed
        // axis. A direction that rounds to such an axis supplies no authority.
        let direction = |id: u64| {
            let raw = source.placement_point(id)?;
            if raw.iter().filter(|v| !v.is_zero()).count() != 1 {
                return None;
            }
            Some(raw.map(|v| i32::from(v.is_positive()) - i32::from(v.is_negative())))
        };
        let axis = |id: Option<u64>, default| match id {
            Some(id) => direction(id),
            None => Some(default),
        };
        let z = axis(self.z, [0, 0, 1])?;
        let x = axis(self.x, if z[0] == 0 { [1, 0, 0] } else { [0, 1, 0] })?;
        if (0..3).map(|i| x[i] * z[i]).sum::<i32>() != 0 {
            return None;
        }
        let mut y = std::array::from_fn(|i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            z[j] * x[k] - z[k] * x[j]
        });
        if let Some(id) = self.y {
            let authored = direction(id)?;
            if authored == y.map(|v| -v) {
                y = authored;
            } else if authored != y {
                return None;
            }
        }
        // For a mapped operator, second_proj_axis projects default +Y onto
        // the remaining axis. The renderer uses Z × X for an omitted axis2.
        // On signed coordinate frames the two agree exactly only in this case.
        if self.mapped && self.y.is_none() && y != [0, 1, 0] {
            return None;
        }
        let scale = match self.scale {
            Some(id) => source.placement_scale(id)?,
            None => Q::from_integer(1.into()),
        };
        let factor = power_of_ten(mm_exponent);
        ExactPlacement::from_coefficients(
            [x, y, z].map(|axis| axis.map(|v| &scale * Q::from_integer(v.into()))),
            source.placement_point(self.origin)?.map(|v| v * &factor),
        )
    }
}

fn power_of_ten(exp: i32) -> Q {
    Q::from_integer(10.into()).pow(exp)
}
