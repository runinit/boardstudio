use crate::model::{
    CaseKind, Finding, FindingMarker, HardwareOutput, MatrixScene, MountKind, ProjectDoc,
    ResolvedModule, Scope, Severity,
};
use std::collections::{BTreeMap, BTreeSet};

/// Rejects duplicate identities in each top-level identity collection.
///
/// Duplicates are compared within one collection, not across entity kinds. Admission runs
/// this before scripts or layout normalization, and publication runs it again on the
/// generated document, so a later stage cannot hide or introduce an ambiguous identity.
pub(crate) fn duplicate_top_level_identity(doc: &ProjectDoc) -> Option<String> {
    fn duplicate<'a>(kind: &str, ids: impl Iterator<Item = &'a str>) -> Option<String> {
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id) {
                return Some(format!("Duplicate {kind} identity {id:?}"));
            }
        }
        None
    }
    duplicate("part", doc.parts.iter().map(|entry| entry.id.as_str()))
        .or_else(|| {
            duplicate(
                "definition",
                doc.definitions.iter().map(|entry| entry.id.as_str()),
            )
        })
        .or_else(|| duplicate("board", doc.boards.iter().map(|entry| entry.id.as_str())))
        .or_else(|| duplicate("matrix", doc.matrices.iter().map(|entry| entry.id.as_str())))
        .or_else(|| duplicate("net", doc.nets.iter().map(|entry| entry.id.as_str())))
        .or_else(|| duplicate("constraint", doc.constraints.iter().map(|entry| entry.id())))
}

fn error(scope: Scope, id: String, message: &str, targets: Vec<String>) -> Finding {
    Finding {
        id,
        severity: Severity::Error,
        scope,
        message: message.into(),
        target_ids: targets,
    }
}

pub(super) struct SceneDiagnostics {
    pub findings: Vec<Finding>,
    pub hardware_markers: Vec<FindingMarker>,
    pub keycaps: Vec<crate::keycaps::KeycapProjection>,
    pub modules: Vec<ResolvedModule>,
    pub matrices: Vec<Option<MatrixScene>>,
}

