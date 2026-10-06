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
    pub fn DefinitionNameEditor(
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: PartDefinition,
        children: Element,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut draft = use_signal(|| definition.name.clone());
        let pad_count = definition.pads.len();
        let mut section_open = use_signal(|| pad_count == 0);
        let mut section_chosen = use_signal(|| false);
        let section_owner = (scope.clone(), definition.id.clone());
        // A fresh accepted snapshot is required at commit time, but it is not
        // a reason to discard a dirty field draft. React DraftInput keys its
        // reset to the accepted field value; keep this target identity
        // independent from the snapshot capture that protects submission.
        let draft_identity = (
            scope.clone(),
            selection(),
            definition.id.clone(),
            definition.name.clone(),
        );
        let capture_identity = (
            draft_identity.clone(),
            snapshot.token,
            snapshot.session_epoch,
            snapshot.document.id.clone(),
            snapshot.document.revision,
        );
        let mut capture =
            use_signal(|| DefinitionNameCapture::new(&snapshot, scope.clone(), &definition));
        use_effect(use_reactive((&draft_identity,), {
            let definition = definition.clone();
            move |(_identity,)| {
                draft.set(definition.name.clone());
            }
        }));
        use_effect(use_reactive((&capture_identity,), {
            let snapshot = snapshot.clone();
            let scope = scope.clone();
            let definition = definition.clone();
            move |(_identity,)| {
                capture.set(DefinitionNameCapture::new(
                    &snapshot,
                    scope.clone(),
                    &definition,
                ));
            }
        }));
        use_effect(use_reactive((&pad_count,), move |(pad_count,)| {
            if !section_chosen() {
                section_open.set(pad_count == 0);
            }
        }));
        // React keys this Inspector subtree by the selected definition. Keep
        // the user's disclosure choice through edits to that definition, but
        // start a newly selected definition from its own pad-count default.
        use_effect(use_reactive((&section_owner,), {
            move |(_owner,)| {
                section_chosen.set(false);
                section_open.set(pad_count == 0);
            }
        }));

        let on_blur = {
            let runtime = runtime.clone();
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
        let on_section_toggle = move |event: MouseEvent| {
            event.prevent_default();
            section_chosen.set(true);
            section_open.set(!section_open());
        };

        rsx! {
            details {
                class: "m1-parts-definition-editor",
                open: section_open(),
                summary { onclick: on_section_toggle,
                    span { "Edit footprint" }
                    small { "Custom geometry" }
                }
                div { class: "m1-parts-definition-editor-body",
                    section { class: "m1-parts-definition-editor-fields", "aria-label": "Custom component definition editor",
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
                    {children}
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use ui::DefinitionNameEditor;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionNameCapture {
    scope: Option<Scope>,
    definition_id: String,
    session_epoch: SessionEpoch,
    document_id: String,
    snapshot_token: SnapshotToken,
    revision: u64,
}

impl DefinitionNameCapture {
    pub fn new(
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

pub fn prepare_definition_name_edit(
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

    #[test]
    fn refreshed_capture_commits_dirty_name_against_the_latest_accepted_document() {
        let original = document();
        let (mut session, mut core) = open_document(original);
        let initial = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        let selection = Some((scope.clone(), "selected".into()));

        let mut unrelated_document = initial.document.as_ref().clone();
        unrelated_document
            .parameters
            .insert("unrelated-edit".into(), serde_json::json!(true));
        let unrelated = Event::Edit {
            operation_id: OperationId(20),
            command: EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "parts-name-unrelated-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["unrelated".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated_document),
                },
            },
        };
        let effects = session.submit(unrelated);
        advance(&mut session, &mut core, effects);
        let latest = session.read_model().accepted.as_ref().unwrap().clone();
        let current_definition = latest
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == "selected")
            .unwrap();
        let refreshed = DefinitionNameCapture::new(&latest, scope.clone(), current_definition);
        let name_edit = prepare_definition_name_edit(
            &latest,
            scope,
            selection,
            &refreshed,
            "Dirty name draft",
            OperationId(21),
        )
        .unwrap()
        .expect("the dirty field remains admissible with a refreshed accepted capture");
        let effects = session.submit(name_edit);
        advance(&mut session, &mut core, effects);

        let accepted = session.read_model().accepted.as_ref().unwrap();
        assert_eq!(
            accepted.document.parameters.get("unrelated-edit"),
            Some(&serde_json::json!(true)),
            "the name commit must retain the edit accepted while its local draft was dirty"
        );
        assert_eq!(
            accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == "selected")
                .unwrap()
                .name,
            "Dirty name draft"
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::runtime::Runtime;
    use boardstudio_application::{
        Completion, Effect, Event as AppEvent, SaveResult, Scope, Session,
    };
    use boardstudio_core::{
        CoreEngine,
        model::ProjectDoc,
        model::{EditCommand, EditOperation, EditPhase},
    };
    use dioxus::prelude::*;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{Event as DomEvent, HtmlInputElement};

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct State {
        snapshot: Signal<AcceptedSnapshot>,
        scope: Signal<Option<Scope>>,
        selection: Signal<Option<(Option<Scope>, String)>>,
        definition: Signal<PartDefinition>,
        runtime: Rc<Runtime>,
    }

    struct Seed {
        snapshot: AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Option<(Option<Scope>, String)>,
        definition: PartDefinition,
        state: Rc<RefCell<Option<State>>>,
    }

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

    fn host() -> Element {
        let seed = use_context::<Rc<Seed>>();
        let snapshot = use_signal(|| seed.snapshot.clone());
        let scope = use_signal(|| seed.scope.clone());
        let selection = use_signal(|| seed.selection.clone());
        let definition = use_signal(|| seed.definition.clone());
        *seed.state.borrow_mut() = Some(State {
            snapshot,
            scope,
            selection,
            definition,
            runtime: use_context::<Rc<Runtime>>(),
        });
        rsx! {
            div { "data-snapshot-revision": "{snapshot().document.revision}",
                DefinitionNameEditor {
                    snapshot: snapshot(),
                    scope: scope(),
                    selection,
                    definition: definition(),
                    children: rsx! {
                        crate::parts_custom_definition::CustomDefinitionFields {
                            snapshot: snapshot(),
                            scope: scope(),
                            selection,
                            definition: definition(),
                        }
                    },
                }
            }
        }
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(80).await;
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

    fn accept_document_replacement(
        session: &mut Session,
        core: &mut CoreEngine,
        operation_id: u64,
        document: ProjectDoc,
        target_id: &str,
    ) -> AcceptedSnapshot {
        let current = session.read_model().accepted.as_ref().unwrap().clone();
        let effects = session.submit(AppEvent::Edit {
            operation_id: OperationId(operation_id),
            command: EditCommand {
                base_revision: current.document.revision,
                transaction_id: format!("parts04-mounted-refresh-{operation_id}"),
                phase: EditPhase::Commit,
                target_ids: vec![target_id.to_owned()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            },
        });
        advance(session, core, effects);
        session.read_model().accepted.as_ref().unwrap().clone()
    }

    fn open_document(document: ProjectDoc) -> (Session, CoreEngine) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(AppEvent::Open {
            operation_id: OperationId(900),
            document,
        });
        advance(&mut session, &mut core, effects);
        assert!(session.read_model().accepted.is_some());
        (session, core)
    }

    fn input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Definition name']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn section_is_open() -> bool {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression details")
            .unwrap()
            .unwrap()
            .has_attribute("open")
    }

    fn pad_id_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Pad 1 ID']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_number_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Pad 1 number']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn courtyard_width_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Courtyard width']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn courtyard_height_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Courtyard height']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_x_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Pad 1 X']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_y_input() -> HtmlInputElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression input[aria-label='Pad 1 Y']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn mounted_definition_owner_switch_and_pad_id_refresh_retain_and_retire_drafts() {
        crate::parts_custom_definition::clear_pad_number_draft_for_test();
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        let mut first = definition("first", "First");
        let mut second = definition("second", "Second");
        for definition in [&mut first, &mut second] {
            definition.courtyard = vec![
                boardstudio_core::model::Vec2 { x: -5.0, y: -3.0 },
                boardstudio_core::model::Vec2 { x: 5.0, y: -3.0 },
                boardstudio_core::model::Vec2 { x: 5.0, y: 3.0 },
                boardstudio_core::model::Vec2 { x: -5.0, y: 3.0 },
            ];
            definition.pads = serde_json::from_value(serde_json::json!([
                {"id":"shared-pad","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
            ])).unwrap();
        }
        document.definitions = vec![first, second];
        let runtime = Runtime::new().unwrap();
        let (mut session, mut core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        runtime.set_definition_name_test_state(snapshot.clone(), scope.clone());
        let state = Rc::new(RefCell::new(None));
        let seed = Rc::new(Seed {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            selection: Some((scope.clone(), "first".into())),
            definition: snapshot.document.definitions[0].clone(),
            state: state.clone(),
        });
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("parts-name-mounted-regression");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(seed);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        root.query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let pad_id = pad_id_input();
        type_value(&pad_id, "");
        settle().await;
        let _ = input().focus();
        settle().await;
        assert!(root.query_selector("[role='alert']").unwrap().is_some());

        let mut controls = state.borrow().as_ref().unwrap().clone();
        controls
            .selection
            .set(Some((scope.clone(), "second".into())));
        controls
            .definition
            .set(snapshot.document.definitions[1].clone());
        settle().await;
        root.query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;

        assert_eq!(pad_id_input().value(), "shared-pad");
        assert!(root.query_selector("[role='alert']").unwrap().is_none());

        // A real accepted pad-ID and Y-coordinate update must preserve a dirty
        // number draft in the same row. This catches rows keyed by mutable ID.
        assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("7"));
        settle().await;
        assert_eq!(pad_number_input().value(), "7");
        let mut refreshed_doc = session
            .read_model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref()
            .clone();
        refreshed_doc.definitions[1].pads[0].id = "renamed-pad".into();
        refreshed_doc.definitions[1].pads[0].at.y = 4.0;
        let refreshed =
            accept_document_replacement(&mut session, &mut core, 911, refreshed_doc, "second");
        controls
            .runtime
            .set_definition_name_test_state(refreshed.clone(), scope.clone());
        controls.snapshot.set(refreshed.clone());
        controls
            .definition
            .set(refreshed.document.definitions[1].clone());
        settle().await;
        assert_eq!(pad_y_input().value(), "4");
        assert_eq!(pad_id_input().value(), "renamed-pad");
        assert_eq!(pad_number_input().value(), "7");
        let _ = pad_number_input().focus();
        settle().await;
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .is_same_node(Some(&pad_number_input()))
        );
        let _ = pad_number_input().blur();
        settle().await;
        let event = runtime
            .take_definition_name_test_event()
            .expect("dirty row draft submits against the accepted renamed pad");
        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        let accepted = session.read_model().accepted.as_ref().unwrap().clone();
        assert_eq!(accepted.document.definitions[1].pads[0].id, "renamed-pad");
        assert_eq!(accepted.document.definitions[1].pads[0].number, "7");
        assert_eq!(accepted.document.definitions[1].pads[0].at.y, 4.0);

        // Deleting a row must retire its draft, even when the next pad has
        // identical accepted scalar values and moves into that row's index.
        let mut with_survivor = accepted.document.as_ref().clone();
        let mut survivor = with_survivor.definitions[1].pads[0].clone();
        survivor.id = "surviving-pad".into();
        with_survivor.definitions[1].pads.push(survivor);
        let expanded =
            accept_document_replacement(&mut session, &mut core, 912, with_survivor, "second");
        controls
            .runtime
            .set_definition_name_test_state(expanded.clone(), scope.clone());
        controls.snapshot.set(expanded.clone());
        controls
            .definition
            .set(expanded.document.definitions[1].clone());
        settle().await;
        assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("9"));
        settle().await;
        assert_eq!(pad_number_input().value(), "9");
        root.query_selector(".m1-definition-remove-pad")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let removal = runtime
            .take_definition_name_test_event()
            .expect("production Remove pad emits one scoped edit");
        let effects = session.submit(removal);
        advance(&mut session, &mut core, effects);
        let accepted = session.read_model().accepted.as_ref().unwrap().clone();
        assert_eq!(accepted.document.definitions[1].pads.len(), 1);
        assert_eq!(accepted.document.definitions[1].pads[0].id, "surviving-pad");
        controls
            .runtime
            .set_definition_name_test_state(accepted.clone(), scope.clone());
        controls.snapshot.set(accepted.clone());
        controls
            .definition
            .set(accepted.document.definitions[1].clone());
        settle().await;
        assert_eq!(pad_id_input().value(), "surviving-pad");
        assert_eq!(
            pad_number_input().value(),
            "7",
            "a removed pad's dirty draft cannot move into the surviving pad"
        );

        controls
            .selection
            .set(Some((scope.clone(), "first".into())));
        controls
            .definition
            .set(accepted.document.definitions[0].clone());
        controls.snapshot.set(accepted.clone());
        controls
            .runtime
            .set_definition_name_test_state(accepted.clone(), scope.clone());
        settle().await;
        assert_eq!(pad_id_input().value(), "shared-pad");
        assert_eq!(pad_number_input().value(), "1");
        let _ = pad_number_input().blur();
        settle().await;
        assert!(
            runtime.take_definition_name_test_event().is_none(),
            "dirty values owned by the prior definition cannot commit into the next owner"
        );
        crate::parts_custom_definition::clear_pad_number_draft_for_test();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_definition_owner_switch_retires_dirty_courtyard_pad_and_error_state() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        let mut first = definition("first", "First");
        let mut second = definition("second", "Second");
        for definition in [&mut first, &mut second] {
            definition.courtyard = vec![
                boardstudio_core::model::Vec2 { x: -5.0, y: -3.0 },
                boardstudio_core::model::Vec2 { x: 5.0, y: -3.0 },
                boardstudio_core::model::Vec2 { x: 5.0, y: 3.0 },
                boardstudio_core::model::Vec2 { x: -5.0, y: 3.0 },
            ];
            definition.pads = serde_json::from_value(serde_json::json!([
                {"id":"shared-pad","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
            ])).unwrap();
        }
        document.definitions = vec![first, second];
        let runtime = Runtime::new().unwrap();
        let (session, _core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        runtime.set_definition_name_test_state(snapshot.clone(), scope.clone());
        let state = Rc::new(RefCell::new(None));
        let seed = Rc::new(Seed {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            selection: Some((scope.clone(), "first".into())),
            definition: snapshot.document.definitions[0].clone(),
            state: state.clone(),
        });
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("parts-name-mounted-regression");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(seed);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        root.query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let pad_id = pad_id_input();
        type_value(&pad_id, "");
        settle().await;
        let _ = courtyard_width_input().focus();
        settle().await;
        assert!(root.query_selector("[role='alert']").unwrap().is_some());
        type_value(&courtyard_width_input(), "12");
        settle().await;

        let mut controls = state.borrow().as_ref().unwrap().clone();
        controls
            .selection
            .set(Some((scope.clone(), "second".into())));
        controls
            .definition
            .set(snapshot.document.definitions[1].clone());
        settle().await;

        assert_eq!(courtyard_width_input().value(), "10");
        assert!(root.query_selector("[role='alert']").unwrap().is_none());

        // Accepted sibling fields synchronize independently. A refreshed
        // height must not discard the local width draft.
        type_value(&courtyard_width_input(), "12");
        settle().await;
        let mut refreshed_second = snapshot.document.definitions[1].clone();
        refreshed_second.courtyard = vec![
            boardstudio_core::model::Vec2 { x: -5.0, y: -4.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: -4.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: 4.0 },
            boardstudio_core::model::Vec2 { x: -5.0, y: 4.0 },
        ];
        controls.definition.set(refreshed_second.clone());
        settle().await;
        assert_eq!(courtyard_width_input().value(), "12");
        assert_eq!(courtyard_height_input().value(), "8");

        // A different definition may reuse the same pad ID. Its local draft
        // must reset when the owner changes, even though the pad identity is
        // otherwise identical.
        type_value(&pad_x_input(), "9");
        settle().await;
        let _ = runtime.take_definition_name_test_event();
        let mut refreshed_second_pad = refreshed_second;
        refreshed_second_pad.pads[0].at.y = 4.0;
        controls.definition.set(refreshed_second_pad);
        settle().await;
        assert_eq!(pad_x_input().value(), "9");
        assert_eq!(pad_y_input().value(), "4");

        controls
            .selection
            .set(Some((scope.clone(), "first".into())));
        controls
            .definition
            .set(snapshot.document.definitions[0].clone());
        settle().await;
        assert_eq!(courtyard_width_input().value(), "10");
        assert_eq!(pad_x_input().value(), "0");
        let _ = courtyard_width_input().blur();
        let _ = pad_x_input().blur();
        settle().await;
        assert!(
            runtime.take_definition_name_test_event().is_none(),
            "dirty values owned by the prior definition cannot commit into the next owner"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_unchanged_blur_does_not_rewrite_empty_courtyard_or_create_history() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        document.definitions = vec![definition("selected", "Empty courtyard")];
        let runtime = Runtime::new().unwrap();
        let (session, _core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        runtime.set_definition_name_test_state(snapshot.clone(), scope.clone());
        let state = Rc::new(RefCell::new(None));
        let seed = Rc::new(Seed {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            selection: Some((scope.clone(), "selected".into())),
            definition: snapshot.document.definitions[0].clone(),
            state,
        });
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("parts-name-mounted-regression");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(seed);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        root.query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let width = courtyard_width_input();
        assert_eq!(width.value(), "10");
        let _ = width.focus();
        let _ = width.blur();
        settle().await;
        assert!(
            runtime.take_definition_name_test_event().is_none(),
            "unchanged DraftInput blur must not turn an empty courtyard into an authored rectangle"
        );
        assert!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .definitions[0]
                .courtyard
                .is_empty()
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_pad_id_action_remaps_every_matching_instance_through_the_runtime_edit() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        let mut definition = definition("selected", "Footprint");
        definition.pads = serde_json::from_value(serde_json::json!([
            {"id":"old","number":"1","at":{"x":0.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"},
            {"id":"keep","number":"2","at":{"x":1.0,"y":0.0},"size":{"x":2.0,"y":2.0},"shape":"circle"}
        ])).unwrap();
        document.definitions = vec![definition.clone()];
        document.parts = serde_json::from_value(serde_json::json!([
            {"id":"instance-a","definitionId":"selected","reference":"U1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
            {"id":"instance-b","definitionId":"selected","reference":"U2","pose":{"at":{"x":1.0,"y":0.0},"rotation":0.0},"side":"front"}
        ])).unwrap();
        document.nets = serde_json::from_value(serde_json::json!([
            {"id":"net-a","name":"A","pins":[{"partId":"instance-a","padId":"old"},{"partId":"instance-b","padId":"old"}]},
            {"id":"net-b","name":"B","pins":[{"partId":"instance-a","padId":"keep"}]}
        ])).unwrap();
        let runtime = Runtime::new().unwrap();
        let (mut session, mut core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        runtime.set_definition_name_test_state(snapshot.clone(), scope.clone());
        let state = Rc::new(RefCell::new(None));
        let seed = Rc::new(Seed {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            selection: Some((scope.clone(), "selected".into())),
            definition: snapshot.document.definitions[0].clone(),
            state: state.clone(),
        });
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("parts-name-mounted-regression");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(seed);
        dom.provide_root_context(runtime.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        root.query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        let field = pad_id_input();
        type_value(&field, "renamed");
        let _ = field.blur();
        settle().await;

        let event = runtime
            .take_definition_name_test_event()
            .expect("mounted definition field submits one Runtime edit");
        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        let accepted = session.read_model().accepted.as_ref().unwrap();
        assert_eq!(accepted.document.definitions[0].pads[0].id, "renamed");
        assert_eq!(accepted.document.nets[0].pins[0].pad_id, "renamed");
        assert_eq!(accepted.document.nets[0].pins[1].pad_id, "renamed");
        assert_eq!(accepted.document.nets[1].pins[0].pad_id, "keep");

        let undo_effects = session.submit(AppEvent::Undo {
            operation_id: OperationId(81),
        });
        advance(&mut session, &mut core, undo_effects);
        assert_eq!(
            session
                .read_model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .definitions[0]
                .pads[0]
                .id,
            "old"
        );
        root.remove();
    }

    fn type_value(input: &HtmlInputElement, value: &str) {
        let _ = input.focus();
        input.set_value(value);
        let event = DomEvent::new("input").unwrap();
        event.init_event_with_bubbles_and_cancelable("input", true, true);
        input.dispatch_event(&event).unwrap();
    }

    #[wasm_bindgen_test]
    async fn mounted_name_commit_uses_refreshed_runtime_capture_and_definition_owner_defaults() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        document.definitions = vec![
            definition("selected", "Original name"),
            definition("other", "Other definition"),
        ];
        let runtime = Runtime::new().unwrap();
        let (mut session, mut core) = open_document(document);
        let snapshot = session.read_model().accepted.as_ref().unwrap().clone();
        let scope = session.scope();
        assert!(
            scope.is_some(),
            "the mounted editor uses the accepting Session scope"
        );
        runtime.set_definition_name_test_state(snapshot.clone(), scope.clone());
        let state = Rc::new(RefCell::new(None));
        let seed = Rc::new(Seed {
            snapshot: snapshot.clone(),
            scope: scope.clone(),
            selection: Some((scope.clone(), "selected".into())),
            definition: snapshot.document.definitions[0].clone(),
            state: state.clone(),
        });
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        root.set_id("parts-name-mounted-regression");
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap()
            .append_child(&root)
            .unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(seed);
        dom.provide_root_context(runtime);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        let field = input();
        type_value(&field, "Dirty name draft");
        settle().await;
        assert_eq!(input().value(), "Dirty name draft");

        let mut controls = state.borrow().as_ref().unwrap().clone();
        let initial_revision = snapshot.document.revision;
        let mut unrelated = snapshot.document.as_ref().clone();
        unrelated
            .parameters
            .insert("independent".into(), serde_json::json!(42));
        let unrelated_event = AppEvent::Edit {
            operation_id: controls.runtime.operation(),
            command: EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "parts-name-unrelated-edit".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["unrelated".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated),
                },
            },
        };
        let effects = session.submit(unrelated_event);
        advance(&mut session, &mut core, effects);
        let latest = session.read_model().accepted.as_ref().unwrap().clone();
        assert!(latest.document.revision > initial_revision);
        controls
            .runtime
            .set_definition_name_test_state(latest.clone(), scope.clone());
        controls.snapshot.set(latest.clone());
        settle().await;
        assert_eq!(
            input().value(),
            "Dirty name draft",
            "an unrelated accepted revision must refresh admission capture without clearing the local field draft"
        );

        let _ = input().blur();
        let event = controls
            .runtime
            .take_definition_name_test_event()
            .expect("the mounted blur callback submits through Runtime");
        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        let committed = session.read_model().accepted.as_ref().unwrap().clone();
        assert_eq!(
            committed.document.definitions[0].name, "Dirty name draft",
            "the mounted blur must submit using the refreshed accepted capture"
        );
        assert_eq!(
            committed.document.parameters.get("independent"),
            Some(&serde_json::json!(42)),
            "the production commit must preserve the unrelated accepted edit"
        );

        let mut latest = committed;
        let mut renamed = latest.document.as_ref().clone();
        renamed.definitions[0].name = "Accepted external name".into();
        let rename_event = AppEvent::Edit {
            operation_id: controls.runtime.operation(),
            command: EditCommand {
                base_revision: latest.document.revision,
                transaction_id: "parts-name-external-rename".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["selected".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(renamed),
                },
            },
        };
        let effects = session.submit(rename_event);
        advance(&mut session, &mut core, effects);
        latest = session.read_model().accepted.as_ref().unwrap().clone();
        controls
            .runtime
            .set_definition_name_test_state(latest.clone(), scope.clone());
        controls.snapshot.set(latest.clone());
        controls
            .definition
            .set(latest.document.definitions[0].clone());
        settle().await;
        assert_eq!(
            input().value(),
            "Accepted external name",
            "an accepted Name change must synchronize the draft"
        );

        assert!(section_is_open());
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#parts-name-mounted-regression details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(!section_is_open());

        controls
            .selection
            .set(Some((scope.clone(), "other".into())));
        controls
            .definition
            .set(latest.document.definitions[1].clone());
        settle().await;
        assert_eq!(input().value(), "Other definition");
        assert!(
            section_is_open(),
            "a newly selected empty definition uses its own default-open state"
        );

        let changed_scope = Some(Scope {
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            board_id: "another-board".into(),
            instance_id: None,
        });
        controls.scope.set(changed_scope.clone());
        controls
            .selection
            .set(Some((changed_scope, "selected".into())));
        controls
            .definition
            .set(definition("selected", "Scoped target"));
        settle().await;
        assert_eq!(input().value(), "Scoped target");
        root.remove();
    }
}
