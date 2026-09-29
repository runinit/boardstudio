//! Stateless mechanical assembly resolution. Explicit profiles are the geometry authority.
use crate::model::*;
mod gasket;
use i_overlay::core::{fill_rule::FillRule, overlay_rule::OverlayRule};
use i_overlay::float::single::SingleFloatOverlay;

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
        part.generator_parameters
            .as_ref()
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

fn is_switch_position(part: &Part, definition: &PartDefinition) -> bool {
    matches!(definition.kind, PartKind::Switch)
        || inferred_switch_family(part, definition).is_some()
        || definition
            .generator
            .as_ref()
            .is_some_and(|g| g.source.ends_with("/switch_choc_v1_v2"))
}

fn default_material(part_id: &str, method: &PlateMethod) -> &'static str {
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

fn default_layer_process(
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

fn fallback_foam_geometry(
    part: &Part,
    definition: &PartDefinition,
) -> Option<(Vec<Vec2>, &'static str)> {
    if definition.courtyard.len() >= 3 {
        return Some((definition.courtyard.clone(), "footprint courtyard"));
    }

    let mut pad_corners = Vec::new();
    for pad in &definition.pads {
        if !pad.at.x.is_finite()
            || !pad.at.y.is_finite()
            || !pad.size.x.is_finite()
            || !pad.size.y.is_finite()
            || pad.size.x <= 0.0
            || pad.size.y <= 0.0
        {
            continue;
        }
        let (sin, cos) = pad.rotation.unwrap_or(0.0).to_radians().sin_cos();
        for (x, y) in [
            (-pad.size.x / 2.0, -pad.size.y / 2.0),
            (pad.size.x / 2.0, -pad.size.y / 2.0),
            (pad.size.x / 2.0, pad.size.y / 2.0),
            (-pad.size.x / 2.0, pad.size.y / 2.0),
        ] {
            pad_corners.push(Vec2 {
                x: pad.at.x + x * cos - y * sin,
                y: pad.at.y + x * sin + y * cos,
            });
        }
    }
    if pad_corners.len() >= 3 {
        return Some((convex_envelope(pad_corners), "conservative pad bounds"));
    }

    let keycap = part.keycap.or(definition.keycap)?;
    if !keycap.x.is_finite() || !keycap.y.is_finite() || keycap.x <= 0.0 || keycap.y <= 0.0 {
        return None;
    }
    Some((
        vec![
            Vec2 {
                x: -keycap.x / 2.0,
                y: -keycap.y / 2.0,
            },
            Vec2 {
                x: keycap.x / 2.0,
                y: -keycap.y / 2.0,
            },
            Vec2 {
                x: keycap.x / 2.0,
                y: keycap.y / 2.0,
            },
            Vec2 {
                x: -keycap.x / 2.0,
                y: keycap.y / 2.0,
            },
        ],
        "keycap bounds",
    ))
}

fn expanded_clearance(points: &[Vec2], amount: f64, revision: u64) -> Result<Vec<Vec2>, String> {
    if points.len() < 3
        || points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err("Foam clearance needs a finite closed polygon".into());
    }
    if amount == 0.0 {
        return Ok(points.to_vec());
    }
    let expanded = expanded_outline(
        &[Contour {
            hole: false,
            points: points.to_vec(),
        }],
        amount,
        revision,
    )?;
    expanded
        .into_iter()
        .find(|contour| !contour.hole)
        .map(|contour| contour.points)
        .ok_or_else(|| "Foam clearance collapsed while applying its margin".into())
}

fn add_foam_clearance(
    target: &mut Vec<Contour>,
    part: &Part,
    points: &[Vec2],
    margin: f64,
    revision: u64,
) -> Result<(), String> {
    let expanded = expanded_clearance(points, margin, revision)?;
    target.push(Contour {
        hole: true,
        points: transform_part_points(part, &expanded),
    });
    Ok(())
}

fn subtract_foam_exclusions(
    base: &[Contour],
    exclusions: &[Contour],
) -> Result<Vec<Contour>, String> {
    let mut solids: FloatShapes = Vec::new();
    let mut cuts: FloatShapes = Vec::new();
    let path = |contour: &Contour| -> Result<FloatPath, String> {
        if contour.points.len() < 3
            || contour.points.iter().any(|point| {
                !point.x.is_finite()
                    || !point.y.is_finite()
                    || point.x.abs() > 1_000_000.0
                    || point.y.abs() > 1_000_000.0
            })
        {
            return Err("Foam contours must contain finite, in-range polygons".into());
        }
        Ok(contour
            .points
            .iter()
            .map(|point| [point.x, point.y])
            .collect())
    };
    let add_union = |shapes: &mut FloatShapes, polygon: FloatPath| {
        let next = vec![vec![polygon]];
        if shapes.is_empty() {
            *shapes = next;
        } else {
            *shapes = shapes.overlay(&next, OverlayRule::Union, FillRule::EvenOdd);
        }
    };

    for contour in base {
        if !contour.hole {
            add_union(&mut solids, path(contour)?);
        }
    }
    if solids.is_empty() {
        return Err("Foam geometry requires at least one outer contour".into());
    }
    for contour in base.iter().filter(|contour| contour.hole).chain(exclusions) {
        add_union(&mut cuts, path(contour)?);
    }
    if cuts.is_empty() {
        return Ok(base.to_vec());
    }
    let output = solids.overlay(&cuts, OverlayRule::Difference, FillRule::EvenOdd);
    if output.is_empty() {
        return Err("Foam clearances remove the complete sheet".into());
    }
    Ok(output
        .into_iter()
        .flat_map(|shape| {
            shape
                .into_iter()
                .enumerate()
                .map(|(index, points)| Contour {
                    hole: index > 0,
                    points: points
                        .into_iter()
                        .map(|point| Vec2 {
                            x: point[0],
                            y: point[1],
                        })
                        .collect(),
                })
        })
        .collect())
}

struct ResolutionInputs {
    config: MechanicalConfiguration,
    incompatible_switches: Vec<String>,
}

