//! Retain PCB source, constituent footprints and net identities without inventing bought-board internals.
use super::{mechanical_extract, sexpr, source};
use crate::model::*;
use kiutils_sexpr::Node;
use std::collections::{BTreeMap, BTreeSet};

fn error(message: impl Into<String>) -> ArtifactError {
    ArtifactError::new(ArtifactErrorCode::Validation, message)
}
fn values(node: &Node) -> &[Node] {
    sexpr::items(node).unwrap_or(&[])
}
fn field<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    sexpr::child(node, name)
        .and_then(|n| values(n).get(1))
        .and_then(sexpr::atom)
}
fn number(node: &Node, index: usize) -> Option<f64> {
    values(node)
        .get(index)
        .and_then(sexpr::atom)
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}
fn property(node: &Node, key: &str) -> Option<String> {
    sexpr::children(node, "property")
        .find(|p| values(p).get(1).and_then(sexpr::atom) == Some(key))
        .and_then(|p| values(p).get(2))
        .and_then(sexpr::atom)
        .map(str::to_owned)
        .or_else(|| {
            sexpr::children(node, "fp_text")
                .find(|p| {
                    values(p).get(1).and_then(sexpr::atom)
                        == Some(if key == "Reference" {
                            "reference"
                        } else {
                            "value"
                        })
                })
                .and_then(|p| values(p).get(2))
                .and_then(sexpr::atom)
                .map(str::to_owned)
        })
}
fn reference(node: &Node, index: usize) -> String {
    property(node, "Reference")
        .filter(|reference| {
            !reference.trim().is_empty()
                && !reference
                    .chars()
                    .any(|character| matches!(character, '*' | '?'))
        })
        .unwrap_or_else(|| format!("PART{index}"))
}
fn retained(raw: &str, node: &Node) -> String {
    let span = sexpr::span(node);
    raw[span.start..span.end].to_owned()
}
fn gate(output: HardwareOutput, code: &str, message: &str) -> HardwareGate {
    HardwareGate {
        output,
        code: code.into(),
        message: message.into(),
    }
}

