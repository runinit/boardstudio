//! Mounted controller requests run through the real Session and Core, with gated replies.
use super::binding_controller::{BindingActions, BindingProjectionSources, use_binding_operations};
use super::binding_editor::{BindingEditRequest, BindingField, BindingTarget};
use super::layer_controller::{LayerActions, LayerSource, use_layer_operations};
use super::macro_controller::{MacroActions, use_macro_operations};
use super::macro_editor::{MacroEditChange, MacroEditRequest, MacroEditTarget};
use crate::layer_edit::KeymapLayerOperation;
use crate::runtime::Runtime;
use boardstudio_application::{Event, SelectionMode};
use boardstudio_model::model::{KeyBinding, MacroStep, ProjectDoc};
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::{settle_until, wait_until, wait_until_future};
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

/// A document with the shared single-macro keymap: one `base` layer and the
/// `Original` macro (tap A). Queued macro scenarios share this shape instead
/// of repeating the JSON.
fn macro_document() -> ProjectDoc {
    let mut document = document();
    document.keymap = Some(serde_json::from_value(serde_json::json!({
        "layers": [{"id": "base", "name": "Base", "bindings": {}, "sensors": {}}],
        "macros": [{"id": "macro", "name": "Original", "tapMs": 30, "waitMs": 0, "steps": [{"kind": "tap", "binding": {"kind": "key-press", "keycode": "A"}}]}]
    })).unwrap());
    document
}

/// A document with `base` and `child` layers and no macros for layer
/// queue/removal scenarios.
fn child_layer_document() -> ProjectDoc {
    let mut document = document();
    document.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}},{"id":"child","name":"Child","bindings":{},"sensors":{}}],"macros":[]})).unwrap());
    document
}

/// A document with the shared macro plus two wait steps for step-queue scenarios.
fn wait_steps_document() -> ProjectDoc {
    let mut document = document();
    document.keymap = Some(serde_json::from_value(serde_json::json!({"layers":[{"id":"base","name":"Base","bindings":{},"sensors":{}}],"macros":[{"id":"macro","name":"Original","tapMs":30,"waitMs":0,"steps":[{"kind":"tap","binding":{"kind":"key-press","keycode":"A"}},{"kind":"wait","ms":10},{"kind":"wait","ms":20}]}]})).unwrap());
    document
}

async fn mounted() -> (Probe, web_sys::Element, MountedFixture) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    mount_runtime(runtime).await
}

