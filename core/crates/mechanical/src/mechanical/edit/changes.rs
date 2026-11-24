//! Default configurations and application of one typed mechanical change.
use super::super::change::{
    MechanicalChange, MechanicalCriticalFitTextField, MechanicalDimension,
    MechanicalHardwareTextField, MechanicalMountCollection,
};
use super::super::inferred_switch_family;
use super::validation::{find_item_mut, upsert_item, validate_gasket_anchor_selection};
use super::{
    IdentityFloor, ProfileTargetKind, closure_hardware_mut, critical_fit_mut, hardware_mut,
    identity_index_for, initial_switch_family, is_process_target, material_options,
    mount_collection_mut, normalize_processes, process_default_thickness, process_mut,
    profile_family, reserve_identity, reserve_opening_identity, reserve_vertex_identity,
    resize_closure_insert, set_dimension, stabilizer_target_exists, valid_profile_polygons,
    validate_dimension, validate_gasket_support_anchors, validate_mounts, validate_openings,
    validate_profile_openings, validate_profile_target,
};
use crate::model::*;

const OPENING_UNAVAILABLE: &str = "The selected access opening is no longer available.";
const VERTEX_UNAVAILABLE: &str = "The selected opening vertex is no longer available.";
const MOUNT_UNAVAILABLE: &str = "The selected mount is no longer available.";

pub(super) mod construction;
pub(super) mod gaskets;
pub(super) mod mounts;
pub(super) mod openings;
pub(super) mod profiles;

