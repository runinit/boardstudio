//! Change reports for materialized mirror projections.
use crate::model::{Constraint, OutlineFeature, ProjectDoc};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const ENTITY_ARRAYS: &[&str] = &[
    "parts",
    "matrices",
    "boards",
    "definitions",
    "nets",
    "constraints",
    "outline",
    "layouts",
    "modules",
    "embeddedCircuits",
];

#[doc(hidden)]
pub fn changed_document_fields(
    before: &ProjectDoc,
    after: &ProjectDoc,
) -> Result<(Vec<String>, Vec<String>), String> {
    Ok(diff(&json(before)?, &json(after)?))
}

fn diff(before: &Value, after: &Value) -> (Vec<String>, Vec<String>) {
    let mut ids = BTreeSet::new();
    let mut fields = BTreeSet::new();
    walk(before, after, "$", None, false, &mut ids, &mut fields);
    (ids.into_iter().collect(), fields.into_iter().collect())
}

fn walk(
    before: &serde_json::Value,
    after: &serde_json::Value,
    path: &str,
    owner_id: Option<&str>,
    entity_array: bool,
    ids: &mut BTreeSet<String>,
    fields: &mut BTreeSet<String>,
) {
    if before == after {
        return;
    }
    match (before, after) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            let keys: BTreeSet<_> = left.keys().chain(right.keys()).collect();
            for key in keys {
                let child_path = format!("{path}.{key}");
                let child_entity_array = (path == "$" && ENTITY_ARRAYS.contains(&key.as_str()))
                    || child_path == "$.hardware.pairs";
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => walk(
                        left,
                        right,
                        &child_path,
                        owner_id,
                        child_entity_array,
                        ids,
                        fields,
                    ),
                    _ if owner_id.is_some() => {
                        ids.insert(owner_id.unwrap().to_owned());
                    }
                    _ => {
                        fields.insert(child_path);
                    }
                }
            }
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            let left_by_id: Option<BTreeMap<_, _>> = left
                .iter()
                .map(|value| {
                    value
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(|id| (id, value))
                })
                .collect();
            let right_by_id: Option<BTreeMap<_, _>> = right
                .iter()
                .map(|value| {
                    value
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .map(|id| (id, value))
                })
                .collect();
            if let (Some(left_by_id), Some(right_by_id)) = (left_by_id, right_by_id) {
                let keys: BTreeSet<_> = left_by_id
                    .keys()
                    .chain(right_by_id.keys())
                    .copied()
                    .collect();
                for id in keys {
                    match (left_by_id.get(id), right_by_id.get(id)) {
                        (Some(left), Some(right)) => {
                            let item_owner = if entity_array { Some(id) } else { owner_id };
                            walk(
                                left,
                                right,
                                &format!("{path}[id={id:?}]"),
                                item_owner,
                                false,
                                ids,
                                fields,
                            )
                        }
                        _ => {
                            if entity_array {
                                ids.insert(id.to_owned());
                            } else if let Some(owner_id) = owner_id {
                                ids.insert(owner_id.to_owned());
                            } else {
                                fields.insert(format!("{path}[id={id:?}]"));
                            }
                        }
                    }
                }
                return;
            }
            let count = left.len().max(right.len());
            for index in 0..count {
                let child_path = format!("{path}[{index}]");
                match (left.get(index), right.get(index)) {
                    (Some(left), Some(right)) => {
                        walk(left, right, &child_path, owner_id, false, ids, fields)
                    }
                    _ if owner_id.is_some() => {
                        ids.insert(owner_id.unwrap().to_owned());
                    }
                    _ => {
                        fields.insert(child_path);
                    }
                }
            }
        }
        _ if owner_id.is_some() => {
            ids.insert(owner_id.unwrap().to_owned());
        }
        _ => {
            fields.insert(path.to_owned());
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum ChangeScope {
    Full,
    Placement,
}

impl ChangeScope {
    pub(super) fn ids(
        self,
        before: &ProjectDoc,
        after: &ProjectDoc,
        candidates: &[String],
    ) -> Result<Vec<String>, String> {
        let (before, after) = match self {
            Self::Placement if let Some(values) = placement_values(before, after)? => values,
            _ => (json(before)?, json(after)?),
        };
        let (mut ids, _) = diff(&before, &after);
        let candidates: BTreeSet<_> = candidates
            .iter()
            .filter(|id| ids.binary_search(id).is_err())
            .map(String::as_str)
            .collect();
        if !candidates.is_empty() {
            let before = reported_entities(&before, &candidates);
            let after = reported_entities(&after, &candidates);
            ids.extend(
                candidates
                    .into_iter()
                    .filter(|id| before.get(id) != after.get(id))
                    .map(str::to_owned),
            );
            ids.sort_unstable();
            ids.dedup();
        }
        Ok(ids)
    }
}

fn json(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

// Under stable identities the wire diff compares each record independently. Feed the
// same walker only records whose values can contribute IDs, retaining its treatment of
// nested id-keyed arrays and the serialized equality used by explicit edit reports.
fn placement_values(
    before: &ProjectDoc,
    after: &ProjectDoc,
) -> Result<Option<(Value, Value)>, String> {
    type Rows = [Map<String, Value>; 2];

    fn add<T: PartialEq + Serialize>(
        values: &mut Rows,
        key: &str,
        before: &[T],
        after: &[T],
        id: impl Fn(&T) -> &str,
    ) -> Result<bool, String> {
        if before == after {
            return Ok(true);
        }
        if before.len() != after.len() {
            return Ok(false);
        }
        let mut seen = BTreeSet::new();
        let mut left = Vec::new();
        let mut right = Vec::new();
        for (before, after) in before.iter().zip(after) {
            let identity = id(before);
            if identity != id(after) || !seen.insert(identity) {
                return Ok(false);
            }
            if before != after {
                left.push(json(before)?);
                right.push(json(after)?);
            }
        }
        values[0].insert(key.to_owned(), Value::Array(left));
        values[1].insert(key.to_owned(), Value::Array(right));
        Ok(true)
    }

    let mut root = Rows::default();
    let mut hardware = Rows::default();
    macro_rules! rows {
        ($scope:ident, $before:ident, $after:ident; $($key:literal => $field:ident, $id:expr);+ $(;)?) => {
            $(if !add(&mut $scope, $key, &$before.$field, &$after.$field, $id)? {
                return Ok(None);
            })+
        };
    }
    rows!(root, before, after;
        "parts" => parts, |row| &row.id;
        "matrices" => matrices, |row| &row.id;
        "boards" => boards, |row| &row.id;
        "definitions" => definitions, |row| &row.id;
        "nets" => nets, |row| &row.id;
        "constraints" => constraints, Constraint::id;
        "outline" => outline, OutlineFeature::id;
        "layouts" => layouts, |row| &row.id;
        "modules" => modules, |row| &row.id;
        "embeddedCircuits" => embedded_circuits, |row| &row.id;
        "caseBodies" => case_bodies, |row| &row.id;
    );
    match (&before.hardware, &after.hardware) {
        (Some(before), Some(after)) => {
            rows!(hardware, before, after;
                "pairs" => pairs, |row| &row.id;
                "instances" => instances, |row| &row.id;
                "boards" => boards, |row| &row.board_id;
            );
        }
        (None, None) => {}
        _ => return Ok(None),
    }
    for (root, hardware) in root.iter_mut().zip(hardware) {
        root.insert("hardware".to_owned(), Value::Object(hardware));
    }
    let [before, after] = root.map(Value::Object);
    Ok(Some((before, after)))
}

// Explicit edit reports also name electrical rows, case bodies and physical instances.
// A physical instance can share an ID with its Board, so preserve the owner of each row.
fn reported_entities<'a>(
    document: &'a Value,
    candidates: &BTreeSet<&str>,
) -> BTreeMap<&'a str, Vec<(&'static str, &'a Value)>> {
    let mut entities = BTreeMap::<_, Vec<_>>::new();
    let top_level = ENTITY_ARRAYS.iter().map(|&key| (document, key, "id", key));
    for (root, key, id_key, owner) in top_level.chain([
        (document, "caseBodies", "id", "caseBodies"),
        (&document["hardware"], "pairs", "id", "hardware.pairs"),
        (
            &document["hardware"],
            "instances",
            "id",
            "hardware.instances",
        ),
        (
            &document["hardware"],
            "boards",
            "boardId",
            "hardware.boards",
        ),
    ]) {
        let mut seen = BTreeSet::new();
        for row in root[key].as_array().into_iter().flatten() {
            if let Some(id) = row[id_key].as_str()
                && candidates.contains(id)
                && seen.insert(id)
            {
                entities.entry(id).or_default().push((owner, row));
            }
        }
    }
    for pair in document["hardware"]["pairs"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let mut seen = BTreeSet::new();
        for member in pair["members"].as_array().into_iter().flatten() {
            if let Some(id) = member.as_str()
                && candidates.contains(id)
                && seen.insert(id)
            {
                // Shared construction changes a member's effective configuration.
                entities
                    .entry(id)
                    .or_default()
                    .push(("hardware.memberPairs", pair));
            }
        }
    }
    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mirrored_half::reconcile::tests::fixtures::fixture_document;

    #[test]
    fn placement_reports_keep_explicit_physical_member_ids() {
        let before = fixture_document().unwrap();
        let mut after = before.clone();
        let pair = &mut after.hardware.as_mut().unwrap().pairs[0];
        pair.mapping.at.x += 1.0;
        let pair_id = pair.id.clone();
        let candidates = vec![pair.members[0].clone(), "unrelated".into()];
        let Some((_, snapshot)) = placement_values(&before, &after).unwrap() else {
            panic!("stable rows must use the scoped report");
        };
        assert!(snapshot["parts"].is_null());
        assert_eq!(snapshot["hardware"]["pairs"].as_array().unwrap().len(), 1);
        let expected = vec![candidates[0].clone(), pair_id];
        for scope in [ChangeScope::Placement, ChangeScope::Full] {
            assert_eq!(scope.ids(&before, &after, &candidates).unwrap(), expected);
        }
        after.parts.pop().unwrap();
        assert!(placement_values(&before, &after).unwrap().is_none());
    }
}
