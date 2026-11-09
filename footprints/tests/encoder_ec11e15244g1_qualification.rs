//! Numeric and terminal-role qualification of the EC11E15244G1 front pattern.
//!
//! Manufacturer coordinates are expressed in the mounting-side view (x right,
//! y down). This explicitly configured pattern checks nominal layout only; it
//! does not establish the tab profile, plating, rear fit, terminal clearance,
//! model fit, firmware behavior or ticket 118 acceptance.

use std::collections::BTreeMap;

use boardstudio_footprints::definition::DefinitionInput;
use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::nets::NetAllocator;
use boardstudio_footprints::sexpr::{self, Expr};
use boardstudio_footprints::types::{GeneratorRef, PadShape, PartRef, Pose2, Side, Vec2};
use boardstudio_footprints::{bundled, generators};
use serde_json::{Value, json};

const SOURCE: &str = "ceoloide/rotary_encoder_ec11_ec12";
const EPSILON: f64 = 1.0e-6;
const SENTINELS: [(&str, &str); 5] = [
    ("A", "ENCODER_B_SENTINEL"),
    ("B", "ENCODER_COMMON_SENTINEL"),
    ("C", "ENCODER_A_SENTINEL"),
    ("S1", "ENCODER_PUSH_D_SENTINEL"),
    ("S2", "ENCODER_PUSH_E_SENTINEL"),
];

fn parameters() -> Value {
    json!({
        "side": "F",
        "reversible": false,
        // This fixture selects the generator's NPTH mounting-hole branch; it
        // makes no claim about the manufacturer's plating requirement.
        "include_plated_mounting_holes": false,
        "signal_hole_width": 1.0,
        "signal_hole_height": 1.0,
        "encoder_pads_position": 7.5,
        "include_momentary_switch_pads": true,
        "mounting_holes_position": 6.25,
        "mounting_holes_height": 2.6,
        "mounting_holes_width": 1.5,
        "A": SENTINELS[0].1,
        "B": SENTINELS[1].1,
        "C": SENTINELS[2].1,
        "S1": SENTINELS[3].1,
        "S2": SENTINELS[4].1,
    })
}

fn generator() -> GeneratorRef {
    GeneratorRef {
        source: SOURCE.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters()).expect("generator parameters"),
    }
}

fn render(rotation: f64) -> Vec<Expr> {
    let part = PartRef {
        id: "ec11-qualification".into(),
        reference: "RE1".into(),
        pose: Pose2 {
            at: Vec2 { x: 13.25, y: -8.75 },
            rotation,
        },
        side: Side::Front,
        generator_parameters: None,
    };
    let mut nets = NetAllocator::new(&[], 1);
    bundled()
        .render("ec11-qualification", &generator(), Some(&part), &mut nets)
        .expect("explicit front encoder fixture renders")
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
        .expect("field value")
        .value()
        .expect("scalar")
}

fn number(items: &[Expr], index: usize) -> f64 {
    scalar(items, index).parse().expect("numeric field")
}

fn close(actual: f64, expected: f64, label: &str) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "{label}: expected {expected}, found {actual}"
    );
}

fn footprint(forms: &[Expr]) -> &[Expr] {
    named(forms, "footprint").expect("rendered footprint")
}

fn pad<'a>(forms: &'a [Expr], id: &str) -> &'a [Expr] {
    let index = id
        .strip_prefix("pad-")
        .expect("literal generated pad ID")
        .parse::<usize>()
        .expect("numeric generated pad ID");
    sexpr::children(footprint(forms), "pad")
        .nth(index)
        .unwrap_or_else(|| panic!("missing {id} at emitted pad index {index}"))
}

fn pad_at(pad: &[Expr]) -> (f64, f64, f64) {
    let at = sexpr::child(pad, "at").expect("pad at");
    (number(at, 1), number(at, 2), number(at, 3))
}

fn layers(pad: &[Expr]) -> Vec<&str> {
    sexpr::child(pad, "layers")
        .expect("pad layers")
        .iter()
        .skip(1)
        .map(|layer| layer.value().expect("layer name"))
        .collect()
}

fn net(pad: &[Expr]) -> Option<(&str, f64)> {
    sexpr::child(pad, "net").map(|net| (scalar(net, 2), number(net, 1)))
}

fn expected_world(x: f64, y: f64, rotation: f64) -> (f64, f64) {
    let (sin, cos) = rotation.to_radians().sin_cos();
    // Local coordinates have already had generator Y normalized once. The
    // independently fixed document pose is (13.25, -8.75).
    (13.25 + x * cos - y * sin, -8.75 + x * sin + y * cos)
}

