//! Default configurations and application of one typed mechanical change.

use super::super::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension,
    MechanicalHardwareTextField, MechanicalMountCollection,
};
use super::super::inferred_switch_family;
use super::{
    IdentityFloor, ProfileTargetKind, closure_hardware_mut, critical_fit_mut,
    family_assignment_changes, hardware_mut, identity_index_for, initial_switch_family,
    is_process_target, material_options, mount_collection_mut, normalize_processes,
    process_default_thickness, process_mut, profile_family, reserve_identity,
    reserve_opening_identity, reserve_vertex_identity, resize_closure_insert, set_dimension,
    stabilizer_target_exists, valid_profile_polygons, validate_dimension,
    validate_gasket_support_anchors, validate_mounts, validate_openings, validate_profile_openings,
    validate_profile_target,
};
use crate::model::*;

fn remove_changed<T>(
    items: Option<&mut Vec<T>>,
    matches: impl FnMut(&T) -> bool,
    id: &str,
    unavailable: &str,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let items = items.ok_or(unavailable)?;
    let index = items.iter().position(matches).ok_or(unavailable)?;
    items.remove(index);
    changed.push(id.to_owned());
    Ok(())
}

/// Reference defaults for a board that has no stored configuration.
///
/// Ports the adapter's `case_settings::initial_settings` family selection, plate
/// and foam derivation, board thickness and standard process overrides onto the
/// Core-owned default helpers.
pub fn default_configuration(
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
        next_opening_identity: 0,
        next_critical_fit_identity: 0,
        next_hardware_identity: 0,
        next_mount_identity: 0,
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

pub(super) fn apply_change(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    board_id: &str,
    identity_floor: IdentityFloor,
    changed: &mut Vec<String>,
) -> Result<(), String> {
    let previous = configuration.clone();
    match change {
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures
        | MechanicalChange::LinkMirrorPair { .. }
        | MechanicalChange::UnlinkMirrorPair { .. }
        | MechanicalChange::RemovePhysicalInstance
        | MechanicalChange::MaterializeLegacyConstruction { .. }
        | MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => {
            return Err("This change is handled by the mechanical edit lifecycle.".into());
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
                .and_then(|part| inferred_switch_family(part, definition));
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
            let highest = configuration
                .critical_fits
                .as_ref()
                .into_iter()
                .flatten()
                .filter_map(|fit| identity_index_for(&fit.id, "case-critical-fit"))
                .max()
                .unwrap_or(0);
            let fit_id = reserve_identity(
                &mut configuration.next_critical_fit_identity,
                highest,
                identity_floor.critical_fit,
                "case-critical-fit",
                "Critical fit",
            )?;
            configuration
                .critical_fits
                .get_or_insert_with(Vec::new)
                .push(MechanicalCriticalFit {
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
            remove_changed(
                configuration.critical_fits.as_mut(),
                |item| item.id == *fit_id,
                fit_id,
                "The selected critical fit is no longer available.",
                changed,
            )?;
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
            let highest = configuration
                .hardware
                .as_ref()
                .into_iter()
                .flatten()
                .filter_map(|entry| identity_index_for(&entry.id, "case-mechanical-hardware"))
                .max()
                .unwrap_or(0);
            let hardware_id = reserve_identity(
                &mut configuration.next_hardware_identity,
                highest,
                identity_floor.hardware,
                "case-mechanical-hardware",
                "Hardware",
            )?;
            configuration.hardware.get_or_insert_with(Vec::new).push(
                MechanicalHardwareSpecification {
                    id: hardware_id.clone(),
                    part_id: part_id.clone(),
                    feature_id: feature_id.clone(),
                    designation: "Socket screw".into(),
                    thread: "M2 × 0.4".into(),
                    length: 6.0,
                    quantity: 4,
                    notes: Some(String::new()),
                    tolerance: None,
                },
            );
            changed.push(hardware_id);
        }
        MechanicalChange::RemoveHardware { hardware_id } => {
            remove_changed(
                configuration.hardware.as_mut(),
                |item| item.id == *hardware_id,
                hardware_id,
                "The selected hardware entry is no longer available.",
                changed,
            )?;
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
                    validate_dimension(*field, *value)?;
                    entry.length = *value;
                }
                MechanicalDimension::HardwareQuantity => {
                    validate_dimension(*field, *value)?;
                    entry.quantity = *value as u32;
                }
                _ => return Err("The selected hardware dimension is unavailable.".into()),
            }
            changed.push(hardware_id.clone());
        }
        MechanicalChange::AddOpening => {
            let id = reserve_opening_identity(configuration, identity_floor.opening)?;
            let mut opening = MechanicalOpening {
                id: id.clone(),
                vertices: Vec::new(),
                z: 0.0,
                height: 10.0,
                next_vertex_identity: 0,
            };
            for point in [
                Vec2 { x: -2.5, y: -2.5 },
                Vec2 { x: 2.5, y: -2.5 },
                Vec2 { x: 2.5, y: 2.5 },
                Vec2 { x: -2.5, y: 2.5 },
            ] {
                let vertex_id = reserve_vertex_identity(&mut opening, identity_floor.vertex)?;
                opening.vertices.push(MechanicalOpeningVertex {
                    id: vertex_id,
                    point,
                });
            }
            configuration
                .openings
                .get_or_insert_with(Vec::new)
                .push(opening);
            changed.push(id);
        }
        MechanicalChange::RemoveOpening { opening_id } => {
            remove_changed(
                configuration.openings.as_mut(),
                |item| item.id == *opening_id,
                opening_id,
                "The selected access opening is no longer available.",
                changed,
            )?;
        }
        MechanicalChange::AddOpeningVertex { opening_id } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or("The selected access opening is no longer available.")?;
            // Reserved from the opening's own monotonic counter, so a removed vertex
            // id can never be handed out again within that opening.
            let vertex_id = reserve_vertex_identity(opening, identity_floor.vertex)?;
            opening.vertices.push(MechanicalOpeningVertex {
                id: vertex_id.clone(),
                point: Vec2 { x: 0.0, y: 0.0 },
            });
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
            changed.push(vertex_id);
        }
        MechanicalChange::RemoveOpeningVertex {
            opening_id,
            vertex_id,
        } => {
            let openings = configuration
                .openings
                .as_mut()
                .ok_or("The selected access opening is no longer available.")?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or("The selected access opening is no longer available.")?;
            let position = opening
                .vertices
                .iter()
                .position(|vertex| vertex.id == *vertex_id)
                .ok_or("The selected opening vertex is no longer available.")?;
            if opening.vertices.len() <= 3 {
                return Err("An opening footprint needs at least three vertices.".into());
            }
            opening.vertices.remove(position);
            let opening_id = opening.id.clone();
            validate_openings(openings)?;
            changed.push(opening_id);
            changed.push(vertex_id.clone());
        }
        MechanicalChange::SetOpeningDimension {
            opening_id,
            vertex_id,
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
            if !(is_volume_field && vertex_id.is_none() || is_point_field && vertex_id.is_some()) {
                return Err("The selected opening field is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            let openings = configuration
                .openings
                .as_mut()
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            let opening = openings
                .iter_mut()
                .find(|opening| opening.id == *opening_id)
                .ok_or_else(|| "The selected access opening is no longer available.".to_owned())?;
            if let Some(vertex_id) = vertex_id {
                let vertex = opening
                    .vertices
                    .iter_mut()
                    .find(|vertex| vertex.id == *vertex_id)
                    .ok_or_else(|| {
                        "The selected opening vertex is no longer available.".to_owned()
                    })?;
                match field {
                    MechanicalDimension::OpeningPointX => vertex.point.x = *value,
                    MechanicalDimension::OpeningPointY => vertex.point.y = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
                let opening_id = opening.id.clone();
                changed.push(opening_id);
                changed.push(vertex_id.clone());
            } else {
                match field {
                    MechanicalDimension::OpeningBottomZ => opening.z = *value,
                    MechanicalDimension::OpeningHeight => opening.height = *value,
                    _ => return Err("The selected opening field is unavailable.".into()),
                }
                changed.push(opening.id.clone());
            }
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
            // The old first-gap search skipped ids used by the whole document. Keeping
            // those collision exclusions as the high-water mark makes the next id
            // monotonic without ever colliding with a Part or Case-body mount.
            let highest = configuration
                .mounts
                .iter()
                .chain(configuration.closure_mounts.iter().flatten())
                .chain(
                    document
                        .case_bodies
                        .iter()
                        .flat_map(|body| body.mounts.iter().flatten()),
                )
                .map(|mount| &mount.id)
                .chain(document.parts.iter().map(|part| &part.id))
                .filter_map(|id| identity_index_for(id, "case-mechanical-mount"))
                .max()
                .unwrap_or(0);
            let id = reserve_identity(
                &mut configuration.next_mount_identity,
                highest,
                identity_floor.mount,
                "case-mechanical-mount",
                "Mount",
            )?;
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
            remove_changed(
                Some(mount_collection_mut(configuration, *collection)),
                |mount| mount.id == *mount_id,
                mount_id,
                "The selected mount is no longer available.",
                changed,
            )?;
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
            validate_gasket_support_anchors(anchors)?;
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

pub(super) fn skip_process_normalization(change: &MechanicalChange) -> bool {
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
            | MechanicalChange::SetMountPosition { .. }
            | MechanicalChange::SetGasketSupportPlacement { .. }
            | MechanicalChange::ResetGasketPlacement
            | MechanicalChange::SetDimension {
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
