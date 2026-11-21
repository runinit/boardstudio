wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use super::test_support::*;
use super::*;
use crate::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use crate::objects::{self, LayoutSnapSettings};
use crate::runtime::Runtime;
use crate::selection::SelectionAdapter;
use crate::setup_guide::{SetupGuidePreferences, SetupGuideStage};
use boardstudio_application::TerminalOutcome;
use boardstudio_application::{Event, Lifecycle};
use boardstudio_model::model::{Board, OutlineSettings};
use boardstudio_model::model::{
    MatrixAssembly, MatrixCell, OutlineFeature, PartDefinition, PartKind, ProjectDoc, Vec2,
};
use boardstudio_web_ui_model::tree::{ScopedTreeContext, TreeContext};
use dioxus::prelude::*;
use std::rc::Rc;
use std::{
    cell::{Cell, RefCell},
    task::{Context, Waker},
};

#[derive(Clone)]
struct HookProbe {
    runtime: Rc<Runtime>,
    canvas_interaction: CanvasInteractionArbiter,
    mounted: Rc<Cell<bool>>,
    unmounted: Rc<Cell<bool>>,
    latest: Rc<RefCell<Option<PartPlacementMount>>>,
    assembly_3d: Rc<RefCell<Option<Signal<bool>>>>,
    view_mode: Rc<RefCell<Option<EventHandler<bool>>>>,
    workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
    selected_context: Rc<RefCell<Option<Signal<Option<ScopedTreeContext>>>>>,
    selection_kind: Rc<RefCell<Option<Signal<objects::LayoutSelectionKind>>>>,
    guide: Rc<RefCell<Option<Signal<Option<SetupGuidePreferences>>>>>,
    version: Rc<Cell<u64>>,
    loader_reply: Rc<RefCell<Option<Result<PartDefinition, String>>>>,
    loader_waker: Rc<RefCell<Option<Waker>>>,
}

fn hook_host() -> Element {
    let probe = use_context::<HookProbe>();
    if probe.mounted.get() {
        rsx! { HookMounted {} }
    } else {
        rsx! { div {} }
    }
}

#[component]
fn HookMounted() -> Element {
    let probe = use_context::<HookProbe>();
    use_drop({
        let unmounted = probe.unmounted.clone();
        move || unmounted.set(true)
    });
    let workspace = use_signal(|| "Layout");
    let guide = use_signal(|| {
        Some(SetupGuidePreferences {
            project_id: "project".into(),
            open: true,
            current_stage: SetupGuideStage::Wiring,
        })
    });
    let mut version = use_signal(|| 0u64);
    let current_version = probe.version.get();
    if *version.peek() != current_version {
        version.set(current_version);
    }
    *probe.workspace.borrow_mut() = Some(workspace);
    *probe.guide.borrow_mut() = Some(guide);
    let selected_context = use_signal(|| None);
    *probe.selected_context.borrow_mut() = Some(selected_context);
    let layout_selection_kind = use_signal(objects::LayoutSelectionKind::default);
    *probe.selection_kind.borrow_mut() = Some(layout_selection_kind);
    let anchor_scope = use_signal(|| None);
    let generation = use_signal(|| 4u64);
    let adapter = SelectionAdapter::new(selected_context, anchor_scope, generation);
    let loader_reply = probe.loader_reply.clone();
    let loader_waker = probe.loader_waker.clone();
    let mount = use_controller_placement(PartPlacementHost {
        runtime: probe.runtime.clone(),
        load_definition: Rc::new(move |_, _| {
            let reply = loader_reply.clone();
            let waker = loader_waker.clone();
            Box::pin(std::future::poll_fn(move |context| {
                match reply.borrow_mut().take() {
                    Some(value) => std::task::Poll::Ready(value),
                    None => {
                        *waker.borrow_mut() = Some(context.waker().clone());
                        std::task::Poll::Pending
                    }
                }
            }))
        }),
        workspace,
        generation,
        version,
        adapter,
        layout_selection_kind,
        guide_preferences: guide,
        parts_query: use_signal(String::new),
        parts_selection: use_signal(|| None),
        snap_settings: use_signal(LayoutSnapSettings::default),
        layout_target: use_signal(|| None),
        canvas_center: Vec2::default(),
        objects_open: use_signal(|| false),
        inspect_open: use_signal(|| false),
        canvas_interaction: probe.canvas_interaction.clone(),
    });
    *probe.latest.borrow_mut() = Some(mount.clone());
    let assembly_3d = use_signal(|| false);
    let assembly_3d_read = assembly_3d;
    let mut assembly_3d_write = assembly_3d;
    *probe.assembly_3d.borrow_mut() = Some(assembly_3d);
    let matrix_placement = objects::MatrixPlacementMount {
        cancel_owner: None,
        placement: None,
        busy: false,
        error: None,
        on_place: EventHandler::new(|_| {}),
        on_move: EventHandler::new(|_| {}),
        on_commit: EventHandler::new(|_| {}),
        on_cancel: EventHandler::new(|_| {}),
    };
    *probe.view_mode.borrow_mut() = Some(crate::presentation::layout_view_mode_handler(
        || true,
        assembly_3d_read,
        move |value| assembly_3d_write.set(value),
        crate::presentation::LayoutPlacementCancellation {
            parts: mount,
            matrices: matrix_placement,
            interactions: probe.canvas_interaction.clone(),
        },
        || {},
        || {},
    ));
    rsx! { div {} }
}

