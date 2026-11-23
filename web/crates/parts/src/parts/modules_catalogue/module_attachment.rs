use super::{ModuleEntry, load_horizontal_host_connector_definition};
use crate::parts_custom_definition::replacement_commit;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_model::model::{
    EditOperation, ModuleAttachment as AttachmentKind, ModuleConnection, ModuleDefinition,
    MountedModule, PartDefinition, Side, Vec2, VikRole,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{SelectField, ToggleField};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::Button,
    callout::Callout,
    spinner::Spinner,
    toast::{ToastOptions, use_toast},
};
use dioxus::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

const AUTOMATIC_CONNECTOR_SELECTION: &str = "__automatic_vik_host_connector__";

#[derive(Clone)]
pub struct AttachedModuleNavigation {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub module_id: String,
    pub definition_id: String,
    pub selection_generation: u64,
}

#[component]
pub fn ModuleAttachment(
    snapshot: AcceptedSnapshot,
    module: ModuleEntry,
    scope: Option<Scope>,
    selected: super::super::PartsSelection,
    on_attached: EventHandler<AttachedModuleNavigation>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let toasts = use_toast();
    let workspace = use_context::<crate::presentation::WorkspaceState>().0;
    let selection_generation = use_context::<super::super::PartsSelectionGeneration>().0;
    let mut preparing = use_signal(|| false);
    let mut feedback = use_signal(String::new);
    let connectors = eligible_host_connectors(
        &snapshot,
        scope
            .as_ref()
            .map(|scope| scope.board_id.as_str())
            .unwrap_or_default(),
    );
    let initial_connector = AUTOMATIC_CONNECTOR_SELECTION.to_owned();
    let mut connector_selection = use_signal(|| initial_connector.clone());
    let module_port_id = module
        .definition
        .interfaces
        .iter()
        .find(|port| port.role == VikRole::Module)
        .map(|port| port.id.clone());
    let mut host_connection_enabled = use_signal(|| true);
    let definition = module.definition.clone();
    let definition_id = definition.id.clone();
    let expected_selection = (scope.clone(), format!("module:{definition_id}"));
    let owner_scope = scope.clone();
    let initial_generation = selection_generation();

    let can_attach = scope.as_ref().is_some_and(|scope| {
        snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
    });
    let board_name = scope
        .as_ref()
        .and_then(|scope| {
            snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == scope.board_id)
        })
        .map(|board| board.name.clone())
        .unwrap_or_else(|| "selected board".into());

    let attachment_port_id = module_port_id.clone();
    let attach = move |_| {
        let module_port_id = &attachment_port_id;
        if preparing() {
            return;
        }
        let Some(scope) = owner_scope.clone() else {
            feedback.set("Choose an accepted PCB before attaching a module.".into());
            return;
        };
        if !can_attach {
            feedback.set("The selected PCB is no longer available.".into());
            return;
        }
        let Some(accepted) = attachment_owner_current(
            &runtime,
            &scope,
            selected,
            selection_generation,
            &expected_selection,
            initial_generation,
            workspace,
        ) else {
            feedback.set("The selected module, PCB, or accepted project changed. Reselect the module before attaching it.".into());
            return;
        };
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            feedback.set("The selected PCB is no longer available.".into());
            return;
        }

        if host_connection_enabled() && module_port_id.is_some() && connector_selection().is_empty()
        {
            feedback
                .set("Choose an on-board VIK host connector before attaching this module.".into());
            return;
        }
        let selected_connector = connector_selection();
        if host_connection_enabled()
            && module_port_id.is_some()
            && selected_connector != AUTOMATIC_CONNECTOR_SELECTION
            && !eligible_host_connectors(&accepted, &scope.board_id)
                .iter()
                .any(|(id, _)| id == &selected_connector)
        {
            feedback.set(
                "Choose a VIK host connector on the selected PCB, or create a new connector."
                    .into(),
            );
            return;
        }
        let Some(module_id) = fresh_module_id(&accepted.document) else {
            feedback.set("The browser could not create a unique mounted-module identity.".into());
            return;
        };
        let host_connector_id = format!("{module_id}/vik-host-connector");
        let needs_host_connector = host_connection_enabled()
            && module_port_id.is_some()
            && selected_connector == AUTOMATIC_CONNECTOR_SELECTION;
        let connection = if host_connection_enabled() {
            module_port_id.as_ref().map(|port_id| ModuleConnection {
                host_connector_part_id: if needs_host_connector {
                    host_connector_id
                } else {
                    selected_connector.clone()
                },
                module_port_id: port_id.clone(),
                bus_id: format!("vik/{module_id}"),
                assignments: BTreeMap::new(),
                cable_type: "type-a-12-0.5".into(),
                supply_current_ma: None,
                rail_voltages: BTreeMap::new(),
                upstream_module_id: None,
                upstream_port_id: None,
            })
        } else {
            None
        };
        let instance = MountedModule {
            id: module_id.clone(),
            definition_id: definition_id.clone(),
            host_board_id: scope.board_id.clone(),
            host_instance_id: None,
            host_face: Side::Front,
            facing_face: Side::Back,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            gap: 3.0,
            attachment: AttachmentKind::Board,
            detached: false,
            connection,
            service_clearance: 0.0,
            mount_supports: Vec::new(),
        };
        preparing.set(true);
        feedback.set(if needs_host_connector {
            "Loading the source-backed VIK host connector…".into()
        } else {
            "Attaching module…".into()
        });

        let runtime = runtime.clone();
        let definition = (*definition).clone();
        let selected = selected;
        let selection_generation = selection_generation;
        let expected_selection = expected_selection.clone();
        let workspace = workspace;
        let mut preparing = preparing;
        let mut feedback = feedback;
        spawn(async move {
            let connector_definition = if needs_host_connector {
                Some(load_horizontal_host_connector_definition().await)
            } else {
                None
            };
            let Some(_current) = attachment_owner_current(
                &runtime,
                &scope,
                selected,
                selection_generation,
                &expected_selection,
                initial_generation,
                workspace,
            ) else {
                feedback.set("The accepted project changed while the connector was loading. Reselect the module before attaching it.".into());
                preparing.set(false);
                return;
            };
            let connector_definition = match connector_definition {
                Some(Ok(definition)) => Some(definition),
                Some(Err(error)) => {
                    feedback.set(format!(
                        "Could not load the source-backed VIK connector: {error}"
                    ));
                    preparing.set(false);
                    return;
                }
                None => None,
            };
            preparing.set(false);
            let resolver = attachment_resolver(
                scope.clone(),
                instance,
                definition.clone(),
                connector_definition,
            );
            let accepted = match runtime.commit("attach-mounted-module", resolver) {
                Ok(accepted) => accepted,
                Err(message) => {
                    feedback.set(message);
                    return;
                }
            };
            let Some(mounted) = accepted.document.modules.iter().rev().find(|mounted| {
                mounted.id == module_id
                    || mounted
                        .id
                        .strip_prefix(&format!("{module_id}-"))
                        .is_some_and(|suffix| suffix.parse::<u64>().is_ok())
            }) else {
                return;
            };
            toasts.success("Module attached.".into(), ToastOptions::new());
            feedback.set(String::new());
            on_attached.call(AttachedModuleNavigation {
                scope,
                snapshot_token: accepted.token,
                revision: accepted.document.revision,
                module_id: mounted.id.clone(),
                definition_id: definition.id,
                selection_generation: initial_generation,
            });
        });
    };

    rsx! {
        section { "aria-label": "Attach module to PCB",
            Button { class: "w-full max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled: preparing() || !can_attach,
                onclick: attach,
                "Attach module to {board_name}"
            }
            ToggleField {
                label: "Assign host connection", checked: host_connection_enabled(),
                disabled_reason: if preparing() { Some("Attaching module".to_owned()) }
                    else { module_port_id.is_none().then(|| "No module-role VIK port".to_owned()) },
                on_commit: move |checked| {
                    host_connection_enabled.set(checked);
                    if checked && connector_selection().is_empty() {
                        connector_selection.set(initial_connector.clone());
                    }
                },
            }
            if host_connection_enabled() && module_port_id.is_some() {
                SelectField::<String> {
                    label: "Host connector", aria_label: "Module host connector",
                    value: connector_selection(), disabled: preparing(),
                    options: std::iter::once((AUTOMATIC_CONNECTOR_SELECTION.to_owned(), "Add source-backed horizontal VIK connector beside module".into()))
                        .chain(connectors.iter().cloned()).collect(),
                    on_commit: move |value| connector_selection.set(value),
                }
            }
            if module_port_id.is_none() {
                div { role: "status", Callout { "This module has no module-role VIK port; it will attach without a host connection." } }
            }
            if !can_attach {
                div { role: "status", Callout { "Choose a PCB in the Parts workspace before attaching this module." } }
            }
            if !feedback().is_empty() {
                if preparing() { div { role: "status", Spinner {} "{feedback()}" } }
                else { Alert { variant: AlertVariant::Destructive, "{feedback()}" } }
            }
        }
    }
}

