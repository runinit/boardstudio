use super::*;
use boardstudio_model::model::ProjectDoc;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::{
    settle_until, wait_until, wait_until_future, wait_until_step,
};
use wasm_bindgen_test::*;

fn definition() -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":"component", "name":"Component", "kind":"custom", "courtyard":[], "pads":[],
        "models":[{"assetId":"model", "offset":{"x":0,"y":0,"z":0}, "rotation":{"x":0,"y":0,"z":0}, "scale":{"x":1,"y":1,"z":1}}]
    })).unwrap()
}

async fn opened() -> Rc<Runtime> {
    let runtime = support::new_runtime();
    let mut document = ProjectDoc::empty("model-edits", "Model edits");
    document.definitions.push(definition());
    document.assets.push(Asset {
        id: "model".into(),
        name: "Model".into(),
        media_type: "model/stl".into(),
        sha256: "a".repeat(64),
        license: None,
        source: None,
    });
    support::open_document(&runtime, document).await;
    runtime
}

fn offset_edit(axis: Axis, value: f64) -> EditResolver {
    model_resolver(definition(), true, move |definition, _| {
        let model = definition
            .models
            .as_mut()
            .and_then(|models| models.first_mut())
            .ok_or("The attached model is no longer available.")?;
        set_model_axis(model, VectorField::Offset, axis, value);
        Ok(())
    })
}

fn host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let _ = version();
    use_hook({
        let runtime = runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    let accepted = runtime.model().accepted.unwrap();
    let model = initial_model(&accepted.document.definitions[0]).unwrap();
    rsx! { ModelVectorEditor { title: "Offset", value: model.offset, unit: "mm", positive: false,
        on_commit: move |(axis, _submitted, value)| Some(offset_edit(axis, value))
    } }
}

type UploadEventProbe = Rc<std::cell::RefCell<Option<FormEvent>>>;

fn upload_host() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let event_probe = use_context::<UploadEventProbe>();
    let selected = use_signal(|| Some((runtime.scope(), "component".into())));
    let selection_generation = use_signal(|| 1_u64);
    use_context_provider(|| PartsSelectionGeneration(selection_generation));
    let scope_generation = use_signal(|| 1_u64);
    let selected_context = use_signal(|| None);
    let anchor_scope = use_signal(|| runtime.scope());
    use_context_provider(|| {
        SelectionAdapter::new(selected_context, anchor_scope, scope_generation)
    });
    let workspace = use_signal(|| "Parts");
    use_context_provider(|| WorkspaceState(workspace));
    let version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let _ = version();
    use_hook({
        let runtime = runtime.clone();
        move || {
            runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version += 1;
            }))
        }
    });
    let snapshot = runtime.model().accepted.unwrap();
    let definition = snapshot.document.definitions[0].clone();
    rsx! {
        div {
            onchange: move |event| *event_probe.borrow_mut() = Some(event),
            ComponentModelEditor {
                snapshot,
                scope: runtime.scope(),
                selected,
                definition,
                project_owned: true,
            }
        }
    }
}

