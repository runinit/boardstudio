use boardstudio_footprints::{
    generators::bundled,
    nets::NetAllocator,
    sexpr::{self, Expr},
    types::{GeneratorRef, PartRef, Pose2, Side, Vec2},
};
use serde_json::json;

const SOURCE: &str = "ceoloide/switch_choc_v1_v2";
const MODEL: &str = "Choc_V1_Hotswap.step";
const PCB_THICKNESS_MM: f64 = 1.6;
const EPSILON_MM: f64 = 1.0e-6;
const FROM_NET: &str = "CHOC_ROW_QUAL";
const TO_NET: &str = "CHOC_COL_QUAL";

// This fixture selects Choc v1 only, nonreversible hot-swap, no solder pads,
// nonplated holes, a 1.6 mm PCB, 2.6 mm front/back outer-pad widths, and
// hotswap_pads_same_side=false. The additional side, terminal names, generator
// version, definition, reference, and source pose are set explicitly below.
// The terminal names are configuration identities, not manufacturer polarity.
fn render(side: &str, part_side: Side, at: Vec2, rotation: f64) -> Vec<Expr> {
    let generator = GeneratorRef {
        source: SOURCE.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(json!({
            "side": side,
            "choc_v1_support": true,
            "choc_v2_support": false,
            "hotswap": true,
            "reversible": false,
            "solder": false,
            "include_plated_holes": false,
            "pcb_thickness": PCB_THICKNESS_MM,
            "outer_pad_width_front": 2.6,
            "outer_pad_width_back": 2.6,
            "hotswap_pads_same_side": false,
            "from": FROM_NET,
            "to": TO_NET,
        }))
        .expect("selected Choc socket parameters are valid"),
    };
    let part = PartRef {
        id: "choc-socket-qualification".into(),
        reference: "S1".into(),
        pose: Pose2 { at, rotation },
        side: part_side,
        generator_parameters: None,
    };
    let mut nets = NetAllocator::new(&[], 1);
    bundled()
        .render(
            "choc-socket-qualification",
            &generator,
            Some(&part),
            &mut nets,
        )
        .expect("the bundled Choc socket renders")
}

fn list_named<'a>(forms: &'a [Expr], name: &str) -> &'a [Expr] {
    forms
        .iter()
        .find_map(|form| {
            let items = form.as_list()?;
            (items.first().and_then(|item| item.value().ok()) == Some(name)).then_some(items)
        })
        .unwrap_or_else(|| panic!("missing {name} form"))
}

fn child<'a>(items: &'a [Expr], name: &str) -> &'a [Expr] {
    sexpr::child(items, name).unwrap_or_else(|| panic!("missing {name} field"))
}

fn children<'a>(items: &'a [Expr], name: &'a str) -> Vec<&'a [Expr]> {
    sexpr::children(items, name).collect()
}

fn scalar(item: &Expr) -> &str {
    item.value().expect("field value is a scalar")
}

fn numbers_in(items: &[Expr]) -> Vec<f64> {
    items
        .iter()
        .skip(1)
        .map(|item| {
            scalar(item)
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("expected numeric S-expression value, found {item:?}"))
        })
        .collect()
}

fn numbers_in_field(items: &[Expr], name: &str) -> Vec<f64> {
    numbers_in(child(items, name))
}

fn strings_in_field(items: &[Expr], name: &str) -> Vec<String> {
    child(items, name)
        .iter()
        .skip(1)
        .map(|item| scalar(item).to_owned())
        .collect()
}

fn socket_holes(footprint: &[Expr]) -> Vec<&[Expr]> {
    let nonplated = children(footprint, "pad")
        .into_iter()
        .filter(|pad| {
            pad.get(2)
                .is_some_and(|kind| scalar(kind) == "np_thru_hole")
        })
        .collect::<Vec<_>>();
    // In this explicit configuration the source emits one centerhole from
    // common_top, two Choc v1 stabilizers, then hotswap_common and one
    // face-specific socket hole. The last two NPTH forms therefore retain the
    // source-block identity of the documented socket pair; no geometry value
    // is used to select them.
    assert_eq!(
        nonplated.len(),
        5,
        "centerhole, two v1 stabilizers, shared socket hole, and face socket hole"
    );
    nonplated.into_iter().skip(3).collect()
}

