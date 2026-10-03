//! Layout presentation for the existing Core-owned Rhai script workflow.
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, Lifecycle};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Finding, ProjectDoc, Script, Severity,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub(super) fn GeometryScriptsEditor(on_back: EventHandler<()>) -> Element {
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
    let active_script = scripts.iter().find(|script| script.id == active());
    let active_identity = active_script.map(|script| {
        (
            script.id.clone(),
            script.name.clone(),
            script.source.clone(),
            script.enabled,
        )
    });
    use_effect(use_reactive((&active_identity,), {
        let mut name = name;
        let mut source = source;
        let mut enabled = enabled;
        move |(identity,)| match identity {
            Some((_, accepted_name, accepted_source, accepted_enabled)) => {
                name.set(accepted_name);
                source.set(accepted_source);
                enabled.set(accepted_enabled);
            }
            None => {
                name.set(String::new());
                source.set(String::new());
                enabled.set(true);
            }
        }
    }));
    let can_edit = model.lifecycle == Lifecycle::Ready;
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
                    disabled: !can_edit,
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let active = active;
                        let mut name = name;
                        let mut source = source;
                        let mut enabled = enabled;
                        move |_| {
                            if !current_snapshot_is_ready(&runtime, &snapshot) {
                                return;
                            }
                            let operation_id = runtime.operation();
                            let mut next = (*snapshot.document).clone();
                            let script = Script {
                                id: format!("geometry-script-{}", operation_id.0),
                                name: format!("Script {}", next.scripts.len() + 1),
                                source: String::new(),
                                enabled: false,
                            };
                            next.scripts.push(script.clone());
                            active.set(script.id.clone());
                            name.set(script.name.clone());
                            source.set(String::new());
                            enabled.set(false);
                            submit_script_document(
                                &runtime,
                                &snapshot,
                                next,
                                operation_id,
                                script.id,
                            );
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
                    disabled: !can_edit || source().trim().is_empty(),
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let script_id = script.id.clone();
                        move |_| {
                            if !current_snapshot_is_ready(&runtime, &snapshot) {
                                return;
                            }
                            let mut next = (*snapshot.document).clone();
                            let Some(script) = next.scripts.iter_mut().find(|item| item.id == script_id) else {
                                return;
                            };
                            script.name = name();
                            script.source = source();
                            script.enabled = enabled();
                            let operation_id = runtime.operation();
                            submit_script_document(
                                &runtime,
                                &snapshot,
                                next,
                                operation_id,
                                script_id.clone(),
                            );
                        }
                    },
                    "Apply script"
                }
                if let Some(error) = current_error.as_ref() {
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
                        li { class: severity_class(finding.severity),
                            strong { "{severity_label(finding.severity)}" }
                            p { "{finding.message}" }
                        }
                    }
                }
            }
        }
    }
}

fn current_snapshot_is_ready(runtime: &Runtime, snapshot: &AcceptedSnapshot) -> bool {
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && model.accepted.as_ref().is_some_and(|accepted| {
            accepted.session_epoch == snapshot.session_epoch
                && accepted.token == snapshot.token
                && accepted.document.id == snapshot.document.id
                && accepted.document.revision == snapshot.document.revision
        })
}

fn submit_script_document(
    runtime: &Rc<Runtime>,
    snapshot: &AcceptedSnapshot,
    document: ProjectDoc,
    operation_id: boardstudio_application::OperationId,
    script_id: String,
) {
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: format!("geometry-script-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![script_id],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    });
}

fn severity_class(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "is-error",
        Severity::Warning => "is-warning",
        Severity::Info => "is-info",
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Information",
    }
}
