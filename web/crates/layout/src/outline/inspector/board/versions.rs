use super::*;
use crate::outline::OutlineVersionChoice;
use boardstudio_web_ui_shared::inspector_fields::{SelectField, TextField};

pub(super) fn version_controls(
    view: &BoardInspectorView,
    active_choice: Option<&OutlineVersionChoice>,
    active_ownership: OutlineVersionOwnership,
) -> Element {
    let projection = &view.projection;
    let active_selection = view.active_selection;
    let mut version_ui = projection.ui.version_ui;
    let name_draft = version_ui.read().version_name_draft.clone();
    let active_version = projection.active_version_id.clone();
    let accepted_name = projection.version_name.clone();
    let name_failure = version_ui.read().version_name_failure.clone();
    let enabled = projection.enabled;
    let active_problem = active_choice.filter(|item| item.problem_message.is_some());
    // Alternatives that cannot resolve are listed without pretending the active outline
    // is blocked: Core decides the affected exports and the current output stays valid.
    let blocked_alternatives = projection
        .versions
        .iter()
        .filter(|item| item.problem_message.is_some())
        .filter(|item| Some(&item.id) != projection.active_version_id.as_ref())
        .cloned()
        .collect::<Vec<_>>();
    rsx! {
        SelectField {
            label: "Active outline".to_owned(),
            value: active_selection().unwrap_or_default(),
            options: std::iter::once(("".to_owned(), "Generated".to_owned()))
                .chain(
                    projection
                        .versions
                        .iter()
                        .map(|item| (item.id.clone(), item.label())),
                )
                .collect(),
            aria_label: "Active outline",
            disabled: !enabled,
            on_commit: {
                let projection = projection.clone();
                move |value: String| {
                    projection.dispatch(OutlineActionKind::Activate {
                        version_id: (!value.is_empty()).then_some(value),
                    });
                }
            },
        }
        if let Some(version_id) = active_version.as_ref() {
            {
                let control_input_id = format!(
                    "layout-input-{}-1",
                    dioxus::core::current_scope_id().0,
                );
                rsx! {
                    TextField {
                        id: control_input_id, label: "Version name", aria_label: "Outline version name",
                        class: "grid gap-[5px] text-xs text-muted-foreground", control_class: "",
                        maxlength: Some("120".to_owned()), aria_invalid: Some(name_draft.trim().is_empty()),
                        value: "{name_draft}", disabled: !enabled,
                        oninput: move |event: FormEvent| version_ui.write().version_name_draft = event.value(),
                        onblur: {
                            let version_id = version_id.clone();
                            let accepted_name = accepted_name.clone();
                            let projection = projection.clone();
                            move |_| {
                                let name = version_ui.read().version_name_draft.trim().to_owned();
                                version_ui.write().version_name_draft = name.clone();
                                if !name.is_empty() && name.len() <= 120 && name != accepted_name {
                                    projection.update(OutlineEdit::RenameVersion { version_id: version_id.clone(), name });
                                }
                            }
                        },
                        onkeydown: {
                            let accepted_name = accepted_name.clone();
                            move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                "Enter" => {
                                    event.prevent_default();
                                    if let Some(input) = event.data().try_as_web_event()
                                        .and_then(|event| event.target())
                                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    { let _ = input.blur(); }
                                }
                                "Escape" => {
                                    event.prevent_default();
                                    version_ui.write().version_name_draft = accepted_name.clone();
                                }
                                _ => {}
                            }
                        },
                    }
                }
            }
            if let Some(message) = name_failure.as_ref() {
                    FieldError { "{message}" }
            }
        }
        if let Some(problem) = active_problem {
            div {
                "data-finding-id": "{problem.problem_finding_id.clone().unwrap_or_default()}",
                "data-ownership": "{active_ownership.label()}",
                    Alert { variant: AlertVariant::Destructive,
                        AlertDescription {
                p { "{problem.problem_message.clone().unwrap_or_default()}" }
                p {
                    if !problem.blocked_exports.is_empty() {
                        "Disabled while this refinement cannot resolve: {problem.blocked_exports.join(\", \")}. Saving and further editing stay available."
                    } else {
                        "This refinement is not selected, so it does not block current output. Saving and further editing stay available."
                    }
                }
                if problem.problem_selectable {
                    p {
                        "Select the located finding on the canvas to inspect it."
                    }
                }
            }
        }
                }
            }
        for blocked in blocked_alternatives.iter() {
                div {
                "data-finding-id": "{blocked.problem_finding_id.clone().unwrap_or_default()}",
                "data-ownership": "{blocked.ownership.label()}",
                    Callout { variant: CalloutVariant::Info,
                "{blocked.ownership.label()} \u{201c}{blocked.name}\u{201d}: {blocked.problem_message.clone().unwrap_or_default()}"
                    }
            }
        }
        if !projection.has_generated && projection.active_version_id.is_none() {
            Button { variant: ButtonVariant::Secondary,
    disabled: !enabled,
    onclick: {
            let projection = projection.clone();
            move |_| projection.update(OutlineEdit::CreateAutomatic)
        },

                "Generate automatic outline"

             }
        }
    }
}
