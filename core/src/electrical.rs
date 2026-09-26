//! One reviewed circuit plan feeds the inspector, PCB handoff and firmware.
use crate::model::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ElectricalPlanRequest {
    pub document: ProjectDoc,
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub mode: ElectricalMode,
    #[serde(default)]
    pub locks: BTreeMap<String, String>,
    #[serde(default)]
    pub controller_profile: Option<String>,
    #[serde(default)]
    pub board_id: Option<String>,
    #[serde(default)]
    pub controller_part_id: Option<String>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "kebab-case")]
pub enum ElectricalMode {
    #[default]
    Matrix,
    Direct,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ElectricalAssignment {
    pub key_id: String,
    pub matrix_id: String,
    pub row: u32,
    pub column: u32,
    pub row_pin: String,
    pub column_pin: String,
    pub locked: bool,
    pub row_firmware_gpio: Option<String>,
    pub column_firmware_gpio: Option<String>,
    pub direct_gpio: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ElectricalDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub key_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "export-types", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ElectricalPlan {
    pub instance_id: Option<String>,
    pub jumpers: Vec<crate::electrical_jumpers::JumperRecipe>,
    pub module_aliases: BTreeMap<String, String>,
    pub mode: ElectricalMode,
    pub assignments: Vec<ElectricalAssignment>,
    pub row_pins: Vec<String>,
    pub column_pins: Vec<String>,
    pub diagnostics: Vec<ElectricalDiagnostic>,
    pub fingerprint: String,
    pub board_id: Option<String>,
    pub controller_part_id: Option<String>,
    pub revision: u64,
    pub controller_profile: Option<String>,
    pub free_pins: Vec<String>,
    pub nets: Vec<Net>,
    pub diode_direction: String,
    pub peripherals: Vec<crate::electrical_peripherals::PeripheralRequirement>,
    pub peripheral_pins: BTreeMap<String, String>,
    pub peripheral_terminals: BTreeMap<String, String>,
}
fn diagnostic(
    items: &mut Vec<ElectricalDiagnostic>,
    code: &str,
    message: impl Into<String>,
    key: Option<&str>,
) {
    items.push(ElectricalDiagnostic {
        code: code.into(),
        severity: "error".into(),
        message: message.into(),
        key_id: key.map(str::to_string),
    });
}
fn definition<'a>(doc: &'a ProjectDoc, part: &Part) -> Option<&'a PartDefinition> {
    doc.definitions
        .iter()
        .find(|item| item.id == part.definition_id)
}
fn parameter<'a>(
    part: &'a Part,
    definition: &'a PartDefinition,
    name: &str,
) -> Option<&'a serde_json::Value> {
    let value = part
        .generator_parameters
        .as_ref()
        .and_then(|values| values.get(name))
        .or_else(|| {
            definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(name))
        })?;
    Some(value.get("value").unwrap_or(value))
}
fn enabled(part: &Part, definition: &PartDefinition, name: &str, fallback: bool) -> bool {
    parameter(part, definition, name)
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}
fn pins(definition: &PartDefinition, terminal: &str, part: &str) -> Vec<Pin> {
    let ids = definition
        .terminals
        .get(terminal)
        .cloned()
        .unwrap_or_else(|| {
            definition
                .pads
                .iter()
                .filter(|pad| pad.id == terminal)
                .map(|pad| pad.id.clone())
                .collect()
        });
    ids.into_iter()
        .filter(|id| {
            definition
                .pads
                .iter()
                .any(|pad| pad.id == *id && pad.plated != Some(false))
        })
        .map(|pad_id| Pin {
            part_id: part.into(),
            pad_id,
        })
        .collect()
}
fn terminal<'a>(definition: &'a PartDefinition, row: bool) -> &'a str {
    definition
        .matrix_terminals
        .as_ref()
        .map(|terminals| {
            if row {
                terminals.row.as_str()
            } else {
                terminals.column.as_str()
            }
        })
        .unwrap_or(if row { "one" } else { "two" })
}
fn diode_terminals(definition: &PartDefinition) -> Option<(&str, &str)> {
    // Only known semantic terminals qualify; pad numbers never establish polarity.
    for (anode, cathode) in [("anode", "cathode"), ("A", "K"), ("from", "to")] {
        let known_source = definition.generator.as_ref().is_some_and(|generator| {
            generator.source == "ceoloide/diode_tht_sod123"
        });
        if anode == "from" && !known_source {
            continue;
        }
        if !pins(definition, anode, "").is_empty() && !pins(definition, cathode, "").is_empty() {
            return Some((anode, cathode));
        }
    }
    None
}
fn add_net(nets: &mut BTreeMap<String, Net>, prefix: &str, suffix: &str, pins: Vec<Pin>) {
    let id = format!("{prefix}{suffix}");
    nets.entry(id.clone())
        .or_insert_with(|| Net {
            id,
            name: suffix.replace('/', "_").to_uppercase(),
            pins: vec![],
        })
        .pins
        .extend(pins);
}
fn signature<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("electrical input serializes"))
    )
}

