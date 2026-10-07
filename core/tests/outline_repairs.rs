use boardstudio_core::{CoreEngine, model::*};

fn reviung() -> ProjectDoc {
    serde_json::from_str(include_str!("fixtures/reviung41-outline-original.json")).unwrap()
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

fn protected_thumb() -> ProjectDoc {
    let mut document = reviung();
    let result = open(document.clone());
    let gap = result.board_outline_scenes[0]
        .gaps
        .iter()
        .find(|gap| {
            let points = gap
                .points
                .iter()
                .map(|control| {
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
                })
                .collect();
            contains(
                &[Contour {
                    points,
                    hole: false,
                }],
                Vec2 { x: 104.7, y: -80.0 },
            )
        })
        .expect("the repaired thumb recess is selectable")
        .clone();
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut document.outline[0] {
        settings.repair = Some(OutlineRepairSettings {
            keep_gaps: vec![ProtectedOutlineGap {
                id: gap.id,
                points: gap.points,
            }],
            ..Default::default()
        });
    }
    document
}

#[test]
fn annotated_reviung_narrow_recesses_are_repaired_by_default() {
    let result = open(reviung());
    let contours = &result.board_contours[0].contours;
    let missing: Vec<_> = [
        ("left thumb notch", Vec2 { x: 104.7, y: -80.0 }),
        ("right thumb notch", Vec2 { x: 152.7, y: -80.0 }),
        ("controller slot", Vec2 { x: 256.0, y: -15.0 }),
        ("reset recess", Vec2 { x: 280.5, y: -37.4 }),
    ]
    .into_iter()
    .filter_map(|(name, at)| (!contains(contours, at)).then_some(name))
    .collect();
    assert!(
        missing.is_empty(),
        "Unrepaired structural recesses: {missing:?}"
    );
}

#[test]
fn bridges_include_the_matrix_context_of_attached_components() {
    let result = open(reviung());
    let bridge = result.board_outline_scenes[0]
        .bridges
        .iter()
        .find(|bridge| bridge.part_ids.contains(&"main/RST".into()))
        .unwrap();
    assert!(bridge.matrix_ids.contains(&"main-right-keys".into()));
}

#[test]
fn annotated_reviung_keeps_the_wide_centre_valley_and_required_material() {
    let result = open(reviung());
    let contours = &result.board_contours[0].contours;
    assert!(!contains(contours, Vec2 { x: 128.615, y: 5.0 }));
    assert!(contains(
        contours,
        Vec2 {
            x: 128.615,
            y: -30.0
        }
    ));
    assert!(contains(
        contours,
        Vec2 {
            x: 128.615,
            y: -73.33
        }
    ));
    assert_eq!(contours.iter().filter(|contour| !contour.hole).count(), 1);
}

#[test]
fn shallow_stagger_alternatives_are_not_automatically_straightened() {
    let result = open(reviung());
    let contours = &result.board_contours[0].contours;
    assert!(!contains(contours, Vec2 { x: 106.0, y: 6.0 }));
    assert!(!contains(contours, Vec2 { x: 37.0, y: -52.0 }));
}

#[test]
fn keep_gap_preserves_the_recess_and_follows_component_frames() {
    let mut document = protected_thumb();
    let result = open(document.clone());
    assert!(!contains(
        &result.board_contours[0].contours,
        Vec2 { x: 104.7, y: -80.0 }
    ));
    assert!(contains(
        &result.board_contours[0].contours,
        Vec2 { x: 152.7, y: -80.0 }
    ));
    for part in &mut document.parts {
        part.pose.at.x += 25.0;
    }
    for matrix in &mut document.matrices {
        matrix.origin.x += 25.0;
    }
    let moved = open(document);
    assert!(!contains(
        &moved.board_contours[0].contours,
        Vec2 { x: 129.7, y: -80.0 }
    ));
    assert!(contains(
        &moved.board_contours[0].contours,
        Vec2 { x: 177.7, y: -80.0 }
    ));
}

#[test]
fn changed_recess_identity_still_exposes_its_original_protection_for_removal() {
    let mut doc = reviung();
    let original = open(doc.clone()).board_outline_scenes[0].gaps[0].clone();
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
        settings.repair = Some(OutlineRepairSettings {
            keep_gaps: vec![ProtectedOutlineGap {
                id: original.id.clone(),
                points: original.points,
            }],
            ..Default::default()
        });
    }
    doc.parts
        .iter_mut()
        .find(|part| part.id == "main/U1")
        .unwrap()
        .pose
        .at
        .x += 0.8;
    let after = open(doc);
    let protected = after.board_outline_scenes[0]
        .gaps
        .iter()
        .find(|gap| gap.protected)
        .unwrap();
    assert_ne!(
        protected.id, original.id,
        "This regression must change the derived recess identity"
    );
    let value = serde_json::to_value(protected).unwrap();
    assert_eq!(value["protectedIds"], serde_json::json!([original.id]));
}

#[test]
fn missing_keep_gap_source_retains_last_valid_geometry_and_blocks_only_generated() {
    let mut engine = CoreEngine::new();
    let document = protected_thumb();
    let source = match &document.outline[0] {
        OutlineFeature::PartEnvelope { settings, .. } => {
            settings.repair.as_ref().unwrap().keep_gaps[0]
                .points
                .iter()
                .find_map(|point| point.part_id.clone())
                .unwrap()
        }
        _ => unreachable!(),
    };
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
            .is_some_and(|state| state.generated_last_valid.is_some()),
        "Recovery must survive saving/reopening"
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
fn authored_cutouts_and_a_smaller_gap_limit_survive_automatic_cleanup() {
    let mut document = reviung();
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut document.outline[0] {
        settings.repair = Some(OutlineRepairSettings {
            maximum_gap_span: 12.0,
            ..Default::default()
        });
    }
    document.outline.push(OutlineFeature::Rect {
        id: "intentional-hole".into(),
        anchor_part_id: None,
        rotation: None,
        center: Vec2 {
            x: 128.615,
            y: -73.33,
        },
        size: Vec2 { x: 5.0, y: 5.0 },
        radius: 1.0,
        operation: Operation::Subtract,
    });
    document.boards[0]
        .outline_ids
        .push("intentional-hole".into());
    let result = open(document);
    let contours = &result.board_contours[0].contours;
    assert!(!contains(
        contours,
        Vec2 {
            x: 128.615,
            y: -73.33
        }
    ));
    assert!(
        !contains(contours, Vec2 { x: 256.0, y: -15.0 }),
        "An 18.7 mm controller mouth exceeds the 12 mm setting"
    );
    assert!(contains(contours, Vec2 { x: 104.7, y: -80.0 }));
}
