#[path = "../src/firmware_request_adapter.rs"]
mod firmware_request_adapter;

use boardstudio_core::{
    electrical::{ElectricalAssignment, ElectricalDiagnostic, ElectricalMode, ElectricalPlan},
    electrical_peripherals::PeripheralRequirement,
    firmware::{FirmwareScanMode, ScanPin, SplitTransport},
    model::{
        Board, ElectricalBoardConfiguration, HardwareConfiguration, HardwareTransport, ProjectDoc,
    },
};
use std::collections::BTreeMap;

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Sofle");
    document.boards.push(Board {
        id: "left".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let mut board = ElectricalBoardConfiguration {
        board_id: "left".into(),
        ..ElectricalBoardConfiguration::default()
    };
    board.key_bindings.insert("left/SW1".into(), "&kp A".into());
    document.hardware = Some(HardwareConfiguration {
        boards: vec![board],
        ..HardwareConfiguration::default()
    });
    document
}

fn plan(mode: ElectricalMode) -> ElectricalPlan {
    ElectricalPlan {
        instance_id: Some("left-half".into()),
        jumpers: vec![],
        module_aliases: BTreeMap::new(),
        mode,
        assignments: vec![ElectricalAssignment {
            key_id: "left/SW1".into(),
            matrix_id: "left/matrix".into(),
            row: 0,
            column: 0,
            row_pin: "ROW0".into(),
            column_pin: "COL0".into(),
            locked: false,
            row_firmware_gpio: (mode == ElectricalMode::Matrix).then(|| "P0.31".into()),
            column_firmware_gpio: (mode == ElectricalMode::Matrix).then(|| "P0.29".into()),
            direct_gpio: (mode == ElectricalMode::Direct).then(|| "P0.02".into()),
        }],
        row_pins: if mode == ElectricalMode::Matrix {
            vec!["ROW0".into()]
        } else {
            vec![]
        },
        column_pins: if mode == ElectricalMode::Matrix {
            vec!["COL0".into()]
        } else {
            vec![]
        },
        diagnostics: vec![],
        fingerprint: "plan".into(),
        board_id: Some("left".into()),
        controller_part_id: Some("mcu".into()),
        revision: 9,
        controller_profile: Some("ceoloide/mcu_nice_nano".into()),
        free_pins: vec![],
        nets: vec![],
        diode_direction: "col2row".into(),
        peripherals: vec![],
        peripheral_pins: BTreeMap::new(),
        peripheral_terminals: BTreeMap::new(),
    }
}

#[test]
fn matrix_and_direct_plans_keep_scan_rows_keys_and_bindings() {
    let document = document();
    let (matrix, _) =
        firmware_request_adapter::firmware_request(&document, &plan(ElectricalMode::Matrix), None)
            .expect("matrix firmware request");
    assert_eq!(matrix.mode, FirmwareScanMode::Matrix);
    assert_eq!(
        matrix.rows,
        vec![ScanPin {
            terminal: "ROW0".into(),
            gpio: "P0.31".into()
        }]
    );
    assert_eq!(
        matrix.columns,
        vec![ScanPin {
            terminal: "COL0".into(),
            gpio: "P0.29".into()
        }]
    );
    assert_eq!(matrix.key_bindings, ["&kp A"]);
    assert_eq!(matrix.board_name, "left");

    let (direct, _) =
        firmware_request_adapter::firmware_request(&document, &plan(ElectricalMode::Direct), None)
            .expect("direct firmware request");
    assert_eq!(direct.mode, FirmwareScanMode::Direct);
    assert!(direct.rows.is_empty());
    assert!(direct.columns.is_empty());
    assert_eq!(
        direct.direct_pins,
        vec![ScanPin {
            terminal: "COL0".into(),
            gpio: "P0.02".into()
        }]
    );
}

#[test]
fn split_wired_request_preserves_peripheral_offset_and_reverses_uart() {
    let mut document = document();
    document.hardware.as_mut().unwrap().transport = HardwareTransport::Wired;
    let mut central = plan(ElectricalMode::Matrix);
    central
        .peripheral_pins
        .insert("split-tx".into(), "P0.06".into());
    central
        .peripheral_pins
        .insert("split-rx".into(), "P0.08".into());
    central
        .peripheral_terminals
        .insert("split-tx".into(), "TX".into());
    central
        .peripheral_terminals
        .insert("split-rx".into(), "RX".into());
    let mut peripheral = plan(ElectricalMode::Matrix);
    peripheral.board_id = Some("right".into());
    peripheral.instance_id = Some("right-half".into());
    peripheral
        .peripheral_pins
        .insert("split-tx".into(), "P0.20".into());
    peripheral
        .peripheral_pins
        .insert("split-rx".into(), "P0.22".into());
    peripheral
        .peripheral_terminals
        .insert("split-tx".into(), "TX".into());
    peripheral
        .peripheral_terminals
        .insert("split-rx".into(), "RX".into());

    let (request, _) =
        firmware_request_adapter::firmware_request(&document, &central, Some(&peripheral))
            .expect("wired split request");
    assert_eq!(request.transport, Some(SplitTransport::WiredUart));
    assert_eq!(
        request.uart_tx,
        Some(ScanPin {
            terminal: "TX".into(),
            gpio: "P0.06".into()
        })
    );
    assert_eq!(
        request.uart_rx,
        Some(ScanPin {
            terminal: "RX".into(),
            gpio: "P0.08".into()
        })
    );
    let peer = request.peripheral.expect("peripheral request");
    assert_eq!(peer.matrix_row_offset, 1);
    assert_eq!(peer.transport, Some(SplitTransport::WiredUart));
    assert_eq!(
        peer.uart_tx,
        Some(ScanPin {
            terminal: "RX".into(),
            gpio: "P0.22".into()
        })
    );
    assert_eq!(
        peer.uart_rx,
        Some(ScanPin {
            terminal: "TX".into(),
            gpio: "P0.20".into()
        })
    );
}

#[test]
fn peripheral_helper_emits_source_verified_display_overlay_and_config() {
    let document = document();
    let mut accepted = plan(ElectricalMode::Matrix);
    accepted.peripherals.push(PeripheralRequirement {
        part_id: "display-1".into(),
        source: "catalogue/display".into(),
        kind: "display-i2c".into(),
        gpio_terminals: vec![
            ("SDA".into(), "display-1/i2c/SDA".into()),
            ("SCL".into(), "display-1/i2c/SCL".into()),
        ],
        fixed_terminals: vec![],
        rotary: None,
        press_key_id: None,
    });
    accepted
        .peripheral_pins
        .insert("display-1/i2c/SDA".into(), "P0.26".into());
    accepted
        .peripheral_pins
        .insert("display-1/i2c/SCL".into(), "P0.27".into());
    let (request, _) = firmware_request_adapter::firmware_request(&document, &accepted, None)
        .expect("display request");
    assert_eq!(
        request.peripheral_config,
        [
            "CONFIG_ZMK_DISPLAY=y",
            "CONFIG_I2C=y",
            "CONFIG_SSD1306=y",
            "CONFIG_LVGL=y"
        ]
    );
    assert!(request.peripheral_overlays[0].contains("ssd1306@3c"));
    assert!(request.peripheral_overlays[0].contains("NRF_PSEL(TWIM_SDA, 0, 26)"));
    assert!(request.peripheral_overlays[0].contains("NRF_PSEL(TWIM_SCL, 0, 27)"));
}

#[test]
fn module_diagnostics_are_forwarded_while_other_plan_errors_block_export() {
    let document = document();
    let mut accepted = plan(ElectricalMode::Matrix);
    accepted.diagnostics.push(ElectricalDiagnostic {
        code: "module/left/host-role".into(),
        severity: "error".into(),
        message: "Choose a host connector".into(),
        key_id: Some("left".into()),
    });
    let (request, _) = firmware_request_adapter::firmware_request(&document, &accepted, None)
        .expect("module findings are presented by the worker, not rejected here");
    assert_eq!(
        request.hardware.unwrap().module_findings[0].message,
        "Choose a host connector"
    );

    accepted.diagnostics.push(ElectricalDiagnostic {
        code: "pin/unresolved".into(),
        severity: "error".into(),
        message: "No resolved row pin".into(),
        key_id: None,
    });
    assert_eq!(
        firmware_request_adapter::firmware_request(&document, &accepted, None)
            .expect_err("unresolved electrical plans block export"),
        "No resolved row pin"
    );
}

fn encoder_plan(id: &str, a: &str, b: &str, rotary_b: Option<&str>) -> ElectricalPlan {
    let mut result = plan(ElectricalMode::Matrix);
    let b_terminal = rotary_b.unwrap_or("C");
    result.peripherals.push(PeripheralRequirement {
        part_id: id.into(),
        source: "ceoloide/rotary_encoder_ec11_ec12".into(),
        kind: "encoder".into(),
        gpio_terminals: vec![
            ("A".into(), format!("{id}/encoder/A")),
            (b_terminal.into(), format!("{id}/encoder/{b_terminal}")),
        ],
        fixed_terminals: vec![],
        rotary: rotary_b.map(|terminal| boardstudio_core::model::RotaryProfile {
            a: "A".into(),
            b: terminal.into(),
            common: "common".into(),
            steps: Some(24),
            triggers_per_rotation: Some(4),
            driver: Some(boardstudio_core::model::EncoderDriver::Ec11),
        }),
        press_key_id: None,
    });
    result
        .peripheral_pins
        .insert(format!("{id}/encoder/A"), a.into());
    result
        .peripheral_pins
        .insert(format!("{id}/encoder/{b_terminal}"), b.into());
    result
}

#[test]
fn split_profiled_encoders_keep_each_plans_identity_and_gpio() {
    let mut document = document();
    document.hardware.as_mut().unwrap().transport = HardwareTransport::Wireless;
    for (left_id, right_id) in [("left-knob", "right-knob"), ("shared-knob", "shared-knob")] {
        let left = encoder_plan(left_id, "P0.02", "P0.03", Some("C"));
        let mut right = encoder_plan(right_id, "P1.12", "P1.13", Some("C"));
        right.instance_id = Some("right-half".into());
        let (request, _) =
            firmware_request_adapter::firmware_request(&document, &left, Some(&right)).unwrap();
        assert_eq!(
            request.encoder_ids,
            [left_id],
            "central sensor membership must match React's current-plan projection"
        );
        assert_eq!(request.encoders.len(), 1);
        assert_eq!(
            (
                &*request.encoders[0].id,
                &*request.encoders[0].a_gpio,
                &*request.encoders[0].b_gpio
            ),
            (left_id, "P0.02", "P0.03")
        );
        let peer = request.peripheral.unwrap();
        assert_eq!(peer.encoder_ids, [right_id]);
        assert_eq!(peer.encoders.len(), 1);
        assert_eq!(
            (
                &*peer.encoders[0].id,
                &*peer.encoders[0].a_gpio,
                &*peer.encoders[0].b_gpio
            ),
            (right_id, "P1.12", "P1.13")
        );
        assert_eq!(
            request.hardware.unwrap().physical_instance_id.as_deref(),
            Some("left-half")
        );
        assert_eq!(
            peer.hardware.unwrap().physical_instance_id.as_deref(),
            Some("right-half")
        );
    }
}

#[test]
fn mixed_encoder_profile_failure_is_not_replaced_by_empty_peripheral_overlays() {
    let document = document();
    let central = encoder_plan("left-knob", "P0.02", "P0.03", None);
    // This reviewed rotary profile is valid by itself, but cannot use the legacy
    // EC11 C-terminal overlay required by a mixed legacy/profiled split.
    let peripheral = encoder_plan("right-knob", "P0.12", "P0.13", Some("D"));
    firmware_request_adapter::firmware_request(&document, &peripheral, None).expect(
        "the recursive profiled peripheral request succeeds before the outer legacy override",
    );
    let error = firmware_request_adapter::firmware_request(&document, &central, Some(&peripheral))
        .expect_err("React throws when the outer peripheral legacy overlay cannot be built");
    assert_eq!(
        error,
        "No resolved GPIO for peripheral function right-knob/encoder/C"
    );
}

#[test]
fn legacy_encoder_uses_resolved_c_terminal_and_shared_sensor_overlay_order() {
    let mut document = document();
    document.hardware.as_mut().unwrap().transport = HardwareTransport::Wireless;
    let legacy = |id: &str| {
        let mut value = encoder_plan(id, "P0.02", "P0.03", None);
        // Exact mapping from React firmwareHandoff.test.ts split legacy fixture:
        // terminal C is encoder-b; it is not the optional B terminal or encoder/C fallback.
        value.peripherals[0].gpio_terminals = vec![
            ("A".into(), "encoder-a".into()),
            ("C".into(), "encoder-b".into()),
        ];
        value.peripheral_pins = BTreeMap::from([
            ("encoder-a".into(), "P0.02".into()),
            ("encoder-b".into(), "P0.03".into()),
        ]);
        value
    };
    let (request, _) = firmware_request_adapter::firmware_request(
        &document,
        &legacy("left-knob"),
        Some(&legacy("right-knob")),
    )
    .expect("source-matched legacy C terminal is supported");
    assert_eq!(request.encoder_ids, ["left-knob"]);
    let peer = request.peripheral.unwrap();
    assert_eq!(peer.encoder_ids, ["right-knob"]);
    let sensors = |overlays: &[String]| {
        overlays
            .iter()
            .find(|row| row.contains("zmk,keymap-sensors"))
            .cloned()
            .unwrap()
    };
    assert_eq!(
        sensors(&request.peripheral_overlays),
        sensors(&peer.peripheral_overlays)
    );
    for overlays in [&request.peripheral_overlays, &peer.peripheral_overlays] {
        let text = overlays.join("\n");
        assert!(text.contains("b-gpios = <&gpio0 3 (GPIO_ACTIVE_HIGH | GPIO_PULL_UP)>"));
        assert!(text.contains("status = \"disabled\""));
    }
}
