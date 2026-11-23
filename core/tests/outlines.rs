use crate::support::fixture;
use crate::support::query;
use crate::support::session::Session;
use crate::support::session::builders::{
    anchored_polygon, move_key, rect, remove_parts, replace_document,
};
use boardstudio_core::model::*;
use serde_json::json;

fn document() -> ProjectDoc {
    serde_json::from_value(json!({
        "id":"outline-test", "name":"Outline test", "format":"boardstudio/v2", "formatVersion":boardstudio_core::document_format::CURRENT_VERSION, "revision":0, "parameters":{}, "scripts":[],
        "definitions":[{"id":"switch", "name":"Switch", "kind":"switch",
            "courtyard":[{"x":-9,"y":-9},{"x":9,"y":-9},{"x":9,"y":9},{"x":-9,"y":9}],"pads":[]}],
        "parts":(0..9).map(|i|json!({"id":format!("k{i}"),"definitionId":"switch","reference":format!("SW{i}"),
            "pose":{"at":{"x":(i%3) as f64*19.05,"y":(i/3) as f64*19.05},"rotation":0},"side":"front"})).collect::<Vec<_>>(),
        "outline":[{"id":"edge","kind":"part-envelope","partIds":(0..9).map(|i|format!("k{i}")).collect::<Vec<_>>(),"margin":4,"operation":"add"}],
        "boards":[{"id":"board","name":"Board","partIds":(0..9).map(|i|format!("k{i}")).collect::<Vec<_>>(),"outlineIds":["edge"],"netIds":[],"thickness":1.6}],
        "nets":[],"constraints":[],"matrices":[],"caseBodies":[],"assets":[],"materials":[]
    })).unwrap()
}
fn matrix_document() -> ProjectDoc {
    let mut doc = document();
    for (index, part) in doc.parts.iter_mut().enumerate() {
        part.id = format!("matrix/main/r{}c{}", index / 3, index % 3);
    }
    doc.boards[0].part_ids = doc.parts.iter().map(|part| part.id.clone()).collect();
    if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut doc.outline[0] {
        *part_ids = doc.boards[0].part_ids.clone();
    }
    doc
}

fn scene(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } | CoreReply::Preview { scene, .. } => scene,
        other => panic!("{other:?}"),
    }
}
#[test]
fn deleting_corner_creates_notch_and_retains_neighbors() {
    let mut core = Session::default();
    let before = scene(core.open(document()));
    assert!(query::contains(&before.contours, 0.0, 0.0));
    let after = scene(core.commit(
        0,
        EditOperation::RemoveParts {
            ids: vec!["k0".into()],
        },
    ));
    assert!(
        !query::contains(&after.contours, 0.0, 0.0),
        "deleted corner must be outside the contour"
    );
    assert!(query::contains(&after.contours, 19.05, 0.0));
    assert!(query::contains(&after.contours, 0.0, 19.05));
}

#[test]
fn automatic_part_envelopes_exclude_nonphysical_utilities() {
    let mut doc = document();
    doc.definitions.push(PartDefinition {
        courtyard: vec![
            Vec2 { x: -10.0, y: -10.0 },
            Vec2 { x: 10.0, y: -10.0 },
            Vec2 { x: 10.0, y: 10.0 },
            Vec2 { x: -10.0, y: 10.0 },
        ],
        ..fixture::part_definition("utility", "Board note", PartKind::Utility)
    });
    doc.parts.push(Part {
        pose: Pose2 {
            at: Vec2 { x: 100.0, y: 0.0 },
            rotation: 0.0,
        },
        ..fixture::part("label", "utility", "TXT")
    });
    doc.boards[0].part_ids.push("label".into());
    if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut doc.outline[0] {
        part_ids.push("label".into());
    }
    let mut core = Session::default();
    let resolved = scene(core.open(doc));
    assert!(query::contains(&resolved.contours, 19.05, 0.0));
    assert!(!query::contains(&resolved.contours, 100.0, 0.0));
}

