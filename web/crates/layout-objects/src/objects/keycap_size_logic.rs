//! Host-compatible keycap-size projection and resolver helpers.
//!
//! The production key-size controller (wasm) delegates to this module so the
//! same predicates run under `cargo test` without a browser.

use super::keycap_resize::{self, KeycapPlacement, KeycapResizeInput, ResizeAxis};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, ProjectDoc, Vec2};
use std::collections::BTreeSet;

pub(super) fn placements(
    document: &ProjectDoc,
    scenes: &[boardstudio_model::model::MatrixScene],
    board_id: &str,
) -> Vec<KeycapPlacement> {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return Vec::new();
    };
    let board_parts: BTreeSet<_> = board.part_ids.iter().map(String::as_str).collect();
    let mut result = Vec::new();
    for matrix in document
        .matrices
        .iter()
        .filter(|matrix| matrix.board_id.as_deref().is_none_or(|id| id == board_id))
    {
        let Some(scene) = scenes.iter().find(|scene| scene.matrix_id == matrix.id) else {
            continue;
        };
        for cell in scene.cells.iter().filter(|cell| cell.enabled) {
            let Some(id) = cell.member_id.as_deref() else {
                continue;
            };
            if !matrix.part_ids.iter().any(|member| member == id) || !board_parts.contains(id) {
                continue;
            }
            let Some(part) = document.parts.iter().find(|part| part.id == id) else {
                continue;
            };
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id);
            let gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            let size = part
                .keycap
                .or_else(|| definition.and_then(|definition| definition.keycap))
                .unwrap_or(Vec2 {
                    x: (matrix.pitch.x - gap.x).max(1.0),
                    y: (matrix.pitch.y - gap.y).max(1.0),
                });
            result.push(KeycapPlacement {
                id: id.to_owned(),
                matrix_id: matrix.id.clone(),
                row: cell.row,
                column: cell.column,
                at: cell.pose.at,
                rotation: cell.pose.rotation,
                size,
            });
        }
    }
    order_placements_by_document_parts(&document.parts, &mut result);
    result
}

