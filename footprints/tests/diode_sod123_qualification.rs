//! Qualification of the explicitly selected Vishay 1N4148W SOD-123 land pattern.
//! The selected left-land height and centered origin are BoardStudio choices.
use boardstudio_footprints::geometry::geometry;
use boardstudio_footprints::models::model_bindings;
use boardstudio_footprints::nets::NetAllocator;
use boardstudio_footprints::sexpr::{Expr, child, children};
use boardstudio_footprints::types::{GeneratorRef, PartRef};
use boardstudio_footprints::{GeneratorError, bundled};
use serde_json::{Value, json};

const SOURCE: &str = "ceoloide/diode_tht_sod123";
const FLAG: &str = "use_vishay_1n4148w_land_pattern";
const EPSILON: f64 = 1e-9;

fn generator(parameters: Value) -> GeneratorRef {
    GeneratorRef {
        source: SOURCE.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).expect("generator parameters"),
    }
}

fn part(face: &str, rotation: f64, parameters: Value) -> PartRef {
    serde_json::from_value(json!({
        "id": "diode-1",
        "reference": "D1",
        "pose": {"at": {"x": 12.5, "y": -4.25}, "rotation": rotation},
        "side": face.to_lowercase(),
        "generatorParameters": parameters,
    }))
    .expect("part reference")
}

fn render(parameters: Value, placed: Option<&PartRef>) -> Result<Vec<Expr>, GeneratorError> {
    let registry = bundled();
    let mut nets = NetAllocator::new(&[], 1);
    registry.render("diode-1", &generator(parameters), placed, &mut nets)
}

fn footprint(forms: &[Expr]) -> &[Expr] {
    forms
        .iter()
        .find_map(|form| {
            let items = form.as_list()?;
            matches!(items.first(), Some(Expr::Atom(name)) if name == "footprint").then_some(items)
        })
        .expect("rendered footprint")
}

fn number(node: Option<&Expr>) -> f64 {
    node.and_then(|node| node.value().ok())
        .expect("numeric atom")
        .parse()
        .expect("valid number")
}

fn point(node: &[Expr]) -> (f64, f64) {
    (number(node.get(1)), number(node.get(2)))
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "{actual} differs from {expected}"
    );
}

fn qualified_parameters(face: &str) -> Value {
    json!({
        "side": face,
        "use_vishay_1n4148w_land_pattern": true,
        "to": "K_CATHODE",
        "from": "A_ANODE",
    })
}

