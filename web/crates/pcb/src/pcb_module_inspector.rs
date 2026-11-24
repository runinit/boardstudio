//! Private editor for the placement of one accepted mounted-module instance.
//! Module source definitions and their footprint/circuit ownership remain untouched.
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_model::model::{
    EditOperation, ModuleAttachment, ModuleConnection, ModuleSupport, MountedModule,
    PartDefinition, Side, VikRole, VikSignal,
};
use boardstudio_web_parts::parts::load_horizontal_host_connector_definition;
use boardstudio_web_ui_model::{
    state::ComponentPlacementAction,
    tree::{ScopedTreeContext, TreeContext},
};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TOUCH_CLS, TextCommitField, ToggleField,
};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use std::collections::BTreeMap;
use std::rc::Rc;

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
    pub selected_context: Signal<Option<ScopedTreeContext>>,
    pub on_place_component: EventHandler<ComponentPlacementAction>,
}

#[derive(Clone, Default)]
struct SupportDraft {
    mount_id: String,
    outer_diameter: Option<f64>,
    hole_diameter: Option<f64>,
    z: Option<f64>,
    height: Option<f64>,
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
    selected_context: Signal<Option<ScopedTreeContext>>,
    on_place_component: EventHandler<ComponentPlacementAction>,
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
    let mut preparing = use_signal(|| false);
    let _ = use_context::<Signal<u64>>()();
    let accepted_instance = instance.clone();
    use_effect(use_reactive(
        (&accepted_instance,),
        move |(accepted_instance,)| {
            if !*draft_dirty.peek() && *draft.peek() != accepted_instance {
                draft.set(accepted_instance);
            }
        },
    ));
    let module_name = definition.name.clone();
    let automatic_connector_id = format!("{}/vik-host-connector", instance.id);
    let current = {
        let runtime = input.runtime.clone();
        let scope = input.scope.clone();
        let id = input.module_id.clone();
        let selected = input.selected_context;
        let token = input.snapshot.token;
        let revision = input.snapshot.document.revision;
        move || mounted_owner_current(&runtime, selected, &scope, &id, token, revision)
    };
    let submit = {
        let runtime = input.runtime.clone();
        let scope = input.scope.clone();
        let id = input.module_id.clone();
        let current = current.clone();
        move |label: &str, operation: EditOperation| {
            current().ok_or("The selected module or accepted project changed.")?;
            runtime.commit(label, module_resolver(scope.clone(), id.clone(), operation))
        }
    };
    let save = {
        let current = current.clone();
        let submit = submit.clone();
        let module_id = input.module_id.clone();
        move |_| {
            if preparing() {
                return;
            }
            let Some(snapshot) = current() else {
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
                feedback.set(
                    "Choose a VIK host connector on this board, or clear the connection.".into(),
                );
                return;
            }
            let submit = submit.clone();
            let module_id = module_id.clone();
            let mut finish = move |connector| match submit(
                "mounted-module-placement",
                EditOperation::SetMountedModule {
                    instance: Box::new(value.clone()),
                    definition: None,
                    host_connector_definition: connector,
                },
            ) {
                Ok(snapshot) => {
                    if *draft.peek() == value
                        && let Some(accepted) = snapshot
                            .document
                            .modules
                            .iter()
                            .find(|module| module.id == module_id)
                    {
                        draft.set(accepted.clone());
                        draft_dirty.set(false);
                    }
                    feedback.set(String::new());
                }
                Err(message) => feedback.set(message),
            };
            if needs_connector {
                preparing.set(true);
                spawn(async move {
                    let result = load_horizontal_host_connector_definition().await;
                    preparing.set(false);
                    match result {
                        Ok(definition) => finish(Some(Box::new(definition))),
                        Err(message) => feedback.set(message),
                    }
                });
            } else {
                finish(None);
            }
        }
    };
    let remove = {
        let submit = submit.clone();
        let id = input.module_id.clone();
        move |_| {
            let result = submit(
                "remove-mounted-module",
                EditOperation::RemoveMountedModule { id: id.clone() },
            );
            feedback.set(result.err().unwrap_or_default());
        }
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
    let embed = {
        let source = definition.clone();
        let runtime = input.runtime.clone();
        let submit = submit.clone();
        move |_| {
            let placement = draft();
            let result = submit(
                "embed-module-circuit",
                EditOperation::EmbedModuleCircuit {
                    id: format!("circuit/embedded-{}", runtime.operation().0),
                    definition: source.clone(),
                    host_board_id: placement.host_board_id,
                    pose: boardstudio_model::model::Pose2 {
                        at: placement.at,
                        rotation: placement.rotation,
                    },
                    side: placement.host_face,
                    joins: joins(),
                },
            );
            feedback.set(result.err().unwrap_or_default());
        }
    };

    rsx! {
        section { class: "grid gap-2.5 min-w-0 overflow-auto p-4 text-foreground [&_h2]:m-0 [&_h2]:text-lg [&_p]:m-0 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground", "aria-label": "Mounted module placement",
            h2 { "{module_name} placement" }
            p { "Placement edits affect this project instance. Source footprint artwork remains owned by the module." }
            SelectField::<String> {
                label: "Host board",
                aria_label: "Module host board",
                value: "{draft().host_board_id}",
                disabled: !editable,
                on_commit: move |next: String| {
                        edit_module_draft(draft, draft_dirty, |value| {
                            value.host_board_id = next.clone();
                            if value.host_instance_id.as_ref().is_some_and(|id| !instances.iter().any(|instance| instance.id == *id && instance.board_id == next)) {
                                value.host_instance_id = None;
                            }
                        });
                    },
                options: boards.iter().map(|board| (board.id.clone(), board.name.clone())).collect(),
            }
            SelectField::<String> {
                label: "Physical instance",
                aria_label: "Module physical instance",
                value: "{draft().host_instance_id.clone().unwrap_or_default()}",
                disabled: !editable,
                on_commit: move |id: String| {
                        edit_module_draft(draft, draft_dirty, |value| value.host_instance_id = (!id.is_empty()).then_some(id));
                    },
                options: std::iter::once((String::new(), "Board-wide".into())).chain(options.iter().map(|(id, name)| (id.clone(), name.clone()))).collect(),
            }
            SelectField::<Side> {
                label: "Mounted face",
                aria_label: "Module host face",
                value: draft().host_face,
                disabled: !editable,
                on_commit: move |next| edit_module_draft(draft, draft_dirty, |value| value.host_face = next),
                options: vec![(Side::Front, "Front".into()), (Side::Back, "Back".into())],
            }
            SelectField::<Side> {
                label: "Facing face",
                aria_label: "Module facing face",
                value: draft().facing_face,
                disabled: !editable,
                on_commit: move |next| edit_module_draft(draft, draft_dirty, |value| value.facing_face = next),
                options: vec![(Side::Front, "Front".into()), (Side::Back, "Back".into())],
            }
            for (label, value, minimum, field) in [
                ("X (mm)", draft().at.x, None, 0u8),
                ("Y (mm)", draft().at.y, None, 1u8),
                ("Yaw (degrees)", draft().rotation, None, 2u8),
                ("Gap (mm)", draft().gap, Some(0.0), 3u8),
            ] {
                NumericField { key: "{label}", label, value, min: minimum,
                    disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                    on_commit: move |number| edit_module_draft(draft, draft_dirty, |value| match field {
                        0 => value.at.x = number,
                        1 => value.at.y = number,
                        2 => value.rotation = number,
                        _ => value.gap = number,
                    }),
                }
            }
            SelectField::<ModuleAttachment> {
                label: "Attachment", aria_label: "Module attachment", value: draft().attachment, disabled: !editable,
                on_commit: move |next| edit_module_draft(draft, draft_dirty, |value| value.attachment = next),
                options: vec![(ModuleAttachment::Board, "Board".into()), (ModuleAttachment::Case, "Case".into())],
            }
            NumericField {
                label: "Extra service clearance (mm)", aria_label: "Module service clearance",
                value: draft().service_clearance, min: Some(0.0), step: Some(0.1),
                disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                on_commit: move |number| edit_module_draft(draft, draft_dirty, |value| value.service_clearance = number),
            }
            section { class: "grid gap-2 min-w-0 p-2.5 border border-border rounded-md bg-card [&_h3]:m-0 [&_h3]:text-xs [&_h3]:text-foreground", "aria-label": "{support_section}",
                h3 { "{support_section}" }
                p { "Choose a source PCB hole and enter designer-selected ring dimensions. Z and height are module-midplane millimetres; these values are not vendor specifications." }
                for (index, support) in draft().mount_supports.iter().enumerate() {
                    div { key: "{support.mount_id}", class: "flex items-center justify-between gap-2 py-1.5 border-b border-border text-foreground text-xs [&_button]:flex-none",
                        span { "{support.mount_id} · OD {support.outer_diameter} / ID {support.hole_diameter} · Z {support.z} · height {support.height} mm" }
                        Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: !editable, aria_label: "Remove support {support.mount_id}", onclick: move |_| {
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
                SelectField::<String> {
                    label: "Source mounting hole",
                    aria_label: "Support source mounting hole",
                    value: "{support_draft().mount_id}",
                    disabled: !editable || !has_source_mounts,
                    on_commit: move |next: String| support_draft.with_mut(|value| value.mount_id = next),
                    options: std::iter::once((String::new(), "Choose module hole…".into())).chain(source_mounts.iter().map(|mount| (mount.source_id.clone(), format!("{} · source drill {} mm", mount.source_id, mount.diameter)))).collect(),
                }
                div { class: "grid grid-cols-2 gap-2",
                    for (label, aria, value, field) in [
                        ("Outer diameter (mm)", "Support outer diameter", support_draft().outer_diameter, 0u8),
                        ("Hole diameter (mm)", "Support hole diameter", support_draft().hole_diameter, 1u8),
                        ("Z from midplane (mm)", "Support Z from midplane", support_draft().z, 2u8),
                        ("Ring height (mm)", "Support ring height", support_draft().height, 3u8),
                    ] {
                        NumericField { key: "{label}", label, aria_label: aria, step: Some(0.1),
                            value: value.unwrap_or_default(), mixed: value.is_none(),
                            placeholder: "Enter a dimension",
                            disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                            on_commit: move |number: f64| {
                                support_draft.with_mut(|draft| match field {
                                    0 => draft.outer_diameter = Some(number),
                                    1 => draft.hole_diameter = Some(number),
                                    2 => draft.z = Some(number),
                                    _ => draft.height = Some(number),
                                });
                            },
                        }
                    }
                }
                Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: !editable || !has_source_mounts, onclick: move |_| {
                    let fields = support_draft().clone();
                    let source_mount = add_source_mounts.iter().find(|mount| mount.source_id == fields.mount_id);
                    let Some(mount) = source_mount else {
                        feedback.set("Choose a source mounting hole before adding a support.".into());
                        return;
                    };
                    let (Some(outer_diameter), Some(hole_diameter), Some(z), Some(height)) =
                        (fields.outer_diameter, fields.hole_diameter, fields.z, fields.height)
                    else {
                        feedback.set("Enter finite support dimensions before adding a support.".into());
                        return;
                    };
                    if ![outer_diameter, hole_diameter, z, height].iter().all(|number| number.is_finite())
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
            ToggleField { label: "Detached", checked: draft().detached,
                disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                on_commit: move |checked| edit_module_draft(draft, draft_dirty, |value| value.detached = checked)
            }
            InspectorSection { title: "VIK connection",
                ToggleField { label: "Assign host connection",
                    checked: draft().connection.is_some(),
                    disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                    on_commit: move |checked| {
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
                    SelectField::<String> {
                        label: "Host connector",
                        aria_label: "Module host connector",
                        value: "{connection.host_connector_part_id}",
                        disabled: !editable,
                        on_commit: move |next: String| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.host_connector_part_id = next; }),
                        options: std::iter::once((String::new(), "Select VIK host connector".into()))
                            .chain((connectors.is_empty() && automatic_connector_can_be_created).then(|| (automatic_connector_id.clone(), "Add source-backed horizontal VIK connector beside module".into())))
                            .chain(connectors.iter().map(|(id, reference)| (id.clone(), reference.clone()))).collect(),
                    }
                    if connectors.is_empty() {
                        if automatic_connector_can_be_created {
                            p { "No source-backed host-role VIK connector is present on this board. Add one before saving a connected placement." }
                        } else {
                            p { "The automatic connector ID is already used by a part outside this board. Choose a valid host connector or clear the connection." }
                        }
                    }
                    SelectField::<String> {
                        label: "Module port",
                        aria_label: "Module input port",
                        value: "{connection.module_port_id}",
                        disabled: !editable,
                        on_commit: move |next: String| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.module_port_id = next; }),
                        options: definition.interfaces.iter().filter(|port| port.role == VikRole::Module).map(|port| (port.id.clone(), port.id.clone())).collect(),
                    }
                    TextCommitField { label: "Bus name", aria_label: "Module bus name", value: "{connection.bus_id}",
                        disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
                        on_commit: move |name| edit_module_draft(draft, draft_dirty, |value| if let Some(connection) = &mut value.connection { connection.bus_id = name; })
                    }
                    p { "12 contacts · 0.5 mm pitch · Type A cable · 3.3 V logic. Enter actual MCU terminals; shared buses are checked by their wiring." }
                    div {
                        for (signal, label) in vik_signals() {
                            ClearableVikText { key: "{label}", label: label.to_owned(), aria_label: format!("VIK {label} terminal"),
                                value: connection.assignments.get(&signal).cloned().unwrap_or_default(),
                                editable, has_value: connection.assignments.contains_key(&signal),
                                clear_label: format!("Clear {label} terminal"),
                                on_commit: move |terminal| edit_module_draft(draft, draft_dirty, |value| {
                                    if let Some(connection) = &mut value.connection { connection.assignments.insert(signal, terminal); }
                                }),
                                on_clear: move |_| edit_module_draft(draft, draft_dirty, |value| {
                                    if let Some(connection) = &mut value.connection { connection.assignments.remove(&signal); }
                                }),
                            }
                        }
                    }
                    div {
                        for (signal, label) in [(VikSignal::V3v3, "3.3V supply"), (VikSignal::V5, "5V supply"), (VikSignal::Gnd, "Ground") ] {
                            ClearableVikNumber { key: "{label}", label: format!("{label} · V"), aria_label: format!("VIK {label} voltage"),
                                value: connection.rail_voltages.get(&signal).copied(), editable, step: 0.1,
                                clear_label: format!("Clear {label} voltage"),
                                on_commit: move |number| edit_module_draft(draft, draft_dirty, |module| {
                                    if let Some(connection) = &mut module.connection {
                                        connection.rail_voltages.insert(signal, number);
                                    }
                                }),
                                on_clear: move |_| edit_module_draft(draft, draft_dirty, |module| {
                                    if let Some(connection) = &mut module.connection {
                                        connection.rail_voltages.remove(&signal);
                                    }
                                }),
                            }
                        }
                    }
                    ClearableVikNumber { label: "Supply budget · mA".to_owned(), aria_label: "VIK supply current budget".to_owned(),
                        value: connection.supply_current_ma, editable, min: Some(0.0), step: 1.0,
                        clear_label: "Clear supply budget".to_owned(),
                        on_commit: move |number| edit_module_draft(draft, draft_dirty, |module| {
                            if let Some(connection) = &mut module.connection {
                                connection.supply_current_ma = Some(number);
                            }
                        }),
                        on_clear: move |_| edit_module_draft(draft, draft_dirty, |module| {
                            if let Some(connection) = &mut module.connection {
                                connection.supply_current_ma = None;
                            }
                        }),
                    }
                }
            }
            if definition.circuit.is_some() {
                InspectorSection { title: "Use circuit on PCB",
                    p { "Creates an independent, editable copy. Choose host nets for explicit joins; other nets remain local to this copy." }
                    for port in circuit_nets.clone() {
                        SelectField::<String> { key: "{port}", label: format!("{} joins", port.to_uppercase()),
                                aria_label: format!("Circuit {port} host net"),
                                value: "{joins.read().get(&port).cloned().unwrap_or_default()}",
                                disabled: !editable,
                                on_commit: move |net_id: String| {
                                    joins.with_mut(|value| {
                                        if net_id.is_empty() { value.remove(&port); }
                                        else { value.insert(port.clone(), net_id); }
                                    });
                                },
                                options: std::iter::once((String::new(), "Separate local net".into())).chain(host_nets.iter().map(|(id, name)| (id.clone(), name.clone()))).collect(),
                        }
                    }
                    Button { class: TOUCH_CLS, disabled: !editable, onclick: embed, "Copy circuit to PCB" }
                    for circuit in &embedded_circuits {
                        div { key: "{circuit.id}",
                            span {
                                "{circuit.part_ids.len()} components · {circuit.id.rsplit('/').next().unwrap_or(&circuit.id)}"
                            }
                            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, aria_label: "Remove copy {circuit.id}",
                                disabled: !editable,
                                onclick: {
                                    let circuit_id = circuit.id.clone();
                                    let submit = submit.clone();
                                    move |_| {
                                        let result = submit("remove-embedded-circuit", EditOperation::RemoveEmbeddedCircuit { id: circuit_id.clone() });
                                        feedback.set(result.err().unwrap_or_default());
                                    }
                                },
                                "Remove copy"
                            }
                        }
                    }
                }
            }
            InspectorSection { title: "Individual components",
                p { "Place available source footprints independently. Component heights and assembly completeness need separate review." }
                if definition.circuit.is_some() {
                    for constituent in constituent_actions.clone() {
                        div { key: "{constituent.reference}-{constituent.index}",
                            div { strong { "{constituent.reference} · {constituent.name}" } small { if constituent.purchased { "Purchased assembly · footprint and external contacts only" } else { "{constituent.footprint}" } } }
                            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, disabled: !editable || constituent.source_definition.is_none(), aria_label: "Place {constituent.reference}", onclick: move |_| {
                                if let Some(definition) = constituent.source_definition.clone() {
                                    on_place_component.call(ComponentPlacementAction::AddSourceObject {
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
                Button { class: TOUCH_CLS, disabled: !editable || preparing() || draft().connection.as_ref().is_some_and(|connection| connection.host_connector_part_id.is_empty()), onclick: save, "Save placement" }
                Button { variant: ButtonVariant::Destructive, class: TOUCH_CLS, disabled: !editable, onclick: remove, "Remove module" }
            }
            if !feedback().is_empty() {
                Alert { variant: AlertVariant::Destructive, "{feedback()}" }

            }
        }
    }
}

/// Text VIK row with its conditional clear button (signal terminals).
#[component]
fn ClearableVikText(
    label: String,
    aria_label: String,
    value: String,
    editable: bool,
    has_value: bool,
    clear_label: String,
    on_commit: EventHandler<String>,
    on_clear: EventHandler<()>,
) -> Element {
    rsx! {
        TextCommitField { label, aria_label, value,
            disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
            on_commit: move |terminal: String| on_commit.call(terminal),
        }
        if has_value {
            Button { disabled: !editable, onclick: move |_| on_clear.call(()), "{clear_label}" }
        }
    }
}

/// Numeric VIK row with its conditional clear button (rail voltages, supply).
#[component]
fn ClearableVikNumber(
    label: String,
    aria_label: String,
    value: Option<f64>,
    editable: bool,
    #[props(default)] min: Option<f64>,
    step: f64,
    clear_label: String,
    on_commit: EventHandler<f64>,
    on_clear: EventHandler<()>,
) -> Element {
    rsx! {
        NumericField { label, aria_label, value: value.unwrap_or_default(),
            mixed: value.is_none(), placeholder: "Unset", min, step: Some(step),
            disabled_reason: (!editable).then(|| "Module is read-only".to_owned()),
            on_commit: move |number| on_commit.call(number),
        }
        if value.is_some() {
            Button { disabled: !editable, onclick: move |_| on_clear.call(()), "{clear_label}" }
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

// VIK connector admission lives in the host-compatible `connector_admission`
// module so the same predicate runs under `cargo test` without a browser.
use crate::connector_admission::{
    ConnectorAdmission, connector_admission, host_connector_is_on_board,
};

fn mounted_owner_current(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope: &Scope,
    module_id: &str,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
) -> Option<AcceptedSnapshot> {
    if !mounted_selection_current(runtime, selected_context, scope, module_id) {
        return None;
    }
    let snapshot = crate::pcb_wiring::ready_snapshot(runtime)?;
    (snapshot.token == token
        && snapshot.document.revision == revision
        && snapshot.document.id == scope.document_id
        && snapshot.session_epoch == scope.session_epoch)
        .then_some(snapshot)
}

fn mounted_selection_current(
    runtime: &Rc<Runtime>,
    selected_context: Signal<Option<ScopedTreeContext>>,
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
            != (TreeContext::MountedModule {
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

fn module_resolver(scope: Scope, module_id: String, operation: EditOperation) -> EditResolver {
    EditResolver::new("pcb-module", move |accepted: &AcceptedSnapshot| {
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
        match &operation {
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

// Connector admission runs natively via `crate::connector_admission` tests.