async fn hook_mounted() -> (HookProbe, VirtualDom) {
    hook_mounted_with_document(fixture()).await
}

async fn hook_mounted_with_document(document: ProjectDoc) -> (HookProbe, VirtualDom) {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    let expected_board_count = document.boards.len();
    boardstudio_web_test_runtime::project_name_test_support::open_document(&runtime, document)
        .await;
    let model = runtime.model();
    assert_eq!(
        model.lifecycle,
        Lifecycle::Ready,
        "open fixture must be accepted; last error: {:?}",
        model.last_error
    );
    assert_eq!(
        model
            .accepted
            .as_ref()
            .expect("open fixture is accepted")
            .document
            .boards
            .len(),
        expected_board_count,
        "opened fixture must preserve its boards; last error: {:?}",
        model.last_error
    );
    let probe = HookProbe {
        runtime,
        canvas_interaction: CanvasInteractionArbiter::default(),
        mounted: Rc::new(Cell::new(true)),
        unmounted: Rc::new(Cell::new(false)),
        latest: Rc::default(),
        assembly_3d: Rc::default(),
        view_mode: Rc::default(),
        workspace: Rc::default(),
        selected_context: Rc::default(),
        selection_kind: Rc::default(),
        guide: Rc::default(),
        version: Rc::new(Cell::new(0)),
        loader_reply: Rc::new(RefCell::new(None)),
        loader_waker: Rc::default(),
    };
    let mut dom = VirtualDom::new(hook_host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_to_vec();
    flush_hook(&mut dom);
    (probe, dom)
}

fn accepted_document(probe: &HookProbe) -> ProjectDoc {
    (*probe
        .runtime
        .model()
        .accepted
        .expect("test project is open")
        .document)
        .clone()
}

async fn replace_accepted_document(probe: &HookProbe, document: ProjectDoc) {
    let expected_board_ids: Vec<_> = document
        .boards
        .iter()
        .map(|board| board.id.clone())
        .collect();
    let transaction_id = format!(
        "part-placement-test-replacement-{}",
        probe.runtime.model().accepted.unwrap().document.revision
    );
    let accepted = boardstudio_web_test_runtime::project_name_test_support::replace_document(
        &probe.runtime,
        &transaction_id,
        "board-main",
        document,
    )
    .await;
    let actual_board_ids: Vec<_> = accepted
        .document
        .boards
        .iter()
        .map(|board| board.id.clone())
        .collect();
    assert_eq!(
        actual_board_ids, expected_board_ids,
        "replacement must land"
    );
}

fn flush_hook(dom: &mut VirtualDom) {
    dom.mark_all_dirty();
    for _ in 0..5 {
        dom.render_immediate_to_vec();
        let mut work = std::pin::pin!(dom.wait_for_work());
        let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
    }
}

fn workspace(probe: &HookProbe) -> &'static str {
    probe.workspace.borrow().as_ref().unwrap()()
}

async fn let_hook_tasks_run() {
    gloo_timers::future::TimeoutFuture::new(20).await;
}

/// Run the runtime to settlement, let hook tasks run, and re-render.
async fn settle_hook(probe: &HookProbe, dom: &mut VirtualDom) {
    run_pending_runtime(probe).await;
    let_hook_tasks_run().await;
    flush_hook(dom);
}

/// Let spawned hook tasks run and re-render without driving the runtime.
async fn flush_tasks(dom: &mut VirtualDom) {
    let_hook_tasks_run().await;
    flush_hook(dom);
}

async fn wait_for_outcome(
    outcome: &boardstudio_web_runtime::operation_outcomes::OutcomeSlot,
) -> TerminalOutcome {
    for _ in 0..100 {
        if let Some(outcome) = outcome.borrow().clone() {
            return outcome;
        }
        gloo_timers::future::TimeoutFuture::new(20).await;
    }
    panic!("runtime did not settle the observed operation");
}

async fn run_pending_runtime(probe: &HookProbe) {
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
}

fn start_pending_runtime(probe: &HookProbe) {
    boardstudio_web_test_runtime::project_name_test_support::drive_pending(&probe.runtime);
}

