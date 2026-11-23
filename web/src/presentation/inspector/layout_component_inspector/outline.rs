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
        InspectorSection {
            title: "Board outline",
            badge: if projection.outline.excluded { "Excluded" } else { "Included" },
            open: outline_section_open(),
            on_open_change: move |next| outline_section_open.set(next),
            fieldset { class: "grid min-w-0 gap-[9px] border-0 px-0 pt-0 pb-3", disabled: projection.locked,
                CheckboxField {
                    inline: true,
                    label: "Include in outline",
                    id: "layout-outline-include",
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
                if !projection.outline.excluded {
                    CheckboxField {
                        inline: true,
                        label: "Use board margin",
                        id: "layout-outline-board-margin",
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
                    if outline.margin.is_some() {
                        NumericField {
                            id: "layout-outline-margin",
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
                CheckboxField {
                    inline: true,
                    label: "Allow component body overhang",
                    id: "layout-outline-overhang",
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
                small {
                    "Pad and drill support remains required when a part is excluded or body overhang is allowed."
                }
            }
        }
    }
}
