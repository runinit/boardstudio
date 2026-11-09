//! Presentation-only outlet for the existing Matrix Setup draft. No runtime requests.
use super::matrix_setup::{MatrixSetupMount, MatrixSetupOwner};
use super::matrix_setup_fields::parse_matrix_dimensions;
use boardstudio_core::{
    CoreEngine,
    model::{CoreReply, CoreRequest, Matrix, Vec2},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct Paint {
    source: Rc<()>,
    owner: MatrixSetupOwner,
    rows: u32,
    columns: u32,
}

impl PartialEq for Paint {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.source, &other.source)
            && self.owner == other.owner
            && self.rows == other.rows
            && self.columns == other.columns
    }
}

#[derive(Clone, Copy)]
struct PreviewStore(Signal<Option<Paint>>);

/// Install private, editor-lifetime paint storage above Matrix Setup and its canvas outlet.
/// The form remains the only draft owner; this provider exposes no draft mutation interface.
pub fn use_matrix_setup_preview_provider() {
    use_context_provider(|| PreviewStore(Signal::new(None)));
}

pub(super) fn use_matrix_setup_preview(owner: &MatrixSetupOwner, rows: &str, columns: &str) {
    let store = try_use_context::<PreviewStore>();
    let source = use_hook(|| Rc::new(()));
    let effect_source = source.clone();
    let dimensions = parse_matrix_dimensions(rows, columns);
    use_effect(use_reactive(
        (owner, &dimensions),
        move |(owner, dimensions)| {
            if let Some(PreviewStore(mut paint)) = store {
                let next = dimensions.map(|(rows, columns)| Paint {
                    source: effect_source.clone(),
                    owner,
                    rows,
                    columns,
                });
                if *paint.peek() != next {
                    paint.set(next);
                }
            }
        },
    ));
    use_drop(move || {
        if let Some(PreviewStore(mut paint)) = store
            && let Ok(mut current) = paint.try_write()
            && current
                .as_ref()
                .is_some_and(|paint| Rc::ptr_eq(&paint.source, &source))
        {
            *current = None;
        }
    });
}

/// Paint the current setup in a page-projected, unobscured canvas frame (Y-up).
/// This is a schematic footprint, not placement: it never changes the camera or accepted scene.
#[component]
pub fn MatrixSetupCanvasPreview(
    setup: MatrixSetupMount,
    center: Vec2,
    available_size: Vec2,
) -> Element {
    let paint = try_use_context::<PreviewStore>().and_then(|store| (store.0)());
    let Some(paint) = paint.filter(|paint| {
        setup
            .projection
            .as_ref()
            .is_some_and(|projection| projection.editable && projection.owner == paint.owner)
    }) else {
        return rsx! {};
    };
    if !center.x.is_finite()
        || !center.y.is_finite()
        || !available_size.x.is_finite()
        || !available_size.y.is_finite()
        || available_size.x <= 0.0
        || available_size.y <= 0.0
    {
        return rsx! {};
    }
    let cells = preview_cells(paint.rows, paint.columns);
    // Keep small grids on one scale so both slider dimensions visibly grow. Large grids
    // shrink only the schematic, never the accepted canvas camera or its fit owner.
    let scale = (available_size.x / (f64::from(paint.columns.max(8)) * 19.05))
        .min(available_size.y / (f64::from(paint.rows.max(8)) * 19.05))
        * 0.8;
    let offset_x = f64::from(paint.columns - 1) * 19.05 / 2.0;
    let offset_y = f64::from(paint.rows - 1) * 19.05 / 2.0;
    rsx! {
        g {
            class: "pointer-events-none", "aria-label": "New matrix preview",
            transform: "translate({center.x} {center.y}) scale({scale}) translate({-offset_x} {-offset_y})",
            for cell in cells {
                g { key: "{cell.row}-{cell.column}", transform: "translate({cell.center.x} {cell.center.y}) rotate({cell.rotation})",
                    rect { class: "fill-primary/20 stroke-primary stroke-[0.35] [stroke-dasharray:1.5_1]", x: "{-cell.size.x / 2.0}", y: "{-cell.size.y / 2.0}", width: "{cell.size.x}", height: "{cell.size.y}", rx: "1" }
                }
            }
        }
    }
}

fn preview_cells(rows: u32, columns: u32) -> Vec<crate::mirrored_pair_geometry::PairPreviewCell> {
    // The creation seed's 19.05 mm pitch / 1 mm edge gap, without catalogue definitions,
    // identities or Parts. Core projects cell poses; the existing canvas adapter sizes them.
    let matrix = Matrix {
        id: "matrix-setup-paint".into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.05, y: 19.05 },
        origin: Vec2::default(),
        definition_id: String::new(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: Some(Vec2 { x: 1.0, y: 1.0 }),
        diode_direction: None,
        column_layout: vec![],
        row_layout: vec![],
        anchor: None,
        cells: vec![],
    };
    match CoreEngine::new().handle(CoreRequest::ProjectMatrices {
        id: "matrix-setup-paint".into(),
        base_revision: 0,
        matrices: vec![matrix.clone()],
    }) {
        CoreReply::MatrixProjections { matrix_scenes, .. } => matrix_scenes
            .first()
            .map(|scene| crate::mirrored_pair_geometry::preview_cells(scene, &matrix))
            .unwrap_or_default(),
        _ => vec![],
    }
}
