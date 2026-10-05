//! Provider behaviour that no recorded generator output can reach in step 2:
//! normalization, terminal discovery, the catalogue, typed parameters and
//! registry validation, exercised with synthetic generators.
use std::collections::BTreeMap;

use boardstudio_footprints::context::RenderContext;
use boardstudio_footprints::definition::{DEFINITION_ID_PREFIX, DefinitionInput};
use boardstudio_footprints::export::export_forms;
use boardstudio_footprints::nets::{NetAllocator, NoNets};
use boardstudio_footprints::params::ParamSpec;
use boardstudio_footprints::registry::{GeneratorSpec, KeycapParameters, License, Registry};
use boardstudio_footprints::sexpr::{parse_forms, serialize};
use boardstudio_footprints::types::*;
use boardstudio_footprints::{GeneratorError, Result};
use serde_json::{Value, json};

const MIT: License = License {
    spdx: "MIT",
    author: "test",
};

static SWITCH_PARAMS: [ParamSpec; 7] = [
    ParamSpec::SIDE,
    ParamSpec::boolean("hotswap", false),
    ParamSpec::number("keycap_width", 18.0),
    ParamSpec::number("keycap_height", 18.0),
    ParamSpec::string("label", "SW"),
    ParamSpec::net("from"),
    ParamSpec::net("to"),
];

