use boardstudio_footprints::{
    generators::bundled,
    nets::NetAllocator,
    sexpr::{self, Expr},
    types::{GeneratorRef, PartRef, Pose2, Side, Vec2},
};
use serde_json::{Value, json};

const SOURCE: &str = "ceoloide/reset_switch_smd_side";
const FROM_NET: &str = "RESET_FROM_QUAL";
const TO_NET: &str = "RESET_TO_QUAL";
const EPSILON_MM: f64 = 1.0e-6;
const LAND_SPAN_TOLERANCE_MM: f64 = 0.1;
const LAND_GAP_TOLERANCE_MM: f64 = 0.1;
const BOSS_CENTER_TOLERANCE_MM: f64 = 0.05;
const LOWER_BOSS_DRILL_MIN_MM: f64 = 0.75;
const LOWER_BOSS_DRILL_MAX_MM: f64 = 0.85;

#[derive(Clone, Debug)]
struct Pad<'a> {
    items: &'a [Expr],
    number: &'a str,
    kind: &'a str,
    shape: &'a str,
    x: f64,
    y: f64,
    angle: f64,
    width: f64,
    height: f64,
}

fn render(side: &str, part_side: Side, at: Vec2, rotation: f64, overrides: Value) -> Vec<Expr> {
    let mut parameters = json!({
        "side": side,
        "designator": "SW_RST_QUAL",
        "reversible": false,
        "include_bosses": true,
        "include_silkscreen": false,
        "include_courtyard": false,
        "from": FROM_NET,
        "to": TO_NET,
    })
    .as_object()
    .expect("fixture parameters are an object")
    .clone();
    if let Some(overrides) = overrides.as_object() {
        parameters.extend(overrides.clone());
    }

    let generator = GeneratorRef {
        source: SOURCE.into(),
        version: "bundled-1".into(),
        parameters: parameters.into_iter().collect(),
    };
    let part = PartRef {
        id: "reset-switch-side-qualification".into(),
        reference: "SW_RST_QUAL".into(),
        pose: Pose2 { at, rotation },
        side: part_side,
        generator_parameters: None,
    };
    let mut nets = NetAllocator::new(&[], 1);
    bundled()
        .render(
            "reset-switch-side-qualification",
            &generator,
            Some(&part),
            &mut nets,
        )
        .expect("the bundled side-reset switch renders")
}

fn named<'a>(items: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    items.iter().find_map(|item| {
        let list = item.as_list()?;
        (list.first()?.value().ok()? == name).then_some(list)
    })
}

fn scalar(items: &[Expr], index: usize) -> &str {
    items
        .get(index)
        .unwrap_or_else(|| panic!("missing item {index} in {items:?}"))
        .value()
        .unwrap_or_else(|_| panic!("item {index} is not scalar in {items:?}"))
}

fn number(items: &[Expr], index: usize) -> f64 {
    scalar(items, index)
        .parse()
        .unwrap_or_else(|_| panic!("item {index} is not a number in {items:?}"))
}

fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= EPSILON_MM,
        "{context}: expected {expected}, found {actual} (numeric epsilon {EPSILON_MM} mm)"
    );
}

fn form<'a>(forms: &'a [Expr], name: &str) -> &'a [Expr] {
    named(forms, name).unwrap_or_else(|| panic!("missing {name} form"))
}

fn pads(footprint: &[Expr]) -> Vec<Pad<'_>> {
    sexpr::children(footprint, "pad")
        .map(|items| {
            let at = sexpr::child(items, "at").expect("pad has an at field");
            let size = sexpr::child(items, "size").expect("pad has a size field");
            Pad {
                items,
                number: scalar(items, 1),
                kind: scalar(items, 2),
                shape: scalar(items, 3),
                x: number(at, 1),
                y: number(at, 2),
                angle: number(at, 3),
                width: number(size, 1),
                height: number(size, 2),
            }
        })
        .collect()
}