fn attachment_selection_current(
    runtime: &Rc<Runtime>,
    scope: &Scope,
    selected: super::super::PartsSelection,
    selection_generation: Signal<u64>,
    expected_selection: &(Option<Scope>, String),
    expected_generation: u64,
    workspace: Signal<Workspace>,
) -> bool {
    workspace() == Workspace::Parts
        && runtime.scope().as_ref() == Some(scope)
        && selected() == Some(expected_selection.clone())
        && selection_generation() == expected_generation
}

fn attachment_owner_current(
    runtime: &Rc<Runtime>,
    scope: &Scope,
    selected: super::super::PartsSelection,
    selection_generation: Signal<u64>,
    expected_selection: &(Option<Scope>, String),
    expected_generation: u64,
    workspace: Signal<Workspace>,
) -> Option<AcceptedSnapshot> {
    if !attachment_selection_current(
        runtime,
        scope,
        selected,
        selection_generation,
        expected_selection,
        expected_generation,
        workspace,
    ) {
        return None;
    }
    let model = runtime.model();
    let accepted = model.accepted?;
    (accepted.document.id == scope.document_id && accepted.session_epoch == scope.session_epoch)
        .then_some(accepted)
}

fn attachment_resolver(
    scope: Scope,
    instance: MountedModule,
    definition: ModuleDefinition,
    connector: Option<PartDefinition>,
) -> EditResolver {
    EditResolver::new(
        "attach-mounted-module",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != scope.document_id
                || accepted.session_epoch != scope.session_epoch
                || !accepted
                    .document
                    .boards
                    .iter()
                    .any(|board| board.id == scope.board_id)
            {
                return Resolution::Retire("The selected PCB is no longer available.".into());
            }
            if connector.is_none()
                && instance.connection.as_ref().is_some_and(|connection| {
                    !eligible_host_connectors(accepted, &scope.board_id)
                        .iter()
                        .any(|(id, _)| id == &connection.host_connector_part_id)
                })
            {
                return Resolution::Retire(
                    "The selected VIK host connector is no longer available on this PCB.".into(),
                );
            }
            let latest = accepted
                .document
                .module_definitions
                .iter()
                .find(|latest| latest.id == definition.id)
                .unwrap_or(&definition);
            if latest.source != definition.source
                || latest.circuit != definition.circuit
                || latest.interfaces != definition.interfaces
            {
                return Resolution::Retire(
                    "The module source or connector mapping changed.".into(),
                );
            }
            let mut mounted = instance.clone();
            let mut suffix = 0_u64;
            while accepted
                .document
                .modules
                .iter()
                .any(|item| item.id == mounted.id)
                || accepted.document.parts.iter().any(|part| {
                    part.id == mounted.id || part.id == format!("{}/vik-host-connector", mounted.id)
                })
            {
                suffix += 1;
                mounted.id = format!("{}-{suffix}", instance.id);
            }
            if let Some(connection) = mounted.connection.as_mut() {
                connection.bus_id = format!("vik/{}", mounted.id);
                if connector.is_some() {
                    connection.host_connector_part_id =
                        format!("{}/vik-host-connector", mounted.id);
                }
            }
            let ids = vec![mounted.id.clone(), scope.board_id.clone()];
            replacement_commit(
                EditOperation::SetMountedModule {
                    instance: Box::new(mounted),
                    definition: Some(Box::new(latest.clone())),
                    host_connector_definition: connector.clone().map(Box::new),
                },
                ids,
            )
        },
    )
}

fn eligible_host_connectors(snapshot: &AcceptedSnapshot, board_id: &str) -> Vec<(String, String)> {
    let Some(board) = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)
    else {
        return Vec::new();
    };
    snapshot
        .document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter(|part| {
            snapshot
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
                .and_then(|definition| definition.hardware_profile.as_ref())
                .is_some_and(|profile| profile.vik_role == Some(VikRole::Host))
        })
        .map(|part| (part.id.clone(), part.reference.clone()))
        .collect()
}

fn fresh_module_id(document: &boardstudio_model::model::ProjectDoc) -> Option<String> {
    for _ in 0..8 {
        let uuid = crate::runtime::new_project_id().ok()?;
        let module_id = format!("module/{uuid}");
        let connector_id = format!("{module_id}/vik-host-connector");
        if document.modules.iter().all(|module| module.id != module_id)
            && document
                .parts
                .iter()
                .all(|part| part.id != module_id && part.id != connector_id)
        {
            return Some(module_id);
        }
    }
    None
}
