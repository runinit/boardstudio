//! Private owner for the Case mechanical-settings field requests.
//!
//! This module is deliberately unmounted. The page supplies the current accepted snapshot,
//! the existing asynchronous Core resolver, the already-normalized bundled mounting-hole
//! definition, the page's exact Runtime operation observer/submission seam, and the existing
//! pure closure-clearance projector. It does not create a second document/session authority.
use super::mechanical_settings::{
    MechanicalDimension, MechanicalSettingsFeedback, MechanicalSettingsFeedbackState,
    MechanicalSettingsIdentity, MechanicalSettingsPatch, MechanicalSettingsRequest,
};
use crate::operation_outcomes::OutcomeSlot;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, OperationId, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{
    GasketConstructionVersion, GasketPlacement, HardwareTransport, InsertInstallation,
    InternalClosureHardware, InternalGasketConfiguration, MechanicalAssembly, MechanicalBattery,
    MechanicalBottomStyle, MechanicalConfiguration, MechanicalGasketLayout, MechanicalMount,
    MechanicalSwitchFamily, Mount, MountKind, Part, PartDefinition, PartGenerator, PartKind,
    PlateMethod, ProjectDoc, ScrewDrive, ScrewHeadProfile, ScrewLengthDatum, Vec2, Vec3,
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
};
use wasm_bindgen_futures::spawn_local;

type LocalFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;
pub(crate) type ResolveMechanicalPort = Rc<
    dyn Fn(
        AcceptedSnapshot,
        Scope,
        ProjectDoc,
    ) -> LocalFuture<Result<MechanicalResolution, String>>,
>;
pub(crate) type ProjectClosureClearancePort =
    Rc<dyn Fn(ProjectDoc, &PartDefinition) -> Result<ProjectDoc, String>>;

/// The page copies only accepted Arc handles and the small configuration projection displayed
/// by the controls. `editable` must be computed from the current Runtime, workspace,
/// physical-instance, readiness, preview, gesture, lifecycle and durability admission guards.
/// It must exclude this controller's own in-flight bit; use `is_busy` to disable the UI. The
/// controller still re-reads it at every asynchronous boundary.
#[derive(Clone)]
pub(crate) struct MechanicalSettingsCurrent {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) accepted: AcceptedSnapshot,
    pub(crate) configuration: Option<Rc<MechanicalConfiguration>>,
    pub(crate) editable: bool,
    pub(crate) lifecycle: Lifecycle,
    pub(crate) durability: Durability,
}

/// The page's existing effective-case policy is the source for both a fresh Core resolution and
/// generated boss height (notably the wireless battery default). The reflected/effective document
/// remains read-only; only this mechanical value is written back through the canonical ownership
/// mapping.
#[derive(Clone)]
pub(crate) struct MechanicalResolution {
    pub(crate) assembly: MechanicalAssembly,
    pub(crate) effective_configuration: MechanicalConfiguration,
}

/// Narrow page ports. `submit_replace` must register the supplied fresh operation with the
/// existing `Runtime::observe_operation` before submitting one `ReplaceDocument` event.
/// Returning its exact weak-observed slot is required; a page-wide last-error/status string is
/// not an operation result.
#[derive(Clone)]
pub(crate) struct MechanicalSettingsPorts {
    pub(crate) current: Rc<dyn Fn() -> Option<MechanicalSettingsCurrent>>,
    pub(crate) resolve: ResolveMechanicalPort,
    pub(crate) load_mounting_hole: Rc<dyn Fn() -> LocalFuture<Result<Rc<PartDefinition>, String>>>,
    pub(crate) next_operation: Rc<dyn Fn() -> OperationId>,
    pub(crate) submit_replace:
        Rc<dyn Fn(OperationId, u64, ProjectDoc) -> Result<OutcomeSlot, String>>,
    pub(crate) project_closure_clearance: ProjectClosureClearancePort,
    pub(crate) publish: Rc<dyn Fn(MechanicalSettingsFeedback)>,
}

/// A bounded page-local coordinator. The parent should create this once per Case editor lifetime
/// and call `settle` from its existing Runtime-version effect. The field subtree remains the
/// owner of its drafts; this controller owns only one in-flight request and its exact outcome.
pub(crate) struct MechanicalSettingsController {
    ports: MechanicalSettingsPorts,
    pending: RefCell<Option<PendingRequest>>,
    last_request_id: Cell<u64>,
}

#[derive(Clone)]
enum PendingPhase {
    Resolving,
    Submitted {
        outcome: OutcomeSlot,
        accepted_token: SnapshotToken,
        base_revision: u64,
        expected: Rc<ExpectedCommit>,
    },
}

#[derive(Clone)]
struct PendingRequest {
    request: MechanicalSettingsRequest,
    phase: PendingPhase,
}

