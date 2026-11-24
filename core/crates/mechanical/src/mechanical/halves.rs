//! Validation of physical half ownership and derived mirror records.

use crate::model::{
    HardwareConfiguration, MechanicalMirrorPair, MirroredCompanionMode, MirroredCompanionTweak,
    ProjectDoc,
};
use boardstudio_model::mirrored_half::MirrorDestination;
use std::collections::{BTreeMap, BTreeSet};

/// Shared validation for the creation request and each item's effective placement settings.
pub fn validate_companion(
    mode: MirroredCompanionMode,
    tweak: Option<MirroredCompanionTweak>,
) -> Result<(), String> {
    if let Some(tweak) = tweak {
        if mode != MirroredCompanionMode::LiteralMirrorWithTweak {
            return Err(
                "companionTweak is only valid with companionMode literal-mirror-with-tweak".into(),
            );
        }
        if !(tweak.x.is_finite() && tweak.y.is_finite() && tweak.rotation.is_finite()) {
            return Err("companionTweak x, y and rotation must be finite".into());
        }
    }
    Ok(())
}

/// Checks mechanical half ownership and its agreement with any derived mirror pair.
pub fn validate_document(document: &ProjectDoc) -> Result<(), String> {
    let mut claimed = BTreeMap::new();
    if let Some(hardware) = &document.hardware {
        for pair in &hardware.pairs {
            if let Some(source) = &pair.construction_source
                && !pair.members.contains(source)
            {
                return Err(format!(
                    "mechanical pair {:?} names construction source {source:?}, which is not a pair member",
                    pair.id
                ));
            }
            let mirror = document
                .mirrored_pairs
                .iter()
                .find(|mirror| mirror.id == pair.id);
            if pair.halves.is_none() && mirror.is_none() {
                continue;
            }
            let boards = member_boards(document, hardware, pair)?;
            if pair.halves.is_some() {
                // A reversible PCB appears on both sides of a single record.
                for board in
                    std::iter::once(boards[0]).chain((boards[0] != boards[1]).then_some(boards[1]))
                {
                    if let Some(other) = claimed.insert(board, &pair.id) {
                        return Err(format!(
                            "mechanical pair {:?} halves name board {board:?}, which is already a half of pair {other:?}",
                            pair.id
                        ));
                    }
                }
            }
            if let Some(mirror) = mirror {
                let board_of = |end: &MirrorDestination| match end {
                    MirrorDestination::SecondBoard(id) => document
                        .boards
                        .iter()
                        .find(|board| board.id == *id)
                        .map(|board| board.id.as_str()),
                    MirrorDestination::SameBoardLayout(id) => document
                        .layouts
                        .iter()
                        .find(|layout| layout.id == *id)
                        .map(|layout| layout.board_id.as_str()),
                };
                if [board_of(&mirror.source), board_of(&mirror.destination)] != boards.map(Some) {
                    return Err(format!(
                        "Mirror pair {} source and destination must match the mechanical pair's boards",
                        mirror.id
                    ));
                }
            }
        }
    }
    for mirror in &document.mirrored_pairs {
        let mut sources = BTreeSet::new();
        if mirror
            .overrides
            .iter()
            .any(|item| !sources.insert(&item.source_id))
        {
            return Err(format!(
                "Mirror pair {} has a duplicate override",
                mirror.id
            ));
        }
    }
    Ok(())
}

