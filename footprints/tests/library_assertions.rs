//! Assertions carried over from the footprint-library and provider Node tests
//! (`ergogen/library/scripts/*.test.mjs`, `ergogen/scripts/{catalogue,runtime}.test.mjs`)
//! that describe generator behaviour beyond what the golden fixtures pin down:
//! default model bindings and overrides, side-dependent model frames, drill
//! validation, one-token model paths, and catalogue shape. The tests for the
//! three vendored generators the catalogue excludes (`choc`, `diode`,
//! `nice_nano_pretty`) and for the upstream refresh and inventory scripts are
//! retired with that code.
mod common;

use boardstudio_footprints::nets::NetAllocator;
use boardstudio_footprints::sexpr::{Expr, child, children, serialize};
use boardstudio_footprints::types::{GeneratorRef, PartRef, Pose2, Side, Vec2};
use boardstudio_footprints::{GeneratorError, Result, bundled};
use serde_json::{Value, json};

fn generator(source: &str, parameters: Value) -> GeneratorRef {
    GeneratorRef {
        source: source.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).unwrap(),
    }
}

fn part(side: Side, rotation: f64) -> PartRef {
    PartRef {
        id: "part-1".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation,
        },
        side,
        generator_parameters: None,
    }
}

/// Render a placed front-side part and return the forms.
fn forms(source: &str, parameters: Value) -> Result<Vec<Expr>> {
    let mut nets = NetAllocator::new(&[], 1);
    bundled().render(
        "definition",
        &generator(source, parameters),
        Some(&part(Side::Front, 0.0)),
        &mut nets,
    )
}