#[derive(Clone, Debug, PartialEq)]
enum ExpectedSettings {
    Canonical(Option<MechanicalConfiguration>),
    Instance {
        instance_id: String,
        shared: Option<MechanicalConfiguration>,
        instances: Vec<(String, bool, Option<MechanicalConfiguration>)>,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct ExpectedCommit {
    settings: ExpectedSettings,
    closure: ClosureEvidence,
}

/// This proof intentionally retains only generated closure rows and their membership, never a
/// second predicted ProjectDoc or a copy of unrelated KiCad definitions.
#[derive(Clone, Debug, PartialEq)]
struct ClosureEvidence {
    parts: Vec<Part>,
    definitions: Vec<ClosureDefinitionEvidence>,
    boards: Vec<(String, Vec<String>)>,
    layouts: Vec<(String, Vec<String>)>,
}

#[derive(Clone, Debug, PartialEq)]
struct ClosureDefinitionEvidence {
    id: String,
    kind: PartKind,
    generator: Option<PartGenerator>,
}

impl MechanicalSettingsController {
    pub(crate) fn new(ports: MechanicalSettingsPorts) -> Rc<Self> {
        Rc::new(Self {
            ports,
            pending: RefCell::new(None),
            last_request_id: Cell::new(0),
        })
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.pending.borrow().is_some()
    }

    /// Admission rejection reports against this request without displacing the operation already
    /// admitted for another field. Every child submission gets its own terminal response.
    pub(crate) fn submit(self: &Rc<Self>, request: MechanicalSettingsRequest) -> bool {
        if request.request_id <= self.last_request_id.get() {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some(
                    "This mechanical settings request was already handled. Retry the field.".into(),
                ),
            );
            return false;
        }
        self.last_request_id.set(request.request_id);
        if self.pending.borrow().is_some() {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some("Wait for the current mechanical settings change to finish.".into()),
            );
            return false;
        }
        if request.field_id != patch_field_id(&request.patch) {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some("The mechanical settings request did not match its field.".into()),
            );
            return false;
        }
        let Some(current) = (self.ports.current)() else {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some("The accepted mechanical settings are unavailable.".into()),
            );
            return false;
        };
        if !admitted(&current, &request.identity) {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some("Finish the active edit and wait for saving before changing mechanical settings.".into()),
            );
            return false;
        }
        if matches!(&request.patch, MechanicalSettingsPatch::InitializeClosures)
            && !can_initialize_closures(&current, &request.identity)
        {
            self.emit(
                &request,
                MechanicalSettingsFeedbackState::Failed,
                Some(
                    "Mounting defaults are no longer available for this configuration. Retry after the current stack is ready.".into(),
                ),
            );
            return false;
        }

        *self.pending.borrow_mut() = Some(PendingRequest {
            request: request.clone(),
            phase: PendingPhase::Resolving,
        });
        self.emit(&request, MechanicalSettingsFeedbackState::Pending, None);
        let controller = Rc::downgrade(self);
        let ports = self.ports.clone();
        spawn_local(async move {
            MechanicalSettingsController::prepare_and_submit(controller, ports, request, current)
                .await;
        });
        true
    }

    /// Called by the page's existing version signal effect. It observes only the exact slot
    /// registered for this request and never infers success from unrelated Runtime status.
    pub(crate) fn settle(&self) {
        let Some(waiting) = self.pending.borrow().clone() else {
            return;
        };
        let Some(current) = (self.ports.current)() else {
            self.fail_pending(
                &waiting.request,
                "The mechanical settings operation lost its active editor.".into(),
            );
            return;
        };
        let same_owner = match &waiting.phase {
            PendingPhase::Resolving => same_owner(&current.identity, &waiting.request.identity),
            PendingPhase::Submitted { .. } => {
                same_submitted_owner(&current.identity, &waiting.request.identity)
            }
        };
        if !same_owner {
            self.fail_pending(
                &waiting.request,
                "The mechanical settings operation belongs to an earlier editor scope.".into(),
            );
            return;
        }
        if matches!(&waiting.phase, PendingPhase::Resolving) {
            if !admitted(&current, &waiting.request.identity) {
                self.fail_pending(
                    &waiting.request,
                    "The accepted document changed while the mechanical settings were being prepared. Review the current values and retry.".into(),
                );
            }
            return;
        }
        let PendingPhase::Submitted {
            outcome,
            accepted_token,
            base_revision,
            expected,
        } = waiting.phase
        else {
            return;
        };
        let Some(terminal) = outcome.borrow().clone() else {
            return;
        };
        match terminal {
            TerminalOutcome::Completed => {
                let snapshot = &current.accepted;
                let saved = current.lifecycle == Lifecycle::Ready
                    && current.durability
                        == (Durability::Saved {
                            revision: snapshot.document.revision,
                        });
                if !saved
                    || snapshot.token == accepted_token
                    || snapshot.document.revision <= base_revision
                {
                    if matches!(&current.durability, Durability::Failed { .. })
                        || matches!(
                            &current.lifecycle,
                            Lifecycle::RecoveryRequired | Lifecycle::Closed
                        )
                    {
                        self.fail_pending(
                            &waiting.request,
                            "The mechanical settings operation completed but its document was not durably saved. Retry after recovery.".into(),
                        );
                    }
                    // Completed is the operation's result, but Session acceptance and durable
                    // save may publish on a later Runtime notification.
                    return;
                }
                if expected_matches(&snapshot.document, &expected) {
                    self.finish(
                        &waiting.request,
                        MechanicalSettingsFeedbackState::Saved,
                        None,
                    );
                } else {
                    self.fail_pending(
                        &waiting.request,
                        "The saved document does not contain the requested mechanical settings and closure clearances. Review the current stack and retry.".into(),
                    );
                }
            }
            TerminalOutcome::Rejected(message)
            | TerminalOutcome::PersistenceFailed(message)
            | TerminalOutcome::BlockedByRecovery(message)
            | TerminalOutcome::ExecutorFailed(message) => {
                self.fail_pending(&waiting.request, message);
            }
            TerminalOutcome::Superseded | TerminalOutcome::Cancelled | TerminalOutcome::Closed => {
                self.fail_pending(
                    &waiting.request,
                    "The mechanical settings operation did not complete in the active session."
                        .into(),
                );
            }
        }
    }

    async fn prepare_and_submit(
        controller: Weak<Self>,
        ports: MechanicalSettingsPorts,
        request: MechanicalSettingsRequest,
        admitted_current: MechanicalSettingsCurrent,
    ) {
        let operation = Self::prepare(
            ports.clone(),
            controller.clone(),
            &request,
            &admitted_current,
        )
        .await;
        let (document, expected) = match operation {
            Ok(value) => value,
            Err(message) => {
                if let Some(controller) = controller.upgrade() {
                    controller.fail_pending(&request, message);
                }
                return;
            }
        };
        let Some(owner) = controller.upgrade() else {
            return;
        };
        if !owner.owns_resolving(&request) {
            return;
        }
        let Some(current) = owner.current_for_request(&request.identity) else {
            owner.fail_pending(
                &request,
                "The mechanical settings scope changed before it could be saved.".into(),
            );
            return;
        };
        if current.accepted.token != request.identity.snapshot_token
            || current.accepted.document.revision != request.identity.revision
        {
            owner.fail_pending(
                &request,
                "The accepted document changed while the mechanical settings were being prepared. Review the current values and retry.".into(),
            );
            return;
        }
        let operation_id = (ports.next_operation)();
        // The root callback observes this ID before submitting; the resolving slot stays reserved
        // until the synchronous callback returns, so a reentrant field event cannot displace it.
        let outcome =
            match (ports.submit_replace)(operation_id, request.identity.revision, document) {
                Ok(outcome) => outcome,
                Err(message) => {
                    owner.fail_pending(&request, message);
                    return;
                }
            };
        if !owner.owns_resolving(&request) {
            return;
        }
        *owner.pending.borrow_mut() = Some(PendingRequest {
            request: request.clone(),
            phase: PendingPhase::Submitted {
                outcome,
                accepted_token: current.accepted.token,
                base_revision: current.accepted.document.revision,
                expected: Rc::new(expected),
            },
        });
    }

    async fn prepare(
        ports: MechanicalSettingsPorts,
        controller: Weak<Self>,
        request: &MechanicalSettingsRequest,
        current: &MechanicalSettingsCurrent,
    ) -> Result<(ProjectDoc, ExpectedCommit), String> {
        let accepted = &current.accepted;
        let base_document = &accepted.document;
        let instance_id = request.identity.scope.instance_id.as_deref();
        let next_configuration = match &request.patch {
            MechanicalSettingsPatch::Enable => {
                if current.configuration.is_some() {
                    return Err("Mechanical settings are already configured for this board.".into());
                }
                // Physical Configure follows React's effectiveCaseDocument: it starts from fresh
                // target-board defaults after Disable, even when canonical mechanical settings
                // remain stored for the project-level case.
                let physical_defaults;
                let defaults_document = if instance_id.is_some() {
                    physical_defaults = {
                        let mut document = base_document.as_ref().clone();
                        document.mechanical = None;
                        document
                    };
                    &physical_defaults
                } else {
                    base_document
                };
                Some(boardstudio_web::case_settings::initial_settings(
                    defaults_document,
                    &request.identity.active_board_id,
                )?)
            }
            MechanicalSettingsPatch::Disable => {
                if !current.configuration.as_ref().is_some_and(|config| {
                    config.board_id == request.identity.configuration_board_id
                        && config.board_id == request.identity.active_board_id
                }) {
                    return Err("There is no mechanical configuration to disable.".into());
                }
                None
            }
            MechanicalSettingsPatch::InitializeClosures => {
                let configuration = current
                    .configuration
                    .as_ref()
                    .filter(|_| can_initialize_closures(current, &request.identity))
                    .ok_or_else(|| {
                        "The current stack no longer needs mounting-location initialization."
                            .to_owned()
                    })?;
                Some(configuration.as_ref().clone())
            }
            _ => {
                let mut configuration = current
                    .configuration
                    .as_ref()
                    .filter(|config| {
                        config.board_id == request.identity.configuration_board_id
                            && config.board_id == request.identity.active_board_id
                    })
                    .map(|configuration| configuration.as_ref().clone())
                    .ok_or_else(|| {
                        "The active board has no matching mechanical configuration.".to_owned()
                    })?;
                apply_patch(
                    &mut configuration,
                    &request.patch,
                    base_document,
                    &request.identity.active_board_id,
                )?;
                Some(configuration)
            }
        };

        // React initializes closure mounts only when the setting is absent; explicit [] is a
        // deliberate accepted value. Resolution uses the proposed document and a fresh scene in
        // the root port, never a cached preview's contours.
        let mut configuration = next_configuration;
        if let Some(config) = configuration.as_mut()
            && config.closure_mounts.is_none()
            && config.mount != MechanicalMount::Gasket
        {
            let proposed_height = (
                config.plate_to_pcb,
                config.pcb_thickness,
                config.bottom_foam_thickness,
            );
            let proposed = persist_configuration(base_document, instance_id, Some(config.clone()))?;
            let assembly =
                (ports.resolve)(accepted.clone(), request.identity.scope.clone(), proposed).await?;
            if !Self::still_current(&ports, &controller, request) {
                return Err(
                    "The mechanical settings scope changed during mounting-location resolution."
                        .into(),
                );
            }
            *config = assembly.effective_configuration;
            if config.board_id != request.identity.configuration_board_id {
                return Err(
                    "Mechanical resolution returned settings for a different board.".into(),
                );
            }
            config.closure_mounts = Some(closure_mounts(
                proposed_height,
                config.battery_height,
                assembly.assembly,
            ));
        }

        // The bundled definition is loaded for each commit because the generator module is lazy.
        // It has already passed the application's normalizeDefinition path before this port returns.
        let mounting_hole = (ports.load_mounting_hole)().await?;
        if !Self::still_current(&ports, &controller, request) {
            return Err(
                "The mechanical settings scope changed while loading the mounting-hole definition."
                    .into(),
            );
        }
        let candidate = persist_configuration(base_document, instance_id, configuration)?;
        let candidate = (ports.project_closure_clearance)(candidate, &mounting_hole)?;
        let settings = expected_settings(&candidate, instance_id)?;
        let closure = closure_evidence(&candidate);
        Ok((candidate, ExpectedCommit { settings, closure }))
    }

    fn still_current(
        ports: &MechanicalSettingsPorts,
        controller: &Weak<Self>,
        request: &MechanicalSettingsRequest,
    ) -> bool {
        controller
            .upgrade()
            .is_some_and(|owner| owner.owns_resolving(request))
            && (ports.current)().is_some_and(|current| admitted(&current, &request.identity))
    }

    fn owns_resolving(&self, request: &MechanicalSettingsRequest) -> bool {
        self.pending.borrow().as_ref().is_some_and(|pending| {
            pending.request == *request && matches!(&pending.phase, PendingPhase::Resolving)
        })
    }

    fn current_for_request(
        &self,
        identity: &MechanicalSettingsIdentity,
    ) -> Option<MechanicalSettingsCurrent> {
        (self.ports.current)().filter(|current| admitted(current, identity))
    }

    fn emit(
        &self,
        request: &MechanicalSettingsRequest,
        state: MechanicalSettingsFeedbackState,
        message: Option<String>,
    ) {
        (self.ports.publish)(MechanicalSettingsFeedback {
            identity: request.identity.clone(),
            request_id: request.request_id,
            field_id: request.field_id.clone(),
            state,
            message,
        });
    }

    fn finish(
        &self,
        request: &MechanicalSettingsRequest,
        state: MechanicalSettingsFeedbackState,
        message: Option<String>,
    ) {
        self.clear_if_current(request);
        self.emit(request, state, message);
    }

    fn fail_pending(&self, request: &MechanicalSettingsRequest, message: String) {
        self.finish(
            request,
            MechanicalSettingsFeedbackState::Failed,
            Some(message),
        );
    }

    fn clear_if_current(&self, request: &MechanicalSettingsRequest) {
        let mut pending = self.pending.borrow_mut();
        if pending
            .as_ref()
            .is_some_and(|current| current.request == *request)
        {
            pending.take();
        }
    }
}

