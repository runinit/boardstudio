use super::definition_name_resolver;
use crate::parts_custom_definition::DefinitionPanelCapture;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_model::model::PartDefinition;
use boardstudio_web_ui_shared::inspector_fields::TextField;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
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
    let mut draft_dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    let pad_count = definition.pads.len();
    let mut section_open = use_signal(|| pad_count == 0);
    let mut section_chosen = use_signal(|| false);
    let section_owner = (scope.clone(), definition.id.clone());
    let capture = DefinitionPanelCapture::new(&snapshot, scope.clone(), &definition);
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
            if current
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
            let result = runtime.commit("parts-definition-name", resolver);
            if let Some(name) = runtime
                .model()
                .accepted
                .as_ref()
                .and_then(|snapshot| {
                    snapshot
                        .document
                        .definitions
                        .iter()
                        .find(|definition| definition.id == capture.definition_id)
                })
                .map(|definition| definition.name.clone())
            {
                draft.set(name);
            }
            draft_dirty.set(false);
            failure.set(result.err());
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
                        TextField {
                            id: name_id.clone(),
                            label: "Name",
                            aria_label: Some("Definition name".to_owned()),
                            value: draft(),
                            error: name_error.clone(),
                            oninput: move |event: FormEvent| {
                                failure.set(None);
                                draft.set(event.value());
                                draft_dirty.set(true);
                            },
                            onblur: on_blur,
                            onkeydown: on_keydown,
                        }
                    }
                    {children}
                }
            }
        }
    }
}