pub(super) fn scene_diagnostics(doc: &ProjectDoc) -> SceneDiagnostics {
    let (mut findings, hardware_markers) = crate::hardware::nominal_fit(doc);
    let mut keycaps = vec![];
    let mut modules = vec![];
    findings.extend(crate::generators::parameter_findings(doc));
    for board in &doc.boards {
        if doc.keycaps.is_some() {
            let mut keycap_projection = crate::keycaps::project(doc, &board.id, None);
            findings.append(&mut keycap_projection.resolution.findings);
            keycaps.push(keycap_projection);
        }
        let module_projection = crate::modules::resolve(doc, &board.id);
        findings.extend(module_projection.findings);
        modules.extend(module_projection.modules);
        findings.extend(crate::modules::embedded_findings(doc, &board.id));
    }
    for part in &doc.parts {
        let Some(def) = doc.definitions.iter().find(|d| d.id == part.definition_id) else {
            continue;
        };
        if crate::inputs::profile(def).rotary.is_some() && crate::inputs::matrix_member(doc, part) {
            findings.push(Finding{id:format!("input/{}/assembly-fit",part.id),scope:Scope::Layout,severity:Severity::Warning,
                message:"The encoder retains this key's bindings and companion parts. Review companion clearance and its own knob or wheel envelope; the former keycap is not a qualified encoder assembly.".into(),target_ids:vec![part.id.clone()]});
        }
        for gate in def.hardware_profile.iter().flat_map(|p| &p.gates) {
            let (scope, severity) = match gate.output {
                HardwareOutput::Footprint | HardwareOutput::Electrical => {
                    (Scope::Pcb, Severity::Error)
                }
                HardwareOutput::Mechanical => (Scope::Case, Severity::Error),
                HardwareOutput::Model | HardwareOutput::Firmware => {
                    (Scope::Layout, Severity::Warning)
                }
            };
            findings.push(Finding {
                id: format!("hardware/{}/{}", part.id, gate.code),
                scope,
                severity,
                message: gate.message.clone(),
                target_ids: vec![part.id.clone()],
            });
        }
    }
    let parts_by_id: BTreeMap<_, _> = doc
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let definitions_by_id: BTreeMap<_, _> = doc
        .definitions
        .iter()
        .map(|def| (def.id.as_str(), def))
        .collect();
    let mut parts_by_definition: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for part in &doc.parts {
        parts_by_definition
            .entry(part.definition_id.as_str())
            .or_default()
            .push(part.id.as_str());
    }
    let outline_ids: BTreeSet<_> = doc.outline.iter().map(|feature| feature.id()).collect();
    let net_ids: BTreeSet<_> = doc.nets.iter().map(|net| net.id.as_str()).collect();
    for def in &doc.definitions {
        let Some(parts) = parts_by_definition.get(def.id.as_str()) else {
            continue;
        };
        let mut report = |suffix: String, message: &str| {
            let targets = std::iter::once(def.id.as_str())
                .chain(parts.iter().copied())
                .map(str::to_owned)
                .collect();
            findings.push(error(
                Scope::Pcb,
                format!("definition:{}:{suffix}", def.id),
                message,
                targets,
            ));
        };
        if def.id.trim().is_empty() {
            report("id".into(), "Definition ID must not be empty");
        }
        if def
            .courtyard
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            report("courtyard".into(), "Courtyard points must be finite");
        }
        let mut pad_ids = BTreeSet::new();
        for (index, pad) in def.pads.iter().enumerate() {
            let key = if pad.id.is_empty() {
                index.to_string()
            } else {
                pad.id.clone()
            };
            if pad.id.trim().is_empty() {
                report(format!("pad:{key}:id"), "Pad ID must not be empty");
            }
            if !pad_ids.insert(&pad.id) {
                report(format!("pad:{key}:duplicate-id"), "Pad IDs must be unique");
            }
            if !pad.at.x.is_finite() || !pad.at.y.is_finite() {
                report(format!("pad:{key}:position"), "Pad position must be finite");
            }
            if !pad.size.x.is_finite()
                || !pad.size.y.is_finite()
                || pad.size.x <= 0.0
                || pad.size.y <= 0.0
            {
                report(
                    format!("pad:{key}:size"),
                    "Pad size must be positive and finite",
                );
            }
            if pad
                .drill
                .is_some_and(|drill| !drill.is_finite() || drill <= 0.0)
            {
                report(
                    format!("pad:{key}:drill"),
                    "Pad drill must be positive and finite",
                );
            }
        }
    }
    for board in &doc.boards {
        if let Some(state) = doc
            .board_outlines
            .iter()
            .find(|state| state.board_id == board.id)
            && let Some(id) = &state.active_version_id
        {
            if let Some(version) = state.versions.iter().find(|version| &version.id == id) {
                // A linked refinement follows the board's components, so its authored
                // attachments are legitimate; a fixed edited copy cannot contain one.
                if version.ownership.is_fixed() {
                    for feature in &version.geometry.features {
                        let fixed = matches!(
                            feature,
                            crate::model::OutlineFeature::Polygon {
                                anchor_part_id: None,
                                ..
                            } | crate::model::OutlineFeature::Rect {
                                anchor_part_id: None,
                                ..
                            }
                        );
                        if !fixed {
                            findings.push(error(
                                Scope::Pcb,
                                format!(
                                    "board:{}:outline:fixed-attachment:{}",
                                    board.id,
                                    feature.id()
                                ),
                                "A fixed version cannot contain live component attachments",
                                vec![board.id.clone(), feature.id().into()],
                            ));
                        }
                    }
                }
            } else {
                findings.push(error(
                    Scope::Pcb,
                    format!("board:{}:outline:version", board.id),
                    "The active outline version is missing; select Generated or another version",
                    vec![board.id.clone()],
                ));
            }
        }
        if !board.thickness.is_finite() || board.thickness <= 0.0 {
            findings.push(error(
                Scope::Pcb,
                format!("board:{}:thickness", board.id),
                "Board thickness must be positive",
                vec![board.id.clone()],
            ));
        }
        if board.outline_ids.is_empty()
            && crate::outline_versions::active_snapshot(doc, &board.id).is_none()
        {
            findings.push(error(
                Scope::Pcb,
                format!("board:{}:outline", board.id),
                "Board has no outline feature",
                vec![board.id.clone()],
            ));
        }
        for id in &board.outline_ids {
            if !outline_ids.contains(id.as_str())
                && crate::outline_versions::active_snapshot(doc, &board.id).is_none()
            {
                findings.push(error(
                    Scope::Pcb,
                    format!("board:{}:outline:{}", board.id, id),
                    "Board outline feature is missing",
                    vec![board.id.clone(), id.clone()],
                ));
            }
        }
        for id in &board.part_ids {
            if !parts_by_id.contains_key(id.as_str()) {
                findings.push(error(
                    Scope::Pcb,
                    format!("board:{}:part:{}", board.id, id),
                    "Board part is missing",
                    vec![board.id.clone(), id.clone()],
                ));
            }
        }
        for id in &board.net_ids {
            if !net_ids.contains(id.as_str()) {
                findings.push(error(
                    Scope::Pcb,
                    format!("board:{}:net:{}", board.id, id),
                    "Board net is missing",
                    vec![board.id.clone(), id.clone()],
                ));
            }
        }
    }
    let mut matrices = Vec::with_capacity(doc.matrices.len());
    for matrix in &doc.matrices {
        let scene = match crate::matrix::validated_matrix(matrix, doc) {
            Ok(validated) => validated.project().ok().map(|mut scene| {
                scene.companions = crate::matrix::scene_companions(matrix, doc);
                scene
            }),
            Err(message) => {
                findings.push(error(
                    Scope::Layout,
                    format!("matrix:{}:invalid", matrix.id),
                    &message,
                    vec![matrix.id.clone()],
                ));
                None
            }
        };
        matrices.push(scene);
        if let Some(board_id) = &matrix.board_id
            && !doc.boards.iter().any(|board| &board.id == board_id)
        {
            findings.push(error(
                Scope::Pcb,
                format!("matrix:{}:board", matrix.id),
                "Matrix board is missing",
                vec![matrix.id.clone(), board_id.clone()],
            ));
        }
        if crate::matrix::valid_dimensions(matrix).is_err()
            || !matrix.pitch.x.is_finite()
            || !matrix.pitch.y.is_finite()
            || matrix.pitch.x <= 0.0
            || matrix.pitch.y <= 0.0
            || !matrix.origin.x.is_finite()
            || !matrix.origin.y.is_finite()
            || matrix.rotation.is_some_and(|value| !value.is_finite())
        {
            findings.push(error(
                Scope::Layout,
                format!("matrix:{}:dimensions", matrix.id),
                "Matrix dimensions or transform are invalid",
                vec![matrix.id.clone()],
            ));
        }
        if !doc
            .definitions
            .iter()
            .any(|def| def.id == matrix.definition_id)
        {
            findings.push(error(
                Scope::Layout,
                format!("matrix:{}:definition", matrix.id),
                "Matrix definition is missing",
                vec![matrix.id.clone()],
            ));
        }
        let disabled = matrix
            .cells
            .iter()
            .filter(|cell| !cell.enabled || cell.deleted)
            .count();
        let companions: usize = matrix
            .cells
            .iter()
            .filter(|cell| cell.enabled && !cell.deleted)
            .map(|cell| cell.assemblies.len())
            .sum();
        if matrix
            .rows
            .checked_mul(matrix.columns)
            .is_some_and(|count| {
                matrix.part_ids.len() != (count as usize).saturating_sub(disabled) + companions
            })
        {
            findings.push(error(
                Scope::Layout,
                format!("matrix:{}:members", matrix.id),
                "Matrix member count does not match dimensions",
                vec![matrix.id.clone()],
            ));
        }
        for id in &matrix.part_ids {
            if !parts_by_id.contains_key(id.as_str()) {
                findings.push(error(
                    Scope::Layout,
                    format!("matrix:{}:part:{}", matrix.id, id),
                    "Matrix part is missing",
                    vec![matrix.id.clone(), id.clone()],
                ));
            }
        }
    }
    let mut pad_nets: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    let mut terminal_nets: BTreeMap<(&str, &str), &str> = BTreeMap::new();
    for net in &doc.nets {
        for pin in &net.pins {
            let Some(part) = parts_by_id.get(pin.part_id.as_str()) else {
                findings.push(error(
                    Scope::Pcb,
                    format!("net:{}:part:{}", net.id, pin.part_id),
                    "Net part is missing",
                    vec![net.id.clone(), pin.part_id.clone()],
                ));
                continue;
            };
            let Some(def) = definitions_by_id.get(part.definition_id.as_str()) else {
                continue;
            };
            if !def.pads.iter().any(|pad| pad.id == pin.pad_id) {
                findings.push(error(
                    Scope::Pcb,
                    format!("net:{}:pad:{}:{}", net.id, pin.part_id, pin.pad_id),
                    "Net pad is missing",
                    vec![net.id.clone(), pin.part_id.clone()],
                ));
            }
            let key = (pin.part_id.as_str(), pin.pad_id.as_str());
            if let Some(pad) = def
                .pads
                .iter()
                .find(|pad| pad.id == pin.pad_id && !pad.number.is_empty())
            {
                let terminal = (pin.part_id.as_str(), pad.number.as_str());
                if let Some(previous) = terminal_nets.insert(terminal, &net.id)
                    && previous != net.id
                {
                    findings.push(error(
                        Scope::Pcb,
                        format!("terminal:{}:{}:multiple-nets", pin.part_id, pad.number),
                        "Repeated physical pads of one logical contact must use the same net",
                        vec![previous.into(), net.id.clone(), pin.part_id.clone()],
                    ));
                }
            }
            if let Some(previous) = pad_nets.insert(key, &net.id)
                && previous != net.id
            {
                findings.push(error(
                    Scope::Pcb,
                    format!("pad:{}:{}:multiple-nets", pin.part_id, pin.pad_id),
                    "Pad is assigned to multiple nets",
                    vec![previous.into(), net.id.clone(), pin.part_id.clone()],
                ));
            }
        }
    }
    for body in &doc.case_bodies {
        if !doc.boards.iter().any(|board| board.id == body.board_id) {
            findings.push(error(
                Scope::Case,
                format!("case:{}:board", body.id),
                "Case board is missing",
                vec![body.id.clone(), body.board_id.clone()],
            ));
        }
        if !body.thickness.is_finite()
            || body.thickness <= 0.0
            || !body.clearance.is_finite()
            || body.clearance < 0.0
        {
            findings.push(error(
                Scope::Case,
                format!("case:{}:dimensions", body.id),
                "Case thickness and clearance are invalid",
                vec![body.id.clone()],
            ));
        }
        if body.z.is_some_and(|z| !z.is_finite()) {
            findings.push(error(
                Scope::Case,
                format!("case:{}:z", body.id),
                "Case Z must be finite",
                vec![body.id.clone()],
            ));
        }
        if matches!(body.kind, CaseKind::Tray | CaseKind::Lid)
            && (body.wall_height.is_none() || body.wall_thickness.is_none())
        {
            findings.push(error(
                Scope::Case,
                format!("case:{}:walls", body.id),
                "Tray and lid require wall height and thickness",
                vec![body.id.clone()],
            ));
        }
        if body
            .wall_height
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
            || body
                .wall_thickness
                .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            findings.push(error(
                Scope::Case,
                format!("case:{}:walls", body.id),
                "Wall height and thickness must be positive",
                vec![body.id.clone()],
            ));
        }
        if let Some(mounts) = &body.mounts {
            let mut ids = std::collections::BTreeSet::new();
            for mount in mounts {
                if !ids.insert(&mount.id) {
                    findings.push(error(
                        Scope::Case,
                        format!("case:{}:mount:{}:duplicate", body.id, mount.id),
                        "Mount IDs must be unique within a body",
                        vec![body.id.clone(), mount.id.clone()],
                    ));
                }
                if !mount.at.x.is_finite()
                    || !mount.at.y.is_finite()
                    || !mount.hole_diameter.is_finite()
                    || mount.hole_diameter <= 0.0
                {
                    findings.push(error(
                        Scope::Case,
                        format!("case:{}:mount:{}:hole", body.id, mount.id),
                        "Mount position and hole diameter are invalid",
                        vec![body.id.clone(), mount.id.clone()],
                    ));
                }
                if matches!(mount.kind, MountKind::Boss)
                    && (mount
                        .boss_diameter
                        .is_none_or(|value| !value.is_finite() || value <= mount.hole_diameter)
                        || mount
                            .height
                            .is_none_or(|value| !value.is_finite() || value <= 0.0))
                {
                    findings.push(error(
                        Scope::Case,
                        format!("case:{}:mount:{}:boss", body.id, mount.id),
                        "Boss diameter must exceed hole diameter and height must be positive",
                        vec![body.id.clone(), mount.id.clone()],
                    ));
                }
            }
        }
        if let Some(gasket) = &body.gasket
            && (!gasket.inset.is_finite()
                || gasket.inset < 0.0
                || !gasket.width.is_finite()
                || gasket.width <= 0.0
                || !gasket.depth.is_finite()
                || gasket.depth <= 0.0
                || gasket.depth >= body.thickness)
        {
            findings.push(error(
                Scope::Case,
                format!("case:{}:gasket", body.id),
                "Gasket inset, width, or depth is invalid",
                vec![body.id.clone()],
            ));
        }
        if let Some(id) = &body.material_id
            && !doc.materials.iter().any(|material| &material.id == id)
        {
            findings.push(error(
                Scope::Case,
                boardstudio_model::case_readiness::case_material_finding_id(&body.id),
                "Case material is missing",
                vec![body.id.clone(), id.clone()],
            ));
        }
    }
    SceneDiagnostics {
        findings,
        hardware_markers,
        keycaps,
        modules,
        matrices,
    }
}