fn admitted(current: &MechanicalSettingsCurrent, identity: &MechanicalSettingsIdentity) -> bool {
    current.editable
        && current.identity == *identity
        && current.accepted.token == identity.snapshot_token
        && current.accepted.document.revision == identity.revision
        && current.accepted.session_epoch == identity.scope.session_epoch
        && current.accepted.document.id == identity.scope.document_id
        && identity.scope.board_id == identity.active_board_id
}

fn can_initialize_closures(
    current: &MechanicalSettingsCurrent,
    identity: &MechanicalSettingsIdentity,
) -> bool {
    current.configuration.as_ref().is_some_and(|configuration| {
        configuration.board_id == identity.configuration_board_id
            && configuration.board_id == identity.active_board_id
            && configuration.closure_mounts.is_none()
            && configuration.mount != MechanicalMount::Gasket
    })
}

fn same_owner(current: &MechanicalSettingsIdentity, request: &MechanicalSettingsIdentity) -> bool {
    current.editor_instance_id == request.editor_instance_id
        && current.scope_generation == request.scope_generation
        && current.presentation_generation == request.presentation_generation
        && current.scope == request.scope
        && current.active_board_id == request.active_board_id
        && current.configuration_board_id == request.configuration_board_id
}

/// Once the exact Runtime operation is submitted, leaving and returning to the same Case scope
/// does not cancel that Session-owned write. Ignore only the presentation generation here; the
/// original feedback identity is retained, and Resolving continuations still use `same_owner`.
fn same_submitted_owner(
    current: &MechanicalSettingsIdentity,
    request: &MechanicalSettingsIdentity,
) -> bool {
    current.editor_instance_id == request.editor_instance_id
        && current.scope_generation == request.scope_generation
        && current.scope == request.scope
        && current.active_board_id == request.active_board_id
        && current.configuration_board_id == request.configuration_board_id
}

