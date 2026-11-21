use super::*;
use crate::case_generation_lifecycle::physical_case_fingerprint;
use wasm_bindgen_test::wasm_bindgen_test;

async fn runtime_with_scene(exact: bool) -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
    let runtime = project_name_test_support::new_runtime();
    let mut document = in_process_support::board_document();
    document.boards.push(Board {
        id: "other-board".into(),
        name: "Other board".into(),
        outline_ids: Vec::new(),
        part_ids: Vec::new(),
        net_ids: Vec::new(),
        thickness: 1.6,
        traces: Vec::new(),
        vias: Vec::new(),
    });
    project_name_test_support::open_document(&runtime, document).await;
    let accepted = runtime
        .model()
        .accepted
        .expect("the Case project is accepted");
    let scope = runtime
        .scope()
        .expect("the accepted Case project has a scope");
    let fingerprint = physical_case_fingerprint(&accepted, &scope)
        .expect("the accepted board fixture has physical inputs");
    *runtime.cad_scene.borrow_mut() = Some(Rc::new(CadScene {
        scope: scope.clone(),
        token: accepted.token,
        snapshot: accepted.clone(),
        result: CadResult {
            revision: accepted.document.revision,
            ..CadResult::default()
        },
        prepared: boardstudio_model::model::PreparedCaseAssemblyIR {
            revision: accepted.document.revision,
            bodies: Vec::new(),
        },
        physical_fingerprint: Some(fingerprint),
        mechanical: None,
        exact,
        contours: Vec::new(),
    }));
    (runtime, accepted, scope)
}

/// Land an edit through the real Session: the project name changes and, when given, so
/// does the first board's thickness (a physical Case input).
async fn edited(
    runtime: &Rc<Runtime>,
    scope: &Scope,
    board_thickness: Option<f64>,
) -> AcceptedSnapshot {
    let mut document = (*runtime
        .model()
        .accepted
        .expect("the Case project is accepted")
        .document)
        .clone();
    document.name.push_str(" updated");
    if let Some(thickness) = board_thickness {
        document.boards[0].thickness = thickness;
    }
    project_name_test_support::replace_document(
        runtime,
        "cad-scene-rebind-edit",
        &scope.board_id,
        document,
    )
    .await
}

#[wasm_bindgen_test]
async fn cad_scene_rebinds_only_exact_output_with_matching_physical_inputs() {
    let (exact_runtime, _original, scope) = runtime_with_scene(true).await;
    let current = edited(&exact_runtime, &scope, None).await;
    let rebound = exact_runtime
        .cad_scene()
        .expect("same-scope completed output remains available");
    assert_eq!(rebound.token, current.token);
    assert_eq!(
        rebound.snapshot.document.revision,
        current.document.revision
    );
    assert_eq!(rebound.result.revision, current.document.revision);
    assert_eq!(rebound.prepared.revision, current.document.revision);

    let (preview_runtime, original, scope) = runtime_with_scene(false).await;
    edited(&preview_runtime, &scope, None).await;
    let preview = preview_runtime
        .cad_scene()
        .expect("captured in-flight output remains inspectable");
    assert!(!preview.exact);
    assert_eq!(
        preview.token, original.token,
        "in-flight output keeps its captured token"
    );
    assert_eq!(
        preview.snapshot.document.revision,
        original.document.revision
    );

    let (changed_runtime, original, scope) = runtime_with_scene(true).await;
    edited(&changed_runtime, &scope, Some(2.0)).await;
    let previous = changed_runtime
        .cad_scene()
        .expect("same-scope old output remains available for stale display");
    assert_eq!(previous.token, original.token);
    assert_eq!(
        previous.snapshot.document.revision,
        original.document.revision
    );
}

#[wasm_bindgen_test]
async fn cad_scene_retires_old_scope_instead_of_rebinding_across_physical_owner() {
    let (runtime, _original, scope) = runtime_with_scene(true).await;
    edited(&runtime, &scope, None).await;
    let old_owner_scene = runtime.cad_scene.borrow().clone();
    // Navigating retires the cached scene on its own; put the old board's scene back so the
    // accessor itself has to refuse it for the new physical owner.
    project_name_test_support::navigate(&runtime, "other-board").await;
    *runtime.cad_scene.borrow_mut() = old_owner_scene;
    assert!(runtime.cad_scene().is_none());
}
