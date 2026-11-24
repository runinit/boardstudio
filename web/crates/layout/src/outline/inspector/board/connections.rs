use super::*;
use boardstudio_web_ui_shared::inspector_fields::SelectField;

fn set_connection_feature(
    projection: &OutlineInspectorProjection,
    version_id: Option<String>,
    before: OutlineFeature,
    after: OutlineFeature,
) {
    projection.dispatch(OutlineActionKind::SetFeature {
        version_id,
        before,
        after,
    });
}

pub(super) fn connection_controls(view: &BoardInspectorView) -> Element {
    let projection = &view.projection;
    let point_attachment = view.point_attachment;
    let active_version = projection.active_version_id.clone();
    let enabled = projection.enabled;
    let mut canvas_ui = projection.ui.canvas_ui;
    let selected_conn = canvas_ui.read().selected_connection_id.clone();
    let selected_pt = canvas_ui.read().selected_point;
    rsx! {
        if !projection.connections.is_empty() {
            div {
                class: "grid gap-1.5 [&_h4]:mt-1.5 [&_h4]:mb-0 [&_h4]:text-xs",
                role: "group",
                aria_label: "Outline connections",
                h4 { "Connections" }
                for (index, connection) in projection.connections.iter().enumerate() {
                    div {
                        class: "flex items-center gap-1.5 [&>button:first-child]:flex-1 [&>button:first-child]:text-left",
                        key: "outline-connection-{connection.id}",
                        ToggleGroup {
                            aria_label: "Geometry selection",
                            pressed: Some(
                                if selected_conn.as_deref() == Some(connection.id.as_str()) {
                                    std::collections::HashSet::from([0usize])
                                } else {
                                    std::collections::HashSet::new()
                                },
                            ),
                            on_pressed_change: {
                                let id = connection.id.clone();
                                move |_| canvas_ui.write().selected_connection_id = Some(id.clone())
                            },
                            ToggleItem { index: 0usize,
                                "Connection {index + 1} · {connection.points.len()} points · {connection.width:.2} mm"
                            }
                        }
                        if let Some(before) = projection.connection_feature.as_ref() {
                            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    aria_label: "Remove connection {index + 1}",
    onclick: {
                            let projection = projection.clone();
                            let before = before.clone();
                            let version_id = active_version.clone();
                            let connection_id = connection.id.clone();
                            move |_| {
                                let mut after = before.clone();
                                if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                    connections.retain(|item| item.id != connection_id);
                                }
                                set_connection_feature(&projection, version_id.clone(), before.clone(), after);
                                canvas_ui.write().selected_connection_id = None;
                            }
                        },

                                "Remove"

                             }
                        }
                    }
                }
            }
        }
        if let (Some(connection_id), Some(before)) = (
            selected_conn.as_ref(),
            projection.connection_feature.as_ref(),
        )
        {
            if let OutlineFeature::PartEnvelope { connections, .. } = before {
                if let Some(connection) = connections
                    .iter()
                    .find(|connection| &connection.id == connection_id)
                {
                    OutlineDimension {
                        label: "Connection width",
                        value: connection.width,
                        minimum: 0.001,
                        editable: enabled,
                        on_commit: {
                            let projection = projection.clone();
                            let before = before.clone();
                            let version_id = active_version.clone();
                            let connection_id = connection_id.clone();
                            move |value| {
                                let mut after = before.clone();
                                if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                    && let Some(connection) = connections
                                        .iter_mut()
                                        .find(|connection| connection.id == connection_id)
                                {
                                    connection.width = value;
                                }
                                set_connection_feature(&projection, version_id.clone(), before.clone(), after);
                            }
                        },
                    }
                    if !connection.points.is_empty() {
                        {
                            let index = selected_pt.min(connection.points.len() - 1);
                            let point = &connection.points[index];
                            let world = connection_point_world(point, &projection.outline_parts);
                            rsx! {
                                h4 { "Connection point {index + 1} of {connection.points.len()}" }
                                {
                                    let projection = projection.clone();
                                    let before = before.clone();
                                    let version_id = active_version.clone();
                                    let connection_id = connection.id.clone();
                                    let parts = projection.outline_parts.clone();
                                    vec2_fields(format!("Point {}", index + 1), world, enabled, move |world| {
                                        let next = move_connection_point(&before, &connection_id, index, world, &parts);
                                        set_connection_feature(&projection, version_id.clone(), before.clone(), next);
                                    })
                                }
                                SelectField {
                                    label: "Point attachment".to_owned(),
                                    value: point_attachment().unwrap_or_default(),
                                    options: {
                                        let mut attachment = vec![(
                                            "".to_owned(),
                                            "Fixed on board".to_owned(),
                                        )];
                                        if let Some(id) = point.part_id.as_ref().filter(|id| {
                                            !projection
                                                .outline_parts
                                                .iter()
                                                .any(|part| &part.id == *id)
                                        }) {
                                            attachment.push((
                                                id.to_string(),
                                                "Missing component".to_owned(),
                                            ));
                                        }
                                        attachment.extend(
                                            projection.outline_parts.iter().map(|part| {
                                                (part.id.clone(), part.reference.clone())
                                            }),
                                        );
                                        attachment
                                    },
                                    aria_label: format!("Point {} attachment", index + 1),
                                    disabled: !enabled,
                                        on_commit: {
                                            let projection = projection.clone();
                                            let before = before.clone();
                                            let version_id = active_version.clone();
                                            let connection_id = connection.id.clone();
                                            let parts = projection.outline_parts.clone();
                                            move |value: String| {
                                                let next = attach_connection_point(
                                                    &before,
                                                    &connection_id,
                                                    index,
                                                    (!value.is_empty()).then_some(value.as_str()),
                                                    &parts,
                                                );
                                                set_connection_feature(&projection, version_id.clone(), before.clone(), next);
                                            }
                                        },
                                }
                                div { class: "my-3 flex flex-wrap gap-2",
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled || index + 1 == connection.points.len(),
    aria_label: "Insert after connection point {index + 1}",
    onclick: {
                                    let projection = projection.clone();
                                    let before = before.clone();
                                    let version_id = active_version.clone();
                                    let connection_id = connection.id.clone();
                                    let parts = projection.outline_parts.clone();
                                    move |_| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                            && index + 1 < connection.points.len()
                                        {
                                            let a = connection_point_world(&connection.points[index], &parts);
                                            let b = connection_point_world(&connection.points[index + 1], &parts);
                                            connection.points.insert(index + 1, OutlineControlPoint {
                                                at: Vec2 { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 },
                                                part_id: None,
                                            });
                                            canvas_ui.write().selected_point = index + 1;
                                            set_connection_feature(&projection, version_id.clone(), before.clone(), after);
                                        }
                                    }
                                },

                                        "Insert after"

                                     }
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled || connection.points.len() <= 2,
    aria_label: "Remove connection point {index + 1}",
    onclick: {
                                    let projection = projection.clone();
                                    let before = before.clone();
                                    let version_id = active_version.clone();
                                    let connection_id = connection.id.clone();
                                    move |_| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                            && connection.points.len() > 2 && index < connection.points.len()
                                        {
                                            connection.points.remove(index);
                                            canvas_ui.write().selected_point = index.saturating_sub(1);
                                            set_connection_feature(&projection, version_id.clone(), before.clone(), after);
                                        }
                                    }
                                },

                                        "Remove point"

                                     }
                                }
                                div {
                                    class: "max-h-[220px] overflow-auto border border-border",
                                    role: "group",
                                    aria_label: "Connection points",
                                    for (index, control) in connection.points.iter().enumerate() {
                                        {
                                            let point = connection_point_world(control, &projection.outline_parts);
                                            rsx! {
                                                ToggleGroup {
                                                    aria_label: "Select connection point {index + 1}",
                                                    pressed: Some(
                                                        if index == selected_pt {
                                                            std::collections::HashSet::from([0usize])
                                                        } else {
                                                            std::collections::HashSet::new()
                                                        },
                                                    ),
                                                    on_pressed_change: move |_| canvas_ui.write().selected_point = index,
                                                    ToggleItem {
                                                        index: 0usize,
                                                        key: "connection-point-{index}",
                                                        aria_label: "Select connection point {index + 1}",
                                                        span { "{index + 1}" }
                                                        span { "{point.x:.3}" }
                                                        span { "{point.y:.3}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
