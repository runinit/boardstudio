use super::*;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_wait::wait_until_step;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
async fn library_rename_queued_after_parts_edit_preserves_both_and_undo_order() {
    let runtime = support::new_runtime();
    let mut document = ProjectDoc::empty("queued-rename", "Original");
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "switch", "name": "Original switch", "kind": "switch", "courtyard": [], "pads": []
        }))
        .unwrap(),
    );
    support::open_document(&runtime, document).await;
    let accepted = runtime.model().accepted.unwrap();
    let owner = ProjectNameOwner::from(&accepted);
    let mut replacement = accepted.document.as_ref().clone();
    replacement.definitions[0].name = "Updated switch".into();
    let (mut entered, release) = support::gate_next_core_reply(&runtime);
    support::submit_fixed_command(
        &runtime,
        EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: "parts-name".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["switch".into()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        },
    );
    wait_until_step("the parts edit reaches its gated Core reply", async || {
        support::drive_pending(&runtime);
        matches!(entered.try_recv(), Ok(Some(())))
    })
    .await;
    let dom = VirtualDom::new(VNode::empty);
    dom.in_scope(ScopeId::ROOT, || {
        ProjectNameCommitAction {
            runtime: runtime.clone(),
            owner,
            mounted: Rc::new(Cell::new(true)),
            edits: DraftEdits::default(),
            name: FieldView::new(Signal::new("Renamed".into()), Signal::new(None)),
            submitted_owner: Rc::default(),
        }
        .commit("Renamed");
    });
    support::drive_pending(&runtime);
    release.send(()).unwrap();
    wait_until_step(
        "the parts edit and the queued rename both land",
        async || {
            support::run_pending(&runtime).await;
            let result = runtime.model().accepted.unwrap();
            result.document.name == "Renamed"
                && result.document.definitions[0].name == "Updated switch"
        },
    )
    .await;
    let result = runtime.model().accepted.unwrap();
    assert_eq!(result.document.name, "Renamed");
    assert_eq!(result.document.definitions[0].name, "Updated switch");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step("the first undo reverts only the rename", async || {
        support::run_pending(&runtime).await;
        let result = runtime.model().accepted.unwrap();
        result.document.name == "Original"
            && result.document.definitions[0].name == "Updated switch"
    })
    .await;
    let result = runtime.model().accepted.unwrap();
    assert_eq!(result.document.name, "Original");
    assert_eq!(result.document.definitions[0].name, "Updated switch");
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    wait_until_step("the second undo reverts the parts edit", async || {
        support::run_pending(&runtime).await;
        runtime.model().accepted.unwrap().document.definitions[0].name == "Original switch"
    })
    .await;
    assert_eq!(
        runtime.model().accepted.unwrap().document.definitions[0].name,
        "Original switch"
    );
}
