use super::*;

pub(super) fn use_constraint_controls(
    projection: &LayoutComponentInspectorProjection,
    identity: &InspectorOwnerIdentity,
    latest_capture: Signal<LayoutComponentInspectorOwner>,
    on_action: EventHandler<LayoutComponentInspectorAction>,
    mut error: Signal<Option<String>>,
) -> (Element, impl FnMut() + Clone + use<>) {
    let owner = &projection.owner;
    let mut constraint_open = use_signal(|| projection.active_constraint.is_some());
    let mut constraint_kind = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { .. },
            ..
        }) => ConstraintKind::MirrorLine,
        _ => ConstraintKind::Offset,
    });
    let drafts = offset_drafts(
        projection.active_constraint.as_ref(),
        &projection.board_parts,
        &owner.part_id,
    );
    let mut source_part_id = use_signal(|| drafts.source_part_id.clone());
    let mut offset_axes = use_signal(|| drafts.axes);
    let mut offset_x = use_signal(|| drafts.offset_x.clone());
    let mut offset_y = use_signal(|| drafts.offset_y.clone());
    let mut rotation = use_signal(|| drafts.rotation.clone());
    let mut mirror_axis = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { axis, .. }) => axis.clone(),
        _ => MirrorAxis::Vertical,
    });
    let mut mirror_coordinate = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { coordinate, .. }) => coordinate.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_x = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.x.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_y = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { at, .. },
            ..
        }) => at.y.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_line_angle = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Framed {
            relation: ConstraintRelation::MirrorLine { angle, .. },
            ..
        }) => angle.to_string(),
        _ => "90".to_owned(),
    });
    let kind_choice = use_memo(move || {
        Some(
            match constraint_kind() {
                ConstraintKind::Offset => "offset",
                ConstraintKind::Mirror => "mirror",
                ConstraintKind::MirrorLine => "mirror-line",
            }
            .to_owned(),
        )
    });
    let axes_choice = use_memo(move || Some(offset_axes_value(offset_axes()).to_owned()));
    let axis_choice = use_memo(move || {
        Some(
            if mirror_axis() == MirrorAxis::Vertical {
                "vertical"
            } else {
                "horizontal"
            }
            .to_owned(),
        )
    });
    let initial_constraint = projection.active_constraint.clone();
    let has_initial_constraint = initial_constraint.is_some();
    let board_parts = projection.board_parts.clone();
    let part_id = owner.part_id.clone();
    use_effect(use_reactive((identity,), {
        move |_| {
            constraint_open.set(has_initial_constraint);
            error.set(None);
        }
    }));
    let reset_constraint = move |constraint: Option<Constraint>| {
        constraint_open.set(constraint.is_some());
        constraint_kind.set(match constraint.as_ref() {
            Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
            Some(Constraint::Framed {
                relation: ConstraintRelation::MirrorLine { .. },
                ..
            }) => ConstraintKind::MirrorLine,
            _ => ConstraintKind::Offset,
        });
        let drafts = offset_drafts(constraint.as_ref(), &board_parts, &part_id);
        source_part_id.set(drafts.source_part_id);
        offset_axes.set(drafts.axes);
        offset_x.set(drafts.offset_x);
        offset_y.set(drafts.offset_y);
        rotation.set(drafts.rotation);
        match constraint {
            Some(Constraint::Mirror {
                axis, coordinate, ..
            }) => {
                mirror_axis.set(axis.clone());
                mirror_coordinate.set(coordinate.to_string());
            }
            Some(Constraint::Framed {
                relation: ConstraintRelation::MirrorLine { at, angle },
                ..
            }) => {
                mirror_line_x.set(at.x.to_string());
                mirror_line_y.set(at.y.to_string());
                mirror_line_angle.set(angle.to_string());
            }
            _ => {
                mirror_axis.set(MirrorAxis::Vertical);
                mirror_coordinate.set("0".to_owned());
                mirror_line_x.set("0".to_owned());
                mirror_line_y.set("0".to_owned());
                mirror_line_angle.set("90".to_owned());
            }
        }
        error.set(None);
    };
    let accepted_constraint = (identity.clone(), initial_constraint.clone());
    use_effect(use_reactive((&accepted_constraint,), {
        let mut reset = reset_constraint.clone();
        move |((_, constraint),)| reset(constraint)
    }));
    let reset_properties_drafts = {
        let mut reset = reset_constraint;
        move || reset(initial_constraint.clone())
    };
    let has_active_constraint = projection.active_constraint.is_some();
    // A choice on a saved offset converts it at once; a new one only sets the default.
    let mut choose_offset_axes = {
        let action = on_action;
        let saved_offset_id = projection
            .active_constraint
            .as_ref()
            .filter(|constraint| constraint.offset_axes().is_some())
            .map(|constraint| constraint.id().to_owned());
        let mut offset_axes = offset_axes;
        move |to: OffsetAxes| {
            offset_axes.set(to);
            if let Some(constraint_id) = saved_offset_id.as_ref() {
                action.call(LayoutComponentInspectorAction::ConvertOffsetAxes {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                    to,
                });
            }
        }
    };
    let save_constraint = {
        let offset_axes = offset_axes;
        let mirror_line_x = mirror_line_x;
        let mirror_line_y = mirror_line_y;
        let mirror_line_angle = mirror_line_angle;
        let retain_legacy_form = projection
            .active_constraint
            .as_ref()
            .is_some_and(|constraint| !matches!(constraint, Constraint::Framed { .. }));
        let action = on_action;
        let mut error = error;
        move |_| {
            let source = source_part_id();
            let owner = latest_capture();
            if source.is_empty() || source == owner.part_id {
                error.set(Some("Choose another part on this board.".to_owned()));
                return;
            }
            let values = match constraint_kind() {
                ConstraintKind::Offset => {
                    let (Ok(x), Ok(y), Ok(rotation)) = (
                        offset_x().parse::<f64>(),
                        offset_y().parse::<f64>(),
                        rotation().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    }
                    let offset = Vec2 { x, y };
                    if retain_legacy_form {
                        offset_values(offset_axes(), offset, rotation)
                    } else {
                        LayoutConstraintValues::Framed {
                            relation: match offset_axes() {
                                OffsetAxes::Board => {
                                    ConstraintRelation::Offset { offset, rotation }
                                }
                                OffsetAxes::Part => {
                                    ConstraintRelation::PartOffset { offset, rotation }
                                }
                            },
                        }
                    }
                }
                ConstraintKind::Mirror => {
                    let Ok(coordinate) = mirror_coordinate().parse::<f64>() else {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    };
                    if !coordinate.is_finite() {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    }
                    if retain_legacy_form {
                        LayoutConstraintValues::Mirror {
                            axis: mirror_axis(),
                            coordinate,
                        }
                    } else {
                        let axis = mirror_axis();
                        let at = match axis {
                            MirrorAxis::Vertical => Vec2 {
                                x: coordinate,
                                y: 0.0,
                            },
                            MirrorAxis::Horizontal => Vec2 {
                                x: 0.0,
                                y: coordinate,
                            },
                        };
                        LayoutConstraintValues::Framed {
                            relation: ConstraintRelation::MirrorLine {
                                at,
                                angle: if axis == MirrorAxis::Vertical {
                                    90.0
                                } else {
                                    0.0
                                },
                            },
                        }
                    }
                }
                ConstraintKind::MirrorLine => {
                    let (Ok(x), Ok(y), Ok(angle)) = (
                        mirror_line_x().parse::<f64>(),
                        mirror_line_y().parse::<f64>(),
                        mirror_line_angle().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !angle.is_finite() {
                        error.set(Some("Enter finite mirror line values.".to_owned()));
                        return;
                    }
                    LayoutConstraintValues::Framed {
                        relation: ConstraintRelation::MirrorLine {
                            at: Vec2 { x, y },
                            angle,
                        },
                    }
                }
            };
            error.set(None);
            action.call(LayoutComponentInspectorAction::SetConstraint {
                owner: owner.clone(),
                source_part_id: source,
                values,
            });
        }
    };
    let active_constraint_id = projection
        .active_constraint
        .as_ref()
        .map(|constraint| constraint.id().to_owned());
    let remove_constraint = {
        let action = on_action;
        move |_| {
            if let Some(constraint_id) = active_constraint_id.as_ref() {
                action.call(LayoutComponentInspectorAction::RemoveConstraint {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                });
            }
        }
    };
    let active_constraint = projection.active_constraint.clone();
    let board_parts = projection.board_parts.clone();
    let constraint_source_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.source())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.source().to_owned()))
        })
        .unwrap_or_else(|| "A part".to_owned());
    let constraint_target_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.target())
                .map(|part| part.reference.clone())
                .or_else(|| Some(constraint.target().to_owned()))
        })
        .unwrap_or_else(|| projection.reference.clone());

    let controls = rsx! {
        InspectorSection {
            title: "Layout constraint",
            badge: if active_constraint.is_some() { "Active" } else { "Optional" },
            open: constraint_open(),
            on_open_change: move |next| constraint_open.set(next),
                if board_parts.iter().filter(|part| part.id != owner.part_id).count() > 0 {
                    SelectField::<String> {
                        id: "layout-constraint-type",
                        label: "Constraint type",
                        aria_label: "Constraint type",
                        value: kind_choice().unwrap_or_default(),
                        options: vec![
                            ("offset".into(), "Offset from part".into()),
                            ("mirror".into(), "Mirror placement across axis".into()),
                            ("mirror-line".into(), "Mirror line".into()),
                        ],
                        on_commit: move |choice: String| {
                            constraint_kind
                                .set(
                                    match choice.as_str() {
                                        "mirror" => ConstraintKind::Mirror,
                                        "mirror-line" => ConstraintKind::MirrorLine,
                                        _ => ConstraintKind::Offset,
                                    },
                                );
                        },
                    }
                    {
                        let mut source_options: Vec<(String, String)> = Vec::new();
                        if !board_parts.iter().any(|part| part.id == source_part_id()) {
                            source_options
                                .push((
                                    source_part_id.to_string(),
                                    constraint_source_reference.to_string(),
                                ));
                        }
                        for part in board_parts.iter().filter(|part| part.id != owner.part_id) {
                            source_options
                                .push((part.id.to_string(), part.reference.to_string()));
                        }
                        let board_parts = board_parts.clone();
                        rsx! {
                            SelectField::<String> {
                                id: "layout-constraint-source",
                                label: "Source part",
                                aria_label: "Constraint source part",
                                value: source_part_id(),
                                options: source_options,
                                on_commit: move |choice: String| {
                                    if !has_active_constraint {
                                        offset_axes.set(default_axes_for_source(&board_parts, &choice));
                                    }
                                    source_part_id.set(choice);
                                },
                            }
                        }
                    }
                    if constraint_kind() == ConstraintKind::Offset {
                        SelectField::<String> {
                            id: "layout-constraint-offset-axes",
                            label: "Offset axes",
                            aria_label: "Offset axes",
                            value: axes_choice().unwrap_or_default(),
                            options: vec![
                                (offset_axes_value(OffsetAxes::Board).to_owned(), "Offset on board axes".into()),
                                (offset_axes_value(OffsetAxes::Part).to_owned(), "Offset along part axes".into()),
                            ],
                            on_commit: move |choice: String| choose_offset_axes(offset_axes_from_value(&choice)),
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            NumericField {
                                id: "layout-constraint-offset-x",
                                label: "Offset X (mm)",
                                aria_label: "Offset X (mm)",
                                value: offset_x().parse::<f64>().unwrap_or_default(),
                                on_commit: move |value: f64| offset_x.set(value.to_string()),
                            }
                            NumericField {
                                id: "layout-constraint-offset-y",
                                label: "Offset Y (mm)",
                                aria_label: "Offset Y (mm)",
                                value: offset_y().parse::<f64>().unwrap_or_default(),
                                on_commit: move |value: f64| offset_y.set(value.to_string()),
                            }
                        }
                        NumericField {
                            id: "layout-constraint-rotation",
                            label: "Rotation (degrees)",
                            aria_label: "Rotation (degrees)",
                            value: rotation().parse::<f64>().unwrap_or_default(),
                            on_commit: move |value: f64| rotation.set(value.to_string()),
                        }
                    } else if constraint_kind() == ConstraintKind::Mirror {
                        SelectField::<String> {
                            id: "layout-constraint-mirror-axis",
                            label: "Axis",
                            aria_label: "Mirror axis",
                            value: axis_choice().unwrap_or_default(),
                            options: vec![
                                ("vertical".into(), "Vertical".into()),
                                ("horizontal".into(), "Horizontal".into()),
                            ],
                            on_commit: move |choice: String| {
                                mirror_axis
                                    .set(
                                        if choice == "horizontal" {
                                            MirrorAxis::Horizontal
                                        } else {
                                            MirrorAxis::Vertical
                                        },
                                    );
                            },
                        }
                        NumericField {
                            id: "layout-constraint-axis-coordinate",
                            label: "Axis coordinate (mm)",
                            aria_label: "Axis coordinate (mm)",
                            value: mirror_coordinate().parse::<f64>().unwrap_or_default(),
                            on_commit: move |value: f64| mirror_coordinate.set(value.to_string()),
                        }
                        p { "Footprint geometry stays unchanged. Use a handed definition where needed." }
                    } else {
                        div { class: "grid grid-cols-2 gap-2",
                            NumericField {
                                id: "layout-constraint-line-x",
                                label: "Line anchor X (mm)",
                                aria_label: "Mirror line anchor X (mm)",
                                value: mirror_line_x().parse::<f64>().unwrap_or_default(),
                                on_commit: move |value: f64| mirror_line_x.set(value.to_string()),
                            }
                            NumericField {
                                id: "layout-constraint-line-y",
                                label: "Line anchor Y (mm)",
                                aria_label: "Mirror line anchor Y (mm)",
                                value: mirror_line_y().parse::<f64>().unwrap_or_default(),
                                on_commit: move |value: f64| mirror_line_y.set(value.to_string()),
                            }
                        }
                        NumericField {
                            id: "layout-constraint-line-angle",
                            label: "Line angle (degrees)",
                            aria_label: "Mirror line angle (degrees)",
                            value: mirror_line_angle().parse::<f64>().unwrap_or_default(),
                            on_commit: move |value: f64| mirror_line_angle.set(value.to_string()),
                        }
                        p { "Footprint geometry stays unchanged. Use a handed definition where needed." }
                    }
                    if active_constraint.is_some() {
                        p { class: "text-[11px] leading-[1.45] text-muted-foreground",
                            "{constraint_source_reference} drives {constraint_target_reference}."
                        }
                    }
                    div { class: "flex flex-wrap gap-2 [&_button]:min-h-[34px]",
                        Button {
                            variant: ButtonVariant::Secondary,
                            button_type: "button",
                            onclick: save_constraint,
                            if active_constraint.is_some() {
                                "Save constraint"
                            } else {
                                "Add constraint"
                            }
                        }
                        if active_constraint.is_some() {
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                onclick: remove_constraint,
                                "Remove"
                            }
                        }
                    }
                } else {
                    p { "Add another part on this board to create a layout constraint." }
                }
        }
    };
    (controls, reset_properties_drafts)
}
