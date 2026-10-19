//! Lint over every registered generator's parameter declarations (generator-parameters
//! ticket 05, ADR-0003 and ADR-0009).
//!
//! The lint is a plain function over one `GeneratorSpec`, so the negative tests feed it
//! deliberately wrong declarations that no registry holds.
//!
//! Three rules (unique names, a default of the declared kind, `side` as text defaulting to `F`)
//! are also enforced when the registry is built, which stops the whole program. The lint repeats
//! them so one run reports every problem at once, and a test shows the registry refuses the same
//! declarations. The two rules only the lint holds are the name rule and the numeric-text rule.
//!
//! Names are snake_case, with two exemptions that follow from the declaration, not from a list:
//! a `net` parameter may be a pad or pin name in capitals (`GND`, `P21`, `BAT_P`), because the
//! saved key is the pad's own name; and a text parameter `<NET>_label` is the label of that
//! net's pin. Renaming either is out of scope (generator-parameters map). The number of
//! parameters each exemption covers is pinned, so a new one is a reviewed change.
use boardstudio_footprints::Result;
use boardstudio_footprints::bundled;
use boardstudio_footprints::context::RenderContext;
use boardstudio_footprints::params::{DefaultValue, ParamKind, ParamSpec};
use boardstudio_footprints::registry::{GeneratorSpec, License, Registry};
use boardstudio_footprints::types::PartKind;

