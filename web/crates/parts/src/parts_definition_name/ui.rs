use super::definition_name_resolver;
use crate::parts_custom_definition::DefinitionPanelCapture;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::PartDefinition;
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::draft_edits::{DraftEdits, FieldView};
use boardstudio_web_ui_shared::ui::{
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::{Field, FieldError, FieldLabel},
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// This panel's one committed field. A bounded logical key: the helper keeps one
/// observation for the name field, whatever definition the panel currently shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NameFieldKey {
    Name,
}

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
    let mut draft_dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let pending = use_hook(DraftEdits::<NameFieldKey>::default);
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
    let observation_owner = use_hook(|| Rc::new(RefCell::new(capture.clone())));
    if *observation_owner.borrow() != capture {
        // Detach the old owner's Signals before silently retiring its observations.
        // The Session still owns and executes the already committed edits.
        pending.clear();
        *observation_owner.borrow_mut() = capture.clone();
        draft.set(definition.name.clone());
        draft_dirty.set(false);
        failure.set(None);
    }
    // A fresh accepted snapshot is required at commit time, but it is not
    // a reason to discard a dirty field draft. React DraftInput keys its
    // reset to the accepted field value; keep this target identity
    // independent from the snapshot capture that protects submission.
    let draft_identity = (scope.clone(), selection(), definition.id.clone());
    use_effect(use_reactive((&draft_identity,), {
        let definition = definition.clone();
        move |(_identity,)| {
            draft.set(definition.name.clone());
            draft_dirty.set(false);
            failure.set(None);
        }
    }));
    let accepted_name = definition.name.clone();
    use_effect(use_reactive((&accepted_name,), move |(value,)| {
        // A newer user draft may equal the previous accepted name. Its dirty
        // marker, rather than value equality, protects it when an older edit lands.
        if !draft_dirty() {
            draft.set(value);
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
    // retired edit drops so the field follows the accepted document again. The
    // editor outlives selection changes, so owner liveness is a real answer.
    {
        let model = runtime.model();
        let owner_live = model
            .accepted
            .as_ref()
            .is_some_and(|current| capture.owner_matches(current, runtime.scope(), selection()));
        let results = pending.settle(owner_live, |_| definition.name.clone());
        if !results.is_empty()
            && !pending.is_pending(&NameFieldKey::Name)
            && *draft.peek() == definition.name
        {
            draft_dirty.set(false);
        }
        for result in &results {
            if let PendingEditResult::Failed { message, .. } = result {
                failure.set(Some(message.clone()));
            }
        }
    }

    let accepted_name = definition.name.clone();
    let on_blur = {
        let runtime = runtime.clone();
        let capture = capture.clone();
        let selection = selection;
        move |_| {
            let model = runtime.model();
            let Some(current) = model.accepted else {
                return;
            };
            if !capture.owner_matches(&current, runtime.scope(), selection()) {
                return;
            }
            let name = draft();
            if !pending.is_pending(&NameFieldKey::Name)
                && current
                    .document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == capture.definition_id)
                    .is_some_and(|definition| definition.name == name)
            {
                draft_dirty.set(false);
                return;
            }
            let resolver = definition_name_resolver(capture.definition_id.clone(), name);
            pending.submit(
                &runtime,
                NameFieldKey::Name,
                "parts-definition-name",
                Some("part definition".into()),
                resolver,
                FieldView::new(draft, failure).submitted(draft.peek().clone()),
            );
            failure.set(None);
        }
    };
    let on_keydown = {
        let mut keydown_draft = draft;
        let value = accepted_name;
        move |event: KeyboardEvent| {
            if event.key() == Key::Escape {
                event.prevent_default();
                keydown_draft.set(value.clone());
                draft_dirty.set(false);
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
    let name_id = format!(
        "parts-definition-name-{}",
        dioxus::core::current_scope_id().0
    );
    let name_error = failure();
    rsx! {
        Collapsible {
            keep_mounted: true,
            open: Some(section_open()),
            on_open_change: move |next| {
                section_chosen.set(true);
                section_open.set(next);
            },
            CollapsibleTrigger { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: "Edit footprint Custom geometry",
                span { "Edit footprint" }
                small { "Custom geometry" }
            }
            CollapsibleContent { class: "data-[open=false]:hidden",
                div { class: "grid gap-2",
                    section { class: "grid gap-2", "aria-label": "Custom component definition editor",
                        Field { invalid: name_error.is_some(), class: "gap-1.5",
                            FieldLabel { html_for: name_id.clone(), "Name" }
                            InputGroup {
                                InputGroupInput {
                                    id: name_id.clone(),
                                    r#type: InputType::Text,
                                    aria_label: "Definition name",
                                    value: "{draft()}",
                                    aria_invalid: name_error.is_some(),
                                    oninput: move |event: FormEvent| {
                                        draft.set(event.value());
                                        draft_dirty.set(true);
                                    },
                                    attributes: vec![
                                        dioxus::events::onblur(on_blur),
                                        dioxus::events::onkeydown(on_keydown),
                                    ],
                                }
                            }
                            if let Some(message) = name_error.as_deref() {
                                FieldError { "{message}" }
                            }
                        }
                    }
                    {children}
                }
            }
        }
    }
}
