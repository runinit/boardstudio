use boardstudio_model::electrical_jumpers::*;
use boardstudio_model::model::*;
use std::collections::BTreeMap;
fn fixture(
    source: &str,
    side: Side,
    params: BTreeMap<String, serde_json::Value>,
) -> (Part, PartDefinition) {
    (
        Part {
            id: "u1".into(),
            definition_id: "d".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side,
            anchor: None,
            locked: None,
            keycap: None,
            outline: None,
            properties: None,
            generator_parameters: Some(params),
        },
        PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "d".into(),
            name: "d".into(),
            kind: PartKind::Controller,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],

            models: None,
            generator: Some(PartGenerator {
                source: source.into(),
                version: "test".into(),
                parameters: BTreeMap::new(),
            }),
            mechanical_profile: None,
        },
    )
}
#[test]
fn mcu_full_and_reduced_sites_are_deterministic() {
    let mut all = BTreeMap::new();
    all.insert("reversible".into(), serde_json::json!(true));
    let (p, d) = fixture("ceoloide/mcu_nice_nano", Side::Front, all);
    assert_eq!(describe(&p, &d).unwrap().sites.len(), 48);
    let mut q = BTreeMap::new();
    q.insert("reversible".into(), serde_json::json!(true));
    q.insert("only_required_jumpers".into(), serde_json::json!(true));
    let (p, d) = fixture("ceoloide/mcu_nice_nano", Side::Back, q);
    let r = describe(&p, &d).unwrap();
    assert_eq!(r.sites.len(), 16);
    assert!(r.sites.iter().any(|s| s.close));
}
#[test]
fn extra_pins_are_never_fabricated_jumper_sites() {
    let mut q = BTreeMap::new();
    q.insert("reversible".into(), serde_json::json!(true));
    q.insert("include_extra_pins".into(), serde_json::json!(true));
    let (p, d) = fixture("ceoloide/mcu_supermini_nrf52840", Side::Front, q);
    assert!(
        describe(&p, &d)
            .unwrap()
            .sites
            .iter()
            .all(|s| s.signal_terminal != "P107")
    );
}
#[test]
fn traces_disabled_is_routing_obligation() {
    let mut q = BTreeMap::new();
    q.insert("reversible".into(), serde_json::json!(true));
    q.insert("include_traces".into(), serde_json::json!(false));
    let (p, d) = fixture("ceoloide/display_nice_view", Side::Front, q);
    assert!(
        describe(&p, &d)
            .unwrap()
            .sites
            .iter()
            .all(|s| s.routing_required)
    );
}
