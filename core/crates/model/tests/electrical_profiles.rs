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
fn controller_profiles_declare_supported_assembly_options() {
    let supermini = profile("ceoloide/mcu_supermini_nrf52840").unwrap();
    assert!(supermini.supports_reversible);
    assert!(supermini.supports_extra_pins);
    assert!(supermini.rejects_invert_jumpers_position);

    let nice_view = profile("infused-kim/nice_view").unwrap();
    assert!(nice_view.supports_reversible);
}
