//! Layout presentation for the existing Core-owned Rhai script workflow.
use crate::pending_edit_helpers::PendingEditSignals;
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Lifecycle, Resolution, SessionEpoch,
};
use boardstudio_core::model::{EditOperation, Finding, Script, Severity};
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

#[derive(Clone, Debug)]
enum ScriptActionKey {
    New,
    Apply(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScriptOwner {
    document_id: String,
    session_epoch: SessionEpoch,
    script_id: String,
}

impl PartialEq for ScriptActionKey {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::New, Self::New) | (Self::Apply(_), Self::Apply(_))
        )
    }
}

impl Eq for ScriptActionKey {}

fn encode_script_draft(name: &str, source: &str, enabled: bool) -> String {
    serde_json::to_string(&(name, source, enabled)).expect("script draft tuple serializes")
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
    let mut draft = use_signal(|| encode_script_draft("", "", true));
    let failure = use_signal(|| None::<String>);
    let pending = use_hook(|| PendingEditSignals::<ScriptActionKey>::new());
    let pending_owner = use_signal(|| None::<ScriptOwner>);
    let new_disabled = use_signal(|| false);
    let apply_disabled = use_signal(|| false);
    pending.bind_one_shot(ScriptActionKey::New, new_disabled);
    pending.bind_one_shot(ScriptActionKey::Apply(String::new()), apply_disabled);
    pending.bind_field(ScriptActionKey::Apply(String::new()), draft, failure);
    let pending_for_drop = pending.clone();
    let new_pending = new_disabled();
    let active_script = scripts.iter().find(|script| script.id == active());
    let current_owner = active_script.map(|script| ScriptOwner {
        document_id: document.id.clone(),
        session_epoch: snapshot.session_epoch,
        script_id: script.id.clone(),
    });
    let new_active = new_pending && !active().is_empty();
    let owner_is_live = match (pending_owner(), current_owner.as_ref()) {
        (Some(owner), Some(active_owner)) => &owner == active_owner,
        (Some(owner), None) => {
            new_active
                && owner.document_id == document.id
                && owner.session_epoch == snapshot.session_epoch
                && owner.script_id == active()
        }
        _ => false,
    };
    let accepted_scripts = scripts.clone();
    pending.settle(owner_is_live, move |key| match key {
        ScriptActionKey::Apply(script_id) => accepted_scripts
            .iter()
            .find(|script| script.id == *script_id)
            .map(|script| encode_script_draft(&script.name, &script.source, script.enabled))
            .unwrap_or_default(),
        ScriptActionKey::New => String::new(),
    });
    use_drop(move || {
        pending_for_drop.settle(false, |_| String::new());
    });
    let active_identity = if active().is_empty() {
        None
    } else {
        Some((active(), active_script.is_some()))
    };
    let draft_owner = (snapshot.document.id.clone(), snapshot.session_epoch);
    let accepted_identity = (
        draft_owner.0.clone(),
        draft_owner.1,
        active_identity.clone(),
    );
    let observed_identity = use_hook(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            None::<(String, SessionEpoch, Option<(String, bool)>)>,
        ))
    });
    let accepted_active = active_script.cloned();
    use_effect(use_reactive((&accepted_identity,), {
        let observed_identity = observed_identity.clone();
        let mut name = name;
        let mut source = source;
        let mut enabled = enabled;
        let mut draft = draft;
        move |(identity,)| {
            let owner_changed = observed_identity.borrow().as_ref() != Some(&identity);
            *observed_identity.borrow_mut() = Some(identity.clone());
            if owner_changed {
                let (accepted_name, accepted_source, accepted_enabled) = accepted_active
                    .as_ref()
                    .map(|script| (script.name.clone(), script.source.clone(), script.enabled))
                    .unwrap_or_else(|| (String::new(), String::new(), true));
                name.set(accepted_name.clone());
                source.set(accepted_source.clone());
                enabled.set(accepted_enabled);
                draft.set(encode_script_draft(
                    &accepted_name,
                    &accepted_source,
                    accepted_enabled,
                ));
            }
        }
    }));
    let current_draft = draft();
    use_effect(use_reactive((&current_draft,), {
        let mut name = name;
        let mut source = source;
        let mut enabled = enabled;
        move |(serialized,)| {
            if let Ok((next_name, next_source, next_enabled)) =
                serde_json::from_str::<(String, String, bool)>(&serialized)
            {
                if name.peek().as_str() != next_name {
                    name.set(next_name);
                }
                if source.peek().as_str() != next_source {
                    source.set(next_source);
                }
                if enabled() != next_enabled {
                    enabled.set(next_enabled);
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
                    disabled: !can_edit || new_pending,
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let pending = pending.clone();
                        let mut failure = failure;
                        let mut active = active;
                        let mut name = name;
                        let mut source = source;
                        let mut enabled = enabled;
                        let mut draft = draft;
                        let mut pending_owner = pending_owner;
                        move |_| {
                            if !current_session_can_accept_edits(&runtime, &snapshot) {
                                return;
                            }
                            let Ok(script_identity) = crate::runtime::new_project_id() else {
                                return;
                            };
                            let script_id = format!("geometry-script-{script_identity}");
                            pending.settle(false, |_| String::new());
                            pending_owner.set(Some(ScriptOwner {
                                document_id: snapshot.document.id.clone(),
                                session_epoch: snapshot.session_epoch,
                                script_id: script_id.clone(),
                            }));
                            active.set(script_id.clone());
                            name.set(format!("Script {}", snapshot.document.scripts.len() + 1));
                            source.set(String::new());
                            enabled.set(false);
                            draft.set(encode_script_draft(&name(), &source(), enabled()));
                            failure.set(None);
                            pending.begin_one_shot(
                                &runtime,
                                ScriptActionKey::New,
                                "geometry-script-new",
                                Some("script".into()),
                                new_script_resolver(script_id),
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
                            let pending = pending.clone();
                            let mut pending_owner = pending_owner;
                            let document_id = document.id.clone();
                            let session_epoch = snapshot.session_epoch;
                            move |event| {
                                let id = event.value();
                                pending.settle(false, |_| String::new());
                                pending_owner.set((!id.is_empty()).then(|| ScriptOwner {
                                    document_id: document_id.clone(),
                                    session_epoch,
                                    script_id: id.clone(),
                                }));
                                active.set(id.clone());
                                if let Some(script) = scripts.iter().find(|script| script.id == id) {
                                    name.set(script.name.clone());
                                    source.set(script.source.clone());
                                    enabled.set(script.enabled);
                                    draft.set(encode_script_draft(&script.name, &script.source, script.enabled));
                                } else {
                                    name.set(String::new());
                                    source.set(String::new());
                                    enabled.set(true);
                                    draft.set(encode_script_draft("", "", true));
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
                        oninput: move |event| { let value = event.value(); name.set(value.clone()); draft.set(encode_script_draft(&value, &source(), enabled())); },
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
                        oninput: move |event| { let value = event.value(); source.set(value.clone()); draft.set(encode_script_draft(&name(), &value, enabled())); },
                    }
                }
                label { class: "m1-geometry-script-enabled",
                    input {
                        r#type: "checkbox",
                        checked: enabled(),
                        disabled: !can_edit,
                        onchange: move |event| { let value = event.checked(); enabled.set(value); draft.set(encode_script_draft(&name(), &source(), value)); },
                    }
                    "Enable on Apply"
                }
                button {
                    r#type: "button",
                    class: "m1-geometry-script-apply",
                    disabled: !can_edit || apply_disabled() || source().trim().is_empty(),
                    onclick: {
                        let runtime = runtime.clone();
                        let snapshot = snapshot.clone();
                        let script_id = script.id.clone();
                        let pending = pending.clone();
                        let mut failure = failure;
                        let draft = draft;
                        let mut pending_owner = pending_owner;
                        move |_| {
                            if !current_session_can_accept_edits(&runtime, &snapshot) {
                                return;
                            }
                            failure.set(None);
                            pending_owner.set(Some(ScriptOwner {
                                document_id: snapshot.document.id.clone(),
                                session_epoch: snapshot.session_epoch,
                                script_id: script_id.clone(),
                            }));
                            pending.begin_field(
                                &runtime,
                                ScriptActionKey::Apply(script_id.clone()),
                                "geometry-script-apply",
                                Some("script".into()),
                                apply_script_resolver(
                                    script_id.clone(),
                                    name(),
                                    source(),
                                    enabled(),
                                ),
                                &draft(),
                            );
                        }
                    },
                    "Apply script"
                }
                if let Some(error) = failure.read().as_ref() {
                    p { role: "alert", class: "m1-geometry-script-error", "data-error-source": "script", "{error}" }
                } else if let Some(error) = current_error.as_ref() {
                    p { role: "alert", class: "m1-geometry-script-error", "data-error-source": "runtime", "{error}" }
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
    use boardstudio_core::model::{Board, EditCommand, EditPhase, ProjectDoc};
    use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
    use std::cell::{Cell, RefCell};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::{Event, EventInit, HtmlSelectElement, HtmlTextAreaElement};

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct ScriptProbe {
        runtime: Rc<Runtime>,
        version: Rc<Cell<u64>>,
        version_signal: Rc<RefCell<Option<Signal<u64>>>>,
    }

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

    fn editor_host() -> Element {
        let probe = use_context::<ScriptProbe>();
        let mut version = use_signal(|| 0u64);
        use_context_provider(|| version);
        if version() != probe.version.get() {
            version.set(probe.version.get());
        }
        *probe.version_signal.borrow_mut() = Some(version);
        use_context_provider(|| probe.runtime.clone());
        rsx! { GeometryScriptsEditor { on_back: EventHandler::default() } }
    }

    async fn render_wait() {
        gloo_timers::future::TimeoutFuture::new(10).await;
    }

    async fn mount_editor(runtime: Rc<Runtime>) -> ScriptProbe {
        let probe = ScriptProbe {
            runtime,
            version: Rc::new(Cell::new(0)),
            version_signal: Rc::default(),
        };
        let document = web_sys::window().unwrap().document().unwrap();
        if let Some(previous) = document.get_element_by_id("geometry-script-owner-test") {
            previous.remove();
        }
        let root = document.create_element("div").unwrap();
        root.set_id("geometry-script-owner-test");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(editor_host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.into()),
        );
        render_wait().await;
        render_wait().await;
        probe
    }

    async fn flush(probe: &ScriptProbe) {
        let next = probe.version.get() + 1;
        probe.version.set(next);
        if let Some(mut version) = *probe.version_signal.borrow() {
            version.set(next);
        }
        render_wait().await;
        render_wait().await;
    }

    fn select_script(id: &str) {
        let select = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#geometry-script-owner-test select[aria-label='Active script']")
            .unwrap()
            .expect("active script select")
            .dyn_into::<HtmlSelectElement>()
            .unwrap();
        select.set_value(id);
        select.dispatch_event(&bubbling_event("change")).unwrap();
    }

    fn source_value() -> String {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#geometry-script-owner-test textarea[aria-label='Rhai source']")
            .unwrap()
            .expect("source field")
            .dyn_into::<HtmlTextAreaElement>()
            .unwrap()
            .value()
    }

    fn type_source(value: &str) {
        let source = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#geometry-script-owner-test textarea[aria-label='Rhai source']")
            .unwrap()
            .expect("source field")
            .dyn_into::<HtmlTextAreaElement>()
            .unwrap();
        source.set_value(value);
        source.dispatch_event(&bubbling_event("input")).unwrap();
    }

    fn bubbling_event(name: &str) -> Event {
        let init = EventInit::new();
        init.set_bubbles(true);
        Event::new_with_event_init_dict(name, &init).unwrap()
    }

    fn click_apply() {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#geometry-script-owner-test button.m1-geometry-script-apply")
            .unwrap()
            .expect("apply button")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
    }

    fn script_field_failure() -> Option<String> {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#geometry-script-owner-test [data-error-source='script']")
            .unwrap()
            .map(|element| element.text_content().unwrap_or_default())
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
    async fn held_script_failure_is_retired_when_active_script_changes() {
        let runtime = support::new_runtime();
        let mut document = document();
        document.scripts.push(Script {
            id: "script-2".into(),
            name: "Script 2".into(),
            source: "// script 2".into(),
            enabled: false,
        });
        support::open_document(&runtime, document).await;
        let probe = mount_editor(runtime.clone()).await;
        select_script("script-1");
        flush(&probe).await;
        type_source("// script 1 edited");
        flush(&probe).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        click_apply();

        let runner = runtime.clone();
        let done = Rc::new(Cell::new(false));
        let done_task = done.clone();
        wasm_bindgen_futures::spawn_local(async move {
            support::run_pending(&runner).await;
            done_task.set(true);
        });
        let entered_core = Rc::new(Cell::new(false));
        let entered_core_task = entered_core.clone();
        wasm_bindgen_futures::spawn_local(async move {
            entered.await.expect("script A reached Core");
            entered_core_task.set(true);
        });
        for _ in 0..100 {
            if entered_core.get() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(entered_core.get(), "script A reached Core");
        select_script("script-2");
        flush(&probe).await;
        assert_eq!(source_value(), "// script 2");
        support::fail_next_persist(&runtime, "disk full");
        release.send(()).expect("release held script A reply");
        for _ in 0..100 {
            if done.get() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(done.get(), "Session work completed");
        flush(&probe).await;

        assert_eq!(runtime.model().lifecycle, Lifecycle::RecoveryRequired);
        assert_eq!(source_value(), "// script 2");
        assert!(
            script_field_failure().is_none(),
            "script A failure must not be attached to active script B"
        );
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