fn persist_configuration(
    document: &ProjectDoc,
    instance_id: Option<&str>,
    configuration: Option<MechanicalConfiguration>,
) -> Result<ProjectDoc, String> {
    if let Some(instance_id) = instance_id {
        boardstudio_web::case_settings::update_instance_settings(
            document,
            instance_id,
            configuration,
        )
    } else {
        let mut next = document.clone();
        next.mechanical = configuration;
        Ok(next)
    }
}

fn expected_settings(
    document: &ProjectDoc,
    instance_id: Option<&str>,
) -> Result<ExpectedSettings, String> {
    let Some(instance_id) = instance_id else {
        return Ok(ExpectedSettings::Canonical(document.mechanical.clone()));
    };
    let hardware = document.hardware.as_ref().ok_or_else(|| {
        "Physical instances are unavailable after the mechanical update.".to_owned()
    })?;
    if !hardware
        .instances
        .iter()
        .any(|instance| instance.id == instance_id)
    {
        return Err(
            "The selected physical instance is unavailable after the mechanical update.".to_owned(),
        );
    }
    Ok(ExpectedSettings::Instance {
        instance_id: instance_id.to_owned(),
        shared: hardware.shared_construction.clone(),
        instances: hardware
            .instances
            .iter()
            .map(|instance| {
                (
                    instance.id.clone(),
                    instance.construction_linked,
                    instance.mechanical.clone(),
                )
            })
            .collect(),
    })
}

fn expected_matches(document: &ProjectDoc, expected: &ExpectedCommit) -> bool {
    let settings_match = match &expected.settings {
        ExpectedSettings::Canonical(configuration) => document.mechanical == *configuration,
        ExpectedSettings::Instance {
            instance_id,
            shared,
            instances,
        } => document.hardware.as_ref().is_some_and(|hardware| {
            hardware.shared_construction == *shared
                && hardware
                    .instances
                    .iter()
                    .any(|instance| instance.id == *instance_id)
                && hardware.instances.len() == instances.len()
                && hardware
                    .instances
                    .iter()
                    .zip(instances)
                    .all(|(actual, expected)| {
                        actual.id == expected.0
                            && actual.construction_linked == expected.1
                            && actual.mechanical == expected.2
                    })
        }),
    };
    settings_match && closure_evidence(document) == expected.closure
}

fn closure_evidence(document: &ProjectDoc) -> ClosureEvidence {
    let parts = document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .cloned()
        .collect();
    let definitions = document
        .definitions
        .iter()
        .filter(|definition| definition.id.starts_with("assembly-closure/definition/"))
        .map(|definition| ClosureDefinitionEvidence {
            id: definition.id.clone(),
            kind: definition.kind.clone(),
            generator: definition.generator.clone(),
        })
        .collect();
    let boards = document
        .boards
        .iter()
        .map(|board| {
            (
                board.id.clone(),
                board
                    .part_ids
                    .iter()
                    .filter(|id| id.starts_with("case-closure/"))
                    .cloned()
                    .collect(),
            )
        })
        .collect();
    let layouts = document
        .layouts
        .iter()
        .map(|layout| {
            (
                layout.id.clone(),
                layout
                    .part_ids
                    .iter()
                    .filter(|id| id.starts_with("case-closure/"))
                    .cloned()
                    .collect(),
            )
        })
        .collect();
    ClosureEvidence {
        parts,
        definitions,
        boards,
        layouts,
    }
}

fn closure_mounts(
    proposed_height: (f64, f64, f64),
    effective_battery_height: f64,
    assembly: MechanicalAssembly,
) -> Vec<Mount> {
    let (plate_to_pcb, pcb_thickness, bottom_foam_thickness) = proposed_height;
    assembly
        .suggested_mounts
        .into_iter()
        .map(|mut mount| {
            mount.id = format!("auto-closure/{}", mount.id);
            mount.kind = MountKind::Boss;
            mount.hole_diameter = 2.2;
            mount.height = Some(
                plate_to_pcb + pcb_thickness + bottom_foam_thickness.max(effective_battery_height),
            );
            mount
        })
        .collect()
}

