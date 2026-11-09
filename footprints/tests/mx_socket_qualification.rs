//! Public-boundary qualification of the existing Kailh MX hot-swap layout.
//!
//! Independent source: Kailh CPG151101S11-16, drawing KHA-PG1511-388EN Rev A
//! (cover recommended PCB layout) and specification KH-PS2206-43 Rev A,
//! printed page 8/9, section 9. Dimensions are millimetres. The drawing gives
//! a 6.35 mm by 2.54 mm hole-centre vector, Ø3.00 mm drills, and parenthesized
//! rectangular-land reference dimensions (2.55) by (2.5) mm. A recommended-
//! layout tolerance note of ±0.05 mm appears directly beneath that figure.
//! The layout labels the rectangular land dimensions as references; the nearby
//! ±0.05 mm note is not treated as an acceptance tolerance for those references.
//! This fixture selects the manufacturer nominal widths explicitly. The
//! generator's default 2.6 mm outer widths remain unchanged and are not
//! qualified against the 2.55 mm reference width here. The drawing does not
//! dimension the socket datum relative to the MX switch origin or label the
//! host lands with an electrical terminal number.

use boardstudio_footprints::bundled;
use boardstudio_footprints::nets::NetAllocator;
use boardstudio_footprints::types::{GeneratorRef, PartRef, Pose2, Side, Vec2};
use serde_json::json;

const SOURCE: &str = "ceoloide/switch_mx";
const SOCKET_MODEL: &str = "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Hotswap_Kailh_MX.stp";
const NUMERIC_EPSILON: f64 = 1e-6;
const PCB_THICKNESS_MM: f64 = 1.6;
const SOCKET_MODEL_Z_DATUM_MM: f64 = 0.27094;

fn render(side: &str, part_side: Side, rotation: f64) -> String {
    render_at(side, part_side, Vec2 { x: 0.0, y: 0.0 }, rotation)
}

fn render_at(side: &str, part_side: Side, at: Vec2, rotation: f64) -> String {
    let parameters = json!({
        "side": side,
        "hotswap": true,
        "reversible": false,
        "pcb_thickness": PCB_THICKNESS_MM,
        "outer_pad_width_front": 2.55,
        "outer_pad_width_back": 2.55,
        "from": "MX_ROW_QUAL",
        "to": "MX_COL_QUAL"
    });
    let generator = GeneratorRef {
        source: SOURCE.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).unwrap(),
    };
    let part = PartRef {
        id: "mx-socket-qualification".into(),
        reference: "S1".into(),
        pose: Pose2 { at, rotation },
        side: part_side,
        generator_parameters: None,
    };
    let mut nets = NetAllocator::new(&[], 1);
    bundled()
        .render("definition", &generator, Some(&part), &mut nets)
        .unwrap_or_else(|error| panic!("render {side} at {rotation}°: {error}"))
        .iter()
        .map(boardstudio_footprints::sexpr::serialize)
        .collect::<Vec<_>>()
        .join("\n")
}

fn blocks_named(text: &str, name: &str) -> Vec<String> {
    let marker = format!("({name} ");
    text.match_indices(&marker)
        .filter_map(|(start, _)| {
            let mut depth = 0_i32;
            let mut in_string = false;
            let mut escaped = false;
            for (relative, ch) in text[start..].char_indices() {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        in_string = false;
                    }
                    continue;
                }
                match ch {
                    '"' => in_string = true,
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(text[start..start + relative + ch.len_utf8()].to_owned());
                        }
                    }
                    _ => {}
                }
            }
            None
        })
        .collect()
}

fn numbers_in_field(block: &str, field: &str) -> Vec<f64> {
    let marker = format!("({field} ");
    let start = block
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {field}: {block}"));
    let values_start = start + marker.len();
    let values_end = block[values_start..]
        .find(')')
        .map(|end| values_start + end)
        .unwrap_or_else(|| panic!("unterminated {field}: {block}"));
    block[values_start..values_end]
        .split_whitespace()
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("non-number in {field}: {value}"))
        })
        .collect()
}

fn pad(text: &str, number: &str) -> String {
    let prefix = format!("(pad \"{number}\" smd ");
    blocks_named(text, "pad")
        .into_iter()
        .find(|block| block.starts_with(&prefix))
        .unwrap_or_else(|| panic!("missing SMD pad {number}"))
}

