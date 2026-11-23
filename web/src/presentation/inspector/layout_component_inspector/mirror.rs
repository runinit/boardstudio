use super::*;

pub(super) fn mirror_controls(
    projection: &LayoutComponentInspectorProjection,
    latest_capture: Signal<LayoutComponentInspectorOwner>,
    on_action: EventHandler<LayoutComponentInspectorAction>,
    mut mirror_section_open: Signal<bool>,
) -> Element {
    let owner = &projection.owner;
    let mirrored_half_action = |state| {
        let mirrored_half = projection.mirrored_half.clone();
        let owner = latest_capture;
        let action = on_action;
        move |_| {
            if let Some(mirrored_half) = mirrored_half.as_ref() {
                action.call(LayoutComponentInspectorAction::SetMirroredHalfState {
                    owner: owner(),
                    pair_id: mirrored_half.pair_id.clone(),
                    mirror_id: mirrored_half.mirror_id.clone(),
                    state,
                });
            }
        }
    };

    rsx! {
        if let Some(mirrored) = projection.part_mirror {
            InspectorSection {
                title: "Mirrored half",
                badge: if mirrored { "Both halves" } else { "Local" },
                open: mirror_section_open(),
                on_open_change: move |next| mirror_section_open.set(next),
                CheckboxField {
                    inline: true,
                    id: "component-inspector-part-mirror",
                    label: "Mirror on both halves",
                    aria_label: "Mirror on both halves",
                    checked: if mirrored { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: {
                        let action = on_action;
                        let part_id = owner.part_id.clone();
                        move |next| {
                            action
                                .call(LayoutComponentInspectorAction::SetPartMirrored {
                                    owner: latest_capture(),
                                    part_id: part_id.clone(),
                                    mirrored: next == CheckboxState::Checked,
                                })
                        }
                    },
                }
                p { "A mirrored part edits both halves together and stops at the split axis." }
            }
        } else if let Some(mirrored_half) = projection.mirrored_half.as_ref() {
            InspectorSection {
                title: "Mirrored half",
                badge: match mirrored_half.state {
                    HalfExceptionState::Linked => "Linked",
                    HalfExceptionState::Local => "Local",
                    HalfExceptionState::Omitted => "Omitted",
                },
                open: mirror_section_open(),
                on_open_change: move |next| mirror_section_open.set(next),
                div {
                    p { "This part's state on the mirrored half." }
                    Button {
                        variant: ButtonVariant::Secondary,
                        id: "component-inspector-mirrored-half-local",
                        button_type: "button",
                        disabled: mirrored_half.state == HalfExceptionState::Local,
                        onclick: mirrored_half_action(HalfExceptionState::Local),
                        "Make local"
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        id: "component-inspector-mirrored-half-omit",
                        button_type: "button",
                        disabled: mirrored_half.state == HalfExceptionState::Omitted,
                        onclick: mirrored_half_action(HalfExceptionState::Omitted),
                        "Omit"
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        id: "component-inspector-mirrored-half-rejoin",
                        button_type: "button",
                        disabled: mirrored_half.state == HalfExceptionState::Linked,
                        onclick: mirrored_half_action(HalfExceptionState::Linked),
                        "Rejoin"
                    }
                }
            }
        }
    }
}
