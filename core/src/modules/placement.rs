use super::*;
use std::collections::BTreeSet;

fn finite_polygon(points: &[Vec2]) -> bool {
    (3..=100_000).contains(&points.len())
        && points.iter().all(|p| p.x.is_finite() && p.y.is_finite())
}

pub(super) fn validate_definition(def: &ModuleDefinition) -> Result<(), String> {
    if def.id.trim().is_empty()
        || def.id.len() > 256
        || def.name.trim().is_empty()
        || def.source.repository.trim().is_empty()
        || def.source.revision.trim().is_empty()
    {
        return Err("A module requires a named, pinned source snapshot".into());
    }
    if def
        .board
        .thickness
        .is_some_and(|t| !t.is_finite() || t <= 0.0)
        || def.board.contours.len() > 1000
        || def
            .board
            .contours
            .iter()
            .any(|c| !finite_polygon(&c.points))
        || def.board.holes.len() > 10000
        || def.board.holes.iter().any(|hole| !finite_polygon(hole))
    {
        return Err(
            "Module board geometry requires finite closed polygons and a positive thickness".into(),
        );
    }
    let mut ids = BTreeSet::new();
    for volume in def.volumes.iter().chain(&def.openings) {
        if volume.id.is_empty()
            || !ids.insert(&volume.id)
            || !finite_polygon(&volume.geometry.points)
            || !volume.geometry.z.is_finite()
            || !volume.geometry.height.is_finite()
            || volume.geometry.height <= 0.0
            || volume.source.trim().is_empty()
        {
            return Err("Module volumes require unique identities, source evidence and positive finite height".into());
        }
    }
    for mount in &def.mounts {
        if mount.source_id.is_empty()
            || !mount.at.x.is_finite()
            || !mount.at.y.is_finite()
            || !mount.diameter.is_finite()
            || mount.diameter <= 0.0
        {
            return Err(
                "Module mounting holes require finite positions and positive diameters".into(),
            );
        }
    }
    let mut ports = BTreeSet::new();
    for model in &def.models {
        let values = [
            model.offset.x,
            model.offset.y,
            model.offset.z,
            model.rotation.x,
            model.rotation.y,
            model.rotation.z,
            model.scale.x,
            model.scale.y,
            model.scale.z,
        ];
        if model.asset_id.trim().is_empty()
            || values.iter().any(|value| !value.is_finite())
            || [model.scale.x, model.scale.y, model.scale.z]
                .iter()
                .any(|value| *value <= 0.0)
        {
            return Err(
                "Module model transforms require finite coordinates and positive scale".into(),
            );
        }
    }
    for port in &def.interfaces {
        if port.id.is_empty()
            || !ports.insert(&port.id)
            || port.signals.len() != 12
            || port.signals != vik_signals(port.role)
        {
            return Err("A VIK port must retain its explicit host/module role and all twelve contact mappings".into());
        }
    }
    Ok(())
}

