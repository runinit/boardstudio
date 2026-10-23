use boardstudio_core::{CoreEngine, model::*};

fn synthetic_document() -> ProjectDoc {
    let mut document: ProjectDoc =
        serde_json::from_str(include_str!("fixtures/synthetic-outline.json")).unwrap();
    document.format_version = boardstudio_core::migrate::CURRENT_VERSION;
    document
}

fn open(document: ProjectDoc) -> SceneDelta {
    match CoreEngine::new().handle(CoreRequest::Open {
        id: "outline-regression".into(),
        document,
    }) {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

// Independent oracle: deliberately not the production predicate it checks.
fn contains(contours: &[Contour], at: Vec2) -> bool {
    contours.iter().fold(false, |inside, contour| {
        let mut hit = false;
        for (a, b) in contour
            .points
            .iter()
            .zip(contour.points.iter().cycle().skip(1))
            .take(contour.points.len())
        {
            if (a.y > at.y) != (b.y > at.y) && at.x < (b.x - a.x) * (at.y - a.y) / (b.y - a.y) + a.x
            {
                hit = !hit;
            }
        }
        inside != hit
    })
}

fn world_point(document: &ProjectDoc, control: &OutlineControlPoint) -> Vec2 {
    let Some(id) = &control.part_id else {
        return control.at;
    };
    let part = document.parts.iter().find(|part| &part.id == id).unwrap();
    let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
    let x = if part.side == Side::Back {
        -control.at.x
    } else {
        control.at.x
    };
    Vec2 {
        x: part.pose.at.x + x * cos - control.at.y * sin,
        y: part.pose.at.y + x * sin + control.at.y * cos,
    }
}

fn gap_probe(document: &ProjectDoc, gap: &OutlineGap) -> Vec2 {
    let points: Vec<_> = gap
        .points
        .iter()
        .map(|control| world_point(document, control))
        .collect();
    let count = points.len() as f64;
    Vec2 {
        x: points.iter().map(|point| point.x).sum::<f64>() / count,
        y: points.iter().map(|point| point.y).sum::<f64>() / count,
    }
}

fn protected_gap_document() -> (ProjectDoc, Vec2, String) {
    let mut document = synthetic_document();
    // Two authored square courtyards with a 4 mm gap produce automatic bridge pockets,
    // independent of recesses in any shipped board.
    document
        .parts
        .iter_mut()
        .find(|part| part.id == "main/SW2")
        .expect("the second synthetic switch exists")
        .pose
        .at = Vec2 { x: 24.0, y: 0.0 };
    let OutlineFeature::PartEnvelope {
        part_ids,
        margin,
        settings,
        ..
    } = &mut document.outline[0]
    else {
        unreachable!("the synthetic outline is a part envelope")
    };
    *part_ids = vec!["main/SW1".into(), "main/SW2".into()];
    *margin = 1.0;
    settings.bridge_width = 10.0;
    document.boards[0].part_ids = vec!["main/SW1".into(), "main/SW2".into()];
    let result = open(document.clone());
    let gap = result.board_outline_scenes[0]
        .gaps
        .first()
        .expect("the synthetic component bridge exposes a candidate gap")
        .clone();
    let probe = gap_probe(&document, &gap);
    let source = gap
        .points
        .iter()
        .find_map(|point| point.part_id.clone())
        .expect("the selected concavity follows a component frame");
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut document.outline[0] {
        settings.repair = Some(OutlineRepairSettings {
            keep_gaps: vec![ProtectedOutlineGap {
                id: gap.id,
                points: gap.points,
            }],
            ..Default::default()
        });
    }
    (document, probe, source)
}

#[test]
fn default_outline_uses_the_synthetic_component_envelopes() {
    let result = open(synthetic_document());
    assert!(!result.board_contours[0].contours.is_empty());
    // The fixture places a switch courtyard at the origin, so its centre belongs to the board.
    assert!(contains(
        &result.board_contours[0].contours,
        Vec2 { x: 0.0, y: 0.0 }
    ));
}

fn two_matrix_component_outline(second_x: f64, second_y: f64) -> ProjectDoc {
    let mut document = synthetic_document();
    document.outline.truncate(1);
    document.boards[0].outline_ids = vec!["board-envelope".into()];
    let second = document
        .parts
        .iter_mut()
        .find(|part| part.id == "matrix/matrix/r0c1")
        .unwrap();
    second.pose.at = Vec2 {
        x: second_x,
        y: second_y,
    };
    document.matrices[0].pitch.x = second_x;
    {
        let values = vec![0.0, second_y];
        let count = values.len().max(document.matrices[0].column_layout.len());
        document.matrices[0]
            .column_layout
            .resize_with(count, Default::default);
        for record in &mut document.matrices[0].column_layout {
            record.stagger = 0.0;
        }
        for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
            record.stagger = value;
        }
    }
    if let OutlineFeature::PartEnvelope {
        part_ids,
        margin,
        settings,
        ..
    } = &mut document.outline[0]
    {
        *part_ids = vec!["matrix/matrix/r0c0".into(), "matrix/matrix/r0c1".into()];
        *margin = 0.0;
        settings.size = 0.0;
        settings.bridge_width = 20.0;
    }
    document.boards[0].part_ids = vec!["matrix/matrix/r0c0".into(), "matrix/matrix/r0c1".into()];
    document
}

#[test]
fn an_automatic_bridge_exposes_the_matrix_context_of_its_members() {
    let result = open(two_matrix_component_outline(24.0, 0.0));
    let bridge = result.board_outline_scenes[0]
        .bridges
        .iter()
        .find(|bridge| bridge.part_ids.contains(&"matrix/matrix/r0c0".into()))
        .expect("the authored component gap produces an automatic bridge");
    assert_eq!(
        bridge.part_ids,
        vec!["matrix/matrix/r0c0", "matrix/matrix/r0c1"]
    );
    assert_eq!(bridge.matrix_ids, vec!["matrix"]);
    assert!(!bridge.authored);
}

#[test]
fn a_shallow_matrix_stagger_remains_an_open_outline_alternative() {
    let result = open(two_matrix_component_outline(19.0, 3.0));
    let scene = &result.board_outline_scenes[0];
    assert!(
        scene.bridges.is_empty(),
        "shallow steps are not converted into automatic bridges: {:?}",
        scene.bridges
    );
    let retained_shallow_corner = result.board_contours[0]
        .contours
        .iter()
        .flat_map(|contour| &contour.points)
        .any(|point| (point.x - 9.0).abs() < 0.1 && (point.y - 8.0).abs() < 0.1);
    assert!(
        retained_shallow_corner,
        "the 3 mm authored stagger retains its independently derived shallow boundary corner"
    );
}

#[test]
fn a_kept_gap_follows_its_component_frame_when_the_board_moves() {
    let (mut document, probe, _) = protected_gap_document();
    let result = open(document.clone());
    assert!(!contains(&result.board_contours[0].contours, probe));

    for part in &mut document.parts {
        part.pose.at.x += 25.0;
    }
    for matrix in &mut document.matrices {
        matrix.origin.x += 25.0;
    }
    let moved = open(document);
    assert!(!contains(
        &moved.board_contours[0].contours,
        Vec2 {
            x: probe.x + 25.0,
            y: probe.y
        }
    ));
}

#[test]
fn changing_a_recess_identity_keeps_its_original_protection_removable() {
    let (mut document, _, source) = protected_gap_document();
    let protected_id = match &document.outline[0] {
        OutlineFeature::PartEnvelope { settings, .. } => {
            settings.repair.as_ref().unwrap().keep_gaps[0].id.clone()
        }
        _ => unreachable!(),
    };
    let original = open(document.clone()).board_outline_scenes[0]
        .gaps
        .iter()
        .find(|gap| gap.id == protected_id)
        .unwrap()
        .clone();
    document
        .parts
        .iter_mut()
        .find(|part| part.id == source)
        .unwrap()
        .pose
        .at
        .y += 2.0;
    let after = open(document);
    let protected = after.board_outline_scenes[0]
        .gaps
        .iter()
        .find(|gap| gap.protected)
        .expect("the protected gap remains visible after its source moves");
    assert_ne!(protected.id, original.id);
    let value = serde_json::to_value(protected).unwrap();
    assert_eq!(value["protectedIds"], serde_json::json!([original.id]));
}

#[test]
fn removing_a_protected_gap_source_retains_last_valid_geometry() {
    let mut engine = CoreEngine::new();
    let (document, _, source) = protected_gap_document();
    let CoreReply::Scene {
        scene: before,
        document,
        ..
    } = engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    })
    else {
        panic!("open")
    };
    assert!(
        document
            .board_outlines
            .first()
            .is_some_and(|state| state.generated_last_valid.is_some())
    );
    let reply = engine.handle(CoreRequest::Edit {
        id: "remove".into(),
        command: EditCommand {
            base_revision: before.revision,
            transaction_id: "remove".into(),
            phase: EditPhase::Commit,
            target_ids: vec![source.clone()],
            operation: EditOperation::RemoveParts { ids: vec![source] },
        },
    });
    let CoreReply::Scene {
        scene: after,
        document,
        ..
    } = reply
    else {
        panic!("remove")
    };
    assert_eq!(before.board_contours, after.board_contours);
    assert!(!after.board_readiness[0].outline);
    assert!(
        after
            .findings
            .iter()
            .any(|finding| finding.id.contains(":keep-gap:"))
    );
    assert!(!after.finding_markers.is_empty());
    let reopened = open(*document);
    assert_eq!(after.board_contours, reopened.board_contours);
    assert!(!reopened.board_readiness[0].outline);
}

#[test]
fn an_authored_cutout_subtracts_material_at_its_declared_centre() {
    let mut document = synthetic_document();
    document.outline.push(OutlineFeature::Rect {
        id: "synthetic-cutout".into(),
        anchor_part_id: None,
        rotation: None,
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 2.0, y: 2.0 },
        radius: 0.0,
        operation: Operation::Subtract,
    });
    document.boards[0]
        .outline_ids
        .push("synthetic-cutout".into());
    let result = open(document);
    assert!(!contains(
        &result.board_contours[0].contours,
        Vec2 { x: 0.0, y: 0.0 }
    ));
}
