//! Mirrored-half records of explicit mechanical mirror pairs.
//!
//! A pair with `halves` records which board is the source, which board is its reflection,
//! the counterpart of each item (correspondences) and any per-item exception. Absent `halves`
//! means the pair is unpaired: it links mechanical features and nothing else.

use crate::model::{MechanicalMirrorPair, MirroredHalves, MirroredItemKind, ProjectDoc};

/// Checks the construction source and the mirrored-half record of every pair in the document.
///
/// Returns the first violation, located by pair id and by the record's index. It only reads the
/// document, so a refused record changes nothing. Pairs without `halves` are checked for their
/// construction source only. A board may be a half of one record only.
pub fn validate_document(document: &ProjectDoc) -> Result<(), String> {
    let Some(hardware) = document.hardware.as_ref() else {
        return Ok(());
    };
    // (board id, pair id) for every board a record already claims.
    let mut claimed: Vec<(&str, &str)> = Vec::new();
    for pair in &hardware.pairs {
        if let Some(source) = &pair.construction_source
            && !pair.members.contains(source)
        {
            return Err(format!(
                "mechanical pair {:?} names construction source {source:?}, which is not a pair member",
                pair.id
            ));
        }
        let Some(halves) = &pair.halves else {
            continue;
        };
        validate_halves(document, pair, halves)?;
        let mut boards = vec![halves.source_board_id.as_str()];
        if halves.mirror_board_id != halves.source_board_id {
            boards.push(halves.mirror_board_id.as_str());
        }
        for board_id in boards {
            if let Some((_, other)) = claimed.iter().find(|(board, _)| *board == board_id) {
                return Err(format!(
                    "mechanical pair {:?} halves name board {board_id:?}, which is already a half of pair {other:?}",
                    pair.id
                ));
            }
            claimed.push((board_id, &pair.id));
        }
    }
    Ok(())
}

fn validate_halves(
    document: &ProjectDoc,
    pair: &MechanicalMirrorPair,
    halves: &MirroredHalves,
) -> Result<(), String> {
    for (field, board_id) in [
        ("sourceBoardId", &halves.source_board_id),
        ("mirrorBoardId", &halves.mirror_board_id),
    ] {
        if !document.boards.iter().any(|board| board.id == *board_id) {
            return Err(format!(
                "mechanical pair {:?} halves.{field} {board_id:?} is not a board in the document",
                pair.id
            ));
        }
    }
    for (index, correspondence) in halves.correspondences.iter().enumerate() {
        let location = format!(
            "mechanical pair {:?} halves.correspondences[{index}]",
            pair.id
        );
        let kind = kind_name(correspondence.kind);
        if !item_exists(
            document,
            pair,
            &halves.source_board_id,
            correspondence.kind,
            &correspondence.source_id,
        ) {
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                correspondence.source_id, halves.source_board_id
            ));
        }
        if !item_exists(
            document,
            pair,
            &halves.mirror_board_id,
            correspondence.kind,
            &correspondence.mirror_id,
        ) {
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                correspondence.mirror_id, halves.mirror_board_id
            ));
        }
        let earlier = &halves.correspondences[..index];
        if earlier.iter().any(|other| {
            other.kind == correspondence.kind && other.source_id == correspondence.source_id
        }) {
            return Err(format!(
                "{location} repeats source {kind} {:?}",
                correspondence.source_id
            ));
        }
        if earlier.iter().any(|other| {
            other.kind == correspondence.kind && other.mirror_id == correspondence.mirror_id
        }) {
            return Err(format!(
                "{location} repeats mirror {kind} {:?}",
                correspondence.mirror_id
            ));
        }
    }
    for (index, exception) in halves.exceptions.iter().enumerate() {
        let location = format!("mechanical pair {:?} halves.exceptions[{index}]", pair.id);
        let kind = kind_name(exception.kind);
        if !item_exists(
            document,
            pair,
            &halves.mirror_board_id,
            exception.kind,
            &exception.mirror_id,
        ) {
            return Err(format!(
                "{location} names {kind} {:?}, which is not on board {:?}",
                exception.mirror_id, halves.mirror_board_id
            ));
        }
        if halves.correspondences.iter().any(|correspondence| {
            correspondence.kind == exception.kind && correspondence.mirror_id == exception.mirror_id
        }) {
            return Err(format!(
                "{location} names {kind} {:?}, which a correspondence links; an exception needs an unlinked mirror item",
                exception.mirror_id
            ));
        }
        let earlier = &halves.exceptions[..index];
        if earlier
            .iter()
            .any(|other| other.kind == exception.kind && other.mirror_id == exception.mirror_id)
        {
            return Err(format!(
                "{location} repeats {kind} {:?}",
                exception.mirror_id
            ));
        }
    }
    Ok(())
}

