//! Parts-private custom-component creation through the edit ticket.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_core::model::{EditOperation, PartDefinition};

use crate::parts_custom_definition::replacement_commit;

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{created_definition, new_component_resolver};
    use crate::runtime::Runtime;
    use boardstudio_application::Scope;
    use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
    use dioxus::prelude::*;
    use js_sys::{Date, Function, Reflect};
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue};

    /// One queued create: the identity base the resolver derives the definition from
    /// and the identities that already existed when the click was admitted.
    #[derive(Clone)]
    struct ComponentCreation {
        base: String,
        known: Vec<String>,
        scope: Scope,
        view_generation: u64,
        scope_generation: u64,
        ticket: EditTicket,
    }

    /// The Parts parent mounts this action outside its catalogue loading/error branches so
    /// module-source availability never controls whether a project definition can be created.
    #[component]
    pub fn NewCustomComponentAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<&'static str>,
        query: Signal<String>,
        selected: Signal<Option<(Option<Scope>, String)>>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_context::<Signal<u64>>();
        let pending = use_signal(|| None::<ComponentCreation>);
        let mut error = use_signal(|| None::<(Scope, u64, u64, String)>);
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
            let view_generation = view_generation;
            let scope_generation = scope_generation;
            let workspace = workspace;
            let on_select = on_select.clone();
            move |_| {
                let Some(waiting) = pending.read().clone() else {
                    return;
                };
                let owner_is_live = owner_is_mounted.get()
                    && workspace() == "Parts"
                    && view_generation() == waiting.view_generation
                    && scope_generation() == waiting.scope_generation
                    && runtime.scope().as_ref() == Some(&waiting.scope)
                    && runtime.model().accepted.as_ref().is_some_and(|current| {
                        current.session_epoch == waiting.scope.session_epoch
                            && current.document.id == waiting.scope.document_id
                    });
                match waiting.ticket.settlement(owner_is_live) {
                    Settlement::Pending => {}
                    Settlement::Landed { .. } => {
                        pending.set(None);
                        // Landed means landed: select the created definition by reading the
                        // accepted document at the landing, and only if it still exists.
                        let created = runtime.model().accepted.as_ref().and_then(|current| {
                            created_definition(
                                &waiting.base,
                                &waiting.known,
                                &current.document.definitions,
                            )
                            .map(|definition| definition.id.clone())
                        });
                        if let Some(definition_id) = created {
                            selected.set(Some((Some(waiting.scope.clone()), definition_id)));
                            query.set(String::new());
                            on_select.call(());
                        }
                    }
                    Settlement::Failed { message } => {
                        pending.set(None);
                        error.set(Some((
                            waiting.scope,
                            waiting.view_generation,
                            waiting.scope_generation,
                            message,
                        )));
                    }
                    Settlement::Retired => pending.set(None),
                }
            }
        }));

        let create = {
            let runtime = runtime.clone();
            let mut pending = pending;
            let owner_is_mounted = owner_is_mounted.clone();
            let view_generation = view_generation;
            let scope_generation = scope_generation;
            let workspace = workspace;
            move |_| {
                if pending
                    .read()
                    .as_ref()
                    .is_some_and(|waiting| waiting.ticket.is_pending())
                    || !owner_is_mounted.get()
                    || workspace() != "Parts"
                {
                    return;
                }
                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
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
                let base = format!("ui-{}", browser_identity().unwrap_or_else(date_identity));
                let known = snapshot
                    .document
                    .definitions
                    .iter()
                    .map(|definition| definition.id.clone())
                    .collect::<Vec<_>>();
                error.set(None);
                let ticket = EditTicket::begin(
                    &runtime,
                    "parts-new-component",
                    Some("component".into()),
                    new_component_resolver(base.clone()),
                );
                pending.set(Some(ComponentCreation {
                    base,
                    known,
                    scope: current_scope,
                    view_generation: view_generation(),
                    scope_generation: scope_generation(),
                    ticket,
                }));
            }
        };

        rsx! {
            button {
                class: "m1-parts-create-component",
                type: "button",
                "aria-label": "New custom component",
                disabled: pending
                    .read()
                    .as_ref()
                    .is_some_and(|waiting| waiting.ticket.is_pending()),
                onclick: create,
                "New custom component"
            }
            if let Some((_, _, _, message)) = error().filter(|(owner, view, generation, _)| runtime.scope().as_ref() == Some(owner) && workspace() == "Parts" && view_generation() == *view && scope_generation() == *generation) {
                p { role: "alert", "{message}" }
            }
        }
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
pub use ui::NewCustomComponentAction;