/// Resolve configuration and switch fit before any geometry is emitted.
/// Keeping this stage independent makes profile authority and fit diagnostics
/// testable without coupling them to case construction.
fn resolve_profiles(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
) -> Option<ResolutionInputs> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == config.board_id)?;
    let switch_parts = document
        .parts
        .iter()
        .filter_map(|part| {
            if !board.part_ids.contains(&part.id) {
                return None;
            }
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            is_switch_position(part, definition).then_some((part, definition))
        })
        .collect::<Vec<_>>();
    let mut effective_config = config.clone();
    for definition in &document.definitions {
        if let Some(profile) = &definition.mechanical_profile {
            effective_config
                .profiles
                .retain(|entry| entry.definition_id != definition.id);
            let mut inherited = profile.clone();
            inherited.definition_id = definition.id.clone();
            effective_config.profiles.push(inherited);
        }
    }
    effective_config.pcb_thickness = board.thickness;
    if effective_config.battery.is_none() {
        effective_config.battery_height = 0.0;
    }
    effective_config.part_processes = Some(normalized_processes(&effective_config));
    for (part, definition) in &switch_parts {
        let family = effective_config
            .profiles
            .iter()
            .find(|profile| profile.definition_id == part.definition_id)
            .and_then(|profile| profile.switch_family)
            .or_else(|| inferred_switch_family(part, definition));
        let Some(family) = family else { continue };
        if !effective_config
            .profiles
            .iter()
            .any(|profile| profile.definition_id == part.definition_id)
        {
            if let Ok(mut profile) = builtin_profile(
                part.definition_id.clone(),
                profile_source_for_family(family),
                switch_mounting_datum(family) - effective_config.plate_thickness,
            ) {
                profile.switch_family = Some(family);
                effective_config.profiles.push(profile);
            }
        }
        if let Some(profile) = effective_config
            .profiles
            .iter_mut()
            .find(|profile| profile.definition_id == part.definition_id)
        {
            profile.switch_family.get_or_insert(family);
        }
    }
    for profile in &mut effective_config.profiles {
        // Earlier saved catalogue profiles stored the MX mounting datum as a gap.
        // Recognize only the exact reviewed sources and original value; custom
        // profiles and user-specified engagement distances retain their meaning.
        if profile.switch_family.is_none()
            && profile.plate_to_pcb == 5.0
            && profile.source_geometry.as_ref().is_some_and(|source| {
                source.text == include_str!("../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod")
                    || source.text == include_str!("../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod")
            })
        {
            profile.switch_family = Some(MechanicalSwitchFamily::Mx);
        }
        // Family compatibility also applies to separate stabilizer definitions.
        if let Some(family) = profile.switch_family {
            profile.plate_to_pcb = switch_mounting_datum(family) - effective_config.plate_thickness;
        }
    }
    let resolved_family = switch_parts.iter().find_map(|(part, definition)| {
        effective_config
            .profiles
            .iter()
            .find(|profile| profile.definition_id == part.definition_id)
            .and_then(|profile| profile.switch_family)
            .or_else(|| inferred_switch_family(part, definition))
    });
    let mut incompatible_switches = resolved_family
        .map(|family| {
            let datum = switch_mounting_datum(family);
            switch_parts
                .iter()
                .filter_map(|(part, definition)| {
                    let other = effective_config
                        .profiles
                        .iter()
                        .find(|profile| profile.definition_id == part.definition_id)
                        .and_then(|profile| profile.switch_family)
                        .or_else(|| inferred_switch_family(part, definition))?;
                    ((switch_mounting_datum(other) - datum).abs() > 0.001)
                        .then_some(part.id.clone())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (part, definition) in &switch_parts {
        let assigned = effective_config
            .profiles
            .iter()
            .find(|profile| profile.definition_id == part.definition_id)
            .and_then(|profile| profile.switch_family);
        let inferred = inferred_switch_family(part, definition);
        if assigned.is_some() && inferred.is_some() && assigned != inferred {
            incompatible_switches.push(part.id.clone());
        }
    }
    incompatible_switches.sort();
    incompatible_switches.dedup();
    if let Some(family) = resolved_family {
        effective_config.plate_to_pcb =
            switch_mounting_datum(family) - effective_config.plate_thickness;
    }
    if effective_config.mount == MechanicalMount::Gasket && effective_config.gasket_travel.is_none()
    {
        effective_config.gasket_travel = Some(0.3);
    }
    Some(ResolutionInputs {
        config: effective_config,
        incompatible_switches,
    })
}

fn build_stack(
    config: &MechanicalConfiguration,
    pcb_bottom: f64,
    bottom_foam_z: f64,
    battery_z: f64,
    battery_height: f64,
    bottom_z: f64,
) -> Vec<MechanicalStackLayer> {
    [
        ("plate", config.plate_to_pcb, config.plate_thickness),
        ("plate-foam", 0.0, config.plate_foam_thickness),
        ("pcb", pcb_bottom, config.pcb_thickness),
        ("bottom-foam", bottom_foam_z, config.bottom_foam_thickness),
        ("battery", battery_z, battery_height),
        ("bottom", bottom_z, config.bottom_thickness),
    ]
    .into_iter()
    .filter(|(_, _, thickness)| *thickness > 0.0)
    .map(|(id, z, thickness)| MechanicalStackLayer {
        id: id.into(),
        z,
        thickness,
    })
    .collect()
}

fn dimensions_are_valid(config: &MechanicalConfiguration) -> bool {
    let dimensions = [
        config.plate_thickness,
        config.pcb_thickness,
        config.bottom_thickness,
        config.wall_thickness,
        config.plate_to_pcb,
    ];
    let clearances = [
        config.plate_foam_thickness,
        config.bottom_foam_thickness,
        config.battery_height,
        config.clearance,
    ];
    dimensions
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
        && clearances
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
}

fn resolve_foam_contours(base: &[Contour], clearances: &[Contour]) -> Result<Vec<Contour>, String> {
    if clearances.is_empty() {
        return Ok(base.to_vec());
    }
    subtract_foam_exclusions(base, clearances)
}

struct PartGeometry {
    pcb_reference_contours: Vec<Contour>,
    component_volumes: Vec<(String, CaseOpening)>,
    profile_openings: Vec<CaseOpening>,
    plate_foam_clearances: Vec<Contour>,
    bottom_foam_clearances: Vec<Contour>,
}

fn collect_part_geometry(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    board: &Board,
    contours: &[Contour],
    plate_contours: &mut Vec<Contour>,
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> PartGeometry {
    let mut pcb_reference_contours = contours.to_vec();
    let mut component_volumes = Vec::new();
    let mut profile_openings = Vec::new();
    let mut plate_foam_clearances = Vec::new();
    let mut bottom_foam_clearances = Vec::new();
    let pcb_bottom = -config.pcb_thickness;
    let bottom_foam_z = pcb_bottom - config.bottom_foam_thickness;
    for part in document
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
    {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id);
        let is_switch = definition.is_some_and(|definition| is_switch_position(part, definition));
        if !part.pose.at.x.is_finite()
            || !part.pose.at.y.is_finite()
            || !part.pose.rotation.is_finite()
        {
            issue(
                &format!("pose:{}", part.id),
                Severity::Error,
                "Mechanical part pose must be finite.",
                vec![part.id.clone()],
            );
            continue;
        }
        let profile = config
            .profiles
            .iter()
            .find(|p| p.definition_id == part.definition_id);
        let explicit_clearances = profile.is_some_and(|profile| {
            profile
                .clearances
                .as_ref()
                .is_some_and(|items| !items.is_empty())
                || profile
                    .clearance_volumes
                    .as_ref()
                    .is_some_and(|items| !items.is_empty())
        });
        if !explicit_clearances
            && (config.plate_foam_thickness > 0.0 || config.bottom_foam_thickness > 0.0)
        {
            if let Some(definition) = definition {
                if let Some((polygon, source)) = fallback_foam_geometry(part, definition) {
                    let target = if part.side == Side::Back {
                        &mut bottom_foam_clearances
                    } else {
                        &mut plate_foam_clearances
                    };
                    match add_foam_clearance(
                        target,
                        part,
                        &polygon,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => issue(
                            &format!("foam-approximation:{}", part.id),
                            Severity::Warning,
                            &format!(
                                "Foam clearance uses the {source} as a conservative footprint approximation because no explicit mechanical clearance is assigned; a 3D model is not required."
                            ),
                            vec![part.id.clone()],
                        ),
                        Err(message) => issue(
                            &format!("foam-approximation:{}", part.id),
                            Severity::Error,
                            &message,
                            vec![part.id.clone()],
                        ),
                    }
                } else {
                    issue(
                        &format!("foam-bounds:{}", part.id),
                        Severity::Warning,
                        "No courtyard, closed pad bounds, or keycap dimensions are available for a foam clearance; no component model was used to guess one.",
                        vec![part.id.clone()],
                    );
                }
            } else {
                issue(
                    &format!("foam-bounds:{}", part.id),
                    Severity::Warning,
                    "The footprint definition is missing, so no outline-based foam clearance could be generated.",
                    vec![part.id.clone()],
                );
            }
        }
        let Some(profile) = profile else {
            if is_switch {
                let unresolved = definition
                    .is_some_and(|definition| inferred_switch_family(part, definition).is_none());
                issue(
                    if unresolved {
                        "switch-family"
                    } else {
                        "profile"
                    },
                    Severity::Error,
                    if unresolved {
                        "Choose the MX, Choc v1, or Choc v2 fit for this switch before generating its plate cutout."
                    } else {
                        "Switch has no qualified mechanical profile; no cutout was guessed."
                    },
                    vec![part.id.clone()],
                );
            }
            continue;
        };
        if is_switch && profile.switch_family.is_none() {
            issue(
                "switch-family",
                Severity::Error,
                "Choose the switch family before generating a plate for this imported footprint.",
                vec![part.id.clone()],
            );
        }
        let has_access = profile.openings.as_ref().is_some_and(|v| !v.is_empty())
            || profile.clearances.as_ref().is_some_and(|v| !v.is_empty())
            || profile
                .clearance_volumes
                .as_ref()
                .is_some_and(|v| !v.is_empty());
        if (profile.cutouts.is_empty() && (is_switch || !has_access))
            || profile.source.trim().is_empty()
        {
            issue(
                &format!("profile-empty:{}", part.id),
                Severity::Error,
                "Mechanical profile requires cutout geometry and source provenance.",
                vec![part.id.clone()],
            );
        }
        if !profile.plate_to_pcb.is_finite()
            || (profile.plate_to_pcb - config.plate_to_pcb).abs() > 0.001
        {
            issue(
                &format!("engagement:{}", part.id),
                Severity::Error,
                "Selected mechanical profiles have incompatible plate-to-PCB engagement distances.",
                vec![part.id.clone()],
            );
        }
        if let Some(range) = profile.supported_thickness {
            if !range.x.is_finite()
                || !range.y.is_finite()
                || range.x <= 0.0
                || range.y < range.x
                || config.plate_thickness < range.x
                || config.plate_thickness > range.y
            {
                issue(
                    &format!("thickness:{}", part.id),
                    Severity::Error,
                    "Plate thickness is outside this profile's supported range.",
                    vec![part.id.clone()],
                );
            }
        } else {
            issue(
                &format!("qualification:{}", part.id),
                Severity::Warning,
                "Profile plate thickness compatibility has not been qualified.",
                vec![part.id.clone()],
            );
        }
        let mut polygons = profile.cutouts.clone();
        let mut pcb_holes = profile.pcb_holes.clone().unwrap_or_default();
        let stabilizer = config
            .stabilizers
            .iter()
            .flatten()
            .find(|s| s.part_id == part.id);
        if let Some(stabilizer) = stabilizer {
            if stabilizer.kind == MechanicalStabilizerKind::PlateMount {
                if let Some(custom) = &stabilizer.profile {
                    if custom.cutouts.is_empty()
                        || custom.source.trim().is_empty()
                        || !custom.plate_to_pcb.is_finite()
                        || (custom.plate_to_pcb - config.plate_to_pcb).abs() > 0.001
                    {
                        issue(
                            &format!("plate-stabilizer:{}", part.id),
                            Severity::Error,
                            "Custom plate-mount profile requires source geometry and matching engagement.",
                            vec![part.id.clone()],
                        );
                    } else {
                        let (sin, cos) = stabilizer.rotation.unwrap_or(0.0).to_radians().sin_cos();
                        polygons.extend(custom.cutouts.iter().map(|polygon| {
                            polygon
                                .iter()
                                .map(|p| Vec2 {
                                    x: p.x * cos - p.y * sin,
                                    y: p.x * sin + p.y * cos,
                                })
                                .collect::<Vec<_>>()
                        }));
                        issue(
                            &format!("custom-stabilizer:{}", part.id),
                            Severity::Warning,
                            "Custom plate-mount geometry requires supplier review; compatibility has not been qualified.",
                            vec![part.id.clone()],
                        );
                    }
                } else {
                    issue(
                        &format!("plate-stabilizer:{}", part.id),
                        Severity::Error,
                        "Plate-mount stabilizer requires an explicit custom profile; PCB-mount geometry cannot substitute.",
                        vec![part.id.clone()],
                    );
                }
            } else if stabilizer.kind == MechanicalStabilizerKind::PcbMount {
                let source = if (stabilizer.units - 2.0).abs() < 0.001 {
                    Some(MechanicalBuiltinProfile::MxStab2u)
                } else if (stabilizer.units - 6.25).abs() < 0.001 {
                    Some(MechanicalBuiltinProfile::MxStab625u)
                } else {
                    None
                };
                if let Some(source) = source {
                    match builtin_profile(part.definition_id.clone(), source, config.plate_to_pcb) {
                        Ok(stab) => {
                            let (sin, cos) =
                                stabilizer.rotation.unwrap_or(0.0).to_radians().sin_cos();
                            pcb_holes.extend(stab.pcb_holes.unwrap_or_default().into_iter().map(
                                |mut hole| {
                                    hole.at = Vec2 {
                                        x: hole.at.x * cos - hole.at.y * sin,
                                        y: hole.at.x * sin + hole.at.y * cos,
                                    };
                                    hole
                                },
                            ));
                            polygons.extend(stab.cutouts.into_iter().map(|polygon| {
                                polygon
                                    .into_iter()
                                    .map(|p| Vec2 {
                                        x: p.x * cos - p.y * sin,
                                        y: p.x * sin + p.y * cos,
                                    })
                                    .collect::<Vec<_>>()
                            }));
                        }
                        Err(message) => issue(
                            &format!("stabilizer-source:{}", part.id),
                            Severity::Error,
                            &message,
                            vec![part.id.clone()],
                        ),
                    }
                } else {
                    issue(
                        &format!("stabilizer-size:{}", part.id),
                        Severity::Error,
                        "No qualified stabilizer profile exists for this key size.",
                        vec![part.id.clone()],
                    );
                }
            }
        } else if part.keycap.is_some_and(|size| size.x.max(size.y) >= 37.0) {
            issue(
                &format!("stabilizer-proposal:{}", part.id),
                Severity::Warning,
                "Wide keycap needs a stabilizer decision: confirm key units and PCB-mount orientation; no cutout was guessed.",
                vec![part.id.clone()],
            );
        }
        let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
        if let Some(source) = &profile.source_geometry {
            use sha2::{Digest, Sha256};
            let hash: String = Sha256::digest(source.text.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if hash != source.sha256 {
                issue(
                    &format!("source-hash:{}", part.id),
                    Severity::Error,
                    "Mechanical source hash does not match retained source text.",
                    vec![part.id.clone()],
                );
            }
        }
        for hole in &pcb_holes {
            if !hole.diameter.is_finite()
                || hole.diameter <= 0.0
                || !hole.at.x.is_finite()
                || !hole.at.y.is_finite()
            {
                issue(
                    &format!("pcb-hole:{}", part.id),
                    Severity::Error,
                    "PCB hole references require finite positions and positive diameters.",
                    vec![part.id.clone()],
                );
                continue;
            }
            let x = if part.side == Side::Back {
                -hole.at.x
            } else {
                hole.at.x
            };
            let at = Vec2 {
                x: part.pose.at.x + x * cos - hole.at.y * sin,
                y: part.pose.at.y + x * sin + hole.at.y * cos,
            };
            let points = (0..64)
                .map(|index| {
                    let angle = index as f64 * std::f64::consts::TAU / 64.0;
                    Vec2 {
                        x: at.x + hole.diameter / 2.0 * angle.cos(),
                        y: at.y + hole.diameter / 2.0 * angle.sin(),
                    }
                })
                .collect();
            let linked = document
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)
                .is_some_and(|definition| {
                    definition.pads.iter().any(|pad| {
                        pad.drill
                            .is_some_and(|diameter| (diameter - hole.diameter).abs() < 0.001)
                            && (pad.at.x - hole.at.x).abs() < 0.001
                            && (pad.at.y - hole.at.y).abs() < 0.001
                    })
                });
            if linked {
                pcb_reference_contours.push(Contour { hole: true, points });
            }
            if !linked {
                issue(
                    &format!("pcb-hole-link:{}:{}", part.id, hole.source_id),
                    if board.traces.is_empty() {
                        Severity::Warning
                    } else {
                        Severity::Error
                    },
                    "Mechanical mounting hole is not linked to a matching electrical PCB hole; the PCB remains unchanged.",
                    vec![part.id.clone()],
                );
            }
        }
        for volume in profile.clearance_volumes.iter().flatten() {
            let transformed = CaseOpening {
                z: volume.z,
                height: volume.height,
                points: transform_part_points(part, &volume.points),
            };
            if transformed.points.len() < 3
                || transformed
                    .points
                    .iter()
                    .any(|p| !p.x.is_finite() || !p.y.is_finite())
                || !volume.z.is_finite()
                || !volume.height.is_finite()
                || volume.height <= 0.0
            {
                issue(
                    &format!("clearance-volume:{}", part.id),
                    Severity::Error,
                    "Clearance volumes need finite polygons/Z and positive height.",
                    vec![part.id.clone()],
                );
            } else {
                let volume_bottom = transformed.z;
                let volume_top = transformed.z + transformed.height;
                let plate_foam_top = config.plate_foam_thickness;
                if config.plate_foam_thickness > 0.0
                    && volume_bottom < plate_foam_top
                    && volume_top > 0.0
                {
                    match add_foam_clearance(
                        &mut plate_foam_clearances,
                        part,
                        &volume.points,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => {}
                        Err(message) => issue(
                            &format!("clearance-volume:{}", part.id),
                            Severity::Error,
                            &message,
                            vec![part.id.clone()],
                        ),
                    }
                }
                if config.bottom_foam_thickness > 0.0
                    && volume_bottom < pcb_bottom
                    && volume_top > bottom_foam_z
                {
                    match add_foam_clearance(
                        &mut bottom_foam_clearances,
                        part,
                        &volume.points,
                        config.clearance,
                        document.revision,
                    ) {
                        Ok(()) => {}
                        Err(message) => issue(
                            &format!("clearance-volume:{}", part.id),
                            Severity::Error,
                            &message,
                            vec![part.id.clone()],
                        ),
                    }
                }
                component_volumes.push((part.id.clone(), transformed));
            }
        }
        for opening in profile.openings.iter().flatten() {
            profile_openings.push(CaseOpening {
                z: opening.z,
                height: opening.height,
                points: transform_part_points(part, &opening.points),
            });
        }
        for polygon in profile.clearances.iter().flatten() {
            if polygon.len() < 3 || polygon.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                issue(
                    &format!("clearance:{}", part.id),
                    Severity::Error,
                    "Profile clearance geometry must be finite polygons.",
                    vec![part.id.clone()],
                );
            } else {
                let target = if part.side == Side::Back {
                    &mut bottom_foam_clearances
                } else {
                    &mut plate_foam_clearances
                };
                if let Err(message) =
                    add_foam_clearance(target, part, polygon, config.clearance, document.revision)
                {
                    issue(
                        &format!("clearance:{}", part.id),
                        Severity::Error,
                        &message,
                        vec![part.id.clone()],
                    );
                }
                issue(
                    &format!("access:{}", part.id),
                    Severity::Warning,
                    "Explicit component clearance is excluded from foam on the component side; case wall access still requires review.",
                    vec![part.id.clone()],
                );
            }
        }
        for polygon in &polygons {
            if polygon.len() < 3 || polygon.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                issue(
                    &format!("polygon:{}", part.id),
                    Severity::Error,
                    "Mechanical profile has an invalid polygon.",
                    vec![part.id.clone()],
                );
                continue;
            }
            let cutout = Contour {
                hole: true,
                points: polygon
                    .iter()
                    .map(|p| Vec2 {
                        x: part.pose.at.x
                            + (if part.side == Side::Back { -p.x } else { p.x }) * cos
                            - p.y * sin,
                        y: part.pose.at.y
                            + (if part.side == Side::Back { -p.x } else { p.x }) * sin
                            + p.y * cos,
                    })
                    .collect(),
            };
            if !plate_contours
                .iter()
                .any(|existing| same_ring(existing, &cutout))
            {
                plate_contours.push(cutout);
            }
        }
    }
    PartGeometry {
        pcb_reference_contours,
        component_volumes,
        profile_openings,
        plate_foam_clearances,
        bottom_foam_clearances,
    }
}

struct BatterySpace {
    height: f64,
    bottom_foam_contours: Vec<Contour>,
}

fn resolve_battery_space(
    config: &MechanicalConfiguration,
    contours: &[Contour],
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> Option<BatterySpace> {
    let battery_height = config
        .battery
        .as_ref()
        .map_or(config.battery_height, |b| b.size.z);
    let mut bottom_foam_contours = contours.to_vec();
    if let Some(battery) = &config.battery {
        if [battery.size.x, battery.size.y, battery.size.z]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            issue(
                "battery-size",
                Severity::Error,
                "Battery dimensions must be positive finite millimetres.",
                vec!["battery".into()],
            );
            return None;
        }
        let margin = config.clearance;
        let hx = battery.size.x / 2.0 + margin;
        let hy = battery.size.y / 2.0 + margin;
        let envelope = vec![
            Vec2 {
                x: battery.at.x - hx,
                y: battery.at.y - hy,
            },
            Vec2 {
                x: battery.at.x + hx,
                y: battery.at.y - hy,
            },
            Vec2 {
                x: battery.at.x + hx,
                y: battery.at.y + hy,
            },
            Vec2 {
                x: battery.at.x - hx,
                y: battery.at.y + hy,
            },
        ];
        if ![
            battery.at.x,
            battery.at.y,
            battery.cable_exit.x,
            battery.cable_exit.y,
        ]
        .iter()
        .all(|v| v.is_finite())
            || envelope.iter().any(|p| !inside_outline(*p, contours))
            || !inside_outline(battery.cable_exit, contours)
        {
            issue(
                "battery-containment",
                Severity::Error,
                "Battery envelope or cable exit lies outside the assembly outline.",
                vec!["battery".into()],
            );
        }
        for mount in config
            .mounts
            .iter()
            .chain(config.closure_mounts.iter().flatten())
        {
            let radius = mount.boss_diameter.unwrap_or(mount.hole_diameter) / 2.0;
            if (mount.at.x - battery.at.x).abs() < hx + radius
                && (mount.at.y - battery.at.y).abs() < hy + radius
            {
                issue(
                    &format!("battery-mount:{}", mount.id),
                    Severity::Error,
                    "Mount overlaps the battery access envelope.",
                    vec!["battery".into(), mount.id.clone()],
                );
            }
        }
        let cable_width = battery.cable_width.unwrap_or(2.0);
        if !cable_width.is_finite() || cable_width <= 0.0 {
            issue(
                "battery-cable-width",
                Severity::Error,
                "Battery cable clearance width must be positive finite millimetres.",
                vec!["battery".into()],
            );
        } else {
            let clearance = cable_width / 2.0 + config.clearance;
            let mut access_points = envelope;
            for (x, y) in [
                (-clearance, -clearance),
                (clearance, -clearance),
                (clearance, clearance),
                (-clearance, clearance),
            ] {
                access_points.push(Vec2 {
                    x: battery.cable_exit.x + x,
                    y: battery.cable_exit.y + y,
                });
            }
            let access = convex_envelope(access_points);
            if access.iter().any(|point| !inside_outline(*point, contours)) {
                issue(
                    "battery-cable-containment",
                    Severity::Error,
                    "Battery cable clearance envelope crosses the assembly outline.",
                    vec!["battery".into()],
                );
            }
            bottom_foam_contours.push(Contour {
                points: access,
                hole: true,
            });
            issue(
                "battery-cable-envelope",
                Severity::Info,
                if battery.cable_width.is_none() {
                    "Bottom foam excludes a conservative battery-to-cable envelope using the default 2 mm cable width; review cable fit."
                } else {
                    "Bottom foam excludes a conservative battery-to-cable envelope using the configured cable width."
                },
                vec!["battery".into(), "bottom-foam".into()],
            );
        }
        issue(
            "battery-fit",
            Severity::Warning,
            "Battery envelope and cable exit require mechanical fit review; retention features are not generated.",
            vec!["battery".into()],
        );
    }
    Some(BatterySpace {
        height: battery_height,
        bottom_foam_contours,
    })
}

struct ConstructionGeometry {
    plate_contours: Vec<Contour>,
    frame_contours: Vec<Contour>,
    plate_foam_contours: Vec<Contour>,
    bottom_foam_contours: Vec<Contour>,
    profile_openings: Vec<CaseOpening>,
    suspension_bosses: Vec<Mount>,
    bottom_z: f64,
    bottom_foam_z: f64,
    battery_z: f64,
    support_height: f64,
}

struct ConstructedBodies {
    bodies: Vec<CaseIR>,
    stack: Vec<MechanicalStackLayer>,
}

fn construct_bodies(
    revision: u64,
    config: &MechanicalConfiguration,
    geometry: ConstructionGeometry,
    issue: &mut impl FnMut(&str, Severity, &str, Vec<String>),
) -> ConstructedBodies {
    let ConstructionGeometry {
        plate_contours,
        frame_contours,
        plate_foam_contours,
        bottom_foam_contours,
        profile_openings,
        suspension_bosses,
        bottom_z,
        bottom_foam_z,
        battery_z,
        support_height,
    } = geometry;
    let mut bodies: Vec<CaseIR> = Vec::new();
    let mut stack = Vec::new();
    let board_id = config.board_id.clone();
    let sheet_bottom = config.bottom_style == Some(MechanicalBottomStyle::Sheet)
        || config.part_processes.iter().flatten().any(|process| {
            process.part_id == "bottom"
                && matches!(process.method, PlateMethod::PcbFr4 | PlateMethod::CutSheet)
        });

    if sheet_bottom && !config.middle_frame.unwrap_or(false) && !config.integrated_plate_frame {
        issue(
            "sheet-enclosure",
            Severity::Warning,
            "A sheet bottom without a middle frame leaves the assembly sides open.",
            vec!["bottom".into()],
        );
    }
    if config.middle_frame.unwrap_or(false) && (!sheet_bottom || config.integrated_plate_frame) {
        issue(
            "middle-frame-configuration",
            Severity::Error,
            "A separate middle frame requires a sheet bottom and a separate plate.",
            vec!["middle-frame".into()],
        );
    }
    let frame_outer = frame_contours.clone();
    for (id, kind, z, thickness, body_contours) in [
        (
            "plate",
            if config.integrated_plate_frame {
                CaseKind::Lid
            } else {
                CaseKind::Plate
            },
            if config.integrated_plate_frame {
                bottom_z + config.bottom_thickness
            } else {
                config.plate_to_pcb
            },
            config.plate_thickness,
            plate_contours,
        ),
        (
            "bottom",
            if config.integrated_plate_frame || sheet_bottom {
                CaseKind::Plate
            } else {
                CaseKind::Tray
            },
            bottom_z,
            config.bottom_thickness,
            frame_contours,
        ),
        (
            "plate-foam",
            CaseKind::Plate,
            0.0,
            config.plate_foam_thickness,
            plate_foam_contours,
        ),
        (
            "bottom-foam",
            CaseKind::Plate,
            bottom_foam_z,
            config.bottom_foam_thickness,
            bottom_foam_contours,
        ),
    ] {
        if thickness <= 0.0 {
            continue;
        }
        bodies.push(CaseIR {
            revision: revision,
            contours: body_contours,
            body: CaseBody {
                features: None,
                openings: if (id == "bottom" && !config.integrated_plate_frame)
                    || (id == "plate" && config.integrated_plate_frame)
                {
                    let mut openings = config.openings.clone().unwrap_or_default();
                    openings.extend(profile_openings.clone());
                    Some(openings)
                } else {
                    None
                },
                id: id.into(),
                name: id.into(),
                board_id: board_id.clone(),
                kind,
                thickness,
                clearance: 0.0,
                material_id: None,
                z: Some(z),
                wall_height: if (id == "bottom" && !config.integrated_plate_frame)
                    || (id == "plate" && config.integrated_plate_frame)
                {
                    Some(
                        config.plate_to_pcb
                            - bottom_z
                            - config.bottom_thickness
                            - if config.mount == MechanicalMount::Gasket {
                                config.gasket_travel.unwrap_or(0.0)
                            } else {
                                0.0
                            },
                    )
                } else {
                    None
                },
                wall_thickness: Some(config.wall_thickness),
                mounts: Some(if id == "bottom" {
                    let mut mounts = config.closure_mounts.clone().unwrap_or_default();
                    mounts.extend(suspension_bosses.clone());
                    mounts
                } else if id == "plate" {
                    let mut mounts: Vec<Mount> = config
                        .mounts
                        .iter()
                        .filter(|_| config.mount == MechanicalMount::Rigid)
                        .map(|mount| {
                            let mut hole = mount.clone();
                            hole.kind = MountKind::Hole;
                            hole.height = None;
                            hole.boss_diameter = None;
                            hole
                        })
                        .collect();
                    mounts.extend(config.closure_mounts.iter().flatten().map(|mount| {
                        let mut mating = mount.clone();
                        if !config.integrated_plate_frame {
                            mating.kind = MountKind::Hole;
                            mating.height = None;
                            mating.boss_diameter = None;
                        }
                        mating
                    }));
                    mounts
                } else {
                    vec![]
                }),
                gasket: None,
            },
        });
    }
    if sheet_bottom && config.middle_frame.unwrap_or(false) && !config.integrated_plate_frame {
        if let Some(bottom) = bodies.iter().find(|body| body.body.id == "bottom") {
            let mut frame = bottom.clone();
            frame.body.id = "middle-frame".into();
            frame.body.name = "Middle frame".into();
            frame.body.kind = CaseKind::Tray;
            frame.body.wall_height = Some(config.plate_to_pcb - bottom_z - config.bottom_thickness);
            frame.contours = frame_outer;
            if let Ok(prepared) = crate::case::prepare(&CaseAssemblyIR {
                revision: revision,
                bodies: vec![frame.clone()],
            }) {
                let mut rings = Vec::new();
                for region in &prepared.bodies[0].regions {
                    rings.push(Contour {
                        hole: false,
                        points: region.outer.clone(),
                    });
                    rings.extend(region.cavities.iter().map(|points| Contour {
                        hole: true,
                        points: points.clone(),
                    }));
                }
                frame.contours = rings;
                frame.body.kind = CaseKind::Plate;
                frame.body.z = Some(bottom_z + config.bottom_thickness);
                frame.body.thickness = config.plate_to_pcb - bottom_z - config.bottom_thickness;
                frame.body.wall_height = None;
                frame.body.gasket = None;
                stack.push(MechanicalStackLayer {
                    id: "middle-frame".into(),
                    z: frame.body.z.unwrap(),
                    thickness: frame.body.thickness,
                });
                bodies.push(frame);
            }
        }
    }
    if sheet_bottom {
        if let Some(bottom) = bodies.iter_mut().find(|body| body.body.id == "bottom") {
            for mount in bottom.body.mounts.iter_mut().flatten() {
                mount.kind = MountKind::Hole;
                mount.boss_diameter = None;
                mount.height = None;
            }
        }
        for support in &suspension_bosses {
            let Some(diameter) = support.boss_diameter else {
                continue;
            };
            let ring = |diameter: f64, hole: bool| Contour {
                hole,
                points: (0..64)
                    .map(|index| {
                        let angle = index as f64 * std::f64::consts::TAU / 64.0;
                        Vec2 {
                            x: support.at.x + diameter / 2.0 * angle.cos(),
                            y: support.at.y + diameter / 2.0 * angle.sin(),
                        }
                    })
                    .collect(),
            };
            let id = format!("spacer:{}", support.id);
            bodies.push(CaseIR {
                revision: revision,
                contours: vec![ring(diameter, false), ring(support.hole_diameter, true)],
                body: CaseBody {
                    features: None,
                    id: id.clone(),
                    name: id.clone(),
                    board_id: config.board_id.clone(),
                    kind: CaseKind::Plate,
                    thickness: support_height,
                    clearance: 0.0,
                    material_id: None,
                    z: Some(battery_z),
                    wall_height: None,
                    wall_thickness: None,
                    mounts: None,
                    gasket: None,
                    openings: None,
                },
            });
            stack.push(MechanicalStackLayer {
                id,
                z: battery_z,
                thickness: support_height,
            });
        }
    }
    ConstructedBodies { bodies, stack }
}

fn finalize_assembly(
    document: &ProjectDoc,
    config: &MechanicalConfiguration,
    result: &mut MechanicalAssembly,
    component_volumes: &[(String, CaseOpening)],
) {
    crate::mechanical_checks::apply_allowance(config, result);
    if config.mount == MechanicalMount::Gasket && !result.generation_blocked {
        if let Err(message) = gasket::generate(document, config, result, component_volumes) {
            result.generation_blocked = true;
            result.diagnostics.push(Finding {
                id: "mechanical:gasket-layout".into(),
                severity: Severity::Error,
                scope: Scope::Case,
                message,
                target_ids: vec!["plate".into()],
            });
        }
    }
    if let Err(message) = crate::case::prepare(&result.case) {
        result.generation_blocked = true;
        result.diagnostics.push(Finding {
            id: "mechanical:case-preparation".into(),
            severity: Severity::Error,
            scope: Scope::Case,
            message,
            target_ids: vec![],
        });
    }
    if config.internal_gasket.is_none() || config.mount != MechanicalMount::Gasket {
    let mut mounting_contours = result.plate_contours.clone();
    let board = document.boards.iter().find(|board| board.id == config.board_id);
    for part in document.parts.iter().filter(|part| board.is_some_and(|board| board.part_ids.contains(&part.id))) {
        let Some(definition) = document.definitions.iter().find(|definition| definition.id == part.definition_id) else { continue };
        if definition.kind != PartKind::Switch && definition.courtyard.len() >= 3 {
            mounting_contours.push(Contour { hole: true, points: transform_part_points(part, &definition.courtyard) });
        }
        for pad in &definition.pads {
            let (sin, cos) = pad.rotation.unwrap_or(0.0).to_radians().sin_cos();
            let points = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2 {
                x: pad.at.x + x * pad.size.x / 2.0 * cos - y * pad.size.y / 2.0 * sin,
                y: pad.at.y + x * pad.size.x / 2.0 * sin + y * pad.size.y / 2.0 * cos,
            });
            mounting_contours.push(Contour { hole: true, points: transform_part_points(part, &points) });
        }
    }
    for opening in config.openings.iter().flatten() {
        mounting_contours.push(Contour { hole: true, points: opening.points.clone() });
    }
    result.suggested_mounts =
        crate::mechanical_checks::propose_mounts(config, &mounting_contours);
    }
    result.diagnostics.extend(crate::mechanical_checks::check(
        config,
        &result.plate_contours,
    ));
    result
        .diagnostics
        .extend(crate::mechanical_checks::check_specifications(
            config,
            &result.case,
        ));
}

