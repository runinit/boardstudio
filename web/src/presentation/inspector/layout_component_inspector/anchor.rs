use super::*;

pub(super) fn use_anchor_controls(
    projection: &LayoutComponentInspectorProjection,
    latest_capture: Signal<LayoutComponentInspectorOwner>,
    on_action: EventHandler<LayoutComponentInspectorAction>,
    error: Signal<Option<String>>,
) -> Element {
    let mut pick_matrix = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.matrix_id.clone())
            .or_else(|| {
                projection
                    .anchor_choices
                    .first()
                    .map(|choice| choice.matrix_id.clone())
            })
            .unwrap_or_default()
    });
    let mut pick_row = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| (anchor.row + 1).to_string())
            .unwrap_or_else(|| "1".to_owned())
    });
    let mut pick_column = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| (anchor.column + 1).to_string())
            .unwrap_or_else(|| "1".to_owned())
    });
    let mut pick_follow_x = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.x)
            .unwrap_or(true)
    });
    let mut pick_follow_y = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.y)
            .unwrap_or(true)
    });
    let mut pick_follow_rotation = use_signal(|| {
        projection
            .anchor
            .as_ref()
            .map(|anchor| anchor.follow.rotation)
            .unwrap_or(true)
    });
    // Ticket 13: column-vocabulary commits. Number fields parse on Enter or
    // blur; Follow, Pick and Detach submit immediately.
    let commit_anchor_value: Rc<dyn Fn(PartAnchorValueChange)> = {
        let action = on_action;
        Rc::new(move |change| {
            action.call(LayoutComponentInspectorAction::SetPartAnchorValue {
                owner: latest_capture(),
                change,
            })
        })
    };
    let commit_anchor_follow: Rc<dyn Fn(Follow)> = {
        let action = on_action;
        Rc::new(move |follow| {
            action.call(LayoutComponentInspectorAction::SetPartAnchorFollow {
                owner: latest_capture(),
                follow,
            })
        })
    };
    let commit_anchor_pick: Rc<dyn Fn(String, u32, u32, Follow)> = {
        let action = on_action;
        Rc::new(move |matrix_id, row, column, follow| {
            action.call(LayoutComponentInspectorAction::SetPartAnchor {
                owner: latest_capture(),
                matrix_id,
                row,
                column,
                follow,
            })
        })
    };
    let commit_anchor_detach: Rc<dyn Fn()> = {
        let action = on_action;
        Rc::new(move || {
            action.call(LayoutComponentInspectorAction::DetachPartAnchor {
                owner: latest_capture(),
            })
        })
    };
    let anchor_follow = projection.anchor.as_ref().map(|anchor| anchor.follow);
    // Ticket 13: one shared applier for the Follow X/Y/Rotation toggles.
    let apply_follow = {
        let commit = commit_anchor_follow.clone();
        move |axis: u8, next: CheckboxState| {
            let mut follow = anchor_follow.unwrap_or(Follow {
                x: true,
                y: true,
                rotation: true,
            });
            let checked = next == CheckboxState::Checked;
            match axis {
                0 => follow.x = checked,
                1 => follow.y = checked,
                _ => follow.rotation = checked,
            }
            commit(follow);
        }
    };
    let anchor_follow_text = projection.anchor.as_ref().map(|anchor| {
        let mut parts = Vec::new();
        if anchor.follow.x {
            parts.push("X");
        }
        if anchor.follow.y {
            parts.push("Y");
        }
        if anchor.follow.rotation {
            parts.push("rotation");
        }
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(", ")
        }
    });
    let anchor_locked = position_unavailable(projection);
    // Ticket 13: cloned before the render so Pick closures borrow the list,
    // not the projection the anchor sections also read.
    let anchor_choices = projection.anchor_choices.clone();
    let anchor_number_field =
        move |label: &'static str,
              aria: &'static str,
              field: &'static str,
              current: f64,
              change: Rc<dyn Fn(f64) -> PartAnchorValueChange>| {
            let commit = commit_anchor_value.clone();
            let value = format!("{current:.2}").parse::<f64>().unwrap_or(current);
            rsx! {
                NumericField {
                    id: format!("part-anchor-{field}"),
                    label,
                    aria_label: aria,
                    value,
                    precision: 2,
                    disabled_reason: anchor_locked.then(|| "Position is unavailable".to_owned()),
                    on_commit: move |value| commit(change(value)),
                }
            }
        };
    rsx! {
        if projection.selection_count <= 1 && !projection.anchor_choices.is_empty() {
            h3 { class: "flex items-baseline justify-between text-[13px]",
                "Part anchor"
                small { "column vocabulary" }
            }
            if let Some(anchor) = projection.anchor.as_ref() {
                p {
                    "Anchored to {anchor.key_label()} · follows {anchor_follow_text.as_deref().unwrap_or(\"none\")}"
                }
                h3 { class: "flex items-baseline justify-between text-[13px]",
                    "Carried from the anchor"
                    small { "anchor frame" }
                }
                div { class: "grid grid-cols-2 gap-2",
                    {
                        anchor_number_field(
                            "Spread (mm)",
                            "Anchor spread millimetres",
                            "spread",
                            anchor.spread,
                            Rc::new(PartAnchorValueChange::Spread),
                        )
                    }
                    {
                        anchor_number_field(
                            "Stagger (mm)",
                            "Anchor stagger millimetres",
                            "stagger",
                            anchor.stagger,
                            Rc::new(PartAnchorValueChange::Stagger),
                        )
                    }
                    {
                        anchor_number_field(
                            "Splay (degrees)",
                            "Anchor splay degrees",
                            "splay",
                            anchor.splay,
                            Rc::new(PartAnchorValueChange::Splay),
                        )
                    }
                }
                h3 { class: "flex items-baseline justify-between text-[13px]",
                    "This part only"
                    small { "millimetres, degrees" }
                }
                div { class: "grid grid-cols-2 gap-2",
                    {
                        anchor_number_field(
                            "Offset X (mm)",
                            "Anchor offset X millimetres",
                            "offset-x",
                            anchor.offset.x,
                            Rc::new({
                                let y = anchor.offset.y;
                                move |x| PartAnchorValueChange::Offset(Vec2 { x, y })
                            }),
                        )
                    }
                    {
                        anchor_number_field(
                            "Offset Y (mm)",
                            "Anchor offset Y millimetres",
                            "offset-y",
                            anchor.offset.y,
                            Rc::new({
                                let x = anchor.offset.x;
                                move |y| PartAnchorValueChange::Offset(Vec2 { x, y })
                            }),
                        )
                    }
                    {
                        anchor_number_field(
                            "Rotation (degrees)",
                            "Anchor rotation degrees",
                            "rotation",
                            anchor.rotation,
                            Rc::new(PartAnchorValueChange::Rotation),
                        )
                    }
                }
                fieldset { class: "flex flex-wrap gap-3", disabled: anchor_locked,
                    legend { "Follow" }
                    CheckboxField {
                        inline: true,
                        label: "X",
                        id: "anchor-follow-x",
                        aria_label: "Follow X",
                        disabled: anchor_locked,
                        checked: if anchor_follow.is_some_and(|follow| follow.x) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: {
                            let apply = apply_follow.clone();
                            move |next| apply(0, next)
                        },
                    }
                    CheckboxField {
                        inline: true,
                        label: "Y",
                        id: "anchor-follow-y",
                        aria_label: "Follow Y",
                        disabled: anchor_locked,
                        checked: if anchor_follow.is_some_and(|follow| follow.y) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: {
                            let apply = apply_follow.clone();
                            move |next| apply(1, next)
                        },
                    }
                    CheckboxField {
                        inline: true,
                        label: "Rotation",
                        id: "anchor-follow-rotation",
                        aria_label: "Follow rotation",
                        disabled: anchor_locked,
                        checked: if anchor_follow.is_some_and(|follow| follow.rotation) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                        on_checked_change: {
                            let apply = apply_follow.clone();
                            move |next| apply(2, next)
                        },
                    }
                }
                Button {
                    button_type: "button",
                    variant: ButtonVariant::Secondary,
                    disabled: anchor_locked,
                    onclick: {
                        let commit = commit_anchor_detach.clone();
                        move |_| commit()
                    },
                    "Detach"
                }
            } else {
                p { "Detached · placed from the half origin." }
            }
            div { class: "grid grid-cols-2 gap-2",
                SelectField::<String> {
                    id: "anchor-matrix",
                    label: "Anchor matrix",
                    aria_label: "Anchor matrix",
                    value: pick_matrix(),
                    disabled: anchor_locked,
                    options: projection
                        .anchor_choices
                        .iter()
                        .map(|choice| (choice.matrix_id.clone(), choice.matrix_name.clone()))
                        .collect(),
                    on_commit: move |choice: String| pick_matrix.set(choice),
                }
                NumericField {
                    id: "anchor-row",
                    label: "Row",
                    step: Some(1.0),
                    min: Some(1.0),
                    value: pick_row().parse::<f64>().unwrap_or(1.0),
                    aria_label: "Anchor row",
                    disabled_reason: anchor_locked.then(|| "Anchor is unavailable".to_owned()),
                    on_commit: move |value: f64| pick_row.set(value.to_string()),
                }
                NumericField {
                    id: "anchor-column",
                    label: "Column",
                    step: Some(1.0),
                    min: Some(1.0),
                    value: pick_column().parse::<f64>().unwrap_or(1.0),
                    aria_label: "Anchor column",
                    disabled_reason: anchor_locked.then(|| "Anchor is unavailable".to_owned()),
                    on_commit: move |value: f64| pick_column.set(value.to_string()),
                }
                CheckboxField {
                    inline: true,
                    label: "X",
                    id: "pick-follow-x",
                    aria_label: "Pick follows X",
                    disabled: anchor_locked,
                    checked: if pick_follow_x() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |next| pick_follow_x.set(next == CheckboxState::Checked),
                }
                CheckboxField {
                    inline: true,
                    label: "Y",
                    id: "pick-follow-y",
                    aria_label: "Pick follows Y",
                    disabled: anchor_locked,
                    checked: if pick_follow_y() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |next| pick_follow_y.set(next == CheckboxState::Checked),
                }
                CheckboxField {
                    inline: true,
                    label: "Rotation",
                    id: "pick-follow-rotation",
                    aria_label: "Pick follows rotation",
                    disabled: anchor_locked,
                    checked: if pick_follow_rotation() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                    on_checked_change: move |next| pick_follow_rotation.set(next == CheckboxState::Checked),
                }
                Button {
                    button_type: "button",
                    variant: ButtonVariant::Secondary,
                    disabled: anchor_locked,
                    onclick: {
                        let commit = commit_anchor_pick.clone();
                        let mut error = error;
                        let anchor_choices = anchor_choices.clone();
                        move |_| {
                            let matrix_id = pick_matrix();
                            let Some(choice) = anchor_choices
                                .iter()
                                .find(|choice| choice.matrix_id == matrix_id) else {
                                error.set(Some("Choose an anchor matrix.".to_owned()));
                                return;
                            };
                            let (Ok(row), Ok(column)) = (
                                pick_row().trim().parse::<u32>(),
                                pick_column().trim().parse::<u32>(),
                            ) else {
                                error.set(Some("Enter a finite anchor row and column.".to_owned()));
                                return;
                            };
                            if row == 0 || row > choice.rows || column == 0 || column > choice.columns {
                                error
                                    .set(
                                        Some(
                                            format!(
                                                "Row and column must fit {} ({} rows, {} columns).",
                                                choice.matrix_name,
                                                choice.rows,
                                                choice.columns,
                                            ),
                                        ),
                                    );
                                return;
                            }
                            error.set(None);
                            commit(
                                matrix_id,
                                row - 1,
                                column - 1,
                                Follow {
                                    x: pick_follow_x(),
                                    y: pick_follow_y(),
                                    rotation: pick_follow_rotation(),
                                },
                            );
                        }
                    },
                    "Pick anchor key"
                }
            }
        }
    }
}
