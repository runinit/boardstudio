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

use crate::common::qualification::{
    self, field, footprint, kicad_world_delta, near as assert_near, numbers_in_field, smd_pad,
    socket_model_rotation, xyz as xyz_numbers_in_field,
};
use boardstudio_footprints::{
    sexpr::{self, Expr},
    types::{Side, Vec2},
};
use serde_json::json;

const SOURCE: &str = "ceoloide/switch_mx";
const SOCKET_MODEL: &str = "${KIPRJMOD}/models/boardstudio/kiswitch/SW_Hotswap_Kailh_MX.stp";
const PCB_THICKNESS_MM: f64 = 1.6;
const SOCKET_MODEL_Z_DATUM_MM: f64 = 0.27094;

fn render(side: &str, part_side: Side, rotation: f64) -> Vec<Expr> {
    render_at(side, part_side, Vec2 { x: 0.0, y: 0.0 }, rotation)
}

fn render_at(side: &str, part_side: Side, at: Vec2, rotation: f64) -> Vec<Expr> {
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
    let generator = qualification::generator(SOURCE, parameters);
    let part = qualification::part("mx-socket-qualification", "S1", part_side, at, rotation);
    let text = qualification::render("definition", &generator, Some(&part))
        .unwrap_or_else(|error| panic!("render {side} at {rotation}°: {error}"))
        .iter()
        .map(sexpr::serialize)
        .collect::<Vec<_>>()
        .join("\n");
    sexpr::parse_forms(&text).expect("serialized generator forms parse")
}

fn pad<'a>(forms: &'a [Expr], number: &str) -> &'a [Expr] {
    let pad = smd_pad(footprint(forms), number);
    assert!(matches!(&pad[1], Expr::Str(value) if value == number));
    assert!(matches!(&pad[2], Expr::Atom(kind) if kind == "smd"));
    pad
}

fn socket_holes(forms: &[Expr]) -> Vec<&[Expr]> {
    sexpr::children(footprint(forms), "pad")
        .filter(|pad| {
            matches!(&pad[1], Expr::Str(value) if value.is_empty())
                && matches!(&pad[2], Expr::Atom(kind) if kind == "np_thru_hole")
                && quoted_values_in_field(pad, "layers").contains(&"F&B.Cu".to_owned())
        })
        .collect()
}

fn hole_delta(text: &[Expr]) -> [f64; 2] {
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

fn assert_socket_spacing(text: &[Expr], side: &str, rotation: u16) {
    let actual = kicad_world_delta(footprint(text), hole_delta(text));
    let expected = expected_kicad_delta(side, rotation);
    assert_near(actual[0], expected[0], &format!("{side} at {rotation}° x"));
    assert_near(actual[1], expected[1], &format!("{side} at {rotation}° y"));
}

fn quoted_values_in_field(items: &[Expr], name: &str) -> Vec<String> {
    field(items, name)[1..]
        .iter()
        .filter_map(|item| match item {
            Expr::Str(value) => Some(value.clone()),
            _ => None,
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
    let placement = numbers_in_field(footprint(&text), "at");
    assert_eq!(placement, [96.7, -133.41, 300.0]);

    let holes = socket_holes(&text);
    assert_eq!(holes.len(), 2);
    assert_socket_spacing(&text, "F", 300);
}

#[test]
fn recommended_land_sizes_match_drawing_reference_dimensions() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        let text = render(side, part_side, 0.0);
        let pad_1_size = numbers_in_field(pad(&text, "1"), "size");
        let pad_2_size = numbers_in_field(pad(&text, "2"), "size");
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
        let pad_1_text = sexpr::serialize(&Expr::List(pad_1.to_vec()));
        let pad_2_text = sexpr::serialize(&Expr::List(pad_2.to_vec()));
        assert!(pad_1_text.contains("\"MX_ROW_QUAL\""), "{side} pad 1 net");
        assert!(pad_2_text.contains("\"MX_COL_QUAL\""), "{side} pad 2 net");
        assert!(
            !pad_1_text.contains("\"MX_COL_QUAL\""),
            "{side} pad 1 is distinct"
        );
        assert!(
            !pad_2_text.contains("\"MX_ROW_QUAL\""),
            "{side} pad 2 is distinct"
        );
        assert_eq!(
            quoted_values_in_field(pad_1, "layers"),
            expected_layers,
            "{side} pad 1 layers"
        );
        assert_eq!(
            quoted_values_in_field(pad_2, "layers"),
            expected_layers,
            "{side} pad 2 layers"
        );
        let hole_layers = quoted_values_in_field(socket_holes(&text)[0], "layers");
        assert_eq!(
            hole_layers,
            ["F&B.Cu", "*.Mask"],
            "{side} NPTH effective layers"
        );
        assert_socket_model_binding(&text, side);
    }
}

fn assert_socket_model_binding(text: &[Expr], side: &str) {
    let model = sexpr::children(footprint(text), "model")
        .find(|block| block[1].value().is_ok_and(|path| path == SOCKET_MODEL))
        .unwrap_or_else(|| panic!("{side} missing socket model binding {SOCKET_MODEL}"));
    let offset = xyz_numbers_in_field(model, "offset");
    let scale = xyz_numbers_in_field(model, "scale");
    let rotation = xyz_numbers_in_field(model, "rotate");
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
    assert_eq!(
        rotation,
        socket_model_rotation(side),
        "{side} socket model rotation"
    );
}