fn socket_holes(text: &str) -> Vec<String> {
    blocks_named(text, "pad")
        .into_iter()
        .filter(|block| {
            block.starts_with("(pad \"\" np_thru_hole ") && block.contains("\"F&B.Cu\"")
        })
        .collect()
}

fn kicad_world_delta(text: &str, local_delta: [f64; 2]) -> [f64; 2] {
    let footprint = blocks_named(text, "footprint")
        .into_iter()
        .next()
        .expect("rendered footprint");
    let placement = numbers_in_field(&footprint, "at");
    let angle = placement[2].to_radians();
    // KiCad serializes footprint rotation in its down-Y coordinate frame. This
    // applies -theta to the emitted local vector; expected values below are
    // independent, precomputed drawing vectors rather than this calculation.
    [
        local_delta[0] * angle.cos() + local_delta[1] * angle.sin(),
        -local_delta[0] * angle.sin() + local_delta[1] * angle.cos(),
    ]
}

fn assert_near(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= NUMERIC_EPSILON,
        "{context}: got {actual}, expected {expected}"
    );
}

fn hole_delta(text: &str) -> [f64; 2] {
    let holes = socket_holes(text);
    assert_eq!(holes.len(), 2, "socket hole count");
    for (index, hole) in holes.iter().enumerate() {
        let drill = numbers_in_field(hole, "drill");
        assert_eq!(drill.len(), 1, "hole {index} drill");
        assert_near(drill[0], 3.0, "manufacturer nominal hole drill");
    }
    let points = holes
        .iter()
        .map(|hole| numbers_in_field(hole, "at"))
        .collect::<Vec<_>>();
    [points[1][0] - points[0][0], points[1][1] - points[0][1]]
}

fn expected_kicad_delta(side: &str, rotation: u16) -> [f64; 2] {
    // Manufacturer layout vector transformed using KiCad's -theta/down-Y
    // convention. These table entries were computed independently of the
    // implementation transform above.
    match (side, rotation) {
        ("F", 0) => [6.35, 2.54],
        ("F", 90) => [2.54, -6.35],
        ("F", 300) => [0.975295482, 6.769261314],
        ("B", 0) => [-6.35, 2.54],
        ("B", 90) => [2.54, 6.35],
        ("B", 300) => [-5.374704518, -4.229261314],
        _ => panic!("missing independent expected vector for {side} at {rotation}°"),
    }
}

fn assert_socket_spacing(text: &str, side: &str, rotation: u16) {
    let actual = kicad_world_delta(text, hole_delta(text));
    let expected = expected_kicad_delta(side, rotation);
    assert_near(actual[0], expected[0], &format!("{side} at {rotation}° x"));
    assert_near(actual[1], expected[1], &format!("{side} at {rotation}° y"));
}

fn quoted_values_in_field(block: &str, field: &str) -> Vec<String> {
    let marker = format!("({field} ");
    let start = block
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {field}: {block}"));
    let values_start = start + marker.len();
    let values_end = block[values_start..]
        .find(')')
        .map(|end| values_start + end)
        .unwrap_or_else(|| panic!("unterminated {field}: {block}"));
    block[values_start..values_end]
        .split_whitespace()
        .filter(|value| value.starts_with('"') && value.ends_with('"'))
        .map(|value| value.trim_matches('"').to_owned())
        .collect()
}

fn xyz_numbers_in_field(block: &str, field: &str) -> Vec<f64> {
    let marker = format!("({field} (xyz ");
    let start = block
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {field} xyz vector: {block}"));
    let values_start = start + marker.len();
    let values_end = block[values_start..]
        .find(')')
        .map(|end| values_start + end)
        .unwrap_or_else(|| panic!("unterminated {field} xyz vector: {block}"));
    block[values_start..values_end]
        .split_whitespace()
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("non-number in {field}: {value}"))
        })
        .collect()
}

#[test]
fn recommended_socket_hole_pair_spacing_survives_side_and_placement_rotation() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        for rotation in [0_u16, 90, 300] {
            let text = render(side, part_side.clone(), f64::from(rotation));
            assert_socket_spacing(&text, side, rotation);
        }
    }
}