#[test]
fn front_pattern_geometry_roles_and_export_match_independent_nominals() {
    let registry = bundled();
    let generator = generator();
    let normalized = registry
        .normalize(&DefinitionInput {
            id: "generator:ec11-qualification".into(),
            kind: boardstudio_footprints::types::PartKind::Encoder,
            keycap: None,
            envelope_source: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            terminals: Default::default(),
            generator: Some(generator.clone()),
        })
        .expect("normalization succeeds")
        .expect("generated definition normalizes");

    let expected = [
        ("pad-0", "A", 2.5, 7.5, "ENCODER_B_SENTINEL", 3.0),
        ("pad-1", "B", 0.0, 7.5, "ENCODER_COMMON_SENTINEL", 4.0),
        ("pad-2", "C", -2.5, 7.5, "ENCODER_A_SENTINEL", 5.0),
        ("pad-3", "S1", -2.5, -7.0, "ENCODER_PUSH_D_SENTINEL", 1.0),
        ("pad-4", "S2", 2.5, -7.0, "ENCODER_PUSH_E_SENTINEL", 2.0),
    ];
    assert_eq!(normalized.pads.len(), 7);
    for (id, number, source_x, source_y, _net_name, _net_index) in expected {
        let normalized_pad = &normalized.pads[id[4..].parse::<usize>().expect("pad index")];
        assert_eq!(normalized_pad.id, id);
        assert_eq!(normalized_pad.number, number);
        // Manufacturer source view is y-down; geometry normalization changes
        // that local Y once into the document's y-up coordinates.
        close(normalized_pad.at.x, source_x, "normalized local x");
        close(normalized_pad.at.y, -source_y, "normalized local y");
        close(
            normalized_pad.rotation.expect("normalized pad angle"),
            0.0,
            "normalized electrical pad angle",
        );
        assert_eq!(normalized_pad.shape, PadShape::Oval);
        close(
            normalized_pad.size.x,
            1.6,
            "electrical land width configuration",
        );
        close(
            normalized_pad.size.y,
            1.6,
            "electrical land height configuration",
        );
        close(
            normalized_pad.drill.expect("electrical drill"),
            1.0,
            "Ø1 nominal electrical drill",
        );
    }
    assert_eq!(normalized.pads[5].id, "pad-5");
    assert_eq!(normalized.pads[6].id, "pad-6");
    assert_ne!(normalized.pads[5].id, normalized.pads[6].id);
    assert_eq!(normalized.pads[5].number, "");
    assert_eq!(normalized.pads[6].number, "");
    close(
        normalized.pads[5]
            .rotation
            .expect("left normalized tab angle"),
        90.0,
        "normalized left tab angle",
    );
    close(
        normalized.pads[6]
            .rotation
            .expect("right normalized tab angle"),
        90.0,
        "normalized right tab angle",
    );

    // The source-view spans and offsets come from fixed manufacturer values.
    close(2.5 - (-2.5), 5.0, "rotary row span");
    close(2.5 - (-2.5), 5.0, "push row span");
    close(7.5 - 7.0, 0.5, "rotary/push row offset");
    close(6.25 - (-6.25), 12.5, "tab centre span");

    for rotation in [0.0, 90.0, 120.0] {
        let forms = render(rotation);
        let root = footprint(&forms);
        let root_at = sexpr::child(root, "at").expect("root placement");
        close(number(root_at, 1), 13.25, "placed root x");
        close(number(root_at, 2), 8.75, "placed root Y conversion");
        close(number(root_at, 3), rotation, "placed root rotation");
        assert_eq!(sexpr::children(root, "pad").count(), 7);

        for (pad_id, pad_number, source_x, source_y, net_name, net_index) in expected {
            let rendered_pad = pad(&forms, pad_id);
            assert_eq!(scalar(rendered_pad, 1), pad_number);
            assert_eq!(scalar(rendered_pad, 2), "thru_hole");
            assert_eq!(scalar(rendered_pad, 3), "oval");
            let (x, y, angle) = pad_at(rendered_pad);
            close(x, source_x, "rendered local x");
            close(y, source_y, "rendered source-view y");
            close(angle, rotation, "rendered electrical pad angle");
            assert_eq!(net(rendered_pad), Some((net_name, net_index)));
            let layers = sexpr::child(rendered_pad, "layers").expect("layers");
            assert_eq!(
                layers
                    .iter()
                    .skip(1)
                    .map(|layer| layer.value().unwrap())
                    .collect::<Vec<_>>(),
                ["*.Cu", "*.Mask"]
            );
            let drill = sexpr::child(rendered_pad, "drill").expect("electrical drill");
            close(number(drill, 1), 1.0, "round Ø1 nominal drill");
            assert_eq!(drill.len(), 2, "equal dimensions emit a round drill");
            let expected_point = expected_world(source_x, -source_y, rotation);
            let actual_world = expected_world(x, -y, number(root_at, 3));
            close(actual_world.0, expected_point.0, "placed terminal world x");
            close(actual_world.1, expected_point.1, "placed terminal world y");
        }

        let tabs = [pad(&forms, "pad-5"), pad(&forms, "pad-6")];
        for (tab, x) in tabs.into_iter().zip([-6.25, 6.25]) {
            assert_eq!(scalar(tab, 2), "np_thru_hole");
            assert_eq!(scalar(tab, 3), "oval");
            assert_eq!(net(tab), None, "mounting tabs have no electrical net");
            let (actual_x, actual_y, angle) = pad_at(tab);
            close(actual_x, x, "tab local x");
            close(actual_y, 0.0, "tab local y");
            close(angle, rotation - 90.0, "tab orientation composed once");
            let expected_tab = expected_world(x, 0.0, rotation);
            let actual_tab = expected_world(actual_x, -actual_y, number(root_at, 3));
            close(actual_tab.0, expected_tab.0, "placed tab world x");
            close(actual_tab.1, expected_tab.1, "placed tab world y");
            let size = sexpr::child(tab, "size").expect("tab extents");
            close(number(size, 1), 2.6, "configured tab oval long axis");
            close(number(size, 2), 1.5, "configured tab oval short axis");
            let drill = sexpr::child(tab, "drill").expect("tab drill");
            assert_eq!(scalar(drill, 1), "oval");
            close(number(drill, 2), 2.6, "tab long drill extent");
            close(number(drill, 3), 1.5, "tab short drill extent");
            if rotation == 0.0 {
                // The raw oval axes (2.6 × 1.5) and -90° local angle yield
                // total horizontal × vertical extents of 1.5 × 2.6 mm.
                close(number(drill, 3), 1.5, "tab oval horizontal total extent");
                close(number(drill, 2), 2.6, "tab oval vertical total extent");
            }
            assert_eq!(
                sexpr::child(tab, "layers")
                    .unwrap()
                    .iter()
                    .skip(1)
                    .map(|layer| layer.value().unwrap())
                    .collect::<Vec<_>>(),
                ["*.Cu", "*.Mask"]
            );
        }

        let export = export_forms(forms.clone(), &BTreeMap::new()).expect("export forms");
        assert_eq!(export.footprints.len(), 1);
        let exported = export.footprints.join("\n");
        let exported_forms = sexpr::parse_forms(&exported).expect("export reparses");
        let exported_root = footprint(&exported_forms);
        assert_eq!(
            scalar(sexpr::child(exported_root, "layer").unwrap(), 1),
            "F.Cu"
        );
        let exported_at = sexpr::child(exported_root, "at").expect("exported placement");
        close(number(exported_at, 1), 13.25, "exported root x");
        close(number(exported_at, 2), 8.75, "exported root Y conversion");
        close(number(exported_at, 3), rotation, "exported root angle");
        let exported_pads: Vec<_> = sexpr::children(exported_root, "pad").collect();
        assert_eq!(exported_pads.len(), 7);
        for (index, (pad_id, pad_number, source_x, source_y, net_name, net_index)) in
            expected.iter().enumerate()
        {
            let exported_pad = exported_pads[index];
            assert_eq!(*pad_id, format!("pad-{index}"), "fresh literal identity");
            assert_eq!(scalar(exported_pad, 1), *pad_number);
            let (x, y, angle) = pad_at(exported_pad);
            close(x, *source_x, "exported electrical center x");
            close(y, *source_y, "exported electrical center y");
            close(angle, rotation, "exported electrical pad angle");
            assert_eq!(scalar(exported_pad, 2), "thru_hole");
            assert_eq!(scalar(exported_pad, 3), "oval");
            let size = sexpr::child(exported_pad, "size").expect("exported electrical size");
            close(number(size, 1), 1.6, "exported electrical land width");
            close(number(size, 2), 1.6, "exported electrical land height");
            let drill = sexpr::child(exported_pad, "drill").expect("exported electrical drill");
            assert_eq!(drill.len(), 2, "exported equal axes remain a round drill");
            close(number(drill, 1), 1.0, "exported round Ø1 drill");
            assert_eq!(layers(exported_pad), ["*.Cu", "*.Mask"]);
            assert_eq!(net(exported_pad), Some((*net_name, *net_index)));
        }
        for (index, expected_x) in [(5, -6.25), (6, 6.25)] {
            let exported_tab = exported_pads[index];
            let (x, y, angle) = pad_at(exported_tab);
            close(x, expected_x, "exported distinct tab identity x");
            close(y, 0.0, "exported tab y");
            close(angle, rotation - 90.0, "exported tab angle");
            assert_eq!(scalar(exported_tab, 1), "");
            assert_eq!(scalar(exported_tab, 2), "np_thru_hole");
            assert_eq!(scalar(exported_tab, 3), "oval");
            let size = sexpr::child(exported_tab, "size").expect("exported tab size");
            close(number(size, 1), 2.6, "exported tab long axis");
            close(number(size, 2), 1.5, "exported tab short axis");
            let drill = sexpr::child(exported_tab, "drill").expect("exported tab drill");
            assert_eq!(scalar(drill, 1), "oval");
            close(number(drill, 2), 2.6, "exported tab drill long axis");
            close(number(drill, 3), 1.5, "exported tab drill short axis");
            assert_eq!(layers(exported_tab), ["*.Cu", "*.Mask"]);
            assert_eq!(net(exported_tab), None);
        }
        assert_eq!(export.objects.len(), 0);
    }

    for (role, id) in [
        ("A", "pad-0"),
        ("B", "pad-1"),
        ("C", "pad-2"),
        ("S1", "pad-3"),
        ("S2", "pad-4"),
    ] {
        assert_eq!(normalized.terminals[role], [id]);
    }
}