async fn mount_runtime(runtime: Rc<Runtime>) -> (Probe, web_sys::Element, MountedFixture) {
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
    let mounted = MountedFixture::new(root.clone(), dom);
    rendered_after(&probe, 0).await;
    (probe, root, mounted)
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
    let (probe, root, _mounted) = mounted().await;
    let runtime = &probe.runtime;
    let (entered, release) = support::gate_next_core_reply(runtime);
    binding(&probe, "a", "A").await;
    support::drive_pending(runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    wait_until("the behaviour select shows the key-press value", || {
        by_role_name(&root, "button", "a behavior").is_some_and(|select| {
            select
                .text_content()
                .is_some_and(|text| text.trim() == "Key press")
        })
    })
    .await;
    let behavior = by_role_name(&root, "button", "a behavior").unwrap();
    assert_eq!(behavior.text_content().unwrap().trim(), "Key press");
    binding(&probe, "b", "B").await;
    support::drive_pending(runtime);
    release.send(()).unwrap();
    settle_until(
        runtime,
        support::run_pending,
        "both binding edits are accepted",
        || {
            key_binding(runtime, "a")
                == Some(KeyBinding::KeyPress {
                    keycode: "A".into(),
                })
                && key_binding(runtime, "b")
                    == Some(KeyBinding::KeyPress {
                        keycode: "B".into(),
                    })
        },
    )
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
        support::run_pending,
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
        support::run_pending,
        "the undo of the first binding removes the keymap",
        || runtime.model().accepted.unwrap().document.keymap.is_none(),
    )
    .await;
    assert!(runtime.model().accepted.unwrap().document.keymap.is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn two_queued_layer_adds_get_distinct_ids() {
    let (probe, root, _mounted) = mounted().await;
    let (second, second_root, _second_mounted) = mount_runtime(probe.runtime.clone()).await;
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
    settle_until(
        runtime,
        support::run_pending,
        "both layer adds are accepted",
        || layer_count(runtime) == Some(3),
    )
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
        support::run_pending,
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
    let doc = macro_document();
    support::open_document(&runtime, doc).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
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
        boardstudio_model::model::MacroChange::Name {
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
        boardstudio_model::model::MacroChange::TapMs { value: 50 },
    );
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "the name and tap edits are accepted",
        || {
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
        },
    )
    .await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 50);
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "the undo of the tap edit is accepted",
        || {
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
        },
    )
    .await;
    let accepted = runtime.model().accepted.unwrap();
    let item = &accepted.document.keymap.as_ref().unwrap().macros[0];
    assert_eq!(item.name, "Renamed");
    assert_eq!(item.tap_ms, 30);
    // Only the submitted one-shot control is disabled, with no pending message.
    let add = by_role_name(&root, "button", "Add macro").unwrap();
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
    let tap = macro_input(&root, "Renamed tapMs");
    assert!(!tap.disabled());
    assert!(!root.text_content().unwrap().contains("Saving macro"));
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
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
    let doc = child_layer_document();
    support::open_document(&runtime, doc).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
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
    settle_until(&runtime, support::run_pending, "the removal is accepted and the rename retires", || {
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
        support::run_pending,
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
    let runtime = support::new_runtime();
    let doc = macro_document();
    support::open_document(&runtime, doc).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
    let input = macro_input(&root, "Original tapMs");
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
    settle_until(
        &runtime,
        support::run_pending,
        "the return to 30 is accepted",
        || {
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
            .macros[0]
            .tap_ms,
        30
    );
    assert_eq!(input.value(), "30");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "the undo of the 30 edit is accepted",
        || {
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
            .macros[0]
            .tap_ms,
        50
    );
    root.remove();
}

/// Mount with key `a` bound to `A` and return its keycode combobox input.
/// The three keycode scenarios share this admission instead of repeating the
/// mount/bind/settle/lookup sequence.
async fn open_keycode_input(probe: &Probe, root: &web_sys::Element) -> web_sys::HtmlInputElement {
    use wasm_bindgen::JsCast;
    let runtime = &probe.runtime;
    binding(probe, "a", "A").await;
    settle_until(
        runtime,
        support::run_pending,
        "the binding editor renders its keycode field",
        || by_role_name(root, "combobox", "a keycode").is_some(),
    )
    .await;
    by_role_name(root, "combobox", "a keycode")
        .expect("keycode combobox")
        .dyn_into()
        .unwrap()
}

fn commit_input_value(input: &web_sys::HtmlInputElement, value: &str) {
    input.focus().unwrap();
    input.set_value(value);
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    input.blur().unwrap();
}

#[wasm_bindgen_test]
async fn failed_binding_restores_accepted_field_and_explains_failure() {
    let (probe, root, _mounted) = mounted().await;
    let runtime = &probe.runtime;
    let input = open_keycode_input(&probe, &root).await;
    support::fail_next_core_reply(runtime, "binding executor failed");
    commit_input_value(&input, "B");
    settle_until(
        runtime,
        support::run_pending,
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
async fn keycode_combobox_commits_arbitrary_code_on_blur_and_keeps_it_displayed() {
    let (probe, root, _mounted) = mounted().await;
    let runtime = &probe.runtime;
    let input = open_keycode_input(&probe, &root).await;
    commit_input_value(&input, "CUSTOM_CODE");
    settle_until(
        runtime,
        support::run_pending,
        "arbitrary keycode is accepted",
        || {
            key_binding(runtime, "a")
                == Some(KeyBinding::KeyPress {
                    keycode: "CUSTOM_CODE".into(),
                })
        },
    )
    .await;
    assert_eq!(input.value(), "CUSTOM_CODE");
    root.remove();
}

#[wasm_bindgen_test]
async fn keycode_combobox_filters_and_commits_a_suggestion_on_blur() {
    let (probe, root, _mounted) = mounted().await;
    let runtime = &probe.runtime;
    let input = open_keycode_input(&probe, &root).await;
    input.focus().unwrap();
    input.set_value("Backspace");
    let event = web_sys::EventInit::new();
    event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
        .unwrap();
    wait_until("the list is filtered", || {
        by_role_name(&root, "option", "BSPC · Backspace").is_some()
    })
    .await;
    let key = |value: &str| {
        let event = web_sys::KeyboardEventInit::new();
        event.set_key(value);
        event.set_bubbles(true);
        event.set_cancelable(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
    };
    key("ArrowDown");
    wait_until("the suggestion is active", || {
        by_role_name(&root, "option", "BSPC · Backspace").is_some_and(|option| {
            input.get_attribute("aria-activedescendant").as_deref() == Some(option.id().as_str())
        })
    })
    .await;
    key("Enter");
    input.blur().unwrap();
    settle_until(
        runtime,
        support::run_pending,
        "the suggested code is accepted",
        || {
            key_binding(runtime, "a")
                == Some(KeyBinding::KeyPress {
                    keycode: "BSPC".into(),
                })
        },
    )
    .await;
    assert_eq!(input.value(), "BSPC");
    root.remove();
}

#[wasm_bindgen_test]
async fn queued_macro_steps_preserve_eligible_positions_and_retire_shifted_targets() {
    use boardstudio_model::model::{MacroChange, MacroStep};
    let tap = || MacroStep::Tap {
        binding: KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    };
    for removed in [None, Some(2), Some(0), Some(1)] {
        let runtime = support::new_runtime();
        let doc = wait_steps_document();
        support::open_document(&runtime, doc).await;
        let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
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
            by_role_name(&root, "button", "Add step")
                .is_some_and(|button| button.has_attribute("disabled"))
        })
        .await;
        // One sequence-changing action at a time; all step fields remain available.
        assert!(
            by_role_name(&root, "button", "Add step")
                .unwrap()
                .has_attribute("disabled")
        );
        assert!(
            by_role_name(&root, "button", "Remove step 1")
                .unwrap()
                .has_attribute("disabled")
        );
        assert!(
            !by_role_name(&root, "button", "Original step 1")
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
        settle_until(
            &runtime,
            support::run_pending,
            "the queued macro steps are accepted",
            || {
                steps_are(&runtime, &expected)
                    && (eligible
                        || root
                            .query_selector("[role='alert']")
                            .ok()
                            .flatten()
                            .and_then(|alert| alert.text_content())
                            .is_some_and(|message| message.contains("macro step")))
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
                root.query_selector("[role='alert']")
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
        settle_until(
            &runtime,
            support::run_pending,
            "the undo is accepted",
            || {
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
            },
        )
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
            settle_until(
                &runtime,
                support::run_pending,
                "the second undo restores the final steps",
                || steps_are(&runtime, &final_steps),
            )
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

mod layer_macro_tests;

fn macro_input(root: &web_sys::Element, label: &str) -> web_sys::HtmlInputElement {
    use wasm_bindgen::JsCast;
    by_role_name(
        root,
        if label.starts_with("Macro name") || label == "Keycode" {
            "textbox"
        } else {
            "spinbutton"
        },
        label,
    )
    .unwrap()
    .dyn_into()
    .unwrap()
}
