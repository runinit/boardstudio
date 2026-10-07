//! Layout presentation for the existing Core-owned Rhai script workflow.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Lifecycle, Resolution, SessionEpoch,
};
use boardstudio_core::model::{EditOperation, Finding, Script, Severity};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::rc::Rc;

fn script_commit(document: boardstudio_core::model::ProjectDoc, script_id: &str) -> Resolution {
    Resolution::submit(
        vec![script_id.to_owned()],
        EditOperation::ReplaceDocument {
            document: Box::new(document),
        },
    )
}

/// Resolve "+ New script": the script is appended to the accepted document when the edit
/// runs, so its default name counts the scripts accepted by then.
fn new_script_resolver(script_id: String) -> EditResolver {
    EditResolver::new("geometry-script-new", move |accepted: &AcceptedSnapshot| {
        if accepted
            .document
            .scripts
            .iter()
            .any(|script| script.id == script_id)
        {
            return Resolution::Retire("That script already exists.".into());
        }
        let mut next = accepted.document.as_ref().clone();
        next.scripts.push(Script {
            id: script_id.clone(),
            name: format!("Script {}", next.scripts.len() + 1),
            source: String::new(),
            enabled: false,
        });
        script_commit(next, &script_id)
    })
}

/// Resolve "Apply script": the typed name, source and flag are applied to the accepted
/// script, which must still exist.
fn apply_script_resolver(
    script_id: String,
    name: String,
    source: String,
    enabled: bool,
) -> EditResolver {
    EditResolver::new(
        "geometry-script-apply",
        move |accepted: &AcceptedSnapshot| {
            let mut next = accepted.document.as_ref().clone();
            let Some(script) = next.scripts.iter_mut().find(|item| item.id == script_id) else {
                return Resolution::Retire("That script no longer exists.".into());
            };
            if script.name == name && script.source == source && script.enabled == enabled {
                return Resolution::Unchanged;
            }
            script.name = name.clone();
            script.source = source.clone();
            script.enabled = enabled;
            script_commit(next, &script_id)
        },
    )
}