struct MatrixKey<'a> {
    part: &'a Part,
    definition: &'a PartDefinition,
    matrix: &'a Matrix,
    diode: Option<(&'a Part, &'a PartDefinition)>,
}

/// Discover the logical key population before any pin allocation occurs.
/// Keeping this separate makes the electrical planner operate on one stable,
/// board-scoped view of the layout.
fn discover_keys<'a>(
    doc: &'a ProjectDoc,
    board: &Board,
    mode: ElectricalMode,
    diagnostics: &mut Vec<ElectricalDiagnostic>,
) -> Vec<MatrixKey<'a>> {
    let mut keys = vec![];
    for matrix in &doc.matrices {
        if matrix.board_id.as_ref().is_some_and(|id| id != &board.id) {
            continue;
        }
        let prefix = format!("matrix/{}/", matrix.id);
        let mut members = BTreeMap::new();
        for row in 0..matrix.rows {
            for column in 0..matrix.columns {
                if matrix.cells.iter().any(|cell| cell.row == row && cell.column == column && !cell.enabled) {
                    continue;
                }
                members.insert((row, column), format!("{prefix}r{row}c{column}"));
            }
        }
        for column in 0..matrix.columns {
            for row in 0..matrix.rows {
                let Some(id) = members.get(&(row, column)) else { continue };
                let Some(part) = doc.parts.iter().find(|part| part.id == *id && board.part_ids.contains(&part.id)) else { continue };
                let Some(definition) = definition(doc, part) else { continue };
                let companions = doc.parts.iter().filter(|candidate| {
                    candidate.id.starts_with(&format!("{id}/")) && board.part_ids.contains(&candidate.id)
                }).filter_map(|candidate| {
                    let definition = doc.definitions.iter().find(|definition| definition.id == candidate.definition_id)?;
                    diode_terminals(definition).map(|_| (candidate, definition))
                }).collect::<Vec<_>>();
                if mode == ElectricalMode::Matrix && companions.len() != 1 {
                    diagnostic(diagnostics, "matrix-diode-required", format!("{} needs exactly one diode with known polarity", part.reference), Some(id));
                }
                if pins(definition, terminal(definition, true), id).is_empty() || pins(definition, terminal(definition, false), id).is_empty() {
                    diagnostic(diagnostics, "switch-terminals-missing", format!("{} needs two named switch terminals", part.reference), Some(id));
                }
                keys.push(MatrixKey { part, definition, matrix, diode: companions.first().copied() });
            }
        }
    }
    keys
}

fn allocate_pins<'a>(
    functions: &[String],
    capabilities: &[&'a crate::electrical_profiles::PinCapability],
    locks: &BTreeMap<String, String>,
    baseline: Option<&ElectricalHandoffBaseline>,
    previous: &BTreeMap<String, String>,
    diagnostics: &mut Vec<ElectricalDiagnostic>,
) -> (BTreeMap<String, &'a crate::electrical_profiles::PinCapability>, BTreeSet<&'a str>) {
    let mut allocated = BTreeMap::new();
    let mut used = BTreeSet::new();
    for function in functions {
        if let (Some(lock), Some(protected)) = (locks.get(function), baseline.and_then(|item| item.assignments.get(function))) {
            let requested = capabilities.iter().find(|pin| pin.terminal == lock || pin.firmware_gpio == lock);
            if !requested.is_some_and(|pin| pin.terminal == protected || pin.firmware_gpio == protected) {
                diagnostic(diagnostics, "protected-pin-change", format!("{function} was handed off on {protected}; review a remap before changing it"), None);
            }
        }
        let pinned = locks.get(function).or_else(|| baseline.and_then(|item| item.assignments.get(function)));
        if let Some(value) = pinned {
            match capabilities.iter().find(|pin| pin.terminal == value || pin.firmware_gpio == value).copied() {
                Some(pin) if used.insert(pin.firmware_gpio) => { allocated.insert(function.clone(), pin); }
                Some(_) => diagnostic(diagnostics, "conflicting-lock", format!("{function} conflicts with another assignment to {value}"), None),
                None => diagnostic(diagnostics, "unavailable-locked-pin", format!("{function} requires {value}; review the pin assignment before changing it"), None),
            }
        }
    }
    for function in functions {
        if allocated.contains_key(function) || locks.contains_key(function) || baseline.is_some_and(|item| item.assignments.contains_key(function)) { continue; }
        if let Some(previous_pin) = previous.get(function) {
            if let Some(pin) = capabilities.iter().find(|pin| pin.terminal == previous_pin && !used.contains(pin.firmware_gpio)).copied() {
                used.insert(pin.firmware_gpio);
                allocated.insert(function.clone(), pin);
            }
        }
    }
    for function in functions {
        if allocated.contains_key(function) || locks.contains_key(function) || baseline.is_some_and(|item| item.assignments.contains_key(function)) { continue; }
        if let Some(pin) = capabilities.iter().find(|pin| !used.contains(pin.firmware_gpio)).copied() {
            used.insert(pin.firmware_gpio);
            allocated.insert(function.clone(), pin);
        } else {
            diagnostic(diagnostics, "insufficient-controller-pins", format!("No available GPIO for {function}. Choose a larger controller or change wiring mode explicitly."), None);
        }
    }
    (allocated, used)
}

