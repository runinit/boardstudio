//! Change reports for materialized mirror projections.
use crate::model::{
    Constraint, HardwareConfiguration, MechanicalMirrorPair, OutlineFeature, ProjectDoc,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
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

fn json(value: impl Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|error| error.to_string())
}

#[doc(hidden)]
pub fn changed_document_ids(
    before: &ProjectDoc,
    after: &ProjectDoc,
    candidates: &[String],
) -> Result<Vec<String>, String> {
    let candidates = candidates.iter().map(String::as_str).collect();
    let mut ids = BTreeSet::new();
    macro_rules! rows {
        ($before:ident, $after:ident; $($field:ident => $id:expr, $implicit:expr);+ $(;)?) => {
            $(diff_rows(&$before.$field, &$after.$field, $id, $implicit, &candidates, &mut ids)?;)+
        };
    }
    // An omitted array produces a field path when it appears or disappears. Rows
    // in that array are eligible for IDs only through explicit candidates.
    rows!(before, after;
        parts => |row| &row.id, true;
        matrices => |row| &row.id, true;
        boards => |row| &row.id, true;
        definitions => |row| &row.id, true;
        nets => |row| &row.id, true;
        constraints => Constraint::id, true;
        outline => OutlineFeature::id, true;
        layouts => |row| &row.id, !before.layouts.is_empty() && !after.layouts.is_empty();
        modules => |row| &row.id, !before.modules.is_empty() && !after.modules.is_empty();
        embedded_circuits => |row| &row.id,
            !before.embedded_circuits.is_empty() && !after.embedded_circuits.is_empty();
        case_bodies => |row| &row.id, false;
    );
    let empty = HardwareConfiguration::default();
    let before = before.hardware.as_ref().unwrap_or(&empty);
    let after = after.hardware.as_ref().unwrap_or(&empty);
    rows!(before, after;
        pairs => |row| &row.id, !before.pairs.is_empty() && !after.pairs.is_empty();
        instances => |row| &row.id, false;
        boards => |row| &row.board_id, false;
    );
    diff_pair_members(&before.pairs, &after.pairs, &candidates, &mut ids)?;
    Ok(ids.into_iter().collect())
}

fn diff_rows<T: PartialEq + Serialize>(
    before: &[T],
    after: &[T],
    id: impl Fn(&T) -> &str,
    implicit: bool,
    candidates: &BTreeSet<&str>,
    ids: &mut BTreeSet<String>,
) -> Result<(), String> {
    if (!implicit && candidates.is_empty()) || before == after {
        return Ok(());
    }
    let mut rows = BTreeMap::<_, [Option<(&T, &T)>; 2]>::new();
    for (side, collection) in [before, after].into_iter().enumerate() {
        for row in collection {
            let id = id(row);
            if implicit || candidates.contains(id) {
                // Explicit reports use the first row of each kind. The implicit
                // diff indexes entity arrays by ID, keeping the last occurrence.
                rows.entry(id).or_default()[side]
                    .get_or_insert((row, row))
                    .1 = row;
            }
        }
    }
    for (id, rows) in rows {
        if ids.contains(id) {
            continue;
        }
        let changed = match rows {
            [
                Some((first_before, last_before)),
                Some((first_after, last_after)),
            ] => {
                (candidates.contains(id) && row_changed(first_before, first_after, true)?)
                    || (implicit && row_changed(last_before, last_after, false)?)
            }
            _ => true,
        };
        if changed {
            ids.insert(id.to_owned());
        }
    }
    Ok(())
}

fn diff_pair_members(
    before: &[MechanicalMirrorPair],
    after: &[MechanicalMirrorPair],
    candidates: &BTreeSet<&str>,
    ids: &mut BTreeSet<String>,
) -> Result<(), String> {
    if candidates.is_empty() || before == after {
        return Ok(());
    }
    let mut by_member = BTreeMap::<_, [Vec<_>; 2]>::new();
    for (side, pairs) in [before, after].into_iter().enumerate() {
        for pair in pairs {
            let distinct_members = if pair.members[0] == pair.members[1] {
                1
            } else {
                2
            };
            for member in &pair.members[..distinct_members] {
                let member = member.as_str();
                if candidates.contains(member) && !ids.contains(member) {
                    by_member.entry(member).or_default()[side].push(pair);
                }
            }
        }
    }
    // Per-member pair contributions keep their document order, including repeats.
    for (member, [before, after]) in by_member {
        let mut changed = before.len() != after.len();
        for (before, after) in before.into_iter().zip(after) {
            if changed || row_changed(before, after, true)? {
                changed = true;
                break;
            }
        }
        if changed {
            ids.insert(member.to_owned());
        }
    }
    Ok(())
}

fn row_changed<T: PartialEq + Serialize>(
    before: &T,
    after: &T,
    explicit: bool,
) -> Result<bool, String> {
    if before == after {
        return Ok(false);
    }
    let (before, after) = (json(before)?, json(after)?);
    // Explicit reports compare the full wire records, including nested array order.
    Ok(if explicit {
        before != after
    } else {
        !same_record(&before, &after)
    })
}

