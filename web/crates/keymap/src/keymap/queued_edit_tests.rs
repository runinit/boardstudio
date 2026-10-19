//! Mounted controller requests run through the real Session and Core, with gated replies.
use super::binding_controller::{BindingActions, BindingProjectionSources, use_binding_operations};
use super::binding_editor::{BindingEditRequest, BindingField, BindingTarget};
use super::layer_controller::{LayerActions, LayerSource, use_layer_operations};
use super::macro_controller::{MacroActions, use_macro_operations};
use super::macro_editor::{MacroEditChange, MacroEditRequest, MacroEditTarget};
use crate::layer_edit::KeymapLayerOperation;
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::{KeyBinding, MacroStep, ProjectDoc};
use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_future, wait_until_step};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen_test::*;

type LayerNameSignals = (Signal<String>, Signal<Option<String>>);

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    bindings: Rc<RefCell<Option<BindingActions>>>,
    layers: Rc<RefCell<Option<LayerActions>>>,
    macros: Rc<RefCell<Option<MacroActions>>>,
    active_layer: Rc<RefCell<Option<Signal<String>>>>,
    generation: Rc<RefCell<Option<Signal<u64>>>>,
    layer_name: Rc<RefCell<Option<LayerNameSignals>>>,
    hide_macros: Rc<RefCell<Option<Signal<bool>>>>,
    renders: Rc<Cell<u64>>,
    rendered_revision: Rc<Cell<Option<u64>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
    probe.renders.set(probe.renders.get() + 1);
    let rendered_accepted = runtime_revision(&probe.runtime);
    probe.rendered_revision.set(rendered_accepted);
    let runtime = probe.runtime.clone();
    let version = use_signal(|| 0u64);
    use_context_provider(|| version);
    let subscribe_runtime = runtime.clone();
    use_hook(move || {
        subscribe_runtime.subscribe(Rc::new(move || {
            let mut version = version;
            version += 1;
        }))
    });
    let _ = version();
    let active_layer = use_signal(|| "base".to_owned());
    *probe.active_layer.borrow_mut() = Some(active_layer);
    let workspace = use_signal(|| "Keymap");
    let generation = use_signal(|| 0u64);
    *probe.generation.borrow_mut() = Some(generation);
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let source = Some(LayerSource {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
    });
    let view = super::view::project(&accepted, Some(&scope), &scope.board_id, &active_layer());
    let encoder_projection = use_memo(|| None);
    *probe.layers.borrow_mut() = Some(use_layer_operations(
        runtime.clone(),
        source.clone(),
        active_layer,
        workspace,
        generation,
        Rc::new(|| true),
    ));
    let layer_edits = use_context::<super::layer_controller::LayerEditsContext>();
    *probe.layer_name.borrow_mut() = Some((layer_edits.name_draft, layer_edits.name_failure));
    let macro_actions = use_macro_operations(
        runtime.clone(),
        source.clone(),
        workspace,
        generation,
        Rc::new(|| true),
    );
    let hide_macros = use_signal(|| false);
    *probe.hide_macros.borrow_mut() = Some(hide_macros);
    let macro_ui = macro_actions.source.as_ref().filter(|_| !hide_macros()).map(|source| rsx! { super::macro_editor::MacroEditor {
        scope: scope.clone(), scope_generation: 0, editor_instance_id: macro_actions.editor_instance_id,
        request_sequence: macro_actions.request_sequence, source: source.clone(), sequences: macro_actions.sequences.clone(),
        enabled: macro_actions.enabled, feedback: macro_actions.feedback.clone(), on_change: macro_actions.on_change,
    } });
    *probe.macros.borrow_mut() = Some(macro_actions);
    let actions = use_binding_operations(
        runtime.clone(),
        BindingProjectionSources {
            source,
            view,
            encoder_projection,
            current_encoder_projection: Rc::new(|| None),
        },
        active_layer,
        workspace,
        generation,
        Rc::new(|| true),
    );
    let binding_ui = actions.projection.as_ref().map(|projection| rsx! {
        super::binding_editor::BindingEditor {
            scope: scope.clone(), admission_token: accepted.token, admission_revision: accepted.document.revision,
            active_layer_id: projection.effective_layer_id.clone(), target: BindingTarget::Key { key_id: runtime.model().selected_part_ids[0].clone() },
            input_identity: None, key_label: projection.key_label.clone(), editor_instance_id: actions.editor_instance_id,
            request_sequence: actions.request_sequence, value: projection.binding.clone(), layers: projection.layers.clone(),
            macros: projection.macros.clone(), enabled: actions.enabled, feedback: actions.feedback.clone(), on_change: actions.on_change,
        }
    });
    *probe.bindings.borrow_mut() = Some(actions);
    rsx! { div { "Keymap edit test" {macro_ui} {binding_ui} } }
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("queued-keymap", "Keymap");
    document.boards.push(
        serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": ["a", "b"],
            "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        }))
        .unwrap(),
    );
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch", "name": "Switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    for id in ["a", "b"] {
        document.parts.push(serde_json::from_value(serde_json::json!({
            "id": id, "definitionId": "switch", "reference": id, "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front"
        })).unwrap());
    }
    document
}