fn switch_body(p: &RenderContext<'_>) -> Result<String> {
    if p.number("keycap_width") == 99.0 {
        return Err(GeneratorError::rejected(
            "test/switch",
            Some("keycap_width"),
            "No such keycap.",
        ));
    }
    let hotswap = if p.flag("hotswap") {
        "(pad \"\" np_thru_hole circle (at 0 0) (size 3 3) (drill 3) (layers \"*.Cu\"))"
    } else {
        ""
    };
    Ok(format!(
        "(footprint \"test:switch\" (layer \"F.Cu\") {at}
  (pad \"1\" smd rect (at -2 0) (size 1.5 1.5) (layers \"F.Cu\") {from})
  (pad \"2\" smd rect (at 2 0 90) (size 1.5 1.5) (layers B.Cu) {to})
  {hotswap}
  (fp_rect (start -4 -3) (end 4 3) (layer \"F.CrtYd\")))",
        at = p.at(),
        from = p.net("from"),
        to = p.net("to"),
    ))
}

static SWITCH: GeneratorSpec = GeneratorSpec {
    source: "test/switch",
    display_name: "test switch",
    kind: PartKind::Switch,
    matrix_terminals: Some(("from", "to")),
    keycap_parameters: Some(KeycapParameters {
        width: "keycap_width",
        height: "keycap_height",
    }),
    parameters: &SWITCH_PARAMS,
    license: MIT,
    body: switch_body,
};

static BARE_PARAMS: [ParamSpec; 1] = [ParamSpec::SIDE];

fn bare_body(_: &RenderContext<'_>) -> Result<String> {
    Ok("(footprint \"test:bare\" (pad \"1\" smd rect (at 3 4) (size 2 1) (layers \"F.Cu\") (net 0 \"\")))".into())
}

static BARE: GeneratorSpec = GeneratorSpec {
    source: "test/bare",
    display_name: "test bare",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &BARE_PARAMS,
    license: MIT,
    body: bare_body,
};

static UTILITY: GeneratorSpec = GeneratorSpec {
    source: "test/utility",
    display_name: "test utility",
    kind: PartKind::Utility,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &BARE_PARAMS,
    license: MIT,
    body: bare_body,
};

fn registry() -> Registry {
    Registry::new(&[&SWITCH, &BARE, &UTILITY]).unwrap()
}

fn generator(source: &str, parameters: Value) -> GeneratorRef {
    GeneratorRef {
        source: source.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).unwrap(),
    }
}

fn definition(source: &str, kind: PartKind, parameters: Value) -> DefinitionInput {
    DefinitionInput {
        id: format!("{DEFINITION_ID_PREFIX}{source}"),
        kind,
        keycap: None,
        envelope_source: None,
        courtyard: Vec::new(),
        pads: Vec::new(),
        terminals: BTreeMap::new(),
        generator: Some(generator(source, parameters)),
    }
}

fn pad(id: &str, net: Option<&str>) -> Pad {
    Pad {
        id: id.into(),
        number: String::new(),
        at: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 1.0, y: 1.0 },
        shape: PadShape::Rect,
        drill: None,
        plated: None,
        side: None,
        rotation: None,
        net_id: net.map(str::to_owned),
    }
}

#[test]
fn the_registry_rejects_inconsistent_declarations() {
    use boardstudio_footprints::params::{DefaultValue, ParamKind};
    fn spec(
        parameters: Vec<ParamSpec>,
        matrix: Option<(&'static str, &'static str)>,
    ) -> &'static GeneratorSpec {
        Box::leak(Box::new(GeneratorSpec {
            source: "test/bad",
            display_name: "bad",
            kind: PartKind::Custom,
            matrix_terminals: matrix,
            keycap_parameters: None,
            parameters: Box::leak(parameters.into_boxed_slice()),
            license: MIT,
            body: bare_body,
        }))
    }
    let reason = |spec| Registry::new(&[spec]).err().expect("rejected").0;
    assert!(reason(spec(vec![], None)).contains("`side` parameter with default F"));
    assert!(reason(spec(vec![ParamSpec::string("side", "B")], None)).contains("default F"));
    assert!(reason(spec(vec![ParamSpec::number("side", 1.0)], None)).contains("default F"));
    let twice = vec![
        ParamSpec::SIDE,
        ParamSpec::boolean("a", true),
        ParamSpec::boolean("a", false),
    ];
    assert!(reason(spec(twice, None)).contains("declared twice"));
    let mismatched = ParamSpec {
        name: "n",
        kind: ParamKind::Number,
        default: DefaultValue::Str("x"),
    };
    assert!(
        reason(spec(vec![ParamSpec::SIDE, mismatched], None)).contains("does not match its type")
    );
    let nothing = ParamSpec {
        name: "n",
        kind: ParamKind::Number,
        default: DefaultValue::None,
    };
    assert!(reason(spec(vec![ParamSpec::SIDE, nothing], None)).contains("needs a default"));
    let terminals = vec![ParamSpec::SIDE, ParamSpec::boolean("row", true)];
    assert!(reason(spec(terminals, Some(("row", "row")))).contains("not a net parameter"));
    assert!(
        Registry::new(&[&BARE, &BARE])
            .err()
            .unwrap()
            .0
            .contains("duplicate generator")
    );
}

#[test]
fn the_schema_lists_declared_types_in_declaration_order() {
    let schema = registry().parameters("test/switch").unwrap();
    let names: Vec<&str> = schema.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "side",
            "hotswap",
            "keycap_width",
            "keycap_height",
            "label",
            "from",
            "to"
        ]
    );
    assert_eq!(
        serde_json::to_value(&schema[..2]).unwrap(),
        json!([{ "name": "side", "type": "string", "value": "F" }, { "name": "hotswap", "type": "boolean", "value": false }])
    );
    assert_eq!(
        serde_json::to_value(&schema[5]).unwrap(),
        json!({ "name": "from", "type": "net" })
    );
    assert!(matches!(
        registry().parameters("test/none"),
        Err(GeneratorError::UnknownGenerator { .. })
    ));
}

#[test]
fn saved_values_must_match_their_declared_types() {
    let registry = registry();
    let render = |parameters: Value| {
        registry.render(
            "d",
            &generator("test/switch", parameters),
            None,
            &mut NoNets,
        )
    };
    let error = render(json!({ "label": 5 })).unwrap_err();
    assert_eq!(
        error.message(),
        "test/switch: parameter label must be text."
    );
    assert!(
        matches!(error, GeneratorError::InvalidParameter { ref parameter, .. } if parameter == "label")
    );
    assert_eq!(
        render(json!({ "hotswap": "true" })).unwrap_err().message(),
        "test/switch: parameter hotswap must be true or false."
    );
    assert_eq!(
        render(json!({ "keycap_width": "18" }))
            .unwrap_err()
            .message(),
        "test/switch: parameter keycap_width must be a number."
    );
    assert_eq!(
        render(json!({ "from": 5 })).unwrap_err().message(),
        "test/switch: parameter from must be a net name."
    );
    // Null and missing values use the default; undeclared names are ignored.
    assert!(render(json!({ "label": null, "unknown": [1, 2] })).is_ok());
}

