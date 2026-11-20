use super::*;
use boardstudio_web_runtime::pending_edits::PendingEdits;
use boardstudio_web_test_runtime::project_name_test_support as support;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use wasm_bindgen_test::*;

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("assembly-resolution", "Assembly resolution");
    document.definitions.push(serde_json::from_value(serde_json::json!({"id":"part", "name":"Part", "kind":"custom", "courtyard":[], "pads":[]})).unwrap());
    document.boards.push(serde_json::from_value(serde_json::json!({"id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":1.6, "traces":[], "vias":[]})).unwrap());
    document.matrices.push(serde_json::from_value(serde_json::json!({"id":"matrix", "name":"Keys", "rows":1, "columns":1, "pitch":{"x":19,"y":19}, "origin":{"x":0,"y":0}, "definitionId":"part", "partIds":[], "boardId":"board", "cells":[]})).unwrap());
    document
}
fn assembly() -> AssemblyDefinition {
    serde_json::from_value(serde_json::json!({"id":"assembly", "name":"Assembly", "members":[{"id":"primary", "definitionId":"part", "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front", "models":[]}]})).unwrap()
}
async fn release(
    runtime: &Rc<crate::runtime::Runtime>,
    release: futures_channel::oneshot::Sender<()>,
) {
    release.send(()).unwrap();
    gloo_timers::future::TimeoutFuture::new(30).await;
    support::run_pending(runtime).await;
}
fn mounted_host() -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
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
    let workspace = use_signal(|| "Parts");
    use_context_provider(|| crate::presentation::WorkspaceState(workspace));
    let selection = use_signal(|| None);
    use_context_provider(|| super::super::PartsAssemblySelection(selection));
    let orientation = use_signal(super::super::assembly_presets::SwitchOrientation::default);
    use_context_provider(|| super::super::PartsAssemblyOrientation(orientation));
    let generation = use_signal(|| 1_u64);
    use_context_provider(|| super::super::PartsSelectionGeneration(generation));
    let activation = use_signal(|| 0_u64);
    use_context_provider(|| super::super::PartsPreviewActivation(activation));
    let theme = use_memo(|| "light");
    use_context_provider(|| crate::presentation::ResolvedAppearance(theme));
    let selected_context = use_signal(|| None);
    let scope = use_signal(|| runtime.scope());
    let adapter = use_hook(|| {
        crate::presentation::SelectionAdapter::new(selected_context, scope, generation)
    });
    use_context_provider(|| adapter.clone());
    let accepted = runtime.model().accepted.unwrap();
    rsx! { SavedAssembliesEditor { snapshot: accepted.clone(), scope: runtime.scope(), definitions: accepted.document.definitions.clone(), preset_definitions: Vec::new(), selected_context, on_place: |_| {}, on_board_placed: |_| {} } }
}

