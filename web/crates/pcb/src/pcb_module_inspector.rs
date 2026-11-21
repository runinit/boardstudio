//! Private editor for the placement of one accepted mounted-module instance.
//! Module source definitions and their footprint/circuit ownership remain untouched.
use crate::pcb_controls::{PcbCheck, PcbChoice, PcbInput};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_model::model::{
    EditOperation, ModuleAttachment, ModuleConnection, ModuleSupport, MountedModule,
    PartDefinition, Side, VikRole, VikSignal,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::input::InputType;
use dioxus::prelude::*;
use std::rc::Rc;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
};
use wasm_bindgen_futures::spawn_local;

/// One mounted-module inspector action's bounded logical key.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ModuleEditKey {
    Placement,
    RemoveModule,
    Embed,
    RemoveCircuit,
}

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
    let mut draft_dirty = use_signal(|| false);
    let mut feedback = use_signal(String::new);
    let mut support_draft = use_signal(SupportDraft::default);
    let mut joins = use_signal(BTreeMap::<String, String>::new);
    // Typed placement and bounded action kinds share submitted-value restoration,
    // inline failures, one-shot disabling and binding lifetime in one helper.
    let edits =
        use_hook(PendingEditSignals::<ModuleEditKey, boardstudio_model::model::MountedModule>::new);
    let placement_failure = use_signal(|| None::<String>);
    let remove_disabled = use_signal(|| false);
    let embed_disabled = use_signal(|| false);
    let circuit_removal_disabled = use_signal(|| false);
    edits.bind_field(ModuleEditKey::Placement, draft, placement_failure);
    edits.bind_one_shot(ModuleEditKey::RemoveModule, remove_disabled);
    edits.bind_one_shot(ModuleEditKey::Embed, embed_disabled);
    edits.bind_one_shot(ModuleEditKey::RemoveCircuit, circuit_removal_disabled);
    let latest = use_signal(|| None::<ModuleEditKey>);
    let save_queue = use_hook(|| {
        Rc::new(RefCell::new(VecDeque::<(
            boardstudio_model::model::MountedModule,
            bool,
        )>::new()))
    });
    let preparing = use_hook(|| Rc::new(Cell::new(false)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        let edits = edits.clone();
        move || {
            alive.set(false);
            edits.unbind_field(&ModuleEditKey::Placement);
            edits.unbind_one_shot(&ModuleEditKey::RemoveModule);
            edits.unbind_one_shot(&ModuleEditKey::Embed);
            edits.unbind_one_shot(&ModuleEditKey::RemoveCircuit);
        }
    });
    let version = use_context::<Signal<u64>>()();
    use_effect(use_reactive((&version,), {
        let runtime = input.runtime.clone();
        let scope = input.scope.clone();
        let module_id = input.module_id.clone();
        let selected_context = input.selected_context;
        let edits = edits.clone();
        let mut feedback = feedback;
        let preparing = preparing.clone();
        move |_| {
            let panel_is_live =
                mounted_selection_current(&runtime, selected_context, &scope, &module_id);
            let accepted_module = runtime.model().accepted.and_then(|accepted| {
                accepted
                    .document
                    .modules
                    .iter()
                    .find(|module| module.id == module_id)
                    .cloned()
            });
            for result in edits.settle(panel_is_live, |_| {
                accepted_module
                    .clone()
                    .unwrap_or_else(|| draft.peek().clone())
            }) {
                let (key, message) = match result {
                    PendingEditResult::Failed { key, message } => (key, Some(message)),
                    PendingEditResult::Landed { key, .. } | PendingEditResult::Retired { key } => {
                        (key, None)
                    }
                };
                // The helper owns submitted-value restoration. Once its placement
                // drain leaves the live draft at accepted, ordinary accepted projection
                // can resume; a newer user draft keeps its dirty marker.
                if key == ModuleEditKey::Placement
                    && panel_is_live
                    && !edits.is_pending(&ModuleEditKey::Placement)
                    && !preparing.get()
                    && accepted_module
                        .as_ref()
                        .is_some_and(|accepted| *draft.peek() == *accepted)
                {
                    draft_dirty.set(false);
                }
                // Placement's bound field owns its inline failure; action failures keep
                // their existing panel placement and latest-action attribution.
                if key != ModuleEditKey::Placement && latest.peek().as_ref() == Some(&key) {
                    feedback.set(message.unwrap_or_default());
                }
            }
        }
    }));
    let accepted_instance = instance.clone();
    use_effect(use_reactive((&accepted_instance,), {
        let edits = edits.clone();
        let preparing = preparing.clone();
        move |(accepted_instance,)| {
            // Value equality with a previous accepted value cannot identify a clean
            // draft: a newer edit may have returned to that value while a save waited.
            if !draft_dirty() {
                draft.set(accepted_instance);
            } else if !edits.is_pending(&ModuleEditKey::Placement)
                && !preparing.get()
                && *draft.peek() == accepted_instance
            {
                draft_dirty.set(false);
            }
        }
    }));
    let module_name = definition.name.clone();
    let automatic_connector_id = format!("{}/vik-host-connector", instance.id);
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let save_alive = alive.clone();
    let save_edits = edits.clone();
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
        let save_edits = save_edits.clone();
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
                let submitted = value.clone();
                let operation =
                    connector.map(
                        |host_connector_definition| EditOperation::SetMountedModule {
                            instance: Box::new(value),
                            definition: None,
                            host_connector_definition,
                        },
                    );
                let key = ModuleEditKey::Placement;
                latest.set(Some(key.clone()));
                feedback.set(String::new());
                save_edits.begin_value(
                    &runtime,
                    key,
                    "mounted-module-placement",
                    Some("placement".into()),
                    module_resolver(scope.clone(), module_id.clone(), operation),
                    &submitted,
                );
            }
        });
    };
    let scope = input.scope.clone();
    let module_id = input.module_id.clone();
    let owner_token = input.snapshot.token;
    let owner_revision = input.snapshot.document.revision;
    let runtime = input.runtime.clone();
    let selected_context = input.selected_context;
    let remove_actions = edits.clone();
    let remove = move |_| {
        if remove_actions.is_pending(&ModuleEditKey::RemoveModule)
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
        let key = ModuleEditKey::RemoveModule;
        let mut latest = latest;
        latest.set(Some(key.clone()));
        feedback.set(String::new());
        remove_actions.begin_one_shot(
            &runtime,
            key,
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
    let embed_actions = edits.clone();
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
        if embed_actions.is_pending(&ModuleEditKey::Embed) {
            return;
        }
        let seed = embed_runtime.operation().0;
        let id = format!("circuit/embedded-{seed}");
        let placement = draft();
        let key = ModuleEditKey::Embed;
        let mut latest = latest;
        latest.set(Some(key.clone()));
        feedback.set(String::new());
        embed_actions.begin_one_shot(
            &embed_runtime,
            key,
            "embed-module-circuit",
            Some("circuit".into()),
            module_resolver(
                embed_scope.clone(),
                embed_module_id.clone(),
                Ok(EditOperation::EmbedModuleCircuit {
                    id,
                    definition: embed_source.clone(),
                    host_board_id: placement.host_board_id,
                    pose: boardstudio_model::model::Pose2 {
                        at: placement.at,
                        rotation: placement.rotation,
                    },
                    side: placement.host_face,
                    joins: embed_joins.read().clone(),
                }),
            ),
        );
    };

    rsx! {
        section { class: "grid gap-2.5 min-w-0 overflow-auto p-4 text-foreground [&_h2]:m-0 [&_h2]:text-lg [&_p]:m-0 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground", "aria-label": "Mounted module placement",
            h2 { "{module_name} placement" }
            p { "Placement edits affect this project instance. Source footprint artwork remains owned by the module." }
            PcbChoice {
                label: "Host board",
                aria_label: "Module host board",
                value: "{draft().host_board_id}",
                disabled: !editable,
                searchable: true,
                on_change: move |next: String| {
                        edit_module_draft(draft, draft_dirty, |value| {
                            value.host_board_id = next.clone();
                            if value.host_instance_id.as_ref().is_some_and(|id| !instances.iter().any(|instance| instance.id == *id && instance.board_id == next)) {
                                value.host_instance_id = None;
                            }
                        });
                    },
                options: boards.iter().map(|board| (board.id.clone(), board.name.clone(), false)).collect(),
            }
            PcbChoice {
                label: "Physical instance",
                aria_label: "Module physical instance",
                value: "{draft().host_instance_id.clone().unwrap_or_default()}",
                disabled: !editable,
                searchable: true,
                on_change: move |id: String| {
                        edit_module_draft(draft, draft_dirty, |value| value.host_instance_id = (!id.is_empty()).then_some(id));
                    },
                options: std::iter::once((String::new(), "Board-wide".into(), false)).chain(options.iter().map(|(id, name)| (id.clone(), name.clone(), false))).collect(),
            }
            PcbChoice {
                label: "Mounted face",
                aria_label: "Module host face",
                value: (if draft().host_face == Side::Front { "front" } else { "back" }).to_string(),
                disabled: !editable,
                on_change: move |next: String| edit_module_draft(draft, draft_dirty, |value| value.host_face = side(&next)),
                options: vec![("front".into(), "Front".into(), false), ("back".into(), "Back".into(), false)],
            }
            PcbChoice {
                label: "Facing face",
                aria_label: "Module facing face",
                value: (if draft().facing_face == Side::Front { "front" } else { "back" }).to_string(),
                disabled: !editable,
                on_change: move |next: String| edit_module_draft(draft, draft_dirty, |value| value.facing_face = side(&next)),
                options: vec![("front".into(), "Front".into(), false), ("back".into(), "Back".into(), false)],
            }
            PcbInput {
                label: "X (mm)", aria_label: "X (mm)", r#type: InputType::Number,
                step: Some("any".into()),
                value: "{draft().at.x}",
                disabled: !editable,
                oninput: move |event: FormEvent| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { edit_module_draft(draft, draft_dirty, |value| value.at.x = number); },
            }
            PcbInput {
                label: "Y (mm)", aria_label: "Y (mm)", r#type: InputType::Number,
                step: Some("any".into()),
                value: "{draft().at.y}",
                disabled: !editable,
                oninput: move |event: FormEvent| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { edit_module_draft(draft, draft_dirty, |value| value.at.y = number); },
            }
            PcbInput {
                label: "Yaw (degrees)", aria_label: "Yaw (degrees)", r#type: InputType::Number,
                step: Some("any".into()),
                value: "{draft().rotation}",
                disabled: !editable,
                oninput: move |event: FormEvent| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() { edit_module_draft(draft, draft_dirty, |value| value.rotation = number); },
            }
            PcbInput {
                label: "Gap (mm)", aria_label: "Gap (mm)", r#type: InputType::Number,
                min: Some("0".into()),
                step: Some("any".into()),
                value: "{draft().gap}",
                disabled: !editable,
                oninput: move |event: FormEvent| if let Ok(number) = event.value().parse::<f64>() && number.is_finite() && number >= 0.0 { edit_module_draft(draft, draft_dirty, |value| value.gap = number); },
            }
            PcbChoice {
                label: "Attachment",
                aria_label: "Module attachment",
                value: (if draft().attachment == ModuleAttachment::Board { "board" } else { "case" }).to_string(),
                disabled: !editable,
                on_change: move |next: String| edit_module_draft(draft, draft_dirty, |value| value.attachment = if next == "case" { ModuleAttachment::Case } else { ModuleAttachment::Board }),
                options: vec![("board".into(), "Board".into(), false), ("case".into(), "Case".into(), false)],
            }
            PcbInput {
                label: "Extra service clearance (mm)",
                r#type: InputType::Number, min: Some("0".into()),
                step: Some("0.1".into()),
                aria_label: "Module service clearance",
                value: "{draft().service_clearance}",
                disabled: !editable,
                oninput: move |event: FormEvent| {
                        let raw = event.value();
                        let number = if raw.trim().is_empty() { Some(0.0) } else { raw.parse::<f64>().ok() };
                        if let Some(number) = number.filter(|number| number.is_finite() && *number >= 0.0) {
                            edit_module_draft(draft, draft_dirty, |value| value.service_clearance = number);
                        }
                    },
            }
            section { class: "grid gap-2 min-w-0 p-2.5 border border-border rounded-md bg-card [&_h3]:m-0 [&_h3]:text-xs [&_h3]:text-foreground", "aria-label": "{support_section}",
                h3 { "{support_section}" }
                p { "Choose a source PCB hole and enter designer-selected ring dimensions. Z and height are module-midplane millimetres; these values are not vendor specifications." }
                for (index, support) in draft().mount_supports.iter().enumerate() {
                    div { key: "{support.mount_id}", class: "flex items-center justify-between gap-2 py-1.5 border-b border-border text-foreground text-xs [&_button]:flex-none",
                        span { "{support.mount_id} · OD {support.outer_diameter} / ID {support.hole_diameter} · Z {support.z} · height {support.height} mm" }
                        Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable, aria_label: "Remove support {support.mount_id}", onclick: move |_| {
                            edit_module_draft(draft, draft_dirty, |value| { value.mount_supports.remove(index); });
                        }, "Remove" }
                    }
                }
                if !resolved_supports.is_empty() {
                    div { class: "p-2 border-l-2 border-primary bg-canvas [&_strong]:block [&_strong]:mb-1 [&_strong]:text-foreground [&_strong]:text-xs", "aria-label": "Resolved support geometry",
                        strong { "Resolved {support_section}" }
                        for (index, support) in resolved_supports.iter().enumerate() {
                            p { key: "{support.mount_id}-{index}", "{support.mount_id} · center {support.at.x:.2}, {support.at.y:.2} mm · OD {support.outer_diameter:.2} / ID {support.hole_diameter:.2} mm · Z {support.z:.2} · height {support.height:.2} mm" }
                        }
                    }
                }
                PcbChoice {
                    label: "Source mounting hole",
                    aria_label: "Support source mounting hole",
                    value: "{support_draft().mount_id}",
                    disabled: !editable || !has_source_mounts,
                    searchable: true,
                    on_change: move |next: String| support_draft.with_mut(|value| value.mount_id = next),
                    options: std::iter::once((String::new(), "Choose module hole…".into(), false)).chain(source_mounts.iter().map(|mount| (mount.source_id.clone(), format!("{} · source drill {} mm", mount.source_id, mount.diameter), false))).collect(),
                }
                div { class: "grid grid-cols-2 gap-2",
                    PcbInput {
                        label: "Outer diameter (mm)",
                        r#type: InputType::Number, step: Some("0.1".into()),
                        aria_label: "Support outer diameter",
                        value: "{support_draft().outer_diameter}",
                        disabled: !editable,
                        oninput: move |event: FormEvent| support_draft.with_mut(|value| value.outer_diameter = event.value()),
                    }
                    PcbInput {
                        label: "Hole diameter (mm)",
                        r#type: InputType::Number, step: Some("0.1".into()),
                        aria_label: "Support hole diameter",
                        value: "{support_draft().hole_diameter}",
                        disabled: !editable,
                        oninput: move |event: FormEvent| support_draft.with_mut(|value| value.hole_diameter = event.value()),
                    }
                    PcbInput {
                        label: "Z from midplane (mm)",
                        r#type: InputType::Number, step: Some("0.1".into()),
                        aria_label: "Support Z from midplane",
                        value: "{support_draft().z}",
                        disabled: !editable,
                        oninput: move |event: FormEvent| support_draft.with_mut(|value| value.z = event.value()),
                    }
                    PcbInput {
                        label: "Ring height (mm)",
                        r#type: InputType::Number, step: Some("0.1".into()),
                        aria_label: "Support ring height",
                        value: "{support_draft().height}",
                        disabled: !editable,
                        oninput: move |event: FormEvent| support_draft.with_mut(|value| value.height = event.value()),
                    }
                }
                Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable || !has_source_mounts, onclick: move |_| {
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
                    edit_module_draft(draft, draft_dirty, |value| value.mount_supports.push(ModuleSupport {
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
            PcbCheck { label: "Detached", checked: draft().detached, disabled: !editable,
                on_change: move |checked| edit_module_draft(draft, draft_dirty, |value| value.detached = checked)
            }
            section { "aria-label": "VIK connection",
                h3 { "VIK connection" }
                PcbCheck { label: "Assign host connection",
                    checked: draft().connection.is_some(),
                    disabled: !editable,
                    on_change: move |checked| {
                        if checked {
                            edit_module_draft(draft, draft_dirty, |value| value.connection = Some(ModuleConnection {
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
                            edit_module_draft(draft, draft_dirty, |value| value.connection = None);
                        }
                    }
                }
                if let Some(connection) = draft().connection {
                    PcbChoice {
                        label: "Host connector",
                        aria_label: "Module host connector",
                        value: "{connection.host_connector_part_id}",
                        disabled: !editable,
                        searchable: true,
                        on_change: move |next: String| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.host_connector_part_id = next; }),
                        options: std::iter::once((String::new(), "Select VIK host connector".into(), false))
                            .chain((connectors.is_empty() && automatic_connector_can_be_created).then(|| (automatic_connector_id.clone(), "Add source-backed horizontal VIK connector beside module".into(), false)))
                            .chain(connectors.iter().map(|(id, reference)| (id.clone(), reference.clone(), false))).collect(),
                    }
                    if connectors.is_empty() {
                        if automatic_connector_can_be_created {
                            p { "No source-backed host-role VIK connector is present on this board. Add one before saving a connected placement." }
                        } else {
                            p { "The automatic connector ID is already used by a part outside this board. Choose a valid host connector or clear the connection." }
                        }
                    }
                    PcbChoice {
                        label: "Module port",
                        aria_label: "Module input port",
                        value: "{connection.module_port_id}",
                        disabled: !editable,
                        searchable: true,
                        on_change: move |next: String| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.module_port_id = next; }),
                        options: definition.interfaces.iter().filter(|port| port.role == VikRole::Module).map(|port| (port.id.clone(), port.id.clone(), false)).collect(),
                    }
                    PcbInput { label: "Bus name", r#type: InputType::Text, aria_label: "Module bus name", value: "{connection.bus_id}", disabled: !editable,
                        oninput: move |event: FormEvent| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.bus_id = event.value(); })
                    }
                    p { "12 contacts · 0.5 mm pitch · Type A cable · 3.3 V logic. Enter actual MCU terminals; shared buses are checked by their wiring." }
                    div {
                        for (signal, label) in vik_signals() {
                            PcbInput { key: "{label}", label: label.to_owned(), r#type: InputType::Text, aria_label: format!("VIK {label} terminal"), value: "{connection.assignments.get(&signal).cloned().unwrap_or_default()}", disabled: !editable,
                                    oninput: move |event: FormEvent| {
                                        let terminal = event.value().trim().to_owned();
                                        edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection {
                                            if terminal.is_empty() { connection.assignments.remove(&signal); }
                                            else { connection.assignments.insert(signal, terminal); }
                                        });
                                    }
                            }
                        }
                    }
                    div {
                        for (signal, label) in [(VikSignal::V3v3, "3.3V supply"), (VikSignal::V5, "5V supply"), (VikSignal::Gnd, "Ground") ] {
                            PcbInput { key: "{label}", label: format!("{label} · V"), r#type: InputType::Number, step: Some("0.1".into()), aria_label: format!("VIK {label} voltage"), value: "{connection.rail_voltages.get(&signal).map(ToString::to_string).unwrap_or_default()}", disabled: !editable,
                                    oninput: move |event: FormEvent| {
                                        let raw = event.value();
                                        edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection {
                                            if raw.trim().is_empty() { connection.rail_voltages.remove(&signal); }
                                            else if let Ok(voltage) = raw.parse::<f64>() && voltage.is_finite() { connection.rail_voltages.insert(signal, voltage); }
                                        });
                                    }
                            }
                        }
                    }
                    PcbInput {
                        label: "Supply budget · mA",
                        r#type: InputType::Number, min: Some("0".into()),
                        aria_label: "VIK supply current budget",
                        value: "{connection.supply_current_ma.map(|value| value.to_string()).unwrap_or_default()}",
                        disabled: !editable,
                        oninput: move |event: FormEvent| {
                                let raw = event.value();
                                edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection {
                                    connection.supply_current_ma = if raw.trim().is_empty() { None } else { raw.parse::<f64>().ok().filter(|number| number.is_finite() && *number >= 0.0) };
                                });
                            },
                        step: Some("1".into()),
                    }
                }
            }
            if definition.circuit.is_some() {
                section { "aria-label": "Use circuit on PCB",
                    h3 { "Use circuit on PCB" }
                    p { "Creates an independent, editable copy. Choose host nets for explicit joins; other nets remain local to this copy." }
                    for port in circuit_nets.clone() {
                        PcbChoice { key: "{port}", label: format!("{} joins", port.to_uppercase()),
                                aria_label: format!("Circuit {port} host net"), searchable: true,
                                value: "{joins.read().get(&port).cloned().unwrap_or_default()}",
                                disabled: !editable,
                                on_change: move |net_id: String| {
                                    joins.with_mut(|value| {
                                        if net_id.is_empty() { value.remove(&port); }
                                        else { value.insert(port.clone(), net_id); }
                                    });
                                },
                                options: std::iter::once((String::new(), "Separate local net".into(), false)).chain(host_nets.iter().map(|(id, name)| (id.clone(), name.clone(), false))).collect(),
                        }
                    }
                    Button { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable || embed_disabled(), onclick: embed, "Copy circuit to PCB" }
                    // A copy row cannot own its disabled Signal: the row unmounts when its
                    // removal lands, before the settlement drain releases the binding, and
                    // the drain would write a dropped Signal. The bounded removal action
                    // owns this panel Signal, so one pending removal disables all copy
                    // controls; the begin guard keeps a second click a no-op while a
                    // sibling removal is still in flight.
                    for circuit in &embedded_circuits {
                        div { key: "{circuit.id}",
                            span {
                                "{circuit.part_ids.len()} components · {circuit.id.rsplit('/').next().unwrap_or(&circuit.id)}"
                            }
                            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", aria_label: "Remove copy {circuit.id}",
                                disabled: !editable || circuit_removal_disabled(),
                                onclick: {
                                    let circuit_id = circuit.id.clone();
                                    let remove_runtime = input.runtime.clone();
                                    let remove_scope = input.scope.clone();
                                    let remove_module_id = input.module_id.clone();
                                    let remove_context = input.selected_context;
                                    let remove_token = input.snapshot.token;
                                    let remove_revision = input.snapshot.document.revision;
                                    let remove_actions = edits.clone();
                                    move |_| {
                                        if remove_actions.is_pending(&ModuleEditKey::RemoveCircuit) {
                                            return;
                                        }
                                        let Some(_snapshot) = mounted_owner_current(&remove_runtime, remove_context, &remove_scope, &remove_module_id, remove_token, remove_revision) else {
                                            feedback.set("The selected module or accepted project changed. Reopen its placement before removing the circuit copy.".into());
                                            return;
                                        };
                                        let key = ModuleEditKey::RemoveCircuit;
                                        let mut latest = latest;
                                        latest.set(Some(key.clone()));
                                        feedback.set(String::new());
                                        remove_actions.begin_one_shot(&remove_runtime, key, "remove-embedded-circuit", Some("circuit".into()), module_resolver(remove_scope.clone(), remove_module_id.clone(), Ok(EditOperation::RemoveEmbeddedCircuit { id: circuit_id.clone() })));
                                    }
                                },
                                "Remove copy"
                            }
                        }
                    }
                }
            }
            section { "aria-label": "Individual components",
                h3 { "Individual components" }
                p { "Place available source footprints independently. Component heights and assembly completeness need separate review." }
                if definition.circuit.is_some() {
                    for constituent in constituent_actions.clone() {
                        div { key: "{constituent.reference}-{constituent.index}",
                            div { strong { "{constituent.reference} · {constituent.name}" } small { if constituent.purchased { "Purchased assembly · footprint and external contacts only" } else { "{constituent.footprint}" } } }
                            Button { variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable || constituent.source_definition.is_none(), aria_label: "Place {constituent.reference}", onclick: move |_| {
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
            div { class: "flex flex-wrap gap-2 mt-1",
                Button { class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable || draft().connection.as_ref().is_some_and(|connection| connection.host_connector_part_id.is_empty()), onclick: save, "Save placement" }
                Button { variant: ButtonVariant::Destructive, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !editable || remove_disabled(), onclick: remove, "Remove module" }
            }
            if !feedback().is_empty() {
                Alert { variant: AlertVariant::Destructive, "{feedback()}" }
            } else if latest() == Some(ModuleEditKey::Placement) {
                if let Some(message) = placement_failure() {
                    Alert { variant: AlertVariant::Destructive, "{message}" }
                }
            }
        }
    }
}

/// Mark actual user changes independently of their value. Accepted projection and
/// helper settlement writes do not go through this path.
fn edit_module_draft(
    mut draft: Signal<MountedModule>,
    mut dirty: Signal<bool>,
    edit: impl FnOnce(&mut MountedModule),
) {
    let before = draft.peek().clone();
    draft.with_mut(edit);
    if *draft.peek() != before {
        dirty.set(true);
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
    document: &boardstudio_model::model::ProjectDoc,
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

fn module_resolver(
    scope: Scope,
    module_id: String,
    operation: Result<EditOperation, String>,
) -> EditResolver {
    EditResolver::new("pcb-module", move |accepted: &AcceptedSnapshot| {
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
        Resolution::submit(vec![module_id.clone()], operation.clone())
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "pcb_module_inspector/wasm_tests.rs"]
mod wasm_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "pcb_module_inspector/mounted_save_replacement_tests.rs"]
mod mounted_save_replacement_tests;