#[test]
fn automatic_envelope_reflects_asymmetric_back_side_footprints() {
    let mut doc = document();
    doc.definitions[0].kind = PartKind::Passive;
    doc.definitions[0].courtyard = vec![
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 6.0, y: 0.0 },
        Vec2 { x: 6.0, y: 2.0 },
        Vec2 { x: 0.0, y: 2.0 },
    ];
    doc.parts = vec![Part {
        anchor: None,
        id: "back-part".into(),
        definition_id: "switch".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Back,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }];
    doc.boards[0].part_ids = vec!["back-part".into()];
    doc.boards[0].outline_ids = vec!["edge".into()];
    doc.outline[0] = OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "edge".into(),
        part_ids: vec!["back-part".into()],
        margin: 0.0,
        operation: Operation::Add,
    };

    let mut core = Session::default();
    let resolved = scene(core.open(doc));
    assert!(query::contains(&resolved.contours, -3.0, 1.0));
    assert!(!query::contains(&resolved.contours, 3.0, 1.0));
}

#[test]
fn deleted_matrix_cell_stays_disabled_when_resized() {
    let mut core = Session::default();
    let mut doc = document();
    doc.parts.clear();
    doc.outline.clear();
    doc.boards.clear();
    scene(core.open(doc));
    core.commit(
        0,
        fixture::set_matrix(Matrix {
            pitch: Vec2 { x: 19.05, y: 19.05 },
            ..fixture::matrix("keys", 3, 3)
        }),
    );
    core.commit(1, remove_parts(&["matrix/keys/r0c0"]));
    let doc = core.snapshot();
    let mut matrix = doc.matrices[0].clone();
    matrix.columns = 4;
    core.commit(2, fixture::set_matrix(matrix));
    let doc = core.snapshot();
    assert!(!doc.parts.iter().any(|p| p.id == "matrix/keys/r0c0"));
}
#[test]
fn finishing_modes_and_holes_are_shared_with_board_contours() {
    for corners in ["sharp", "fillet", "chamfer"] {
        let mut doc = document();
        let mut feature = serde_json::to_value(&doc.outline[0]).unwrap();
        feature["settings"] = json!({"corners":corners,"size":2,"bridgeWidth":10});
        doc.outline[0] = serde_json::from_value(feature).unwrap();
        doc.outline.push(rect(
            "hole",
            Vec2 { x: 19.05, y: 19.05 },
            Vec2 { x: 6.0, y: 6.0 },
            Operation::Subtract,
        ));
        doc.boards[0].outline_ids.push("hole".into());
        let mut core = Session::default();
        let result = scene(core.open(doc));
        assert!(
            !result
                .findings
                .iter()
                .any(|f| f.scope == Scope::Outline && f.severity == Severity::Error),
            "{:?}",
            result.findings
        );
        assert!(!query::contains(&result.contours, 19.05, 19.05));
        assert_eq!(result.board_contours[0].contours, result.contours);
        assert!(query::contains(&result.contours, 0.0, 0.0));
        if corners != "sharp" {
            assert!(result.contours[0].points.len() > 4);
        }
    }
}
#[test]
fn invalid_polygon_and_empty_selection_block_outline() {
    for feature in [
        json!({"id":"bad","kind":"polygon","points":[{"x":0,"y":0},{"x":10,"y":10},{"x":0,"y":10},{"x":10,"y":0}],"operation":"add"}),
        json!({"id":"bad","kind":"part-envelope","partIds":[],"margin":4,"operation":"add"}),
    ] {
        let mut doc = document();
        doc.outline = vec![serde_json::from_value(feature).unwrap()];
        doc.boards[0].outline_ids = vec!["bad".into()];
        let result = scene(Session::default().open(doc));
        assert!(!result.readiness.outline);
        assert!(!result.board_readiness[0].outline);
    }
}
#[test]
fn components_can_use_zero_margin_or_be_excluded() {
    let mut doc = document();
    doc.definitions[0].kind = PartKind::Controller;
    doc.parts.truncate(1);
    doc.parts[0].outline = Some(PartOutline {
        excluded: false,
        margin: Some(0.0),
        ..Default::default()
    });
    let mut core = Session::default();
    let result = scene(core.open(doc.clone()));
    assert!(!query::contains(&result.contours, 10.0, 0.0));
    assert!(query::contains(&result.contours, 8.0, 0.0));
    doc.parts[0].outline.as_mut().unwrap().excluded = true;
    let result = scene(core.open(doc));
    assert!(!result.readiness.outline);
}
#[test]
fn keycap_dimensions_rotation_and_settings_roundtrip() {
    let mut doc = document();
    doc.parts.truncate(1);
    doc.definitions[0].keycap = Some(Vec2 { x: 30.0, y: 18.0 });
    doc.parts[0].keycap = Some(Vec2 { x: 40.0, y: 18.0 });
    doc.parts[0].pose.rotation = 90.0;
    let saved = serde_json::to_string(&doc).unwrap();
    let restored: ProjectDoc = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored, doc);
    let result = scene(Session::default().open(restored));
    assert!(query::contains(&result.contours, 0.0, 22.0));
    assert!(!query::contains(&result.contours, 22.0, 0.0));
}
#[test]
fn islands_bridge_interior_gaps_fill_and_undo_restores_corner() {
    let mut doc = document();
    doc.parts.retain(|p| p.id != "k4");
    let mut core = Session::default();
    let before = scene(core.open(doc));
    assert!(query::contains(&before.contours, 19.05, 19.05));
    let after = scene(core.commit(0, remove_parts(&["k0"])));
    assert!(!query::contains(&after.contours, 0.0, 0.0));
    let undone = scene(core.undo());
    assert_eq!(undone.contours, before.contours);
    let redone = scene(core.redo());
    assert_eq!(redone.contours, after.contours);
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at.x = 80.0;
    let result = scene(core.open(doc));
    assert_eq!(result.contours.len(), 1);
    assert!(query::contains(&result.contours, 40.0, 0.0));
}

