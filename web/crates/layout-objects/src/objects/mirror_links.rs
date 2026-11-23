//! Shared mirror-link partner lookups over accepted layouts.
//!
//! Layouts join a mirrored pair on either side of the link: the linked layout
//! carries `mirror_link` (pointing at its source), and the source layout is
//! referenced back by the linked layout's `source_id`.
use boardstudio_model::model::{Layout, ProjectDoc};
use boardstudio_web_ui_model::tree::matrix_visible_on_board;

/// Find the layout that owns `matrix_id` on `board_id`.
pub fn layout_for_matrix<'a>(
    layouts: &'a [Layout],
    board_id: &str,
    matrix_id: &str,
) -> Option<&'a Layout> {
    layouts.iter().find(|layout| {
        layout.board_id == board_id && layout.matrix_ids.iter().any(|id| id == matrix_id)
    })
}

/// Whether the layout participates in a mirror link on either side.
pub fn layout_is_linked(layouts: &[Layout], layout: &Layout) -> bool {
    layout.mirror_link.is_some() || partner_layout(layouts, layout).is_some()
}

/// Follow `layout`'s own link to its source layout, if it carries one.
pub fn link_source<'a>(layouts: &'a [Layout], layout: &'a Layout) -> Option<&'a Layout> {
    let link = layout.mirror_link.as_ref()?;
    layouts
        .iter()
        .find(|candidate| candidate.id == link.source_id)
}

/// Resolve the partner layout of `layout`: the link source when `layout` carries
/// the link, otherwise the layout whose link points back at `layout`.
pub fn partner_layout<'a>(layouts: &'a [Layout], layout: &'a Layout) -> Option<&'a Layout> {
    link_source(layouts, layout).or_else(|| {
        layouts.iter().find(|other| {
            other
                .mirror_link
                .as_ref()
                .is_some_and(|link| link.source_id == layout.id)
        })
    })
}

/// Whether the matrix's owning layout on `board_id` participates in a mirror
/// link on either side.
pub fn matrix_is_linked(layouts: &[Layout], board_id: &str, matrix_id: &str) -> bool {
    layout_for_matrix(layouts, board_id, matrix_id)
        .is_some_and(|layout| layout_is_linked(layouts, layout))
}

/// Matrices visible on `board_id` that are not yet part of a mirror link
/// (including matrices that have no `Layout` yet).
pub fn unlinked_board_matrices(document: &ProjectDoc, board_id: &str) -> Vec<(String, String)> {
    let mut options = Vec::new();
    for matrix in document
        .matrices
        .iter()
        .filter(|matrix| matrix_visible_on_board(document, board_id, &matrix.id))
    {
        if !matrix_is_linked(&document.layouts, board_id, &matrix.id) {
            options.push((
                matrix.id.clone(),
                matrix
                    .name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("Matrix {}", options.len() + 1)),
            ));
        }
    }
    options
}

#[cfg(test)]
mod tests {
    use super::unlinked_board_matrices;
    use boardstudio_model::model::{Board, Layout, LayoutMirrorLink, Matrix, ProjectDoc, Vec2};

    fn test_matrix(id: &str, name: &str, board_id: &str) -> Matrix {
        Matrix {
            id: id.into(),
            name: Some(name.into()),
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2::default(),
            definition_id: "sw".into(),
            part_ids: vec![],
            board_id: Some(board_id.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            column_layout: vec![],
            row_layout: vec![],
            anchor: None,
            cells: vec![],
        }
    }

    fn test_layout(id: &str, matrix_id: &str, source_id: Option<&str>) -> Layout {
        Layout {
            id: id.into(),
            name: id.into(),
            board_id: "board".into(),
            matrix_ids: vec![matrix_id.into()],
            part_ids: vec![],
            mirror_link: source_id.map(|source_id| LayoutMirrorLink {
                source_id: source_id.into(),
                axis_x: 0.0,
                split_gap: None,
                spacing: None,
                correspondences: vec![],
                exceptions: vec![],
            }),
        }
    }

    #[test]
    fn unlinked_board_matrices_keeps_matrices_without_a_layout() {
        let mut document = ProjectDoc::empty("doc", "Doc");
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
        document.matrices = vec![
            test_matrix("no-layout", "Free Matrix", "board"),
            test_matrix("unlinked", "Standalone Layout", "board"),
            test_matrix("source", "Source Half", "board"),
            test_matrix("linked", "Mirrored Half", "board"),
            test_matrix("other-board", "Other Board", "other"),
        ];
        document.layouts = vec![
            test_layout("layout-unlinked", "unlinked", None),
            test_layout("layout-source", "source", None),
            test_layout("layout-linked", "linked", Some("layout-source")),
        ];

        assert_eq!(
            unlinked_board_matrices(&document, "board"),
            vec![
                ("no-layout".into(), "Free Matrix".into()),
                ("unlinked".into(), "Standalone Layout".into()),
            ]
        );
    }
}
