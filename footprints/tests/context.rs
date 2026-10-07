//! The render context against the real JavaScript context. A synthetic generator
//! echoes every field and helper; the expected text was recorded from the
//! JavaScript provider running the same body.
mod common;

use boardstudio_footprints::context::RenderContext;
use boardstudio_footprints::nets::{NetAllocator, NetIndexer, NoNets};
use boardstudio_footprints::number::js_number;
use boardstudio_footprints::params::ParamSpec;
use boardstudio_footprints::registry::{GeneratorSpec, License, Registry};
use boardstudio_footprints::sexpr::serialize;
use boardstudio_footprints::types::{GeneratorRef, PartKind, PartRef, ReservedNet, Side};
use boardstudio_footprints::{GeneratorError, Result};
use common::read_json;
use serde_json::Value;

static PARAMS: [ParamSpec; 12] = [
    ParamSpec::SIDE,
    ParamSpec::boolean("flag", false),
    ParamSpec::number("count", 3.0),
    ParamSpec::number("ratio", 1.5),
    ParamSpec::string("label", "hello"),
    ParamSpec::array("items", "[1,2,3]"),
    ParamSpec::array("grid", "[[0,0],[1,2.5]]"),
    ParamSpec::net_default("in", "DEFAULT_NET"),
    ParamSpec::net_default("out", ""),
    ParamSpec::net("open"),
    ParamSpec::number("big", 1e21),
    ParamSpec::number("tiny", 1e-7),
];

fn numbers(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|v| js_number(v.as_f64().unwrap()))
        .collect()
}

fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

fn body(p: &RenderContext<'_>) -> Result<String> {
    let point = p.point();
    let layer = if p.side() == "B" { "B.Cu" } else { "F.Cu" };
    let grid: Vec<String> = p
        .list("grid")
        .iter()
        .map(|row| numbers(row.as_array().unwrap()).join(" "))
        .collect();
    let (input, output, open) = (p.net("in"), p.net("out"), p.net("open"));
    let mut text = format!(
        "\n(footprint \"test:context\" (layer \"{layer}\") {at}\n  (x {x}) (y {y}) (r {r}) (rot {r}) (xy {xy}) (side {side})\n  (ref \"{reference}\") (hide \"{hide}\")\n  (point {px} {py} {pr} {mirrored})\n  (isxy {isxy}) (isxy0 {isxy0}) (iaxy {iaxy})\n  (esxy {esxy}) (esxy0 {esxy0}) (eaxy {eaxy}) (eaxy2 {eaxy2})\n  (flag {flag}) (count {count}) (ratio {ratio}) (label {label})\n  (items {items}) (grid {grid})\n  (big {big}) (tiny {tiny})\n  {input} {output} {open}\n  (in {in_index} {in_name}) (out {out_index} {out_name}) (open {open_index} {open_name})\n  ",
        at = p.at(),
        x = js_number(p.x()),
        y = js_number(p.y()),
        r = js_number(p.rotation()),
        xy = p.xy(),
        side = p.side(),
        reference = p.reference(),
        hide = p.ref_hide(),
        px = js_number(point.x),
        py = js_number(point.y),
        pr = js_number(point.r),
        mirrored = point.mirrored,
        isxy = p.isxy(1.5, -2.25),
        isxy0 = p.isxy(0.0, 0.0),
        iaxy = p.iaxy(1.5, -2.25),
        esxy = p.esxy(1.5, -2.25),
        esxy0 = p.esxy(0.0, 0.0),
        eaxy = p.eaxy(3.0, 4.0),
        eaxy2 = p.eaxy(-7.5, 0.125),
        flag = p.flag("flag"),
        count = js_number(p.number("count")),
        ratio = js_number(p.number("ratio")),
        label = json_string(p.text("label")),
        items = numbers(p.list("items")).join(" "),
        grid = grid.join(" / "),
        big = js_number(p.number("big")),
        tiny = js_number(p.number("tiny")),
        in_index = input.index,
        in_name = json_string(&input.name),
        out_index = output.index,
        out_name = json_string(&output.name),
        open_index = open.index,
        open_name = json_string(&open.name),
    );
    text.push_str(&format!(
        "{} {} {}\n)",
        p.local_net("a b")?,
        p.local_net("é")?,
        p.local_net("a b")?
    ));
    Ok(text)
}