#[test]
fn oversized_finishing_reports_requested_and_applied_sizes() {
    let mut doc = document();
    // This test isolates corner fitting. Body/support conflicts have separate checks.
    for part in &mut doc.parts {
        part.outline = Some(PartOutline {
            allow_body_overhang: true,
            ..Default::default()
        });
    }
    let mut feature = serde_json::to_value(&doc.outline[0]).unwrap();
    feature["settings"] = json!({"corners":"fillet","size":1000,"bridgeWidth":10});
    doc.outline[0] = serde_json::from_value(feature).unwrap();
    let result = scene(Session::default().open(doc));
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.severity == Severity::Warning
                && f.message.contains("1000 mm")
                && f.message.contains("as little as"))
    );
    assert!(result.board_readiness[0].outline);
}
#[test]
fn separate_boards_are_not_bridged_and_mixed_membership_is_rejected() {
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at.x = 100.0;
    let mut right = doc.boards[0].clone();
    right.id = "right".into();
    right.part_ids = vec!["k1".into()];
    right.outline_ids = vec!["right-edge".into()];
    doc.boards[0].part_ids = vec!["k0".into()];
    doc.boards.push(right);
    doc.outline=vec![
        serde_json::from_value(json!({"id":"edge","kind":"part-envelope","partIds":["k0"],"margin":4,"operation":"add"})).unwrap(),
        serde_json::from_value(json!({"id":"right-edge","kind":"part-envelope","partIds":["k1"],"margin":4,"operation":"add"})).unwrap()];
    let result = scene(Session::default().open(doc.clone()));
    assert_eq!(result.board_contours.len(), 2);
    assert!(!query::contains(&result.contours, 50.0, 0.0));
    if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut doc.outline[0] {
        part_ids.push("k1".into());
    }
    let result = scene(Session::default().open(doc));
    assert!(!result.board_readiness[0].outline);
}
#[test]
fn definition_edits_invalidate_geometry_and_preserve_manual_additions() {
    let mut doc = document();
    doc.parts.truncate(1);
    doc.definitions[0].keycap = Some(Vec2 { x: 18.0, y: 18.0 });
    doc.outline.push(rect(
        "tab",
        Vec2 { x: 15.0, y: 0.0 },
        Vec2 { x: 10.0, y: 6.0 },
        Operation::Add,
    ));
    doc.boards[0].outline_ids.push("tab".into());
    let mut core = Session::default();
    let before = scene(core.open(doc.clone()));
    assert!(query::contains(&before.contours, 19.0, 0.0));
    doc.definitions[0].keycap = Some(Vec2 { x: 50.0, y: 18.0 });
    let after = scene(core.commit(0, replace_document(doc)));
    assert!(query::contains(&after.contours, 28.0, 0.0));
    let undone = scene(core.undo());
    assert_eq!(undone.contours, before.contours);
}

