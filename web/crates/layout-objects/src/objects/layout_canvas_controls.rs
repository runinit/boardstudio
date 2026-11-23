//! Selection-attached measurements using the Inspector's typed edit route.
use super::LayoutTransformTool;
use super::matrix_transform::{MatrixTransformInspectorMount, number};
use crate::matrix_transform_operation::{
    MatrixTransformField as Field, MatrixTransformFields, column_angle,
};
use dioxus::prelude::*;

#[component]
pub fn LayoutCanvasControls(
    mount: MatrixTransformInspectorMount,
    #[props(default)] active_tool: Option<LayoutTransformTool>,
) -> Element {
    let error = use_signal(|| None::<String>);
    let Some((_, _, fields)) = mount.parts() else {
        return rsx! {};
    };
    let rows: Vec<(Field, &str, &str, f64)> = match &fields {
        MatrixTransformFields::Column {
            stagger,
            offset,
            splay_angle,
            rotation,
            splay_origin,
            ..
        } => {
            let mut rows = vec![
                (Field::ColumnOffsetX, "Offset X", "mm", offset.x),
                (Field::ColumnOffsetY, "Offset Y", "mm", offset.y),
                (Field::ColumnStagger, "Stagger", "mm", *stagger),
                (
                    Field::ColumnSplay,
                    "Splay",
                    "°",
                    column_angle(*splay_angle, *rotation, &(mount.splay_affect)()),
                ),
            ];
            if active_tool == Some(LayoutTransformTool::Origin) {
                rows.extend([
                    (Field::SplayOriginX, "Origin X", "mm", splay_origin.x),
                    (Field::SplayOriginY, "Origin Y", "mm", splay_origin.y),
                ]);
            }
            rows
        }
        MatrixTransformFields::Row { offset, .. } => vec![
            (Field::RowOffsetX, "Offset X", "mm", offset.x),
            (Field::RowOffsetY, "Offset Y", "mm", offset.y),
        ],
        MatrixTransformFields::Matrix { origin, .. } => vec![
            (Field::OriginX, "Origin X", "mm", origin.x),
            (Field::OriginY, "Origin Y", "mm", origin.y),
        ],
        MatrixTransformFields::Key { offset, .. } => vec![
            (Field::KeyOffsetX, "Offset X", "mm", offset.x),
            (Field::KeyOffsetY, "Offset Y", "mm", offset.y),
        ],
    };
    rsx! {
        div { class: "grid gap-1.5 text-xs text-foreground", role: "group", aria_label: "Selection geometry",
            onpointerdown: move |event| event.stop_propagation(), onkeydown: move |event| event.stop_propagation(),
            div { class: "box-border grid w-[276px] grid-cols-2 gap-2 rounded-md border border-toolbar-border bg-toolbar p-2 [&_input]:min-w-0 [&_input]:w-full [&_input]:font-mono [&_input]:text-xs [&_input]:tabular-nums",
                for (field, label, unit, value) in rows {
                    {number(&mount, field, label, unit, value, error)}
                }
            }
            if let Some(message) = error() {
                p { role: "alert", class: "text-xs text-destructive", "{message}" }
            }
        }
    }
}