pub(super) fn import(
    raw: &str,
    id: &str,
    name: &str,
    mut provenance: HardwareSource,
    family: String,
    variant: String,
    repair: Option<ModuleCircuitRepair>,
) -> Result<ModuleDefinition, ArtifactError> {
    let parsed = kiutils_sexpr::parse_one(raw).map_err(|e| error(e.to_string()))?;
    let root = parsed
        .nodes
        .first()
        .filter(|n| sexpr::head(n) == Some("kicad_pcb"))
        .ok_or_else(|| error("Expected a KiCad PCB source"))?;
    if id.is_empty() || name.is_empty() {
        return Err(error("A module import requires a stable identity and name"));
    }
    let mut references = BTreeSet::new();
    for (index, fp) in values(root)
        .iter()
        .filter(|n| matches!(sexpr::head(n), Some("footprint" | "module")))
        .enumerate()
    {
        let fp_name = values(fp).get(1).and_then(sexpr::atom).unwrap_or("Unknown");
        let source_reference = property(fp, "Reference").unwrap_or_default();
        let reference = reference(fp, index);
        if fp_name.to_lowercase().contains("logo") || source_reference.starts_with("G***") {
            continue;
        }
        if !references.insert(reference.clone()) {
            return Err(error(format!(
                "Module PCB source contains duplicate footprint reference: {reference}"
            )));
        }
    }
    use sha2::{Digest, Sha256};
    provenance.sha256 = Some(
        Sha256::digest(raw.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    );
    let thickness = sexpr::child(root, "general")
        .and_then(|n| sexpr::child(n, "thickness"))
        .and_then(|n| number(n, 1))
        .filter(|t| *t > 0.0);
    // Adapt only explicitly authored board edges to the existing exact graphic parser.
    let edges: Vec<_> = values(root)
        .iter()
        .filter(|n| {
            field(n, "layer") == Some("Edge.Cuts")
                && matches!(
                    sexpr::head(n),
                    Some("gr_line" | "gr_arc" | "gr_circle" | "gr_rect" | "gr_poly")
                )
        })
        .map(|n| retained(raw, n).replacen("(gr_", "(fp_", 1))
        .collect();
    let footprint = format!("(footprint \"module-board\" {} )", edges.join("\n"));
    let mappings = [
        MechanicalGeometryKind::Line,
        MechanicalGeometryKind::Arc,
        MechanicalGeometryKind::Circle,
        MechanicalGeometryKind::Rectangle,
        MechanicalGeometryKind::Polygon,
    ]
    .map(|kind| MechanicalPurposeMapping {
        source_id: None,
        kind: Some(kind),
        layer: Some("Edge.Cuts".into()),
        purpose: MechanicalPurpose::PlateCutout,
    });
    let outline = mechanical_extract::extract(&footprint, &mappings)
        .and_then(|geometry| mechanical_extract::plate_cutout_contours(&geometry, 0.02));
    let mut gates = vec![
        gate(
            HardwareOutput::Mechanical,
            "assembled-envelope",
            "Select and qualify component heights, functional openings, mounting hardware and cable/service space before exact case output.",
        ),
        gate(
            HardwareOutput::Model,
            "assembly-model",
            "The selected complete assembly model and its footprint datums require qualification.",
        ),
        gate(
            HardwareOutput::Firmware,
            "module-driver",
            "The selected module's local firmware driver and resource configuration require qualification.",
        ),
    ];
    let mut polygons = match outline {
        Ok(polygons) => polygons,
        Err(e) => {
            gates.push(gate(
                HardwareOutput::Mechanical,
                "board-outline",
                &e.message,
            ));
            vec![]
        }
    };
    if polygons.is_empty() {
        gates.push(gate(
            HardwareOutput::Mechanical,
            "board-outline-missing",
            "No closed module board outline is available; its occupied volume cannot be inferred.",
        ));
    }
    if thickness.is_none() {
        gates.push(gate(
            HardwareOutput::Mechanical,
            "board-thickness",
            "Module PCB thickness is unknown.",
        ));
    }
    let mut min = Vec2 {
        x: f64::INFINITY,
        y: f64::INFINITY,
    };
    let mut max = Vec2 {
        x: f64::NEG_INFINITY,
        y: f64::NEG_INFINITY,
    };
    for p in polygons.iter().flatten() {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    let center = if min.x.is_finite() {
        Vec2 {
            x: (min.x + max.x) / 2.0,
            y: (min.y + max.y) / 2.0,
        }
    } else {
        Vec2::default()
    };
    for polygon in &mut polygons {
        for p in polygon {
            p.x -= center.x;
            p.y -= center.y;
        }
    }
    let contours = polygons
        .into_iter()
        .map(|points| Contour {
            points,
            hole: false,
        })
        .collect();
    let holes = match super::preview::board(raw, 0) {
        Ok(preview) => preview
            .holes
            .into_iter()
            .map(|hole| {
                hole.into_iter()
                    .map(|p| Vec2 {
                        x: p.x - center.x,
                        y: p.y - center.y,
                    })
                    .collect()
            })
            .collect(),
        Err(error) => {
            gates.push(gate(
                HardwareOutput::Mechanical,
                "board-hole-geometry",
                &format!(
                    "Source board hole geometry requires review: {}",
                    error.message
                ),
            ));
            vec![]
        }
    };
    let mut definitions = Vec::new();
    let mut parts = Vec::new();
    let mut nets: BTreeMap<String, Net> = BTreeMap::new();
    let mut constituents = Vec::new();
    let mut mounts = Vec::new();
    let mut interfaces = Vec::new();
    for (index, fp) in values(root)
        .iter()
        .filter(|n| matches!(sexpr::head(n), Some("footprint" | "module")))
        .enumerate()
    {
        let fp_name = values(fp)
            .get(1)
            .and_then(sexpr::atom)
            .unwrap_or("Unknown")
            .to_owned();
        let reference = reference(fp, index);
        let value = property(fp, "Value").unwrap_or_else(|| fp_name.clone());
        if fp_name.to_lowercase().contains("logo") || reference.starts_with("G***") {
            continue;
        }
        let definition_id = format!("{id}/component/{index}");
        let mut compiled = source::import_footprint(&retained(raw, fp), &definition_id)?;
        let at = sexpr::child(fp, "at");
        let pose = Pose2 {
            at: Vec2 {
                x: at.and_then(|n| number(n, 1)).unwrap_or(0.0) - center.x,
                y: -at.and_then(|n| number(n, 2)).unwrap_or(0.0) - center.y,
            },
            rotation: at.and_then(|n| number(n, 3)).unwrap_or(0.0),
        };
        let side = if field(fp, "layer") == Some("B.Cu") {
            Side::Back
        } else {
            Side::Front
        };
        let mut terminal_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (pad_index, pad) in sexpr::children(fp, "pad").enumerate() {
            let physical_id = format!("pad-{pad_index}");
            let pad_number = values(pad).get(1).and_then(sexpr::atom).unwrap_or("");
            if !pad_number.is_empty()
                && values(pad).get(2).and_then(sexpr::atom) != Some("np_thru_hole")
            {
                terminal_map
                    .entry(pad_number.into())
                    .or_default()
                    .push(physical_id.clone());
            }
            if let Some(net) = sexpr::child(pad, "net") {
                let items = values(net);
                let net_name = items.last().and_then(sexpr::atom).unwrap_or("");
                if !net_name.is_empty() && net_name != "0" {
                    let net_id = format!(
                        "net:{}",
                        net_name
                            .as_bytes()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    );
                    nets.entry(net_id.clone())
                        .or_insert_with(|| Net {
                            id: net_id,
                            name: net_name.into(),
                            pins: vec![],
                        })
                        .pins
                        .push(Pin {
                            part_id: reference.clone(),
                            pad_id: physical_id,
                        });
                }
            }
        }
        compiled.definition.terminals = terminal_map;
        let value_role_name = value
            .to_ascii_lowercase()
            .replace('_', "-")
            .replace(' ', "-");
        compiled.definition.name = format!(
            "{value} · {}",
            fp_name.split(':').next_back().unwrap_or(&fp_name)
        );
        let lower = fp_name.to_lowercase();
        let role_name = lower.replace('_', "-").replace(' ', "-");
        let role = if role_name.contains("vik-keyboard") || value_role_name.contains("vik-keyboard")
        {
            Some(VikRole::Host)
        } else if role_name.contains("vik-module") || value_role_name.contains("vik-module") {
            Some(VikRole::Module)
        } else {
            None
        };
        compiled.definition.hardware_profile = Some(HardwareProfile {
            source: provenance.clone(),
            gates: vec![],
            vik_role: role,
            footprint_surface_volumes: true,
        });
        compiled.definition.kind = if role.is_some() {
            PartKind::Connector
        } else if reference.starts_with(['R', 'C', 'D']) {
            PartKind::Passive
        } else {
            PartKind::Custom
        };
        if let Some(role) = role {
            use VikSignal::*;
            let mut signals = vec![
                Sclk, Miso, Cs, Gpio2, Mosi, Gpio1, V5, Rgb, Scl, Sda, Gnd, V3v3,
            ];
            if role == VikRole::Host {
                signals.reverse();
            }
            interfaces.push(ModuleInterface {
                id: reference.clone(),
                role,
                signals,
            });
        }
        if lower.contains("mountinghole")
            || reference.starts_with("MH")
            || reference.starts_with('H')
        {
            for pad in &compiled.definition.pads {
                if let Some(diameter) = pad.drill {
                    let (sin, cos) = pose.rotation.to_radians().sin_cos();
                    let x = if side == Side::Back {
                        -pad.at.x
                    } else {
                        pad.at.x
                    };
                    mounts.push(MechanicalPcbHole {
                        source_id: reference.clone(),
                        at: Vec2 {
                            x: pose.at.x + x * cos - pad.at.y * sin,
                            y: pose.at.y + x * sin + pad.at.y * cos,
                        },
                        diameter,
                    });
                }
            }
        }
        constituents.push(ModuleConstituent {
            reference: reference.clone(),
            name: value,
            footprint: fp_name,
            definition_id: Some(definition_id.clone()),
            purchased: false,
        });
        definitions.push(compiled.definition);
        parts.push(Part {
            id: reference.clone(),
            definition_id,
            reference,
            pose,
            side,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: None,
        });
    }
    // Ports are semantic contacts, independent of each author's net names.
    let mut ports = BTreeMap::new();
    for interface in &interfaces {
        let Some(part) = parts.iter().find(|p| p.id == interface.id) else {
            continue;
        };
        let Some(def) = definitions.iter().find(|d| d.id == part.definition_id) else {
            continue;
        };
        for (index, signal) in interface.signals.iter().enumerate() {
            let Some(pads) = def.terminals.get(&(index + 1).to_string()) else {
                continue;
            };
            if let Some(net) = nets.values().find(|n| {
                n.pins
                    .iter()
                    .any(|p| p.part_id == part.id && pads.contains(&p.pad_id))
            }) {
                let role = serde_json::to_value(signal).map_err(|e| error(e.to_string()))?;
                ports.insert(role.as_str().unwrap_or_default().to_owned(), net.id.clone());
            }
        }
    }
    let is_haptic = constituents
        .iter()
        .any(|item| item.reference == "U1" && item.name.to_ascii_uppercase().contains("DRV2605L"));
    if is_haptic {
        gates.push(gate(HardwareOutput::Electrical,"pullup-supply","Source JP1 connects the I2C pullups to an unpowered VCC net. Select the explicit 3.3V pullup repair with JP1 bridged."));
    }
    let mut definition = ModuleDefinition {
        id: id.into(),
        catalogue_row: None,
        name: name.into(),
        family,
        variant,
        source: provenance,
        board: ModuleBoard {
            contours,
            thickness,
            holes,
        },
        mounts,
        volumes: vec![],
        openings: vec![],
        models: vec![],
        candidate_models: vec![],
        gates,
        interfaces,
        electrical: ModuleElectrical {
            protocol: ModuleProtocol::Nonstandard,
            required_signals: vec![],
            logic_voltage: None,
            current_ma: None,
            i2c_address: None,
            pullup_ohms: None,
            driver: None,
            rotary_profile: None,
        },
        constituents,
        circuit: Some(ModuleCircuit {
            definitions,
            parts,
            nets: nets.into_values().collect(),
            ports,
            adaptations: vec![],
        }),
    };
    if let Some(ModuleCircuitRepair::Drv2605lPullups3v3) = repair {
        if !is_haptic {
            return Err(error(
                "The DRV2605L pullup repair requires its reviewed source circuit",
            ));
        }
        repair_haptic_pullups(&mut definition)?;
    }
    Ok(definition)
}

fn repair_haptic_pullups(definition: &mut ModuleDefinition) -> Result<(), ArtifactError> {
    let circuit = definition
        .circuit
        .as_mut()
        .ok_or_else(|| error("Repair requires an editable circuit"))?;
    let supply = circuit
        .nets
        .iter()
        .find(|net| net.name == "+3V3")
        .map(|net| net.id.clone())
        .ok_or_else(|| error("Reviewed 3.3V supply is missing"))?;
    let pullups = circuit
        .nets
        .iter()
        .find(|net| net.name == "I2C_3V3")
        .ok_or_else(|| error("Reviewed pullup supply is missing"))?;
    for reference in ["R1", "R2", "JP1"] {
        if !pullups.pins.iter().any(|pin| pin.part_id == reference) {
            return Err(error("Pullup source differs from the reviewed repair"));
        }
    }
    let jumper = circuit
        .nets
        .iter()
        .find(|net| net.name == "VCC")
        .ok_or_else(|| error("Reviewed jumper supply is missing"))?;
    if jumper.pins.is_empty() || jumper.pins.iter().any(|pin| pin.part_id != "JP1") {
        return Err(error("Jumper supply differs from the reviewed repair"));
    }
    let old_ids = [pullups.id.clone(), jumper.id.clone()];
    let mut added = Vec::new();
    circuit.nets.retain(|net| {
        if old_ids.contains(&net.id) {
            added.extend(net.pins.clone());
            false
        } else {
            true
        }
    });
    let rail = circuit
        .nets
        .iter_mut()
        .find(|net| net.id == supply)
        .expect("Supply retained");
    for pin in added {
        if !rail.pins.contains(&pin) {
            rail.pins.push(pin)
        }
    }
    for net in circuit.ports.values_mut() {
        if old_ids.contains(net) {
            *net = supply.clone()
        }
    }
    circuit.adaptations.push("Connect JP1 pad 2 to +3V3 and bridge JP1; R1/R2 4.7k pullups now use the 3.3V logic rail. Original source bytes remain unchanged.".into());
    definition.gates.retain(|item| item.code != "pullup-supply");
    definition.electrical = ModuleElectrical {
        protocol: ModuleProtocol::I2c,
        required_signals: vec![
            VikSignal::Gnd,
            VikSignal::V3v3,
            VikSignal::Scl,
            VikSignal::Sda,
        ],
        logic_voltage: Some(3.3),
        current_ma: None,
        i2c_address: Some(0x5a),
        pullup_ohms: Some(4_700.0),
        driver: None,
        rotary_profile: None,
    };
    Ok(())
}
