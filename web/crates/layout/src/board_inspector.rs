//! The Layout board-level Inspector and its accepted-session rename action.
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::EditOperation;
use boardstudio_web_ui_shared::draft_edits::{DraftEdits, FieldView};
use boardstudio_web_ui_shared::ui::field::FieldError;
use boardstudio_web_ui_shared::ui::field::{Field, FieldLabel};
use boardstudio_web_ui_shared::ui::input_group::{InputGroup, InputGroupInput};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use super::objects::{ScopedTreeContext, TreeContext};

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContextIdentity {
    scope: Option<Scope>,
    selected: Option<ScopedTreeContext>,
    workspace: &'static str,
    scope_generation: u64,
}

#[derive(Default)]
struct ContextGeneration {
    identity: Option<ContextIdentity>,
    value: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoardInspectorOwner {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    context: ContextIdentity,
    generation: u64,
    board_id: String,
    accepted_name: String,
}

#[derive(Clone, PartialEq)]
pub struct BoardInspectorProjection {
    owner: BoardInspectorOwner,
    pub board_name: String,
    pub outline_status: &'static str,
    pub placed_parts: usize,
    pub editable: bool,
    pending: bool,
    failure: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardRenameAction {
    owner: BoardInspectorOwner,
    name: String,
}

#[derive(Clone, PartialEq)]
pub struct BoardInspectorMount {
    pub projection: Option<BoardInspectorProjection>,
    pub on_rename: EventHandler<BoardRenameAction>,
    pending: Signal<DraftEdits<()>>,
    mounted: Signal<bool>,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoardNameDraftIdentity {
    scope: Scope,
    context: ContextIdentity,
    generation: u64,
    board_id: String,
}

impl From<&BoardInspectorOwner> for BoardNameDraftIdentity {
    fn from(owner: &BoardInspectorOwner) -> Self {
        Self {
            scope: owner.scope.clone(),
            context: owner.context.clone(),
            generation: owner.generation,
            board_id: owner.board_id.clone(),
        }
    }
}

/// Owns only the transient Inspector context generation. Board data always comes from the
/// current accepted Runtime snapshot, and edits use the existing ReplaceDocument history path.
pub fn use_board_inspector(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> BoardInspectorMount {
    let context_generation = use_hook(|| Rc::new(RefCell::new(ContextGeneration::default())));
    let pending = use_signal(DraftEdits::<()>::default);
    let mounted = use_signal(|| false);
    let initial_name = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == runtime.model().active_board_id)
                .map(|board| board.name.clone())
        })
        .unwrap_or_default();
    let mut draft = use_signal(|| initial_name);
    let mut failure = use_signal(|| None::<String>);
    let mut pending_owner = use_signal(|| None::<BoardNameDraftIdentity>);
    let selected = selected_context.read().clone();
    let current_workspace = workspace();
    let current_scope_generation = scope_generation();
    let context = context_identity(
        &runtime,
        selected.clone(),
        current_workspace,
        current_scope_generation,
    );
    let generation = {
        let mut tracker = context_generation.borrow_mut();
        if tracker.identity.as_ref() != Some(&context) {
            tracker.value = tracker
                .value
                .checked_add(1)
                .expect("board Inspector context generation exhausted");
            tracker.identity = Some(context.clone());
        }
        tracker.value
    };
    let mut projection = project_current(&runtime, selected.as_ref(), context.clone(), generation);
    let version = use_context::<Signal<u64>>()();
    let owner = projection
        .as_ref()
        .map(|projection| projection.owner.clone());
    let accepted_name_for_settlement = projection
        .as_ref()
        .map(|projection| projection.board_name.clone())
        .unwrap_or_default();
    let owner_identity = owner.as_ref().map(BoardNameDraftIdentity::from);
    let mut field_owner = use_signal(|| owner_identity.clone());
    if field_owner.peek().as_ref() != owner_identity.as_ref()
        && let Some(accepted) = projection
            .as_ref()
            .map(|projection| projection.board_name.clone())
    {
        field_owner.set(owner_identity.clone());
        draft.set(accepted);
        failure.set(None);
    }
    let settle_pending = pending.peek().clone();
    let mut pending_owner_for_effect = pending_owner;
    use_effect(use_reactive(
        (
            &version,
            &owner_identity,
            &accepted_name_for_settlement,
            &mounted,
            &pending_owner,
        ),
        move |(_, owner, accepted_name, mounted, captured_owner)| {
            settle_pending.settle(
                mounted() && owner.as_ref() == captured_owner.read().as_ref(),
                |_| accepted_name.clone(),
            );
            if !settle_pending.is_pending(&()) && captured_owner.peek().is_some() {
                pending_owner_for_effect.set(None);
            }
        },
    ));
    if let Some(projection) = projection.as_mut() {
        projection.pending = pending.read().is_pending(&());
    }