fn finalize_nets(
    mut nets: BTreeMap<String, Net>,
    prefix: &str,
    manual_nets: &[Net],
    diagnostics: &mut Vec<ElectricalDiagnostic>,
) -> Vec<Net> {
    for net in nets.values_mut() {
        net.pins.sort_by(|a, b| (&a.part_id, &a.pad_id).cmp(&(&b.part_id, &b.pad_id)));
        net.pins.dedup();
        for pin in &net.pins {
            if let Some(manual) = manual_nets.iter().find(|other| !other.id.starts_with(prefix) && other.pins.contains(pin)) {
                diagnostic(diagnostics, "manual-net-conflict", format!("{} already belongs to manual net {}; review it before automatic wiring", pin.part_id, manual.name), Some(&pin.part_id));
            }
        }
    }
    nets.into_values().filter(|net| !net.pins.is_empty()).collect()
}

fn resolve_jumpers(
    doc: &ProjectDoc,
    board: &Board,
    config: Option<&ElectricalBoardConfiguration>,
    instance: Option<&PhysicalBoardInstance>,
    controller_id: Option<&String>,
    diagnostics: &mut Vec<ElectricalDiagnostic>,
) -> (Vec<crate::electrical_jumpers::JumperRecipe>, BTreeMap<String, String>) {
    let mut recipes = vec![];
    let mut aliases = BTreeMap::new();
    for part in doc.parts.iter().filter(|part| board.part_ids.contains(&part.id)) {
        let Some(definition) = definition(doc, part) else { continue };
        let mut populated = part.clone();
        if instance.is_some_and(|item| item.flipped) {
            populated.side = if part.side == Side::Front { Side::Back } else { Side::Front };
            populated.generator_parameters.get_or_insert_with(Default::default).insert("side".into(), serde_json::json!(if populated.side == Side::Front { "F" } else { "B" }));
        }
        if let Some(recipe) = crate::electrical_jumpers::describe(&populated, definition) {
            diagnostics.extend(recipe.diagnostics.iter().map(|item| ElectricalDiagnostic { code: item.code.clone(), severity: item.severity.clone(), message: format!("{}: {}", part.reference, item.message), key_id: Some(part.id.clone()) }));
            if controller_id == Some(&part.id) { aliases.extend(recipe.terminal_aliases.clone()); }
            if let Some(config) = config {
                for site in &recipe.sites {
                    if let Some(state) = config.jumper_states.get(&site.id) {
                        if (*state == crate::electrical_profiles::JumperState::Bridged) != site.close {
                            diagnostic(diagnostics, "jumper-state-conflict", format!("{}: jumper {} disagrees with the selected population face", part.reference, site.id), Some(&part.id));
                        }
                    }
                }
            }
            recipes.push(recipe);
        } else if enabled(part, definition, "reversible", false) && definition.generator.as_ref().is_some_and(|generator| generator.source.contains("nice_view")) {
            diagnostic(diagnostics, "jumper-profile-required", format!("{} needs a reviewed jumper recipe", part.reference), Some(&part.id));
        }
    }
    (recipes, aliases)
}

fn required_signals<'a>(
    peripherals: &[crate::electrical_peripherals::PeripheralRequirement],
    mode: ElectricalMode,
    rows: usize,
    columns: usize,
    keys: impl Iterator<Item = &'a MatrixKey<'a>>,
) -> Vec<String> {
    let mut functions = peripherals
        .iter()
        .flat_map(|peripheral| peripheral.gpio_terminals.iter().map(|(_, function)| function.clone()))
        .collect::<BTreeSet<_>>();
    if mode == ElectricalMode::Matrix {
        functions.extend((0..rows).map(|row| format!("row/{row}")));
        functions.extend((0..columns).map(|column| format!("column/{column}")));
    } else {
        functions.extend(keys.map(|key| key.part.id.clone()));
    }
    functions.into_iter().collect()
}

