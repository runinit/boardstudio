use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn role(signal: VikSignal) -> String {
    serde_json::to_value(signal)
        .unwrap()
        .as_str()
        .unwrap()
        .into()
}
fn finding(
    id: &str,
    code: &str,
    message: String,
    targets: Vec<String>,
    severity: Severity,
) -> Finding {
    Finding {
        id: format!("module/{id}/{code}"),
        scope: Scope::Pcb,
        severity,
        message,
        target_ids: targets,
    }
}

fn rail_matches(signal: VikSignal, name: &str) -> bool {
    let normalized = name.trim().trim_start_matches('+').to_ascii_uppercase();
    match signal {
        VikSignal::Gnd => matches!(normalized.as_str(), "GND" | "GROUND" | "VSS"),
        VikSignal::V3v3 => matches!(normalized.as_str(), "3V3" | "3.3V" | "VDD_3V3"),
        VikSignal::V5 => matches!(normalized.as_str(), "5V" | "V5" | "VBUS"),
        _ => false,
    }
}

pub(crate) fn connection_findings(doc: &ProjectDoc, board_id: &str) -> Vec<Finding> {
    let instances: Vec<_> = super::active_instances(doc, board_id).collect();
    if instances.is_empty()
        && !doc
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.host_board_id == board_id)
    {
        return Vec::new();
    }
    let controller_id = doc
        .hardware
        .as_ref()
        .and_then(|h| h.boards.iter().find(|b| b.board_id == board_id))
        .and_then(|b| b.controller_part_id.as_deref());
    let board = doc.boards.iter().find(|b| b.id == board_id);
    let controller = controller_id
        .and_then(|id| doc.parts.iter().find(|p| p.id == id))
        .or_else(|| {
            let mut p = doc.parts.iter().filter(|p| {
                board.is_some_and(|b| b.part_ids.contains(&p.id))
                    && doc
                        .definitions
                        .iter()
                        .any(|d| d.id == p.definition_id && d.kind == PartKind::Controller)
            });
            let first = p.next();
            if p.next().is_none() { first } else { None }
        });
    let controller_def =
        controller.and_then(|p| doc.definitions.iter().find(|d| d.id == p.definition_id));
    let profile = controller_def
        .and_then(|d| d.generator.as_ref())
        .and_then(|g| crate::electrical_profiles::profile(&g.source));
    let wire = |terminal: &String| {
        if let (Some(controller), Some(definition)) = (controller, controller_def) {
            if let Some(pads) = definition.terminals.get(terminal) {
                if let Some(net) = doc.nets.iter().find(|net| {
                    board.is_some_and(|board| board.net_ids.contains(&net.id))
                        && net
                            .pins
                            .iter()
                            .any(|pin| pin.part_id == controller.id && pads.contains(&pin.pad_id))
                }) {
                    return net.id.clone();
                }
                if let Some(pad) = pads.first() {
                    return format!("controller/{}/{pad}", controller.id);
                }
            }
        }
        format!("terminal/{terminal}")
    };
    let mut findings = Vec::new();
    let mut private_findings = Vec::new();
    let mut addresses: BTreeMap<(String, String, u8), String> = BTreeMap::new();
    let mut private: BTreeMap<String, (String, VikSignal, ModuleProtocol)> = BTreeMap::new();
    let mut currents: BTreeMap<String, (f64, Option<f64>, Vec<String>)> = BTreeMap::new();
    let mut pullups: BTreeMap<(String, String), Vec<(String, f64)>> = BTreeMap::new();
    let mut connector_assignments: BTreeMap<(String, VikSignal), (String, String)> =
        BTreeMap::new();
    for instance in instances {
        let Some(def) = doc
            .module_definitions
            .iter()
            .find(|d| d.id == instance.definition_id)
        else {
            continue;
        };
        let Some(connection) = &instance.connection else {
            continue;
        };
        let targets = vec![instance.id.clone(), board_id.into()];
        for (signal, terminal) in &connection.assignments {
            if terminal.is_empty() {
                continue;
            }
            let key = (connection.host_connector_part_id.clone(), *signal);
            if let Some((previous, owner)) = connector_assignments.get(&key) {
                if previous != terminal {
                    let mut targets = vec![
                        owner.clone(),
                        instance.id.clone(),
                        connection.host_connector_part_id.clone(),
                        board_id.into(),
                    ];
                    targets.sort();
                    targets.dedup();
                    findings.push(finding(
                        &instance.id,
                        &format!("host-contact-assignment/{}", role(*signal)),
                        format!(
                            "Modules {owner} and {} assign the same shared host connector's {} contact to different MCU terminals ({previous} and {terminal}).",
                            instance.id,
                            role(*signal)
                        ),
                        targets,
                        Severity::Error,
                    ));
                }
            } else {
                connector_assignments.insert(key, (terminal.clone(), instance.id.clone()));
            }
        }
        let mut report = |code: &str, message: String, severity: Severity| {
            findings.push(finding(
                &instance.id,
                code,
                message,
                targets.clone(),
                severity,
            ))
        };
        for gate in def.gates.iter().filter(|gate| {
            matches!(
                gate.output,
                HardwareOutput::Footprint | HardwareOutput::Electrical
            )
        }) {
            report(
                &format!("gate/{}", gate.code),
                gate.message.clone(),
                Severity::Error,
            );
        }
        let connector = doc.parts.iter().find(|p| {
            p.id == connection.host_connector_part_id
                && board.is_some_and(|b| b.part_ids.contains(&p.id))
        });
        let connector_def =
            connector.and_then(|p| doc.definitions.iter().find(|d| d.id == p.definition_id));
        if !connector_def.is_some_and(|d| {
            d.hardware_profile
                .as_ref()
                .is_some_and(|h| h.vik_role == Some(VikRole::Host))
        }) {
            report("host-role","Choose a VIK host connector on this PCB; a module-side connector has reversed numbering.".into(),Severity::Error);
        }
        if !def
            .interfaces
            .iter()
            .any(|p| p.id == connection.module_port_id && p.role == VikRole::Module)
        {
            report(
                "module-role",
                "Connect a module-side input port, preserving all twelve reversed contact roles."
                    .into(),
                Severity::Error,
            );
        }
        if connection.cable_type != "type-a-12-0.5" {
            report(
                "cable",
                "VIK requires a 12-contact, 0.5 mm pitch Type A FFC cable.".into(),
                Severity::Error,
            );
        }
        if let Some(upstream) = &connection.upstream_module_id {
            let parent = doc
                .modules
                .iter()
                .find(|m| m.id == *upstream && m.host_board_id == board_id && !m.detached);
            if upstream == &instance.id
                || !parent.is_some_and(|p| {
                    p.connection.as_ref().is_some_and(|c| {
                        c.host_connector_part_id == connection.host_connector_part_id
                            && c.assignments == connection.assignments
                    }) && doc
                        .module_definitions
                        .iter()
                        .find(|d| d.id == p.definition_id)
                        .is_some_and(|d| {
                            d.interfaces.iter().any(|port| {
                                Some(&port.id) == connection.upstream_port_id.as_ref()
                                    && port.role == VikRole::Host
                            })
                        })
                })
            {
                report("pass-through","A splitter output must share its parent's actual host contacts and select a host-side output port.".into(),Severity::Error);
            }
            let mut seen = BTreeSet::from([instance.id.as_str()]);
            let mut next = Some(upstream.as_str());
            while let Some(id) = next {
                if !seen.insert(id) {
                    report(
                        "connection-cycle",
                        "Module connections contain a cycle.".into(),
                        Severity::Error,
                    );
                    break;
                }
                next = doc
                    .modules
                    .iter()
                    .find(|m| m.id == id)
                    .and_then(|m| m.connection.as_ref())
                    .and_then(|c| c.upstream_module_id.as_deref());
            }
        }
        if def
            .electrical
            .logic_voltage
            .is_some_and(|v| (v - 3.3).abs() > 0.01)
        {
            report(
                "logic-voltage",
                "VIK signals require 3.3 V logic; a 5 V rail does not permit 5 V GPIO.".into(),
                Severity::Error,
            );
        }
        if def.electrical.protocol == ModuleProtocol::I2c {
            if let Some(resistance) = def.electrical.pullup_ohms {
                if !resistance.is_finite() || resistance <= 0.0 {
                    report(
                        "pullup-profile",
                        "I²C pull-up resistance must be a finite positive value.".into(),
                        Severity::Error,
                    );
                } else if !def.gates.iter().any(|gate| gate.code == "pullup-supply")
                    && connection
                        .rail_voltages
                        .get(&VikSignal::V3v3)
                        .is_some_and(|voltage| voltage.is_finite() && (*voltage - 3.3).abs() < 0.01)
                {
                    if let (Some(scl), Some(sda)) = (
                        connection.assignments.get(&VikSignal::Scl),
                        connection.assignments.get(&VikSignal::Sda),
                    ) {
                        pullups
                            .entry((wire(scl), wire(sda)))
                            .or_default()
                            .push((instance.id.clone(), resistance));
                    }
                }
            }
        }
        let mut used = BTreeSet::new();
        for signal in &def.electrical.required_signals {
            let Some(terminal) = connection.assignments.get(signal).filter(|s| !s.is_empty())
            else {
                report(
                    &format!("contact/{}", role(*signal)),
                    format!(
                        "Assign the {} contact to a real controller terminal or power rail.",
                        role(*signal)
                    ),
                    Severity::Error,
                );
                continue;
            };
            if matches!(signal, VikSignal::Gnd | VikSignal::V3v3 | VikSignal::V5) {
                if !controller_def
                    .is_some_and(|d| d.terminals.get(terminal).is_some_and(|p| !p.is_empty()))
                {
                    report(
                        &format!("rail/{}", role(*signal)),
                        format!(
                            "Power terminal {terminal} is not exposed by the selected controller."
                        ),
                        Severity::Error,
                    );
                }
                let expected = if *signal == VikSignal::V5 {
                    5.0
                } else if *signal == VikSignal::V3v3 {
                    3.3
                } else {
                    0.0
                };
                if *signal != VikSignal::Gnd
                    && !connection
                        .rail_voltages
                        .get(signal)
                        .is_some_and(|v| v.is_finite() && (*v - expected).abs() < 0.01)
                {
                    report(
                        &format!("rail-voltage/{}", role(*signal)),
                        format!(
                            "Confirm the {expected} V supply separately from the 3.3 V signal logic."
                        ),
                        Severity::Error,
                    );
                }
                continue;
            }
            let capability = profile
                .as_ref()
                .and_then(|p| p.pins.iter().find(|p| p.terminal == terminal));
            if !capability.is_some_and(|p| {
                controller_def.is_some_and(|d| d.terminals.contains_key(p.terminal))
                    && !profile
                        .as_ref()
                        .unwrap()
                        .reserved_gpios
                        .contains(&p.firmware_gpio)
            }) {
                report(
                    &format!("gpio/{}", role(*signal)),
                    format!("{terminal} is not an available reviewed controller GPIO."),
                    Severity::Error,
                );
            }
            if !used.insert(wire(terminal)) {
                report(
                    "contact-alias",
                    format!("Two different VIK signals cannot use controller contact {terminal}."),
                    Severity::Error,
                );
            }
            if def.electrical.protocol != ModuleProtocol::PassThrough
                && matches!(
                    signal,
                    VikSignal::Cs | VikSignal::Gpio1 | VikSignal::Gpio2 | VikSignal::Rgb
                )
            {
                if let Some((owner, _, _)) = private.insert(
                    wire(terminal),
                    (instance.id.clone(), *signal, def.electrical.protocol),
                ) {
                    private_findings.push(finding(
                        &instance.id,
                        "shared-private-contact",
                        format!(
                            "{terminal} is also consumed by {owner}; splitter outputs do not provide independent CS or GPIO."
                        ),
                        vec![owner, instance.id.clone(), board_id.into()],
                        Severity::Error,
                    ));
                }
            }
        }
        if let Some(address) = def.electrical.i2c_address {
            if let (Some(scl), Some(sda)) = (
                connection.assignments.get(&VikSignal::Scl),
                connection.assignments.get(&VikSignal::Sda),
            ) {
                if let Some(owner) =
                    addresses.insert((wire(scl), wire(sda), address), instance.id.clone())
                {
                    findings.push(finding(&instance.id,"i2c-address",format!("{owner} and {} share I2C address 0x{address:02X} on {scl}/{sda}. Use separate buses or explicit isolation.",instance.id),vec![owner,instance.id.clone(),board_id.into()],Severity::Error));
                }
            }
        }
        let current = currents
            .entry(connection.host_connector_part_id.clone())
            .or_insert((0.0, connection.supply_current_ma, vec![]));
        current.2.push(instance.id.clone());
        if let Some(ma) = def
            .electrical
            .current_ma
            .filter(|v| v.is_finite() && *v >= 0.0)
        {
            current.0 += ma;
        } else {
            findings.push(finding(
                &instance.id,
                "power-unverified",
                "Module current is unknown; the shared supply/cable budget is not qualified."
                    .into(),
                targets,
                Severity::Warning,
            ));
        }
    }
    for (_, (used, budget, targets)) in currents {
        if budget.is_some_and(|v| !v.is_finite() || v < used || v <= 0.0) {
            let owner = targets[0].clone();
            findings.push(finding(&owner,"current-budget",format!("Combined known module current is {used:.1} mA and exceeds the assigned supply budget."),targets,Severity::Error));
        }
    }
    // Embedded copies use actual joined net identities, rather than a bus label.
    for instance in doc
        .embedded_circuits
        .iter()
        .filter(|c| c.host_board_id == board_id)
    {
        let Some(def) = doc
            .module_definitions
            .iter()
            .find(|d| d.id == instance.definition_id)
        else {
            continue;
        };
        if def
            .electrical
            .logic_voltage
            .is_some_and(|voltage| !voltage.is_finite() || (voltage - 3.3).abs() > 0.01)
        {
            findings.push(finding(
                &instance.id,
                "logic-voltage",
                "VIK signals require 3.3 V logic; a 5 V rail does not permit 5 V GPIO.".into(),
                vec![instance.id.clone(), board_id.into()],
                Severity::Error,
            ));
        }
        let mut embedded_ports_confirmed = true;
        for signal in &def.electrical.required_signals {
            let name = role(*signal);
            let Some(net_id) = instance.ports.get(&name) else {
                findings.push(finding(
                    &instance.id,
                    &format!("port/{name}"),
                    format!("Required {name} signal is absent from the editable circuit."),
                    vec![instance.id.clone(), board_id.into()],
                    Severity::Error,
                ));
                embedded_ports_confirmed = false;
                continue;
            };
            let host_net = (!net_id.starts_with("embedded/"))
                .then(|| {
                    board
                        .filter(|board| board.net_ids.contains(net_id))
                        .and_then(|_| doc.nets.iter().find(|net| net.id == *net_id))
                })
                .flatten();
            if let Some(expected) = host_net {
                if matches!(signal, VikSignal::Gnd | VikSignal::V3v3 | VikSignal::V5)
                    && !rail_matches(*signal, &expected.name)
                {
                    findings.push(finding(
                        &instance.id,
                        &format!("rail/{}", name),
                        format!(
                            "Joined {name} port to host net {} instead of its matching power rail.",
                            expected.name
                        ),
                        vec![instance.id.clone(), board_id.into()],
                        Severity::Error,
                    ));
                    embedded_ports_confirmed = false;
                }
            } else {
                findings.push(finding(
                    &instance.id,
                    &format!("unjoined/{}", name),
                    format!("Join the circuit's {name} port to an existing host PCB net."),
                    vec![instance.id.clone(), board_id.into()],
                    Severity::Error,
                ));
                embedded_ports_confirmed = false;
            }
        }
        if def.electrical.protocol != ModuleProtocol::PassThrough {
            for signal in def.electrical.required_signals.iter().filter(|signal| {
                matches!(
                    signal,
                    VikSignal::Cs | VikSignal::Gpio1 | VikSignal::Gpio2 | VikSignal::Rgb
                )
            }) {
                let Some(net) = instance.ports.get(&role(*signal)) else {
                    continue;
                };
                if let Some((owner, _, _)) = private.insert(
                    net.clone(),
                    (instance.id.clone(), *signal, def.electrical.protocol),
                ) {
                    private_findings.push(finding(
                        &instance.id, "shared-private-contact",
                        format!("Wire {net} is also consumed by {owner}; embedded and mounted devices do not have independent CS or GPIO on a joined net."),
                        vec![owner, instance.id.clone(), board_id.into()], Severity::Error,
                    ));
                }
            }
        }
        if let (Some(address), Some(scl), Some(sda)) = (
            def.electrical.i2c_address,
            instance.ports.get("scl"),
            instance.ports.get("sda"),
        ) {
            if let Some(owner) =
                addresses.insert((scl.clone(), sda.clone(), address), instance.id.clone())
            {
                findings.push(finding(&instance.id,"i2c-address",format!("Devices {owner} and {} share fixed address 0x{address:02X}; use separate buses or isolation.",instance.id),vec![owner,instance.id.clone(),board_id.into()],Severity::Error));
            }
        }
        if def.electrical.protocol == ModuleProtocol::I2c {
            if let Some(resistance) = def.electrical.pullup_ohms {
                if !resistance.is_finite() || resistance <= 0.0 {
                    findings.push(finding(
                        &instance.id,
                        "pullup-profile",
                        "I²C pull-up resistance must be a finite positive value.".into(),
                        vec![instance.id.clone(), board_id.into()],
                        Severity::Error,
                    ));
                } else if !def.gates.iter().any(|gate| gate.code == "pullup-supply")
                    && embedded_ports_confirmed
                    && def.electrical.required_signals.contains(&VikSignal::V3v3)
                {
                    if let (Some(scl), Some(sda)) =
                        (instance.ports.get("scl"), instance.ports.get("sda"))
                    {
                        pullups
                            .entry((scl.clone(), sda.clone()))
                            .or_default()
                            .push((instance.id.clone(), resistance));
                    }
                }
            }
        }
    }
    for ((scl, sda), devices) in pullups {
        if devices.len() < 2 {
            continue;
        }
        let conductance = devices.iter().map(|(_, ohms)| 1.0 / ohms).sum::<f64>();
        let equivalent = 1.0 / conductance;
        if !equivalent.is_finite() || equivalent <= 0.0 {
            continue;
        }
        let owner = devices[0].0.clone();
        let mut targets: Vec<_> = devices.iter().map(|(id, _)| id.clone()).collect();
        targets.push(board_id.into());
        findings.push(finding(
            &owner,
            "i2c-pullups",
            format!(
                "{} enabled pull-up networks share SCL/SDA and produce approximately {:.0} Ω in parallel. Review the bus against the selected controller and devices.",
                devices.len(), equivalent
            ),
            targets,
            Severity::Warning,
        ));
        let _ = (scl, sda);
    }
    findings.extend(private_findings);
    findings
}