// Within an entity, the walker attributes every field difference to its owner.
// Nested arrays match by last row per ID when every element on both sides has one.
fn same_record(before: &Value, after: &Value) -> bool {
    fn by_id(rows: &[Value]) -> Option<BTreeMap<&str, &Value>> {
        rows.iter()
            .map(|row| Some((row.get("id")?.as_str()?, row)))
            .collect()
    }

    if before == after {
        return true;
    }
    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            before.len() == after.len()
                && before
                    .iter()
                    .all(|(key, left)| after.get(key).is_some_and(|right| same_record(left, right)))
        }
        (Value::Array(before), Value::Array(after)) => {
            if let (Some(before), Some(after)) = (by_id(before), by_id(after)) {
                return before.len() == after.len()
                    && before.iter().all(|(id, left)| {
                        after.get(id).is_some_and(|right| same_record(left, right))
                    });
            }
            before.len() == after.len()
                && before
                    .iter()
                    .zip(after)
                    .all(|(left, right)| same_record(left, right))
        }
        _ => false,
    }
}

#[cfg(test)]
pub(super) fn legacy_changed_document_ids(
    before: &ProjectDoc,
    after: &ProjectDoc,
    candidates: &[String],
) -> Result<Vec<String>, String> {
    let (before, after) = (json(before)?, json(after)?);
    // The reference report works from the wire arrays, without using typed row selectors.
    let mut ids = BTreeSet::new();
    for (before, after, keys) in [
        (&before, &after, ENTITY_ARRAYS),
        (&before["hardware"], &after["hardware"], &["pairs"][..]),
    ] {
        fn by_id(rows: &[Value]) -> BTreeMap<&str, &Value> {
            rows.iter()
                .filter_map(|row| Some((row["id"].as_str()?, row)))
                .collect()
        }
        for key in keys {
            if let (Some(before), Some(after)) = (before[*key].as_array(), after[*key].as_array()) {
                let (before, after) = (by_id(before), by_id(after));
                for (id, old) in &before {
                    if after.get(id).is_none_or(|new| !same_record(old, new)) {
                        ids.insert((*id).to_owned());
                    }
                }
                ids.extend(
                    after
                        .keys()
                        .filter(|id| !before.contains_key(*id))
                        .map(|id| (*id).to_owned()),
                );
            }
        }
    }
    let mut ids: Vec<_> = ids.into_iter().collect();
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

// Explicit edit reports also name electrical rows, case bodies and physical instances.
// A physical instance can share an ID with its Board, so preserve the owner of each row.
#[cfg(test)]
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
    use crate::model::{ElectricalBoardConfiguration, Net};
    use serde_json::json;

    fn assert_report(
        before: &ProjectDoc,
        after: &ProjectDoc,
        candidates: &[&str],
        expected: &[&str],
    ) {
        let candidates: Vec<_> = candidates.iter().map(|id| (*id).to_owned()).collect();
        assert_eq!(
            legacy_changed_document_ids(before, after, &candidates).unwrap(),
            expected
        );
        assert_eq!(
            changed_document_ids(before, after, &candidates).unwrap(),
            expected
        );
    }

    #[test]
    fn pair_changes_report_explicit_member_ids() {
        let before = fixture_document().unwrap();
        let mut after = before.clone();
        let pair = &mut after.hardware.as_mut().unwrap().pairs[0];
        pair.mapping.at.x += 1.0;
        assert_report(
            &before,
            &after,
            &["unrelated", "left", "left"],
            &["left", "pair-source-mirror"],
        );
    }

    #[test]
    fn entity_arrays_compare_last_rows_and_candidates_compare_first_rows() {
        let net = |id: &str, name: &str| Net {
            id: id.into(),
            name: name.into(),
            pins: vec![],
        };
        let mut before = ProjectDoc::empty("test", "Test");
        before.nets = vec![
            net("repeat", "first"),
            net("gone", ""),
            net("repeat", "last"),
        ];
        let mut after = before.clone();
        after.nets.swap(0, 1);
        assert_report(&before, &after, &["repeat", "gone"], &[]);

        after.nets[1].name = "modified first".into();
        assert_report(&before, &after, &[], &[]);
        assert_report(&before, &after, &["repeat"], &["repeat"]);

        after.nets = vec![
            net("repeat", "last"),
            net("added", ""),
            net("repeat", "first"),
        ];
        assert_report(&before, &after, &[], &["added", "gone", "repeat"]);
    }

    #[test]
    fn nested_arrays_match_by_id_or_position_and_keep_the_outer_owner() {
        let mut before = fixture_document().unwrap();
        let id = before.parts[0].id.clone();
        let properties = |rows| Some(BTreeMap::from([("nested".into(), rows)]));
        before.parts[0].properties = properties(json!([
            {"id": "repeat", "value": 0},
            {"id": "other"},
            {"id": "repeat", "value": 1}
        ]));
        let mut after = before.clone();
        after.parts[0].properties = properties(json!([
            {"id": "repeat", "value": 1},
            {"id": "other"}
        ]));
        assert_report(&before, &after, &[], &[]);
        assert_report(&before, &after, &[&id], &[&id]);

        after.parts[0].properties = properties(json!([
            {"id": "other"},
            {"id": 42},
            {"id": "repeat", "value": 1}
        ]));
        assert_report(&before, &after, &["repeat", "other"], &[&id]);

        before.parts[0].properties = None;
        after.parts[0].properties = Some(BTreeMap::new());
        assert_report(&before, &after, &[], &[&id]);
    }

    #[test]
    fn omitted_arrays_need_candidates_when_appearing_or_disappearing() {
        let after = fixture_document().unwrap();
        let mut wire = json(&after).unwrap();
        wire["layouts"] = json!([{"id":"layout","name":"","boardId":"source-board",
            "matrixIds":[],"partIds":[]}]);
        wire["modules"] = json!([{"id":"module","definitionId":"","hostBoardId":"source-board",
            "hostFace":"front","facingFace":"back","at":{"x":0,"y":0},"rotation":0,"gap":0,
            "attachment":"board"}]);
        wire["embeddedCircuits"] = json!([{"id":"circuit","definitionId":"",
            "hostBoardId":"source-board","partIds":[],"netIds":[],"ports":{}}]);
        let after: ProjectDoc = serde_json::from_value(wire).unwrap();
        let mut before = after.clone();
        before.layouts.clear();
        before.modules.clear();
        before.embedded_circuits.clear();
        before.hardware = None;
        let candidates = ["module", "left", "pair-source-mirror", "layout", "circuit"];
        let expected = ["circuit", "layout", "left", "module", "pair-source-mirror"];
        assert_report(&before, &after, &[], &[]);
        assert_report(&after, &before, &[], &[]);
        assert_report(&before, &after, &candidates, &expected);
        assert_report(&after, &before, &candidates, &expected);

        let mut replacement = after.clone();
        replacement.layouts[0].id = "replacement-layout".into();
        assert_report(&after, &replacement, &[], &["layout", "replacement-layout"]);

        let mut before = after.clone();
        before.hardware.as_mut().unwrap().pairs.clear();
        assert_report(&before, &after, &[], &[]);
        assert_report(
            &before,
            &after,
            &["pair-source-mirror"],
            &["pair-source-mirror"],
        );
    }

    #[test]
    fn candidate_collisions_keep_each_row_kind_and_ordered_pair_contributions() {
        let mut before = fixture_document().unwrap();
        let hardware = before.hardware.as_mut().unwrap();
        let mut pair = hardware.pairs[0].clone();
        pair.id = "another-pair".into();
        pair.members = ["left".into(), "left".into()];
        hardware.pairs.push(pair);
        hardware.boards.push(ElectricalBoardConfiguration {
            board_id: "left".into(),
            ..Default::default()
        });
        before.case_bodies.push(
            serde_json::from_value(json!({
                "id":"left","name":"tray","boardId":"source-board","kind":"tray",
                "thickness":1,"clearance":0
            }))
            .unwrap(),
        );
        let mut after = before.clone();
        let hardware = after.hardware.as_mut().unwrap();
        hardware.pairs.reverse();
        assert_report(&before, &after, &["right", "left"], &["left"]);
        assert_report(&before, &after, &[], &[]);

        let mut after = before.clone();
        after.hardware.as_mut().unwrap().boards.clear();
        assert_report(&before, &after, &["left"], &["left"]);
        let mut after = before.clone();
        after.case_bodies[0].thickness += 1.0;
        assert_report(&before, &after, &["left"], &["left"]);
        let mut after = before.clone();
        after.hardware.as_mut().unwrap().instances[0].name = "renamed".into();
        assert_report(&before, &after, &["left"], &["left"]);
    }

    #[test]
    fn unequal_typed_rows_can_serialize_to_equal_records() {
        let mut before = fixture_document().unwrap();
        let matrix = &mut before.matrices[0];
        matrix.anchor = serde_json::from_value(json!({"follow":[],"aggregate":{}})).unwrap();
        matrix.rotation = Some(f64::NAN);
        let id = matrix.id.clone();

        let mut after = before.clone();
        let matrix = &mut after.matrices[0];
        // An aggregate anchor omits the single-key coordinates. Both non-finite
        // values serialize as null in the present optional rotation field.
        matrix.anchor.as_mut().unwrap().column = 7;
        matrix.rotation = Some(f64::INFINITY);
        assert_report(&before, &after, &[&id], &[]);

        after.matrices[0].rotation = None;
        assert_report(&before, &after, &[], &[&id]);
    }
}
