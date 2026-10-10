//! Mirrored-half records of explicit mechanical mirror pairs.
//!
//! A pair with `halves` records which board is the source, which board is its reflection,
//! the counterpart of each item (correspondences) and any per-item exception. Absent `halves`
//! means the pair is unpaired: it links mechanical features and nothing else.

use crate::model::{
    MechanicalMirrorPair, MirroredCompanionMode, MirroredCompanionTweak, MirroredHalves,
    MirroredItemKind, ProjectDoc,
};

/// Shared validation for the creation request and each item's effective placement settings.
pub(crate) fn validate_companion(
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

/// Checks the construction source and the mirrored-half record of every pair in the document.
///
/// Returns the first violation, located by pair id and by the record's index. It only reads the
/// document, so a refused record changes nothing. Pairs without `halves` are checked for their
/// construction source only. A record needs a construction source, and its boards are the boards
/// of the two members. A board may be a half of one record only; a record whose source and mirror
/// are the same board (a reused board) is one record. Ticket 18 decides whether one board can be
/// a half of several records.
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
    // The record is relative to the construction source, so it needs one, and the source half
    // must sit on the source board. A member that is not an instance is reported by the pair
    // checks, so it is not repeated here.
    let Some(source) = pair.construction_source.as_deref() else {
        return Err(format!(
            "mechanical pair {:?} records halves without a constructionSource",
            pair.id
        ));
    };
    let instance_board = |member: &str| {
        document
            .hardware
            .as_ref()
            .and_then(|hardware| hardware.instances.iter().find(|entry| entry.id == member))
            .map(|entry| entry.board_id.as_str())
    };
    let mirror = pair.members.iter().find(|member| member.as_str() != source);
    for (field, member, board_id) in [
        ("sourceBoardId", Some(source), &halves.source_board_id),
        (
            "mirrorBoardId",
            mirror.map(String::as_str),
            &halves.mirror_board_id,
        ),
    ] {
        if let Some(member) = member
            && let Some(actual) = instance_board(member)
            && actual != board_id
        {
            return Err(format!(
                "mechanical pair {:?} halves.{field} {board_id:?} is not the board of {member:?}, which is on {actual:?}",
                pair.id
            ));
        }
    }
    for (index, correspondence) in halves.correspondences.iter().enumerate() {
        let location = format!(
            "mechanical pair {:?} halves.correspondences[{index}]",
            pair.id
        );
        validate_companion(
            correspondence
                .companion_mode
                .unwrap_or(halves.companion_mode),
            correspondence.companion_tweak,
        )
        .map_err(|error| format!("{location}: {error}"))?;
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
        validate_companion(
            exception.companion_mode.unwrap_or(halves.companion_mode),
            exception.companion_tweak,
        )
        .map_err(|error| format!("{location}: {error}"))?;
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

/// Whether an item of `kind` with `id` exists on the board `board_id`. Assemblies are definitions
/// that belong to the document rather than to a board, so the board is not consulted for them.
/// Parts, outlines and nets are listed by the board; a matrix, layout, module or embedded circuit names
/// its board; mechanical features belong to the pair.
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
        // A matrix that names no board is not placed on one, so it matches either half.
        MirroredItemKind::Matrix => document.matrices.iter().any(|matrix| {
            matrix.id == id
                && matrix
                    .board_id
                    .as_deref()
                    .is_none_or(|matrix_board| matrix_board == board_id)
        }),
        MirroredItemKind::Layout => document
            .layouts
            .iter()
            .any(|layout| layout.id == id && layout.board_id == board_id),
        MirroredItemKind::Assembly => document.assemblies.iter().any(|assembly| assembly.id == id),
        MirroredItemKind::Module => document
            .modules
            .iter()
            .any(|module| module.id == id && module.host_board_id == board_id),
        MirroredItemKind::ElectricalBoard => {
            id == board_id
                && document.hardware.as_ref().is_some_and(|hardware| {
                    hardware.boards.iter().any(|entry| entry.board_id == id)
                })
        }
        MirroredItemKind::OutlineVersion => document.board_outlines.iter().any(|outline| {
            outline.board_id == board_id && outline.versions.iter().any(|version| version.id == id)
        }),
        MirroredItemKind::MechanicalFeature => pair.features.iter().any(|feature| feature.id == id),
        MirroredItemKind::EmbeddedCircuit => document
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == id && circuit.host_board_id == board_id),
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
        MirroredItemKind::Module => "module",
        MirroredItemKind::ElectricalBoard => "electrical-board",
        MirroredItemKind::OutlineVersion => "outline-version",
        MirroredItemKind::MechanicalFeature => "mechanical-feature",
        MirroredItemKind::EmbeddedCircuit => "embedded-circuit",
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
            "constructionSource": "other-left",
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

    fn module(id: &str, host_board_id: &str) -> crate::modules::MountedModule {
        serde_json::from_value(json!({
            "id": id, "definitionId": "reference", "hostBoardId": host_board_id,
            "hostFace": "front", "facingFace": "back", "at": { "x": 0.0, "y": 0.0 },
            "rotation": 0.0, "gap": 0.0, "attachment": "board"
        }))
        .unwrap()
    }

    #[test]
    fn module_correspondences_must_be_hosted_on_their_board() {
        let mut document = document(vec![pair(halves(
            json!([{ "kind": "module", "sourceId": "l-module", "mirrorId": "r-module" }]),
            json!([{ "kind": "module", "mirrorId": "r-extra-module", "state": "local" }]),
        ))]);
        document.modules = vec![
            module("l-module", "left-board"),
            module("r-module", "right-board"),
            module("r-extra-module", "right-board"),
        ];
        validate_document(&document).unwrap();

        // A module hosted on the other board is not an item of this half.
        document.modules[1] = module("r-module", "left-board");
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("module \"r-module\""), "{message}");
        assert!(message.contains("\"right-board\""), "{message}");
    }

    fn matrix(id: &str, board_id: Option<&str>) -> crate::model::Matrix {
        serde_json::from_value(json!({
            "id": id, "rows": 1, "columns": 1, "pitch": { "x": 19.0, "y": 19.0 },
            "origin": { "x": 0.0, "y": 0.0 }, "definitionId": "switch", "partIds": [],
            "boardId": board_id
        }))
        .unwrap()
    }

    fn layout(id: &str, board_id: &str) -> crate::model::Layout {
        serde_json::from_value(json!({
            "id": id, "name": id, "boardId": board_id, "matrixId": "l-matrix", "partIds": []
        }))
        .unwrap()
    }

    #[test]
    fn matrix_and_layout_correspondences_must_belong_to_their_board() {
        let correspondences = json!([
            { "kind": "matrix", "sourceId": "l-matrix", "mirrorId": "r-matrix" },
            { "kind": "layout", "sourceId": "l-layout", "mirrorId": "r-layout" }
        ]);
        let mut document = document(vec![pair(halves(correspondences, json!([])))]);
        document.matrices = vec![
            matrix("l-matrix", Some("left-board")),
            matrix("r-matrix", Some("right-board")),
        ];
        document.layouts = vec![
            layout("l-layout", "left-board"),
            layout("r-layout", "right-board"),
        ];
        validate_document(&document).unwrap();

        // A matrix that names no board is not placed on one, so it is not refused for it.
        document.matrices[1] = matrix("r-matrix", None);
        validate_document(&document).unwrap();

        document.matrices[1] = matrix("r-matrix", Some("left-board"));
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[0]"), "{message}");
        assert!(message.contains("matrix \"r-matrix\""), "{message}");

        document.matrices[1] = matrix("r-matrix", Some("right-board"));
        document.layouts[1] = layout("r-layout", "left-board");
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("correspondences[1]"), "{message}");
        assert!(message.contains("layout \"r-layout\""), "{message}");
    }

    fn instance(id: &str, board_id: &str) -> crate::model::PhysicalBoardInstance {
        serde_json::from_value(json!({
            "id": id, "name": id, "boardId": board_id, "half": id, "role": "controller",
            "flipped": false
        }))
        .unwrap()
    }

    #[test]
    fn halves_need_a_source_whose_board_is_the_source_board() {
        let mut document = document(vec![pair(halves(valid_correspondence(), json!([])))]);
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "left-board"),
            instance("right", "right-board"),
        ];
        validate_document(&document).unwrap();

        // The members' boards are the record's boards in the other order.
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "right-board"),
            instance("right", "left-board"),
        ];
        let message = validate_document(&document).unwrap_err();
        assert!(message.contains("halves.sourceBoardId"), "{message}");
        assert!(message.contains("\"left\""), "{message}");

        // Reusing one board for both halves is a record of one board, not a mismatch.
        let mut reuse = halves(json!([]), json!([]));
        reuse["halves"]["mirrorBoardId"] = json!("left-board");
        let mut document = document_with(reuse);
        document.hardware.as_mut().unwrap().instances = vec![
            instance("left", "left-board"),
            instance("right", "left-board"),
        ];
        validate_document(&document).unwrap();

        // A record cannot say which member is its source without a construction source.
        let mut missing = halves(json!([]), json!([]));
        missing
            .as_object_mut()
            .unwrap()
            .remove("constructionSource");
        let message = refusal(missing);
        assert!(message.contains("constructionSource"), "{message}");
    }

    fn document_with(extra: Value) -> ProjectDoc {
        document(vec![pair(extra)])
    }
}