pub(crate) fn host_requirements(
    doc: &ProjectDoc,
    board_id: &str,
) -> Vec<crate::electrical_peripherals::PeripheralRequirement> {
    let mut connections: BTreeMap<String, (BTreeSet<VikSignal>, ModuleConnection)> =
        BTreeMap::new();
    for m in super::active_instances(doc, board_id) {
        if let (Some(c), Some(d)) = (
            &m.connection,
            doc.module_definitions
                .iter()
                .find(|d| d.id == m.definition_id),
        ) {
            let entry = connections
                .entry(c.host_connector_part_id.clone())
                .or_insert((BTreeSet::new(), c.clone()));
            entry
                .0
                .extend(d.electrical.required_signals.iter().copied());
        }
    }
    connections
        .into_iter()
        .filter_map(|(part_id, (signals, c))| {
            let part = doc.parts.iter().find(|p| p.id == part_id)?;
            let def = doc
                .definitions
                .iter()
                .find(|d| d.id == part.definition_id)?;
            if def.hardware_profile.as_ref()?.vik_role != Some(VikRole::Host) {
                return None;
            }
            let mapping = placement::vik_signals(VikRole::Host);
            let mut gpio = Vec::new();
            let mut fixed = Vec::new();
            for signal in signals {
                let number = (mapping.iter().position(|s| *s == signal)? + 1).to_string();
                if matches!(signal, VikSignal::Gnd | VikSignal::V3v3 | VikSignal::V5) {
                    if let Some(terminal) = c.assignments.get(&signal) {
                        fixed.push((number, terminal.clone()));
                    }
                } else {
                    gpio.push((
                        number,
                        format!(
                            "vik/{}/{}/{}",
                            board_id,
                            c.host_connector_part_id,
                            role(signal)
                        ),
                    ));
                }
            }
            Some(crate::electrical_peripherals::PeripheralRequirement {
                part_id,
                source: "vik-host".into(),
                kind: "vik".into(),
                gpio_terminals: gpio,
                fixed_terminals: fixed,
                rotary: None,
                press_key_id: None,
            })
        })
        .collect()
}

pub(crate) fn connection_locks(doc: &ProjectDoc, board_id: &str) -> BTreeMap<String, String> {
    super::active_instances(doc, board_id)
        .filter_map(|m| m.connection.as_ref())
        .flat_map(|c| {
            c.assignments
                .iter()
                .filter(|(signal, _)| {
                    !matches!(signal, VikSignal::Gnd | VikSignal::V3v3 | VikSignal::V5)
                })
                .map(move |(signal, pin)| {
                    (
                        format!(
                            "vik/{board_id}/{}/{}",
                            c.host_connector_part_id,
                            role(*signal)
                        ),
                        pin.clone(),
                    )
                })
        })
        .collect()
}
