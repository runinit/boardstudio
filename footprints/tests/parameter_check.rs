//! The shared parameter check: a saved value must have its declared kind, and a name
//! must be declared. Null is absent, as render treats it.
use std::collections::BTreeMap;

use boardstudio_footprints::bundled;
use boardstudio_footprints::params::{ParamKind, ParameterProblem};
use serde_json::{Value, json};

fn check(source: &str, parameters: Value) -> Vec<ParameterProblem> {
    let map: BTreeMap<String, Value> = serde_json::from_value(parameters).unwrap();
    bundled().check_parameters(source, &map).unwrap()
}

#[test]
fn every_bundled_default_passes_its_own_check() {
    for spec in bundled().specs() {
        let defaults: BTreeMap<String, Value> = bundled()
            .parameters(spec.source)
            .unwrap()
            .into_iter()
            .filter_map(|parameter| parameter.value.map(|value| (parameter.name, value)))
            .collect();
        assert!(
            bundled()
                .check_parameters(spec.source, &defaults)
                .unwrap()
                .is_empty(),
            "{} defaults must pass the check",
            spec.source
        );
    }
}

#[test]
fn text_for_a_number_is_a_wrong_type_named_by_parameter() {
    assert_eq!(
        check("ceoloide/mounting_hole_npth", json!({ "hole_size": "2" })),
        vec![ParameterProblem::WrongType {
            parameter: "hole_size".into(),
            expected: ParamKind::Number,
            found: json!("2"),
        }]
    );
}

#[test]
fn an_undeclared_name_is_reported_and_null_is_absent() {
    assert_eq!(
        check("ceoloide/mounting_hole_npth", json!({ "hole_sise": 2.0 })),
        vec![ParameterProblem::Undeclared {
            parameter: "hole_sise".into(),
            found: json!(2.0),
        }]
    );
    assert!(check("ceoloide/mounting_hole_npth", json!({ "hole_size": null })).is_empty());
}

#[test]
fn non_empty_list_text_is_a_wrong_type_for_an_array_parameter() {
    assert_eq!(
        check(
            "ceoloide/mcu_nice_nano",
            json!({ "mcu_3dmodel_xyz_offset": "[1,2,3]" })
        ),
        vec![ParameterProblem::WrongType {
            parameter: "mcu_3dmodel_xyz_offset".into(),
            expected: ParamKind::Array,
            found: json!("[1,2,3]"),
        }]
    );
}

#[test]
fn every_problem_is_reported_in_name_order() {
    assert_eq!(
        check(
            "ceoloide/mounting_hole_npth",
            json!({ "hole_size": "2", "hole_drill": true, "zz_extra": 1 })
        ),
        vec![
            ParameterProblem::WrongType {
                parameter: "hole_drill".into(),
                expected: ParamKind::Number,
                found: json!(true),
            },
            ParameterProblem::WrongType {
                parameter: "hole_size".into(),
                expected: ParamKind::Number,
                found: json!("2"),
            },
            ParameterProblem::Undeclared {
                parameter: "zz_extra".into(),
                found: json!(1),
            },
        ]
    );
}

#[test]
fn an_unknown_generator_is_an_error() {
    let map = BTreeMap::new();
    assert!(
        bundled()
            .check_parameters("ceoloide/missing", &map)
            .is_err()
    );
}
