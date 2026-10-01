mod output;
mod planning;

pub use output::finish_export;
pub(super) use output::finish_preview;
pub use planning::prepare_export;
pub(super) use planning::prepare_preview;

use super::{ArtifactError, ArtifactErrorCode, sexpr};
use crate::artifact::{compile, source};
use crate::model::*;
use kiutils_sexpr::{Atom, CstDocument, Node};
use std::collections::{BTreeMap, HashMap, HashSet};

const FILE_VERSION: u32 = 20241229;
const EDGE_WIDTH: f64 = 0.05;
const COURTYARD_WIDTH: f64 = 0.05;
const TEXT_STROKE: f64 = 0.15;
const TEXT_SIZE: f64 = 1.0;
const MAX_COORD: f64 = 1_000_000.0;

fn err(code: ArtifactErrorCode, message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(code, message)
}

fn validation(message: impl Into<String>) -> ArtifactError {
    err(ArtifactErrorCode::Validation, message)
}

fn num(value: f64) -> Result<String, ArtifactError> {
    if !value.is_finite() || value.abs() > MAX_COORD {
        return Err(validation(format!("Invalid KiCad coordinate: {value}")));
    }
    let rounded = format!("{value:.6}")
        .parse::<f64>()
        .map_err(|_| validation(format!("Invalid KiCad coordinate: {value}")))?;
    if rounded == 0.0 {
        return Ok("0".into());
    }
    Ok(rounded.to_string())
}

fn q(value: &str) -> Result<String, ArtifactError> {
    if value.chars().any(|character| character <= '\u{1f}') {
        return Err(validation("KiCad text contains a control character"));
    }
    Ok(sexpr::quote(value))
}

fn xy(point: Vec2) -> Result<String, ArtifactError> {
    Ok(format!("{} {}", num(point.x)?, num(-point.y)?))
}

fn xy_side(point: Vec2, back: bool) -> Result<String, ArtifactError> {
    Ok(format!(
        "{} {}",
        num(point.x)?,
        num(if back { point.y } else { -point.y })?
    ))
}

fn safe_name(value: &str) -> String {
    let mut out = String::new();
    let mut replacing = false;
    for ch in value.chars() {
        let valid = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
        if valid {
            out.push(ch);
            replacing = false;
        } else if !replacing {
            out.push('_');
            replacing = true;
        }
    }
    while out.starts_with('.') {
        out.remove(0);
    }
    out.truncate(
        out.char_indices()
            .nth(80)
            .map_or(out.len(), |(index, _)| index),
    );
    if out.is_empty() { "board".into() } else { out }
}

// Keep object identities stable and byte-compatible with the TS FNV-style UUID.
fn uuid(value: &str) -> String {
    let mut words = [0x811c9dc5_u32, 0x91f832ec, 0xd1f5ab43, 0x67ed3a21];
    for character in value.chars() {
        let code = character as u32;
        for (index, word) in words.iter_mut().enumerate() {
            *word = (*word ^ code.wrapping_add(index as u32)).wrapping_mul(0x01000193);
        }
    }
    let hex = words
        .iter()
        .map(|word| format!("{word:08x}"))
        .collect::<String>();
    format!(
        "{}-{}-5{}-a{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32]
    )
}

fn uuid_text(scope: &str, kind: &str, id: &str) -> Result<String, ArtifactError> {
    q(&uuid(&format!("{scope}:{kind}:{id}")))
}

fn render(form: &str) -> Result<String, ArtifactError> {
    source::render_form(form)
}

fn unique_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), ArtifactError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.is_empty() || !seen.insert(id) {
            return Err(validation(format!("Duplicate or empty {label} ID: {id}")));
        }
    }
    Ok(())
}