#[test]
fn selected_sofle_switch_pose_preserves_source_layout_hole_spacing() {
    // SW26 is stored at this pose in the pinned upstream Sofle v2 PCB source.
    // KiCad source y increases down the board; the public renderer serializes
    // that stored y as negative in its footprint frame. Rotation stays 300°.
    // This fixture does not claim a board-origin or heading conversion beyond
    // the observed renderer frame.
    let text = render_at("F", Side::Front, Vec2 { x: 96.7, y: 133.41 }, 300.0);
    let placement = numbers_in_field(
        &blocks_named(&text, "footprint")
            .into_iter()
            .next()
            .expect("rendered footprint"),
        "at",
    );
    assert_eq!(placement, [96.7, -133.41, 300.0]);

    let holes = socket_holes(&text);
    assert_eq!(holes.len(), 2);
    assert_socket_spacing(&text, "F", 300);
}

#[test]
fn recommended_land_sizes_match_drawing_reference_dimensions() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        let text = render(side, part_side, 0.0);
        let pad_1_size = numbers_in_field(&pad(&text, "1"), "size");
        let pad_2_size = numbers_in_field(&pad(&text, "2"), "size");
        for (number, size) in [("1", pad_1_size), ("2", pad_2_size)] {
            assert_eq!(size.len(), 2, "{side} pad {number}");
            assert_near(
                size[0],
                2.55,
                &format!("{side} pad {number} width reference"),
            );
            assert_near(
                size[1],
                2.5,
                &format!("{side} pad {number} height reference"),
            );
        }
    }
}

#[test]
fn socket_terminals_keep_distinct_nets_and_effective_side_layers() {
    for (side, part_side, expected_layers) in [
        ("F", Side::Front, vec!["F.Cu", "F.Paste", "F.Mask"]),
        ("B", Side::Back, vec!["B.Cu", "B.Paste", "B.Mask"]),
    ] {
        let text = render(side, part_side, 0.0);
        let pad_1 = pad(&text, "1");
        let pad_2 = pad(&text, "2");
        assert!(pad_1.contains("\"MX_ROW_QUAL\""), "{side} pad 1 net");
        assert!(pad_2.contains("\"MX_COL_QUAL\""), "{side} pad 2 net");
        assert!(
            !pad_1.contains("\"MX_COL_QUAL\""),
            "{side} pad 1 is distinct"
        );
        assert!(
            !pad_2.contains("\"MX_ROW_QUAL\""),
            "{side} pad 2 is distinct"
        );
        assert_eq!(
            quoted_values_in_field(&pad_1, "layers"),
            expected_layers,
            "{side} pad 1 layers"
        );
        assert_eq!(
            quoted_values_in_field(&pad_2, "layers"),
            expected_layers,
            "{side} pad 2 layers"
        );
        let hole_layers = quoted_values_in_field(&socket_holes(&text)[0], "layers");
        assert_eq!(
            hole_layers,
            ["F&B.Cu", "*.Mask"],
            "{side} NPTH effective layers"
        );
        assert_socket_model_binding(&text, side);
    }
}

fn assert_socket_model_binding(text: &str, side: &str) {
    let model = blocks_named(text, "model")
        .into_iter()
        .find(|block| block.contains(SOCKET_MODEL))
        .unwrap_or_else(|| panic!("{side} missing socket model binding {SOCKET_MODEL}"));
    let offset = xyz_numbers_in_field(&model, "offset");
    let scale = xyz_numbers_in_field(&model, "scale");
    let rotation = xyz_numbers_in_field(&model, "rotate");
    let expected_rotation = if side == "F" {
        [0.0, 180.0, 0.0]
    } else {
        [180.0, 0.0, 0.0]
    };
    assert_eq!(offset.len(), 3, "{side} socket model offset");
    assert_near(offset[0], 0.0, &format!("{side} socket model x offset"));
    assert_near(offset[1], 0.0, &format!("{side} socket model y offset"));
    // This is the generator's configured offset for the explicitly selected
    // PCB thickness; its datum is not independent manufacturer evidence.
    assert_near(
        offset[2],
        -PCB_THICKNESS_MM - SOCKET_MODEL_Z_DATUM_MM,
        &format!("{side} socket model z offset at {PCB_THICKNESS_MM} mm"),
    );
    assert_eq!(scale, [1.0, 1.0, 1.0], "{side} socket model scale");
    assert_eq!(rotation, expected_rotation, "{side} socket model rotation");
}
