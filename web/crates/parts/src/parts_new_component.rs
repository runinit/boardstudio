//! Parts-private custom-component creation through the edit ticket.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Scope};
use boardstudio_model::model::PartDefinition;

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{created_definition, new_component_resolver};
    use crate::runtime::Runtime;
    use boardstudio_application::Scope;
    use boardstudio_web_ui_model::Workspace;
    use boardstudio_web_ui_shared::inspector_fields::TOUCH_CLS;
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::{Button, ButtonVariant},
    };
    use dioxus::prelude::*;
    use js_sys::{Date, Function, Reflect};
    use std::rc::Rc;
    use wasm_bindgen::{JsCast, JsValue};

    /// The Parts parent mounts this action outside its catalogue loading/error branches so
    /// module-source availability never controls whether a project definition can be created.
    #[component]
    pub fn NewCustomComponentAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<Workspace>,
        mut query: Signal<String>,
        mut selected: Signal<Option<(Option<Scope>, String)>>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut error = use_signal(|| None::<(Scope, u64, u64, String)>);
        let create = {
            let runtime = runtime.clone();
            move |_| {
                if workspace() != Workspace::Parts || runtime.scope() != scope {
                    return;
                }
                let Some(current_scope) = scope.clone() else {
                    return;
                };
                let Some(snapshot) = runtime.model().accepted else {
                    return;
                };
                if snapshot.document.id != current_scope.document_id
                    || snapshot.session_epoch != current_scope.session_epoch
                {
                    return;
                }
                let base = format!("ui-{}", new_definition_identity());
                let known = snapshot
                    .document
                    .definitions
                    .iter()
                    .map(|definition| definition.id.clone())
                    .collect::<Vec<_>>();
                error.set(None);
                match runtime.commit("parts-new-component", new_component_resolver(base.clone())) {
                    Ok(current) => {
                        if let Some(definition) =
                            created_definition(&base, &known, &current.document.definitions)
                        {
                            selected.set(Some((Some(current_scope), definition.id.clone())));
                            query.set(String::new());
                            on_select.call(());
                        }
                    }
                    Err(message) => error.set(Some((
                        current_scope,
                        view_generation(),
                        scope_generation(),
                        message,
                    ))),
                }
            }
        };

        rsx! {
            Button {
                variant: ButtonVariant::Secondary,
                class: TOUCH_CLS,
                onclick: create,
                "New custom component"
            }
            if let Some((_, _, _, message)) = error().filter(|(owner, view, generation, _)| runtime.scope().as_ref() == Some(owner) && workspace() == Workspace::Parts && view_generation() == *view && scope_generation() == *generation) {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
        }
    }

    pub(crate) fn new_definition_identity() -> String {
        browser_identity().unwrap_or_else(date_identity)
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
#[cfg(target_arch = "wasm32")]
pub(crate) use ui::new_definition_identity;

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
    crate::parts_custom_definition::replace_document_resolver(
        "parts-new-component",
        move |accepted, replacement| {
            let definition_id = non_colliding_id(&base, &accepted.document.definitions)
                .ok_or("Could not allocate a unique component identity.")?;
            let definition = custom_definition(
                definition_id.clone(),
                accepted.document.definitions.len() + 1,
            )?;
            replacement.definitions.push(definition);
            Ok(vec![definition_id])
        },
    )
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
        use boardstudio_model::model::{PartKind, ProjectDoc};
        use boardstudio_web_runtime::{operation_outcomes::OutcomeSlot, runtime::Runtime};
        use std::rc::Rc;

        fn open_document(document: ProjectDoc) -> Rc<Runtime> {
            let runtime = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
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
