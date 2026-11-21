use super::*;
use boardstudio_application::{SessionEpoch, SnapshotToken};
use boardstudio_model::model::{
    Board, BoardContours, Contour, FindingMarker, KeycapResolution, Part, Pose2, Readiness,
    SceneDelta, Side, Transform, Vec2,
};
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_shared::test_mount::MountedFixture;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

fn source(revision: u64) -> KeycapsFitSource {
    KeycapsFitSource {
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "document".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        token: SnapshotToken(revision),
        revision,
        case_preview_current: false,
    }
}

#[test]
fn same_source_retry_and_failure_keep_retained_result_stale() {
    let mut accepted = KeycapsFitState::begin(source(1), None);
    accepted.finish(Ok(KeycapResolution {
        revision: 1,
        specs: vec![],
        findings: vec![],
    }));
    assert!(accepted.is_current());

    let mut retrying = KeycapsFitState::begin(source(1), Some(&accepted));
    assert!(
        !retrying.is_current(),
        "same-source retry is pending, so prior result is stale"
    );
    retrying.finish(Err("worker unavailable".into()));
    assert!(
        !retrying.is_current(),
        "failed retry cannot make retained result current"
    );
}

#[test]
fn layout_fit_basis_keeps_react_padding_and_source_surface_aspect() {
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let scene = SceneDelta {
        module_scenes: vec![],
        revision: document.revision,
        transaction_id: "accepted".into(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: vec![],
        contours: vec![],
        board_contours: vec![BoardContours {
            board_id: "board".into(),
            contours: vec![Contour {
                points: vec![Vec2 { x: -5.0, y: -10.0 }, Vec2 { x: 5.0, y: 10.0 }],
                hole: false,
            }],
        }],
        board_readiness: vec![],
        board_outline_scenes: vec![],
        finding_markers: vec![],
        findings: vec![],
        readiness: Readiness {
            layout: true,
            outline: true,
            pcb: true,
            case_ready: false,
        },
    };

    assert_eq!(
        layout_canvas_bounds(&document, &scene, "board"),
        Some((-17.0, 17.0, -22.0, 22.0))
    );
    let basis = layout_camera_basis(&document, &scene, "board", (725.0, 455.0))
        .expect("active source canvas has geometry");
    assert!((basis.bounds.2 + 22.0).abs() < 0.001);
    assert!((basis.bounds.3 - 22.0).abs() < 0.001);
    assert!((basis.bounds.1 - basis.bounds.0 - 44.0 * 725.0 / 455.0).abs() < 0.001);
    assert_eq!(basis.surface, (725.0, 455.0));
}

/// Ticket 18: the Layout view basis follows the content both ways, so the
/// view grows and shrinks with the spacing after commits. Matrix cells 5
/// mm further out on each side widen the basis by exactly 10 mm; the
/// member-less cells here keep the bounds on the keys alone.
#[test]
fn layout_bounds_follow_spacing_widening_and_narrowing() {
    use boardstudio_model::model::{MatrixScene, MatrixSceneCell};
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    for id in ["left", "right"] {
        document.matrices.push(
            serde_json::from_value(serde_json::json!({
                "id": id, "rows": 1, "columns": 1,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 0.0, "y": 0.0},
                "definitionId": "switch", "partIds": [], "boardId": "board",
                "cells": [{"row": 0, "column": 0, "enabled": true, "assemblies": []}]
            }))
            .unwrap(),
        );
    }
    let scene_with = |half_gap: f64| SceneDelta {
        module_scenes: vec![],
        revision: 0,
        transaction_id: "accepted".into(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: ["left", "right"]
            .into_iter()
            .map(|matrix_id| {
                let x = if matrix_id == "left" {
                    -half_gap
                } else {
                    half_gap
                };
                MatrixScene {
                    matrix_id: matrix_id.into(),
                    cells: vec![MatrixSceneCell {
                        row: 0,
                        column: 0,
                        enabled: true,
                        member_id: None,
                        pose: Pose2 {
                            at: Vec2 { x, y: 0.0 },
                            rotation: 0.0,
                        },
                        shift_frame: Default::default(),
                        offset_frame: Default::default(),
                        row_shift: Vec2 { x: 0.0, y: 0.0 },
                        key_shift: Vec2 { x: 0.0, y: 0.0 },
                        row_bend: 0.0,
                        key_bend: 0.0,
                        cumulative_shift: Vec2 { x: 0.0, y: 0.0 },
                        cumulative_bend: 0.0,
                    }],
                    columns: vec![],
                    companions: vec![],
                }
            })
            .collect(),
        contours: vec![],
        board_contours: vec![],
        board_readiness: vec![],
        board_outline_scenes: vec![],
        finding_markers: vec![],
        findings: vec![],
        readiness: Readiness {
            layout: true,
            outline: true,
            pcb: true,
            case_ready: false,
        },
    };
    let narrow = layout_canvas_bounds(&document, &scene_with(30.0), "board")
        .expect("narrow spacing has bounds");
    let wide = layout_canvas_bounds(&document, &scene_with(35.0), "board")
        .expect("wide spacing has bounds");
    // Both halves move 5 mm out: the basis grows by exactly 10 mm,
    // symmetrically, with identical height.
    assert!((wide.1 - wide.0 - (narrow.1 - narrow.0) - 10.0).abs() < 1e-9);
    assert!((wide.0 - (narrow.0 - 5.0)).abs() < 1e-9);
    assert!((wide.1 - (narrow.1 + 5.0)).abs() < 1e-9);
    assert!((wide.2 - narrow.2).abs() < 1e-9);
    assert!((wide.3 - narrow.3).abs() < 1e-9);
    // The camera basis carries the same growth, so the refit view shows
    // the widened content instead of cropping it.
    let narrow_basis = layout_camera_basis(&document, &scene_with(30.0), "board", (725.0, 455.0))
        .expect("narrow basis exists");
    let wide_basis = layout_camera_basis(&document, &scene_with(35.0), "board", (725.0, 455.0))
        .expect("wide basis exists");
    assert!(
        (wide_basis.bounds.1
            - wide_basis.bounds.0
            - (narrow_basis.bounds.1 - narrow_basis.bounds.0)
            - 10.0)
            .abs()
            < 1e-6
    );
}

#[test]
fn findings_are_deduplicated_severity_sorted_and_grouped_like_react() {
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let warning = Finding {
        id: "board:board-1:feature:clearance:near-key".into(),
        severity: Severity::Warning,
        scope: FindingScope::Layout,
        message: "Keycaps are close".into(),
        target_ids: vec!["board".into()],
    };
    let duplicate = Finding {
        id: "other-board:board-1:feature:clearance:near-key".into(),
        ..warning.clone()
    };
    let error = Finding {
        id: "worker:error".into(),
        severity: Severity::Error,
        scope: FindingScope::Layout,
        message: "Worker failed".into(),
        target_ids: vec![],
    };
    let groups = grouped_findings(&[warning, duplicate, error], &document);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].label, "Layout review");
    assert_eq!(groups[0].findings.len(), 1);
    assert_eq!(groups[0].findings[0].severity, Severity::Error);
    assert_eq!(groups[1].label, "Left PCB");
    assert_eq!(
        groups[1].findings.len(),
        1,
        "same feature/message/geometry collapses"
    );
}

