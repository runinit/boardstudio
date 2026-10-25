//! Native-testable preparation for linking copies of existing unpaired matrices.
//!
//! The [`prepare_existing_half`] builder is shared with mirrored-pair creation:
//! both emit one explicit [`MirroredCorrespondence`] per matrix pair in the link
//! so Core never sees an empty correspondence list for a linked pair.
use boardstudio_core::model::{Layout, LayoutMirrorLink, ProjectDoc};

#[derive(Clone)]
pub(crate) struct PreparedExistingHalf {
    pub(crate) document: ProjectDoc,
}

pub(crate) fn prepare_existing_half(
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
        if source.mirror == Some(boardstudio_core::model::Mirror::Y) {
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
            layout.mirror_link.is_some()
                || next.layouts.iter().any(|candidate| {
                    candidate
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == layout.id)
                })
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

        let target_matrix_id = fresh_id(&mut used_ids, new_id)?;
        let target_layout_id = fresh_id(&mut used_ids, new_id)?;
        let target_name = format!("{} · mirrored", source_layout.name);
        let mut target_matrix = source.clone();
        target_matrix.id = target_matrix_id.clone();
        target_matrix.name = Some(target_name.clone());
        target_matrix.part_ids.clear();
        for cell in &mut target_matrix.cells {
            for assembly in &mut cell.assemblies {
                assembly.offset.x = -assembly.offset.x;
                assembly.rotation = assembly.rotation.map(|rotation| -rotation);
            }
        }
        next.matrices.push(target_matrix);
        next.layouts.push(Layout {
            id: target_layout_id.clone(),
            name: target_name,
            board_id: board_id.to_owned(),
            matrix_ids: vec![target_matrix_id.clone()],
            part_ids: Vec::new(),
            mirror_link: Some(LayoutMirrorLink {
                correspondences: crate::mirrored_pair_geometry::matrix_pair_correspondences(
                    &source.id,
                    &target_matrix_id,
                ),
                exceptions: vec![],
                source_id: source_layout.id,
                axis_x,
            }),
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
    use boardstudio_core::model::{Board, Matrix};

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
        prepare_existing_half(document, "board", &["matrix-source".into()], 10.0, &mut new_id)
            .expect("mirror-existing-half prepares a linked copy")
    }

    fn mirror_matrix_id(prepared: &PreparedExistingHalf) -> String {
        prepared
            .document
            .layouts
            .iter()
            .find(|layout| layout.mirror_link.is_some())
            .expect("mirror layout exists")
            .matrix_ids
            .iter()
            .next()
            .expect("mirror layout carries its matrix")
            .clone()
    }

    #[test]
    fn mirror_existing_half_links_source_and_mirror_matrices_with_one_explicit_correspondence() {
        let prepared = prepared(&document_fixture(false));

        let link = prepared
            .document
            .layouts
            .iter()
            .find_map(|layout| layout.mirror_link.as_ref())
            .expect("mirror-existing-half creates a linked half");
        assert_eq!(
            link.correspondences,
            vec![boardstudio_core::model::MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: boardstudio_core::model::MirroredItemKind::Matrix,
                source_id: "matrix-source".into(),
                mirror_id: mirror_matrix_id(&prepared),
            }],
            "mirror-existing-half must emit one explicit per-matrix correspondence"
        );
    }

    #[test]
    fn mirror_existing_half_reuses_source_layout_and_still_links_matrices_explicitly() {
        let prepared = prepared(&document_fixture(true));

        let sources = prepared
            .document
            .layouts
            .iter()
            .filter(|layout| {
                layout.mirror_link.is_none() && layout.matrix_ids == ["matrix-source"]
            })
            .count();
        assert_eq!(
            sources, 1,
            "the existing source layout is reused, not duplicated"
        );
        let link = prepared
            .document
            .layouts
            .iter()
            .find_map(|layout| layout.mirror_link.as_ref())
            .expect("mirror-existing-half creates a linked half");
        assert_eq!(link.source_id, "layout-source");
        assert_eq!(link.correspondences.len(), 1);
        let correspondence = &link.correspondences[0];
        assert_eq!(
            correspondence.kind,
            boardstudio_core::model::MirroredItemKind::Matrix
        );
        assert_eq!(correspondence.source_id, "matrix-source");
        assert_eq!(correspondence.mirror_id, mirror_matrix_id(&prepared));
    }
}