fn assert_qualified_lands(forms: &[Expr], face: &str, rotation: f64) {
    let root = footprint(forms);
    let pads: Vec<_> = children(root, "pad").collect();
    assert_eq!(pads.len(), 2, "only the two ordinary SMD lands are present");

    let measured = geometry(forms).expect("land geometry");
    assert_eq!(measured.pads.len(), 2);
    let cathode = &measured.pads[0];
    let anode = &measured.pads[1];
    assert_eq!((cathode.number.as_str(), anode.number.as_str()), ("1", "2"));
    // The two width callouts and gap determine centers around the chosen origin.
    let cathode_width = 0.85;
    let anode_width = 0.85;
    let inner_gap = 2.5;
    close(cathode.at.x, -(inner_gap + cathode_width) / 2.0);
    close(cathode.at.y, 0.0);
    close(anode.at.x, (inner_gap + anode_width) / 2.0);
    close(anode.at.y, 0.0);
    close(cathode.size.x, cathode_width);
    close(anode.size.x, anode_width);
    close(anode.size.y, 0.85); // Manufacturer-dimensioned right-land height.
    close(cathode.size.y, 0.85); // Configured left-land height.
    close(
        (anode.at.x - anode.size.x / 2.0) - (cathode.at.x + cathode.size.x / 2.0),
        inner_gap,
    );
    close(
        (anode.at.x + anode.size.x / 2.0) - (cathode.at.x - cathode.size.x / 2.0),
        4.2,
    );

    let expected_layers = [
        format!("{face}.Cu"),
        format!("{face}.Paste"),
        format!("{face}.Mask"),
    ];
    for &pad in &pads {
        close(number(child(pad, "at").unwrap().get(3)), rotation);
        assert_eq!(pad.get(2).and_then(|value| value.value().ok()), Some("smd"));
        assert_eq!(
            pad.get(3).and_then(|value| value.value().ok()),
            Some("rect")
        );
        let layers = child(pad, "layers").expect("pad layers");
        let actual: Vec<&str> = layers
            .iter()
            .skip(1)
            .map(|layer| layer.value().expect("layer name"))
            .collect();
        assert_eq!(actual, expected_layers);
    }
    assert_eq!(
        cathode.net_id, None,
        "geometry extraction does not assign document net IDs"
    );

    let root_at = child(root, "at").expect("root pose");
    close(number(root_at.get(1)), 12.5);
    close(number(root_at.get(2)), 4.25);

    // Identify the existing cathode bar from its geometry before considering nets.
    let bars: Vec<_> = children(root, "fp_line")
        .filter(|line| {
            let layer = child(line, "layer")
                .and_then(|entry| entry.get(1))
                .and_then(|value| value.value().ok());
            let Some(start) = child(line, "start") else {
                return false;
            };
            let Some(end) = child(line, "end") else {
                return false;
            };
            point(start) == (-0.35, 0.0)
                && (point(end) == (-0.35, 0.55) || point(end) == (-0.35, -0.55))
                && layer == Some(if face == "F" { "F.SilkS" } else { "B.SilkS" })
        })
        .collect();
    assert_eq!(bars.len(), 2, "the two existing cathode-bar strokes remain");
    for endpoint in [(-0.35, 0.55), (-0.35, -0.55)] {
        assert!(
            bars.iter()
                .any(|line| point(child(line, "end").unwrap()) == endpoint),
            "missing cathode-bar endpoint {endpoint:?}"
        );
    }
    for line in &bars {
        let stroke = child(line, "stroke").expect("line stroke");
        close(
            number(child(stroke, "width").and_then(|width| width.get(1))),
            0.1,
        );
    }

    // Local bar midpoint is nearer pad 1/K than pad 2/A, independent of net data.
    let midpoint = -0.35;
    close((midpoint - cathode.at.x).abs(), 1.325);
    close((midpoint - anode.at.x).abs(), 2.025);

    // Transform the fixed local bar endpoints independently at each root pose.
    let angle = rotation.to_radians();
    let transform = |x: f64, y: f64| {
        (
            12.5 + x * angle.cos() + y * angle.sin(),
            -4.25 - x * angle.sin() + y * angle.cos(),
        )
    };
    for line in &bars {
        let start = point(child(line, "start").unwrap());
        let end = point(child(line, "end").unwrap());
        let actual_start = transform(start.0, -start.1);
        let actual_end = transform(end.0, -end.1);
        let expected_start = transform(-0.35, 0.0);
        let expected_end = if end.1 > 0.0 {
            transform(-0.35, -0.55)
        } else {
            transform(-0.35, 0.55)
        };
        close(actual_start.0, expected_start.0);
        close(actual_start.1, expected_start.1);
        close(actual_end.0, expected_end.0);
        close(actual_end.1, expected_end.1);
    }

    // Electrical identity is checked separately from locating the bar.
    for (pad, expected_name, expected_index) in
        [(pads[0], "K_CATHODE", 2.0), (pads[1], "A_ANODE", 1.0)]
    {
        let net = child(pad, "net").expect("assigned net");
        assert_eq!(
            net.get(2).and_then(|value| value.value().ok()),
            Some(expected_name)
        );
        close(number(net.get(1)), expected_index);
    }
}

