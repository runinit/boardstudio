use boardstudio_core::{CoreEngine, model::*};
use serde_json::json;

fn doc() -> ProjectDoc {
    let mut value = serde_json::to_value(ProjectDoc::empty("caps", "Caps")).unwrap();
    value["definitions"] = json!([{"id":"mx","name":"MX","kind":"switch","keycap":{"x":18.2,"y":18.2},"courtyard":[],"pads":[]}]);
    value["parts"] = json!([{"id":"matrix/m/r0c0","definitionId":"mx","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},{"id":"matrix/m/r1c0","definitionId":"mx","reference":"SW2","pose":{"at":{"x":0,"y":19.05},"rotation":0},"side":"front"}]);
    value["matrices"] = json!([{"id":"m","rows":2,"columns":1,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"mx","partIds":["matrix/m/r0c0","matrix/m/r1c0"],"boardId":"board"}]);
    value["boards"] = json!([{"id":"board","name":"Board","partIds":["matrix/m/r0c0","matrix/m/r1c0"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    value["keycaps"] = json!({"matrices":{"m":{"profile":"cherry","mount":"mx"}},"keys":{"matrix/m/r0c0":{"legend":"A","color":"#ff0088"}},"boards":{"board":{"color":"#112233"}}});
    serde_json::from_value(value).unwrap()
}
fn resolve(document: ProjectDoc, cases: Option<PreparedCaseAssemblyIR>) -> KeycapResolution {
    let reply = CoreEngine::new().handle(CoreRequest::ResolveKeycaps {
        id: "caps".into(),
        board_id: "board".into(),
        document,
        cases,
    });
    match reply {
        CoreReply::KeycapsResolved { result, .. } => result,
        _ => panic!("wrong reply"),
    }
}
#[test]
fn resolves_rows_sizes_colors_and_backward_compatible_absence() {
    let document = doc();
    let result = resolve(document.clone(), None);
    assert_eq!(result.specs.len(), 2);
    assert!(result.findings.is_empty());
    assert_eq!(result.specs[0].row, 1);
    assert_eq!(result.specs[1].row, 2);
    assert_eq!(result.specs[0].legend, "A");
    assert_eq!(result.specs[0].color, "#ff0088");
    assert_eq!(result.specs[1].color, "#112233");
    assert!((result.specs[0].z - 10.0).abs() < 1e-9);
    let mut legacy = document;
    legacy.keycaps = None;
    assert!(resolve(legacy, None).specs.is_empty());
}
#[test]
fn unit_override_rotated_overlap_blank_and_validation() {
    let mut document = doc();
    document
        .keycaps
        .as_mut()
        .unwrap()
        .keys
        .get_mut("matrix/m/r0c0")
        .unwrap()
        .units = Some(Vec2 { x: 1.0, y: 2.0 });
    document.parts[1].pose.rotation = 20.0;
    let result = resolve(document.clone(), None);
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.message.contains("clearance"))
    );
    document
        .keycaps
        .as_mut()
        .unwrap()
        .keys
        .get_mut("matrix/m/r0c0")
        .unwrap()
        .legend = Some("".into());
    assert_eq!(resolve(document.clone(), None).specs[0].legend, "");
    document
        .keycaps
        .as_mut()
        .unwrap()
        .matrices
        .get_mut("m")
        .unwrap()
        .wall_thickness = 0.1;
    assert!(
        resolve(document, None)
            .findings
            .iter()
            .all(|f| f.severity == Severity::Error)
    );
}
#[test]
fn case_walls_are_checked_over_full_travel_and_ignore_stale_cases() {
    let document = doc();
    let square = |r: f64| {
        vec![
            Vec2 { x: -r, y: -r },
            Vec2 { x: r, y: -r },
            Vec2 { x: r, y: r },
            Vec2 { x: -r, y: r },
        ]
    };
    let cases=PreparedCaseAssemblyIR{revision:document.revision,bodies:vec![PreparedCaseIR{revision:document.revision,body:serde_json::from_value(json!({"id":"wall","name":"Wall","boardId":"board","kind":"tray","outlineIds":[],"thickness":2,"clearance":0,"wallHeight":12,"z":0})).unwrap(),regions:vec![PreparedCaseRegion{outer:square(25.0),holes:vec![],cavities:vec![square(8.0)],gaskets:vec![],mounts:vec![]}]}]};
    assert!(
        resolve(document.clone(), Some(cases.clone()))
            .findings
            .iter()
            .any(|f| f.target_ids.contains(&"wall".into()))
    );
    let mut stale = cases;
    stale.revision += 1;
    assert!(
        resolve(document, Some(stale))
            .findings
            .iter()
            .any(|finding| finding.id.ends_with("case-pending"))
    );
}
#[test]
fn open_replace_undo_and_roundtrip_preserve_settings() {
    let document = doc();
    let mut core = CoreEngine::new();
    let opened = core.handle(CoreRequest::Open {
        id: "open".into(),
        document: document.clone(),
    });
    assert!(matches!(opened, CoreReply::Scene { .. }));
    let mut changed = document.clone();
    changed
        .keycaps
        .as_mut()
        .unwrap()
        .boards
        .get_mut("board")
        .unwrap()
        .color = "#aabbcc".into();
    core.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: document.revision,
            phase: EditPhase::Commit,
            transaction_id: "color".into(),
            target_ids: vec![],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(changed),
            },
        },
    });
    let undo = core.handle(CoreRequest::Undo { id: "undo".into() });
    match undo {
        CoreReply::Scene {
            document: undone, ..
        } => assert_eq!(undone.keycaps, document.keycaps),
        _ => panic!("undo failed"),
    }
    let decoded: ProjectDoc =
        serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
    assert_eq!(decoded.keycaps, document.keycaps);
}

#[test]
fn custom_standalone_keys_can_select_a_socket_and_profiles_resolve_valid_roofs() {
    for profile in [
        KeycapProfile::Cherry,
        KeycapProfile::Oem,
        KeycapProfile::Dcs,
        KeycapProfile::Dsa,
        KeycapProfile::Sa,
        KeycapProfile::HiPro,
        KeycapProfile::G20,
        KeycapProfile::Choc,
    ] {
        for row in 1..=5 {
            let mut document = doc();
            document.matrices.clear();
            let config = document.keycaps.as_mut().unwrap();
            config.keys.clear();
            let key = KeycapKeySettings {
                profile: Some(profile),
                mount: Some(if profile == KeycapProfile::Choc {
                    KeycapMount::ChocV1
                } else {
                    KeycapMount::Mx
                }),
                row: Some(row),
                ..Default::default()
            };
            config.keys.insert(document.parts[0].id.clone(), key);
            let result = resolve(document, None);
            assert!(
                result.findings.is_empty(),
                "{profile:?} row {row}: {:?}",
                result.findings
            );
            assert_eq!(result.specs.len(), 1);
            assert_eq!(result.specs[0].row, row);
        }
    }
}

#[test]
fn support_solids_are_checked_even_when_the_case_shell_is_below_the_cap() {
    let document = doc();
    let body: CaseBody = serde_json::from_value(json!({"id":"support","name":"Support","boardId":"board","kind":"plate","outlineIds":[],"thickness":1,"clearance":0,"z":0,"features":[{"kind":"support-prism","id":"post","points":[{"x":-1,"y":-1},{"x":1,"y":-1},{"x":1,"y":1},{"x":-1,"y":1}],"z":12,"height":3}]})).unwrap();
    let cases = PreparedCaseAssemblyIR {
        revision: document.revision,
        bodies: vec![PreparedCaseIR {
            revision: document.revision,
            body,
            regions: vec![],
        }],
    };
    let result = resolve(document, Some(cases));
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.id.ends_with("feature/post"))
    );
}