#[wasm_bindgen_test]
async fn mounted_board_placement_selects_new_parts_only_after_landing() {
    let runtime = support::new_runtime();
    let mut document = document();
    document.assemblies.push(assembly());
    support::open_document(&runtime, document).await;
    let dom_document = web_sys::window().unwrap().document().unwrap();
    let root = dom_document.create_element("div").unwrap();
    dom_document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(mounted_host);
    dom.provide_root_context(runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    gloo_timers::future::TimeoutFuture::new(40).await;
    let button = |label: &str| {
        by_role_name(&root, "button", label)
            .unwrap_or_else(|| panic!("missing mounted assembly button {label}"))
    };
    button("Assembly").click();
    gloo_timers::future::TimeoutFuture::new(40).await;
    let (mut entered, release) = support::gate_next_persist(&runtime);
    button("Place on selected board").click();
    let mut held = false;
    for _ in 0..100 {
        support::drive_pending(&runtime);
        gloo_timers::future::TimeoutFuture::new(20).await;
        if entered.try_recv().unwrap().is_some() {
            held = true;
            break;
        }
    }
    assert!(held, "{}", root.text_content().unwrap_or_default());
    assert!(runtime.model().selected_part_ids.is_empty());
    assert!(button("Place on selected board").has_attribute("disabled"));
    release.send(()).unwrap();
    gloo_timers::future::TimeoutFuture::new(40).await;
    support::run_pending(&runtime).await;
    gloo_timers::future::TimeoutFuture::new(40).await;
    let model = runtime.model();
    assert_eq!(model.selected_part_ids.len(), 1);
    assert!(
        model
            .accepted
            .unwrap()
            .document
            .parts
            .iter()
            .any(|part| model.selected_part_ids.contains(&part.id))
    );
    runtime.unsubscribe();
    root.remove();
}

#[wasm_bindgen_test]
async fn assembly_followup_keeps_exact_save_and_placement_identity() {
    let original = document();
    let recipe = assembly();
    let (proposal, ids) = super::super::assembly_presets::document_with_assembly(
        &original,
        &recipe,
        &[],
        &[],
        "board",
        Vec2::default(),
        "placement",
    )
    .unwrap();
    let mut existing = proposal.clone();
    existing
        .parts
        .iter_mut()
        .find(|part| part.id == ids[0])
        .unwrap()
        .reference = "Existing placement".into();
    existing.assemblies.push(recipe.clone());
    let mut sibling = recipe.clone();
    sibling.id = "assembly-1".into();
    existing.assemblies.push(sibling.clone());
    let runtime = support::new_runtime();
    support::open_document(&runtime, existing).await;
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let mut value = recipe.clone();
    value.name = "Updated".into();
    let mut edits = PendingEdits::default();
    edits.begin(
        &runtime,
        0_u64,
        "save-identity-test",
        None,
        assembly_save_resolver(
            AssemblyDraft {
                base: Some(recipe),
                value,
                assets: Vec::new(),
                document_id: accepted.document.id.clone(),
                session_epoch: accepted.session_epoch,
                scope: Some(scope.clone()),
            },
            Vec::new(),
        ),
    );
    support::run_pending(&runtime).await;
    assert!(matches!(
        edits.settle(true).as_slice(),
        [PendingEditResult::Landed { .. }]
    ));
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(
        saved_assembly_for_submission(&accepted.document, "assembly", false)
            .unwrap()
            .id,
        "assembly"
    );
    assert_eq!(accepted.document.assemblies[1], sibling);
    edits.begin(
        &runtime,
        1_u64,
        "place-identity-test",
        None,
        assembly_place_resolver(scope, proposal, Vec::new(), "placement".into(), ids.clone()),
    );
    support::run_pending(&runtime).await;
    assert!(matches!(
        edits.settle(true).as_slice(),
        [PendingEditResult::Landed { .. }]
    ));
    let accepted = runtime.model().accepted.unwrap();
    let selected = placed_parts_for_submission(&accepted.document, "board", "placement");
    assert_eq!(selected, vec!["placement-1/primary".to_string()]);
    assert!(accepted.document.parts.iter().any(|part| part.id == ids[0]));
}

#[wasm_bindgen_test]
async fn matrix_recipe_keeps_queued_geometry_and_undo_order() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let (_, prepared) = super::super::assembly_presets::matrix_with_assembly(
        &accepted.document.matrices[0],
        &assembly(),
        &[],
        &accepted.document,
        "prepared",
    )
    .unwrap();
    let (entered, gate) = support::gate_next_core_reply(&runtime);
    let mut edits = PendingEdits::default();
    edits.begin(
        &runtime,
        0_u64,
        "matrix-position-test",
        None,
        EditResolver::new("matrix-position-test", |accepted: &AcceptedSnapshot| {
            let mut matrix = accepted.document.matrices[0].clone();
            matrix.origin.x = 42.0;
            replacement_commit(
                EditOperation::SetMatrix {
                    matrix,
                    definitions: None,
                },
                vec!["matrix".into()],
            )
        }),
    );
    support::drive_pending(&runtime);
    entered.await.unwrap();
    edits.begin(
        &runtime,
        1_u64,
        "matrix-recipe-test",
        None,
        assembly_matrix_resolver(
            scope,
            "matrix".into(),
            assembly(),
            prepared,
            "recipe".into(),
        ),
    );
    release(&runtime, gate).await;
    assert!(
        matches!(
            edits.settle(true).as_slice(),
            [
                PendingEditResult::Landed { .. },
                PendingEditResult::Landed { .. }
            ]
        ),
        "the queued recipe edit runs behind the held position edit"
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.matrices[0].origin.x, 42.0);
    assert!(
        accepted.document.matrices[0]
            .definition_id
            .contains("assembly-recipe")
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.matrices[0].origin.x, 42.0);
    assert_eq!(accepted.document.matrices[0].definition_id, "part");
}
#[wasm_bindgen_test]
async fn assembly_save_and_place_compose_with_queued_rename() {
    let runtime = support::new_runtime();
    support::open_document(&runtime, document()).await;
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    let draft = AssemblyDraft {
        base: None,
        value: assembly(),
        assets: Vec::new(),
        document_id: accepted.document.id.clone(),
        session_epoch: accepted.session_epoch,
        scope: Some(scope.clone()),
    };
    let (proposal, ids) = super::super::assembly_presets::document_with_assembly(
        &accepted.document,
        &draft.value,
        &[],
        &[],
        "board",
        Vec2 { x: 3.0, y: 5.0 },
        "placement",
    )
    .unwrap();
    let (entered, gate) = support::gate_next_core_reply(&runtime);
    let mut edits = PendingEdits::default();
    edits.begin(
        &runtime,
        0_u64,
        "rename-test",
        None,
        crate::parts_definition_name::definition_name_resolver("part".into(), "Renamed".into()),
    );
    support::drive_pending(&runtime);
    entered.await.unwrap();
    edits.begin(
        &runtime,
        1_u64,
        "assembly-save-test",
        None,
        assembly_save_resolver(draft, Vec::new()),
    );
    edits.begin(
        &runtime,
        2_u64,
        "assembly-place-test",
        None,
        assembly_place_resolver(scope, proposal, Vec::new(), "placement".into(), ids.clone()),
    );
    assert!(
        edits.is_pending(&2_u64),
        "the placement queues behind the held rename"
    );
    release(&runtime, gate).await;
    assert!(
        matches!(
            edits.settle(true).as_slice(),
            [
                PendingEditResult::Landed { .. },
                PendingEditResult::Landed { .. },
                PendingEditResult::Landed { .. }
            ]
        ),
        "rename, save and place all land in queue order"
    );
    let accepted = runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.definitions[0].name, "Renamed");
    assert_eq!(accepted.document.assemblies.len(), 1);
    assert!(accepted.document.boards[0].part_ids.contains(&ids[0]));
    assert_eq!(
        accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == ids[0])
            .unwrap()
            .pose
            .at,
        Vec2 { x: 3.0, y: 5.0 }
    );
    runtime.submit(Event::Undo {
        operation_id: runtime.operation(),
    });
    support::run_pending(&runtime).await;
    let accepted = runtime.model().accepted.unwrap();
    assert!(!accepted.document.parts.iter().any(|part| part.id == ids[0]));
    assert_eq!(accepted.document.assemblies.len(), 1);
    assert_eq!(accepted.document.definitions[0].name, "Renamed");
}
