//! Parts-private admission and resolver construction for one definition-name commit.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::model::{EditOperation, PartDefinition};

use crate::parts_custom_definition::{
    DEFINITION_GONE, DefinitionPanelCapture, GENERATOR_LOCKED, replacement_commit,
};

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::definition_name_resolver;
    use crate::parts_custom_definition::DefinitionPanelCapture;
    use crate::parts_custom_definition::ui::{apply_text_settlement, settle_ticket};
    use crate::runtime::Runtime;
    use boardstudio_application::{AcceptedSnapshot, Scope};
    use boardstudio_core::model::PartDefinition;
    use boardstudio_web_runtime::edit_ticket::EditTicket;
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
        let mut error = use_signal(String::new);
        let mut pending = use_signal(|| None::<EditTicket>);
        // Subscribe to the workspace's runtime-change version: outcomes settle outside
        // Dioxus (Core replies, saves), so this read is what wakes the settle pass below
        // even when the accepted document did not change (for example a failed save).
        let version = use_context::<Signal<u64>>();
        let _ = version();
        let pad_count = definition.pads.len();
        let mut section_open = use_signal(|| pad_count == 0);
        let mut section_chosen = use_signal(|| false);
        let section_owner = (scope.clone(), definition.id.clone());
        let capture = DefinitionPanelCapture::new(&snapshot, scope.clone(), &definition);
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
        use_effect(use_reactive((&draft_identity,), {
            let definition = definition.clone();
            move |(_identity,)| {
                draft.set(definition.name.clone());
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

        // Settle the pending name edit before rendering: pending keeps the draft, a
        // failure restores the accepted value with the message inline, and a landed or
        // retired ticket drops so the field follows the accepted document again. The
        // editor outlives selection changes, so owner liveness is a real answer.
        {
            let model = runtime.model();
            let owner_live = model.accepted.as_ref().is_some_and(|current| {
                capture.owner_is_live(current, runtime.scope(), selection())
            });
            let mut edits = pending.peek().clone();
            if let Some(settlement) = settle_ticket(&mut edits, owner_live) {
                pending.set(edits);
                apply_text_settlement(settlement, definition.name.as_str(), &mut draft, &mut error);
            }
        }

        let accepted_name = definition.name.clone();
        let on_blur = {
            let runtime = runtime.clone();
            let capture = capture.clone();
            let selection = selection;
            let accepted_name = accepted_name.clone();
            move |_| {
                let model = runtime.model();
                let Some(current) = model.accepted else {
                    return;
                };
                if !capture.owner_is_live(&current, runtime.scope(), selection()) {
                    return;
                }
                let name = draft();
                if name == accepted_name {
                    return;
                }
                let resolver = definition_name_resolver(capture.definition_id.clone(), name);
                let ticket = EditTicket::begin(
                    &runtime,
                    "parts-definition-name",
                    Some("part definition".into()),
                    resolver,
                );
                pending.set(Some(ticket));
                error.set(String::new());
            }
        };
        let on_keydown = {
            let mut keydown_draft = draft;
            let value = accepted_name;
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
                        if !error().is_empty() {
                            p { class: "m1-definition-error", role: "alert", "{error()}" }
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

/// Resolve a definition-name commit against the accepted document at execution: retire
/// when the definition has gone or is generated, resolve `Unchanged` when the accepted
/// name already equals the committed one, and otherwise submit a replacement built from
/// the accepted document, so a rename queued behind another edit never reverts it.
pub fn definition_name_resolver(definition_id: String, name: String) -> EditResolver {
    EditResolver::new(
        "parts-definition-name",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(existing) = document
                .definitions
                .iter()
                .find(|definition| definition.id == definition_id)
            else {
                return Resolution::Retire(DEFINITION_GONE.into());
            };
            if existing.generator.is_some() {
                return Resolution::Retire(GENERATOR_LOCKED.into());
            }
            if existing.name == name {
                return Resolution::Unchanged;
            }
            let mut replacement = document.as_ref().clone();
            let Some(definition) = replacement
                .definitions
                .iter_mut()
                .find(|definition| definition.id == definition_id)
            else {
                return Resolution::Retire(DEFINITION_GONE.into());
            };
            definition.name = name.clone();
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![definition_id.clone()],
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{
        Completion, Durability, Effect, Event, OperationId, SaveResult, Session, TerminalOutcome,
    };
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

    /// Drive effects to completion, collecting every settlement Session reported.
    fn advance_collecting(
        session: &mut Session,
        core: &mut CoreEngine,
        initial: Vec<Effect>,
    ) -> Vec<TerminalOutcome> {
        let mut pending = initial;
        let mut settlements = Vec::new();
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
                Effect::Settled { outcome, .. } => settlements.push(outcome),
                _ => {}
            }
        }
        settlements
    }

    fn submit_name_edit(
        session: &mut Session,
        core: &mut CoreEngine,
        operation: u64,
        definition_id: &str,
        name: &str,
    ) -> Vec<TerminalOutcome> {
        let effects = session.submit(Event::ResolveEdit {
            operation_id: OperationId(operation),
            label: "parts-definition-name".into(),
            resolver: definition_name_resolver(definition_id.into(), name.into()),
        });
        advance_collecting(session, core, effects)
    }

    fn selected_name(document: &ProjectDoc) -> &str {
        document
            .definitions
            .iter()
            .find(|definition| definition.id == "selected")
            .unwrap()
            .name
            .as_str()
    }

    #[test]
    fn production_name_edit_commits_one_field_and_round_trips_through_session_history() {
        let original = document();
        let (mut session, mut core) = open_document(original.clone());
        assert_eq!(
            submit_name_edit(&mut session, &mut core, 2, "selected", "Renamed device"),
            vec![TerminalOutcome::Completed],
            "a changed name lands as one accepted edit"
        );
        let accepted = session.read_model().accepted.as_ref().unwrap();
        let mut expected_renamed = original.clone();
        expected_renamed.definitions[0].name = "Renamed device".into();
        assert_same_content_ignoring_revision(&accepted.document, &expected_renamed);
        assert_eq!(
            session.read_model().durability,
            Durability::Saved {
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
    fn admission_keeps_departed_owners_out_but_newer_revisions_queue_freely() {
        let (session, _core) = open_document(document());
        let snapshot = session.read_model().accepted.clone().unwrap();
        let scope = session.scope();
        let selected = Some((scope.clone(), "selected".into()));
        let capture = DefinitionPanelCapture::new(
            &snapshot,
            scope.clone(),
            &snapshot.document.definitions[0],
        );

        assert!(capture.owner_is_live(&snapshot, scope.clone(), selected.clone()));
        assert!(!capture.owner_is_live(
            &snapshot,
            scope.clone(),
            Some((scope.clone(), "other".into()))
        ));
        let other_scope = scope.as_ref().map(|scope| {
            let mut changed = scope.clone();
            changed.instance_id = Some("other-instance".into());
            changed
        });
        assert!(!capture.owner_is_live(&snapshot, other_scope, selected.clone()));

        let mut changed_identity_doc = snapshot.document.as_ref().clone();
        changed_identity_doc.id = "another-project".into();
        let changed_identity_snapshot = AcceptedSnapshot {
            document: std::sync::Arc::new(changed_identity_doc),
            ..snapshot.clone()
        };
        assert!(!capture.owner_is_live(&changed_identity_snapshot, scope.clone(), selected));

        // A newer accepted revision is not a departed owner: field edits queue freely
        // and resolve against the document accepted when they run (ADR-0005).
        let mut newer_doc = snapshot.document.as_ref().clone();
        newer_doc.revision += 1;
        let newer = AcceptedSnapshot {
            token: boardstudio_application::SnapshotToken(snapshot.token.0 + 1),
            document: std::sync::Arc::new(newer_doc),
            ..snapshot.clone()
        };
        assert!(capture.owner_is_live(
            &newer,
            session.scope(),
            Some((session.scope().clone(), "selected".into()))
        ));
    }

    #[test]
    fn generator_definitions_and_vanished_targets_retire_a_name_commit() {
        let mut generator_doc = document();
        generator_doc.definitions[0].generator = Some(boardstudio_core::model::PartGenerator {
            source: "generator/source".into(),
            version: "1".into(),
            parameters: Default::default(),
        });
        let (mut session, mut core) = open_document(generator_doc);
        assert_eq!(
            submit_name_edit(&mut session, &mut core, 5, "selected", "Generator name"),
            vec![TerminalOutcome::Rejected(GENERATOR_LOCKED.into())]
        );
        assert_eq!(
            selected_name(&session.read_model().accepted.clone().unwrap().document),
            "Original name"
        );

        let (mut session, mut core) = open_document(document());
        assert_eq!(
            submit_name_edit(&mut session, &mut core, 6, "vanished", "No target"),
            vec![TerminalOutcome::Rejected(DEFINITION_GONE.into())]
        );
    }

    #[test]
    fn an_unchanged_name_commit_lands_without_moving_the_revision() {
        let (mut session, mut core) = open_document(document());
        let before = session.read_model().accepted.clone().unwrap();
        assert_eq!(
            submit_name_edit(&mut session, &mut core, 7, "selected", "Original name"),
            vec![TerminalOutcome::Completed]
        );
        let after = session.read_model().accepted.clone().unwrap();
        assert_eq!(after.document.revision, before.document.revision);
        assert_eq!(after.token, before.token);
    }

    #[test]
    fn a_name_commit_resolves_against_the_latest_accepted_document() {
        let original = document();
        let (mut session, mut core) = open_document(original);
        let initial = session.read_model().accepted.as_ref().unwrap().clone();

        let mut unrelated_document = initial.document.as_ref().clone();
        unrelated_document
            .parameters
            .insert("unrelated-edit".into(), serde_json::json!(true));
        let unrelated = Event::Edit {
            operation_id: OperationId(20),
            command: boardstudio_core::model::EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "parts-name-unrelated-edit".into(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec!["unrelated".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated_document),
                },
            },
        };
        let effects = session.submit(unrelated);
        advance(&mut session, &mut core, effects);

        assert_eq!(
            submit_name_edit(&mut session, &mut core, 21, "selected", "Dirty name draft"),
            vec![TerminalOutcome::Completed]
        );
        let accepted = session.read_model().accepted.as_ref().unwrap();
        assert_eq!(
            accepted.document.parameters.get("unrelated-edit"),
            Some(&serde_json::json!(true)),
            "the name commit must retain the edit accepted while its local draft was dirty"
        );
        assert_eq!(selected_name(&accepted.document), "Dirty name draft");
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::runtime::{Runtime, project_name_test_support as support};
    use boardstudio_application::{Event as AppEvent, Scope};
    use boardstudio_core::model::ProjectDoc;
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
        version: Signal<u64>,
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
        let version = use_signal(|| 0u64);
        use_context_provider(|| version);
        *seed.state.borrow_mut() = Some(State {
            snapshot,
            scope,
            selection,
            definition,
            version,
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

    /// Open `document` through the real Session and Core on a fresh Runtime and return what the
    /// mounted editor is given: the accepted snapshot and its scope.
    async fn opened(document: ProjectDoc) -> (Rc<Runtime>, AcceptedSnapshot, Option<Scope>) {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let snapshot = runtime
            .model()
            .accepted
            .expect("the opened document is accepted");
        let scope = runtime.scope();
        (runtime, snapshot, scope)
    }

    fn input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Definition name']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn section_is_open(root: &web_sys::Element) -> bool {
        root.query_selector("details")
            .unwrap()
            .unwrap()
            .has_attribute("open")
    }

    fn pad_id_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Pad 1 ID']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_number_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Pad 1 number']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn courtyard_width_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Courtyard width']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn courtyard_height_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Courtyard height']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_x_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Pad 1 X']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn pad_y_input(root: &web_sys::Element) -> HtmlInputElement {
        root.query_selector("input[aria-label='Pad 1 Y']")
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
        let (runtime, snapshot, scope) = opened(document).await;
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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
        let pad_id = pad_id_input(&root);
        type_value(&pad_id, "");
        settle().await;
        let _ = input(&root).focus();
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

        assert_eq!(pad_id_input(&root).value(), "shared-pad");
        assert!(root.query_selector("[role='alert']").unwrap().is_none());

        // A real accepted pad-ID and Y-coordinate update must preserve a dirty
        // number draft in the same row. This catches rows keyed by mutable ID.
        assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("7"));
        settle().await;
        assert_eq!(pad_number_input(&root).value(), "7");
        let mut refreshed_doc = runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref()
            .clone();
        refreshed_doc.definitions[1].pads[0].id = "renamed-pad".into();
        refreshed_doc.definitions[1].pads[0].at.y = 4.0;
        let refreshed = support::replace_document(
            &runtime,
            "parts04-mounted-refresh-911",
            "second",
            refreshed_doc,
        )
        .await;
        controls.snapshot.set(refreshed.clone());
        controls
            .definition
            .set(refreshed.document.definitions[1].clone());
        settle().await;
        assert_eq!(pad_y_input(&root).value(), "4");
        assert_eq!(pad_id_input(&root).value(), "renamed-pad");
        assert_eq!(pad_number_input(&root).value(), "7");
        let _ = pad_number_input(&root).focus();
        settle().await;
        assert!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .is_same_node(Some(&pad_number_input(&root)))
        );
        let _ = pad_number_input(&root).blur();
        settle().await;
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.as_ref().unwrap().clone();
        assert_eq!(
            accepted.document.revision,
            refreshed.document.revision + 1,
            "dirty row draft submits one edit against the accepted renamed pad"
        );
        assert_eq!(accepted.document.definitions[1].pads[0].id, "renamed-pad");
        assert_eq!(accepted.document.definitions[1].pads[0].number, "7");
        assert_eq!(accepted.document.definitions[1].pads[0].at.y, 4.0);

        // Deleting a row must retire its draft, even when the next pad has
        // identical accepted scalar values and moves into that row's index.
        let mut with_survivor = accepted.document.as_ref().clone();
        let mut survivor = with_survivor.definitions[1].pads[0].clone();
        survivor.id = "surviving-pad".into();
        with_survivor.definitions[1].pads.push(survivor);
        let expanded = support::replace_document(
            &runtime,
            "parts04-mounted-refresh-912",
            "second",
            with_survivor,
        )
        .await;
        controls.snapshot.set(expanded.clone());
        controls
            .definition
            .set(expanded.document.definitions[1].clone());
        settle().await;
        assert!(crate::parts_custom_definition::set_pad_number_draft_for_test("9"));
        settle().await;
        assert_eq!(pad_number_input(&root).value(), "9");
        root.query_selector(".m1-definition-remove-pad")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.as_ref().unwrap().clone();
        assert_eq!(
            accepted.document.revision,
            expanded.document.revision + 1,
            "production Remove pad emits one scoped edit"
        );
        assert_eq!(accepted.document.definitions[1].pads.len(), 1);
        assert_eq!(accepted.document.definitions[1].pads[0].id, "surviving-pad");
        controls.snapshot.set(accepted.clone());
        controls
            .definition
            .set(accepted.document.definitions[1].clone());
        settle().await;
        assert_eq!(pad_id_input(&root).value(), "surviving-pad");
        assert_eq!(
            pad_number_input(&root).value(),
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
        settle().await;
        assert_eq!(pad_id_input(&root).value(), "shared-pad");
        assert_eq!(pad_number_input(&root).value(), "1");
        let _ = pad_number_input(&root).blur();
        settle().await;
        assert!(
            support::take_held_effects(&runtime).is_empty(),
            "dirty values owned by the prior definition cannot commit into the next owner"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            accepted.document.revision
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
        let (runtime, snapshot, scope) = opened(document).await;
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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
        let pad_id = pad_id_input(&root);
        type_value(&pad_id, "");
        settle().await;
        let _ = courtyard_width_input(&root).focus();
        settle().await;
        assert!(root.query_selector("[role='alert']").unwrap().is_some());
        type_value(&courtyard_width_input(&root), "12");
        settle().await;

        let mut controls = state.borrow().as_ref().unwrap().clone();
        controls
            .selection
            .set(Some((scope.clone(), "second".into())));
        controls
            .definition
            .set(snapshot.document.definitions[1].clone());
        settle().await;

        assert_eq!(courtyard_width_input(&root).value(), "10");
        assert!(root.query_selector("[role='alert']").unwrap().is_none());

        // Accepted sibling fields synchronize independently. A refreshed
        // height must not discard the local width draft.
        type_value(&courtyard_width_input(&root), "12");
        settle().await;
        let mut refreshed_doc = runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref()
            .clone();
        refreshed_doc.definitions[1].courtyard = vec![
            boardstudio_core::model::Vec2 { x: -5.0, y: -4.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: -4.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: 4.0 },
            boardstudio_core::model::Vec2 { x: -5.0, y: 4.0 },
        ];
        let refreshed = support::replace_document(
            &runtime,
            "parts-name-mounted-height-refresh",
            "second",
            refreshed_doc,
        )
        .await;
        controls.snapshot.set(refreshed.clone());
        controls
            .definition
            .set(refreshed.document.definitions[1].clone());
        settle().await;
        assert_eq!(courtyard_width_input(&root).value(), "12");
        assert_eq!(courtyard_height_input(&root).value(), "8");

        // A different definition may reuse the same pad ID. Its local draft
        // must reset when the owner changes, even though the pad identity is
        // otherwise identical.
        type_value(&pad_x_input(&root), "9");
        settle().await;
        // Anything the focus change submitted lands before the next accepted refresh, so the
        // refresh below is built on the Session's latest accepted document.
        support::run_pending(&runtime).await;
        let latest = runtime.model().accepted.as_ref().unwrap().clone();
        let mut refreshed_pad_doc = latest.document.as_ref().clone();
        refreshed_pad_doc.definitions[1].pads[0].at.y = 4.0;
        let refreshed_pad = support::replace_document(
            &runtime,
            "parts-name-mounted-pad-refresh",
            "second",
            refreshed_pad_doc,
        )
        .await;
        controls.snapshot.set(refreshed_pad.clone());
        controls
            .definition
            .set(refreshed_pad.document.definitions[1].clone());
        settle().await;
        assert_eq!(pad_x_input(&root).value(), "9");
        assert_eq!(pad_y_input(&root).value(), "4");

        controls
            .selection
            .set(Some((scope.clone(), "first".into())));
        controls
            .definition
            .set(refreshed_pad.document.definitions[0].clone());
        settle().await;
        assert_eq!(courtyard_width_input(&root).value(), "10");
        assert_eq!(pad_x_input(&root).value(), "0");
        let _ = courtyard_width_input(&root).blur();
        let _ = pad_x_input(&root).blur();
        settle().await;
        assert!(
            support::take_held_effects(&runtime).is_empty(),
            "dirty values owned by the prior definition cannot commit into the next owner"
        );
        assert_eq!(
            runtime.model().accepted.unwrap().document.revision,
            refreshed_pad.document.revision
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_unchanged_blur_does_not_rewrite_empty_courtyard_or_create_history() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        document.definitions = vec![definition("selected", "Empty courtyard")];
        let (runtime, snapshot, scope) = opened(document).await;
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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
        let width = courtyard_width_input(&root);
        assert_eq!(width.value(), "10");
        let _ = width.focus();
        let _ = width.blur();
        settle().await;
        assert!(
            support::take_held_effects(&runtime).is_empty(),
            "unchanged DraftInput blur must not turn an empty courtyard into an authored rectangle"
        );
        let accepted = runtime.model().accepted.unwrap();
        assert!(accepted.document.definitions[0].courtyard.is_empty());
        assert_eq!(
            accepted.document.revision, snapshot.document.revision,
            "an unchanged blur creates no history"
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
        let (runtime, snapshot, scope) = opened(document).await;
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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
        let field = pad_id_input(&root);
        type_value(&field, "renamed");
        let _ = field.blur();
        settle().await;

        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.as_ref().unwrap().clone();
        assert_eq!(
            accepted.document.revision,
            snapshot.document.revision + 1,
            "mounted definition field submits one Runtime edit"
        );
        assert_eq!(accepted.document.definitions[0].pads[0].id, "renamed");
        assert_eq!(accepted.document.nets[0].pins[0].pad_id, "renamed");
        assert_eq!(accepted.document.nets[0].pins[1].pad_id, "renamed");
        assert_eq!(accepted.document.nets[1].pins[0].pad_id, "keep");

        runtime.submit(AppEvent::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime
                .model()
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

    fn enter_keydown(input: &HtmlInputElement) {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("Enter");
        init.set_bubbles(true);
        let keydown = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
            .expect("keydown event");
        input.dispatch_event(&keydown).unwrap();
    }

    thread_local! {
        static NEXT_ROOT: std::cell::Cell<u64> = const { std::cell::Cell::new(1) };
    }

    /// Mount the definition panel over `document` under its own root element, so one
    /// test's leftovers cannot satisfy another test's selectors.
    async fn mount_panel(
        document: ProjectDoc,
        open_section: bool,
    ) -> (
        Rc<Runtime>,
        AcceptedSnapshot,
        Option<Scope>,
        Rc<RefCell<Option<State>>>,
        web_sys::Element,
    ) {
        let (runtime, snapshot, scope) = opened(document).await;
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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
        if open_section {
            root.query_selector("details summary")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click();
            settle().await;
        }
        (runtime, snapshot, scope, state, root)
    }

    /// Drive a released gate's continuation and every pending effect to completion, then
    /// push the accepted snapshot and definition back into the mounted host so the panel
    /// settles its tickets against the final document.
    async fn accept_edits(runtime: &Rc<Runtime>, controls: &State) {
        settle().await;
        support::run_pending(runtime).await;
        refresh_host(runtime, controls).await;
    }

    async fn refresh_host(runtime: &Rc<Runtime>, controls: &State) {
        let accepted = runtime.model().accepted.clone().unwrap();
        let mut snapshot = controls.snapshot;
        let mut definition_signal = controls.definition;
        snapshot.set(accepted.clone());
        if let Some(definition) = accepted
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == "selected")
            .cloned()
        {
            definition_signal.set(definition);
        }
        // Wake the panel's settle pass the way the production workspace's runtime-change
        // version does, even when the accepted content did not change.
        let mut version = controls.version;
        version += 1;
        settle().await;
    }

    fn definition_bounds(document: &ProjectDoc) -> (f64, f64) {
        let (width, height, _) =
            crate::parts_custom_definition::courtyard_bounds(&document.definitions[0].courtyard);
        (width, height)
    }

    fn courtyard_document(name: &str) -> ProjectDoc {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        let mut selected = definition("selected", name);
        selected.courtyard = vec![
            boardstudio_core::model::Vec2 { x: -5.0, y: -3.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: -3.0 },
            boardstudio_core::model::Vec2 { x: 5.0, y: 3.0 },
            boardstudio_core::model::Vec2 { x: -5.0, y: 3.0 },
        ];
        document.definitions = vec![selected];
        document
    }

    #[wasm_bindgen_test]
    async fn mounted_dirty_courtyard_width_commits_when_focus_moves_away() {
        // Ticket 07 saw a dirty courtyard width submit nothing when focus moved away.
        // The cause was the harness: a pad-less definition's custom section defaults
        // open, so the old test's summary click closed it and focus/blur no-opped on
        // hidden inputs. With the section open the blur commits and lands.
        let (runtime, snapshot, _scope, state, root) =
            mount_panel(courtyard_document("Courtyard"), false).await;
        let controls = state.borrow().as_ref().unwrap().clone();
        assert!(
            section_is_open(&root),
            "a pad-less definition's custom section defaults open"
        );

        type_value(&courtyard_width_input(&root), "12");
        let _ = courtyard_width_input(&root).blur();
        accept_edits(&runtime, &controls).await;

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.revision,
            snapshot.document.revision + 1,
            "a dirty courtyard width must commit when focus moves away"
        );
        assert_eq!(definition_bounds(&accepted.document), (12.0, 6.0));
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_rapid_width_height_keeps_both_and_undo_reverts_only_height() {
        let (runtime, snapshot, _scope, state, root) =
            mount_panel(courtyard_document("Courtyard"), false).await;
        let controls = state.borrow().as_ref().unwrap().clone();
        let width = courtyard_width_input(&root);
        let height = courtyard_height_input(&root);

        // Hold the width edit's Core reply so the height edit queues behind it, exactly
        // as rapid Tab-entry does when the engine is busy.
        let (entered, release) = support::gate_next_core_reply(&runtime);
        type_value(&width, "12");
        let _ = height.focus();
        settle().await;
        support::drive_pending(&runtime);
        entered
            .await
            .expect("the width edit reached the in-process Core");

        type_value(&height, "8");
        enter_keydown(&height);
        settle().await;

        release.send(()).expect("release the held width reply");
        accept_edits(&runtime, &controls).await;

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.revision,
            snapshot.document.revision + 2,
            "both queued courtyard edits land"
        );
        assert_eq!(
            definition_bounds(&accepted.document),
            (12.0, 8.0),
            "width, Tab, height keeps both values"
        );
        assert_eq!(courtyard_width_input(&root).value(), "12");
        assert_eq!(courtyard_height_input(&root).value(), "8");

        runtime.submit(AppEvent::Undo {
            operation_id: runtime.operation(),
        });
        accept_edits(&runtime, &controls).await;
        let undone = runtime.model().accepted.unwrap();
        assert_eq!(
            definition_bounds(&undone.document),
            (12.0, 6.0),
            "one Undo reverts only the height change"
        );
        assert_eq!(courtyard_height_input(&root).value(), "6");

        runtime.submit(AppEvent::Undo {
            operation_id: runtime.operation(),
        });
        accept_edits(&runtime, &controls).await;
        let undone_again = runtime.model().accepted.unwrap();
        assert_eq!(
            definition_bounds(&undone_again.document),
            (10.0, 6.0),
            "a second Undo reverts the width change: the edits undo in order"
        );
        assert_eq!(courtyard_width_input(&root).value(), "10");
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_rename_then_field_edit_queued_behind_it_both_survive() {
        let (runtime, snapshot, _scope, state, root) =
            mount_panel(courtyard_document("Original name"), false).await;
        let controls = state.borrow().as_ref().unwrap().clone();

        let (entered, release) = support::gate_next_core_reply(&runtime);
        type_value(&input(&root), "Renamed");
        let _ = courtyard_width_input(&root).focus();
        settle().await;
        support::drive_pending(&runtime);
        entered
            .await
            .expect("the rename reached the in-process Core");

        type_value(&courtyard_width_input(&root), "12");
        let _ = courtyard_width_input(&root).blur();
        settle().await;

        release.send(()).expect("release the held rename reply");
        accept_edits(&runtime, &controls).await;

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.revision,
            snapshot.document.revision + 2,
            "the rename and the queued width edit both land"
        );
        assert_eq!(accepted.document.definitions[0].name, "Renamed");
        assert_eq!(definition_bounds(&accepted.document), (12.0, 6.0));

        runtime.submit(AppEvent::Undo {
            operation_id: runtime.operation(),
        });
        accept_edits(&runtime, &controls).await;
        let undone = runtime.model().accepted.unwrap();
        assert_eq!(
            undone.document.definitions[0].name, "Renamed",
            "one Undo reverts only the width change; the rename survives"
        );
        assert_eq!(definition_bounds(&undone.document), (10.0, 6.0));
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_failed_save_returns_the_field_to_the_accepted_value_with_a_message() {
        let (runtime, snapshot, _scope, state, root) =
            mount_panel(courtyard_document("Courtyard"), false).await;
        let controls = state.borrow().as_ref().unwrap().clone();
        support::fail_next_persist(&runtime, "injected durable write failure");

        type_value(&courtyard_width_input(&root), "12");
        let _ = courtyard_width_input(&root).blur();
        settle().await;
        support::run_pending(&runtime).await;
        refresh_host(&runtime, &controls).await;

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.revision, snapshot.document.revision,
            "a failed save does not move the accepted document"
        );
        assert_eq!(
            courtyard_width_input(&root).value(),
            "10",
            "the failed field shows the accepted value again"
        );
        let alert = root
            .query_selector("[role='alert']")
            .unwrap()
            .expect("the failure is explained inline")
            .text_content()
            .unwrap();
        assert!(
            alert.contains("did not save") && alert.contains("injected durable write failure"),
            "the inline message names the failure: {alert}"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_definition_deletion_retires_the_queued_field_edit_with_a_reason() {
        let (runtime, snapshot, _scope, state, root) =
            mount_panel(courtyard_document("Courtyard"), false).await;
        let controls = state.borrow().as_ref().unwrap().clone();

        // Hold a definition deletion's Core reply so the width edit queues behind it.
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut deleted = snapshot.document.as_ref().clone();
        deleted.definitions.clear();
        runtime.submit(AppEvent::Edit {
            operation_id: runtime.operation(),
            command: boardstudio_core::model::EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: "delete-definition-before-queued-edit".into(),
                phase: boardstudio_core::model::EditPhase::Commit,
                target_ids: vec!["selected".into()],
                operation: boardstudio_core::model::EditOperation::ReplaceDocument {
                    document: Box::new(deleted),
                },
            },
        });
        support::drive_pending(&runtime);
        entered
            .await
            .expect("the deletion reached the in-process Core");

        type_value(&courtyard_width_input(&root), "12");
        let _ = courtyard_width_input(&root).blur();
        settle().await;

        release.send(()).expect("release the held deletion reply");
        accept_edits(&runtime, &controls).await;

        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.revision,
            snapshot.document.revision + 1,
            "only the deletion lands; the retired edit moves nothing"
        );
        assert!(
            accepted.document.definitions.is_empty(),
            "the deletion is accepted"
        );
        assert_eq!(
            courtyard_width_input(&root).value(),
            "10",
            "the retired field shows the accepted value again"
        );
        let alert = root
            .query_selector("[role='alert']")
            .unwrap()
            .expect("the retirement is explained inline")
            .text_content()
            .unwrap();
        assert!(
            alert.contains("no longer exists"),
            "the inline message explains why the edit retired: {alert}"
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_name_commit_uses_refreshed_runtime_capture_and_definition_owner_defaults() {
        let mut document = ProjectDoc::empty("parts-name-mounted", "Parts name mounted");
        document.definitions = vec![
            definition("selected", "Original name"),
            definition("other", "Other definition"),
        ];
        let (runtime, snapshot, scope) = opened(document).await;
        assert!(
            scope.is_some(),
            "the mounted editor uses the accepting Session scope"
        );
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
        let root_index = NEXT_ROOT.with(|next| next.replace(next.get() + 1));
        root.set_id(&format!("parts-name-mounted-regression-{root_index}"));
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

        let field = input(&root);
        type_value(&field, "Dirty name draft");
        settle().await;
        assert_eq!(input(&root).value(), "Dirty name draft");

        let mut controls = state.borrow().as_ref().unwrap().clone();
        let initial_revision = snapshot.document.revision;
        let mut unrelated = snapshot.document.as_ref().clone();
        unrelated
            .parameters
            .insert("independent".into(), serde_json::json!(42));
        let latest = support::replace_document(
            &runtime,
            "parts-name-unrelated-edit",
            "unrelated",
            unrelated,
        )
        .await;
        assert!(latest.document.revision > initial_revision);
        controls.snapshot.set(latest.clone());
        settle().await;
        assert_eq!(
            input(&root).value(),
            "Dirty name draft",
            "an unrelated accepted revision must refresh admission capture without clearing the local field draft"
        );

        let _ = input(&root).blur();
        support::run_pending(&runtime).await;
        let committed = runtime.model().accepted.as_ref().unwrap().clone();
        assert_eq!(
            committed.document.revision,
            latest.document.revision + 1,
            "the mounted blur callback submits through Runtime"
        );
        assert_eq!(
            committed.document.definitions[0].name, "Dirty name draft",
            "the mounted blur must submit using the refreshed accepted capture"
        );
        assert_eq!(
            committed.document.parameters.get("independent"),
            Some(&serde_json::json!(42)),
            "the production commit must preserve the unrelated accepted edit"
        );

        let mut renamed = committed.document.as_ref().clone();
        renamed.definitions[0].name = "Accepted external name".into();
        let latest =
            support::replace_document(&runtime, "parts-name-external-rename", "selected", renamed)
                .await;
        controls.snapshot.set(latest.clone());
        controls
            .definition
            .set(latest.document.definitions[0].clone());
        settle().await;
        assert_eq!(
            input(&root).value(),
            "Accepted external name",
            "an accepted Name change must synchronize the draft"
        );

        assert!(section_is_open(&root));
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("details summary")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(!section_is_open(&root));

        controls
            .selection
            .set(Some((scope.clone(), "other".into())));
        controls
            .definition
            .set(latest.document.definitions[1].clone());
        settle().await;
        assert_eq!(input(&root).value(), "Other definition");
        assert!(
            section_is_open(&root),
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
        assert_eq!(input(&root).value(), "Scoped target");
        root.remove();
    }
}