#[test]
fn grid_aligned_matrix_does_not_report_spurious_corner_reductions() {
    let mut doc = document();
    doc.parts = (0..15)
        .map(|i| {
            let mut p = doc.parts[0].clone();
            p.id = format!("k{i}");
            p.pose.at = Vec2 {
                x: (i % 5) as f64 * 19.05,
                y: -((i / 5) as f64) * 19.05,
            };
            p
        })
        .collect();
    doc.boards[0].part_ids = doc.parts.iter().map(|p| p.id.clone()).collect();
    if let OutlineFeature::PartEnvelope { part_ids, .. } = &mut doc.outline[0] {
        *part_ids = doc.boards[0].part_ids.clone();
    }
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
        settings.corners = CornerStyle::Fillet;
    }
    let result = scene(Session::default().open(doc));
    assert!(
        !result
            .findings
            .iter()
            .any(|f| f.message.contains("Corner size reduced")),
        "{:?}",
        result.findings
    );
    for point in result.contours.iter().flat_map(|c| &c.points) {
        assert!((point.x * 1000.0 - (point.x * 1000.0).round()).abs() < 1e-6);
        assert!((point.y * 1000.0 - (point.y * 1000.0).round()).abs() < 1e-6);
    }
}

#[test]
fn edge_notch_is_preserved_and_interior_deletion_is_filled() {
    let mut doc = document();
    doc.parts.retain(|p| p.id != "k1" && p.id != "k4");
    // The 11.15 mm mouth qualifies for cleanup by default. A lower limit preserves it.
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
        settings.repair = Some(OutlineRepairSettings {
            maximum_gap_span: 10.0,
            ..Default::default()
        });
    }
    let result = scene(Session::default().open(doc));
    // This void reaches the perimeter, so it is not an incidental enclosed gap.
    assert!(!query::contains(&result.contours, 19.05, 0.0));
    assert!(!query::contains(&result.contours, 19.05, 19.05));
    assert!(query::contains(&result.contours, 0.0, 19.05));
}

#[test]
fn matrices_follow_stagger_rotation_growth_and_shrink() {
    let mut doc = document();
    doc.parts.clear();
    doc.boards[0].part_ids.clear();
    let mut core = Session::default();
    scene(core.open(doc));
    let mut matrix: Matrix = serde_json::from_value(json!({"id":"keys","boardId":"board","rows":3,"columns":3,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"switch","partIds":[],"rotation":30,"rowLayout": [{"offset": {"x": 0, "y": 0}}, {"offset": {"x": 5, "y": 0}}, {"offset": {"x": 10, "y": 0}}],
"cells":[{"row":0,"column":0,"enabled":false}]})).unwrap();
    let rotated = scene(core.commit(0, fixture::set_matrix(matrix.clone())));
    assert!(!query::contains(&rotated.contours, 0.0, 0.0));
    assert!(query::contains(
        &rotated.contours,
        19.05 * 30f64.to_radians().cos(),
        19.05 * 30f64.to_radians().sin()
    ));
    matrix.rows = 4;
    matrix.columns = 4;
    let grown = scene(core.commit(1, fixture::set_matrix(matrix.clone())));
    assert!(!query::contains(&grown.contours, 0.0, 0.0));
    let p = 57.15;
    let (sin, cos) = 30f64.to_radians().sin_cos();
    assert!(query::contains(
        &grown.contours,
        p * cos - p * sin,
        p * sin + p * cos
    ));
    matrix.rows = 2;
    matrix.columns = 2;
    let shrunk = scene(core.commit(2, fixture::set_matrix(matrix.clone())));
    assert!(!query::contains(
        &shrunk.contours,
        p * cos - p * sin,
        p * sin + p * cos
    ));
    let second: Matrix = serde_json::from_value(json!({"id":"right","boardId":"board","rows":1,"columns":1,"pitch":{"x":19.05,"y":19.05},"origin":{"x":120,"y":0},"definitionId":"switch","partIds":[]})).unwrap();
    let joined = scene(core.commit(3, fixture::set_matrix(second)));
    assert_eq!(joined.board_contours[0].contours.len(), 1);
    assert!(query::contains(&joined.contours, 120.0, 0.0));
}

