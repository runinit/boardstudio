//! Pure conversion from a resolved Parts recipe into an editable assembly draft.
use boardstudio_core::model::{AssemblyDefinition, AssemblyMember, AssemblyModelMode, Pose2};

pub(crate) fn from_recipe(
    id: String,
    name: String,
    members: Vec<crate::parts_preview::PartsPreviewRecipeMember>,
) -> AssemblyDefinition {
    AssemblyDefinition {
        id,
        name,
        members: members
            .into_iter()
            .map(|member| AssemblyMember {
                parameters: Some(member.generator_parameters),
                model_mode: Some(AssemblyModelMode::Defaults),
                id: member.id,
                definition_id: Some(member.definition.id),
                pose: Pose2 {
                    at: member.at,
                    rotation: member.rotation,
                },
                side: member.side,
                models: Vec::new(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{
        AssemblyMember, AssemblyModelMode, PartDefinition, PartGenerator, PartKind, PartModel,
        Pose2, Side, Vec2, Vec3,
    };
    use std::collections::BTreeMap;

    fn model(asset_id: &str) -> PartModel {
        PartModel {
            asset_id: asset_id.into(),
            offset: Vec3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            rotation: Vec3 {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }

    fn recipe_member(
        id: &str,
        definition_id: &str,
        source: &str,
        at: Vec2,
        rotation: f64,
        side: Side,
        parameters: serde_json::Value,
    ) -> crate::parts_preview::PartsPreviewRecipeMember {
        crate::parts_preview::PartsPreviewRecipeMember {
            id: id.into(),
            definition: PartDefinition {
                hardware_profile: None,
                input_profile: None,
                id: definition_id.into(),
                name: definition_id.into(),
                kind: PartKind::Switch,
                keycap: None,
                envelope_source: None,
                kicad_source: None,
                terminals: BTreeMap::new(),
                matrix_terminals: None,
                envelope_notice: None,
                courtyard: Vec::new(),
                pads: Vec::new(),
                models: Some(vec![model("default-model")]),
                generator: Some(PartGenerator {
                    source: source.into(),
                    version: "1".into(),
                    parameters: BTreeMap::new(),
                }),
                mechanical_profile: None,
            },
            assets: Vec::new(),
            at,
            rotation,
            side,
            generator_parameters: serde_json::from_value(parameters).unwrap(),
        }
    }

    #[test]
    fn resolved_mx_hotswap_rgb_recipe_becomes_an_independent_editable_assembly() {
        let recipe = vec![
            recipe_member(
                "switch",
                "generator:ceoloide/switch_mx",
                "ceoloide/switch_mx",
                Vec2 { x: 0.0, y: 0.0 },
                0.0,
                Side::Front,
                serde_json::json!({
                    "hotswap": true,
                    "solder": false,
                    "reversible": false,
                    "side": "B",
                    "include_keycap": true
                }),
            ),
            recipe_member(
                "diode",
                "generator:ceoloide/diode_tht_sod123",
                "ceoloide/diode_tht_sod123",
                Vec2 { x: 7.4, y: -1.5 },
                90.0,
                Side::Back,
                serde_json::json!({"side": "B", "reversible": false, "include_tht": false}),
            ),
            recipe_member(
                "led",
                "generator:ceoloide/led_sk6812mini-e",
                "ceoloide/led_sk6812mini-e",
                Vec2 { x: 0.0, y: -4.75 },
                180.0,
                Side::Back,
                serde_json::json!({"side": "B", "reverse_mount": true, "reversible": false}),
            ),
        ];
        let source = recipe.clone();

        let assembly = from_recipe(
            "assembly-test-id".into(),
            "MX HOTSWAP RGB".into(),
            recipe.clone(),
        );

        assert_eq!(assembly.id, "assembly-test-id");
        assert_eq!(assembly.name, "MX HOTSWAP RGB");
        assert_eq!(
            assembly.members,
            vec![
                AssemblyMember {
                    parameters: Some(
                        serde_json::from_value(serde_json::json!({
                            "hotswap": true, "solder": false, "reversible": false,
                            "side": "B", "include_keycap": true
                        }))
                        .unwrap()
                    ),
                    model_mode: Some(AssemblyModelMode::Defaults),
                    id: "switch".into(),
                    definition_id: Some("generator:ceoloide/switch_mx".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: 0.0 },
                        rotation: 0.0
                    },
                    side: Side::Front,
                    models: Vec::new(),
                },
                AssemblyMember {
                    parameters: Some(
                        serde_json::from_value(serde_json::json!({
                            "side": "B", "reversible": false, "include_tht": false
                        }))
                        .unwrap()
                    ),
                    model_mode: Some(AssemblyModelMode::Defaults),
                    id: "diode".into(),
                    definition_id: Some("generator:ceoloide/diode_tht_sod123".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 7.4, y: -1.5 },
                        rotation: 90.0
                    },
                    side: Side::Back,
                    models: Vec::new(),
                },
                AssemblyMember {
                    parameters: Some(
                        serde_json::from_value(serde_json::json!({
                            "side": "B", "reverse_mount": true, "reversible": false
                        }))
                        .unwrap()
                    ),
                    model_mode: Some(AssemblyModelMode::Defaults),
                    id: "led".into(),
                    definition_id: Some("generator:ceoloide/led_sk6812mini-e".into()),
                    pose: Pose2 {
                        at: Vec2 { x: 0.0, y: -4.75 },
                        rotation: 180.0
                    },
                    side: Side::Back,
                    models: Vec::new(),
                },
            ]
        );
        assert_eq!(recipe, source);
    }
}