#[test]
fn actionable_board_target_has_react_label_and_targetless_finding_has_no_action() {
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let finding = Finding {
        id: "board:board:invalid-settings".into(),
        severity: Severity::Error,
        scope: FindingScope::Layout,
        message: "Invalid board setting".into(),
        target_ids: vec!["board".into()],
    };
    assert_eq!(
        finding_action_label(&finding, &document, "board", &[]),
        Some("Select affected geometry")
    );
    assert_eq!(
        finding_action_label(&finding, &document, "other-board", &[]),
        None
    );
    let targetless = Finding {
        target_ids: vec![],
        ..finding
    };
    assert_eq!(
        finding_action_label(&targetless, &document, "board", &[]),
        None
    );
}

#[test]
fn current_case_stack_target_precedes_companion_part_target() {
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/keys/diode-1".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.push(Part {
        id: "matrix/keys/diode-1".into(),
        definition_id: "diode".into(),
        reference: "D1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
        keycap: None,
        outline: None,
    });
    let finding = Finding {
        id: "case:collision".into(),
        severity: Severity::Warning,
        scope: FindingScope::Case,
        message: "Mechanical collision".into(),
        target_ids: vec!["matrix/keys/diode-1".into(), "case:plate".into()],
    };
    let layer_ids = vec!["case:plate".to_owned()];
    assert_eq!(
        finding_navigation_target_with_layers(&finding, &document, "board", &layer_ids),
        Some(FindingNavigationTarget::MechanicalLayer {
            board_id: "board".into(),
            layer_id: "case:plate".into(),
        })
    );
    assert!(matches!(
        finding_navigation_target(&finding, &document),
        Some(FindingNavigationTarget::Part { .. })
    ));
}