/// Resolve editable project-owned selection from the accepted document even when the
/// module catalogue is still loading or has failed.
pub fn accepted_project_definition(
    snapshot: &AcceptedSnapshot,
    current_scope: &Option<Scope>,
    selection: &Option<(Option<Scope>, String)>,
) -> Option<PartDefinition> {
    let (selected_scope, definition_id) = selection.as_ref()?;
    if selected_scope != current_scope {
        return None;
    }
    snapshot
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == *definition_id)
        .cloned()
}

/// Resolve one "new custom component" click against the accepted document at execution:
/// the definition identity is made unique against the definitions that exist then, and
/// the replacement is cloned from the accepted document, so a create queued behind
/// another edit never reverts it.
pub fn new_component_resolver(base: String) -> EditResolver {
    EditResolver::new("parts-new-component", move |accepted: &AcceptedSnapshot| {
        let document = &accepted.document;
        let definition_id = match non_colliding_id(&base, &document.definitions) {
            Some(id) => id,
            None => {
                return Resolution::Retire(
                    "Could not allocate a unique component identity.".into(),
                );
            }
        };
        let definition =
            match custom_definition(definition_id.clone(), document.definitions.len() + 1) {
                Ok(definition) => definition,
                Err(error) => return Resolution::Retire(error),
            };
        let mut replacement = document.as_ref().clone();
        replacement.definitions.push(definition);
        replacement_commit(
            EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
            vec![definition_id],
        )
    })
}

/// The definition this create landed, read from the accepted document at the landing:
/// the definition whose identity the resolver derived from `base` that did not exist when
/// the click was admitted. Selects only what exists.
pub fn created_definition<'a>(
    base: &str,
    known: &[String],
    definitions: &'a [PartDefinition],
) -> Option<&'a PartDefinition> {
    definitions.iter().rev().find(|definition| {
        identity_derives_from(base, &definition.id) && !known.contains(&definition.id)
    })
}