fn apply_patch(
    configuration: &mut MechanicalConfiguration,
    patch: &MechanicalSettingsPatch,
    document: &ProjectDoc,
    board_id: &str,
) -> Result<(), String> {
    if matches!(patch, MechanicalSettingsPatch::InitializeClosures) {
        return Ok(());
    }
    let previous = configuration.clone();
    match patch {
        MechanicalSettingsPatch::Enable | MechanicalSettingsPatch::Disable => {
            return Err("Configure and Disable cannot be applied as configuration updates.".into());
        }
        MechanicalSettingsPatch::InitializeClosures => unreachable!("handled above"),
        MechanicalSettingsPatch::SetBatteryEnabled(enabled) => {
            if document
                .hardware
                .as_ref()
                .is_some_and(|hardware| hardware.transport == HardwareTransport::Wireless)
            {
                return Err(
                    "Wireless battery envelopes are managed by the wireless configuration.".into(),
                );
            }
            if *enabled {
                if configuration.battery.is_some() {
                    return Err("A battery envelope is already configured.".into());
                }
                configuration.battery = Some(MechanicalBattery {
                    cable_width: Some(2.0),
                    size: Vec3 {
                        x: 30.0,
                        y: 20.0,
                        z: 6.0,
                    },
                    at: Vec2 { x: 0.0, y: 0.0 },
                    cable_exit: Vec2 { x: 0.0, y: 0.0 },
                });
            } else {
                if configuration.battery.is_none() {
                    return Err("There is no battery envelope to remove.".into());
                }
                configuration.battery = None;
            }
        }
        MechanicalSettingsPatch::SetMethod(method) => configuration.method = method.clone(),
        MechanicalSettingsPatch::SetMount(mount) => {
            configuration.mount = mount.clone();
            if *mount == MechanicalMount::Gasket {
                if let Some(closures) = &mut configuration.closure_mounts {
                    closures.retain(|entry| !entry.id.starts_with("auto-closure/"));
                }
                configuration
                    .gasket_layout
                    .get_or_insert_with(default_gasket_layout);
                if configuration.internal_gasket.is_none() {
                    configuration.gasket_travel = Some(0.1);
                }
                configuration
                    .internal_gasket
                    .get_or_insert_with(default_internal_gasket);
                configuration.integrated_plate_frame = false;
                configuration.bottom_style = Some(MechanicalBottomStyle::Shell);
                configuration.middle_frame = Some(false);
            }
        }
        MechanicalSettingsPatch::SetBottomStyle(style) => {
            configuration.bottom_style = Some(style.clone());
        }
        MechanicalSettingsPatch::SetMiddleFrame(enabled) => {
            configuration.middle_frame = Some(*enabled);
        }
        MechanicalSettingsPatch::SetIntegratedPlateFrame(enabled) => {
            configuration.integrated_plate_frame = *enabled;
        }
        MechanicalSettingsPatch::SetDimension { field, value } => {
            if matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err(
                    "Gasket support dimensions require a current support selection.".into(),
                );
            }
            validate_dimension(*field, *value)?;
            set_dimension(configuration, *field, *value)?;
        }
        MechanicalSettingsPatch::SetGasketSupportDimension {
            support_id,
            anchors,
            field,
            value,
        } => {
            if !matches!(
                *field,
                MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth
            ) {
                return Err("The selected gasket field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            if support_id.is_empty()
                || anchors.is_empty()
                || anchors.len() > 2
                || !anchors.iter().any(|anchor| anchor.id == *support_id)
            {
                return Err("The selected gasket support is no longer available.".into());
            }
            let mut ids = std::collections::HashSet::new();
            if anchors
                .iter()
                .any(|anchor| anchor.id.is_empty() || !ids.insert(&anchor.id))
            {
                return Err("The linked gasket support selection is invalid.".into());
            }
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
                if *field == MechanicalDimension::GasketSupportLength {
                    anchor.length = Some(*value);
                } else {
                    anchor.width = Some(*value);
                }
                anchor.placement = Some(GasketPlacement::User);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|entry| entry.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
        }
        MechanicalSettingsPatch::SetGasketSupportUnlinked {
            support_id,
            pair_id,
            anchors,
        } => {
            if support_id.is_empty()
                || anchors.is_empty()
                || anchors.len() > 2
                || !anchors.iter().any(|anchor| anchor.id == *support_id)
            {
                return Err("The selected gasket support is no longer available.".into());
            }
            let mut ids = std::collections::HashSet::new();
            if anchors
                .iter()
                .any(|anchor| anchor.id.is_empty() || !ids.insert(&anchor.id))
            {
                return Err("The linked gasket support selection is invalid.".into());
            }
            let pair_is_present = pair_id
                .as_ref()
                .is_some_and(|pair_id| ids.contains(pair_id));
            if pair_id.as_deref() == Some(support_id.as_str())
                || pair_id.is_some() != pair_is_present
                || anchors.len() != if pair_id.is_some() { 2 } else { 1 }
            {
                return Err("The selected gasket support pair is no longer available.".into());
            }
            if anchors
                .iter()
                .find(|anchor| anchor.id == *support_id)
                .is_some_and(|anchor| anchor.unlinked)
            {
                return Err("The selected gasket support is already unlinked.".into());
            }
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
                anchor.unlinked = true;
                anchor.placement = Some(GasketPlacement::User);
                if let Some(existing) = layout
                    .supports
                    .iter_mut()
                    .find(|entry| entry.id == anchor.id)
                {
                    *existing = anchor;
                } else {
                    layout.supports.push(anchor);
                }
            }
        }
        MechanicalSettingsPatch::SetSwitchFamily {
            definition_id,
            family,
        } => {
            if !configuration
                .profiles
                .iter()
                .any(|profile| profile.definition_id == *definition_id)
            {
                return Err("The selected switch profile is no longer available.".into());
            }
            let plate_thickness = default_plate_thickness(*family);
            let plate_to_pcb = mounting_datum(*family) - plate_thickness;
            for profile in &mut configuration.profiles {
                if profile.definition_id == *definition_id {
                    profile.switch_family = Some(*family);
                    profile.plate_to_pcb = plate_to_pcb;
                } else if let Some(other_family) = profile.switch_family {
                    profile.plate_to_pcb = mounting_datum(other_family) - plate_thickness;
                }
            }
            configuration.plate_thickness = plate_thickness;
            configuration.plate_to_pcb = plate_to_pcb;
            configuration.plate_foam_thickness = default_plate_foam_thickness(plate_to_pcb);
        }
    }

    let family =
        profile_family(configuration).or_else(|| initial_switch_family(document, board_id));
    let new_plate_thickness = match patch {
        MechanicalSettingsPatch::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value,
        } => Some(*value),
        MechanicalSettingsPatch::SetSwitchFamily { .. } => Some(configuration.plate_thickness),
        _ => None,
    };
    if let Some(new_plate_thickness) = new_plate_thickness
        && let Some(family) = family
    {
        let previous_gap = mounting_datum(family) - previous.plate_thickness;
        configuration.plate_to_pcb = mounting_datum(family) - new_plate_thickness;
        for profile in &mut configuration.profiles {
            if profile.switch_family == Some(family) {
                profile.plate_to_pcb = configuration.plate_to_pcb;
            }
        }
        if (previous.plate_foam_thickness - default_plate_foam_thickness(previous_gap)).abs()
            < 0.001
        {
            configuration.plate_foam_thickness =
                default_plate_foam_thickness(configuration.plate_to_pcb);
        }
    }
    let battery_only_patch = matches!(patch, MechanicalSettingsPatch::SetBatteryEnabled(_))
        || matches!(
            patch,
            MechanicalSettingsPatch::SetDimension {
                field: MechanicalDimension::BatteryWidth
                    | MechanicalDimension::BatteryDepth
                    | MechanicalDimension::BatteryHeight
                    | MechanicalDimension::BatteryCableWidth
                    | MechanicalDimension::BatteryPositionX
                    | MechanicalDimension::BatteryPositionY
                    | MechanicalDimension::BatteryCableExitX
                    | MechanicalDimension::BatteryCableExitY,
                ..
            }
        );
    if !battery_only_patch {
        normalize_processes(configuration, patch);
    }
    Ok(())
}