#[wasm_bindgen_test]
async fn mounted_model_upload_rejects_a_retained_handler_while_pending() {
    let runtime = opened().await;
    let handler_probe: UploadHandlerProbe = Rc::new(std::cell::RefCell::new(None));
    let event_probe: UploadEventProbe = Rc::new(std::cell::RefCell::new(None));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(upload_host);
    dom.provide_root_context(runtime.clone());
    dom.provide_root_context(handler_probe.clone());
    dom.provide_root_context(event_probe.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("model upload control mounts", || {
        root.query_selector("input[type=file]").unwrap().is_some()
            && handler_probe.borrow().is_some()
    })
    .await;
    let input = root
        .query_selector("input[type=file]")
        .unwrap()
        .unwrap()
        .dyn_into::<HtmlInputElement>()
        .unwrap();
    let file = web_sys::File::new_with_str_sequence(
        &js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(
            "solid test\nendsolid test\n",
        )),
        "retained-model.stl",
    )
    .unwrap();
    let descriptor = js_sys::Object::new();
    js_sys::Reflect::set(&descriptor, &"value".into(), &js_sys::Array::of1(&file)).unwrap();
    js_sys::Object::define_property(
        input.unchecked_ref::<js_sys::Object>(),
        &"files".into(),
        &descriptor,
    );
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
        .unwrap();
    let mut entered_core = false;
    wait_until_step("uploaded model enters Core", async || {
        support::drive_pending(&runtime);
        if !entered_core {
            entered_core = entered.try_recv().unwrap().is_some();
        }
        entered_core
    })
    .await;
    assert!(input.disabled(), "the pending upload disables its control");
    assert!(
        !root
            .text_content()
            .unwrap_or_default()
            .contains("Reading and saving model file"),
        "file preparation has finished while the upload edit is held"
    );
    let retained = handler_probe.borrow().as_ref().copied().unwrap();
    let event = event_probe.borrow().clone().unwrap();
    for _ in 0..10 {
        retained.call(event.clone());
    }
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "uploaded model lands once",
        || {
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .assets
                .iter()
                .filter(|asset| asset.name == "retained-model.stl")
                .count()
                == 1
                && !input.disabled()
        },
    )
    .await;
    assert_eq!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .assets
            .iter()
            .filter(|asset| asset.name == "retained-model.stl")
            .count(),
        1,
        "a retained upload handler cannot admit a second model while the first is pending"
    );
    runtime.unsubscribe();
    root.remove();
}

fn input(root: &web_sys::Element, axis: &str) -> HtmlInputElement {
    by_role_name(root, "spinbutton", &format!("Offset {axis} mm"))
        .unwrap_or_else(|| panic!("offset {axis} field"))
        .dyn_into()
        .unwrap()
}
fn type_value(input: &HtmlInputElement, value: &str) {
    input.set_value(value);
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
        .unwrap();
}
async fn commit(input: &HtmlInputElement, value: &str) {
    input.focus().unwrap();
    type_value(input, value);
    input.blur().unwrap();
}

