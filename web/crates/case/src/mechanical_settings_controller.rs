//! Private owner for the Case mechanical-settings field requests.
//!
//! Catalogue resources are loaded before dispatch, in commit order. Each edit ticket
//! then plans from the accepted snapshot with Core's pure mechanical functions and the
//! deterministic closure-clearance projection; settlement never compares document content.
use super::mechanical_settings::{
    MechanicalDimension, MechanicalMountCollection, MechanicalSettingsFeedback,
    MechanicalSettingsFeedbackState, MechanicalSettingsIdentity, MechanicalSettingsPatch,
    MechanicalSettingsRequest,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core::model::{
    CaseOpening, EditCommand, EditOperation, EditPhase, GasketConstructionVersion, GasketPlacement,
    HardwareTransport, InsertInstallation, InternalClosureHardware, InternalGasketConfiguration,
    MechanicalAssembly, MechanicalBattery, MechanicalBottomStyle, MechanicalBuiltinProfile,
    MechanicalConfiguration, MechanicalCriticalFit, MechanicalGasketLayout,
    MechanicalHardwareSpecification, MechanicalMount, MechanicalPartProcess, MechanicalPartProfile,
    MechanicalSwitchFamily, Mount, MountKind, Part, PartDefinition, PartKind, PlateMethod,
    ProjectDoc, ScrewDrive, ScrewHeadProfile, ScrewLengthDatum, Vec2, Vec3,
};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

type LocalFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;
#[derive(Clone, Copy)]
enum ProfileTargetKind {
    Switch,
    Stabilizer,
    ImportedGeometry,
}

/// The page copies only accepted Arc handles and the small configuration projection displayed
/// by the controls. `editable` must be computed from the current Runtime, workspace,
/// physical-instance, readiness, preview, gesture, lifecycle and durability admission guards.
/// It must exclude this controller's own in-flight bit; use `is_busy` to disable the UI. The
/// controller still re-reads it at every asynchronous boundary.
#[derive(Clone)]
pub struct MechanicalSettingsCurrent {
    pub identity: MechanicalSettingsIdentity,
    pub accepted: AcceptedSnapshot,
    pub configuration: Option<Rc<MechanicalConfiguration>>,
    pub editable: bool,
    pub lifecycle: Lifecycle,
    pub durability: Durability,
}

/// The page's existing effective-case policy is the source for both a fresh Core resolution and
/// generated boss height (notably the wireless battery default). The reflected/effective document
/// remains read-only; only this mechanical value is written back through the canonical ownership
/// mapping.
#[derive(Clone)]
pub struct MechanicalResolution {
    pub assembly: MechanicalAssembly,
    pub effective_configuration: MechanicalConfiguration,
}

/// Runtime supplies admission, catalogue loading and the edit-ticket port. Only the
/// normalized catalogue definition is prepared asynchronously; document planning runs
/// against the accepted snapshot when Session executes the intent.
#[derive(Clone)]
pub struct MechanicalSettingsPorts {
    pub current: Rc<dyn Fn() -> Option<MechanicalSettingsCurrent>>,
    pub load_mounting_hole: Rc<dyn Fn() -> LocalFuture<Result<Rc<PartDefinition>, String>>>,
    pub begin_edit: Rc<dyn Fn(EditResolver) -> EditTicket>,
    pub publish: Rc<dyn Fn(MechanicalSettingsFeedback)>,
}

pub struct MechanicalSettingsController {
    ports: MechanicalSettingsPorts,
    requests: RefCell<Vec<SettingsEdit>>,
    last_request_id: Cell<u64>,
}

#[derive(Clone)]
struct SettingsEdit {
    request: MechanicalSettingsRequest,
    phase: SettingsPhase,
}

#[derive(Clone)]
enum SettingsPhase {
    Preparing,
    Prepared(EditResolver),
    Submitted(EditTicket),
}

impl MechanicalSettingsController {
    pub fn new(ports: MechanicalSettingsPorts) -> Rc<Self> {
        Rc::new(Self {
            ports,
            requests: RefCell::new(Vec::new()),
            last_request_id: Cell::new(0),
        })
    }

    pub fn is_busy(&self) -> bool {
        !self.requests.borrow().is_empty()
    }

    pub fn submit(self: &Rc<Self>, request: MechanicalSettingsRequest) -> bool {
        if request.request_id <= self.last_request_id.get() {
            return false;
        }
        self.last_request_id.set(request.request_id);
        let Some(current) = (self.ports.current)() else {
            return false;
        };
        if !admitted(&current, &request.identity)
            || request.field_id != patch_field_id(&request.patch)
        {
            return false;
        }
        if is_one_shot(&request.patch)
            && self
                .requests
                .borrow()
                .iter()
                .any(|pending| pending.request.field_id == request.field_id)
        {
            return false;
        }
        self.requests.borrow_mut().push(SettingsEdit {
            request: request.clone(),
            phase: SettingsPhase::Preparing,
        });
        self.emit(&request, MechanicalSettingsFeedbackState::Pending, None);
        let owner = Rc::downgrade(self);
        let loader = self.ports.load_mounting_hole.clone();
        spawn_local(async move {
            let result = loader().await;
            let Some(owner) = owner.upgrade() else {
                return;
            };
            if !(owner.ports.current)()
                .is_some_and(|current| same_owner(&current.identity, &request.identity))
            {
                owner
                    .requests
                    .borrow_mut()
                    .retain(|pending| pending.request.request_id != request.request_id);
                owner.flush_prepared();
                return;
            }
            match result {
                Ok(template) => {
                    let intent = request.clone();
                    let resolver = EditResolver::new(
                        "mechanical-settings",
                        move |accepted: &AcceptedSnapshot| match prepare_document(
                            accepted, &intent, &template,
                        ) {
                            Ok(document) if document == *accepted.document => Resolution::Unchanged,
                            Ok(document) => Resolution::Submit(EditCommand {
                                base_revision: 0,
                                transaction_id: String::new(),
                                phase: EditPhase::Commit,
                                target_ids: vec![intent.identity.active_board_id.clone()],
                                operation: EditOperation::ReplaceDocument {
                                    document: Box::new(document),
                                },
                            }),
                            Err(message) => Resolution::Retire(message),
                        },
                    );
                    if let Some(pending) = owner
                        .requests
                        .borrow_mut()
                        .iter_mut()
                        .find(|pending| pending.request.request_id == request.request_id)
                    {
                        pending.phase = SettingsPhase::Prepared(resolver);
                    }
                }
                Err(message) => {
                    owner
                        .requests
                        .borrow_mut()
                        .retain(|pending| pending.request.request_id != request.request_id);
                    owner.emit(
                        &request,
                        MechanicalSettingsFeedbackState::Failed,
                        Some(message),
                    );
                }
            }
            owner.flush_prepared();
        });
        true
    }

    // Preserve commit order even if catalogue loads complete in a different order.
    fn flush_prepared(&self) {
        let prepared = {
            let requests = self.requests.borrow();
            requests
                .iter()
                .take_while(|pending| !matches!(pending.phase, SettingsPhase::Preparing))
                .filter_map(|pending| match &pending.phase {
                    SettingsPhase::Prepared(resolver) => {
                        Some((pending.request.request_id, resolver.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        for (id, resolver) in prepared {
            let ticket = (self.ports.begin_edit)(resolver);
            if let Some(pending) = self
                .requests
                .borrow_mut()
                .iter_mut()
                .find(|pending| pending.request.request_id == id)
            {
                pending.phase = SettingsPhase::Submitted(ticket);
            }
        }
    }

    pub fn settle(&self) {
        let current = (self.ports.current)();
        let requests = self.requests.borrow().clone();
        for pending in requests {
            let live = current.as_ref().is_some_and(|current| {
                same_submitted_owner(&current.identity, &pending.request.identity)
            });
            let settlement = match &pending.phase {
                SettingsPhase::Submitted(ticket) => ticket.settlement(live),
                _ if live => continue,
                _ => Settlement::Retired,
            };
            match settlement {
                Settlement::Pending => continue,
                Settlement::Landed { .. } => self.emit(
                    &pending.request,
                    MechanicalSettingsFeedbackState::Saved,
                    None,
                ),
                Settlement::Failed { message } => self.emit(
                    &pending.request,
                    MechanicalSettingsFeedbackState::Failed,
                    Some(message),
                ),
                Settlement::Retired => {}
            }
            self.requests
                .borrow_mut()
                .retain(|entry| entry.request.request_id != pending.request.request_id);
        }
        self.flush_prepared();
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
}

fn is_one_shot(patch: &MechanicalSettingsPatch) -> bool {
    matches!(
        patch,
        MechanicalSettingsPatch::InitializeClosures
            | MechanicalSettingsPatch::ResetGasketPlacement
            | MechanicalSettingsPatch::AddHardware { .. }
            | MechanicalSettingsPatch::RemoveHardware { .. }
            | MechanicalSettingsPatch::RemoveMount { .. }
            | MechanicalSettingsPatch::RemoveProfile { .. }
            | MechanicalSettingsPatch::AddProcessOverride { .. }
            | MechanicalSettingsPatch::RemoveProcessOverride { .. }
            | MechanicalSettingsPatch::AddCriticalFit { .. }
            | MechanicalSettingsPatch::RemoveCriticalFit { .. }
            | MechanicalSettingsPatch::AssignSwitchProfile { .. }
            | MechanicalSettingsPatch::AssignStabilizerProfile { .. }
            | MechanicalSettingsPatch::AssignImportedGeometryProfile { .. }
    )
}

fn prepare_document(
    accepted: &AcceptedSnapshot,
    request: &MechanicalSettingsRequest,
    mounting_hole: &PartDefinition,
) -> Result<ProjectDoc, String> {
    let base_document = &accepted.document;
    let instance_id = request.identity.scope.instance_id.as_deref();
    let effective =
        boardstudio_web_host::cad_jobs::captured_case_document(accepted, &request.identity.scope)
            .map_err(|error| format!("The mechanical settings scope is unavailable: {error:?}"))?;
    let configuration = effective.mechanical.map(Rc::new);
    let next_configuration = match &request.patch {
        MechanicalSettingsPatch::Enable => {
            if configuration.is_some() {
                return Ok(base_document.as_ref().clone());
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
            Some(boardstudio_web_host::case_settings::initial_settings(
                defaults_document,
                &request.identity.active_board_id,
            )?)
        }
        MechanicalSettingsPatch::Disable => {
            if !configuration.as_ref().is_some_and(|config| {
                config.board_id == request.identity.configuration_board_id
                    && config.board_id == request.identity.active_board_id
            }) {
                return Ok(base_document.as_ref().clone());
            }
            None
        }
        MechanicalSettingsPatch::InitializeClosures => {
            let configuration = configuration
                .as_ref()
                .filter(|configuration| {
                    configuration.closure_mounts.is_none()
                        && configuration.mount != MechanicalMount::Gasket
                })
                .ok_or_else(|| {
                    "The current stack no longer needs mounting-location initialization.".to_owned()
                })?;
            Some(configuration.as_ref().clone())
        }
        _ => {
            let mut configuration = configuration
                .as_ref()
                .filter(|config| {
                    config.board_id == request.identity.configuration_board_id
                        && config.board_id == request.identity.active_board_id
                })
                .map(|configuration| configuration.as_ref().clone())
                .ok_or_else(|| {
                    "The active board has no matching mechanical configuration.".to_owned()
                })?;
            match &request.patch {
                MechanicalSettingsPatch::AssignSwitchProfile {
                    definition_id,
                    family,
                } => {
                    validate_profile_target(
                        base_document,
                        &request.identity.active_board_id,
                        &configuration,
                        definition_id,
                        ProfileTargetKind::Switch,
                    )?;
                    let existing_family = profile_family(&configuration)
                        .or_else(|| initial_switch_family(base_document, &configuration.board_id));
                    let family_changed = family_assignment_changes(existing_family, *family);
                    let plate_thickness = if family_changed {
                        default_plate_thickness(*family)
                    } else {
                        configuration.plate_thickness
                    };
                    let plate_to_pcb = mounting_datum(*family) - plate_thickness;
                    let source = match family {
                        MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
                        MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
                        MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
                    };
                    let mut profile = boardstudio_core::mechanical::builtin_profile(
                        definition_id.clone(),
                        source,
                        plate_to_pcb,
                    )?;
                    if profile.definition_id != *definition_id {
                        return Err("Core returned a switch profile for another part type.".into());
                    }
                    profile.switch_family = Some(*family);
                    profile.plate_to_pcb = plate_to_pcb;
                    if family_changed {
                        configuration.plate_thickness = plate_thickness;
                        configuration.plate_to_pcb = plate_to_pcb;
                        configuration.plate_foam_thickness =
                            default_plate_foam_thickness(plate_to_pcb);
                    }
                    configuration.profiles.push(profile);
                }
                MechanicalSettingsPatch::AssignStabilizerProfile {
                    definition_id,
                    source,
                } => {
                    if !matches!(
                        source,
                        MechanicalBuiltinProfile::MxStab2u | MechanicalBuiltinProfile::MxStab625u
                    ) {
                        return Err("The selected stabilizer profile is not supported.".into());
                    }
                    validate_profile_target(
                        base_document,
                        &request.identity.active_board_id,
                        &configuration,
                        definition_id,
                        ProfileTargetKind::Stabilizer,
                    )?;
                    let mut profile = boardstudio_core::mechanical::builtin_profile(
                        definition_id.clone(),
                        source.clone(),
                        configuration.plate_to_pcb,
                    )?;
                    if profile.definition_id != *definition_id || profile.switch_family.is_some() {
                        return Err("Core returned a different stabilizer profile.".into());
                    }
                    profile.plate_to_pcb = configuration.plate_to_pcb;
                    configuration.profiles.push(profile);
                }
                MechanicalSettingsPatch::AssignImportedGeometryProfile { definition_id } => {
                    validate_profile_target(
                        base_document,
                        &request.identity.active_board_id,
                        &configuration,
                        definition_id,
                        ProfileTargetKind::ImportedGeometry,
                    )?;
                    let definition = base_document
                        .definitions
                        .iter()
                        .find(|definition| definition.id == *definition_id)
                        .ok_or_else(|| {
                            "The imported part definition is no longer available.".to_owned()
                        })?;
                    let family = base_document
                        .parts
                        .iter()
                        .find(|part| {
                            part.definition_id == *definition_id
                                && base_document.boards.iter().any(|board| {
                                    board.id == request.identity.active_board_id
                                        && board.part_ids.contains(&part.id)
                                })
                        })
                        .and_then(|part| definition_family(definition, part));
                    configuration.profiles.push(MechanicalPartProfile {
                        source_geometry: None,
                        pcb_holes: None,
                        clearance_volumes: None,
                        openings: None,
                        clearances: None,
                        supported_thickness: None,
                        switch_family: family,
                        definition_id: definition.id.clone(),
                        source: format!("KiCad {}", definition.name),
                        cutouts: Vec::new(),
                        plate_to_pcb: family.map_or(configuration.plate_to_pcb, |family| {
                            mounting_datum(family) - configuration.plate_thickness
                        }),
                    });
                }
                _ => apply_patch(
                    &mut configuration,
                    &request.patch,
                    base_document,
                    &request.identity.active_board_id,
                )?,
            }
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
        let assembly = resolve_proposed(accepted, &request.identity.scope, proposed)?;
        *config = assembly.effective_configuration;
        if config.board_id != request.identity.configuration_board_id {
            return Err("Mechanical resolution returned settings for a different board.".into());
        }
        config.closure_mounts = Some(closure_mounts(
            proposed_height,
            config.battery_height,
            assembly.assembly,
        ));
    }

    let candidate = persist_configuration(base_document, instance_id, configuration)?;
    crate::closure_clearance::project_owned_closure_clearance(candidate, mounting_hole)
}

fn resolve_proposed(
    accepted: &AcceptedSnapshot,
    scope: &Scope,
    proposed: ProjectDoc,
) -> Result<MechanicalResolution, String> {
    let projected = AcceptedSnapshot {
        document: std::sync::Arc::new(proposed),
        ..accepted.clone()
    };
    let effective = boardstudio_web_host::cad_jobs::captured_case_document(&projected, scope)
        .map_err(|error| format!("Could not project proposed mechanical settings: {error:?}"))?;
    let effective_configuration = effective
        .mechanical
        .clone()
        .ok_or("The mechanical configuration no longer exists.")?;
    let contours = boardstudio_web_host::cad_jobs::captured_case_scene(accepted, scope)
        .map_err(|error| format!("Could not project accepted case contours: {error:?}"))?
        .board_contours
        .into_iter()
        .find(|board| board.board_id == scope.board_id)
        .map_or_else(Vec::new, |board| board.contours);
    Ok(MechanicalResolution {
        assembly: boardstudio_core::mechanical::resolve(&effective, &contours),
        effective_configuration,
    })
}

fn validate_profile_target(
    document: &ProjectDoc,
    board_id: &str,
    configuration: &MechanicalConfiguration,
    definition_id: &str,
    target_kind: ProfileTargetKind,
) -> Result<(), String> {
    if configuration.board_id != board_id {
        return Err("The mechanical configuration belongs to another board.".into());
    }
    if configuration
        .profiles
        .iter()
        .any(|profile| profile.definition_id == definition_id)
    {
        return Err("A mechanical profile is already assigned to this part type.".into());
    }
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| "The selected board is no longer available.".to_owned())?;
    let part = document
        .parts
        .iter()
        .find(|part| board.part_ids.contains(&part.id) && part.definition_id == definition_id);
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == definition_id);
    let eligible = match (part, definition) {
        (Some(_), Some(definition)) if matches!(target_kind, ProfileTargetKind::Switch) => {
            definition.kind == PartKind::Switch
                || definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source.ends_with("/switch_choc_v1_v2"))
        }
        (Some(_), Some(_)) if matches!(target_kind, ProfileTargetKind::Stabilizer) => true,
        (Some(_), Some(definition)) => definition.kicad_source.is_some(),
        _ => false,
    };
    if !eligible {
        return Err("The selected part type is not available for this profile assignment.".into());
    }
    Ok(())
}

fn admitted(current: &MechanicalSettingsCurrent, identity: &MechanicalSettingsIdentity) -> bool {
    current.editable
        && same_owner(&current.identity, identity)
        && current.accepted.session_epoch == identity.scope.session_epoch
        && current.accepted.document.id == identity.scope.document_id
        && identity.scope.board_id == identity.active_board_id
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
        boardstudio_web_host::case_settings::update_instance_settings(
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
        MechanicalSettingsPatch::SetClosureInsertPreset(id) => {
            resize_closure_insert(closure_hardware_mut(configuration)?, id)?;
        }
        MechanicalSettingsPatch::SetClosureDrive(drive) => {
            closure_hardware_mut(configuration)?.drive = drive.clone();
        }
        MechanicalSettingsPatch::SetClosureInstallation(installation) => {
            closure_hardware_mut(configuration)?.installation = installation.clone();
        }
        MechanicalSettingsPatch::SetClosureFixedLength(length) => {
            let hardware = closure_hardware_mut(configuration)?;
            if let Some(length) = length {
                if !length.is_finite() || *length <= 0.0 || !hardware.screw_lengths.contains(length)
                {
                    return Err(
                        "Choose an available positive screw length or automatic length.".into(),
                    );
                }
            }
            hardware.fixed_length = *length;
        }
        MechanicalSettingsPatch::SetClosureThread(thread) => {
            closure_hardware_mut(configuration)?.thread = thread.clone();
        }
        MechanicalSettingsPatch::SetClosureScrewLengths(lengths) => {
            if lengths.is_empty()
                || lengths
                    .iter()
                    .any(|length| !length.is_finite() || *length <= 0.0)
            {
                return Err("Enter positive lengths separated by commas.".into());
            }
            let hardware = closure_hardware_mut(configuration)?;
            hardware.screw_lengths = lengths.clone();
            if hardware
                .fixed_length
                .is_some_and(|fixed| !hardware.screw_lengths.contains(&fixed))
            {
                hardware.fixed_length = None;
            }
        }
        MechanicalSettingsPatch::SetClosureHeadProfile(profile) => {
            closure_hardware_mut(configuration)?.head_profile = profile.clone();
        }
        MechanicalSettingsPatch::SetClosureLengthDatum(datum) => {
            closure_hardware_mut(configuration)?.length_datum = datum.clone();
        }
        MechanicalSettingsPatch::AddCriticalFit { part_id } => {
            if part_id.is_empty() {
                return Err("A generated part must be selected for a critical fit.".into());
            }
            let fits = configuration.critical_fits.get_or_insert_with(Vec::new);
            let mut suffix = 1_u64;
            let fit_id = loop {
                let candidate = format!("case-critical-fit-{suffix}");
                if !fits.iter().any(|fit| fit.id == candidate) {
                    break candidate;
                }
                suffix = suffix
                    .checked_add(1)
                    .ok_or_else(|| "Critical fit identity is exhausted.".to_owned())?;
            };
            fits.push(MechanicalCriticalFit {
                id: fit_id,
                part_id: part_id.clone(),
                label: "Critical dimension".into(),
                from: Vec2 { x: 0.0, y: 0.0 },
                to: Vec2 { x: 10.0, y: 0.0 },
                tolerance: "±0.2 mm".into(),
            });
        }
        MechanicalSettingsPatch::RemoveCriticalFit { fit_id } => {
            let fits = configuration
                .critical_fits
                .as_mut()
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            let index = fits
                .iter()
                .position(|fit| fit.id == *fit_id)
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            fits.remove(index);
        }
        MechanicalSettingsPatch::SetCriticalFitText {
            fit_id,
            field,
            value,
        } => {
            let fit = critical_fit_mut(configuration, fit_id)?;
            match field {
                super::mechanical_settings::MechanicalCriticalFitTextField::Label => {
                    fit.label = value.clone()
                }
                super::mechanical_settings::MechanicalCriticalFitTextField::Tolerance => {
                    fit.tolerance = value.clone()
                }
            }
        }
        MechanicalSettingsPatch::SetCriticalFitPart { fit_id, part_id } => {
            if part_id.is_empty() {
                return Err("A generated part must be selected for a critical fit.".into());
            }
            critical_fit_mut(configuration, fit_id)?.part_id = part_id.clone();
        }
        MechanicalSettingsPatch::SetCriticalFitPoint {
            fit_id,
            field,
            value,
        } => {
            if !matches!(
                *field,
                MechanicalDimension::CriticalFitFromX
                    | MechanicalDimension::CriticalFitFromY
                    | MechanicalDimension::CriticalFitToX
                    | MechanicalDimension::CriticalFitToY
            ) {
                return Err("The selected critical-fit endpoint is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            let fit = critical_fit_mut(configuration, fit_id)?;
            match field {
                MechanicalDimension::CriticalFitFromX => fit.from.x = *value,
                MechanicalDimension::CriticalFitFromY => fit.from.y = *value,
                MechanicalDimension::CriticalFitToX => fit.to.x = *value,
                MechanicalDimension::CriticalFitToY => fit.to.y = *value,
                _ => return Err("The selected critical-fit endpoint is unavailable.".into()),
            }
        }
        MechanicalSettingsPatch::AddHardware {
            part_id,
            feature_id,
        } => {
            if part_id.is_empty() || feature_id.is_empty() {
                return Err("Link the hardware to a generated mount feature.".into());
            }
            let entries = configuration.hardware.get_or_insert_with(Vec::new);
            let mut suffix = 1_u64;
            let hardware_id = loop {
                let candidate = format!("case-mechanical-hardware-{suffix}");
                if !entries.iter().any(|entry| entry.id == candidate) {
                    break candidate;
                }
                suffix = suffix
                    .checked_add(1)
                    .ok_or_else(|| "Hardware identity is exhausted.".to_owned())?;
            };
            entries.push(MechanicalHardwareSpecification {
                id: hardware_id,
                part_id: part_id.clone(),
                feature_id: feature_id.clone(),
                designation: "Socket screw".into(),
                thread: "M2 × 0.4".into(),
                length: 6.0,
                quantity: 4,
                notes: Some(String::new()),
                tolerance: None,
            });
        }
        MechanicalSettingsPatch::RemoveHardware { hardware_id } => {
            let entries = configuration
                .hardware
                .as_mut()
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            let index = entries
                .iter()
                .position(|entry| entry.id == *hardware_id)
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            entries.remove(index);
        }
        MechanicalSettingsPatch::SetHardwareMount {
            hardware_id,
            part_id,
            feature_id,
        } => {
            if part_id.is_empty() || feature_id.is_empty() {
                return Err("Link the hardware to a generated mount feature.".into());
            }
            let entry = hardware_mut(configuration, hardware_id)?;
            entry.part_id = part_id.clone();
            entry.feature_id = feature_id.clone();
        }
        MechanicalSettingsPatch::SetHardwareText {
            hardware_id,
            field,
            value,
        } => {
            let entry = hardware_mut(configuration, hardware_id)?;
            match field {
                super::mechanical_settings::MechanicalHardwareTextField::Designation => {
                    entry.designation = value.clone()
                }
                super::mechanical_settings::MechanicalHardwareTextField::Thread => {
                    entry.thread = value.clone()
                }
                super::mechanical_settings::MechanicalHardwareTextField::Tolerance => {
                    entry.tolerance = Some(value.clone())
                }
                super::mechanical_settings::MechanicalHardwareTextField::Notes => {
                    entry.notes = Some(value.clone())
                }
            }
        }
        MechanicalSettingsPatch::SetHardwareDimension {
            hardware_id,
            field,
            value,
        } => {
            let entry = hardware_mut(configuration, hardware_id)?;
            match field {
                MechanicalDimension::HardwareLength => {
                    if !value.is_finite() || *value < 0.1 {
                        return Err("Hardware length must be at least 0.1 mm.".into());
                    }
                    entry.length = *value;
                }
                MechanicalDimension::HardwareQuantity => {
                    if !value.is_finite()
                        || *value < 1.0
                        || value.fract() != 0.0
                        || *value > f64::from(u32::MAX)
                    {
                        return Err("Hardware quantity must be a positive whole number.".into());
                    }
                    entry.quantity = *value as u32;
                }
                _ => return Err("The selected hardware dimension is unavailable.".into()),
            }
        }
        MechanicalSettingsPatch::SetOpenings(openings) => {
            validate_openings(openings)?;
            configuration.openings = Some(openings.clone());
        }
        MechanicalSettingsPatch::SetOpeningDimension {
            opening_index,
            point_index,
            field,
            value,
        } => {
            let is_volume_field = matches!(
                *field,
                MechanicalDimension::OpeningBottomZ | MechanicalDimension::OpeningHeight
            );
            let is_point_field = matches!(
                *field,
                MechanicalDimension::OpeningPointX | MechanicalDimension::OpeningPointY
            );
            if !(is_volume_field && point_index.is_none()
                || is_point_field && point_index.is_some())
            {
                return Err("The selected opening field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            let openings = configuration
                .openings
                .as_mut()
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            let opening = openings
                .get_mut(*opening_index)
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            if let Some(point_index) = point_index {
                let point = opening.points.get_mut(*point_index).ok_or_else(|| {
                    "The selected opening vertex is no longer available.".to_owned()
                })?;
                match field {
                    MechanicalDimension::OpeningPointX => point.x = *value,
                    MechanicalDimension::OpeningPointY => point.y = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
            } else {
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = *value,
                    MechanicalDimension::OpeningHeight => opening.height = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
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
        MechanicalSettingsPatch::SetMountCollection { collection, mounts } => {
            validate_mounts(mounts)?;
            match collection {
                MechanicalMountCollection::Suspension => {
                    configuration.mounts = mounts.clone();
                }
                MechanicalMountCollection::Closure => {
                    if configuration.internal_gasket.is_some() {
                        let existing = configuration
                            .closure_mounts
                            .iter()
                            .flatten()
                            .map(|mount| mount.id.as_str())
                            .collect::<std::collections::HashSet<_>>();
                        if mounts
                            .iter()
                            .any(|mount| !existing.contains(mount.id.as_str()))
                        {
                            return Err(
                                "Adding closure screws is unavailable with an internal gasket."
                                    .into(),
                            );
                        }
                    }
                    configuration.closure_mounts = Some(mounts.clone());
                }
            }
        }
        MechanicalSettingsPatch::AdoptClosurePositions(mounts) => {
            if configuration.internal_gasket.is_none() {
                return Err("Closure positions can only be adopted for an internal gasket.".into());
            }
            validate_mounts(mounts)?;
            configuration.closure_mounts = Some(mounts.clone());
        }
        MechanicalSettingsPatch::SetMountDimension {
            collection,
            mount_id,
            field,
            value,
        } => {
            validate_dimension(*field, *value)?;
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            match field {
                MechanicalDimension::MountPositionX => mount.at.x = *value,
                MechanicalDimension::MountPositionY => mount.at.y = *value,
                MechanicalDimension::MountHoleDiameter => mount.hole_diameter = *value,
                MechanicalDimension::MountBossDiameter => mount.boss_diameter = Some(*value),
                MechanicalDimension::MountBossHeight if mount.kind == MountKind::Boss => {
                    mount.height = Some(*value)
                }
                _ => return Err("The selected mount dimension is unavailable.".into()),
            }
        }
        MechanicalSettingsPatch::SetMountPosition {
            collection,
            mount_id,
            at,
        } => {
            if mount_id.is_empty() || !at.x.is_finite() || !at.y.is_finite() {
                return Err("Mount positions must be finite values.".into());
            }
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            mount.at = *at;
        }
        MechanicalSettingsPatch::SetMountKind {
            collection,
            mount_id,
            kind,
        } => {
            let mounts = mount_collection_mut(configuration, *collection);
            let mount = mounts
                .iter_mut()
                .find(|mount| mount.id == *mount_id)
                .ok_or_else(|| "The selected mount is no longer available.".to_owned())?;
            mount.kind = kind.clone();
        }
        MechanicalSettingsPatch::RemoveMount {
            collection,
            mount_id,
        } => {
            if mount_id.is_empty() {
                return Err("The selected mount is no longer available.".into());
            }
            if *collection == MechanicalMountCollection::Suspension
                && configuration.mount == MechanicalMount::Gasket
            {
                return Err("Suspension mounts are unavailable for a gasket mount.".into());
            }
            let mounts = mount_collection_mut(configuration, *collection);
            let Some(index) = mounts.iter().position(|mount| mount.id == *mount_id) else {
                return Err("The selected mount is no longer available.".into());
            };
            mounts.remove(index);
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
            if matches!(
                *field,
                MechanicalDimension::OpeningBottomZ
                    | MechanicalDimension::OpeningHeight
                    | MechanicalDimension::OpeningPointX
                    | MechanicalDimension::OpeningPointY
            ) {
                return Err(
                    "Opening dimensions require a current access opening selection.".into(),
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
        MechanicalSettingsPatch::SetGasketSupportPlacement {
            support_id,
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
            if anchors.iter().any(|anchor| {
                anchor.id.is_empty()
                    || !ids.insert(&anchor.id)
                    || !anchor.anchor.is_finite()
                    || !(0.0..=1.0).contains(&anchor.anchor)
            }) {
                return Err("The gasket support placement is invalid.".into());
            }
            if configuration.mount != MechanicalMount::Gasket {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            let layout = configuration
                .gasket_layout
                .get_or_insert_with(default_gasket_layout);
            for source in anchors {
                let mut anchor = source.clone();
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
        MechanicalSettingsPatch::ResetGasketPlacement => {
            if configuration.mount != MechanicalMount::Gasket
                || configuration.internal_gasket.is_none()
            {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            configuration
                .gasket_layout
                .get_or_insert_with(default_gasket_layout)
                .supports
                .clear();
        }
        MechanicalSettingsPatch::AssignSwitchProfile { .. }
        | MechanicalSettingsPatch::AssignStabilizerProfile { .. }
        | MechanicalSettingsPatch::AssignImportedGeometryProfile { .. } => {
            return Err(
                "Profile assignment must be prepared from the current accepted Case scope.".into(),
            );
        }
        MechanicalSettingsPatch::UpdateProfile(next) => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let Some(profile) = configuration
                .profiles
                .iter_mut()
                .find(|profile| profile.definition_id == next.definition_id)
            else {
                return Err("The selected mechanical profile is no longer assigned.".into());
            };
            let extraction_changed = profile.source_geometry != next.source_geometry;
            if extraction_changed {
                let definition = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == next.definition_id)
                    .ok_or_else(|| {
                        "The selected mechanical profile is no longer available.".to_owned()
                    })?;
                if definition.kicad_source.as_ref().is_none_or(|source| {
                    next.source_geometry
                        .as_ref()
                        .is_none_or(|geometry| geometry.text != source.source)
                }) {
                    return Err(
                        "Extracted geometry must come from the assigned KiCad source.".into(),
                    );
                }
            } else if profile.pcb_holes != next.pcb_holes {
                return Err("PCB mounting holes can only be changed by extracting the assigned KiCad geometry.".into());
            }
            if profile.switch_family != next.switch_family
                || profile.supported_thickness != next.supported_thickness
                || profile.plate_to_pcb != next.plate_to_pcb
                || profile.source != next.source
            {
                return Err(
                    "Only extracted geometry for the assigned KiCad profile can be updated here."
                        .into(),
                );
            }
            if !valid_profile_polygons(&next.cutouts)
                || next
                    .clearances
                    .as_deref()
                    .is_some_and(|polygons| !valid_profile_polygons(polygons))
                || next.pcb_holes.iter().flatten().any(|hole| {
                    !hole.at.x.is_finite()
                        || !hole.at.y.is_finite()
                        || !hole.diameter.is_finite()
                        || hole.diameter <= 0.0
                })
            {
                return Err(
                    "Profile polygons and mounting holes must contain valid finite coordinates."
                        .into(),
                );
            }
            validate_profile_openings(
                next.openings.as_deref().unwrap_or_default(),
                "Access opening",
            )?;
            validate_profile_openings(
                next.clearance_volumes.as_deref().unwrap_or_default(),
                "Clearance volume",
            )?;
            *profile = next.clone();
        }
        MechanicalSettingsPatch::RemoveProfile { definition_id } => {
            if configuration.board_id != board_id {
                return Err("The mechanical configuration belongs to another board.".into());
            }
            let previous_len = configuration.profiles.len();
            configuration
                .profiles
                .retain(|profile| profile.definition_id != *definition_id);
            if configuration.profiles.len() == previous_len {
                return Err("The selected mechanical profile is no longer assigned.".into());
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
        MechanicalSettingsPatch::AddProcessOverride { part_id } => {
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            if configuration
                .part_processes
                .iter()
                .flatten()
                .any(|process| process.part_id == *part_id)
            {
                return Err("A process override already exists for this part.".into());
            }
            let thickness = process_default_thickness(configuration, part_id);
            let method = if part_id.ends_with("foam") {
                PlateMethod::CutSheet
            } else {
                configuration.method.clone()
            };
            configuration
                .part_processes
                .get_or_insert_with(Vec::new)
                .push(default_process(part_id, method, thickness));
        }
        MechanicalSettingsPatch::RemoveProcessOverride { part_id } => {
            let processes = configuration.part_processes.as_mut().ok_or_else(|| {
                "The selected process override is no longer available.".to_owned()
            })?;
            let previous_len = processes.len();
            processes.retain(|process| process.part_id != *part_id);
            if processes.len() == previous_len {
                return Err("The selected process override is no longer available.".into());
            }
        }
        MechanicalSettingsPatch::SetProcessMethod { part_id, method } => {
            if !is_process_target(document, part_id) || part_id.ends_with("foam") {
                return Err("The selected process method is not available.".into());
            }
            if part_id == "plate" {
                configuration.method = method.clone();
            } else {
                let process = process_mut(configuration, document, part_id)?;
                let previous_method = process.method.clone();
                process.method = method.clone();
                if previous_method != *method
                    && !material_options(part_id, method).contains(&process.material.as_str())
                {
                    process.material = default_material(part_id, method).into();
                }
            }
        }
        MechanicalSettingsPatch::SetProcessMaterial { part_id, material } => {
            let process = process_mut(configuration, document, part_id)?;
            if !material_options(part_id, &process.method).contains(&material.as_str()) {
                return Err("The selected material is not available for this process.".into());
            }
            process.material = material.clone();
        }
        MechanicalSettingsPatch::SetProcessThickness { part_id, thickness } => {
            let min = if part_id.ends_with("foam") { 0.0 } else { 0.1 };
            if !thickness.is_finite() || *thickness < min {
                return Err(if min == 0.0 {
                    "Foam thickness must be finite and nonnegative.".into()
                } else {
                    "Finished thickness must be at least 0.1 mm.".into()
                });
            }
            if !is_process_target(document, part_id) {
                return Err("The selected Case part is no longer available.".into());
            }
            match part_id.as_str() {
                "plate" => configuration.plate_thickness = *thickness,
                "plate-foam" => configuration.plate_foam_thickness = *thickness,
                "bottom-foam" => configuration.bottom_foam_thickness = *thickness,
                "bottom" => configuration.bottom_thickness = *thickness,
                _ => process_mut(configuration, document, part_id)?.thickness = *thickness,
            }
        }
        MechanicalSettingsPatch::SetStabilizerKind(stabilizer) => {
            if !stabilizer_target_exists(document, board_id, &stabilizer.part_id) {
                return Err("The selected wide key no longer needs a stabilizer.".into());
            }
            let values = configuration.stabilizers.get_or_insert_with(Vec::new);
            if let Some(existing) = values
                .iter_mut()
                .find(|existing| existing.part_id == stabilizer.part_id)
            {
                *existing = stabilizer.clone();
            } else {
                values.push(stabilizer.clone());
            }
        }
    }

    let family =
        profile_family(configuration).or_else(|| initial_switch_family(document, board_id));
    let new_plate_thickness = match patch {
        MechanicalSettingsPatch::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value,
        } => Some(*value),
        MechanicalSettingsPatch::SetProcessThickness { part_id, thickness }
            if part_id == "plate" =>
        {
            Some(*thickness)
        }
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
    let skip_process_normalization = matches!(
        patch,
        MechanicalSettingsPatch::SetBatteryEnabled(_)
            | MechanicalSettingsPatch::SetClosureInsertPreset(_)
            | MechanicalSettingsPatch::SetClosureDrive(_)
            | MechanicalSettingsPatch::SetClosureInstallation(_)
            | MechanicalSettingsPatch::SetClosureFixedLength(_)
            | MechanicalSettingsPatch::SetClosureThread(_)
            | MechanicalSettingsPatch::SetClosureScrewLengths(_)
            | MechanicalSettingsPatch::SetClosureHeadProfile(_)
            | MechanicalSettingsPatch::SetClosureLengthDatum(_)
            | MechanicalSettingsPatch::AddCriticalFit { .. }
            | MechanicalSettingsPatch::RemoveCriticalFit { .. }
            | MechanicalSettingsPatch::SetCriticalFitText { .. }
            | MechanicalSettingsPatch::SetCriticalFitPart { .. }
            | MechanicalSettingsPatch::SetCriticalFitPoint { .. }
            | MechanicalSettingsPatch::AddHardware { .. }
            | MechanicalSettingsPatch::RemoveHardware { .. }
            | MechanicalSettingsPatch::SetHardwareMount { .. }
            | MechanicalSettingsPatch::SetHardwareText { .. }
            | MechanicalSettingsPatch::SetHardwareDimension { .. }
            | MechanicalSettingsPatch::SetOpenings(_)
            | MechanicalSettingsPatch::SetOpeningDimension { .. }
            | MechanicalSettingsPatch::SetStabilizerKind(_)
    ) || matches!(
        patch,
        MechanicalSettingsPatch::SetMountPosition { .. }
            | MechanicalSettingsPatch::SetGasketSupportPlacement { .. }
            | MechanicalSettingsPatch::ResetGasketPlacement
    ) || matches!(
        patch,
        MechanicalSettingsPatch::SetDimension {
            field: MechanicalDimension::BatteryWidth
                | MechanicalDimension::OpeningBottomZ
                | MechanicalDimension::OpeningHeight
                | MechanicalDimension::OpeningPointX
                | MechanicalDimension::OpeningPointY
                | MechanicalDimension::BatteryDepth
                | MechanicalDimension::BatteryHeight
                | MechanicalDimension::BatteryCableWidth
                | MechanicalDimension::BatteryPositionX
                | MechanicalDimension::BatteryPositionY
                | MechanicalDimension::BatteryCableExitX
                | MechanicalDimension::BatteryCableExitY
                | MechanicalDimension::ClosureThreadDiameter
                | MechanicalDimension::ClosurePitch
                | MechanicalDimension::ClosureHeadDiameter
                | MechanicalDimension::ClosureHeadHeight
                | MechanicalDimension::ClosureHoleDiameter
                | MechanicalDimension::ClosureInsertDiameter
                | MechanicalDimension::ClosureInsertLength
                | MechanicalDimension::ClosureSeatDiameter
                | MechanicalDimension::ClosureSeatDepth
                | MechanicalDimension::ClosureEngagement
                | MechanicalDimension::ClosureThreadStart
                | MechanicalDimension::ClosureTipAllowance
                | MechanicalDimension::ClosureBottomingClearance
                | MechanicalDimension::ClosureRoof
                | MechanicalDimension::ClosureSurround
                | MechanicalDimension::ClosureSeatLeadDepth
                | MechanicalDimension::ClosureSeatLeadDiameter
                | MechanicalDimension::ClosureBearingThickness,
            ..
        }
    );
    if !skip_process_normalization {
        normalize_processes(configuration, patch);
    }
    Ok(())
}

fn mount_collection_mut(
    configuration: &mut MechanicalConfiguration,
    collection: MechanicalMountCollection,
) -> &mut Vec<Mount> {
    match collection {
        MechanicalMountCollection::Suspension => &mut configuration.mounts,
        MechanicalMountCollection::Closure => {
            configuration.closure_mounts.get_or_insert_with(Vec::new)
        }
    }
}

fn validate_mounts(mounts: &[Mount]) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for mount in mounts {
        if mount.id.is_empty() || !ids.insert(mount.id.as_str()) {
            return Err("Mount IDs must be non-empty and unique.".into());
        }
        if !mount.at.x.is_finite() || !mount.at.y.is_finite() {
            return Err("Mount positions must be finite values.".into());
        }
        validate_mount_positive(mount.hole_diameter, "Hole diameter")?;
        validate_mount_positive(mount.boss_diameter.unwrap_or(5.0), "Boss diameter")?;
        if mount.kind == MountKind::Boss {
            validate_mount_positive(mount.height.unwrap_or(5.0), "Boss height")?;
        }
    }
    Ok(())
}

fn valid_profile_polygons(polygons: &[Vec<Vec2>]) -> bool {
    polygons.iter().all(|polygon| {
        polygon.len() >= 3
            && polygon
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
    })
}

fn validate_profile_openings(openings: &[CaseOpening], label: &str) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        if opening.points.len() < 3
            || opening.points.iter().any(|point| {
                !point.x.is_finite()
                    || !point.y.is_finite()
                    || point.x < -1_000_000.0
                    || point.y < -1_000_000.0
            })
        {
            return Err(format!(
                "{label} {} needs at least three vertices with finite coordinates no smaller than −1,000,000 mm.",
                index + 1
            ));
        }
        if !opening.z.is_finite() || opening.z < -1_000_000.0 {
            return Err(format!(
                "{label} {} bottom Z must be a finite coordinate no smaller than −1,000,000 mm.",
                index + 1
            ));
        }
        if !opening.height.is_finite() || opening.height < 0.1 {
            return Err(format!(
                "{label} {} height must be at least 0.1 mm.",
                index + 1
            ));
        }
    }
    Ok(())
}

fn validate_openings(openings: &[CaseOpening]) -> Result<(), String> {
    validate_profile_openings(openings, "Access opening")
}

fn validate_mount_positive(value: f64, label: &str) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{label} must be a finite value greater than zero."))
    }
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
        MechanicalDimension::MountPositionX
        | MechanicalDimension::MountPositionY
        | MechanicalDimension::MountHoleDiameter
        | MechanicalDimension::MountBossDiameter
        | MechanicalDimension::MountBossHeight => {
            return Err("Mount dimensions require a current mount selection.".into());
        }
        MechanicalDimension::GasketSupportLength | MechanicalDimension::GasketSupportWidth => {
            return Err("Gasket support dimensions require a current support selection.".into());
        }
        MechanicalDimension::OpeningBottomZ
        | MechanicalDimension::OpeningHeight
        | MechanicalDimension::OpeningPointX
        | MechanicalDimension::OpeningPointY => {
            return Err("Opening dimensions require a current access opening selection.".into());
        }
        MechanicalDimension::ClosureThreadDiameter
        | MechanicalDimension::ClosurePitch
        | MechanicalDimension::ClosureHeadDiameter
        | MechanicalDimension::ClosureHeadHeight
        | MechanicalDimension::ClosureHoleDiameter
        | MechanicalDimension::ClosureInsertDiameter
        | MechanicalDimension::ClosureInsertLength
        | MechanicalDimension::ClosureSeatDiameter
        | MechanicalDimension::ClosureSeatDepth
        | MechanicalDimension::ClosureEngagement
        | MechanicalDimension::ClosureThreadStart
        | MechanicalDimension::ClosureTipAllowance
        | MechanicalDimension::ClosureBottomingClearance
        | MechanicalDimension::ClosureRoof
        | MechanicalDimension::ClosureSurround
        | MechanicalDimension::ClosureSeatLeadDepth
        | MechanicalDimension::ClosureSeatLeadDiameter
        | MechanicalDimension::ClosureBearingThickness => {
            let hardware = closure_hardware_mut(configuration)?;
            match field {
                MechanicalDimension::ClosureThreadDiameter => hardware.thread_diameter = value,
                MechanicalDimension::ClosurePitch => hardware.pitch = value,
                MechanicalDimension::ClosureHeadDiameter => hardware.head_diameter = value,
                MechanicalDimension::ClosureHeadHeight => hardware.head_height = value,
                MechanicalDimension::ClosureHoleDiameter => hardware.hole_diameter = value,
                MechanicalDimension::ClosureInsertDiameter => hardware.insert_diameter = value,
                MechanicalDimension::ClosureInsertLength => hardware.insert_length = value,
                MechanicalDimension::ClosureSeatDiameter => hardware.seat_diameter = value,
                MechanicalDimension::ClosureSeatDepth => hardware.seat_depth = value,
                MechanicalDimension::ClosureEngagement => hardware.engagement = value,
                MechanicalDimension::ClosureThreadStart => hardware.thread_start = value,
                MechanicalDimension::ClosureTipAllowance => hardware.tip_allowance = value,
                MechanicalDimension::ClosureBottomingClearance => {
                    hardware.bottoming_clearance = value
                }
                MechanicalDimension::ClosureRoof => hardware.roof = value,
                MechanicalDimension::ClosureSurround => hardware.surround = value,
                MechanicalDimension::ClosureSeatLeadDepth => hardware.seat_lead_depth = value,
                MechanicalDimension::ClosureSeatLeadDiameter => hardware.seat_lead_diameter = value,
                MechanicalDimension::ClosureBearingThickness => hardware.bearing_thickness = value,
                _ => unreachable!("closure dimension variant checked above"),
            }
        }
        MechanicalDimension::CriticalFitFromX
        | MechanicalDimension::CriticalFitFromY
        | MechanicalDimension::CriticalFitToX
        | MechanicalDimension::CriticalFitToY => {
            return Err("Critical-fit endpoint dimensions require a current fit selection.".into());
        }
        MechanicalDimension::HardwareLength | MechanicalDimension::HardwareQuantity => {
            return Err("Hardware dimensions require a current hardware selection.".into());
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
            | MechanicalDimension::BatteryCableWidth
            | MechanicalDimension::HardwareLength => value >= 0.1,
            MechanicalDimension::HardwareQuantity => {
                value >= 1.0 && value.fract() == 0.0 && value <= f64::from(u32::MAX)
            }
            MechanicalDimension::OpeningHeight => value >= 0.1,
            MechanicalDimension::GasketSupportLength => value >= 5.0,
            MechanicalDimension::GasketSupportWidth => value >= 0.5,
            MechanicalDimension::OpeningBottomZ
            | MechanicalDimension::OpeningPointX
            | MechanicalDimension::OpeningPointY
            | MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
            | MechanicalDimension::MountPositionX
            | MechanicalDimension::MountPositionY
            | MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY => value >= -1_000_000.0,
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
    } else if field == MechanicalDimension::HardwareLength {
        Err("Hardware length must be at least 0.1 mm.".into())
    } else if field == MechanicalDimension::HardwareQuantity {
        Err("Hardware quantity must be a positive whole number.".into())
    } else if field == MechanicalDimension::OpeningHeight {
        Err("Access opening height must be at least 0.1 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::BatteryPositionX
            | MechanicalDimension::BatteryPositionY
            | MechanicalDimension::BatteryCableExitX
            | MechanicalDimension::BatteryCableExitY
    ) {
        Err("Battery positions and cable exits must be at least −1,000,000 mm.".into())
    } else if matches!(
        field,
        MechanicalDimension::CriticalFitFromX
            | MechanicalDimension::CriticalFitFromY
            | MechanicalDimension::CriticalFitToX
            | MechanicalDimension::CriticalFitToY
    ) {
        Err("Critical-fit coordinates must be at least −1,000,000 mm.".into())
    } else {
        Err("Mechanical dimensions must be finite and nonnegative.".into())
    }
}

fn normalize_processes(
    configuration: &mut MechanicalConfiguration,
    patch: &MechanicalSettingsPatch,
) {
    let processes = configuration.part_processes.get_or_insert_with(Vec::new);
    let top_level_method_change = matches!(patch, MechanicalSettingsPatch::SetMethod(_))
        || matches!(
            patch,
            MechanicalSettingsPatch::SetProcessMethod { part_id, .. } if part_id == "plate"
        );
    for process in processes.iter_mut() {
        let is_standard = matches!(
            process.part_id.as_str(),
            "plate" | "plate-foam" | "bottom-foam" | "bottom"
        );
        let is_foam = process.part_id.ends_with("foam");
        let method = if is_foam {
            PlateMethod::CutSheet
        } else if process.part_id == "plate" || (top_level_method_change && is_standard) {
            configuration.method.clone()
        } else {
            process.method.clone()
        };
        let method_changed =
            method != process.method || (top_level_method_change && is_standard && !is_foam);
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

fn is_process_target(document: &ProjectDoc, part_id: &str) -> bool {
    matches!(part_id, "plate" | "plate-foam" | "bottom-foam" | "bottom")
        || document.parts.iter().any(|part| part.id == part_id)
}

fn process_default_thickness(configuration: &MechanicalConfiguration, part_id: &str) -> f64 {
    match part_id {
        "plate" => configuration.plate_thickness,
        "plate-foam" => configuration.plate_foam_thickness,
        "bottom-foam" => configuration.bottom_foam_thickness,
        "bottom" => configuration.bottom_thickness,
        _ => configuration.plate_thickness,
    }
}

fn default_process(part_id: &str, method: PlateMethod, thickness: f64) -> MechanicalPartProcess {
    let method = if part_id.ends_with("foam") {
        PlateMethod::CutSheet
    } else {
        method
    };
    MechanicalPartProcess {
        part_id: part_id.to_owned(),
        material: default_material(part_id, &method).to_owned(),
        method,
        thickness,
        constraints_version: "2026-09-24".into(),
    }
}

fn process_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    document: &ProjectDoc,
    part_id: &str,
) -> Result<&'a mut MechanicalPartProcess, String> {
    if !is_process_target(document, part_id) {
        return Err("The selected Case part is no longer available.".into());
    }
    configuration
        .part_processes
        .as_mut()
        .and_then(|processes| {
            processes
                .iter_mut()
                .find(|process| process.part_id == part_id)
        })
        .ok_or_else(|| "The selected process override is no longer available.".to_owned())
}

fn stabilizer_target_exists(document: &ProjectDoc, board_id: &str, part_id: &str) -> bool {
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return false;
    };
    let Some(part) = document
        .parts
        .iter()
        .find(|part| part.id == part_id && board.part_ids.contains(&part.id))
    else {
        return false;
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return false;
    };
    let Some(size) = part.keycap.or(definition.keycap) else {
        return false;
    };
    if size.x.max(size.y) < 37.0
        || definition
            .generator
            .as_ref()
            .is_none_or(|generator| generator.source.to_lowercase() != "ceoloide/switch_mx")
    {
        return false;
    }
    !document.parts.iter().any(|stabilizer| {
        board.part_ids.contains(&stabilizer.id)
            && document
                .definitions
                .iter()
                .find(|candidate| candidate.id == stabilizer.definition_id)
                .and_then(|candidate| candidate.kicad_source.as_ref())
                .is_some_and(|source| source.source.contains("(footprint \"STAB_MX_"))
            && (stabilizer.pose.at.x - part.pose.at.x).hypot(stabilizer.pose.at.y - part.pose.at.y)
                < 0.01
    })
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

fn family_assignment_changes(
    existing_family: Option<MechanicalSwitchFamily>,
    selected_family: MechanicalSwitchFamily,
) -> bool {
    existing_family != Some(selected_family)
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
    patch.field_id()
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

fn closure_hardware_mut(
    configuration: &mut MechanicalConfiguration,
) -> Result<&mut InternalClosureHardware, String> {
    configuration
        .internal_gasket
        .as_mut()
        .map(|settings| &mut settings.hardware)
        .ok_or_else(|| "Internal gasket closure hardware is no longer available.".into())
}

fn critical_fit_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    fit_id: &str,
) -> Result<&'a mut MechanicalCriticalFit, String> {
    configuration
        .critical_fits
        .as_mut()
        .and_then(|fits| fits.iter_mut().find(|fit| fit.id == fit_id))
        .ok_or_else(|| "The selected critical fit is no longer available.".into())
}

fn hardware_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    hardware_id: &str,
) -> Result<&'a mut MechanicalHardwareSpecification, String> {
    configuration
        .hardware
        .as_mut()
        .and_then(|entries| entries.iter_mut().find(|entry| entry.id == hardware_id))
        .ok_or_else(|| "The selected hardware entry is no longer available.".into())
}

fn resize_closure_insert(hardware: &mut InternalClosureHardware, id: &str) -> Result<(), String> {
    let (thread_diameter, pitch, insert_length, insert_diameter) = match id {
        "m2-3" => (2.0, 0.4, 3.0, 3.2),
        "m2-4" => (2.0, 0.4, 4.0, 3.2),
        "m2.5-3" => (2.5, 0.45, 3.0, 3.5),
        "m2.5-4" => (2.5, 0.45, 4.0, 3.5),
        "m2.5-5" => (2.5, 0.45, 5.0, 3.5),
        "m3-3" => (3.0, 0.5, 3.0, 4.2),
        _ => return Err("The selected insert preset is unavailable.".into()),
    };
    hardware.id = format!("custom-{id}");
    hardware.thread = format!("M{thread_diameter} × {pitch}");
    hardware.thread_diameter = thread_diameter;
    hardware.pitch = pitch;
    hardware.insert_length = insert_length;
    hardware.insert_diameter = insert_diameter;
    hardware.seat_diameter = insert_diameter - 0.4;
    hardware.seat_lead_diameter = insert_diameter - 0.2;
    hardware.seat_depth = insert_length + hardware.bottoming_clearance;
    hardware.engagement = (insert_length - 0.5).min(insert_length - hardware.thread_start);
    hardware.hole_diameter = thread_diameter + 0.2;
    hardware.head_diameter = thread_diameter * 2.0;
    hardware.head_height = thread_diameter / 2.0;
    Ok(())
}

#[cfg(test)]
mod battery_patch_tests {
    use super::*;
    use boardstudio_core::model::{Board, Part, PartDefinition, Pose2, Side};
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
    async fn mechanical_fields_queue_without_losing_settings_or_undo_steps() {
        use crate::runtime::project_name_test_support as support;
        use boardstudio_application::Event;
        let runtime = support::new_runtime();
        let mut document = ProjectDoc::empty("mechanical-queue", "Mechanical");
        document.boards.push(serde_json::from_value(serde_json::json!({
            "id": "board", "name": "Board", "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6, "traces": [], "vias": []
        })).unwrap());
        let mut config = configuration();
        config.closure_mounts = Some(vec![]);
        document.mechanical = Some(config);
        support::open_document(&runtime, document).await;
        let current_runtime = runtime.clone();
        let current = Rc::new(move || {
            let model = current_runtime.model();
            let accepted = model.accepted?;
            let scope = current_runtime.scope()?;
            Some(MechanicalSettingsCurrent {
                identity: MechanicalSettingsIdentity {
                    editor_instance_id: 1,
                    scope_generation: 1,
                    presentation_generation: 1,
                    scope,
                    snapshot_token: accepted.token,
                    revision: accepted.document.revision,
                    active_board_id: "board".into(),
                    configuration_board_id: "board".into(),
                },
                configuration: accepted.document.mechanical.clone().map(Rc::new),
                accepted,
                editable: true,
                lifecycle: model.lifecycle,
                durability: model.durability,
            })
        });
        let template = Rc::new(
            serde_json::from_value::<PartDefinition>(serde_json::json!({
                "id": "hole", "name": "Hole", "kind": "utility", "courtyard": [], "pads": []
            }))
            .unwrap(),
        );
        let submit_runtime = runtime.clone();
        let controller = MechanicalSettingsController::new(MechanicalSettingsPorts {
            current: current.clone(),
            load_mounting_hole: Rc::new(move || {
                let template = template.clone();
                Box::pin(async move { Ok(template) })
            }),
            begin_edit: Rc::new(move |resolver| {
                EditTicket::begin(
                    &submit_runtime,
                    "mechanical-settings",
                    Some("mechanical settings".into()),
                    resolver,
                )
            }),
            publish: Rc::new(|_| {}),
        });
        let request = |request_id, field, value| {
            let patch = MechanicalSettingsPatch::SetDimension { field, value };
            MechanicalSettingsRequest {
                identity: current().unwrap().identity,
                request_id,
                field_id: patch_field_id(&patch),
                patch,
            }
        };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        assert!(controller.submit(request(1, MechanicalDimension::WallThickness, 3.0)));
        gloo_timers::future::TimeoutFuture::new(10).await;
        support::drive_pending(&runtime);
        entered.await.unwrap();
        let second_admitted = controller.submit(request(2, MechanicalDimension::Clearance, 0.8));
        gloo_timers::future::TimeoutFuture::new(10).await;
        support::drive_pending(&runtime);
        release.send(()).unwrap();
        for _ in 0..20 {
            support::run_pending(&runtime).await;
            controller.settle();
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            second_admitted,
            "field edits must queue while a prior field is pending"
        );
        let accepted = runtime.model().accepted.unwrap();
        let mechanical = accepted.document.mechanical.as_ref().unwrap();
        assert_eq!(mechanical.wall_thickness, 3.0);
        assert_eq!(mechanical.clearance, 0.8);
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        let accepted = runtime.model().accepted.unwrap();
        let mechanical = accepted.document.mechanical.as_ref().unwrap();
        assert_eq!(mechanical.wall_thickness, 3.0);
        assert_eq!(mechanical.clearance, 0.3);
        runtime.submit(Event::Undo {
            operation_id: runtime.operation(),
        });
        support::run_pending(&runtime).await;
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            2.0
        );
    }

    #[wasm_bindgen_test]
    fn manufacturing_process_edits_route_standard_thickness_and_preserve_other_targets() {
        let document = ProjectDoc::empty("doc", "doc");
        let mut configuration = configuration();
        configuration.part_processes = Some(vec![
            MechanicalPartProcess {
                part_id: "plate".into(),
                method: PlateMethod::Printed,
                material: "ABS".into(),
                thickness: 1.5,
                constraints_version: "2026-09-24".into(),
            },
            MechanicalPartProcess {
                part_id: "bottom".into(),
                method: PlateMethod::Printed,
                material: "PLA".into(),
                thickness: 3.0,
                constraints_version: "2026-09-24".into(),
            },
        ]);

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::SetProcessThickness {
                part_id: "plate".into(),
                thickness: 2.2,
            },
            &document,
            "board",
        )
        .unwrap();
        assert_eq!(configuration.plate_thickness, 2.2);
        assert_eq!(
            configuration.part_processes.as_ref().unwrap()[0].thickness,
            2.2
        );
        assert_eq!(
            configuration.part_processes.as_ref().unwrap()[1].thickness,
            3.0
        );

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::SetProcessMethod {
                part_id: "plate".into(),
                method: PlateMethod::Cnc,
            },
            &document,
            "board",
        )
        .unwrap();
        let processes = configuration.part_processes.as_ref().unwrap();
        assert_eq!(configuration.method, PlateMethod::Cnc);
        assert_eq!(processes[0].material, "Aluminium");
        assert_eq!(processes[1].material, "Aluminium");
        assert_eq!(processes[0].constraints_version, "2026-09-24");

        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::RemoveProcessOverride {
                part_id: "plate".into(),
            },
            &document,
            "board",
        )
        .unwrap();
        let remaining = configuration.part_processes.as_ref().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].part_id, "bottom");
    }

    #[wasm_bindgen_test]
    fn plate_process_thickness_matches_dimension_profile_and_default_foam_sync() {
        let document = ProjectDoc::empty("doc", "doc");
        let mut base = configuration();
        base.plate_foam_thickness = default_plate_foam_thickness(3.5);
        base.profiles.push(MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: Some(MechanicalSwitchFamily::Mx),
            definition_id: "mx-switch".into(),
            source: "fixture MX profile".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        });
        base.part_processes = Some(vec![MechanicalPartProcess {
            part_id: "plate".into(),
            method: PlateMethod::Printed,
            material: "PLA".into(),
            thickness: 1.5,
            constraints_version: "2026-09-24".into(),
        }]);

        let mut process_route = base.clone();
        let mut dimension_route = base;
        apply_patch(
            &mut process_route,
            &MechanicalSettingsPatch::SetProcessThickness {
                part_id: "plate".into(),
                thickness: 2.5,
            },
            &document,
            "board",
        )
        .unwrap();
        apply_patch(
            &mut dimension_route,
            &MechanicalSettingsPatch::SetDimension {
                field: MechanicalDimension::PlateThickness,
                value: 2.5,
            },
            &document,
            "board",
        )
        .unwrap();

        assert_eq!(process_route.plate_to_pcb, dimension_route.plate_to_pcb);
        assert_eq!(process_route.profiles, dimension_route.profiles);
        assert_eq!(
            process_route.plate_foam_thickness,
            dimension_route.plate_foam_thickness
        );
        assert_eq!(process_route.part_processes, dimension_route.part_processes);
        assert_eq!(process_route.plate_to_pcb, 2.5);
        assert_eq!(process_route.plate_foam_thickness, 2.3);
    }

    #[wasm_bindgen_test]
    fn profile_assignment_rejects_a_different_board_and_duplicate_target() {
        let document = ProjectDoc::empty("doc", "doc");
        let mut other_board_configuration = configuration();
        other_board_configuration.board_id = "other-board".into();
        assert!(
            validate_profile_target(
                &document,
                "board",
                &other_board_configuration,
                "switch",
                ProfileTargetKind::Switch,
            )
            .unwrap_err()
            .contains("another board")
        );

        let mut duplicate_configuration = configuration();
        duplicate_configuration
            .profiles
            .push(MechanicalPartProfile {
                source_geometry: None,
                pcb_holes: None,
                clearance_volumes: None,
                openings: None,
                clearances: None,
                supported_thickness: None,
                switch_family: Some(MechanicalSwitchFamily::Mx),
                definition_id: "switch".into(),
                source: "fixture".into(),
                cutouts: Vec::new(),
                plate_to_pcb: 3.5,
            });
        assert!(
            validate_profile_target(
                &document,
                "board",
                &duplicate_configuration,
                "switch",
                ProfileTargetKind::Switch,
            )
            .unwrap_err()
            .contains("already assigned")
        );
    }

    fn placed_custom_stabilizer_document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "doc");
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["stabilizer-part".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "stabilizer-part".into(),
            definition_id: "stab-mx-2u".into(),
            reference: "STAB1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "stab-mx-2u".into(),
            name: "STAB_MX_2u".into(),
            kind: PartKind::Custom,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document
    }

    #[wasm_bindgen_test]
    fn stabilizer_profile_accepts_a_placed_custom_definition() {
        let document = placed_custom_stabilizer_document();
        assert!(
            validate_profile_target(
                &document,
                "board",
                &configuration(),
                "stab-mx-2u",
                ProfileTargetKind::Stabilizer,
            )
            .is_ok()
        );
    }

    #[wasm_bindgen_test]
    fn assigned_custom_profile_updates_geometry_and_removes_by_current_definition() {
        use boardstudio_core::model::{KicadSource, MechanicalProfileSource};

        let mut document = placed_custom_stabilizer_document();
        document.definitions[0].kicad_source = Some(KicadSource {
            format_version: 1,
            source: "(footprint \"custom\")".into(),
        });
        let mut configuration = configuration();
        configuration.profiles.push(MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: None,
            definition_id: "stab-mx-2u".into(),
            source: "KiCad STAB_MX_2u".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        });
        let unrelated_profile = MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: Some(MechanicalSwitchFamily::Mx),
            definition_id: "library-switch".into(),
            source: "Library MX fit".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        };
        configuration.profiles.push(unrelated_profile.clone());
        let mut updated = configuration.profiles[0].clone();
        updated.cutouts = vec![vec![
            Vec2 { x: -9.4, y: 9.5 },
            Vec2 { x: 9.5, y: 9.5 },
            Vec2 { x: 9.5, y: -9.5 },
        ]];
        updated.source_geometry = Some(MechanicalProfileSource {
            text: "(footprint \"custom\")".into(),
            sha256: "sha".into(),
            mappings: vec![],
            source_ids: vec!["geometry-70".into()],
        });
        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::UpdateProfile(updated.clone()),
            &document,
            "board",
        )
        .unwrap();
        assert_eq!(configuration.profiles[0], updated);
        let mut edited_without_extraction = unrelated_profile.clone();
        edited_without_extraction.cutouts = vec![vec![
            Vec2 { x: -2.0, y: 2.0 },
            Vec2 { x: 2.0, y: 2.0 },
            Vec2 { x: 2.0, y: -2.0 },
        ]];
        edited_without_extraction.clearances = Some(vec![vec![
            Vec2 { x: -3.0, y: 3.0 },
            Vec2 { x: 3.0, y: 3.0 },
            Vec2 { x: 3.0, y: -3.0 },
        ]]);
        edited_without_extraction.openings = Some(vec![CaseOpening {
            points: vec![
                Vec2 { x: -1.0, y: 1.0 },
                Vec2 { x: 1.0, y: 1.0 },
                Vec2 { x: 1.0, y: -1.0 },
            ],
            z: 0.5,
            height: 5.0,
        }]);
        edited_without_extraction.clearance_volumes = Some(vec![CaseOpening {
            points: vec![
                Vec2 { x: -4.0, y: 4.0 },
                Vec2 { x: 4.0, y: 4.0 },
                Vec2 { x: 4.0, y: -4.0 },
            ],
            z: -2.0,
            height: 12.0,
        }]);
        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::UpdateProfile(edited_without_extraction.clone()),
            &document,
            "board",
        )
        .unwrap();
        assert_eq!(configuration.profiles[1], edited_without_extraction);
        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::RemoveProfile {
                definition_id: "stab-mx-2u".into(),
            },
            &document,
            "board",
        )
        .unwrap();
        assert_eq!(
            configuration.profiles,
            vec![edited_without_extraction.clone()]
        );
        apply_patch(
            &mut configuration,
            &MechanicalSettingsPatch::RemoveProfile {
                definition_id: "library-switch".into(),
            },
            &document,
            "board",
        )
        .unwrap();
        assert!(configuration.profiles.is_empty());
        assert!(
            apply_patch(
                &mut configuration,
                &MechanicalSettingsPatch::RemoveProfile {
                    definition_id: "stab-mx-2u".into(),
                },
                &document,
                "board",
            )
            .unwrap_err()
            .contains("no longer assigned")
        );
    }

    #[wasm_bindgen_test]
    fn first_switch_profile_family_assignment_is_treated_as_a_family_change() {
        assert!(family_assignment_changes(
            None,
            MechanicalSwitchFamily::ChocV1
        ));
        assert!(!family_assignment_changes(
            Some(MechanicalSwitchFamily::ChocV1),
            MechanicalSwitchFamily::ChocV1,
        ));
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
