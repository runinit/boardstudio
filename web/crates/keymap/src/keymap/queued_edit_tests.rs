//! Mounted controller requests run through the real Session and Core, with gated replies.
use super::binding_controller::{BindingActions, BindingProjectionSources, use_binding_operations};
use super::binding_editor::{BindingEditRequest, BindingField, BindingTarget};
use super::layer_controller::{LayerActions, LayerSource, use_layer_operations};
use super::layer_edit::KeymapLayerOperation;
use super::macro_controller::{MacroActions, use_macro_operations};
use super::macro_editor::{MacroEditChange, MacroEditRequest, MacroEditTarget};
use crate::runtime::{Runtime, project_name_test_support as support};
use boardstudio_application::{Event, SelectionMode};
use boardstudio_core::model::{KeyBinding, ProjectDoc};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_test::*;

#[derive(Clone)]
struct Probe {
    runtime: Rc<Runtime>,
    bindings: Rc<RefCell<Option<BindingActions>>>,
    layers: Rc<RefCell<Option<LayerActions>>>,
    macros: Rc<RefCell<Option<MacroActions>>>,
    active_layer: Rc<RefCell<Option<Signal<String>>>>,
}

fn host() -> Element {
    let probe = use_context::<Probe>();
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
    let macro_actions = use_macro_operations(
        runtime.clone(),
        source.clone(),
        workspace,
        generation,
        Rc::new(|| true),
    );
    let macro_ui = macro_actions.source.as_ref().map(|source| rsx! { super::macro_editor::MacroEditor {
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
    rendered().await;
    (probe, root)
}

async fn rendered() {
    gloo_timers::future::TimeoutFuture::new(50).await;
}

async fn binding(probe: &Probe, key: &str, keycode: &str) {
    let runtime = &probe.runtime;
    runtime.submit(Event::SelectParts {
        operation_id: runtime.operation(),
        part_ids: vec![key.into()],
        range_part_ids: vec![],
        mode: SelectionMode::Replace,
    });
    rendered().await;
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

async fn settle(runtime: &Rc<Runtime>) {
    for _ in 0..15 {
        support::run_pending(runtime).await;
        rendered().await;
    }
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
    entered.await.unwrap();
    binding(&probe, "b", "B").await;
    support::drive_pending(runtime);
    release.send(()).unwrap();
    settle(runtime).await;
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
    settle(runtime).await;
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
    settle(runtime).await;
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
    entered.await.unwrap();
    second
        .layers
        .borrow()
        .as_ref()
        .unwrap()
        .on_operation
        .call(KeymapLayerOperation::Add);
    release.send(()).unwrap();
    settle(runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    let layers = &accepted.document.keymap.as_ref().unwrap().layers;
    assert_eq!(layers.len(), 3);
    assert_ne!(layers[1].id, layers[2].id);
    assert_eq!(layers[1].name, "Layer 1");
    assert_eq!(layers[2].name, "Layer 2");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(runtime).await;
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
    submit(
        MacroEditTarget::Name,
        boardstudio_core::model::MacroChange::Name {
            value: "Renamed".into(),
        },
    );
    support::drive_pending(&runtime);
    entered.await.unwrap();
    rendered().await;
    submit(
        MacroEditTarget::TapMs,
        boardstudio_core::model::MacroChange::TapMs { value: 50 },
    );
    release.send(()).unwrap();
    settle(&runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 50);
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(&runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 30);
    root.remove();
}

#[wasm_bindgen_test]
async fn rename_of_layer_removed_before_execution_retires_with_reason() {
    let runtime = support::new_runtime();
    let mut doc = document();
    doc.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}},{"id":"child","name":"Child","bindings":{},"sensors":{}}],"macros":[]})).unwrap());
    support::open_document(&runtime, doc).await;
    let (probe, root) = mount_runtime(runtime.clone()).await;
    probe.active_layer.borrow().unwrap().set("child".into());
    rendered().await;
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
    entered.await.unwrap();
    rendered().await;
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
    settle(&runtime).await;
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
        matches!(&probe.layers.borrow().as_ref().unwrap().feedback, Some(super::layer_edit::KeymapLayerFeedback::Failed(message)) if message.contains("no longer exists"))
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle(&runtime).await;
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
    let (_, root) = mount_runtime(runtime.clone()).await;
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
    commit("50");
    support::drive_pending(&runtime);
    entered.await.unwrap();
    rendered().await;
    assert!(!input.disabled());
    commit("30");
    release.send(()).unwrap();
    settle(&runtime).await;
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
    settle(&runtime).await;
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
    settle(runtime).await;
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
    settle(runtime).await;
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