struct NetConstruction<'a> {
    doc: &'a ProjectDoc,
    keys: &'a [MatrixKey<'a>],
    controller: Option<&'a Part>,
    controller_definition: Option<&'a PartDefinition>,
    peripherals: &'a [crate::electrical_peripherals::PeripheralRequirement],
    rgb_parts: &'a [String],
    allocated: &'a BTreeMap<String, &'a crate::electrical_profiles::PinCapability>,
    locks: &'a BTreeMap<String, String>,
    mode: ElectricalMode,
    columns: usize,
    diode_direction: &'a str,
    prefix: &'a str,
}

#[derive(Default)]
struct ConstructedNets {
    nets: BTreeMap<String, Net>,
    assignments: Vec<ElectricalAssignment>,
    peripheral_pins: BTreeMap<String, String>,
    diagnostics: Vec<ElectricalDiagnostic>,
}

fn construct_nets(input: NetConstruction<'_>) -> ConstructedNets {
    let NetConstruction {
        doc,
        keys,
        controller,
        controller_definition,
        peripherals,
        rgb_parts,
        allocated,
        locks,
        mode,
        columns,
        diode_direction,
        prefix,
    } = input;
    let mut result = ConstructedNets::default();
    let has_power_switch = peripherals
        .iter()
        .any(|peripheral| peripheral.kind == "power-switch");
    for peripheral in peripherals {
        let Some(part) = doc.parts.iter().find(|part| part.id == peripheral.part_id) else {
            continue;
        };
        let Some(definition) = definition(doc, part) else {
            continue;
        };
        for (terminal, function) in &peripheral.gpio_terminals {
            let connections = pins(definition, terminal, &part.id);
            if connections.is_empty() {
                diagnostic(
                    &mut result.diagnostics,
                    "peripheral-terminal-missing",
                    format!("{} has no available {terminal} terminal", part.reference),
                    Some(&part.id),
                );
            }
            add_net(&mut result.nets, prefix, function, connections);
            if let (Some(controller), Some(definition), Some(pin)) =
                (controller, controller_definition, allocated.get(function))
            {
                add_net(
                    &mut result.nets,
                    prefix,
                    function,
                    pins(definition, pin.terminal, &controller.id),
                );
                result
                    .peripheral_pins
                    .insert(function.clone(), pin.firmware_gpio.into());
            }
        }
        for (terminal, destination) in &peripheral.fixed_terminals {
            let target = if destination == "BAT_P" && !has_power_switch {
                "RAW"
            } else {
                destination.as_str()
            };
            let function = format!("power/{}", target.to_lowercase());
            add_net(
                &mut result.nets,
                prefix,
                &function,
                pins(definition, terminal, &part.id),
            );
            if let (Some(controller), Some(definition)) = (controller, controller_definition) {
                if target != "BAT_P" {
                    add_net(
                        &mut result.nets,
                        prefix,
                        &function,
                        pins(definition, target, &controller.id),
                    );
                }
            }
        }
    }
    for pair in rgb_parts.windows(2) {
        let Some(previous) = doc.parts.iter().find(|part| part.id == pair[0]) else {
            continue;
        };
        let Some(next) = doc.parts.iter().find(|part| part.id == pair[1]) else {
            continue;
        };
        let (Some(previous_def), Some(next_def)) =
            (definition(doc, previous), definition(doc, next))
        else {
            continue;
        };
        let mut connections = pins(previous_def, "P2", &previous.id);
        connections.extend(pins(next_def, "P4", &next.id));
        if connections.len() < 2 {
            diagnostic(
                &mut result.diagnostics,
                "rgb-terminal-missing",
                "RGB chain needs reviewed DIN and DOUT pads",
                Some(&previous.id),
            );
        }
        add_net(
            &mut result.nets,
            prefix,
            &format!("rgb-chain/{}", previous.id),
            connections,
        );
    }
    for (index, key) in keys.iter().enumerate() {
        let row = if mode == ElectricalMode::Direct {
            0
        } else {
            index / columns.max(1)
        };
        let column = if mode == ElectricalMode::Direct {
            index
        } else {
            index % columns.max(1)
        };
        let row_function = format!("row/{row}");
        let column_function = format!("column/{column}");
        let row_cap = allocated.get(&row_function);
        let col_cap = allocated.get(&column_function);
        let direct = allocated.get(&key.part.id);
        let switch_row = pins(key.definition, terminal(key.definition, true), &key.part.id);
        let switch_column = pins(
            key.definition,
            terminal(key.definition, false),
            &key.part.id,
        );
        if mode == ElectricalMode::Direct {
            add_net(
                &mut result.nets,
                prefix,
                &format!("direct/{}", key.part.id),
                switch_column,
            );
            add_net(&mut result.nets, prefix, "power/gnd", switch_row);
            if let (Some(controller), Some(definition), Some(pin)) =
                (controller, controller_definition, direct)
            {
                add_net(
                    &mut result.nets,
                    prefix,
                    &format!("direct/{}", key.part.id),
                    pins(definition, pin.terminal, &controller.id),
                );
                add_net(
                    &mut result.nets,
                    prefix,
                    "power/gnd",
                    pins(definition, "GND", &controller.id),
                );
            }
        } else {
            add_net(&mut result.nets, prefix, &column_function, switch_column);
            if let Some((diode, definition)) = key.diode {
                if let Some((anode, cathode)) = diode_terminals(definition) {
                    let (row_terminal, link_terminal) = if diode_direction == "row2col" {
                        (anode, cathode)
                    } else {
                        (cathode, anode)
                    };
                    add_net(
                        &mut result.nets,
                        prefix,
                        &row_function,
                        pins(definition, row_terminal, &diode.id),
                    );
                    let mut link = switch_row;
                    link.extend(pins(definition, link_terminal, &diode.id));
                    add_net(
                        &mut result.nets,
                        prefix,
                        &format!("link/{}", key.part.id),
                        link,
                    );
                }
            }
            if let (Some(controller), Some(definition)) = (controller, controller_definition) {
                if let Some(pin) = row_cap {
                    add_net(
                        &mut result.nets,
                        prefix,
                        &row_function,
                        pins(definition, pin.terminal, &controller.id),
                    );
                }
                if let Some(pin) = col_cap {
                    add_net(
                        &mut result.nets,
                        prefix,
                        &column_function,
                        pins(definition, pin.terminal, &controller.id),
                    );
                }
            }
        }
        result.assignments.push(ElectricalAssignment {
            key_id: key.part.id.clone(),
            matrix_id: key.matrix.id.clone(),
            row: row as u32,
            column: column as u32,
            row_pin: row_cap
                .map(|pin| pin.terminal.to_string())
                .unwrap_or_default(),
            column_pin: col_cap
                .or(direct)
                .map(|pin| pin.terminal.to_string())
                .unwrap_or_default(),
            row_firmware_gpio: row_cap.map(|pin| pin.firmware_gpio.to_string()),
            column_firmware_gpio: col_cap.map(|pin| pin.firmware_gpio.to_string()),
            direct_gpio: direct.map(|pin| pin.firmware_gpio.to_string()),
            locked: locks.contains_key(&key.part.id)
                || locks.contains_key(&row_function)
                || locks.contains_key(&column_function),
        });
    }
    result
}

