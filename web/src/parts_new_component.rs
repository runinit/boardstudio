//! Parts-private create and acceptance reconciliation for custom definitions.

use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, SnapshotToken};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, PartDefinition};

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{CreateCapture, non_colliding_id, prepare_create_edit, reconciliation_is_current};
    use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
    use boardstudio_application::{AcceptedSnapshot, Scope, TerminalOutcome};
    use dioxus::prelude::*;
    use js_sys::{Date, Function, Reflect};
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue};

    #[derive(Clone)]
    struct PendingCreate {
        capture: CreateCapture,
        definition_id: String,
        outcome: OutcomeSlot,
    }

    /// The Parts parent mounts this action outside its catalogue loading/error branches so
    /// module-source availability never controls whether a project definition can be created.
    #[component]
    pub(crate) fn NewCustomComponentAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        workspace: Signal<&'static str>,
        query: Signal<String>,
        selected: Signal<Option<(Option<Scope>, String)>>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_context::<Signal<u64>>();
        let pending = use_signal(|| None::<PendingCreate>);
        let owner_is_mounted = use_hook(|| Rc::new(Cell::new(true)));
        use_drop({
            let owner_is_mounted = owner_is_mounted.clone();
            move || owner_is_mounted.set(false)
        });

        // The owner generation is advanced by the Parts composition for query, selection,
        // workspace and scope intent. Runtime's version only wakes this effect on settlement.
        use_effect(use_reactive((&version(),), {
            let runtime = runtime.clone();
            let owner_is_mounted = owner_is_mounted.clone();
            let mut pending = pending;
            let mut selected = selected;
            let mut query = query;
            move |_| {
                let Some(waiting) = pending.read().clone() else {
                    return;
                };
                let Some(outcome) = waiting.outcome.borrow().clone() else {
                    return;
                };
                pending.set(None);
                if outcome != TerminalOutcome::Completed || !owner_is_mounted.get() {
                    return;
                }

                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                if !reconciliation_is_current(
                    &waiting.capture,
                    runtime.scope().as_ref(),
                    view_generation(),
                    workspace(),
                    snapshot,
                    &waiting.definition_id,
                ) {
                    return;
                }

                selected.set(Some((
                    Some(waiting.capture.scope.clone()),
                    waiting.definition_id,
                )));
                query.set(String::new());
                on_select.call(());
            }
        }));

        let create = {
            let runtime = runtime.clone();
            let mut pending = pending;
            let owner_is_mounted = owner_is_mounted.clone();
            move |_| {
                if pending.read().is_some() || !owner_is_mounted.get() || workspace() != "Parts" {
                    return;
                }
                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref().cloned() else {
                    return;
                };
                let Some(current_scope) = runtime.scope() else {
                    return;
                };
                if scope.as_ref() != Some(&current_scope)
                    || snapshot.document.id != current_scope.document_id
                    || snapshot.session_epoch != current_scope.session_epoch
                {
                    return;
                }

                let definition_id = match create_id(&snapshot) {
                    Ok(id) => id,
                    Err(error) => {
                        runtime.report(error);
                        return;
                    }
                };
                let operation_id = runtime.operation();
                let capture = CreateCapture::new(
                    &snapshot,
                    current_scope,
                    view_generation(),
                    definition_id.clone(),
                );
                let Ok(event) = prepare_create_edit(&snapshot, &capture, operation_id) else {
                    return;
                };
                let outcome = runtime.observe_operation(operation_id);
                // Install the slot before submit because an edit may settle synchronously.
                pending.set(Some(PendingCreate {
                    capture,
                    definition_id,
                    outcome,
                }));
                runtime.submit(event);
            }
        };

        rsx! {
            button {
                class: "m1-parts-create-component",
                type: "button",
                "aria-label": "New custom component",
                disabled: pending.read().is_some(),
                onclick: create,
                "New custom component"
            }
        }
    }

    fn create_id(snapshot: &AcceptedSnapshot) -> Result<String, String> {
        let base = browser_identity().unwrap_or_else(date_identity);
        non_colliding_id(&base, &snapshot.document.definitions)
            .ok_or_else(|| "Could not allocate a unique component identity.".to_string())
    }

    fn browser_identity() -> Option<String> {
        let crypto = Reflect::get(&js_sys::global(), &JsValue::from_str("crypto")).ok()?;
        let random_uuid = Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
            .ok()?
            .dyn_into::<Function>()
            .ok()?;
        random_uuid.call0(&crypto).ok()?.as_string()
    }

    fn date_identity() -> String {
        let milliseconds = Date::now().max(0.0) as u64;
        radix36(milliseconds)
    }

    fn radix36(mut value: u64) -> String {
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        if value == 0 {
            return "0".into();
        }
        let mut digits = Vec::new();
        while value > 0 {
            digits.push(DIGITS[(value % 36) as usize]);
            value /= 36;
        }
        digits.reverse();
        String::from_utf8(digits).expect("base-36 digits are valid UTF-8")
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use ui::NewCustomComponentAction;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CreateCapture {
    scope: Scope,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    snapshot_token: SnapshotToken,
    revision: u64,
    view_generation: u64,
    definition_id: String,
}

impl CreateCapture {
    pub(crate) fn new(
        snapshot: &AcceptedSnapshot,
        scope: Scope,
        view_generation: u64,
        definition_id: String,
    ) -> Self {
        Self {
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            view_generation,
            definition_id,
        }
    }
}

pub(crate) fn prepare_create_edit(
    snapshot: &AcceptedSnapshot,
    capture: &CreateCapture,
    operation_id: OperationId,
) -> Result<Event, String> {
    if snapshot.session_epoch != capture.session_epoch
        || snapshot.document.id != capture.document_id
        || snapshot.token != capture.snapshot_token
        || snapshot.document.revision != capture.revision
        || capture.scope.document_id != capture.document_id
        || capture.scope.session_epoch != capture.session_epoch
    {
        return Err("The accepted Parts document changed before the component was created.".into());
    }

    let mut document = snapshot.document.as_ref().clone();
    if document
        .definitions
        .iter()
        .any(|definition| definition.id == capture.definition_id)
    {
        return Err("The new component identity is already in use.".into());
    }
    let definition = custom_definition(
        capture.definition_id.clone(),
        document.definitions.len() + 1,
    )?;
    document.definitions.push(definition);

    Ok(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: capture.revision,
            transaction_id: format!("parts-create-component-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![capture.definition_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    })
}

fn custom_definition(id: String, number: usize) -> Result<PartDefinition, String> {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": format!("Custom component {number}"),
        "kind": "custom",
        "courtyard": [
            { "x": -5.0, "y": -3.0 },
            { "x": 5.0, "y": -3.0 },
            { "x": 5.0, "y": 3.0 },
            { "x": -5.0, "y": 3.0 }
        ],
        "pads": []
    }))
    .map_err(|error| format!("Could not prepare the default custom component: {error}"))
}