fn set_dimension(
    configuration: &mut MechanicalConfiguration,
    field: MechanicalDimension,
    value: f64,
) -> Result<(), String> {
    match field {
        MechanicalDimension::PlateThickness => configuration.plate_thickness = value,
        MechanicalDimension::PlateFoamThickness => configuration.plate_foam_thickness = value,
        MechanicalDimension::PcbThickness => configuration.pcb_thickness = value,
        MechanicalDimension::BottomFoamThickness => configuration.bottom_foam_thickness = value,
        MechanicalDimension::BottomThickness => configuration.bottom_thickness = value,
        MechanicalDimension::WallThickness => configuration.wall_thickness = value,
        MechanicalDimension::Clearance => configuration.clearance = value,
        MechanicalDimension::OpeningAllowance => configuration.opening_allowance = Some(value),
        MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth => {
            return Err("Gasket support dimensions require a current support selection.".into());
        }
        MechanicalDimension::BatteryWidth
        | MechanicalDimension::BatteryDepth
        | MechanicalDimension::BatteryHeight
        | MechanicalDimension::BatteryCableWidth
        | MechanicalDimension::BatteryPositionX
        | MechanicalDimension::BatteryPositionY
        | MechanicalDimension::BatteryCableExitX
        | MechanicalDimension::BatteryCableExitY => {
            let battery = configuration.battery.as_mut().ok_or_else(|| {
                "The battery envelope was removed before this field update.".to_owned()
            })?;
            match field {
                MechanicalDimension::BatteryWidth => battery.size.x = value,
                MechanicalDimension::BatteryDepth => battery.size.y = value,
                MechanicalDimension::BatteryHeight => battery.size.z = value,
                MechanicalDimension::BatteryCableWidth => battery.cable_width = Some(value),
                MechanicalDimension::BatteryPositionX => battery.at.x = value,
                MechanicalDimension::BatteryPositionY => battery.at.y = value,
                MechanicalDimension::BatteryCableExitX => battery.cable_exit.x = value,
                MechanicalDimension::BatteryCableExitY => battery.cable_exit.y = value,
                _ => unreachable!("battery dimension variant checked above"),
            }
        }
    }
    Ok(())
}

fn validate_dimension(field: MechanicalDimension, value: f64) -> Result<(), String> {
    let valid = value.is_finite()
        && match field {
            MechanicalDimension::OpeningAllowance => (-1.0..=1.0).contains(&value),
            MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth => value >= 0.1,
            MechanicalDimension::GasketSupportLength => value >= 5.0,
            MechanicalDimension::GasketSupportWidth => value >= 0.5,
            MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY => value >= -1_000_000.0,
            _ => value >= 0.0,
        };
    if valid {
        Ok(())
    } else if field == MechanicalDimension::GasketSupportLength {
        Err("Cut length must be at least 5 mm.".into())
    } else if field == MechanicalDimension::GasketSupportWidth {
        Err("Pad width must be at least 0.5 mm.".into())
    } else if field == MechanicalDimension::OpeningAllowance {
        Err("Opening allowance must be between −1 and 1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryWidth
            | MechanicalDimension::BatteryDepth
            | MechanicalDimension::BatteryHeight
            | MechanicalDimension::BatteryCableWidth
    ) {
        Err("Battery dimensions and cable width must be at least 0.1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
    ) {
        Err("Battery positions and cable exits must be at least −1,000,000 mm.".into())
    } else {
        Err("Mechanical dimensions must be finite and nonnegative.".into())
    }
}

fn normalize_processes(
    configuration: &mut MechanicalConfiguration,
    patch: &MechanicalSettingsPatch,
) {
    let processes = configuration.part_processes.get_or_insert_with(Vec::new);
    for process in processes.iter_mut() {
        let is_standard = matches!(
            process.part_id.as_str(),
            "plate" | "plate-foam" | "bottom-foam" | "bottom"
        );
        let is_foam = process.part_id.ends_with("foam");
        let method = if is_foam {
            PlateMethod::CutSheet
        } else if process.part_id == "plate"
            || (matches!(patch, MechanicalSettingsPatch::SetMethod(_)) && is_standard)
        {
            configuration.method.clone()
        } else {
            process.method.clone()
        };
        let method_changed = method != process.method
            || (matches!(patch, MechanicalSettingsPatch::SetMethod(_)) && is_standard && !is_foam);
        let valid_material =
            material_options(&process.part_id, &method).contains(&process.material.as_str());
        if is_standard {
            if method_changed || !valid_material {
                process.material = default_material(&process.part_id, &method).into();
            }
            process.method = method;
            process.thickness = match process.part_id.as_str() {
                "plate" => configuration.plate_thickness,
                "plate-foam" => configuration.plate_foam_thickness,
                "bottom-foam" => configuration.bottom_foam_thickness,
                "bottom" => configuration.bottom_thickness,
                _ => process.thickness,
            };
        }
    }
}

fn material_options(part_id: &str, method: &PlateMethod) -> &'static [&'static str] {
    if part_id.ends_with("foam") {
        &["EVA"]
    } else {
        match method {
            PlateMethod::Printed => &["PLA", "ABS"],
            PlateMethod::Cnc => &["Aluminium"],
            PlateMethod::CutSheet => &["Acrylic"],
            PlateMethod::PcbFr4 => &["FR-4"],
        }
    }
}

fn default_material(part_id: &str, method: &PlateMethod) -> &'static str {
    if part_id.ends_with("foam") {
        "EVA"
    } else {
        match method {
            PlateMethod::Printed => "PLA",
            PlateMethod::Cnc => "Aluminium",
            PlateMethod::CutSheet => "Acrylic",
            PlateMethod::PcbFr4 => "FR-4",
        }
    }
}

fn profile_family(configuration: &MechanicalConfiguration) -> Option<MechanicalSwitchFamily> {
    let mut families = configuration
        .profiles
        .iter()
        .filter_map(|profile| profile.switch_family);
    let first = families.next()?;
    families.all(|family| family == first).then_some(first)
}

fn initial_switch_family(document: &ProjectDoc, board_id: &str) -> Option<MechanicalSwitchFamily> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let candidates = document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter_map(|part| {
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            let family = definition_family(definition, part);
            (definition.kind == PartKind::Switch
                || family.is_some()
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2")))
            .then_some(family)
        })
        .collect::<Vec<_>>();
    let [Some(first), rest @ ..] = candidates.as_slice() else {
        return None;
    };
    rest.iter()
        .all(|family| family.as_ref() == Some(first))
        .then_some(*first)
}