#[test]
fn touching_islands_get_a_finite_width_bridge() {
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at = Vec2 { x: 18.0, y: 18.0 };
    if let OutlineFeature::PartEnvelope { margin, .. } = &mut doc.outline[0] {
        *margin = 0.0;
    }
    let result = scene(Session::default().open(doc));
    assert!(result.board_readiness[0].outline, "{:?}", result.findings);
    assert_eq!(result.contours.len(), 1);
    assert!(query::contains(&result.contours, 8.0, 10.0));
}

#[test]
fn finishing_cannot_leave_a_cutout_outside_its_exterior() {
    let mut doc = document();
    doc.parts.truncate(1);
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
        settings.corners = CornerStyle::Fillet;
        settings.size = 100.0;
    }
    doc.outline.push(rect(
        "hole",
        Vec2 { x: 12.0, y: 12.0 },
        Vec2 { x: 0.5, y: 0.5 },
        Operation::Subtract,
    ));
    doc.boards[0].outline_ids.push("hole".into());
    let result = scene(Session::default().open(doc));
    assert!(!result.board_readiness[0].outline, "{:?}", result.findings);
}

#[test]
fn current_matrix_deletions_record_disabled_cells_before_resizing() {
    let mut doc = matrix_document();
    doc.matrices.push(serde_json::from_value(json!({"id":"main","rows":3,"columns":3,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"switch","partIds":doc.boards[0].part_ids})).unwrap());
    let mut core = Session::default();
    scene(core.open(doc));
    let removed = scene(core.commit(0, remove_parts(&["matrix/main/r0c0"])));
    assert!(
        !removed
            .findings
            .iter()
            .any(|f| f.message.contains("Matrix member count")),
        "{:?}",
        removed.findings
    );
    core.commit(1, remove_parts(&["matrix/main/r0c1"]));
    let doc = core.snapshot();
    assert!(
        doc.matrices[0]
            .cells
            .iter()
            .any(|c| c.row == 0 && c.column == 0 && !c.enabled)
    );
    assert!(
        doc.matrices[0]
            .cells
            .iter()
            .any(|c| c.row == 0 && c.column == 1 && !c.enabled)
    );
    let mut matrix = doc.matrices[0].clone();
    matrix.columns = 4;
    let grown = scene(core.commit(2, fixture::set_matrix(matrix)));
    assert!(!query::contains(&grown.contours, 0.0, 0.0));
    assert!(!query::contains(&grown.contours, 19.05, 0.0));
}

#[test]
fn subgrid_bridge_cannot_export_a_self_touching_sharp_outline() {
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at.x = 80.0;
    if let OutlineFeature::PartEnvelope { settings, .. } = &mut doc.outline[0] {
        settings.bridge_width = 0.0001;
    }
    let result = scene(Session::default().open(doc));
    assert!(
        !result.board_readiness[0].outline,
        "sub-grid bridges must not yield an exportable self-touching perimeter"
    );
}

#[test]
fn editing_current_matrix_preserves_member_ids_coordinates_and_nets() {
    let mut doc = matrix_document();
    let matrix: Matrix = serde_json::from_value(json!({"id":"main", "rows":3,"columns":3,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"switch","partIds":(0..9).map(|i|format!("matrix/main/r{}c{}",i/3,i%3)).collect::<Vec<_>>()})).unwrap();
    doc.matrices.push(matrix.clone());
    doc.nets.push(Net {
        id: "original-net".into(),
        name: "ROW0".into(),
        pins: vec![Pin {
            part_id: "matrix/main/r0c0".into(),
            pad_id: "1".into(),
        }],
    });
    let original = doc.parts.clone();
    let mut core = Session::default();
    scene(core.open(doc));
    let reply = core.send(fixture::commit(
        "edit",
        "current-edit",
        0,
        serde_json::from_value(json!({"kind":"set-matrix", "matrix":matrix})).unwrap(),
    ));
    let CoreReply::Scene {
        document: updated, ..
    } = reply
    else {
        panic!("{reply:?}")
    };
    assert_eq!(
        updated
            .parts
            .iter()
            .map(|part| &part.id)
            .collect::<Vec<_>>(),
        original.iter().map(|part| &part.id).collect::<Vec<_>>()
    );
    for (actual, expected) in updated.parts.iter().zip(original.iter()) {
        assert_eq!(actual.pose, expected.pose);
    }
    assert_eq!(
        updated
            .nets
            .iter()
            .find(|net| net.id == "original-net")
            .unwrap()
            .pins[0]
            .part_id,
        "matrix/main/r0c0"
    );
}

