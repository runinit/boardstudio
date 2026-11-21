//! Host-compatible mirror-board predicate shared by every presentation crate.
//!
//! The Parts library owns the presentation call sites; this module is the single
//! production implementation used on both targets and in native tests.
use boardstudio_model::model::ProjectDoc;

/// Whether `board_id` is the unique mirror half of a split pair. Requires exactly one
/// pair whose mirror half is `board_id`, distinct source/mirror boards, and exactly one
/// board entry for each side.
pub fn has_unique_mirror_board_pair(document: &ProjectDoc, board_id: &str) -> bool {
    let Some(hardware) = document.hardware.as_ref() else {
        return false;
    };
    let mut matches = hardware.pairs.iter().filter(|pair| {
        pair.halves
            .as_ref()
            .is_some_and(|halves| halves.mirror_board_id == board_id)
    });
    let Some(pair) = matches.next() else {
        return false;
    };
    if matches.next().is_some() {
        return false;
    }
    let Some(halves) = pair.halves.as_ref() else {
        return false;
    };
    halves.source_board_id != halves.mirror_board_id
        && [
            halves.source_board_id.as_str(),
            halves.mirror_board_id.as_str(),
        ]
        .into_iter()
        .all(|id| {
            document
                .boards
                .iter()
                .filter(|board| board.id == id)
                .count()
                == 1
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("placement-ui", "Placement UI");
        for (id, name) in [("board-main", "Main"), ("board-mirror", "Mirror")] {
            document.boards.push(boardstudio_model::model::Board {
                id: id.into(),
                name: name.into(),
                outline_ids: vec![],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            document.layouts.push(boardstudio_model::model::Layout {
                id: format!("layout-{id}"),
                name: format!("{name} layout"),
                board_id: id.into(),
                matrix_ids: vec![format!("matrix-{id}")],
                part_ids: vec![],
                mirror_link: None,
            });
        }
        document.hardware = Some(
            serde_json::from_value(serde_json::json!({
                "topology": "split", "transport": "wired", "instances": [], "boards": [],
                "pairs": [{
                    "id": "pair-main-mirror", "members": ["instance-main", "instance-mirror"],
                    "mapping": {"at": {"x": 200.0, "y": 0.0}, "rotation": 0.0},
                    "construction": {
                        "method": "printed", "mount": "rigid", "integratedPlateFrame": false,
                        "plateThickness": 1.5, "plateFoamThickness": 0.0, "bottomFoamThickness": 0.0,
                        "bottomThickness": 2.0, "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.2
                    },
                    "halves": {"sourceBoardId": "board-main", "mirrorBoardId": "board-mirror"}
                }]
            }))
            .unwrap(),
        );
        document
    }

    #[test]
    fn mirror_only_requires_one_distinct_owned_mirror_board() {
        let document = document();
        assert!(has_unique_mirror_board_pair(&document, "board-mirror"));
        assert!(!has_unique_mirror_board_pair(&document, "board-main"));

        let mut ambiguous = document.clone();
        let duplicate_pair = ambiguous.hardware.as_ref().unwrap().pairs[0].clone();
        ambiguous
            .hardware
            .as_mut()
            .unwrap()
            .pairs
            .push(duplicate_pair);
        assert!(!has_unique_mirror_board_pair(&ambiguous, "board-mirror"));

        let mut duplicate_board = document.clone();
        duplicate_board
            .boards
            .push(duplicate_board.boards[1].clone());
        assert!(!has_unique_mirror_board_pair(
            &duplicate_board,
            "board-mirror"
        ));

        let mut unpaired = document.clone();
        unpaired.hardware = None;
        assert!(!has_unique_mirror_board_pair(&unpaired, "board-mirror"));
    }
}
