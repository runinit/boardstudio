//! Stateless mechanical assembly resolution. Explicit profiles are the geometry authority.
use crate::model::*;
use boardstudio_contracts::polygon_contains;
pub use boardstudio_model::mechanical::change;
pub mod clearance;
pub mod edit;
pub mod features;
mod gasket;
pub mod halves;
pub mod inputs;
pub use boardstudio_model::mechanical::pair;
pub use change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
pub use edit::{MechanicalEditDisposition, evaluate_edit};
pub use inputs::{MechanicalInputs, effective_inputs};

mod construction;
mod foam;
mod part_geometry;
mod stack;

use construction::{ConstructionGeometry, construct_bodies, finalize_assembly};
use foam::{add_foam_clearance, fallback_foam_geometry, subtract_foam_exclusions};
use part_geometry::{BatterySpace, PartGeometry, collect_part_geometry, resolve_battery_space};
use stack::{build_stack, dimensions_are_valid, resolve_foam_contours, resolve_profiles};

type FloatPath = Vec<[f64; 2]>;
type FloatShapes = Vec<Vec<FloatPath>>;

fn profile_source_for_family(family: MechanicalSwitchFamily) -> MechanicalBuiltinProfile {
    match family {
        MechanicalSwitchFamily::Mx => MechanicalBuiltinProfile::MxSwitch,
        MechanicalSwitchFamily::ChocV1 => MechanicalBuiltinProfile::ChocV1Switch,
        MechanicalSwitchFamily::ChocV2 => MechanicalBuiltinProfile::ChocV2Switch,
    }
}

fn switch_mounting_datum(family: MechanicalSwitchFamily) -> f64 {
    match family {
        MechanicalSwitchFamily::ChocV1 => 3.5,
        MechanicalSwitchFamily::Mx | MechanicalSwitchFamily::ChocV2 => 5.0,
    }
}

fn inferred_switch_family(
    part: &Part,
    definition: &PartDefinition,
) -> Option<MechanicalSwitchFamily> {
    generator_switch_family(definition, part.generator_parameters.as_ref())
}

/// The switch family a definition's generator implies; a part's own generator parameters
/// replace the definition's for the same key. A definition seen without a part passes `None`.
pub fn generator_switch_family(
    definition: &PartDefinition,
    part_parameters: Option<&std::collections::BTreeMap<String, serde_json::Value>>,
) -> Option<MechanicalSwitchFamily> {
    let source = definition
        .generator
        .as_ref()
        .map(|generator| generator.source.to_ascii_lowercase())
        .unwrap_or_default();
    if source == "ceoloide/switch_mx" {
        return Some(MechanicalSwitchFamily::Mx);
    }
    if !source.ends_with("/switch_choc_v1_v2") {
        return None;
    }
    let flag = |key: &str| {
        part_parameters
            .and_then(|values| values.get(key))
            .or_else(|| definition.generator.as_ref()?.parameters.get(key))
            .and_then(|value| {
                value.as_bool().or_else(|| {
                    value
                        .as_object()
                        .and_then(|object| object.get("value"))
                        .and_then(serde_json::Value::as_bool)
                })
            })
    };
    let v1 = flag("choc_v1_support").unwrap_or(true);
    let v2 = flag("choc_v2_support").unwrap_or(true);
    match (v1, v2) {
        (true, false) => Some(MechanicalSwitchFamily::ChocV1),
        (false, true) => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
    }
}

/// The family on the profile assigned to the part's definition.
fn assigned_family(
    config: &MechanicalConfiguration,
    part: &Part,
) -> Option<MechanicalSwitchFamily> {
    config
        .profiles
        .iter()
        .find(|profile| profile.definition_id == part.definition_id)
        .and_then(|profile| profile.switch_family)
}

/// The assigned family, else the one the definition's generator implies.
fn effective_family(
    config: &MechanicalConfiguration,
    part: &Part,
    definition: &PartDefinition,
) -> Option<MechanicalSwitchFamily> {
    assigned_family(config, part).or_else(|| inferred_switch_family(part, definition))
}