pub fn resolve(document: &ProjectDoc, contours: &[Contour]) -> MechanicalAssembly {
    let mut result = MechanicalAssembly {
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
        component_volumes,
        profile_openings,
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
    let Some(BatterySpace {
        height: battery_height,
        bottom_foam_contours,
    }) = resolve_battery_space(config, contours, &mut issue)
    else {
        return result;
    };
    // PCB top is the stable zero plane; layers below it have negative Z.
    let battery_z = if config.battery.is_some() { pcb_bottom - battery_height } else { bottom_foam_z - battery_height };
    let bottom_z = bottom_foam_z.min(battery_z) - config.bottom_thickness;
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
    result.stack.extend(constructed.stack);
    finalize_assembly(document, config, &mut result, &component_volumes);
    result
}

fn expanded_outline(
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

fn inside_outline(point: Vec2, contours: &[Contour]) -> bool {
    let contains = |ring: &Contour| {
        let mut inside = false;
        for i in 0..ring.points.len() {
            let a = ring.points[i];
            let b = ring.points[(i + 1) % ring.points.len()];
            if (a.y > point.y) != (b.y > point.y)
                && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
            {
                inside = !inside;
            }
        }
        inside
    };
    contours.iter().any(|c| !c.hole && contains(c))
        && !contours.iter().any(|c| c.hole && contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> MechanicalConfiguration {
        MechanicalConfiguration {
            internal_gasket: None,
            gasket_layout: None,
            hardware: None,
            critical_fits: None,
            bottom_style: None,
            middle_frame: None,
            gasket_travel: None,
            openings: None,
            opening_allowance: None,
            stabilizers: None,
            part_processes: None,
            gasket: None,
            closure_mounts: None,
            board_id: "board".into(),
            method: PlateMethod::Printed,
            mount: MechanicalMount::Tray,
            integrated_plate_frame: false,
            battery: None,
            mounts: vec![],
            plate_thickness: 1.5,
            plate_foam_thickness: 1.0,
            pcb_thickness: 1.6,
            bottom_foam_thickness: 0.5,
            battery_height: 3.0,
            bottom_thickness: 2.0,
            plate_to_pcb: 3.5,
            wall_thickness: 2.0,
            clearance: 0.2,
            profiles: vec![],
        }
    }
    #[test]
    fn disabled_mechanical_configuration_has_no_stack() {
        let doc = ProjectDoc::empty("test", "test");
        assert!(
            serde_json::to_value(&doc)
                .unwrap()
                .get("mechanical")
                .is_none()
        );
        assert!(resolve(&doc, &[]).stack.is_empty());
    }
    #[test]
    fn stack_is_explicit_and_revision_tagged() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.revision = 7;
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.mechanical = Some(config());
        let result = resolve(&doc, &[]);
        assert_eq!(result.revision, 7);
        assert_eq!(
            result.stack.iter().find(|l| l.id == "bottom").unwrap().z,
            -4.1
        );
        assert_eq!(result.case.bodies[0].contours, result.plate_contours);
    }
    #[test]
    fn custom_dimensions_and_disabled_foam_survive_normalization() {
        let mut c = config();
        c.plate_thickness = 1.8;
        c.plate_foam_thickness = 0.0;
        c.bottom_foam_thickness = 0.0;
        c.bottom_thickness = 4.2;
        c.clearance = 0.45;
        let processes = normalized_processes(&c);
        assert_eq!(c.plate_thickness, 1.8);
        assert_eq!(c.bottom_thickness, 4.2);
        assert_eq!(c.clearance, 0.45);
        assert_eq!(
            processes
                .iter()
                .find(|item| item.part_id == "plate-foam")
                .unwrap()
                .thickness,
            0.0
        );
        assert_eq!(
            processes
                .iter()
                .find(|item| item.part_id == "bottom-foam")
                .unwrap()
                .thickness,
            0.0
        );
    }
    #[test]
    fn no_model_foam_fallbacks_transform_union_and_warn() {
        let mut doc = ProjectDoc::empty("foam", "foam");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["front-a".into(), "front-b".into(), "back".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.extend([
            serde_json::from_value(serde_json::json!({
                "id":"front-a", "definitionId":"courtyard-part", "reference":"U1",
                "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "id":"front-b", "definitionId":"courtyard-part", "reference":"U2",
                "pose":{"at":{"x":23,"y":20},"rotation":0}, "side":"front"
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "id":"back", "definitionId":"pad-only-part", "reference":"J1",
                "pose":{"at":{"x":70,"y":70},"rotation":0}, "side":"back"
            }))
            .unwrap(),
        ]);
        doc.definitions.extend([
            serde_json::from_value(serde_json::json!({
                "id":"courtyard-part", "name":"No model front component", "kind":"custom",
                "courtyard":[{"x":-4,"y":-3},{"x":4,"y":-3},{"x":4,"y":3},{"x":-4,"y":3}],
                "pads":[]
            }))
            .unwrap(),
            serde_json::from_value(serde_json::json!({
                "id":"pad-only-part", "name":"No model back component", "kind":"connector",
                "courtyard":[], "pads":[{"id":"1","number":"1","at":{"x":1,"y":0},
                "size":{"x":4,"y":2},"shape":"rect","rotation":90}]
            }))
            .unwrap(),
        ]);
        let mut configuration = config();
        configuration.clearance = 0.3;
        doc.mechanical = Some(configuration);
        let outline = [Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 100.0, y: 0.0 },
                Vec2 { x: 100.0, y: 100.0 },
                Vec2 { x: 0.0, y: 100.0 },
            ],
        }];

        let assembly = resolve(&doc, &outline);
        let plate_foam = assembly
            .case
            .bodies
            .iter()
            .find(|body| body.body.id == "plate-foam")
            .unwrap();
        let bottom_foam = assembly
            .case
            .bodies
            .iter()
            .find(|body| body.body.id == "bottom-foam")
            .unwrap();
        let hole_bounds = |body: &CaseIR| {
            body.contours
                .iter()
                .filter(|contour| contour.hole)
                .map(|contour| {
                    let min_x = contour
                        .points
                        .iter()
                        .map(|point| point.x)
                        .fold(f64::INFINITY, f64::min);
                    let max_x = contour
                        .points
                        .iter()
                        .map(|point| point.x)
                        .fold(f64::NEG_INFINITY, f64::max);
                    (min_x, max_x)
                })
                .collect::<Vec<_>>()
        };
        let front_holes = hole_bounds(plate_foam);
        let back_holes = hole_bounds(bottom_foam);
        assert_eq!(
            front_holes.len(),
            1,
            "overlapping courtyard clearances are unioned"
        );
        assert!((front_holes[0].0 - 15.7).abs() < 0.01);
        assert!((front_holes[0].1 - 27.3).abs() < 0.01);
        assert_eq!(back_holes.len(), 1, "back-side pads clear only bottom foam");
        assert!((back_holes[0].0 - 67.7).abs() < 0.01);
        assert!((back_holes[0].1 - 70.3).abs() < 0.01);
        assert!(assembly.diagnostics.iter().any(|finding| {
            finding.id == "mechanical:foam-approximation:front-a"
                && finding.severity == Severity::Warning
        }));
        assert!(!assembly.generation_blocked);
    }
    #[test]
    fn integrated_frame_uses_lid_and_separate_bottom() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut c = config();
        c.integrated_plate_frame = true;
        c.mount = MechanicalMount::Rigid;
        doc.mechanical = Some(c);
        let resolved = resolve(
            &doc,
            &[Contour {
                hole: false,
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 50.0, y: 0.0 },
                    Vec2 { x: 50.0, y: 50.0 },
                    Vec2 { x: 0.0, y: 50.0 },
                ],
            }],
        );
        let plate = &resolved
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "plate")
            .unwrap()
            .body;
        assert_eq!(plate.kind, CaseKind::Lid);
        assert!((plate.z.unwrap() + plate.wall_height.unwrap() - 3.5).abs() < 1e-9);
        assert_eq!(
            resolved
                .case
                .bodies
                .iter()
                .find(|b| b.body.id == "bottom")
                .unwrap()
                .body
                .kind,
            CaseKind::Plate
        );
    }
    #[test]
    fn gasket_mode_places_discrete_supports_and_retaining_frame_from_defaults() {
        let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut settings = config();
        settings.mount = MechanicalMount::Gasket;
        doc.mechanical = Some(settings);
        let contours = vec![Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0., y: 0. },
                Vec2 { x: 130., y: 0. },
                Vec2 { x: 130., y: 90. },
                Vec2 { x: 0., y: 90. },
            ],
        }];
        let result = resolve(&doc, &contours);
        assert!(!result.generation_blocked, "{:?}", result.diagnostics);
        assert!(result.case.bodies.iter().any(|b| b.body.id == "retainer"));
        assert_eq!(
            result
                .case
                .bodies
                .iter()
                .filter(|b| b.body.id.starts_with("gasket:"))
                .count(),
            12
        );
    }

    #[test]
    fn manual_gasket_anchors_are_exact_persistent_and_invalidated_by_outline_changes() {
        let mut doc = ProjectDoc::empty("gaskets", "Gaskets");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut settings = config();
        settings.mount = MechanicalMount::Gasket;
        doc.mechanical = Some(settings);
        let mut contours = vec![Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0., y: 0. },
                Vec2 { x: 130., y: 0. },
                Vec2 { x: 130., y: 90. },
                Vec2 { x: 0., y: 90. },
            ],
        }];
        let first = resolve(&doc, &contours);
        let support = &first.gasket_supports[0];
        let anchor = support.anchor + 0.0007;
        doc.mechanical.as_mut().unwrap().gasket_layout = Some(MechanicalGasketLayout {
            auto_size: None,
            adhesive_thickness: None,
            minimum_foam_thickness: None,
            preset_id: None,
            material: None,
            length: 12.,
            width: 3.,
            thickness: 2.,
            compression: 0.15,
            supports: vec![MechanicalGasketAnchor {
                length: None,
                width: None,
                placement: None,
                id: support.id.clone(),
                region_id: support.region_id.clone(),
                outline_key: support.outline_key.clone(),
                anchor,
                unlinked: false,
            }],
        });
        let moved = resolve(&doc, &contours);
        assert!(!moved.generation_blocked, "{:?}", moved.diagnostics);
        assert!((moved.gasket_supports[0].anchor - anchor).abs() < 1e-9);
        let saved = serde_json::to_string(&doc).unwrap();
        let reopened: ProjectDoc = serde_json::from_str(&saved).unwrap();
        assert_eq!(
            moved.gasket_supports,
            resolve(&reopened, &contours).gasket_supports
        );
        contours[0].points[1].x += 1.;
        assert!(resolve(&doc, &contours).generation_blocked);
    }

    #[test]
    fn gasket_support_reaches_plate_without_rigid_contact() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut c = config();
        c.mount = MechanicalMount::Gasket;
        c.gasket_travel = Some(0.5);
        c.gasket = Some(Gasket {
            inset: 0.2,
            width: 1.0,
            depth: 0.4,
        });
        doc.mechanical = Some(c);
        let resolved = resolve(
            &doc,
            &[Contour {
                hole: false,
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 50.0, y: 0.0 },
                    Vec2 { x: 50.0, y: 50.0 },
                    Vec2 { x: 0.0, y: 50.0 },
                ],
            }],
        );
        let bottom = &resolved
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "bottom")
            .unwrap()
            .body;
        let strips = resolved
            .case
            .bodies
            .iter()
            .filter(|b| b.body.id.starts_with("gasket:"))
            .collect::<Vec<_>>();
        assert_eq!(strips.len(), 12);
        for strip in strips {
            if strip.body.id.ends_with(":lower") {
                assert!((strip.body.z.unwrap() + strip.body.thickness - 3.5).abs() < 1e-9);
            } else {
                assert!((strip.body.z.unwrap() - 5.0).abs() < 1e-9);
            }
        }
        let retainer = resolved
            .case
            .bodies
            .iter()
            .find(|b| b.body.id == "retainer")
            .unwrap();
        assert!((bottom.z.unwrap() + bottom.thickness - retainer.body.z.unwrap()).abs() < 1e-9);
        assert!(
            bottom
                .mounts
                .as_ref()
                .unwrap()
                .iter()
                .all(|m| m.id.starts_with("closure:"))
        );
        assert!(
            bottom
                .openings
                .as_ref()
                .unwrap()
                .iter()
                .any(|o| (o.z - 1.8).abs() < 1e-9)
        );
    }
    #[test]
    fn cable_exit_regenerates_single_foam_envelope_without_mutation() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        let mut c = config();
        c.battery = Some(MechanicalBattery {
            size: Vec3 {
                x: 8.0,
                y: 8.0,
                z: 3.0,
            },
            at: Vec2 { x: 20.0, y: 20.0 },
            cable_exit: Vec2 { x: 35.0, y: 20.0 },
            cable_width: Some(2.0),
        });
        doc.mechanical = Some(c);
        let outline = [Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 50.0, y: 0.0 },
                Vec2 { x: 50.0, y: 50.0 },
                Vec2 { x: 0.0, y: 50.0 },
            ],
        }];
        let saved = doc.clone();
        let first = resolve(&doc, &outline);
        assert_eq!(doc, saved);
        doc.mechanical
            .as_mut()
            .unwrap()
            .battery
            .as_mut()
            .unwrap()
            .cable_exit
            .y = 35.0;
        let second = resolve(&doc, &outline);
        let foam = |assembly: MechanicalAssembly| {
            assembly
                .case
                .bodies
                .into_iter()
                .find(|body| body.body.id == "bottom-foam")
                .unwrap()
                .contours
        };
        let a = foam(first);
        let b = foam(second);
        assert_ne!(a, b);
        assert_eq!(a.iter().filter(|contour| contour.hole).count(), 1);
        assert_eq!(b.iter().filter(|contour| contour.hole).count(), 1);
    }
    #[test]
    fn part_owned_profiles_supply_case_cutouts_without_case_assignment() {
        let mut doc = ProjectDoc::empty("profiles", "Profiles");
        let profile = builtin_profile(
            "custom-switch".into(),
            MechanicalBuiltinProfile::MxSwitch,
            3.5,
        )
        .unwrap();
        doc.definitions.push(
            serde_json::from_value(serde_json::json!({
                "id": "custom-switch", "name": "Qualified switch", "kind": "switch",
                "courtyard": [], "pads": [], "mechanicalProfile": profile
            }))
            .unwrap(),
        );
        doc.parts.push(
            serde_json::from_value(serde_json::json!({
                "id":"key", "definitionId":"custom-switch", "reference":"SW1",
                "pose":{"at":{"x":20,"y":20},"rotation":0}, "side":"front"
            }))
            .unwrap(),
        );
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.2,
            traces: vec![],
            vias: vec![],
        });
        doc.mechanical = Some(config());
        let outline = Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0., y: 0. },
                Vec2 { x: 40., y: 0. },
                Vec2 { x: 40., y: 40. },
                Vec2 { x: 0., y: 40. },
            ],
        };
        let saved = doc.clone();
        let result = resolve(&doc, &[outline]);
        assert_eq!(
            doc, saved,
            "resolving must not copy inherited fits into the project"
        );
        assert_eq!(
            result
                .plate_contours
                .iter()
                .filter(|contour| contour.hole)
                .count(),
            1
        );
        assert!(
            !result
                .diagnostics
                .iter()
                .any(|finding| finding.id.contains("profile-missing"))
        );
        assert_eq!(
            result
                .stack
                .iter()
                .find(|layer| layer.id == "pcb")
                .unwrap()
                .thickness,
            1.2
        );
    }

    #[test]
    fn switch_profiles_drive_spacing_and_reject_incompatible_engagement() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into(), "key2".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        for (id, definition_id) in [("key", "switch"), ("key2", "switch2")] {
            doc.parts.push(serde_json::from_value(serde_json::json!({"id":id,"definitionId":definition_id,"reference":id,"pose":{"at":{"x":20,"y":20},"rotation":0},"side":"front"})).unwrap());
            doc.definitions.push(serde_json::from_value(serde_json::json!({"id":definition_id,"name":"Switch","kind":"switch","courtyard":[],"pads":[]})).unwrap());
        }
        let mut c = config();
        c.plate_to_pcb = 5.0;
        c.profiles.push(
            builtin_profile("switch".into(), MechanicalBuiltinProfile::MxSwitch, 4.0).unwrap(),
        );
        c.profiles.push(
            builtin_profile("switch2".into(), MechanicalBuiltinProfile::MxSwitch, 4.0).unwrap(),
        );
        doc.mechanical = Some(c);
        let first = resolve(&doc, &[]);
        assert_eq!(
            first
                .stack
                .iter()
                .find(|layer| layer.id == "plate")
                .unwrap()
                .z,
            3.5
        );
        assert_eq!(doc.mechanical.as_ref().unwrap().plate_to_pcb, 5.0);
        doc.mechanical.as_mut().unwrap().profiles[1].switch_family =
            Some(MechanicalSwitchFamily::ChocV1);
        doc.mechanical.as_mut().unwrap().profiles[1].plate_to_pcb = 2.0;
        let second = resolve(&doc, &[]);
        assert!(
            second
                .diagnostics
                .iter()
                .any(|finding| finding.id == "mechanical:switch-family"
                    && finding.severity == Severity::Error)
        );
        assert!(second.generation_blocked);
    }
    #[test]
    fn imported_stabilizers_use_the_mx_mounting_datum() {
        let catalogue: serde_json::Value = serde_json::from_str(include_str!(
            "../../app/src/parts/imported-parts.json"
        )).unwrap();
        for imported in catalogue["parts"].as_array().unwrap() {
            let definition: PartDefinition = serde_json::from_value(imported["definition"].clone()).unwrap();
            for (thickness, legacy) in [(1.4, false), (1.5, false), (1.6, false), (1.5, true)] {
                let mut doc = ProjectDoc::empty("test", "test");
                doc.boards.push(serde_json::from_value(serde_json::json!({
                    "id":"board", "name":"Board", "outlineIds":[],
                    "partIds":["key", "stab"], "netIds":[], "thickness":1.6
                })).unwrap());
                doc.definitions.push(serde_json::from_value(serde_json::json!({
                    "id":"switch", "name":"Switch", "kind":"switch", "courtyard":[], "pads":[],
                    "generator":{"source":"ceoloide/switch_mx","version":"bundled-1","parameters":{}}
                })).unwrap());
                doc.definitions.push(definition.clone());
                if legacy {
                    let profile = doc.definitions[1].mechanical_profile.as_mut().unwrap();
                    profile.switch_family = None;
                    profile.plate_to_pcb = 5.0;
                }
                for (id, definition_id) in [("key", "switch"), ("stab", definition.id.as_str())] {
                    doc.parts.push(serde_json::from_value(serde_json::json!({
                        "id":id,"definitionId":definition_id,"reference":id,
                        "pose":{"at":{"x":70,"y":30},"rotation":0},"side":"front"
                    })).unwrap());
                }
                let mut settings = config();
                settings.plate_thickness = thickness;
                doc.mechanical = Some(settings);
                let outline = Contour { hole: false, points: vec![
                    Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 140.0, y: 0.0 },
                    Vec2 { x: 140.0, y: 60.0 }, Vec2 { x: 0.0, y: 60.0 },
                ] };
                let saved = doc.clone();
                let result = resolve(&doc, std::slice::from_ref(&outline));
                assert_eq!(doc, saved);
                assert!(!result.generation_blocked, "{:?}", result.diagnostics);
                assert!(!result.diagnostics.iter().any(|d| d.id.contains("engagement:")),
                    "{} at {thickness}: {:?}", definition.id, result.diagnostics);
                assert_eq!(result.plate_contours.iter().filter(|c| c.hole).count(), 3);
                let plate = result.stack.iter().find(|layer| layer.id == "plate").unwrap();
                assert!((plate.z + plate.thickness - 5.0).abs() < 1e-9);

                // Unknown/custom engagement remains authoritative, even for a
                // profile that uses the same stabilizer cutout geometry.
                let profile = doc.definitions[1].mechanical_profile.as_mut().unwrap();
                profile.switch_family = None;
                profile.plate_to_pcb = 4.0;
                let invalid = resolve(&doc, std::slice::from_ref(&outline));
                assert!(invalid.generation_blocked);
                assert!(invalid.diagnostics.iter().any(|d| d.id == "mechanical:engagement:stab"));

                // A low-profile switch cannot share the MX stabilizer stack.
                doc.definitions[1] = definition.clone();
                doc.mechanical.as_mut().unwrap().profiles.push(
                    builtin_profile("switch".into(), MechanicalBuiltinProfile::ChocV1Switch, 2.0).unwrap()
                );
                let incompatible = resolve(&doc, &[outline]);
                assert!(incompatible.generation_blocked);
                assert!(incompatible.diagnostics.iter().any(|d| d.id == "mechanical:engagement:stab"));
            }
        }
    }

    #[test]
    fn canonical_switch_profiles_produce_seventy_square_openings() {
        for source in ["ceoloide/switch_mx", "ceoloide/switch_choc_v1_v2"] {
            let definition_id = "switch-def";
            let mut doc = ProjectDoc::empty("split", "split");
            doc.definitions.push(serde_json::from_value(serde_json::json!({
                "id":definition_id,"name":"Switch","kind":"switch","courtyard":[],"pads":[],
                "generator":{"source":source,"version":"bundled-1","parameters":{"choc_v1_support":true,"choc_v2_support":false}}
            })).unwrap());
            for half in 0..2 {
                for row in 0..5 {
                    for column in 0..7 {
                        let index = half * 35 + row * 7 + column;
                        doc.parts.push(
                            serde_json::from_value(serde_json::json!({
                                "id":format!("switch-{index}"), "definitionId":definition_id,
                                "reference":format!("SW{index}"), "side":"back",
                                "pose":{"at":{"x":half as f64 * 160.0 + column as f64 * 19.0 + 15.0,
                                    "y": row as f64 * 19.0 + 15.0},"rotation":0}
                            }))
                            .unwrap(),
                        );
                    }
                }
            }
            doc.boards.push(Board {
                id: "board".into(),
                name: "Board".into(),
                outline_ids: vec![],
                part_ids: doc.parts.iter().map(|p| p.id.clone()).collect(),
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            let mut settings = config();
            settings.plate_thickness = if source.ends_with("switch_mx") {
                1.5
            } else {
                1.3
            };
            settings.plate_to_pcb = if source.ends_with("switch_mx") {
                3.5
            } else {
                2.2
            };
            settings.plate_foam_thickness = 0.0;
            doc.mechanical = Some(settings);
            let outlines = (0..2)
                .map(|half| {
                    let x = half as f64 * 160.0;
                    Contour {
                        hole: false,
                        points: vec![
                            Vec2 { x, y: 0.0 },
                            Vec2 {
                                x: x + 145.0,
                                y: 0.0,
                            },
                            Vec2 {
                                x: x + 145.0,
                                y: 105.0,
                            },
                            Vec2 { x, y: 105.0 },
                        ],
                    }
                })
                .collect::<Vec<_>>();
            let assembly = resolve(&doc, &outlines);
            assert!(!assembly.generation_blocked, "{:?}", assembly.diagnostics);
            let openings = assembly
                .plate_contours
                .iter()
                .filter(|c| c.hole)
                .collect::<Vec<_>>();
            assert_eq!(openings.len(), 70, "{definition_id}");
            for opening in openings {
                for axis in [true, false] {
                    let values = opening.points.iter().map(|p| if axis { p.x } else { p.y });
                    let min = values.clone().fold(f64::INFINITY, f64::min);
                    let max = values.fold(f64::NEG_INFINITY, f64::max);
                    assert!(
                        (max - min - 14.0).abs() < 0.001,
                        "{definition_id}: {min}..{max}"
                    );
                }
            }
        }
    }

    #[test]
    fn generator_switches_derive_their_fit_and_plate_gap() {
        for (source, parameters, family, plate_thickness, gap) in [
            (
                "ceoloide/switch_mx",
                serde_json::json!({}),
                MechanicalSwitchFamily::Mx,
                1.5,
                3.5,
            ),
            (
                "ceoloide/switch_choc_v1_v2",
                serde_json::json!({"choc_v1_support":true,"choc_v2_support":false}),
                MechanicalSwitchFamily::ChocV1,
                1.3,
                2.2,
            ),
            (
                "ceoloide/switch_choc_v1_v2",
                serde_json::json!({"choc_v1_support":{"value":false},"choc_v2_support":{"value":true}}),
                MechanicalSwitchFamily::ChocV2,
                1.5,
                3.5,
            ),
        ] {
            let mut doc = ProjectDoc::empty("switch-fit", "switch-fit");
            doc.boards.push(Board {
                id: "board".into(),
                name: "Board".into(),
                outline_ids: vec![],
                part_ids: vec!["switch".into()],
                net_ids: vec![],
                thickness: 1.6,
                traces: vec![],
                vias: vec![],
            });
            doc.parts.push(
                serde_json::from_value(serde_json::json!({
                    "id":"switch", "definitionId":"switch-def", "reference":"SW1",
                    "pose":{"at":{"x":50,"y":50},"rotation":0}, "side":"front"
                }))
                .unwrap(),
            );
            doc.definitions.push(
                serde_json::from_value(serde_json::json!({
                    "id":"switch-def", "name":"Switch", "kind":"switch", "courtyard":[], "pads":[],
                    "generator":{"source":source,"version":"1","parameters":parameters}
                }))
                .unwrap(),
            );
            let mut configuration = config();
            configuration.plate_thickness = plate_thickness;
            configuration.plate_to_pcb = gap;
            configuration.plate_foam_thickness = 0.0;
            doc.mechanical = Some(configuration);
            let outline = [Contour {
                hole: false,
                points: vec![
                    Vec2 { x: 0.0, y: 0.0 },
                    Vec2 { x: 100.0, y: 0.0 },
                    Vec2 { x: 100.0, y: 100.0 },
                    Vec2 { x: 0.0, y: 100.0 },
                ],
            }];

            let assembly = resolve(&doc, &outline);
            assert!(
                !assembly.generation_blocked,
                "{source}: {:#?}",
                assembly.diagnostics
            );
            let plate = assembly
                .stack
                .iter()
                .find(|layer| layer.id == "plate")
                .unwrap();
            assert_eq!(plate.z, gap, "{source} mounting datum gap");
            assert_eq!(plate.thickness, plate_thickness, "{source} plate preset");
            let cutout = assembly
                .plate_contours
                .iter()
                .find(|contour| contour.hole)
                .unwrap();
            let min_x = cutout
                .points
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min);
            let max_x = cutout
                .points
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max);
            let expected_width = match family {
                MechanicalSwitchFamily::Mx => 14.0,
                MechanicalSwitchFamily::ChocV1 => 14.0,
                MechanicalSwitchFamily::ChocV2 => 14.0,
            };
            assert!(
                (max_x - min_x - expected_width).abs() < 0.01,
                "{source} cutout width"
            );
        }
    }

    #[test]
    fn unknown_switch_requires_a_fit_choice_before_plate_generation() {
        let mut doc = ProjectDoc::empty("unknown-switch", "unknown-switch");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["switch".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(
            serde_json::from_value(serde_json::json!({
                "id":"switch", "definitionId":"imported", "reference":"SW1",
                "pose":{"at":{"x":50,"y":50},"rotation":0}, "side":"front"
            }))
            .unwrap(),
        );
        doc.definitions.push(serde_json::from_value(serde_json::json!({
            "id":"imported", "name":"Imported switch", "kind":"switch", "courtyard":[], "pads":[]
        })).unwrap());
        let configuration = config();
        doc.mechanical = Some(configuration);
        let outline = [Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 100.0, y: 0.0 },
                Vec2 { x: 100.0, y: 100.0 },
                Vec2 { x: 0.0, y: 100.0 },
            ],
        }];

        let unresolved = resolve(&doc, &outline);
        assert!(unresolved.generation_blocked);
        assert!(
            unresolved
                .diagnostics
                .iter()
                .any(|finding| finding.id == "mechanical:switch-family")
        );

        let family = MechanicalSwitchFamily::ChocV1;
        let mut profile = builtin_profile(
            "imported".into(),
            profile_source_for_family(family),
            switch_mounting_datum(family) - 1.3,
        )
        .unwrap();
        profile.switch_family = Some(family);
        let configuration = doc.mechanical.as_mut().unwrap();
        configuration.plate_thickness = 1.3;
        configuration.plate_to_pcb = 2.2;
        configuration.plate_foam_thickness = 2.0;
        configuration.profiles.push(profile);
        let selected = resolve(&doc, &outline);
        assert!(!selected.generation_blocked, "{:#?}", selected.diagnostics);
        assert_eq!(
            selected
                .stack
                .iter()
                .find(|layer| layer.id == "plate")
                .unwrap()
                .z,
            2.2
        );
    }
    #[test]
    fn duplicate_rings_ignore_winding_start_and_closure() {
        let a = Contour {
            hole: true,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 1.0, y: 0.0 },
                Vec2 { x: 1.0, y: 1.0 },
            ],
        };
        let b = Contour {
            hole: true,
            points: vec![a.points[1], a.points[0], a.points[2], a.points[1]],
        };
        assert!(same_ring(&a, &b));
    }
    #[test]
    fn profiles_follow_rotation_and_back_side_mirror() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":10,"y":20},"rotation":90},"side":"back"})).unwrap());
        let mut configuration = config();
        configuration.profiles.push(MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            switch_family: None,
            definition_id: "switch".into(),
            source: "custom".into(),
            cutouts: vec![vec![
                Vec2 { x: 1.0, y: 0.0 },
                Vec2 { x: 2.0, y: 0.0 },
                Vec2 { x: 1.0, y: 1.0 },
            ]],
            plate_to_pcb: 3.5,
            supported_thickness: None,
        });
        doc.mechanical = Some(configuration);
        let resolved = resolve(&doc, &[]);
        assert!((resolved.plate_contours[0].points[0].x - 10.0).abs() < 1e-9);
        assert!((resolved.plate_contours[0].points[0].y - 19.0).abs() < 1e-9);
    }
    #[test]
    fn combined_switch_stabilizer_deduplicates_and_regenerates_with_pose() {
        let mut doc = ProjectDoc::empty("test", "test");
        doc.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["key".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        doc.parts.push(serde_json::from_value(serde_json::json!({"id":"key","definitionId":"switch","reference":"SW1","pose":{"at":{"x":30,"y":30},"rotation":0},"side":"front"})).unwrap());
        let mut c = config();
        let mut profile =
            builtin_profile("switch".into(), MechanicalBuiltinProfile::MxSwitch, 3.5).unwrap();
        profile.cutouts.extend(
            builtin_profile("switch".into(), MechanicalBuiltinProfile::MxStab2u, 3.5)
                .unwrap()
                .cutouts,
        );
        let expected = profile.cutouts.len();
        c.profiles.push(profile);
        c.stabilizers = Some(vec![MechanicalStabilizerOverride {
            part_id: "key".into(),
            kind: MechanicalStabilizerKind::PcbMount,
            units: 2.0,
            rotation: None,
            profile: None,
        }]);
        doc.mechanical = Some(c);
        let first = resolve(&doc, &[]);
        assert_eq!(first.plate_contours.len(), expected);
        doc.parts[0].pose.at.x += 10.0;
        doc.revision += 1;
        let second = resolve(&doc, &[]);
        assert_eq!(second.plate_contours.len(), expected);
        assert!(
            (second.plate_contours[0].points[0].x - first.plate_contours[0].points[0].x - 10.0)
                .abs()
                < 1e-9
        );
        assert_eq!(second.revision, 1);
    }
    #[test]
    fn outer_expansion_preserves_nominal_holes() {
        let outer = Contour {
            hole: false,
            points: vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 20.0, y: 0.0 },
                Vec2 { x: 20.0, y: 20.0 },
                Vec2 { x: 0.0, y: 20.0 },
            ],
        };
        let hole = Contour {
            hole: true,
            points: vec![
                Vec2 { x: 5.0, y: 5.0 },
                Vec2 { x: 5.0, y: 10.0 },
                Vec2 { x: 10.0, y: 10.0 },
                Vec2 { x: 10.0, y: 5.0 },
            ],
        };
        let expanded = expanded_outline(&[outer, hole.clone()], 2.0, 1).unwrap();
        assert!(expanded.iter().any(|c| *c == hole));
        assert!(
            expanded
                .iter()
                .filter(|c| !c.hole)
                .flat_map(|c| c.points.iter())
                .any(|p| p.x < 0.0)
        );
    }
    #[test]
    fn builtin_profiles_have_library_cutouts() {
        for source in [
            MechanicalBuiltinProfile::MxStab2u,
            MechanicalBuiltinProfile::MxStab625u,
        ] {
            let profile = builtin_profile("switch".into(), source, 3.5).unwrap();
            assert!(!profile.cutouts.is_empty());
            assert!(profile.cutouts.iter().all(|p| p.len() >= 3));
            assert!(profile.supported_thickness.is_none());
        }
        for (source, family, cutout_width, provenance) in [
            (
                MechanicalBuiltinProfile::MxSwitch,
                MechanicalSwitchFamily::Mx,
                14.0,
                "Cherry MX datasheet",
            ),
            (
                MechanicalBuiltinProfile::ChocV1Switch,
                MechanicalSwitchFamily::ChocV1,
                14.0,
                "CPG135001D01-16",
            ),
            (
                MechanicalBuiltinProfile::ChocV2Switch,
                MechanicalSwitchFamily::ChocV2,
                14.0,
                "CPG135301D03",
            ),
        ] {
            let profile = builtin_profile("switch".into(), source, 2.0).unwrap();
            assert_eq!(profile.switch_family, Some(family));
            assert!(profile.source.contains(provenance));
            let min_x = profile.cutouts[0]
                .iter()
                .map(|point| point.x)
                .fold(f64::INFINITY, f64::min);
            let max_x = profile.cutouts[0]
                .iter()
                .map(|point| point.x)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!((max_x - min_x - cutout_width).abs() < 0.001);
            assert!(profile.supported_thickness.is_some());
        }
    }
    #[test]
    fn rejects_impossible_foam_and_rigid_gasket() {
        let mut doc = ProjectDoc::empty("test", "test");
        let mut c = config();
        c.plate_foam_thickness = 4.0;
        c.mount = MechanicalMount::Gasket;
        c.integrated_plate_frame = true;
        doc.mechanical = Some(c);
        let result = resolve(&doc, &[]);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.id == "mechanical:foam-engagement")
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.id == "mechanical:gasket-rigid-frame")
        );
    }
}

/// Reviewed library fit geometry with a manufacturer-sourced switch datum.
pub fn builtin_profile(
    definition_id: String,
    source: MechanicalBuiltinProfile,
    plate_to_pcb: f64,
) -> Result<MechanicalPartProfile, String> {
    use crate::artifact::mechanical_extract::{
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
                "../tests/fixtures/mechanical/STAB_MX_2u.kicad_mod"
            )),
            None,
            None,
        ),
        MechanicalBuiltinProfile::MxStab625u => (
            "STAB_MX_6.25u",
            Some(include_str!(
                "../tests/fixtures/mechanical/STAB_MX_6.25u.kicad_mod"
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