/// An exception to a rule: `(generator, parameter, reason)`.
type Exception = (&'static str, &'static str, &'static str);

/// String parameters declared for a numeric-looking default or a dimension name. Empty: the
/// audit of the bundled generators found none. Each entry needs a reason; a new numeric string is
/// a mistyped number unless the reason says why it is text.
const NUMERIC_STRING: &[Exception] = &[];

/// Pad or pin names in capitals that the bundled `net` parameters use as their saved key.
const PAD_NAMES: usize = 88;
/// Pin labels (`<NET>_label`) that the bundled generators declare for those pins.
const PIN_LABELS: usize = 50;

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
    /// Not snake_case, and not a pad name on a `net` or a label of one.
    BadName(String),
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

/// How a parameter name is accepted.
#[derive(Debug, PartialEq)]
enum NameClass {
    Snake,
    /// A `net` parameter named for its pad or pin, in capitals.
    PadName,
    /// A text parameter `<NET>_label`, the label of a declared net's pin.
    PinLabel,
    Bad,
}

fn name_class(spec: &GeneratorSpec, param: &ParamSpec) -> NameClass {
    if is_snake_case(param.name) {
        return NameClass::Snake;
    }
    match param.kind {
        ParamKind::Net if is_pad_name(param.name) => NameClass::PadName,
        ParamKind::String => match param.name.strip_suffix("_label") {
            Some(stem)
                if spec.parameters.iter().any(|other| {
                    other.name == stem && other.kind == ParamKind::Net && is_pad_name(other.name)
                }) =>
            {
                NameClass::PinLabel
            }
            _ => NameClass::Bad,
        },
        _ => NameClass::Bad,
    }
}

/// Every problem of one declaration, before the exception list applies.
fn raw_problems(spec: &GeneratorSpec) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for param in spec.parameters {
        if seen.contains(&param.name) {
            problems.push(Problem::DuplicateName(param.name.into()));
        }
        seen.push(param.name);
        if name_class(spec, param) == NameClass::Bad {
            problems.push(Problem::BadName(param.name.into()));
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

/// The problems of one declaration that the numeric-text exceptions do not cover.
fn lint_with(spec: &GeneratorSpec, numeric_exceptions: &[Exception]) -> Vec<Problem> {
    raw_problems(spec)
        .into_iter()
        .filter(|problem| match problem {
            Problem::NumericString(parameter) => numeric_exceptions
                .iter()
                .all(|(source, name, _)| !(*source == spec.source && name == parameter)),
            _ => true,
        })
        .collect()
}

fn lint(spec: &GeneratorSpec) -> Vec<Problem> {
    lint_with(spec, NUMERIC_STRING)
}

/// Exceptions with no reason, or that no longer match a real numeric-text declaration.
fn invalid_exceptions(registry: &Registry, list: &[Exception]) -> Vec<String> {
    let mut invalid = Vec::new();
    for (generator, parameter, reason) in list {
        if reason.trim().is_empty() {
            invalid.push(format!("{generator} {parameter}: no reason"));
        }
        match registry.get(generator) {
            None => invalid.push(format!("{generator} {parameter}: generator not registered")),
            Some(spec) => {
                if !raw_problems(spec).contains(&Problem::NumericString((*parameter).into())) {
                    invalid.push(format!("{generator} {parameter}: no longer numeric text"));
                }
            }
        }
    }
    invalid
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

/// Capital letters, digits and single underscores, starting with a capital: `GND`, `P21`, `BAT_P`.
fn is_pad_name(name: &str) -> bool {
    matches!(name.chars().next(), Some('A'..='Z'))
        && !name.contains("__")
        && !name.ends_with('_')
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
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
        body: boardstudio_footprints::registry::Body::Text(unused_body),
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

static CORRECT: [ParamSpec; 6] = [
    ParamSpec::SIDE,
    ParamSpec::number("hole_size", 2.2),
    ParamSpec::number("hole_drill", 3.0),
    ParamSpec::net("GND"),
    ParamSpec::string("GND_label", "GND"),
    ParamSpec::string("note", "see datasheet"),
];

static DUPLICATE: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::number("hole_size", 2.2),
    ParamSpec::number("hole_size", 3.0),
];

static CAMEL: [ParamSpec; 2] = [ParamSpec::SIDE, ParamSpec::number("holeSize", 2.2)];

// Only a `net` may be named for a pad; a number or text in capitals is a bad name.
static CAPITALS_NOT_A_NET: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::number("GND", 1.0),
    ParamSpec::string("VCC", "x"),
];

// A net that is neither snake_case nor capitals.
static MIXED_CASE_NET: [ParamSpec; 4] = [
    ParamSpec::SIDE,
    ParamSpec::net("Gnd"),
    ParamSpec::net("netName"),
    ParamSpec::string("Gnd_label", "Gnd"),
];

// A label needs a declared net of that name, and must be text.
static ORPHAN_LABEL: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::net("GND"),
    ParamSpec::string("VCC_label", "VCC"),
];

static LABEL_NOT_TEXT: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::net("GND"),
    ParamSpec::number("GND_label", 1.0),
];

static NO_SIDE: [ParamSpec; 1] = [ParamSpec::number("hole_size", 2.2)];

static SIDE_BACK: [ParamSpec; 1] = [ParamSpec::string("side", "B")];

static SIDE_NUMBER: [ParamSpec; 1] = [ParamSpec {
    name: "side",
    kind: ParamKind::Number,
    default: DefaultValue::Num(0.0),
}];

// `side` as a net with the right default text: the kind is wrong.
static SIDE_NET: [ParamSpec; 1] = [ParamSpec::net_default("side", "F")];

static NO_DEFAULT: [ParamSpec; 2] = [
    ParamSpec::SIDE,
    ParamSpec {
        name: "hole_size",
        kind: ParamKind::Number,
        default: DefaultValue::None,
    },
];

// Numeric text by default alone, and by dimension name alone.
static TEXT_DEFAULT_IS_A_NUMBER: [ParamSpec; 2] =
    [ParamSpec::SIDE, ParamSpec::string("label", "3")];
static TEXT_NAMED_FOR_A_DIMENSION: [ParamSpec; 2] =
    [ParamSpec::SIDE, ParamSpec::string("mount_width", "wide")];
static TEXT_NOT_A_DIMENSION: [ParamSpec; 3] = [
    ParamSpec::SIDE,
    ParamSpec::string("widget", "wide"),
    ParamSpec::string("sizing_note", "text"),
];

fn problems(parameters: &'static [ParamSpec]) -> Vec<Problem> {
    lint(&synthetic(parameters))
}

#[test]
fn every_registered_generator_passes_the_parameter_lint() {
    let specs = bundled().specs();
    assert!(!specs.is_empty(), "the registry holds no generator");
    let mut failures = Vec::new();
    for spec in specs {
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
    let problems = problems(&MISTYPED);
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
    assert_eq!(problems(&CORRECT), Vec::new());
    assert_eq!(problems(&TEXT_NOT_A_DIMENSION), Vec::new());
}

#[test]
fn the_lint_reports_duplicate_names() {
    assert!(problems(&DUPLICATE).contains(&Problem::DuplicateName("hole_size".into())));
}

#[test]
fn the_lint_reports_names_that_are_neither_snake_case_nor_a_pad_name_or_its_label() {
    assert!(problems(&CAMEL).contains(&Problem::BadName("holeSize".into())));
    let capitals = problems(&CAPITALS_NOT_A_NET);
    assert!(
        capitals.contains(&Problem::BadName("GND".into())),
        "{capitals:?}"
    );
    assert!(
        capitals.contains(&Problem::BadName("VCC".into())),
        "{capitals:?}"
    );
    let mixed = problems(&MIXED_CASE_NET);
    assert!(mixed.contains(&Problem::BadName("Gnd".into())), "{mixed:?}");
    assert!(
        mixed.contains(&Problem::BadName("netName".into())),
        "{mixed:?}"
    );
    assert!(
        mixed.contains(&Problem::BadName("Gnd_label".into())),
        "{mixed:?}"
    );
    assert!(problems(&ORPHAN_LABEL).contains(&Problem::BadName("VCC_label".into())));
    assert!(problems(&LABEL_NOT_TEXT).contains(&Problem::BadName("GND_label".into())));
}

#[test]
fn the_name_helpers_classify_edge_cases() {
    for name in ["a", "a1", "hole_size", "net_1_from"] {
        assert!(is_snake_case(name), "{name}");
    }
    for name in ["", "A", "_a", "a_", "a__b", "aB", "1a", "P21"] {
        assert!(!is_snake_case(name), "{name}");
    }
    for name in ["A", "GND", "P21", "BAT_P"] {
        assert!(is_pad_name(name), "{name}");
    }
    for name in ["", "1P", "_P", "P_", "P__1", "Gnd", "gnd"] {
        assert!(!is_pad_name(name), "{name}");
    }
}

#[test]
fn the_lint_requires_side_as_text_with_default_f() {
    assert!(problems(&NO_SIDE).contains(&Problem::SideNotFront));
    assert!(problems(&SIDE_BACK).contains(&Problem::SideNotFront));
    assert!(problems(&SIDE_NUMBER).contains(&Problem::SideNotFront));
    assert!(problems(&SIDE_NET).contains(&Problem::SideNotFront));
}

#[test]
fn the_lint_reports_a_number_with_no_default() {
    assert!(problems(&NO_DEFAULT).contains(&Problem::DefaultRejected {
        parameter: "hole_size".into(),
        kind: ParamKind::Number,
    }));
}

#[test]
fn numeric_text_is_found_by_its_default_and_by_its_name_separately() {
    assert_eq!(
        problems(&TEXT_DEFAULT_IS_A_NUMBER),
        vec![Problem::NumericString("label".into())]
    );
    assert_eq!(
        problems(&TEXT_NAMED_FOR_A_DIMENSION),
        vec![Problem::NumericString("mount_width".into())]
    );
}

#[test]
fn a_numeric_text_exception_applies_only_to_its_generator_and_parameter() {
    let spec = synthetic(&TEXT_DEFAULT_IS_A_NUMBER);
    let own: &[Exception] = &[("test/synthetic", "label", "kept as text on purpose")];
    let other_generator: &[Exception] = &[("test/other", "label", "elsewhere")];
    let other_parameter: &[Exception] = &[("test/synthetic", "other", "elsewhere")];
    assert_eq!(lint_with(&spec, own), Vec::new());
    for list in [other_generator, other_parameter] {
        assert_eq!(
            lint_with(&spec, list),
            vec![Problem::NumericString("label".into())]
        );
    }
}

#[test]
fn the_registry_refuses_what_the_lint_reports_for_its_own_rules() {
    // Registry::new runs when the program starts, so these never reach the real-registry test.
    for parameters in [&MISTYPED[..], &DUPLICATE, &NO_SIDE, &SIDE_BACK, &SIDE_NET] {
        let spec: &'static GeneratorSpec = Box::leak(Box::new(synthetic(parameters)));
        assert!(
            Registry::new(&[spec]).is_err(),
            "the registry accepted {parameters:?}"
        );
    }
    let correct: &'static GeneratorSpec = Box::leak(Box::new(synthetic(&CORRECT)));
    assert!(Registry::new(&[correct]).is_ok());
}

#[test]
fn every_exception_has_a_reason_and_still_matches_a_real_declaration() {
    assert_eq!(
        invalid_exceptions(bundled(), NUMERIC_STRING),
        Vec::<String>::new()
    );
}

#[test]
fn the_exception_check_reports_a_stale_or_reasonless_entry() {
    // `side` is a real parameter of a real generator, but it is not numeric text.
    let stale: &[Exception] = &[("ceoloide/switch_mx", "side", "was numeric text")];
    let reasonless: &[Exception] = &[("ceoloide/switch_mx", "side", "  ")];
    let unregistered: &[Exception] = &[("no/such_generator", "x", "gone")];
    assert_eq!(invalid_exceptions(bundled(), stale).len(), 1);
    assert_eq!(invalid_exceptions(bundled(), reasonless).len(), 2);
    assert_eq!(invalid_exceptions(bundled(), unregistered).len(), 1);
}

#[test]
fn the_numeric_text_list_is_sorted_and_unique() {
    let mut sorted = NUMERIC_STRING.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, NUMERIC_STRING);
}

#[test]
fn pad_names_and_pin_labels_are_pinned() {
    let (mut pad_names, mut pin_labels) = (0, 0);
    for spec in bundled().specs() {
        for param in spec.parameters {
            match name_class(spec, param) {
                NameClass::PadName => pad_names += 1,
                NameClass::PinLabel => pin_labels += 1,
                NameClass::Snake | NameClass::Bad => {}
            }
        }
    }
    // Growing either is a reviewed change: update the count with the new generator.
    assert_eq!(pad_names, PAD_NAMES, "capital pad names on net parameters");
    assert_eq!(pin_labels, PIN_LABELS, "pin labels of those pads");
    assert_eq!(NUMERIC_STRING.len(), 0, "numeric-text exceptions");
}
