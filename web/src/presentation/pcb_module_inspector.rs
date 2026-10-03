//! Private editor for the placement of one accepted mounted-module instance.
//! Module source definitions and their footprint/circuit ownership remain untouched.
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, Event, Lifecycle, Scope};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, ModuleAttachment, ModuleSupport, Side,
};
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

#[derive(Clone, Default)]
struct SupportDraft {
    mount_id: String,
    outer_diameter: String,
    hole_diameter: String,
    z: String,
    height: String,
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    let owner_key = format!("{:?}:{}", input.scope, input.module_id);
    rsx! {
        PcbMountedModuleInspector {
            key: "{owner_key}",
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
    let mut support_draft = use_signal(SupportDraft::default);
    let accepted_instance = instance.clone();
    use_effect(use_reactive!(|accepted_instance| {
        if *draft.peek() != accepted_instance {
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
    let source_mounts = definition.mounts.clone();
    let add_source_mounts = source_mounts.clone();
    let has_source_mounts = !source_mounts.is_empty();
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
    let resolved_supports = input
        .snapshot
        .scene
        .module_scenes
        .iter()
        .find(|module| module.id == input.module_id)
        .map(|module| module.mount_supports.clone())
        .unwrap_or_default();
    let support_section = if draft().attachment == ModuleAttachment::Case {
        "Case support rings"
    } else {
        "Board standoffs"
    };
    let support_kind = if draft().attachment == ModuleAttachment::Case {
        "ring"
    } else {
        "standoff"
    };
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
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { draft.with_mut(|value| value.at.x = number); }
                }
            }
            label { "Y (mm)"
                input { r#type: "number", step: "any", value: "{draft().at.y}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { draft.with_mut(|value| value.at.y = number); }
                }
            }
            label { "Yaw (degrees)"
                input { r#type: "number", step: "any", value: "{draft().rotation}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { draft.with_mut(|value| value.rotation = number); }
                }
            }
            label { "Gap (mm)"
                input { r#type: "number", min: "0", step: "any", value: "{draft().gap}", disabled: !editable,
                    oninput: move |event| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() && number >= 0.0 { draft.with_mut(|value| value.gap = number); }
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
            label { "Extra service clearance (mm)"
                input { r#type: "number", min: "0", step: "0.1", aria_label: "Module service clearance", value: "{draft().service_clearance}", disabled: !editable,
                    oninput: move |event| {
                        let raw = event.value();
                        let number = if raw.trim().is_empty() { Some(0.0) } else { raw.parse::<f64>().ok() };
                        if let Some(number) = number.filter(|number| number.is_finite() && *number >= 0.0) {
                            draft.with_mut(|value| value.service_clearance = number);
                        }
                    }
                }
            }
            section { class: "m1-pcb-module-supports", "aria-label": "{support_section}",
                h3 { "{support_section}" }
                p { "Choose a source PCB hole and enter designer-selected ring dimensions. Z and height are module-midplane millimetres; these values are not vendor specifications." }
                for (index, support) in draft().mount_supports.iter().enumerate() {
                    div { key: "{support.mount_id}", class: "m1-pcb-module-support-row",
                        span { "{support.mount_id} · OD {support.outer_diameter} / ID {support.hole_diameter} · Z {support.z} · height {support.height} mm" }
                        button { r#type: "button", disabled: !editable, aria_label: "Remove support {support.mount_id}", onclick: move |_| draft.with_mut(|value| value.mount_supports.remove(index)), "Remove" }
                    }
                }
                if !resolved_supports.is_empty() {
                    div { class: "m1-pcb-module-support-preview", "aria-label": "Resolved support geometry",
                        strong { "Resolved {support_section}" }
                        for (index, support) in resolved_supports.iter().enumerate() {
                            p { key: "{support.mount_id}-{index}", "{support.mount_id} · center {support.at.x:.2}, {support.at.y:.2} mm · OD {support.outer_diameter:.2} / ID {support.hole_diameter:.2} mm · Z {support.z:.2} · height {support.height:.2} mm" }
                        }
                    }
                }
                label { "Source mounting hole"
                    select {
                        aria_label: "Support source mounting hole",
                        value: "{support_draft().mount_id}",
                        disabled: !editable || !has_source_mounts,
                        onchange: move |event| support_draft.with_mut(|value| value.mount_id = event.value()),
                        option { value: "", "Choose module hole…" }
                        for mount in &source_mounts {
                            option { value: "{mount.source_id}", "{mount.source_id} · source drill {mount.diameter} mm" }
                        }
                    }
                }
                div { class: "m1-pcb-module-support-fields",
                    label { "Outer diameter (mm)"
                        input { r#type: "number", step: "0.1", aria_label: "Support outer diameter", value: "{support_draft().outer_diameter}", disabled: !editable,
                            oninput: move |event| support_draft.with_mut(|value| value.outer_diameter = event.value())
                        }
                    }
                    label { "Hole diameter (mm)"
                        input { r#type: "number", step: "0.1", aria_label: "Support hole diameter", value: "{support_draft().hole_diameter}", disabled: !editable,
                            oninput: move |event| support_draft.with_mut(|value| value.hole_diameter = event.value())
                        }
                    }
                    label { "Z from midplane (mm)"
                        input { r#type: "number", step: "0.1", aria_label: "Support Z from midplane", value: "{support_draft().z}", disabled: !editable,
                            oninput: move |event| support_draft.with_mut(|value| value.z = event.value())
                        }
                    }
                    label { "Ring height (mm)"
                        input { r#type: "number", step: "0.1", aria_label: "Support ring height", value: "{support_draft().height}", disabled: !editable,
                            oninput: move |event| support_draft.with_mut(|value| value.height = event.value())
                        }
                    }
                }
                button { r#type: "button", disabled: !editable || !has_source_mounts, onclick: move |_| {
                    let fields = support_draft().clone();
                    let source_mount = add_source_mounts.iter().find(|mount| mount.source_id == fields.mount_id);
                    let Some(mount) = source_mount else {
                        feedback.set("Choose a source mounting hole before adding a support.".into());
                        return;
                    };
                    let parsed = [fields.outer_diameter.as_str(), fields.hole_diameter.as_str(), fields.z.as_str(), fields.height.as_str()]
                        .map(str::parse::<f64>);
                    let [Ok(outer_diameter), Ok(hole_diameter), Ok(z), Ok(height)] = parsed else {
                        feedback.set("Enter finite support dimensions before adding a support.".into());
                        return;
                    };
                    if ![outer_diameter, hole_diameter, z, height].iter().all(|number| number.is_finite())
                        || fields.outer_diameter.trim().is_empty()
                        || fields.hole_diameter.trim().is_empty()
                        || fields.z.trim().is_empty()
                        || fields.height.trim().is_empty()
                        || outer_diameter <= hole_diameter
                        || hole_diameter < mount.diameter
                        || height <= 0.0
                    {
                        feedback.set("Support dimensions must be finite; outer diameter must exceed the hole, the hole must clear the source drill, and height must be positive.".into());
                        return;
                    }
                    if draft().mount_supports.iter().any(|support| support.mount_id == mount.source_id) {
                        feedback.set("This source mounting hole already has a support. Remove it before adding another.".into());
                        return;
                    }
                    draft.with_mut(|value| value.mount_supports.push(ModuleSupport {
                        mount_id: mount.source_id.clone(),
                        outer_diameter,
                        hole_diameter,
                        z,
                        height,
                    }));
                    support_draft.set(SupportDraft::default());
                    feedback.set(String::new());
                }, "Add specified {support_kind}" }
                if !has_source_mounts { p { "This module snapshot has no source mounting holes for support placement." } }
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
