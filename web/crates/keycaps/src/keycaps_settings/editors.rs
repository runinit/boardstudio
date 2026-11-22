use super::retry_drafts::retry_identity;
use super::*;
use boardstudio_web_ui_shared::inspector_fields::{
    DraftNumericField as NumericField, DraftSelectField as SelectField, InspectorSection, TextField,
};

#[component]
fn SettingsSection(
    label: String,
    #[props(default)] class: String,
    #[props(default)] default_open: bool,
    children: Element,
) -> Element {
    let mut open = use_signal(|| default_open);
    let _ = class;
    rsx! {
        InspectorSection {
            title: "{label}",
            open: open(),
            on_open_change: move |value: bool| open.set(value),
            {children}
        }
    }
}

#[component]
fn SettingsChoice(
    label: String,
    aria_label: String,
    value: String,
    options: Vec<(String, String)>,
    on_change: EventHandler<String>,
) -> Element {
    rsx! {
        SelectField {
            label,
            aria_label: Some(aria_label),
            value,
            options,
            on_select: move |value: String| on_change.call(value),
        }
    }
}

fn settings_options(first: &str, remaining: &[(&str, &str)]) -> Vec<(String, String)> {
    std::iter::once((String::new(), first.to_owned()))
        .chain(
            remaining
                .iter()
                .map(|(value, label)| ((*value).to_owned(), (*label).to_owned())),
        )
        .collect()
}