#[test]
fn generator_rejections_keep_their_text_and_name_the_parameter() {
    let error = registry()
        .render(
            "d",
            &generator("test/switch", json!({ "keycap_width": 99 })),
            None,
            &mut NoNets,
        )
        .unwrap_err();
    assert_eq!(error.message(), "No such keycap.");
    assert!(
        matches!(error, GeneratorError::Rejected { ref generator, parameter: Some(ref p), .. } if generator == "test/switch" && p == "keycap_width")
    );
}

#[test]
fn net_parameters_allocate_in_declaration_order_not_input_order() {
    let registry = registry();
    let mut nets = NetAllocator::new(&[], 1);
    let generator = generator("test/switch", json!({ "to": "COL", "from": "ROW" }));
    registry.render("d", &generator, None, &mut nets).unwrap();
    let names: Vec<_> = nets
        .snapshot()
        .into_iter()
        .map(|n| (n.name, n.index))
        .collect();
    assert_eq!(names, [("ROW".to_owned(), 1), ("COL".to_owned(), 2)]);
}

#[test]
fn normalization_leaves_non_generator_definitions_alone_and_rejects_unknown_ones() {
    let registry = registry();
    let mut input = definition("test/switch", PartKind::Switch, json!({}));
    input.generator = None;
    assert_eq!(registry.normalize(&input).unwrap(), None);
    let unknown = definition("test/missing", PartKind::Custom, json!({}));
    assert_eq!(
        registry.normalize(&unknown).unwrap_err(),
        GeneratorError::UnknownGenerator {
            source: "test/missing".into()
        }
    );
    let mut future = definition("test/switch", PartKind::Switch, json!({}));
    future.generator.as_mut().unwrap().version = "future".into();
    assert_eq!(
        registry.normalize(&future).unwrap_err().message(),
        "Unsupported Ergogen generator version: future"
    );
}

#[test]
fn normalization_generates_pads_terminals_and_envelopes() {
    let normalized = registry()
        .normalize(&definition("test/switch", PartKind::Switch, json!({})))
        .unwrap()
        .unwrap();
    let ids: Vec<&str> = normalized.pads.iter().map(|pad| pad.id.as_str()).collect();
    assert_eq!(ids, ["pad-0", "pad-1"]);
    assert_eq!(
        normalized.pads[1].side,
        Some(Side::Back),
        "a bare B.Cu layer atom marks a back pad"
    );
    assert_eq!(normalized.pads[1].rotation, Some(-90.0));
    assert_eq!(normalized.terminals["from"], ["pad-0"]);
    assert_eq!(normalized.terminals["to"], ["pad-1"]);
    assert_eq!(normalized.keycap, Some(Vec2 { x: 18.0, y: 18.0 }));
    assert_eq!(normalized.courtyard.len(), 4);
    assert_eq!(
        normalized.courtyard[0],
        Vec2 { x: -4.0, y: 3.0 },
        "the rectangle flips to the document's upward Y"
    );
    assert_eq!(normalized.envelope_notice, None);
    assert_eq!(
        normalized.envelope_source.courtyard,
        Some(EnvelopeOrigin::Generated)
    );
    assert_eq!(
        normalized.envelope_source.keycap,
        Some(EnvelopeOrigin::Generated)
    );
}

#[test]
fn saved_pad_ids_and_nets_follow_index_and_extra_pads_get_fresh_ids() {
    let registry = registry();
    let mut input = definition("test/switch", PartKind::Switch, json!({ "hotswap": true }));
    input.pads = vec![pad("saved-a", Some("net-a")), pad("saved-b", Some(""))];
    let normalized = registry.normalize(&input).unwrap().unwrap();
    let pads: Vec<(&str, Option<&str>)> = normalized
        .pads
        .iter()
        .map(|p| (p.id.as_str(), p.net_id.as_deref()))
        .collect();
    assert_eq!(
        pads,
        [
            ("saved-a", Some("net-a")),
            ("saved-b", None),
            ("pad-2", None)
        ]
    );
    assert_eq!(
        normalized.terminals["to"],
        ["saved-b"],
        "terminals use the saved pad IDs"
    );
    input
        .generator
        .as_mut()
        .unwrap()
        .parameters
        .insert("hotswap".into(), json!(false));
    input.pads.push(pad("saved-c", None));
    assert_eq!(
        registry.normalize(&input).unwrap().unwrap().pads.len(),
        2,
        "fewer generated pads drop the saved extras"
    );
}

