use boardstudio_core::firmware::*;
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/boardstudio-zmk-check"));
    let request = FirmwareRequest {
        qualification: None,
        hardware: None,
        encoders: vec![],
        keymap: None,
        encoder_ids: vec![],
        controller_profile: "ceoloide/mcu_nice_nano".into(),
        board_name: "boardstudio-check".into(),
        rows: vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }],
        columns: vec![ScanPin {
            terminal: "P20".into(),
            gpio: "P0.29".into(),
        }],
        keys: vec![FirmwareKey {
            id: "k1".into(),
            row: 0,
            column: 0,
        }],
        diode_direction: "col2row".into(),
        matrix_row_offset: 0,
        mode: FirmwareScanMode::Matrix,
        direct_pins: vec![],
        auxiliary_pins: vec![],
        key_bindings: vec![],
        peripheral_config: vec![],
        transport: Some(SplitTransport::WiredUart),
        uart_tx: Some(ScanPin {
            terminal: "P1".into(),
            gpio: "P0.06".into(),
        }),
        uart_rx: Some(ScanPin {
            terminal: "P0".into(),
            gpio: "P0.08".into(),
        }),
        peripheral_overlays: vec![],
        peripheral: Some(Box::new(FirmwareRequest {
            qualification: None,
            hardware: None,
            encoders: vec![],
            keymap: None,
            encoder_ids: vec![],
            transport: Some(SplitTransport::WiredUart),
            ..FirmwareRequest {
                qualification: None,
                hardware: None,
                encoders: vec![],
                keymap: None,
                encoder_ids: vec![],
                controller_profile: "ceoloide/mcu_nice_nano".into(),
                board_name: "boardstudio-check-right".into(),
                rows: vec![ScanPin {
                    terminal: "P21".into(),
                    gpio: "P0.31".into(),
                }],
                columns: vec![ScanPin {
                    terminal: "P20".into(),
                    gpio: "P0.29".into(),
                }],
                keys: vec![FirmwareKey {
                    id: "k1".into(),
                    row: 0,
                    column: 0,
                }],
                diode_direction: "col2row".into(),
                matrix_row_offset: 0,
                mode: FirmwareScanMode::Matrix,
                direct_pins: vec![],
                auxiliary_pins: vec![],
                key_bindings: vec![],
                peripheral_config: vec![],
                transport: None,
                uart_tx: Some(ScanPin {
                    terminal: "P0".into(),
                    gpio: "P0.08".into(),
                }),
                uart_rx: Some(ScanPin {
                    terminal: "P1".into(),
                    gpio: "P0.06".into(),
                }),
                peripheral_overlays: vec![],
                peripheral: None,
            }
        })),
    };
    fs::create_dir_all(&output)?;
    for (path, content) in generate(&request)?.files {
        let target = output.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, content)?;
    }
    println!("{}", output.display());
    Ok(())
}
