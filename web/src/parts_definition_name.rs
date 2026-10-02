//! Parts-private admission and command construction for one definition-name commit.
use boardstudio_application::{
    AcceptedSnapshot, Event, OperationId, Scope, SessionEpoch, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, PartDefinition};

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{DefinitionNameCapture, prepare_definition_name_edit};
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_core::model::PartDefinition;
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlInputElement;

    #[component]
    pub(crate) fn DefinitionNameEditor(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut draft = use_signal(|| definition.name.clone());
        let identity = (
            scope.clone(),
            snapshot.token,
            snapshot.session_epoch,
            snapshot.document.id.clone(),
            snapshot.document.revision,
            selection(),
            definition.id.clone(),
            definition.name.clone(),
        );
        let mut capture =
            use_signal(|| DefinitionNameCapture::new(&snapshot, scope.clone(), &definition));
        use_effect(use_reactive((&identity,), {
            let snapshot = snapshot.clone();
            let scope = scope.clone();
            let definition = definition.clone();
            move |(_identity,)| {
                draft.set(definition.name.clone());
                capture.set(DefinitionNameCapture::new(
                    &snapshot,
                    scope.clone(),
                    &definition,
                ));
            }
        }));

        let on_blur = {
            let runtime = runtime.clone();
            let capture = capture.clone();
            let selection = selection;
            move |_| {
                let model = runtime.model();
                let Some(current) = model.accepted else {
                    return;
                };
                let Ok(Some(event)) = prepare_definition_name_edit(
                    &current,
                    runtime.scope(),
                    selection(),
                    &capture(),
                    &draft(),
                    runtime.operation(),
                ) else {
                    return;
                };
                runtime.submit(event);
            }
        };
        let on_keydown = {
            let mut keydown_draft = draft;
            let value = definition.name.clone();
            move |event: KeyboardEvent| {
                if event.key() == Key::Escape {
                    event.prevent_default();
                    keydown_draft.set(value.clone());
                } else if event.key() == Key::Enter
                    && let Some(input) = event
                        .data()
                        .try_as_web_event()
                        .and_then(|event| event.target())
                        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                {
                    let _ = input.blur();
                }
            }
        };

        rsx! {
            label { class: "m1-parts-definition-name",
                "Name"
                input {
                    aria_label: "Definition name",
                    value: "{draft()}",
                    oninput: move |event| draft.set(event.value()),
                    onblur: on_blur,
                    onkeydown: on_keydown,
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use ui::DefinitionNameEditor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DefinitionNameCapture {
    scope: Option<Scope>,
    definition_id: String,
    session_epoch: SessionEpoch,
    document_id: String,
    snapshot_token: SnapshotToken,
    revision: u64,
}

impl DefinitionNameCapture {
    pub(crate) fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        definition: &PartDefinition,
    ) -> Self {
        Self {
            scope,
            definition_id: definition.id.clone(),
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
        }
    }
}

pub(crate) fn prepare_definition_name_edit(
    current: &AcceptedSnapshot,
    current_scope: Option<Scope>,
    current_selection: Option<(Option<Scope>, String)>,
    capture: &DefinitionNameCapture,
    name: &str,
    operation_id: OperationId,
) -> Result<Option<Event>, String> {
    // The edit is admitted only while the accepted document, selection, and
    // scope still belong to the draft that produced it. In particular, a
    // same-named definition in a newly accepted document must not receive an
    // old draft when the Inspector remains mounted.
    if capture.scope.is_none()
        || current_scope != capture.scope
        || current.token != capture.snapshot_token
        || current.session_epoch != capture.session_epoch
        || current.document.id != capture.document_id
        || current.document.revision != capture.revision
        || current_selection != Some((capture.scope.clone(), capture.definition_id.clone()))
    {
        return Ok(None);
    }

    let mut replacement = current.document.as_ref().clone();
    let Some(definition) = replacement
        .definitions
        .iter_mut()
        .find(|definition| definition.id == capture.definition_id)
    else {
        return Ok(None);
    };
    // Generator definitions are edited through their own parameter workflow;
    // their names are not exposed by this first project-definition slice.
    if definition.generator.is_some() || definition.name == name {
        return Ok(None);
    }
    definition.name = name.to_owned();

    Ok(Some(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: capture.revision,
            transaction_id: format!("parts-definition-name-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![capture.definition_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{Completion, Effect, SaveResult, Session};
    use boardstudio_core::{
        CoreEngine,
        model::ProjectDoc,
        model::{AssemblyDefinition, Asset, Net, Part, Pin, Pose2, Side, Vec2},
    };

    fn definition(id: &str, name: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name,
            "kind": "custom",
            "courtyard": [],
            "pads": []
        }))
        .unwrap()
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("parts-name-project", "Parts name fixture");
        document.definitions = vec![
            definition("selected", "Original name"),
            definition("other", "Unaffected"),
        ];
        document.parts.push(Part {
            id: "placed-part".into(),
            definition_id: "selected".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 5.0, y: 8.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.nets.push(Net {
            id: "net-1".into(),
            name: "DATA".into(),
            pins: vec![Pin {
                part_id: "placed-part".into(),
                pad_id: "1".into(),
            }],
        });
        document.assets.push(Asset {
            id: "asset-1".into(),
            name: "model.step".into(),
            media_type: "model/step".into(),
            sha256: "0".repeat(64),
            license: None,
            source: None,
        });
        document.assemblies.push(AssemblyDefinition {
            id: "assembly-1".into(),
            name: "Saved assembly".into(),
            members: vec![],
        });
        document
    }

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
        assert!(session.read_model().accepted.is_some());
        (session, core)
    }

    fn assert_same_content_ignoring_revision(actual: &ProjectDoc, expected: &ProjectDoc) {
        let mut actual = actual.clone();
        actual.revision = expected.revision;
        assert_eq!(actual, *expected);
    }

    #[test]
    fn production_name_edit_commits_one_field_and_round_trips_through_session_history() {
        let original = document();
        let (mut session, mut core) = open_document(original.clone());
        let snapshot = session.read_model().accepted.clone().unwrap();
        let scope = session.scope();
        let capture =
            DefinitionNameCapture::new(&snapshot, scope.clone(), &snapshot.document.definitions[0]);
        let selected = Some((scope.clone(), "selected".into()));

        let event = prepare_definition_name_edit(
            &snapshot,
            scope,
            selected,
            &capture,
            "Renamed device",
            OperationId(2),
        )
        .unwrap()
        .expect("a changed name produces one accepted edit event");

        let Event::Edit { command, .. } = &event else {
            panic!("name edit must use normal Session edit");
        };
        assert_eq!(command.base_revision, snapshot.document.revision);
        assert_eq!(command.phase, EditPhase::Commit);
        assert_eq!(command.target_ids, vec!["selected"]);
        let EditOperation::ReplaceDocument {
            document: replacement,
        } = &command.operation
        else {
            panic!("the private Parts adapter must reuse the supported ReplaceDocument path");
        };
        let mut expected_renamed = original.clone();
        expected_renamed.definitions[0].name = "Renamed device".into();
        assert_same_content_ignoring_revision(replacement, &expected_renamed);

        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        let accepted = session.read_model().accepted.as_ref().unwrap();
        assert_same_content_ignoring_revision(&accepted.document, &expected_renamed);
        assert_eq!(
            session.read_model().durability,
            boardstudio_application::Durability::Saved {
                revision: accepted.document.revision
            }
        );

        let undo_effects = session.submit(Event::Undo {
            operation_id: OperationId(3),
        });
        advance(&mut session, &mut core, undo_effects);
        let undone = session.read_model().accepted.as_ref().unwrap();
        assert_same_content_ignoring_revision(&undone.document, &original);
        let redo_effects = session.submit(Event::Redo {
            operation_id: OperationId(4),
        });
        advance(&mut session, &mut core, redo_effects);
        let redone = session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .clone();
        assert_same_content_ignoring_revision(&redone, &expected_renamed);

        let persisted_round_trip: ProjectDoc =
            serde_json::from_str(&serde_json::to_string(&*redone).unwrap()).unwrap();
        let (mut reopened, mut reopen_core) = open_document(persisted_round_trip);
        let reopened_document = reopened
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .clone();
        assert_same_content_ignoring_revision(&reopened_document, &expected_renamed);
        assert_eq!(
            reopened_document
                .definitions
                .iter()
                .find(|definition| definition.id == "other")
                .unwrap()
                .name,
            "Unaffected"
        );
        let _ = (&mut reopened, &mut reopen_core);
    }

    #[test]
    fn stale_selection_document_and_generator_captures_are_rejected() {
        let (session, _core) = open_document(document());
        let snapshot = session.read_model().accepted.clone().unwrap();
        let scope = session.scope();
        let selected = Some((scope.clone(), "selected".into()));
        let capture =
            DefinitionNameCapture::new(&snapshot, scope.clone(), &snapshot.document.definitions[0]);

        assert!(
            prepare_definition_name_edit(
                &snapshot,
                scope.clone(),
                Some((scope.clone(), "other".into())),
                &capture,
                "stale selection",
                OperationId(10),
            )
            .unwrap()
            .is_none()
        );
        assert!(
            prepare_definition_name_edit(
                &snapshot,
                scope.clone(),
                selected.clone(),
                &capture,
                "Original name",
                OperationId(14),
            )
            .unwrap()
            .is_none()
        );

        let other_scope = scope.as_ref().map(|scope| {
            let mut changed = scope.clone();
            changed.instance_id = Some("other-instance".into());
            changed
        });
        assert!(
            prepare_definition_name_edit(
                &snapshot,
                other_scope,
                selected.clone(),
                &capture,
                "stale scope",
                OperationId(11),
            )
            .unwrap()
            .is_none()
        );

        let mut changed_doc = snapshot.document.as_ref().clone();
        changed_doc.revision += 1;
        let changed_snapshot = AcceptedSnapshot {
            document: std::sync::Arc::new(changed_doc),
            ..snapshot.clone()
        };
        assert!(
            prepare_definition_name_edit(
                &changed_snapshot,
                scope.clone(),
                selected.clone(),
                &capture,
                "stale revision",
                OperationId(12),
            )
            .unwrap()
            .is_none()
        );

        let mut changed_identity_doc = snapshot.document.as_ref().clone();
        changed_identity_doc.id = "another-project".into();
        let changed_identity_snapshot = AcceptedSnapshot {
            document: std::sync::Arc::new(changed_identity_doc),
            ..snapshot.clone()
        };
        assert!(
            prepare_definition_name_edit(
                &changed_identity_snapshot,
                scope.clone(),
                selected.clone(),
                &capture,
                "stale document identity",
                OperationId(15),
            )
            .unwrap()
            .is_none()
        );

        let mut generator_doc = snapshot.document.as_ref().clone();
        generator_doc.definitions[0].generator = Some(boardstudio_core::model::PartGenerator {
            source: "generator/source".into(),
            version: "1".into(),
            parameters: Default::default(),
        });
        let generator_snapshot = AcceptedSnapshot {
            document: std::sync::Arc::new(generator_doc),
            ..snapshot.clone()
        };
        let generator_capture = DefinitionNameCapture::new(
            &generator_snapshot,
            scope.clone(),
            &generator_snapshot.document.definitions[0],
        );
        assert!(
            prepare_definition_name_edit(
                &generator_snapshot,
                scope,
                selected,
                &generator_capture,
                "generator name",
                OperationId(13),
            )
            .unwrap()
            .is_none()
        );
    }
}