#[test]
fn manual_connection_precedes_automatic_bridging() {
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at.x = 80.0;
    doc.boards[0].part_ids = vec!["k0".into(), "k1".into()];
    doc.outline[0] = serde_json::from_value(json!({
        "kind":"part-envelope", "id":"edge", "partIds":["k0","k1"], "margin":4, "operation":"add",
        "connections":[{"id":"route","width":4,"points":[
            {"at":{"x":0,"y":0},"partId":"k0"}, {"at":{"x":0,"y":40}},
            {"at":{"x":80,"y":40}}, {"at":{"x":0,"y":0},"partId":"k1"}
        ]}]
    }))
    .unwrap();
    let result = scene(Session::default().open(doc));
    assert!(result.readiness.outline, "{:?}", result.findings);
    assert!(query::contains(&result.contours, 40.0, 40.0));
    assert!(!query::contains(&result.contours, 40.0, 0.0));
}

#[test]
fn attached_addition_moves_and_detaches_on_anchor_deletion() {
    let mut core = Session::default();
    let mut doc = document();
    doc.parts[0].pose.at = Vec2 { x: 100.0, y: 0.0 };
    doc.outline.push(anchored_polygon(
        "tab",
        "k0",
        &[(8.0, -4.0), (30.0, -4.0), (30.0, 4.0), (8.0, 4.0)],
        Operation::Add,
    ));
    doc.boards[0].outline_ids.push("tab".into());
    let opened = scene(core.open(doc));
    assert!(query::contains(&opened.contours, 125.0, 0.0));
    let moved = scene(core.commit(
        0,
        EditOperation::MoveParts {
            positions: vec![Position {
                id: "k0".into(),
                at: Vec2 { x: 120.0, y: 0.0 },
            }],
        },
    ));
    assert!(query::contains(&moved.contours, 145.0, 0.0));
    let removed = core.send(CoreRequest::Edit {
        id: "remove".into(),
        command: EditCommand {
            base_revision: 1,
            transaction_id: "remove".into(),
            phase: EditPhase::Commit,
            target_ids: vec!["k0".into()],
            operation: EditOperation::RemoveParts {
                ids: vec!["k0".into()],
            },
        },
    });
    let CoreReply::Scene {
        document, scene, ..
    } = removed
    else {
        panic!("Expected scene")
    };
    assert!(query::contains(&scene.contours, 145.0, 0.0));
    let tab =
        serde_json::to_value(document.outline.iter().find(|f| f.id() == "tab").unwrap()).unwrap();
    assert!(tab.get("anchorPartId").is_none());
}

#[test]
fn invalid_connections_and_dangling_attachments_block_outline_readiness() {
    for control in [
        json!({"id":"c","width":0,"points":[{"at":{"x":0,"y":0}},{"at":{"x":30,"y":0}}]}),
        json!({"id":"c","width":2,"points":[{"at":{"x":0,"y":0}},{"at":{"x":0,"y":0}}]}),
        json!({"id":"c","width":2,"points":[{"at":{"x":0,"y":0},"partId":"missing"},{"at":{"x":30,"y":0}}]}),
        json!({"id":"c","width":0.0001,"points":[{"at":{"x":0,"y":0}},{"at":{"x":100,"y":0}}]}),
    ] {
        let mut doc = document();
        if let OutlineFeature::PartEnvelope { connections, .. } = &mut doc.outline[0] {
            *connections = vec![serde_json::from_value(control).unwrap()];
        }
        let result = scene(Session::default().open(doc));
        assert!(!result.readiness.outline, "{:?}", result.findings);
        assert!(
            result
                .findings
                .iter()
                .any(|finding| finding.severity == Severity::Error)
        );
    }
}