#[test]
fn keycap_envelope_follows_parameters_unless_authored() {
    let registry = registry();
    let resized = registry
        .normalize(&definition(
            "test/switch",
            PartKind::Switch,
            json!({ "keycap_width": 23, "keycap_height": 17.5 }),
        ))
        .unwrap()
        .unwrap();
    assert_eq!(resized.keycap, Some(Vec2 { x: 23.0, y: 17.5 }));

    let mut authored = definition(
        "test/switch",
        PartKind::Switch,
        json!({ "keycap_width": 25 }),
    );
    authored.keycap = Some(Vec2 { x: 22.0, y: 21.0 });
    authored.envelope_source = Some(EnvelopeSource {
        courtyard: None,
        keycap: Some(EnvelopeOrigin::Authored),
    });
    let kept = registry.normalize(&authored).unwrap().unwrap();
    assert_eq!(kept.keycap, Some(Vec2 { x: 22.0, y: 21.0 }));
    assert_eq!(kept.envelope_source.keycap, Some(EnvelopeOrigin::Authored));

    authored.keycap = Some(Vec2 { x: 0.0, y: 5.0 });
    let replaced = registry.normalize(&authored).unwrap().unwrap();
    assert_eq!(
        replaced.keycap,
        Some(Vec2 { x: 25.0, y: 18.0 }),
        "an invalid authored keycap is regenerated"
    );

    for (name, value) in [("keycap_width", 0.0), ("keycap_height", -1.0)] {
        let zero = definition("test/switch", PartKind::Switch, json!({ name: value }));
        let error = registry.normalize(&zero).unwrap_err();
        assert_eq!(
            error.message(),
            "Keycap envelope dimensions must be greater than zero."
        );
        assert!(
            matches!(error, GeneratorError::Rejected { parameter: Some(ref p), .. } if p == name)
        );
    }
    let wrong = definition(
        "test/switch",
        PartKind::Switch,
        json!({ "keycap_width": "23" }),
    );
    assert!(matches!(
        registry.normalize(&wrong).unwrap_err(),
        GeneratorError::InvalidParameter { .. }
    ));
}

#[test]
fn an_authored_courtyard_is_kept_and_a_missing_one_falls_back_to_pad_extents() {
    let registry = registry();
    let mut authored = definition("test/switch", PartKind::Switch, json!({}));
    let polygon = vec![
        Vec2 { x: -20.0, y: -19.0 },
        Vec2 { x: 20.0, y: -19.0 },
        Vec2 { x: 20.0, y: 19.0 },
    ];
    authored.courtyard = polygon.clone();
    authored.envelope_source = Some(EnvelopeSource {
        courtyard: Some(EnvelopeOrigin::Authored),
        keycap: None,
    });
    let kept = registry.normalize(&authored).unwrap().unwrap();
    assert_eq!(kept.courtyard, polygon);
    assert_eq!(
        kept.envelope_source.courtyard,
        Some(EnvelopeOrigin::Authored)
    );

    authored.courtyard.truncate(2);
    assert_eq!(
        registry
            .normalize(&authored)
            .unwrap()
            .unwrap()
            .courtyard
            .len(),
        4,
        "a two-point authored courtyard is not valid"
    );

    let bare = registry
        .normalize(&definition("test/bare", PartKind::Custom, json!({})))
        .unwrap()
        .unwrap();
    assert_eq!(
        bare.courtyard,
        [
            Vec2 { x: 2.0, y: -4.5 },
            Vec2 { x: 4.0, y: -4.5 },
            Vec2 { x: 4.0, y: -3.5 },
            Vec2 { x: 2.0, y: -3.5 }
        ]
    );
    assert_eq!(
        bare.envelope_notice.as_deref(),
        Some(
            "No closed courtyard is available; the outline uses physical graphics and pad extents."
        )
    );
    let utility = registry
        .normalize(&definition("test/utility", PartKind::Utility, json!({})))
        .unwrap()
        .unwrap();
    assert_eq!(
        utility.envelope_notice, None,
        "utilities are not warned about a missing courtyard"
    );
}