fn definition_family(definition: &PartDefinition, part: &Part) -> Option<MechanicalSwitchFamily> {
    let generator = definition.generator.as_ref()?;
    let source = generator.source.to_lowercase();
    if source == "ceoloide/switch_mx" {
        return Some(MechanicalSwitchFamily::Mx);
    }
    if !source.ends_with("/switch_choc_v1_v2") {
        return None;
    }
    let enabled = |name: &str| {
        part.generator_parameters
            .as_ref()
            .and_then(|parameters| parameters.get(name))
            .or_else(|| generator.parameters.get(name))
            .and_then(|value| {
                value
                    .as_bool()
                    .or_else(|| value.get("value").and_then(serde_json::Value::as_bool))
            })
            .unwrap_or(true)
    };
    match (enabled("choc_v1_support"), enabled("choc_v2_support")) {
        (true, false) => Some(MechanicalSwitchFamily::ChocV1),
        (false, true) => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
    }
}

fn default_plate_thickness(family: MechanicalSwitchFamily) -> f64 {
    if family == MechanicalSwitchFamily::ChocV1 {
        1.3
    } else {
        1.5
    }
}

fn mounting_datum(family: MechanicalSwitchFamily) -> f64 {
    if family == MechanicalSwitchFamily::ChocV1 {
        3.5
    } else {
        5.0
    }
}

fn default_plate_foam_thickness(gap: f64) -> f64 {
    (((gap - 0.2).min(3.0) + 0.000001) * 10.0).floor().max(0.0) / 10.0
}

fn patch_field_id(patch: &MechanicalSettingsPatch) -> String {
    match patch {
        MechanicalSettingsPatch::Enable => "configure".into(),
        MechanicalSettingsPatch::InitializeClosures => "initialize-closures".into(),
        MechanicalSettingsPatch::Disable => "disable".into(),
        MechanicalSettingsPatch::SetBatteryEnabled(_) => "battery-enabled".into(),
        MechanicalSettingsPatch::SetMethod(_) => "method".into(),
        MechanicalSettingsPatch::SetMount(_) => "mount".into(),
        MechanicalSettingsPatch::SetBottomStyle(_) => "bottom-style".into(),
        MechanicalSettingsPatch::SetMiddleFrame(_) => "middle-frame".into(),
        MechanicalSettingsPatch::SetIntegratedPlateFrame(_) => "integrated-plate-frame".into(),
        MechanicalSettingsPatch::SetDimension { field, .. } => match field {
            MechanicalDimension::PlateThickness => "plate-thickness".into(),
            MechanicalDimension::PlateFoamThickness => "plate-foam-thickness".into(),
            MechanicalDimension::PcbThickness => "pcb-thickness".into(),
            MechanicalDimension::BottomFoamThickness => "bottom-foam-thickness".into(),
            MechanicalDimension::BottomThickness => "bottom-thickness".into(),
            MechanicalDimension::WallThickness => "wall-thickness".into(),
            MechanicalDimension::Clearance => "clearance".into(),
            MechanicalDimension::GasketSupportLength => "gasket-support-length".into(),
            MechanicalDimension::GasketSupportWidth => "gasket-support-width".into(),
            MechanicalDimension::OpeningAllowance => "opening-allowance".into(),
            MechanicalDimension::BatteryWidth => "battery-width".into(),
            MechanicalDimension::BatteryDepth => "battery-depth".into(),
            MechanicalDimension::BatteryHeight => "battery-height".into(),
            MechanicalDimension::BatteryCableWidth => "battery-cable-width".into(),
            MechanicalDimension::BatteryPositionX => "battery-position-x".into(),
            MechanicalDimension::BatteryPositionY => "battery-position-y".into(),
            MechanicalDimension::BatteryCableExitX => "battery-cable-exit-x".into(),
            MechanicalDimension::BatteryCableExitY => "battery-cable-exit-y".into(),
        },
        MechanicalSettingsPatch::SetGasketSupportDimension {
            support_id, field, ..
        } => format!(
            "gasket-support:{support_id}:{}",
            match *field {
                MechanicalDimension::GasketSupportLength => "length",
                MechanicalDimension::GasketSupportWidth => "width",
                _ => "dimension",
            }
        ),
        MechanicalSettingsPatch::SetGasketSupportUnlinked { support_id, .. } => {
            format!("gasket-support:{support_id}:link")
        }
        MechanicalSettingsPatch::SetSwitchFamily { definition_id, .. } => {
            format!("switch-family:{definition_id}")
        }
    }
}

fn default_gasket_layout() -> MechanicalGasketLayout {
    MechanicalGasketLayout {
        auto_size: Some(true),
        adhesive_thickness: None,
        minimum_foam_thickness: None,
        preset_id: None,
        material: None,
        length: 80.0,
        width: 3.0,
        thickness: 2.0,
        compression: 0.15,
        supports: Vec::new(),
    }
}

fn default_internal_gasket() -> InternalGasketConfiguration {
    InternalGasketConfiguration {
        version: GasketConstructionVersion::InternalV1,
        minimum_wall: 2.0,
        support_clearance: None,
        tolerance: 0.05,
        support_count: 4,
        auto_count: Some(true),
        hardware: InternalClosureHardware {
            id: "custom-m2".into(),
            thread: "M2 × 0.4".into(),
            screw_lengths: vec![8.0, 10.0, 12.0, 14.0, 15.0, 16.0],
            thread_diameter: 2.0,
            pitch: 0.4,
            drive: ScrewDrive::Hex,
            installation: InsertInstallation::HeatSet,
            length_datum: ScrewLengthDatum::UnderHead,
            head_profile: ScrewHeadProfile::Flat,
            fixed_length: None,
            head_diameter: 4.0,
            head_height: 1.0,
            hole_diameter: 2.2,
            insert_diameter: 3.2,
            insert_length: 3.0,
            seat_diameter: 2.8,
            seat_depth: 3.5,
            engagement: 2.5,
            thread_start: 0.2,
            tip_allowance: 0.1,
            bottoming_clearance: 0.5,
            roof: 1.5,
            surround: 1.5,
            seat_lead_depth: 0.25,
            seat_lead_diameter: 3.0,
            bearing_thickness: 1.5,
        },
    }
}

