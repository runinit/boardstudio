use super::*;

pub(super) fn drawing_controls(view: &BoardInspectorView) -> Element {
    let projection = &view.projection;
    let draft_serial = view.draft_serial;
    let mut perimeter_open = projection.editing_points;
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let mut drawing_operation = projection.drawing_operation;
    let mut drawing_points = projection.drawing_points;
    let mut draft_serial = draft_serial;
    rsx! {
        if let Some(tool) = drawing_operation() {
            div { role: "status",  Callout {  variant: CalloutVariant::Info,
                "Click the canvas to add points. Enter finishes; Escape cancels."
             } }
            p { "{drawing_points.read().len()} points" }
            div { class: "mt-3 flex flex-wrap gap-2 [&_button]:min-h-9",
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: drawing_points.read().is_empty(),
    onclick: move |_| { let mut points = drawing_points.read().clone(); points.pop(); drawing_points.set(points); },

                    "Undo point"

                 }
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    onclick: move |_| { drawing_operation.set(None); drawing_points.set(Vec::new()); },
                    "Cancel drawing"
                 }
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled
                        || drawing_points.read().len()
                            < if tool == OutlineDrawTool::Connect { 2 } else { 3 }
                        || matches!(tool, OutlineDrawTool::Polygon(_))
                            && polygon_signed_area(&drawing_points.read()).abs() < 1e-6,
    onclick: {
                let action_context = action_context.clone();
                let board_id = projection.board_id.clone();
                let revision = projection.revision;
                move |_| {
                    let minimum_points = if tool == OutlineDrawTool::Connect { 2 } else { 3 };
                    let points = drawing_points.read().clone();
                    if points.len() < minimum_points { return; }
                    match tool {
                        OutlineDrawTool::Polygon(operation) => {
                            if polygon_signed_area(&points).abs() < 1e-6 { return; }
                            let serial = draft_serial() + 1;
                            draft_serial.set(serial);
                            let feature = OutlineFeature::Polygon {
                                id: format!("outline-manual-{board_id}-{}-{serial}", revision),
                                points,
                                anchor_part_id: None,
                                operation,
                            };
                            on_action.call(action_context.add_feature(feature));
                        }
                        OutlineDrawTool::Connect => on_action.call(action_context.add_connection(points)),
                    }
                    drawing_operation.set(None);
                    drawing_points.set(Vec::new());
                }
            },

                    "Finish drawing"

                 }
            }
        } else {
            div { class: "mt-3 flex flex-wrap gap-2 [&_button]:min-h-9",
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Add))); },

                    "Draw addition"

                 }
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Subtract))); },

                    "Draw cutout"

                 }
                Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled || projection.active_version_id.is_some()
                        || !projection.has_generated,
    onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Connect)); },

                    "Connect points"

                 }
            }
        }
    }
}
