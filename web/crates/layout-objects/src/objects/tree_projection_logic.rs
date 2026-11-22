//! Host-compatible tree projection for the Objects dock grouping.
//!
//! The production Objects tree (wasm) delegates to this module so the same
//! grouping runs under `cargo test` without a browser.

use boardstudio_application::ReadModel;
use boardstudio_web_ui_model::tree::{self, TreeKind};
use std::collections::BTreeSet;

pub(crate) fn grouped_part_tree(
    model: &ReadModel,
    expanded: &BTreeSet<String>,
) -> Vec<tree::TreeItem> {
    let Some(snapshot) = model.accepted.as_ref() else {
        return Vec::new();
    };
    let flat = tree::build_pcb_tree(model, expanded);
    let document = boardstudio_application::resolved_document(&snapshot.document);
    let mut result: Vec<_> = flat
        .iter()
        .filter(|row| row.kind != TreeKind::Component)
        .cloned()
        .collect();
    let mut groups = std::collections::BTreeMap::<&str, Vec<tree::TreeItem>>::new();
    for mut row in flat
        .into_iter()
        .filter(|row| row.kind == TreeKind::Component)
    {
        let definition = row
            .primary_id
            .as_ref()
            .and_then(|id| document.parts.iter().find(|part| &part.id == id))
            .and_then(|part| {
                document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
            });
        let category = definition.map_or("Other", |definition| {
            let name = format!("{} {}", definition.id, definition.name).to_lowercase();
            match definition.kind {
                boardstudio_model::model::PartKind::Switch => "Switches",
                boardstudio_model::model::PartKind::Controller => "MCUs",
                boardstudio_model::model::PartKind::Encoder => "Encoders",
                boardstudio_model::model::PartKind::Connector => "Connectors",
                _ if name.contains("diode") => "Diodes",
                _ if name.contains("led") => "LEDs",
                _ => "Other",
            }
        });
        row.level = 2;
        groups.entry(category).or_default().push(row);
    }
    for name in [
        "Switches",
        "MCUs",
        "Diodes",
        "LEDs",
        "Encoders",
        "Connectors",
        "Other",
    ] {
        let Some(mut rows) = groups.remove(name) else {
            continue;
        };
        rows.sort_by(|a, b| a.label.cmp(&b.label));
        let id = format!("category:{name}");
        let open = expanded.contains(&id);
        result.push(tree::TreeItem {
            id,
            label: name.into(),
            detail: Some(format!("{} parts", rows.len())),
            level: 1,
            kind: TreeKind::Components,
            context: None,
            expanded: Some(open),
            primary_id: None,
            expandable: true,
        });
        if open {
            result.extend(rows);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{AcceptedSnapshot, SessionEpoch, SnapshotToken};
    use boardstudio_model::model::{Board, Part, Pose2, ProjectDoc, Side};
    use std::sync::Arc;

    fn snapshot(document: ProjectDoc) -> ReadModel {
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
        ReadModel {
            accepted: Some(AcceptedSnapshot {
                session_epoch: SessionEpoch(1),
                token: SnapshotToken(7),
                document: Arc::new(document),
                scene: Arc::new(scene),
            }),
            active_board_id: "board-1".into(),
            selected_part_ids: vec![],
            ..ReadModel::default()
        }
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Keyboard");
        document.boards.push(Board {
            id: "board-1".into(),
            name: "Main".into(),
            outline_ids: vec![],
            part_ids: vec!["part-1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            id: "part-1".into(),
            definition_id: "switch:base".into(),
            reference: "SW1".into(),
            pose: Pose2 {
                at: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            anchor: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "switch:base",
                "name": "MX switch",
                "kind": "switch",
                "courtyard": [],
                "pads": []
            }))
            .unwrap(),
        );
        document
    }

    #[test]
    fn grouped_part_tree_buckets_switches_and_reports_counts() {
        let model = snapshot(document());
        let expanded = BTreeSet::from(["board:board-1".to_owned(), "category:Switches".to_owned()]);
        let items = grouped_part_tree(&model, &expanded);
        let category = items
            .iter()
            .find(|item| item.id == "category:Switches")
            .expect("switch category");
        assert_eq!(category.detail.as_deref(), Some("1 parts"));
        assert_eq!(category.expanded, Some(true));
        assert!(
            items
                .iter()
                .any(|item| item.primary_id.as_deref() == Some("part-1"))
        );
    }

    #[test]
    fn collapsed_category_hides_its_members_but_keeps_the_row() {
        let model = snapshot(document());
        let items = grouped_part_tree(&model, &BTreeSet::from(["board:board-1".to_owned()]));
        assert!(items.iter().any(|item| item.id == "category:Switches"));
        assert!(
            !items
                .iter()
                .any(|item| item.primary_id.as_deref() == Some("part-1")),
            "collapsed categories hide members"
        );
    }

    #[test]
    fn empty_model_projects_no_groups() {
        let model = ReadModel::default();
        assert!(grouped_part_tree(&model, &BTreeSet::new()).is_empty());
    }

    #[test]
    fn grouped_part_tree_buckets_derived_mirrored_parts() {
        let mut doc = document();
        doc.boards.push(Board {
            id: "board-2".into(),
            name: "Right".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.mirrored_pairs
            .push(boardstudio_model::mirrored_half::MirroredPair {
                id: "pair-1".into(),
                source: boardstudio_model::mirrored_half::MirrorDestination::SecondBoard(
                    "board-1".into(),
                ),
                destination: boardstudio_model::mirrored_half::MirrorDestination::SecondBoard(
                    "board-2".into(),
                ),
                mapping: Pose2 {
                    at: boardstudio_model::model::Vec2 { x: 100.0, y: 0.0 },
                    rotation: 0.0,
                },
                overrides: vec![],
            });
        let mut model = snapshot(doc);
        model.active_board_id = "board-2".into();
        let expanded = BTreeSet::from(["board:board-2".to_owned(), "category:Switches".to_owned()]);
        let items = grouped_part_tree(&model, &expanded);
        let switches = items
            .iter()
            .find(|item| item.id == "category:Switches")
            .expect("derived mirrored switch should be bucketed under Switches");
        assert_eq!(switches.detail.as_deref(), Some("1 parts"));
        assert!(
            items.iter().all(|item| item.id != "category:Other"),
            "derived mirrored switch must not fall back to Other: {items:?}"
        );
    }
}
