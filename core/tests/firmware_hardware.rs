use boardstudio_core::firmware::{
    FirmwareHardware, FirmwareModuleFinding, FirmwareModuleQualification, FirmwarePhysicalInstance,
    FirmwareRequest, FirmwareScanMode, ScanPin,
};
use boardstudio_core::model::{HardwareGate, HardwareOutput, ModuleProtocol};

fn request() -> FirmwareRequest {
    FirmwareRequest {
        controller_profile: "ceoloide/mcu_nice_nano".into(),
        board_name: "host".into(),
        rows: vec![ScanPin {
            terminal: "P21".into(),
            gpio: "P0.31".into(),
        }],
        columns: vec![ScanPin {
            terminal: "P20".into(),
            gpio: "P0.29".into(),
        }],
        diode_direction: "col2row".into(),
        mode: FirmwareScanMode::Matrix,
        ..FirmwareRequest::default()
    }
}

#[test]
fn firmware_request_remains_value_comparable_with_compact_module_snapshot() {
    let request = request();
    assert_eq!(request, request.clone());
}

#[test]
fn unsupported_selected_module_blocks_firmware_with_its_qualification_reason() {
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: Some("left".into()),
        modules: vec![FirmwareModuleQualification {
            id: "trackball-left".into(),
            name: "Trackball module".into(),
            host_instance_id: Some("left".into()),
            protocol: ModuleProtocol::Spi,
            catalogue_row: None,
            source: None,
            rotary_profile: None,
            gates: vec![HardwareGate {
                output: HardwareOutput::Firmware,
                code: "driver-unqualified".into(),
                message: "Trackball firmware driver is not qualified".into(),
            }],
        }],
        physical_instances: vec![FirmwarePhysicalInstance {
            id: "left".into(),
            board_id: "host".into(),
        }],
        module_findings: vec![],
        embedded_circuit_ids: vec![],
    });
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(
        error.contains("Trackball firmware driver is not qualified"),
        "{error}"
    );
}

#[test]
fn unsupported_embedded_circuit_blocks_functional_firmware() {
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: None,
        modules: vec![],
        physical_instances: vec![],
        module_findings: vec![],
        embedded_circuit_ids: vec!["drv2605l-circuit".into()],
    });
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(
        error.contains("Embedded module circuits require a qualified local firmware driver"),
        "{error}"
    );
}

#[test]
fn scoped_module_validation_uses_the_selected_physical_host() {
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: Some("left".into()),
        modules: vec![FirmwareModuleQualification {
            id: "right-trackball".into(),
            name: "Right trackball".into(),
            host_instance_id: Some("right".into()),
            protocol: ModuleProtocol::Spi,
            catalogue_row: None,
            source: None,
            rotary_profile: None,
            gates: vec![],
        }],
        physical_instances: vec![
            FirmwarePhysicalInstance {
                id: "left".into(),
                board_id: "host".into(),
            },
            FirmwarePhysicalInstance {
                id: "right".into(),
                board_id: "host".into(),
            },
        ],
        module_findings: vec![],
        embedded_circuit_ids: vec![],
    });
    assert!(boardstudio_core::firmware::generate(&request).is_ok());
}

#[test]
fn gpio_module_without_explicit_rotary_timing_is_not_treated_as_supported() {
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: None,
        modules: vec![FirmwareModuleQualification {
            id: "vik-encoder".into(),
            name: "VIK EC11".into(),
            host_instance_id: None,
            protocol: ModuleProtocol::Gpio,
            catalogue_row: Some("ec11-evqwgd001".into()),
            source: None,
            rotary_profile: Some(boardstudio_core::model::RotaryProfile {
                a: "gpio1".into(),
                b: "gpio2".into(),
                common: "gnd".into(),
                steps: None,
                triggers_per_rotation: None,
                driver: Some(boardstudio_core::model::EncoderDriver::Ec11),
            }),
            gates: vec![],
        }],
        physical_instances: vec![],
        module_findings: vec![],
        embedded_circuit_ids: vec![],
    });
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(
        error.contains("local ZMK driver support is not qualified"),
        "{error}"
    );
}

#[test]
fn invalid_or_missing_physical_context_cannot_hide_scoped_modules() {
    let scoped = FirmwareModuleQualification {
        id: "left-trackball".into(),
        name: "Left trackball".into(),
        host_instance_id: Some("left".into()),
        protocol: ModuleProtocol::PassThrough,
        catalogue_row: None,
        source: None,
        rotary_profile: None,
        gates: vec![],
    };
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: Some("missing".into()),
        modules: vec![scoped.clone()],
        physical_instances: vec![FirmwarePhysicalInstance {
            id: "left".into(),
            board_id: "host".into(),
        }],
        module_findings: vec![],
        embedded_circuit_ids: vec![],
    });
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(error.contains("physical host instance"), "{error}");

    request.hardware.as_mut().unwrap().physical_instance_id = None;
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(error.contains("Select a physical host instance"), "{error}");
}

#[test]
fn rust_generated_module_connection_findings_block_functional_firmware() {
    let mut request = request();
    request.hardware = Some(FirmwareHardware {
        board_id: "host".into(),
        physical_instance_id: None,
        modules: vec![],
        physical_instances: vec![],
        module_findings: vec![FirmwareModuleFinding {
            id: "module/trackball/host-role".into(),
            message: "Choose a host-side connector".into(),
        }],
        embedded_circuit_ids: vec![],
    });
    let error = boardstudio_core::firmware::generate(&request).unwrap_err();
    assert!(error.contains("Choose a host-side connector"), "{error}");
}