fn non_colliding_id(base: &str, definitions: &[PartDefinition]) -> Option<String> {
    let stem = format!("ui-{base}");
    (0_u64..)
        .map(|suffix| {
            if suffix == 0 {
                stem.clone()
            } else {
                format!("{stem}-{suffix}")
            }
        })
        .find(|candidate| {
            definitions
                .iter()
                .all(|definition| definition.id != *candidate)
        })
}

pub(crate) fn reconciliation_is_current(
    capture: &CreateCapture,
    current_scope: Option<&Scope>,
    current_generation: u64,
    current_workspace: &str,
    current: &AcceptedSnapshot,
    definition_id: &str,
) -> bool {
    current_workspace == "Parts"
        && current_generation == capture.view_generation
        && current_scope == Some(&capture.scope)
        && current.session_epoch == capture.session_epoch
        && current.document.id == capture.document_id
        && current.document.revision > capture.revision
        && current.document.definitions.iter().any(|definition| {
            definition.id == definition_id && definition_id == capture.definition_id
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Completion, Effect, SaveResult, Session};
    use boardstudio_core::CoreEngine;
    use boardstudio_core::model::{PartDefinition, ProjectDoc};

    fn advance(session: &mut Session, core: &mut CoreEngine, initial: Vec<Effect>) {
        let mut pending = initial;
        while let Some(effect) = pending.pop() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => {
                    let reply = core.handle(*request);
                    pending.extend(session.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }));
                }
                Effect::Persist {
                    save_attempt_id, ..
                } => {
                    pending.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
    }

    fn open_document(document: ProjectDoc) -> (Session, CoreEngine) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document,
        });
        advance(&mut session, &mut core, effects);
        (session, core)
    }

    #[test]
    fn create_edit_appends_reference_default_and_preserves_the_accepted_document() {
        let mut document = ProjectDoc::empty("parts-create-test", "Parts create fixture");
        document.definitions.push(
            serde_json::from_value::<PartDefinition>(serde_json::json!({
                "id": "existing", "name": "Existing", "kind": "custom",
                "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        let original_parts = document.parts.clone();
        let original_boards = document.boards.clone();
        let (mut session, mut core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope().unwrap();
        let capture = CreateCapture::new(&snapshot, scope.clone(), 4, "ui-fresh".into());
        let event = prepare_create_edit(&snapshot, &capture, OperationId(21)).unwrap();

        let Event::Edit {
            operation_id,
            command,
        } = &event
        else {
            panic!("creation must use the ordinary edit path");
        };
        let EditOperation::ReplaceDocument { document } = &command.operation else {
            panic!("creation must replace the current accepted document");
        };
        let created = document
            .definitions
            .iter()
            .find(|definition| definition.id == "ui-fresh")
            .unwrap();
        assert_eq!(*operation_id, OperationId(21));
        assert_eq!(command.base_revision, snapshot.document.revision);
        assert_eq!(command.target_ids, ["ui-fresh"]);
        assert_eq!(command.phase, EditPhase::Commit);
        assert_eq!(created.name, "Custom component 2");
        assert!(matches!(
            &created.kind,
            boardstudio_core::model::PartKind::Custom
        ));
        assert_eq!(created.pads.len(), 0);
        assert_eq!(
            created
                .courtyard
                .iter()
                .map(|point| (point.x, point.y))
                .collect::<Vec<_>>(),
            [(-5.0, -3.0), (5.0, -3.0), (5.0, 3.0), (-5.0, 3.0)]
        );
        assert_eq!(document.parts, original_parts);
        assert_eq!(document.boards, original_boards);
        assert_eq!(snapshot.document.definitions.len(), 1);

        let accepted_effects = session.submit(event);
        advance(&mut session, &mut core, accepted_effects);
        let accepted_create = session.read_model().accepted.as_ref().unwrap();
        assert!(reconciliation_is_current(
            &capture,
            session.scope().as_ref(),
            4,
            "Parts",
            accepted_create,
            "ui-fresh",
        ));

        let undo_effects = session.submit(Event::Undo {
            operation_id: OperationId(24),
        });
        advance(&mut session, &mut core, undo_effects);
        assert!(
            !session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .definitions
                .iter()
                .any(|definition| definition.id == "ui-fresh")
        );
        let redo_effects = session.submit(Event::Redo {
            operation_id: OperationId(25),
        });
        advance(&mut session, &mut core, redo_effects);
        assert!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .definitions
                .iter()
                .any(|definition| definition.id == "ui-fresh")
        );
    }

    #[test]
    fn create_edit_rejects_changed_capture_and_duplicate_identity() {
        let document = ProjectDoc::empty("parts-create-test", "Parts create fixture");
        let (session, _) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let capture = CreateCapture::new(&snapshot, session.scope().unwrap(), 4, "ui-fresh".into());
        let newer = AcceptedSnapshot {
            token: SnapshotToken(snapshot.token.0 + 1),
            ..snapshot.clone()
        };
        assert!(prepare_create_edit(&newer, &capture, OperationId(22)).is_err());

        let mut collided_document = (*snapshot.document).clone();
        collided_document.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "ui-fresh", "name": "Existing", "kind": "custom",
                "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        let collided = AcceptedSnapshot {
            document: std::sync::Arc::new(collided_document),
            ..snapshot
        };
        assert!(prepare_create_edit(&collided, &capture, OperationId(23)).is_err());
    }

    #[test]
    fn generated_ui_identity_skips_existing_ids() {
        let definitions = ["ui-same", "ui-same-1"]
            .into_iter()
            .map(|id| {
                serde_json::from_value::<PartDefinition>(serde_json::json!({
                    "id": id, "name": id, "kind": "custom",
                    "courtyard": [], "pads": []
                }))
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            non_colliding_id("same", &definitions).as_deref(),
            Some("ui-same-2")
        );
    }

    #[test]
    fn reconciliation_requires_accepted_definition_and_current_parts_owner_intent() {
        let (mut session, mut core) = open_document(ProjectDoc::empty(
            "parts-create-test",
            "Parts create fixture",
        ));
        let original = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope().unwrap();
        let capture = CreateCapture::new(&original, scope.clone(), 4, "ui-fresh".into());
        let event = prepare_create_edit(&original, &capture, OperationId(30)).unwrap();
        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        let accepted_create = session.read_model().accepted.as_ref().unwrap();

        assert!(reconciliation_is_current(
            &capture,
            Some(&scope),
            4,
            "Parts",
            accepted_create,
            "ui-fresh",
        ));
        assert!(!reconciliation_is_current(
            &capture,
            Some(&scope),
            5,
            "Parts",
            accepted_create,
            "ui-fresh",
        ));
        assert!(!reconciliation_is_current(
            &capture,
            Some(&scope),
            4,
            "Layout",
            accepted_create,
            "ui-fresh",
        ));
        assert!(!reconciliation_is_current(
            &capture,
            None,
            4,
            "Parts",
            accepted_create,
            "ui-fresh",
        ));
        assert!(!reconciliation_is_current(
            &capture,
            Some(&scope),
            4,
            "Parts",
            &original,
            "ui-fresh",
        ));
    }
}
