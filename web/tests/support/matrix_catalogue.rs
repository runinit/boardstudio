use boardstudio_core::model::{PartDefinition, PartGenerator, PartKind};
use std::collections::BTreeMap;

pub fn definition(id: &str, source: &str, kind: PartKind) -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
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
            version: "fixture".into(),
            parameters: BTreeMap::new(),
        }),
    }
}

pub fn catalogue() -> Vec<PartDefinition> {
    vec![
        definition(
            "ergogen:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            PartKind::Switch,
        ),
        definition(
            "ergogen:ceoloide/diode_tht_sod123",
            "ceoloide/diode_tht_sod123",
            PartKind::Passive,
        ),
        definition(
            "ergogen:ceoloide/led_sk6812mini-e",
            "ceoloide/led_sk6812mini-e",
            PartKind::Passive,
        ),
    ]
}
