//! Connected mirrored outline gap tests.
use super::*;
use wasm_bindgen_test::wasm_bindgen_test;

fn gap_world_points(
    document: &ProjectDoc,
    gap: &boardstudio_model::model::OutlineGap,
) -> Vec<Vec2> {
    let resolved = boardstudio_application::resolved_document(document);
    gap.points
        .iter()
        .map(|point| {
            let Some(id) = &point.part_id else {
                return point.at;
            };
            let part = resolved.parts.iter().find(|part| &part.id == id).unwrap();
            let x = if part.side == Side::Back {
                -point.at.x
            } else {
                point.at.x
            };
            let (sin, cos) = part.pose.rotation.to_radians().sin_cos();
            Vec2 {
                x: part.pose.at.x + x * cos - point.at.y * sin,
                y: part.pose.at.y + x * sin + point.at.y * cos,
            }
        })
        .collect()
}

fn connected_mirrored_gap_document() -> ProjectDoc {
    use boardstudio_model::model::{CoreReply, CoreRequest};
    use serde_json::json;
    let mut input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../core/tests/fixtures/synthetic-outline.json"
    ))
    .unwrap();
    for key in ["parts", "matrices", "layouts", "outline"] {
        input[key] = json!([]);
    }
    input["boards"][0]["partIds"] = json!([]);
    input["boards"][0]["outlineIds"] = json!([]);
    let mut engine = boardstudio_core::CoreEngine::new();
    let reply = engine.handle(CoreRequest::Open {
        id: "gap-fixture".into(),
        document: serde_json::from_value(input).unwrap(),
    });
    let CoreReply::Scene { document, .. } = reply else {
        panic!("the synthetic gap board opens: {reply:?}");
    };
    let apply = |engine: &mut boardstudio_core::CoreEngine, document: &ProjectDoc, operation| {
        let reply = engine.request(&json!({"id":"gap-fixture", "kind":"edit", "command":{
            "baseRevision":document.revision,"transactionId":format!("gap-{}",document.revision),
            "phase":"commit","targetIds":[],"operation":operation
        }}).to_string());
        let CoreReply::Scene {
            document, scene, ..
        } = serde_json::from_str(&reply).unwrap()
        else {
            panic!("fixture edit is accepted: {reply}");
        };
        (*document, scene)
    };
    let (document, _) = apply(
        &mut engine,
        &document,
        json!({"kind":"create-mirrored-pair",
            "matrix":{"id":"left-matrix","rows":1,"columns":2,"pitch":{"x":24.,"y":19.},
                "origin":{"x":-36.,"y":0.},"columnLayout": [{"offset": {"x": 0.0, "y": 0.0}}, {"offset": {"x": 0.0, "y": 10.0}}],
        "definitionId":"synthetic/switch","boardId":"main","partIds":[]},
            "left":{"id":"left-layout","name":"Left","boardId":"main","matrixIds":["left-matrix"],"partIds":[]},
            "right":{"id":"right-layout","name":"Right","boardId":"main","matrixIds":["right-matrix"],"partIds":[],
                "mirrorLink":{"sourceId":"left-layout","axisX":0.}}
        }),
    );
    let (document, _) = apply(
        &mut engine,
        &document,
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"main"},"change":{"create-generated":{
                "margin":1.,"part_ids":document.boards[0].part_ids,
                "settings":{"corners":"fillet","size":2.,"bridgeWidth":10.,
                    "repair":{"enabled":true,"maximumGapSpan":20.,"minimumConnectionWidth":2.,"edgeClearance":0.}}
            }},"seed":141,"reviewed":null
        }}),
    );
    let (mut document, mut scene) = apply(
        &mut engine,
        &document,
        json!({"kind":"apply-outline-edit","edit":{
            "target":{"kind":"board","boardId":"main"},"change":{"add-connection":{"connection":{
                "id":"cross-split","width":6.,"points":[{"at":{"x":-4.,"y":20.}},{"at":{"x":4.,"y":20.}}]
            }}},"seed":144,"reviewed":null
        }}),
    );
    for (index, side) in [-1., 1.].into_iter().enumerate() {
        let gap = scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps
            .iter()
            .find(|gap| {
                gap_world_points(&document, gap)
                    .iter()
                    .all(|point| point.x * side > 20.)
            })
            .expect("each half has a distinct recess")
            .clone();
        (document, scene) = apply(
            &mut engine,
            &document,
            json!({"kind":"apply-outline-edit","edit":{
                "target":{"kind":"board","boardId":"main"},"change":{"set-protected-gap":{
                    "gap_id":gap.id,"points":gap.points
                }},"seed":150 + index,"reviewed":null
            }}),
        );
    }
    document
}

