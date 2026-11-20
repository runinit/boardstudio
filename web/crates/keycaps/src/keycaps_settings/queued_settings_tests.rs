use super::*;
use boardstudio_application::{Event, TerminalOutcome};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_future, wait_until_step};
use wasm_bindgen_test::*;

#[derive(Clone)]
struct RetirementExpectation {
    request: KeycapsEditRequest,
    original_workspace: &'static str,
    outcome: Rc<RefCell<Option<TerminalOutcome>>>,
}

#[derive(Clone, PartialEq)]
struct RetirementCheckpoint {
    original_workspace: &'static str,
    shared_owner: Option<(Option<Scope>, u64, &'static str)>,
    key_owner: u64,
    request: Option<KeycapsEditRequest>,
    observed_requests: Vec<(Option<KeycapsEditRequest>, bool)>,
    request_pending: bool,
    terminal_outcome: Option<TerminalOutcome>,
    render_generation: u64,
    effect_generation: u64,
}

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    actions: Rc<RefCell<Option<KeycapsSettingsActions>>>,
    workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
    retirement: Rc<RefCell<Option<RetirementExpectation>>>,
    retirement_checkpoint: Rc<RefCell<Option<RetirementCheckpoint>>>,
    edits: Rc<RefCell<Option<KeycapsEditsContext>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    let runtime = probe.runtime.clone();
    let version = use_signal(|| 0u64);
    let retirement_render_generation = use_hook(|| Rc::new(RefCell::new(0u64)));
    let retirement_effect_generation = use_hook(|| Rc::new(RefCell::new(0u64)));
    use_context_provider(|| version);
    let observed = runtime.clone();
    use_hook(move || {
        observed.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
    let _ = version();
    let workspace = use_signal(|| "Keycaps");
    *probe.workspace.borrow_mut() = Some(workspace);
    let generation = use_signal(|| 0u64);
    let accepted = runtime.model().accepted.unwrap();
    let source = runtime.scope().map(|scope| KeycapsEditSource {
        scope,
        token: accepted.token,
        revision: accepted.document.revision,
    });
    let scope = runtime.scope().unwrap();
    let view = keycaps_scene::project(&accepted, &scope, &scope.board_id).unwrap();
    let selected = project_selected_key(&accepted.document, &view, Some("key")).unwrap();
    // Outside the Keycaps workspace the hook offers no actions and nothing renders.
    let actions = use_keycaps_settings_actions(runtime, source, workspace, generation);
    *probe.actions.borrow_mut() = actions.clone();
    let edit_context = use_context::<KeycapsEditsContext>();
    *probe.edits.borrow_mut() = Some(edit_context);
    let edits = edit_context.edits.read();
    let render_generation = if probe.retirement.borrow().is_some() {
        let mut generation = retirement_render_generation.borrow_mut();
        *generation += 1;
        *generation
    } else {
        0
    };
    let retirement_checkpoint = probe.retirement.borrow().as_ref().map(|expected| {
        let entry = edits.requests.iter().find(|entry| {
            entry
                .request
                .as_deref()
                .is_some_and(|request| request == &expected.request)
        });
        RetirementCheckpoint {
            original_workspace: expected.original_workspace,
            shared_owner: edits.shared_owner.clone(),
            key_owner: edits.key_owner,
            request: entry
                .and_then(|entry| entry.request.as_ref())
                .map(|request| (**request).clone()),
            observed_requests: edits
                .requests
                .iter()
                .map(|entry| (entry.request.as_deref().cloned(), edits.is_pending(entry)))
                .collect(),
            request_pending: entry.is_some_and(|entry| edits.is_pending(entry)),
            terminal_outcome: expected.outcome.borrow().clone(),
            render_generation,
            effect_generation: 0,
        }
    });
    drop(edits);
    let checkpoint = probe.retirement_checkpoint.clone();
    let effect_generation = retirement_effect_generation.clone();
    use_effect(use_reactive(
        (&retirement_checkpoint,),
        move |(retirement_checkpoint,)| {
            if let Some(mut retirement_checkpoint) = retirement_checkpoint.clone() {
                let mut generation = effect_generation.borrow_mut();
                *generation += 1;
                retirement_checkpoint.effect_generation = *generation;
                let request_ids = retirement_checkpoint
                    .observed_requests
                    .iter()
                    .map(|(request, pending)| {
                        request.as_ref().map_or_else(
                            || format!("<none>:pending={pending}"),
                            |request| format!("{}:pending={pending}", request.request_id),
                        )
                    })
                    .collect::<Vec<_>>();
                let shared_owner = retirement_checkpoint
                    .shared_owner
                    .as_ref()
                    .map(|(_, generation, workspace)| (*workspace, *generation));
                web_sys::console::debug_1(
                &format!(
                    "ticket 128 retirement checkpoint: expected_request={}, observed_requests={request_ids:?}, owner={shared_owner:?}, key_owner={}, outcome={:?}, render_generation={}, effect_generation={}",
                    retirement_checkpoint
                        .request
                        .as_ref()
                        .map_or(0, |request| request.request_id),
                    retirement_checkpoint.key_owner,
                    retirement_checkpoint.terminal_outcome,
                    retirement_checkpoint.render_generation,
                    retirement_checkpoint.effect_generation,
                )
                .into(),
            );
                *checkpoint.borrow_mut() = Some(retirement_checkpoint);
            }
        },
    ));
    let Some(actions) = actions else {
        return rsx! {};
    };
    rsx! { KeycapsSettingsEditor { selected, actions } }
}

#[wasm_bindgen_test]
async fn keycap_dimensions_queue_and_preserve_the_accepted_sibling() {
    let runtime = support::new_runtime();
    let mut doc = ProjectDoc::empty("keycaps-queue", "Keycaps");
    doc.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":["key"],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
    doc.definitions.push(serde_json::from_value(serde_json::json!({"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"})).unwrap());
    support::open_document(&runtime, doc).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    let probe = Probe {
        runtime: runtime.clone(),
        actions: Rc::new(RefCell::new(None)),
        workspace: Rc::new(RefCell::new(None)),
        retirement: Rc::new(RefCell::new(None)),
        retirement_checkpoint: Rc::new(RefCell::new(None)),
        edits: Rc::new(RefCell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until(
        "the keycap editor actions and width input are mounted",
        || {
            probe.actions.borrow().is_some()
                && by_role_name(&root, "spinbutton", "Keycap width for SW1").is_some()
        },
    )
    .await;
    let submit = |change| {
        let borrowed = probe.actions.borrow();
        let actions = borrowed.as_ref().unwrap();
        let mut sequence = actions.request_sequence;
        let request_id = sequence() + 1;
        sequence.set(request_id);
        actions.on_change.call(KeycapsEditRequest {
            scope: actions.scope.clone(),
            scope_generation: actions.scope_generation,
            selection_generation: actions.selection_generation,
            editor_instance_id: actions.editor_instance_id,
            request_id,
            admission_token: actions.token,
            admission_revision: actions.revision,
            target: KeycapsEditTarget::Key("key".into()),
            baseline: KeycapsEditBaseline::Key(actions.accepted_settings.clone()),
            change,
        });
    };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    use wasm_bindgen::JsCast;
    let width = by_role_name(&root, "spinbutton", "Keycap width for SW1")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    width.set_value("2");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    width
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    support::drive_pending(&runtime);
    wait_until_future("the width request reaches the held Core executor", entered)
        .await
        .unwrap();
    wait_until("the width draft is visible before the sibling edit", || {
        width.value() == "2"
            && probe
                .actions
                .borrow()
                .as_ref()
                .is_some_and(|actions| (actions.request_sequence)() >= 1)
    })
    .await;
    assert_eq!(width.value(), "2");
    submit(KeycapEditChange::UnitsDepth(1.5));
    release.send(()).unwrap();
    wait_until_step(
        "the accepted dimensions include the queued sibling edit",
        async || {
            support::run_pending(&runtime).await;
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units
                == Some(Vec2 { x: 2.0, y: 1.5 })
        },
    )
    .await;
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
        Some(Vec2 { x: 2.0, y: 1.5 })
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step(
        "Undo restores the preceding accepted dimensions",
        async || {
            support::run_pending(&runtime).await;
            settings_for_key(&runtime.model().accepted.unwrap().document, "key").units
                == Some(Vec2 { x: 2.0, y: 1.0 })
        },
    )
    .await;
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
        Some(Vec2 { x: 2.0, y: 1.0 })
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step(
        "Undo restores the original accepted dimensions",
        async || {
            support::run_pending(&runtime).await;
            settings_for_key(&runtime.model().accepted.unwrap().document, "key")
                .units
                .is_none()
        },
    )
    .await;
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
        None
    );
    // A return to the old accepted legend is still a newer committed intent.
    wait_until("the legend input is mounted after dimension undos", || {
        by_role_name(&root, "textbox", "Legend for SW1").is_some()
    })
    .await;
    let input = by_role_name(&root, "textbox", "Legend for SW1")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    wait_until(
        "the legend input and edit source match the accepted snapshot after dimension undos",
        || {
            let accepted = runtime.model().accepted.unwrap();
            let Some(scope) = runtime.scope() else {
                return false;
            };
            let expected_settings = settings_for_key(&accepted.document, "key");
            let actions = probe.actions.borrow();
            let Some(actions) = actions.as_ref() else {
                return false;
            };
            input.value() == accepted_legend_text(&accepted.document, "key")
                && probe
                    .workspace
                    .borrow()
                    .as_ref()
                    .is_some_and(|workspace| workspace() == "Keycaps")
                && actions.scope == scope
                && actions.token == accepted.token
                && actions.revision == accepted.document.revision
                && actions.selected_key_id == "key"
                && actions.accepted_settings.units == expected_settings.units
                && actions.accepted_settings.legend == expected_settings.legend
        },
    )
    .await;
    let commit = |value: &str| {
        input.focus().unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        input.blur().unwrap();
    };
    let b_request_id = {
        let actions = probe.actions.borrow();
        (actions.as_ref().unwrap().request_sequence)() + 1
    };
    let b_outcome = support::observe_next(&runtime);
    let (entered, release) = support::gate_next_core_reply(&runtime);
    commit("B");
    support::drive_pending(&runtime);
    wait_until_future(
        "the first legend edit reaches the held Core executor",
        entered,
    )
    .await
    .unwrap();
    wait_until(
        "the first legend draft is visible before it is replaced",
        || {
            input.value() == "B"
                && probe
                    .actions
                    .borrow()
                    .as_ref()
                    .is_some_and(|actions| (actions.request_sequence)() == b_request_id)
        },
    )
    .await;
    let empty_request_id = {
        let actions = probe.actions.borrow();
        (actions.as_ref().unwrap().request_sequence)() + 1
    };
    let empty_outcome = support::observe_next(&runtime);
    assert!(
        !Rc::ptr_eq(&b_outcome, &empty_outcome),
        "the B and empty requests must observe distinct Runtime operations"
    );
    commit("");
    wait_until("the newer empty legend draft is registered", || {
        input.value().is_empty()
            && probe
                .actions
                .borrow()
                .as_ref()
                .is_some_and(|actions| (actions.request_sequence)() == empty_request_id)
    })
    .await;
    release.send(()).unwrap();
    wait_until_step("both legend operations settle before Undo", async || {
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.unwrap();
        let settings = settings_for_key(&accepted.document, "key");
        let actions_ready = probe.actions.borrow().as_ref().is_some_and(|actions| {
            actions.revision == accepted.document.revision
                && actions.token == accepted.token
                && actions.selected_key_id == "key"
                && actions.accepted_settings.legend == settings.legend
        });
        b_outcome.borrow().is_some()
            && empty_outcome.borrow().is_some()
            && (probe.actions.borrow().as_ref().unwrap().request_sequence)() == empty_request_id
            && input.value().is_empty()
            && settings.legend.is_none()
            && actions_ready
    })
    .await;
    assert!(
        matches!(
            b_outcome.borrow().as_ref(),
            Some(TerminalOutcome::Completed)
        ),
        "B request {b_request_id} terminal outcome: {:?}",
        b_outcome.borrow().as_ref()
    );
    assert!(
        matches!(
            empty_outcome.borrow().as_ref(),
            Some(TerminalOutcome::Completed)
        ),
        "empty request {empty_request_id} terminal outcome: {:?}",
        empty_outcome.borrow().as_ref()
    );
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").legend,
        None
    );
    assert_eq!(input.value(), "");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key")
            .legend
            .as_deref(),
        Some("B")
    );
    wait_until(
        "the width input remains mounted after the legend undo",
        || {
            let accepted = runtime.model().accepted.unwrap();
            accepted_legend_text(&accepted.document, "key") == "B"
                && by_role_name(&root, "spinbutton", "Keycap width for SW1").is_some()
                && probe.actions.borrow().as_ref().is_some_and(|actions| {
                    actions.revision == accepted.document.revision
                        && actions.token == accepted.token
                })
        },
    )
    .await;
    let width = by_role_name(&root, "spinbutton", "Keycap width for SW1")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    support::fail_next_core_reply(&runtime, "keycap executor failed");
    width.set_value("3");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    width
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    wait_until_step(
        "the failed width draft restores accepted settings and reports inline",
        async || {
            support::run_pending(&runtime).await;
            settings_for_key(&runtime.model().accepted.unwrap().document, "key")
                .units
                .is_none()
                && width.value().is_empty()
                && root
                    .text_content()
                    .unwrap()
                    .contains("keycap executor failed")
        },
    )
    .await;
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").units,
        None
    );
    assert_eq!(width.value(), "");
    assert!(
        root.text_content()
            .unwrap()
            .contains("keycap executor failed")
    );
    root.remove();
}

