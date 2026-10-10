//! Public-boundary qualification of the existing Kailh MX hot-swap layout.
//!
//! Independent source: Kailh CPG151101S11-16, drawing KHA-PG1511-388EN Rev A
//! (cover recommended PCB layout) and specification KH-PS2206-43 Rev A,
//! printed page 8/9, section 9. Dimensions are millimetres. The drawing gives
//! a 6.35 mm by 2.54 mm hole-centre vector, Ø3.00 mm drills, and parenthesized
//! rectangular-land reference dimensions (2.55) by (2.5) mm. A recommended-
//! layout tolerance note of ±0.05 mm appears directly beneath that figure.
//! Applying that note to the parenthesized land references is a qualification
//! interpretation for the verifier to confirm. The drawing does not dimension
//! the socket datum relative to the MX
//! switch origin or label the host lands with an electrical terminal number.
//!
//! Pad 1 currently uses 2.6 mm width against the drawing's 2.55 mm nominal,
//! exactly the upper bound of the ±0.05 mm recommended-layout tolerance printed
//! beneath that layout. Keep the nominal, tolerance and emitted value explicit
//! in the evidence table and assertion rather than replacing the nominal with
//! the implementation value.

use boardstudio_footprints::bundled;
use boardstudio_footprints::nets::NetAllocator;
use boardstudio_footprints::types::{GeneratorRef, PartRef, Pose2, Side, Vec2};
use serde_json::json;

const SOURCE: &str = "ceoloide/switch_mx";
const SOCKET_MODEL: &str = "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Hotswap_Kailh_MX.stp";

fn render(side: &str, part_side: Side, rotation: f64) -> String {
    render_at(side, part_side, Vec2 { x: 0.0, y: 0.0 }, rotation)
}

fn render_at(side: &str, part_side: Side, at: Vec2, rotation: f64) -> String {
    let parameters = json!({
        "side": side,
        "hotswap": true,
        "reversible": false,
        "pcb_thickness": 1.6,
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
            if !block.starts_with("(pad \"\" np_thru_hole ") || !block.contains("\"F&B.Cu\"") {
                return false;
            }
            let drill = numbers_in_field(block, "drill");
            drill.len() == 1 && within_layout_tolerance(drill[0], 3.0)
        })
        .collect()
}

fn world_xy(text: &str, local: &[f64]) -> [f64; 2] {
    let footprint = blocks_named(text, "footprint")
        .into_iter()
        .next()
        .expect("rendered footprint");
    let placement = numbers_in_field(&footprint, "at");
    let angle = placement[2].to_radians();
    [
        placement[0] + local[0] * angle.cos() - local[1] * angle.sin(),
        placement[1] + local[0] * angle.sin() + local[1] * angle.cos(),
    ]
}

fn within_layout_tolerance(actual: f64, nominal: f64) -> bool {
    (actual - nominal).abs() <= 0.05 + 1e-6
}

#[test]
fn recommended_socket_hole_pair_spacing_survives_side_and_placement_rotation() {
    for (side, part_side, expected_x) in [("F", Side::Front, 6.35), ("B", Side::Back, -6.35)] {
        for rotation in [0.0, 90.0] {
            let text = render(side, part_side.clone(), rotation);
            let holes = socket_holes(&text);
            assert_eq!(holes.len(), 2, "{side} at {rotation}°");
            for hole in &holes {
                assert_eq!(numbers_in_field(hole, "drill").len(), 1, "{side}");
                assert!(
                    within_layout_tolerance(numbers_in_field(hole, "drill")[0], 3.0),
                    "{side} drill against drawing nominal Ø3.00 ±0.05 mm"
                );
            }
            let first = world_xy(&text, &numbers_in_field(&holes[0], "at"));
            let second = world_xy(&text, &numbers_in_field(&holes[1], "at"));
            let angle = rotation.to_radians();
            let expected = [
                expected_x * angle.cos() - 2.54 * angle.sin(),
                expected_x * angle.sin() + 2.54 * angle.cos(),
            ];
            assert!(
                within_layout_tolerance(second[0] - first[0], expected[0]),
                "{side}: {first:?} -> {second:?}"
            );
            assert!(
                within_layout_tolerance(second[1] - first[1], expected[1]),
                "{side}: {first:?} -> {second:?}"
            );
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
    let first = world_xy(&text, &numbers_in_field(&holes[0], "at"));
    let second = world_xy(&text, &numbers_in_field(&holes[1], "at"));
    let angle = 300.0_f64.to_radians();
    let expected = [
        6.35 * angle.cos() - 2.54 * angle.sin(),
        6.35 * angle.sin() + 2.54 * angle.cos(),
    ];
    assert!(
        within_layout_tolerance(second[0] - first[0], expected[0]),
        "{first:?} -> {second:?}"
    );
    assert!(
        within_layout_tolerance(second[1] - first[1], expected[1]),
        "{first:?} -> {second:?}"
    );
}

#[test]
fn recommended_land_sizes_fit_the_printed_layout_tolerance() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        let text = render(side, part_side, 0.0);
        let pad_1_size = numbers_in_field(&pad(&text, "1"), "size");
        let pad_2_size = numbers_in_field(&pad(&text, "2"), "size");
        for (number, size) in [("1", pad_1_size), ("2", pad_2_size)] {
            assert_eq!(size.len(), 2, "{side} pad {number}");
            assert!(
                within_layout_tolerance(size[0], 2.55),
                "{side} pad {number} width {} against drawing nominal 2.55 ±0.05 mm",
                size[0]
            );
            assert!(
                within_layout_tolerance(size[1], 2.5),
                "{side} pad {number} height {} against drawing nominal 2.5 ±0.05 mm",
                size[1]
            );
        }
    }
}

#[test]
fn socket_terminals_keep_distinct_nets_and_effective_side_layers() {
    for (side, part_side, layer) in [
        ("F", Side::Front, "\"F.Cu\""),
        ("B", Side::Back, "\"B.Cu\""),
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
        assert!(
            pad_1.contains(layer) && pad_2.contains(layer),
            "{side} copper layer"
        );
        assert!(
            text.contains(SOCKET_MODEL),
            "{side} keeps the existing socket model binding"
        );
    }
}
