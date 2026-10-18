//! Source identity coverage for presentations outside the geometry scene walk.
use super::*;

impl Scene<'_> {
    pub(in crate::step) fn require_complete_presentations(
        &self,
        retained: &HashSet<u64>,
    ) -> Result<(), String> {
        // A PCURVE's definitional representation supplies its parameter-space
        // support, not a separately presented occurrence of that support.
        let mut definitions = HashSet::new();
        for parts in self.records.values() {
            for record in parts.iter().filter(|record| record.name == "PCURVE") {
                if let Parameter::List(params) = &record.parameter {
                    definitions.insert(reference(params.last().ok_or("malformed PCURVE")?)?);
                }
            }
        }
        for (&id, parts) in &self.records {
            for record in parts {
                if styled(&record.name) {
                    self.require_retained_target(id, retained, &mut HashSet::new())?;
                    continue;
                }
                if self.items.contains_key(&id)
                    || (record.name == "DEFINITIONAL_REPRESENTATION" && definitions.contains(&id))
                {
                    continue;
                }
                // Inspect representation structure and its authenticated context,
                // including unknown subtypes and complex base records. Product
                // definition/reference records do not have this item-list form.
                let Parameter::List(params) = &record.parameter else {
                    continue;
                };
                let [
                    Parameter::String(_),
                    Parameter::List(items),
                    Parameter::Ref(super::super::Name::Entity(context)),
                ] = params.as_slice()
                else {
                    continue;
                };
                if !self.records.get(context).is_some_and(|parts| {
                    parts
                        .iter()
                        .any(|record| record.name == "REPRESENTATION_CONTEXT")
                }) {
                    return Err(format!(
                        "source completeness: representation-like #{id} lacks its context"
                    ));
                }
                for item in items {
                    self.require_retained_target(reference(item)?, retained, &mut HashSet::new())?;
                }
            }
        }
        Ok(())
    }

    fn require_retained_target(
        &self,
        id: u64,
        retained: &HashSet<u64>,
        path: &mut HashSet<u64>,
    ) -> Result<(), String> {
        if retained.contains(&id) {
            return Ok(());
        }
        if path.len() >= MAX_MAPPING_DEPTH || !path.insert(id) {
            return Err(format!(
                "source completeness: cyclic or excessively deep presentation at #{id}"
            ));
        }
        let record = self.simple(id)?;
        if styled(&record.name) {
            let Parameter::List(params) = &record.parameter else {
                return Err("source completeness: malformed styled item".into());
            };
            let target = params
                .get(2)
                .ok_or("source completeness: styled item lacks target")?;
            self.require_retained_target(reference(target)?, retained, path)?;
        } else if !matches!(
            record.name.as_str(),
            "DESCRIPTIVE_REPRESENTATION_ITEM" | "MEASURE_REPRESENTATION_ITEM"
        ) {
            return Err(format!(
                "source completeness: presented {} #{id} is not retained by scene discovery",
                record.name
            ));
        }
        path.remove(&id);
        Ok(())
    }
}

fn styled(name: &str) -> bool {
    matches!(
        name,
        "STYLED_ITEM" | "OVER_RIDING_STYLED_ITEM" | "CONTEXT_DEPENDENT_OVER_RIDING_STYLED_ITEM"
    )
}