async fn mounted() -> (Probe, web_sys::Element) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    mount_runtime(runtime).await
}

async fn mount_runtime(runtime: Rc<Runtime>) -> (Probe, web_sys::Element) {
    let probe = Probe {
        runtime,
        bindings: Rc::new(RefCell::new(None)),
        layers: Rc::new(RefCell::new(None)),
        macros: Rc::new(RefCell::new(None)),
        active_layer: Rc::new(RefCell::new(None)),
        generation: Rc::new(RefCell::new(None)),
        layer_name: Rc::new(RefCell::new(None)),
        hide_macros: Rc::new(RefCell::new(None)),
        renders: Rc::new(Cell::new(0)),
        rendered_revision: Rc::new(Cell::new(None)),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    rendered_after(&probe, 0).await;
    (probe, root)
}

/// The revision of the accepted document, if one is accepted.
fn runtime_revision(runtime: &Runtime) -> Option<u64> {
    runtime
        .model()
        .accepted
        .as_ref()
        .map(|accepted| accepted.document.revision)
}

/// Waits until the host has rendered the accepted document, so the next action carries the
/// accepted admission token and revision.
async fn rendered_accepted(probe: &Probe, runtime: &Runtime) {
    wait_until("the keymap host renders the accepted document", || {
        probe.rendered_revision.get() == runtime_revision(runtime)
    })
    .await;
}

/// Waits until the host has rendered after `mark` (taken before the change), so the DOM and
/// the probe show that change. Bounded by the shared wait bound, never a fixed sleep.
async fn rendered_after(probe: &Probe, mark: u64) {
    wait_until("the keymap host renders after the change", || {
        probe.renders.get() > mark
    })
    .await;
}

async fn binding(probe: &Probe, key: &str, keycode: &str) {
    let runtime = &probe.runtime;
    let mark = probe.renders.get();
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec![key.into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    rendered_after(probe, mark).await;
    let borrowed = probe.bindings.borrow();
    let actions = borrowed.as_ref().unwrap();
    let mut sequence = actions.request_sequence;
    let request_id = sequence() + 1;
    sequence.set(request_id);
    let accepted = runtime.model().accepted.unwrap();
    actions.on_change.call(BindingEditRequest {
        scope: runtime.scope().unwrap(),
        admission_token: accepted.token,
        admission_revision: accepted.document.revision,
        active_layer_id: "base".into(),
        target: BindingTarget::Key { key_id: key.into() },
        input_identity: None,
        field: BindingField::Behavior,
        editor_instance_id: actions.editor_instance_id,
        request_id,
        binding: KeyBinding::KeyPress {
            keycode: keycode.into(),
        },
    });
}

/// Drains held effects, then waits until `done` holds: the state the assertion after it needs.
async fn settle_until(runtime: &Rc<Runtime>, what: &str, done: impl Fn() -> bool) {
    wait_until_step(what, async || {
        support::run_pending(runtime).await;
        done()
    })
    .await;
}

/// The number of layers in the accepted keymap, once one is accepted.
fn layer_count(runtime: &Runtime) -> Option<usize> {
    let accepted = runtime.model().accepted?;
    Some(accepted.document.keymap.as_ref()?.layers.len())
}

/// Whether the accepted first macro has exactly the `expected` steps.
fn steps_are(runtime: &Runtime, expected: &[MacroStep]) -> bool {
    let accepted = runtime.model().accepted.unwrap();
    accepted
        .document
        .keymap
        .as_ref()
        .is_some_and(|keymap| keymap.macros[0].steps == expected)
}

fn key_binding(runtime: &Runtime, key: &str) -> Option<KeyBinding> {
    runtime.model().accepted?.document.keymap.as_ref()?.layers[0]
        .bindings
        .get(key)
        .cloned()
}

#[wasm_bindgen_test]
async fn two_binding_fields_queue_and_undo_in_commit_order() {
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    let (entered, release) = support::gate_next_core_reply(runtime);
    binding(&probe, "a", "A").await;
    support::drive_pending(runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    wait_until("the behaviour select shows the key-press value", || {
        root.query_selector("select[aria-label='a behavior']")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .is_some_and(|select| select.value() == "key-press")
    })
    .await;
    use wasm_bindgen::JsCast;
    let behavior = root
        .query_selector("select[aria-label='a behavior']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlSelectElement>()
        .unwrap();
    assert_eq!(behavior.value(), "key-press");
    binding(&probe, "b", "B").await;
    support::drive_pending(runtime);
    release.send(()).unwrap();
    settle_until(runtime, "both binding edits are accepted", || {
        key_binding(runtime, "a")
            == Some(KeyBinding::KeyPress {
                keycode: "A".into(),
            })
            && key_binding(runtime, "b")
                == Some(KeyBinding::KeyPress {
                    keycode: "B".into(),
                })
    })
    .await;
    assert_eq!(
        key_binding(runtime, "a"),
        Some(KeyBinding::KeyPress {
            keycode: "A".into()
        })
    );
    assert_eq!(
        key_binding(runtime, "b"),
        Some(KeyBinding::KeyPress {
            keycode: "B".into()
        })
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        runtime,
        "the undo of the second binding is accepted",
        || {
            key_binding(runtime, "a")
                == Some(KeyBinding::KeyPress {
                    keycode: "A".into(),
                })
                && !matches!(key_binding(runtime, "b"), Some(KeyBinding::KeyPress { .. }))
        },
    )
    .await;
    assert_eq!(
        key_binding(runtime, "a"),
        Some(KeyBinding::KeyPress {
            keycode: "A".into()
        })
    );
    assert!(!matches!(
        key_binding(runtime, "b"),
        Some(KeyBinding::KeyPress { .. })
    ));
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        runtime,
        "the undo of the first binding removes the keymap",
        || runtime.model().accepted.unwrap().document.keymap.is_none(),
    )
    .await;
    assert!(runtime.model().accepted.unwrap().document.keymap.is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn two_queued_layer_adds_get_distinct_ids() {
    let (probe, root) = mounted().await;
    let (second, second_root) = mount_runtime(probe.runtime.clone()).await;
    let runtime = &probe.runtime;
    let (entered, release) = support::gate_next_core_reply(runtime);
    let on_operation = probe.layers.borrow().as_ref().unwrap().on_operation;
    on_operation.call(KeymapLayerOperation::Add);
    support::drive_pending(runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    second
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Add);
    release.send(()).unwrap();
    settle_until(runtime, "both layer adds are accepted", || {
        layer_count(runtime) == Some(3)
    })
    .await;
    let accepted = runtime.model().accepted.unwrap();
    let layers = &accepted.document.keymap.as_ref().unwrap().layers;
    assert_eq!(layers.len(), 3);
    assert_ne!(layers[1].id, layers[2].id);
    assert_eq!(layers[1].name, "Layer 1");
    assert_eq!(layers[2].name, "Layer 2");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        runtime,
        "the undo of the second layer add is accepted",
        || layer_count(runtime) == Some(2),
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .layers
            .len(),
        2
    );
    root.remove();
    second_root.remove();
}

#[wasm_bindgen_test]
async fn macro_fields_queue_and_undo_separately() {
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({
        "layers": [{"id": "base", "name": "Base", "bindings": {}, "sensors": {}}],
        "macros": [{"id": "macro", "name": "Original", "tapMs": 30, "waitMs": 0, "steps": [{"kind": "tap", "binding": {"kind": "key-press", "keycode": "A"}}]}]
    })).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let submit = |target, change| {
        let borrowed = probe.macros.borrow();
        let actions = borrowed.as_ref().unwrap();
        let mut sequence = actions.request_sequence;
        let request_id = sequence() + 1;
        sequence.set(request_id);
        let accepted = runtime.model().accepted.unwrap();
        actions.on_change.call(MacroEditRequest {
            scope: runtime.scope().unwrap(),
            scope_generation: 0,
            admission_token: accepted.token,
            admission_revision: accepted.document.revision,
            editor_instance_id: actions.editor_instance_id,
            request_id,
            macro_id: Some("macro".into()),
            target,
            step_sequence: None,
            change: MacroEditChange::Change(change),
        });
    };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let mark = probe.renders.get();
    submit(
        MacroEditTarget::Name,
        boardstudio_core::model::MacroChange::Name {
            value: "Renamed".into(),
        },
    );
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    rendered_after(&probe, mark).await;
    submit(
        MacroEditTarget::TapMs,
        boardstudio_core::model::MacroChange::TapMs { value: 50 },
    );
    release.send(()).unwrap();
    settle_until(&runtime, "the name and tap edits are accepted", || {
        let keymap_macro = runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .clone();
        keymap_macro.name == "Renamed" && keymap_macro.tap_ms == 50
    })
    .await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 50);
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(&runtime, "the undo of the tap edit is accepted", || {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms
            == 30
    })
    .await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 30);
    // Only the submitted one-shot control is disabled, with no pending message.
    use wasm_bindgen::JsCast;
    let add = root
        .query_selector("section.m1-keymap-macros > button")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap();
    rendered_accepted(&probe, &runtime).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    add.click();
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    wait_until("the add control is disabled while its edit is held", || {
        add.has_attribute("disabled")
    })
    .await;
    assert!(add.has_attribute("disabled"));
    let tap = root
        .query_selector("input[aria-label='Renamed tapMs']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    assert!(!tap.disabled());
    assert!(!root.text_content().unwrap().contains("Saving macro"));
    release.send(()).unwrap();
    settle_until(
        &runtime,
        "the add is accepted and the add control re-enables",
        || {
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros
                .len()
                == 2
                && !add.has_attribute("disabled")
        },
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros
            .len(),
        2
    );
    assert!(!add.has_attribute("disabled"));
    root.remove();
}

#[wasm_bindgen_test]
async fn rename_of_layer_removed_before_execution_retires_with_reason() {
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}},{"id":"child","name":"Child","bindings":{},"sensors":{}}],"macros":[]})).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let mark = probe.renders.get();
    probe.active_layer.borrow().unwrap().set("child".into());
    rendered_after(&probe, mark).await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let mark = probe.renders.get();
    probe
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Remove {
            layer_id: "child".into(),
        });
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    // A render after the removal is held shows the pending state, not the one before it.
    rendered_after(&probe, mark).await;
    assert!(
        probe.layers.borrow().as_ref().unwrap().enabled,
        "layer fields stay enabled while removal is pending"
    );
    probe
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Rename {
            layer_id: "child".into(),
            name: "Renamed".into(),
        });
    release.send(()).unwrap();
    settle_until(&runtime, "the removal is accepted and the rename retires", || {
        layer_count(&runtime) == Some(1)
            && matches!(&probe.layers.borrow().as_ref().unwrap().feedback, Some(crate::layer_edit::KeymapLayerFeedback::Failed(message)) if message.contains("no longer exists"))
    })
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .layers
            .len(),
        1
    );
    assert!(
        matches!(&probe.layers.borrow().as_ref().unwrap().feedback, Some(crate::layer_edit::KeymapLayerFeedback::Failed(message)) if message.contains("no longer exists"))
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        "the undo of the removal restores the child layer",
        || {
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .layers
                .get(1)
                .is_some_and(|layer| layer.name == "Child")
        },
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .layers[1]
            .name,
        "Child"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn macro_field_can_queue_a_return_to_the_preceding_accepted_value() {
    use wasm_bindgen::JsCast;
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}}],"macros":[{"id":"macro","name":"Original","tapMs":30,"waitMs":0,"steps":[{"kind":"tap","binding":{"kind":"key-press","keycode":"A"}}]}]})).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let input = root
        .query_selector("input[aria-label='Original tapMs']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
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
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let mark = probe.renders.get();
    commit("50");
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    // A render after the commit shows the pending state; a check before it would pass on the old one.
    rendered_after(&probe, mark).await;
    assert!(!input.disabled());
    commit("30");
    release.send(()).unwrap();
    settle_until(&runtime, "the return to 30 is accepted", || {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms
            == 30
            && input.value() == "30"
    })
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms,
        30
    );
    assert_eq!(input.value(), "30");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(&runtime, "the undo of the 30 edit is accepted", || {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms
            == 50
    })
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms,
        50
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn failed_binding_restores_accepted_field_and_explains_failure() {
    use wasm_bindgen::JsCast;
    let (probe, root) = mounted().await;
    let runtime = &probe.runtime;
    binding(&probe, "a", "A").await;
    settle_until(
        runtime,
        "the binding editor renders its keycode field",
        || {
            root.query_selector("input[aria-label='a keycode']")
                .ok()
                .flatten()
                .is_some()
        },
    )
    .await;
    let input = root
        .query_selector("input[aria-label='a keycode']")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    support::fail_next_core_reply(runtime, "binding executor failed");
    input.focus().unwrap();
    input.set_value("B");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    input.blur().unwrap();
    settle_until(
        runtime,
        "the failed binding is restored and explained",
        || {
            key_binding(runtime, "a")
                == Some(KeyBinding::KeyPress {
                    keycode: "A".into(),
                })
                && input.value() == "A"
                && root
                    .text_content()
                    .is_some_and(|text| text.contains("binding executor failed"))
        },
    )
    .await;
    assert_eq!(
        key_binding(runtime, "a"),
        Some(KeyBinding::KeyPress {
            keycode: "A".into()
        })
    );
    assert_eq!(input.value(), "A");
    assert!(
        root.text_content()
            .unwrap()
            .contains("binding executor failed")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn queued_macro_steps_preserve_eligible_positions_and_retire_shifted_targets() {
    use boardstudio_core::model::{MacroChange, MacroStep};
    let tap = || MacroStep::Tap {
        binding: KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    };
    for removed in [None, Some(2), Some(0), Some(1)] {
        let runtime = support::new_runtime();
        let mut doc = document();
        doc.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}}],"macros":[{"id":"macro","name":"Original","tapMs":30,"waitMs":0,"steps":[{"kind":"tap","binding":{"kind":"key-press","keycode":"A"}},{"kind":"wait","ms":10},{"kind":"wait","ms":20}]}]})).unwrap());
        support::open_document(&runtime, doc).await;
        let (probe, root) = mount_runtime(runtime.clone()).await;
        let submit = |target, change| {
            let borrowed = probe.macros.borrow();
            let actions = borrowed.as_ref().unwrap();
            let mut sequence = actions.request_sequence;
            let request_id = sequence() + 1;
            sequence.set(request_id);
            let accepted = runtime.model().accepted.unwrap();
            actions.on_change.call(MacroEditRequest {
                scope: runtime.scope().unwrap(),
                scope_generation: 0,
                admission_token: accepted.token,
                admission_revision: accepted.document.revision,
                editor_instance_id: actions.editor_instance_id,
                request_id,
                macro_id: Some("macro".into()),
                target,
                step_sequence: Some(actions.sequences[0].steps.clone()),
                change: MacroEditChange::Change(change),
            });
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        if let Some(index) = removed {
            submit(
                MacroEditTarget::RemoveStep { index },
                MacroChange::RemoveStep { index },
            );
        } else {
            submit(
                MacroEditTarget::AddStep,
                MacroChange::AddStep { value: tap() },
            );
        }
        support::drive_pending(&runtime);
        wait_until_future("the held edit reaches the Core executor", entered)
            .await
            .unwrap();
        wait_until("the pending macro step controls are disabled", || {
            root.query_selector("fieldset.m1-keymap-macro > button")
                .ok()
                .flatten()
                .is_some_and(|button| button.has_attribute("disabled"))
        })
        .await;
        // One sequence-changing action at a time; all step fields remain available.
        assert!(
            root.query_selector("fieldset.m1-keymap-macro > button")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        assert!(
            root.query_selector("div.m1-keymap-macro-step > button")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        assert!(
            !root
                .query_selector("div.m1-keymap-macro-step select")
                .unwrap()
                .unwrap()
                .has_attribute("disabled")
        );
        // A stale callback cannot queue an add behind a pending removal and restore the old length.
        if removed.is_some() {
            submit(
                MacroEditTarget::AddStep,
                MacroChange::AddStep { value: tap() },
            );
        }
        submit(
            MacroEditTarget::StepDelay { index: 1 },
            MacroChange::Step {
                index: 1,
                value: MacroStep::Wait { ms: 99 },
            },
        );
        release.send(()).unwrap();
        let expected = match removed {
            None => vec![
                tap(),
                MacroStep::Wait { ms: 99 },
                MacroStep::Wait { ms: 20 },
                tap(),
            ],
            Some(2) => vec![tap(), MacroStep::Wait { ms: 99 }],
            Some(0) => vec![MacroStep::Wait { ms: 10 }, MacroStep::Wait { ms: 20 }],
            _ => vec![tap(), MacroStep::Wait { ms: 20 }],
        };
        // The step field's failure renders after Core accepts the edits, so the wait includes it.
        let eligible = removed.is_none_or(|index| index > 1);
        settle_until(&runtime, "the queued macro steps are accepted", || {
            steps_are(&runtime, &expected)
                && (eligible
                    || root
                        .query_selector("label small[role='alert']")
                        .ok()
                        .flatten()
                        .and_then(|alert| alert.text_content())
                        .is_some_and(|message| message.contains("macro step")))
        })
        .await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros[0]
                .steps,
            expected
        );
        assert!(
            probe.macros.borrow().as_ref().unwrap().feedback.is_none(),
            "a bound step field reports at the field, never in the panel status"
        );
        if !eligible {
            assert!(
                root.query_selector("label small[role='alert']")
                    .unwrap()
                    .and_then(|alert| alert.text_content())
                    .is_some_and(|message| message.contains("macro step")),
                "an ineligible step edit reports its failure at the step field"
            );
        }
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        let final_steps = vec![
            tap(),
            MacroStep::Wait { ms: 10 },
            MacroStep::Wait { ms: 20 },
        ];
        settle_until(&runtime, "the undo is accepted", || {
            if eligible {
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .keymap
                    .as_ref()
                    .unwrap()
                    .macros[0]
                    .steps
                    .get(1)
                    == Some(&MacroStep::Wait { ms: 10 })
            } else {
                steps_are(&runtime, &final_steps)
            }
        })
        .await;
        if eligible {
            assert_eq!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .keymap
                    .as_ref()
                    .unwrap()
                    .macros[0]
                    .steps[1],
                MacroStep::Wait { ms: 10 }
            );
            runtime.submit(Event::Undo {
                operation_id: runtime.operation(),
            });
            settle_until(&runtime, "the second undo restores the final steps", || {
                steps_are(&runtime, &final_steps)
            })
            .await;
        }
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros[0]
                .steps,
            vec![
                tap(),
                MacroStep::Wait { ms: 10 },
                MacroStep::Wait { ms: 20 }
            ]
        );
        root.remove();
    }
}

