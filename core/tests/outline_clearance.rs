use boardstudio_core::{CoreEngine, model::*};
use serde_json::json;

fn fixture() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("clearance", "Clearance");
    doc.definitions.push(serde_json::from_value(json!({
        "id":"device", "name":"Device", "kind":"controller",
        "courtyard":[{"x":-1,"y":-1},{"x":1,"y":-1},{"x":1,"y":1},{"x":-1,"y":1}],
        "pads":[{"id":"pad","number":"1","at":{"x":0,"y":0},"size":{"x":2,"y":2},"shape":"circle","drill":1}]
    })).unwrap());
    doc.parts.push(serde_json::from_value(json!({"id":"part","definitionId":"device","reference":"U1","side":"front","pose":{"at":{"x":0,"y":0},"rotation":0}})).unwrap());
    doc.boards.push(serde_json::from_value(json!({"id":"board","name":"Board","outlineIds":["perimeter"],"partIds":["part"],"netIds":[],"thickness":1.6})).unwrap());
    doc.outline
        .push(rect("perimeter", 0.0, 0.0, 20.0, 20.0, Operation::Add));
    doc
}
fn rect(id: &str, x: f64, y: f64, width: f64, height: f64, operation: Operation) -> OutlineFeature {
    OutlineFeature::Rect {
        id: id.into(),
        anchor_part_id: None,
        rotation: None,
        center: Vec2 { x, y },
        size: Vec2 {
            x: width,
            y: height,
        },
        radius: 0.0,
        operation,
    }
}
fn open(doc: ProjectDoc) -> SceneDelta {
    match CoreEngine::new().handle(CoreRequest::Open {
        id: "open".into(),
        document: doc,
    }) {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}
fn has(scene: &SceneDelta, suffix: &str) -> bool {
    scene
        .findings
        .iter()
        .any(|finding| finding.id.contains(suffix) && finding.severity == Severity::Error)
}

#[test]
fn a_hole_inside_a_pad_is_detected_even_when_every_pad_vertex_is_in_material() {
    let mut doc = fixture();
    doc.outline
        .push(rect("hole", 0.0, 0.0, 0.2, 0.2, Operation::Subtract));
    doc.boards[0].outline_ids.push("hole".into());
    let scene = open(doc);
    assert!(has(&scene, ":support:"));
    assert!(!scene.board_readiness[0].outline);
    assert!(
        scene
            .finding_markers
            .iter()
            .any(|marker| marker.finding_id.contains(":support:"))
    );
}

#[test]
fn exclusion_and_body_permission_do_not_waive_pad_and_drill_support() {
    let mut doc = fixture();
    doc.parts[0].pose.at.x = 12.0;
    doc.parts[0].outline = Some(PartOutline {
        excluded: true,
        allow_body_overhang: true,
        ..Default::default()
    });
    let scene = open(doc);
    assert!(has(&scene, ":support:"));
    assert!(!has(&scene, ":body:"));
    assert!(!scene.board_readiness[0].outline);
}

#[test]
fn derived_case_closure_clearance_can_cross_the_edge_without_waiving_ordinary_npth_support() {
    let mut doc = fixture();
    doc.parts[0].id = "case-closure/board/10.00000/0.00000".into();
    doc.parts[0].definition_id = "assembly-closure/definition/board/10.00000/0.00000".into();
    doc.boards[0].part_ids = vec![doc.parts[0].id.clone()];
    doc.parts[0].pose.at.x = 10.0;
    doc.parts[0].outline = Some(PartOutline {
        excluded: true,
        ..Default::default()
    });
    doc.definitions[0].id = doc.parts[0].definition_id.clone();
    doc.definitions[0].kind = PartKind::Custom;
    doc.definitions[0].generator = Some(
        serde_json::from_value(json!({
            "source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}
        }))
        .unwrap(),
    );
    doc.definitions[0].pads[0].number.clear();
    doc.definitions[0].pads[0].plated = Some(false);
    let scene = open(doc.clone());
    assert!(scene.board_readiness[0].outline, "{:?}", scene.findings);
    doc.parts[0].id = "ordinary-mounting-hole".into();
    doc.boards[0].part_ids = vec![doc.parts[0].id.clone()];
    assert!(has(&open(doc), ":support:"));
}

#[test]
fn another_boards_invalid_support_and_layout_do_not_block_valid_output() {
    let mut doc = fixture();
    let mut bad = doc.boards[0].clone();
    bad.id = "bad".into();
    bad.part_ids = vec!["missing-definition".into()];
    let mut part = doc.parts[0].clone();
    part.id = "missing-definition".into();
    part.definition_id = "absent".into();
    doc.parts.push(part);
    doc.boards.push(bad);
    let scene = open(doc);
    let good = scene
        .board_readiness
        .iter()
        .find(|board| board.board_id == "board")
        .unwrap();
    let bad = scene
        .board_readiness
        .iter()
        .find(|board| board.board_id == "bad")
        .unwrap();
    assert!(good.outline && good.pcb);
    assert!(!bad.pcb);
}

#[test]
fn unintended_islands_and_an_authored_thin_neck_are_export_blockers() {
    let mut doc = fixture();
    doc.outline
        .push(rect("island", 30.0, 0.0, 10.0, 10.0, Operation::Add));
    doc.boards[0].outline_ids.push("island".into());
    assert!(has(&open(doc.clone()), ":disconnected"));
    doc.outline
        .push(rect("neck", 17.5, 0.0, 20.0, 0.5, Operation::Add));
    doc.boards[0].outline_ids.push("neck".into());
    let scene = open(doc);
    assert!(!has(&scene, ":disconnected"));
    assert!(has(&scene, ":connection:"));
    assert!(!scene.board_readiness[0].outline);
}