fn member_boards<'a>(
    document: &ProjectDoc,
    hardware: &'a HardwareConfiguration,
    pair: &MechanicalMirrorPair,
) -> Result<[&'a str; 2], String> {
    let Some(source) = pair.construction_source.as_deref() else {
        let record = if pair.halves.is_some() {
            "halves"
        } else {
            "a derived mirror"
        };
        return Err(format!(
            "mechanical pair {:?} records {record} without a constructionSource",
            pair.id
        ));
    };
    let [first, second] = pair.members.each_ref().map(String::as_str);
    let mirror = if source == first { second } else { first };
    let board = |member: &str| {
        hardware.instances.iter().find(|entry| entry.id == member)
            .map(|entry| entry.board_id.as_str())
            .ok_or_else(|| format!(
                "mechanical pair {:?} names physical instance {member:?}, which is not in the document",
                pair.id
            ))
    };
    let boards = [board(source)?, board(mirror)?];
    if let Some(halves) = &pair.halves {
        for (field, board_id, member, actual) in [
            ("sourceBoardId", &halves.source_board_id, source, boards[0]),
            ("mirrorBoardId", &halves.mirror_board_id, mirror, boards[1]),
        ] {
            if !document.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!(
                    "mechanical pair {:?} halves.{field} {board_id:?} is not a board in the document",
                    pair.id
                ));
            }
            if actual != board_id {
                return Err(format!(
                    "mechanical pair {:?} halves.{field} {board_id:?} is not the board of {member:?}, which is on {actual:?}",
                    pair.id
                ));
            }
        }
        validate_companion(halves.companion_mode, None)?;
    }
    Ok(boards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::mirrored_half::{MirrorOverride, MirroredPair, PartOverride};
    use serde_json::json;

    fn document() -> ProjectDoc {
        let mut doc = ProjectDoc::empty("doc", "Doc");
        doc.boards = ["left", "right", "other"]
            .map(|id| crate::test_support::board(id, vec![], 1.6))
            .into();
        let instances = ["left", "right"].map(|id| json!({
            "id": id, "name": id, "boardId": id, "half": id, "role": id, "flipped": id == "right"
        }));
        doc.hardware = Some(serde_json::from_value(json!({
            "instances": instances,
            "pairs": [{
                "id": "pair", "members": ["left", "right"], "constructionSource": "left",
                "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": {
                    "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
                    "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3
                },
                "halves": { "sourceBoardId": "left", "mirrorBoardId": "right" }
            }]
        })).unwrap());
        doc
    }

    fn pair(doc: &mut ProjectDoc) -> &mut MechanicalMirrorPair {
        &mut doc.hardware.as_mut().unwrap().pairs[0]
    }

    fn expect_error(doc: &ProjectDoc, expected: &str) {
        let error = validate_document(doc).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }

    #[test]
    fn physical_halves_must_name_their_members_boards() {
        let mut doc = document();
        validate_document(&doc).unwrap();
        pair(&mut doc).construction_source = Some("right".into());
        expect_error(&doc, "halves.sourceBoardId");
        pair(&mut doc).construction_source = Some("unknown".into());
        expect_error(&doc, "not a pair member");
        pair(&mut doc).construction_source = None;
        expect_error(&doc, "without a constructionSource");
        pair(&mut doc).halves = None;
        validate_document(&doc).unwrap();

        let mut doc = document();
        doc.boards.retain(|board| board.id != "right");
        expect_error(&doc, "is not a board in the document");
        doc.hardware.as_mut().unwrap().instances.pop();
        expect_error(&doc, "physical instance");
    }

    #[test]
    fn a_shared_reversible_board_is_claimed_once_per_pair() {
        let mut doc = document();
        let pair = pair(&mut doc);
        pair.halves.as_mut().unwrap().mirror_board_id = "left".into();
        let mut second = pair.clone();
        second.id = "second".into();
        second.members = ["other-left".into(), "other-right".into()];
        second.construction_source = Some("other-left".into());
        let hardware = doc.hardware.as_mut().unwrap();
        hardware.instances[1].board_id = "left".into();
        assert!(hardware.instances[1].flipped);
        validate_document(&doc).unwrap();

        let hardware = doc.hardware.as_mut().unwrap();
        for (index, id) in second.members.iter().enumerate() {
            let mut instance = hardware.instances[index].clone();
            instance.id.clone_from(id);
            hardware.instances.push(instance);
        }
        hardware.pairs.push(second);
        expect_error(&doc, "already a half of pair");
    }

    #[test]
    fn derived_pair_endpoints_follow_the_construction_source_and_overrides_are_unique() {
        let mut doc = document();
        let pair = pair(&mut doc);
        pair.halves = None; // The two-board creation path stores a derived pair instead.
        let mirror = MirroredPair {
            id: pair.id.clone(),
            source: MirrorDestination::SecondBoard("left".into()),
            destination: MirrorDestination::SecondBoard("right".into()),
            mapping: pair.mapping,
            overrides: vec![PartOverride {
                source_id: "key".into(),
                state: MirrorOverride::Omitted,
            }],
        };
        doc.mirrored_pairs.push(mirror);
        validate_document(&doc).unwrap();
        doc.mirrored_pairs[0].destination = MirrorDestination::SecondBoard("other".into());
        expect_error(&doc, "must match the mechanical pair's boards");

        doc.hardware = None;
        let overrides = &mut doc.mirrored_pairs[0].overrides;
        overrides.push(overrides[0].clone());
        expect_error(&doc, "duplicate override");
    }
}