fn rename_base(probe: &Probe, name: &str) {
    probe
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Rename {
            layer_id: "base".into(),
            name: name.into(),
        });
}

#[wasm_bindgen_test]
async fn a_newer_layer_name_survives_an_older_failure_that_reports_inline() {
    let (probe, root) = mounted().await;
    let runtime = probe.runtime.clone();
    let (mut draft, failure) = probe.layer_name.borrow().unwrap();
    draft.set("A".into());
    support::fail_next_core_reply(&runtime, "layer executor failed");
    rename_base(&probe, "A");
    // The older rename has not settled when the user types the next draft.
    draft.set("AB".into());
    settle_until(&runtime, "the older failure is reported inline", || {
        failure().is_some_and(|message| message.contains("layer executor failed"))
    })
    .await;
    assert_eq!(draft(), "AB", "a newer draft is never overwritten");
    assert!(
        failure().is_some_and(|message| message.contains("layer executor failed")),
        "the older failure reports inline at the name field"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn leaving_the_keymap_owner_retires_a_pending_layer_name_silently() {
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}},{"id":"child","name":"Child","bindings":{},"sensors":{}}],"macros":[]})).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let mark = probe.renders.get();
    probe.active_layer.borrow().unwrap().set("child".into());
    rendered_after(&probe, mark).await;
    let (mut draft, failure) = probe.layer_name.borrow().unwrap();
    // Removing the layer is held in Core; the rename behind it can only fail once it runs.
    let (entered, release) = support::gate_next_core_reply(&runtime);
    probe
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Remove {
            layer_id: "child".into(),
        });
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    draft.set("Gone".into());
    probe
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Rename {
            layer_id: "child".into(),
            name: "Gone".into(),
        });
    // The owner departs before either edit settles.
    let mut generation = probe.generation.borrow().unwrap();
    let mark = probe.renders.get();
    generation.set(generation() + 1);
    rendered_after(&probe, mark).await;
    release.send(()).unwrap();
    settle_until(&runtime, "the removal is accepted", || {
        layer_count(&runtime) == Some(1)
    })
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .layers
            .len(),
        1,
        "the queued Session edits kept executing"
    );
    assert_eq!(draft(), "Gone", "a departed owner leaves the draft alone");
    assert!(
        failure().is_none(),
        "the rename would fail with \"no longer exists\"; retirement keeps it silent"
    );
    assert!(
        probe.layers.borrow().as_ref().unwrap().feedback.is_none(),
        "no status survives the owner"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn an_unmounted_macro_field_does_not_break_its_held_edit_and_remounts_cleanly() {
    use wasm_bindgen::JsCast;
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({
        "layers": [{"id": "base", "name": "Base", "bindings": {}, "sensors": {}}],
        "macros": [{"id": "macro", "name": "Original", "tapMs": 30, "waitMs": 0, "steps": [{"kind": "tap", "binding": {"kind": "key-press", "keycode": "A"}}]}]
    })).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let tap = || {
        root.query_selector("input[aria-label='Original tapMs']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let input = tap();
    input.focus().unwrap();
    input.set_value("55");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    input.blur().unwrap();
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    let mut hide = probe.hide_macros.borrow().unwrap();
    hide.set(true);
    wait_until(
        "the macro field is unmounted while its edit is held",
        || {
            root.query_selector("input[aria-label='Original tapMs']")
                .ok()
                .flatten()
                .is_none()
        },
    )
    .await;
    assert!(
        root.query_selector("input[aria-label='Original tapMs']")
            .unwrap()
            .is_none(),
        "the macro field is unmounted while its edit is held"
    );
    release.send(()).unwrap();
    settle_until(&runtime, "the held macro edit is accepted", || {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms
            == 55
    })
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms,
        55,
        "the queued edit kept executing"
    );
    hide.set(false);
    wait_until("the remounted field shows the accepted value", || {
        root.query_selector("input[aria-label='Original tapMs']")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
            .is_some_and(|input| input.value() == "55")
    })
    .await;
    assert_eq!(
        tap().value(),
        "55",
        "the remounted field shows the accepted value"
    );
    root.remove();
}

fn macro_doc() -> ProjectDoc {
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({
        "layers": [{"id": "base", "name": "Base", "bindings": {}, "sensors": {}}],
        "macros": [{"id": "macro", "name": "Original", "tapMs": 30, "waitMs": 0, "steps": [{"kind": "tap", "binding": {"kind": "key-press", "keycode": "A"}}]}]
    })).unwrap());
    doc
}