#[cfg(test)]
mod battery_patch_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn configuration() -> MechanicalConfiguration {
        MechanicalConfiguration {
            internal_gasket: None,
            gasket_layout: None,
            hardware: None,
            critical_fits: None,
            bottom_style: Some(MechanicalBottomStyle::Shell),
            middle_frame: Some(false),
            gasket_travel: None,
            openings: None,
            opening_allowance: Some(0.25),
            stabilizers: None,
            part_processes: None,
            gasket: None,
            closure_mounts: None,
            board_id: "board".into(),
            integrated_plate_frame: false,
            battery: None,
            mounts: vec![],
            method: PlateMethod::Printed,
            mount: MechanicalMount::Rigid,
            plate_thickness: 1.5,
            plate_foam_thickness: 0.5,
            pcb_thickness: 1.6,
            bottom_foam_thickness: 0.4,
            battery_height: 4.0,
            bottom_thickness: 3.0,
            plate_to_pcb: 3.5,
            wall_thickness: 2.0,
            clearance: 0.3,
            profiles: vec![],
        }
    }

    #[wasm_bindgen_test]
    fn legacy_gasket_support_resize_preserves_saved_anchor_metadata() {
        use boardstudio_core::model::MechanicalGasketAnchor;

        let mut configuration = configuration();
        configuration.mount = MechanicalMount::Gasket;
        configuration.part_processes = Some(vec![]);
        let anchors = vec![
            MechanicalGasketAnchor {
                id: "left:0".into(),
                region_id: "left".into(),
                outline_key: "left-outline".into(),
                anchor: 0.25,
                length: Some(12.0),
                width: Some(3.0),
                placement: Some(GasketPlacement::User),
                unlinked: false,
            },
            MechanicalGasketAnchor {
                id: "right:0".into(),
                region_id: "right".into(),
                outline_key: "right-outline".into(),
                anchor: 0.75,
                length: Some(12.0),
                width: Some(3.0),
                placement: Some(GasketPlacement::User),
                unlinked: false,
            },
        ];
        let mut layout = default_gasket_layout();
        layout.supports = anchors.clone();
        configuration.gasket_layout = Some(layout);
        assert!(configuration.internal_gasket.is_none());
        let mut expected = configuration.clone();
        for anchor in &mut expected.gasket_layout.as_mut().unwrap().supports {
            anchor.length = Some(20.0);
        }

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::SetGasketSupportDimension {
                support_id: "left:0".into(),
                anchors,
                field: MechanicalDimension::GasketSupportLength,
                value: 20.0,
            },
            &ProjectDoc::empty("doc", "doc"),
            "board",
        )
        .expect("accepted legacy supports must retain the reference resize operation");
        assert_eq!(configuration, expected);
    }

    #[wasm_bindgen_test]
    fn enabling_wired_battery_uses_reference_defaults_without_changing_other_settings() {
        let mut configuration = configuration();
        let before = configuration.clone();

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::SetBatteryEnabled(true),
            &ProjectDoc::empty("doc", "doc"),
            "board",
        )
        .unwrap();

        assert_eq!(configuration.board_id, before.board_id);
        assert_eq!(configuration.method, before.method);
        assert_eq!(configuration.mount, before.mount);
        assert_eq!(configuration.plate_thickness, before.plate_thickness);
        assert_eq!(configuration.bottom_thickness, before.bottom_thickness);
        assert_eq!(configuration.part_processes, before.part_processes);
        assert_eq!(
            configuration.battery,
            Some(MechanicalBattery {
                cable_width: Some(2.0),
                size: Vec3 {
                    x: 30.0,
                    y: 20.0,
                    z: 6.0,
                },
                at: Vec2 { x: 0.0, y: 0.0 },
                cable_exit: Vec2 { x: 0.0, y: 0.0 },
            })
        );
    }

    #[wasm_bindgen_test]
    fn removing_battery_preserves_all_non_battery_settings() {
        let mut configuration = configuration();
        configuration.battery = Some(MechanicalBattery {
            cable_width: None,
            size: Vec3 {
                x: 42.0,
                y: 18.0,
                z: 5.0,
            },
            at: Vec2 { x: 8.0, y: -3.0 },
            cable_exit: Vec2 { x: 40.0, y: -3.0 },
        });
        let mut expected = configuration.clone();
        expected.battery = None;

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::SetBatteryEnabled(false),
            &ProjectDoc::empty("doc", "doc"),
            "board",
        )
        .unwrap();

        assert_eq!(configuration, expected);
    }

    #[wasm_bindgen_test]
    fn battery_fields_update_their_own_values_and_reject_missing_or_invalid_envelopes() {
        let mut configuration = configuration();
        configuration.battery = Some(MechanicalBattery {
            cable_width: None,
            size: Vec3 {
                x: 30.0,
                y: 20.0,
                z: 6.0,
            },
            at: Vec2 { x: 0.0, y: 0.0 },
            cable_exit: Vec2 { x: 0.0, y: 0.0 },
        });
        let fields = [
            (MechanicalDimension::BatteryWidth, 31.0),
            (MechanicalDimension::BatteryDepth, 21.0),
            (MechanicalDimension::BatteryHeight, 7.0),
            (MechanicalDimension::BatteryCableWidth, 2.5),
            (MechanicalDimension::BatteryPositionX, -12.0),
            (MechanicalDimension::BatteryPositionY, 13.0),
            (MechanicalDimension::BatteryCableExitX, 30.0),
            (MechanicalDimension::BatteryCableExitY, -5.0),
        ];
        for (field, value) in fields {
            apply_patch(
                &mut configuration,
                &MechanicalSettingsPatch::SetDimension { field, value },
                &ProjectDoc::empty("doc", "doc"),
                "board",
            )
            .unwrap();
        }
        assert_eq!(
            configuration.battery,
            Some(MechanicalBattery {
                cable_width: Some(2.5),
                size: Vec3 {
                    x: 31.0,
                    y: 21.0,
                    z: 7.0,
                },
                at: Vec2 { x: -12.0, y: 13.0 },
                cable_exit: Vec2 { x: 30.0, y: -5.0 },
            })
        );

        assert!(
            apply_patch(
                &mut configuration,
                &MechanicalSettingsPatch::SetDimension {
                    field: MechanicalDimension::BatteryWidth,
                    value: 0.0,
                },
                &ProjectDoc::empty("doc", "doc"),
                "board",
            )
            .is_err()
        );
        configuration.battery = None;
        assert!(
            apply_patch(
                &mut configuration,
                &MechanicalSettingsPatch::SetDimension {
                    field: MechanicalDimension::BatteryWidth,
                    value: 1.0,
                },
                &ProjectDoc::empty("doc", "doc"),
                "board",
            )
            .unwrap_err()
            .contains("removed")
        );
    }

    #[wasm_bindgen_test]
    fn wireless_transport_rejects_manual_battery_toggle() {
        let mut configuration = configuration();
        let mut document = ProjectDoc::empty("doc", "doc");
        document.hardware = Some(boardstudio_core::model::HardwareConfiguration {
            topology: Default::default(),
            transport: HardwareTransport::Wireless,
            instances: vec![],
            boards: vec![],
            shared_construction: None,
        });

        assert!(
            apply_patch(
                &mut configuration,
                &MechanicalSettingsPatch::SetBatteryEnabled(true),
                &document,
                "board",
            )
            .unwrap_err()
            .contains("Wireless")
        );
        assert!(configuration.battery.is_none());
    }
}
