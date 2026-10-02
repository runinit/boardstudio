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