/// Whether an item of `kind` with `id` exists on the board `board_id`. Matrices, layouts and
/// assemblies belong to the document rather than to a board, so the board is not consulted for
/// them. Mechanical features belong to the pair.
fn item_exists(
    document: &ProjectDoc,
    pair: &MechanicalMirrorPair,
    board_id: &str,
    kind: MirroredItemKind,
    id: &str,
) -> bool {
    let board = document.boards.iter().find(|board| board.id == board_id);
    match kind {
        MirroredItemKind::Part => {
            board.is_some_and(|board| board.part_ids.iter().any(|part| part == id))
                && document.parts.iter().any(|part| part.id == id)
        }
        MirroredItemKind::Outline => {
            board.is_some_and(|board| board.outline_ids.iter().any(|outline| outline == id))
        }
        MirroredItemKind::Net => {
            board.is_some_and(|board| board.net_ids.iter().any(|net| net == id))
        }
        MirroredItemKind::Matrix => document.matrices.iter().any(|matrix| matrix.id == id),
        MirroredItemKind::Layout => document.layouts.iter().any(|layout| layout.id == id),
        MirroredItemKind::Assembly => document.assemblies.iter().any(|assembly| assembly.id == id),
        MirroredItemKind::MechanicalFeature => pair.features.iter().any(|feature| feature.id == id),
    }
}