fn point_valid(point: Vec2) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn assert_geometry(geometry: &FootprintGeometry) -> Result<(), ArtifactError> {
    unique_ids(geometry.pads.iter().map(|pad| pad.id.as_str()), "pad")?;
    for pad in &geometry.pads {
        if pad.number.trim().is_empty() && pad.plated != Some(false) {
            return Err(validation(format!(
                "Empty electrical pad number: {}",
                pad.id
            )));
        }
        if !point_valid(pad.at)
            || !point_valid(pad.size)
            || pad.size.x <= 0.0
            || pad.size.y <= 0.0
            || pad
                .drill
                .is_some_and(|drill| !drill.is_finite() || drill <= 0.0)
        {
            return Err(validation(format!("Invalid pad geometry: {}", pad.id)));
        }
    }
    if geometry.courtyard.iter().any(|point| !point_valid(*point)) {
        return Err(validation("Invalid courtyard coordinate"));
    }
    Ok(())
}

fn pad_form(
    pad: &Pad,
    net: Option<&ReservedNet>,
    scope: &str,
    back: bool,
    root_rotation: f64,
) -> Result<String, ArtifactError> {
    let through = pad.drill.is_some();
    let pad_type = if through {
        if pad.plated == Some(false) {
            "np_thru_hole"
        } else {
            "thru_hole"
        }
    } else {
        "smd"
    };
    let layers = if pad.plated == Some(false) || through {
        "\"*.Cu\" \"*.Mask\""
    } else if pad
        .side
        .as_ref()
        .is_some_and(|side| matches!(side, Side::Back))
        || (pad.side.is_none() && back)
    {
        "\"B.Cu\" \"B.Paste\" \"B.Mask\""
    } else {
        "\"F.Cu\" \"F.Paste\" \"F.Mask\""
    };
    let local_rotation = pad.rotation.unwrap_or(0.0);
    let absolute_rotation = if back {
        root_rotation + 360.0 - local_rotation
    } else {
        root_rotation + local_rotation
    };
    let rotation = if absolute_rotation == 0.0 {
        String::new()
    } else {
        format!(" {}", num(absolute_rotation)?)
    };
    let drill = pad
        .drill
        .map(num)
        .transpose()?
        .map(|value| format!(" (drill {value})"))
        .unwrap_or_default();
    let roundrect = if matches!(pad.shape, PadShape::Roundrect) {
        " (roundrect_rratio 0.25)"
    } else {
        ""
    };
    let net = net
        .map(|net| Ok(format!(" (net {} {})", net.index, q(&net.name)?)))
        .transpose()?
        .unwrap_or_default();
    render(&format!(
        "(pad {} {pad_type} {} (at {}{rotation}) (size {} {}){drill} (layers {layers}){roundrect}{net} (uuid {}))",
        q(&pad.number)?,
        shape_name(&pad.shape),
        xy_side(pad.at, back)?,
        num(pad.size.x)?,
        num(pad.size.y)?,
        uuid_text(scope, "pad", &pad.id)?
    ))
}

fn shape_name(shape: &PadShape) -> &'static str {
    match shape {
        PadShape::Circle => "circle",
        PadShape::Oval => "oval",
        PadShape::Rect => "rect",
        PadShape::Roundrect => "roundrect",
    }
}

fn model_forms(
    definition: &PartDefinition,
    models: &BTreeMap<String, String>,
) -> Result<Vec<String>, ArtifactError> {
    let model_list = definition.models.iter().flatten();
    model_list
        .map(|model| {
            let path = models.get(&model.asset_id).ok_or_else(|| {
                validation(format!(
                    "Model {} needs a safe relative path",
                    model.asset_id
                ))
            })?;
            if !safe_model_path(path) {
                return Err(validation(format!(
                    "Model {} needs a safe relative path",
                    model.asset_id
                )));
            }
            render(&format!(
                "(model {} (offset (xyz {} {} {})) (scale (xyz {} {} {})) (rotate (xyz {} {} {})))",
                q(&format!("${{KIPRJMOD}}/{path}"))?,
                num(model.offset.x)?,
                num(-model.offset.y)?,
                num(model.offset.z)?,
                num(model.scale.x)?,
                num(model.scale.y)?,
                num(model.scale.z)?,
                num(model.rotation.x)?,
                num(model.rotation.y)?,
                num(-model.rotation.z)?
            ))
        })
        .collect()
}

