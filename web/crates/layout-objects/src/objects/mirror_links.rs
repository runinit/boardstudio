//! Shared mirror-link partner lookups over accepted layouts.
//!
//! Layouts join a mirrored pair on either side of the link: the linked layout
//! carries `mirror_link` (pointing at its source), and the source layout is
//! referenced back by the linked layout's `source_id`.
use boardstudio_model::model::Layout;

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
