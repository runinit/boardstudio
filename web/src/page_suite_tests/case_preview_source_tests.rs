use boardstudio_core::model::Board;
use boardstudio_web_test_runtime::{in_process_support, project_name_test_support};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn imported_capture_failure_is_visible_only_to_its_current_case_owner() {
    let runtime = project_name_test_support::new_runtime();
    let mut document = in_process_support::board_document();
    document.boards.push(Board {
        id: "another-board".into(),
        name: "Another board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    document
        .board_references
        .push(boardstudio_core::model::BoardReference {
            id: "missing-routed-source".into(),
            board_id: document.boards[0].id.clone(),
            asset_id: "missing-asset".into(),
            enabled: true,
            pose: boardstudio_core::model::Pose2 {
                at: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            elevation: 0.0,
            model_assets: Default::default(),
        });
    project_name_test_support::open_document(&runtime, document).await;
    let accepted = runtime
        .model()
        .accepted
        .expect("the Case project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted Case project has a scope");
    let error = runtime
        .prepare_native_case_preview(scope.clone(), accepted.token, accepted.document.revision)
        .await
        .expect_err("missing imported asset must fail before Core dispatch");
    assert!(error.contains("Imported board asset is unavailable"));
    assert_eq!(runtime.native_case_preview_error().as_ref(), Some(&error));
    assert!(!runtime.native_case_preview_pending());
    assert!(runtime.native_case_preview().is_none());
    project_name_test_support::navigate(&runtime, "another-board").await;
    assert!(
        runtime.native_case_preview_error().is_none(),
        "old capture failures cannot follow another owner"
    );
}