async fn mounted_legend(
    id: &str,
) -> (
    Rc<Runtime>,
    Probe,
    web_sys::Element,
    web_sys::HtmlInputElement,
    MountedFixture,
) {
    use wasm_bindgen::JsCast;
    let runtime = support::new_runtime();
    let mut doc = ProjectDoc::empty(id, "Keycaps");
    doc.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":["key"],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
    doc.definitions.push(serde_json::from_value(serde_json::json!({"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
    doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"})).unwrap());
    support::open_document(&runtime, doc).await;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec!["key".into()],
        range_part_ids: vec![],
        mode: boardstudio_application::SelectionMode::Replace,
    });
    let probe = Probe {
        runtime: runtime.clone(),
        actions: Rc::new(RefCell::new(None)),
        workspace: Rc::new(RefCell::new(None)),
        retirement: Rc::new(RefCell::new(None)),
        retirement_checkpoint: Rc::new(RefCell::new(None)),
        edits: Rc::new(RefCell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    let mounted = MountedFixture::new(root.clone(), dom);
    wait_until("the legend editor actions and input are mounted", || {
        probe.actions.borrow().is_some()
            && by_role_name(&root, "textbox", "Legend for SW1").is_some()
    })
    .await;
    let input = by_role_name(&root, "textbox", "Legend for SW1")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    (runtime, probe, root, input, mounted)
}

#[wasm_bindgen_test]
async fn keycap_overrides_are_named_and_profile_picker_selects_by_keyboard() {
    let (_runtime, _probe, root, _, _mounted) = mounted_legend("keycaps-choice-floor").await;
    let disclosure =
        by_role_name(&root, "button", "Keycap overrides").expect("override disclosure");
    assert_eq!(
        disclosure.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    disclosure.click();
    wait_until("override controls appear", || {
        by_role_name(&root, "button", "Profile override for SW1").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Profile override for SW1").unwrap();
    trigger.focus().unwrap();
    let key = |target: &web_sys::HtmlElement, value: &str| {
        let event = web_sys::KeyboardEventInit::new();
        event.set_key(value);
        event.set_bubbles(true);
        event.set_cancelable(true);
        target
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
    };
    key(&trigger, "Enter");
    wait_until("profile list opens", || {
        by_role_name(&root, "listbox", "Profile override for SW1").is_some()
    })
    .await;
    let list = by_role_name(&root, "listbox", "Profile override for SW1").unwrap();
    key(&list, "o");
    wait_until("OEM is focused", || {
        by_role_name(&root, "option", "OEM").is_some_and(|option| option.matches(":focus").unwrap())
    })
    .await;
    key(&by_role_name(&root, "option", "OEM").unwrap(), "Enter");
    wait_until("profile select returns focus", || {
        trigger.matches(":focus").unwrap()
            && trigger.get_attribute("aria-expanded").as_deref() == Some("false")
    })
    .await;
    root.remove();
}

fn type_legend(input: &web_sys::HtmlInputElement, value: &str) {
    input.focus().unwrap();
    input.set_value(value);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
}

#[wasm_bindgen_test]
async fn a_newer_legend_draft_survives_an_older_failure_that_reports_inline() {
    let (runtime, _probe, root, input, _mounted) = mounted_legend("keycaps-newer-draft").await;
    support::fail_next_core_reply(&runtime, "legend executor failed");
    type_legend(&input, "A");
    input.blur().unwrap();
    // The older edit has not settled when the user types the next draft.
    type_legend(&input, "AB");
    wait_until_step(
        "the older legend failure is inline while the newer draft survives",
        async || {
            support::run_pending(&runtime).await;
            input.value() == "AB"
                && root
                    .text_content()
                    .unwrap()
                    .contains("legend executor failed")
        },
    )
    .await;
    assert_eq!(input.value(), "AB", "a newer draft is never overwritten");
    assert!(
        root.text_content()
            .unwrap()
            .contains("legend executor failed"),
        "the older failure still reports inline"
    );
    assert_eq!(
        settings_for_key(&runtime.model().accepted.unwrap().document, "key").legend,
        None
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_legend_restores_the_accepted_text_with_an_inline_failure() {
    let (runtime, _probe, root, input, _mounted) =
        mounted_legend("keycaps-unchanged-failure").await;
    support::fail_next_core_reply(&runtime, "legend executor failed");
    type_legend(&input, "A");
    input.blur().unwrap();
    wait_until_step(
        "the unchanged failed legend restores accepted text and reports inline",
        async || {
            support::run_pending(&runtime).await;
            input.value().is_empty()
                && root
                    .text_content()
                    .unwrap()
                    .contains("legend executor failed")
        },
    )
    .await;
    assert_eq!(
        input.value(),
        "",
        "the unchanged draft restores the accepted text"
    );
    assert!(
        root.text_content()
            .unwrap()
            .contains("legend executor failed"),
        "the failure reports inline"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_second_legend_edit_replaces_the_observation_and_both_edits_undo() {
    let (runtime, _probe, root, input, _mounted) = mounted_legend("keycaps-latest-per-key").await;
    let legend = |runtime: &Rc<Runtime>| {
        accepted_legend_text(&runtime.model().accepted.unwrap().document, "key")
    };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    type_legend(&input, "A");
    input.blur().unwrap();
    support::drive_pending(&runtime);
    wait_until_future(
        "the first legend edit reaches the held Core executor",
        entered,
    )
    .await
    .unwrap();
    type_legend(&input, "B");
    input.blur().unwrap();
    support::drive_pending(&runtime);
    wait_until("the second legend edit replaces the request", || {
        input.value() == "B"
            && _probe
                .actions
                .borrow()
                .as_ref()
                .is_some_and(|actions| (actions.request_sequence)() >= 2)
    })
    .await;
    release.send(()).unwrap();
    wait_until_step("the second legend edit is accepted", async || {
        support::run_pending(&runtime).await;
        legend(&runtime) == "B" && input.value() == "B"
    })
    .await;
    assert_eq!(legend(&runtime), "B");
    assert_eq!(input.value(), "B");
    for expected in ["A", ""] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(legend(&runtime), expected, "each edit has an Undo step");
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn leaving_the_keycaps_workspace_retires_a_pending_edit_silently() {
    let (runtime, probe, root, input, _mounted) = mounted_legend("keycaps-owner-departs").await;
    let actions = probe.actions.borrow().as_ref().unwrap().clone();
    let request_id = (actions.request_sequence)()
        .checked_add(1)
        .expect("the test request sequence has room for one edit");
    let request = KeycapsEditRequest {
        scope: actions.scope.clone(),
        scope_generation: actions.scope_generation,
        selection_generation: actions.selection_generation,
        editor_instance_id: actions.editor_instance_id,
        request_id,
        admission_token: actions.token,
        admission_revision: actions.revision,
        target: KeycapsEditTarget::Key(actions.selected_key_id.clone()),
        baseline: KeycapsEditBaseline::Key(actions.accepted_settings.clone()),
        change: KeycapEditChange::Legend(Some("Gone".into())),
    };
    let outcome = support::observe_next(&runtime);
    *probe.retirement.borrow_mut() = Some(RetirementExpectation {
        request: request.clone(),
        original_workspace: "Keycaps",
        outcome: outcome.clone(),
    });
    let reason = "late keycap executor failure";
    let (entered, release) = support::gate_next_core_reply_then_fail(&runtime, reason);
    type_legend(&input, "Gone");
    input.blur().unwrap();
    support::drive_pending(&runtime);
    wait_until_future(
        "the exact legend edit reaches the held Core executor",
        entered,
    )
    .await
    .unwrap();
    let last_live_diagnostics = Rc::new(RefCell::new(None::<String>));
    let reported_live_diagnostics = last_live_diagnostics.clone();
    wait_until(
        "the exact submitted edit is pending while its reply is held",
        || {
            let context = probe.edits.borrow();
            let Some(context) = context.as_ref() else {
                return false;
            };
            let edits = context.edits.peek();
            let entry = edits.requests.iter().find(|entry| {
                entry
                    .request
                    .as_deref()
                    .is_some_and(|candidate| candidate == &request)
            });
            let request_ids = edits
                .requests
                .iter()
                .map(|entry| {
                    entry.request.as_deref().map_or_else(
                        || format!("<none>:pending={}", edits.is_pending(entry)),
                        |request| {
                            format!("{}:pending={}", request.request_id, edits.is_pending(entry))
                        },
                    )
                })
                .collect::<Vec<_>>();
            let shared_owner = edits
                .shared_owner
                .as_ref()
                .map(|(_, generation, workspace)| (*workspace, *generation));
            let diagnostic = format!(
                "request_ids={request_ids:?}, owner={shared_owner:?}, key_owner={}, outcome={:?}",
                edits.key_owner,
                outcome.borrow(),
            );
            let mut last = reported_live_diagnostics.borrow_mut();
            if last.as_ref() != Some(&diagnostic) {
                web_sys::console::debug_1(
                    &format!("ticket 128 live held-reply state: {diagnostic}").into(),
                );
                *last = Some(diagnostic);
            }
            probe
                .retirement
                .borrow()
                .as_ref()
                .is_some_and(|expected| expected.original_workspace == "Keycaps")
                && entry.is_some_and(|entry| {
                    entry.request.as_deref() == Some(&request)
                        && edits.is_pending(entry)
                        && outcome.borrow().is_none()
                        && edits.shared_owner.as_ref()
                            == Some(&(
                                Some(request.scope.clone()),
                                request.scope_generation,
                                "Keycaps",
                            ))
                        && edits.key_owner == request.selection_generation
                })
        },
    )
    .await;
    assert!(outcome.borrow().is_none(), "the Core reply is still held");
    let mut workspace = probe
        .workspace
        .borrow()
        .expect("host exposes its workspace");
    workspace.set("Layout");
    wait_until(
        "owner effects replace and prune the submitted edit before its reply",
        || {
            let context = probe.edits.borrow();
            let Some(context) = context.as_ref() else {
                return false;
            };
            let edits = context.edits.peek();
            probe
                .retirement
                .borrow()
                .as_ref()
                .is_some_and(|expected| expected.original_workspace == "Keycaps")
                && edits.shared_owner.as_ref()
                    == Some(&(
                        Some(request.scope.clone()),
                        request.scope_generation,
                        "Layout",
                    ))
                && edits.key_owner != request.selection_generation
                && !edits.requests.iter().any(|entry| {
                    entry
                        .request
                        .as_deref()
                        .is_some_and(|candidate| candidate == &request)
                })
                && outcome.borrow().is_none()
        },
    )
    .await;
    assert!(
        outcome.borrow().is_none(),
        "the exact Core operation remains unsettled after owner retirement"
    );
    release.send(()).unwrap();
    wait_until_step(
        "the retired edit reaches its exact terminal outcome",
        async || {
            support::run_pending(&runtime).await;
            outcome.borrow().is_some()
        },
    )
    .await;
    let terminal_outcome = outcome.borrow().clone().unwrap();
    assert!(
        !matches!(&terminal_outcome, TerminalOutcome::Completed),
        "the injected late Core failure should remain a failure: {terminal_outcome:?}"
    );
    assert_eq!(
        terminal_outcome,
        TerminalOutcome::ExecutorFailed(reason.into()),
        "the retired edit reports the specific injected late executor failure"
    );
    wait_until(
        "the rendered retirement checkpoint captures the terminal failure",
        || {
            probe
                .retirement_checkpoint
                .borrow()
                .as_ref()
                .is_some_and(|checkpoint| {
                    checkpoint.shared_owner.as_ref()
                        == Some(&(
                            Some(request.scope.clone()),
                            request.scope_generation,
                            "Layout",
                        ))
                        && checkpoint.key_owner != request.selection_generation
                        && checkpoint.original_workspace == "Keycaps"
                        && checkpoint.request.is_none()
                        && !checkpoint.request_pending
                        && checkpoint.terminal_outcome.as_ref() == Some(&terminal_outcome)
                })
        },
    )
    .await;
    workspace.set("Keycaps");
    wait_until(
        "the remounted Keycaps owner and legend input are ready",
        || {
            let accepted = runtime.model().accepted.unwrap();
            by_role_name(&root, "textbox", "Legend for SW1").is_some()
                && probe.actions.borrow().as_ref().is_some_and(|actions| {
                    probe
                        .workspace
                        .borrow()
                        .as_ref()
                        .is_some_and(|workspace| workspace() == "Keycaps")
                        && actions.scope == request.scope
                        && actions.revision == accepted.document.revision
                        && actions.token == accepted.token
                        && actions.selected_key_id == "key"
                })
        },
    )
    .await;
    let text = root.text_content().unwrap();
    assert!(
        !text.contains("Unaccepted") && !text.contains("Saving keycap"),
        "a departed owner leaves no status or retry draft: {text}"
    );
    root.remove();
}