/// `base` itself or one of its numbered siblings (`base-2`, `base-3`, …), the identities
/// [`non_colliding_id`] derives.
fn identity_derives_from(base: &str, id: &str) -> bool {
    id == base
        || id
            .strip_prefix(&format!("{base}-"))
            .is_some_and(|suffix| suffix.parse::<u64>().is_ok())
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
    (0_u64..)
        .map(|suffix| {
            if suffix == 0 {
                base.to_owned()
            } else {
                format!("{base}-{suffix}")
            }
        })
        .find(|candidate| {
            definitions
                .iter()
                .all(|definition| definition.id != *candidate)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            non_colliding_id("ui-same", &definitions).as_deref(),
            Some("ui-same-2")
        );
    }

    #[test]
    fn created_definition_selects_only_a_new_identity_derived_from_the_base() {
        let definitions = ["ui-base", "ui-base-2", "ui-other", "existing"]
            .into_iter()
            .map(|id| {
                serde_json::from_value::<PartDefinition>(serde_json::json!({
                    "id": id, "name": id, "kind": "custom",
                    "courtyard": [], "pads": []
                }))
                .unwrap()
            })
            .collect::<Vec<_>>();
        let known = vec!["existing".to_owned(), "ui-other".to_owned()];
        assert_eq!(
            created_definition("ui-base", &known, &definitions)
                .unwrap()
                .id,
            "ui-base-2"
        );
        let known_with_base = vec!["existing".to_owned(), "ui-base".to_owned()];
        assert_eq!(
            created_definition("ui-base", &known_with_base, &definitions)
                .unwrap()
                .id,
            "ui-base-2"
        );
        let known_all = definitions
            .iter()
            .map(|definition| definition.id.clone())
            .collect::<Vec<_>>();
        assert!(created_definition("ui-base", &known_all, &definitions).is_none());
        assert!(
            created_definition("ui-unrelated", &known, &definitions).is_none(),
            "an identity that does not derive from the base is never selected"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    mod native {
        use super::*;
        use boardstudio_application::{Event, OperationId, TerminalOutcome};
        use boardstudio_core::model::{PartKind, ProjectDoc};
        use boardstudio_web_runtime::{operation_outcomes::OutcomeSlot, runtime::Runtime};
        use std::rc::Rc;

        fn open_document(document: ProjectDoc) -> Rc<Runtime> {
            let runtime = Runtime::new();
            runtime.submit(Event::Open {
                operation_id: runtime.operation(),
                document,
            });
            assert!(runtime.model().accepted.is_some());
            runtime
        }

        /// Submit the event built for a fresh operation. The slot holds its settlement once
        /// settled; a gated Runtime leaves it empty until the gate is released.
        fn submit(runtime: &Runtime, event: impl FnOnce(OperationId) -> Event) -> OutcomeSlot {
            let operation = runtime.operation();
            let slot = runtime.observe_operation(operation);
            runtime.submit(event(operation));
            slot
        }

        fn settled(slot: &OutcomeSlot) -> Option<TerminalOutcome> {
            slot.borrow().clone()
        }

        fn definitions(runtime: &Runtime) -> Vec<PartDefinition> {
            runtime
                .model()
                .accepted
                .expect("accepted document")
                .document
                .definitions
                .clone()
        }

        fn create(runtime: &Runtime, base: &str) -> OutcomeSlot {
            submit(runtime, |operation_id| Event::ResolveEdit {
                operation_id,
                label: "parts-new-component".into(),
                resolver: new_component_resolver(base.into()),
            })
        }

        fn existing_definition() -> PartDefinition {
            serde_json::from_value::<PartDefinition>(serde_json::json!({
                "id": "existing", "name": "Existing", "kind": "custom",
                "courtyard": [], "pads": []
            }))
            .unwrap()
        }

        #[test]
        fn create_lands_against_the_accepted_document_and_round_trips_history() {
            let mut document = ProjectDoc::empty("parts-create-test", "Parts create fixture");
            document.definitions.push(existing_definition());
            let original_parts = document.parts.clone();
            let original_boards = document.boards.clone();
            let runtime = open_document(document);
            let known = definitions(&runtime)
                .iter()
                .map(|definition| definition.id.clone())
                .collect::<Vec<_>>();

            assert_eq!(
                settled(&create(&runtime, "ui-fresh")),
                Some(TerminalOutcome::Completed),
                "the create lands"
            );
            let accepted = runtime.model().accepted.expect("accepted document");
            let created =
                created_definition("ui-fresh", &known, &accepted.document.definitions).unwrap();
            assert_eq!(created.name, "Custom component 2");
            assert!(matches!(created.kind, PartKind::Custom));
            assert_eq!(created.pads.len(), 0);
            assert_eq!(
                created
                    .courtyard
                    .iter()
                    .map(|point| (point.x, point.y))
                    .collect::<Vec<_>>(),
                [(-5.0, -3.0), (5.0, -3.0), (5.0, 3.0), (-5.0, 3.0)]
            );
            assert_eq!(accepted.document.parts, original_parts);
            assert_eq!(accepted.document.boards, original_boards);

            submit(&runtime, |operation_id| Event::Undo { operation_id });
            assert!(
                !definitions(&runtime)
                    .iter()
                    .any(|definition| identity_derives_from("ui-fresh", &definition.id))
            );
            submit(&runtime, |operation_id| Event::Redo { operation_id });
            assert!(
                definitions(&runtime)
                    .iter()
                    .any(|definition| definition.id == "ui-fresh")
            );
        }

        #[test]
        fn two_queued_creates_derive_distinct_identities_and_keep_unrelated_edits() {
            let runtime = open_document(ProjectDoc::empty(
                "parts-create-test",
                "Parts create fixture",
            ));
            assert_eq!(
                settled(&create(&runtime, "ui-first")),
                Some(TerminalOutcome::Completed)
            );
            assert_eq!(
                settled(&create(&runtime, "ui-first")),
                Some(TerminalOutcome::Completed)
            );
            let definitions = definitions(&runtime);
            let ids: Vec<&String> = definitions
                .iter()
                .map(|definition| &definition.id)
                .collect();
            assert_eq!(ids, [&"ui-first".to_owned(), &"ui-first-1".to_owned()]);
        }

        #[test]
        fn creates_queued_behind_a_held_core_request_keep_the_prior_accepted_definition() {
            let mut document = ProjectDoc::empty("parts-create-test", "Parts create fixture");
            document.definitions.push(existing_definition());
            let runtime = open_document(document);

            runtime.hold_next_core();
            let first = create(&runtime, "ui-held");
            assert!(runtime.core_entered(), "the first create reaches Core");
            let second = create(&runtime, "ui-held");
            assert_eq!(settled(&first), None);
            assert_eq!(settled(&second), None, "the second create waits for Core");

            runtime.release_core();
            assert_eq!(settled(&first), Some(TerminalOutcome::Completed));
            assert_eq!(settled(&second), Some(TerminalOutcome::Completed));
            let ids = definitions(&runtime)
                .into_iter()
                .map(|definition| definition.id)
                .collect::<Vec<_>>();
            assert_eq!(ids, ["existing", "ui-held", "ui-held-1"]);
        }

        #[test]
        fn accepted_project_selection_resolves_without_catalogue_state() {
            let mut document = ProjectDoc::empty("parts-create-test", "Parts create fixture");
            document.definitions.push(
                serde_json::from_value(serde_json::json!({
                    "id": "project-custom", "name": "Project custom", "kind": "custom",
                    "courtyard": [], "pads": []
                }))
                .unwrap(),
            );
            let runtime = open_document(document);
            let snapshot = runtime.model().accepted.expect("accepted document");
            let scope = runtime.scope();
            let selection = Some((scope.clone(), "project-custom".to_string()));

            let definition = accepted_project_definition(&snapshot, &scope, &selection).unwrap();
            assert_eq!(definition.id, "project-custom");
            assert_eq!(definition.name, "Project custom");

            let other_scope = Some(Scope {
                board_id: "another-board".into(),
                ..scope.clone().unwrap()
            });
            assert!(accepted_project_definition(&snapshot, &other_scope, &selection).is_none());
            assert!(
                accepted_project_definition(
                    &snapshot,
                    &scope,
                    &Some((scope.clone(), "missing".to_string()))
                )
                .is_none()
            );
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use boardstudio_core::model::ProjectDoc;
    use boardstudio_web_runtime::{
        runtime::Runtime, runtime::project_name_test_support as support,
    };
    use dioxus::prelude::*;
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    type SelectionProbe = Rc<RefCell<Option<Signal<Option<(Option<Scope>, String)>>>>>;
    fn host() -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let probe = use_context::<SelectionProbe>();
        let selected = use_signal(|| None);
        *probe.borrow_mut() = Some(selected);
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let view = use_signal(|| 1);
        let scope_generation = use_signal(|| 1);
        let workspace = use_signal(|| "Parts");
        let query = use_signal(String::new);
        rsx! { NewCustomComponentAction { scope: runtime.scope(), view_generation: view, scope_generation, workspace, query, selected, on_select: |_| {} } }
    }
    #[wasm_bindgen_test]
    async fn create_selects_only_after_the_exact_ticket_lands() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("create-selection", "Create selection");
        document.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
        support::open_document(&runtime, document).await;
        let probe: SelectionProbe = Rc::new(RefCell::new(None));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(runtime.clone());
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(30).await;
        let button = root
            .query_selector("button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let (entered, release) = support::gate_next_core_reply(&runtime);
        button.click();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        assert!(button.has_attribute("disabled"));
        assert!(probe.borrow().as_ref().unwrap()().is_none());
        release.send(()).unwrap();
        gloo_timers::future::TimeoutFuture::new(30).await;
        support::run_pending(&runtime).await;
        gloo_timers::future::TimeoutFuture::new(30).await;
        let selected = probe.borrow().as_ref().unwrap()().unwrap();
        assert_eq!(selected.0, runtime.scope());
        assert!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .definitions
                .iter()
                .any(|definition| definition.id == selected.1)
        );
        assert!(!button.has_attribute("disabled"));
        runtime.unsubscribe();
        root.remove();
    }
}