#[test]
fn outline_attachments_cannot_cross_boards() {
    let mut doc = document();
    let mut other = doc.boards[0].clone();
    other.id = "other".into();
    other.part_ids = vec!["k8".into()];
    other.outline_ids.clear();
    doc.boards[0].part_ids.retain(|id| id != "k8");
    doc.boards.push(other);
    doc.outline.push(serde_json::from_value(json!({"kind":"polygon","id":"tab","operation":"add","anchorPartId":"k8","points":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10}]})).unwrap());
    doc.boards[0].outline_ids.push("tab".into());
    let result = scene(Session::default().open(doc));
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.message.contains("cannot cross separate PCBs"))
    );
}

#[test]
fn rotated_back_side_rectangle_detaches_with_radius_and_roundtrips_history() {
    let mut doc = document();
    doc.parts[0].pose.at = Vec2 { x: 100.0, y: 100.0 };
    doc.parts[0].pose.rotation = 30.0;
    doc.parts[0].side = Side::Back;
    doc.outline=vec![serde_json::from_value(json!({"kind":"rect","id":"edge","operation":"add","anchorPartId":"k0","center":{"x":15,"y":0},"size":{"x":25,"y":8},"radius":2,"rotation":12})).unwrap()];
    let mut core = Session::default();
    let before = scene(core.open(doc));
    let removed = core.send(fixture::commit(
        "remove",
        "delete",
        0,
        EditOperation::RemoveParts {
            ids: vec!["k0".into()],
        },
    ));
    let CoreReply::Scene {
        document,
        scene: after,
        ..
    } = removed
    else {
        panic!("Expected scene")
    };
    let OutlineFeature::Rect {
        anchor_part_id,
        rotation,
        radius,
        ..
    } = &document.outline[0]
    else {
        panic!("Rectangle should remain editable")
    };
    assert!(anchor_part_id.is_none());
    assert_eq!(*rotation, Some(18.0));
    assert_eq!(*radius, 2.0);
    // Quantization can reorder vertices, so compare containment on a dense grid.
    for x in 70..110 {
        for y in 80..110 {
            assert_eq!(
                query::contains(&before.contours, x as f64 + 0.123, y as f64 + 0.123),
                query::contains(&after.contours, x as f64 + 0.123, y as f64 + 0.123)
            );
        }
    }
    let undone = scene(core.undo());
    assert_eq!(undone.contours, before.contours);
    let redone = scene(core.redo());
    assert_eq!(redone.contours, after.contours);
    let serialized = serde_json::to_string(&document).unwrap();
    let reopened = Session::projected(serde_json::from_str(&serialized).unwrap());
    assert_eq!(reopened.contours, after.contours);
}

#[test]
fn shrinking_matrix_preview_and_commit_detach_outline_at_the_same_position() {
    let mut doc = matrix_document();
    doc.matrices.push(serde_json::from_value(json!({"id":"main","rows":3,"columns":3,"pitch":{"x":19.05,"y":19.05},"origin":{"x":0,"y":0},"definitionId":"switch","partIds":doc.parts.iter().map(|part|part.id.clone()).collect::<Vec<_>>()})).unwrap());
    doc.outline.push(anchored_polygon(
        "tab",
        "matrix/main/r2c2",
        &[(8.0, -4.0), (30.0, -4.0), (30.0, 4.0), (8.0, 4.0)],
        Operation::Add,
    ));
    doc.boards[0].outline_ids.push("tab".into());
    let mut matrix = doc.matrices[0].clone();
    matrix.rows = 2;
    let mut core = Session::default();
    scene(core.open(doc));
    let preview = scene(core.preview(0, fixture::set_matrix(matrix.clone())));
    assert!(query::contains(&preview.contours, 63.0, 38.1));
    let committed = scene(core.commit(0, fixture::set_matrix(matrix)));
    assert_eq!(preview.contours, committed.contours);
}