    let on_rename = use_callback({
        let runtime = runtime.clone();
        let context_generation = context_generation.clone();
        move |action: BoardRenameAction| {
            let owner = &action.owner;
            let current_context = context_identity(
                &runtime,
                selected_context.read().clone(),
                workspace(),
                scope_generation(),
            );
            if workspace() != "Layout"
                || context_generation.borrow().value != owner.generation
                || current_context != owner.context
                || runtime.scope().as_ref() != Some(&owner.scope)
            {
                return;
            }
            let model = runtime.model();
            let Some(snapshot) = model.accepted.as_ref() else {
                return;
            };
            if !matches!(
                model.lifecycle,
                Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
            ) || snapshot.session_epoch != owner.scope.session_epoch
                || snapshot.document.id != owner.scope.document_id
                || model.active_board_id != owner.board_id
                || !board_context_is_current(
                    &runtime,
                    &model,
                    &owner.scope,
                    selected_context.read().as_ref(),
                )
            {
                return;
            }
            let submitted_name = action.name.clone();
            let name = action.name.trim();
            if name.is_empty() {
                return;
            }
            let board_id = owner.board_id.clone();
            let name = name.to_owned();
            let resolver_name = name.clone();
            pending_owner.set(Some(BoardNameDraftIdentity::from(owner)));
            let resolver = EditResolver::new("board-name", move |accepted: &AcceptedSnapshot| {
                let mut document = accepted.document.as_ref().clone();
                let Some(board) = document
                    .boards
                    .iter_mut()
                    .find(|board| board.id == board_id)
                else {
                    return Resolution::Retire("The board no longer exists.".into());
                };
                if board.name == resolver_name {
                    return Resolution::Unchanged;
                }
                board.name = resolver_name.clone();
                Resolution::submit(
                    vec![board_id.clone()],
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                )
            });
            pending.peek().submit(
                &runtime,
                (),
                "board-name",
                Some("board name".into()),
                resolver,
                FieldView::new(draft, failure).submitted(submitted_name),
            );
        }
    });

    BoardInspectorMount {
        projection,
        on_rename,
        pending,
        mounted,
        draft,
        failure,
    }
}

fn context_identity(
    runtime: &Runtime,
    selected: Option<ScopedTreeContext>,
    workspace: &'static str,
    scope_generation: u64,
) -> ContextIdentity {
    ContextIdentity {
        scope: runtime.scope(),
        selected,
        workspace,
        scope_generation,
    }
}

fn project_current(
    runtime: &Runtime,
    selected: Option<&ScopedTreeContext>,
    context: ContextIdentity,
    generation: u64,
) -> Option<BoardInspectorProjection> {
    if context.workspace != "Layout" {
        return None;
    }
    let model = runtime.model();
    let snapshot = model.accepted.as_ref()?;
    let scope = context.scope.clone()?;
    if !board_context_is_current(runtime, &model, &scope, selected)
        || scope.board_id != model.active_board_id
        || scope.document_id != snapshot.document.id
        || scope.session_epoch != snapshot.session_epoch
    {
        return None;
    }
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    let placed_parts = board
        .part_ids
        .iter()
        .filter(|id| snapshot.document.parts.iter().any(|part| part.id == **id))
        .count();
    let outline_status = if snapshot
        .scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board.id)
        .is_some_and(|readiness| readiness.outline)
    {
        "Resolved"
    } else {
        "Not defined"
    };
    Some(BoardInspectorProjection {
        owner: BoardInspectorOwner {
            scope,
            token: snapshot.token,
            revision: snapshot.document.revision,
            context,
            generation,
            board_id: board.id.clone(),
            accepted_name: board.name.clone(),
        },
        board_name: board.name.clone(),
        outline_status,
        placed_parts,
        editable: matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        ),
        pending: false,
        failure: None,
    })
}

fn board_context_is_current(
    runtime: &Runtime,
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    selected: Option<&ScopedTreeContext>,
) -> bool {
    if runtime.scope().as_ref() != Some(scope) {
        return false;
    }
    match selected {
        None => model.selected_part_ids.is_empty(),
        Some(selected) => {
            selected.scope == *scope
                && matches!(
                    &selected.context,
                    TreeContext::Board { board_id } if board_id == &scope.board_id
                )
                && model.selected_part_ids.is_empty()
                && super::selection::context_is_current(model, scope, &selected.context)
        }
    }
}

