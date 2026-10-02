//! Private translation from an accepted electrical plan to the existing firmware worker request.
//!
//! This is the Rust counterpart of `app/src/firmwareHandoff.ts` and
//! `app/src/firmwarePeripherals.ts`. It only maps accepted electrical facts; it does not
//! assemble plans or generate DTS itself.
use boardstudio_core::{
    electrical::{ElectricalMode, ElectricalPlan},
    electrical_peripherals::PeripheralRequirement,
    firmware::{
        FirmwareEncoder, FirmwareHardware, FirmwareKey, FirmwareModuleFinding,
        FirmwareModuleQualification, FirmwarePartQualification, FirmwarePhysicalInstance,
        FirmwareQualification, FirmwareRequest, FirmwareScanMode, ScanPin, SplitTransport,
    },
    model::{HardwareTransport, ProjectDoc},
};

pub(crate) fn firmware_request(
    document: &ProjectDoc,
    plan: &ElectricalPlan,
    peripheral_plan: Option<&ElectricalPlan>,
) -> Result<(FirmwareRequest, Vec<String>), String> {
    validate_plan(plan)?;
    let transport = document
        .hardware
        .as_ref()
        .map(|hardware| hardware.transport)
        .unwrap_or_default();
    if transport != HardwareTransport::None && peripheral_plan.is_none() {
        return Err(
            "A split firmware handoff requires a distinct peripheral electrical plan".into(),
        );
    }
    build_request(document, plan, peripheral_plan, transport)
}