/// The name a kind is written under in the document, for messages.
fn kind_name(kind: MirroredItemKind) -> &'static str {
    match kind {
        MirroredItemKind::Matrix => "matrix",
        MirroredItemKind::Layout => "layout",
        MirroredItemKind::Part => "part",
        MirroredItemKind::Outline => "outline",
        MirroredItemKind::Net => "net",
        MirroredItemKind::Assembly => "assembly",
        MirroredItemKind::MechanicalFeature => "mechanical-feature",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::validate_document;
    use crate::model::{Board, HardwareConfiguration, MechanicalMirrorPair, Part, ProjectDoc};

    fn construction() -> Value {
        json!({
            "method": "printed", "mount": "tray", "integratedPlateFrame": false,
            "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
            "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3
        })
    }

    fn pair(extra: Value) -> MechanicalMirrorPair {
        let mut value = json!({
            "id": "pair", "members": ["left", "right"],
            "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
            "construction": construction(), "features": []
        });
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        serde_json::from_value(value).unwrap()
    }

    fn halves(correspondences: Value, exceptions: Value) -> Value {
        json!({
            "halves": {
                "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                "correspondences": correspondences, "exceptions": exceptions
            },
            "constructionSource": "left"
        })
    }

    fn board(id: &str, parts: &[&str], outlines: &[&str], nets: &[&str]) -> Board {
        serde_json::from_value(json!({
            "id": id, "name": id, "outlineIds": outlines, "partIds": parts,
            "netIds": nets, "thickness": 1.6
        }))
        .unwrap()
    }

    fn part(id: &str) -> Part {
        serde_json::from_value(json!({
            "id": id, "definitionId": "switch", "reference": id,
            "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 }, "side": "front"
        }))
        .unwrap()
    }

    fn document(pairs: Vec<MechanicalMirrorPair>) -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Doc");
        document.boards = vec![
            board(
                "left-board",
                &["l-key", "l-extra"],
                &["l-outline"],
                &["l-net"],
            ),
            board(
                "right-board",
                &["r-key", "r-extra"],
                &["r-outline"],
                &["r-net"],
            ),
        ];
        document.parts = ["l-key", "l-extra", "r-key", "r-extra"]
            .iter()
            .map(|id| part(id))
            .collect();
        document.hardware = Some(HardwareConfiguration {
            pairs,
            ..HardwareConfiguration::default()
        });
        document
    }

    fn valid_correspondence() -> Value {
        json!([{ "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" }])
    }

    fn refusal(pair_extra: Value) -> String {
        validate_document(&document(vec![pair(pair_extra)])).unwrap_err()
    }

    #[test]
    fn accepts_a_pair_without_halves_and_a_consistent_record() {
        validate_document(&document(vec![pair(json!({}))])).unwrap();
        validate_document(&document(vec![pair(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }]),
        ))]))
        .unwrap();
        validate_document(&ProjectDoc::empty("doc", "Doc")).unwrap();
    }

    #[test]
    fn refuses_a_construction_source_that_is_not_a_member() {
        let message = refusal(json!({ "constructionSource": "elsewhere" }));
        assert!(message.contains("construction source"), "{message}");
        assert!(message.contains("\"pair\""), "{message}");
        assert!(message.contains("\"elsewhere\""), "{message}");
    }

    #[test]
    fn refuses_a_source_board_that_is_not_in_the_document() {
        let mut value = halves(valid_correspondence(), json!([]));
        value["halves"]["sourceBoardId"] = json!("missing-board");
        let message = refusal(value);
        assert!(message.contains("halves.sourceBoardId"), "{message}");
        assert!(message.contains("\"missing-board\""), "{message}");
    }

    #[test]
    fn refuses_a_correspondence_whose_source_is_on_the_other_board() {
        let message = refusal(halves(
            json!([{ "kind": "part", "sourceId": "r-key", "mirrorId": "r-extra" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("\"r-key\""), "{message}");
        assert!(message.contains("\"left-board\""), "{message}");
    }

    #[test]
    fn refuses_a_correspondence_whose_mirror_is_not_on_the_mirror_board() {
        let message = refusal(halves(
            json!([{ "kind": "part", "sourceId": "l-key", "mirrorId": "ghost" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("\"ghost\""), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    #[test]
    fn refuses_a_repeated_correspondence_source_or_mirror() {
        let repeated_source = refusal(halves(
            json!([
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-extra" }
            ]),
            json!([]),
        ));
        assert!(
            repeated_source.contains("correspondences[1]"),
            "{repeated_source}"
        );
        assert!(repeated_source.contains("repeats"), "{repeated_source}");

        let repeated_mirror = refusal(halves(
            json!([
                { "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" },
                { "kind": "part", "sourceId": "l-extra", "mirrorId": "r-key" }
            ]),
            json!([]),
        ));
        assert!(
            repeated_mirror.contains("correspondences[1]"),
            "{repeated_mirror}"
        );
        assert!(repeated_mirror.contains("repeats"), "{repeated_mirror}");
    }

    #[test]
    fn refuses_an_exception_that_names_a_source_only_item() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "l-extra", "state": "local" }]),
        ));
        assert!(message.contains("exceptions[0]"), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    #[test]
    fn refuses_an_exception_on_a_mirror_item_that_a_correspondence_links() {
        let message = refusal(halves(
            valid_correspondence(),
            json!([{ "kind": "part", "mirrorId": "r-key", "state": "omitted" }]),
        ));
        assert!(message.contains("exceptions[0]"), "{message}");
        assert!(message.contains("linked"), "{message}");
    }

    #[test]
    fn refuses_a_repeated_exception() {
        let message = refusal(halves(
            json!([]),
            json!([
                { "kind": "part", "mirrorId": "r-extra", "state": "local" },
                { "kind": "part", "mirrorId": "r-extra", "state": "omitted" }
            ]),
        ));
        assert!(message.contains("exceptions[1]"), "{message}");
        assert!(message.contains("repeats"), "{message}");
    }

    #[test]
    fn refuses_a_board_that_is_a_half_of_two_pairs() {
        let first = pair(halves(valid_correspondence(), json!([])));
        let second = pair(json!({
            "id": "second", "members": ["other-left", "other-right"],
            "halves": { "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                "correspondences": [], "exceptions": [] }
        }));
        let message = validate_document(&document(vec![first, second])).unwrap_err();
        assert!(message.contains("\"left-board\""), "{message}");
        assert!(
            message.contains("\"pair\"") && message.contains("\"second\""),
            "{message}"
        );
    }

    #[test]
    fn outline_and_net_correspondences_must_be_on_their_board() {
        validate_document(&document(vec![pair(halves(
            json!([
                { "kind": "outline", "sourceId": "l-outline", "mirrorId": "r-outline" },
                { "kind": "net", "sourceId": "l-net", "mirrorId": "r-net" }
            ]),
            json!([]),
        ))]))
        .unwrap();
        let message = refusal(halves(
            json!([{ "kind": "net", "sourceId": "l-net", "mirrorId": "l-net" }]),
            json!([]),
        ));
        assert!(message.contains("correspondences[0]"), "{message}");
    }
}
