//! Lint over every registered generator's parameter declarations (generator-parameters
//! ticket 05, ADR-0003 and ADR-0009).
//!
//! The lint is a plain function over one `GeneratorSpec`, so the negative tests feed it a
//! deliberately mistyped declaration that no registry holds. Exceptions are listed below with one
//! reason per entry, and each entry must still match a real declaration, so a stale entry fails.
use boardstudio_footprints::Result;
use boardstudio_footprints::bundled;
use boardstudio_footprints::context::RenderContext;
use boardstudio_footprints::params::{DefaultValue, ParamKind, ParamSpec};
use boardstudio_footprints::registry::{GeneratorSpec, License};
use boardstudio_footprints::types::PartKind;

/// Parameter names that are not snake_case, as `(generator, parameter)`. Each one is an existing
/// name that saved documents hold; renaming the vocabulary is out of scope (generator-parameters
/// map). Net names are the pad or pin the net connects; `*_label` is the text of a pin label.
const NOT_SNAKE_CASE: &[(&str, &str)] = &[
    ("ceoloide/battery_connector_jst_ph_2", "BAT_N"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/battery_connector_jst_ph_2", "BAT_P"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/battery_connector_molex_pico_ezmate_1x02", "BAT_N"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/battery_connector_molex_pico_ezmate_1x02", "BAT_P"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_nice_view", "CS"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_nice_view", "GND"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_nice_view", "MOSI"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_nice_view", "SCK"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_nice_view", "VCC"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_ssd1306", "GND"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_ssd1306", "SCL"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_ssd1306", "SDA"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/display_ssd1306", "VCC"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/led_sk6812mini-e", "P1"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/led_sk6812mini-e", "P2"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/led_sk6812mini-e", "P3"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/led_sk6812mini-e", "P4"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "GND"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "GND_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P0"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P0_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P1"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P10"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P101"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P101_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P102"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P102_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P107"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P107_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P10_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P14"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P14_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P15"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P15_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P16"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P16_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P18"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P18_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P19"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P19_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P1_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P2"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P20"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P20_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P21"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P21_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P2_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P3"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P3_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P4"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P4_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P5"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P5_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P6"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P6_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P7"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P7_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P8"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P8_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P9"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "P9_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "RAW"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "RAW_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "RST"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "RST_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "VCC"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_nice_nano", "VCC_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "GND"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "GND_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P0"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P0_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P1"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P10"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P101"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P101_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P102"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P102_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P107"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P107_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P10_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P14"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P14_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P15"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P15_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P16"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P16_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P18"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P18_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P19"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P19_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P1_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P2"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P20"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P20_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P21"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P21_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P2_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P3"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P3_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P4"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P4_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P5"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P5_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P6"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P6_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P7"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P7_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P8"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P8_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P9"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "P9_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "RAW"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "RAW_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "RST"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "RST_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "VCC"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/mcu_supermini_nrf52840", "VCC_label"), // Pin label text: the pin name is the saved key; renaming is out of scope (map).
    ("ceoloide/rotary_encoder_ec11_ec12", "A"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/rotary_encoder_ec11_ec12", "B"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/rotary_encoder_ec11_ec12", "C"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/rotary_encoder_ec11_ec12", "S1"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/rotary_encoder_ec11_ec12", "S2"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_choc_v1_v2", "CENTERHOLE"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_choc_v1_v2", "LEFTSTAB"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_choc_v1_v2", "RIGHTSTAB"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_gateron_ks27_ks33", "CENTERHOLE"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_mx", "CENTERHOLE"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_mx", "LEFTSTAB"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/switch_mx", "RIGHTSTAB"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/trrs_pj320a", "R1"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/trrs_pj320a", "R2"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/trrs_pj320a", "SL"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("ceoloide/trrs_pj320a", "TP"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("infused-kim/nice_view", "CS"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("infused-kim/nice_view", "GND"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("infused-kim/nice_view", "MOSI"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("infused-kim/nice_view", "SCK"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
    ("infused-kim/nice_view", "VCC"), // Net terminal: the pad name is the saved key; renaming is out of scope (map).
];

/// String parameters declared for a numeric-looking default or a dimension name, as
/// `(generator, parameter)`. Empty: the audit found none in the bundled generators.
const NUMERIC_STRING: &[(&str, &str)] = &[];

/// Dimension words that a string parameter must not end with: a length, angle or scale declared
/// as text is a mistyped number.
const DIMENSION_WORDS: &[&str] = &[
    "size",
    "drill",
    "width",
    "height",
    "diameter",
    "radius",
    "thickness",
    "clearance",
    "offset",
    "scale",
    "rotation",
    "angle",
    "gap",
    "length",
    "spacing",
];

#[derive(Debug, PartialEq)]
enum Problem {
    DuplicateName(String),
    NotSnakeCase(String),
    /// The default is missing or not accepted by the declared kind.
    DefaultRejected {
        parameter: String,
        kind: ParamKind,
    },
    /// A string declared for a number: its default reads as a number, or its name is a dimension.
    NumericString(String),
    /// `side` is missing, or is not a string whose default is `F`.
    SideNotFront,
}

/// Every problem of one declaration, before the exception lists apply.
fn raw_problems(spec: &GeneratorSpec) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for param in spec.parameters {
        if seen.contains(&param.name) {
            problems.push(Problem::DuplicateName(param.name.into()));
        }
        seen.push(param.name);
        if !is_snake_case(param.name) {
            problems.push(Problem::NotSnakeCase(param.name.into()));
        }
        if !default_fits(param) {
            problems.push(Problem::DefaultRejected {
                parameter: param.name.into(),
                kind: param.kind,
            });
        }
        if param.kind == ParamKind::String
            && (numeric_default(param.default) || dimension_name(param.name))
        {
            problems.push(Problem::NumericString(param.name.into()));
        }
    }
    let side_is_front = spec
        .parameters
        .iter()
        .find(|param| param.name == "side")
        .is_some_and(|param| {
            param.kind == ParamKind::String && param.default == DefaultValue::Str("F")
        });
    if !side_is_front {
        problems.push(Problem::SideNotFront);
    }
    problems
}

/// The problems of one declaration that no exception covers.
fn lint(spec: &GeneratorSpec) -> Vec<Problem> {
    raw_problems(spec)
        .into_iter()
        .filter(|problem| !excepted(spec.source, problem))
        .collect()
}

fn excepted(generator: &str, problem: &Problem) -> bool {
    let covered = |list: &[(&str, &str)], parameter: &str| {
        list.iter()
            .any(|(source, name)| *source == generator && *name == parameter)
    };
    match problem {
        Problem::NotSnakeCase(parameter) => covered(NOT_SNAKE_CASE, parameter),
        Problem::NumericString(parameter) => covered(NUMERIC_STRING, parameter),
        _ => false,
    }
}

/// Lowercase ASCII letters, digits and single underscores, starting with a letter.
fn is_snake_case(name: &str) -> bool {
    matches!(name.chars().next(), Some('a'..='z'))
        && !name.contains("__")
        && !name.ends_with('_')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Whether the default is present and of the declared kind. A net needs no default, or text.
fn default_fits(param: &ParamSpec) -> bool {
    match (param.kind, param.default.to_value()) {
        (ParamKind::Net, None) => true,
        (kind, Some(value)) => kind.accepts(&value),
        (_, None) => false,
    }
}

fn numeric_default(default: DefaultValue) -> bool {
    match default {
        DefaultValue::Str(text) => text
            .trim()
            .parse::<f64>()
            .is_ok_and(|number| number.is_finite()),
        _ => false,
    }
}

fn dimension_name(name: &str) -> bool {
    DIMENSION_WORDS.iter().any(|word| {
        name == *word
            || name
                .strip_suffix(word)
                .is_some_and(|rest| rest.ends_with('_'))
    })
}

fn synthetic(parameters: &'static [ParamSpec]) -> GeneratorSpec {
    GeneratorSpec {
        source: "test/synthetic",
        display_name: "synthetic",
        kind: PartKind::Custom,
        matrix_terminals: None,
        keycap_parameters: None,
        parameters,
        license: License {
            spdx: "MIT",
            author: "test",
        },
        body: unused_body,
    }
}

fn unused_body(_: &RenderContext<'_>) -> Result<String> {
    Ok(String::new())
}

static MISTYPED: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    // A number declared with a text default: the mistyped default the lint must report.
    ParamSpec {
        name: "hole_size",
        kind: ParamKind::Number,
        default: DefaultValue::Str("2.2"),
    },
    // Text declared for a number: its default reads as one.
    ParamSpec::string("hole_drill", "3"),
];

static CORRECT: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::number("hole_size", 2.2),
    ParamSpec::number("hole_drill", 3.0),
];

static DUPLICATE: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::number("hole_size", 2.2),
    ParamSpec::number("hole_size", 3.0),
];

static CAMEL: [ParamSpec; 2] = [ParamSpec::SIDE, ParamSpec::number("holeSize", 2.2)];

static NO_SIDE: [ParamSpec; 1] = [ParamSpec::number("hole_size", 2.2)];

static SIDE_BACK: [ParamSpec; 1] = [ParamSpec::string("side", "B")];

static SIDE_NUMBER: [ParamSpec; 1] = [ParamSpec {
    name: "side",
    kind: ParamKind::Number,
    default: DefaultValue::Num(0.0),
}];

static NO_DEFAULT: [ParamSpec; 2] = [
    ParamSpec::SIDE,
    ParamSpec {
        name: "hole_size",
        kind: ParamKind::Number,
        default: DefaultValue::None,
    },
];

#[test]
fn every_registered_generator_passes_the_parameter_lint() {
    let mut failures = Vec::new();
    for spec in bundled().specs() {
        for problem in lint(spec) {
            failures.push(format!("{}: {problem:?}", spec.source));
        }
    }
    assert!(
        failures.is_empty(),
        "{} problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_lint_reports_a_mistyped_default_on_a_synthetic_declaration() {
    let spec = synthetic(&MISTYPED);
    let problems = lint(&spec);
    assert!(
        problems.contains(&Problem::DefaultRejected {
            parameter: "hole_size".into(),
            kind: ParamKind::Number,
        }),
        "a number with a text default must be reported: {problems:?}"
    );
    assert!(
        problems.contains(&Problem::NumericString("hole_drill".into())),
        "text for a number must be reported: {problems:?}"
    );
}

#[test]
fn the_lint_accepts_the_corrected_synthetic_declaration() {
    assert_eq!(lint(&synthetic(&CORRECT)), Vec::new());
}

#[test]
fn the_lint_reports_duplicate_and_non_snake_case_names() {
    assert!(lint(&synthetic(&DUPLICATE)).contains(&Problem::DuplicateName("hole_size".into())));
    assert!(lint(&synthetic(&CAMEL)).contains(&Problem::NotSnakeCase("holeSize".into())));
}

#[test]
fn the_lint_requires_side_as_text_with_default_f() {
    assert!(lint(&synthetic(&NO_SIDE)).contains(&Problem::SideNotFront));
    assert!(lint(&synthetic(&SIDE_BACK)).contains(&Problem::SideNotFront));
    assert!(lint(&synthetic(&SIDE_NUMBER)).contains(&Problem::SideNotFront));
}

#[test]
fn the_lint_reports_a_number_with_no_default() {
    assert!(
        lint(&synthetic(&NO_DEFAULT)).contains(&Problem::DefaultRejected {
            parameter: "hole_size".into(),
            kind: ParamKind::Number,
        })
    );
}

#[test]
fn every_exception_still_matches_a_real_declaration() {
    for (generator, parameter) in NOT_SNAKE_CASE {
        let spec = bundled()
            .get(generator)
            .unwrap_or_else(|| panic!("{generator} is not registered"));
        assert!(
            raw_problems(spec).contains(&Problem::NotSnakeCase((*parameter).into())),
            "{generator} {parameter} is listed as not snake_case but is not"
        );
    }
    for (generator, parameter) in NUMERIC_STRING {
        let spec = bundled()
            .get(generator)
            .unwrap_or_else(|| panic!("{generator} is not registered"));
        assert!(
            raw_problems(spec).contains(&Problem::NumericString((*parameter).into())),
            "{generator} {parameter} is listed as numeric text but is not"
        );
    }
}

#[test]
fn the_exception_lists_are_sorted_unique_and_pinned() {
    for list in [NOT_SNAKE_CASE, NUMERIC_STRING] {
        let mut sorted = list.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, list, "exceptions must be sorted and unique");
    }
    // Growing either list is a reviewed change: update the count with the new entry.
    assert_eq!(NOT_SNAKE_CASE.len(), 138, "not-snake-case exceptions");
    assert_eq!(NUMERIC_STRING.len(), 0, "numeric-text exceptions");
}