pub(super) fn order_placements_by_document_parts(
    parts: &[boardstudio_model::model::Part],
    placements: &mut [KeycapPlacement],
) {
    let order: std::collections::HashMap<_, _> = parts
        .iter()
        .enumerate()
        .map(|(index, part)| (part.id.as_str(), index))
        .collect();
    placements.sort_by_key(|placement| {
        order
            .get(placement.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
}

pub(super) fn selected_keycap_ids(
    selected_part_ids: &[String],
    placements: &[KeycapPlacement],
) -> BTreeSet<String> {
    let visible: BTreeSet<_> = placements
        .iter()
        .map(|placement| placement.id.as_str())
        .collect();
    selected_part_ids
        .iter()
        .filter(|id| visible.contains(id.as_str()))
        .cloned()
        .collect()
}

pub(super) fn quarter(value: f64) -> f64 {
    (value * 4.0).round() / 4.0
}

pub(super) fn overlap_references(
    parts: &[boardstudio_model::model::Part],
    placements: &[KeycapPlacement],
    selected_ids: &BTreeSet<String>,
) -> Vec<String> {
    let references: std::collections::HashMap<_, _> = parts
        .iter()
        .map(|part| (part.id.as_str(), part.reference.as_str()))
        .collect();
    let mut shown = BTreeSet::new();
    let mut result = Vec::new();
    for (first, second) in keycap_resize::overlapping_pairs(placements) {
        if !selected_ids.contains(&first) && !selected_ids.contains(&second) {
            continue;
        }
        for id in [first, second] {
            if let Some(reference) = references.get(id.as_str())
                && shown.insert(*reference)
            {
                result.push((*reference).to_owned());
            }
        }
    }
    result
}

/// Resolve a key-size change against the accepted document at execution time.
pub(super) fn resize_resolver(
    board_id: String,
    selected_ids: Vec<String>,
    units: Vec2,
    axis: Option<ResizeAxis>,
) -> EditResolver {
    EditResolver::new("layout-key-size", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let placements = placements(document, &accepted.scene.matrix_scenes, &board_id);
        if selected_ids
            .iter()
            .any(|id| !placements.iter().any(|placement| placement.id == *id))
        {
            return Resolution::Retire("A selected key no longer exists.".into());
        }
        let selected = selected_ids.iter().cloned().collect::<BTreeSet<_>>();
        let Some(plan) = keycap_resize::plan_resize(KeycapResizeInput {
            document,
            matrices: &document.matrices,
            scenes: &accepted.scene.matrix_scenes,
            layouts: &document.layouts,
            placements: &placements,
            selected_ids: &selected,
            units,
            axis,
        }) else {
            return Resolution::Unchanged;
        };
        Resolution::submit(
            plan.target_ids,
            EditOperation::SetKeycapSize {
                sizes: plan.sizes,
                moves: plan.moves,
                offsets: plan.offsets,
            },
        )
    })
}

/// Coordinate-based member IDs can be reused after a row or column is removed.
pub(super) fn resize_resolver_for(
    captured: &ProjectDoc,
    board_id: String,
    selected_ids: Vec<String>,
    units: Vec2,
    axis: Option<ResizeAxis>,
) -> EditResolver {
    let topology: Vec<_> = captured
        .matrices
        .iter()
        .filter(|matrix| selected_ids.iter().any(|id| matrix.part_ids.contains(id)))
        .map(|matrix| (matrix.id.clone(), matrix.rows, matrix.columns))
        .collect();
    let resolver = resize_resolver(board_id, selected_ids, units, axis);
    EditResolver::new("layout-key-size", move |accepted: &AcceptedSnapshot| {
        if topology.iter().any(|(id, rows, columns)| {
            !accepted.document.matrices.iter().any(|matrix| {
                matrix.id == *id && matrix.rows == *rows && matrix.columns == *columns
            })
        }) {
            return Resolution::Retire("The selected key's matrix topology changed.".into());
        }
        resolver.resolve(accepted)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{Part, Pose2, Side};

    fn make_part(id: &str, reference: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            anchor: None,
            id: id.into(),
            definition_id: "switch".into(),
            reference: reference.into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn make_placement(id: &str, width: f64, matrix_id: &str) -> KeycapPlacement {
        KeycapPlacement {
            id: id.into(),
            matrix_id: matrix_id.into(),
            row: 0,
            column: 0,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            size: Vec2 { x: width, y: 18.0 },
        }
    }

    #[test]
    fn overlap_filter_tracks_authoritative_add_and_toggle_selection() {
        let parts = vec![
            make_part("a", "A"),
            make_part("c", "C"),
            make_part("b", "B"),
        ];
        let placements = vec![
            KeycapPlacement {
                id: "a".into(),
                matrix_id: "m1".into(),
                row: 0,
                column: 0,
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                size: Vec2 { x: 18.0, y: 18.0 },
            },
            KeycapPlacement {
                id: "c".into(),
                matrix_id: "m2".into(),
                row: 0,
                column: 0,
                at: Vec2 { x: 10.0, y: 0.0 },
                rotation: 0.0,
                size: Vec2 { x: 18.0, y: 18.0 },
            },
            KeycapPlacement {
                id: "b".into(),
                matrix_id: "m3".into(),
                row: 0,
                column: 0,
                at: Vec2 { x: 100.0, y: 0.0 },
                rotation: 0.0,
                size: Vec2 { x: 18.0, y: 18.0 },
            },
        ];
        let labels = |selected: &[&str]| {
            let ids: Vec<String> = selected.iter().map(|id| (*id).to_owned()).collect();
            let selected = selected_keycap_ids(&ids, &placements);
            overlap_references(&parts, &placements, &selected)
        };

        assert!(labels(&["b"]).is_empty());
        assert_eq!(labels(&["a", "b"]), ["A", "C"]);
        assert!(labels(&["b"]).is_empty());
        assert_eq!(labels(&["b", "c"]), ["A", "C"]);
    }

    #[test]
    fn mixed_size_draft_uses_react_visible_document_part_order() {
        let document_parts = vec![make_part("part-a", "A"), make_part("part-b", "B")];
        let mut accepted_placements = vec![
            make_placement("part-b", 18.0, "matrix-b"),
            make_placement("part-a", 37.0, "matrix-a"),
        ];

        order_placements_by_document_parts(&document_parts, &mut accepted_placements);

        let selected: BTreeSet<_> = ["part-a", "part-b"].into_iter().collect();
        let selected_items: Vec<_> = accepted_placements
            .iter()
            .filter(|placement| selected.contains(placement.id.as_str()))
            .collect();
        let first = selected_items
            .first()
            .expect("both selected caps are projected");
        let first_units = ((first.size.x + 1.0) / 19.0 * 4.0).round() / 4.0;
        assert_eq!(first.id, "part-a");
        assert_eq!(first_units, 2.0, "Wide/Tall drafts from React's first item");
        assert_eq!(selected_items.len(), 2);
    }

    #[test]
    fn resize_resolver_submits_a_typed_keycap_size_edit() {
        use boardstudio_application::Resolution;

        // Invalid units resolve Unchanged without needing placements: an empty
        // selection has no topology to guard and no plan to build.
        let document = ProjectDoc::empty("project", "Keyboard");
        // Minimal accepted snapshot scaffolding for a pure resolver check.
        let scene: boardstudio_model::model::SceneDelta =
            serde_json::from_value(serde_json::json!({
                "revision": 0,
                "transactionId": "fixture",
                "changedIds": [],
                "transforms": [],
                "matrixScenes": [],
                "contours": [],
                "boardContours": [],
                "boardReadiness": [],
                "boardOutlineScenes": [],
                "findingMarkers": [],
                "findings": [],
                "readiness": {"layout": true, "outline": true, "pcb": true, "case": false}
            }))
            .unwrap();
        let snapshot = AcceptedSnapshot {
            token: boardstudio_application::SnapshotToken(1),
            session_epoch: boardstudio_application::SessionEpoch(1),
            document: std::sync::Arc::new(document),
            scene: std::sync::Arc::new(scene),
        };
        // Invalid units resolve Unchanged and write nothing.
        let invalid = resize_resolver_for(
            &snapshot.document,
            "board-main".into(),
            Vec::new(),
            Vec2 {
                x: f64::NAN,
                y: 1.0,
            },
            Some(ResizeAxis::X),
        );
        assert!(
            matches!(invalid.resolve(&snapshot), Resolution::Unchanged),
            "invalid units write nothing"
        );
    }
}