fn size_doc() -> ProjectDoc {
    let mut value = serde_json::to_value(ProjectDoc::empty("caps", "Caps")).unwrap();
    value["definitions"] =
        json!([{"id":"mx","name":"MX","kind":"switch","courtyard":[],"pads":[]}]);
    value["parts"] = json!([
        {"id":"a","definitionId":"mx","reference":"SW1","pose":{"at":{"x":0.0,"y":0.0},"rotation":0.0},"side":"front"},
        {"id":"b","definitionId":"mx","reference":"SW2","pose":{"at":{"x":19.0,"y":0.0},"rotation":0.0},"side":"front"}
    ]);
    value["matrices"] = json!([{"id":"m","rows":1,"columns":2,"pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},"definitionId":"mx","partIds":["a","b"],"boardId":"board",
        "cells":[{"row":0,"column":0,"enabled":true},{"row":0,"column":1,"enabled":true}]}]);
    value["boards"] = json!([{"id":"board","name":"Board","partIds":["a","b"],"outlineIds":[],"netIds":[],"thickness":1.6}]);
    serde_json::from_value(value).unwrap()
}

fn commit_size(engine: &mut CoreEngine, base: u64, operation: EditOperation) -> CoreReply {
    engine.handle(CoreRequest::Edit {
        id: "keycap-size".into(),
        command: EditCommand {
            base_revision: base,
            transaction_id: "keycap-size".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    })
}

fn scene_doc(reply: CoreReply) -> ProjectDoc {
    match reply {
        CoreReply::Scene { document, .. } => *document,
        other => panic!("expected scene, got {other:?}"),
    }
}

/// Ticket 33 RED: a typed keycap-size edit writes sizes, reflow poses and cell
/// offsets atomically in one history entry with Undo/Redo round-trip.
#[test]
fn typed_keycap_size_writes_sizes_moves_and_offsets_in_one_undo_step() {
    let mut engine = CoreEngine::new();
    scene_doc(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: size_doc(),
    }));
    let committed = scene_doc(commit_size(
        &mut engine,
        0,
        EditOperation::SetKeycapSize {
            sizes: [("a".to_owned(), Vec2 { x: 37.0, y: 18.0 })].into(),
            moves: vec![Position {
                id: "b".into(),
                at: Vec2 { x: 28.5, y: 0.0 },
            }],
            offsets: vec![KeycapCellOffset {
                matrix_id: "m".into(),
                row: 0,
                column: 1,
                offset: Some(Vec2 { x: 9.5, y: 0.0 }),
            }],
        },
    ));
    assert_eq!(
        committed
            .parts
            .iter()
            .find(|part| part.id == "a")
            .unwrap()
            .keycap,
        Some(Vec2 { x: 37.0, y: 18.0 })
    );
    assert_eq!(
        committed
            .parts
            .iter()
            .find(|part| part.id == "b")
            .unwrap()
            .pose
            .at,
        Vec2 { x: 28.5, y: 0.0 }
    );
    assert_eq!(
        committed.matrices[0]
            .cells
            .iter()
            .find(|cell| cell.row == 0 && cell.column == 1)
            .unwrap()
            .offset,
        Some(Vec2 { x: 9.5, y: 0.0 })
    );
    assert_eq!(committed.revision, 1);
    let undone = scene_doc(engine.handle(CoreRequest::Undo { id: "undo".into() }));
    assert_eq!(undone.revision, 2);
    assert!(
        undone
            .parts
            .iter()
            .find(|part| part.id == "a")
            .unwrap()
            .keycap
            .is_none()
    );
    assert_eq!(
        undone
            .parts
            .iter()
            .find(|part| part.id == "b")
            .unwrap()
            .pose
            .at,
        Vec2 { x: 19.0, y: 0.0 }
    );
    let mut redone = scene_doc(engine.handle(CoreRequest::Redo { id: "redo".into() }));
    assert_eq!(redone.revision, 3);
    redone.revision = committed.revision;
    assert_eq!(redone, committed);
}