#[component]
pub fn GeometryScriptsEditor(on_back: EventHandler<()>) -> Element {
    let _ = use_context::<Signal<u64>>()();
    let runtime = use_context::<Rc<Runtime>>();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return rsx! {};
    };
    let document = snapshot.document.clone();
    let scripts = document.scripts.clone();
    let active = use_signal(String::new);
    let mut active = active;
    let mut name = use_signal(String::new);
    let mut source = use_signal(String::new);
    let mut enabled = use_signal(|| true);
    let script_ticket = use_signal(|| None::<EditTicket>);
    let script_pending = script_ticket
        .read()
        .as_ref()
        .is_some_and(EditTicket::is_pending);
    let ticket_error =
        script_ticket
            .read()
            .as_ref()
            .and_then(|ticket| match ticket.settlement(true) {
                Settlement::Failed { message } => Some(message),
                _ => None,
            });
    let active_script = scripts.iter().find(|script| script.id == active());
    let active_identity = active_script.map(|script| {
        (
            script.id.clone(),
            script.name.clone(),
            script.source.clone(),
            script.enabled,
        )
    });
    let draft_owner = (snapshot.document.id.clone(), snapshot.session_epoch);
    let accepted_identity = (
        draft_owner.0.clone(),
        draft_owner.1,
        active_identity.clone(),
    );
    let observed_identity = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            None::<(String, SessionEpoch, Option<(String, String, String, bool)>)>,
        ))
    });
    use_effect(use_reactive((&accepted_identity,), {
        let mut name = name;
        let mut source = source;
        let mut enabled = enabled;
        let observed_identity = observed_identity.clone();
        move |(identity,)| {
            let owner_changed = observed_identity.borrow().as_ref() != Some(&identity);
            *observed_identity.borrow_mut() = Some(identity.clone());
            match identity.2 {
                Some((_, accepted_name, accepted_source, accepted_enabled)) => {
                    if owner_changed {
                        name.set(accepted_name);
                        source.set(accepted_source);
                        enabled.set(accepted_enabled);
                    }
                }
                None => {
                    if owner_changed {
                        name.set(String::new());
                        source.set(String::new());
                        enabled.set(true);
                    }
                }
            }
        }
    }));
    let can_edit = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    );
    let current_error = model.last_error.clone();
    let current_findings = snapshot.scene.findings.clone();
    let on_back_key = on_back;
    rsx! {
        section {
            class: "m1-geometry-scripts",
            aria_label: "Geometry scripts",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == "Escape" {
                    event.prevent_default();
                    event.stop_propagation();
                    on_back_key.call(());
                }
            },
            header { class: "m1-geometry-scripts-heading",
                button { r#type: "button", class: "m1-geometry-scripts-back", onclick: move |_| on_back.call(()), "Back to selection" }
                h2 { "Geometry scripts" }
            }
            div { class: "m1-geometry-scripts-list-heading",
                h3 { "Scripts" }
                button {
                    r#type: "button",
                    disabled: !can_edit || script_pending,
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let mut script_ticket = script_ticket;
                        let mut active = active;
                        let mut name = name;
                        let mut source = source;
                        let mut enabled = enabled;
                        move |_| {
                            if !current_session_can_accept_edits(&runtime, &snapshot) {
                                return;
                            }
                            let Ok(script_identity) = crate::runtime::new_project_id() else {
                                return;
                            };
                            let script_id = format!("geometry-script-{script_identity}");
                            active.set(script_id.clone());
                            name.set(format!("Script {}", snapshot.document.scripts.len() + 1));
                            source.set(String::new());
                            enabled.set(false);
                            script_ticket.set(Some(EditTicket::begin(
                                &runtime,
                                "geometry-script-new",
                                Some("script".into()),
                                new_script_resolver(script_id),
                            )));
                        }
                    },
                    "+ New script"
                }
            }
            if !scripts.is_empty() {
                label { class: "m1-geometry-script-field",
                    "Active script"
                    select {
                        aria_label: "Active script",
                        value: "{active()}",
                        disabled: !can_edit,
                        onchange: {
                            let scripts = scripts.clone();
                            move |event| {
                                let id = event.value();
                                active.set(id.clone());
                                if let Some(script) = scripts.iter().find(|script| script.id == id) {
                                    name.set(script.name.clone());
                                    source.set(script.source.clone());
                                    enabled.set(script.enabled);
                                } else {
                                    name.set(String::new());
                                    source.set(String::new());
                                    enabled.set(true);
                                }
                            }
                        },
                        option { value: "", "Select a script" }
                        for script in scripts.iter() {
                            option { value: "{script.id}", "{script.name}" }
                        }
                    }
                }
            }
            if let Some(script) = active_script {
                label { class: "m1-geometry-script-field",
                    "Name"
                    input {
                        aria_label: "Name",
                        value: "{name()}",
                        disabled: !can_edit,
                        oninput: move |event| name.set(event.value()),
                    }
                }
                label { class: "m1-geometry-script-field",
                    "Rhai source"
                    textarea {
                        aria_label: "Rhai source",
                        spellcheck: "false",
                        value: "{source()}",
                        placeholder: "// Describe generated geometry and component groups",
                        disabled: !can_edit,
                        oninput: move |event| source.set(event.value()),
                    }
                }
                label { class: "m1-geometry-script-enabled",
                    input {
                        r#type: "checkbox",
                        checked: enabled(),
                        disabled: !can_edit,
                        onchange: move |event| enabled.set(event.checked()),
                    }
                    "Enable on Apply"
                }
                button {
                    r#type: "button",
                    class: "m1-geometry-script-apply",
                    disabled: !can_edit || script_pending || source().trim().is_empty(),
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let script_id = script.id.clone();
                        let mut script_ticket = script_ticket;
                        move |_| {
                            if !current_session_can_accept_edits(&runtime, &snapshot) {
                                return;
                            }
                            script_ticket.set(Some(EditTicket::begin(
                                &runtime,
                                "geometry-script-apply",
                                Some("script".into()),
                                apply_script_resolver(
                                    script_id.clone(),
                                    name(),
                                    source(),
                                    enabled(),
                                ),
                            )));
                        }
                    },
                    "Apply script"
                }
                if let Some(error) = ticket_error.as_ref().or(current_error.as_ref()) {
                    p { role: "alert", class: "m1-geometry-script-error", "{error}" }
                }
                ScriptFindings { findings: current_findings }
            } else {
                p { class: "m1-geometry-scripts-empty", "Scripts generate named geometry groups. Apply a script to run it in the core." }
            }
        }
    }
}