static SPEC: GeneratorSpec = GeneratorSpec {
    source: "test/context",
    display_name: "context",
    kind: PartKind::Custom,
    matrix_terminals: None,
    keycap_parameters: None,
    parameters: &PARAMS,
    license: License {
        spdx: "MIT",
        author: "test",
    },
    body,
};

fn render(case: &Value) -> (Result<Vec<String>>, Option<Vec<ReservedNet>>) {
    let registry = Registry::new(&[&SPEC]).unwrap();
    let input = &case["input"];
    let generator = GeneratorRef {
        source: "test/context".into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(input["definitionParameters"].clone()).unwrap(),
    };
    let mut part: Option<PartRef> =
        (!input["part"].is_null()).then(|| serde_json::from_value(input["part"].clone()).unwrap());
    // The migration gives back-side parts the `side` the old provider derived.
    if let Some(part) = &mut part {
        let params = part
            .generator_parameters
            .get_or_insert_with(Default::default);
        let saved = params.contains_key("side") || generator.parameters.contains_key("side");
        if part.side == Side::Back && !saved {
            params.insert("side".into(), Value::String("B".into()));
        }
    }
    let id = input["definitionId"].as_str().unwrap();
    if input["reservedNets"].is_null() {
        let forms = registry.render(id, &generator, part.as_ref(), &mut NoNets);
        (
            forms.map(|forms| forms.iter().map(serialize).collect()),
            None,
        )
    } else {
        let reserved: Vec<ReservedNet> =
            serde_json::from_value(input["reservedNets"].clone()).unwrap();
        let mut nets = NetAllocator::new(&reserved, input["nextNetIndex"].as_u64().unwrap() as u32);
        let forms = registry.render(
            id,
            &generator,
            part.as_ref(),
            &mut nets as &mut dyn NetIndexer,
        );
        (
            forms.map(|forms| forms.iter().map(serialize).collect()),
            Some(nets.snapshot()),
        )
    }
}

#[test]
fn the_context_matches_the_javascript_provider() {
    let vectors = read_json("context_vectors.json");
    let mut compared = 0;
    for case in vectors["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let (forms, nets) = render(case);
        if id == "nets:numeric-name" {
            // Net names must be strings now; the old provider stringified 5.
            assert!(
                matches!(forms, Err(GeneratorError::InvalidParameter { .. })),
                "{id}"
            );
            continue;
        }
        if id == "nets:index-limit" {
            // The recording allocator has no limit; the real worker's does.
            assert_eq!(
                forms.unwrap_err().message(),
                "Allocated net index exceeds the 32-bit range"
            );
            continue;
        }
        let expected = &case["output"];
        let forms = forms.unwrap_or_else(|error| panic!("{id}: {error}"));
        let recorded: Vec<&str> = expected["forms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect();
        assert_eq!(forms, recorded, "{id}");
        if let Some(nets) = nets {
            let nets: Vec<Value> = nets
                .iter()
                .map(|n| serde_json::to_value(n).unwrap())
                .collect();
            common::assert_json_eq(&Value::Array(nets), &expected["nets"], id);
        }
        compared += 1;
    }
    assert!(compared >= 38, "only {compared} context cases compared");
}

#[test]
fn unknown_generators_and_versions_keep_their_baseline_text() {
    let vectors = read_json("context_vectors.json");
    let registry = Registry::new(&[&SPEC]).unwrap();
    let mut generator = GeneratorRef {
        source: "test/missing".into(),
        version: "bundled-1".into(),
        parameters: Default::default(),
    };
    let error = registry
        .render("x", &generator, None, &mut NoNets)
        .unwrap_err();
    assert_eq!(
        error.message(),
        vectors["unknownGenerator"]["error"].as_str().unwrap()
    );
    assert!(matches!(error, GeneratorError::UnknownGenerator { .. }));
    generator.source = "test/context".into();
    generator.version = "future".into();
    let error = registry
        .render("x", &generator, None, &mut NoNets)
        .unwrap_err();
    assert_eq!(
        error.message(),
        vectors["unsupportedVersion"]["error"].as_str().unwrap()
    );
}