/// Ticket 33: invalid keycap sizes refuse atomically and change nothing.
#[test]
fn typed_keycap_size_refuses_invalid_writes_atomically() {
    for operation in [
        EditOperation::SetKeycapSize {
            sizes: [("ghost".to_owned(), Vec2 { x: 37.0, y: 18.0 })].into(),
            moves: vec![],
            offsets: vec![],
        },
        EditOperation::SetKeycapSize {
            sizes: [(
                "a".to_owned(),
                Vec2 {
                    x: f64::NAN,
                    y: 18.0,
                },
            )]
            .into(),
            moves: vec![],
            offsets: vec![],
        },
        EditOperation::SetKeycapSize {
            sizes: [("a".to_owned(), Vec2 { x: 0.0, y: 18.0 })].into(),
            moves: vec![Position {
                id: "b".into(),
                at: Vec2 { x: 28.5, y: 0.0 },
            }],
            offsets: vec![],
        },
        EditOperation::SetKeycapSize {
            sizes: [("a".to_owned(), Vec2 { x: 37.0, y: 18.0 })].into(),
            moves: vec![],
            offsets: vec![KeycapCellOffset {
                matrix_id: "ghost".into(),
                row: 0,
                column: 0,
                offset: None,
            }],
        },
    ] {
        let mut engine = CoreEngine::new();
        let before = scene_doc(engine.handle(CoreRequest::Open {
            id: "open".into(),
            document: size_doc(),
        }));
        let reply = commit_size(&mut engine, 0, operation);
        assert!(matches!(reply, CoreReply::Error { .. }), "got {reply:?}");
        let after = scene_doc(engine.handle(CoreRequest::Snapshot { id: "snap".into() }));
        assert_eq!(after, before, "a refused keycap size changes nothing");
    }
}
