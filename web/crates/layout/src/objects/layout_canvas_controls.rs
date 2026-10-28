//! Selection-attached measurements using the Inspector's typed edit route.
use super::matrix_transform_controller::MatrixTransformInspectorMount;
use super::matrix_transform_inspector::NumericTransformField;
use super::{LayoutTransformMenuMount, LayoutTransformTool};
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, column_angle,
};
use dioxus::prelude::*;

#[component]
pub fn LayoutCanvasControls(
    mount: MatrixTransformInspectorMount,
    tools: LayoutTransformMenuMount,
) -> Element {
    let mut mounted = mount.canvas_mounted;
    let visible = mount.projection.is_some();
    use_effect(use_reactive((&visible,), move |(visible,)| {
        mounted.set(visible)
    }));
    use_drop(move || mounted.set(false));
    let Some(projection) = mount.projection.as_ref() else {
        return rsx! {};
    };
    let fields = match &projection.fields {
        MatrixTransformFields::Column {
            stagger,
            offset,
            splay_angle,
            rotation,
            splay_origin,
            ..
        } => {
            let mut fields = vec![
                (
                    MatrixTransformField::ColumnOffsetX,
                    "Offset X",
                    "mm",
                    offset.x,
                ),
                (
                    MatrixTransformField::ColumnOffsetY,
                    "Offset Y",
                    "mm",
                    offset.y,
                ),
                (
                    MatrixTransformField::ColumnStagger,
                    "Stagger",
                    "mm",
                    *stagger,
                ),
                (
                    MatrixTransformField::ColumnSplay,
                    "Splay",
                    "°",
                    column_angle(*splay_angle, *rotation, &(mount.splay_affect)()),
                ),
            ];
            if tools.active_tool == Some(LayoutTransformTool::Origin) {
                fields.extend([
                    (
                        MatrixTransformField::SplayOriginX,
                        "Origin X",
                        "mm",
                        splay_origin.x,
                    ),
                    (
                        MatrixTransformField::SplayOriginY,
                        "Origin Y",
                        "mm",
                        splay_origin.y,
                    ),
                ]);
            }
            fields
        }
        MatrixTransformFields::Row { offset, .. } => vec![
            (MatrixTransformField::RowOffsetX, "Offset X", "mm", offset.x),
            (MatrixTransformField::RowOffsetY, "Offset Y", "mm", offset.y),
        ],
        MatrixTransformFields::Matrix { origin, .. } => vec![
            (MatrixTransformField::OriginX, "Origin X", "mm", origin.x),
            (MatrixTransformField::OriginY, "Origin Y", "mm", origin.y),
        ],
        MatrixTransformFields::Key { offset, .. } => vec![
            (MatrixTransformField::KeyOffsetX, "Offset X", "mm", offset.x),
            (MatrixTransformField::KeyOffsetY, "Offset Y", "mm", offset.y),
        ],
    };
    let owner_key = format!("{:?}", projection.owner);
    rsx! {
        div {
            class: "m1-canvas-measurements",
            role: "group", aria_label: "Selection geometry",
            onpointerdown: move |event| event.stop_propagation(),
            onkeydown: move |event| event.stop_propagation(),
            div { class: "m1-canvas-dimensions",
                for (field, label, unit, value) in fields {
                    CanvasDimension {
                        key: "{owner_key}-{field:?}", mount: mount.clone(), field, label, unit, value,
                    }
                }
            }
        }
    }
}

// Separate drafts let typing on the canvas coexist with an unfinished Inspector edit.
// Requests still share the controller's sequence, owner checks and PendingEdits.
#[component]
fn CanvasDimension(
    mount: MatrixTransformInspectorMount,
    field: MatrixTransformField,
    label: &'static str,
    unit: &'static str,
    value: f64,
) -> Element {
    let Some((_, draft, failure)) = mount
        .canvas_numeric_fields
        .iter()
        .find(|(candidate, _, _)| *candidate == field)
    else {
        return rsx! {};
    };
    let mut draft = *draft;
    let mut failure = *failure;
    use_hook(move || {
        draft.set(value.to_string());
        failure.set(None);
    });
    let Some(projection) = mount.projection.as_ref() else {
        return rsx! {};
    };
    rsx! {
        NumericTransformField {
            owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
            revision: projection.revision, field, label, unit, value,
            display_precision: Some(4),
            request_sequence: mount.request_sequence, editable: mount.editable,
            feedback: mount.feedback.clone(), on_edit: mount.on_edit,
            splay_affect: mount.splay_affect, draft, failure,
        }
    }
}