#[component]
fn SettingsInput(
    label: String,
    aria_label: String,
    value: String,
    r#type: InputType,
    #[props(default)] min: Option<String>,
    #[props(default)] max: Option<String>,
    #[props(default)] step: Option<String>,
    #[props(default)] placeholder: Option<String>,
    oninput: EventHandler<FormEvent>,
) -> Element {
    let id = format!("keycaps-input-{}", dioxus::core::current_scope_id().0);
    if r#type == InputType::Number {
        return rsx! {
            NumericField {
                id,
                label,
                aria_label: Some(aria_label),
                value,
                min,
                max,
                step: step.unwrap_or_else(|| "any".to_owned()),
                placeholder,
                oninput,
            }
        };
    }
    rsx! {
        TextField {
            id,
            label,
            aria_label: Some(aria_label),
            r#type,
            value,
            placeholder,
            oninput,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsBoardSettingsEditorProps {
    pub settings: KeycapBoardSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsBoardSettingsEditor(props: KeycapsBoardSettingsEditorProps) -> Element {
    let settings = props.settings;
    let actions = props.actions;
    let target = KeycapsEditTarget::Board;
    let baseline = KeycapsEditBaseline::Board(settings.clone());
    let retries = actions
        .retry_drafts
        .iter()
        .filter(|draft| draft.target == target)
        .cloned()
        .collect::<Vec<_>>();
    let color_actions = actions.clone();
    let color_target = target.clone();
    let color_baseline = baseline.clone();
    let legend_actions = actions.clone();
    let legend_target = target.clone();
    let legend_baseline = baseline.clone();
    let clearance_actions = actions.clone();
    let clearance_target = target.clone();
    let clearance_baseline = baseline.clone();
    rsx! {
        SettingsSection { class: "min-w-0", label: "Board colors", default_open: true,
            section { class: "grid min-w-0 gap-2 p-3 text-foreground", aria_label: "Board colors",
                SettingsInput { label: "Keycap color", aria_label: "Board keycap color", r#type: InputType::Color, value: settings.color.clone(),
                    oninput: move |event: FormEvent| emit_settings_change(&color_actions, color_target.clone(), color_baseline.clone(), KeycapEditChange::BoardColor(event.value()))
                }
                SettingsInput { label: "Legend color", aria_label: "Board legend color", r#type: InputType::Color, value: settings.legend_color.clone(),
                    oninput: move |event: FormEvent| emit_settings_change(&legend_actions, legend_target.clone(), legend_baseline.clone(), KeycapEditChange::BoardLegendColor(event.value()))
                }
                SettingsInput { label: "Minimum clearance (mm)", aria_label: "Keycap clearance", r#type: InputType::Number,
                    min: Some("0".into()), max: Some("5".into()), step: Some("0.1".into()), value: settings.clearance.to_string(),
                    oninput: move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() { emit_settings_change(&clearance_actions, clearance_target.clone(), clearance_baseline.clone(), KeycapEditChange::BoardClearance(value)); }
                }
                {settings_feedback(retries, &actions)}
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsMatrixSettingsEditorProps {
    pub matrix_id: String,
    pub matrix_name: String,
    pub settings: KeycapMatrixSettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsMatrixSettingsEditor(props: KeycapsMatrixSettingsEditorProps) -> Element {
    let target = KeycapsEditTarget::Matrix(props.matrix_id);
    let settings = props.settings;
    let actions = props.actions;
    let baseline = KeycapsEditBaseline::Matrix(settings.clone());
    let retries = actions
        .retry_drafts
        .iter()
        .filter(|draft| draft.target == target)
        .cloned()
        .collect::<Vec<_>>();
    let profile = settings.profile.map_or("", profile_value);
    let mount = settings.mount.map_or("", mount_value);
    let profile_actions = actions.clone();
    let profile_target = target.clone();
    let profile_baseline = baseline.clone();
    let row_actions = actions.clone();
    let row_target = target.clone();
    let row_baseline = baseline.clone();
    let wall_actions = actions.clone();
    let wall_target = target.clone();
    let wall_baseline = baseline.clone();
    let mount_actions = actions.clone();
    let mount_target = target.clone();
    let mount_baseline = baseline.clone();
    rsx! {
        fieldset { class: "m-0 mb-2 min-w-0 border-0 p-2",
            legend { class: "px-1 text-sm text-foreground", "{props.matrix_name}" }
            SettingsChoice { label: "Profile", aria_label: format!("Keycap profile for {}", props.matrix_name), value: profile.to_owned(),
                options: settings_options("No generated keycap", PROFILES),
                on_change: move |value: String| emit_settings_change(&profile_actions, profile_target.clone(), profile_baseline.clone(), KeycapEditChange::MatrixProfile(parse_profile(&value))),
            }
            SettingsSection { label: "Profile dimensions & socket", default_open: false,
                SettingsInput { label: "First profile row", aria_label: format!("First profile row for {}", props.matrix_name), r#type: InputType::Number,
                    min: Some("1".into()), max: Some("5".into()), step: Some("1".into()), value: settings.first_row.to_string(),
                    oninput: move |event: FormEvent| if let Ok(value) = event.value().parse::<u8>() { emit_settings_change(&row_actions, row_target.clone(), row_baseline.clone(), KeycapEditChange::MatrixFirstRow(value)); }
                }
                SettingsInput { label: "Wall thickness (mm)", aria_label: format!("Wall thickness for {}", props.matrix_name), r#type: InputType::Number,
                    min: Some("0.8".into()), max: Some("2".into()), step: Some("0.1".into()), value: settings.wall_thickness.to_string(),
                    oninput: move |event: FormEvent| if let Ok(value) = event.value().parse::<f64>() { emit_settings_change(&wall_actions, wall_target.clone(), wall_baseline.clone(), KeycapEditChange::MatrixWallThickness(value)); }
                }
                SettingsChoice { label: "Socket", aria_label: format!("Keycap socket for {}", props.matrix_name), value: mount.to_owned(),
                    options: settings_options("From switch profile", MOUNTS),
                    on_change: move |value: String| emit_settings_change(&mount_actions, mount_target.clone(), mount_baseline.clone(), KeycapEditChange::MatrixMount(parse_mount(&value))),
                }
            }
            {settings_feedback(retries, &actions)}
        }
    }
}

fn settings_feedback(retries: Vec<KeycapsRetryDraft>, actions: &KeycapsSettingsActions) -> Element {
    rsx! {
        KeycapsRetryList { retries, on_retry: actions.on_retry, on_discard: actions.on_discard }
    }
}

/// Shared retry drafts list: one Alert with Retry/Discard per unaccepted change.
/// Board, matrix and key editors share this table so their form rows stay thin.
#[component]
fn KeycapsRetryList(
    retries: Vec<KeycapsRetryDraft>,
    on_retry: EventHandler<KeycapsRetryIdentity>,
    on_discard: EventHandler<KeycapsRetryIdentity>,
) -> Element {
    rsx! {
        for draft in retries {
            Alert { variant: AlertVariant::Destructive,
                p { "Unaccepted {field_label(&draft.field)} change: {change_label(&draft.change)}. {draft.message}" }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: { let id = retry_identity(&draft); move |_| on_retry.call(id.clone()) }, "Retry" }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: { let id = retry_identity(&draft); move |_| on_discard.call(id.clone()) }, "Discard" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct KeycapsSettingsEditorProps {
    pub selected: SelectedKeySettings,
    pub actions: KeycapsSettingsActions,
}

#[component]
pub fn KeycapsSettingsEditor(props: KeycapsSettingsEditorProps) -> Element {
    let selected = props.selected;
    let actions = props.actions;
    let settings = selected.settings;
    let key = selected.key;
    let accepted_legend = settings.legend.clone();
    let accepted_color = settings.color.clone();
    let accepted_profile = settings.profile;
    let accepted_mount = settings.mount;
    let accepted_row = settings.row;
    let accepted_units = settings.units;
    // The controller provides the legend Signals; an editor mounted without one keeps its own.
    let own_draft = use_signal(String::new);
    let own_failure = use_signal(|| None::<String>);
    let edits_context = try_consume_context::<KeycapsEditsContext>();
    let mut legend_draft = edits_context.map_or(own_draft, |context| context.legend_draft);
    let mut legend_failure = edits_context.map_or(own_failure, |context| context.legend_failure);
    let mut legend_dirty = use_signal(|| false);
    let legend_owner = (
        actions.editor_instance_id,
        actions.scope.clone(),
        actions.scope_generation,
        actions.selection_generation,
        key.id.to_string(),
    );
    use_effect(use_reactive((&legend_owner,), {
        let mut legend_draft = legend_draft;
        let legend = accepted_legend.clone().unwrap_or_default();
        move |_| {
            legend_dirty.set(false);
            legend_draft.set(legend.clone());
        }
    }));
    use_effect(use_reactive((&accepted_legend,), move |(accepted,)| {
        if !legend_dirty() {
            legend_draft.set(accepted.unwrap_or_default());
        }
    }));
    let request = |change: KeycapEditChange,
                   request_sequence: &mut Signal<u64>,
                   on_change: EventHandler<KeycapsEditRequest>,
                   actions: &KeycapsSettingsActions| {
        let Some(request_id) = request_sequence().checked_add(1) else {
            return;
        };
        request_sequence.set(request_id);
        on_change.call(KeycapsEditRequest {
            scope: actions.scope.clone(),
            scope_generation: actions.scope_generation,
            selection_generation: actions.selection_generation,
            editor_instance_id: actions.editor_instance_id,
            request_id,
            admission_token: actions.token,
            admission_revision: actions.revision,
            target: KeycapsEditTarget::Key(actions.selected_key_id.clone()),
            baseline: KeycapsEditBaseline::Key(actions.accepted_settings.clone()),
            change,
        });
    };
    let profile = accepted_profile.map_or("", profile_value);
    let mount = accepted_mount.map_or("", mount_value);
    let width = accepted_units.map_or_else(String::new, |units| units.x.to_string());
    let depth = accepted_units.map_or_else(String::new, |units| units.y.to_string());
    let retry_drafts = actions.retry_drafts.clone();
    let legend_retry_pending = retry_drafts.iter().any(|draft| {
        draft.field == KeycapEditField::Legend
            && draft.editor_instance_id == actions.editor_instance_id
            && draft.scope == actions.scope
            && draft.scope_generation == actions.scope_generation
            && draft.selection_generation == actions.selection_generation
            && draft.target == KeycapsEditTarget::Key(actions.selected_key_id.clone())
    });
    let on_retry = actions.on_retry;
    let on_discard = actions.on_discard;
    let mut legend_sequence = actions.request_sequence;
    let mut inherit_sequence = actions.request_sequence;
    let mut blank_sequence = actions.request_sequence;
    let mut color_sequence = actions.request_sequence;
    let mut reset_color_sequence = actions.request_sequence;
    let mut profile_sequence = actions.request_sequence;
    let mut mount_sequence = actions.request_sequence;
    let mut row_sequence = actions.request_sequence;
    let mut width_sequence = actions.request_sequence;
    let mut depth_sequence = actions.request_sequence;
    let legend_actions = actions.clone();
    let inherit_actions = actions.clone();
    let blank_actions = actions.clone();
    let color_actions = actions.clone();
    let reset_color_actions = actions.clone();
    let profile_actions = actions.clone();
    let mount_actions = actions.clone();
    let row_actions = actions.clone();
    let width_actions = actions.clone();
    let depth_actions = actions.clone();
    let legend_change = actions.on_change;
    let inherit_change = actions.on_change;
    let blank_change = actions.on_change;
    let color_change = actions.on_change;
    let reset_color_change = actions.on_change;
    let profile_change = actions.on_change;
    let mount_change = actions.on_change;
    let row_change = actions.on_change;
    let width_change = actions.on_change;
    let depth_change = actions.on_change;
    let legend_id = format!("keycaps-legend-{}", dioxus::core::current_scope_id().0);
    let legend_error = legend_failure();
    rsx! {
        section { class: "grid min-w-0 gap-2 text-foreground", aria_label: "Keycap overrides for {key.reference}",
            TextField {
                id: legend_id,
                label: "Legend",
                aria_label: Some(format!("Legend for {}", key.reference)),
                value: legend_draft(),
                placeholder: Some(if key.binding_label.is_empty() { "From binding".to_owned() } else { key.binding_label.to_string() }),
                maxlength: Some("12".to_owned()),
                error: legend_error.clone(),
                oninput: move |event: FormEvent| { legend_dirty.set(true); legend_failure.set(None); legend_draft.set(event.value()); },
                onblur: move |_| {
                    let value = legend_draft();
                    if legend_dirty() || legend_blur_requires_reconciliation(&value, accepted_legend.as_deref(), legend_retry_pending) {
                        legend_dirty.set(false);
                        let change = if legend_change_required(&value, accepted_legend.as_deref()) {
                            KeycapEditChange::Legend(Some(value))
                        } else {
                            KeycapEditChange::Legend(accepted_legend.clone())
                        };
                        request(change, &mut legend_sequence, legend_change, &legend_actions);
                    }
                },
            }
            div { class: "flex flex-wrap gap-1.5",
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| { legend_draft.set(String::new()); request(KeycapEditChange::Legend(None), &mut inherit_sequence, inherit_change, &inherit_actions) }, "Use binding legend" }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| { legend_draft.set(String::new()); request(KeycapEditChange::Legend(Some(String::new())), &mut blank_sequence, blank_change, &blank_actions) }, "Blank keycap" }
            }
            SettingsInput { label: "Keycap color", aria_label: format!("Keycap color for {}", key.reference), r#type: InputType::Color,
                value: accepted_color.as_deref().unwrap_or(&selected.board_color).to_owned(),
                oninput: move |event: FormEvent| request(KeycapEditChange::Color(Some(event.value())), &mut color_sequence, color_change, &color_actions),
            }
            if accepted_color.is_some() {
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", onclick: move |_| request(KeycapEditChange::Color(None), &mut reset_color_sequence, reset_color_change, &reset_color_actions), "Use board color" }
            }
            SettingsSection { label: "Keycap overrides", default_open: false,
                SettingsChoice {
                    label: "Profile", aria_label: format!("Profile override for {}", key.reference), value: profile.to_owned(),
                    options: settings_options("From matrix", PROFILES),
                    on_change: move |value: String| request(KeycapEditChange::Profile(parse_profile(&value)), &mut profile_sequence, profile_change, &profile_actions),
                }
                SettingsChoice {
                    label: "Socket", aria_label: format!("Socket override for {}", key.reference), value: mount.to_owned(),
                    options: settings_options("From matrix / switch", MOUNTS),
                    on_change: move |value: String| request(KeycapEditChange::Mount(parse_mount(&value)), &mut mount_sequence, mount_change, &mount_actions),
                }
                SettingsInput { label: "Profile row", aria_label: format!("Profile row for {}", key.reference), r#type: InputType::Number,
                    min: Some("1".into()), max: Some("5".into()),
                    value: accepted_row.map_or_else(String::new, |row| row.to_string()), placeholder: Some("From matrix".into()),
                        oninput: move |event: FormEvent| {
                            let value = event.value();
                            if value.is_empty() {
                                request(KeycapEditChange::Row(None), &mut row_sequence, row_change, &row_actions);
                            } else if let Ok(row) = value.parse::<u8>() {
                                request(KeycapEditChange::Row(Some(row)), &mut row_sequence, row_change, &row_actions);
                            }
                        },
                }
                SettingsInput { label: "Width (u)", aria_label: format!("Keycap width for {}", key.reference), r#type: InputType::Number,
                    min: Some("0.75".into()), max: Some("7".into()), step: Some("0.25".into()),
                    value: width, placeholder: Some("From envelope".into()),
                        oninput: move |event: FormEvent| {
                            let value = event.value();
                            if value.is_empty() {
                            request(KeycapEditChange::ClearUnits(KeycapEditField::UnitsWidth), &mut width_sequence, width_change, &width_actions);
                            } else if let Ok(width) = value.parse::<f64>() {
                                request(KeycapEditChange::UnitsWidth(width), &mut width_sequence, width_change, &width_actions);
                            }
                        },
                }
                SettingsInput { label: "Depth (u)", aria_label: format!("Keycap depth for {}", key.reference), r#type: InputType::Number,
                    min: Some("0.75".into()), max: Some("7".into()), step: Some("0.25".into()),
                    value: depth, placeholder: Some("From envelope".into()),
                        oninput: move |event: FormEvent| {
                            let value = event.value();
                            if value.is_empty() {
                            request(KeycapEditChange::ClearUnits(KeycapEditField::UnitsDepth), &mut depth_sequence, depth_change, &depth_actions);
                            } else if let Ok(depth) = value.parse::<f64>() {
                                request(KeycapEditChange::UnitsDepth(depth), &mut depth_sequence, depth_change, &depth_actions);
                            }
                        },
                }
            }
            KeycapsRetryList { retries: retry_drafts.iter().cloned().collect::<Vec<_>>(), on_retry, on_discard }
        }
    }
}
