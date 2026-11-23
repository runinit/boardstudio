//! Prepare canonical pairs from existing authored matrices.
use boardstudio_model::mirrored_half::{MirrorDestination, MirroredPair};
use boardstudio_model::model::{Layout, Pose2, ProjectDoc, Vec2};

#[derive(Clone)]
pub struct PreparedExistingHalf {
    pub document: ProjectDoc,
}

pub fn prepare_existing_half(
    document: &ProjectDoc,
    board_id: &str,
    matrix_ids: &[String],
    axis_x: f64,
    new_id: &mut impl FnMut() -> Result<String, String>,
) -> Result<PreparedExistingHalf, String> {
    if matrix_ids.is_empty() || !axis_x.is_finite() {
        return Err("Select a layout and a finite mirror axis.".into());
    }
    let mut next = document.clone();
    let mut used_ids = document
        .boards
        .iter()
        .map(|item| item.id.clone())
        .chain(document.matrices.iter().map(|item| item.id.clone()))
        .chain(document.layouts.iter().map(|item| item.id.clone()))
        .chain(document.parts.iter().map(|item| item.id.clone()))
        .collect::<std::collections::BTreeSet<_>>();

    for matrix_id in matrix_ids {
        let source = next
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)
            .cloned()
            .ok_or_else(|| "The selected matrix no longer exists.".to_string())?;
        if source.board_id.as_deref() != Some(board_id) {
            return Err("The layout must belong to the active board.".into());
        }
        if source.mirror == Some(boardstudio_model::model::Mirror::Y) {
            return Err(
                "Y-mirrored layouts cannot be linked. Use an independent layout or an X-mirrored source."
                    .into(),
            );
        }
        if source
            .part_ids
            .iter()
            .any(|part_id| next.constraint_driving_part(part_id) != Ok(None))
        {
            return Err("Remove key placement constraints before linking this layout.".into());
        }
        let existing = next
            .layouts
            .iter()
            .find(|layout| {
                layout.matrix_ids.iter().any(|id| id == &source.id) && layout.board_id == board_id
            })
            .cloned();
        if existing.as_ref().is_some_and(|layout| {
            let location = MirrorDestination::SameBoardLayout(layout.id.clone());
            next.mirrored_pairs
                .iter()
                .any(|pair| pair.source == location || pair.destination == location)
        }) {
            return Err("This layout already belongs to a linked pair.".into());
        }
        let source_layout = if let Some(layout) = existing {
            layout
        } else {
            let id = fresh_id(&mut used_ids, new_id)?;
            let name = source
                .name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or("Original half")
                .to_owned();
            let layout = Layout {
                id,
                name,
                board_id: board_id.to_owned(),
                matrix_ids: vec![source.id.clone()],
                part_ids: Vec::new(),
                mirror_link: None,
            };
            next.layouts.push(layout.clone());
            layout
        };

        let target_layout_id = fresh_id(&mut used_ids, new_id)?;
        next.layouts.push(Layout {
            id: target_layout_id.clone(),
            name: format!("{} · mirrored", source_layout.name),
            board_id: board_id.to_owned(),
            matrix_ids: vec![],
            part_ids: vec![],
            mirror_link: None,
        });
        next.mirrored_pairs.push(MirroredPair {
            id: target_layout_id.clone(),
            source: MirrorDestination::SameBoardLayout(source_layout.id),
            destination: MirrorDestination::SameBoardLayout(target_layout_id),
            mapping: Pose2 {
                at: Vec2 {
                    x: 2.0 * axis_x,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            overrides: vec![],
        });
    }

    Ok(PreparedExistingHalf { document: next })
}

fn fresh_id(
    used: &mut std::collections::BTreeSet<String>,
    new_id: &mut impl FnMut() -> Result<String, String>,
) -> Result<String, String> {
    for _ in 0..8 {
        let id = new_id()?;
        if !id.trim().is_empty() && used.insert(id.clone()) {
            return Ok(id);
        }
    }
    Err("Could not allocate a unique layout identity.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_model::model::{Board, Matrix};

    fn matrix_fixture() -> Matrix {
        serde_json::from_value(serde_json::json!({
            "id": "matrix-source",
            "name": "Source half",
            "rows": 2,
            "columns": 3,
            "pitch": { "x": 19.05, "y": 19.05 },
            "origin": { "x": 0.0, "y": 0.0 },
            "definitionId": "switch",
            "partIds": [],
            "boardId": "board",
            "cells": []
        }))
        .unwrap()
    }

    fn document_fixture(with_source_layout: bool) -> ProjectDoc {
        let mut document = ProjectDoc::empty("existing-half-fixture", "Existing half fixture");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.matrices.push(matrix_fixture());
        if with_source_layout {
            document.layouts.push(Layout {
                id: "layout-source".into(),
                name: "Source half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["matrix-source".into()],
                part_ids: Vec::new(),
                mirror_link: None,
            });
        }
        document
    }

    fn prepared(document: &ProjectDoc) -> PreparedExistingHalf {
        let mut counter = 0u32;
        let mut new_id = || {
            counter += 1;
            Ok(format!("fresh-{counter}"))
        };
        prepare_existing_half(
            document,
            "board",
            &["matrix-source".into()],
            10.0,
            &mut new_id,
        )
        .expect("mirror-existing-half prepares a linked copy")
    }

    #[test]
    fn mirror_existing_half_stores_only_source_geometry() {
        for with_layout in [false, true] {
            let prepared = prepared(&document_fixture(with_layout));
            let document = prepared.document;
            assert_eq!(document.matrices.len(), 1);
            assert_eq!(document.layouts.len(), 2);
            assert!(
                document
                    .layouts
                    .iter()
                    .all(|layout| layout.mirror_link.is_none())
            );
            let pair = &document.mirrored_pairs[0];
            let MirrorDestination::SameBoardLayout(source) = &pair.source else {
                panic!("same board source")
            };
            let MirrorDestination::SameBoardLayout(destination) = &pair.destination else {
                panic!("same board destination")
            };
            let layout = |id: &str| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.id == id)
                    .unwrap()
            };
            assert_eq!(layout(source).matrix_ids, ["matrix-source"]);
            assert!(layout(destination).matrix_ids.is_empty());
            if with_layout {
                assert_eq!(source, "layout-source");
            }
        }
    }
}
