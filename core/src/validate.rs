use crate::model::{CaseKind, Finding, MountKind, ProjectDoc, Scope, Severity};
use std::collections::{BTreeMap, BTreeSet};

fn error(scope: Scope, id: String, message: &str, targets: Vec<String>) -> Finding {
    Finding {
        id,
        severity: Severity::Error,
        scope,
        message: message.into(),
        target_ids: targets,
    }
}

pub fn validate(doc: &ProjectDoc) -> Vec<Finding> {
    let mut findings = vec![];
    for board in &doc.boards {
        findings.extend(crate::keycaps::resolve(doc, &board.id, None).findings);
    }
    for part in &doc.parts {
        let Some(def) = doc.definitions.iter().find(|d| d.id == part.definition_id) else {
            continue;
        };
        if crate::inputs::profile(def).rotary.is_some() && crate::inputs::matrix_member(doc, part) {
            findings.push(Finding{id:format!("input/{}/assembly-fit",part.id),scope:Scope::Layout,severity:Severity::Warning,
                message:"The encoder retains this key's bindings and companion parts. Review companion clearance and its own knob or wheel envelope; the former keycap is not a qualified encoder assembly.".into(),target_ids:vec![part.id.clone()]});
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
    let mut parts_by_definition: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for part in &doc.parts {
        parts_by_definition
            .entry(part.definition_id.as_str())
            .or_default()
            .push(part.id.clone());
    }
    let outline_ids: BTreeSet<_> = doc.outline.iter().map(|feature| feature.id()).collect();
    let net_ids: BTreeSet<_> = doc.nets.iter().map(|net| net.id.as_str()).collect();
    for def in &doc.definitions {
        let parts = parts_by_definition
            .get(def.id.as_str())
            .cloned()
            .unwrap_or_default();
        if parts.is_empty() {
            continue;
        }
        let targets: Vec<_> = std::iter::once(def.id.clone())
            .chain(parts.iter().cloned())
            .collect();
        let mut report = |suffix: String, message: &str| {
            findings.push(error(
                Scope::Pcb,
                format!("definition:{}:{suffix}", def.id),
                message,
                targets.clone(),
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
        {
            if let Some(id) = &state.active_version_id {
                if let Some(version) = state.versions.iter().find(|version| &version.id == id) {
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
                } else {
                    findings.push(error(Scope::Pcb, format!("board:{}:outline:version", board.id), "The active outline version is missing; select Generated or another version", vec![board.id.clone()]));
                }
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
    for matrix in &doc.matrices {
        if let Err(message) = crate::matrix::valid_matrix(matrix, doc) {
            findings.push(error(
                Scope::Layout,
                format!("matrix:{}:invalid", matrix.id),
                &message,
                vec![matrix.id.clone()],
            ));
        }
        if let Some(board_id) = &matrix.board_id {
            if !doc.boards.iter().any(|board| &board.id == board_id) {
                findings.push(error(
                    Scope::Pcb,
                    format!("matrix:{}:board", matrix.id),
                    "Matrix board is missing",
                    vec![matrix.id.clone(), board_id.clone()],
                ));
            }
        }
        if matrix.rows == 0
            || matrix.columns == 0
            || matrix
                .rows
                .checked_mul(matrix.columns)
                .is_none_or(|count| count > crate::matrix::MAX_MATRIX_PARTS)
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
        let disabled = matrix.cells.iter().filter(|cell| !cell.enabled).count();
        let companions: usize = matrix
            .cells
            .iter()
            .filter(|cell| cell.enabled)
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
                if let Some(previous) = terminal_nets.insert(terminal, &net.id) {
                    if previous != net.id {
                        findings.push(error(
                            Scope::Pcb,
                            format!("terminal:{}:{}:multiple-nets", pin.part_id, pad.number),
                            "Repeated physical pads of one logical contact must use the same net",
                            vec![previous.into(), net.id.clone(), pin.part_id.clone()],
                        ));
                    }
                }
            }
            if let Some(previous) = pad_nets.insert(key, &net.id) {
                if previous != net.id {
                    findings.push(error(
                        Scope::Pcb,
                        format!("pad:{}:{}:multiple-nets", pin.part_id, pin.pad_id),
                        "Pad is assigned to multiple nets",
                        vec![previous.into(), net.id.clone(), pin.part_id.clone()],
                    ));
                }
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
        if let Some(gasket) = &body.gasket {
            if !gasket.inset.is_finite()
                || gasket.inset < 0.0
                || !gasket.width.is_finite()
                || gasket.width <= 0.0
                || !gasket.depth.is_finite()
                || gasket.depth <= 0.0
                || gasket.depth >= body.thickness
            {
                findings.push(error(
                    Scope::Case,
                    format!("case:{}:gasket", body.id),
                    "Gasket inset, width, or depth is invalid",
                    vec![body.id.clone()],
                ));
            }
        }
        if let Some(id) = &body.material_id {
            if !doc.materials.iter().any(|material| &material.id == id) {
                findings.push(error(
                    Scope::Case,
                    format!("case:{}:material", body.id),
                    "Case material is missing",
                    vec![body.id.clone(), id.clone()],
                ));
            }
        }
    }
    findings
}