pub fn resolve(request: ElectricalPlanRequest) -> ElectricalPlan {
    let doc = &request.document;
    let board = request
        .board_id
        .as_ref()
        .and_then(|id| doc.boards.iter().find(|board| board.id == *id))
        .or_else(|| {
            if request.board_id.is_none() && doc.boards.len() == 1 {
                doc.boards.first()
            } else {
                None
            }
        });
    let mut plan = ElectricalPlan {
        instance_id: request.instance_id.clone(),
        jumpers: vec![],
        module_aliases: BTreeMap::new(),
        mode: request.mode,
        assignments: vec![],
        row_pins: vec![],
        column_pins: vec![],
        diagnostics: vec![],
        fingerprint: String::new(),
        board_id: board.map(|board| board.id.clone()),
        controller_part_id: None,
        controller_profile: None,
        revision: doc.revision,
        free_pins: vec![],
        nets: vec![],
        diode_direction: "row2col".into(),
        peripherals: vec![],
        peripheral_pins: BTreeMap::new(),
        peripheral_terminals: BTreeMap::new(),
    };
    let Some(board) = board else {
        diagnostic(
            &mut plan.diagnostics,
            "missing-board",
            "Select one PCB design to resolve wiring",
            None,
        );
        return plan;
    };
    let config = doc.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|entry| entry.board_id == board.id)
    });
    let selected = request
        .controller_part_id
        .as_ref()
        .or_else(|| config.and_then(|config| config.controller_part_id.as_ref()));
    let controllers = doc
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
        .filter(|part| {
            definition(doc, part).is_some_and(|definition| {
                definition.kind == PartKind::Controller
                    || definition
                        .generator
                        .as_ref()
                        .and_then(|generator| {
                            crate::electrical_profiles::profile(&generator.source)
                        })
                        .is_some_and(|profile| profile.kind == "controller")
            })
        })
        .collect::<Vec<_>>();
    let controller = selected
        .and_then(|id| controllers.iter().find(|part| part.id == *id).copied())
        .or_else(|| {
            if selected.is_none() && controllers.len() == 1 {
                controllers.first().copied()
            } else {
                None
            }
        });
    let controller_definition = controller.and_then(|part| definition(doc, part));
    let source = controller_definition
        .and_then(|definition| definition.generator.as_ref())
        .map(|generator| generator.source.as_str());
    let profile = source
        .and_then(crate::electrical_profiles::profile)
        .filter(|profile| profile.kind == "controller");
    if controller.is_none() {
        diagnostic(
            &mut plan.diagnostics,
            "missing-controller",
            "Place and select a controller on this PCB before resolving wiring",
            None,
        );
    } else if profile.is_none() {
        diagnostic(
            &mut plan.diagnostics,
            "unsupported-controller",
            "This controller needs a reviewed pin profile",
            None,
        );
    }
    plan.controller_part_id = controller.map(|part| part.id.clone());
    plan.controller_profile = source.map(str::to_string);
    if request
        .controller_profile
        .as_deref()
        .is_some_and(|requested| Some(requested) != source)
    {
        diagnostic(
            &mut plan.diagnostics,
            "controller-profile-mismatch",
            "The selected profile does not match the placed controller",
            None,
        );
    }
    let mut capabilities = vec![];
    if let (Some(controller), Some(definition), Some(profile)) =
        (controller, controller_definition, profile.as_ref())
    {
        let extras = enabled(controller, definition, "include_extra_pins", false);
        let reversible = enabled(controller, definition, "reversible", false);
        let reduced = enabled(controller, definition, "only_required_jumpers", false);
        if enabled(controller, definition, "invert_jumpers_position", false) {
            diagnostic(
                &mut plan.diagnostics,
                "unsupported-jumper-variant",
                "Inverted MCU jumper positions are unsupported by this footprint",
                Some(&controller.id),
            );
        }
        for pin in profile.pins {
            if pin.terminal.starts_with("P10") && pin.terminal.len() > 3 && !extras {
                continue;
            }
            if pin.terminal == "P107"
                && source.is_some_and(|source| source.contains("supermini"))
                && reversible
                && reduced
            {
                continue;
            }
            if profile.reserved_gpios.contains(&pin.firmware_gpio)
                || pins(definition, pin.terminal, &controller.id).is_empty()
            {
                continue;
            }
            capabilities.push(pin);
        }
    }
    let keys = discover_keys(doc, board, plan.mode, &mut plan.diagnostics);
    if keys.is_empty() {
        diagnostic(
            &mut plan.diagnostics,
            "no-keys",
            "Place a key layout on this PCB before resolving wiring",
            None,
        );
    }
    let directions = keys
        .iter()
        .map(|key| {
            match key
                .matrix
                .diode_direction
                .unwrap_or(DiodeDirection::Row2col)
            {
                DiodeDirection::Row2col => "row2col",
                DiodeDirection::Col2row => "col2row",
            }
        })
        .collect::<BTreeSet<_>>();
    if directions.len() > 1 && plan.mode == ElectricalMode::Matrix {
        diagnostic(
            &mut plan.diagnostics,
            "mixed-diode-directions",
            "All clusters on one controller must use the same diode direction",
            None,
        );
    }
    plan.diode_direction = directions.first().unwrap_or(&"row2col").to_string();
    let count = keys.len();
    let (rows, columns) = if plan.mode == ElectricalMode::Direct {
        (0, count)
    } else {
        (1..=count.max(1))
            .map(|rows| (rows, count.div_ceil(rows)))
            .min_by_key(|(rows, cols)| (rows + cols, rows.abs_diff(*cols), *rows))
            .unwrap()
    };
    let mut locks = config
        .map(|config| config.locks.clone())
        .unwrap_or_default();
    locks.extend(request.locks.clone());
    let baseline = config.and_then(|config| config.protected_handoff.as_ref());
    plan.peripherals = crate::electrical_peripherals::describe(doc, &board.id);
    let rgb_parts = plan
        .peripherals
        .iter()
        .filter(|peripheral| peripheral.kind == "rgb")
        .map(|peripheral| peripheral.part_id.clone())
        .collect::<Vec<_>>();
    for peripheral in plan
        .peripherals
        .iter_mut()
        .filter(|peripheral| peripheral.kind == "rgb")
        .skip(1)
    {
        peripheral.gpio_terminals.clear();
    }
    if plan
        .peripherals
        .iter()
        .filter(|peripheral| peripheral.kind == "display-i2c")
        .count()
        > 1
    {
        diagnostic(
            &mut plan.diagnostics,
            "i2c-address-conflict",
            "The SSD1306 displays share address 0x3C; configure distinct addresses before wiring",
            None,
        );
    }
    for part in doc
        .parts
        .iter()
        .filter(|part| board.part_ids.contains(&part.id))
    {
        let Some(definition) = definition(doc, part) else {
            continue;
        };
        if Some(&part.id) == plan.controller_part_id.as_ref()
            || keys.iter().any(|key| {
                key.part.id == part.id || key.diode.is_some_and(|(diode, _)| diode.id == part.id)
            })
            || plan
                .peripherals
                .iter()
                .any(|peripheral| peripheral.part_id == part.id)
            || definition
                .pads
                .iter()
                .all(|pad| pad.plated == Some(false) || pad.number.is_empty())
        {
            continue;
        }
        diagnostic(
            &mut plan.diagnostics,
            "peripheral-profile-required",
            format!(
                "{} needs a reviewed electrical profile before automatic handoff",
                part.reference
            ),
            Some(&part.id),
        );
    }
    let functions = required_signals(
        &plan.peripherals,
        plan.mode,
        rows,
        columns,
        keys.iter(),
    );
    let (allocated, used) = allocate_pins(
        &functions,
        &capabilities,
        &locks,
        baseline,
        &config.map(|item| item.assignments.clone()).unwrap_or_default(),
        &mut plan.diagnostics,
    );
    plan.free_pins = capabilities
        .iter()
        .filter(|pin| !used.contains(pin.firmware_gpio))
        .map(|pin| pin.terminal.to_string())
        .collect();
    plan.row_pins = (0..rows)
        .map(|row| {
            allocated
                .get(&format!("row/{row}"))
                .map(|pin| pin.terminal.to_string())
                .unwrap_or_default()
        })
        .collect();
    plan.column_pins = if plan.mode == ElectricalMode::Matrix {
        (0..columns)
            .map(|column| {
                allocated
                    .get(&format!("column/{column}"))
                    .map(|pin| pin.terminal.to_string())
                    .unwrap_or_default()
            })
            .collect()
    } else {
        vec![]
    };
    let prefix = format!("generated/electrical/{}/", board.id);
    let constructed = construct_nets(NetConstruction {
        doc,
        keys: &keys,
        controller,
        controller_definition,
        peripherals: &plan.peripherals,
        rgb_parts: &rgb_parts,
        allocated: &allocated,
        locks: &locks,
        mode: plan.mode,
        columns,
        diode_direction: &plan.diode_direction,
        prefix: &prefix,
    });
    plan.assignments = constructed.assignments;
    plan.peripheral_pins = constructed.peripheral_pins;
    plan.diagnostics.extend(constructed.diagnostics);
    plan.nets = finalize_nets(constructed.nets, &prefix, &doc.nets, &mut plan.diagnostics);
    let instance = request.instance_id.as_ref().and_then(|id| {
        doc.hardware
            .as_ref()?
            .instances
            .iter()
            .find(|instance| instance.id == *id && instance.board_id == board.id)
    });
    if request.instance_id.is_some() && instance.is_none() {
        diagnostic(
            &mut plan.diagnostics,
            "missing-physical-instance",
            "The selected physical assembly no longer belongs to this PCB",
            None,
        );
    }
    let (jumpers, aliases) = resolve_jumpers(
        doc,
        board,
        config,
        instance,
        plan.controller_part_id.as_ref(),
        &mut plan.diagnostics,
    );
    plan.jumpers = jumpers;
    plan.module_aliases = aliases;
    if let Some(profile) = profile.as_ref() {
        let gpio = |terminal: &str| {
            profile
                .pins
                .iter()
                .find(|pin| {
                    pin.terminal
                        == plan
                            .module_aliases
                            .get(terminal)
                            .map(String::as_str)
                            .unwrap_or(terminal)
                })
                .map(|pin| pin.firmware_gpio.to_string())
        };
        for assignment in &mut plan.assignments {
            if plan.mode == ElectricalMode::Direct {
                assignment.direct_gpio = gpio(&assignment.column_pin);
            } else {
                assignment.row_firmware_gpio = gpio(&assignment.row_pin);
                assignment.column_firmware_gpio = gpio(&assignment.column_pin);
            }
        }
        for (function, pin) in &allocated {
            if plan.peripheral_pins.contains_key(function) {
                if let Some(gpio) = gpio(pin.terminal) {
                    plan.peripheral_pins.insert(function.clone(), gpio);
                    plan.peripheral_terminals.insert(
                        function.clone(),
                        plan.module_aliases
                            .get(pin.terminal)
                            .cloned()
                            .unwrap_or_else(|| pin.terminal.into()),
                    );
                }
            }
        }
    }

    plan.fingerprint = signature(&(
        plan.mode,
        &plan.board_id,
        &plan.controller_part_id,
        &plan.assignments,
        &plan.nets,
        &plan.diode_direction,
        &plan.instance_id,
        &plan.jumpers,
        &config.map(|config| &config.jumper_states),
    ));
    plan
}