fn pad_number<'a>(pads: &'a [Pad<'a>], number: &str) -> &'a Pad<'a> {
    pads.iter()
        .find(|pad| pad.number == number)
        .unwrap_or_else(|| panic!("missing numbered pad {number}"))
}

fn layer_names<'a>(pad: &'a Pad<'_>) -> Vec<&'a str> {
    sexpr::child(pad.items, "layers")
        .expect("pad has a layers field")
        .iter()
        .skip(1)
        .map(|layer| layer.value().expect("layer name is scalar"))
        .collect()
}

fn net_name<'a>(pad: &'a Pad<'_>) -> Option<&'a str> {
    sexpr::child(pad.items, "net").map(|net| scalar(net, 2))
}

fn assert_land_pattern(pads: &[Pad<'_>]) {
    let numbered: Vec<_> = pads.iter().filter(|pad| !pad.number.is_empty()).collect();
    assert_eq!(numbered.len(), 4, "four numbered terminals are emitted");
    assert!(
        numbered
            .iter()
            .all(|pad| pad.kind == "smd" && pad.shape == "rect")
    );
    assert!(numbered.iter().all(|pad| {
        (pad.width - 1.55).abs() <= EPSILON_MM && (pad.height - 1.0).abs() <= EPSILON_MM
    }));

    let left_x = numbered
        .iter()
        .map(|pad| pad.x)
        .fold(f64::INFINITY, f64::min);
    let right_x = numbered
        .iter()
        .map(|pad| pad.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let outside_span = (right_x + 1.55 / 2.0) - (left_x - 1.55 / 2.0);
    let inner_gap = (right_x - 1.55 / 2.0) - (left_x + 1.55 / 2.0);

    let left_column: Vec<_> = numbered
        .iter()
        .filter(|pad| (pad.x - left_x).abs() <= EPSILON_MM)
        .collect();
    let right_column: Vec<_> = numbered
        .iter()
        .filter(|pad| (pad.x - right_x).abs() <= EPSILON_MM)
        .collect();
    assert_eq!(left_column.len(), 2);
    assert_eq!(right_column.len(), 2);
    let left_row_gap = (left_column[0].y - left_column[1].y).abs() - 1.0;
    let right_overall = (right_column[0].y - right_column[1].y).abs() + 1.0;

    // Panasonic's reference land pattern labels 6.8 ± 0.1, 3.7 ± 0.1,
    // 0.7 ± 0.1 and 2.7 ± 0.1 mm respectively. These explicit source
    // tolerances are manufacturing intervals; EPSILON_MM below is only
    // floating point comparison epsilon and is deliberately much tighter.
    for (actual, nominal, source_tolerance, label) in [
        (
            outside_span,
            6.8,
            LAND_SPAN_TOLERANCE_MM,
            "outside pad span",
        ),
        (inner_gap, 3.7, LAND_GAP_TOLERANCE_MM, "inner pad gap"),
        (
            left_row_gap,
            0.7,
            LAND_GAP_TOLERANCE_MM,
            "left-column row gap",
        ),
        (
            right_overall,
            2.7,
            LAND_GAP_TOLERANCE_MM,
            "right-column overall span",
        ),
    ] {
        assert!(
            (actual - nominal).abs() <= source_tolerance,
            "{label} outside Panasonic tolerance"
        );
        close(actual, nominal, label);
    }
}

fn expected_upper_to_lower(rotation: f64) -> (f64, f64) {
    match rotation as i32 {
        0 => (0.0, 2.75),
        90 => (2.75, 0.0),
        120 => (2.381_569_860_407, -1.375),
        _ => panic!("unexpected qualification rotation {rotation}"),
    }
}

fn place(local_x: f64, local_y: f64, at: Vec2, rotation: f64) -> (f64, f64) {
    let radians = rotation.to_radians();
    let (sin, cos) = radians.sin_cos();
    // KiCad's down-Y pose: (x, -y, rotation). Apply this placement once.
    (
        at.x + local_x * cos + local_y * sin,
        -at.y - local_x * sin + local_y * cos,
    )
}

fn assert_case(side: &str, part_side: Side, rotation: f64) {
    let pose = Vec2 { x: 17.25, y: -8.5 };
    let forms = render(side, part_side, pose, rotation, Value::Null);
    let footprint = form(&forms, "footprint");
    let all_pads = pads(footprint);
    assert_eq!(all_pads.len(), 6, "four SMD terminals and two boss holes");
    assert_land_pattern(&all_pads);

    let face = if side == "F" { "F" } else { "B" };
    let expected_layers = [
        format!("{face}.Cu"),
        format!("{face}.Paste"),
        format!("{face}.Mask"),
    ];
    for number in ["1", "2", "3", "4"] {
        let pad = pad_number(&all_pads, number);
        assert_eq!(pad.kind, "smd");
        assert_eq!(pad.shape, "rect");
        assert_eq!(layer_names(pad), expected_layers);
        let expected_net = if number == "1" || number == "3" {
            FROM_NET
        } else {
            TO_NET
        };
        assert_eq!(
            net_name(pad),
            Some(expected_net),
            "terminal {number} contact group"
        );
        close(pad.angle, 180.0 + rotation, "terminal pad angle");

        let is_right = if side == "F" {
            number == "1" || number == "2"
        } else {
            number == "3" || number == "4"
        };
        let is_upper = number == "1" || number == "3";
        close(
            pad.x,
            if is_right { 2.625 } else { -2.625 },
            "terminal x configuration",
        );
        close(
            pad.y,
            if is_upper { -0.85 } else { 0.85 },
            "terminal y configuration",
        );
    }

    // Identify unnumbered NPTH holes by their emitted semantic/type fields,
    // never by a drill-size filter. The lower hole is the one at positive
    // local Y in the generator's source view.
    let holes: Vec<_> = all_pads
        .iter()
        .filter(|pad| pad.number.is_empty() && pad.kind == "np_thru_hole" && pad.shape == "circle")
        .collect();
    assert_eq!(holes.len(), 2, "two unnumbered circular NPTH boss holes");
    assert_eq!(layer_names(holes[0]), ["*.Cu", "*.Mask"]);
    assert_eq!(layer_names(holes[1]), ["*.Cu", "*.Mask"]);
    assert!(holes.iter().all(|hole| net_name(hole).is_none()));

    let upper = holes
        .iter()
        .min_by(|left, right| left.y.total_cmp(&right.y))
        .expect("upper boss hole exists");
    let lower = holes
        .iter()
        .max_by(|left, right| left.y.total_cmp(&right.y))
        .expect("lower boss hole exists");
    close(upper.x, 0.0, "upper boss hole x configuration");
    close(upper.y, -1.375, "upper boss hole y configuration");
    close(lower.x, 0.0, "lower boss hole x configuration");
    close(lower.y, 1.375, "lower boss hole y configuration");
    let upper_drill = number(
        sexpr::child(upper.items, "drill").expect("upper hole drill"),
        1,
    );
    let lower_drill = number(
        sexpr::child(lower.items, "drill").expect("lower hole drill"),
        1,
    );
    close(upper_drill, 0.75, "upper boss drill producer configuration");
    assert!(
        (LOWER_BOSS_DRILL_MIN_MM..=LOWER_BOSS_DRILL_MAX_MM).contains(&lower_drill),
        "Panasonic labels only the lower-hole drill Ø0.75 +0.1/−0 mm; found {lower_drill} mm"
    );
    assert!(
        (lower_drill - 0.75).abs() <= EPSILON_MM,
        "nominal lower boss drill"
    );

    let center_spacing = (lower.y - upper.y).abs();
    assert!(
        (center_spacing - 2.75).abs() <= BOSS_CENTER_TOLERANCE_MM,
        "Panasonic labels boss-hole center spacing 2.75 ±0.05 mm; found {center_spacing} mm"
    );
    close(center_spacing, 2.75, "nominal boss-hole center spacing");

    // Independently transform the fixed manufacturer-labelled upper-to-lower
    // vector with the pose convention. The source coordinates above are fixed
    // expectations; no generated-coordinate helper supplies the oracle.
    let expected_vector = expected_upper_to_lower(rotation);
    let placed_upper = place(0.0, -1.375, pose, rotation);
    let placed_lower = place(0.0, 1.375, pose, rotation);
    close(
        placed_upper.0,
        pose.x - expected_vector.0 / 2.0,
        "placed upper boss x",
    );
    close(
        placed_upper.1,
        -pose.y - expected_vector.1 / 2.0,
        "placed upper boss y",
    );
    close(
        placed_lower.0 - placed_upper.0,
        expected_vector.0,
        "placed boss vector x",
    );
    close(
        placed_lower.1 - placed_upper.1,
        expected_vector.1,
        "placed boss vector y",
    );

    let footprint_at = sexpr::child(footprint, "at").expect("footprint placement exists");
    close(number(footprint_at, 1), pose.x, "footprint pose x");
    close(number(footprint_at, 2), -pose.y, "footprint down-Y pose y");
    close(number(footprint_at, 3), rotation, "footprint pose rotation");
}

fn vec3(parent: &[Expr], name: &str) -> [f64; 3] {
    let vector = sexpr::child(parent, name).expect("model vector exists");
    let xyz = sexpr::child(vector, "xyz").expect("model vector has xyz values");
    [number(xyz, 1), number(xyz, 2), number(xyz, 3)]
}

fn model(footprint: &[Expr]) -> &[Expr] {
    let models: Vec<_> = sexpr::children(footprint, "model").collect();
    assert_eq!(models.len(), 1, "exactly one model binding is emitted");
    models[0]
}

#[test]
fn land_pattern_contacts_layers_bosses_and_poses_match_independent_oracle() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        for rotation in [0.0, 90.0, 120.0] {
            assert_case(side, part_side.clone(), rotation);
        }
    }
}