fn macro_input(root: &web_sys::Element, label: &str) -> web_sys::HtmlInputElement {
    use wasm_bindgen::JsCast;
    root.query_selector(&format!("input[aria-label='{label}']"))
        .unwrap()
        .unwrap()
        .dyn_into()
        .unwrap()
}

fn type_value(input: &web_sys::HtmlInputElement, value: &str) {
    input.set_value(value);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
}

/// A held failing macro field edit with a newer dirty draft typed behind it: the older
/// failure must report at the field, and nowhere in the panel status.
async fn newer_macro_draft_with_older_failure(label: &str, first: &str, newer: &str) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, macro_doc()).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    let input = macro_input(&root, label);
    support::fail_next_core_reply(&runtime, "macro executor failed");
    let mark = probe.renders.get();
    input.focus().unwrap();
    type_value(&input, first);
    input.blur().unwrap();
    rendered_after(&probe, mark).await;
    // The older edit has not settled when the user types the next draft.
    type_value(&input, newer);
    settle_until(
        &runtime,
        "the newer draft and the older inline failure are shown",
        || {
            macro_input(&root, label).value() == newer
                && root
                    .query_selector("label small[role='alert']")
                    .ok()
                    .flatten()
                    .and_then(|alert| alert.text_content())
                    .is_some_and(|text| text.contains("macro executor failed"))
        },
    )
    .await;
    let input = macro_input(&root, label);
    assert_eq!(input.value(), newer, "a newer draft is never overwritten");
    let alert = root
        .query_selector("label small[role='alert']")
        .unwrap()
        .expect("the older failure reports inline beside the newer draft");
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("macro executor failed")
    );
    assert!(
        root.query_selector("p.m1-keymap-macro-status")
            .unwrap()
            .is_none(),
        "a bound field's failure is not duplicated in the panel status"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_newer_macro_field_draft_keeps_an_older_failure_inline_only() {
    for (label, first, newer) in [
        ("Macro name Original", "Renamed", "Renamed again"),
        ("Original tapMs", "55", "66"),
    ] {
        newer_macro_draft_with_older_failure(label, first, newer).await;
    }
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_macro_number_restores_the_accepted_value_with_an_inline_failure() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, macro_doc()).await;
    let (_probe, root) = mount_runtime(runtime.clone()).await;
    support::fail_next_core_reply(&runtime, "macro executor failed");
    let input = macro_input(&root, "Original tapMs");
    input.focus().unwrap();
    type_value(&input, "55");
    input.blur().unwrap();
    settle_until(
        &runtime,
        "the unchanged macro number is restored with its failure",
        || {
            macro_input(&root, "Original tapMs").value() == "30"
                && root
                    .query_selector("label small[role='alert']")
                    .ok()
                    .flatten()
                    .and_then(|alert| alert.text_content())
                    .is_some_and(|text| text.contains("macro executor failed"))
        },
    )
    .await;
    assert_eq!(macro_input(&root, "Original tapMs").value(), "30");
    let alert = root
        .query_selector("label small[role='alert']")
        .unwrap()
        .expect("the failure reports inline");
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("macro executor failed")
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_second_macro_field_edit_replaces_the_observation_and_both_edits_undo() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, macro_doc()).await;
    let (_probe, root) = mount_runtime(runtime.clone()).await;
    let tap_ms = |runtime: &Rc<Runtime>| {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .tap_ms
    };
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let input = macro_input(&root, "Original tapMs");
    input.focus().unwrap();
    type_value(&input, "55");
    input.blur().unwrap();
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    input.focus().unwrap();
    type_value(&input, "66");
    input.blur().unwrap();
    support::drive_pending(&runtime);
    release.send(()).unwrap();
    settle_until(
        &runtime,
        "the second macro edit is accepted and shown",
        || tap_ms(&runtime) == 66 && macro_input(&root, "Original tapMs").value() == "66",
    )
    .await;
    assert_eq!(tap_ms(&runtime), 66);
    assert_eq!(macro_input(&root, "Original tapMs").value(), "66");
    assert!(
        root.query_selector("label small[role='alert']")
            .unwrap()
            .is_none()
    );
    for expected in [55, 30] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        settle_until(&runtime, "the undo is accepted", || {
            tap_ms(&runtime) == expected
        })
        .await;
        assert_eq!(tap_ms(&runtime), expected, "each edit has an Undo step");
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_layer_name_restores_the_accepted_name_with_an_inline_failure() {
    let (probe, root) = mounted().await;
    let runtime = probe.runtime.clone();
    let (mut draft, failure) = probe.layer_name.borrow().unwrap();
    draft.set("A".into());
    support::fail_next_core_reply(&runtime, "layer executor failed");
    rename_base(&probe, "A");
    settle_until(
        &runtime,
        "the unchanged layer name is restored with its failure",
        || {
            draft() == "Base"
                && failure().is_some_and(|message| message.contains("layer executor failed"))
        },
    )
    .await;
    assert_eq!(
        draft(),
        "Base",
        "the unchanged draft restores the accepted name"
    );
    assert!(failure().is_some_and(|message| message.contains("layer executor failed")));
    root.remove();
}