pub fn materialize(document: &mut ProjectDoc, plan: &ElectricalPlan) -> Result<(), String> {
    materialize_reviewed(document, plan, false)
}

pub(crate) fn materialize_reviewed(
    document: &mut ProjectDoc,
    plan: &ElectricalPlan,
    draft: bool,
) -> Result<(), String> {
    if plan.revision != document.revision {
        return Err("Wiring plan is stale; resolve the current board".into());
    }
    if plan.diagnostics.iter().any(|item| {
        item.severity == "error"
            && (!draft
                || matches!(
                    item.code.as_str(),
                    "manual-net-conflict"
                        | "conflicting-lock"
                        | "unavailable-locked-pin"
                        | "protected-pin-change"
                ))
    }) {
        return Err("Resolve wiring findings before applying the plan".into());
    }
    let board_id = plan
        .board_id
        .as_ref()
        .ok_or("Wiring plan has no PCB design")?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == *board_id)
        .ok_or("PCB design is missing")?;
    for net in &plan.nets {
        for pin in &net.pins {
            if !board.part_ids.contains(&pin.part_id)
                || !document
                    .parts
                    .iter()
                    .find(|part| part.id == pin.part_id)
                    .and_then(|part| definition(document, part))
                    .is_some_and(|definition| {
                        definition
                            .pads
                            .iter()
                            .any(|pad| pad.id == pin.pad_id && pad.plated != Some(false))
                    })
            {
                return Err(format!(
                    "Wiring contains a missing or non-electrical pad: {}/{}",
                    pin.part_id, pin.pad_id
                ));
            }
        }
    }
    let prefix = format!("generated/electrical/{board_id}/");
    let removed = document
        .nets
        .iter()
        .filter(|net| net.id.starts_with(&prefix))
        .map(|net| net.id.clone())
        .collect::<BTreeSet<_>>();
    document.nets.retain(|net| !removed.contains(&net.id));
    for board in &mut document.boards {
        board.net_ids.retain(|id| !removed.contains(id));
    }
    document.nets.extend(plan.nets.clone());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == *board_id)
        .unwrap()
        .net_ids
        .extend(plan.nets.iter().map(|net| net.id.clone()));
    let hardware = document.hardware.get_or_insert_with(Default::default);
    let index = hardware
        .boards
        .iter()
        .position(|config| config.board_id == *board_id)
        .unwrap_or_else(|| {
            hardware.boards.push(ElectricalBoardConfiguration {
                board_id: board_id.clone(),
                ..Default::default()
            });
            hardware.boards.len() - 1
        });
    hardware.boards[index].assignments = handoff_assignments(plan);
    if plan.mode == ElectricalMode::Direct {
        hardware.boards[index].assignments.extend(
            plan.assignments
                .iter()
                .map(|assignment| (assignment.key_id.clone(), assignment.column_pin.clone())),
        );
    } else {
        hardware.boards[index].assignments.extend(
            plan.row_pins
                .iter()
                .enumerate()
                .map(|(i, pin)| (format!("row/{i}"), pin.clone()))
                .chain(
                    plan.column_pins
                        .iter()
                        .enumerate()
                        .map(|(i, pin)| (format!("column/{i}"), pin.clone())),
                ),
        );
    }
    hardware.boards[index].mode = plan.mode;
    hardware.boards[index].controller_part_id = plan.controller_part_id.clone();
    Ok(())
}