#[wasm_bindgen_test]
async fn mounted_model_axis_can_commit_the_same_text_after_undo() {
    let runtime = opened().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("model offset input mounts", || {
        by_role_name(&root, "spinbutton", "Offset X mm").is_some()
    })
    .await;
    let x = input(&root, "X");
    commit(&x, "4").await;
    settle_until(
        &runtime,
        support::run_pending,
        "first model axis edit lands",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 4.0
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset
            .x,
        4.0,
        "the first model alignment edit lands"
    );
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "model axis Undo renders",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 0.0
                && x.value() == "0"
        },
    )
    .await;
    assert_eq!(x.value(), "0", "Undo restores the accepted axis value");

    commit(&x, "4").await;
    settle_until(
        &runtime,
        support::run_pending,
        "same model axis edit lands again",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 4.0
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset
            .x,
        4.0,
        "the same typed text can land again after Undo"
    );
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_model_axis_preserves_newer_baseline_typing_when_edit_lands() {
    let runtime = opened().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("model offset input mounts", || {
        by_role_name(&root, "spinbutton", "Offset X mm").is_some()
    })
    .await;
    let x = input(&root, "X");
    let (entered, release) = support::gate_next_core_reply(&runtime);
    commit(&x, "4").await;
    support::drive_pending(&runtime);
    wait_until_future("model axis commit enters Core", entered)
        .await
        .unwrap();
    x.focus().unwrap();
    type_value(&x, "0");
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "older model axis edit lands",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 4.0
                && x.value() == "0"
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset
            .x,
        4.0,
        "the older committed axis value lands"
    );
    assert_eq!(
        x.value(),
        "0",
        "new typing at the previous accepted baseline survives the older landing"
    );

    x.blur().unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "preserved baseline draft commits",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 0.0
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset
            .x,
        0.0,
        "the preserved baseline draft can commit after the older edit lands"
    );
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "clean axis draft follows Undo",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 4.0
                && x.value() == "4"
        },
    )
    .await;
    assert_eq!(x.value(), "4", "the clean axis follows Undo");
    runtime.submit(boardstudio_application::Event::Redo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "clean axis draft follows Redo",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 0.0
                && x.value() == "0"
        },
    )
    .await;
    assert_eq!(x.value(), "0", "the clean axis follows Redo");
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_model_axes_queue_preserve_typing_and_undo_in_order() {
    let runtime = opened().await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(host);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("model offset inputs mount", || {
        by_role_name(&root, "spinbutton", "Offset X mm").is_some()
            && by_role_name(&root, "spinbutton", "Offset Y mm").is_some()
    })
    .await;
    let x = input(&root, "X");
    let y = input(&root, "Y");
    let (entered, release) = support::gate_next_core_reply(&runtime);
    commit(&x, "4").await;
    support::drive_pending(&runtime);
    wait_until_future("first model axis edit enters Core", entered)
        .await
        .unwrap();
    commit(&y, "7").await;
    type_value(&y, "9");
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "queued model axes land",
        || {
            let offset = initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset;
            offset.x == 4.0 && offset.y == 7.0 && x.value() == "4" && y.value() == "9"
        },
    )
    .await;
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        initial_model(&accepted.document.definitions[0])
            .unwrap()
            .offset,
        Vec3 {
            x: 4.0,
            y: 7.0,
            z: 0.0
        }
    );
    assert_eq!(x.value(), "4");
    assert_eq!(y.value(), "9", "new typing survives an earlier landing");
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "first model axis Undo settles",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                == Vec3 {
                    x: 4.0,
                    y: 0.0,
                    z: 0.0,
                }
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset,
        Vec3 {
            x: 4.0,
            y: 0.0,
            z: 0.0
        }
    );
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "second model axis Undo settles",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                == Vec3::default()
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset,
        Vec3::default()
    );
    let (entered, release) = support::gate_next_core_reply(&runtime);
    commit(&x, "7").await;
    support::drive_pending(&runtime);
    wait_until_future("next model axis edit enters Core", entered)
        .await
        .unwrap();
    x.focus().unwrap();
    type_value(&x, "8");
    // The shared field is its own component, so the typed text must render before Escape reverts it.
    gloo_timers::future::TimeoutFuture::new(10).await;
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
    )
    .unwrap();
    x.blur().unwrap();
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "Escape cancels the newer model axis draft",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 7.0
                && x.value() == "7"
        },
    )
    .await;
    assert_eq!(
        initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
            .unwrap()
            .offset
            .x,
        7.0,
        "Escape cancels typing without reversing the preceding committed edit"
    );
    runtime.submit(boardstudio_application::Event::Undo {
        operation_id: runtime.operation(),
    });
    settle_until(
        &runtime,
        support::run_pending,
        "model axis Undo restores the prior X",
        || {
            initial_model(&runtime.model().accepted.unwrap().document.definitions[0])
                .unwrap()
                .offset
                .x
                == 0.0
                && x.value() == "0"
        },
    )
    .await;
    support::fail_next_core_reply(&runtime, "controlled model failure");
    commit(&x, "12").await;
    settle_until(
        &runtime,
        support::run_pending,
        "failed model edit restores its accepted value",
        || x.value() == "0",
    )
    .await;
    assert_eq!(
        x.value(),
        "0",
        "failed committed text returns to accepted value"
    );
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn model_edit_retires_if_definition_deleted_before_execution() {
    let runtime = opened().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let edits = DraftEdits::<_, ()>::default();
    let deletion = support::observe_next(&runtime);
    edits.begin(
        &runtime,
        0_u64,
        "delete-definition-test",
        None,
        EditResolver::new("delete-definition-test", |accepted: &AcceptedSnapshot| {
            let mut document = accepted.document.as_ref().clone();
            document.definitions.clear();
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
                vec!["component".into()],
            )
        }),
    );
    support::drive_pending(&runtime);
    wait_until_future("definition deletion enters Core", entered)
        .await
        .unwrap();
    let model_edit = support::observe_next(&runtime);
    edits.begin(
        &runtime,
        1_u64,
        "model-test",
        Some("component model".into()),
        offset_edit(Axis::X, 4.0),
    );
    release.send(()).unwrap();
    wait_until_step("deletion and model edit settle", async || {
        support::run_pending(&runtime).await;
        deletion.borrow().is_some() && model_edit.borrow().is_some()
    })
    .await;
    let results = edits.discard_drafts(true);
    assert!(
        matches!(&results[..], [PendingEditResult::Landed { .. }, PendingEditResult::Failed { message, .. }] if message.contains("removed")),
        "the deletion lands and the queued model edit fails against the removed definition"
    );
    assert!(
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .definitions
            .is_empty()
    );
}
