//! Validate authored edge uses before cadmpeg can reorder or omit them.
use super::*;

#[derive(Default)]
pub(super) struct ShellUses {
    edges: HashMap<u64, Vec<(u64, bool)>>,
}

impl ShellUses {
    pub(super) fn face(
        &mut self,
        records: &SourceRecords<'_>,
        face: u64,
        bounds: &Parameter,
    ) -> Result<(), String> {
        for bound in references(bounds)? {
            let record = simple_record(records, bound)?;
            if !matches!(record.name.as_str(), "FACE_BOUND" | "FACE_OUTER_BOUND") {
                return Err(format!("invalid source boundary #{bound}"));
            }
            let args = parameters(record, 3)?;
            let forward_bound = flag(&args[2])?;
            let ring = named_record(records, reference(&args[1])?, "EDGE_LOOP")?;
            let uses = references(&parameters(ring, 2)?[1])?;
            let mut first = None;
            let mut last = None;
            for usage in uses {
                let oriented = parameters(named_record(records, usage, "ORIENTED_EDGE")?, 5)?;
                let edge_id = reference(&oriented[3])?;
                let forward = flag(&oriented[4])?;
                let edge = parameters(named_record(records, edge_id, "EDGE_CURVE")?, 5)?;
                let mut endpoints = [reference(&edge[1])?, reference(&edge[2])?];
                if !forward {
                    endpoints.reverse();
                }
                if last.is_some_and(|previous| previous != endpoints[0]) {
                    return Err(format!(
                        "source face #{face} has a disconnected or unordered boundary"
                    ));
                }
                first.get_or_insert(endpoints[0]);
                last = Some(endpoints[1]);
                self.edges
                    .entry(edge_id)
                    .or_default()
                    .push((face, forward == forward_bound));
            }
            if first != last {
                return Err(format!("source face #{face} has an open boundary"));
            }
        }
        Ok(())
    }

    pub(super) fn finish(
        self,
        records: &SourceRecords<'_>,
        retained: &mut HashSet<u64>,
    ) -> Result<bool, String> {
        let mut two_use = true;
        for (id, uses) in self.edges {
            if uses.len() != 2 {
                let record = named_record(records, id, "EDGE_CURVE")?;
                let curve = simple_record(records, reference(&parameters(record, 5)?[3])?)?;
                if uses.len() != 4
                    || curve.name != "LINE"
                    || uses
                        .iter()
                        .map(|(face, _)| face)
                        .collect::<HashSet<_>>()
                        .len()
                        != 4
                    || uses.iter().filter(|(_, forward)| *forward).count() != 2
                {
                    return Err(format!(
                        "incomplete or nonmanifold source shell edge uses at #{id}"
                    ));
                }
                two_use = false;
            }
            retained.insert(id);
        }
        Ok(two_use)
    }
}

pub(super) fn references(list: &Parameter) -> Result<Vec<u64>, String> {
    let Parameter::List(list) = list else {
        return Err("missing source reference aggregate".into());
    };
    let ids = list.iter().map(reference).collect::<Result<Vec<_>, _>>()?;
    if ids.is_empty() || ids.iter().copied().collect::<HashSet<_>>().len() != ids.len() {
        return Err("empty or repeated source topology references".into());
    }
    Ok(ids)
}

fn flag(param: &Parameter) -> Result<bool, String> {
    match param {
        Parameter::Enumeration(value) if value == "T" => Ok(true),
        Parameter::Enumeration(value) if value == "F" => Ok(false),
        _ => Err("invalid source orientation flag".into()),
    }
}