#[component]
pub fn BoardInspector(mount: BoardInspectorMount) -> Element {
    let pending = mount.pending;
    let mut draft = mount.draft;
    let failure = mount.failure;
    let mut mounted = mount.mounted;
    let Some(projection) = mount.projection else {
        return rsx! {};
    };
    let on_rename = mount.on_rename;
    let did_mount = use_hook(|| Rc::new(Cell::new(false)));
    let mut mounted_for_effect = mounted;
    let did_mount_for_effect = did_mount.clone();
    use_effect(move || {
        if !did_mount_for_effect.replace(true) {
            mounted_for_effect.set(true);
        }
    });
    use_drop(move || mounted.set(false));
    let owner = projection.owner.clone();
    let accepted_name = projection.owner.accepted_name.clone();
    let name = draft();
    let blur_owner = owner.clone();
    let key_owner = owner.clone();
    let blur_handler = on_rename;
    let key_handler = on_rename;
    let blur_pending = pending;
    let key_pending = pending;
    let submit_blur = move |_| {
        commit_draft(&mut draft, blur_pending, &blur_owner, blur_handler);
    };
    let submit_key = move |event: KeyboardEvent| match event.key().to_string().as_str() {
        "Enter" => {
            event.prevent_default();
            commit_draft(&mut draft, key_pending, &key_owner, key_handler);
        }
        "Escape" => {
            event.prevent_default();
            event.stop_propagation();
            draft.set(accepted_name.clone());
        }
        _ => {}
    };
    rsx! {
            section { class: "grid content-start gap-2.5 min-w-0 px-4 pt-3 pb-5 text-foreground", aria_label: "Board Inspector",
                p { class: "m-0 mb-[5px] text-muted-foreground text-[11px] leading-[1.6]",
                    "Select a key, component, or matrix to edit it."
                }
                {
                    let control_input_id = format!(
                        "layout-input-{}-1",
                        dioxus::core::current_scope_id().0,
                    );
                    rsx! {
                        Field { class: "grid gap-1.5 m-0 text-muted-foreground text-[11px]",
                            FieldLabel { html_for: control_input_id.clone(), "Board name" }
                            InputGroup { class: "max-[760px]:h-control-touch",
                                InputGroupInput { class: "max-[760px]:h-control-touch", aria_label: "Board name",
    value: "{name}",
    disabled: !projection.editable,
    oninput: move |event: FormEvent| draft.set(event.value()),
    id: control_input_id.clone(),
    attributes: vec![dioxus::events::onblur(submit_blur),dioxus::events::onkeydown(submit_key)], }
                            }
                        }
                    }
                }
                if let Some(message) = failure() {
                    FieldError { "{message}" }
                }
                div { class: "flex justify-between py-2 border-b border-border text-muted-foreground text-[10px]",
                    span { "Outline" }
                    strong { class: "text-foreground font-mono text-[10px] font-normal", "{projection.outline_status}" }
                }
                div { class: "flex justify-between py-2 border-b border-border text-muted-foreground text-[10px]",
                    span { "Placed parts" }
                    strong { class: "text-foreground font-mono text-[10px] font-normal", "{projection.placed_parts}" }
                }
            }
        }
}

fn commit_draft(
    draft: &mut Signal<String>,
    pending: Signal<DraftEdits<()>>,
    owner: &BoardInspectorOwner,
    on_rename: EventHandler<BoardRenameAction>,
) {
    let submitted_name = draft.peek().clone();
    let resolver_name = submitted_name.trim();
    if resolver_name.is_empty() {
        draft.set(owner.accepted_name.clone());
        return;
    }
    if resolver_name == owner.accepted_name && !pending.peek().is_pending(&()) {
        draft.set(owner.accepted_name.clone());
        return;
    }
    on_rename.call(BoardRenameAction {
        owner: owner.clone(),
        name: submitted_name,
    });
}

