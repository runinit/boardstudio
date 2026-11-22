//! Selection-attached measurements using the Inspector's typed edit route.
use super::matrix_transform::{MatrixTransformInspectorMount, number};
use crate::matrix_transform_operation::{MatrixTransformField as Field, MatrixTransformFields};
use dioxus::prelude::*;

#[component]
pub fn LayoutCanvasControls(mount: MatrixTransformInspectorMount) -> Element {
    let error = use_signal(|| None::<String>);
    let Some((_, _, fields)) = mount.parts() else {
        return rsx! {};
    };
    let rows: Vec<(Field, &str, f64)> = match &fields {
        MatrixTransformFields::Column { stagger, offset, .. } => vec![
            (Field::ColumnOffsetX, "Offset X", offset.x),
            (Field::ColumnOffsetY, "Offset Y", offset.y),
            (Field::ColumnStagger, "Stagger", *stagger),
        ],
        MatrixTransformFields::Row { offset, .. } => vec![(Field::RowOffsetX, "Offset X", offset.x), (Field::RowOffsetY, "Offset Y", offset.y)],
        MatrixTransformFields::Matrix { origin, .. } => vec![(Field::OriginX, "Origin X", origin.x), (Field::OriginY, "Origin Y", origin.y)],
        MatrixTransformFields::Key { offset, .. } => vec![(Field::KeyOffsetX, "Offset X", offset.x), (Field::KeyOffsetY, "Offset Y", offset.y)],
    };
    rsx! {
        div { class: "grid gap-1.5 text-xs text-foreground", role: "group", aria_label: "Selection geometry",
            onpointerdown: move |event| event.stop_propagation(), onkeydown: move |event| event.stop_propagation(),
            div { class: "box-border grid w-[276px] grid-cols-2 gap-2 rounded-md border border-toolbar-border bg-toolbar p-2 [&_input]:min-w-0 [&_input]:w-full [&_input]:font-mono [&_input]:text-xs [&_input]:tabular-nums",
                for (field, label, value) in rows {
                    {number(&mount, field, label, "mm", value, error)}
                }
            }
        }
    }
}