#[wasm_bindgen_test]
async fn connected_mirrored_outline_gap_controls_edit_distinct_regions() {
    for side in [-1., 1.] {
        let fixture = connected_mirrored_gap_document();
        let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
        boardstudio_web_test_runtime::project_name_test_support::open_document(&runtime, fixture)
            .await;
        let probe = InspectorProbe {
            scope: runtime.scope().unwrap(),
            runtime,
            connection_action: Rc::default(),
            selected_context: Rc::default(),
        };
        let root = new_root("connected-gap-inspector");
        let dom = VirtualDom::new(mounted_outline_inspector_host);
        dom.provide_root_context(probe.clone());
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_until("the connected outline Inspector mounts", || {
            probe.connection_action.borrow().is_some()
        })
        .await;
        let before = probe.runtime.model().accepted.unwrap();
        assert_eq!(before.document.boards.len(), 1);
        assert_eq!(before.document.boards[0].outline_ids.len(), 1);
        let contours = |snapshot: &boardstudio_application::AcceptedSnapshot| {
            snapshot
                .scene
                .board_contours
                .iter()
                .find(|board| board.board_id == "main")
                .unwrap()
                .contours
                .clone()
        };
        assert_eq!(
            contours(&before)
                .iter()
                .filter(|contour| !contour.hole)
                .count(),
            1
        );
        let gaps = &before
            .scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps;
        let selected = gaps
            .iter()
            .position(|gap| {
                gap.protected
                    && gap_world_points(&before.document, gap)
                        .iter()
                        .all(|point| point.x * side > 20.)
            })
            .unwrap();
        let other = gaps
            .iter()
            .find(|gap| {
                gap.protected
                    && gap_world_points(&before.document, gap)
                        .iter()
                        .all(|point| point.x * side < -20.)
            })
            .unwrap();
        let selected_id = gaps[selected].id.clone();
        let other_id = other.id.clone();
        assert_ne!(selected_id, other_id);
        assert_eq!(gaps[selected].feature_id, other.feature_id);
        assert!(
            gaps[selected]
                .protected_ids
                .iter()
                .all(|id| !other.protected_ids.contains(id))
        );
        let controls = root.query_selector_all("[role=checkbox]").unwrap();
        let gap_count = (0..controls.length())
            .filter_map(|index| controls.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .filter(|control| {
                control
                    .get_attribute("aria-label")
                    .is_some_and(|name| name.starts_with("Keep gap "))
            })
            .count();
        assert_eq!(gap_count, gaps.len());
        let checkbox =
            by_role_name(&root, "checkbox", &format!("Keep gap {}", selected + 1)).unwrap();
        assert_eq!(
            checkbox.get_attribute("aria-checked").as_deref(),
            Some("true")
        );
        checkbox.click();
        accept(&probe).await;
        wait_until("the gap edit lands through the production resolver", || {
            probe
                .runtime
                .model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .revision
                > before.document.revision
        })
        .await;
        let after = probe.runtime.model().accepted.unwrap();
        let after_gaps = &after
            .scene
            .board_outline_scenes
            .iter()
            .find(|board| board.board_id == "main")
            .unwrap()
            .gaps;
        assert!(
            !after_gaps
                .iter()
                .find(|gap| gap.id == selected_id)
                .unwrap()
                .protected
        );
        assert!(
            after_gaps
                .iter()
                .find(|gap| gap.id == other_id)
                .unwrap()
                .protected
        );
        assert_eq!(
            contours(&after)
                .iter()
                .filter(|contour| !contour.hole)
                .count(),
            1
        );
        let half_points = |snapshot: &boardstudio_application::AcceptedSnapshot, side: f64| {
            let mut points: Vec<_> = contours(snapshot)
                .into_iter()
                .flat_map(|contour| contour.points)
                .filter(|point| point.x * side > 20.)
                .collect();
            points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
            points
        };
        assert_ne!(half_points(&before, side), half_points(&after, side));
        assert_eq!(half_points(&before, -side), half_points(&after, -side));
        web_sys::console::log_1(&serde_json::json!({"ticket":141,"boardId":"main", "side":side,
            "selectedGapId":selected_id,"otherGapId":other_id,"featureId":gaps[selected].feature_id,
            "selectedWorldPoints":gap_world_points(&before.document, &gaps[selected]),
            "otherWorldPoints":gap_world_points(&before.document, other),
            "beforeGaps":gaps,"afterGaps":after_gaps,"beforeContours":contours(&before),"afterContours":contours(&after)
        }).to_string().into());
        root.remove();
    }
}
