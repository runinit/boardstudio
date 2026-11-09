use super::{ModuleEntry, load_horizontal_host_connector_definition};
use crate::parts_custom_definition::replacement_commit;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_core::model::{
    EditOperation, ModuleAttachment as AttachmentKind, ModuleConnection, ModuleDefinition,
    MountedModule, PartDefinition, Side, Vec2, VikRole,
};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use boardstudio_web_ui_shared::pending_edit_helpers::PendingEditSignals;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::Button,
    callout::Callout,
    checkbox::{Checkbox, CheckboxState},
    label::Label,
    spinner::Spinner,
    toast::{ToastOptions, use_toast},
};
use dioxus::prelude::*;
use std::{cell::Cell, collections::BTreeMap, rc::Rc};
use wasm_bindgen_futures::spawn_local;

const AUTOMATIC_CONNECTOR_SELECTION: &str = "__automatic_vik_host_connector__";

/// This action's one bounded key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttachKey {
    Attach,
}

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
    let mut pending = use_signal(|| false);
    let pending_edits = use_hook(PendingEditSignals::<AttachKey>::new);
    // The mounted-module identity this panel's attachment was submitted with; the
    // Landed follow-up selects exactly that module.
    let attached_module_id = use_signal(|| None::<String>);
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
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let definition = module.definition.clone();
    let definition_id = definition.id.clone();
    let expected_selection = (scope.clone(), format!("module:{definition_id}"));
    let owner_scope = scope.clone();
    let initial_generation = selection_generation();
    let options_baseline = use_hook(|| {
        (
            connector_selection.peek().clone(),
            *host_connection_enabled.peek(),
        )
    });
    let mut submitted_options = use_signal(|| None::<(String, bool)>);
    let options_guard =
        use_signal(|| None::<boardstudio_web_runtime::draft_owners::DraftOwnerGuard>);
    let registry_owner = (
        snapshot.document.id.clone(),
        snapshot.session_epoch,
        scope.clone(),
        definition_id.clone(),
        initial_generation,
    );
    use_effect(use_reactive((&registry_owner,), {
        let runtime = runtime.clone();
        let edits = pending_edits.clone();
        let baseline = options_baseline.clone();
        let expected = expected_selection.clone();
        let owner = registry_owner.clone();
        let mut guard = options_guard;
        move |_| {
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
            let probe_runtime = runtime.clone();
            let baseline = baseline.clone();
            let expected = expected.clone();
            let scope = owner.2.clone();
            let generation = owner.4;
            let edits = edits.clone();
            guard.set(Some(runtime.register_draft_owner(
                boardstudio_web_runtime::draft_owners::DraftOwnerRegistration {
                    project_scope: (owner.0.clone(), owner.1),
                    workspace: "Parts",
                    owner: "module-attachment",
                    admission_key: "attach",
                    area: "Module attach options".into(),
                    remedy: "Attach the module, restore the options, or close".into(),
                    is_unresolved: Rc::new(move || {
                        if probe_runtime.scope() != scope
                            || selected.peek().as_ref() != Some(&expected)
                            || *selection_generation.peek() != generation
                        {
                            return false;
                        }
                        let live = (
                            connector_selection.peek().clone(),
                            *host_connection_enabled.peek(),
                        );
                        if live == baseline {
                            return false;
                        }
                        // Loading the connector is preparation, not an admitted Session
                        // submission; do not exempt options until begin_one_shot runs.
                        !(edits.is_pending(&AttachKey::Attach)
                            && submitted_options.peek().as_ref() == Some(&live))
                    }),
                },
            )));
        }
    }));
    use_drop({
        let runtime = runtime.clone();
        let mut guard = options_guard;
        move || {
            if let Some(old) = guard.write().take() {
                runtime.retire_draft_owner(old);
            }
        }
    });

    // The attachment settles outside Dioxus (Core replies, saves), so the workspace
    // version wakes the settle pass below even when nothing else changed.
    let version = use_context::<Signal<u64>>();
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let pending_edits = pending_edits.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let selected = selected;
        let mut attached_module_id = attached_module_id;
        let scope = owner_scope.clone();
        let definition = definition.clone();
        let expected_selection = expected_selection.clone();
        move |_| {
            let (Some(attach_scope), Some(module_id)) =
                (scope.clone(), attached_module_id.peek().clone())
            else {
                return;
            };
            let live = attachment_selection_current(
                &runtime,
                &attach_scope,
                selected,
                selection_generation,
                &expected_selection,
                initial_generation,
                workspace,
            );
            for result in pending_edits.settle(live, |_| String::new()) {
                pending.set(false);
                attached_module_id.take();
                match result {
                    PendingEditResult::Landed { .. } => {
                        let model = runtime.model();
                        let Some(accepted) = model.accepted.as_ref() else {
                            return;
                        };
                        let Some(mounted) =
                            accepted.document.modules.iter().rev().find(|mounted| {
                                mounted.id == module_id
                                    || mounted
                                        .id
                                        .strip_prefix(&format!("{module_id}-"))
                                        .is_some_and(|suffix| suffix.parse::<u64>().is_ok())
                            })
                        else {
                            return;
                        };
                        let navigation = AttachedModuleNavigation {
                            scope: attach_scope.clone(),
                            snapshot_token: accepted.token,
                            revision: accepted.document.revision,
                            module_id: mounted.id.clone(),
                            definition_id: definition.id.clone(),
                            selection_generation: initial_generation,
                        };
                        toasts.success("Module attached.".into(), ToastOptions::new());
                        feedback.set(String::new());
                        on_attached.call(navigation);
                    }
                    PendingEditResult::Failed { message, .. } => feedback.set(message),
                    PendingEditResult::Retired { .. } => {}
                }
            }
        }
    }));
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
    let attach_pending_edits = pending_edits.clone();
    let attach = move |_| {
        let pending_edits = attach_pending_edits.clone();
        let module_port_id = &attachment_port_id;
        if pending() || pending_edits.is_pending(&AttachKey::Attach) {
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
        let submitted = (selected_connector.clone(), host_connection_enabled());
        pending.set(true);
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
        let alive = alive.clone();
        let pending_edits = pending_edits.clone();
        let mut attached_module_id = attached_module_id;
        let mut pending = pending;
        let mut feedback = feedback;
        spawn_local(async move {
            let connector_definition = if needs_host_connector {
                Some(load_horizontal_host_connector_definition().await)
            } else {
                None
            };
            if !alive.get() {
                return;
            }
            if !attachment_selection_current(
                &runtime,
                &scope,
                selected,
                selection_generation,
                &expected_selection,
                initial_generation,
                workspace,
            ) {
                pending.set(false);
                return;
            }
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
                pending.set(false);
                return;
            };
            let connector_definition = match connector_definition {
                Some(Ok(definition)) => Some(definition),
                Some(Err(error)) => {
                    feedback.set(format!(
                        "Could not load the source-backed VIK connector: {error}"
                    ));
                    pending.set(false);
                    return;
                }
                None => None,
            };
            submitted_options.set(Some(submitted));
            pending_edits.begin_one_shot(
                &runtime,
                AttachKey::Attach,
                "attach-mounted-module",
                Some("module attachment".into()),
                attachment_resolver(
                    scope.clone(),
                    instance,
                    definition.clone(),
                    connector_definition,
                ),
            );
            attached_module_id.set(Some(module_id));
            feedback.set(String::new());
            // Preparation is over: the settle pass observes the edit's terminal.
            pending.set(false);
        });
    };

    rsx! {
        section { "aria-label": "Attach module to PCB",
            Button { class: "w-full max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                disabled: pending() || pending_edits.is_pending(&AttachKey::Attach) || !can_attach,
                onclick: attach,
                "Attach module to {board_name}"
            }
            Label { html_for: format!("parts-host-connection-{}", dioxus::core::current_scope_id().0),
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
                Checkbox {
                    id: format!("parts-host-connection-{}", dioxus::core::current_scope_id().0),
                    aria_label: "Assign host connection",
                    checked: Some(if host_connection_enabled() { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                    disabled: pending() || pending_edits.is_pending(&AttachKey::Attach) || module_port_id.is_none(),
                    on_checked_change: move |value: CheckboxState| {
                        let checked = bool::from(value);
                        host_connection_enabled.set(checked);
                        if checked && connector_selection().is_empty() {
                            connector_selection.set(initial_connector.clone());
                        }
                    }
                }
                "Assign host connection"
            }
            if host_connection_enabled() && module_port_id.is_some() {
                super::super::PartsChoiceField {
                    label: "Host connector", aria_label: "Module host connector",
                    value: connector_selection(), disabled: pending(),
                    options: std::iter::once((AUTOMATIC_CONNECTOR_SELECTION.to_owned(), "Add source-backed horizontal VIK connector beside module".into()))
                        .chain(connectors.iter().map(|(id, reference)| (id.clone(), reference.clone()))).collect(),
                    on_value_change: move |value| connector_selection.set(value),
                }
            }
            if module_port_id.is_none() {
                div { role: "status", Callout { "This module has no module-role VIK port; it will attach without a host connection." } }
            }
            if !can_attach {
                div { role: "status", Callout { "Choose a PCB in the Parts workspace before attaching this module." } }
            }
            if !feedback().is_empty() {
                if pending() { div { role: "status", Spinner {} "{feedback()}" } }
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
    workspace: Signal<&'static str>,
) -> bool {
    workspace() == "Parts"
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
    workspace: Signal<&'static str>,
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

fn fresh_module_id(document: &boardstudio_core::model::ProjectDoc) -> Option<String> {
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

#[cfg(test)]
mod mounted_settlement_tests {
    use super::*;
    use boardstudio_core::model::{Board, ProjectDoc};
    use boardstudio_web_runtime::runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::{wait_until, wait_until_future, wait_until_step};
    use std::cell::RefCell;
    use wasm_bindgen_test::*;

    struct Probe {
        runtime: Rc<Runtime>,
        navigations: RefCell<Vec<AttachedModuleNavigation>>,
    }

    fn host() -> Element {
        let probe = use_context::<Rc<Probe>>();
        let runtime = probe.runtime.clone();
        use_context_provider(|| runtime.clone());
        let version = use_signal(|| 0_u64);
        use_context_provider(|| version);
        let workspace = use_signal(|| "Parts");
        use_context_provider(|| crate::presentation::WorkspaceState(workspace));
        let generation = use_signal(|| 1_u64);
        use_context_provider(|| super::super::super::PartsSelectionGeneration(generation));
        let mut selected = use_signal(|| Some((runtime.scope(), "module:module".into())));
        let _ = version();
        use_hook({
            let runtime = runtime.clone();
            move || {
                runtime.subscribe(Rc::new(move || {
                    let mut version = version;
                    version += 1;
                }))
            }
        });
        let mut shown = use_signal(|| true);
        let mut hidden = use_signal(|| false);
        let snapshot = runtime.model().accepted.unwrap();
        let module = ModuleEntry {
            row: "Module".into(),
            definition: Rc::new(snapshot.document.module_definitions[0].clone()),
            source: super::super::EntrySource::Project,
        };
        rsx! {
            boardstudio_web_ui_shared::ui::toast::ToastProvider {
            div { hidden: hidden(),
                if shown() {
                    ModuleAttachment {
                        snapshot,
                        module,
                        scope: runtime.scope(),
                        selected,
                        on_attached: move |navigation| probe.navigations.borrow_mut().push(navigation),
                    }
                }
            }
            button { onclick: move |_| hidden.set(true), "Hide attachment" }
            button { onclick: move |_| shown.set(false), "Unmount attachment" }
            button { id: "depart-attachment-owner", onclick: move |_| selected.set(None), "Depart" }
            button { id: "return-attachment-owner", onclick: move |_| selected.set(Some((runtime.scope(), "module:module".into()))), "Return" }
            }
        }
    }

    async fn settle_until(runtime: &Rc<Runtime>, what: &str, done: impl Fn() -> bool) {
        wait_until_step(what, async || {
            support::run_pending(runtime).await;
            done()
        })
        .await;
    }

    struct AttachmentUpdate;
    impl boardstudio_web_host::host::UpdateSignals for AttachmentUpdate {
        fn snapshot(&self) -> boardstudio_web_host::offline::UpdateSnapshot {
            boardstudio_web_host::offline::UpdateSnapshot {
                waiting: true,
                installing: false,
                controlled: true,
            }
        }
        fn watch(&self, _: Rc<dyn Fn()>) -> Box<dyn boardstudio_web_host::host::UpdateWatch> {
            struct Stop;
            impl boardstudio_web_host::host::UpdateWatch for Stop {
                fn stop(&mut self) {}
            }
            Box::new(Stop)
        }
        fn adopt_waiting(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<(), boardstudio_web_host::offline::AdoptError>,
                    >,
            >,
        > {
            Box::pin(async { Ok(()) })
        }
        fn reload(&self) {}
    }

    fn attachment_areas(
        runtime: &Rc<Runtime>,
        opener: boardstudio_web_runtime::update_safety::UpdateOpener,
    ) -> Vec<String> {
        use boardstudio_web_runtime::update_safety::{UpdateBlocker, UpdateDialog};
        runtime.cancel_update_dialog();
        runtime.begin_update(opener);
        match runtime.update_dialog() {
            Some(UpdateDialog::Deferred { blockers }) => blockers
                .into_iter()
                .filter_map(|blocker| match blocker {
                    UpdateBlocker::UnresolvedDraft { area, remedy } => {
                        assert_eq!(area, "Module attach options");
                        assert_eq!(remedy, "Attach the module, restore the options, or close");
                        Some(area)
                    }
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
            other => panic!("unexpected update dialog: {other:?}"),
        }
    }

    async fn mount_options() -> (Rc<Runtime>, web_sys::Element) {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("attach-options-013", "Attach options");
        document.boards.push(serde_json::from_value(serde_json::json!({
                "id":"board", "name":"Board", "outlineIds":[], "partIds":[], "netIds":[], "thickness":1.6, "traces":[], "vias":[]
            })).unwrap());
        document.module_definitions.push(serde_json::from_value(serde_json::json!({
                "id":"module", "name":"Module", "family":"test", "variant":"test",
                "source":{"repository":"test", "revision":"test", "path":"test", "license":"test"},
                "board":{"contours":[{"hole":false,"points":[{"x":0,"y":0},{"x":20,"y":0},{"x":20,"y":20},{"x":0,"y":20}]}]}, "electrical":{"protocol":"gpio"},
                "interfaces":[{"id":"vik", "role":"module", "signals":["sclk","miso","cs","gpio2","mosi","gpio1","v5","rgb","scl","sda","gnd","v3v3"]}]
            })).unwrap());
        document.definitions.push(serde_json::from_value(serde_json::json!({
                "id":"host-definition", "name":"Host connector", "kind":"connector", "courtyard":[], "pads":[],
                "hardwareProfile":{"source":{"repository":"test", "revision":"test", "path":"test", "license":"test"}, "vikRole":"host"}
            })).unwrap());
        document.parts.push(serde_json::from_value(serde_json::json!({
                "id":"host-connector", "definitionId":"host-definition", "reference":"J1", "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front"
            })).unwrap());
        document.boards[0].part_ids.push("host-connector".into());
        let mut other_board = document.boards[0].clone();
        other_board.id = "other-board".into();
        other_board.part_ids.clear();
        document.boards.push(other_board);
        support::open_document(&runtime, document).await;
        let probe = Rc::new(Probe {
            runtime: runtime.clone(),
            navigations: RefCell::new(Vec::new()),
        });
        let doc = web_sys::window().unwrap().document().unwrap();
        let root = doc.create_element("div").unwrap();
        doc.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("attachment options mount", || {
            by_role_name(&root, "checkbox", "Assign host connection").is_some()
        })
        .await;
        runtime.observe_update_waiting(Rc::new(AttachmentUpdate));
        (runtime, root)
    }

    #[wasm_bindgen_test]
    async fn mounted_attachment_options_defer_both_openers_and_revert_013() {
        use boardstudio_web_runtime::update_safety::UpdateOpener;
        for opener in [UpdateOpener::Toast, UpdateOpener::Header] {
            let (runtime, root) = mount_options().await;
            let checkbox = by_role_name(&root, "checkbox", "Assign host connection").unwrap();
            checkbox.click();
            wait_until("host assignment changes", || {
                checkbox.get_attribute("aria-checked").as_deref() == Some("false")
            })
            .await;
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"]
            );
            assert_eq!(
                checkbox.get_attribute("aria-checked").as_deref(),
                Some("false")
            );
            // Changed options use the no-connector preparation branch. The loading
            // branch itself requires the unchanged (automatic, true) mount defaults.
            by_role_name(&root, "button", "Attach module to Board")
                .unwrap()
                .click();
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"],
                "preparation is not an admitted submission"
            );
            by_role_name(&root, "button", "Depart").unwrap().click();
            // Yield to the queued preparation before restoring its selection.
            gloo_timers::future::TimeoutFuture::new(20).await;
            wait_until("departed preparation releases controls", || {
                !checkbox.has_attribute("disabled")
            })
            .await;
            support::run_pending(&runtime).await;
            assert!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .modules
                    .is_empty()
            );
            by_role_name(&root, "button", "Return").unwrap().click();
            gloo_timers::future::TimeoutFuture::new(20).await;
            checkbox.click();
            wait_until("host assignment reverts", || {
                checkbox.get_attribute("aria-checked").as_deref() == Some("true")
            })
            .await;
            assert!(attachment_areas(&runtime, opener).is_empty());
            runtime.unsubscribe();
            root.remove();
        }
    }

    // Keep scope retirement independent of the preparation story's selection
    // departure and safe revert (which adopts its waiting worker).
    #[wasm_bindgen_test]
    async fn mounted_attachment_scope_departure_retires_changed_options_013() {
        use boardstudio_web_runtime::update_safety::UpdateOpener;
        let (runtime, root) = mount_options().await;
        let checkbox = by_role_name(&root, "checkbox", "Assign host connection").unwrap();
        checkbox.click();
        wait_until("changed options before scope departure", || {
            checkbox.get_attribute("aria-checked").as_deref() == Some("false")
        })
        .await;
        assert_eq!(
            attachment_areas(&runtime, UpdateOpener::Toast),
            vec!["Module attach options"]
        );
        support::navigate(&runtime, "other-board").await;
        gloo_timers::future::TimeoutFuture::new(20).await;
        assert!(
            attachment_areas(&runtime, UpdateOpener::Header).is_empty(),
            "scope change retires the old attach options"
        );
        assert!(checkbox.is_connected(), "the form remains mounted");
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_attachment_real_connector_loading_locks_default_options_013() {
        use boardstudio_web_runtime::update_safety::UpdateOpener;
        use wasm_bindgen::{JsCast, JsValue};

        // wasm-bindgen-test-runner serves its cwd: stage the real catalogue at
        // web/crates/parts/assets/imported-modules.json, just as build-web.py does.
        // Hold the real fetch response, not a fabricated connector or Response.
        let gate = js_sys::eval(
            r#"(() => {
            const original = window.fetch;
            const state = { entered: false, status: 0, calls: 0 };
            let release;
            const held = new Promise(resolve => { release = resolve; });
            state.release = release;
            state.restore = () => { window.fetch = original; release(); };
            window.fetch = async function(input, ...args) {
                if (typeof input !== 'string' ||
                    new URL(input, location.href).pathname !== '/assets/imported-modules.json') {
                    return original.call(this, input, ...args);
                }
                state.calls++;
                const response = await original.call(this, input, ...args);
                state.status = response.status;
                state.entered = true;
                await held;
                return response;
            };
            return state;
        })()"#,
        )
        .unwrap();
        let property = |key: &str| js_sys::Reflect::get(&gate, &key.into()).unwrap();
        let (runtime, root) = mount_options().await;
        let attach = by_role_name(&root, "button", "Attach module to Board").unwrap();
        let checkbox = by_role_name(&root, "checkbox", "Assign host connection").unwrap();
        let selector = by_role_name(&root, "button", "Module host connector").unwrap();
        attach.click();
        wait_until("real connector fetch is held during preparation", || {
            property("entered").as_bool() == Some(true)
                && root
                    .text_content()
                    .unwrap_or_default()
                    .contains("Loading the source-backed VIK host connector")
        })
        .await;
        assert_eq!(
            property("status").as_f64(),
            Some(200.0),
            "serve the real catalogue; a missing asset is not a passing loading proof"
        );
        assert_eq!(property("calls").as_f64(), Some(1.0));
        for opener in [UpdateOpener::Toast, UpdateOpener::Header] {
            assert!(attach.has_attribute("disabled"));
            assert!(checkbox.has_attribute("disabled"));
            assert!(selector.has_attribute("disabled"));
            checkbox.click();
            selector.click();
            attach.click();
            assert_eq!(
                checkbox.get_attribute("aria-checked").as_deref(),
                Some("true")
            );
            assert!(by_role_name(&root, "listbox", "Module host connector").is_none());
            assert!(
                attachment_areas(&runtime, opener).is_empty(),
                "unchanged automatic defaults are not an unresolved draft"
            );
            assert!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .modules
                    .is_empty(),
                "loading must not admit an attachment early"
            );
        }
        runtime.cancel_update_dialog();
        property("release")
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call0(&JsValue::NULL)
            .unwrap();
        settle_until(&runtime, "real connector attachment lands", || {
            !attach.has_attribute("disabled")
        })
        .await;
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.modules.len(),
            1,
            "{}",
            root.text_content().unwrap_or_default()
        );
        let mounted = &accepted.document.modules[0];
        let connection = mounted.connection.as_ref().unwrap();
        let connector = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == connection.host_connector_part_id)
            .unwrap();
        assert_eq!(
            connector.definition_id,
            "vik:source:horizontal-host-connector"
        );
        let definition = accepted
            .document
            .definitions
            .iter()
            .find(|definition| definition.id == connector.definition_id)
            .unwrap();
        assert!(
            !definition.pads.is_empty(),
            "the real source footprint must land"
        );
        assert_eq!(
            definition.hardware_profile.as_ref().unwrap().vik_role,
            Some(VikRole::Host)
        );
        assert_eq!(
            property("calls").as_f64(),
            Some(1.0),
            "lockout prevents duplicate loads"
        );
        assert!(!checkbox.has_attribute("disabled"));
        assert!(!selector.has_attribute("disabled"));
        wait_until("source-backed attachment reports genuine success", || {
            by_role_name(&root, "status", "Module attached.").is_some()
        })
        .await;
        for opener in [UpdateOpener::Toast, UpdateOpener::Header] {
            assert!(attachment_areas(&runtime, opener).is_empty());
        }
        property("restore")
            .dyn_into::<js_sys::Function>()
            .unwrap()
            .call0(&JsValue::NULL)
            .unwrap();
        runtime.unsubscribe();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn mounted_attachment_invalid_connector_survives_rejection_and_hidden_retention_013() {
        use boardstudio_web_runtime::update_safety::UpdateOpener;
        for opener in [UpdateOpener::Toast, UpdateOpener::Header] {
            let (runtime, root) = mount_options().await;
            by_role_name(&root, "button", "Module host connector")
                .unwrap()
                .click();
            wait_until("existing connector option opens", || {
                by_role_name(&root, "option", "J1").is_some()
            })
            .await;
            let key = web_sys::KeyboardEventInit::new();
            key.set_key("End");
            key.set_bubbles(true);
            key.set_cancelable(true);
            by_role_name(&root, "listbox", "Module host connector")
                .unwrap()
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key)
                        .unwrap(),
                )
                .unwrap();
            wait_until("existing connector option receives keyboard focus", || {
                by_role_name(&root, "option", "J1")
                    .unwrap()
                    .matches(":focus")
                    .unwrap()
            })
            .await;
            key.set_key("Enter");
            by_role_name(&root, "option", "J1")
                .unwrap()
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key)
                        .unwrap(),
                )
                .unwrap();
            wait_until("existing connector is selected", || {
                by_role_name(&root, "button", "Module host connector")
                    .unwrap()
                    .text_content()
                    .unwrap_or_default()
                    .contains("J1")
            })
            .await;
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"]
            );
            let mut document = (*runtime.model().accepted.unwrap().document).clone();
            document.parts.clear();
            document.boards[0].part_ids.clear();
            support::replace_document(&runtime, "remove-host", "host-connector", document).await;
            gloo_timers::future::TimeoutFuture::new(20).await;
            // The selected connector disappeared: an ordinary stale option, not a synthetic invalid marker.
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"]
            );
            by_role_name(&root, "button", "Attach module to Board")
                .unwrap()
                .click();
            wait_until("stale connector admission refuses", || {
                root.text_content()
                    .unwrap_or_default()
                    .contains("Choose a VIK host connector on the selected PCB")
            })
            .await;
            assert!(
                runtime
                    .model()
                    .accepted
                    .unwrap()
                    .document
                    .modules
                    .is_empty()
            );
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"]
            );
            by_role_name(&root, "button", "Hide attachment")
                .unwrap()
                .click();
            gloo_timers::future::TimeoutFuture::new(20).await;
            assert_eq!(
                attachment_areas(&runtime, opener),
                vec!["Module attach options"]
            );
            by_role_name(&root, "button", "Unmount attachment")
                .unwrap()
                .click();
            wait_until("attachment unmounts", || {
                by_role_name(&root, "checkbox", "Assign host connection").is_none()
            })
            .await;
            assert!(attachment_areas(&runtime, opener).is_empty());
            runtime.unsubscribe();
            root.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_attachment_drains_landing_failure_and_silent_retirement() {
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("attachment-settlement", "Attachment settlement");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.module_definitions.push(
            serde_json::from_value(serde_json::json!({
                "id":"module", "name":"Module", "family":"test", "variant":"test",
                "source":{"repository":"test", "revision":"test", "path":"test", "license":"test"},
                "board":{"contours":[]}, "electrical":{"protocol":"gpio"}
            }))
            .unwrap(),
        );
        support::open_document(&runtime, document).await;
        let probe = Rc::new(Probe {
            runtime: runtime.clone(),
            navigations: RefCell::new(Vec::new()),
        });
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(host);
        dom.provide_root_context(probe.clone());
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("module attachment control mounts", || {
            by_role_name(&root, "button", "Attach module to Board").is_some()
        })
        .await;
        let attach = by_role_name(&root, "button", "Attach module to Board").unwrap();
        let (entered, release) = support::gate_next_core_reply(&runtime);
        attach.click();
        wait_until("module attachment is admitted", || {
            attach.has_attribute("disabled")
        })
        .await;
        support::drive_pending(&runtime);
        wait_until_future("module attachment enters Core", entered)
            .await
            .unwrap();
        assert!(attach.has_attribute("disabled"));
        assert!(
            probe.navigations.borrow().is_empty(),
            "navigation waits for landing"
        );
        release.send(()).unwrap();
        settle_until(&runtime, "module attachment lands and navigates", || {
            runtime.model().accepted.unwrap().document.modules.len() == 1
                && probe.navigations.borrow().len() == 1
                && !attach.has_attribute("disabled")
        })
        .await;
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(accepted.document.modules.len(), 1);
        assert_eq!(
            probe.navigations.borrow().len(),
            1,
            "landing delivers one exact selection follow-up"
        );
        let navigation = probe.navigations.borrow()[0].clone();
        assert_eq!(navigation.module_id, accepted.document.modules[0].id);
        assert_eq!(navigation.definition_id, "module");
        assert_eq!(navigation.snapshot_token, accepted.token);
        assert_eq!(navigation.revision, accepted.document.revision);
        assert_eq!(navigation.selection_generation, 1);
        assert!(!attach.has_attribute("disabled"));
        wait_until("attachment success is a transient Toast", || {
            by_role_name(&root, "status", "Module attached.").is_some()
        })
        .await;
        by_role_name(&root, "button", "close").unwrap().click();
        wait_until("the success Toast dismisses", || {
            by_role_name(&root, "status", "Module attached.").is_none()
        })
        .await;

        let (entered, release) = support::gate_next_core_reply(&runtime);
        attach.click();
        wait_until("second module attachment is admitted", || {
            attach.has_attribute("disabled")
        })
        .await;
        support::drive_pending(&runtime);
        wait_until_future("second module attachment enters Core", entered)
            .await
            .unwrap();
        by_role_name(&root, "button", "Depart").unwrap().click();
        wait_until("departed module attachment owner retires", || {
            !attach.has_attribute("disabled")
        })
        .await;
        release.send(()).unwrap();
        settle_until(
            &runtime,
            "departed module attachment settles silently",
            || {
                runtime.model().accepted.unwrap().document.modules.len() == 2
                    && probe.navigations.borrow().len() == 1
                    && !root
                        .text_content()
                        .unwrap()
                        .contains("controlled attachment failure")
                    && !attach.has_attribute("disabled")
            },
        )
        .await;
        assert_eq!(
            probe.navigations.borrow().len(),
            1,
            "departed owner retires its follow-up silently"
        );
        assert!(
            !root
                .text_content()
                .unwrap()
                .contains("controlled attachment failure")
        );
        assert!(!root.text_content().unwrap().contains("Module attached."));
        assert!(!attach.has_attribute("disabled"));

        by_role_name(&root, "button", "Return").unwrap().click();
        wait_until("module attachment owner returns", || {
            by_role_name(&root, "button", "Attach module to Board").is_some()
        })
        .await;
        support::fail_next_core_reply(&runtime, "controlled attachment failure");
        attach.click();
        settle_until(&runtime, "module attachment failure is shown", || {
            root.text_content()
                .unwrap()
                .contains("controlled attachment failure")
                && !attach.has_attribute("disabled")
        })
        .await;
        assert!(
            root.text_content()
                .unwrap()
                .contains("controlled attachment failure")
        );
        assert_eq!(
            probe.navigations.borrow().len(),
            1,
            "failure does not navigate"
        );
        assert!(!attach.has_attribute("disabled"));
        runtime.unsubscribe();
        root.remove();
    }
}
