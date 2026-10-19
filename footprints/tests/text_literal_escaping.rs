//! Free-text generator fields keep their literal value through the render seam.
//!
//! The parser decodes KiCad strings with the project's serialized string
//! representation, so a user's quote, backslash or Unicode text must be escaped
//! once at emission and come back unchanged.
use boardstudio_footprints::nets::NoNets;
use boardstudio_footprints::sexpr::{child, Expr};
use boardstudio_footprints::types::{GeneratorRef, PartRef, Pose2, Side, Vec2};
use boardstudio_footprints::{bundled, Result};
use serde_json::{json, Value};

const LITERALS: &[&str] = &[
    "Board 12\" inch",
    r"back\slash and \n stays literal",
    "Résumé — 日本語 ✓",
    "line one\nline two\tend",
    r#"mixed "quote" \ and \\ pair"#,
];

fn render(source: &str, parameters: Value, part: Option<&PartRef>) -> Result<Vec<Expr>> {
    let generator = GeneratorRef {
        source: source.into(),
        version: "bundled-1".into(),
        parameters: serde_json::from_value(parameters).unwrap(),
    };
    bundled().render("definition", &generator, part, &mut NoNets)
}

fn placed(reference: &str) -> PartRef {
    PartRef {
        id: "part-1".into(),
        reference: reference.into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        generator_parameters: None,
    }
}

/// Strings carried by the first positional text value of `gr_text` and
/// `fp_text <kind>` forms.
fn texts(forms: &[Expr], kind: &str) -> Vec<String> {
    fn visit(node: &Expr, kind: &str, out: &mut Vec<String>) {
        let Some(list) = node.as_list() else {
            return;
        };
        if matches!(list.first(), Some(Expr::Atom(head)) if head == "gr_text")
            && let Some(Expr::Str(text)) = list.get(1)
        {
            out.push(text.clone());
        }
        if matches!(
            (list.first(), list.get(1)),
            (Some(Expr::Atom(head)), Some(Expr::Atom(name))) if head == "fp_text" && name == kind
        ) && let Some(Expr::Str(text)) = list.get(2)
        {
            out.push(text.clone());
        }
        for item in list {
            visit(item, kind, out);
        }
    }
    let mut out = Vec::new();
    for form in forms {
        visit(form, kind, &mut out);
    }
    out
}

fn layer_values(forms: &[Expr]) -> Vec<String> {
    fn visit(node: &Expr, out: &mut Vec<String>) {
        let Some(list) = node.as_list() else {
            return;
        };
        if matches!(list.first(), Some(Expr::Atom(head)) if head == "layer")
            && let Some(Expr::Str(layer)) = list.get(1)
        {
            out.push(layer.clone());
        }
        for item in list {
            visit(item, out);
        }
    }
    let mut out = Vec::new();
    for form in forms {
        visit(form, &mut out);
    }
    out
}

fn face_names(forms: &[Expr]) -> Vec<String> {
    fn visit(node: &Expr, out: &mut Vec<String>) {
        let Some(list) = node.as_list() else {
            return;
        };
        if matches!(list.first(), Some(Expr::Atom(head)) if head == "face")
            && let Some(Expr::Str(name)) = list.get(1)
        {
            out.push(name.clone());
        }
        for item in list {
            visit(item, out);
        }
    }
    let mut out = Vec::new();
    for form in forms {
        visit(form, &mut out);
    }
    out
}

#[test]
fn both_text_generators_emit_the_literal_text_on_front_and_back() {
    let face = r#"Noto "Sans" \ Regular"#;
    for source in ["ceoloide/utility_text", "infused-kim/text"] {
        for side in ["F", "B"] {
            for literal in LITERALS {
                let parameters = if source == "ceoloide/utility_text" {
                    json!({ "side": side, "reversible": true, "text": literal, "face": face })
                } else {
                    json!({ "side": side, "reverse": true, "text": literal })
                };
                let forms = render(source, parameters, None)
                    .unwrap_or_else(|error| panic!("{source}: {error}"));
                let emitted = texts(&forms, "");
                assert_eq!(
                    emitted,
                    vec![(*literal).to_owned(), (*literal).to_owned()],
                    "{source} {side}"
                );
                if source == "ceoloide/utility_text" {
                    assert_eq!(face_names(&forms), vec![face.to_owned(), face.to_owned()]);
                }
            }
        }
    }
}

#[test]
fn plain_text_controls_keep_their_existing_output() {
    let forms = render(
        "ceoloide/utility_text",
        json!({ "side": "F", "text": "Board 12 inch" }),
        None,
    )
    .unwrap();
    assert_eq!(texts(&forms, ""), ["Board 12 inch"]);
    let form = forms[0].as_list().unwrap();
    assert_eq!(child(form, "layer").unwrap()[1], Expr::string("F.SilkS"));
}

#[test]
fn logo_layer_parameter_is_literal_on_both_sides() {
    let layer = "Silk\"S\\X";
    let forms = render(
        "ceoloide/utility_logo",
        json!({ "layer": layer, "reversible": true }),
        None,
    )
    .unwrap();
    let layers = layer_values(&forms);
    assert!(layers.contains(&format!("F.{layer}")));
    assert!(layers.contains(&format!("B.{layer}")));
}

#[test]
fn bare_references_stay_atoms_until_quoting_is_needed() {
    let plain = render(
        "infused-kim/point_debugger",
        json!({}),
        Some(&placed("P1")),
    )
    .unwrap();
    assert!(texts(&plain, "reference").is_empty());
    assert!(plain
        .iter()
        .map(boardstudio_footprints::sexpr::serialize)
        .any(|form| form.contains("(fp_text reference P1 ")));

    let quoted = r#"P"1\X"#;
    let forms = render(
        "infused-kim/point_debugger",
        json!({}),
        Some(&placed(quoted)),
    )
    .unwrap();
    assert_eq!(texts(&forms, "reference"), vec![quoted.to_owned()]);
}

#[test]
fn pad_row_labels_and_reference_are_literal_on_front_and_back() {
    let reference = r#"P"1\X"#;
    for side in ["F", "B"] {
        for literal in LITERALS {
            let forms = render(
                "infused-kim/pads",
                json!({
                    "side": side, "reverse": true, "pads": 2,
                    "label_1": literal, "label_2": literal,
                }),
                Some(&placed(reference)),
            )
            .unwrap_or_else(|error| panic!("pads {side}: {error}"));
            let labels = texts(&forms, "user");
            assert_eq!(labels, vec![(*literal).to_owned(); 4], "pads {side}");
            assert_eq!(
                texts(&forms, "reference"),
                vec![reference.to_owned()],
                "pads {side}"
            );
        }
    }
}

#[test]
fn smd_labels_and_reference_are_literal_on_front_and_back() {
    let reference = r#"S"1\Y"#;
    for side in ["F", "B"] {
        for literal in LITERALS {
            let forms = render(
                "infused-kim/smd_0805",
                json!({
                    "side": side, "reverse": true, "components": 2,
                    "label_1": literal, "label_2": literal,
                }),
                Some(&placed(reference)),
            )
            .unwrap_or_else(|error| panic!("smd {side}: {error}"));
            let labels = texts(&forms, "user");
            assert_eq!(labels, vec![(*literal).to_owned(); 8], "smd {side}");
            assert_eq!(
                texts(&forms, "reference"),
                vec![reference.to_owned()],
                "smd {side}"
            );
        }
    }
}