#[test]
fn boss_model_default_and_explicit_model_transforms_are_preserved() {
    let pose = Vec2 { x: 17.25, y: -8.5 };
    for (side, part_side, expected_rotation) in [
        ("F", Side::Front, [0.0, 0.0, 0.0]),
        ("B", Side::Back, [0.0, 0.0, 180.0]),
    ] {
        let forms = render(side, part_side, pose, 0.0, Value::Null);
        let footprint = form(&forms, "footprint");
        let model = model(footprint);
        assert_eq!(
            scalar(model, 1),
            "${KIPRJMOD}/models/boardstudio/kicad/Panasonic_EVQPUL_EVQPUC.step"
        );
        assert_eq!(vec3(model, "offset"), [0.0, 0.0, 0.0]);
        assert_eq!(vec3(model, "scale"), [1.0, 1.0, 1.0]);
        assert_eq!(vec3(model, "rotate"), expected_rotation);
    }

    let overrides = json!({
        "reset_switch_3dmodel_filename": "custom/reset-switch.step",
        "reset_switch_3dmodel_xyz_offset": [1.25, -2.5, 3.75],
        "reset_switch_3dmodel_xyz_rotation": [10.0, 20.0, 30.0],
        "reset_switch_3dmodel_xyz_scale": [1.2, 0.8, 1.1],
    });
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        let forms = render(side, part_side, pose, 0.0, overrides.clone());
        let model = model(form(&forms, "footprint"));
        assert_eq!(scalar(model, 1), "custom/reset-switch.step");
        assert_eq!(vec3(model, "offset"), [1.25, -2.5, 3.75]);
        assert_eq!(vec3(model, "scale"), [1.2, 0.8, 1.1]);
        assert_eq!(vec3(model, "rotate"), [10.0, 20.0, 30.0]);
    }
}
