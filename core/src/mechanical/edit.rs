//! Applies one typed mechanical edit against the accepted document.
//!
//! Defaults, validation, linked-feature propagation and closure-clearance
//! projection live here, behind the mechanical module's seam. The Case adapter
//! submits a typed [`MechanicalEdit`] and the operation reports only the durable
//! ids whose values or memberships actually changed.
//!
//! One accepted edit owns its whole result: the effective configuration change,
//! absent closure-mount initialization and the generated closure-clearance
//! projection. Every mutation is staged on a clone, so a failure (including a
//! missing required mounting-hole template) leaves the accepted document untouched,
//! and the complete result is one Undo step. A change equal to the accepted value
//! that needs no initialization is a no-op that reports nothing.
use crate::model::*;

use super::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension, MechanicalEdit,
    MechanicalHardwareTextField, MechanicalMountCollection, MechanicalTarget,
};
use super::inputs::{effective_document, effective_inputs};

const GENERATED_PART_PREFIX: &str = "case-closure/";
const GENERATED_DEFINITION_PREFIX: &str = "assembly-closure/definition/";
const AUTO_CLOSURE_PREFIX: &str = "auto-closure/";
const MISSING_TEMPLATE: &str =
    "A normalized mounting-hole definition is required to project the generated closure clearance.";

/// Applies one mechanical edit, returning the changed durable ids.
///
/// The configuration owner (the canonical board id or the physical instance id)
/// is always the first reported id; profile, hardware, critical-fit, opening,
/// mount and process ids follow when the ported code changes them, then the
/// generated clearance part ids, definition ids and membership owners the
/// projection actually changed.
pub fn apply(doc: &mut ProjectDoc, edit: &MechanicalEdit) -> Result<Vec<String>, String> {
    // Stage the whole edit on a clone so any later failure leaves the accepted
    // document - configuration, initialization and projection - untouched.
    let mut next = doc.clone();
    let changed = apply_staged(&mut next, edit)?;
    *doc = next;
    Ok(changed)
}

fn apply_staged(doc: &mut ProjectDoc, edit: &MechanicalEdit) -> Result<Vec<String>, String> {
    let board_id = target_board_id(doc, &edit.target)?;
    match &edit.change {
        MechanicalChange::Enable => enable(doc, &edit.target, &board_id, edit),
        MechanicalChange::Disable => disable(doc, &edit.target, &board_id, edit),
        MechanicalChange::InitializeClosures => {
            initialize_closures(doc, &edit.target, &board_id, edit)
        }
        change => apply_settings(doc, edit, change, &board_id),
    }
}

/// Whether a mechanical change can create, move or remove generated clearance
/// geometry.
///
/// Projection rebuilds the generated closure-hole parts, definitions and
/// memberships. Whenever it is required the caller must supply the normalized
/// mounting-hole template, because the projected holes and their definitions are
/// derived from it. Enable/Disable and closure initialization change the closure
/// mount set itself; openings, mounts and mount collections, closure mounts, the
/// clearance and the mount dimensions that decide a generated hole diameter,
/// gasket support placement/reset and profile changes can all move the holes the
/// projection derives. Pure settings edits (battery, closure hardware, critical
/// fits, hardware text/quantity, process overrides, method/style switches, mount
/// type) leave every generated hole untouched.
pub fn requires_projection(change: &MechanicalChange) -> bool {
    match change {
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures => true,
        MechanicalChange::AddOpening
        | MechanicalChange::RemoveOpening { .. }
        | MechanicalChange::AddOpeningVertex { .. }
        | MechanicalChange::RemoveOpeningVertex { .. }
        | MechanicalChange::SetOpeningDimension { .. }
        | MechanicalChange::AddMount { .. }
        | MechanicalChange::AddSuggestedClosureMounts(_)
        | MechanicalChange::SetMountCollection { .. }
        | MechanicalChange::AdoptClosurePositions(_)
        | MechanicalChange::SetMountDimension { .. }
        | MechanicalChange::SetMountPosition { .. }
        | MechanicalChange::SetMountKind { .. }
        | MechanicalChange::RemoveMount { .. }
        | MechanicalChange::SetGasketSupportPlacement { .. }
        | MechanicalChange::ResetGasketPlacement
        | MechanicalChange::AssignSwitchProfile { .. }
        | MechanicalChange::AssignStabilizerProfile { .. }
        | MechanicalChange::AssignImportedGeometryProfile { .. }
        | MechanicalChange::UpdateProfile(_)
        | MechanicalChange::RemoveProfile { .. }
        | MechanicalChange::SetSwitchFamily { .. } => true,
        MechanicalChange::SetDimension { field, .. } => {
            // Only the clearance changes a generated closure-hole diameter.
            matches!(field, MechanicalDimension::Clearance)
        }
        _ => false,
    }
}

/// Whether a mechanical change can alter the inputs the outline composer reads.
///
/// Pure settings edits leave every outline input untouched, so the committed
/// outline cache is reused. Changes that can create, move or remove generated
/// parts/definitions or closure geometry - enable/disable, closure
/// initialization, openings, mounts and mount collections, closure clearance and
/// the dimensions that change generated holes, gasket support placement/reset and
/// profiles - feed the outline composer and must recompute. This is exactly the
/// projection classification: those are the changes that can alter the generated
/// clearances the composer reads.
pub fn affects_outline(change: &MechanicalChange) -> bool {
    requires_projection(change)
}

fn apply_settings(
    doc: &mut ProjectDoc,
    edit: &MechanicalEdit,
    change: &MechanicalChange,
    board_id: &str,
) -> Result<Vec<String>, String> {
    let inputs = effective_inputs(doc, &edit.target)?;
    let mut configuration = inputs.configuration;
    let before = configuration.clone();
    let mut changed = Vec::new();
    apply_change(&mut configuration, change, doc, board_id, &mut changed)?;
    // Closure mounts are initialized whenever the resolved stack still has none
    // and the mount is not a gasket; explicit `Some(vec![])` is deliberate.
    let needs_initialization =
        configuration.closure_mounts.is_none() && configuration.mount != MechanicalMount::Gasket;
    if configuration == before && !needs_initialization {
        return Ok(Vec::new());
    }
    let projection_required = requires_projection(change) || needs_initialization;
    if projection_required && edit.mounting_hole.is_none() {
        return Err(MISSING_TEMPLATE.into());
    }
    changed.insert(0, owner_id(&edit.target, board_id));
    if needs_initialization {
        persist(doc, &edit.target, configuration)?;
        configuration = initialize_closure_mounts(doc, &edit.target)?;
    }
    persist(doc, &edit.target, configuration)?;
    if projection_required {
        project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    }
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Runs the owned projection and records only the generated ids it changed.
fn project(
    doc: &mut ProjectDoc,
    mounting_hole: Option<&PartDefinition>,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let template = mounting_hole.ok_or_else(|| MISSING_TEMPLATE.to_string())?;
    let before = doc.clone();
    let projected =
        crate::mechanical::clearance::project_owned_closure_clearance(before.clone(), template)?;
    changed.extend(generated_change_ids(&before, &projected));
    *doc = projected;
    Ok(())
}

/// Resolves the assembly from the proposed document and materializes its
/// suggested closure mounts onto the proposed configuration.
///
/// The caller must already have persisted the proposed configuration into the
/// target slot so [`effective_document`] resolves the same stack the user is
/// configuring. The result is the resolved effective configuration, matching the
/// adapter's `prepare_document`: initialization adopts the effective value the
/// resolver returned.
fn initialize_closure_mounts(
    doc: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<MechanicalConfiguration, String> {
    let (effective, contours) = effective_document(doc, target)?;
    let mut configuration = effective
        .mechanical
        .clone()
        .ok_or("The mechanical configuration no longer exists.")?;
    let assembly = crate::mechanical::resolve(&effective, &contours);
    let height = configuration.plate_to_pcb
        + configuration.pcb_thickness
        + configuration
            .bottom_foam_thickness
            .max(configuration.battery_height);
    configuration.closure_mounts = Some(
        assembly
            .suggested_mounts
            .into_iter()
            .map(|mut mount| {
                mount.id = format!("{AUTO_CLOSURE_PREFIX}{}", mount.id);
                mount.kind = MountKind::Boss;
                mount.hole_diameter = 2.2;
                mount.height = Some(height);
                mount
            })
            .collect(),
    );
    Ok(configuration)
}

/// The generated part ids, generated definition ids and membership owners that
/// differ between two documents.
fn generated_change_ids(before: &ProjectDoc, after: &ProjectDoc) -> Vec<String> {
    let mut ids = std::collections::BTreeSet::new();
    for part in &after.parts {
        if part.id.starts_with(GENERATED_PART_PREFIX)
            && before.parts.iter().find(|entry| entry.id == part.id) != Some(part)
        {
            ids.insert(part.id.clone());
        }
    }
    for part in &before.parts {
        if part.id.starts_with(GENERATED_PART_PREFIX)
            && !after.parts.iter().any(|entry| entry.id == part.id)
        {
            ids.insert(part.id.clone());
        }
    }
    for definition in &after.definitions {
        if definition.id.starts_with(GENERATED_DEFINITION_PREFIX)
            && before
                .definitions
                .iter()
                .find(|entry| entry.id == definition.id)
                != Some(definition)
        {
            ids.insert(definition.id.clone());
        }
    }
    for definition in &before.definitions {
        if definition.id.starts_with(GENERATED_DEFINITION_PREFIX)
            && !after
                .definitions
                .iter()
                .any(|entry| entry.id == definition.id)
        {
            ids.insert(definition.id.clone());
        }
    }
    let generated_set = |part_ids: Option<&Vec<String>>| {
        part_ids
            .into_iter()
            .flatten()
            .filter(|id| id.starts_with(GENERATED_PART_PREFIX))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
    };
    for board in &after.boards {
        let before_ids = before
            .boards
            .iter()
            .find(|entry| entry.id == board.id)
            .map(|entry| &entry.part_ids);
        if generated_set(before_ids) != generated_set(Some(&board.part_ids)) {
            ids.insert(board.id.clone());
        }
    }
    for layout in &after.layouts {
        let before_ids = before
            .layouts
            .iter()
            .find(|entry| entry.id == layout.id)
            .map(|entry| &entry.part_ids);
        if generated_set(before_ids) != generated_set(Some(&layout.part_ids)) {
            ids.insert(layout.id.clone());
        }
    }
    ids.into_iter().collect()
}

fn dedup_changed(changed: &mut Vec<String>) {
    let mut seen = std::collections::BTreeSet::new();
    changed.retain(|id| seen.insert(id.clone()));
}

/// Resolves an edit target to its board id and optional physical instance.
pub(crate) fn resolve_target<'a>(
    doc: &'a ProjectDoc,
    target: &MechanicalTarget,
) -> Result<(String, Option<&'a PhysicalBoardInstance>), String> {
    match target {
        MechanicalTarget::Board { board_id } => Ok((board_id.clone(), None)),
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_ref()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            Ok((instance.board_id.clone(), Some(instance)))
        }
    }
}