fn text(source: &str, parameters: Value) -> Result<String> {
    Ok(forms(source, parameters)?
        .iter()
        .map(serialize)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn rendered(source: &str, parameters: Value) -> String {
    text(source, parameters).unwrap_or_else(|error| panic!("{source}: {error}"))
}

fn model_nodes(forms: &[Expr]) -> Vec<&[Expr]> {
    forms
        .iter()
        .filter_map(Expr::as_list)
        .flat_map(|form| children(form, "model"))
        .collect()
}

fn xyz(node: &[Expr], name: &str) -> String {
    let xyz = child(child(node, name).expect("transform"), "xyz").expect("xyz");
    xyz[1..].iter().map(serialize).collect::<Vec<_>>().join(" ")
}

fn parameters(source: &str) -> Vec<(String, Value)> {
    bundled()
        .parameters(source)
        .unwrap()
        .into_iter()
        .map(|p| (p.name, p.value.unwrap_or(Value::Null)))
        .collect()
}

fn default_of(source: &str, name: &str) -> Value {
    parameters(source)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap()
}

#[test]
fn default_models_are_bound_and_overridable() {
    assert!(
        default_of("ceoloide/switch_choc_v1_v2", "switch_3dmodel_filename")
            .as_str()
            .unwrap()
            .ends_with(".step")
    );
    assert_eq!(
        default_of(
            "ceoloide/power_switch_smd_side",
            "switch_3dmodel_xyz_rotation"
        ),
        json!([-90, 0, -90])
    );
    // Bosses select the boss-compatible reset switch model; an explicit model wins.
    assert!(
        rendered(
            "ceoloide/reset_switch_smd_side",
            json!({ "include_bosses": true })
        )
        .contains("Panasonic_EVQPUL_EVQPUC.step")
    );
    let custom = rendered(
        "ceoloide/reset_switch_smd_side",
        json!({ "include_bosses": true, "reset_switch_3dmodel_filename": "custom.step" }),
    );
    assert!(custom.contains("custom.step") && !custom.contains("Panasonic_EVQPUL"));
    assert!(
        rendered("ceoloide/reset_switch_smd_side", json!({}))
            .contains("Panasonic_EVQPUJ_EVQPUA.step")
    );
}

#[test]
fn smd_components_get_one_model_each_over_their_own_pads() {
    let source = "infused-kim/smd_0805";
    for side in ["F", "B"] {
        for components in [1, 2, 6] {
            let output = forms(
                source,
                json!({ "side": side, "reverse": false, "components": components }),
            )
            .unwrap();
            assert_eq!(
                model_nodes(&output).len(),
                components,
                "{side}: {components} models"
            );
        }
    }
    let base = json!({ "side": "B", "reverse": false, "components": 2, "component_1_3dmodel_filename": "first.step",
        "component_2_3dmodel_filename": "second.step", "component_1_3dmodel_xyz_scale": [2, 3, 4] });
    assert!(rendered(source, base.clone()).contains("(model \"first.step\" (at (xyz"));
    assert!(rendered(source, base.clone()).contains("(scale (xyz 2 3 4))"));
    // With mirroring off, each model stays over its own net's pads.
    let mut unmirrored = base.clone();
    unmirrored["mirror"] = json!(false);
    let output = forms(source, unmirrored).unwrap();
    let first: f64 = xyz(model_nodes(&output)[0], "at")
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (first * 25.4 - -1.5125).abs() < 1e-9,
        "model x = {}",
        first * 25.4
    );

    for side in ["F", "B"] {
        for mirror in [false, true] {
            for components in [1usize, 2, 6] {
                let mut sample = json!({ "side": side, "reverse": false, "mirror": mirror, "components": components });
                for index in 1..=components {
                    sample[format!("component_{index}_3dmodel_filename")] =
                        json!(format!("part{index}.step"));
                }
                let output = forms(source, sample).unwrap();
                let models = model_nodes(&output);
                assert_eq!(models.len(), components);
                for (index, model) in models.iter().enumerate() {
                    let at: Vec<f64> = xyz(model, "at")
                        .split(' ')
                        .map(|v| v.parse().unwrap())
                        .collect();
                    let expected_x = (1.025 + 2.0)
                        * (index as f64 - (components as f64 - 1.0) / 2.0)
                        * if side == "B" && mirror { -1.0 } else { 1.0 };
                    assert!(
                        (at[0] * 25.4 - expected_x).abs() < 1e-9,
                        "{side}/{mirror}/{components}: model {index}"
                    );
                    assert!((at[2] * 25.4 - if side == "B" { -1.6 } else { 0.0 }).abs() < 1e-9);
                }
            }
        }
    }
}

/// A source with a bundled default model, rendered on both sides with custom
/// overrides: defaults follow the side, overrides win.
#[test]
fn model_frames_follow_the_side_and_explicit_transforms_win() {
    for side in ["F", "B"] {
        let jst = rendered(
            "ceoloide/battery_connector_jst_ph_2",
            json!({ "side": side }),
        );
        assert!(jst.contains("JST_PH_S2B-PH-K_1x02_P2.00mm_Horizontal.step"));
        assert!(jst.contains(&format!(
            "(offset (xyz {} 0 0))",
            if side == "F" { -1 } else { 1 }
        )));
        assert!(jst.contains(&format!(
            "(rotate (xyz 0 0 {}))",
            if side == "F" { 0 } else { 180 }
        )));
        let jst_custom = rendered(
            "ceoloide/battery_connector_jst_ph_2",
            json!({ "side": side,
            "battery_connector_3dmodel_xyz_offset": [3, 4, 5], "battery_connector_3dmodel_xyz_rotation": [6, 7, 8] }),
        );
        assert!(
            jst_custom.contains("(offset (xyz 3 4 5))")
                && jst_custom.contains("(rotate (xyz 6 7 8))")
        );

        let mx = rendered("ceoloide/switch_mx", json!({ "side": side }));
        assert!(mx.contains("SW_Cherry_MX_PCB.stp"));
        assert!(mx.contains(if side == "F" {
            "(rotate (xyz 0 180 0))"
        } else {
            "(rotate (xyz 180 0 0))"
        }));
        let mx_custom = rendered(
            "ceoloide/switch_mx",
            json!({ "side": side,
            "switch_3dmodel_xyz_offset": [3, 4, 5], "switch_3dmodel_xyz_rotation": [6, 7, 8] }),
        );
        assert!(
            mx_custom.contains("(offset (xyz 3 4 5))")
                && mx_custom.contains("(rotate (xyz 6 7 8))")
        );

        let nice_view = rendered("ceoloide/display_nice_view", json!({ "side": side }));
        assert!(nice_view.contains("PinSocket_1x05_P2.54mm_Vertical.step"));
        for name in ["niceview", "pin_header", "pin_socket"] {
            let custom = rendered(
                "ceoloide/display_nice_view",
                json!({ "side": side,
                format!("{name}_3dmodel_xyz_offset"): [1, 2, 3], format!("{name}_3dmodel_xyz_rotation"): [4, 5, 6] }),
            );
            assert!(
                custom.contains("(offset (xyz 1 2 3))") && custom.contains("(rotate (xyz 4 5 6))"),
                "{name}"
            );
        }
    }
}

#[test]
fn supermini_models_are_complete_for_every_mounting() {
    for side in ["F", "B"] {
        for reversible in [false, true] {
            for reverse_mount in [false, true] {
                let base = json!({ "side": side, "reversible": reversible, "reverse_mount": reverse_mount });
                let output = rendered("ceoloide/mcu_supermini_nrf52840", base.clone());
                assert!(output.contains("models/boardstudio/tsuki/nrf52840.step"));
                assert!(!output.contains("undefined") && !output.contains("NaN"));
                let mut custom = base;
                custom["mcu_3dmodel_xyz_offset"] = json!([1, 2, 3]);
                custom["mcu_3dmodel_xyz_rotation"] = json!([4, 5, 6]);
                let custom = rendered("ceoloide/mcu_supermini_nrf52840", custom);
                assert!(
                    custom.contains("(offset (xyz 1 2 3))")
                        && custom.contains("(rotate (xyz 4 5 6))")
                );
            }
        }
    }
}

#[test]
fn gateron_model_follows_mounting_and_hotswap() {
    assert_eq!(
        default_of(
            "ceoloide/switch_gateron_ks27_ks33",
            "switch_3dmodel_filename"
        ),
        json!("${KIPRJMOD}/models/boardstudio/gdek/KS33.stp")
    );
    for side in ["F", "B"] {
        for hotswap in [false, true] {
            let back = side == "B";
            let rotation = if hotswap {
                if back { "180 0 0" } else { "0 180 0" }
            } else if back {
                "0 0 180"
            } else {
                "0 0 0"
            };
            let x = if back == hotswap { -60 } else { 60 };
            let z = if hotswap { "1.65" } else { "-3.25" };
            let output = rendered(
                "ceoloide/switch_gateron_ks27_ks33",
                json!({ "side": side, "hotswap": hotswap }),
            );
            assert!(
                output.contains(&format!("(offset (xyz {x} 0 {z}))")),
                "{side}/{hotswap}"
            );
            assert!(
                output.contains(&format!("(rotate (xyz {rotation}))")),
                "{side}/{hotswap}"
            );
            let custom = rendered(
                "ceoloide/switch_gateron_ks27_ks33",
                json!({ "side": side, "hotswap": hotswap,
                "switch_3dmodel_xyz_offset": [1, 2, 3], "switch_3dmodel_xyz_rotation": [4, 5, 6] }),
            );
            assert!(
                custom.contains("(offset (xyz 1 2 3))") && custom.contains("(rotate (xyz 4 5 6))")
            );
        }
    }
}

#[test]
fn trackpoint_extension_needs_a_large_enough_drill() {
    let source = "infused-kim/trackpoint_mount";
    assert!(text(source, json!({})).is_ok());
    let error = text(source, json!({ "drill": 3.5 })).unwrap_err();
    assert!(
        error
            .message()
            .contains("extension requires a center drill of at least 5 mm")
    );
    assert!(matches!(error, GeneratorError::Rejected { .. }));
    for override_ in [
        json!({ "tp_extension_3dmodel_filename": "custom.step" }),
        json!({ "tp_extension_3dmodel_filename": "" }),
        json!({ "tp_extension_3dmodel_xyz_scale": [0.5, 0.5, 1] }),
    ] {
        let mut parameters = json!({ "drill": 3.5 });
        parameters
            .as_object_mut()
            .unwrap()
            .extend(override_.as_object().unwrap().clone());
        assert!(text(source, parameters).is_ok());
    }
}

#[test]
fn reset_tht_pads_follow_the_pts636_land_pattern() {
    for side in ["F", "B"] {
        for reversible in [false, true] {
            let output = forms(
                "ceoloide/reset_switch_tht_top",
                json!({ "side": side, "reversible": reversible, "from": "GND", "to": "RST" }),
            )
            .unwrap();
            let footprint = output[0].as_list().unwrap();
            let pad = |number: &str| {
                children(footprint, "pad")
                    .find(|p| p[1] == Expr::string(number))
                    .map(|p| p.iter().map(serialize).collect::<Vec<_>>().join(" "))
                    .unwrap()
            };
            // 6.4 mm centers and 1.2 mm drills.
            let two = pad("2");
            assert!(
                two.contains("(at -3.2 0 0) (size 1.9 1.9) (drill 1.2)") && two.contains("\"GND\""),
                "{two}"
            );
            let one = pad("1");
            assert!(
                one.contains("(at 3.2 0 0) (size 1.9 1.9) (drill 1.2)") && one.contains("\"RST\""),
                "{one}"
            );
        }
    }
}

#[test]
fn encoder_signal_drills_keep_nets_and_centers() {
    let source = "ceoloide/rotary_encoder_ec11_ec12";
    for side in ["F", "B"] {
        for angle in [0.0, 90.0] {
            let render = |extra: Value| {
                let mut parameters = json!({ "side": side, "A": "A", "B": "GND", "C": "C", "S1": "SW1", "S2": "SW2" });
                parameters
                    .as_object_mut()
                    .unwrap()
                    .extend(extra.as_object().unwrap().clone());
                let mut nets = NetAllocator::new(&[], 1);
                bundled().render(
                    "d",
                    &generator(source, parameters),
                    Some(&part(Side::Front, angle)),
                    &mut nets,
                )
            };
            let pads = |forms: &[Expr]| -> Vec<Vec<Expr>> {
                children(forms[0].as_list().unwrap(), "pad")
                    .filter(|p| matches!(&p[1], Expr::Str(n) if ["A", "B", "C", "S1", "S2"].contains(&n.as_str())))
                    .map(<[Expr]>::to_vec)
                    .collect()
            };
            let describe = |pad: &[Expr], names: &[&str]| -> String {
                names
                    .iter()
                    .map(|name| serialize(&Expr::List(child(pad, name).unwrap().to_vec())))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let legacy = pads(&render(json!({})).unwrap());
            assert_eq!(legacy.len(), 5);
            assert!(legacy.iter().all(|p| describe(p, &["size", "drill"]) == "(size 1.6 1.1) (drill oval 1 0.5)"));
            // Alps specifies five round signal holes; changing drills must retain nets and centers.
            let round =
                pads(&render(json!({ "signal_hole_width": 1, "signal_hole_height": 1 })).unwrap());
            assert_eq!(round.len(), 5);
            assert!(
                round
                    .iter()
                    .all(|p| describe(p, &["size", "drill"]) == "(size 1.6 1.6) (drill 1)")
            );
            let geometry = |pad: &Vec<Expr>| -> Vec<Expr> {
                pad.iter().filter(|e| !matches!(e, Expr::List(l) if matches!(l.first(), Some(Expr::Atom(h)) if h == "size" || h == "drill"))).cloned().collect()
            };
            assert_eq!(
                round.iter().map(geometry).collect::<Vec<_>>(),
                legacy.iter().map(geometry).collect::<Vec<_>>()
            );
            let error = render(json!({ "signal_hole_width": 0 })).unwrap_err();
            assert!(error.message().contains("signal hole"));
        }
    }
    // Imported assets can have spaces; KiCad requires one quoted filename token.
    let filename = "/tmp/encoder models/EC11E-05SW.STEP";
    assert!(
        rendered(source, json!({ "encoder_3dmodel_filename": filename })).contains(&format!(
            "(model {}",
            serde_json::to_string(filename).unwrap()
        ))
    );
}

#[test]
fn choc_v2_only_uses_the_bundled_v2_model_when_it_fits() {
    let source = "ceoloide/switch_choc_v1_v2";
    let base = json!({ "choc_v1_support": false, "choc_v2_support": true, "from": "input", "to": "output" });
    let with = |extra: Value| {
        let mut parameters = base.clone();
        parameters
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        parameters
    };
    let output = rendered(source, base.clone());
    assert!(output.contains("koktoh/Choc_V2_Red.step") && output.contains("Choc_V1_Hotswap.step"));
    assert!(!output.contains("Choc_V1_Switch") && !output.contains("Keycap_MBK"));
    assert!(
        rendered(
            source,
            with(json!({ "switch_3dmodel_filename": "custom.step" }))
        )
        .contains("custom.step")
    );
    assert!(!rendered(source, with(json!({ "choc_v2_support": false }))).contains("(model"));
    for changes in [
        json!({ "include_stabilizer_pad": false }),
        json!({ "oval_stabilizer_pad": true }),
        json!({ "center_hole_diameter": 4.7 }),
    ] {
        let error = text(source, with(changes.clone())).unwrap_err();
        assert!(
            error.message().contains("round stabilizer hole"),
            "{changes}"
        );
        let mut custom = changes.clone();
        custom["switch_3dmodel_filename"] = json!("custom.step");
        assert!(text(source, with(custom)).is_ok());
    }
    assert!(text(source, with(json!({ "center_hole_diameter": 4.8 }))).is_ok());
    assert!(
        !rendered(source, with(json!({ "hotswap": false, "solder": true })))
            .contains("Choc_V1_Hotswap")
    );
    for (side, rotation) in [("F", "180 0 0"), ("B", "0 180 0")] {
        let output = forms(source, with(json!({ "side": side }))).unwrap();
        let v2 = model_nodes(&output)
            .into_iter()
            .find(|m| matches!(&m[1], Expr::Str(p) if p.contains("koktoh/")))
            .unwrap();
        assert_eq!(xyz(v2, "rotate"), rotation);
        assert_eq!(xyz(v2, "offset"), "0 0 -1.6");
    }
    let custom = rendered(
        source,
        with(
            json!({ "switch_3dmodel_xyz_rotation": [10, 20, 30], "switch_3dmodel_xyz_offset": [1, 2, 3] }),
        ),
    );
    assert!(custom.contains("(rotate (xyz 10 20 30))") && custom.contains("(offset (xyz 1 2 3))"));
}

#[test]
fn choc_v2_combined_mounting_keeps_both_stabilizer_holes() {
    let source = "ceoloide/switch_choc_v1_v2";
    for side in ["F", "B"] {
        let base = json!({ "choc_v1_support": false, "choc_v2_support": true, "hotswap": true, "solder": true, "side": side });
        let count = |text: &str| text.matches("(drill 1.6)").count();
        assert_eq!(
            count(&rendered(source, base.clone())),
            2,
            "{side}: both mounting choices need a stabilizer hole"
        );
        let mut single = base.clone();
        single["solder"] = json!(false);
        assert_eq!(count(&rendered(source, single)), 1);
    }
}

#[test]
fn every_model_path_is_one_quoted_token() {
    let mut checked = 0;
    for spec in bundled().specs() {
        let models: Vec<String> = spec
            .parameters
            .iter()
            .filter(|p| p.name.ends_with("_3dmodel_filename"))
            .map(|p| p.name.to_owned())
            .collect();
        if models.is_empty() {
            continue;
        }
        let mut parameters = serde_json::Map::new();
        for name in &models {
            parameters.insert(
                name.clone(),
                json!(format!("/tmp/model folder/{name}.step")),
            );
        }
        let output = forms(spec.source, Value::Object(parameters))
            .unwrap_or_else(|e| panic!("{}: {e}", spec.source));
        let nodes = model_nodes(&output);
        assert!(!nodes.is_empty(), "{}: no models emitted", spec.source);
        for model in nodes {
            assert!(
                matches!(&model[1], Expr::Str(path) if path.starts_with("/tmp/model folder/") && path.ends_with(".step")),
                "{}: filename must be one quoted token, got {:?}",
                spec.source,
                model[1]
            );
            checked += 1;
        }
    }
    assert!(checked >= 30, "only {checked} model paths checked");
}

#[test]
fn the_catalogue_lists_every_bundled_generator_with_shape() {
    let registry = bundled();
    let entries = registry.catalogue().unwrap();
    assert_eq!(entries.len(), 36);
    assert!(
        !entries
            .iter()
            .any(|e| e.generator.source.contains("nice_nano_pretty")),
        "the retired controller stays out of the catalogue"
    );
    assert!(
        entries
            .iter()
            .all(|e| registry.contains(&e.generator.source))
    );
    for entry in entries
        .iter()
        .filter(|e| e.kind != boardstudio_footprints::types::PartKind::Utility)
    {
        assert!(
            entry.courtyard.len() >= 3,
            "{} must have a physical outline envelope",
            entry.id
        );
    }
    for source in [
        "ceoloide/switch_mx",
        "ceoloide/switch_choc_v1_v2",
        "ceoloide/switch_gateron_ks27_ks33",
    ] {
        let entry = entries
            .iter()
            .find(|e| e.generator.source == source)
            .unwrap();
        assert!(
            entry.keycap.is_some(),
            "{source} must expose its keycap dimensions"
        );
        assert_eq!(
            default_of(source, "include_keycap"),
            json!(true),
            "{source} shows its keycap by default"
        );
        assert!(
            !entry.terminals["from"].is_empty() && !entry.terminals["to"].is_empty(),
            "{source} exposes its terminal pad groups"
        );
        let matrix = entry.matrix_terminals.as_ref().unwrap();
        assert_eq!(
            (matrix.row.as_str(), matrix.column.as_str()),
            ("from", "to")
        );
    }
    // Every generator accepts the layer variant parameter.
    for spec in registry.specs() {
        assert!(
            spec.parameters.iter().any(|p| p.name == "side"),
            "{} lost its side parameter",
            spec.source
        );
    }
}

#[test]
fn every_generator_renders_in_front_and_back_poses() {
    let registry = bundled();
    for entry in registry.catalogue().unwrap() {
        let source = entry.generator.source.as_str();
        let utility = source.contains("/utility_")
            || source.contains("/text")
            || source.contains("/pads")
            || source.contains("point_debugger");
        for side in [Side::Front, Side::Back] {
            let placed = PartRef {
                pose: Pose2 {
                    at: Vec2 { x: 12.0, y: -7.0 },
                    rotation: 23.0,
                },
                ..part(side.clone(), 0.0)
            };
            let mut nets = NetAllocator::new(&[], 1);
            let mut parameters = serde_json::Map::new();
            if side == Side::Back {
                parameters.insert("side".into(), json!("B"));
            }
            let output = registry
                .render(
                    &entry.id,
                    &generator(source, Value::Object(parameters)),
                    Some(&placed),
                    &mut nets,
                )
                .unwrap_or_else(|e| panic!("{source}: {e}"));
            assert!(!output.is_empty() || utility, "{source} produced no forms");
            if !output.is_empty() {
                boardstudio_footprints::geometry::geometry(&output)
                    .unwrap_or_else(|e| panic!("{source} geometry: {e}"));
            }
        }
    }
}

#[test]
fn authored_envelopes_and_generated_keycaps() {
    use boardstudio_footprints::definition::DefinitionInput;
    let registry = bundled();
    let entry = registry
        .catalogue()
        .unwrap()
        .into_iter()
        .find(|e| e.generator.source == "ceoloide/switch_mx")
        .unwrap();
    let mut value = serde_json::to_value(&entry).unwrap();
    value["generator"]["parameters"] = json!({ "keycap_width": 23 });
    let resized: DefinitionInput = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        registry
            .normalize(&resized)
            .unwrap()
            .unwrap()
            .keycap
            .unwrap()
            .x,
        23.0,
        "generated keycap dimensions follow generator values"
    );
    let courtyard = json!([{ "x": -20, "y": -19 }, { "x": 20, "y": -19 }, { "x": 20, "y": 19 }, { "x": -20, "y": 19 }]);
    value["courtyard"] = courtyard.clone();
    value["keycap"] = json!({ "x": 22, "y": 21 });
    value["envelopeSource"] = json!({ "courtyard": "authored", "keycap": "authored" });
    value["generator"]["parameters"] = json!({ "keycap_width": 25 });
    let authored: DefinitionInput = serde_json::from_value(value).unwrap();
    let normalized = registry.normalize(&authored).unwrap().unwrap();
    common::assert_json_eq(
        &serde_json::to_value(&normalized.courtyard).unwrap(),
        &courtyard,
        "authored courtyard is preserved",
    );
    common::assert_json_eq(
        &serde_json::to_value(normalized.keycap).unwrap(),
        &json!({ "x": 22, "y": 21 }),
        "authored keycap envelope is preserved",
    );
}

/// Jumper local nets follow stable part identity, never the editable reference.
#[test]
fn local_nets_follow_part_identity() {
    let local_names = |id: &str, reference: &str| -> Vec<String> {
        let mut placed = part(Side::Front, 0.0);
        placed.id = id.into();
        placed.reference = reference.into();
        placed.generator_parameters =
            Some(serde_json::from_value(json!({ "reversible": true })).unwrap());
        let mut nets = NetAllocator::new(&[], 1);
        bundled()
            .render(
                "definition",
                &generator("ceoloide/mcu_nice_nano", json!({})),
                Some(&placed),
                &mut nets,
            )
            .unwrap();
        let mut names: Vec<String> = nets
            .snapshot()
            .into_iter()
            .map(|n| n.name)
            .filter(|n| n.starts_with("__boardstudio_local_"))
            .collect();
        names.sort();
        names
    };
    let stable = local_names("mcu/left", "U1");
    assert!(
        stable.len() >= 24,
        "reversible MCU has distinct local socket nets"
    );
    assert_eq!(
        stable,
        local_names("mcu/left", "U99"),
        "renaming a part preserves local nets"
    );
    assert!(
        local_names("mcu/right", "U1")
            .iter()
            .all(|name| !stable.contains(name)),
        "same reference on different parts cannot join local nets"
    );
}