#[component]
fn ScriptFindings(findings: Vec<Finding>) -> Element {
    rsx! {
        section { class: "m1-geometry-script-findings", aria_label: "Core findings",
            h3 { "Core findings" }
            if findings.is_empty() {
                p { "No active findings" }
            } else {
                ul {
                    for finding in findings {
                        li { class: severity_class(&finding.severity),
                            strong { "{severity_label(&finding.severity)}" }
                            p { "{finding.message}" }
                        }
                    }
                }
            }
        }
    }
}

/// The panel's snapshot must still belong to the open project session; the edit itself
/// resolves against whatever is accepted when it runs.
fn current_session_can_accept_edits(runtime: &Runtime, snapshot: &AcceptedSnapshot) -> bool {
    let model = runtime.model();
    matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.accepted.as_ref().is_some_and(|accepted| {
        accepted.session_epoch == snapshot.session_epoch
            && accepted.document.id == snapshot.document.id
    })
}

fn severity_class(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "is-error",
        Severity::Warning => "is-warning",
        Severity::Info => "is-info",
    }
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Information",
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod queued_script_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::{Board, ProjectDoc};
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("script-doc", "Scripts");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.scripts.push(Script {
            id: "script-1".into(),
            name: "Script 1".into(),
            source: String::new(),
            enabled: false,
        });
        document
    }

    async fn settle(runtime: &Rc<Runtime>, ticket: &EditTicket) {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if !ticket.is_pending() {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
    }

    #[wasm_bindgen_test]
    async fn script_apply_queued_behind_an_unrelated_edit_keeps_that_edit() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document()).await;
        let accepted = runtime.model().accepted.expect("the fixture opens");
        // Hold the unrelated edit's reply so Apply queues behind it.
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut renamed = accepted.document.as_ref().clone();
        renamed.boards[0].name = "Renamed".into();
        crate::runtime::project_name_test_support::submit_fixed_command(
            &runtime,
            EditCommand {
                base_revision: accepted.document.revision,
                transaction_id: "rename-board".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["board".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(renamed),
                },
            },
        );
        support::drive_pending(&runtime);
        entered.await.expect("the rename reached Core");
        let apply = EditTicket::begin(
            &runtime,
            "geometry-script-apply",
            Some("script".into()),
            apply_script_resolver("script-1".into(), "Frame".into(), "// frame".into(), true),
        );
        assert!(
            apply.is_pending(),
            "the one-shot control stays disabled while pending"
        );
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        settle(&runtime, &apply).await;

        assert!(matches!(apply.settlement(true), Settlement::Landed { .. }));
        let document = runtime.model().accepted.unwrap().document;
        assert_eq!(
            document.boards[0].name, "Renamed",
            "the unrelated edit survived"
        );
        assert_eq!(document.scripts[0].name, "Frame");
        assert_eq!(document.scripts[0].source, "// frame");
        assert!(document.scripts[0].enabled);
    }

    #[wasm_bindgen_test]
    async fn two_queued_new_scripts_both_exist_and_apply_on_a_deleted_script_retires() {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document()).await;
        let first = EditTicket::begin(
            &runtime,
            "geometry-script-new",
            Some("script".into()),
            new_script_resolver("script-a".into()),
        );
        let second = EditTicket::begin(
            &runtime,
            "geometry-script-new",
            Some("script".into()),
            new_script_resolver("script-b".into()),
        );
        settle(&runtime, &second).await;
        assert!(matches!(first.settlement(true), Settlement::Landed { .. }));
        assert!(matches!(second.settlement(true), Settlement::Landed { .. }));
        let scripts = runtime.model().accepted.unwrap().document.scripts.clone();
        assert_eq!(scripts.len(), 3);
        assert_eq!(scripts[1].name, "Script 2");
        assert_eq!(scripts[2].name, "Script 3");

        let missing = EditTicket::begin(
            &runtime,
            "geometry-script-apply",
            Some("script".into()),
            apply_script_resolver("gone".into(), "x".into(), "y".into(), true),
        );
        settle(&runtime, &missing).await;
        assert!(matches!(
            missing.settlement(true),
            Settlement::Failed { ref message } if message.contains("no longer exists")
        ));
    }
}
