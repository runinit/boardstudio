macro_rules! render_inspector {
    ($props:ident, $mirror_section_open:ident, $outline_section_open:ident, $projection:ident, $owner:ident, $latest_capture:ident, $tab:ident, $constraint_open:ident, $x:ident, $y:ident, $margin:ident, $x_failure:ident, $y_failure:ident, $margin_failure:ident, $constraint_kind:ident, $source_part_id:ident, $offset_axes:ident, $offset_x:ident, $offset_y:ident, $rotation:ident, $mirror_axis:ident, $mirror_coordinate:ident, $mirror_line_x:ident, $mirror_line_y:ident, $mirror_line_angle:ident, $layout_choice:ident, $kind_choice:ident, $source_choice:ident, $axes_choice:ident, $axis_choice:ident, $error:ident, $anchor_spread:ident, $anchor_stagger:ident, $anchor_splay:ident, $anchor_offset_x:ident, $anchor_offset_y:ident, $anchor_rotation:ident, $pick_matrix:ident, $pick_matrix_choice:ident, $pick_row:ident, $pick_column:ident, $pick_follow_x:ident, $pick_follow_y:ident, $pick_follow_rotation:ident, $mirrored_half_action:ident, $board_parts:ident, $reset_properties_drafts:ident, $last_enter_commit:ident, $commit_position:ident, $set_outline:ident, $commit_anchor_follow:ident, $commit_anchor_pick:ident, $commit_anchor_detach:ident, $anchor_follow:ident, $anchor_follow_text:ident, $anchor_locked:ident, $anchor_choices:ident, $anchor_number_field:ident, $has_active_constraint:ident, $choose_offset_axes:ident, $save_constraint:ident, $remove_constraint:ident, $position:ident, $group_selection:ident, $outline:ident, $active_constraint:ident, $constraint_source_reference:ident, $constraint_target_reference:ident) => {{
        let props = $props.clone();
        let mut mirror_section_open = $mirror_section_open;
        let mut outline_section_open = $outline_section_open;
        let projection = $projection.clone();
        let owner = $owner;
        let latest_capture = $latest_capture;
        let mut tab = $tab;
        let mut constraint_open = $constraint_open;
        let mut x = $x;
        let mut y = $y;
        let mut margin = $margin;
        let x_failure = $x_failure;
        let y_failure = $y_failure;
        let margin_failure = $margin_failure;
        let mut constraint_kind = $constraint_kind;
        let mut source_part_id = $source_part_id;
        let mut offset_axes = $offset_axes;
        let mut offset_x = $offset_x;
        let mut offset_y = $offset_y;
        let mut rotation = $rotation;
        let mut mirror_axis = $mirror_axis;
        let mut mirror_coordinate = $mirror_coordinate;
        let mut mirror_line_x = $mirror_line_x;
        let mut mirror_line_y = $mirror_line_y;
        let mut mirror_line_angle = $mirror_line_angle;
        let layout_choice = $layout_choice;
        let kind_choice = $kind_choice;
        let source_choice = $source_choice;
        let axes_choice = $axes_choice;
        let axis_choice = $axis_choice;
        let mut error = $error;
        let anchor_spread = $anchor_spread;
        let anchor_stagger = $anchor_stagger;
        let anchor_splay = $anchor_splay;
        let anchor_offset_x = $anchor_offset_x;
        let anchor_offset_y = $anchor_offset_y;
        let anchor_rotation = $anchor_rotation;
        let mut pick_matrix = $pick_matrix;
        let pick_matrix_choice = $pick_matrix_choice;
        let mut pick_row = $pick_row;
        let mut pick_column = $pick_column;
        let mut pick_follow_x = $pick_follow_x;
        let mut pick_follow_y = $pick_follow_y;
        let mut pick_follow_rotation = $pick_follow_rotation;
        let mirrored_half_action = $mirrored_half_action;
        let board_parts = $board_parts;
        let reset_properties_drafts = $reset_properties_drafts;
        let last_enter_commit = $last_enter_commit;
        let commit_position = $commit_position;
        let set_outline = $set_outline;
        let commit_anchor_follow = $commit_anchor_follow;
        let commit_anchor_pick = $commit_anchor_pick;
        let commit_anchor_detach = $commit_anchor_detach;
        let anchor_follow = $anchor_follow;
        let anchor_follow_text = $anchor_follow_text;
        let anchor_locked = $anchor_locked;
        let anchor_choices = $anchor_choices;
        let anchor_number_field = $anchor_number_field;
        let has_active_constraint = $has_active_constraint;
        let mut choose_offset_axes = $choose_offset_axes;
        let save_constraint = $save_constraint;
        let remove_constraint = $remove_constraint;
        let position = $position;
        let group_selection = $group_selection;
        let outline = $outline;
        let active_constraint = $active_constraint;
        let constraint_source_reference = $constraint_source_reference;
        let constraint_target_reference = $constraint_target_reference;
    rsx! {
                section {
                    class: "grid min-w-0 content-start gap-3 px-4 pt-3 pb-5 text-foreground [&_h2]:m-0 [&_h3]:m-0 [&_p]:m-0 [&_fieldset_small]:text-[11px] [&_fieldset_small]:leading-[1.45] [&_fieldset_small]:text-muted-foreground",
                    aria_label: "Component inspector",
                onkeydown: move |event: KeyboardEvent| {
                    if event.key().to_string() == " " {
                        event.stop_propagation();
                        if let Some(raw) = event.data().try_as_web_event() {
                            raw.stop_propagation();
                        }
                    }
                },
                    header { class: "flex items-start justify-between gap-2 [&_h2]:text-base [&_h2]:font-semibold [&_h2]:[overflow-wrap:anywhere]",
                        h2 { "{projection.reference}" }
                        if projection.locked {
                            span { class: "flex-none rounded-[9px] border border-border px-1.5 py-0.5 text-[11px] text-muted-foreground", "Locked" }
                        }
                    }
                    Tabs { horizontal: true,
    value: Some(
                            if tab() == LayoutInspectorTab::Properties {
                                "properties".to_owned()
                            } else {
                                "relations".to_owned()
                            },
                        ),
    on_value_change: { let mut reset = reset_properties_drafts; move |value: String| { let next = if value == "relations" { LayoutInspectorTab::Relations } else { LayoutInspectorTab::Properties }; if next == LayoutInspectorTab::Properties && *tab.peek() != next { reset(); } tab.set(next); } },
                        TabsList {
                            aria_label: "Inspector details",
                            class: "layout-inspector-tabs",
                            TabsTrigger { index: 0usize, value: "properties", "Properties" }
                            TabsTrigger { index: 1usize, value: "relations", "Relations" }
                        }
                        TabsContent { index: 0usize, value: "properties",
                            if group_selection {
                                p {
                                    "{projection.selection_count} parts selected. Position edits apply to the selection."
                                }
                            } else {
                                p { class: "grid min-w-0 grid-cols-[1fr_auto] items-baseline gap-[5px] text-[13px] text-foreground [&_span]:text-xs [&_span]:leading-[1.45] [&_span]:text-muted-foreground",
                                    "{projection.definition_name}"
                                    span { "{projection.definition_kind}" }
                                }
                                if let Some(notice) = projection.envelope_notice.as_ref() {
                                    Callout { class: "text-xs leading-[1.45] text-muted-foreground", variant: CalloutVariant::Info, "{notice}" }
                                }
                            }
                            if let Some(mirrored) = projection.part_mirror {
                                Collapsible {
                                    keep_mounted: true,
                                    open: Some(mirror_section_open()),
                                    on_open_change: move |next| mirror_section_open.set(next),
                                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Mirrored half",
                                        span { "Mirrored half" }
                                        small { if mirrored { "Both halves" } else { "Local" } }
                                        ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" }
                                    }
                                    CollapsibleContent {
                                        hidden: !mirror_section_open(), inert: (!mirror_section_open()).then_some(""),
                                        CheckboxField {
                                            inline: true, id: "component-inspector-part-mirror",
                                            label: "Mirror on both halves", aria_label: "Mirror on both halves",
                                            checked: if mirrored { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                            on_checked_change: {
                                                let action = props.on_action;
                                                let part_id = owner.part_id.clone();
                                                move |next| action.call(LayoutComponentInspectorAction::SetPartMirrored {
                                                    owner: latest_capture(), part_id: part_id.clone(), mirrored: next == CheckboxState::Checked,
                                                })
                                            },
                                        }
                                        p { "A mirrored part edits both halves together and stops at the split axis." }
                                    }
                                }
                            } else if let Some(mirrored_half) = projection.mirrored_half.as_ref() {
                                Collapsible {
                                    keep_mounted: true,
                                    open: Some(mirror_section_open()),
                                    on_open_change: move |next| mirror_section_open.set(next),
                                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Mirrored half",
                                        span { "Mirrored half" }
                                        small {
                                            match mirrored_half.state {
                                                HalfExceptionState::Linked => "Linked",
                                                HalfExceptionState::Local => "Local",
                                                HalfExceptionState::Omitted => "Omitted",
                                            }
                                        }
                                     ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                                    CollapsibleContent {
                                        hidden: !mirror_section_open(),
                                        inert: (!mirror_section_open()).then_some(""),

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
                            if !group_selection && !projection.layouts.is_empty() {
                                {
                                    let control_label = "Layout".to_owned();
                                    let control_aria_label = "Component layout".to_owned();
                                    let control_id = format!(
                                        "layout-choice-{}-{}",
                                        dioxus::core::current_scope_id().0,
                                        control_aria_label.replace(' ', "-"),
                                    );

                                    let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                    control_choices.push(("".to_owned(), "Board / ungrouped".to_owned(), false));
                                    for layout in &projection.layouts {
                                        control_choices
                                            .push((layout.id.to_string(), layout.name.to_string(), false));
                                    }
                                    let on_value_change = EventHandler::new({

                let action = props.on_action;
                move |choice: Option<String>| {
                    let Some(choice) = choice else {
                        return;
                    };
                    action.call(LayoutComponentInspectorAction::AssignLayout {
                        owner: latest_capture(),
                        layout_id: (!choice.is_empty()).then_some(choice),
                    })
                }
            });
            rsx! {

                                        Field {
                                            FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                            if control_choices.len() > 12 {
                                                Combobox::<String> {
                                                    id: control_id.clone(),
                                                    aria_label: control_aria_label.clone(),
                                                    list_aria_label: control_aria_label.clone(),
                                                    value: Some(layout_choice.into()),
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
                                            } else {
                                                Select::<String> {
                                                    id: control_id.clone(),
                                                    aria_label: control_aria_label.clone(),
                                                    value: Some(layout_choice.into()),
                                                    on_value_change,
                                                    for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                        SelectOption::<String> {
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
                                }
                            }
                if !group_selection && !projection.anchor_choices.is_empty() {
                    h3 { class: "flex items-baseline justify-between text-[13px]", "Part anchor", small { "column vocabulary" } }
                    if let Some(anchor) = projection.anchor.as_ref() {
                        p { "Anchored to {anchor.key_label()} · follows {anchor_follow_text.as_deref().unwrap_or(\"none\")}" }
                        h3 { class: "flex items-baseline justify-between text-[13px]", "Carried from the anchor", small { "anchor frame" } }
                        div { class: "grid grid-cols-2 gap-2",
                            {anchor_number_field("Spread (mm)", "Anchor spread millimetres", "spread", anchor_spread,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.spread),
                                projection.anchor.as_ref().map(|anchor| anchor.spread),
                                Rc::new(PartAnchorValueChange::Spread))}
                            {anchor_number_field("Stagger (mm)", "Anchor stagger millimetres", "stagger", anchor_stagger,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.stagger),
                                projection.anchor.as_ref().map(|anchor| anchor.stagger),
                                Rc::new(PartAnchorValueChange::Stagger))}
                            {anchor_number_field("Splay (degrees)", "Anchor splay degrees", "splay", anchor_splay,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.splay),
                                projection.anchor.as_ref().map(|anchor| anchor.splay),
                                Rc::new(PartAnchorValueChange::Splay))}
                        }
                        h3 { class: "flex items-baseline justify-between text-[13px]", "This part only", small { "millimetres, degrees" } }
                        div { class: "grid grid-cols-2 gap-2",
                            {anchor_number_field("Offset X (mm)", "Anchor offset X millimetres", "offset-x", anchor_offset_x,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.offset.x),
                                projection.anchor.as_ref().map(|anchor| anchor.offset.x),
                                Rc::new({
                                    let other = anchor_offset_y;
                                    let fallback = projection.anchor.as_ref().map(|anchor| anchor.offset.y).unwrap_or(0.0);
                                    move |value: f64| PartAnchorValueChange::Offset(Vec2 {
                                        x: value,
                                        y: anchor_commit_value(&other()).unwrap_or(fallback),
                                    })
                                }))}
                            {anchor_number_field("Offset Y (mm)", "Anchor offset Y millimetres", "offset-y", anchor_offset_y,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.offset.y),
                                projection.anchor.as_ref().map(|anchor| anchor.offset.y),
                                Rc::new({
                                    let other = anchor_offset_x;
                                    let fallback = projection.anchor.as_ref().map(|anchor| anchor.offset.x).unwrap_or(0.0);
                                    move |value: f64| PartAnchorValueChange::Offset(Vec2 {
                                        x: anchor_commit_value(&other()).unwrap_or(fallback),
                                        y: value,
                                    })
                                }))}
                            {anchor_number_field("Rotation (degrees)", "Anchor rotation degrees", "rotation", anchor_rotation,
                                anchor_draft(projection.anchor.as_ref(), |anchor| anchor.rotation),
                                projection.anchor.as_ref().map(|anchor| anchor.rotation),
                                Rc::new(PartAnchorValueChange::Rotation))}
                        }
                        fieldset { class: "flex flex-wrap gap-3", disabled: anchor_locked,
                            legend { "Follow" }
                            CheckboxField { inline: true, label: "X",
                                id: "anchor-follow-x", aria_label: "Follow X", disabled: anchor_locked,
                                checked: if anchor_follow.is_some_and(|follow| follow.x) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                on_checked_change: { let commit = commit_anchor_follow.clone();
                                    let previous = anchor_follow.unwrap_or(Follow { x: true, y: true, rotation: true });
                                    move |next| commit(Follow { x: next == CheckboxState::Checked, y: previous.y, rotation: previous.rotation }) },
                            }
                            CheckboxField { inline: true, label: "Y",
                                id: "anchor-follow-y", aria_label: "Follow Y", disabled: anchor_locked,
                                checked: if anchor_follow.is_some_and(|follow| follow.y) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                on_checked_change: { let commit = commit_anchor_follow.clone();
                                    let previous = anchor_follow.unwrap_or(Follow { x: true, y: true, rotation: true });
                                    move |next| commit(Follow { x: previous.x, y: next == CheckboxState::Checked, rotation: previous.rotation }) },
                            }
                            CheckboxField { inline: true, label: "Rotation",
                                id: "anchor-follow-rotation", aria_label: "Follow rotation", disabled: anchor_locked,
                                checked: if anchor_follow.is_some_and(|follow| follow.rotation) { CheckboxState::Checked } else { CheckboxState::Unchecked },
                                on_checked_change: { let commit = commit_anchor_follow.clone();
                                    let previous = anchor_follow.unwrap_or(Follow { x: true, y: true, rotation: true });
                                    move |next| commit(Follow { x: previous.x, y: previous.y, rotation: next == CheckboxState::Checked }) },
                            }
                        }
                        Button { button_type: "button", variant: ButtonVariant::Secondary, disabled: anchor_locked,
                            onclick: { let commit = commit_anchor_detach.clone(); move |_| commit() },
                            "Detach" }
                    } else {
                        p { "Detached · placed from the half origin." }
                    }
                    div { class: "grid grid-cols-2 gap-2",
                        Field { FieldLabel { html_for: "anchor-matrix", "Anchor matrix" }
                          Select::<String> {
                            id: "anchor-matrix", aria_label: "Anchor matrix", value: Some(pick_matrix_choice.into()), disabled: anchor_locked,
                            on_value_change: move |choice: Option<String>| { if let Some(choice) = choice { pick_matrix.set(choice); } },
                            for (index, choice) in projection.anchor_choices.iter().enumerate() {
                                SelectOption::<String> { index, value: choice.matrix_id.clone(), text_value: choice.matrix_name.clone(), "{choice.matrix_name}" }
                            }
                          }
                        }
                        NumericField {
                            id: "anchor-row", label: "Row", step: "1", min: "1", value: "{pick_row}",
                            class: "", control_class: "", aria_invalid: None, aria_label: "Anchor row",
                            disabled: anchor_locked, oninput: move |event: FormEvent| pick_row.set(event.value()),
                        }
                        NumericField {
                            id: "anchor-column", label: "Column", step: "1", min: "1", value: "{pick_column}",
                            class: "", control_class: "", aria_invalid: None, aria_label: "Anchor column",
                            disabled: anchor_locked, oninput: move |event: FormEvent| pick_column.set(event.value()),
                        }
                        CheckboxField { inline: true, label: "X",
                            id: "pick-follow-x", aria_label: "Pick follows X", disabled: anchor_locked,
                            checked: if pick_follow_x() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                            on_checked_change: move |next| pick_follow_x.set(next == CheckboxState::Checked),
                        }
                        CheckboxField { inline: true, label: "Y",
                            id: "pick-follow-y", aria_label: "Pick follows Y", disabled: anchor_locked,
                            checked: if pick_follow_y() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                            on_checked_change: move |next| pick_follow_y.set(next == CheckboxState::Checked),
                        }
                        CheckboxField { inline: true, label: "Rotation",
                            id: "pick-follow-rotation", aria_label: "Pick follows rotation", disabled: anchor_locked,
                            checked: if pick_follow_rotation() { CheckboxState::Checked } else { CheckboxState::Unchecked },
                            on_checked_change: move |next| pick_follow_rotation.set(next == CheckboxState::Checked),
                        }
                        Button { button_type: "button", variant: ButtonVariant::Secondary, disabled: anchor_locked,
                            onclick: {
                                let commit = commit_anchor_pick.clone();
                                let mut error = error;
                                let anchor_choices = anchor_choices.clone();
                                move |_| {
                                    let matrix_id = pick_matrix();
                                    let Some(choice) = anchor_choices.iter().find(|choice| choice.matrix_id == matrix_id) else {
                                        error.set(Some("Choose an anchor matrix.".to_owned()));
                                        return;
                                    };
                                    let (Ok(row), Ok(column)) = (pick_row().trim().parse::<u32>(), pick_column().trim().parse::<u32>()) else {
                                        error.set(Some("Enter a finite anchor row and column.".to_owned()));
                                        return;
                                    };
                                    if row == 0 || row > choice.rows || column == 0 || column > choice.columns {
                                        error.set(Some(format!("Row and column must fit {} ({} rows, {} columns).", choice.matrix_name, choice.rows, choice.columns)));
                                        return;
                                    }
                                    error.set(None);
                                    commit(matrix_id, row - 1, column - 1, Follow {
                                        x: pick_follow_x(),
                                        y: pick_follow_y(),
                                        rotation: pick_follow_rotation(),
                                    });
                                }
                            },
                            "Pick anchor key" }
                    }
                }
                            h3 { class: "flex items-baseline justify-between text-[13px] [&_small]:text-[11px] [&_small]:font-normal [&_small]:text-muted-foreground",
                                "Position"
                                small { "millimetres" }
                            }
                            div { class: "grid grid-cols-2 gap-2",
                                {
                                    let control_input_id = format!(
                                        "layout-input-{}-10",
                                        dioxus::core::current_scope_id().0,
                                    );
                                    rsx! {
                                        NumericField {
                                            id: control_input_id, label: "X (mm)", class: "", aria_invalid: None,
                                            control_class: "max-[760px]:h-control-touch",
                                            step: "0.1",
                                            value: "{x}",
                                            aria_label: "X mm",
                                            disabled: position_input_disabled(position_unavailable(&projection)),
                                            oninput: { let last_enter_commit = last_enter_commit.clone(); move |event: FormEvent| { last_enter_commit.borrow_mut().take(); x.set(event.value()); } },
                                            onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::X, PositionCommitTrigger::Blur) },
                                            onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                                                    match event.data().key().to_string().as_str() {
                                                        "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::X, PositionCommitTrigger::Enter); }
                                                        "Escape" => { x.set(format!("{:.2}", position.x)); error.set(None); }
                                                        _ => {}
                                                    }
                                                } },
                                        }
                                    }
                                }
                                {
                                    let control_input_id = format!(
                                        "layout-input-{}-9",
                                        dioxus::core::current_scope_id().0,
                                    );
                                    rsx! {
                                        NumericField {
                                            id: control_input_id, label: "Y (mm)", class: "", aria_invalid: None,
                                            control_class: "max-[760px]:h-control-touch",
                                            step: "0.1",
                                            value: "{y}",
                                            aria_label: "Y mm",
                                            disabled: position_input_disabled(position_unavailable(&projection)),
                                            oninput: { let last_enter_commit = last_enter_commit.clone(); move |event: FormEvent| { last_enter_commit.borrow_mut().take(); y.set(event.value()); } },
                                            onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::Y, PositionCommitTrigger::Blur) },
                                            onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                                                    match event.data().key().to_string().as_str() {
                                                        "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::Y, PositionCommitTrigger::Enter); }
                                                        "Escape" => { y.set(format!("{:.2}", position.y)); error.set(None); }
                                                        _ => {}
                                                    }
                                                } },
                                        }
                                    }
                                }
                            }
                            if !group_selection {
                                Collapsible {
                                    class: "[&_fieldset]:grid [&_fieldset]:min-w-0 [&_fieldset]:gap-[9px] [&_fieldset]:border-0 [&_fieldset]:px-0 [&_fieldset]:pt-0 [&_fieldset]:pb-3",
                                    keep_mounted: true,
                                    open: Some(outline_section_open()),
                                    on_open_change: move |next| outline_section_open.set(next),
                                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Board outline",
                                        span { "Board outline" }
                                        small {
                                            if projection.outline.excluded {
                                                "Excluded"
                                            } else {
                                                "Included"
                                            }
                                        }
                                     ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
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
                                                        inline: true, label: "Include in outline",
                                                        id: check_id,
                                                        aria_label: "Include in outline",
                                                        checked: if !projection.outline.excluded {
                                                                CheckboxState::Checked
                                                            } else {
                                                                CheckboxState::Unchecked
                                                            },
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
                                                            inline: true, label: "Use board margin",
                                                            id: check_id,
                                                            aria_label: "Use board margin",
                                                            checked: if projection.outline.margin.is_none() {
                                                                    CheckboxState::Checked
                                                                } else {
                                                                    CheckboxState::Unchecked
                                                                },
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
                                                                id: control_input_id, label: "Part edge margin", class: "", aria_invalid: None,
                                                                control_class: "max-[760px]:h-control-touch",
                                                                min: "0",
                                                                value: "{margin}",
                                                                aria_label: "Part edge margin",
                                                                oninput: move |event: FormEvent| margin.set(event.value()),
                                                                onblur: {
                                                                    let margin_outline = outline.clone();
                                                                    let save_outline = set_outline.clone();
                                                                    move |_| {
                                                                        match margin().parse::<f64>() {
                                                                            Ok(value) if value.is_finite() && value >= 0.0 => {
                                                                                if value != margin_outline.margin.unwrap_or(value) {
                                                                                    let mut next = margin_outline.clone(); next.margin = Some(value); save_outline(next);
                                                                                }
                                                                                error.set(None);
                                                                            }
                                                                            _ => error.set(Some("Enter 0 or greater for the part edge margin.".to_owned())),
                                                                        }
                                                                    }
                                                                },
                                                                onkeydown: {
                                                                    let accepted_margin = outline.margin.unwrap_or_default().to_string();
                                                                    move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                                                        "Enter" => {
                                                                            event.prevent_default();
                                                                            if let Some(input) = event.data().try_as_web_event()
                                                                                .and_then(|event| event.target())
                                                                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                                                            {
                                                                                let _ = input.blur();
                                                                            }
                                                                        }
                                                                        "Escape" => {
                                                                            event.prevent_default();
                                                                            margin.set(accepted_margin.clone());
                                                                            error.set(None);
                                                                        }
                                                                        _ => {}
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
                                                        inline: true, label: "Allow component body overhang",
                                                        id: check_id,
                                                        aria_label: "Allow component body overhang",
                                                        checked: if projection.outline.allow_body_overhang {
                                                                CheckboxState::Checked
                                                            } else {
                                                                CheckboxState::Unchecked
                                                            },
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
                                Collapsible {
                                    class: "grid gap-2.5 [&>label]:grid [&>label]:gap-[5px]",
                                    keep_mounted: true,
                                    open: Some(constraint_open()),
                                    on_open_change: move |next| constraint_open.set(next),
                                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Layout constraint",
                                        span { "Layout constraint" }
                                        small {
                                            if active_constraint.is_some() {
                                                "Active"
                                            } else {
                                                "Optional"
                                            }
                                        }
                                     ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                                    CollapsibleContent {
                                        class: "data-[open=true]:overflow-visible",
                                        hidden: !constraint_open(),
                                        inert: (!constraint_open()).then_some(""),

                                        if board_parts.iter().filter(|part| part.id != owner.part_id).count() > 0 {
                                            {
                                                let control_label = "Constraint type".to_owned();
                                                let control_aria_label = "Constraint type".to_owned();
                                                let control_id = format!(
                                                    "layout-choice-{}-{}",
                                                    dioxus::core::current_scope_id().0,
                                                    control_aria_label.replace(' ', "-"),
                                                );

                                                rsx! {
                                                    SelectField {
                                                        id: control_id, label: control_label, aria_label: control_aria_label,
                                                        value: kind_choice().unwrap_or_default(), class: "", touch_targets: false,
                                                        options: vec![
                                                            ("offset".into(), "Offset from part".into()),
                                                            ("mirror".into(), "Mirror placement across axis".into()),
                                                            ("mirror-line".into(), "Mirror line".into()),
                                                        ],
                                                        on_select: move |choice: String| {
                                                            constraint_kind.set(match choice.as_str() {
                                                                "mirror" => ConstraintKind::Mirror,
                                                                "mirror-line" => ConstraintKind::MirrorLine,
                                                                _ => ConstraintKind::Offset,
                                                            });
                                                        },
                                                    }
                                                }
                                            }
                                            {
                                                let control_label = "Source part".to_owned();
                                                let control_aria_label = "Constraint source part".to_owned();
                                                let control_id = format!(
                                                    "layout-choice-{}-{}",
                                                    dioxus::core::current_scope_id().0,
                                                    control_aria_label.replace(' ', "-"),
                                                );

                                                let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                                if !board_parts.iter().any(|part| part.id == source_part_id()) {
                                                    control_choices
                                                        .push((
                                                            source_part_id.to_string(),
                                                            constraint_source_reference.to_string(),
                                                            false,
                                                        ));
                                                }
                                                for part in board_parts.iter().filter(|part| part.id != owner.part_id) {
                                                    control_choices
                                                        .push((part.id.to_string(), part.reference.to_string(), false));
                                                }
                                                let on_value_change = EventHandler::new({
                let board_parts = board_parts.clone();
                move |choice: Option<String>| {
                    let Some(choice) = choice else {
                        return;
                    };
                    let value = choice;
                    if !has_active_constraint {
                        offset_axes.set(default_axes_for_source(&board_parts, &value));
                    }
                    source_part_id.set(value);
                }
            });
            rsx! {

                                                    Field { class: "",
                                                        FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                                        Combobox::<String> {
                                                            id: control_id,
                                                            aria_label: control_aria_label.clone(),
                                                            list_aria_label: control_aria_label.clone(),
                                                            value: Some(source_choice.into()),
                                                            disabled: false,
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
                                            if constraint_kind() == ConstraintKind::Offset {
                                                {
                                                    let control_label = "Offset axes".to_owned();
                                                    let control_aria_label = "Offset axes".to_owned();
                                                    let control_id = format!(
                                                        "layout-choice-{}-{}",
                                                        dioxus::core::current_scope_id().0,
                                                        control_aria_label.replace(' ', "-"),
                                                    );

                                                    rsx! {
                                                        SelectField {
                                                            id: control_id, label: control_label, aria_label: control_aria_label,
                                                            value: axes_choice().unwrap_or_default(), class: "", touch_targets: false,
                                                            options: vec![
                                                                (offset_axes_value(OffsetAxes::Board).to_owned(), "Offset on board axes".into()),
                                                                (offset_axes_value(OffsetAxes::Part).to_owned(), "Offset along part axes".into()),
                                                            ],
                                                            on_select: move |choice: String| choose_offset_axes(offset_axes_from_value(&choice)),
                                                        }

                                                    }
                                                }
                                                div { class: "grid grid-cols-2 gap-2",
                                                    {
                                                        let control_input_id = format!(
                                                            "layout-input-{}-7",
                                                            dioxus::core::current_scope_id().0,
                                                        );
                                                        rsx! {
                                                            NumericField {
                                                                id: control_input_id, label: "Offset X (mm)", class: "", aria_invalid: None,
                                                                control_class: "max-[760px]:h-control-touch",
                                                                aria_label: "Offset X (mm)",
                                                                value: "{offset_x}",
                                                                oninput: move |event: FormEvent| offset_x.set(event.value()),
                                                            }
                                                        }
                                                    }
                                                    {
                                                        let control_input_id = format!(
                                                            "layout-input-{}-6",
                                                            dioxus::core::current_scope_id().0,
                                                        );
                                                        rsx! {
                                                            NumericField {
                                                                id: control_input_id, label: "Offset Y (mm)", class: "", aria_invalid: None,
                                                                control_class: "max-[760px]:h-control-touch",
                                                                aria_label: "Offset Y (mm)",
                                                                value: "{offset_y}",
                                                                oninput: move |event: FormEvent| offset_y.set(event.value()),
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    let control_input_id = format!(
                                                        "layout-input-{}-5",
                                                        dioxus::core::current_scope_id().0,
                                                    );
                                                    rsx! {
                                                        NumericField {
                                                            id: control_input_id, label: "Rotation (degrees)", class: "", aria_invalid: None,
                                                            control_class: "max-[760px]:h-control-touch",
                                                            aria_label: "Rotation (degrees)",
                                                            value: "{rotation}",
                                                            oninput: move |event: FormEvent| rotation.set(event.value()),
                                                        }
                                                    }
                                                }
                                            } else if constraint_kind() == ConstraintKind::Mirror {
                                                {
                                                    let control_label = "Axis".to_owned();
                                                    let control_aria_label = "Mirror axis".to_owned();
                                                    let control_id = format!(
                                                        "layout-choice-{}-{}",
                                                        dioxus::core::current_scope_id().0,
                                                        control_aria_label.replace(' ', "-"),
                                                    );

                                                    rsx! {
                                                        SelectField {
                                                            id: control_id, label: control_label, aria_label: control_aria_label,
                                                            value: axis_choice().unwrap_or_default(), class: "", touch_targets: false,
                                                            options: vec![("vertical".into(), "Vertical".into()), ("horizontal".into(), "Horizontal".into())],
                                                            on_select: move |choice: String| {
                                                                mirror_axis.set(if choice == "horizontal" { MirrorAxis::Horizontal } else { MirrorAxis::Vertical });
                                                            },
                                                        }
                                                    }
                                                }
                                                {
                                                    let control_input_id = format!(
                                                        "layout-input-{}-4",
                                                        dioxus::core::current_scope_id().0,
                                                    );
                                                    rsx! {
                                                        NumericField {
                                                            id: control_input_id, label: "Axis coordinate (mm)", class: "", aria_invalid: None,
                                                            control_class: "max-[760px]:h-control-touch",
                                                            aria_label: "Axis coordinate (mm)",
                                                            value: "{mirror_coordinate}",
                                                            oninput: move |event: FormEvent| mirror_coordinate.set(event.value()),
                                                        }
                                                    }
                                                }
                                                p {
                                                    "Footprint geometry stays unchanged. Use a handed definition where needed."
                                                }
                                            } else {
                                                div { class: "grid grid-cols-2 gap-2",
                                                    {
                                                        let control_input_id = format!(
                                                            "layout-input-{}-3",
                                                            dioxus::core::current_scope_id().0,
                                                        );
                                                        rsx! {
                                                            NumericField {
                                                                id: control_input_id, label: "Line anchor X (mm)", class: "", aria_invalid: None,
                                                                control_class: "max-[760px]:h-control-touch",
                                                                aria_label: "Mirror line anchor X (mm)",
                                                                value: "{mirror_line_x}",
                                                                oninput: move |event: FormEvent| mirror_line_x.set(event.value()),
                                                            }
                                                        }
                                                    }
                                                    {
                                                        let control_input_id = format!(
                                                            "layout-input-{}-2",
                                                            dioxus::core::current_scope_id().0,
                                                        );
                                                        rsx! {
                                                            NumericField {
                                                                id: control_input_id, label: "Line anchor Y (mm)", class: "", aria_invalid: None,
                                                                control_class: "max-[760px]:h-control-touch",
                                                                aria_label: "Mirror line anchor Y (mm)",
                                                                value: "{mirror_line_y}",
                                                                oninput: move |event: FormEvent| mirror_line_y.set(event.value()),
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    let control_input_id = format!(
                                                        "layout-input-{}-1",
                                                        dioxus::core::current_scope_id().0,
                                                    );
                                                    rsx! {
                                                        NumericField {
                                                            id: control_input_id, label: "Line angle (degrees)", class: "", aria_invalid: None,
                                                            control_class: "max-[760px]:h-control-touch",
                                                            aria_label: "Mirror line angle (degrees)",
                                                            value: "{mirror_line_angle}",
                                                            oninput: move |event: FormEvent| mirror_line_angle.set(event.value()),
                                                        }
                                                    }
                                                }
                                                p {
                                                    "Footprint geometry stays unchanged. Use a handed definition where needed."
                                                }
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
                                }
                            }
                            if let Some(message) = x_failure()
                                .or_else(&*y_failure)
                                .or_else(&*margin_failure)
                                .or_else(&*error)
                            {
                                FieldError {  "{message}"  }
                            }
                            Button { variant: ButtonVariant::Secondary,
    class: "min-h-[34px] justify-self-start text-left",
    button_type: "button",
    onclick: {
                             let action = props.on_action;
                            move |_| action.call(LayoutComponentInspectorAction::NavigateElectrical { owner: latest_capture() })
                        },
                                "Edit electrical connections"
                             }
                        }
                        TabsContent { index: 1usize, value: "relations",
                            h2 { "Relationships" }
                            p { class: "text-xs leading-normal",
                                "{projection.relationship_summary}"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                button_type: "button",
                                onclick: move |_| tab.set(LayoutInspectorTab::Properties),
                                "Go to Properties"
                            }
                            p { class: "text-xs leading-[1.45] text-muted-foreground",
                                "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties."
                            }
                        }
                     }
                }
            }

    }};
}

pub(super) use render_inspector;