/// Earlier saved catalogue profiles stored the MX mounting datum as a gap. Recognize only the
/// exact reviewed sources and original value; custom profiles and user-specified engagement
/// distances retain their meaning.
fn is_legacy_mx_stabilizer(profile: &MechanicalPartProfile) -> bool {
    profile.plate_to_pcb == 5.0
        && profile.source_geometry.as_ref().is_some_and(|source| {
            source.text == include_str!("../../../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod")
                || source.text
                    == include_str!("../../../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod")
        })
}

fn is_switch_position(part: &Part, definition: &PartDefinition) -> bool {
    matches!(definition.kind, PartKind::Switch)
        || inferred_switch_family(part, definition).is_some()
        || definition
            .generator
            .as_ref()
            .is_some_and(|g| g.source.ends_with("/switch_choc_v1_v2"))
}

pub fn default_material(part_id: &str, method: &PlateMethod) -> &'static str {
    if part_id.ends_with("foam") {
        return "EVA";
    }
    match method {
        PlateMethod::Printed => "PLA",
        PlateMethod::Cnc => "Aluminium",
        PlateMethod::CutSheet => "Acrylic",
        PlateMethod::PcbFr4 => "FR-4",
    }
}

pub fn default_layer_process(
    part_id: &str,
    method: PlateMethod,
    thickness: f64,
) -> MechanicalPartProcess {
    let method = if part_id.ends_with("foam") {
        PlateMethod::CutSheet
    } else {
        method
    };
    MechanicalPartProcess {
        part_id: part_id.into(),
        material: default_material(part_id, &method).into(),
        method,
        thickness,
        constraints_version: "2026-09-24".into(),
    }
}

/// The plate-to-PCB mounting datum baseline for a switch family.
pub fn mounting_datum(family: MechanicalSwitchFamily) -> f64 {
    switch_mounting_datum(family)
}

/// The default plate thickness for a switch family.
pub fn default_plate_thickness(family: MechanicalSwitchFamily) -> f64 {
    if family == MechanicalSwitchFamily::ChocV1 {
        1.3
    } else {
        1.5
    }
}

/// The default plate-foam thickness for a plate-to-PCB gap.
pub fn default_plate_foam_thickness(gap: f64) -> f64 {
    (((gap - 0.2).min(3.0) + 0.000001) * 10.0).floor().max(0.0) / 10.0
}