pub(crate) fn handoff_assignments(plan: &ElectricalPlan) -> BTreeMap<String, String> {
    // Protect fabricated signal assignments. A reversible population can map
    // those signals to different module GPIOs without changing the PCB copper.
    let mut pins = plan
        .peripheral_terminals
        .iter()
        .map(|(function, terminal)| {
            let canonical = plan
                .module_aliases
                .iter()
                .find(|(_, module)| *module == terminal)
                .map(|(source, _)| source)
                .unwrap_or(terminal);
            (function.clone(), canonical.clone())
        })
        .collect::<BTreeMap<_, _>>();
    if plan.mode == ElectricalMode::Direct {
        pins.extend(
            plan.assignments
                .iter()
                .filter(|assignment| !assignment.column_pin.is_empty())
                .map(|assignment| (assignment.key_id.clone(), assignment.column_pin.clone())),
        );
    } else {
        pins.extend(
            plan.row_pins
                .iter()
                .enumerate()
                .filter(|(_, pin)| !pin.is_empty())
                .map(|(i, pin)| (format!("row/{i}"), pin.clone()))
                .chain(
                    plan.column_pins
                        .iter()
                        .enumerate()
                        .filter(|(_, pin)| !pin.is_empty())
                        .map(|(i, pin)| (format!("column/{i}"), pin.clone())),
                ),
        );
    }
    pins
}

// A downloaded handoff is outside edit history. Undo cannot erase knowledge of it.
pub(crate) fn preserve_handoff(previous: &ProjectDoc, next: &mut ProjectDoc) {
    let Some(old) = &previous.hardware else {
        return;
    };
    for config in &old.boards {
        let Some(baseline) = &config.protected_handoff else {
            continue;
        };
        let hardware = next.hardware.get_or_insert_with(Default::default);
        if let Some(current) = hardware
            .boards
            .iter_mut()
            .find(|entry| entry.board_id == config.board_id)
        {
            current.protected_handoff = Some(baseline.clone());
        } else {
            hardware.boards.push(ElectricalBoardConfiguration {
                board_id: config.board_id.clone(),
                protected_handoff: Some(baseline.clone()),
                ..Default::default()
            });
        }
    }
}