fn override_models(
    source: String,
    definition: &PartDefinition,
    paths: &BTreeMap<String, String>,
) -> Result<String, ArtifactError> {
    if definition.models.is_none() {
        return Ok(source);
    }
    let parsed = kiutils_sexpr::parse_one(&source).map_err(|e| validation(e.to_string()))?;
    let root = &parsed.nodes[0];
    let replacements = sexpr::children(root, "model")
        .map(|node| (sexpr::span(node), String::new()))
        .collect();
    let mut result = sexpr::replace_spans(&source, replacements)
        .ok_or_else(|| validation("Invalid model spans"))?;
    let at = result
        .rfind(')')
        .ok_or_else(|| validation("Invalid footprint"))?;
    result.insert_str(at, &model_forms(definition, paths)?.join("\n"));
    Ok(result)
}

fn managed_model_forms(
    definition: &PartDefinition,
    models: &BTreeMap<String, String>,
) -> Result<Vec<String>, ArtifactError> {
    model_forms(definition, models)
}

fn safe_model_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains("..")
        && !path.contains('\\')
        && !path.starts_with("${")
        && !path.contains(':')
}

fn courtyard_forms(points: &[Vec2], scope: &str, back: bool) -> Result<Vec<String>, ArtifactError> {
    if points.len() < 3 {
        return Err(validation(format!(
            "Footprint {scope} needs a courtyard polygon"
        )));
    }
    let layer = if back { "B.CrtYd" } else { "F.CrtYd" };
    (0..points.len()).map(|index| {
        let start = points[index];
        let end = points[(index + 1) % points.len()];
        render(&format!("(fp_line (start {}) (end {}) (stroke (width {}) (type solid)) (layer {}) (uuid {}))",
                xy_side(start, back)?, xy_side(end, back)?, num(COURTYARD_WIDTH)?, q(layer)?, uuid_text(scope, "courtyard", &index.to_string())?))
    }).collect()
}

