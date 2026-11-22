//! Authored Case body inspector: admits an edit against the live snapshot and commits it
//! synchronously through the Runtime.
use super::case_bodies::{CaseBoardSummary, CaseBodies, CaseMismatch};
use super::case_body_edit::{CaseBodyEdit, new_case_body};
use super::shared_viewer::{BodySelection, CaseBodyEditDispatch, CaseSelection};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Lifecycle, Resolution, Scope};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use dioxus::prelude::*;
use std::rc::Rc;

/// Mount beside the Case preview in the Inspector slot. The shared page passes
/// its already scope-guarded configured-board navigation callback.
#[component]
pub fn CaseBodyInspector(on_show_configured_board: EventHandler<String>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<super::InstanceSelection>();
    let _ = use_context::<Signal<u64>>()();
    let selection = use_context::<CaseSelection>();
    let mut portal = selection.body_edit_portal;
    let selected_body = selection.body;
    let error = use_signal(|| None::<String>);

    let dispatch = use_hook({
        let runtime = runtime.clone();
        move || {
            Rc::new(move |edit: CaseBodyEdit, scope: Scope, _, _| {
                let (mut selected_body, mut error) = (selected_body, error);
                let result = commit_edit(&runtime, instance_selection, &scope, edit);
                if let Ok(Some(body_id)) = &result {
                    selected_body.set(Some(BodySelection {
                        scope,
                        body_id: body_id.clone(),
                    }));
                }
                error.set(result.err());
            }) as CaseBodyEditDispatch
        }
    });
    if portal
        .dispatch
        .peek()
        .as_ref()
        .is_none_or(|current| !Rc::ptr_eq(current, &dispatch))
    {
        portal.dispatch.set(Some(dispatch.clone()));
    }
    use_drop({
        let dispatch = dispatch.clone();
        move || {
            if portal
                .dispatch
                .peek()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &dispatch))
            {
                portal.dispatch.set(None);
                portal.editable.set(false);
            }
        }
    });

    let model = runtime.model();
    let scope = runtime.scope();
    let projected = use_memo(use_reactive(
        (
            &model.accepted.as_ref().map(|snapshot| snapshot.token),
            &scope,
        ),
        {
            let runtime = runtime.clone();
            move |(token, scope)| {
                let (token, scope) = (token?, scope?);
                let snapshot = runtime.model().accepted?;
                (snapshot.token == token && snapshot.document.id == scope.document_id).then(|| {
                    boardstudio_web_host::cad_jobs::captured_case_document(&snapshot, &scope)
                        .map_err(|error| format!("{error:?}"))
                })
            }
        },
    ));

    let mut notice = |message: &str, alert: bool| {
        if *portal.editable.peek() {
            portal.editable.set(false);
        }
        rsx! { p { role: if alert { "alert" } else { "status" }, "{message}" } }
    };
    let Some(snapshot) = model.accepted.as_ref() else {
        return notice("Open a saved keyboard to edit its case bodies.", false);
    };
    let Some(scope) = scope.filter(|scope| {
        scope.board_id == model.active_board_id && scope.document_id == snapshot.document.id
    }) else {
        return notice(
            "The active Case scope is unavailable or changed. Reopen the inspector.",
            true,
        );
    };
    let effective = match projected.read().clone() {
        Some(Ok(document)) => document,
        Some(Err(message)) => {
            return notice(
                &format!("Could not read the active Case projection: {message}"),
                true,
            );
        }
        None => return notice("The active Case projection is not current.", true),
    };
    let configured = effective.mechanical.as_ref();
    let board_name = |id: &str| {
        snapshot
            .document
            .boards
            .iter()
            .find(|board| board.id == id)
            .map_or_else(|| id.to_owned(), |board| board.name.clone())
    };
    let generated_stack = configured.is_some_and(|config| config.board_id == scope.board_id);
    let editable = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.display_preview.is_none()
        && model.gesture.is_none()
        && !generated_stack;
    if *portal.editable.peek() != editable {
        portal.editable.set(editable);
    }
    rsx! {
        CaseBodies {
            key: "{scope.session_epoch.0}:{scope.document_id}:{scope.board_id}:{scope.instance_id:?}",
            board: snapshot.document.boards.iter().find(|board| board.id == scope.board_id).map(|board| CaseBoardSummary { id: board.id.clone(), name: board.name.clone() }),
            bodies: snapshot.document.case_bodies.iter().filter(|body| body.board_id == scope.board_id).cloned().collect::<Vec<_>>(),
            mismatch: configured.filter(|config| config.board_id != scope.board_id).map(|config| CaseMismatch { board_id: config.board_id.clone(), board_name: board_name(&config.board_id) }),
            scope: scope.clone(),
            generated_stack,
            editable,
            error: error(),
            on_edit: {
                let dispatch = dispatch.clone();
                let (token, revision) = (snapshot.token, snapshot.document.revision);
                move |edit| dispatch(edit, scope.clone(), token, revision)
            },
            on_show_configured_board,
        }
    }
}

/// Admit `edit` for `scope` and commit it; returns the id of a newly added body.
fn commit_edit(
    runtime: &Rc<Runtime>,
    instance_selection: super::InstanceSelection,
    scope: &Scope,
    edit: CaseBodyEdit,
) -> Result<Option<String>, String> {
    let model = runtime.model();
    if !instance_selection.is_current(&model)
        || runtime.scope().as_ref() != Some(scope)
        || !matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        || model.display_preview.is_some()
        || model.gesture.is_some()
    {
        return Err("The case cannot be edited right now.".into());
    }
    let seed = runtime.operation().0;
    let resolver = body_resolver(scope.clone(), edit.clone(), seed);
    let created = match (&edit, &model.accepted) {
        (CaseBodyEdit::AddBody, Some(snapshot)) => match resolver.resolve(snapshot) {
            Resolution::Submit(command) => match command.operation {
                EditOperation::SetCase { body } => Some(body.id),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    };
    runtime.commit("case-body", resolver).map(|_| created)
}

fn body_resolver(scope: Scope, edit: CaseBodyEdit, seed: u64) -> EditResolver {
    EditResolver::new("case-body", move |accepted: &AcceptedSnapshot| {
        let document: &ProjectDoc = &accepted.document;
        let Ok(effective) =
            boardstudio_web_host::cad_jobs::captured_case_document(accepted, &scope)
        else {
            return Resolution::Retire("The Case scope no longer exists.".into());
        };
        if effective
            .mechanical
            .as_ref()
            .is_some_and(|config| config.board_id == scope.board_id)
        {
            return Resolution::Retire(
                "Disable the generated mechanical stack before editing authored case bodies."
                    .into(),
            );
        }
        let existing = document.case_bodies.iter().find(|body| {
            Some(body.id.as_str()) == edit.body_id() && body.board_id == scope.board_id
        });
        let mut body = match (&edit, existing) {
            (CaseBodyEdit::AddBody, _) => match new_case_body(seed, document, &scope.board_id) {
                Ok(body) => body,
                Err(message) => return Resolution::Retire(message),
            },
            (_, Some(body)) => body.clone(),
            _ => {
                return Resolution::Retire(
                    "The selected case body no longer belongs to this board.".into(),
                );
            }
        };
        if let Err(message) = edit.apply(seed, document, &mut body) {
            return Resolution::Retire(message);
        }
        if document
            .case_bodies
            .iter()
            .any(|accepted| accepted == &body)
        {
            return Resolution::Unchanged;
        }
        Resolution::submit(vec![body.id.clone()], EditOperation::SetCase { body })
    })
}