#[test]
fn production_navigation_admission_rejects_stale_results_and_removed_targets() {
    let mut document = ProjectDoc::empty("document", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let finding = Finding {
        id: "board:board:invalid-settings".into(),
        severity: Severity::Warning,
        scope: FindingScope::Layout,
        message: "Board needs review".into(),
        target_ids: vec!["board".into()],
    };
    let current_source = source(1);
    let mut accepted = KeycapsFitState::begin(current_source.clone(), None);
    accepted.finish(Ok(KeycapResolution {
        revision: 1,
        specs: vec![],
        findings: vec![finding.clone()],
    }));
    let request = FindingNavigationRequest {
        source: current_source.clone(),
        finding: finding.clone(),
        target: FindingNavigationTarget::Board {
            board_id: "board".into(),
        },
    };
    let owner = super::super::keycaps_navigation::OwnerIdentity {
        scope: &current_source.scope,
        generation: 4,
    };
    let accepted_source = super::super::keycaps_navigation::AcceptedNavigationSource {
        scope: current_source.scope.clone(),
        session_epoch: current_source.scope.session_epoch,
        token: current_source.token,
        revision: current_source.revision,
        active_board_id: "board".into(),
    };
    let admitted = super::super::keycaps_navigation::admit_accepted_request(
        &request,
        super::super::keycaps_navigation::NavigationAdmission {
            current_workspace: "Keycaps",
            owner,
            live_scope: Some(&current_source.scope),
            live_generation: 4,
            accepted: &accepted_source,
            fit_state: &accepted,
            document: &document,
            live_mechanical_layers: Some(&[]),
        },
        |_| None,
    )
    .expect("the production admission seam accepts current source and target");
    assert_eq!(
        admitted.target, request.target,
        "the production admission seam retains the accepted target"
    );
    assert_eq!(
        admitted.owner.destination,
        super::super::keycaps_navigation::Destination::Layout(
            super::super::objects::TreeContext::Outline {
                board_id: "board".into()
            }
        )
    );

    let pending = KeycapsFitState::begin(source(2), Some(&accepted));
    assert!(
        super::super::keycaps_navigation::admit_accepted_request(
            &request,
            super::super::keycaps_navigation::NavigationAdmission {
                current_workspace: "Keycaps",
                owner,
                live_scope: Some(&current_source.scope),
                live_generation: 4,
                accepted: &accepted_source,
                fit_state: &pending,
                document: &document,
                live_mechanical_layers: Some(&[]),
            },
            |_| None,
        )
        .is_none()
    );
    assert_eq!(
        accepted_navigation_target(&pending, &request, &document, &[]),
        None,
        "an older displayed result cannot navigate after a same-board revision change"
    );

    let deleted = Finding {
        target_ids: vec!["deleted-part".into()],
        ..finding
    };
    let mut old_result = KeycapsFitState::begin(current_source.clone(), None);
    old_result.finish(Ok(KeycapResolution {
        revision: 1,
        specs: vec![],
        findings: vec![deleted.clone()],
    }));
    let removed_request = FindingNavigationRequest {
        source: current_source,
        finding: deleted,
        target: FindingNavigationTarget::Part {
            board_id: "board".into(),
            part_id: "deleted-part".into(),
        },
    };
    assert_eq!(
        accepted_navigation_target(&old_result, &removed_request, &document, &[]),
        None,
        "a retained finding whose target was removed safely declines navigation"
    );
}

#[test]
fn layout_target_bounds_include_non_keycap_matrix_companions() {
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/keys/diode-1".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.push(Part {
        id: "matrix/keys/diode-1".into(),
        definition_id: "diode".into(),
        reference: "D1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
        keycap: None,
        outline: None,
    });
    let scene = SceneDelta {
        module_scenes: vec![],
        revision: document.revision,
        transaction_id: "accepted".into(),
        changed_ids: vec![],
        transforms: vec![Transform {
            id: "matrix/keys/diode-1".into(),
            pose: Pose2 {
                at: Vec2 { x: 40.0, y: 70.0 },
                rotation: 0.0,
            },
        }],
        matrix_scenes: vec![],
        contours: vec![],
        board_contours: vec![],
        board_readiness: vec![],
        board_outline_scenes: vec![],
        finding_markers: vec![FindingMarker {
            finding_id: "overlap:SW1-SW2".into(),
            board_id: "board".into(),
            contours: vec![Contour {
                points: vec![Vec2 { x: -20.0, y: -10.0 }, Vec2 { x: 80.0, y: 110.0 }],
                hole: false,
            }],
        }],
        findings: vec![],
        readiness: Readiness {
            layout: true,
            outline: false,
            pcb: false,
            case_ready: false,
        },
    };
    assert_eq!(
        layout_navigation_bounds(
            &document,
            &scene,
            &FindingNavigationTarget::Part {
                board_id: "board".into(),
                part_id: "matrix/keys/diode-1".into(),
            },
        ),
        Some((28.0, 52.0, 58.0, 82.0))
    );
    let part_target = FindingNavigationTarget::Part {
        board_id: "board".into(),
        part_id: "matrix/keys/diode-1".into(),
    };
    assert_eq!(
        finding_navigation_bounds(&document, &scene, "overlap:SW1-SW2", &part_target),
        Some((-32.0, 92.0, -22.0, 122.0)),
        "the marker contour is the final camera target even when the primary Part has narrower bounds"
    );
    assert_eq!(
        finding_navigation_bounds(&document, &scene, "different-finding", &part_target),
        Some((28.0, 52.0, 58.0, 82.0)),
        "an absent matching marker falls back to target bounds"
    );
    assert_eq!(
        layout_canvas_bounds(&document, &scene, "board"),
        Some((28.0, 52.0, 58.0, 82.0))
    );
    let mut scene = scene;
    let mut duplicate = scene.transforms[0].clone();
    duplicate.pose.at.x = 10_000.0;
    scene.transforms.push(duplicate);
    let mut off_board = document.parts[0].clone();
    off_board.id = "other-board-part".into();
    off_board.pose.at.x = 20_000.0;
    document.parts.push(off_board);
    assert_eq!(
        layout_canvas_bounds(&document, &scene, "board"),
        Some((28.0, 52.0, 58.0, 82.0)),
        "camera fitting uses the first transform and excludes parts outside the board"
    );
    assert_eq!(selected_part_bounds(&document, &scene, "board", &[]), None);
    assert_eq!(
        selected_part_bounds(&document, &scene, "board", &["other-board-part".into()]),
        None
    );
    assert_eq!(
        selected_part_bounds(
            &document,
            &scene,
            "board",
            &["matrix/keys/diode-1".into(), "other-board-part".into()]
        ),
        Some((40.0, 40.0, 70.0, 70.0)),
        "selection fitting keeps board membership and the first displayed transform"
    );
}