fn resolve_loader(probe: &HookProbe, value: Result<PartDefinition, String>) {
    *probe.loader_reply.borrow_mut() = Some(value);
    if let Some(waker) = probe.loader_waker.borrow_mut().take() {
        waker.wake();
    }
}

fn latest(probe: &HookProbe) -> PartPlacementMount {
    probe.latest.borrow().as_ref().unwrap().clone()
}

fn clear_guide(probe: &HookProbe, dom: &mut VirtualDom) {
    clear_guide_no_flush(probe);
    flush_hook(dom);
}

fn clear_guide_no_flush(probe: &HookProbe) {
    let mut guide = *probe.guide.borrow().as_ref().unwrap();
    guide.set(None);
}

fn set_workspace(probe: &HookProbe, workspace: &'static str) {
    let mut signal = *probe.workspace.borrow().as_ref().unwrap();
    signal.set(workspace);
}

fn set_selected(probe: &HookProbe, selected: Option<ScopedTreeContext>) {
    let mut signal = *probe.selected_context.borrow().as_ref().unwrap();
    signal.set(selected);
}

/// Call a component action and finish its definition load, returning the live mount.
async fn start_component_placement(
    probe: &HookProbe,
    dom: &mut VirtualDom,
    action: ComponentPlacementAction,
    definition: PartDefinition,
) -> PartPlacementMount {
    latest(probe).on_place_component.call(action);
    resolve_loader(probe, Ok(definition));
    let_hook_tasks_run().await;
    flush_hook(dom);
    latest(probe)
}

/// Open a fixture document with a matrix, select one key in Parts, and return its context.
async fn select_matrix_key(
    probe: &HookProbe,
    dom: &mut VirtualDom,
    row: u32,
    column: u32,
    with_existing_assembly: bool,
) -> ScopedTreeContext {
    let mut document = accepted_document(probe);
    document.definitions.push(switch_definition("switch:base"));
    let mut matrix = matrix_fixture();
    if with_existing_assembly {
        document
            .definitions
            .push(passive_definition("imported:existing"));
        matrix.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column,
            enabled: true,
            definition_id: None,
            variant: None,
            offset: None,
            rotation: None,
            assemblies: vec![MatrixAssembly {
                id: "library-imported-reset-switch-1".into(),
                definition_id: "imported:existing".into(),
                offset: Vec2::default(),
                rotation: None,
                side: None,
            }],
            assemblies_local: None,
        });
    }
    document.matrices.push(matrix);
    replace_accepted_document(probe, document).await;
    let selected = ScopedTreeContext {
        scope: probe.runtime.scope().unwrap(),
        context: TreeContext::Key {
            matrix_id: "matrix-main".into(),
            row,
            column,
        },
    };
    set_selected(probe, Some(selected.clone()));
    set_workspace(probe, "Parts");
    flush_hook(dom);
    selected
}