fn smd_pad<'a>(footprint: &'a [Expr], number: &str) -> &'a [Expr] {
    children(footprint, "pad")
        .into_iter()
        .find(|pad| {
            pad.get(1).is_some_and(|value| scalar(value) == number)
                && pad.get(2).is_some_and(|kind| scalar(kind) == "smd")
        })
        .unwrap_or_else(|| panic!("missing SMD pad {number}"))
}

fn near(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= EPSILON_MM,
        "{label}: expected {expected}, got {actual}"
    );
}

fn assert_vector_near(actual: [f64; 2], expected: [f64; 2], label: &str) {
    near(actual[0], expected[0], &format!("{label} x"));
    near(actual[1], expected[1], &format!("{label} y"));
}

fn assert_exact_values(actual: Vec<String>, mut expected: Vec<&str>, label: &str) {
    let mut actual = actual;
    actual.sort();
    expected.sort();
    assert_eq!(
        actual,
        expected.into_iter().map(str::to_owned).collect::<Vec<_>>(),
        "{label}"
    );
}

fn assert_socket_model(footprint: &[Expr], side: &str) {
    let model = children(footprint, "model")
        .into_iter()
        .find(|model| {
            model
                .get(1)
                .is_some_and(|path| scalar(path).ends_with(MODEL))
        })
        .unwrap_or_else(|| panic!("{side} missing configured socket model {MODEL}"));
    assert_xyz_near(
        xyz_numbers_in_field(model, "offset"),
        [0.0, 0.0, -PCB_THICKNESS_MM],
        &format!("{side} socket model configured offset"),
    );
    assert_xyz_near(
        xyz_numbers_in_field(model, "scale"),
        [1.0, 1.0, 1.0],
        &format!("{side} socket model configured scale"),
    );
    let expected_rotation = if side == "F" {
        [0.0, 180.0, 0.0]
    } else {
        [180.0, 0.0, 0.0]
    };
    assert_xyz_near(
        xyz_numbers_in_field(model, "rotate"),
        expected_rotation,
        &format!("{side} socket model configured rotation"),
    );
}

fn xyz_numbers_in_field(items: &[Expr], name: &str) -> [f64; 3] {
    let field = child(items, name);
    let xyz = child(field, "xyz");
    let numbers = numbers_in(xyz);
    assert_eq!(numbers.len(), 3, "{name} has three coordinates");
    [numbers[0], numbers[1], numbers[2]]
}

fn assert_xyz_near(actual: [f64; 3], expected: [f64; 3], label: &str) {
    for (index, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        near(actual, expected, &format!("{label} axis {index}"));
    }
}

fn local_hole_delta(holes: &[&[Expr]], expected_rotation: f64) -> [f64; 2] {
    assert_eq!(
        holes.len(),
        2,
        "exactly two socket NPTH drills are selected"
    );
    for (index, hole) in holes.iter().enumerate() {
        let drill = numbers_in_field(hole, "drill");
        assert_eq!(drill.len(), 1, "socket hole {index} has one drill diameter");
        near(drill[0], 3.0, &format!("socket hole {index} nominal drill"));
    }
    let upper = numbers_in_field(holes[0], "at");
    let lower = numbers_in_field(holes[1], "at");
    assert_eq!(upper.len(), 3, "common upper hole x/y/rotation fields");
    assert_eq!(
        lower.len(),
        3,
        "side-specific lower hole x/y/rotation fields"
    );
    near(
        upper[2],
        expected_rotation,
        "common upper hole emitted rotation",
    );
    // Producer configuration regression only; this is not a manufacturer angle or fit claim.
    near(
        lower[2],
        195.0 + expected_rotation,
        "side-specific lower hole emitted rotation",
    );
    [lower[0] - upper[0], lower[1] - upper[1]]
}

fn kicad_world_delta(footprint: &[Expr], local_delta: [f64; 2]) -> [f64; 2] {
    let angle = numbers_in_field(footprint, "at")[2].to_radians();
    // KiCad uses a down-Y frame and serializes the footprint rotation with the
    // corresponding negative-theta transform. The expected vectors below are
    // fixed drawing-derived values, not outputs of this calculation.
    [
        local_delta[0] * angle.cos() + local_delta[1] * angle.sin(),
        -local_delta[0] * angle.sin() + local_delta[1] * angle.cos(),
    ]
}

fn expected_socket_vector(side: &str, rotation: u16) -> [f64; 2] {
    match (side, rotation) {
        ("F", 0) => [-5.0, 2.2],
        ("F", 90) => [2.2, 5.0],
        ("F", 120) => [4.405_255_888, 3.230_127_019],
        ("B", 0) => [5.0, 2.2],
        ("B", 90) => [2.2, -5.0],
        ("B", 120) => [-0.594_744_112, -5.430_127_019],
        _ => panic!("no independent socket vector for {side} at {rotation} degrees"),
    }
}

