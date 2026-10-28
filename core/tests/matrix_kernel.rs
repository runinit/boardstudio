//! Core's staged carried-layout kernel publishes poses, frames and contributions.
//! These tests pin the geometry to committed parity fixtures and worked examples.

use boardstudio_core::layout_kernel::{
    ColumnLayout, Follow, FollowBasis, KeyLayout, LayoutInput, MatrixLayout, RawAnchorContext,
    RowLayout, aggregate_average, aggregate_intersection, evaluate,
};
use boardstudio_core::model::{Pose2, Vec2};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FixtureColumnLayout {
    spread: Option<f64>,
    stagger: f64,
    splay: f64,
    rotation: f64,
    splay_origin: Vec2,
    offset: Vec2,
}

impl From<FixtureColumnLayout> for ColumnLayout {
    fn from(value: FixtureColumnLayout) -> Self {
        Self {
            spread: value.spread,
            stagger: value.stagger,
            splay: value.splay,
            rotation: value.rotation,
            splay_origin: value.splay_origin,
            offset: value.offset,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FixtureKeyLayout {
    row: usize,
    column: usize,
    shift: Vec2,
    bend: f64,
    offset: Vec2,
    rotation: f64,
}

impl From<FixtureKeyLayout> for KeyLayout {
    fn from(value: FixtureKeyLayout) -> Self {
        Self {
            row: value.row,
            column: value.column,
            shift: value.shift,
            bend: value.bend,
            offset: value.offset,
            rotation: value.rotation,
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FixtureRowLayout {
    padding: Option<f64>,
    shift: Vec2,
    bend: f64,
    offset: Vec2,
}

impl From<FixtureRowLayout> for RowLayout {
    fn from(value: FixtureRowLayout) -> Self {
        Self {
            padding: value.padding,
            shift: value.shift,
            bend: value.bend,
            offset: value.offset,
        }
    }
}

#[test]
fn default_steps_place_keys_at_pitch() {
    let layout = evaluate(&LayoutInput::default(), RawAnchorContext::World).unwrap();
    let key = &layout.cells[3];
    assert_eq!((key.row, key.column), (1, 1));
    assert_eq!(
        (key.pose.at.x, key.pose.at.y, key.pose.rotation),
        (19.05, 19.05, 0.0)
    );
}

// Fixture/document adaptation stays here; the kernel has no document IDs or anchor graph.
fn fixture_layouts(case: &Value, matrices: &Value) -> BTreeMap<String, MatrixLayout> {
    let mut layouts: BTreeMap<String, MatrixLayout> = BTreeMap::new();
    for matrix in matrices.as_array().unwrap() {
        let mut input = LayoutInput {
            rows: matrix["rows"].as_u64().unwrap() as usize,
            columns: matrix["columns"].as_u64().unwrap() as usize,
            ..Default::default()
        };
        if let Some(v) = matrix.get("pitch") {
            input.pitch = serde_json::from_value(v.clone()).unwrap();
        }
        if let Some(v) = matrix.get("origin") {
            input.origin = serde_json::from_value(v.clone()).unwrap();
        }
        input.rotation = matrix["rotation"].as_f64().unwrap_or(0.0);
        if let Some(v) = matrix.get("mirror") {
            input.mirror = serde_json::from_value(v.clone()).unwrap();
        }
        if let Some(v) = matrix.get("columnLayout") {
            input.column_layout = serde_json::from_value::<Vec<FixtureColumnLayout>>(v.clone())
                .unwrap()
                .into_iter()
                .map(Into::into)
                .collect();
        }
        if let Some(v) = matrix.get("rowLayout") {
            input.row_layout = serde_json::from_value::<Vec<FixtureRowLayout>>(v.clone())
                .unwrap()
                .into_iter()
                .map(Into::into)
                .collect();
        }
        if let Some(v) = matrix.get("cells") {
            input.cells = serde_json::from_value::<Vec<FixtureKeyLayout>>(v.clone())
                .unwrap()
                .into_iter()
                .map(Into::into)
                .collect();
        }
        let context = if let Some(anchor) = matrix.get("anchor") {
            let part_pose = |part: &Value| {
                layouts[part["matrixId"].as_str().unwrap()]
                    .cells
                    .iter()
                    .find(|c| {
                        c.row == part["row"].as_u64().unwrap() as usize
                            && c.column == part["column"].as_u64().unwrap() as usize
                    })
                    .unwrap()
                    .pose
            };
            // Aggregate anchors resolve every part before the matrix
            // Origin/Rotation and Follow apply, exactly as upstream resolves
            // `aggregate` before orient/shift/rotate/affect.
            let pose = if let Some(aggregate) = anchor.get("aggregate") {
                let poses: Vec<_> = aggregate["parts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(part_pose)
                    .collect();
                match aggregate
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or("average")
                {
                    "average" => aggregate_average(&poses).unwrap(),
                    "intersect" => {
                        assert_eq!(poses.len(), 2);
                        aggregate_intersection(poses[0], poses[1]).unwrap()
                    }
                    other => panic!("unknown aggregate method: {other}"),
                }
            } else {
                part_pose(anchor)
            };
            let follow = if let Some(v) = anchor.get("follow") {
                let has = |s| v.as_array().unwrap().iter().any(|v| v == s);
                Follow {
                    x: has("x"),
                    y: has("y"),
                    rotation: has("rotation"),
                }
            } else {
                Follow::default()
            };
            let basis = case
                .get("links")
                .and_then(Value::as_array)
                .and_then(|links| links.iter().find(|link| link["mirror"] == matrix["id"]))
                .map(|link| FollowBasis::Reflected {
                    translation: Vec2 {
                        x: 2.0 * link["axisX"].as_f64().unwrap(),
                        y: 0.0,
                    },
                    rotation: 0.0,
                })
                .unwrap_or_default();
            RawAnchorContext::Key {
                pose,
                follow,
                basis,
            }
        } else {
            RawAnchorContext::World
        };
        layouts.insert(
            matrix["id"].as_str().unwrap().to_owned(),
            evaluate(&input, context).unwrap(),
        );
    }
    layouts
}

fn compare_fixture(
    case: &Value,
    matrices: &Value,
    expected: &Value,
    tolerance: f64,
    local_tolerance: f64,
) -> usize {
    let layouts = fixture_layouts(case, matrices);
    for expected in expected.as_array().unwrap() {
        let layout = &layouts[expected["matrix"].as_str().unwrap()];
        let cell = layout
            .cells
            .iter()
            .find(|c| {
                c.row == expected["row"].as_u64().unwrap() as usize
                    && c.column == expected["column"].as_u64().unwrap() as usize
            })
            .unwrap();
        if expected["absent"] == true {
            let matrix = matrices
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["id"] == expected["matrix"])
                .unwrap();
            let stored = matrix["cells"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["row"] == expected["row"] && c["column"] == expected["column"])
                .unwrap();
            assert!(stored["enabled"] == false || stored["deleted"] == true);
            assert!(cell.pose.at.x.is_finite() && cell.pose.at.y.is_finite());
            continue;
        }
        let difference = case
            .get("differences")
            .and_then(Value::as_array)
            .and_then(|values| {
                values.iter().find(|v| {
                    v["matrix"] == expected["matrix"]
                        && v["row"] == expected["row"]
                        && v["column"] == expected["column"]
                })
            });
        let (expected, tolerance) = difference
            .map(|v| (&v["pose"], local_tolerance))
            .unwrap_or((&expected["pose"], tolerance));
        for (field, actual) in [
            ("x", cell.pose.at.x),
            ("y", cell.pose.at.y),
            ("rotation", cell.pose.rotation),
        ] {
            assert!(
                (actual - expected[field].as_f64().unwrap()).abs() <= tolerance,
                "{} {field}: actual {actual}, expected {expected}",
                case["id"]
            );
        }
    }
    expected
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["absent"] != true)
        .count()
}

#[test]
fn committed_fixtures_match_carried_layout() {
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/ergogen-layout-parity.json")).unwrap();
    assert_eq!(fixtures["fixtureVersion"], 1);
    assert_eq!(
        fixtures["source"]["commit"],
        "8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37"
    );
    assert_eq!(
        fixtures["tolerances"],
        serde_json::json!({"upstream": 1e-6, "boardStudioOnly": 1e-9})
    );
    let upstream = fixtures["tolerances"]["upstream"].as_f64().unwrap();
    let local = fixtures["tolerances"]["boardStudioOnly"].as_f64().unwrap();
    let mut count = 0;
    for case in fixtures["cases"].as_array().unwrap() {
        count += compare_fixture(
            case,
            &case["boardStudioMatrices"],
            &case["expected"],
            upstream,
            local,
        );
    }
    for case in fixtures["boardStudioOnly"].as_array().unwrap() {
        if case.get("boardStudioMatrices").is_some() {
            count += compare_fixture(
                case,
                &case["boardStudioMatrices"],
                &case["expected"],
                local,
                local,
            );
        } else {
            count += compare_fixture(
                case,
                &case["beforeMatrices"],
                &case["expectedBefore"],
                local,
                local,
            );
            count += compare_fixture(
                case,
                &case["afterMatrices"],
                &case["expectedAfter"],
                local,
                local,
            );
        }
    }
    assert_eq!(fixtures["cases"].as_array().unwrap().len(), 42);
    assert_eq!(fixtures["boardStudioOnly"].as_array().unwrap().len(), 4);
    assert_eq!(count, 401, "all fixture poses must be consumed");
}

#[test]
fn production_core_actual_draft_and_materialized_keys_match_committed_fixtures() {
    use boardstudio_core::{CoreEngine, model::*};
    use serde_json::json;
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/ergogen-layout-parity.json")).unwrap();
    let mut count = 0;
    for (cases, tolerance) in [
        (&fixtures["cases"], 1e-6),
        (&fixtures["boardStudioOnly"], 1e-9),
    ] {
        for case in cases.as_array().unwrap() {
            let variants = if case.get("boardStudioMatrices").is_some() {
                vec![(&case["boardStudioMatrices"], &case["expected"])]
            } else {
                vec![
                    (&case["beforeMatrices"], &case["expectedBefore"]),
                    (&case["afterMatrices"], &case["expectedAfter"]),
                ]
            };
            for (matrices, expected) in variants {
                let mut document =
                    serde_json::to_value(ProjectDoc::empty("fixture", "Fixture")).unwrap();
                document["definitions"] =
                    json!([{"id":"key","name":"Key","kind":"switch","courtyard":[],"pads":[]}]);
                let mut parts = Vec::new();
                let mut inputs = Vec::new();
                for input in matrices.as_array().unwrap() {
                    let mut input = input.clone();
                    input["definitionId"] = json!("key");
                    input["boardId"] = json!("board");
                    if input.get("pitch").is_none() {
                        input["pitch"] = json!({"x":19.05,"y":19.05});
                    }
                    if input.get("origin").is_none() {
                        input["origin"] = json!({"x":0.0,"y":0.0});
                    }
                    if let Some(cells) = input.get_mut("cells").and_then(Value::as_array_mut) {
                        for cell in cells {
                            if cell.get("enabled").is_none() {
                                cell["enabled"] = json!(true);
                            }
                        }
                    }
                    let mut members = Vec::new();
                    for row in 0..input["rows"].as_u64().unwrap() {
                        for column in 0..input["columns"].as_u64().unwrap() {
                            let cell = input["cells"].as_array().and_then(|cells| {
                                cells
                                    .iter()
                                    .find(|cell| cell["row"] == row && cell["column"] == column)
                            });
                            if cell.is_some_and(|cell| {
                                cell["enabled"] == false || cell["deleted"] == true
                            }) {
                                continue;
                            }
                            let id =
                                format!("matrix/{}/r{row}c{column}", input["id"].as_str().unwrap());
                            members.push(id.clone());
                            // Deliberately wrong placeholder poses prove materialization
                            // overwrites them; each placeholder is distinct so
                            // aggregate intersection never sees a spurious parallel
                            // pair before the refresh corrects them.
                            let seed = parts.len() as f64;
                            parts.push(json!({"id":id,"definitionId":"key","reference":format!("SW{}", parts.len()+1),"pose":{"at":{"x":123.0 + seed,"y":-456.0 - seed},"rotation":17.0 + seed},"side":"front"}));
                        }
                    }
                    input["partIds"] = json!(members);
                    inputs.push(input);
                }
                document["boards"] = json!([{"id":"board","name":"Board","outlineIds":[],"netIds":[],"thickness":1.6,"partIds":parts.iter().map(|part| part["id"].clone()).collect::<Vec<_>>()}]);
                document["parts"] = json!(parts);
                document["matrices"] = json!(inputs);
                if let Some(links) = case["links"].as_array() {
                    document["layouts"] = json!([
                        {"id":"source","name":"Source","boardId":"board","partIds":[],"matrixIds":links.iter().map(|link| link["source"].clone()).collect::<Vec<_>>()},
                        {"id":"mirror","name":"Mirror","boardId":"board","partIds":[],"matrixIds":links.iter().map(|link| link["mirror"].clone()).collect::<Vec<_>>(),
                         "mirrorLink":{"sourceId":"source","axisX":links[0]["axisX"],"correspondences":links.iter().map(|link| json!({"kind":"matrix","sourceId":link["source"],"mirrorId":link["mirror"]})).collect::<Vec<_>>()}}
                    ]);
                } else {
                    document["layouts"] = json!([{"id":"layout","name":"Layout","boardId":"board","partIds":[],"matrixIds":inputs.iter().map(|input| input["id"].clone()).collect::<Vec<_>>()}]);
                }
                let mut engine = CoreEngine::new();
                let opened: Value = serde_json::from_str(&engine.request(
                    &json!({"kind":"open","id":"fixture","document":document}).to_string(),
                ))
                .unwrap();
                assert_eq!(opened["kind"], "scene", "{}: {opened}", case["id"]);
                let draft: Value = serde_json::from_str(&engine.request(&json!({"kind":"project-matrices","id":"draft","baseRevision":0,"matrices":opened["document"]["matrices"]}).to_string())).unwrap();
                assert_eq!(
                    draft["kind"], "matrix-projections",
                    "{}: {draft}",
                    case["id"]
                );
                for expected in expected.as_array().unwrap() {
                    let id = format!(
                        "matrix/{}/r{}c{}",
                        expected["matrix"].as_str().unwrap(),
                        expected["row"],
                        expected["column"]
                    );
                    let part = opened["document"]["parts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|part| part["id"] == id);
                    if expected["absent"] == true {
                        assert!(part.is_none(), "{id}");
                        continue;
                    }
                    let difference = case["differences"].as_array().and_then(|values| {
                        values.iter().find(|value| {
                            value["matrix"] == expected["matrix"]
                                && value["row"] == expected["row"]
                                && value["column"] == expected["column"]
                        })
                    });
                    let (pose, tolerance) = difference
                        .map_or((&expected["pose"], tolerance), |difference| {
                            (&difference["pose"], 1e-9)
                        });
                    for scene in [&opened["scene"]["matrixScenes"], &draft["matrixScenes"]] {
                        let matrix = scene
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|matrix| matrix["matrixId"] == expected["matrix"])
                            .unwrap();
                        let cell = matrix["cells"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|cell| {
                                cell["row"] == expected["row"]
                                    && cell["column"] == expected["column"]
                            })
                            .unwrap();
                        for (field, actual) in [
                            ("x", &cell["pose"]["at"]["x"]),
                            ("y", &cell["pose"]["at"]["y"]),
                            ("rotation", &cell["pose"]["rotation"]),
                        ] {
                            assert!(
                                (actual.as_f64().unwrap() - pose[field].as_f64().unwrap()).abs()
                                    <= tolerance,
                                "{} {id} {field}: {actual}, expected {pose}",
                                case["id"]
                            );
                        }
                        assert_eq!(part.unwrap()["pose"], cell["pose"], "{} {id}", case["id"]);
                    }
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 401);
}

#[test]
fn shift_precedes_bend_while_offset_follows_it() {
    let mut input = LayoutInput {
        columns: 1,
        row_layout: vec![RowLayout {
            shift: Vec2 { x: 2.0, y: 0.0 },
            bend: 90.0,
            ..Default::default()
        }],
        ..Default::default()
    };
    let shifted = evaluate(&input, RawAnchorContext::World).unwrap();
    assert_eq!(shifted.cells[0].pose.at, Vec2 { x: 2.0, y: 0.0 });
    assert!((shifted.cells[1].pose.at.x + 17.05).abs() < 1e-9);
    input.row_layout[0].shift = Vec2::default();
    input.row_layout[0].offset = Vec2 { x: 2.0, y: 0.0 };
    let offset = evaluate(&input, RawAnchorContext::World).unwrap();
    assert!(offset.cells[0].pose.at.x.abs() < 1e-9);
    assert!((offset.cells[0].pose.at.y - 2.0).abs() < 1e-9);
    assert!((offset.cells[1].pose.at.x + 19.05).abs() < 1e-9);
    assert_eq!(offset.cells[1].pose.rotation, 90.0);
}

fn near(actual: Vec2, x: f64, y: f64) {
    assert!(
        (actual.x - x).abs() < 1e-9 && (actual.y - y).abs() < 1e-9,
        "{actual:?} != ({x},{y})"
    );
}

#[test]
fn frames_and_contributions_follow_carried_turns() {
    let input = LayoutInput {
        columns: 1,
        pitch: Vec2 { x: 10.0, y: 10.0 },
        column_layout: vec![ColumnLayout {
            splay: 90.0,
            rotation: 90.0,
            offset: Vec2 { x: 3.0, y: 4.0 },
            ..Default::default()
        }],
        row_layout: vec![
            RowLayout {
                shift: Vec2 { x: 2.0, y: 0.0 },
                bend: 90.0,
                ..Default::default()
            },
            RowLayout {
                shift: Vec2 { x: 3.0, y: 0.0 },
                offset: Vec2 { x: 1.0, y: 2.0 },
                ..Default::default()
            },
        ],
        cells: vec![KeyLayout {
            row: 1,
            column: 0,
            shift: Vec2 { x: 1.0, y: 0.0 },
            bend: -90.0,
            offset: Vec2 { x: 5.0, y: 6.0 },
            rotation: 37.0,
        }],
        ..Default::default()
    };
    let layout = evaluate(&input, RawAnchorContext::World).unwrap();
    let cell = &layout.cells[1];
    near(cell.shift_frame.origin, 8.0, 0.0);
    near(cell.shift_frame.axis_x, 0.0, -1.0);
    near(cell.shift_frame.axis_y, 1.0, 0.0);
    near(cell.offset_frame.origin, 8.0, -4.0);
    near(cell.offset_frame.axis_x, -1.0, 0.0);
    near(cell.offset_frame.axis_y, 0.0, -1.0);
    near(cell.pose.at, -1.0, -16.0);
    assert_eq!(cell.pose.rotation, 217.0);
    near(cell.contributions.cumulative_shift, -2.0, -4.0);
    near(cell.contributions.row_shift_matrix, 0.0, -3.0);
    near(cell.contributions.key_shift_matrix, 0.0, -1.0);
    assert_eq!(cell.contributions.cumulative_bend, 0.0);
    assert_eq!(cell.contributions.key_rotation, 37.0);
    assert_eq!(layout.columns[0].rotation, 90.0);
    assert_eq!(layout.columns[0].cumulative_splay, 90.0);
    assert!(layout.columns[0].spread_axis.is_none());
}

#[test]
fn validation_rejects_invalid_values_and_defaults_missing_steps() {
    let mut input = LayoutInput {
        column_layout: vec![ColumnLayout {
            spread: Some(0.0),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(
        evaluate(&input, RawAnchorContext::World).is_err(),
        "no Spread at column zero"
    );
    input.column_layout.clear();
    input.row_layout = vec![RowLayout {
        padding: Some(-1.0),
        ..Default::default()
    }];
    assert!(
        evaluate(&input, RawAnchorContext::World).is_err(),
        "no Padding at row zero"
    );
    input.row_layout = vec![RowLayout::default(); 3];
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
    input.row_layout.clear();
    input.column_layout = vec![ColumnLayout::default(); 3];
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
    input.column_layout.clear();
    input.pitch.x = input.edge_gap;
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
    input.pitch.x = 19.05;
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        input.cells = vec![KeyLayout {
            shift: Vec2 { x: bad, y: 0.0 },
            ..Default::default()
        }];
        assert!(evaluate(&input, RawAnchorContext::World).is_err());
        input.cells.clear();
        input.rotation = bad;
        assert!(evaluate(&input, RawAnchorContext::World).is_err());
    }
    input.rotation = 0.0;
    input.cells = vec![KeyLayout {
        row: 2,
        ..Default::default()
    }];
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
    input.cells = vec![KeyLayout::default(); 2];
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
    input.cells.clear();
    input.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            spread: Some(0.0),
            ..Default::default()
        },
    ];
    input.row_layout = vec![
        RowLayout::default(),
        RowLayout {
            padding: Some(-3.0),
            ..Default::default()
        },
    ];
    let layout = evaluate(&input, RawAnchorContext::World).unwrap();
    near(layout.cells[3].pose.at, 0.0, -3.0);
    assert_eq!(
        layout.columns[1].spread.unwrap().source,
        boardstudio_core::layout_kernel::StepSource::Explicit
    );
    let defaults = evaluate(&LayoutInput::default(), RawAnchorContext::World).unwrap();
    assert_eq!(defaults.columns[1].spread.unwrap().value, 19.05);
    assert_eq!(
        defaults.columns[1].spread.unwrap().source,
        boardstudio_core::layout_kernel::StepSource::Pitch
    );
    assert!(defaults.cells[0].contributions.padding.is_none());
}

#[test]
fn follow_in_slanted_basis_uses_actual_anchor() {
    let input = LayoutInput {
        rows: 1,
        columns: 1,
        origin: Vec2 { x: 2.0, y: 4.0 },
        rotation: 15.0,
        ..Default::default()
    };
    let layout = evaluate(
        &input,
        RawAnchorContext::Key {
            // An actual Local anchor, not its inherited substitute.
            pose: Pose2 {
                at: Vec2 { x: 40.0, y: 50.0 },
                rotation: -30.0,
            },
            follow: Follow {
                x: false,
                y: true,
                rotation: false,
            },
            basis: FollowBasis::Reflected {
                translation: Vec2 { x: 100.0, y: 20.0 },
                rotation: 90.0,
            },
        },
    )
    .unwrap();
    // Candidate (43.73205080756888,52.46410161513776,-15) is filtered in source axes.
    near(layout.cells[0].pose.at, 43.73205080756888, 20.0);
    assert_eq!(layout.cells[0].pose.rotation, 90.0);
    near(layout.frame.axis_x, 0.0, 1.0);
    near(layout.frame.axis_y, -1.0, 0.0);
}

#[test]
fn column_axes_match_step_differences_and_mirrored_frames() {
    for mirror in [
        boardstudio_core::model::Mirror::None,
        boardstudio_core::model::Mirror::X,
        boardstudio_core::model::Mirror::Y,
    ] {
        let mut input = LayoutInput {
            rows: 2,
            columns: 3,
            origin: Vec2 { x: 20.0, y: -3.0 },
            rotation: 17.0,
            mirror,
            column_layout: vec![
                ColumnLayout {
                    splay: 23.0,
                    ..Default::default()
                },
                ColumnLayout {
                    stagger: 4.0,
                    splay: -31.0,
                    rotation: 12.0,
                    splay_origin: Vec2 { x: 2.0, y: -5.0 },
                    ..Default::default()
                },
            ],
            row_layout: vec![RowLayout {
                bend: 38.0,
                ..Default::default()
            }],
            ..Default::default()
        };
        let base = evaluate(&input, RawAnchorContext::World).unwrap();
        assert!(base.columns[1].custom_origin);
        near(base.columns[1].relative_origin, 2.0, -5.0);
        input.column_layout[1].spread = Some(input.pitch.x + 1.0);
        let spread = evaluate(&input, RawAnchorContext::World).unwrap();
        let axis = base.columns[1].spread_axis.unwrap();
        for row in 0..2 {
            let a = base.cells[2 + row].pose.at;
            let b = spread.cells[2 + row].pose.at;
            near(axis, b.x - a.x, b.y - a.y);
        }
        input.column_layout[1].spread = None;
        input.column_layout[1].stagger += 1.0;
        let stagger = evaluate(&input, RawAnchorContext::World).unwrap();
        let axis = base.columns[1].stagger_axis;
        near(
            axis,
            stagger.cells[3].pose.at.x - base.cells[3].pose.at.x,
            stagger.cells[3].pose.at.y - base.cells[3].pose.at.y,
        );
        // Independent offset differencing verifies both published mirror-aware axes.
        input.column_layout[1].stagger -= 1.0;
        for (offset, axis) in [
            (Vec2 { x: 1.0, y: 0.0 }, base.cells[3].offset_frame.axis_x),
            (Vec2 { x: 0.0, y: 1.0 }, base.cells[3].offset_frame.axis_y),
        ] {
            input.cells = vec![KeyLayout {
                row: 1,
                column: 1,
                offset,
                ..Default::default()
            }];
            let moved = evaluate(&input, RawAnchorContext::World).unwrap();
            near(
                axis,
                moved.cells[3].pose.at.x - base.cells[3].pose.at.x,
                moved.cells[3].pose.at.y - base.cells[3].pose.at.y,
            );
        }
    }
}

#[test]
fn disabled_deleted_and_omitted_cells_still_carry() {
    for state in [
        serde_json::json!({"enabled": false}),
        serde_json::json!({"deleted": true}),
        serde_json::json!({"omitted": true}),
    ] {
        let mut key =
            serde_json::json!({"row": 0, "column": 0, "shift": {"x": 2, "y": 0}, "bend": 90});
        key.as_object_mut()
            .unwrap()
            .extend(state.as_object().unwrap().clone());
        let input = LayoutInput {
            columns: 1,
            cells: vec![
                serde_json::from_value::<FixtureKeyLayout>(key)
                    .unwrap()
                    .into(),
            ],
            ..Default::default()
        };
        let layout = evaluate(&input, RawAnchorContext::World).unwrap();
        assert_eq!(
            layout.cells.len(),
            2,
            "logical keys are never filtered in the kernel"
        );
        near(layout.cells[1].pose.at, -17.05, 0.0);
        assert_eq!(layout.cells[1].contributions.cumulative_bend, 90.0);
    }
}

#[test]
fn nonfinite_anchors_and_evaluation_overflow_are_rejected() {
    let input = LayoutInput::default();
    let invalid_anchor = RawAnchorContext::Key {
        pose: Pose2 {
            at: Vec2::default(),
            rotation: f64::NAN,
        },
        follow: Follow {
            x: false,
            y: false,
            rotation: false,
        },
        basis: FollowBasis::World,
    };
    assert!(
        evaluate(&input, invalid_anchor).is_err(),
        "Follow cannot mask invalid input"
    );
    let invalid_basis = RawAnchorContext::Key {
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        follow: Follow::default(),
        basis: FollowBasis::Reflected {
            translation: Vec2::default(),
            rotation: f64::INFINITY,
        },
    };
    assert!(evaluate(&input, invalid_basis).is_err());
    let input = LayoutInput {
        origin: Vec2 {
            x: f64::MAX,
            y: 0.0,
        },
        ..Default::default()
    };
    let anchor = RawAnchorContext::Key {
        pose: Pose2 {
            at: Vec2 {
                x: f64::MAX,
                y: 0.0,
            },
            rotation: 0.0,
        },
        follow: Follow {
            x: false,
            y: false,
            rotation: false,
        },
        basis: FollowBasis::World,
    };
    assert!(
        evaluate(&input, anchor).is_err(),
        "candidate overflow is refused before Follow"
    );
    let input = LayoutInput {
        row_layout: vec![
            RowLayout {
                bend: f64::MAX,
                ..Default::default()
            };
            2
        ],
        ..Default::default()
    };
    assert!(evaluate(&input, RawAnchorContext::World).is_err());
}

#[test]
fn published_frame_overflow_is_rejected_before_key_rotation() {
    let input = LayoutInput {
        rotation: f64::MAX,
        column_layout: vec![ColumnLayout {
            rotation: f64::MAX,
            ..Default::default()
        }],
        cells: vec![
            KeyLayout {
                rotation: -f64::MAX,
                ..Default::default()
            },
            KeyLayout {
                row: 1,
                rotation: -f64::MAX,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(
        evaluate(&input, RawAnchorContext::World).is_err(),
        "finite final keys must not hide overflowed frames"
    );
}

#[test]
fn column_bases_origins_and_prefix_exclude_own_rotation() {
    let input = LayoutInput {
        rows: 1,
        columns: 3,
        pitch: Vec2 { x: 10.0, y: 10.0 },
        column_layout: vec![
            ColumnLayout {
                stagger: 2.0,
                splay: 90.0,
                splay_origin: Vec2 { x: 1.0, y: 0.0 },
                ..Default::default()
            },
            ColumnLayout {
                stagger: 3.0,
                splay: 90.0,
                rotation: 90.0,
                splay_origin: Vec2 { x: 2.0, y: -3.0 },
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let layout = evaluate(&input, RawAnchorContext::World).unwrap();
    near(layout.columns[1].base, -2.0, 11.0);
    near(layout.columns[1].splay_origin, 1.0, 13.0);
    near(layout.columns[1].frame.origin, 4.0, 15.0);
    near(layout.cells[1].pose.at, 4.0, 15.0);
    assert_eq!(layout.cells[1].pose.rotation, 270.0);
    near(layout.cells[2].pose.at, -7.0, 10.0);
    assert_eq!(layout.cells[2].pose.rotation, 180.0);
    assert_eq!(layout.columns[2].cumulative_stagger, 5.0);
}

/// Core projects `document` through its public requests, returning the opened scene's
/// cells, the draft projection's cells and the persisted part ids.
fn open_and_project(document: Value) -> (Value, Value, Vec<String>) {
    use boardstudio_core::CoreEngine;
    use serde_json::json;
    let mut engine = CoreEngine::new();
    let opened: Value = serde_json::from_str(
        &engine.request(&json!({"kind":"open","id":"states","document":document}).to_string()),
    )
    .unwrap();
    assert_eq!(opened["kind"], "scene", "{opened}");
    let draft: Value = serde_json::from_str(&engine.request(&json!({"kind":"project-matrices","id":"draft","baseRevision":0,"matrices":opened["document"]["matrices"]}).to_string())).unwrap();
    assert_eq!(draft["kind"], "matrix-projections", "{draft}");
    let parts = opened["document"]["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].as_str().unwrap().to_owned())
        .collect();
    (
        opened["scene"]["matrixScenes"].clone(),
        draft["matrixScenes"].clone(),
        parts,
    )
}

fn states_document(source_cell: Value, omit_mirror_first: bool) -> Value {
    use boardstudio_core::model::ProjectDoc;
    use serde_json::json;
    let mut document = serde_json::to_value(ProjectDoc::empty("states", "States")).unwrap();
    document["definitions"] =
        json!([{"id":"key","name":"Key","kind":"switch","courtyard":[],"pads":[]}]);
    let matrix = |id: &str, x: f64, cell: &Value, part_ids: &[String]| {
        let cells = if cell.is_null() {
            vec![]
        } else {
            vec![cell.clone()]
        };
        json!({"id":id,"definitionId":"key","boardId":"board","rows":3,"columns":1,
            "pitch":{"x":19.05,"y":19.05},"origin":{"x":x,"y":0.0},"cells":cells,
            "partIds":part_ids})
    };
    let live = |cell: &Value, row: u64| {
        cell["row"] != row || (cell["enabled"] != false && cell["deleted"] != true)
    };
    let mut parts = vec![];
    let (mut source_ids, mut mirror_ids) = (vec![], vec![]);
    let mut push = |id: String, x: f64| {
        let n = parts.len() + 1;
        parts.push(json!({"id":id,"definitionId":"key","reference":format!("SW{n}"),"pose":{"at":{"x":x,"y":-456.0},"rotation":17.0},"side":"front"}));
    };
    for column in 0..3u64 {
        if live(&source_cell, column) {
            let id = format!("matrix/src/r{column}c0");
            push(id.clone(), 1000.0 + column as f64);
            source_ids.push(id);
        }
        if !(omit_mirror_first && column == 0) {
            let id = format!("matrix/mir/r{column}c0");
            push(id.clone(), 2000.0 + column as f64);
            mirror_ids.push(id);
        }
    }
    let source = matrix("src", 0.0, &source_cell, &source_ids);
    let mirror = matrix("mir", 200.0, &source_cell, &mirror_ids);
    let exceptions = if omit_mirror_first {
        json!([{"kind":"part","mirrorId":"matrix/mir/r0c0","state":"omitted"}])
    } else {
        json!([])
    };
    let mut correspondences = vec![json!({"kind":"matrix","sourceId":"src","mirrorId":"mir"})];
    if omit_mirror_first {
        correspondences
            .push(json!({"kind":"part","sourceId":"matrix/src/r0c0","mirrorId":"matrix/mir/r0c0"}));
    }
    document["boards"] = json!([{"id":"board","name":"Board","outlineIds":[],"netIds":[],"thickness":1.6,"partIds":parts.iter().map(|part| part["id"].clone()).collect::<Vec<_>>()}]);
    document["parts"] = json!(parts);
    document["matrices"] = json!([source, mirror]);
    document["layouts"] = json!([
        {"id":"source","name":"Source","boardId":"board","partIds":[],"matrixIds":["src"]},
        {"id":"mirror","name":"Mirror","boardId":"board","partIds":[],"matrixIds":["mir"],
         "mirrorLink":{"sourceId":"source","axisX":100.0,
            "correspondences":correspondences,
            "exceptions":exceptions}}
    ]);
    document
}

fn scene_cell<'a>(scenes: &'a Value, matrix: &str, row: u64) -> Option<&'a Value> {
    scenes
        .as_array()
        .unwrap()
        .iter()
        .find(|scene| scene["matrixId"] == matrix)?["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|cell| cell["row"] == row)
}

/// Ticket 16 Answer: the kernel proves supplied logical values carry, but not that
/// production selects the right state. Enabled, deleted and omitted keys all keep
/// their carried Shift and Bend in Core's actual and draft scenes and materialized parts.
#[test]
fn production_enabled_deleted_and_omitted_keys_carry_into_later_keys() {
    use serde_json::json;
    let carry = json!({"row":0,"column":0,"enabled":true,"shift":{"x":2.0,"y":0.0},"bend":90.0});
    let (base_actual, base_draft, base_parts) =
        open_and_project(states_document(carry.clone(), false));
    assert!(base_parts.contains(&"matrix/src/r0c0".to_owned()));
    let carried = |scenes: &Value, matrix: &str, column| {
        scene_cell(scenes, matrix, column).unwrap()["pose"].clone()
    };
    // The pose carried after a (2, 0) shift and 90 degree bend, pinned exactly.
    let pose = carried(&base_actual, "src", 1);
    assert!(
        (pose["at"]["x"].as_f64().unwrap() + 17.05).abs() < 1e-9,
        "{pose}"
    );
    assert!(pose["at"]["y"].as_f64().unwrap().abs() < 1e-9, "{pose}");
    assert_eq!(pose["rotation"], 90.0);

    let mut disabled = carry.clone();
    disabled["enabled"] = json!(false);
    let mut deleted = carry.clone();
    deleted["deleted"] = json!(true);
    for (state, cell, omit) in [
        ("disabled", disabled, false),
        ("deleted", deleted, false),
        ("omitted", carry.clone(), true),
    ] {
        let (actual, draft, parts) = open_and_project(states_document(cell, omit));
        for (scenes, base) in [(&actual, &base_actual), (&draft, &base_draft)] {
            for matrix in ["src", "mir"] {
                for column in [1, 2] {
                    assert_eq!(
                        carried(scenes, matrix, column),
                        carried(base, matrix, column),
                        "{state} {matrix} r{column}"
                    );
                }
            }
        }
        match state {
            "disabled" => {
                for scenes in [&actual, &draft] {
                    let cell =
                        scene_cell(scenes, "src", 0).expect("disabled key stays in the scene");
                    assert_eq!(cell["enabled"], false);
                    assert_eq!(cell["pose"], carried(&base_actual, "src", 0));
                }
                assert!(!parts.contains(&"matrix/src/r0c0".to_owned()));
            }
            "deleted" => {
                for scenes in [&actual, &draft] {
                    assert!(
                        scene_cell(scenes, "src", 0).is_none(),
                        "deleted key leaves the scene"
                    );
                }
                assert!(!parts.contains(&"matrix/src/r0c0".to_owned()));
            }
            _ => {
                assert!(
                    !parts.contains(&"matrix/mir/r0c0".to_owned()),
                    "omitted key is never rematerialized"
                );
                for scenes in [&actual, &draft] {
                    let cell =
                        scene_cell(scenes, "mir", 0).expect("omitted key keeps its logical cell");
                    assert_eq!(cell["pose"], carried(&base_actual, "mir", 0));
                    assert!(cell["memberId"].is_null());
                }
            }
        }
        for column in [1, 2] {
            let id = format!("matrix/src/r{column}c0");
            assert!(parts.contains(&id), "{state} {id}");
        }
    }
}

/// The captured oracle predates grouped layout records, so the fixture keeps its legacy
/// per-axis arrays and this adapter rewrites them as column and row records.
fn grouped_from_legacy(mut matrix: Value) -> Value {
    use serde_json::{Map, json};
    let object = matrix.as_object_mut().unwrap();
    let mut take = |key: &str| match object.remove(key) {
        Some(Value::Array(items)) => items,
        _ => vec![],
    };
    let (offsets, origins, splays, staggers) = (
        take("columnOffsets"),
        take("columnOrigins"),
        take("columnSplays"),
        take("columnStaggers"),
    );
    let row_offsets = take("rowOffsets");
    let columns = [&offsets, &origins, &splays, &staggers]
        .iter()
        .map(|items| items.len())
        .max()
        .unwrap();
    let record = |fields: Vec<(&str, Option<&Value>)>| {
        let mut out = Map::new();
        for (key, value) in fields {
            if let Some(value) = value.filter(|value| !value.is_null()) {
                out.insert(key.into(), value.clone());
            }
        }
        Value::Object(out)
    };
    let column_layout: Vec<_> = (0..columns)
        .map(|i| {
            record(vec![
                ("offset", offsets.get(i)),
                ("splayOrigin", origins.get(i)),
                ("splay", splays.get(i)),
                ("stagger", staggers.get(i)),
            ])
        })
        .collect();
    let row_layout: Vec<_> = row_offsets
        .iter()
        .map(|offset| record(vec![("offset", Some(offset))]))
        .collect();
    object.insert("columnLayout".into(), json!(column_layout));
    object.insert("rowLayout".into(), json!(row_layout));
    matrix
}

/// The 17a-regenerated oracle fixture, opened through Core's public path: every cell's
/// member id, row, column, enabled flag and pose must match, for none, x and y mirror
/// modes. Saved-pose cases give each existing part a distinct pose, so an exact match
/// proves the actual projection reads the saved pose and the `matrix/<id>/r{r}c{c}` id.
#[test]
fn saved_poses_and_member_ids_of_existing_parts_are_read_in_every_mirror_mode() {
    use boardstudio_core::{CoreEngine, model::ProjectDoc};
    use serde_json::json;
    let fixtures: Value =
        serde_json::from_str(include_str!("fixtures/matrix-projections.json")).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
    let mut saved_cases = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mut matrix = grouped_from_legacy(case["matrix"].clone());
        matrix["boardId"] = json!("board");
        let parts = case["parts"].as_array().unwrap();
        if !parts.is_empty() {
            saved_cases += 1;
            let poses: Vec<_> = parts.iter().map(|part| part["pose"].to_string()).collect();
            let distinct: std::collections::BTreeSet<_> = poses.iter().collect();
            assert_eq!(
                distinct.len(),
                poses.len(),
                "{name}: poses must be distinct"
            );
        }
        let mut document = serde_json::to_value(ProjectDoc::empty("oracle", "Oracle")).unwrap();
        document["definitions"] =
            json!([{"id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]}]);
        document["boards"] = json!([{"id":"board","name":"Board","outlineIds":[],"netIds":[],"thickness":1.6,
            "partIds":parts.iter().map(|part| part["id"].clone()).collect::<Vec<_>>()}]);
        document["parts"] = json!(parts);
        // Open preserves Local parts and parts with layoutOverride == 1. Local is
        // sufficient here; projection itself reads saved member poses regardless of Local.
        // The fixture matrix shares the mirror layout with a dummy mirror of `src`.
        let dummy = |id: &str, x: f64| {
            json!({"id":id,"definitionId":"switch","boardId":"board","rows":1,"columns":1,
                "pitch":{"x":19.0,"y":18.0},"origin":{"x":x,"y":0.0},"cells":[],"partIds":[]})
        };
        document["matrices"] = json!([matrix, dummy("src", 0.0), dummy("mir", 200.0)]);
        let local: Vec<_> = parts
            .iter()
            .map(|part| json!({"kind":"part","mirrorId":part["id"],"state":"local"}))
            .collect();
        document["layouts"] = json!([
            {"id":"source","name":"Source","boardId":"board","partIds":[],"matrixIds":["src"]},
            {"id":"layout","name":"Layout","boardId":"board","partIds":[],"matrixIds":["fixture","mir"],
             "mirrorLink":{"sourceId":"source","axisX":100.0,
                "correspondences":[{"kind":"matrix","sourceId":"src","mirrorId":"mir"}],
                "exceptions":local}}
        ]);
        let mut engine = CoreEngine::new();
        let opened: Value = serde_json::from_str(
            &engine.request(&json!({"kind":"open","id":"oracle","document":document}).to_string()),
        )
        .unwrap();
        assert_eq!(opened["kind"], "scene", "{name}: {opened}");
        let scene = opened["scene"]["matrixScenes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scene| scene["matrixId"] == "fixture")
            .unwrap();
        let cells = scene["cells"].as_array().unwrap();
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(cells.len(), expected.len(), "{name}: cell count");
        for (actual, expected) in cells.iter().zip(expected) {
            let at = format!("{name} r{}c{}", expected["row"], expected["column"]);
            assert_eq!(actual["row"], expected["row"], "{at} row");
            assert_eq!(actual["column"], expected["column"], "{at} column");
            assert_eq!(actual["enabled"], expected["enabled"], "{at} enabled");
            let expected_member = expected.get("memberId").cloned().unwrap_or(Value::Null);
            assert_eq!(actual["memberId"], expected_member, "{at} memberId");
            for (name, a, e) in [
                (
                    "x",
                    &actual["pose"]["at"]["x"],
                    &expected["pose"]["at"]["x"],
                ),
                (
                    "y",
                    &actual["pose"]["at"]["y"],
                    &expected["pose"]["at"]["y"],
                ),
                (
                    "rotation",
                    &actual["pose"]["rotation"],
                    &expected["pose"]["rotation"],
                ),
            ] {
                let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
                assert!(
                    (a - e).abs() < 1e-9,
                    "{at} pose {name}: expected {e}, got {a}"
                );
            }
        }
    }
    assert_eq!(saved_cases, 3, "none, x and y each have a saved-pose case");
}