/// The default gasket layout used when gasket mounting is first selected.
pub fn default_gasket_layout() -> MechanicalGasketLayout {
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

/// The default internal gasket (internal closure) configuration.
pub fn default_internal_gasket() -> InternalGasketConfiguration {
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

fn material_is_valid(part_id: &str, method: &PlateMethod, material: &str) -> bool {
    if part_id.ends_with("foam") {
        return material == "EVA";
    }
    match method {
        PlateMethod::Printed => matches!(material, "PLA" | "ABS"),
        PlateMethod::Cnc => material == "Aluminium",
        PlateMethod::CutSheet => material == "Acrylic",
        PlateMethod::PcbFr4 => material == "FR-4",
    }
}

fn normalized_processes(config: &MechanicalConfiguration) -> Vec<MechanicalPartProcess> {
    let mut processes = config.part_processes.clone().unwrap_or_default();
    for (part_id, thickness) in [
        ("plate", config.plate_thickness),
        ("plate-foam", config.plate_foam_thickness),
        ("bottom-foam", config.bottom_foam_thickness),
        ("bottom", config.bottom_thickness),
    ] {
        if !processes.iter().any(|process| process.part_id == part_id) {
            processes.push(default_layer_process(
                part_id,
                config.method.clone(),
                thickness,
            ));
        }
    }
    for process in &mut processes {
        if process.part_id.ends_with("foam") {
            process.method = PlateMethod::CutSheet;
        } else if process.part_id == "plate" {
            process.method = config.method.clone();
        }
        if !material_is_valid(&process.part_id, &process.method, &process.material) {
            process.material = default_material(&process.part_id, &process.method).into();
        }
        if process.constraints_version.trim().is_empty() {
            process.constraints_version = "2026-09-24".into();
        }
        let stack_thickness = match process.part_id.as_str() {
            "plate" => Some(config.plate_thickness),
            "plate-foam" => Some(config.plate_foam_thickness),
            "bottom-foam" => Some(config.bottom_foam_thickness),
            "bottom" => Some(config.bottom_thickness),
            _ => None,
        };
        if let Some(thickness) = stack_thickness {
            process.thickness = thickness;
        } else if process.thickness < 0.0 {
            process.thickness = 1.0;
        }
    }
    processes
}

fn transform_part_points(part: &Part, points: &[Vec2]) -> Vec<Vec2> {
    let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
    points
        .iter()
        .map(|point| {
            let x = if part.side == Side::Back {
                -point.x
            } else {
                point.x
            };
            Vec2 {
                x: part.pose.at.x + x * cos - point.y * sin,
                y: part.pose.at.y + x * sin + point.y * cos,
            }
        })
        .collect()
}

pub fn resolve(document: &ProjectDoc, contours: &[Contour]) -> MechanicalAssembly {
    let mut result = MechanicalAssembly {
        finding_markers: vec![],
        modules: vec![],
        generated_materials: vec![],
        gasket_supports: vec![],
        gasket_tracks: vec![],
        generated_hardware: vec![],
        pcb_reference: None,
        suggested_mounts: vec![],
        nominal_plate_contours: vec![],
        revision: document.revision,
        plate_contours: contours.to_vec(),
        case: CaseAssemblyIR {
            revision: document.revision,
            bodies: vec![],
        },
        stack: vec![],
        diagnostics: vec![],
        generation_blocked: false,
    };
    let Some(config) = &document.mechanical else {
        return result;
    };
    let profile_stage = resolve_profiles(document, config);
    let (effective_config, incompatible_switches) = profile_stage
        .map(|stage| (stage.config, stage.incompatible_switches))
        .unwrap_or_else(|| (config.clone(), Vec::new()));
    let config = &effective_config;
    let mut frame_contours = contours.to_vec();
    if config.wall_thickness.is_finite()
        && config.clearance.is_finite()
        && config.wall_thickness > 0.0
        && config.clearance >= 0.0
    {
        match expanded_outline(
            contours,
            config.wall_thickness + config.clearance,
            document.revision,
        ) {
            Ok(expanded) => {
                frame_contours = expanded;
                if config.internal_gasket.is_none() || config.mount != MechanicalMount::Gasket {
                    result.plate_contours = frame_contours.clone();
                }
            }
            Err(message) => {
                result.generation_blocked = true;
                result.diagnostics.push(Finding {
                    id: "mechanical:frame-outline".into(),
                    severity: Severity::Error,
                    scope: Scope::Case,
                    message,
                    target_ids: vec![],
                });
            }
        }
    }
    let mut issue = |id: &str, severity: Severity, message: &str, targets: Vec<String>| {
        if severity == Severity::Error {
            result.generation_blocked = true;
        }
        result.diagnostics.push(Finding {
            id: format!("mechanical:{id}"),
            severity,
            scope: Scope::Case,
            message: message.into(),
            target_ids: targets,
        });
    };
    if !incompatible_switches.is_empty() {
        issue(
            "switch-family",
            Severity::Error,
            "Switches with different mounting heights cannot share one plate; split the assembly or choose compatible switch fits.",
            incompatible_switches,
        );
    }
    if !dimensions_are_valid(config) {
        issue(
            "dimensions",
            Severity::Error,
            "Assembly thicknesses must be positive and clearances nonnegative finite millimetres.",
            vec![],
        );
        return result;
    }
    let pcb_bottom = -config.pcb_thickness;
    let bottom_foam_z = pcb_bottom - config.bottom_foam_thickness;
    if contours.iter().any(|c| {
        c.points.len() < 3
            || c.points
                .iter()
                .any(|p| !p.x.is_finite() || !p.y.is_finite())
    }) {
        issue(
            "contours",
            Severity::Error,
            "Assembly contours must contain finite closed polygons.",
            vec![],
        );
        return result;
    }
    if config
        .mounts
        .iter()
        .chain(config.closure_mounts.iter().flatten())
        .any(|m| {
            !m.at.x.is_finite()
                || !m.at.y.is_finite()
                || !m.hole_diameter.is_finite()
                || m.hole_diameter <= 0.0
        })
    {
        issue(
            "mount-geometry",
            Severity::Error,
            "Mount positions and diameters must be finite and diameters positive.",
            vec![],
        );
        return result;
    }
    if contours.is_empty() {
        issue(
            "outline",
            Severity::Error,
            "An assembly requires a resolved board outline.",
            vec![],
        );
    }
    if config.plate_foam_thickness > config.plate_to_pcb {
        issue(
            "foam-engagement",
            Severity::Error,
            "Plate foam exceeds the plate-to-PCB clearance.",
            vec!["plate-foam".into()],
        );
    }
    if config.integrated_plate_frame && config.method == PlateMethod::PcbFr4 {
        issue(
            "fr4-integrated-frame",
            Severity::Error,
            "A planar PCB-FR4 plate cannot encode an integrated case rim.",
            vec!["plate".into()],
        );
    }
    if config.integrated_plate_frame && config.mount == MechanicalMount::Gasket {
        issue(
            "gasket-rigid-frame",
            Severity::Error,
            "A gasket plate must remain mechanically separate from the rigid frame.",
            vec!["plate".into()],
        );
    }
    if config.mount == MechanicalMount::Gasket
        && !config.gasket_travel.is_some_and(|travel| {
            travel.is_finite() && travel > 0.0 && travel < config.plate_to_pcb
        })
    {
        issue(
            "gasket-travel",
            Severity::Error,
            "Gasket mounting requires positive travel smaller than the plate-to-PCB clearance, keeping the plate clear of the rigid rim.",
            vec!["plate".into()],
        );
    }

    if config.mounts.is_empty() && config.mount != MechanicalMount::Gasket {
        issue(
            "mounts",
            Severity::Warning,
            "No mounting points are configured; choose supported mounting locations before manufacturing.",
            vec![],
        );
    }
    if config.method == PlateMethod::PcbFr4 {
        issue(
            "fr4-routing",
            Severity::Warning,
            "Sharp internal rectangular corners require a reviewed routing radius; PCB process capability is not implied by a valid plate outline.",
            vec!["plate".into()],
        );
    }
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == config.board_id)
    else {
        issue(
            "board",
            Severity::Error,
            "Select an existing board for the mechanical assembly.",
            vec![],
        );
        return result;
    };
    let mut seen_stabilizers = std::collections::BTreeSet::new();
    for stabilizer in config.stabilizers.iter().flatten() {
        if !seen_stabilizers.insert(&stabilizer.part_id)
            || !board.part_ids.contains(&stabilizer.part_id)
            || !document.parts.iter().any(|p| p.id == stabilizer.part_id)
            || !stabilizer.units.is_finite()
            || stabilizer.units <= 0.0
            || stabilizer.rotation.is_some_and(|v| !v.is_finite())
        {
            issue(
                &format!("stabilizer-override:{}", stabilizer.part_id),
                Severity::Error,
                "Stabilizer overrides require a unique existing board part, positive finite units and finite rotation.",
                vec![stabilizer.part_id.clone()],
            );
        }
    }
    let PartGeometry {
        pcb_reference_contours,
        mut component_volumes,
        mut profile_openings,
        plate_foam_clearances,
        bottom_foam_clearances,
    } = collect_part_geometry(
        document,
        config,
        board,
        contours,
        &mut result.plate_contours,
        &mut issue,
    );
    let module_resolution = crate::modules::resolve(document, &config.board_id);
    for finding in crate::modules::embedded_findings(document, &config.board_id)
        .iter()
        .filter(|finding| finding.scope == Scope::Case)
    {
        issue(
            &finding.id,
            finding.severity.clone(),
            &finding.message,
            finding.target_ids.clone(),
        );
    }
    for part in document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
    {
        if let Some(definition) = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
        {
            for gate in definition
                .hardware_profile
                .iter()
                .flat_map(|profile| &profile.gates)
                .filter(|gate| gate.output == HardwareOutput::Mechanical)
            {
                issue(
                    &format!("hardware/{}/{}", part.id, gate.code),
                    Severity::Error,
                    &gate.message,
                    vec![part.id.clone()],
                );
            }
        }
    }
    let mut module_floor = f64::INFINITY;
    for finding in module_resolution
        .findings
        .iter()
        .filter(|f| f.scope == Scope::Case)
    {
        issue(
            &finding.id,
            finding.severity.clone(),
            &finding.message,
            finding.target_ids.clone(),
        );
    }
    for module in &module_resolution.modules {
        let Some(instance) = document.modules.iter().find(|m| m.id == module.id) else {
            continue;
        };
        let travel = if instance.attachment == ModuleAttachment::Board
            && config.mount == MechanicalMount::Gasket
        {
            config.gasket_travel.unwrap_or(0.0)
        } else {
            0.0
        };
        for support in crate::modules::resolved_mount_supports(instance, module) {
            module_floor = module_floor.min(support.z);
        }
        for volume in module
            .board
            .iter()
            .chain(module.volumes.iter().map(|v| &v.geometry))
        {
            let mut swept = volume.clone();
            swept.z -= travel + instance.service_clearance;
            swept.height += 2.0 * (travel + instance.service_clearance);
            module_floor = module_floor.min(swept.z);
            component_volumes.push((module.id.clone(), swept));
        }
        profile_openings.extend(module.openings.iter().map(|v| v.geometry.clone()));
        if module
            .volumes
            .iter()
            .chain(&module.openings)
            .any(|v| !v.qualified)
        {
            issue(
                &format!("module/{}/unqualified-volume", module.id),
                Severity::Error,
                "Module occupancy or access geometry requires qualification before exact case output.",
                vec![module.id.clone()],
            );
        }
    }
    let Some(BatterySpace {
        height: battery_height,
        bottom_foam_contours,
    }) = resolve_battery_space(config, contours, &mut issue)
    else {
        return result;
    };
    // PCB top is the stable zero plane; layers below it have negative Z.
    let battery_z = if config.battery.is_some() {
        pcb_bottom - battery_height
    } else {
        bottom_foam_z - battery_height
    };
    let bottom_z = bottom_foam_z.min(battery_z).min(module_floor) - config.bottom_thickness;
    result.stack.extend(build_stack(
        config,
        pcb_bottom,
        bottom_foam_z,
        battery_z,
        battery_height,
        bottom_z,
    ));
    for (part_id, volume) in &component_volumes {
        if let Some(battery) = &config.battery {
            let hx = battery.size.x / 2.0;
            let hy = battery.size.y / 2.0;
            let min_x = volume
                .points
                .iter()
                .map(|p| p.x)
                .fold(f64::INFINITY, f64::min);
            let max_x = volume
                .points
                .iter()
                .map(|p| p.x)
                .fold(f64::NEG_INFINITY, f64::max);
            let min_y = volume
                .points
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min);
            let max_y = volume
                .points
                .iter()
                .map(|p| p.y)
                .fold(f64::NEG_INFINITY, f64::max);
            if min_x < battery.at.x + hx
                && max_x > battery.at.x - hx
                && min_y < battery.at.y + hy
                && max_y > battery.at.y - hy
                && volume.z < bottom_foam_z
                && volume.z + volume.height > battery_z
            {
                issue(
                    &format!("component-battery:{part_id}"),
                    Severity::Error,
                    "Component clearance envelope intersects the battery space.",
                    vec![part_id.clone(), "battery".into()],
                );
            }
        }
        if volume.z < config.plate_to_pcb + config.plate_thickness
            && volume.z + volume.height > config.plate_to_pcb
        {
            issue(
                &format!("component-plate:{part_id}"),
                Severity::Warning,
                "Component clearance reaches the plate layer; review the profile aperture and connector access.",
                vec![part_id.clone(), "plate".into()],
            );
        }
    }
    result.pcb_reference = Some(CaseIR {
        revision: document.revision,
        contours: pcb_reference_contours,
        body: CaseBody {
            features: None,
            id: "pcb-reference".into(),
            name: "PCB reference".into(),
            board_id: config.board_id.clone(),
            kind: CaseKind::Plate,
            thickness: config.pcb_thickness,
            clearance: 0.0,
            material_id: None,
            z: Some(pcb_bottom),
            wall_height: None,
            wall_thickness: None,
            mounts: None,
            pcb_source: None,
            gasket: None,
            openings: None,
        },
    });
    let support_height = if config.mount == MechanicalMount::Tray {
        pcb_bottom - battery_z
    } else {
        config.plate_to_pcb - battery_z
    };
    let mut suspension_bosses = Vec::new();
    if config.mount != MechanicalMount::Gasket {
        for mount in &config.mounts {
            if mount.boss_diameter.is_none() {
                issue(
                    &format!("support-diameter:{}", mount.id),
                    Severity::Error,
                    "Suspension mounting requires an explicit boss diameter for the mating support.",
                    vec![mount.id.clone()],
                );
            } else {
                let mut support = mount.clone();
                support.kind = MountKind::Boss;
                support.height = Some(support_height);
                suspension_bosses.push(support);
            }
        }
        if config.mount == MechanicalMount::Tray && !config.mounts.is_empty() {
            issue(
                "pcb-mount-review",
                Severity::Warning,
                "Tray support positions require matching PCB mounting holes; mechanical configuration does not modify the electrical board.",
                config.mounts.iter().map(|m| m.id.clone()).collect(),
            );
        }
    }
    let plate_foam_contours =
        match resolve_foam_contours(&result.plate_contours, &plate_foam_clearances) {
            Ok(contours) => contours,
            Err(message) => {
                issue(
                    "plate-foam-clearance",
                    Severity::Error,
                    &message,
                    vec!["plate-foam".into()],
                );
                result.plate_contours.clone()
            }
        };
    let bottom_foam_contours =
        match resolve_foam_contours(&bottom_foam_contours, &bottom_foam_clearances) {
            Ok(contours) => contours,
            Err(message) => {
                issue(
                    "bottom-foam-clearance",
                    Severity::Error,
                    &message,
                    vec!["bottom-foam".into()],
                );
                bottom_foam_contours
            }
        };
    let constructed = construct_bodies(
        document.revision,
        config,
        ConstructionGeometry {
            plate_contours: result.plate_contours.clone(),
            frame_contours,
            plate_foam_contours,
            bottom_foam_contours,
            profile_openings,
            suspension_bosses,
            bottom_z,
            bottom_foam_z,
            battery_z,
            support_height,
        },
        &mut issue,
    );
    result.case.bodies = constructed.bodies;
    let (support_findings, support_markers) = crate::modules::attach_case_supports(
        document,
        &config.board_id,
        &module_resolution.modules,
        &mut result.case.bodies,
    );
    result.generation_blocked |= !support_findings.is_empty();
    result.diagnostics.extend(support_findings);
    result.finding_markers.extend(support_markers);
    let (board_support_findings, board_support_markers) = crate::modules::attach_board_supports(
        document,
        &config.board_id,
        &module_resolution.modules,
        result.pcb_reference.as_mut(),
        &mut result.case.bodies,
    );
    result.generation_blocked |= board_support_findings
        .iter()
        .any(|finding| matches!(&finding.severity, Severity::Error));
    result.diagnostics.extend(board_support_findings);
    result.finding_markers.extend(board_support_markers);
    // Functional access applies to every material layer it intersects, using the
    // same prepared module geometry as preview and exact output.
    for body in &mut result.case.bodies {
        for opening in module_resolution.modules.iter().flat_map(|m| &m.openings) {
            let list = body.body.openings.get_or_insert_with(Vec::new);
            if !list.contains(&opening.geometry) {
                list.push(opening.geometry.clone());
            }
        }
    }
    result.modules = module_resolution.modules;
    result.stack.extend(constructed.stack);
    finalize_assembly(document, config, &mut result, &component_volumes, contours);
    result
}