#[test]
fn normalization_preserves_saved_pad_ids_and_nets_by_literal_index() {
    let mut input = json!({
        "id": "generator:ec11-saved",
        "kind": "encoder",
        "generator": {"source": SOURCE, "version": "bundled-1", "parameters": parameters()},
        "pads": [
            {"id":"saved-c","number":"old-0","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","netId":"N0"},
            {"id":"saved-a","number":"old-1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","netId":"N1"},
            {"id":"saved-b","number":"old-2","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","netId":"N2"},
            {"id":"saved-s1","number":"old-3","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","netId":"N3"},
            {"id":"saved-s2","number":"old-4","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle","netId":"N4"},
            {"id":"saved-tab-left","number":"","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"},
            {"id":"saved-tab-right","number":"","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"}
        ]
    });
    // Keep this object as serde input to the public normalization contract.
    let input: DefinitionInput =
        serde_json::from_value(std::mem::take(&mut input)).expect("definition input");
    let normalized = bundled()
        .normalize(&input)
        .expect("normalize")
        .expect("generated");
    assert_eq!(
        normalized
            .pads
            .iter()
            .map(|pad| pad.id.as_str())
            .collect::<Vec<_>>(),
        [
            "saved-c",
            "saved-a",
            "saved-b",
            "saved-s1",
            "saved-s2",
            "saved-tab-left",
            "saved-tab-right"
        ]
    );
    assert_eq!(
        normalized
            .pads
            .iter()
            .take(5)
            .map(|pad| pad.net_id.as_deref())
            .collect::<Vec<_>>(),
        [Some("N0"), Some("N1"), Some("N2"), Some("N3"), Some("N4")]
    );
    assert_eq!(normalized.terminals["A"], ["saved-c"]);
    assert_eq!(normalized.terminals["B"], ["saved-a"]);
    assert_eq!(normalized.terminals["C"], ["saved-b"]);
    assert_eq!(normalized.terminals["S1"], ["saved-s1"]);
    assert_eq!(normalized.terminals["S2"], ["saved-s2"]);
}

#[test]
fn declared_parameters_are_explicitly_available_for_this_pattern() {
    let schema = generators::bundled()
        .parameters(SOURCE)
        .expect("encoder schema");
    let entries: Vec<_> = schema.iter().map(|entry| entry.name.as_str()).collect();
    for name in [
        "signal_hole_width",
        "signal_hole_height",
        "encoder_pads_position",
        "include_momentary_switch_pads",
        "mounting_holes_position",
        "mounting_holes_height",
        "mounting_holes_width",
    ] {
        assert!(entries.contains(&name), "missing explicit parameter {name}");
    }
}