fn build_request(
    document: &ProjectDoc,
    plan: &ElectricalPlan,
    peripheral_plan: Option<&ElectricalPlan>,
    transport: HardwareTransport,
) -> Result<(FirmwareRequest, Vec<String>), String> {
    validate_plan(plan)?;
    let controller_profile = plan
        .controller_profile
        .clone()
        .ok_or("No reviewed controller profile is selected")?;
    let mode = if plan.mode == ElectricalMode::Direct {
        FirmwareScanMode::Direct
    } else {
        FirmwareScanMode::Matrix
    };
    let rows = if mode == FirmwareScanMode::Matrix {
        plan.row_pins
            .iter()
            .enumerate()
            .filter_map(|(index, terminal)| {
                let gpio = plan
                    .assignments
                    .iter()
                    .find(|a| a.row as usize == index)?
                    .row_firmware_gpio
                    .as_ref()?;
                Some(pin(
                    plan.module_aliases.get(terminal).unwrap_or(terminal),
                    gpio,
                ))
            })
            .collect()
    } else {
        vec![]
    };
    let columns = if mode == FirmwareScanMode::Matrix {
        plan.column_pins
            .iter()
            .enumerate()
            .filter_map(|(index, terminal)| {
                let gpio = plan
                    .assignments
                    .iter()
                    .find(|a| a.column as usize == index)?
                    .column_firmware_gpio
                    .as_ref()?;
                Some(pin(
                    plan.module_aliases.get(terminal).unwrap_or(terminal),
                    gpio,
                ))
            })
            .collect()
    } else {
        vec![]
    };
    let assignments = &plan.assignments;
    let mut keys = assignments
        .iter()
        .map(|assignment| FirmwareKey {
            id: assignment.key_id.clone(),
            row: assignment.row as usize,
            column: assignment.column as usize,
        })
        .collect::<Vec<_>>();
    let direct_pins = assignments
        .iter()
        .filter_map(|assignment| {
            assignment.direct_gpio.as_ref().map(|gpio| {
                pin(
                    plan.module_aliases
                        .get(&assignment.column_pin)
                        .unwrap_or(&assignment.column_pin),
                    gpio,
                )
            })
        })
        .collect::<Vec<_>>();
    let auxiliary = plan
        .peripherals
        .iter()
        .flat_map(|peripheral| {
            peripheral
                .gpio_terminals
                .iter()
                .filter(move |(terminal, function)| {
                    matches!(peripheral.kind.as_str(), "encoder" | "press")
                        && (function.ends_with("/encoder-push")
                            || function.ends_with("/input-push")
                            || terminal == "S1")
                })
                .map(move |(_, function)| {
                    (
                        peripheral
                            .press_key_id
                            .clone()
                            .unwrap_or_else(|| format!("{}/push", peripheral.part_id)),
                        function,
                    )
                })
        })
        .collect::<Vec<_>>();
    let auxiliary_pins = auxiliary
        .iter()
        .map(|(_, function)| {
            pin(
                plan.peripheral_terminals
                    .get(*function)
                    .map(String::as_str)
                    .unwrap_or_default(),
                plan.peripheral_pins
                    .get(*function)
                    .map(String::as_str)
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    for (index, (id, _)) in auxiliary.iter().enumerate() {
        keys.push(FirmwareKey {
            id: id.clone(),
            row: if mode == FirmwareScanMode::Direct {
                1
            } else {
                rows.len()
            },
            column: index,
        });
    }
    let configured_bindings = document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|board| Some(board.board_id.as_str()) == plan.board_id.as_deref())
        })
        .map(|board| &board.key_bindings);
    let all_peripherals = plan
        .peripherals
        .iter()
        .chain(
            peripheral_plan
                .into_iter()
                .flat_map(|p| p.peripherals.iter()),
        )
        .collect::<Vec<_>>();
    let sensor_ids = all_peripherals
        .iter()
        .filter(|item| item.kind == "encoder")
        .map(|item| item.part_id.clone())
        .collect::<Vec<_>>();
    let profiled = all_peripherals
        .iter()
        .filter(|item| item.kind == "encoder")
        .all(|item| item.rotary.is_some());
    let peripherals = peripheral_firmware(
        &legacy_plan(plan, profiled),
        if profiled { vec![] } else { sensor_ids.clone() },
    )?;
    let physical_instance_id = plan
        .instance_id
        .clone()
        .or_else(|| document.physical_instance_id.clone());
    let board_modules = document
        .modules
        .iter()
        .filter(|module| {
            Some(module.host_board_id.as_str()) == plan.board_id.as_deref() && !module.detached
        })
        .collect::<Vec<_>>();
    if physical_instance_id.is_none()
        && board_modules
            .iter()
            .any(|module| module.host_instance_id.is_some())
    {
        return Err("Select a physical host instance before exporting modules attached to a specific keyboard half".into());
    }
    let module_qualifications = board_modules
        .iter()
        .map(|module| {
            let definition = document
                .module_definitions
                .iter()
                .find(|definition| definition.id == module.definition_id)
                .ok_or_else(|| format!("Module snapshot is missing for {}", module.id))?;
            Ok(FirmwareModuleQualification {
                id: module.id.clone(),
                name: definition.name.clone(),
                host_instance_id: module.host_instance_id.clone(),
                protocol: definition.electrical.protocol,
                catalogue_row: definition.catalogue_row.clone(),
                source: Some(definition.source.clone()),
                rotary_profile: definition.electrical.rotary_profile.clone(),
                gates: definition.gates.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let module_encoders = module_encoders(
        document,
        plan,
        &board_modules,
        physical_instance_id.as_deref(),
    );
    let encoders = if profiled {
        plan.peripherals
            .iter()
            .filter_map(|item| {
                if item.kind != "encoder" {
                    return None;
                }
                let profile = item.rotary.clone()?;
                let gpio = |terminal: &str| {
                    item.gpio_terminals
                        .iter()
                        .find(|(name, _)| name == terminal)
                        .and_then(|(_, function)| find_peripheral_pin(plan, function))
                        .cloned()
                        .unwrap_or_default()
                };
                Some(FirmwareEncoder {
                    id: item.part_id.clone(),
                    a_gpio: gpio(&profile.a),
                    b_gpio: gpio(&profile.b),
                    profile,
                })
            })
            .chain(module_encoders.clone())
            .collect()
    } else {
        vec![]
    };
    let qualification = plan
        .board_id
        .as_ref()
        .map(|board_id| FirmwareQualification {
            board_id: board_id.clone(),
            parts: document
                .parts
                .iter()
                .filter(|part| {
                    document
                        .boards
                        .iter()
                        .find(|board| &board.id == board_id)
                        .is_some_and(|board| board.part_ids.contains(&part.id))
                })
                .filter_map(|part| {
                    let definition = document
                        .definitions
                        .iter()
                        .find(|definition| definition.id == part.definition_id)?;
                    let profile = definition.hardware_profile.as_ref()?;
                    Some(FirmwarePartQualification {
                        part_id: part.id.clone(),
                        name: definition.name.clone(),
                        source: profile.source.clone(),
                        gates: profile.gates.clone(),
                    })
                })
                .collect(),
        });
    let hardware = plan.board_id.as_ref().map(|board_id| FirmwareHardware {
        board_id: board_id.clone(),
        physical_instance_id: physical_instance_id.clone(),
        modules: module_qualifications,
        physical_instances: document
            .hardware
            .as_ref()
            .map(|hardware| {
                hardware
                    .instances
                    .iter()
                    .map(|instance| FirmwarePhysicalInstance {
                        id: instance.id.clone(),
                        board_id: instance.board_id.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        module_findings: plan
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.severity == "error" && diagnostic.code.starts_with("module/")
            })
            .map(|diagnostic| FirmwareModuleFinding {
                id: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
            })
            .collect(),
        embedded_circuit_ids: document
            .embedded_circuits
            .iter()
            .filter(|circuit| &circuit.host_board_id == board_id)
            .map(|circuit| circuit.id.clone())
            .collect(),
    });
    let transport_value = split_transport(transport);
    let uart_tx = if transport == HardwareTransport::Wired {
        uart_pin(plan, "split-tx")
    } else {
        None
    };
    let uart_rx = if transport == HardwareTransport::Wired {
        uart_pin(plan, "split-rx")
    } else {
        None
    };
    let peripheral_request = peripheral_plan
        .map(|other| {
            let mut unibody = document.clone();
            let mut hardware = unibody.hardware.clone().unwrap_or_default();
            hardware.topology = Default::default();
            hardware.transport = HardwareTransport::None;
            unibody.hardware = Some(hardware);
            build_request(&unibody, other, None, HardwareTransport::None).and_then(
                |(mut request, _)| {
                    let legacy = peripheral_firmware(
                        &legacy_plan(other, profiled),
                        if profiled { vec![] } else { sensor_ids.clone() },
                    )?;
                    request.peripheral_overlays = legacy.overlays;
                    request.transport = transport_value.clone();
                    request.matrix_row_offset = plan.row_pins.len();
                    request.uart_tx = if transport == HardwareTransport::Wired {
                        uart_pin(other, "split-rx")
                    } else {
                        None
                    };
                    request.uart_rx = if transport == HardwareTransport::Wired {
                        uart_pin(other, "split-tx")
                    } else {
                        None
                    };
                    Ok(Box::new(request))
                },
            )
        })
        .transpose()?;
    let mut config = peripherals.config;
    if !auxiliary_pins.is_empty() {
        config.push("CONFIG_ZMK_KSCAN_COMPOSITE_DRIVER=y".into());
    }
    let request = FirmwareRequest {
        qualification,
        hardware,
        encoders,
        keymap: document.keymap.clone(),
        encoder_ids: plan
            .peripherals
            .iter()
            .filter(|item| item.kind == "encoder")
            .map(|item| item.part_id.clone())
            .chain(module_encoders.iter().map(|encoder| encoder.id.clone()))
            .collect(),
        controller_profile,
        board_name: plan
            .board_id
            .clone()
            .unwrap_or_else(|| document.name.clone()),
        rows,
        columns,
        keys,
        diode_direction: plan.diode_direction.clone(),
        mode,
        direct_pins,
        auxiliary_pins,
        key_bindings: keys_for_bindings(assignments, &auxiliary, configured_bindings),
        peripheral_config: config,
        transport: transport_value,
        uart_tx,
        uart_rx,
        peripheral_overlays: peripherals.overlays,
        peripheral: peripheral_request,
        matrix_row_offset: 0,
    };
    let warnings = plan
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity != "error")
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    Ok((request, warnings))
}

fn validate_plan(plan: &ElectricalPlan) -> Result<(), String> {
    let mut failures = plan
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == "error" && !diagnostic.code.starts_with("module/")
        })
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();
    failures.extend(peripheral_warnings(plan));
    if plan.controller_profile.is_none() {
        failures.push("No reviewed controller profile is selected".into());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn legacy_plan(plan: &ElectricalPlan, profiled: bool) -> ElectricalPlan {
    if !profiled {
        return plan.clone();
    }
    let mut legacy = plan.clone();
    legacy.peripherals.retain(|item| item.kind != "encoder");
    legacy
}

fn keys_for_bindings(
    assignments: &[boardstudio_core::electrical::ElectricalAssignment],
    auxiliary: &[(String, &String)],
    bindings: Option<&std::collections::BTreeMap<String, String>>,
) -> Vec<String> {
    assignments
        .iter()
        .map(|item| {
            bindings
                .and_then(|map| map.get(&item.key_id))
                .cloned()
                .unwrap_or_else(|| "&none".into())
        })
        .chain(auxiliary.iter().map(|(id, _)| {
            bindings
                .and_then(|map| map.get(id))
                .cloned()
                .unwrap_or_else(|| "&none".into())
        }))
        .collect()
}
fn module_encoders(
    document: &ProjectDoc,
    plan: &ElectricalPlan,
    modules: &[&boardstudio_core::model::MountedModule],
    physical_instance_id: Option<&str>,
) -> Vec<FirmwareEncoder> {
    modules
        .iter()
        .filter_map(|module| {
            if module
                .host_instance_id
                .as_deref()
                .zip(physical_instance_id)
                .is_some_and(|(wanted, actual)| wanted != actual)
            {
                return None;
            }
            let definition = document
                .module_definitions
                .iter()
                .find(|definition| definition.id == module.definition_id)?;
            if definition.catalogue_row.as_deref() != Some("ec11-evqwgd001") {
                return None;
            }
            let profile = definition.electrical.rotary_profile.clone()?;
            let prefix = format!(
                "vik/{}/{}/",
                plan.board_id.as_deref().unwrap_or(""),
                module
                    .connection
                    .as_ref()
                    .map(|connection| connection.host_connector_part_id.as_str())
                    .unwrap_or("")
            );
            let a_gpio = find_peripheral_pin(plan, &format!("{prefix}{}", profile.a))?;
            let b_gpio = find_peripheral_pin(plan, &format!("{prefix}{}", profile.b))?;
            Some(FirmwareEncoder {
                id: module.id.clone(),
                profile,
                a_gpio: a_gpio.clone(),
                b_gpio: b_gpio.clone(),
            })
        })
        .collect()
}
fn pin(terminal: &str, gpio: &str) -> ScanPin {
    ScanPin {
        terminal: terminal.into(),
        gpio: gpio.into(),
    }
}
fn split_transport(transport: HardwareTransport) -> Option<SplitTransport> {
    match transport {
        HardwareTransport::Wired => Some(SplitTransport::WiredUart),
        HardwareTransport::Wireless => Some(SplitTransport::Wireless),
        HardwareTransport::None => None,
    }
}
fn find_peripheral_pin<'a>(plan: &'a ElectricalPlan, name: &str) -> Option<&'a String> {
    plan.peripheral_pins.get(name).or_else(|| {
        plan.peripheral_pins
            .iter()
            .find(|(key, _)| key.ends_with(&format!("/{name}")))
            .map(|(_, gpio)| gpio)
    })
}
fn uart_pin(plan: &ElectricalPlan, function: &str) -> Option<ScanPin> {
    let gpio = find_peripheral_pin(plan, function)?;
    let terminal = plan.peripheral_terminals.get(function).or_else(|| {
        plan.peripheral_terminals
            .iter()
            .find(|(key, _)| key.ends_with(&format!("/{function}")))
            .map(|(_, terminal)| terminal)
    })?;
    Some(pin(terminal, gpio))
}
fn peripheral_warnings(plan: &ElectricalPlan) -> Vec<String> {
    plan.peripherals
        .iter()
        .flat_map(|item| {
            let unresolved = item
                .gpio_terminals
                .iter()
                .any(|(_, function)| find_peripheral_pin(plan, function).is_none());
            let unsupported = ![
                "split",
                "power-switch",
                "reset",
                "battery",
                "display-i2c",
                "display-spi",
                "encoder",
                "press",
                "rgb",
                "vik",
            ]
            .contains(&item.kind.as_str());
            if unresolved {
                vec![format!(
                    "No resolved GPIO for {} {}",
                    item.kind, item.part_id
                )]
            } else if unsupported {
                vec![format!(
                    "Firmware profile for {} is not implemented; export is blocked",
                    item.kind
                )]
            } else {
                vec![]
            }
        })
        .collect()
}

#[derive(Default)]
struct PeripheralFirmware {
    overlays: Vec<String>,
    config: Vec<String>,
}
fn peripheral_firmware(
    plan: &ElectricalPlan,
    sensor_ids: Vec<String>,
) -> Result<PeripheralFirmware, String> {
    let displays = plan
        .peripherals
        .iter()
        .filter(|item| matches!(item.kind.as_str(), "display-i2c" | "display-spi"))
        .collect::<Vec<_>>();
    if displays.len() > 1 {
        let buses = displays
            .iter()
            .map(|item| item.kind.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        return Err(if buses.len() > 1 {
            "I2C0 and SPI1 displays require separate hardware peripherals"
        } else {
            "Only one firmware display is supported per controller"
        }
        .into());
    }
    let mut result = PeripheralFirmware::default();
    for item in &plan.peripherals {
        match item.kind.as_str() {
            "display-i2c" => {
                result.overlays.push(i2c_overlay(plan, item)?);
                result.config.extend(
                    [
                        "CONFIG_ZMK_DISPLAY=y",
                        "CONFIG_I2C=y",
                        "CONFIG_SSD1306=y",
                        "CONFIG_LVGL=y",
                    ]
                    .map(str::to_owned),
                );
            }
            "display-spi" => {
                result.overlays.push(spi_overlay(plan, item)?);
                result.config.extend(
                    [
                        "CONFIG_ZMK_DISPLAY=y",
                        "CONFIG_SPI=y",
                        "CONFIG_LS0XX=y",
                        "CONFIG_LVGL=y",
                    ]
                    .map(str::to_owned),
                );
            }
            "encoder" => {
                result.overlays.push(encoder_overlay(plan, item)?);
                result.config.extend(
                    ["CONFIG_EC11=y", "CONFIG_EC11_TRIGGER_GLOBAL_THREAD=y"].map(str::to_owned),
                );
            }
            "rgb" => {
                if plan
                    .peripherals
                    .iter()
                    .position(|candidate| candidate.kind == "rgb")
                    == plan
                        .peripherals
                        .iter()
                        .position(|candidate| std::ptr::eq(candidate, item))
                {
                    result.overlays.push(rgb_overlay(plan, item)?);
                }
                result.config.extend(
                    [
                        "CONFIG_ZMK_RGB_UNDERGLOW=y",
                        "CONFIG_SPI=y",
                        "CONFIG_WS2812_STRIP=y",
                    ]
                    .map(str::to_owned),
                );
            }
            "split" | "power-switch" | "reset" | "battery" | "press" | "vik" => {}
            unsupported => {
                return Err(format!(
                    "No source-verified firmware profile for {unsupported}"
                ));
            }
        }
    }
    for id in sensor_ids.iter().filter(|id| {
        !plan
            .peripherals
            .iter()
            .any(|item| item.kind == "encoder" && &item.part_id == *id)
    }) {
        result.overlays.push(format!(
            "/ {{ {}: {} {{ compatible = \"alps,ec11\"; status = \"disabled\"; }}; }};",
            encoder_label(id),
            encoder_label(id)
        ));
    }
    if !sensor_ids.is_empty() {
        result.overlays.push(format!("/ {{ sensors {{ compatible = \"zmk,keymap-sensors\"; sensors = <{}>; triggers-per-rotation = <20>; }}; }};", sensor_ids.iter().map(|id| format!("&{}", encoder_label(id))).collect::<Vec<_>>().join(" ")));
    }
    let mut seen = std::collections::BTreeSet::new();
    result.config.retain(|value| seen.insert(value.clone()));
    Ok(result)
}
fn peripheral_pin(plan: &ElectricalPlan, name: &str) -> Result<String, String> {
    pin_for(plan, name)
        .cloned()
        .ok_or_else(|| format!("No resolved GPIO for peripheral function {name}"))
}

fn pin_for<'a>(plan: &'a ElectricalPlan, name: &str) -> Option<&'a String> {
    let bare = name.split_once('/').map(|(_, rest)| rest).unwrap_or(name);
    plan.peripheral_pins
        .get(name)
        .or_else(|| plan.peripheral_pins.get(bare))
        .or_else(|| {
            plan.peripheral_pins
                .iter()
                .find(|(key, _)| key.ends_with(&format!("/{bare}")))
                .map(|(_, gpio)| gpio)
        })
}

fn role_pin(
    plan: &ElectricalPlan,
    peripheral: &PeripheralRequirement,
    role: &str,
    fallback: &str,
) -> Result<String, String> {
    let function = peripheral
        .gpio_terminals
        .iter()
        .find(|(terminal, _)| terminal.eq_ignore_ascii_case(role))
        .map(|(_, name)| name.as_str())
        .unwrap_or(fallback);
    let resolved_name = if plan.peripheral_pins.contains_key(function) {
        function.to_owned()
    } else {
        format!("{}/{}", peripheral.part_id, function)
    };
    peripheral_pin(plan, &resolved_name)
}
fn parse_gpio(gpio: &str) -> Result<(String, String), String> {
    let parse_number = |number: &str| -> Option<u8> {
        if number.is_empty()
            || number.len() > 2
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return None;
        }
        number.parse().ok()
    };
    let (port, number) = if let Some(rest) = gpio.strip_prefix('P') {
        let (port, number) = rest
            .split_once('.')
            .ok_or_else(|| format!("Invalid allocated GPIO {gpio}"))?;
        (port, number)
    } else if let Some(rest) = gpio.strip_prefix("&gpio") {
        let (port, number) = rest
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("Invalid allocated GPIO {gpio}"))?;
        (port, number.trim_start())
    } else {
        return Err(format!("Invalid allocated GPIO {gpio}"));
    };
    if !matches!(port, "0" | "1") {
        return Err(format!("Invalid allocated GPIO {gpio}"));
    }
    let pin_num = parse_number(number).ok_or_else(|| format!("Invalid allocated GPIO {gpio}"))?;
    if pin_num > 31 {
        return Err(format!("GPIO pin out of range {gpio}"));
    }
    Ok((port.to_owned(), pin_num.to_string()))
}
fn gpio_spec(gpio: &str) -> Result<String, String> {
    let (port, number) = parse_gpio(gpio)?;
    Ok(format!("&gpio{port} {number} GPIO_ACTIVE_HIGH"))
}
fn encoder_label(id: &str) -> String {
    format!(
        "encoder_{}",
        id.as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
fn i2c_overlay(
    plan: &ElectricalPlan,
    peripheral: &PeripheralRequirement,
) -> Result<String, String> {
    let sda = role_pin(plan, peripheral, "SDA", "i2c/SDA")?;
    let scl = role_pin(plan, peripheral, "SCL", "i2c/SCL")?;
    let (sda_port, sda_pin) = parse_gpio(&sda)?;
    let (scl_port, scl_pin) = parse_gpio(&scl)?;
    Ok(format!(
        "&i2c0 {{ status = \"okay\"; pinctrl-0 = <&boardstudio_i2c>; pinctrl-names = \"default\"; oled: ssd1306@3c {{ compatible = \"solomon,ssd1306fb\"; reg = <0x3c>; width = <128>; height = <32>; segment-offset = <0>; page-offset = <0>; display-offset = <0>; multiplex-ratio = <31>; segment-remap; com-invdir; com-sequential; inversion-on; prechargep = <0x22>; }}; }}; &pinctrl {{ boardstudio_i2c: boardstudio_i2c {{ group1 {{ psels = <NRF_PSEL(TWIM_SDA, {sda_port}, {sda_pin})>, <NRF_PSEL(TWIM_SCL, {scl_port}, {scl_pin})>; }}; }}; }}; / {{ chosen {{ zephyr,display = &oled; }}; }};"
    ))
}
fn spi_overlay(
    plan: &ElectricalPlan,
    peripheral: &PeripheralRequirement,
) -> Result<String, String> {
    let mosi = role_pin(plan, peripheral, "MOSI", "spi/MOSI")?;
    let sck = role_pin(plan, peripheral, "SCK", "spi/SCK")?;
    let cs = gpio_spec(&role_pin(plan, peripheral, "CS", "CS")?)?;
    let (mosi_port, mosi_pin) = parse_gpio(&mosi)?;
    let (sck_port, sck_pin) = parse_gpio(&sck)?;
    Ok(format!(
        "&spi1 {{ status = \"okay\"; pinctrl-0 = <&boardstudio_spi>; pinctrl-names = \"default\"; cs-gpios = <{cs}>; nice_view: ls0xx@0 {{ compatible = \"sharp,ls0xx\"; spi-max-frequency = <1000000>; reg = <0>; width = <160>; height = <68>; }}; }}; &pinctrl {{ boardstudio_spi: boardstudio_spi {{ group1 {{ psels = <NRF_PSEL(SPIM_MOSI, {mosi_port}, {mosi_pin})>, <NRF_PSEL(SPIM_SCK, {sck_port}, {sck_pin})>; }}; }}; }}; / {{ chosen {{ zephyr,display = &nice_view; }}; }};"
    ))
}
fn encoder_overlay(
    plan: &ElectricalPlan,
    peripheral: &PeripheralRequirement,
) -> Result<String, String> {
    let mut a = gpio_spec(&role_pin(plan, peripheral, "A", "encoder/A")?)?;
    let mut b = gpio_spec(&role_pin(plan, peripheral, "C", "encoder/C")?)?;
    a = a.replace("GPIO_ACTIVE_HIGH", "(GPIO_ACTIVE_HIGH | GPIO_PULL_UP)");
    b = b.replace("GPIO_ACTIVE_HIGH", "(GPIO_ACTIVE_HIGH | GPIO_PULL_UP)");
    let name = encoder_label(&peripheral.part_id);
    Ok(format!(
        "/ {{ {name}: {name} {{ compatible = \"alps,ec11\"; status = \"okay\"; a-gpios = <{a}>; b-gpios = <{b}>; steps = <80>; }}; }};"
    ))
}
fn rgb_overlay(
    plan: &ElectricalPlan,
    peripheral: &PeripheralRequirement,
) -> Result<String, String> {
    let din = role_pin(plan, peripheral, "P4", "rgb-in")?;
    let (port, pin_number) = parse_gpio(&din)?;
    let count = plan
        .peripherals
        .iter()
        .filter(|item| item.kind == "rgb")
        .count();
    let _ = peripheral;
    Ok(format!(
        "#include <zephyr/dt-bindings/led/led.h>\n&spi3 {{ status = \"okay\"; pinctrl-0 = <&boardstudio_spi3>; pinctrl-names = \"default\"; led_strip: ws2812@0 {{ compatible = \"worldsemi,ws2812-spi\"; reg = <0>; spi-max-frequency = <4000000>; chain-length = <{count}>; spi-one-frame = <0x70>; spi-zero-frame = <0x40>; color-mapping = <LED_COLOR_ID_GREEN LED_COLOR_ID_RED LED_COLOR_ID_BLUE>; }}; }}; &pinctrl {{ boardstudio_spi3: boardstudio_spi3 {{ group1 {{ psels = <NRF_PSEL(SPIM_MOSI, {port}, {pin_number})>; }}; }}; }}; / {{ chosen {{ zmk,underglow = &led_strip; }}; }};"
    ))
}
