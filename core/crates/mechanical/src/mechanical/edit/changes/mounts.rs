//! Mounts, closure mounts and hardware, and critical-fit changes.
use super::*;

/// Applies one change in this domain, or `None` when another domain owns it.
fn mount_mut<'a>(
    configuration: &'a mut MechanicalConfiguration,
    collection: MechanicalMountCollection,
    id: &str,
) -> Result<&'a mut Mount, String> {
    find_item_mut(
        mount_collection_mut(configuration, collection),
        |mount| mount.id == id,
        MOUNT_UNAVAILABLE,
    )
}

pub(super) fn try_apply(
    configuration: &mut MechanicalConfiguration,
    change: &MechanicalChange,
    document: &ProjectDoc,
    identity_floor: IdentityFloor,
    changed: &mut Vec<String>,
) -> Result<Option<()>, String> {
    match change {
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
            let removed = remove_item(
                configuration.critical_fits.as_mut(),
                |item| item.id == *fit_id,
                "The selected critical fit is no longer available.",
            )?;
            changed.push(removed.id);
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
            let endpoint: fn(&mut MechanicalCriticalFit) -> &mut f64 = match field {
                MechanicalDimension::CriticalFitFromX => |fit| &mut fit.from.x,
                MechanicalDimension::CriticalFitFromY => |fit| &mut fit.from.y,
                MechanicalDimension::CriticalFitToX => |fit| &mut fit.to.x,
                MechanicalDimension::CriticalFitToY => |fit| &mut fit.to.y,
                _ => return Err("The selected critical-fit endpoint is unavailable.".into()),
            };
            validate_dimension(*field, *value)?;
            *endpoint(critical_fit_mut(configuration, fit_id)?) = *value;
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
            let removed = remove_item(
                configuration.hardware.as_mut(),
                |item| item.id == *hardware_id,
                "The selected hardware entry is no longer available.",
            )?;
            changed.push(removed.id);
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
            if !matches!(
                field,
                MechanicalDimension::HardwareLength | MechanicalDimension::HardwareQuantity
            ) {
                return Err("The selected hardware dimension is unavailable.".into());
            }
            validate_dimension(*field, *value)?;
            if *field == MechanicalDimension::HardwareLength {
                entry.length = *value;
            } else {
                entry.quantity = *value as u32;
            }
            changed.push(hardware_id.clone());
        }
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
            let mount = mount_mut(configuration, *collection, mount_id)?;
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
            let mount = mount_mut(configuration, *collection, mount_id)?;
            mount.at = *at;
            changed.push(mount_id.clone());
        }
        MechanicalChange::SetMountKind {
            collection,
            mount_id,
            kind,
        } => {
            let mount = mount_mut(configuration, *collection, mount_id)?;
            mount.kind = kind.clone();
            changed.push(mount_id.clone());
        }
        MechanicalChange::RemoveMount {
            collection,
            mount_id,
        } => {
            if mount_id.is_empty() {
                return Err(MOUNT_UNAVAILABLE.into());
            }
            if *collection == MechanicalMountCollection::Suspension
                && configuration.mount == MechanicalMount::Gasket
            {
                return Err("Suspension mounts are unavailable for a gasket mount.".into());
            }
            let removed = remove_item(
                Some(mount_collection_mut(configuration, *collection)),
                |mount| mount.id == *mount_id,
                MOUNT_UNAVAILABLE,
            )?;
            changed.push(removed.id);
        }
        _ => return Ok(None),
    };
    Ok(Some(()))
}