fn target_board_id(doc: &ProjectDoc, target: &MechanicalTarget) -> Result<String, String> {
    resolve_target(doc, target).map(|(board_id, _)| board_id)
}

fn owner_id(target: &MechanicalTarget, board_id: &str) -> String {
    match target {
        MechanicalTarget::Board { .. } => board_id.to_string(),
        MechanicalTarget::Instance { instance_id } => instance_id.clone(),
    }
}

/// Writes the effective configuration back to the target's stored slot.
///
/// Canonical edits write `doc.mechanical`; instance edits write that instance's
/// `mechanical`. The stored value is the effective configuration the patch
/// produced, matching the adapter's `prepare_document` + `persist_configuration`
/// sequence.
fn persist(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    configuration: MechanicalConfiguration,
) -> Result<(), String> {
    match target {
        MechanicalTarget::Board { board_id } => {
            if !doc.boards.iter().any(|board| board.id == *board_id) {
                return Err("Board is unavailable".into());
            }
            doc.mechanical = Some(configuration);
            Ok(())
        }
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_mut()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            instance.mechanical = Some(configuration);
            Ok(())
        }
    }
}

/// Enables a target that has no effective configuration yet.
///
/// The fresh configuration is closure-initialized and projected like any other
/// change, because its `closure_mounts` starts absent.
fn enable(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    // An already-effective target is unchanged; the edit reports nothing.
    if effective_inputs(doc, target).is_ok() {
        return Ok(Vec::new());
    }
    if edit.mounting_hole.is_none() {
        return Err(MISSING_TEMPLATE.into());
    }
    let configuration = default_configuration(doc, board_id)?;
    let mut changed = vec![owner_id(target, board_id)];
    persist(doc, target, configuration)?;
    let configuration = initialize_closure_mounts(doc, target)?;
    persist(doc, target, configuration)?;
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Clears the target's stored configuration; absent configuration is a no-op.
///
/// Disabling can remove generated closure geometry, so it is projection-required
/// and must remove the stale generated clearance parts in the same edit.
fn disable(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let present = match target {
        MechanicalTarget::Board { .. } => doc
            .mechanical
            .as_ref()
            .is_some_and(|configuration| configuration.board_id == *board_id),
        MechanicalTarget::Instance { instance_id } => doc
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .instances
                    .iter()
                    .find(|instance| instance.id == *instance_id)
            })
            .is_some_and(|instance| instance.mechanical.is_some()),
    };
    if !present {
        return Ok(Vec::new());
    }
    if edit.mounting_hole.is_none() {
        return Err(MISSING_TEMPLATE.into());
    }
    match target {
        MechanicalTarget::Board { .. } => doc.mechanical = None,
        MechanicalTarget::Instance { instance_id } => {
            let hardware = doc
                .hardware
                .as_mut()
                .ok_or("Physical instances are unavailable")?;
            let instance = hardware
                .instances
                .iter_mut()
                .find(|instance| instance.id == *instance_id)
                .ok_or("The selected physical instance is unavailable")?;
            instance.mechanical = None;
        }
    }
    let mut changed = vec![owner_id(target, board_id)];
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Initializes absent closure mounts from the resolved assembly and projects them.
///
/// An already-present collection or a gasket mount is not eligible and reports the
/// adapter's message; explicit `Some(vec![])` therefore stays deliberate.
fn initialize_closures(
    doc: &mut ProjectDoc,
    target: &MechanicalTarget,
    board_id: &str,
    edit: &MechanicalEdit,
) -> Result<Vec<String>, String> {
    let inputs = effective_inputs(doc, target)?;
    if inputs.configuration.closure_mounts.is_some()
        || inputs.configuration.mount == MechanicalMount::Gasket
    {
        return Err("The current stack no longer needs mounting-location initialization.".into());
    }
    if edit.mounting_hole.is_none() {
        return Err(MISSING_TEMPLATE.into());
    }
    persist(doc, target, inputs.configuration)?;
    let configuration = initialize_closure_mounts(doc, target)?;
    persist(doc, target, configuration)?;
    let mut changed = vec![owner_id(target, board_id)];
    project(doc, edit.mounting_hole.as_deref(), &mut changed)?;
    dedup_changed(&mut changed);
    Ok(changed)
}

/// Reference defaults for a board that has no stored configuration.
///
/// Ports the adapter's `case_settings::initial_settings` family selection, plate
/// and foam derivation, board thickness and standard process overrides onto the
/// Core-owned default helpers.
fn default_configuration(
    document: &ProjectDoc,
    board_id: &str,
) -> Result<MechanicalConfiguration, String> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or("Board is unavailable")?;
    let family = initial_switch_family(document, board_id).unwrap_or(MechanicalSwitchFamily::Mx);
    let plate_thickness = crate::mechanical::default_plate_thickness(family);
    let plate_to_pcb = crate::mechanical::mounting_datum(family) - plate_thickness;
    let plate_foam_thickness = crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
    let pcb_thickness = if board.thickness.is_finite() && board.thickness > 0.0 {
        board.thickness
    } else {
        1.6
    };
    let bottom_foam_thickness = 2.0;
    let bottom_thickness = 3.0;
    let method = PlateMethod::Printed;
    let part_processes = vec![
        crate::mechanical::default_layer_process("plate", method.clone(), plate_thickness),
        crate::mechanical::default_layer_process(
            "plate-foam",
            PlateMethod::CutSheet,
            plate_foam_thickness,
        ),
        crate::mechanical::default_layer_process(
            "bottom-foam",
            PlateMethod::CutSheet,
            bottom_foam_thickness,
        ),
        crate::mechanical::default_layer_process("bottom", method.clone(), bottom_thickness),
    ];
    Ok(MechanicalConfiguration {
        internal_gasket: None,
        gasket_layout: None,
        hardware: None,
        critical_fits: None,
        bottom_style: Some(MechanicalBottomStyle::Shell),
        middle_frame: Some(false),
        gasket_travel: None,
        openings: None,
        opening_allowance: None,
        stabilizers: None,
        part_processes: Some(part_processes),
        gasket: None,
        closure_mounts: None,
        board_id: board_id.to_string(),
        integrated_plate_frame: false,
        battery: None,
        mounts: Vec::new(),
        method,
        mount: MechanicalMount::Tray,
        plate_thickness,
        plate_foam_thickness,
        pcb_thickness,
        bottom_foam_thickness,
        battery_height: 0.0,
        bottom_thickness,
        plate_to_pcb,
        wall_thickness: 2.0,
        clearance: 0.3,
        profiles: Vec::new(),
    })
}