#[test]
fn opt_in_lands_match_vishay_callouts_and_fixed_pose_oracles() {
    for (face, part_face) in [("F", "front"), ("B", "back")] {
        for (rotation, expected_vector) in [
            (0.0, (3.35, 0.0)),
            (90.0, (0.0, -3.35)),
            (120.0, (-1.675, -2.901185102678)),
        ] {
            let parameters = qualified_parameters(face);
            let placed = part(part_face, rotation, Value::Null);
            let forms = render(parameters, Some(&placed)).expect("opt-in render");
            assert_qualified_lands(&forms, face, rotation);

            let measured = geometry(&forms).expect("land geometry");
            let cathode = measured.pads[0].at;
            let anode = measured.pads[1].at;
            let angle = rotation.to_radians();
            let world = |point: boardstudio_footprints::types::Vec2| {
                (
                    12.5 + point.x * angle.cos() + point.y * angle.sin(),
                    -4.25 - point.x * angle.sin() + point.y * angle.cos(),
                )
            };
            let k = world(cathode);
            let a = world(anode);
            close(a.0 - k.0, expected_vector.0);
            close(a.1 - k.1, expected_vector.1);
            close(
                number(child(footprint(&forms), "at").unwrap().get(3)),
                rotation,
            );
        }
    }
}

#[test]
fn opt_in_requires_a_front_or_back_nonreversible_smd_configuration() {
    for (name, value) in [
        ("side", json!("X")),
        ("reversible", json!(true)),
        ("include_tht", json!(true)),
        ("include_thru_hole_smd_pads", json!(true)),
        ("include_traces_vias", json!(true)),
    ] {
        let mut parameters = qualified_parameters("F");
        parameters[name] = value;
        let error = render(parameters, None).expect_err(name);
        assert!(
            matches!(error, GeneratorError::Rejected { ref parameter, .. } if parameter.as_deref() == Some(FLAG))
        );
        assert!(error.message().contains("requires side F or B"));
    }

    let error = render(
        json!({
            "side": "B",
            "reversible": true,
            "include_tht": true,
            "include_thru_hole_smd_pads": true,
            "include_traces_vias": true,
            "use_vishay_1n4148w_land_pattern": true,
        }),
        None,
    )
    .expect_err("combined unsupported settings");
    assert!(
        matches!(error, GeneratorError::Rejected { ref parameter, .. } if parameter.as_deref() == Some(FLAG))
    );
}

#[test]
fn absent_null_false_and_malformed_side_keep_legacy_rendering() {
    let absent = render(json!({ "side": "F" }), None).expect("absent flag");
    let null = render(
        json!({ "side": "F", "use_vishay_1n4148w_land_pattern": null }),
        None,
    )
    .expect("null flag");
    let false_flag = render(
        json!({ "side": "F", "use_vishay_1n4148w_land_pattern": false }),
        None,
    )
    .expect("false flag");
    assert_eq!(absent, null);
    assert_eq!(absent, false_flag);

    let malformed_absent = render(json!({ "side": "X" }), None).expect("legacy malformed side");
    let malformed_false = render(
        json!({ "side": "X", "use_vishay_1n4148w_land_pattern": false }),
        None,
    )
    .expect("false malformed side");
    assert_eq!(malformed_absent, malformed_false);
    assert!(
        children(footprint(&malformed_absent), "pad")
            .next()
            .is_none()
    );
    assert_eq!(
        render(
            json!({ "side": "X", "use_vishay_1n4148w_land_pattern": true }),
            None
        )
        .unwrap_err()
        .message(),
        "diode_tht_sod123: use_vishay_1n4148w_land_pattern requires side F or B, reversible: false, include_tht: false, include_thru_hole_smd_pads: false, and include_traces_vias: false"
    );

    let wrong_flag = render(json!({ FLAG: "true" }), None).unwrap_err();
    assert_eq!(
        wrong_flag.message(),
        format!("{SOURCE}: parameter {FLAG} must be true or false.")
    );
    let wrong_side = render(json!({ "side": 1 }), None).unwrap_err();
    assert_eq!(
        wrong_side.message(),
        format!("{SOURCE}: parameter side must be text.")
    );
    let old_guard = render(
        json!({
            "include_thru_hole_smd_pads": true,
            "use_vishay_1n4148w_land_pattern": false,
        }),
        None,
    )
    .unwrap_err();
    assert_eq!(
        old_guard.message(),
        "diode_tht_sod123: include_thru_hole_smd_pads requires reversible: true"
    );
}

