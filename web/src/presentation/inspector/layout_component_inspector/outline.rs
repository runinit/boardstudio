use super::*;

pub(super) fn outline_controls(
    projection: &LayoutComponentInspectorProjection,
    latest_capture: Signal<LayoutComponentInspectorOwner>,
    on_action: EventHandler<LayoutComponentInspectorAction>,
    mut outline_section_open: Signal<bool>,
    mut margin: Signal<String>,
    mut error: Signal<Option<String>>,
) -> Element {
    let outline = projection.outline.clone();
    let set_outline: Rc<dyn Fn(PartOutline)> = {
        let action = on_action;
        Rc::new(move |outline| {
            action.call(LayoutComponentInspectorAction::SetOutline {
                owner: latest_capture(),
                outline,
                submitted_draft: margin(),
            })
        })
    };
    rsx! {
        Collapsible {
            class: "[&_fieldset]:grid [&_fieldset]:min-w-0 [&_fieldset]:gap-[9px] [&_fieldset]:border-0 [&_fieldset]:px-0 [&_fieldset]:pt-0 [&_fieldset]:pb-3",
            keep_mounted: true,
            open: Some(outline_section_open()),
            on_open_change: move |next| outline_section_open.set(next),
            CollapsibleTrigger {
                class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold",
                aria_label: "Board outline",
                span { "Board outline" }
                small {
                    if projection.outline.excluded {
                        "Excluded"
                    } else {
                        "Included"
                    }
                }
                ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" }
            }
            CollapsibleContent {
                hidden: !outline_section_open(),
                inert: (!outline_section_open()).then_some(""),

                fieldset { disabled: projection.locked,
                    {
                        let check_id = format!(
                            "layout-check-{}-{}",
                            dioxus::core::current_scope_id().0,
                            ("Include in outline").replace(' ', "-"),
                        );
                        rsx! {
                            CheckboxField {
                                inline: true,
                                label: "Include in outline",
                                id: check_id,
                                aria_label: "Include in outline",
                                checked: if !projection.outline.excluded { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                on_checked_change: {
                                    let baseline = outline.clone();
                                    let submit = set_outline.clone();
                                    move |checked: CheckboxState| {
                                        let mut outline = baseline.clone();
                                        outline.excluded = !bool::from(checked);
                                        submit(outline);
                                    }
                                },
                                disabled: projection.locked,
                            }
                        }
                    }
                    if !projection.outline.excluded {
                        {
                            let check_id = format!(
                                "layout-check-{}-{}",
                                dioxus::core::current_scope_id().0,
                                ("Use board margin").replace(' ', "-"),
                            );
                            rsx! {
                                CheckboxField {
                                    inline: true,
                                    label: "Use board margin",
                                    id: check_id,
                                    aria_label: "Use board margin",
                                    checked: if projection.outline.margin.is_none() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                    on_checked_change: {
                                        let baseline = outline.clone();
                                        let submit = set_outline.clone();
                                        move |checked: CheckboxState| {
                                            let mut outline = baseline.clone();
                                            outline.margin = if bool::from(checked) { None } else { Some(0.0) };
                                            margin
                                                .set(outline.margin.map(|value| value.to_string()).unwrap_or_default());
                                            submit(outline);
                                        }
                                    },
                                    disabled: projection.locked,
                                }
                            }
                        }
                        if outline.margin.is_some() {
                            {
                                let control_input_id = format!(
                                    "layout-input-{}-8",
                                    dioxus::core::current_scope_id().0,
                                );
                                rsx! {
                                    NumericField {
                                        id: control_input_id,
                                        label: "Part edge margin",
                                        min: Some(0.0),
                                        value: outline.margin.unwrap_or_default(),
                                        on_commit: {
                                            let outline = outline.clone();
                                            let save_outline = set_outline.clone();
                                            move |value: f64| {
                                                let mut next = outline.clone();
                                                next.margin = Some(value);
                                                margin.set(value.to_string());
                                                error.set(None);
                                                save_outline(next);
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                    {
                        let check_id = format!(
                            "layout-check-{}-{}",
                            dioxus::core::current_scope_id().0,
                            ("Allow component body overhang").replace(' ', "-"),
                        );
                        rsx! {
                            CheckboxField {
                                inline: true,
                                label: "Allow component body overhang",
                                id: check_id,
                                aria_label: "Allow component body overhang",
                                checked: if projection.outline.allow_body_overhang { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                on_checked_change: {
                                    let baseline = outline.clone();
                                    let submit = set_outline.clone();
                                    move |checked: CheckboxState| {
                                        let mut outline = baseline.clone();
                                        outline.allow_body_overhang = bool::from(checked);
                                        submit(outline);
                                    }
                                },
                                disabled: projection.locked,
                            }
                        }
                    }
                    small {
                        "Pad and drill support remains required when a part is excluded or body overhang is allowed."
                    }
                }
            }
        }
    }
}