fn apply_change(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    board_id: &str,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let previous = configuration.clone();
    match change {
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures => {
            return Err(
                "Configure, Disable and closure initialization are handled by the operation."
                    .into(),
            );
        }
        MechanicalChange::AssignSwitchProfile {
            definition_id,
            family,
        } => {
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Switch,
            )?;
            let existing_family = profile_family(configuration)
                .or_else(|| initial_switch_family(document, &configuration.board_id));
            let family_changed = family_assignment_changes(existing_family, *family);
            let plate_thickness = if family_changed {
                crate::mechanical::default_plate_thickness(*family)
            } else {
                configuration.plate_thickness
            };
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            let source = match family {
                MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
                MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
                MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
            };
            let mut profile =
                crate::mechanical::builtin_profile(definition_id.clone(), source, plate_to_pcb)?;
            if profile.definition_id != *definition_id {
                return Err("Core returned a switch profile for another part type.".into());
            }
            profile.switch_family = Some(*family);
            profile.plate_to_pcb = plate_to_pcb;
            if family_changed {
                configuration.plate_thickness = plate_thickness;
                configuration.plate_to_pcb = plate_to_pcb;
                configuration.plate_foam_thickness =
                    crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            }
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignStabilizerProfile {
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
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::Stabilizer,
            )?;
            let mut profile = crate::mechanical::builtin_profile(
                definition_id.clone(),
                source.clone(),
                configuration.plate_to_pcb,
            )?;
            if profile.definition_id != *definition_id || profile.switch_family.is_some() {
                return Err("Core returned a different stabilizer profile.".into());
            }
            profile.plate_to_pcb = configuration.plate_to_pcb;
            configuration.profiles.push(profile);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AssignImportedGeometryProfile { definition_id } => {
            validate_profile_target(
                document,
                board_id,
                configuration,
                definition_id,
                ProfileTargetKind::ImportedGeometry,
            )?;
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == *definition_id)
                .ok_or_else(|| "The imported part definition is no longer available.".to_owned())?;
            let family = document
                .parts
                .iter()
                .find(|part| {
                    part.definition_id == *definition_id
                        && document
                            .boards
                            .iter()
                            .any(|board| board.id == *board_id && board.part_ids.contains(&part.id))
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
                    crate::mechanical::mounting_datum(family) - configuration.plate_thickness
                }),
            });
            changed.push(definition_id.clone());
        }
        MechanicalChange::SetBatteryEnabled(enabled) => {
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
        MechanicalChange::SetClosureInsertPreset(id) => {
            resize_closure_insert(closure_hardware_mut(configuration)?, id)?;
        }
        MechanicalChange::SetClosureDrive(drive) => {
            closure_hardware_mut(configuration)?.drive = drive.clone();
        }
        MechanicalChange::SetClosureInstallation(installation) => {
            closure_hardware_mut(configuration)?.installation = installation.clone();
        }
        MechanicalChange::SetClosureFixedLength(length) => {
            let hardware = closure_hardware_mut(configuration)?;
            if let Some(length) = length
                && (!length.is_finite()
                    || *length <= 0.0
                    || !hardware.screw_lengths.contains(length))
            {
                return Err(
                    "Choose an available positive screw length or automatic length.".into(),
                );
            }
            hardware.fixed_length = *length;
        }
        MechanicalChange::SetClosureThread(thread) => {
            closure_hardware_mut(configuration)?.thread = thread.clone();
        }
        MechanicalChange::SetClosureScrewLengths(lengths) => {
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
        MechanicalChange::SetClosureHeadProfile(profile) => {
            closure_hardware_mut(configuration)?.head_profile = profile.clone();
        }
        MechanicalChange::SetClosureLengthDatum(datum) => {
            closure_hardware_mut(configuration)?.length_datum = datum.clone();
        }
        MechanicalChange::AddCriticalFit { part_id } => {
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
                id: fit_id.clone(),
                part_id: part_id.clone(),
                label: "Critical dimension".into(),
                from: Vec2 { x: 0.0, y: 0.0 },
                to: Vec2 { x: 10.0, y: 0.0 },
                tolerance: "±0.2 mm".into(),
            });
            changed.push(fit_id);
        }
        MechanicalChange::RemoveCriticalFit { fit_id } => {
            let fits = configuration
                .critical_fits
                .as_mut()
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            let index = fits
                .iter()
                .position(|fit| fit.id == *fit_id)
                .ok_or_else(|| "The selected critical fit is no longer available.".to_owned())?;
            fits.remove(index);
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitText {
            fit_id,
            field,
            value,
        } => {
            let fit = critical_fit_mut(configuration, fit_id)?;
            match field {
                MechanicalCriticalFitTextField::Label => fit.label = value.clone(),
                MechanicalCriticalFitTextField::Tolerance => fit.tolerance = value.clone(),
            }
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitPart { fit_id, part_id } => {
            if part_id.is_empty() {
                return Err("A generated part must be selected for a critical fit.".into());
            }
            critical_fit_mut(configuration, fit_id)?.part_id = part_id.clone();
            changed.push(fit_id.clone());
        }
        MechanicalChange::SetCriticalFitPoint {
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
            changed.push(fit_id.clone());
        }
        MechanicalChange::AddHardware {
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
                id: hardware_id.clone(),
                part_id: part_id.clone(),
                feature_id: feature_id.clone(),
                designation: "Socket screw".into(),
                thread: "M2 × 0.4".into(),
                length: 6.0,
                quantity: 4,
                notes: Some(String::new()),
                tolerance: None,
            });
            changed.push(hardware_id);
        }
        MechanicalChange::RemoveHardware { hardware_id } => {
            let entries = configuration
                .hardware
                .as_mut()
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            let index = entries
                .iter()
                .position(|entry| entry.id == *hardware_id)
                .ok_or_else(|| "The selected hardware entry is no longer available.".to_owned())?;
            entries.remove(index);
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareMount {
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
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareText {
            hardware_id,
            field,
            value,
        } => {
            let entry = hardware_mut(configuration, hardware_id)?;
            match field {
                MechanicalHardwareTextField::Designation => entry.designation = value.clone(),
                MechanicalHardwareTextField::Thread => entry.thread = value.clone(),
                MechanicalHardwareTextField::Tolerance => entry.tolerance = Some(value.clone()),
                MechanicalHardwareTextField::Notes => entry.notes = Some(value.clone()),
            }
            changed.push(hardware_id.clone());
        }
        MechanicalChange::SetHardwareDimension {
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
            changed.push(hardware_id.clone());
        }
        MechanicalChange::AddOpening => {
            let openings = configuration.openings.get_or_insert_with(Vec::new);
            push_mechanical_opening(
                openings,
                vec![
                    Vec2 { x: -2.5, y: -2.5 },
                    Vec2 { x: 2.5, y: -2.5 },
                    Vec2 { x: 2.5, y: 2.5 },
                    Vec2 { x: -2.5, y: 2.5 },
                ],
                0.0,
                10.0,
            );
            if let Some(opening) = openings.last() {
                changed.push(opening.id.clone());
            }
        }
        MechanicalChange::RemoveOpening { opening_index } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            if *opening_index >= openings.len() {
                return Err("The selected access opening is no longer available.".into());
            }
            let id = openings[*opening_index].id.clone();
            openings.remove(*opening_index);
            changed.push(id);
        }
        MechanicalChange::AddOpeningVertex { opening_index } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .get_mut(*opening_index)
                .ok_or("The selected access opening is no longer available.")?;
            let id = next_vertex_id(opening);
            opening.vertices.push(MechanicalOpeningVertex {
                id,
                point: Vec2 { x: 0.0, y: 0.0 },
            });
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
        }
        MechanicalChange::RemoveOpeningVertex {
            opening_index,
            point_index,
        } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .get_mut(*opening_index)
                .ok_or("The selected access opening is no longer available.")?;
            if opening.vertices.len() <= 3 {
                return Err("An opening footprint needs at least three vertices.".into());
            }
            if *point_index >= opening.vertices.len() {
                return Err("The selected opening vertex is no longer available.".into());
            }
            opening.vertices.remove(*point_index);
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
        }
        MechanicalChange::SetOpeningDimension {
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
                let point = opening.vertices.get_mut(*point_index).ok_or_else(|| {
                    "The selected opening vertex is no longer available.".to_owned()
                })?;
                match field {
                    MechanicalDimension::OpeningPointX => point.point.x = *value,
                    MechanicalDimension::OpeningPointY => point.point.y = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
            } else {
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = *value,
                    MechanicalDimension::OpeningHeight => opening.height = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
            }
            changed.push(opening.id.clone());
        }
        MechanicalChange::SetMethod(method) => configuration.method = method.clone(),
        MechanicalChange::SetMount(mount) => {
            configuration.mount = mount.clone();
            if *mount == MechanicalMount::Gasket {
                if let Some(closures) = &mut configuration.closure_mounts {
                    closures.retain(|entry| !entry.id.starts_with("auto-closure/"));
                }
                configuration
                    .gasket_layout
                    .get_or_insert_with(crate::mechanical::default_gasket_layout);
                if configuration.internal_gasket.is_none() {
                    configuration.gasket_travel = Some(0.1);
                }
                configuration
                    .internal_gasket
                    .get_or_insert_with(crate::mechanical::default_internal_gasket);
                configuration.integrated_plate_frame = false;
                configuration.bottom_style = Some(MechanicalBottomStyle::Shell);
                configuration.middle_frame = Some(false);
            }
        }
        MechanicalChange::AddMount { collection } => {
            if *collection == MechanicalMountCollection::Closure
                && configuration.internal_gasket.is_some()
            {
                return Err("Adding closure screws is unavailable with an internal gasket.".into());
            }
            let mut ordinal = 1usize;
            let id = loop {
                let candidate = format!("case-mechanical-mount-{ordinal}");
                let used = configuration
                    .mounts
                    .iter()
                    .chain(configuration.closure_mounts.iter().flatten())
                    .any(|mount| mount.id == candidate)
                    || document.parts.iter().any(|part| part.id == candidate)
                    || document
                        .case_bodies
                        .iter()
                        .flat_map(|body| body.mounts.iter().flatten())
                        .any(|mount| mount.id == candidate);
                if !used {
                    break candidate;
                }
                ordinal += 1;
            };
            mount_collection_mut(configuration, *collection).push(Mount {
                id: id.clone(),
                at: Vec2 { x: 0.0, y: 0.0 },
                kind: MountKind::Hole,
                hole_diameter: 2.5,
                boss_diameter: Some(5.0),
                height: Some(5.0),
            });
            changed.push(id);
        }
        MechanicalChange::AddSuggestedClosureMounts(suggestions) => {
            if configuration.internal_gasket.is_some() {
                return Err("Adding closure screws is unavailable with an internal gasket.".into());
            }
            let height = configuration.plate_to_pcb
                + configuration.pcb_thickness
                + configuration
                    .bottom_foam_thickness
                    .max(configuration.battery_height);
            let mounts = configuration.closure_mounts.get_or_insert_with(Vec::new);
            for suggestion in suggestions {
                let id = format!("auto-closure/{}", suggestion.id);
                if mounts.iter().any(|mount| mount.id == id) {
                    continue;
                }
                let mut mount = suggestion.clone();
                mount.id = id.clone();
                mount.kind = MountKind::Boss;
                mount.hole_diameter = 2.2;
                mount.height = Some(height);
                mounts.push(mount);
                changed.push(id);
            }
            validate_mounts(mounts)?;
        }
        MechanicalChange::SetMountCollection { collection, mounts } => {
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
        MechanicalChange::AdoptClosurePositions(mounts) => {
            if configuration.internal_gasket.is_none() {
                return Err("Closure positions can only be adopted for an internal gasket.".into());
            }
            validate_mounts(mounts)?;
            configuration.closure_mounts = Some(mounts.clone());
        }
        MechanicalChange::SetMountDimension {
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
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetMountPosition {
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
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetMountKind {
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
            changed.push(mount_id.clone());
        }
        MechanicalChange::RemoveMount {
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
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetBottomStyle(style) => {
            configuration.bottom_style = Some(style.clone());
        }
        MechanicalChange::SetMiddleFrame(enabled) => {
            configuration.middle_frame = Some(*enabled);
        }
        MechanicalChange::SetIntegratedPlateFrame(enabled) => {
            configuration.integrated_plate_frame = *enabled;
        }
        MechanicalChange::SetDimension { field, value } => {
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
        MechanicalChange::SetGasketSupportDimension {
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
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
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
            changed.push(support_id.clone());
        }
        MechanicalChange::SetGasketSupportPlacement {
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
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
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
            changed.push(support_id.clone());
        }
        MechanicalChange::SetGasketSupportUnlinked {
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
                .get_or_insert_with(crate::mechanical::default_gasket_layout);
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
            changed.push(support_id.clone());
        }
        MechanicalChange::ResetGasketPlacement => {
            if configuration.mount != MechanicalMount::Gasket
                || configuration.internal_gasket.is_none()
            {
                return Err("The current configuration no longer contains gasket supports.".into());
            }
            configuration
                .gasket_layout
                .get_or_insert_with(crate::mechanical::default_gasket_layout)
                .supports
                .clear();
        }
        MechanicalChange::UpdateProfile(next) => {
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
            let definition_id = next.definition_id.clone();
            *profile = next.clone();
            changed.push(definition_id);
        }
        MechanicalChange::RemoveProfile { definition_id } => {
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
            changed.push(definition_id.clone());
        }
        MechanicalChange::SetSwitchFamily {
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
            let plate_thickness = crate::mechanical::default_plate_thickness(*family);
            let plate_to_pcb = crate::mechanical::mounting_datum(*family) - plate_thickness;
            for profile in &mut configuration.profiles {
                if profile.definition_id == *definition_id {
                    profile.switch_family = Some(*family);
                    profile.plate_to_pcb = plate_to_pcb;
                } else if let Some(other_family) = profile.switch_family {
                    profile.plate_to_pcb =
                        crate::mechanical::mounting_datum(other_family) - plate_thickness;
                }
            }
            configuration.plate_thickness = plate_thickness;
            configuration.plate_to_pcb = plate_to_pcb;
            configuration.plate_foam_thickness =
                crate::mechanical::default_plate_foam_thickness(plate_to_pcb);
            changed.push(definition_id.clone());
        }
        MechanicalChange::AddProcessOverride { part_id } => {
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
                .push(crate::mechanical::default_layer_process(
                    part_id, method, thickness,
                ));
            changed.push(part_id.clone());
        }
        MechanicalChange::RemoveProcessOverride { part_id } => {
            let processes = configuration.part_processes.as_mut().ok_or_else(|| {
                "The selected process override is no longer available.".to_owned()
            })?;
            let previous_len = processes.len();
            processes.retain(|process| process.part_id != *part_id);
            if processes.len() == previous_len {
                return Err("The selected process override is no longer available.".into());
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMethod { part_id, method } => {
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
                    process.material = crate::mechanical::default_material(part_id, method).into();
                }
            }
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessMaterial { part_id, material } => {
            let process = process_mut(configuration, document, part_id)?;
            if !material_options(part_id, &process.method).contains(&material.as_str()) {
                return Err("The selected material is not available for this process.".into());
            }
            process.material = material.clone();
            changed.push(part_id.clone());
        }
        MechanicalChange::SetProcessThickness { part_id, thickness } => {
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
            changed.push(part_id.clone());
        }
        MechanicalChange::SetStabilizerKind(stabilizer) => {
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
            changed.push(stabilizer.part_id.clone());
        }
    }

    let family =
        profile_family(configuration).or_else(|| initial_switch_family(document, board_id));
    let new_plate_thickness = match change {
        MechanicalChange::SetDimension {
            field: MechanicalDimension::PlateThickness,
            value,
        } => Some(*value),
        MechanicalChange::SetProcessThickness { part_id, thickness } if part_id == "plate" => {
            Some(*thickness)
        }
        MechanicalChange::SetSwitchFamily { .. } => Some(configuration.plate_thickness),
        _ => None,
    };
    if let Some(new_plate_thickness) = new_plate_thickness
        && let Some(family) = family
    {
        let previous_gap = crate::mechanical::mounting_datum(family) - previous.plate_thickness;
        configuration.plate_to_pcb =
            crate::mechanical::mounting_datum(family) - new_plate_thickness;
        for profile in &mut configuration.profiles {
            if profile.switch_family == Some(family) {
                profile.plate_to_pcb = configuration.plate_to_pcb;
            }
        }
        if (previous.plate_foam_thickness
            - crate::mechanical::default_plate_foam_thickness(previous_gap))
        .abs()
            < 0.001
        {
            configuration.plate_foam_thickness =
                crate::mechanical::default_plate_foam_thickness(configuration.plate_to_pcb);
        }
    }
    // A change that leaves the effective configuration untouched is a no-op, so the
    // process normalization side effect must not materialize an empty override list.
    if *configuration == previous {
        return Ok(());
    }
    if !skip_process_normalization(change) {
        normalize_processes(configuration, change);
    }
    Ok(())
}

fn skip_process_normalization(change: &MechanicalChange) -> bool {
    matches!(
        change,
        MechanicalChange::SetBatteryEnabled(_)
            | MechanicalChange::SetClosureInsertPreset(_)
            | MechanicalChange::SetClosureDrive(_)
            | MechanicalChange::SetClosureInstallation(_)
            | MechanicalChange::SetClosureFixedLength(_)
            | MechanicalChange::SetClosureThread(_)
            | MechanicalChange::SetClosureScrewLengths(_)
            | MechanicalChange::SetClosureHeadProfile(_)
            | MechanicalChange::SetClosureLengthDatum(_)
            | MechanicalChange::AddCriticalFit { .. }
            | MechanicalChange::RemoveCriticalFit { .. }
            | MechanicalChange::SetCriticalFitText { .. }
            | MechanicalChange::SetCriticalFitPart { .. }
            | MechanicalChange::SetCriticalFitPoint { .. }
            | MechanicalChange::AddHardware { .. }
            | MechanicalChange::RemoveHardware { .. }
            | MechanicalChange::SetHardwareMount { .. }
            | MechanicalChange::SetHardwareText { .. }
            | MechanicalChange::SetHardwareDimension { .. }
            | MechanicalChange::AddOpening
            | MechanicalChange::RemoveOpening { .. }
            | MechanicalChange::AddOpeningVertex { .. }
            | MechanicalChange::RemoveOpeningVertex { .. }
            | MechanicalChange::SetOpeningDimension { .. }
            | MechanicalChange::SetStabilizerKind(_)
    ) || matches!(
        change,
        MechanicalChange::SetMountPosition { .. }
            | MechanicalChange::SetGasketSupportPlacement { .. }
            | MechanicalChange::ResetGasketPlacement
    ) || matches!(
        change,
        MechanicalChange::SetDimension {
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
    )
}

#[derive(Clone, Copy)]
enum ProfileTargetKind {
    Switch,
    Stabilizer,
    ImportedGeometry,
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
        validate_opening_geometry(&opening.points, opening.z, opening.height, label, index)?;
    }
    Ok(())
}

fn validate_opening_geometry(
    points: &[Vec2],
    z: f64,
    height: f64,
    label: &str,
    index: usize,
) -> Result<(), String> {
    if points.len() < 3
        || points.iter().any(|point| {
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
    if !z.is_finite() || z < -1_000_000.0 {
        return Err(format!(
            "{label} {} bottom Z must be a finite coordinate no smaller than −1,000,000 mm.",
            index + 1
        ));
    }
    if !height.is_finite() || height < 0.1 {
        return Err(format!(
            "{label} {} height must be at least 0.1 mm.",
            index + 1
        ));
    }
    Ok(())
}

fn validate_openings(openings: &[MechanicalOpening]) -> Result<(), String> {
    for (index, opening) in openings.iter().enumerate() {
        validate_opening_geometry(
            &opening.points(),
            opening.z,
            opening.height,
            "Access opening",
            index,
        )?;
    }
    Ok(())
}

/// The lowest unused `opening-N` identity in this configuration.
fn next_opening_id(openings: &[MechanicalOpening]) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("opening-{index}");
        if !openings.iter().any(|opening| opening.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// The lowest unused `vertex-N` identity within one opening footprint.
fn next_vertex_id(opening: &MechanicalOpening) -> String {
    let mut index = 1;
    loop {
        let candidate = format!("vertex-{index}");
        if !opening.vertices.iter().any(|vertex| vertex.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}

/// Append a new authored opening with deterministic opening/vertex identities.
fn push_mechanical_opening(
    openings: &mut Vec<MechanicalOpening>,
    points: Vec<Vec2>,
    z: f64,
    height: f64,
) {
    let id = next_opening_id(openings);
    let vertices = points
        .into_iter()
        .enumerate()
        .map(|(index, point)| MechanicalOpeningVertex {
            id: format!("vertex-{}", index + 1),
            point,
        })
        .collect();
    openings.push(MechanicalOpening {
        id,
        vertices,
        z,
        height,
    });
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

fn normalize_processes(configuration: &mut MechanicalConfiguration, change: &MechanicalChange) {
    let processes = configuration.part_processes.get_or_insert_with(Vec::new);
    let top_level_method_change = matches!(change, MechanicalChange::SetMethod(_))
        || matches!(
            change,
            MechanicalChange::SetProcessMethod { part_id, .. } if part_id == "plate"
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
                process.material =
                    crate::mechanical::default_material(&process.part_id, &method).into();
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

fn family_assignment_changes(
    existing_family: Option<MechanicalSwitchFamily>,
    selected_family: MechanicalSwitchFamily,
) -> bool {
    existing_family != Some(selected_family)
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
mod tests {
    use super::*;
    use crate::mechanical::change::{
        MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension,
        MechanicalMountCollection,
    };
    use crate::model::{
        HardwareConfiguration, HardwareTopology, HardwareTransport,
        MechanicalConstructionOwnership, MechanicalGasketAnchor, MechanicalStabilizerOverride,
        Part, PartDefinition, PartGenerator, PartKind, PhysicalBoardInstance, Pose2, Side, Vec2,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn board(id: &str, part_ids: Vec<String>) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids,
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration() -> MechanicalConfiguration {
        serde_json::from_value(json!({
            "boardId": "board",
            "method": "printed",
            "mount": "rigid",
            "plateThickness": 1.5,
            "plateFoamThickness": 0.5,
            "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4,
            "batteryHeight": 4.0,
            "bottomThickness": 3.0,
            "plateToPcb": 3.5,
            "wallThickness": 2.0,
            "clearance": 0.3,
            "bottomStyle": "shell",
            "middleFrame": false,
            "openingAllowance": 0.25,
            "mounts": [],
            // Deliberate empty closure collection: most settings tests must not
            // trigger closure initialization and its required template.
            "closureMounts": [],
            "profiles": []
        }))
        .unwrap()
    }

    /// The same configuration with absent closure mounts, for initialization tests.
    fn configuration_without_closures() -> MechanicalConfiguration {
        let mut configuration = configuration();
        configuration.closure_mounts = None;
        configuration
    }

    /// The normalized bundled mounting-hole catalogue entry projection expects.
    fn template() -> PartDefinition {
        serde_json::from_value(json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board("board", vec![]));
        document.mechanical = Some(configuration());
        document
    }

    /// A board whose outline yields at least one suggested closure mount.
    fn document_with_outline() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        document.boards.push(board("board", vec![]));
        document.boards[0].outline_ids = vec!["outline".into()];
        document.mechanical = Some(configuration_without_closures());
        document
    }

    fn board_target() -> MechanicalTarget {
        MechanicalTarget::Board {
            board_id: "board".into(),
        }
    }

    fn instance_target() -> MechanicalTarget {
        MechanicalTarget::Instance {
            instance_id: "left".into(),
        }
    }

    fn apply_change(doc: &mut ProjectDoc, change: MechanicalChange) -> Result<Vec<String>, String> {
        apply(
            doc,
            &MechanicalEdit {
                target: board_target(),
                change,
                mounting_hole: Some(Box::new(template())),
            },
        )
    }

    fn apply_instance(
        doc: &mut ProjectDoc,
        change: MechanicalChange,
    ) -> Result<Vec<String>, String> {
        apply(
            doc,
            &MechanicalEdit {
                target: instance_target(),
                change,
                mounting_hole: Some(Box::new(template())),
            },
        )
    }

    fn stored(doc: &ProjectDoc) -> &MechanicalConfiguration {
        doc.mechanical.as_ref().expect("stored configuration")
    }

    fn part_definition(
        id: &str,
        kind: PartKind,
        generator: Option<PartGenerator>,
    ) -> PartDefinition {
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: id.into(),
            name: id.into(),
            kind,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator,
            mechanical_profile: None,
        }
    }

    fn part(id: &str, definition_id: &str, keycap: Option<Vec2>) -> Part {
        Part {
            keycap,
            outline: None,
            id: id.into(),
            definition_id: definition_id.into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn mx_generator() -> PartGenerator {
        PartGenerator {
            source: "ceoloide/switch_mx".into(),
            version: "1".into(),
            parameters: BTreeMap::new(),
        }
    }

    fn anchor(id: &str, anchor: f64) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            length: Some(12.0),
            width: Some(3.0),
            placement: Some(GasketPlacement::User),
            id: id.into(),
            region_id: id.into(),
            outline_key: id.into(),
            anchor,
            unlinked: false,
        }
    }

    #[test]
    fn a_dimension_change_applies_and_reports_the_configuration_owner() {
        let mut doc = document();
        let changed = apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )
        .unwrap();
        assert_eq!(changed, vec!["board"]);
        assert_eq!(stored(&doc).wall_thickness, 3.0);
    }

    #[test]
    fn a_change_equal_to_the_accepted_value_is_a_no_op() {
        let mut doc = document();
        let before = stored(&doc).clone();
        let changed = apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 2.0,
            },
        )
        .unwrap();
        assert!(changed.is_empty());
        assert_eq!(stored(&doc), &before);
    }

    #[test]
    fn malformed_values_error_and_mutate_nothing() {
        let mut doc = document();
        let before = stored(&doc).clone();
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: -1.0,
                },
            )
            .is_err()
        );
        assert_eq!(stored(&doc), &before);
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::BatteryWidth,
                    value: 30.0,
                },
            )
            .is_err(),
            "a battery dimension with no envelope errors"
        );
        assert_eq!(stored(&doc), &before);
        assert!(apply_change(&mut doc, MechanicalChange::AddOpening).is_ok());
        let before = stored(&doc).clone();
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetOpeningDimension {
                    opening_index: 9,
                    point_index: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 4.0,
                },
            )
            .is_err()
        );
        assert_eq!(stored(&doc), &before);
    }

    #[test]
    fn enable_and_disable_set_and_clear_the_configuration() {
        let mut doc = ProjectDoc::empty("doc", "Fixture");
        doc.boards.push(board("board", vec![]));
        let changed = apply_change(&mut doc, MechanicalChange::Enable).unwrap();
        assert_eq!(changed, vec!["board"]);
        let created = stored(&doc).clone();
        assert_eq!(created.board_id, "board");
        assert_eq!(created.method, PlateMethod::Printed);
        assert_eq!(created.mount, MechanicalMount::Tray);
        assert_eq!(created.part_processes.as_ref().unwrap().len(), 4);
        assert_eq!(
            created.closure_mounts,
            Some(vec![]),
            "enabling initializes the absent closure collection from the resolved assembly"
        );

        assert!(
            apply_change(&mut doc, MechanicalChange::Enable)
                .unwrap()
                .is_empty(),
            "enabling an already enabled target is a no-op"
        );

        let changed = apply_change(&mut doc, MechanicalChange::Disable).unwrap();
        assert_eq!(changed, vec!["board"]);
        assert!(doc.mechanical.is_none());
        assert!(
            apply_change(&mut doc, MechanicalChange::Disable)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn the_battery_family_applies_and_removes() {
        let mut doc = document();
        assert_eq!(
            apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(true)).unwrap(),
            vec!["board"]
        );
        let battery = stored(&doc).battery.clone().expect("battery envelope");
        assert_eq!(battery.size.z, 6.0);
        assert_eq!(battery.cable_width, Some(2.0));
        assert!(
            apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(true)).is_err(),
            "a second envelope is rejected"
        );
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::BatteryWidth,
                value: 31.0,
            },
        )
        .unwrap();
        assert_eq!(stored(&doc).battery.as_ref().unwrap().size.x, 31.0);
        assert_eq!(
            apply_change(&mut doc, MechanicalChange::SetBatteryEnabled(false)).unwrap(),
            vec!["board"]
        );
        assert!(stored(&doc).battery.is_none());
    }

    #[test]
    fn the_closure_hardware_family_applies() {
        let mut doc = document();
        doc.mechanical.as_mut().unwrap().internal_gasket =
            Some(crate::mechanical::default_internal_gasket());
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetClosureDrive(ScrewDrive::Torx),
            )
            .unwrap()
            .contains(&"board".to_string())
        );
        assert_eq!(
            stored(&doc)
                .internal_gasket
                .as_ref()
                .unwrap()
                .hardware
                .drive,
            ScrewDrive::Torx
        );
        apply_change(
            &mut doc,
            MechanicalChange::SetClosureInsertPreset("m2-4".into()),
        )
        .unwrap();
        let hardware = &stored(&doc).internal_gasket.as_ref().unwrap().hardware;
        assert_eq!(hardware.id, "custom-m2-4");
        assert_eq!(hardware.thread_diameter, 2.0);
        assert_eq!(hardware.insert_length, 4.0);
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetClosureInsertPreset("nope".into()),
            )
            .is_err()
        );
    }

    #[test]
    fn critical_fits_and_hardware_report_their_own_ids() {
        let mut doc = document();
        let changed = apply_change(
            &mut doc,
            MechanicalChange::AddCriticalFit {
                part_id: "plate".into(),
            },
        )
        .unwrap();
        assert_eq!(changed, vec!["board", "case-critical-fit-1"]);
        apply_change(
            &mut doc,
            MechanicalChange::SetCriticalFitText {
                fit_id: "case-critical-fit-1".into(),
                field: MechanicalCriticalFitTextField::Label,
                value: "Width".into(),
            },
        )
        .unwrap();
        assert_eq!(
            stored(&doc).critical_fits.as_ref().unwrap()[0].label,
            "Width"
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveCriticalFit {
                    fit_id: "case-critical-fit-1".into(),
                },
            )
            .unwrap(),
            vec!["board", "case-critical-fit-1"]
        );

        let changed = apply_change(
            &mut doc,
            MechanicalChange::AddHardware {
                part_id: "plate".into(),
                feature_id: "feature".into(),
            },
        )
        .unwrap();
        assert_eq!(changed, vec!["board", "case-mechanical-hardware-1"]);
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetHardwareDimension {
                    hardware_id: "case-mechanical-hardware-1".into(),
                    field: MechanicalDimension::HardwareQuantity,
                    value: 6.0,
                },
            )
            .unwrap(),
            vec!["board", "case-mechanical-hardware-1"]
        );
        assert_eq!(stored(&doc).hardware.as_ref().unwrap()[0].quantity, 6);
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveHardware {
                    hardware_id: "missing".into(),
                },
            )
            .is_err()
        );
    }

    #[test]
    fn the_opening_family_applies_and_reports_opening_ids() {
        let mut doc = document();
        assert_eq!(
            apply_change(&mut doc, MechanicalChange::AddOpening).unwrap(),
            vec!["board", "opening-1"]
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetOpeningDimension {
                    opening_index: 0,
                    point_index: None,
                    field: MechanicalDimension::OpeningHeight,
                    value: 12.0,
                },
            )
            .unwrap(),
            vec!["board", "opening-1"]
        );
        assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].height, 12.0);
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::AddOpeningVertex { opening_index: 0 },
            )
            .unwrap(),
            vec!["board", "opening-1"]
        );
        assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].vertices.len(), 5);
        apply_change(
            &mut doc,
            MechanicalChange::RemoveOpeningVertex {
                opening_index: 0,
                point_index: 0,
            },
        )
        .unwrap();
        assert_eq!(stored(&doc).openings.as_ref().unwrap()[0].vertices.len(), 4);
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveOpening { opening_index: 0 },
            )
            .unwrap(),
            vec!["board", "opening-1"]
        );
        assert!(stored(&doc).openings.as_ref().unwrap().is_empty());
    }

    #[test]
    fn explicit_empty_mount_collections_survive_and_absent_ones_initialize() {
        // A deliberate explicit empty collection is preserved and never re-initialized.
        let mut doc = document();
        assert_eq!(stored(&doc).closure_mounts, Some(vec![]));
        apply_change(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )
        .unwrap();
        assert_eq!(
            stored(&doc).closure_mounts,
            Some(vec![]),
            "an unrelated change preserves a deliberate explicit empty collection"
        );
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetMountCollection {
                    collection: MechanicalMountCollection::Closure,
                    mounts: vec![],
                },
            )
            .unwrap()
            .is_empty(),
            "repeating the explicit empty value is a no-op"
        );

        // An absent collection becomes the explicit empty value on request.
        let mut absent = document_with_outline();
        assert!(stored(&absent).closure_mounts.is_none());
        assert_eq!(
            apply_change(
                &mut absent,
                MechanicalChange::SetMountCollection {
                    collection: MechanicalMountCollection::Closure,
                    mounts: vec![],
                },
            )
            .unwrap(),
            vec!["board"]
        );
        assert_eq!(stored(&absent).closure_mounts, Some(vec![]));
        assert!(
            absent
                .parts
                .iter()
                .all(|part| !part.id.starts_with("case-closure/")),
            "an explicit empty collection projects no clearance parts"
        );

        let changed = apply_change(
            &mut doc,
            MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Closure,
            },
        )
        .unwrap();
        assert!(changed.contains(&"board".to_string()));
        assert!(changed.contains(&"case-mechanical-mount-1".to_string()));
        assert!(
            changed.iter().any(|id| id.starts_with("case-closure/")),
            "the added closure mount projects a generated clearance part"
        );
        assert_eq!(stored(&doc).closure_mounts.as_ref().unwrap().len(), 1);
    }

    /// Case 1: absent closure mounts initialize from the resolved assembly, and an
    /// explicit `Some(vec![])` stays deliberate.
    #[test]
    fn absent_closure_mounts_initialize_from_the_resolved_assembly() {
        let mut doc = document_with_outline();
        assert!(stored(&doc).closure_mounts.is_none());
        let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
        let mounts = stored(&doc)
            .closure_mounts
            .clone()
            .expect("initialized mounts");
        assert!(!mounts.is_empty(), "the outline suggests closure mounts");
        for mount in &mounts {
            assert!(mount.id.starts_with("auto-closure/"));
            assert_eq!(mount.kind, MountKind::Boss);
            assert_eq!(mount.hole_diameter, 2.2);
            assert_eq!(
                mount.height,
                Some(
                    stored(&doc).plate_to_pcb
                        + stored(&doc).pcb_thickness
                        + stored(&doc)
                            .bottom_foam_thickness
                            .max(stored(&doc).battery_height)
                )
            );
        }
        assert!(changed.contains(&"board".to_string()));

        // Explicit empty is deliberate and never re-initialized.
        let mut deliberate = document_with_outline();
        deliberate.mechanical.as_mut().unwrap().closure_mounts = Some(vec![]);
        assert!(
            apply_change(&mut deliberate, MechanicalChange::InitializeClosures).is_err(),
            "an explicit empty collection is not eligible for initialization"
        );
        assert_eq!(stored(&deliberate).closure_mounts, Some(vec![]));
    }

    /// Case 2 and 4: generated clearance parts, definitions and memberships appear
    /// with the configuration owner and nothing else.
    #[test]
    fn initialization_reports_only_generated_ids_and_the_owner() {
        let mut doc = document_with_outline();
        let changed = apply_change(&mut doc, MechanicalChange::InitializeClosures).unwrap();
        assert_eq!(changed[0], "board", "the configuration owner leads");
        assert!(!doc.parts.is_empty(), "generated clearance parts exist");
        for part in &doc.parts {
            assert!(
                changed.contains(&part.id),
                "generated part {} is reported",
                part.id
            );
        }
        for definition in &doc.definitions {
            assert!(changed.contains(&definition.id));
        }
        assert!(changed.contains(&"board".to_string()));
        for id in &changed {
            assert!(
                id == "board"
                    || id.starts_with("case-closure/")
                    || id.starts_with("assembly-closure/definition/"),
                "unexpected changed id {id}"
            );
        }
    }

    /// Case 5: a required projection without a template rolls back the configuration.
    #[test]
    fn a_missing_required_template_rolls_back_the_configuration() {
        let mut doc = document();
        let before = doc.clone();
        let error = apply(
            &mut doc,
            &MechanicalEdit {
                target: board_target(),
                change: MechanicalChange::AddOpening,
                mounting_hole: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("mounting-hole"), "{error}");
        assert_eq!(doc, before, "the configuration change rolled back");
    }

    #[test]
    fn the_gasket_support_family_applies() {
        let mut doc = document();
        doc.mechanical.as_mut().unwrap().mount = MechanicalMount::Gasket;
        doc.mechanical.as_mut().unwrap().internal_gasket =
            Some(crate::mechanical::default_internal_gasket());
        let mut layout = crate::mechanical::default_gasket_layout();
        layout.supports = vec![anchor("left:0", 0.25)];
        doc.mechanical.as_mut().unwrap().gasket_layout = Some(layout);

        apply_change(
            &mut doc,
            MechanicalChange::SetGasketSupportDimension {
                support_id: "left:0".into(),
                anchors: vec![anchor("left:0", 0.25)],
                field: MechanicalDimension::GasketSupportLength,
                value: 20.0,
            },
        )
        .unwrap();
        assert_eq!(
            stored(&doc).gasket_layout.as_ref().unwrap().supports[0]
                .length
                .unwrap(),
            20.0
        );
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetGasketSupportDimension {
                    support_id: "left:0".into(),
                    anchors: vec![anchor("left:0", 0.25)],
                    field: MechanicalDimension::GasketSupportLength,
                    value: 4.0,
                },
            )
            .is_err()
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetGasketSupportPlacement {
                    support_id: "left:0".into(),
                    anchors: vec![anchor("left:0", 0.4)],
                },
            )
            .unwrap(),
            vec!["board", "left:0"]
        );
        assert_eq!(
            stored(&doc).gasket_layout.as_ref().unwrap().supports[0].anchor,
            0.4
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetGasketSupportUnlinked {
                    support_id: "left:0".into(),
                    pair_id: None,
                    anchors: vec![anchor("left:0", 0.4)],
                },
            )
            .unwrap(),
            vec!["board", "left:0"]
        );
        assert!(stored(&doc).gasket_layout.as_ref().unwrap().supports[0].unlinked);
        assert_eq!(
            apply_change(&mut doc, MechanicalChange::ResetGasketPlacement).unwrap(),
            vec!["board"]
        );
        assert!(
            stored(&doc)
                .gasket_layout
                .as_ref()
                .unwrap()
                .supports
                .is_empty()
        );
    }

    #[test]
    fn the_process_override_family_applies() {
        let mut doc = document();
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::AddProcessOverride {
                    part_id: "plate".into(),
                },
            )
            .unwrap(),
            vec!["board", "plate"]
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetProcessMethod {
                    part_id: "plate".into(),
                    method: PlateMethod::Cnc,
                },
            )
            .unwrap(),
            vec!["board", "plate"]
        );
        assert_eq!(stored(&doc).method, PlateMethod::Cnc);
        apply_change(
            &mut doc,
            MechanicalChange::SetProcessMaterial {
                part_id: "plate".into(),
                material: "Aluminium".into(),
            },
        )
        .unwrap();
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetProcessMaterial {
                    part_id: "plate".into(),
                    material: "PLA".into(),
                },
            )
            .is_err(),
            "PLA is not available for CNC"
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveProcessOverride {
                    part_id: "plate".into(),
                },
            )
            .unwrap(),
            vec!["board", "plate"]
        );
    }

    #[test]
    fn profile_assignment_and_switch_family_apply() {
        let mut doc = document();
        doc.boards[0].part_ids = vec!["sw".into()];
        doc.definitions.push(part_definition(
            "switch-mx",
            PartKind::Switch,
            Some(mx_generator()),
        ));
        doc.parts.push(part("sw", "switch-mx", None));

        let changed = apply_change(
            &mut doc,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::ChocV1,
            },
        )
        .unwrap();
        assert_eq!(changed, vec!["board", "switch-mx"]);
        assert_eq!(stored(&doc).plate_thickness, 1.3);
        assert_eq!(stored(&doc).plate_to_pcb, 2.2);
        assert_eq!(stored(&doc).profiles.len(), 1);
        assert_eq!(
            stored(&doc).profiles[0].switch_family,
            Some(MechanicalSwitchFamily::ChocV1)
        );

        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::AssignSwitchProfile {
                    definition_id: "switch-mx".into(),
                    family: MechanicalSwitchFamily::ChocV1,
                },
            )
            .unwrap_err()
            .contains("already assigned")
        );

        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetSwitchFamily {
                    definition_id: "switch-mx".into(),
                    family: MechanicalSwitchFamily::Mx,
                },
            )
            .unwrap(),
            vec!["board", "switch-mx"]
        );
        assert_eq!(stored(&doc).plate_thickness, 1.5);
        assert_eq!(stored(&doc).plate_to_pcb, 3.5);

        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveProfile {
                    definition_id: "switch-mx".into(),
                },
            )
            .unwrap(),
            vec!["board", "switch-mx"]
        );
        assert!(stored(&doc).profiles.is_empty());
    }

    #[test]
    fn imported_geometry_assignment_and_profile_update_apply() {
        let mut doc = document();
        doc.boards[0].part_ids = vec!["part".into()];
        let mut definition = part_definition("imported", PartKind::Custom, None);
        definition.kicad_source = Some(KicadSource {
            format_version: 1,
            source: "(footprint \"x\")".into(),
        });
        doc.definitions.push(definition);
        doc.parts.push(part("part", "imported", None));

        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::AssignImportedGeometryProfile {
                    definition_id: "imported".into(),
                },
            )
            .unwrap(),
            vec!["board", "imported"]
        );
        assert_eq!(stored(&doc).profiles.len(), 1);
        assert!(stored(&doc).profiles[0].source.starts_with("KiCad"));

        let profile = stored(&doc).profiles[0].clone();
        assert!(
            apply_change(&mut doc, MechanicalChange::UpdateProfile(profile.clone()))
                .unwrap()
                .is_empty(),
            "an identical profile update is a no-op"
        );

        let mut updated = profile;
        updated.source_geometry = Some(MechanicalProfileSource {
            text: "(footprint \"x\")".into(),
            sha256: "sha".into(),
            mappings: vec![],
            source_ids: vec![],
        });
        updated.cutouts = vec![vec![
            Vec2 { x: -1.0, y: 1.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 1.0, y: -1.0 },
        ]];
        assert_eq!(
            apply_change(&mut doc, MechanicalChange::UpdateProfile(updated.clone())).unwrap(),
            vec!["board", "imported"]
        );
        assert_eq!(stored(&doc).profiles[0], updated);
    }

    #[test]
    fn mount_editing_reports_the_mount_id() {
        let mut doc = document();
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::AddMount {
                    collection: MechanicalMountCollection::Suspension,
                },
            )
            .unwrap(),
            vec!["board", "case-mechanical-mount-1"]
        );
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::SetMountDimension {
                    collection: MechanicalMountCollection::Suspension,
                    mount_id: "case-mechanical-mount-1".into(),
                    field: MechanicalDimension::MountHoleDiameter,
                    value: 3.0,
                },
            )
            .unwrap(),
            vec!["board", "case-mechanical-mount-1"]
        );
        assert_eq!(stored(&doc).mounts[0].hole_diameter, 3.0);
        apply_change(
            &mut doc,
            MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
                at: Vec2 { x: 4.0, y: 5.0 },
            },
        )
        .unwrap();
        assert_eq!(stored(&doc).mounts[0].at, Vec2 { x: 4.0, y: 5.0 });
        apply_change(
            &mut doc,
            MechanicalChange::SetMountKind {
                collection: MechanicalMountCollection::Suspension,
                mount_id: "case-mechanical-mount-1".into(),
                kind: MountKind::Boss,
            },
        )
        .unwrap();
        assert_eq!(stored(&doc).mounts[0].kind, MountKind::Boss);
        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::RemoveMount {
                    collection: MechanicalMountCollection::Suspension,
                    mount_id: "case-mechanical-mount-1".into(),
                },
            )
            .unwrap(),
            vec!["board", "case-mechanical-mount-1"]
        );
        assert!(stored(&doc).mounts.is_empty());
    }

    #[test]
    fn a_profile_target_on_another_board_errors() {
        let mut doc = document();
        doc.boards.push(board("other", vec!["sw".into()]));
        doc.definitions.push(part_definition(
            "switch-mx",
            PartKind::Switch,
            Some(mx_generator()),
        ));
        doc.parts.push(part("sw", "switch-mx", None));
        let before = stored(&doc).clone();
        let error = apply_change(
            &mut doc,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
        )
        .unwrap_err();
        assert!(error.contains("not available"), "{error}");
        assert_eq!(stored(&doc), &before);
    }

    #[test]
    fn stabilizer_profile_and_kind_apply() {
        let mut doc = document();
        doc.boards[0].part_ids = vec!["wide".into(), "stab".into()];
        doc.definitions.push(part_definition(
            "switch-mx",
            PartKind::Switch,
            Some(mx_generator()),
        ));
        doc.definitions
            .push(part_definition("stab-mx-2u", PartKind::Custom, None));
        doc.parts
            .push(part("wide", "switch-mx", Some(Vec2 { x: 40.0, y: 20.0 })));
        doc.parts.push(part("stab", "stab-mx-2u", None));

        assert_eq!(
            apply_change(
                &mut doc,
                MechanicalChange::AssignStabilizerProfile {
                    definition_id: "stab-mx-2u".into(),
                    source: MechanicalBuiltinProfile::MxStab2u,
                },
            )
            .unwrap(),
            vec!["board", "stab-mx-2u"]
        );
        assert_eq!(stored(&doc).profiles.len(), 1);

        let changed = apply_change(
            &mut doc,
            MechanicalChange::SetStabilizerKind(MechanicalStabilizerOverride {
                profile: None,
                part_id: "wide".into(),
                kind: MechanicalStabilizerKind::PcbMount,
                units: 2.0,
                rotation: None,
            }),
        )
        .unwrap();
        assert_eq!(changed, vec!["board", "wide"]);
        assert_eq!(stored(&doc).stabilizers.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn method_mount_and_frame_switches_apply() {
        let mut doc = document();
        apply_change(&mut doc, MechanicalChange::SetMethod(PlateMethod::Cnc)).unwrap();
        assert_eq!(stored(&doc).method, PlateMethod::Cnc);
        apply_change(
            &mut doc,
            MechanicalChange::SetBottomStyle(MechanicalBottomStyle::Sheet),
        )
        .unwrap();
        assert_eq!(
            stored(&doc).bottom_style,
            Some(MechanicalBottomStyle::Sheet)
        );
        apply_change(&mut doc, MechanicalChange::SetMiddleFrame(true)).unwrap();
        assert_eq!(stored(&doc).middle_frame, Some(true));
        apply_change(&mut doc, MechanicalChange::SetIntegratedPlateFrame(true)).unwrap();
        assert!(stored(&doc).integrated_plate_frame);

        apply_change(
            &mut doc,
            MechanicalChange::SetMount(MechanicalMount::Gasket),
        )
        .unwrap();
        let gasket = stored(&doc);
        assert_eq!(gasket.mount, MechanicalMount::Gasket);
        assert!(gasket.gasket_layout.is_some());
        assert!(gasket.internal_gasket.is_some());
        assert_eq!(gasket.gasket_travel, Some(0.1));
        assert!(!gasket.integrated_plate_frame);
    }

    #[test]
    fn an_instance_target_persists_to_that_instance() {
        let mut doc = document();
        let mut instance = PhysicalBoardInstance {
            id: "left".into(),
            name: "Left".into(),
            board_id: "board".into(),
            half: "left".into(),
            role: "controller".into(),
            flipped: false,
            controller_part_id: None,
            mechanical: Some(configuration()),
            construction_ownership: MechanicalConstructionOwnership::Local,
        };
        instance.mechanical.as_mut().unwrap().wall_thickness = 2.0;
        doc.hardware = Some(HardwareConfiguration {
            topology: HardwareTopology::Unibody,
            transport: HardwareTransport::None,
            instances: vec![instance],
            boards: vec![],
            shared_construction: None,
            pairs: vec![],
        });
        let changed = apply_instance(
            &mut doc,
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
        )
        .unwrap();
        assert_eq!(changed, vec!["left"]);
        assert_eq!(
            doc.hardware.as_ref().unwrap().instances[0]
                .mechanical
                .as_ref()
                .unwrap()
                .wall_thickness,
            3.0
        );
        assert!(
            doc.mechanical.as_ref().unwrap().wall_thickness == 2.0,
            "the canonical configuration is untouched"
        );
    }

    #[test]
    fn vanished_or_ineligible_targets_error() {
        let mut doc = ProjectDoc::empty("doc", "Fixture");
        doc.boards.push(board("board", vec![]));
        assert!(
            apply_change(
                &mut doc,
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 3.0,
                },
            )
            .is_err(),
            "no stored configuration and no enable"
        );
        assert!(
            apply_instance(&mut doc, MechanicalChange::Enable).is_err(),
            "an unknown instance is unavailable"
        );
    }

    /// Case 7: `affects_outline` classifies each change kind.
    #[test]
    fn outline_classification_covers_the_projection_changes() {
        let pure: Vec<MechanicalChange> = vec![
            MechanicalChange::SetBatteryEnabled(true),
            MechanicalChange::SetClosureDrive(ScrewDrive::Torx),
            MechanicalChange::SetClosureInsertPreset("m2-4".into()),
            MechanicalChange::AddCriticalFit {
                part_id: "plate".into(),
            },
            MechanicalChange::AddHardware {
                part_id: "plate".into(),
                feature_id: "feature".into(),
            },
            MechanicalChange::SetMethod(PlateMethod::Cnc),
            MechanicalChange::SetMount(MechanicalMount::Gasket),
            MechanicalChange::SetBottomStyle(MechanicalBottomStyle::Shell),
            MechanicalChange::SetMiddleFrame(true),
            MechanicalChange::SetIntegratedPlateFrame(true),
            MechanicalChange::AddProcessOverride {
                part_id: "plate".into(),
            },
            MechanicalChange::SetDimension {
                field: MechanicalDimension::WallThickness,
                value: 3.0,
            },
            MechanicalChange::SetDimension {
                field: MechanicalDimension::PlateThickness,
                value: 2.0,
            },
            MechanicalChange::SetStabilizerKind(MechanicalStabilizerOverride {
                profile: None,
                part_id: "wide".into(),
                kind: MechanicalStabilizerKind::PcbMount,
                units: 2.0,
                rotation: None,
            }),
        ];
        for change in pure {
            assert!(
                !affects_outline(&change),
                "pure settings edit {change:?} must not rebuild outlines"
            );
        }

        let projection: Vec<MechanicalChange> = vec![
            MechanicalChange::Enable,
            MechanicalChange::Disable,
            MechanicalChange::InitializeClosures,
            MechanicalChange::AddOpening,
            MechanicalChange::RemoveOpening { opening_index: 0 },
            MechanicalChange::AddOpeningVertex { opening_index: 0 },
            MechanicalChange::RemoveOpeningVertex {
                opening_index: 0,
                point_index: 0,
            },
            MechanicalChange::SetOpeningDimension {
                opening_index: 0,
                point_index: None,
                field: MechanicalDimension::OpeningHeight,
                value: 12.0,
            },
            MechanicalChange::AddMount {
                collection: MechanicalMountCollection::Closure,
            },
            MechanicalChange::AddSuggestedClosureMounts(vec![]),
            MechanicalChange::SetMountCollection {
                collection: MechanicalMountCollection::Closure,
                mounts: vec![],
            },
            MechanicalChange::AdoptClosurePositions(vec![]),
            MechanicalChange::SetMountDimension {
                collection: MechanicalMountCollection::Closure,
                mount_id: "m".into(),
                field: MechanicalDimension::MountHoleDiameter,
                value: 2.4,
            },
            MechanicalChange::SetMountPosition {
                collection: MechanicalMountCollection::Closure,
                mount_id: "m".into(),
                at: Vec2 { x: 1.0, y: 2.0 },
            },
            MechanicalChange::SetMountKind {
                collection: MechanicalMountCollection::Closure,
                mount_id: "m".into(),
                kind: MountKind::Boss,
            },
            MechanicalChange::RemoveMount {
                collection: MechanicalMountCollection::Closure,
                mount_id: "m".into(),
            },
            MechanicalChange::SetDimension {
                field: MechanicalDimension::Clearance,
                value: 0.4,
            },
            MechanicalChange::SetGasketSupportPlacement {
                support_id: "s".into(),
                anchors: vec![],
            },
            MechanicalChange::ResetGasketPlacement,
            MechanicalChange::AssignSwitchProfile {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
            MechanicalChange::AssignStabilizerProfile {
                definition_id: "stab".into(),
                source: MechanicalBuiltinProfile::MxStab2u,
            },
            MechanicalChange::AssignImportedGeometryProfile {
                definition_id: "imported".into(),
            },
            MechanicalChange::RemoveProfile {
                definition_id: "switch-mx".into(),
            },
            MechanicalChange::SetSwitchFamily {
                definition_id: "switch-mx".into(),
                family: MechanicalSwitchFamily::Mx,
            },
        ];
        for change in projection {
            assert!(
                affects_outline(&change),
                "projection change {change:?} must rebuild outlines"
            );
            assert_eq!(affects_outline(&change), requires_projection(&change));
        }
    }
}
