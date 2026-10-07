//! Private editor for the placement of one accepted mounted-module instance.
//! Module source definitions and their footprint/circuit ownership remain untouched.
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, ModuleAttachment, ModuleConnection, ModuleSupport,
    PartDefinition, Side, VikRole, VikSignal,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::rc::Rc;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
struct RuntimeHandle(Rc<Runtime>);

impl PartialEq for RuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub struct InspectorInput {
    pub runtime: Rc<Runtime>,
    pub snapshot: AcceptedSnapshot,
    pub scope: Scope,
    pub module_id: String,
    pub selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    pub on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
}

#[derive(Clone, Default)]
struct SupportDraft {
    mount_id: String,
    outer_diameter: String,
    hole_diameter: String,
    z: String,
    height: String,
}

#[derive(Clone)]
struct ConstituentAction {
    index: usize,
    reference: String,
    name: String,
    footprint: String,
    purchased: bool,
    module_definition_id: String,
    source_definition: Option<PartDefinition>,
}

pub fn inspector(input: InspectorInput) -> Element {
    let owner_key = format!("{:?}:{}", input.scope, input.module_id);
    rsx! {
        PcbMountedModuleInspector {
            key: "{owner_key}",
            runtime: RuntimeHandle(input.runtime),
            snapshot: input.snapshot,
            scope: input.scope,
            module_id: input.module_id,
            selected_context: input.selected_context,
            on_place_component: input.on_place_component,
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
    on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let input = InspectorInput {
        runtime: runtime.0,
        snapshot,
        scope,
        module_id,
        selected_context,
        on_place_component,
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
    let mut joins = use_signal(BTreeMap::<String, String>::new);
    let tickets = use_signal(Vec::<(String, EditTicket)>::new);
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let committed_draft = use_signal(|| None::<boardstudio_core::model::MountedModule>);
    let previous_accepted = use_signal(|| instance.clone());
    let save_queue = use_hook(|| {
        Rc::new(RefCell::new(VecDeque::<(
            boardstudio_core::model::MountedModule,
            bool,
        )>::new()))
    });
    let preparing = use_hook(|| Rc::new(Cell::new(false)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let version = use_context::<Signal<u64>>()();
    let pending_operations = tickets
        .read()
        .iter()
        .map(|(_, ticket)| ticket.operation())
        .collect::<Vec<_>>();
    use_effect(use_reactive((&version, &pending_operations), {
        let runtime = input.runtime.clone();
        let scope = input.scope.clone();
        let module_id = input.module_id.clone();
        let selected_context = input.selected_context;
        let mut tickets = tickets;
        let mut feedback = feedback;
        move |_| {
            let mut entries = tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(action, ticket)| {
                let result = ticket.settlement(mounted_selection_current(
                    &runtime,
                    selected_context,
                    &scope,
                    &module_id,
                ));
                let message = match result {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(
                        match action.as_str() {
                            "placement" => "Placement saved.",
                            "remove-module" => "Placement removed.",
                            "embed" => "Circuit copied.",
                            _ => "Circuit copy removed.",
                        }
                        .into(),
                    ),
                    Settlement::Failed { message } => {
                        if action == "placement"
                            && *latest.peek() == Some(ticket.operation())
                            && committed_draft.peek().as_ref() == Some(&*draft.peek())
                            && let Some(accepted) = runtime.model().accepted
                            && let Some(module) = accepted
                                .document
                                .modules
                                .iter()
                                .find(|module| module.id == module_id)
                        {
                            draft.set(module.clone());
                        }
                        Some(message)
                    }
                    Settlement::Retired => None,
                };
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(message.unwrap_or_default());
                }
                false
            });
            if entries.len() != before {
                tickets.set(entries);
            }
        }
    }));
    let accepted_instance = instance.clone();
    let pending_save = preparing.get()
        || tickets
            .read()
            .iter()
            .any(|(action, ticket)| action == "placement" && ticket.is_pending());
    use_effect(use_reactive(
        (&accepted_instance, &pending_save),
        move |(accepted_instance, pending_save)| {
            if *draft.peek() == *previous_accepted.peek()
                || (!pending_save && committed_draft.peek().as_ref() == Some(&*draft.peek()))
            {
                draft.set(accepted_instance.clone());
            }
            let mut previous_accepted = previous_accepted;
            previous_accepted.set(accepted_instance);
        },
    ));
    let module_name = definition.name.clone();
    let automatic_connector_id = format!("{}/vik-host-connector", instance.id);
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let save_alive = alive.clone();
    let save = move |_| {
        let Some(snapshot) = mounted_owner_current(
            &runtime,
            selected_context,
            &scope,
            &module_id,
            owner_token,
            owner_revision,
        ) else {
            return;
        };
        let value = draft();
        let automatic_id = format!("{}/vik-host-connector", value.id);
        let connector = value.connection.as_ref().map(|connection| {
            connector_admission(
                &connection.host_connector_part_id,
                &automatic_id,
                host_connector_is_on_board(
                    &snapshot.document,
                    &value.host_board_id,
                    &connection.host_connector_part_id,
                ),
                snapshot
                    .document
                    .parts
                    .iter()
                    .any(|part| part.id == connection.host_connector_part_id),
            )
        });
        let needs_connector = matches!(connector, Some(ConnectorAdmission::CreateAutomatic));
        if matches!(connector, Some(ConnectorAdmission::Reject)) {
            feedback
                .set("Choose a VIK host connector on this board, or clear the connection.".into());
            return;
        }
        let mut committed_draft = committed_draft;
        committed_draft.set(Some(value.clone()));
        save_queue.borrow_mut().push_back((value, needs_connector));
        if preparing.replace(true) {
            return;
        }
        let runtime = runtime.clone();
        let scope = scope.clone();
        let module_id = module_id.clone();
        let alive = save_alive.clone();
        let save_queue = save_queue.clone();
        let preparing = preparing.clone();
        let mut tickets = tickets;
        let mut latest = latest;
        let mut feedback = feedback;
        spawn_local(async move {
            loop {
                let item = save_queue.borrow_mut().pop_front();
                let Some((value, needs_connector)) = item else {
                    preparing.set(false);
                    break;
                };
                let connector = if needs_connector {
                    super::parts::load_horizontal_host_connector_definition()
                        .await
                        .map(|definition| Some(Box::new(definition)))
                } else {
                    Ok(None)
                };
                if !alive.get() {
                    return;
                }
                if !mounted_selection_current(&runtime, selected_context, &scope, &module_id) {
                    continue;
                }
                let operation =
                    connector.map(
                        |host_connector_definition| EditOperation::SetMountedModule {
                            instance: Box::new(value),
                            definition: None,
                            host_connector_definition,
                        },
                    );
                let ticket = EditTicket::begin(
                    &runtime,
                    "mounted-module-placement",
                    Some("placement".into()),
                    module_resolver(scope.clone(), module_id.clone(), operation),
                );
                latest.set(Some(ticket.operation()));
                feedback.set(String::new());
                tickets.write().push(("placement".into(), ticket));
            }
        });
    };
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let remove = move |_| {
        if module_action_pending(tickets, "remove-module")
            || mounted_owner_current(
                &runtime,
                selected_context,
                &scope,
                &module_id,
                owner_token,
                owner_revision,
            )
            .is_none()
        {
            return;
        }
        let ticket = EditTicket::begin(
            &runtime,
            "remove-mounted-module",
            Some("module".into()),
            module_resolver(
                scope.clone(),
                module_id.clone(),
                Ok(EditOperation::RemoveMountedModule {
                    id: module_id.clone(),
                }),
            ),
        );
        let mut tickets = tickets;
        let mut latest = latest;
        latest.set(Some(ticket.operation()));
        feedback.set(String::new());
        tickets.write().push(("remove-module".into(), ticket));
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
    let connectors = document
        .parts
        .iter()
        .filter(|part| {
            document
                .boards
                .iter()
                .find(|board| board.id == board_id)
                .is_some_and(|board| board.part_ids.contains(&part.id))
                && document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .and_then(|definition| definition.hardware_profile.as_ref())
                    .is_some_and(|profile| profile.vik_role == Some(VikRole::Host))
        })
        .map(|part| (part.id.clone(), part.reference.clone()))
        .collect::<Vec<_>>();
    let automatic_connector_can_be_created = !document
        .parts
        .iter()
        .any(|part| part.id == automatic_connector_id);
    let available_connectors = connectors.clone();
    let automatic_connector_id_for_toggle = automatic_connector_id.clone();
    let create_automatic_connector_on_toggle = automatic_connector_can_be_created;
    let default_module_port_id = definition
        .interfaces
        .iter()
        .find(|port| port.role == VikRole::Module)
        .map(|port| port.id.clone())
        .unwrap_or_default();
    let embedded_circuits = document
        .embedded_circuits
        .iter()
        .filter(|circuit| {
            circuit.definition_id == definition.id && circuit.host_board_id == board_id
        })
        .cloned()
        .collect::<Vec<_>>();
    let circuit_nets = definition
        .circuit
        .as_ref()
        .map(|circuit| circuit.ports.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let constituent_actions = definition
        .constituents
        .iter()
        .enumerate()
        .map(|(index, constituent)| ConstituentAction {
            index,
            reference: constituent.reference.clone(),
            name: constituent.name.clone(),
            footprint: constituent.footprint.clone(),
            purchased: constituent.purchased,
            module_definition_id: definition.id.clone(),
            source_definition: constituent.definition_id.as_ref().and_then(|id| {
                definition
                    .circuit
                    .as_ref()?
                    .definitions
                    .iter()
                    .find(|source| &source.id == id)
                    .cloned()
            }),
        })
        .collect::<Vec<_>>();
    let host_nets = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| {
            board
                .net_ids
                .iter()
                .filter_map(|id| document.nets.iter().find(|net| &net.id == id))
                .map(|net| (net.id.clone(), net.name.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let embed_source = definition.clone();
    let embed_runtime = input.runtime.clone();
    let embed_scope = input.scope.clone();
    let embed_module_id = input.module_id.clone();
    let embed_context = input.selected_context;
    let embed_token = input.snapshot.token;
    let embed_revision = input.snapshot.document.revision;
    let embed_joins = joins;
    let embed = move |_| {
        let Some(_snapshot) = mounted_owner_current(
            &embed_runtime,
            embed_context,
            &embed_scope,
            &embed_module_id,
            embed_token,
            embed_revision,
        ) else {
            feedback.set("The selected module or accepted project changed. Reopen its placement before copying the circuit.".into());
            return;
        };
        if embed_source.circuit.is_none() {
            feedback.set("This module has no editable circuit source.".into());
            return;
        }
        if module_action_pending(tickets, "embed") {
            return;
        }
        let seed = embed_runtime.operation().0;
        let id = format!("circuit/embedded-{seed}");
        let placement = draft();
        let ticket = EditTicket::begin(
            &embed_runtime,
            "embed-module-circuit",
            Some("circuit".into()),
            module_resolver(
                embed_scope.clone(),
                embed_module_id.clone(),
                Ok(EditOperation::EmbedModuleCircuit {
                    id,
                    definition: embed_source.clone(),
                    host_board_id: placement.host_board_id,
                    pose: boardstudio_core::model::Pose2 {
                        at: placement.at,
                        rotation: placement.rotation,
                    },
                    side: placement.host_face,
                    joins: embed_joins.read().clone(),
                }),
            ),
        );
        let mut tickets = tickets;
        let mut latest = latest;
        latest.set(Some(ticket.operation()));
        feedback.set(String::new());
        tickets.write().push(("embed".into(), ticket));
    };

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
                        button { r#type: "button", disabled: !editable, aria_label: "Remove support {support.mount_id}", onclick: move |_| {
                            draft.with_mut(|value| { value.mount_supports.remove(index); });
                        }, "Remove" }
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
            section { class: "m1-pcb-module-connection", "aria-label": "VIK connection",
                h3 { "VIK connection" }
                label { input {
                    r#type: "checkbox",
                    checked: draft().connection.is_some(),
                    disabled: !editable,
                    onchange: move |event| {
                        if event.checked() {
                            draft.with_mut(|value| value.connection = Some(ModuleConnection {
                                host_connector_part_id: if available_connectors.is_empty() && create_automatic_connector_on_toggle { automatic_connector_id_for_toggle.clone() } else { String::new() },
                                module_port_id: default_module_port_id.clone(),
                                bus_id: format!("vik/{}", value.id),
                                assignments: BTreeMap::new(),
                                cable_type: "type-a-12-0.5".into(),
                                supply_current_ma: None,
                                rail_voltages: BTreeMap::new(),
                                upstream_module_id: None,
                                upstream_port_id: None,
                            }));
                        } else {
                            draft.with_mut(|value| value.connection = None);
                        }
                    }
                } "Assign host connection" }
                if let Some(connection) = draft().connection {
                    label { "Host connector"
                        select {
                            aria_label: "Module host connector",
                            value: "{connection.host_connector_part_id}",
                            disabled: !editable,
                            onchange: move |event| draft.with_mut(|value| if let Some(connection) = &mut value.connection { connection.host_connector_part_id = event.value(); }),
                            option { value: "", "Select VIK host connector" }
                            if connectors.is_empty() && automatic_connector_can_be_created {
                                option { value: "{automatic_connector_id}", "Add source-backed horizontal VIK connector beside module" }
                            }
                            for (id, reference) in &connectors { option { value: "{id}", "{reference}" } }
                        }
                    }
                    if connectors.is_empty() {
                        if automatic_connector_can_be_created {
                            p { "No source-backed host-role VIK connector is present on this board. Add one before saving a connected placement." }
                        } else {
                            p { "The automatic connector ID is already used by a part outside this board. Choose a valid host connector or clear the connection." }
                        }
                    }
                    label { "Module port"
                        select {
                            aria_label: "Module input port",
                            value: "{connection.module_port_id}",
                            disabled: !editable,
                            onchange: move |event| draft.with_mut(|value| if let Some(connection) = &mut value.connection { connection.module_port_id = event.value(); }),
                            for port in definition.interfaces.iter().filter(|port| port.role == VikRole::Module) {
                                option { value: "{port.id}", "{port.id}" }
                            }
                        }
                    }
                    label { "Bus name"
                        input { r#type: "text", aria_label: "Module bus name", value: "{connection.bus_id}", disabled: !editable,
                            oninput: move |event| draft.with_mut(|value| if let Some(connection) = &mut value.connection { connection.bus_id = event.value(); })
                        }
                    }
                    p { "12 contacts · 0.5 mm pitch · Type A cable · 3.3 V logic. Enter actual MCU terminals; shared buses are checked by their wiring." }
                    div { class: "m1-pcb-module-signals",
                        for (signal, label) in vik_signals() {
                            label { key: "{label}", "{label}"
                                input { r#type: "text", aria_label: "VIK {label} terminal", value: "{connection.assignments.get(&signal).cloned().unwrap_or_default()}", disabled: !editable,
                                    oninput: move |event| {
                                        let terminal = event.value().trim().to_owned();
                                        draft.with_mut(|value| if let Some(connection) = &mut value.connection {
                                            if terminal.is_empty() { connection.assignments.remove(&signal); }
                                            else { connection.assignments.insert(signal, terminal); }
                                        });
                                    }
                                }
                            }
                        }
                    }
                    div { class: "m1-pcb-module-rail-values",
                        for (signal, label) in [(VikSignal::V3v3, "3.3V supply"), (VikSignal::V5, "5V supply"), (VikSignal::Gnd, "Ground") ] {
                            label { key: "{label}", "{label} · V"
                                input { r#type: "number", step: "0.1", aria_label: "VIK {label} voltage", value: "{connection.rail_voltages.get(&signal).map(ToString::to_string).unwrap_or_default()}", disabled: !editable,
                                    oninput: move |event| {
                                        let raw = event.value();
                                        draft.with_mut(|value| if let Some(connection) = &mut value.connection {
                                            if raw.trim().is_empty() { connection.rail_voltages.remove(&signal); }
                                            else if let Ok(voltage) = raw.parse::<f64>() && voltage.is_finite() { connection.rail_voltages.insert(signal, voltage); }
                                        });
                                    }
                                }
                            }
                        }
                    }
                    label { "Supply budget · mA"
                        input { r#type: "number", min: "0", aria_label: "VIK supply current budget", value: "{connection.supply_current_ma.map(|value| value.to_string()).unwrap_or_default()}", disabled: !editable,
                            oninput: move |event| {
                                let raw = event.value();
                                draft.with_mut(|value| if let Some(connection) = &mut value.connection {
                                    connection.supply_current_ma = if raw.trim().is_empty() { None } else { raw.parse::<f64>().ok().filter(|number| number.is_finite() && *number >= 0.0) };
                                });
                            }
                        }
                    }
                }
            }
            if definition.circuit.is_some() {
                section { class: "m1-pcb-module-circuit", "aria-label": "Use circuit on PCB",
                    h3 { "Use circuit on PCB" }
                    p { "Creates an independent, editable copy. Choose host nets for explicit joins; other nets remain local to this copy." }
                    for port in circuit_nets.clone() {
                        label { key: "{port}", "{port.to_uppercase()} joins"
                            select {
                                aria_label: "Circuit {port} host net",
                                value: "{joins.read().get(&port).cloned().unwrap_or_default()}",
                                disabled: !editable,
                                onchange: move |event| {
                                    let net_id = event.value();
                                    joins.with_mut(|value| {
                                        if net_id.is_empty() { value.remove(&port); }
                                        else { value.insert(port.clone(), net_id); }
                                    });
                                },
                                option { value: "", "Separate local net" }
                                for (id, name) in &host_nets { option { value: "{id}", "{name}" } }
                            }
                        }
                    }
                    button { class: "m1-primary-button", r#type: "button", disabled: !editable || module_action_pending(tickets, "embed"), onclick: embed, "Copy circuit to PCB" }
                    for circuit in &embedded_circuits {
                        div { class: "m1-pcb-module-circuit-copy", key: "{circuit.id}",
                            span { "{circuit.part_ids.len()} components · {circuit.id.rsplit('/').next().unwrap_or(&circuit.id)}" }
                            button { r#type: "button", disabled: !editable || module_action_pending(tickets, &format!("remove/{}", circuit.id)), onclick: {
                                let circuit_id = circuit.id.clone();
                                let remove_runtime = input.runtime.clone();
                                let remove_scope = input.scope.clone();
                                let remove_module_id = input.module_id.clone();
                                let remove_context = input.selected_context;
                                let remove_token = input.snapshot.token;
                                let remove_revision = input.snapshot.document.revision;
                                move |_| {
                                    let Some(_snapshot) = mounted_owner_current(&remove_runtime, remove_context, &remove_scope, &remove_module_id, remove_token, remove_revision) else {
                                        feedback.set("The selected module or accepted project changed. Reopen its placement before removing the circuit copy.".into());
                                        return;
                                    };
                                    let action = format!("remove/{circuit_id}"); if module_action_pending(tickets, &action) { return; }
                                    let ticket = EditTicket::begin(&remove_runtime, "remove-embedded-circuit", Some("circuit".into()), module_resolver(remove_scope.clone(), remove_module_id.clone(), Ok(EditOperation::RemoveEmbeddedCircuit { id: circuit_id.clone() })));
                                    let mut tickets = tickets; let mut latest = latest; latest.set(Some(ticket.operation())); feedback.set(String::new()); tickets.write().push((action, ticket));
                                }
                            }, "Remove copy" }
                        }
                    }
                }
            }
            section { class: "m1-pcb-module-components", "aria-label": "Individual components",
                h3 { "Individual components" }
                p { "Place available source footprints independently. Component heights and assembly completeness need separate review." }
                if definition.circuit.is_some() {
                    for constituent in constituent_actions.clone() {
                        div { class: "m1-pcb-module-constituent", key: "{constituent.reference}-{constituent.index}",
                            div { strong { "{constituent.reference} · {constituent.name}" } small { if constituent.purchased { "Purchased assembly · footprint and external contacts only" } else { "{constituent.footprint}" } } }
                            button { r#type: "button", disabled: !editable || constituent.source_definition.is_none(), onclick: move |_| {
                                if let Some(definition) = constituent.source_definition.clone() {
                                    on_place_component.call(super::part_placement::ComponentPlacementAction::AddSourceObject {
                                        module_definition_id: constituent.module_definition_id.clone(),
                                        definition,
                                    });
                                }
                            }, "Place" }
                        }
                    }
                } else {
                    p { "This module has no editable component circuit source." }
                }
            }
            div { class: "m1-pcb-module-actions",
                button { class: "m1-primary-button", r#type: "button", disabled: !editable || draft().connection.as_ref().is_some_and(|connection| connection.host_connector_part_id.is_empty()), onclick: save, "Save placement" }
                button { class: "m1-danger-button", r#type: "button", disabled: !editable || module_action_pending(tickets, "remove-module"), onclick: remove, "Remove module" }
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

fn vik_signals() -> [(VikSignal, &'static str); 12] {
    [
        (VikSignal::Sclk, "SPI clock"),
        (VikSignal::Miso, "SPI MISO"),
        (VikSignal::Cs, "Chip select"),
        (VikSignal::Gpio2, "GPIO 2"),
        (VikSignal::Mosi, "SPI MOSI"),
        (VikSignal::Gpio1, "GPIO 1"),
        (VikSignal::V5, "5V supply"),
        (VikSignal::Rgb, "RGB data"),
        (VikSignal::Scl, "I²C clock"),
        (VikSignal::Sda, "I²C data"),
        (VikSignal::Gnd, "Ground"),
        (VikSignal::V3v3, "3.3V supply"),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectorAdmission {
    Reuse,
    CreateAutomatic,
    Reject,
}

fn connector_admission(
    connector_id: &str,
    automatic_id: &str,
    is_host_connector_on_board: bool,
    exists_in_document: bool,
) -> ConnectorAdmission {
    if is_host_connector_on_board {
        ConnectorAdmission::Reuse
    } else if connector_id == automatic_id && !exists_in_document {
        ConnectorAdmission::CreateAutomatic
    } else {
        ConnectorAdmission::Reject
    }
}

fn host_connector_is_on_board(
    document: &boardstudio_core::model::ProjectDoc,
    board_id: &str,
    connector_id: &str,
) -> bool {
    document.boards.iter().any(|board| {
        board.id == board_id && board.part_ids.iter().any(|part_id| part_id == connector_id)
    }) && document
        .parts
        .iter()
        .find(|part| part.id == connector_id)
        .and_then(|part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        })
        .and_then(|definition| definition.hardware_profile.as_ref())
        .is_some_and(|profile| profile.vik_role == Some(VikRole::Host))
}

fn mounted_owner_current(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    scope: &Scope,
    module_id: &str,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
) -> Option<AcceptedSnapshot> {
    if !mounted_selection_current(runtime, selected_context, scope, module_id) {
        return None;
    }
    let model = runtime.model();
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || !matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
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

fn mounted_selection_current(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    scope: &Scope,
    module_id: &str,
) -> bool {
    if runtime.scope().as_ref() != Some(scope) {
        return false;
    }
    let Some(selected) = selected_context.read().clone() else {
        return false;
    };
    if selected.scope != *scope
        || selected.context
            != (super::objects::TreeContext::MountedModule {
                board_id: scope.board_id.clone(),
                module_id: module_id.to_owned(),
            })
    {
        return false;
    }
    let model = runtime.model();
    super::selection::context_is_current(&model, scope, &selected.context)
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.document.id == scope.document_id
                && snapshot.session_epoch == scope.session_epoch
        })
}

fn module_action_pending(tickets: Signal<Vec<(String, EditTicket)>>, action: &str) -> bool {
    tickets
        .read()
        .iter()
        .any(|(kind, ticket)| kind == action && ticket.is_pending())
}

fn module_resolver(
    scope: Scope,
    module_id: String,
    operation: Result<EditOperation, String>,
) -> EditResolver {
    EditResolver::new("pcb-module", move |accepted: &AcceptedSnapshot| {
        if accepted.session_epoch != scope.session_epoch
            || accepted.document.id != scope.document_id
        {
            return Resolution::Retire(boardstudio_application::DOCUMENT_SESSION_CHANGED.into());
        }
        let operation = match &operation {
            Ok(operation) => operation,
            Err(reason) => return Resolution::Retire(reason.clone()),
        };
        let Some(module) = accepted
            .document
            .modules
            .iter()
            .find(|module| module.id == module_id && module.host_board_id == scope.board_id)
        else {
            return Resolution::Retire(
                "The mounted module was deleted or moved to another board.".into(),
            );
        };
        match operation {
            EditOperation::SetMountedModule { instance, .. } => {
                if module.definition_id != instance.definition_id
                    || !accepted
                        .document
                        .boards
                        .iter()
                        .any(|board| board.id == instance.host_board_id)
                {
                    return Resolution::Retire(
                        "The module source or destination PCB changed.".into(),
                    );
                }
                if module == instance.as_ref() {
                    return Resolution::Unchanged;
                }
            }
            EditOperation::EmbedModuleCircuit {
                definition,
                host_board_id,
                joins,
                ..
            } => {
                if module.definition_id != definition.id
                    || !accepted
                        .document
                        .module_definitions
                        .iter()
                        .any(|current| current == definition)
                    || !accepted.document.boards.iter().any(|board| {
                        &board.id == host_board_id
                            && joins.values().all(|net| board.net_ids.contains(net))
                    })
                {
                    return Resolution::Retire(
                        "The module circuit source or destination connections changed.".into(),
                    );
                }
            }
            EditOperation::RemoveEmbeddedCircuit { id }
                if !accepted.document.embedded_circuits.iter().any(|circuit| {
                    &circuit.id == id && circuit.host_board_id == scope.board_id
                }) =>
            {
                return Resolution::Retire("The circuit copy was deleted or moved.".into());
            }
            _ => {}
        }
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![module_id.clone()],
            operation: operation.clone(),
        })
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::{ConnectorAdmission, connector_admission};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn connector_admission_reuses_local_host_and_rejects_foreign_automatic_ids() {
        let automatic_id = "module-a/vik-host-connector";

        assert_eq!(
            connector_admission("host-on-board", automatic_id, true, true),
            ConnectorAdmission::Reuse
        );
        assert_eq!(
            connector_admission(automatic_id, automatic_id, false, true),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission("wrong-role", automatic_id, false, true),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission("missing-host", automatic_id, false, false),
            ConnectorAdmission::Reject
        );
        assert_eq!(
            connector_admission(automatic_id, automatic_id, false, false),
            ConnectorAdmission::CreateAutomatic
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_save_replacement_tests {
    use super::*;
    use boardstudio_application::{
        Completion, Effect, OperationId, SaveResult, Scope as AppScope, Session,
    };
    use boardstudio_core::{CoreEngine, model::*};
    use gloo_timers::future::TimeoutFuture;
    use std::{collections::VecDeque, rc::Rc};
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;
    use web_sys::HtmlInputElement;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    struct Probe {
        runtime: Rc<Runtime>,
        initial_scope: AppScope,
        module_id: String,
    }

    fn mounted_editor_host() -> dioxus::prelude::Element {
        let probe = use_context::<Rc<Probe>>();
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        use_hook(|| {
            probe.runtime.subscribe(Rc::new(move || {
                let mut version = version;
                version.set(version() + 1);
            }));
        });
        let _ = version();
        let selected_context = use_signal(|| {
            Some(super::super::objects::ScopedTreeContext {
                scope: probe.initial_scope.clone(),
                context: super::super::objects::TreeContext::MountedModule {
                    board_id: probe.initial_scope.board_id.clone(),
                    module_id: probe.module_id.clone(),
                },
            })
        });
        let model = probe.runtime.model();
        let Some(snapshot) = model.accepted else {
            return rsx! { p { "No accepted project" } };
        };
        let Some(scope) = probe.runtime.scope() else {
            return rsx! { p { "No current project scope" } };
        };
        if scope != probe.initial_scope {
            return rsx! {
                p { id: "replacement-project", "{snapshot.document.name}" }
            };
        }
        super::inspector(InspectorInput {
            runtime: probe.runtime.clone(),
            snapshot,
            scope,
            module_id: probe.module_id.clone(),
            selected_context,
            on_place_component: EventHandler::default(),
        })
    }

    fn module_document(id: &str, name: &str, x: f64) -> ProjectDoc {
        let mut document = ProjectDoc::empty(id, name);
        document.boards.push(Board {
            id: "board-collision".into(),
            name: "Main board".into(),
            outline_ids: Vec::new(),
            part_ids: Vec::new(),
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        document.module_definitions.push(ModuleDefinition {
            id: "definition-collision".into(),
            catalogue_row: None,
            name: "Test module".into(),
            family: "test".into(),
            variant: "test".into(),
            source: HardwareSource {
                repository: "test".into(),
                revision: "1".into(),
                path: "test.module.json".into(),
                license: "test".into(),
                sha256: None,
                upstream_status: None,
            },
            board: ModuleBoard {
                contours: Vec::new(),
                holes: Vec::new(),
                thickness: Some(1.6),
            },
            mounts: Vec::new(),
            volumes: Vec::new(),
            openings: Vec::new(),
            models: Vec::new(),
            candidate_models: Vec::new(),
            gates: Vec::new(),
            interfaces: Vec::new(),
            electrical: ModuleElectrical {
                protocol: ModuleProtocol::Nonstandard,
                required_signals: Vec::new(),
                logic_voltage: None,
                current_ma: None,
                i2c_address: None,
                pullup_ohms: None,
                driver: None,
                rotary_profile: None,
            },
            constituents: Vec::new(),
            circuit: None,
        });
        document.modules.push(MountedModule {
            id: "placement-collision".into(),
            definition_id: "definition-collision".into(),
            host_board_id: "board-collision".into(),
            host_instance_id: None,
            host_face: Side::Front,
            facing_face: Side::Front,
            at: Vec2 { x, y: 2.0 },
            rotation: 0.0,
            gap: 1.0,
            attachment: ModuleAttachment::Board,
            detached: false,
            connection: None,
            service_clearance: 0.0,
            mount_supports: Vec::new(),
        });
        document
    }

    fn accepted(document: ProjectDoc) -> (Session, CoreEngine) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(boardstudio_application::Event::Open {
            operation_id: OperationId(1),
            document,
        });
        let mut effects = VecDeque::from(effects);
        while let Some(effect) = effects.pop_front() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => effects.extend(session.complete(Completion::Core {
                    request_id,
                    executor_epoch,
                    reply: Box::new(core.handle(*request)),
                })),
                Effect::Persist {
                    save_attempt_id, ..
                } => {
                    effects.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
        assert!(session.read_model().accepted.is_some());
        (session, core)
    }

    async fn settle() {
        TimeoutFuture::new(80).await;
    }

    async fn with_stage_timeout<T>(future: impl std::future::Future<Output = T>, stage: &str) -> T {
        let mut future = Box::pin(future);
        let mut timeout = Box::pin(TimeoutFuture::new(5_000));
        let completed = std::future::poll_fn(|context| {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(context) {
                return std::task::Poll::Ready(Some(value));
            }
            if let std::task::Poll::Ready(()) = timeout.as_mut().poll(context) {
                return std::task::Poll::Ready(None);
            }
            std::task::Poll::Pending
        })
        .await;
        completed.unwrap_or_else(|| panic!("timed out during mounted Save stage: {stage}"))
    }

    async fn wait_for_persisted_module_x(
        runtime: &Runtime,
        expected_revision: u64,
        expected_x: f64,
    ) -> ProjectDoc {
        for _ in 0..100 {
            if let Some(document) = with_stage_timeout(
                runtime.store.load_document("old-module-project".into()),
                "IndexedDB read request/transaction completion",
            )
            .await
            .expect("read old project from IndexedDB")
                && document.revision == expected_revision
                && document.modules[0].at.x == expected_x
            {
                return document;
            }
            TimeoutFuture::new(10).await;
        }
        panic!(
            "old-project IndexedDB save did not reach revision {expected_revision} with X={expected_x}"
        );
    }

    #[wasm_bindgen_test]
    async fn placement_fields_queue_two_commits_keep_the_draft_and_undo_in_order() {
        use crate::runtime::project_name_test_support as support;
        let (session, core) = accepted(module_document(
            "module-queue-project",
            "Queue project",
            1.0,
        ));
        let runtime = support::new_runtime();
        support::install(&runtime, session, core);
        let probe = Rc::new(Probe {
            initial_scope: runtime.scope().unwrap(),
            runtime: runtime.clone(),
            module_id: "placement-collision".into(),
        });
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_editor_host);
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;
        let x = root
            .query_selector("input[type='number']")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        let save = root
            .query_selector(".m1-pcb-module-actions button.m1-primary-button")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let input_event = web_sys::EventInit::new();
        input_event.set_bubbles(true);
        x.set_value("7.25");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
            .unwrap();
        settle().await;
        save.click();
        settle().await;
        support::drive_pending(&runtime);
        with_stage_timeout(entered, "first placement Core gate")
            .await
            .unwrap();
        settle().await;
        assert!(!save.has_attribute("disabled"));
        assert!(!x.disabled());
        assert_eq!(x.value(), "7.25");
        x.set_value("9.5");
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
            .unwrap();
        settle().await;
        save.click();
        settle().await;
        assert_eq!(x.value(), "9.5");
        release.send(()).unwrap();
        for _ in 0..12 {
            support::run_pending(&runtime).await;
            settle().await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.modules[0].at.x,
            9.5
        );
        assert_eq!(x.value(), "9.5");
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        for _ in 0..8 {
            support::run_pending(&runtime).await;
            settle().await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.modules[0].at.x,
            7.25
        );
        assert_eq!(x.value(), "7.25");
        runtime.submit(boardstudio_application::Event::Undo {
            operation_id: runtime.operation(),
        });
        for _ in 0..8 {
            support::run_pending(&runtime).await;
            settle().await;
        }
        assert_eq!(
            runtime.model().accepted.unwrap().document.modules[0].at.x,
            1.0
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn failed_placement_restores_accepted_coordinates() {
        use crate::runtime::project_name_test_support as support;
        for preserve_draft in [false, true] {
            let runtime = support::new_runtime();
            support::open_document(
                &runtime,
                module_document("module-failure-project", "Queue project", 1.0),
            )
            .await;
            let probe = Rc::new(Probe {
                initial_scope: runtime.scope().unwrap(),
                runtime: runtime.clone(),
                module_id: "placement-collision".into(),
            });
            let document = web_sys::window().unwrap().document().unwrap();
            let root = document.create_element("div").unwrap();
            document.body().unwrap().append_child(&root).unwrap();
            let dom = VirtualDom::new(mounted_editor_host);
            dom.provide_root_context(probe);
            dioxus_web::launch::launch_virtual_dom(
                dom,
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            settle().await;
            let x = root
                .query_selector("input[type='number']")
                .unwrap()
                .unwrap()
                .dyn_into::<HtmlInputElement>()
                .unwrap();
            let save = root
                .query_selector(".m1-pcb-module-actions button.m1-primary-button")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            support::fail_next_core_reply(&runtime, "injected placement refusal");
            let input_event = web_sys::EventInit::new();
            input_event.set_bubbles(true);
            x.set_value("7.25");
            x.dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
            )
            .unwrap();
            settle().await;
            save.click();
            settle().await;
            if preserve_draft {
                x.set_value("8");
                x.dispatch_event(
                    &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
                )
                .unwrap();
                settle().await;
            }
            for _ in 0..12 {
                support::run_pending(&runtime).await;
                settle().await;
            }
            assert_eq!(
                runtime.model().accepted.unwrap().document.modules[0].at.x,
                1.0
            );
            assert_eq!(
                x.value(),
                if preserve_draft { "8" } else { "1" },
                "failure restores the committed draft and preserves later typing"
            );
            let text = root.text_content().unwrap();
            assert!(
                text.contains("injected placement refusal"),
                "mounted text: {text}"
            );
            runtime.unsubscribe();
            root.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_save_persist_settlement_cannot_cross_replacement_with_colliding_ids() {
        let (session, core) = accepted(module_document("old-module-project", "Old project", 1.0));
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::install(&runtime, session, core);
        let initial = runtime.model().accepted.expect("old project accepted");
        runtime
            .store
            .save_document(&initial.document, &std::collections::BTreeMap::new())
            .await
            .expect("seed the old project's durable baseline");
        let initial_scope = runtime.scope().expect("old project has scope");
        let probe = Rc::new(Probe {
            runtime: runtime.clone(),
            initial_scope,
            module_id: "placement-collision".into(),
        });
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("mounted-module-save-replacement-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_editor_host);
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        settle().await;

        let (entered, release) =
            crate::runtime::project_name_test_support::gate_next_persist(&runtime);
        let operation = crate::runtime::project_name_test_support::observe_next(&runtime);
        let x = root
            .query_selector("input[type='number']")
            .unwrap()
            .expect("mounted placement X input exists")
            .dyn_into::<HtmlInputElement>()
            .unwrap();
        x.focus().unwrap();
        x.set_value("7.25");
        let input = web_sys::EventInit::new();
        input.set_bubbles(true);
        x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input).unwrap())
            .unwrap();
        settle().await;
        assert_eq!(
            root.query_selector("input[type='number']")
                .unwrap()
                .expect("placement X input remains mounted")
                .dyn_into::<HtmlInputElement>()
                .unwrap()
                .value(),
            "7.25",
            "the mounted X field retains the dispatched draft value after rerender"
        );
        root.query_selector("button.m1-primary-button")
            .unwrap()
            .expect("Save placement button exists")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        settle().await;
        assert!(
            root.query_selector("button.m1-primary-button")
                .unwrap()
                .expect("Save placement button remains mounted")
                .has_attribute("disabled")
                == false,
            "placement fields queue freely while their earlier ticket is pending"
        );
        assert_eq!(
            root.query_selector("input[type='number']")
                .unwrap()
                .expect("placement X input remains mounted during Save")
                .dyn_into::<HtmlInputElement>()
                .unwrap()
                .value(),
            "7.25",
            "the mounted draft survives the Save handler's rerender"
        );
        crate::runtime::project_name_test_support::drive_pending(&runtime);
        with_stage_timeout(
            entered,
            "mounted Save click dispatched but actual Persist gate was not entered",
        )
        .await
        .expect("the mounted Save reaches the actual Session Persist effect");

        let saving = runtime.model();
        let saving_snapshot = saving
            .accepted
            .expect("old project remains accepted while saving");
        assert_eq!(saving.lifecycle, Lifecycle::Saving);
        assert_eq!(saving_snapshot.document.id, "old-module-project");
        assert_eq!(
            saving_snapshot.document.modules[0].at.x, 1.0,
            "the accepted read model remains the last durable document while Persist is pending"
        );
        let saved_revision = saving_snapshot.document.revision + 1;

        let (replacement, _replacement_core) = accepted(module_document(
            "new-module-project",
            "Replacement project",
            4.0,
        ));
        crate::runtime::project_name_test_support::replace_session(&runtime, replacement);
        settle().await;
        assert_eq!(
            root.query_selector("#replacement-project")
                .unwrap()
                .expect("replacement owner retires the old mounted editor")
                .text_content()
                .as_deref(),
            Some("Replacement project")
        );
        assert!(
            root.query_selector(".m1-pcb-module-inspector")
                .unwrap()
                .is_none(),
            "old placement feedback/editor is absent in the replacement owner"
        );
        let replacement_before = runtime.model().accepted.expect("replacement accepted");
        assert_eq!(replacement_before.document.id, "new-module-project");
        assert_eq!(replacement_before.document.modules[0].at.x, 4.0);
        let revision = replacement_before.document.revision;

        release
            .send(())
            .expect("release the old project persistence");
        let persisted = wait_for_persisted_module_x(&runtime, saved_revision, 7.25).await;
        assert_eq!(persisted.id, "old-module-project");
        assert_eq!(persisted.revision, saved_revision);
        assert_eq!(persisted.modules[0].at.x, 7.25);
        let replacement_after = runtime
            .model()
            .accepted
            .expect("replacement remains accepted");
        assert_eq!(replacement_after.document.id, "new-module-project");
        assert_eq!(replacement_after.document.revision, revision);
        assert_eq!(replacement_after.document.modules[0].at.x, 4.0);
        assert!(
            operation.borrow().is_none(),
            "after the old IndexedDB write completes, the replacement Session has no old-operation feedback"
        );
        assert!(
            !root
                .text_content()
                .unwrap_or_default()
                .contains("Placement saved.")
        );
        let _ = runtime
            .store
            .delete_project("old-module-project".into())
            .await;
        let _ = document.body().unwrap().remove_child(&root);
        runtime.unsubscribe();
    }
}
