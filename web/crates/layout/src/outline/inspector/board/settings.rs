use super::*;
use boardstudio_web_ui_shared::inspector_fields::SelectField;

pub(super) fn outline_settings(
    view: &BoardInspectorView,
    active_ownership: OutlineVersionOwnership,
) -> Element {
    let projection = &view.projection;
    let corner_selection = view.corner_selection;
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    // Margin, bridge width and cleanup belong to an envelope, which Generated and a linked
    // refinement have and an edited (fixed) copy does not.
    let envelope_editable = active_ownership != OutlineVersionOwnership::Edited;
    let size_label: &'static str = match projection.settings.corners {
        CornerStyle::Fillet => "Fillet radius",
        _ => "Chamfer size",
    };
    rsx! {
        if projection.has_generated || projection.active_version_id.is_some() {
            {
                let id = format!("layout-choice-{}-Outline-corners", dioxus::core::current_scope_id().0);
                rsx! {
                    SelectField::<String> {
                        id, label: "Corners", aria_label: "Outline corners",
                        value: corner_selection().unwrap_or_default(), disabled: !enabled,
                        options: vec![("sharp".into(), "Sharp".into()), ("fillet".into(), "Fillet".into()), ("chamfer".into(), "Chamfer".into())],
                        on_commit: {
                            let action_context = action_context.clone();
                            move |choice: String| {
                                let corner = match choice.as_str() {
                                    "fillet" => CornerStyle::Fillet,
                                    "chamfer" => CornerStyle::Chamfer,
                                    _ => CornerStyle::Sharp,
                                };
                                on_action.call(action_context.action(OutlineEdit::SetCorners(corner)));
                            }
                        },
                    }
                }
            }
            if projection.settings.corners != CornerStyle::Sharp {
                OutlineDimension {
                    label: size_label,
                    value: projection.settings.size,
                    minimum: 0.0,
                    editable: enabled,
                    on_commit: {
                        let action_context = action_context.clone();
                        move |value| on_action.call(action_context.action(OutlineEdit::SetSize(value)))
                    },
                }
            }
            if envelope_editable {
                if let Some(margin) = projection.envelope_margin {
                    OutlineDimension {
                        label: "Outline margin",
                        value: margin,
                        minimum: 0.0,
                        editable: enabled,
                        on_commit: {
                            let action_context = action_context.clone();
                            move |value| on_action.call(action_context.action(OutlineEdit::SetMargin(value)))
                        },
                    }
                }
                OutlineDimension {
                    label: "Bridge width",
                    value: projection.settings.bridge_width,
                    minimum: 0.001,
                    editable: enabled,
                    on_commit: {
                        let action_context = action_context.clone();
                        move |value| {
                            on_action.call(action_context.action(OutlineEdit::SetBridgeWidth(value)))
                        }
                    },
                }
            }
            if projection.active_version_id.is_none() && !projection.gaps.is_empty() {
                fieldset {
                    class: "m-0 grid min-w-0 gap-[9px] border border-border p-2.5 text-foreground [&_legend]:px-1 [&_legend]:font-semibold [&>p]:m-0 [&>p]:text-xs [&>p]:text-muted-foreground",
                    disabled: !enabled,
                    legend { "Gap repair" }
                    p {
                        "Keep gap preserves an intentional recess and follows its source components."
                    }
                    for (index, gap) in projection.gaps.iter().enumerate() {
                        div { class: "flex items-center justify-between gap-2",
                            Button { variant: ButtonVariant::Secondary,
    class: "flex-1 text-left",
    button_type: "button",
    aria_label: "Show gap {index + 1}",
    disabled: !enabled,
    onclick: {
                            let action_context = action_context.clone();
                            let gap_id = gap.id.clone();
                            move |_| on_action.call(action_context.focus_gap(gap_id.clone()))
                        },

                                "Gap {index + 1} · {gap.span:.1} mm span"

                             }
                            {
                                let check_id = format!(
                                    "layout-check-{}-{}",
                                    dioxus::core::current_scope_id().0,
                                    format!("Keep gap {}", index + 1).replace(' ', "-"),
                                );
                                rsx! {
                                    Label { html_for: check_id.clone(), class: "grid grid-cols-[auto_minmax(0,1fr)] items-center gap-[5px] text-xs text-foreground",
                                        Checkbox {
                                            id: check_id.clone(),
                                            aria_label: "Keep gap {index + 1}",
                                            checked: Some(
                                                if gap.protected {
                                                    CheckboxState::Checked
                                                } else {
                                                    CheckboxState::Unchecked
                                                },
                                            ),
                                            on_checked_change: {
                                                let gap_id = gap.id.clone();
                                                let action_context = action_context.clone();
                                                move |checked: CheckboxState| {
                                                    on_action
                                                        .call(
                                                            action_context
                                                                .action(OutlineEdit::SetProtectedGap {
                                                                    gap_id: gap_id.clone(),
                                                                    protected: bool::from(checked),
                                                                }),
                                                        )
                                                }
                                            },
                                            disabled: !enabled,
                                        }
                                        span { "Keep gap" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            fieldset { class: "m-0 grid min-w-0 gap-[9px] border border-border p-2.5 text-foreground [&_legend]:px-1 [&_legend]:font-semibold [&>p]:m-0 [&>p]:text-xs [&>p]:text-muted-foreground", disabled: !enabled,
                legend { "Advanced cleanup and clearance" }
                if envelope_editable {
                    {
                        let check_id = format!(
                            "layout-check-{}-{}",
                            dioxus::core::current_scope_id().0,
                            ("Automatic gap cleanup").replace(' ', "-"),
                        );
                        rsx! {
                            Label { html_for: check_id.clone(), class: "grid grid-cols-[auto_minmax(0,1fr)] items-center gap-[5px] text-xs text-foreground",
                                Checkbox {
                                    id: check_id.clone(),
                                    aria_label: "Automatic gap cleanup",
                                    checked: Some(
                                        if projection.repair.enabled {
                                            CheckboxState::Checked
                                        } else {
                                            CheckboxState::Unchecked
                                        },
                                    ),
                                    on_checked_change: {
                                        let action_context = action_context.clone();
                                        move |checked: CheckboxState| {
                                            on_action
                                                .call(
                                                    action_context
                                                        .action(OutlineEdit::SetRepairEnabled(bool::from(checked))),
                                                )
                                        }
                                    },
                                    disabled: !enabled,
                                }
                                span { "Automatic cleanup" }
                            }
                        }
                    }
                    OutlineDimension {
                        label: "Maximum gap span",
                        value: projection.repair.maximum_gap_span,
                        minimum: 0.0,
                        editable: enabled,
                        on_commit: {
                            let action_context = action_context.clone();
                            move |value| {
                                on_action.call(action_context.action(OutlineEdit::SetMaximumGapSpan(value)))
                            }
                        },
                    }
                }
                OutlineDimension {
                    label: "Minimum connection width",
                    value: projection.repair.minimum_connection_width,
                    minimum: 0.0,
                    editable: enabled,
                    on_commit: {
                        let action_context = action_context.clone();
                        move |value| {
                            on_action
                                .call(
                                    action_context.action(OutlineEdit::SetMinimumConnectionWidth(value)),
                                )
                        }
                    },
                }
                OutlineDimension {
                    label: "PCB edge clearance",
                    value: projection.repair.edge_clearance,
                    minimum: 0.0,
                    editable: enabled,
                    on_commit: {
                        let action_context = action_context.clone();
                        move |value| {
                            on_action.call(action_context.action(OutlineEdit::SetEdgeClearance(value)))
                        }
                    },
                }
                p {
                    "Support and clearance findings block affected fabrication exports. Editing and project saving stay available."
                }
                if projection.active_version_id.is_none() {
                    for (index, gap) in projection.repair.keep_gaps.iter().enumerate() {
                        Button { variant: ButtonVariant::Secondary,
    class: "min-h-8 justify-self-start",
    aria_label: "Remove protected gap {index + 1}",
    disabled: !enabled,
    onclick: {
                        let gap_id = gap.id.clone();
                        let action_context = action_context.clone();
                        move |_| on_action.call(action_context.action(OutlineEdit::RemoveProtectedGap { gap_id: gap_id.clone() }))
                    },

                            "Remove protected gap {index + 1}"

                         }
                    }
                }
            }
        }
    }
}