fn native_footprint(
    definition: &PartDefinition,
    scope: &str,
    part: Option<&Part>,
    nets: &BTreeMap<String, ReservedNet>,
    model_paths: &BTreeMap<String, String>,
) -> Result<String, ArtifactError> {
    let job = FootprintCompileJob {
        id: definition.id.clone(),
        definition: definition.clone(),
        side: part.map_or(Side::Front, |part| part.side.clone()),
    };
    let compiled = compile::compile_authored(&job.definition, job.side).map_err(validation)?;
    let geometry = &compiled.geometry;
    assert_geometry(geometry)?;
    let back = matches!(geometry.side, Side::Back);
    let layer = if back { "B.Cu" } else { "F.Cu" };
    let reference_layer = if back { "B.SilkS" } else { "F.SilkS" };
    let fab_layer = if back { "B.Fab" } else { "F.Fab" };
    let name = safe_name(&definition.name);
    let root_rotation = part.map_or(0.0, |part| {
        if back {
            part.pose.rotation - 180.0
        } else {
            part.pose.rotation
        }
    });
    let placement = part
        .map(|part| {
            Ok(format!(
                " (at {} {})",
                xy(part.pose.at)?,
                num(root_rotation)?
            ))
        })
        .transpose()?
        .unwrap_or_default();
    let reference = part
        .map(|part| part.reference.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("REF**");
    let reference_y = geometry
        .courtyard
        .iter()
        .map(|point| point.y.abs())
        .fold(0.0_f64, f64::max)
        + 2.0;
    let mirror = if back { " (justify mirror)" } else { "" };
    let mut forms = vec![render(&format!(
        "(property \"Reference\" {} (at 0 {} 0) (layer {}) (effects (font (size {} {}) (thickness {})){mirror}))",
        q(reference)?,
        num(reference_y)?,
        q(reference_layer)?,
        num(TEXT_SIZE)?,
        num(TEXT_SIZE)?,
        num(TEXT_STROKE)?
    ))?];
    forms.push(render(&format!("(property \"Value\" {} (at 0 -2 0) (layer {}) (effects (font (size {} {}) (thickness {})){mirror}))",
        q(&definition.name)?, q(fab_layer)?, num(TEXT_SIZE)?, num(TEXT_SIZE)?, num(TEXT_STROKE)?))?);
    forms.extend(courtyard_forms(&geometry.courtyard, scope, back)?);
    for pad in &geometry.pads {
        let net = nets.get(&pad.id);
        forms.push(pad_form(pad, net, scope, back, root_rotation)?);
    }
    if part.is_none() {
        unique_ids(
            geometry
                .traces
                .iter()
                .map(|item| item.id.as_str())
                .chain(geometry.vias.iter().map(|item| item.id.as_str())),
            "copper",
        )?;
        for trace in &geometry.traces {
            let layer = if matches!(trace.layer, Side::Back) {
                "B.Cu"
            } else {
                "F.Cu"
            };
            forms.push(render(&format!(
                "(fp_line (start {}) (end {}) (stroke (width {}) (type solid)) (layer {}) (uuid {}))",
                xy_side(trace.start, back)?, xy_side(trace.end, back)?, num(trace.width)?, q(layer)?,
                uuid_text(scope, "trace", &trace.id)?
            ))?);
        }
        for via in &geometry.vias {
            let source_pad = geometry
                .pads
                .iter()
                .find(|pad| pad.id == via.pad_id.as_deref().unwrap_or_default())
                .ok_or_else(|| validation(format!("Via {} references missing pad", via.id)))?;
            forms.push(pad_form(
                &Pad {
                    id: via.id.clone(),
                    number: source_pad.number.clone(),
                    at: via.at,
                    size: Vec2 {
                        x: via.size,
                        y: via.size,
                    },
                    shape: PadShape::Circle,
                    drill: Some(via.drill),
                    plated: None,
                    side: None,
                    rotation: None,
                    net_id: None,
                },
                None,
                scope,
                false,
                root_rotation,
            )?);
        }
    }
    let models = model_forms(definition, model_paths)?;
    forms.extend(models);
    let uuid = q(&uuid(scope))?;
    let body = forms.join(" ");
    render(&format!(
        "(footprint {} (layer {}){placement} (uuid {uuid}) {body})",
        q(&name)?,
        q(layer)?
    ))
}

fn is_ergogen(definition: &PartDefinition) -> bool {
    definition.generator.as_ref().is_some_and(|generator| {
        generator.source.starts_with("ceoloide/") || generator.source.starts_with("infused-kim/")
    })
}

fn is_imported(definition: &PartDefinition) -> bool {
    definition.kicad_source.is_some()
}

fn net_by_id<'a>(nets: &'a [Net], id: &str) -> Option<&'a Net> {
    nets.iter().find(|net| net.id == id)
}

type BoardMaps<'a> = (
    &'a Board,
    HashMap<&'a str, &'a Part>,
    HashMap<&'a str, &'a PartDefinition>,
);

fn board_and_maps<'a>(doc: &'a ProjectDoc, board_id: &str) -> Result<BoardMaps<'a>, ArtifactError> {
    let board = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| {
            err(
                ArtifactErrorCode::NotFound,
                format!("Missing board: {board_id}"),
            )
        })?;
    if !board.thickness.is_finite() || board.thickness <= 0.0 {
        return Err(validation(format!(
            "Missing board or invalid thickness: {board_id}"
        )));
    }
    unique_ids(
        doc.definitions.iter().map(|item| item.id.as_str()),
        "definition",
    )?;
    unique_ids(doc.parts.iter().map(|item| item.id.as_str()), "part")?;
    unique_ids(doc.nets.iter().map(|item| item.id.as_str()), "net")?;
    let parts = doc
        .parts
        .iter()
        .map(|part| (part.id.as_str(), part))
        .collect();
    let definitions = doc
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect();
    Ok((board, parts, definitions))
}