#[test]
fn the_catalogue_lists_generators_as_definitions() {
    let entries = registry().catalogue().unwrap();
    let sources: Vec<&str> = entries
        .iter()
        .map(|e| e.generator.source.as_str())
        .collect();
    assert_eq!(sources, ["test/bare", "test/switch", "test/utility"]);
    let switch = serde_json::to_value(&entries[1]).unwrap();
    assert_eq!(switch["id"], "ergogen:test/switch");
    assert_eq!(switch["name"], "test switch");
    assert_eq!(switch["kind"], "switch");
    assert_eq!(switch["keycap"], json!({ "x": 18.0, "y": 18.0 }));
    assert_eq!(
        switch["matrixTerminals"],
        json!({ "row": "from", "column": "to" })
    );
    assert_eq!(
        switch["generator"],
        json!({ "source": "test/switch", "version": "bundled-1", "parameters": {} })
    );
    assert_eq!(
        switch["envelopeSource"],
        json!({ "courtyard": "generated", "keycap": "generated" })
    );
    assert_eq!(
        switch["terminals"],
        json!({ "from": ["pad-0"], "to": ["pad-1"] })
    );
    assert!(switch.get("envelopeNotice").is_none());
    assert!(
        serde_json::to_value(&entries[0])
            .unwrap()
            .get("matrixTerminals")
            .is_none()
    );
}

#[test]
fn side_is_a_parameter_and_mirroring_follows_the_part() {
    let registry = registry();
    let part = |side: Side| PartRef {
        id: "p".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 1.0, y: 2.0 },
            rotation: 0.0,
        },
        side,
        generator_parameters: None,
    };
    let text = |part: &PartRef| {
        let forms = registry
            .render(
                "d",
                &generator("test/switch", json!({})),
                Some(part),
                &mut NoNets,
            )
            .unwrap();
        serialize(&forms[0])
    };
    // The layer parameter is not derived from the part; coordinates still follow it.
    assert_eq!(text(&part(Side::Front)), text(&part(Side::Back)));
}

#[test]
fn export_separates_objects_renames_modules_and_rejects_other_forms() {
    let forms =
        parse_forms("(module \"m\" (at 0 0)) (segment (start 0 0) (end 1 1)) (gr_arc (start 0 0))")
            .unwrap();
    let exported = export_forms(forms, &BTreeMap::new()).unwrap();
    assert_eq!(exported.footprints, ["(footprint \"m\" (at 0 0))"]);
    assert_eq!(exported.objects.len(), 2);
    let error = export_forms(parse_forms("(mystery)").unwrap(), &BTreeMap::new()).unwrap_err();
    assert_eq!(error.message(), "Unsupported Ergogen output form: mystery");

    let model = "(footprint \"m\" (model \"${KIPRJMOD}/models/boardstudio/a.step\"))";
    let paths = BTreeMap::from([(
        "ergogen:model:a.step".to_owned(),
        "models/a.step".to_owned(),
    )]);
    let exported = export_forms(parse_forms(model).unwrap(), &paths).unwrap();
    assert_eq!(
        exported.footprints,
        ["(footprint \"m\" (model \"${KIPRJMOD}/models/a.step\"))"]
    );
    for unsafe_path in [
        "/abs.step",
        "../up.step",
        "back\\slash.step",
        "C:/win.step",
        "",
    ] {
        let paths = BTreeMap::from([("ergogen:model:a.step".to_owned(), unsafe_path.to_owned())]);
        let error = export_forms(parse_forms(model).unwrap(), &paths).unwrap_err();
        assert_eq!(
            error.message(),
            "Ergogen model has no safe exported path: ${KIPRJMOD}/models/boardstudio/a.step",
            "{unsafe_path:?}"
        );
    }
}