pub fn expanded_outline(
    contours: &[Contour],
    amount: f64,
    revision: u64,
) -> Result<Vec<Contour>, String> {
    let body = CaseBody {
        features: None,
        id: "mechanical-frame-outline".into(),
        name: "Frame outline".into(),
        board_id: String::new(),
        kind: CaseKind::Plate,
        thickness: 1.0,
        clearance: amount,
        material_id: None,
        z: None,
        wall_height: None,
        wall_thickness: None,
        mounts: None,
        pcb_source: None,
        gasket: None,
        openings: None,
    };
    let ir = CaseAssemblyIR {
        revision,
        bodies: vec![CaseIR {
            revision,
            body,
            contours: contours.iter().filter(|c| !c.hole).cloned().collect(),
        }],
    };
    let prepared = crate::case::prepare(&ir)?;
    let mut expanded: Vec<Contour> = prepared.bodies[0]
        .regions
        .iter()
        .map(|region| Contour {
            points: region.outer.clone(),
            hole: false,
        })
        .collect();
    expanded.extend(contours.iter().filter(|c| c.hole).cloned());
    Ok(expanded)
}

fn convex_envelope(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    let turn = |a: Vec2, b: Vec2, c: Vec2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut lower: Vec<Vec2> = Vec::new();
    for point in points.iter().copied() {
        while lower.len() >= 2 && turn(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for point in points.into_iter().rev() {
        while upper.len() >= 2 && turn(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn same_ring(a: &Contour, b: &Contour) -> bool {
    if a.hole != b.hole {
        return false;
    }
    let clean = |ring: &Contour| {
        let mut points = ring.points.clone();
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        points
    };
    let a = clean(a);
    let b = clean(b);
    if a.len() != b.len() || a.is_empty() {
        return false;
    }
    let equal = |p: Vec2, q: Vec2| (p.x - q.x).abs() < 0.000001 && (p.y - q.y).abs() < 0.000001;
    (0..b.len()).any(|offset| {
        (0..a.len()).all(|i| equal(a[i], b[(offset + i) % b.len()]))
            || (0..a.len()).all(|i| equal(a[i], b[(offset + b.len() - i) % b.len()]))
    })
}

pub fn inside_outline(point: Vec2, contours: &[Contour]) -> bool {
    let contains = |ring: &Contour| polygon_contains(&ring.points, point);
    contours.iter().any(|c| !c.hole && contains(c))
        && !contours.iter().any(|c| c.hole && contains(c))
}

/// Reviewed library fit geometry with a manufacturer-sourced switch datum.
pub fn builtin_profile(
    definition_id: String,
    source: MechanicalBuiltinProfile,
    plate_to_pcb: f64,
) -> Result<MechanicalPartProfile, String> {
    use boardstudio_core_kicad_source::mechanical_extract::{
        MechanicalPurpose, PurposeMapping, extract, pcb_mounting_holes, plate_cutout_contours,
        profile_source,
    };
    if !plate_to_pcb.is_finite() || plate_to_pcb <= 0.0 {
        return Err("Profile engagement must be positive finite millimetres".into());
    }
    let (name, source_text, family, square_cutout) = match source {
        MechanicalBuiltinProfile::MxSwitch => (
            "SW_MX_1u",
            None,
            Some(MechanicalSwitchFamily::Mx),
            Some((14.0, 1.5, Vec2 { x: 1.4, y: 1.6 })),
        ),
        MechanicalBuiltinProfile::ChocV1Switch => (
            "Kailh PG1350 Choc v1",
            None,
            Some(MechanicalSwitchFamily::ChocV1),
            Some((14.0, 1.3, Vec2 { x: 1.2, y: 1.4 })),
        ),
        MechanicalBuiltinProfile::ChocV2Switch => (
            "Kailh PG1353 Choc v2",
            None,
            Some(MechanicalSwitchFamily::ChocV2),
            Some((14.0, 1.5, Vec2 { x: 1.4, y: 1.6 })),
        ),
        MechanicalBuiltinProfile::MxStab2u => (
            "STAB_MX_2u",
            Some(include_str!(
                "../../../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod"
            )),
            None,
            None,
        ),
        MechanicalBuiltinProfile::MxStab625u => (
            "STAB_MX_6.25u",
            Some(include_str!(
                "../../../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod"
            )),
            None,
            None,
        ),
    };
    let is_library_geometry = source_text.is_some();
    let mut source_geometry = None;
    let mut pcb_holes = None;
    let cutouts = if let Some(source_text) = source_text {
        let raw = extract(source_text, &[]).map_err(|e| format!("{e:?}"))?;
        let mappings: Vec<_> = raw
            .primitives
            .iter()
            .filter_map(|primitive| {
                let purpose = if primitive.layer.as_deref() == Some("Eco2.User") {
                    MechanicalPurpose::PlateCutout
                } else if matches!(
                    &primitive.geometry,
                    MechanicalShape::Drill {
                        plated: Some(false),
                        ..
                    }
                ) {
                    MechanicalPurpose::ElectricalPcbMountingHole
                } else {
                    return None;
                };
                Some(PurposeMapping {
                    source_id: Some(primitive.id.clone()),
                    kind: None,
                    layer: None,
                    purpose,
                })
            })
            .collect();
        let geometry = extract(source_text, &mappings).map_err(|e| format!("{e:?}"))?;
        source_geometry =
            Some(profile_source(source_text, &mappings).map_err(|e| format!("{e:?}"))?);
        pcb_holes = Some(pcb_mounting_holes(&geometry).map_err(|e| format!("{e:?}"))?);
        plate_cutout_contours(&geometry, 0.005).map_err(|e| format!("{e:?}"))?
    } else if let Some((width, _, _)) = square_cutout {
        let half = width / 2.0;
        vec![vec![
            Vec2 { x: -half, y: -half },
            Vec2 { x: half, y: -half },
            Vec2 { x: half, y: half },
            Vec2 { x: -half, y: half },
        ]]
    } else {
        Vec::new()
    };
    if cutouts.is_empty() {
        return Err("Bundled profile has no mapped plate contours".into());
    }
    let supported_thickness = if let Some((_, _, range)) = square_cutout {
        Some(range)
    } else if family == Some(MechanicalSwitchFamily::Mx) {
        Some(Vec2 { x: 1.4, y: 1.6 })
    } else {
        None
    };
    let manufacturer_source = match family {
        Some(MechanicalSwitchFamily::Mx) => {
            "Cherry MX datasheet: https://datasheet.octopart.com/MX1A-11NW-Cherry-datasheet-34676.pdf; 5.0 mm nominal mounting datum; 1.5 mm nominal plate"
        }
        Some(MechanicalSwitchFamily::ChocV1) => {
            "Kailh PG1350 drawing CPG135001D01-16: https://m.kailhswitch.com/Content/upload/pdf/202215927/CPG135001D01-16.pdf?rnd=121; 3.5 mm mounting datum; 1.3 mm nominal plate"
        }
        Some(MechanicalSwitchFamily::ChocV2) => {
            "Kailh PG1353 drawing CPG135301D03: https://www.kailhswitch.com/Content/upload/pdf/202015927/PG135301D03.pdf; 5.0 mm mounting datum; 1.5 mm nominal plate"
        }
        None => "",
    };
    Ok(MechanicalPartProfile {
        source_geometry,
        pcb_holes,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        switch_family: family,
        definition_id,
        source: if is_library_geometry {
            format!(
                "marbastlib@6b0a9a73f579e377816d60b58eac2b3252de7868:footprints/marbastlib-mx.pretty/{name}.kicad_mod:CERN-OHL-P-2.0:Eco2.User; chord deviation <=0.005mm; {manufacturer_source}; PCB-mount stabilizers only"
            )
        } else {
            format!("{manufacturer_source}; Board Studio standard 14 x 14 mm plate opening")
        },
        cutouts,
        plate_to_pcb,
        supported_thickness,
    })
}

#[cfg(test)]
mod tests;
