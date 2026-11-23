use super::*;

pub(super) fn manual_features(projection: &OutlineInspectorProjection) -> Element {
    let active_version = projection.active_version_id.clone();
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let mut perimeter_open = projection.editing_points;
    let mut selected_point = projection.selected_point;
    let mut selected_feature_id = projection.selected_feature_id;
    rsx! {
        if !projection.geometry_features.is_empty() {
            div {
                class: "grid gap-1.5 [&_h4]:mt-1.5 [&_h4]:mb-0 [&_h4]:text-xs",
                role: "group",
                aria_label: "Saved outline geometry",
                h4 { "Saved geometry" }
                for (index, feature) in projection.geometry_features.iter().enumerate() {
                    {
                        let feature_id = feature.id().to_owned();
                        let selected = selected_feature_id().as_deref() == Some(feature_id.as_str());
                        let feature_name = match feature {
                            OutlineFeature::Polygon { operation, .. } => {
                                match operation {
                                    Operation::Add => "Addition",
                                    Operation::Subtract => "Cutout",
                                }
                            }
                            OutlineFeature::Rect { operation, .. } => {
                                match operation {
                                    Operation::Add => "Rectangle addition",
                                    Operation::Subtract => "Rectangle cutout",
                                }
                            }
                            OutlineFeature::PartEnvelope { .. } => "Generated perimeter",
                        };
                        rsx! {
                            div { class: "flex items-center gap-1.5 [&>button:first-child]:flex-1 [&>button:first-child]:text-left", key: "outline-feature-{feature_id}",
                                ToggleGroup {
                                    aria_label: "Geometry selection",
                                    pressed: Some(
                                        if selected {
                                            std::collections::HashSet::from([0usize])
                                        } else {
                                            std::collections::HashSet::new()
                                        },
                                    ),
                                    on_pressed_change: {
                                        let feature_id = feature_id.clone();
                                        move |_| selected_feature_id.set(Some(feature_id.clone()))
                                    },
                                    ToggleItem { index: 0usize, disabled: !enabled, "{feature_name} {index + 1}" }
                                }
                                if matches!(feature, OutlineFeature::Polygon { .. }) {
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    onclick: {
                                    let feature_id = feature_id.clone();
                                    move |_| {
                                        selected_feature_id.set(Some(feature_id.clone()));
                                        selected_point.set(0);
                                        perimeter_open.set(true);
                                    }
                                },

                                        "Edit points"

                                     }
                                }
                                if let Some(version_id) = active_version.as_ref().filter(|_| index > 0) {
                                    Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    disabled: !enabled,
    aria_label: "Remove {feature_name} {index + 1}",
    onclick: {
                                    let action_context = action_context.clone();
                                    let version_id = version_id.clone();
                                    let feature_id = feature_id.clone();
                                    move |_| on_action.call(action_context.remove_feature(version_id.clone(), feature_id.clone()))
                                },

                                        "Remove"

                                     }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(feature) = projection
            .geometry_features
            .iter()
            .find(|feature| Some(feature.id()) == selected_feature_id().as_deref())
        {
            if let OutlineFeature::Rect { center, size, anchor_part_id, .. } = feature {
                fieldset {
                    class: "m-0 grid min-w-0 gap-2 border border-border p-2 [&_legend]:px-1 [&_legend]:text-xs [&_legend]:text-muted-foreground [&_h4]:mt-1.5 [&_h4]:mb-0 [&_h4]:text-xs",
                    disabled: !enabled,
                    legend { "Selected rectangle" }
                    {
                        let anchor = anchor_part_id
                            .as_deref()
                            .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                            .map(|part| PerimeterAnchor {
                                at: part.pose.at,
                                rotation: part.pose.rotation,
                                back: part.side == Side::Back,
                            });
                        let world_center = anchor.map_or(*center, |anchor| anchor.world(*center));
                        let action_context = action_context.clone();
                        let before = feature.clone();
                        let version_id = active_version.clone();
                        vec2_fields("Rectangle center".to_owned(), world_center, enabled, move |world| {
                            let mut after = before.clone();
                            if let OutlineFeature::Rect { center, .. } = &mut after {
                                *center = anchor.map_or(world, |anchor| anchor.local(world));
                            }
                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                        })
                    }
                    OutlineDimension {
                        label: "Rectangle width",
                        value: size.x,
                        minimum: 0.001,
                        editable: enabled,
                        on_commit: commit_feature_edit(
                            action_context.clone(),
                            on_action,
                            feature.clone(),
                            active_version.clone(),
                            |after, value| {
                                if let OutlineFeature::Rect { size, .. } = after {
                                    size.x = value;
                                }
                            },
                        ),
                    }
                    OutlineDimension {
                        label: "Rectangle height",
                        value: size.y,
                        minimum: 0.001,
                        editable: enabled,
                        on_commit: commit_feature_edit(
                            action_context.clone(),
                            on_action,
                            feature.clone(),
                            active_version.clone(),
                            |after, value| {
                                if let OutlineFeature::Rect { size, .. } = after {
                                    size.y = value;
                                }
                            },
                        ),
                    }
                    if let OutlineFeature::Rect { radius, .. } = feature {
                        OutlineDimension {
                            label: "Rectangle corner radius",
                            value: *radius,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: commit_feature_edit(
                                action_context.clone(),
                                on_action,
                                feature.clone(),
                                active_version.clone(),
                                |after, value| {
                                    if let OutlineFeature::Rect { radius, .. } = after {
                                        *radius = value;
                                    }
                                },
                            ),
                        }
                    }
                }
            }
        }
    }
}