#[test]
fn datasheet_drills_and_dimensioned_left_land_match_nominals_at_both_faces_and_angles() {
    for (side, part_side) in [("F", Side::Front), ("B", Side::Back)] {
        for rotation in [0_u16, 90, 120] {
            let at = if rotation == 120 {
                // Stored SW26 pose from the pinned Sofle source; this is a pose
                // fixture, not a board-origin conversion or physical-fit claim.
                Vec2 { x: 96.71, y: 133.4 }
            } else {
                Vec2 { x: 0.0, y: 0.0 }
            };
            let forms = render(side, part_side.clone(), at, f64::from(rotation));
            let footprint = list_named(&forms, "footprint");
            let holes = socket_holes(footprint);
            let footprint_angle = numbers_in_field(footprint, "at")[2];
            let actual = kicad_world_delta(footprint, local_hole_delta(&holes, footprint_angle));
            assert_vector_near(
                actual,
                expected_socket_vector(side, rotation),
                &format!("{side} at {rotation}° common-upper-to-lower hole vector"),
            );

            // The datasheet's LEFT land beside the lower hole maps to emitted
            // outer pad 2. This is the only land with an explicit 2.60 x 2.60
            // callout; pad 1 is the undimensioned RIGHT land.
            let outer_pad = smd_pad(footprint, "2");
            let size = numbers_in_field(outer_pad, "size");
            assert_eq!(size.len(), 2, "{side} dimensioned outer land size");
            near(size[0], 2.6, &format!("{side} outer land nominal width"));
            near(size[1], 2.6, &format!("{side} outer land nominal height"));

            if rotation == 120 {
                let placement = numbers_in_field(footprint, "at");
                assert_eq!(placement.len(), 3, "{side} stored Sofle pose fields");
                near(placement[0], 96.71, &format!("{side} SW26 source x pose"));
                near(
                    placement[1],
                    -133.4,
                    &format!("{side} serialized down-Y pose"),
                );
                near(placement[2], 120.0, &format!("{side} SW26 source rotation"));
            }
        }
    }
}

#[test]
fn configured_terminals_and_effective_pad_layers_are_exact_and_distinct() {
    for (side, part_side, copper, paste, mask) in [
        ("F", Side::Front, "F.Cu", "F.Paste", "F.Mask"),
        ("B", Side::Back, "B.Cu", "B.Paste", "B.Mask"),
    ] {
        let forms = render(side, part_side, Vec2 { x: 0.0, y: 0.0 }, 0.0);
        let footprint = list_named(&forms, "footprint");
        let inner_pad = smd_pad(footprint, "1");
        let outer_pad = smd_pad(footprint, "2");
        assert_eq!(
            scalar(&child(inner_pad, "net")[2]),
            FROM_NET,
            "{side} fixed inner pad uses the configured from identity; manufacturer polarity is unknown"
        );
        assert_eq!(
            scalar(&child(outer_pad, "net")[2]),
            TO_NET,
            "{side} outer pad uses the configured to identity; manufacturer polarity is unknown"
        );
        assert_ne!(FROM_NET, TO_NET, "configured terminal names are distinct");

        assert_exact_values(
            strings_in_field(inner_pad, "layers"),
            vec![copper, paste, mask],
            &format!("{side} inner pad effective copper/paste/mask set"),
        );
        assert_exact_values(
            strings_in_field(outer_pad, "layers"),
            vec![copper, paste, mask],
            &format!("{side} outer pad effective copper/paste/mask set"),
        );
        let holes = socket_holes(footprint);
        assert_eq!(holes.len(), 2, "{side} nonplated socket hole count");
        for (index, hole) in holes.iter().enumerate() {
            assert_exact_values(
                strings_in_field(hole, "layers"),
                vec!["*.Cu", "*.Mask"],
                &format!("{side} NPTH hole {index} effective layer set"),
            );
        }

        // Pad 1 corresponds to the drawing's RIGHT land, whose size is not
        // dimensioned. Its emitted configuration is deliberately not asserted
        // as manufacturer-qualified geometry. Model offset, orientation and
        // scale are generator configuration evidence only; no model datum,
        // contact, switch-travel, keycap or plate-fit claim follows.
        assert_socket_model(footprint, side);
    }
}