#[cfg(test)]
mod queued_board_name_tests {
    use super::*;
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let _ = version();
        let selected = use_signal(|| None);
        let workspace = use_signal(|| "Layout");
        let generation = use_signal(|| 0);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let mount = use_board_inspector(runtime, selected, workspace, generation);
        rsx! {
            if mount.projection.is_some() {
                BoardInspector { mount }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn board_inspector_measurements_keep_layout_without_legacy_classes() {
        let runtime = support::new_runtime();
        let mut project = boardstudio_model::model::ProjectDoc::empty("board-measure", "Project");
        project.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        support::open_document(&runtime, project).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(include_str!("../../../assets/tailwind.css")));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        let mut mounted = MountedFixture::new(root.clone(), dom);
        mounted.own_node(stylesheet.clone());
        boardstudio_web_ui_shared::test_wait::wait_until("board inspector mounts", || {
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name")
                .is_some()
        })
        .await;
        let labels = root.query_selector_all("span").unwrap();
        let label = (0..labels.length())
            .filter_map(|index| labels.item(index))
            .find(|node| node.text_content().as_deref() == Some("Placed parts"))
            .unwrap()
            .dyn_into::<web_sys::Element>()
            .unwrap();
        let row = label.parent_element().unwrap();
        let classes = row.get_attribute("class").unwrap_or_default();
        row.set_attribute(
            "class",
            &classes
                .split_whitespace()
                .filter(|class| !class.starts_with("m1-"))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .unwrap();
        let style = web_sys::window()
            .unwrap()
            .get_computed_style(&row)
            .unwrap()
            .unwrap();
        assert_eq!(style.get_property_value("display").unwrap(), "flex");
        assert_eq!(
            style.get_property_value("justify-content").unwrap(),
            "space-between"
        );
        assert_eq!(style.get_property_value("padding-top").unwrap(), "8px");
        let input =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name")
                .unwrap();
        let field = input.closest("[data-slot=field]").unwrap().unwrap();
        let classes = field.get_attribute("class").unwrap_or_default();
        field
            .set_attribute(
                "class",
                &classes
                    .split_whitespace()
                    .filter(|class| !class.starts_with("m1-"))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .unwrap();
        let field_style = web_sys::window()
            .unwrap()
            .get_computed_style(&field)
            .unwrap()
            .unwrap();
        assert_eq!(field_style.get_property_value("display").unwrap(), "grid");
        assert_eq!(field_style.get_property_value("gap").unwrap(), "6px");
        runtime.unsubscribe();
        stylesheet.remove();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_board_rename_queues_with_layout_edits_and_undo_keeps_them() {
        let runtime = support::new_runtime();
        let mut document = boardstudio_model::model::ProjectDoc::empty("board-rename", "Project");
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        gloo_timers::future::TimeoutFuture::new(5).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let change_layout = |thickness| {
            boardstudio_web_runtime::edit_ticket::EditTicket::begin(
                &runtime,
                "layout-edit",
                None,
                boardstudio_application::EditResolver::new(
                    "layout-edit",
                    move |accepted: &boardstudio_application::AcceptedSnapshot| {
                        let mut document = accepted.document.as_ref().clone();
                        document.boards[0].thickness = thickness;
                        boardstudio_application::Resolution::submit(
                            vec!["board".into()],
                            EditOperation::ReplaceDocument {
                                document: Box::new(document),
                            },
                        )
                    },
                ),
            )
        };
        let _first = change_layout(2.0);
        support::drive_pending(&runtime);
        let mut entered = entered;
        let mut reached_core = false;
        for _ in 0..100 {
            match entered.try_recv() {
                Ok(Some(())) => {
                    reached_core = true;
                    break;
                }
                Ok(None) => gloo_timers::future::TimeoutFuture::new(10).await,
                Err(_) => panic!("the Core gate was dropped before the edit arrived"),
            }
        }
        assert!(reached_core, "the first layout edit did not reach Core");
        let input =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name")
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
        input.set_value("Renamed");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        let enter = web_sys::KeyboardEventInit::new();
        enter.set_key("Enter");
        enter.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                    .unwrap(),
            )
            .unwrap();
        let _last = change_layout(2.4);
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].name,
            "Renamed"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].thickness,
            2.4
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].name,
            "Renamed"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].thickness,
            2.0
        );
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].name,
            "Board"
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_spaced_board_name_failure_restores_accepted_name() {
        let runtime = support::new_runtime();
        let mut document = boardstudio_model::model::ProjectDoc::empty("board-spaces", "Project");
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        gloo_timers::future::TimeoutFuture::new(5).await;
        support::fail_next_persist(&runtime, "injected durable write failure");
        let input =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name")
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
        input.set_value(" New ");
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        let enter = web_sys::KeyboardEventInit::new();
        enter.set_key("Enter");
        enter.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                    .unwrap(),
            )
            .unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(input.value(), "Board");
        assert!(
            root.text_content()
                .unwrap()
                .contains("injected durable write failure")
        );
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_accepted_name_draft_queues_behind_an_earlier_rename() {
        let runtime = support::new_runtime();
        let mut document =
            boardstudio_model::model::ProjectDoc::empty("board-queued-reset", "Project");
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        gloo_timers::future::TimeoutFuture::new(5).await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input =
            boardstudio_web_ui_shared::test_role::by_role_name(&root, "textbox", "Board name")
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        let enter = web_sys::KeyboardEventInit::new();
        enter.set_key("Enter");
        enter.set_bubbles(true);
        input.set_value("Earlier");
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                    .unwrap(),
            )
            .unwrap();
        support::drive_pending(&runtime);
        entered.await.expect("the earlier rename reached Core");
        input.set_value(" Board ");
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter)
                    .unwrap(),
            )
            .unwrap();
        support::drive_pending(&runtime);
        release.send(()).expect("release the held earlier rename");
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.boards[0].name,
            "Board"
        );
        runtime.unsubscribe();
        root.remove();
    }
}