#[test]
fn part_null_overrides_definition_true_with_the_declaration_default() {
    let placed = part(
        "front",
        0.0,
        json!({ "use_vishay_1n4148w_land_pattern": null }),
    );
    let legacy_part = part("front", 0.0, Value::Null);
    let legacy = render(json!({ "side": "F" }), Some(&legacy_part)).expect("legacy definition");
    let forms = render(
        json!({ "side": "F", "use_vishay_1n4148w_land_pattern": true }),
        Some(&placed),
    )
    .expect("part null selects declaration default");
    assert_eq!(forms, legacy);
}

#[test]
fn model_binding_keeps_emitted_defaults_overrides_and_empty_omission() {
    let defaults = render(qualified_parameters("F"), None).expect("default model");
    assert_eq!(
        child(footprint(&defaults), "model")
            .unwrap()
            .get(1)
            .unwrap()
            .value()
            .unwrap(),
        "${KIPRJMOD}/models/boardstudio/infused-kim/Diode_1N4148W.step"
    );
    let bindings = model_bindings(&defaults).expect("model binding");
    assert_eq!(bindings.len(), 1);
    assert_eq!(
        bindings[0].asset_id,
        "bundled-model:infused-kim/Diode_1N4148W.step"
    );
    assert_eq!(bindings[0].offset.x, 0.0);
    assert_eq!(bindings[0].offset.y, 0.0);
    assert_eq!(bindings[0].offset.z, 0.7);
    assert_eq!(bindings[0].rotation.x, -90.0);
    assert_eq!(bindings[0].rotation.y, 0.0);
    assert_eq!(bindings[0].rotation.z, 0.0);
    assert_eq!(
        (
            bindings[0].scale.x,
            bindings[0].scale.y,
            bindings[0].scale.z
        ),
        (1.0, 1.0, 1.0)
    );

    let overridden = render(
        json!({
            "side": "F",
            "use_vishay_1n4148w_land_pattern": true,
            "diode_3dmodel_filename": "boardstudio-asset:ticket131-qualified-model",
            "diode_3dmodel_xyz_offset": [1, 2, 3],
            "diode_3dmodel_xyz_rotation": [4, 5, 6],
            "diode_3dmodel_xyz_scale": [0.5, 0.75, 1.25],
        }),
        None,
    )
    .expect("model overrides");
    let bindings = model_bindings(&overridden).expect("overridden model binding");
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].asset_id, "ticket131-qualified-model");
    assert_eq!(bindings[0].offset.x, 1.0);
    assert_eq!(bindings[0].offset.y, -2.0);
    assert_eq!(bindings[0].offset.z, 3.0);
    assert_eq!(bindings[0].rotation.x, 4.0);
    assert_eq!(bindings[0].rotation.y, 5.0);
    assert_eq!(bindings[0].rotation.z, -6.0);
    assert_eq!(
        (
            bindings[0].scale.x,
            bindings[0].scale.y,
            bindings[0].scale.z
        ),
        (0.5, 0.75, 1.25)
    );

    let omitted = render(
        json!({
            "side": "F",
            "use_vishay_1n4148w_land_pattern": true,
            "diode_3dmodel_filename": "",
        }),
        None,
    )
    .expect("empty model path");
    assert!(model_bindings(&omitted).unwrap().is_empty());
}

#[test]
fn schema_exposes_the_opt_in_as_a_default_false_boolean() {
    let schema = bundled().parameters(SOURCE).expect("diode schema");
    assert_eq!(schema.len(), 17);
    assert_eq!(
        serde_json::to_value(schema.last().unwrap()).unwrap(),
        json!({"name": FLAG, "type": "boolean", "value": false})
    );
}
