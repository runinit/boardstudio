use super::*;

pub(super) fn connection_controls(view: &BoardInspectorView) -> Element {
    let projection = &view.projection;
    let point_attachment = view.point_attachment;
    let active_version = projection.active_version_id.clone();
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let mut selected_point = projection.selected_point;
    let mut selected_connection_id = projection.selected_connection_id;
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
                                    if selected_connection_id().as_deref() == Some(connection.id.as_str()) {
                                        std::collections::HashSet::from([0usize])
                                    } else {
                                        std::collections::HashSet::new()
                                    },
                                ),
                                on_pressed_change: {
                                    let id = connection.id.clone();
                                    move |_| selected_connection_id.set(Some(id.clone()))
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
                                let action_context = action_context.clone();
                                let before = before.clone();
                                let version_id = active_version.clone();
                                let connection_id = connection.id.clone();
                                move |_| {
                                    let mut after = before.clone();
                                    if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                        connections.retain(|item| item.id != connection_id);
                                    }
                                    on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    selected_connection_id.set(None);
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
                selected_connection_id().as_ref(),
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
                                let action_context = action_context.clone();
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
                                    on_action
                                        .call(
                                            action_context.set_feature(version_id.clone(), before.clone(), after),
                                        );
                                }
                            },
                        }
                        if !connection.points.is_empty() {
                            {
                                let index = selected_point().min(connection.points.len() - 1);
                                let point = &connection.points[index];
                                let world = connection_point_world(point, &projection.outline_parts);
                                rsx! {
                                    h4 { "Connection point {index + 1} of {connection.points.len()}" }
                                    {
                                        let action_context = action_context.clone();
                                        let before = before.clone();
                                        let version_id = active_version.clone();
                                        let connection_id = connection.id.clone();
                                        let parts = projection.outline_parts.clone();
                                        vec2_fields(format!("Point {}", index + 1), world, enabled, move |world| {
                                            let next = move_connection_point(&before, &connection_id, index, world, &parts);
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                        })
                                    }
                                    {
                                        let control_label = "Point attachment".to_owned();
                                        let control_aria_label = format!("Point {} attachment", index + 1);
                                        let control_id = format!(
                                            "layout-choice-{}-{}",
                                            dioxus::core::current_scope_id().0,
                                            control_aria_label.replace(' ', "-"),
                                        );

                                        let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                        control_choices.push(("".to_owned(), "Fixed on board".to_owned(), false));
                                        if let Some(id) = point
                                            .part_id
                                            .as_ref()
                                            .filter(|id| !projection.outline_parts.iter().any(|part| &part.id == *id))
                                        {
                                            control_choices
                                                .push((id.to_string(), "Missing component".to_owned(), false));
                                        }
                                        for part in projection.outline_parts.iter() {
                                            control_choices
                                                .push((part.id.to_string(), part.reference.to_string(), false));
                                        }
                                        let on_value_change = EventHandler::new({
                    let action_context = action_context.clone();
                    let before = before.clone();
                    let version_id = active_version.clone();
                    let connection_id = connection.id.clone();
                    let parts = projection.outline_parts.clone();
                    move |choice: Option<String>| {
                        let Some(choice) = choice else {
    return;
                        };
                        let value = choice;
                        let next = attach_connection_point(
    &before,
    &connection_id,
    index,
    (!value.is_empty()).then_some(value.as_str()),
    &parts,
                        );
                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                    }
                });
                rsx! {

                                            Field { class: "grid gap-[5px] text-xs text-muted-foreground",
                                                FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                                Combobox::<String> {
                                                    id: control_id,
                                                    aria_label: control_aria_label.clone(),
                                                    list_aria_label: control_aria_label.clone(),
                                                    value: Some(point_attachment.into()),
                                                    disabled: !enabled,
                                                    on_value_change,
                                                    for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                        ComboboxOption::<String> {
                                                            index,
                                                            value: value.clone(),
                                                            text_value: label.clone(),
                                                            disabled: *disabled,
                                                            "{label}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    div { class: "my-3 flex flex-wrap gap-2",
                                        Button { variant: ButtonVariant::Secondary,
        button_type: "button",
        disabled: !enabled || index + 1 == connection.points.len(),
        aria_label: "Insert after connection point {index + 1}",
        onclick: {
                                        let action_context = action_context.clone();
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
                                                selected_point.set(index + 1);
                                                on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
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
                                        let action_context = action_context.clone();
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
                                                selected_point.set(index.saturating_sub(1));
                                                on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
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
                                                            if index == selected_point() {
                                                                std::collections::HashSet::from([0usize])
                                                            } else {
                                                                std::collections::HashSet::new()
                                                            },
                                                        ),
                                                        on_pressed_change: move |_| selected_point.set(index),
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
