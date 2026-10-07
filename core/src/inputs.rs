//! Input capabilities are independent of the library category and footprint source.
use crate::model::{Part, PartDefinition, ProjectDoc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PressContacts {
    pub row: String,
    pub column: String,
    pub independent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncoderDriver {
    Ec11,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotaryProfile {
    pub a: String,
    pub b: String,
    pub common: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers_per_rotation: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<EncoderDriver>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub press: Option<PressContacts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotary: Option<RotaryProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PressScanMode {
    Matrix,
    Direct,
    Unassigned,
}

pub(crate) fn matrix_member(doc: &ProjectDoc, part: &Part) -> bool {
    doc.matrices.iter().any(|matrix| {
        matrix.part_ids.contains(&part.id)
            && part
                .id
                .strip_prefix(&format!("matrix/{}/", matrix.id))
                .is_some_and(|cell| !cell.contains('/'))
    })
}

pub(crate) fn profile(def: &PartDefinition) -> InputProfile {
    if let Some(profile) = &def.input_profile {
        return profile.clone();
    }
    let rotary = def
        .generator
        .as_ref()
        .filter(|g| g.source == "ceoloide/rotary_encoder_ec11_ec12")
        .map(|_| RotaryProfile {
            a: "A".into(),
            b: "C".into(),
            common: "B".into(),
            steps: Some(80),
            triggers_per_rotation: Some(20),
            driver: Some(EncoderDriver::Ec11),
        });
    let press = def
        .matrix_terminals
        .as_ref()
        .map(|p| PressContacts {
            row: p.row.clone(),
            column: p.column.clone(),
            independent: true,
        })
        .or_else(|| {
            rotary.as_ref().map(|_| PressContacts {
                row: "S1".into(),
                column: "S2".into(),
                independent: true,
            })
        });
    InputProfile { press, rotary }
}

pub(crate) fn scan_mode(doc: &ProjectDoc, part: &Part) -> PressScanMode {
    part.properties
        .as_ref()
        .and_then(|p| p.get("pressScanMode"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_else(|| {
            if matrix_member(doc, part) {
                PressScanMode::Matrix
            } else {
                PressScanMode::Direct
            }
        })
}

fn pads_for(def: &PartDefinition, terminal: &str) -> Vec<String> {
    def.terminals.get(terminal).cloned().unwrap_or_else(|| {
        def.pads
            .iter()
            .filter(|pad| pad.id == terminal || pad.number == terminal)
            .map(|pad| pad.id.clone())
            .collect()
    })
}

pub(crate) fn validate_matrix_input(def: &PartDefinition) -> Result<(), String> {
    let profile = profile(def);
    let press = profile
        .press
        .ok_or("Matrix keys require independent press contacts; this input is rotation-only")?;
    if !press.independent {
        return Err("Matrix scanning requires independent switch contacts".into());
    }
    let row = pads_for(def, &press.row);
    let col = pads_for(def, &press.column);
    if row.is_empty()
        || col.is_empty()
        || row.iter().any(|pad| col.contains(pad))
        || row
            .iter()
            .chain(&col)
            .any(|id| !def.pads.iter().any(|p| p.id == *id))
    {
        return Err("Matrix scanning requires two distinct populated press terminals".into());
    }
    if let Some(rotary) = profile.rotary {
        let rotary_pads: Vec<_> = [&rotary.a, &rotary.b, &rotary.common]
            .into_iter()
            .flat_map(|terminal| pads_for(def, terminal))
            .collect();
        if row.iter().chain(&col).any(|id| rotary_pads.contains(id)) {
            return Err("Matrix press contacts must be separate from the rotary contacts".into());
        }
    }
    Ok(())
}

pub(crate) fn set_scan_mode(
    doc: &mut ProjectDoc,
    id: &str,
    mode: PressScanMode,
) -> Result<Vec<String>, String> {
    let part = doc
        .parts
        .iter()
        .find(|p| p.id == id)
        .ok_or("Input part is missing")?;
    if part.locked == Some(true) {
        return Err("Input part is locked".into());
    }
    let def = doc
        .definitions
        .iter()
        .find(|d| d.id == part.definition_id)
        .ok_or("Input definition is missing")?;
    let press = profile(def)
        .press
        .ok_or("This component has no press contacts")?;
    if mode == PressScanMode::Matrix {
        if !matrix_member(doc, part) {
            return Err("Assign the press to a matrix key before selecting matrix scanning".into());
        }
        validate_matrix_input(def)?;
    }
    if mode != scan_mode(doc, part) {
        let contacts: Vec<_> = [&press.row, &press.column]
            .into_iter()
            .flat_map(|terminal| pads_for(def, terminal))
            .collect();
        let board_ids: Vec<_> = doc
            .boards
            .iter()
            .filter(|board| board.part_ids.iter().any(|part| part == id))
            .map(|board| board.id.clone())
            .collect();
        let affected: std::collections::BTreeSet<_> = doc
            .nets
            .iter()
            .filter(|net| {
                board_ids.iter().any(|board| {
                    net.id
                        .starts_with(&format!("generated/electrical/{board}/"))
                }) && net
                    .pins
                    .iter()
                    .any(|pin| pin.part_id == id && contacts.contains(&pin.pad_id))
            })
            .map(|net| net.id.clone())
            .collect();
        if doc.boards.iter().any(|board| {
            board.traces.iter().any(|trace| {
                trace
                    .net_id
                    .as_ref()
                    .is_some_and(|net| affected.contains(net))
            }) || board.vias.iter().any(|via| {
                via.net_id
                    .as_ref()
                    .is_some_and(|net| affected.contains(net))
            })
        }) {
            return Err("This press has routed copper. Remove or reroute its connections before changing the scan mode; existing routing was preserved.".into());
        }
        let controllers: std::collections::BTreeSet<_> = doc
            .parts
            .iter()
            .filter(|part| {
                doc.definitions.iter().any(|definition| {
                    definition.id == part.definition_id
                        && definition.kind == crate::model::PartKind::Controller
                })
            })
            .map(|part| part.id.as_str())
            .collect();
        for net in &mut doc.nets {
            if affected.contains(&net.id) {
                net.pins
                    .retain(|pin| pin.part_id != id || !contacts.contains(&pin.pad_id));
            }
        }
        let empty: std::collections::BTreeSet<_> = doc
            .nets
            .iter()
            .filter(|net| {
                affected.contains(&net.id)
                    && (net.pins.is_empty()
                        || net
                            .pins
                            .iter()
                            .all(|pin| controllers.contains(pin.part_id.as_str())))
            })
            .map(|net| net.id.clone())
            .collect();
        doc.nets.retain(|net| !empty.contains(&net.id));
        for board in &mut doc.boards {
            board.net_ids.retain(|net| !empty.contains(net));
            board
                .traces
                .retain(|trace| !trace.net_id.as_ref().is_some_and(|net| empty.contains(net)));
            board
                .vias
                .retain(|via| !via.net_id.as_ref().is_some_and(|net| empty.contains(net)));
        }
    }
    let part = doc.parts.iter_mut().find(|p| p.id == id).unwrap();
    part.properties.get_or_insert_with(Default::default).insert(
        "pressScanMode".into(),
        serde_json::to_value(mode).map_err(|e| e.to_string())?,
    );
    crate::matrix::mark_override(part);
    Ok(vec![id.into()])
}

pub(crate) fn finding_markers(
    doc: &ProjectDoc,
    findings: &[crate::model::Finding],
) -> Vec<crate::model::FindingMarker> {
    use crate::model::{Contour, FindingMarker, Side, Vec2};
    let mut result = Vec::new();
    for finding in findings
        .iter()
        .filter(|f| f.id.starts_with("terminal:") && f.id.ends_with(":multiple-nets"))
    {
        for part in doc
            .parts
            .iter()
            .filter(|p| finding.target_ids.contains(&p.id))
        {
            let Some(def) = doc.definitions.iter().find(|d| d.id == part.definition_id) else {
                continue;
            };
            let prefix = format!("terminal:{}:", part.id);
            let Some(number) = finding
                .id
                .strip_prefix(&prefix)
                .and_then(|s| s.strip_suffix(":multiple-nets"))
            else {
                continue;
            };
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            let contours: Vec<_> = def
                .pads
                .iter()
                .filter(|p| p.number == number)
                .map(|pad| {
                    let (ps, pc) = pad.rotation.unwrap_or(0.0).to_radians().sin_cos();
                    let points = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
                        .iter()
                        .map(|(x, y)| {
                            let px = pad.at.x + x * pad.size.x * pc - y * pad.size.y * ps;
                            let py = pad.at.y + x * pad.size.x * ps + y * pad.size.y * pc;
                            let px = if part.side == Side::Back { -px } else { px };
                            Vec2 {
                                x: part.pose.at.x + px * cos - py * sin,
                                y: part.pose.at.y + px * sin + py * cos,
                            }
                        })
                        .collect();
                    Contour {
                        points,
                        hole: false,
                    }
                })
                .collect();
            for board in doc.boards.iter().filter(|b| b.part_ids.contains(&part.id)) {
                result.push(FindingMarker {
                    finding_id: finding.id.clone(),
                    board_id: board.id.clone(),
                    contours: contours.clone(),
                });
            }
        }
    }
    result
}