fn net_lookup(
    doc: &ProjectDoc,
    board: &Board,
) -> Result<(Vec<ReservedNet>, HashMap<String, String>), ArtifactError> {
    let mut nets = Vec::new();
    let mut pin_map = HashMap::new();
    let mut logical_terminals = HashMap::new();
    for (index, net_id) in board.net_ids.iter().enumerate() {
        let net = net_by_id(&doc.nets, net_id)
            .ok_or_else(|| validation(format!("Board references missing net {net_id}")))?;
        let reserved = ReservedNet {
            name: net.name.clone(),
            index: (index + 1) as u32,
        };
        nets.push(reserved);
        for pin in &net.pins {
            if !board.part_ids.iter().any(|part_id| part_id == &pin.part_id) {
                return Err(validation(format!(
                    "Net {} references part outside board: {}",
                    net.id, pin.part_id
                )));
            }
            let key = format!("{}\0{}", pin.part_id, pin.pad_id);
            if let Some(pad) = doc
                .parts
                .iter()
                .find(|part| part.id == pin.part_id)
                .and_then(|part| {
                    doc.definitions
                        .iter()
                        .find(|definition| definition.id == part.definition_id)
                })
                .and_then(|definition| definition.pads.iter().find(|pad| pad.id == pin.pad_id))
                .filter(|pad| !pad.number.is_empty())
            {
                let terminal = format!("{}\0{}", pin.part_id, pad.number);
                if logical_terminals
                    .insert(terminal, net.id.clone())
                    .is_some_and(|old| old != net.id)
                {
                    return Err(validation(format!(
                        "Repeated logical pad number {} has conflicting net assignments on {} (different nets)",
                        pad.number, pin.part_id
                    )));
                }
            }
            if pin_map
                .insert(key.clone(), net.id.clone())
                .is_some_and(|old| old != net.id)
            {
                return Err(validation(format!(
                    "Pad {} belongs to two nets",
                    pin.pad_id
                )));
            }
        }
    }
    Ok((nets, pin_map))
}

fn validate_plan(plan: &ExportPlan) -> Result<(), ArtifactError> {
    if plan.snapshot_token.is_empty() || plan.revision != plan.captured_document.revision {
        return Err(err(
            ArtifactErrorCode::StaleResult,
            "Export plan does not match its captured revision",
        ));
    }
    if plan.captured_document.format != "boardstudio/v2" {
        return Err(validation("Export requires a v2 document"));
    }
    unique_ids(
        plan.jobs.iter().map(|job| job.job_id.as_str()),
        "Ergogen job",
    )?;
    let mut indexes = HashSet::new();
    for net in &plan.reserved_nets {
        if net.index == 0 || !indexes.insert(net.index) {
            return Err(validation("Invalid reserved net table"));
        }
    }
    if plan.next_net_index
        != plan
            .reserved_nets
            .iter()
            .map(|net| net.index)
            .max()
            .unwrap_or(0)
            + 1
    {
        return Err(validation("Invalid net allocator state"));
    }
    if plan.fingerprint != plan_fingerprint(plan)? {
        return Err(err(
            ArtifactErrorCode::MismatchedResults,
            "Export plan snapshot was modified",
        ));
    }
    Ok(())
}

fn plan_fingerprint(plan: &ExportPlan) -> Result<String, ArtifactError> {
    let canonical = serde_json::to_vec(&(
        &plan.snapshot_token,
        plan.revision,
        &plan.target,
        &plan.jobs,
        &plan.reserved_nets,
        plan.next_net_index,
        &plan.contours,
        &plan.captured_document,
        &plan.model_paths,
    ))
    .map_err(|error| err(ArtifactErrorCode::Internal, error.to_string()))?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in canonical {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    Ok(format!("{hash:016x}"))
}

fn validate_contours(contours: &[Contour]) -> Result<(), ArtifactError> {
    if contours.is_empty() {
        return Err(validation("Board outline has no resolved contours"));
    }
    if contours.iter().any(|contour| {
        contour.points.len() < 3 || contour.points.iter().any(|point| !point_valid(*point))
    }) {
        return Err(validation(
            "Board contour needs at least three finite points",
        ));
    }
    Ok(())
}
