use super::catalogue::CatalogEntry;
use boardstudio_core::model::{PartDefinition, Side, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixPresetId {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::presentation) enum SwitchOrientation {
    #[default]
    South,
    North,
}

#[derive(Clone, Copy)]
pub(super) struct Preset {
    pub(super) id: MatrixPresetId,
    pub(super) name: &'static str,
    pub(super) definition_id: &'static str,
    family: &'static str,
    hotswap: bool,
    led: bool,
}

pub(super) const PRESETS: [Preset; 8] = [
    Preset {
        id: MatrixPresetId::MxSolder,
        name: "MX Solder",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxHotswap,
        name: "MX Hotswap",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocSolder,
        name: "Choc V1 Solder",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: false,
    },
    Preset {
        id: MatrixPresetId::ChocHotswap,
        name: "Choc V1 Hotswap",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: false,
    },
    Preset {
        id: MatrixPresetId::MxRgb,
        name: "MX RGB",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocRgb,
        name: "Choc V1 RGB",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: false,
        led: true,
    },
    Preset {
        id: MatrixPresetId::MxHotswapRgb,
        name: "MX Hotswap RGB",
        definition_id: "ergogen:ceoloide/switch_mx",
        family: "mx",
        hotswap: true,
        led: true,
    },
    Preset {
        id: MatrixPresetId::ChocHotswapRgb,
        name: "Choc V1 Hotswap RGB",
        definition_id: "ergogen:ceoloide/switch_choc_v1_v2",
        family: "choc",
        hotswap: true,
        led: true,
    },
];

pub(super) fn name(id: MatrixPresetId) -> &'static str {
    preset(id).name
}

fn preset(id: MatrixPresetId) -> &'static Preset {
    PRESETS
        .iter()
        .find(|preset| preset.id == id)
        .expect("all preset ids are listed")
}

pub(super) async fn resolve(
    id: MatrixPresetId,
    entries: &[CatalogEntry],
    reversible: bool,
    orientation: SwitchOrientation,
    preview_definition: Option<PartDefinition>,
) -> Result<Vec<crate::parts_preview::PartsPreviewRecipeMember>, String> {
    let preset = preset(id);
    let find_source = |source: &str| {
        entries.iter().find(|entry| {
            entry
                .definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == source)
        })
    };
    let main_source = preset
        .definition_id
        .strip_prefix("ergogen:")
        .unwrap_or(preset.definition_id);
    let main = find_source(main_source)
        .ok_or_else(|| format!("Missing switch footprint: {main_source}"))?;
    let mut members = vec![member(
        "switch",
        preview_definition
            .as_ref()
            .filter(|definition| definition.id == main.definition.id)
            .unwrap_or(&main.definition),
        Vec2 { x: 0.0, y: 0.0 },
        0.0,
        Side::Front,
        switch_parameters(preset, reversible),
    )];
    if let Some(diode) = find_source("ceoloide/diode_tht_sod123") {
        members.push(member(
            "diode",
            &diode.definition,
            Vec2 { x: 7.4, y: -1.5 },
            90.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reversible": reversible, "include_tht": false
            }))
            .expect("fixed diode parameters are valid"),
        ));
    }
    if preset.led {
        let led = find_source("ceoloide/led_sk6812mini-e")
            .ok_or_else(|| "Missing SK6812 MINI-E footprint".to_owned())?;
        members.push(member(
            "led",
            &led.definition,
            Vec2 {
                x: 0.0,
                y: if preset.family == "choc" { -4.7 } else { -4.75 },
            },
            180.0,
            Side::Back,
            serde_json::from_value(serde_json::json!({
                "side": "B", "reverse_mount": true, "reversible": reversible
            }))
            .expect("fixed LED parameters are valid"),
        ));
    }
    for member in &mut members {
        if let Some(generator) = member.definition.generator.as_mut() {
            generator
                .parameters
                .extend(member.generator_parameters.clone());
        }
        if orientation == SwitchOrientation::North {
            member.at.x = -member.at.x;
            member.at.y = -member.at.y;
            member.rotation = (member.rotation + 180.0) % 360.0;
        }
        member.definition =
            super::catalogue::normalize_generator_definition(member.definition.clone()).await?;
    }
    Ok(members)
}

fn switch_parameters(
    preset: &Preset,
    reversible: bool,
) -> std::collections::BTreeMap<String, serde_json::Value> {
    let mut parameters: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({
            "hotswap": preset.hotswap,
            "solder": !preset.hotswap,
            "reversible": reversible,
            "side": "B",
            "include_keycap": true
        }))
        .expect("fixed switch parameters are valid");
    if preset.family == "choc" {
        parameters.extend(
            serde_json::from_value::<std::collections::BTreeMap<String, serde_json::Value>>(
                serde_json::json!({
                    "choc_v1_support": true,
                    "choc_v2_support": false,
                    "include_choc_v1_led_cutout_marks": true
                }),
            )
            .expect("fixed Choc parameters are valid"),
        );
    }
    parameters
}

fn member(
    id: &str,
    definition: &PartDefinition,
    at: Vec2,
    rotation: f64,
    side: Side,
    generator_parameters: std::collections::BTreeMap<String, serde_json::Value>,
) -> crate::parts_preview::PartsPreviewRecipeMember {
    crate::parts_preview::PartsPreviewRecipeMember {
        id: id.to_owned(),
        definition: definition.clone(),
        at,
        rotation,
        side,
        generator_parameters,
    }
}