#[test]
fn finding_marker_bounds_include_every_contour_of_the_current_board_finding() {
    let mut scene = SceneDelta {
        module_scenes: vec![],
        revision: 11,
        transaction_id: "accepted".into(),
        changed_ids: vec![],
        transforms: vec![],
        matrix_scenes: vec![],
        contours: vec![],
        board_contours: vec![],
        board_readiness: vec![],
        board_outline_scenes: vec![],
        finding_markers: vec![
            FindingMarker {
                finding_id: "overlap:sw1-sw2".into(),
                board_id: "left".into(),
                contours: vec![
                    Contour {
                        points: vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 18.0, y: 0.0 }],
                        hole: false,
                    },
                    Contour {
                        points: vec![Vec2 { x: 42.0, y: 10.0 }, Vec2 { x: 60.0, y: 30.0 }],
                        hole: false,
                    },
                ],
            },
            FindingMarker {
                finding_id: "overlap:sw1-sw2".into(),
                board_id: "right".into(),
                contours: vec![Contour {
                    points: vec![Vec2 {
                        x: -500.0,
                        y: -500.0,
                    }],
                    hole: false,
                }],
            },
            FindingMarker {
                finding_id: "other-finding".into(),
                board_id: "left".into(),
                contours: vec![Contour {
                    points: vec![Vec2 {
                        x: -100.0,
                        y: -100.0,
                    }],
                    hole: false,
                }],
            },
        ],
        findings: vec![],
        readiness: Readiness {
            layout: true,
            outline: false,
            pcb: false,
            case_ready: false,
        },
    };

    assert_eq!(
        finding_marker_bounds(&scene, "overlap:sw1-sw2", "left"),
        Some((-12.0, 72.0, -12.0, 42.0)),
        "the fit includes all same-finding contours and excludes other boards/findings"
    );
    scene.finding_markers[0].contours.clear();
    assert_eq!(
        finding_marker_bounds(&scene, "overlap:sw1-sw2", "left"),
        None,
        "an empty marker leaves target-bounds fallback available"
    );
}