async fn start_hook_placement(probe: &HookProbe, dom: &mut VirtualDom) -> PartPlacementMount {
    resolve_loader(probe, Ok(controller_definition("catalog:controller")));
    latest(probe).on_choose_controller.call(());
    flush_hook(dom);
    latest(probe)
        .on_place_controller
        .call("catalog:controller".into());
    let_hook_tasks_run().await;
    flush_hook(dom);
    let mount = latest(probe);
    assert!(mount.projection.is_some());
    mount
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn general_component_placement_starts_without_the_setup_guide_and_uses_kind_reference() {
    let (probe, mut dom) = hook_mounted().await;
    clear_guide(&probe, &mut dom);

    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::AddObject {
            definition_id: "catalog:reset-switch".into(),
            kind: PartKind::Passive,
        });
    assert!(
        probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
    resolve_loader(&probe, Ok(passive_definition("catalog:reset-switch")));
    flush_tasks(&mut dom).await;

    let mount = latest(&probe);
    let placement = mount
        .projection
        .expect("generic component enters canvas placement");
    assert_eq!(placement.pending.part.reference, "J1");
    assert_eq!(placement.pending.definition.kind, PartKind::Passive);
    assert_eq!(workspace(&probe), "Layout");
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn accepted_general_placement_selects_the_component_and_part_tool_context() {
    let (probe, mut dom) = hook_mounted().await;
    clear_guide(&probe, &mut dom);
    let active = start_component_placement(
        &probe,
        &mut dom,
        ComponentPlacementAction::AddObject {
            definition_id: "catalog:reset-switch".into(),
            kind: PartKind::Passive,
        },
        passive_definition("catalog:reset-switch"),
    )
    .await;
    let outcome =
        boardstudio_web_test_runtime::project_name_test_support::observe_next(&probe.runtime);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    settle_hook(&probe, &mut dom).await;

    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    let landed = probe.runtime.model().accepted.unwrap();
    let part_id = landed
        .document
        .parts
        .last()
        .expect("placement adds a part")
        .id
        .clone();

    settle_hook(&probe, &mut dom).await;

    assert_eq!(workspace(&probe), "Layout");
    assert_eq!(
        probe.selection_kind.borrow().as_ref().unwrap()(),
        objects::LayoutSelectionKind::Part
    );
    let selected = probe.selected_context.borrow().as_ref().unwrap()();
    assert!(matches!(
        selected,
        Some(ScopedTreeContext {
            context: TreeContext::Component { part_id: Some(id), .. },
            ..
        }) if id == part_id
    ));
    assert_eq!(probe.runtime.model().selected_part_ids, vec![part_id]);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn paired_catalogue_default_flows_through_owner_commit_and_creates_linked_parts() {
    let (probe, mut dom) = hook_mounted_with_document(mirrored_fixture()).await;
    clear_guide(&probe, &mut dom);

    let active = start_component_placement(
        &probe,
        &mut dom,
        ComponentPlacementAction::AddObject {
            definition_id: "catalog:paired-reset".into(),
            kind: PartKind::Passive,
        },
        passive_definition("catalog:paired-reset"),
    )
    .await;
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    settle_hook(&probe, &mut dom).await;

    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    let accepted = probe.runtime.model().accepted.unwrap();
    let source_part = accepted
        .document
        .parts
        .iter()
        .find(|part| part.definition_id == "catalog:paired-reset")
        .expect("the catalogue request creates the source part");
    let association = accepted.document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .iter()
        .find(|association| {
            association.kind == boardstudio_model::model::MirroredItemKind::Part
                && association.source_id == source_part.id
        })
        .expect("the ordinary Add Object action creates one correspondence");
    let mirror_part = accepted
        .document
        .parts
        .iter()
        .find(|part| part.id == association.mirror_id)
        .expect("the linked mirror part exists");
    assert_eq!(mirror_part.definition_id, source_part.definition_id);
    let mirror_map_x = accepted.document.hardware.as_ref().unwrap().pairs[0]
        .mapping
        .at
        .x;
    assert!((source_part.pose.at.x + mirror_part.pose.at.x - mirror_map_x).abs() < 1e-6);
    assert!((source_part.pose.at.y - mirror_part.pose.at.y).abs() < 1e-6);
    assert!(
        accepted.document.boards[0]
            .part_ids
            .contains(&source_part.id)
    );
    assert!(
        accepted.document.boards[1]
            .part_ids
            .contains(&mirror_part.id)
    );
    assert_eq!(accepted.document.parts.len(), 3);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn mirror_only_action_flows_through_owner_commit_without_changing_source() {
    let (probe, mut dom) = hook_mounted_with_document(mirrored_fixture()).await;
    clear_guide_no_flush(&probe);
    probe.runtime.submit(Event::Navigate {
        operation_id: probe.runtime.operation(),
        board_id: "board-mirror".into(),
        instance_id: None,
    });
    run_pending_runtime(&probe).await;
    flush_hook(&mut dom);
    assert_eq!(probe.runtime.scope().unwrap().board_id, "board-mirror");

    let active = start_component_placement(
        &probe,
        &mut dom,
        ComponentPlacementAction::AddMirrorOnlyObject {
            definition_id: "catalog:mirror-reset".into(),
            kind: PartKind::Passive,
        },
        passive_definition("catalog:mirror-reset"),
    )
    .await;
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    settle_hook(&probe, &mut dom).await;

    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(accepted.document.parts.len(), 2);
    assert_eq!(accepted.document.parts[0].id, "part-previous");
    assert_eq!(
        accepted.document.parts[1].definition_id,
        "catalog:mirror-reset"
    );
    assert_eq!(accepted.document.boards[0].part_ids, vec!["part-previous"]);
    assert_eq!(
        accepted.document.boards[1].part_ids,
        vec![accepted.document.parts[1].id.clone()]
    );
    let halves = accepted.document.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap();
    assert_eq!(halves.correspondences.len(), 1);
    assert_eq!(halves.correspondences[0].source_id, "part-previous");
    assert_eq!(halves.correspondences[0].mirror_id, "part-previous-mirror");
    assert_eq!(
        halves.exceptions.len(),
        1,
        "the pre-existing Omit stays stored"
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn parts_inspector_action_applies_selected_key_through_production_edit_handler() {
    let (probe, mut dom) = hook_mounted().await;
    let selected = select_matrix_key(&probe, &mut dom, 1, 2, true).await;
    assert!(crate::selection::context_is_current(
        &probe.runtime.model(),
        &selected.scope,
        &selected.context,
    ));
    let selected_context = *probe.selected_context.borrow().as_ref().unwrap();
    let outcome = observe_next_operation(&probe);

    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::PartsInspector {
            definition_id: "imported:reset-switch".into(),
            kind: PartKind::Passive,
        });
    resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
    settle_hook(&probe, &mut dom).await;
    settle_hook(&probe, &mut dom).await;

    assert_eq!(
        *outcome.borrow(),
        Some(TerminalOutcome::Completed),
        "selected-key placement should submit and settle a Runtime edit"
    );
    let accepted = probe.runtime.model().accepted.unwrap();
    let matrix = accepted.document.matrices.first().expect("matrix remains");
    assert_eq!(
        matrix.cells[0].assemblies[0].definition_id,
        "imported:existing"
    );
    assert_eq!(
        matrix.cells[0].assemblies[1].id,
        "library-imported-reset-switch-2"
    );
    assert_eq!(
        matrix.cells[0].assemblies[1].definition_id,
        "imported:reset-switch"
    );
    assert!(
        accepted
            .document
            .definitions
            .iter()
            .any(|definition| definition.id == "imported:reset-switch")
    );
    assert_eq!(workspace(&probe), "Parts");
    assert_eq!(selected_context(), Some(selected));
    assert!(
        !probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
    flush_tasks(&mut dom).await;
    let mount = latest(&probe);
    assert!(!mount.busy);
    assert!(mount.error.is_none());
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn delayed_selected_key_load_error_is_ignored_after_accepted_snapshot_refresh() {
    let (probe, mut dom) = hook_mounted().await;
    select_matrix_key(&probe, &mut dom, 0, 0, false).await;

    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::PartsInspector {
            definition_id: "imported:delayed-error".into(),
            kind: PartKind::Passive,
        });
    let_hook_tasks_run().await;
    assert!(probe.loader_waker.borrow().is_some());

    let mut refreshed = accepted_document(&probe);
    refreshed.name.push_str(" refreshed");
    replace_accepted_document(&probe, refreshed).await;

    resolve_loader(&probe, Err("stale catalogue failure".into()));
    flush_tasks(&mut dom).await;
    assert_eq!(latest(&probe).error, None);
    assert!(probe.runtime.model().selected_part_ids.is_empty());
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn retained_component_action_cannot_retarget_a_new_owner_or_wrong_workspace() {
    let (probe, mut dom) = hook_mounted().await;
    let retained = latest(&probe).on_place_component;

    retained.call(ComponentPlacementAction::PartsInspector {
        definition_id: "imported:wrong-workspace".into(),
        kind: PartKind::Passive,
    });
    flush_hook(&mut dom);
    let_hook_tasks_run().await;
    assert!(probe.loader_waker.borrow().is_none());

    let mut refreshed = accepted_document(&probe);
    refreshed.name.push_str(" refreshed");
    replace_accepted_document(&probe, refreshed).await;
    retained.call(ComponentPlacementAction::AddMirrorOnlyObject {
        definition_id: "imported:stale-owner".into(),
        kind: PartKind::Passive,
    });
    flush_hook(&mut dom);
    let_hook_tasks_run().await;
    assert!(probe.loader_waker.borrow().is_none());
    assert!(!latest(&probe).busy);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn parts_inspector_action_does_not_apply_to_a_key_selected_after_loading_started() {
    let (probe, mut dom) = hook_mounted().await;
    select_matrix_key(&probe, &mut dom, 1, 2, false).await;

    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::PartsInspector {
            definition_id: "imported:reset-switch".into(),
            kind: PartKind::Passive,
        });
    set_selected(&probe, None);
    flush_hook(&mut dom);
    resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
    flush_tasks(&mut dom).await;

    assert!(
        probe.runtime.model().accepted.unwrap().document.matrices[0]
            .cells
            .is_empty()
    );
    assert_eq!(workspace(&probe), "Parts");
    assert!(
        !probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn selected_key_edit_rejection_is_reported_to_the_current_parts_owner() {
    let (probe, mut dom) = hook_mounted().await;
    select_matrix_key(&probe, &mut dom, 1, 2, false).await;

    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::PartsInspector {
            definition_id: "imported:reset-switch".into(),
            kind: PartKind::Passive,
        });
    let outcome = observe_next_operation(&probe);
    boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
        &probe.runtime,
        "disk full",
    );
    resolve_loader(&probe, Ok(passive_definition("imported:reset-switch")));
    settle_hook(&probe, &mut dom).await;
    settle_hook(&probe, &mut dom).await;

    let mount = latest(&probe);
    assert!(!mount.busy);
    assert_eq!(
        *outcome.borrow(),
        Some(TerminalOutcome::PersistenceFailed("disk full".into()))
    );
    assert!(
        mount
            .error
            .as_deref()
            .is_some_and(|message| message.contains("disk full")),
        "the standard failure wording carries the reason: {:?}",
        mount.error
    );
    assert_eq!(workspace(&probe), "Parts");
}

fn observe_next_operation(
    probe: &HookProbe,
) -> boardstudio_web_runtime::operation_outcomes::OutcomeSlot {
    boardstudio_web_test_runtime::project_name_test_support::observe_next(&probe.runtime)
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_owns_canvas_during_definition_preparation_and_escape_restores_guide() {
    let (probe, mut dom) = hook_mounted().await;
    let mount = latest(&probe);
    mount.on_choose_controller.call(());
    flush_hook(&mut dom);
    assert_eq!(workspace(&probe), "Layout");
    latest(&probe)
        .on_place_controller
        .call("catalog:controller".into());
    flush_tasks(&mut dom).await;
    let preparing = latest(&probe);
    assert!(preparing.busy);
    assert!(preparing.owns_canvas());
    assert!(
        probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
    assert!(
        !probe
            .canvas_interaction
            .try_acquire(CanvasInteractionOwner::MirroredPair)
    );
    assert_eq!(
        probe.canvas_interaction.current(),
        Some(CanvasInteractionOwner::PartPlacement)
    );
    assert!(preparing.projection.is_none());
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .is_empty()
    );

    resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
    flush_tasks(&mut dom).await;
    let active = latest(&probe);
    assert!(
        active.projection.is_some(),
        "busy={} error={:?} workspace={}",
        active.busy,
        active.error,
        workspace(&probe)
    );
    assert!(active.owns_canvas());
    active.on_cancel.call(());
    flush_hook(&mut dom);
    assert_eq!(workspace(&probe), "Layout");
    let guide = (*probe.guide.borrow().as_ref().unwrap())().unwrap();
    assert!(guide.open);
    assert_eq!(guide.current_stage, SetupGuideStage::Wiring);
    assert!(latest(&probe).projection.is_none());
    assert_eq!(probe.canvas_interaction.current(), None);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn layout_3d_transition_cancels_suspended_component_preparation_without_ghost() {
    let (probe, mut dom) = hook_mounted().await;
    latest(&probe)
        .on_place_component
        .call(ComponentPlacementAction::AddObject {
            definition_id: "catalog:reset-switch".into(),
            kind: PartKind::Passive,
        });
    flush_tasks(&mut dom).await;

    let preparing = latest(&probe);
    assert!(preparing.busy);
    assert!(preparing.projection.is_none());
    assert!(probe.loader_waker.borrow().is_some());
    assert_eq!(
        probe.canvas_interaction.current(),
        Some(CanvasInteractionOwner::PartPlacement)
    );

    probe.view_mode.borrow().as_ref().unwrap().call(true);
    flush_hook(&mut dom);
    assert!(probe.assembly_3d.borrow().as_ref().unwrap()());
    assert!(!latest(&probe).busy);
    assert_eq!(probe.canvas_interaction.current(), None);
    assert!(probe.runtime.model().selected_part_ids.is_empty());
    let revision_after_cancel = probe
        .runtime
        .model()
        .accepted
        .expect("project remains open")
        .document
        .revision;

    resolve_loader(&probe, Ok(passive_definition("catalog:reset-switch")));
    flush_tasks(&mut dom).await;
    let settled = latest(&probe);
    assert!(!settled.busy);
    assert!(settled.projection.is_none());
    assert_eq!(
        probe.runtime.model().accepted.unwrap().document.revision,
        revision_after_cancel
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn mounted_controller_start_is_rejected_while_mirrored_pair_owns_the_canvas() {
    let (probe, mut dom) = hook_mounted().await;
    probe
        .canvas_interaction
        .try_acquire(CanvasInteractionOwner::MirroredPair);
    let mut workspace = *probe.workspace.borrow().as_ref().unwrap();
    workspace.set("Parts");
    resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
    latest(&probe)
        .on_place_controller
        .call("catalog:controller".into());
    flush_tasks(&mut dom).await;
    let mount = latest(&probe);
    assert!(!mount.busy);
    assert!(mount.projection.is_none());
    assert!(mount.error.is_none());
    assert_eq!(
        probe.canvas_interaction.current(),
        Some(CanvasInteractionOwner::MirroredPair)
    );
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .unwrap()
            .document
            .parts
            .is_empty()
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_registers_before_submit_and_retains_outcome_after_unmount() {
    let (probe, mut dom) = hook_mounted().await;
    resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
    let mount = latest(&probe);
    mount.on_choose_controller.call(());
    flush_hook(&mut dom);
    latest(&probe)
        .on_place_controller
        .call("catalog:controller".into());
    flush_tasks(&mut dom).await;
    let active = latest(&probe);
    assert!(active.projection.is_some());
    assert!(
        probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
    assert!(
        !probe
            .canvas_interaction
            .try_acquire(CanvasInteractionOwner::MirroredPair)
    );
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 4.0, y: -3.0 });
    run_pending_runtime(&probe).await;
    let_hook_tasks_run().await;
    probe.latest.borrow_mut().take();
    drop(dom);
    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_returns_to_wiring_only_after_matching_ready_saved_commit() {
    let (probe, mut dom) = hook_mounted().await;
    let active = start_hook_placement(&probe, &mut dom).await;
    assert!(
        probe
            .canvas_interaction
            .is_owner(CanvasInteractionOwner::PartPlacement)
    );
    assert!(
        !probe
            .canvas_interaction
            .try_acquire(CanvasInteractionOwner::MirroredPair)
    );
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    settle_hook(&probe, &mut dom).await;
    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    settle_hook(&probe, &mut dom).await;
    assert_eq!(workspace(&probe), "Layout");
    assert!(latest(&probe).projection.is_none());
    assert_eq!(probe.canvas_interaction.current(), None);
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_routes_persistence_failure_but_close_drains_pending_save() {
    for fail_save in [true, false] {
        let (probe, mut dom) = hook_mounted().await;
        clear_guide_no_flush(&probe);
        let mut workspace_signal = *probe.workspace.borrow().as_ref().unwrap();
        workspace_signal.set("PCB");
        flush_hook(&mut dom);
        let active = start_hook_placement(&probe, &mut dom).await;
        let save_gate = (!fail_save).then(|| {
            boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(
                &probe.runtime,
            )
        });
        if fail_save {
            boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
                &probe.runtime,
                "disk full",
            );
        }
        let outcome = observe_next_operation(&probe);
        active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
        if let Some((entered, release)) = save_gate {
            start_pending_runtime(&probe);
            entered.await.expect("save reaches the persistence gate");
            probe.runtime.submit(Event::Close {
                operation_id: probe.runtime.operation(),
            });
            let_hook_tasks_run().await;
            let _ = release.send(());
        }
        if fail_save {
            run_pending_runtime(&probe).await;
        }
        flush_tasks(&mut dom).await;
        assert_eq!(
            *outcome.borrow(),
            Some(if fail_save {
                TerminalOutcome::PersistenceFailed("disk full".into())
            } else {
                TerminalOutcome::Completed
            })
        );
        let expected_workspace = if fail_save { "Parts" } else { "PCB" };
        assert_eq!(workspace(&probe), expected_workspace);
        let error = latest(&probe).error.clone();
        assert_eq!(
            error.is_some(),
            fail_save,
            "failure message presence: {error:?}"
        );
        if let Some(error) = error {
            assert!(
                error.contains("disk full"),
                "{error} should carry disk full"
            );
        }
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_never_redirects_a_stale_board_or_project_owner() {
    for stale_identity in ["board", "project"] {
        let (probe, mut dom) = hook_mounted().await;
        if stale_identity == "board" {
            let mut document = accepted_document(&probe);
            document.boards.push(Board {
                id: "board-other".into(),
                name: "Other".into(),
                outline_ids: vec!["envelope-other".into()],
                part_ids: vec![],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            document.outline.push(OutlineFeature::PartEnvelope {
                connections: vec![],
                settings: OutlineSettings::default(),
                id: "envelope-other".into(),
                part_ids: vec![],
                margin: 4.0,
                operation: boardstudio_model::model::Operation::Add,
            });
            replace_accepted_document(&probe, document).await;
            flush_hook(&mut dom);
        }
        let active = start_hook_placement(&probe, &mut dom).await;
        let (entered, release) =
            boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(
                &probe.runtime,
            );
        let outcome = observe_next_operation(&probe);
        active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
        start_pending_runtime(&probe);
        entered.await.expect("save reaches persistence gate");
        let mut open_outcome = None;
        if stale_identity == "board" {
            probe.runtime.submit(Event::Navigate {
                operation_id: probe.runtime.operation(),
                board_id: "board-other".into(),
                instance_id: None,
            });
            start_pending_runtime(&probe);
            assert_eq!(probe.runtime.model().active_board_id, "board-other");
        } else {
            let open_operation = probe.runtime.operation();
            open_outcome = Some(
                boardstudio_web_test_runtime::project_name_test_support::observe(
                    &probe.runtime,
                    open_operation,
                ),
            );
            probe.runtime.submit(Event::Open {
                operation_id: open_operation,
                document: ProjectDoc::empty("replacement-project", "Replacement"),
            });
            // Event::Open queues behind the held save in this Session. Keep that Session
            // alive so its active placement and queued project replacement both settle.
        }
        let _ = release.send(());
        assert_eq!(
            wait_for_outcome(&outcome).await,
            TerminalOutcome::Completed,
            "stale {stale_identity} placement should finish saving"
        );
        if let Some(open_outcome) = open_outcome {
            assert_eq!(
                wait_for_outcome(&open_outcome).await,
                TerminalOutcome::Completed,
                "queued project open should complete after the save"
            );
        }
        flush_hook(&mut dom);
        assert_eq!(
            workspace(&probe),
            "Layout",
            "stale {stale_identity} route redirected"
        );
        assert!(
            probe.runtime.model().selected_part_ids.is_empty(),
            "stale {stale_identity} completion selected a new owner"
        );
    }
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_does_not_read_signals_after_unmount_while_waiting_for_saved_state() {
    let (probe, mut dom) = hook_mounted().await;
    let active = start_hook_placement(&probe, &mut dom).await;
    let (entered, release) =
        boardstudio_web_test_runtime::project_name_test_support::gate_next_persist(&probe.runtime);
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    start_pending_runtime(&probe);
    entered.await.expect("save reaches persistence gate");
    probe.mounted.set(false);
    flush_hook(&mut dom);
    assert!(
        probe.unmounted.get(),
        "mounted hook component was not dropped"
    );
    assert_eq!(
        *outcome.borrow(),
        None,
        "placement should still be pending while its owner is unmounted"
    );
    drop(dom);
    let _ = release.send(());
    let_hook_tasks_run().await;
    assert_eq!(*outcome.borrow(), Some(TerminalOutcome::Completed));
    assert!(
        probe.runtime.model().selected_part_ids.is_empty(),
        "unmounted observer must not submit a selection"
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_suppresses_actionable_failure_after_board_or_project_switch() {
    let (probe, mut dom) = hook_mounted().await;
    let mut replacement = accepted_document(&probe);
    replacement.boards.push(Board {
        id: "board-other".into(),
        name: "Other".into(),
        outline_ids: vec!["envelope-other".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    replacement.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "envelope-other".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: boardstudio_model::model::Operation::Add,
    });
    replace_accepted_document(&probe, replacement).await;
    flush_hook(&mut dom);

    let active = start_hook_placement(&probe, &mut dom).await;
    let (entered, release) =
        boardstudio_web_test_runtime::project_name_test_support::gate_next_core_reply(
            &probe.runtime,
        );
    let outcome = observe_next_operation(&probe);
    active.on_commit.call(Vec2 { x: 5.0, y: -2.0 });
    start_pending_runtime(&probe);
    entered.await.expect("Core reply reaches its gate");

    // Navigate while the real Core operation is pending. Session updates the active board
    // scope immediately; the held edit can still finish, then fail persistence.
    probe.runtime.submit(Event::Navigate {
        operation_id: probe.runtime.operation(),
        board_id: "board-other".into(),
        instance_id: None,
    });
    start_pending_runtime(&probe);
    assert_eq!(probe.runtime.model().active_board_id, "board-other");
    boardstudio_web_test_runtime::project_name_test_support::fail_next_persist(
        &probe.runtime,
        "stale board",
    );
    let _ = release.send(());
    let_hook_tasks_run().await;
    assert_eq!(
        *outcome.borrow(),
        Some(TerminalOutcome::PersistenceFailed("stale board".into()))
    );
    assert_eq!(probe.runtime.model().lifecycle, Lifecycle::RecoveryRequired);
    flush_hook(&mut dom);
    assert_eq!(workspace(&probe), "Layout");
    assert_eq!(latest(&probe).error, None);
    assert_eq!(
        *outcome.borrow(),
        Some(TerminalOutcome::PersistenceFailed("stale board".into()))
    );
    assert!(
        probe.runtime.model().selected_part_ids.is_empty(),
        "stale board failure submitted a new selection"
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn production_hook_retires_stale_preparation_and_accepts_a_fresh_place_request() {
    let (probe, mut dom) = hook_mounted().await;
    latest(&probe).on_choose_controller.call(());
    flush_hook(&mut dom);
    latest(&probe)
        .on_place_controller
        .call("catalog:controller".into());
    flush_tasks(&mut dom).await;
    assert!(latest(&probe).owns_canvas());

    let mut next = accepted_document(&probe);
    next.name.push_str(" refreshed");
    replace_accepted_document(&probe, next).await;
    probe.version.set(probe.version.get() + 1);
    flush_hook(&mut dom);
    assert!(!latest(&probe).owns_canvas());

    resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
    flush_tasks(&mut dom).await;
    assert!(latest(&probe).projection.is_none());

    latest(&probe)
        .on_place_controller
        .call("catalog:controller".into());
    let_hook_tasks_run().await;
    resolve_loader(&probe, Ok(controller_definition("catalog:controller")));
    flush_tasks(&mut dom).await;
    assert!(latest(&probe).projection.is_some());
}
