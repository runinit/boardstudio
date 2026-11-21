//! Queued layer-name and macro-field edit tests.
use super::*;

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
    let (probe, root, _mounted) = mounted().await;
    let runtime = probe.runtime.clone();
    let (mut draft, failure) = probe.layer_name.borrow().unwrap();
    draft.set("A".into());
    support::fail_next_core_reply(&runtime, "layer executor failed");
    rename_base(&probe, "A");
    // The older rename has not settled when the user types the next draft.
    draft.set("AB".into());
    settle_until(
        &runtime,
        support::run_pending,
        "the older failure is reported inline",
        || failure().is_some_and(|message| message.contains("layer executor failed")),
    )
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
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
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
    settle_until(
        &runtime,
        support::run_pending,
        "the removal is accepted",
        || layer_count(&runtime) == Some(1),
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
    let doc = macro_document();
    support::open_document(&runtime, doc).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
    let tap = || macro_input(&root, "Original tapMs");
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let input = tap();
    commit_input_value(&input, "55");
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    let mut hide = probe.hide_macros.borrow().unwrap();
    hide.set(true);
    wait_until(
        "the macro field is unmounted while its edit is held",
        || by_role_name(&root, "spinbutton", "Original tapMs").is_none(),
    )
    .await;
    assert!(
        by_role_name(&root, "spinbutton", "Original tapMs").is_none(),
        "the macro field is unmounted while its edit is held"
    );
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "the held macro edit is accepted",
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
                == 55
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
        55,
        "the queued edit kept executing"
    );
    hide.set(false);
    wait_until("the remounted field shows the accepted value", || {
        by_role_name(&root, "spinbutton", "Original tapMs")
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

fn macro_field_failure(root: &web_sys::Element, label: &str) -> Option<web_sys::Element> {
    let id = macro_input(root, label).get_attribute("aria-describedby")?;
    root.owner_document()?.get_element_by_id(&id)
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
    support::open_document(&runtime, macro_document()).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
    let input = macro_input(&root, label);
    support::fail_next_core_reply(&runtime, "macro executor failed");
    let mark = probe.renders.get();
    commit_input_value(&input, first);
    rendered_after(&probe, mark).await;
    // The older edit has not settled when the user types the next draft.
    type_value(&input, newer);
    settle_until(
        &runtime,
        support::run_pending,
        "the newer draft and the older inline failure are shown",
        || {
            macro_input(&root, label).value() == newer
                && macro_field_failure(&root, label)
                    .and_then(|alert| alert.text_content())
                    .is_some_and(|text| text.contains("macro executor failed"))
        },
    )
    .await;
    let input = macro_input(&root, label);
    assert_eq!(input.value(), newer, "a newer draft is never overwritten");
    let alert = macro_field_failure(&root, label)
        .expect("the older failure reports inline beside the newer draft");
    assert!(
        alert
            .text_content()
            .unwrap()
            .contains("macro executor failed")
    );
    assert!(
        root.query_selector_all("[role='alert']").unwrap().length() == 1,
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
async fn a_macro_field_save_failure_preserves_the_newer_draft_after_rollback() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, macro_document()).await;
    let (probe, root, _mounted) = mount_runtime(runtime.clone()).await;
    let revision = runtime_revision(&runtime).unwrap();
    let label = "Original tapMs";
    let input = macro_input(&root, label);

    support::fail_next_persist(&runtime, "macro disk failure");
    let mark = probe.renders.get();
    commit_input_value(&input, "55");
    rendered_after(&probe, mark).await;
    type_value(&input, "66");
    settle_until(
        &runtime,
        support::run_pending,
        "the macro edit's save failure is shown inline",
        || {
            macro_field_failure(&root, label)
                .and_then(|alert| alert.text_content())
                .is_some_and(|message| message.contains("macro disk failure"))
        },
    )
    .await;
    let model = runtime.model();
    assert!(matches!(
        model.durability,
        boardstudio_application::Durability::Failed { revision: failed, reason }
            if failed == revision + 1 && reason.contains("macro disk failure")
    ));
    let accepted = model.accepted.unwrap();
    assert_eq!(accepted.document.revision, revision);
    assert_eq!(
        accepted.document.keymap.as_ref().unwrap().macros[0].tap_ms,
        30
    );
    assert_eq!(macro_input(&root, label).value(), "66");
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_macro_number_restores_the_accepted_value_with_an_inline_failure() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, macro_document()).await;
    let (_probe, root, _mounted) = mount_runtime(runtime.clone()).await;
    support::fail_next_core_reply(&runtime, "macro executor failed");
    let input = macro_input(&root, "Original tapMs");
    commit_input_value(&input, "55");
    settle_until(
        &runtime,
        support::run_pending,
        "the unchanged macro number is restored with its failure",
        || {
            macro_input(&root, "Original tapMs").value() == "30"
                && macro_field_failure(&root, "Original tapMs")
                    .and_then(|alert| alert.text_content())
                    .is_some_and(|text| text.contains("macro executor failed"))
        },
    )
    .await;
    assert_eq!(macro_input(&root, "Original tapMs").value(), "30");
    let alert = macro_field_failure(&root, "Original tapMs").expect("the failure reports inline");
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
    support::open_document(&runtime, macro_document()).await;
    let (_probe, root, _mounted) = mount_runtime(runtime.clone()).await;
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
    commit_input_value(&input, "55");
    support::drive_pending(&runtime);
    wait_until_future("the held edit reaches the Core executor", entered)
        .await
        .unwrap();
    commit_input_value(&input, "66");
    support::drive_pending(&runtime);
    release.send(()).unwrap();
    settle_until(
        &runtime,
        support::run_pending,
        "the second macro edit is accepted and shown",
        || tap_ms(&runtime) == 66 && macro_input(&root, "Original tapMs").value() == "66",
    )
    .await;
    assert_eq!(tap_ms(&runtime), 66);
    assert_eq!(macro_input(&root, "Original tapMs").value(), "66");
    assert!(macro_field_failure(&root, "Original tapMs").is_none());
    for expected in [55, 30] {
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        settle_until(
            &runtime,
            support::run_pending,
            "the undo is accepted",
            || tap_ms(&runtime) == expected,
        )
        .await;
        assert_eq!(tap_ms(&runtime), expected, "each edit has an Undo step");
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn a_failed_unchanged_layer_name_restores_the_accepted_name_with_an_inline_failure() {
    let (probe, root, _mounted) = mounted().await;
    let runtime = probe.runtime.clone();
    let (mut draft, failure) = probe.layer_name.borrow().unwrap();
    draft.set("A".into());
    support::fail_next_core_reply(&runtime, "layer executor failed");
    rename_base(&probe, "A");
    settle_until(
        &runtime,
        support::run_pending,
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
