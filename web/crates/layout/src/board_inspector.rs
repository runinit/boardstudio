//! The Layout board-level Inspector and its accepted-session rename action.
use crate::runtime::Runtime;
#[cfg(test)]
use boardstudio_application::Event;
use boardstudio_application::{
    AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};

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
    let mut tickets = use_signal(Vec::<(BoardNameDraftIdentity, EditTicket)>::new);
    let mut failure = use_signal(|| None::<(BoardNameDraftIdentity, String)>);
    let version = use_context::<Signal<u64>>()();
    let owner = projection
        .as_ref()
        .map(|projection| BoardNameDraftIdentity::from(&projection.owner));
    use_effect(use_reactive((&version, &owner), move |(_, owner)| {
        let mut remaining = Vec::new();
        for (identity, ticket) in tickets.peek().iter() {
            match ticket.settlement(owner.as_ref() == Some(identity)) {
                Settlement::Pending => remaining.push((identity.clone(), ticket.clone())),
                Settlement::Failed { message } => failure.set(Some((identity.clone(), message))),
                Settlement::Landed { .. } | Settlement::Retired => {}
            }
        }
        if remaining.len() != tickets.peek().len() {
            tickets.set(remaining);
        }
    }));
    if let Some(projection) = projection.as_mut() {
        projection.pending = tickets
            .read()
            .iter()
            .any(|(identity, _)| Some(identity) == owner.as_ref());
        projection.failure = failure
            .read()
            .as_ref()
            .filter(|(identity, _)| Some(identity) == owner.as_ref())
            .map(|(_, message)| message.clone());
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
            let name = action.name.trim();
            if name.is_empty() {
                return;
            }
            let board_id = owner.board_id.clone();
            let name = name.to_owned();
            let target_scope = owner.scope.clone();
            let resolver = EditResolver::new("board-name", move |accepted: &AcceptedSnapshot| {
                if accepted.session_epoch != target_scope.session_epoch
                    || accepted.document.id != target_scope.document_id
                {
                    return Resolution::Retire("The project is no longer open.".into());
                }
                let mut document = accepted.document.as_ref().clone();
                let Some(board) = document
                    .boards
                    .iter_mut()
                    .find(|board| board.id == board_id)
                else {
                    return Resolution::Retire("The board no longer exists.".into());
                };
                if board.name == name {
                    return Resolution::Unchanged;
                }
                board.name = name.clone();
                Resolution::Submit(EditCommand {
                    base_revision: 0,
                    transaction_id: String::new(),
                    phase: EditPhase::Commit,
                    target_ids: vec![board_id.clone()],
                    operation: EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                })
            });
            failure.set(None);
            tickets.write().push((
                BoardNameDraftIdentity::from(owner),
                EditTicket::begin(&runtime, "board-name", Some("board name".into()), resolver),
            ));
        }
    });

    BoardInspectorMount {
        projection,
        on_rename,
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
pub fn BoardInspector(
    projection: BoardInspectorProjection,
    on_rename: EventHandler<BoardRenameAction>,
) -> Element {
    let mut draft = use_signal(|| None::<NameDraft>);
    let owner = projection.owner.clone();
    let accepted_name = projection.owner.accepted_name.clone();
    let name = draft()
        .filter(|draft| {
            draft.identity == BoardNameDraftIdentity::from(&owner)
                && (projection.pending || !draft.submitted)
        })
        .map(|draft| draft.value)
        .unwrap_or_else(|| accepted_name.clone());
    let input_owner = owner.clone();
    let blur_owner = owner.clone();
    let key_owner = owner.clone();
    let input_name = accepted_name.clone();
    let blur_name = accepted_name.clone();
    let key_name = accepted_name.clone();
    let blur_handler = on_rename;
    let key_handler = on_rename;
    let submit_blur = move |_| {
        commit_draft(&mut draft, &blur_owner, &blur_name, blur_handler);
    };
    let submit_key = move |event: KeyboardEvent| match event.key().to_string().as_str() {
        "Enter" => {
            event.prevent_default();
            commit_draft(&mut draft, &key_owner, &key_name, key_handler);
        }
        "Escape" => {
            event.prevent_default();
            event.stop_propagation();
            draft.set(None);
        }
        _ => {}
    };
    rsx! {
        section { class: "m1-board-inspector", aria_label: "Board Inspector",
            p { class: "m1-board-inspector-guidance", "Select a key, component, or matrix to edit it." }
            label { class: "m1-board-inspector-name",
                "Board name"
                input {
                    aria_label: "Board name",
                    value: "{name}",
                    disabled: !projection.editable,
                    oninput: move |event| draft.set(Some(NameDraft {
                        identity: BoardNameDraftIdentity::from(&input_owner),
                        baseline: input_name.clone(),
                        value: event.value(),
                        submitted: false,
                    })),
                    onblur: submit_blur,
                    onkeydown: submit_key,
                }
            }
            if let Some(message) = projection.failure.as_ref() { p { role: "alert", "{message}" } }
            div { class: "m1-board-inspector-measure",
                span { "Outline" }
                strong { "{projection.outline_status}" }
            }
            div { class: "m1-board-inspector-measure",
                span { "Placed parts" }
                strong { "{projection.placed_parts}" }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NameDraft {
    identity: BoardNameDraftIdentity,
    baseline: String,
    value: String,
    submitted: bool,
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

fn commit_draft(
    draft: &mut Signal<Option<NameDraft>>,
    owner: &BoardInspectorOwner,
    _accepted_name: &str,
    on_rename: EventHandler<BoardRenameAction>,
) {
    let Some(mut value) = draft.read().clone() else {
        return;
    };
    if value.identity != BoardNameDraftIdentity::from(owner) || value.submitted {
        return;
    }
    let name = value.value.trim();
    if name.is_empty() {
        draft.set(None);
        return;
    }
    let name = name.to_owned();
    value.submitted = true;
    draft.set(Some(value));
    on_rename.call(BoardRenameAction {
        owner: owner.clone(),
        name,
    });
}

#[cfg(test)]
mod queued_board_name_tests {
    use super::*;
    use crate::runtime::project_name_test_support as support;
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
        rsx! { if let Some(projection) = mount.projection { BoardInspector { projection, on_rename: mount.on_rename } } }
    }

    #[wasm_bindgen_test]
    async fn mounted_board_rename_queues_with_layout_edits_and_undo_keeps_them() {
        let runtime = support::new_runtime();
        let mut document = boardstudio_core::model::ProjectDoc::empty("board-rename", "Project");
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        support::open_document(&runtime, document).await;
        let dom_document = web_sys::window().unwrap().document().unwrap();
        let root = dom_document.create_element("div").unwrap();
        dom_document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
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
                        boardstudio_application::Resolution::Submit(EditCommand {
                            base_revision: 0,
                            transaction_id: String::new(),
                            phase: EditPhase::Commit,
                            target_ids: vec!["board".into()],
                            operation: EditOperation::ReplaceDocument {
                                document: Box::new(document),
                            },
                        })
                    },
                ),
            )
        };
        let _first = change_layout(2.0);
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let input = root
            .query_selector("input")
            .unwrap()
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
}
