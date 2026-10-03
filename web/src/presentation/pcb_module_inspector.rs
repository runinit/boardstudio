//! Private editor for the placement of one accepted mounted-module instance.
//! Module source definitions and their footprint/circuit ownership remain untouched.
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, Lifecycle, Scope};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, ModuleAttachment, Side};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
struct RuntimeHandle(Rc<Runtime>);

impl PartialEq for RuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub(super) struct InspectorInput {
    pub(super) runtime: Rc<Runtime>,
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Scope,
    pub(super) module_id: String,
    pub(super) selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        PcbMountedModuleInspector {
            key: "{input.module_id}",
            runtime: RuntimeHandle(input.runtime),
            snapshot: input.snapshot,
            scope: input.scope,
            module_id: input.module_id,
            selected_context: input.selected_context,
        }
    }
}

#[component]
fn PcbMountedModuleInspector(
    runtime: RuntimeHandle,
    snapshot: AcceptedSnapshot,
    scope: Scope,
    module_id: String,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
) -> Element {
    let input = InspectorInput {
        runtime: runtime.0,
        snapshot,
        scope,
        module_id,
        selected_context,
    };
    let document = &input.snapshot.document;
    let Some(instance) = document.modules.iter().find(|module| {
        module.id == input.module_id && module.host_board_id == input.scope.board_id
    }) else {
        return rsx! { p { role: "status", "This mounted module is no longer available on the selected board." } };
    };
    let Some(definition) = document
        .module_definitions
        .iter()
        .find(|definition| definition.id == instance.definition_id)
    else {
        return rsx! { p { role: "status", "The mounted module source definition is missing." } };
    };
    let mut draft = use_signal(|| instance.clone());
    let mut feedback = use_signal(String::new);
    let accepted_instance = instance.clone();
    use_effect(use_reactive!(|accepted_instance| {
        if draft() != accepted_instance {
            draft.set(accepted_instance);
            feedback.set(String::new());
        }
    }));
    let module_name = definition.name.clone();
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let save = move |_| {
        let Some(snapshot) = mounted_owner_current(
            &runtime,
            selected_context,
            &scope,
            &module_id,
            owner_token,
            owner_revision,
        ) else {
            feedback.set("The selected module or accepted project changed. Reopen its placement before saving.".into());
            return;
        };
        let value = draft();
        let operation_id = runtime.operation();
        runtime.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: format!("mounted-module-placement-{}", operation_id.0),
                phase: EditPhase::Commit,
                target_ids: vec![value.id.clone()],
                operation: EditOperation::SetMountedModule {
                    instance: Box::new(value),
                    definition: None,
                    host_connector_definition: None,
                },
            },
        });
        feedback.set("Placement submitted for save.".into());
    };
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let remove = move |_| {
        let Some(snapshot) = mounted_owner_current(
            &runtime,
            selected_context,
            &scope,
            &module_id,
            owner_token,
            owner_revision,
        ) else {
            feedback.set("The selected module or accepted project changed. Reopen its placement before removing it.".into());
            return;
        };
        let operation_id = runtime.operation();
        runtime.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: snapshot.document.revision,
                transaction_id: format!("remove-mounted-module-{}", operation_id.0),
                phase: EditPhase::Commit,
                target_ids: vec![module_id.clone()],
                operation: EditOperation::RemoveMountedModule {
                    id: module_id.clone(),
                },
            },
        });
        feedback.set("Module removal submitted for save.".into());
    };
    let boards = document.boards.clone();
    let instances = document
        .hardware
        .as_ref()
        .map(|hardware| hardware.instances.clone())
        .unwrap_or_default();
    let board_id = draft().host_board_id.clone();
    let options = instances
        .iter()
        .filter(|item| item.board_id == board_id)
        .map(|item| (item.id.clone(), item.name.clone()))
        .collect::<Vec<_>>();
    let editable = input.snapshot.document.id == input.scope.document_id
        && input.snapshot.session_epoch == input.scope.session_epoch;

    rsx! {
        section { class: "m1-pcb-module-inspector", "aria-label": "Mounted module placement",
            h2 { "{module_name} placement" }
            p { class: "m1-pcb-module-source-note", "Placement edits affect this project instance. Source footprint artwork remains owned by the module." }
            label { "Host board"
                select {
                    aria_label: "Module host board",
                    value: "{draft().host_board_id}",
                    disabled: !editable,
                    onchange: move |event| {
                        let next = event.value();
                        draft.with_mut(|value| {
                            value.host_board_id = next.clone();
                            if value.host_instance_id.as_ref().is_some_and(|id| !instances.iter().any(|instance| instance.id == *id && instance.board_id == next)) {
                                value.host_instance_id = None;
                            }
                        });
                    },
                    for board in &boards { option { value: "{board.id}", "{board.name}" } }
                }
            }
            label { "Physical instance"
                select {
                    aria_label: "Module physical instance",
                    value: "{draft().host_instance_id.clone().unwrap_or_default()}",
                    disabled: !editable,
                    onchange: move |event| {
                        let id = event.value();
                        draft.with_mut(|value| value.host_instance_id = (!id.is_empty()).then_some(id));
                    },
                    option { value: "", "Board-wide" }
                    for (id, name) in &options { option { value: "{id}", "{name}" } }
                }
            }
            label { "Mounted face"
                select {
                    aria_label: "Module host face",
                    value: if draft().host_face == Side::Front { "front" } else { "back" },
                    disabled: !editable,
                    onchange: move |event| draft.with_mut(|value| value.host_face = side(&event.value())),
                    option { value: "front", "Front" }
                    option { value: "back", "Back" }
                }
            }
            label { "Facing face"
                select {
                    aria_label: "Module facing face",
                    value: if draft().facing_face == Side::Front { "front" } else { "back" },
                    disabled: !editable,
                    onchange: move |event| draft.with_mut(|value| value.facing_face = side(&event.value())),
                    option { value: "front", "Front" }
                    option { value: "back", "Back" }
                }
            }
            label { "X (mm)"
                input { r#type: "number", step: "any", value: "{draft().at.x}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() { if number.is_finite() { draft.with_mut(|value| value.at.x = number); } }
                }
            }
            label { "Y (mm)"
                input { r#type: "number", step: "any", value: "{draft().at.y}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() { if number.is_finite() { draft.with_mut(|value| value.at.y = number); } }
                }
            }
            label { "Yaw (degrees)"
                input { r#type: "number", step: "any", value: "{draft().rotation}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() { if number.is_finite() { draft.with_mut(|value| value.rotation = number); } }
                }
            }
            label { "Gap (mm)"
                input { r#type: "number", min: "0", step: "any", value: "{draft().gap}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() { if number.is_finite() && number >= 0.0 { draft.with_mut(|value| value.gap = number); } }
                }
            }
            label { "Attachment"
                select {
                    aria_label: "Module attachment",
                    value: if draft().attachment == ModuleAttachment::Board { "board" } else { "case" },
                    disabled: !editable,
                    onchange: move |event| draft.with_mut(|value| value.attachment = if event.value() == "case" { ModuleAttachment::Case } else { ModuleAttachment::Board }),
                    option { value: "board", "Board" }
                    option { value: "case", "Case" }
                }
            }
            label { input { r#type: "checkbox", checked: draft().detached, disabled: !editable,
                onchange: move |event| draft.with_mut(|value| value.detached = event.checked())
            } "Detached" }
            div { class: "m1-pcb-module-actions",
                button { class: "m1-primary-button", r#type: "button", disabled: !editable, onclick: save, "Save placement" }
                button { class: "m1-danger-button", r#type: "button", disabled: !editable, onclick: remove, "Remove module" }
            }
            if !feedback().is_empty() { p { role: "status", "{feedback()}" } }
        }
    }
}

fn side(value: &str) -> Side {
    if value == "back" {
        Side::Back
    } else {
        Side::Front
    }
}

fn mounted_owner_current(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    scope: &Scope,
    module_id: &str,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
) -> Option<AcceptedSnapshot> {
    if runtime.scope().as_ref() != Some(scope) {
        return None;
    }
    let selected = selected_context.read().clone()?;
    if selected.scope != *scope
        || selected.context
            != (super::objects::TreeContext::MountedModule {
                board_id: scope.board_id.clone(),
                module_id: module_id.to_owned(),
            })
    {
        return None;
    }
    let model = runtime.model();
    if model.lifecycle != Lifecycle::Ready
        || !super::selection::context_is_current(&model, scope, &selected.context)
    {
        return None;
    }
    let snapshot = model.accepted?;
    (snapshot.token == token
        && snapshot.document.revision == revision
        && snapshot.document.id == scope.document_id
        && snapshot.session_epoch == scope.session_epoch)
        .then_some(snapshot)
}