fn remove_item<T>(
    items: Option<&mut Vec<T>>,
    matches: impl FnMut(&T) -> bool,
    unavailable: &str,
) -> Result<T, String> {
    let items = items.ok_or(unavailable)?;
    let index = items.iter().position(matches).ok_or(unavailable)?;
    Ok(items.remove(index))
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
        | MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => {
            return Err("This change is handled by the mechanical edit lifecycle.".into());
        }
        MechanicalChange::SetBatteryEnabled(_)
        | MechanicalChange::SetMethod(_)
        | MechanicalChange::SetBottomStyle(_)
        | MechanicalChange::SetMiddleFrame(_)
        | MechanicalChange::SetIntegratedPlateFrame(_)
        | MechanicalChange::SetDimension { .. }
        | MechanicalChange::SetStabilizerKind(_)
        | MechanicalChange::AddProcessOverride { .. }
        | MechanicalChange::RemoveProcessOverride { .. }
        | MechanicalChange::SetProcessMethod { .. }
        | MechanicalChange::SetProcessMaterial { .. }
        | MechanicalChange::SetProcessThickness { .. } => {
            construction::apply(configuration, change, document, board_id, changed)?
        }
        MechanicalChange::SetClosureInsertPreset(_)
        | MechanicalChange::SetClosureDrive(_)
        | MechanicalChange::SetClosureInstallation(_)
        | MechanicalChange::SetClosureThread(_)
        | MechanicalChange::SetClosureFixedLength(_)
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
        | MechanicalChange::SetMount(_)
        | MechanicalChange::AddMount { .. }
        | MechanicalChange::AddSuggestedClosureMounts(_)
        | MechanicalChange::SetMountCollection { .. }
        | MechanicalChange::AdoptClosurePositions(_)
        | MechanicalChange::SetMountDimension { .. }
        | MechanicalChange::SetMountPosition { .. }
        | MechanicalChange::SetMountKind { .. }
        | MechanicalChange::RemoveMount { .. } => {
            mounts::apply(configuration, change, document, identity_floor, changed)?
        }
        MechanicalChange::AddOpening
        | MechanicalChange::RemoveOpening { .. }
        | MechanicalChange::AddOpeningVertex { .. }
        | MechanicalChange::RemoveOpeningVertex { .. }
        | MechanicalChange::SetOpeningDimension { .. } => {
            openings::apply(configuration, change, identity_floor, changed)?
        }
        MechanicalChange::SetGasketSupportDimension { .. }
        | MechanicalChange::SetGasketSupportPlacement { .. }
        | MechanicalChange::SetGasketSupportUnlinked { .. }
        | MechanicalChange::ResetGasketPlacement => gaskets::apply(configuration, change, changed)?,
        MechanicalChange::AssignSwitchProfile { .. }
        | MechanicalChange::AssignStabilizerProfile { .. }
        | MechanicalChange::AssignImportedGeometryProfile { .. }
        | MechanicalChange::UpdateProfile(_)
        | MechanicalChange::RemoveProfile { .. }
        | MechanicalChange::SetSwitchFamily { .. } => {
            profiles::apply(configuration, change, document, board_id, changed)?
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

/// What one mechanical change does to derived state.
///
/// Process normalization and generated-closure projection classify the same
/// variants along different axes: most settings edits normalize the layer
/// processes without moving generated holes, while geometry edits move holes
/// without touching processes. One table answers both so the two
/// classifications cannot drift apart.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ChangeEffects {
    /// Whether the change normalizes the layer processes afterwards.
    pub(super) normalizes_processes: bool,
    /// Whether the change can create, move or remove generated clearance geometry.
    pub(super) requires_projection: bool,
}

/// Classifies one mechanical change for process normalization and projection.
///
/// Only the clearance dimension changes a generated closure-hole diameter, so
/// it is the one `SetDimension` field that projects; battery, opening and
/// closure-hardware dimensions never do. The pair lifecycle always projects:
/// linking, unlinking and the per-feature localize/omit/rejoin actions move
/// mounts, closure mounts, openings and gasket supports into and out of each
/// half's ordinary configuration, so the generated holes the projection
/// derives can appear, disappear or move.
pub(super) fn change_effects(change: &MechanicalChange) -> ChangeEffects {
    // Geometry that projects but never normalizes: openings, mount
    // repositioning, gasket placement and reset.
    const PROJECTION_ONLY: ChangeEffects = ChangeEffects {
        normalizes_processes: false,
        requires_projection: true,
    };
    // Pure settings that normalize without moving generated holes.
    const NORMALIZE_ONLY: ChangeEffects = ChangeEffects {
        normalizes_processes: true,
        requires_projection: false,
    };
    // Geometry that does both: mounts, closure mounts, profiles and the pair
    // lifecycle (whose normalization arm is unreachable but kept normalizing).
    const BOTH: ChangeEffects = ChangeEffects {
        normalizes_processes: true,
        requires_projection: true,
    };
    // Pure settings that do neither: battery, closure hardware, critical fits,
    // hardware items and stabilizers.
    const NEITHER: ChangeEffects = ChangeEffects {
        normalizes_processes: false,
        requires_projection: false,
    };
    match change {
        MechanicalChange::AddOpening
        | MechanicalChange::RemoveOpening { .. }
        | MechanicalChange::AddOpeningVertex { .. }
        | MechanicalChange::RemoveOpeningVertex { .. }
        | MechanicalChange::SetOpeningDimension { .. }
        | MechanicalChange::SetMountPosition { .. }
        | MechanicalChange::SetGasketSupportPlacement { .. }
        | MechanicalChange::ResetGasketPlacement => PROJECTION_ONLY,
        MechanicalChange::SetMethod(_)
        | MechanicalChange::SetMount(_)
        | MechanicalChange::SetBottomStyle(_)
        | MechanicalChange::SetMiddleFrame(_)
        | MechanicalChange::SetIntegratedPlateFrame(_)
        | MechanicalChange::SetGasketSupportDimension { .. }
        | MechanicalChange::SetGasketSupportUnlinked { .. }
        | MechanicalChange::AddProcessOverride { .. }
        | MechanicalChange::RemoveProcessOverride { .. }
        | MechanicalChange::SetProcessMethod { .. }
        | MechanicalChange::SetProcessMaterial { .. }
        | MechanicalChange::SetProcessThickness { .. } => NORMALIZE_ONLY,
        MechanicalChange::Enable
        | MechanicalChange::Disable
        | MechanicalChange::InitializeClosures
        | MechanicalChange::AddMount { .. }
        | MechanicalChange::AddSuggestedClosureMounts(_)
        | MechanicalChange::SetMountCollection { .. }
        | MechanicalChange::AdoptClosurePositions(_)
        | MechanicalChange::SetMountDimension { .. }
        | MechanicalChange::SetMountKind { .. }
        | MechanicalChange::RemoveMount { .. }
        | MechanicalChange::AssignSwitchProfile { .. }
        | MechanicalChange::AssignStabilizerProfile { .. }
        | MechanicalChange::AssignImportedGeometryProfile { .. }
        | MechanicalChange::UpdateProfile(_)
        | MechanicalChange::RemoveProfile { .. }
        | MechanicalChange::SetSwitchFamily { .. }
        | MechanicalChange::LinkMirrorPair { .. }
        | MechanicalChange::UnlinkMirrorPair { .. }
        | MechanicalChange::RemovePhysicalInstance
        | MechanicalChange::MakeFeatureLocal { .. }
        | MechanicalChange::OmitFeatureMember { .. }
        | MechanicalChange::RejoinFeatureMember { .. } => BOTH,
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
        | MechanicalChange::SetStabilizerKind(_) => NEITHER,
        MechanicalChange::SetDimension { field, .. } => {
            let skip = matches!(
                field,
                MechanicalDimension::BatteryWidth
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
                    | MechanicalDimension::ClosureBearingThickness
            );
            ChangeEffects {
                normalizes_processes: !skip,
                // Only the clearance changes a generated closure-hole diameter.
                requires_projection: matches!(field, MechanicalDimension::Clearance),
            }
        }
    }
}

pub(super) fn skip_process_normalization(change: &MechanicalChange) -> bool {
    !change_effects(change).normalizes_processes
}