#[test]
fn connection_endpoints_follow_moves_and_detach_atomically() {
    let mut doc = document();
    doc.parts.truncate(2);
    doc.parts[1].pose.at.x = 80.0;
    doc.parts[0].pose.rotation = 90.0;
    doc.parts[0].side = Side::Back;
    doc.boards[0].part_ids = vec!["k0".into(), "k1".into()];
    if let OutlineFeature::PartEnvelope {
        connections,
        part_ids,
        ..
    } = &mut doc.outline[0]
    {
        *part_ids = doc.boards[0].part_ids.clone();
        *connections=vec![serde_json::from_value(json!({"id":"route","width":4,"points":[{"at":{"x":-5,"y":0},"partId":"k0"},{"at":{"x":0,"y":40}},{"at":{"x":80,"y":40}},{"at":{"x":0,"y":0},"partId":"k1"}]})).unwrap()];
    }
    let mut core = Session::default();
    core.open(doc);
    let moved = scene(core.commit(0, move_key("k0", -20.0, 0.0)));
    assert!(query::contains(&moved.contours, -10.0, 22.5));
    let removed = core.send(fixture::commit(
        "delete",
        "delete",
        1,
        EditOperation::RemoveParts {
            ids: vec!["k0".into()],
        },
    ));
    let CoreReply::Scene {
        document,
        scene: after,
        ..
    } = removed
    else {
        panic!("Expected scene")
    };
    let OutlineFeature::PartEnvelope { connections, .. } = &document.outline[0] else {
        panic!("Expected envelope")
    };
    let point = &connections[0].points[0];
    assert!(point.part_id.is_none());
    assert!((point.at.x + 20.0).abs() < 1e-8);
    assert!((point.at.y - 5.0).abs() < 1e-8);
    assert!(query::contains(&after.contours, -10.0, 22.5));
    let undone = scene(core.undo());
    assert_eq!(undone.contours, moved.contours);
}

#[test]
fn drawing_addition_closes_recess_without_creating_an_incidental_hole() {
    let mut doc = document();
    doc.outline = serde_json::from_value(json!([
        {"id":"perimeter","kind":"polygon","operation":"add","points":[
            {"x":0,"y":0},{"x":40,"y":0},{"x":40,"y":40},{"x":30,"y":40},
            {"x":30,"y":20},{"x":20,"y":20},{"x":20,"y":40},{"x":0,"y":40}]},
        {"id":"authored","kind":"rect","operation":"subtract","center":{"x":10,"y":10},"size":{"x":4,"y":4},"radius":0},
        {"id":"addition","kind":"polygon","operation":"add","points":[
            {"x":19,"y":39},{"x":31,"y":39},{"x":31,"y":45},{"x":19,"y":45}]},
        {"id":"overlap","kind":"polygon","operation":"add","points":[
            {"x":8,"y":8},{"x":12,"y":8},{"x":12,"y":12},{"x":8,"y":12}]}
    ])).unwrap();
    doc.boards[0].outline_ids = vec![
        "perimeter".into(),
        "authored".into(),
        "addition".into(),
        "overlap".into(),
    ];
    let legacy = scene(Session::default().send(CoreRequest::Open {
        id: "legacy".into(),
        document: doc.clone(),
    }));
    assert!(
        !query::contains(&legacy.contours, 25.0, 30.0),
        "Legacy script composition retains its explicit union topology"
    );
    // Exercise the fixed-copy ownership used by Draw addition; leave legacy
    // unowned script composition unchanged.
    let features = std::mem::take(&mut doc.outline);
    doc.boards[0].outline_ids.clear();
    doc.board_outlines = vec![BoardOutline {
        board_id: "board".into(),
        active_version_id: Some("fixed".into()),
        generated_last_valid: None,
        retired_version_ids: vec![],
        versions: vec![OutlineVersion {
            id: "fixed".into(),
            name: "Edited outline".into(),
            source: OutlineProvenance {
                revision: 0,
                version_id: None,
            },
            geometry: OutlineSnapshot {
                features,
                settings: OutlineSettings::default(),
                expected_regions: 1,
                bridges: vec![],
                protected_gaps: vec![],
            },
            ownership: OutlineOwnership::Fixed,
        }],
    }];
    let result = scene(Session::default().send(CoreRequest::Open {
        id: "addition".into(),
        document: doc,
    }));
    assert!(
        query::contains(&result.contours, 25.0, 30.0),
        "Addition must fill a newly enclosed recess, not turn it into a cutout"
    );
    assert!(
        !query::contains(&result.contours, 10.0, 10.0),
        "Existing authored cutouts must remain"
    );
    assert_eq!(result.contours.iter().filter(|c| c.hole).count(), 1);
}
