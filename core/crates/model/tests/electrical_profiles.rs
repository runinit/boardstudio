use boardstudio_model::electrical_profiles::*;

#[test]
fn reversible_controller_keeps_fabricated_nets_separate() {
    let p = profile("ceoloide/mcu_nice_nano").unwrap();
    assert_eq!(
        p.pins
            .iter()
            .find(|pin| pin.terminal == "P1")
            .unwrap()
            .firmware_gpio,
        "P0.06"
    );
}

#[test]
fn nice_nano_v2_scan_capabilities_have_unique_gpio_targets() {
    let p = profile("ceoloide/mcu_nice_nano").unwrap();
    let mut gpios = p
        .pins
        .iter()
        .map(|pin| pin.firmware_gpio)
        .collect::<Vec<_>>();
    gpios.sort_unstable();
    gpios.dedup();
    assert_eq!(gpios.len(), p.pins.len());
    assert_eq!(
        p.pins
            .iter()
            .find(|pin| pin.terminal == "P14")
            .unwrap()
            .firmware_gpio,
        "P1.11"
    );
}

#[test]
fn reduced_jumpers_still_require_valid_controller_options() {
    let p = profile("ceoloide/mcu_supermini_nrf52840").unwrap();
    assert!(validate_params(&p, true, false, true, true, false).is_ok());
    assert!(validate_params(&p, true, false, true, true, true).is_err());
    assert!(validate_params(&p, true, false, true, false, false).is_ok());
}

#[test]
fn infused_nice_view_requires_routes_to_local_jumper_pads() {
    let p = profile("infused-kim/nice_view").unwrap();
    assert!(validate_params(&p, true, false, false, false, false).is_ok());
}