fn validate_mount_supports(
    instance: &MountedModule,
    definition: &ModuleDefinition,
) -> Result<(), String> {
    if instance.mount_supports.is_empty() {
        return Ok(());
    }
    let thickness = definition
        .board
        .thickness
        .ok_or("Module board thickness is required to place a mount support")?;
    let mut seen = BTreeSet::new();
    for support in &instance.mount_supports {
        if support.mount_id.trim().is_empty()
            || !seen.insert(&support.mount_id)
            || !support.outer_diameter.is_finite()
            || !support.hole_diameter.is_finite()
            || !support.z.is_finite()
            || !support.height.is_finite()
            || !(support.z + support.height).is_finite()
            || support.hole_diameter <= 0.0
            || support.outer_diameter <= support.hole_diameter
            || support.height <= 0.0
        {
            return Err("Module supports require unique source holes and finite positive annulus dimensions".into());
        }
        let hole = definition
            .mounts
            .iter()
            .find(|mount| mount.source_id == support.mount_id)
            .ok_or_else(|| {
                format!(
                    "Module support references missing source hole {}",
                    support.mount_id
                )
            })?;
        if support.hole_diameter + 1e-6 < hole.diameter {
            return Err(format!(
                "Module support hole {} is smaller than the source PCB drill",
                support.mount_id
            ));
        }
        let half = thickness / 2.0;
        match instance.attachment {
            ModuleAttachment::Case => {
                let end = support.z + support.height;
                let extends_above = (support.z - half).abs() <= 1e-6 && end > half + 1e-6;
                let extends_below = (end + half).abs() <= 1e-6 && support.z < -half - 1e-6;
                if !extends_above && !extends_below {
                    return Err(format!(
                        "Case support {} must start at an exposed module PCB face and extend away from the board",
                        support.mount_id
                    ));
                }
            }
            ModuleAttachment::Board => {
                if !instance.gap.is_finite() || instance.gap <= 0.0 {
                    return Err(
                        "Board-attached standoffs require a positive module-to-host gap".into(),
                    );
                }
                let (expected_z, expected_height) = match instance.facing_face {
                    Side::Front => (half, instance.gap),
                    Side::Back => (-half - instance.gap, instance.gap),
                };
                if (support.z - expected_z).abs() > 1e-6
                    || (support.height - expected_height).abs() > 1e-6
                {
                    return Err(format!(
                        "Board support {} must span the selected module-to-host gap from its facing PCB surface",
                        support.mount_id
                    ));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn vik_signals(role: VikRole) -> Vec<VikSignal> {
    use VikSignal::*;
    let mut signals = vec![
        Sclk, Miso, Cs, Gpio2, Mosi, Gpio1, V5, Rgb, Scl, Sda, Gnd, V3v3,
    ];
    if role == VikRole::Host {
        signals.reverse();
    }
    signals
}

pub(crate) fn set_definition(
    doc: &mut ProjectDoc,
    definition: &ModuleDefinition,
) -> Result<Vec<String>, String> {
    validate_definition(definition)?;
    if let Some(old) = doc
        .module_definitions
        .iter_mut()
        .find(|item| item.id == definition.id)
    {
        if old.source != definition.source
            || old.circuit != definition.circuit
            || old.interfaces != definition.interfaces
        {
            return Err("Keep source provenance, circuit and connector mappings intact; revised source circuits require a new variant identity".into());
        }
        for gate in old
            .gates
            .iter()
            .filter(|gate| gate.output == HardwareOutput::Electrical)
        {
            if !definition.gates.contains(gate) {
                return Err(
                    "A source electrical caveat requires an explicit repaired circuit variant"
                        .into(),
                );
            }
        }
        for gate in old
            .gates
            .iter()
            .filter(|gate| gate.output == HardwareOutput::Mechanical)
        {
            if !definition.gates.contains(gate)
                && (gate.code != "assembled-envelope"
                    || !definition
                        .volumes
                        .iter()
                        .any(|volume| volume.purpose == "occupied" && volume.qualified)
                    || definition
                        .volumes
                        .iter()
                        .chain(&definition.openings)
                        .any(|volume| !volume.qualified))
            {
                return Err("Keep missing mechanical evidence gated until the replacement assembly geometry is reviewed".into());
            }
        }
        *old = definition.clone();
    } else {
        if doc.module_definitions.len() >= 512 {
            return Err("Module definition limit exceeded".into());
        }
        doc.module_definitions.push(definition.clone());
    }
    let mut affected = vec![definition.id.clone()];
    for instance in doc
        .modules
        .iter()
        .filter(|instance| instance.definition_id == definition.id)
    {
        affected.extend([instance.id.clone(), instance.host_board_id.clone()]);
    }
    Ok(affected)
}

pub(crate) fn set(
    doc: &mut ProjectDoc,
    instance: &MountedModule,
    definition: Option<&ModuleDefinition>,
    host_connector_definition: Option<&PartDefinition>,
) -> Result<Vec<String>, String> {
    if instance.id.trim().is_empty()
        || instance.id.len() > 256
        || !instance.at.x.is_finite()
        || !instance.at.y.is_finite()
        || !instance.rotation.is_finite()
        || !instance.gap.is_finite()
        || instance.gap < 0.0
        || !instance.service_clearance.is_finite()
        || instance.service_clearance < 0.0
    {
        return Err("Module placement requires an identity, finite coordinates and nonnegative gap and service clearance".into());
    }
    let board = doc
        .boards
        .iter()
        .find(|b| b.id == instance.host_board_id)
        .ok_or("The module host PCB is missing")?;
    if !board.thickness.is_finite() || board.thickness <= 0.0 {
        return Err("The host PCB thickness must be positive".into());
    }
    if let Some(id) = &instance.host_instance_id {
        if !doc.hardware.as_ref().is_some_and(|h| {
            h.instances
                .iter()
                .any(|i| &i.id == id && i.board_id == board.id)
        }) {
            return Err(
                "The module host assembly instance is missing or belongs to another PCB".into(),
            );
        }
    }
    if let Some(def) = definition {
        if def.id != instance.definition_id {
            return Err("Module placement and definition identities differ".into());
        }
        validate_definition(def)?;
        validate_mount_supports(instance, def)?;
        if let Some(old) = doc.module_definitions.iter().find(|d| d.id == def.id) {
            if old != def {
                return Err("The module definition is already an independent project snapshot; choose a new variant identity".into());
            }
        } else {
            doc.module_definitions.push(def.clone());
        }
    }
    let def = doc
        .module_definitions
        .iter()
        .find(|d| d.id == instance.definition_id)
        .ok_or("Module definition is missing")?;
    validate_definition(def)?;
    validate_mount_supports(instance, def)?;
    let module_definition = def.clone();
    let mut saved_instance = instance.clone();
    if let Some(connector_definition) = host_connector_definition {
        super::host_connector::add_for_mount(
            doc,
            &mut saved_instance,
            &module_definition,
            connector_definition,
        )?;
    }
    if let Some(existing) = doc.modules.iter_mut().find(|m| m.id == instance.id) {
        *existing = saved_instance.clone();
    } else {
        if doc.modules.len() >= 128 {
            return Err("Module instance limit exceeded".into());
        }
        doc.modules.push(saved_instance.clone());
    }
    let mut affected = vec![instance.id.clone(), instance.host_board_id.clone()];
    if let Some(connection) = saved_instance.connection {
        if !connection.host_connector_part_id.is_empty() {
            affected.push(connection.host_connector_part_id);
        }
    }
    Ok(affected)
}

pub(crate) fn remove(doc: &mut ProjectDoc, id: &str) -> Result<Vec<String>, String> {
    let index = doc
        .modules
        .iter()
        .position(|m| m.id == id)
        .ok_or("Module instance is missing")?;
    let removed = doc.modules.remove(index);
    Ok(vec![removed.id, removed.host_board_id])
}

fn xy(instance: &MountedModule, flipped: bool, p: Vec2) -> Vec2 {
    let (sin, cos) = instance.rotation.to_radians().sin_cos();
    let x = if flipped { -p.x } else { p.x };
    Vec2 {
        x: instance.at.x + x * cos - p.y * sin,
        y: instance.at.y + x * sin + p.y * cos,
    }
}

fn resolved_footprints(
    instance: &MountedModule,
    def: &ModuleDefinition,
    flipped: bool,
) -> Vec<ResolvedModuleFootprint> {
    let Some(circuit) = &def.circuit else {
        return Vec::new();
    };
    circuit
        .parts
        .iter()
        .filter_map(|part| {
            let definition = circuit
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)?;
            let local_rotation = if flipped {
                -part.pose.rotation
            } else {
                part.pose.rotation
            };
            Some(ResolvedModuleFootprint {
                id: format!("{}/footprint/{}", instance.id, part.id),
                source_part_id: part.id.clone(),
                reference: part.reference.clone(),
                definition_id: definition.id.clone(),
                name: definition.name.clone(),
                pose: Pose2 {
                    at: xy(instance, flipped, part.pose.at),
                    rotation: instance.rotation + local_rotation,
                },
                side: if flipped {
                    opposite_side(part.side.clone())
                } else {
                    part.side.clone()
                },
                courtyard: definition.courtyard.clone(),
                pads: definition
                    .pads
                    .iter()
                    .map(|pad| ResolvedModulePad {
                        id: pad.id.clone(),
                        number: pad.number.clone(),
                        at: pad.at,
                        size: pad.size,
                        shape: pad.shape.clone(),
                        drill: pad.drill,
                        rotation: pad.rotation,
                    })
                    .collect(),
                surfaces: definition
                    .kicad_source
                    .as_ref()
                    .and_then(|source| {
                        crate::artifact::preview_footprint_surfaces(&source.source).ok()
                    })
                    .unwrap_or_default()
                    .into_iter()
                    .map(|mut surface| {
                        if flipped {
                            surface.layer = match surface.layer.as_str() {
                                "F.SilkS" => "B.SilkS".into(),
                                "B.SilkS" => "F.SilkS".into(),
                                "F.Fab" => "B.Fab".into(),
                                "B.Fab" => "F.Fab".into(),
                                _ => surface.layer,
                            };
                        }
                        surface
                    })
                    .collect(),
            })
        })
        .collect()
}

fn opposite_side(side: Side) -> Side {
    match side {
        Side::Front => Side::Back,
        Side::Back => Side::Front,
    }
}

fn volume(
    instance: &MountedModule,
    flipped: bool,
    center: f64,
    source: &CaseOpening,
) -> CaseOpening {
    CaseOpening {
        points: source
            .points
            .iter()
            .map(|p| xy(instance, flipped, *p))
            .collect(),
        z: if flipped {
            center - source.z - source.height
        } else {
            center + source.z
        },
        height: source.height,
    }
}

pub(crate) fn resolved_mount_supports(
    instance: &MountedModule,
    module: &ResolvedModule,
) -> Vec<ResolvedModuleSupport> {
    let Some(midplane) = module
        .board
        .first()
        .map(|board| board.z + board.height / 2.0)
    else {
        return Vec::new();
    };
    instance
        .mount_supports
        .iter()
        .filter_map(|support| {
            let mount = module
                .mounts
                .iter()
                .find(|mount| mount.source_id == support.mount_id)?;
            Some(ResolvedModuleSupport {
                mount_id: support.mount_id.clone(),
                at: mount.at,
                outer_diameter: support.outer_diameter,
                hole_diameter: support.hole_diameter,
                z: if module.flipped {
                    midplane - support.z - support.height
                } else {
                    midplane + support.z
                },
                height: support.height,
            })
        })
        .collect()
}

fn issue(
    result: &mut ModuleResolution,
    instance: &MountedModule,
    code: &str,
    message: &str,
    severity: Severity,
    scope: Scope,
) {
    result.findings.push(Finding {
        id: format!("module/{}/{code}", instance.id),
        message: message.into(),
        severity,
        scope,
        target_ids: vec![instance.id.clone(), instance.host_board_id.clone()],
    });
}

pub(crate) fn resolve(doc: &ProjectDoc, board_id: &str) -> ModuleResolution {
    let mut result = ModuleResolution {
        revision: doc.revision,
        modules: vec![],
        findings: vec![],
        markers: vec![],
        preview: None,
        model_placements: vec![],
    };
    if doc.modules.is_empty() && doc.embedded_circuits.is_empty() {
        return result;
    }
    let Some(host) = doc.boards.iter().find(|b| b.id == board_id) else {
        return result;
    };
    for source_instance in super::active_instances(doc, board_id) {
        let mut physical = source_instance.clone();
        if let Some(id) = doc.physical_instance_id.as_ref() {
            let Some(host) = doc.hardware.as_ref().and_then(|h| {
                h.instances
                    .iter()
                    .find(|i| &i.id == id && i.board_id == board_id)
            }) else {
                issue(
                    &mut result,
                    source_instance,
                    "physical-host",
                    "The selected physical host instance is missing",
                    Severity::Error,
                    Scope::Case,
                );
                continue;
            };
            if host.flipped {
                physical.at.x = -physical.at.x;
                physical.rotation = -physical.rotation;
                physical.host_face = if physical.host_face == Side::Front {
                    Side::Back
                } else {
                    Side::Front
                };
            }
        }
        let instance = &physical;
        let Some(def) = doc
            .module_definitions
            .iter()
            .find(|d| d.id == instance.definition_id)
        else {
            issue(
                &mut result,
                instance,
                "definition",
                "The mounted module snapshot is missing",
                Severity::Error,
                Scope::Case,
            );
            continue;
        };
        if let Err(message) = validate_definition(def) {
            issue(
                &mut result,
                instance,
                "geometry",
                &message,
                Severity::Error,
                Scope::Case,
            );
            continue;
        }
        if let Err(message) = validate_mount_supports(instance, def) {
            issue(
                &mut result,
                instance,
                "mount-support",
                &message,
                Severity::Error,
                Scope::Case,
            );
            if instance.attachment == ModuleAttachment::Board {
                issue(
                    &mut result,
                    instance,
                    "host-drill",
                    "The configured board standoff does not resolve to a safe host PCB drill.",
                    Severity::Error,
                    Scope::Pcb,
                );
            }
            continue;
        }
        let Some(thickness) = def.board.thickness else {
            issue(
                &mut result,
                instance,
                "thickness",
                "Module thickness is unknown; exact mechanical output requires a verified thickness",
                Severity::Error,
                Scope::Case,
            );
            continue;
        };
        if (!def.constituents.is_empty()
            || def
                .circuit
                .as_ref()
                .is_some_and(|circuit| !circuit.parts.is_empty()))
            && !def
                .volumes
                .iter()
                .any(|volume| volume.purpose == "occupied" && volume.qualified)
        {
            issue(
                &mut result,
                instance,
                "assembled-envelope",
                "Module assembly occupancy remains unknown; measured component geometry is required before exact case output",
                Severity::Error,
                Scope::Case,
            );
        }
        if def.board.contours.is_empty() || !host.thickness.is_finite() || host.thickness <= 0.0 {
            issue(
                &mut result,
                instance,
                "board",
                "Module outline or host thickness is unavailable",
                Severity::Error,
                Scope::Case,
            );
            continue;
        }
        let host_sign = if instance.host_face == Side::Front {
            1.0
        } else {
            -1.0
        };
        let face_sign = if instance.facing_face == Side::Front {
            1.0
        } else {
            -1.0
        };
        let flipped = host_sign * face_sign > 0.0;
        let midplane_z = host_sign * (host.thickness / 2.0 + instance.gap + thickness / 2.0);
        let top_zero_center = midplane_z - host.thickness / 2.0;
        let board = def
            .board
            .contours
            .iter()
            .filter(|c| !c.hole)
            .map(|c| {
                volume(
                    instance,
                    flipped,
                    top_zero_center,
                    &CaseOpening {
                        points: c.points.clone(),
                        z: -thickness / 2.0,
                        height: thickness,
                    },
                )
            })
            .collect();
        let transform = |v: &ModuleVolume| ModuleVolume {
            geometry: volume(instance, flipped, top_zero_center, &v.geometry),
            ..v.clone()
        };
        let mut resolved = ResolvedModule {
            id: instance.id.clone(),
            definition_id: def.id.clone(),
            at: instance.at,
            rotation: instance.rotation,
            midplane_z,
            flipped,
            board,
            board_holes: board_holes(def, instance, flipped),
            volumes: def.volumes.iter().map(transform).collect(),
            openings: def.openings.iter().map(transform).collect(),
            mounts: def
                .mounts
                .iter()
                .map(|mount| MechanicalPcbHole {
                    at: xy(instance, flipped, mount.at),
                    ..mount.clone()
                })
                .collect(),
            mount_supports: Vec::new(),
            footprints: resolved_footprints(instance, def, flipped),
            models: def.models.clone(),
            gates: def.gates.clone(),
        };
        resolved.mount_supports = resolved_mount_supports(instance, &resolved)
            .into_iter()
            .map(|support| ModuleSupportGeometry {
                mount_id: support.mount_id,
                at: support.at,
                outer_diameter: support.outer_diameter,
                hole_diameter: support.hole_diameter,
                z: support.z,
                height: support.height,
            })
            .collect();
        if instance.attachment == ModuleAttachment::Board {
            let configured = resolved
                .mount_supports
                .iter()
                .map(|support| support.mount_id.as_str())
                .collect::<BTreeSet<_>>();
            for mount in &resolved.mounts {
                if !configured.contains(mount.source_id.as_str()) {
                    issue(
                        &mut result,
                        instance,
                        &format!("host-drill/{}", mount.source_id),
                        &format!(
                            "Module mount {} needs designer-defined PCB standoff and host drill dimensions.",
                            mount.source_id
                        ),
                        Severity::Error,
                        Scope::Pcb,
                    );
                }
            }
        }
        for gate in &def.gates {
            let mechanical = gate.output == HardwareOutput::Mechanical;
            issue(
                &mut result,
                instance,
                &format!("gate/{}", gate.code),
                &gate.message,
                if mechanical {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                if mechanical {
                    Scope::Case
                } else {
                    Scope::Layout
                },
            );
        }
        if instance.connection.is_none() && def.electrical.protocol != ModuleProtocol::PassThrough {
            issue(
                &mut result,
                instance,
                "connection",
                "Assign a VIK host connector, power and controller resources for this module",
                Severity::Warning,
                Scope::Layout,
            );
        }
        result.modules.push(resolved);
    }
    result
        .findings
        .extend(super::connection_findings(doc, board_id));
    super::occupancy::collisions(doc, board_id, &mut result);
    result
}

fn board_holes(def: &ModuleDefinition, instance: &MountedModule, flipped: bool) -> Vec<Contour> {
    let mut holes: Vec<_> = def
        .board
        .holes
        .iter()
        .map(|points| Contour {
            points: points.clone(),
            hole: true,
        })
        .chain(
            def.board
                .contours
                .iter()
                .filter(|contour| contour.hole)
                .cloned(),
        )
        .collect();
    for mount in &def.mounts {
        if holes.iter().any(|hole| {
            let center = hole.points.iter().fold(Vec2::default(), |sum, p| Vec2 {
                x: sum.x + p.x,
                y: sum.y + p.y,
            });
            ((center.x / hole.points.len() as f64 - mount.at.x).powi(2)
                + (center.y / hole.points.len() as f64 - mount.at.y).powi(2))
            .sqrt()
                < 0.02
        }) {
            continue;
        }
        holes.push(Contour {
            hole: true,
            points: (0..64)
                .map(|i| {
                    let angle = std::f64::consts::TAU * i as f64 / 64.0;
                    Vec2 {
                        x: mount.at.x + mount.diameter / 2.0 * angle.cos(),
                        y: mount.at.y + mount.diameter / 2.0 * angle.sin(),
                    }
                })
                .collect(),
        });
    }
    for hole in &mut holes {
        for point in &mut hole.points {
            *point = xy(instance, flipped, *point)
        }
    }
    holes
}

pub(crate) fn finding_markers(doc: &ProjectDoc, findings: &[Finding]) -> Vec<FindingMarker> {
    if doc.modules.is_empty() && doc.embedded_circuits.is_empty() {
        return Vec::new();
    }
    let mut markers = Vec::new();
    for board in &doc.boards {
        let resolved = resolve(doc, &board.id);
        markers.extend(resolved.markers.clone());
        for finding in findings.iter().filter(|f| f.id.starts_with("module/")) {
            if resolved.markers.iter().any(|m| m.finding_id == finding.id) {
                continue;
            }
            let mut contours = Vec::new();
            for module in resolved
                .modules
                .iter()
                .filter(|m| finding.target_ids.contains(&m.id))
            {
                let specific: Vec<_> = module
                    .volumes
                    .iter()
                    .chain(&module.openings)
                    .filter(|v| {
                        finding
                            .target_ids
                            .contains(&format!("{}/{}", module.id, v.id))
                    })
                    .collect();
                if specific.is_empty() {
                    contours.extend(module.board.iter().map(|v| Contour {
                        points: v.points.clone(),
                        hole: false,
                    }));
                } else {
                    contours.extend(specific.into_iter().map(|v| Contour {
                        points: v.geometry.points.clone(),
                        hole: false,
                    }));
                }
            }
            for circuit in doc
                .embedded_circuits
                .iter()
                .filter(|c| c.host_board_id == board.id && finding.target_ids.contains(&c.id))
            {
                let specific = finding
                    .target_ids
                    .iter()
                    .any(|id| circuit.part_ids.contains(id));
                for part in doc.parts.iter().filter(|p| {
                    circuit.part_ids.contains(&p.id)
                        && (!specific || finding.target_ids.contains(&p.id))
                }) {
                    if let Some(def) = doc.definitions.iter().find(|d| d.id == part.definition_id) {
                        let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
                        contours.push(Contour {
                            points: def
                                .courtyard
                                .iter()
                                .map(|p| {
                                    let x = if part.side == Side::Back { -p.x } else { p.x };
                                    Vec2 {
                                        x: part.pose.at.x + x * cos - p.y * sin,
                                        y: part.pose.at.y + x * sin + p.y * cos,
                                    }
                                })
                                .collect(),
                            hole: false,
                        });
                    }
                }
            }
            if !contours.is_empty() {
                markers.push(FindingMarker {
                    finding_id: finding.id.clone(),
                    board_id: board.id.clone(),
                    contours,
                });
            }
        }
    }
    markers
}