#[cfg(target_arch = "wasm32")]
fn mounted_action() -> Element {
    let mut navigated = use_signal(|| false);
    let accepted_source = source(1);
    let finding = Finding {
        id: "board:board:invalid-settings".into(),
        severity: Severity::Error,
        scope: FindingScope::Layout,
        message: "Invalid board setting".into(),
        target_ids: vec!["board".into()],
    };
    let mut accepted = KeycapsFitState::begin(accepted_source.clone(), None);
    accepted.finish(Ok(KeycapResolution {
        revision: 1,
        specs: vec![],
        findings: vec![finding],
    }));
    let mut document = ProjectDoc::empty("doc", "Project");
    document.boards.push(Board {
        id: "board".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    let on_navigate = EventHandler::new(move |request: FindingNavigationRequest| {
        navigated.set(
            request.target
                == FindingNavigationTarget::Board {
                    board_id: "board".into(),
                },
        );
    });
    rsx! {
        div {
            KeycapsFitInspector {
                document: Rc::new(document),
                state: Some(accepted),
                mechanical_layer_ids: Rc::from([]),
                on_retry: EventHandler::new(|()| {}),
                on_navigate,
            }
            p { id: "fit-navigation-received", "{navigated()}" }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn mounted_finding_action_uses_react_label_and_emits_current_target() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(mounted_action));
    gloo_timers::future::TimeoutFuture::new(50).await;
    let disclosure =
        boardstudio_web_ui_shared::test_role::by_role_name(&root, "button", "Clearance findings 1")
            .expect("findings disclosure");
    assert_eq!(
        disclosure.get_attribute("aria-expanded").as_deref(),
        Some("true")
    );
    let action = boardstudio_web_ui_shared::test_role::by_role_name(
        &root,
        "button",
        "Select affected geometry",
    )
    .expect("current board target has an action");
    action.click();
    gloo_timers::future::TimeoutFuture::new(50).await;
    let received = root
        .query_selector("#fit-navigation-received")
        .unwrap()
        .unwrap()
        .text_content();
    assert_eq!(received.as_deref(), Some("true"));
    disclosure.click();
    gloo_timers::future::TimeoutFuture::new(50).await;
    assert_eq!(
        disclosure.get_attribute("aria-expanded").as_deref(),
        Some("false")
    );
    root.remove();
}
