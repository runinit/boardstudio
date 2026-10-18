use super::*;
use crate::runtime::project_name_test_support as support;
use boardstudio_core::model::{Asset, BoardReference, Pose2, Vec2};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn source_pcb_nested_curve_ownership_survives_actual_wasm_preparation() {
    let reply: serde_json::Value = serde_json::from_str(&boardstudio_core::artifact::request(
        &serde_json::json!({"id":"nested-wasm","kind":"read-board-source",
            "source":r#"(kicad_pcb (general (thickness 1.6))
                (gr_circle (center 10 20) (end 11 20) (layer "Edge.Cuts") (tstamp island-hole))
                (gr_circle (center 10 20) (end 20 20) (layer "Edge.Cuts") (tstamp outer))
                (gr_circle (center 10 20) (end 12 20) (layer "Edge.Cuts") (tstamp island))
                (gr_circle (center 10 20) (end 15 20) (layer "Edge.Cuts") (tstamp moat)))"#,
            "placement":{"at":{"x":0,"y":0},"rotation":0},"flipped":true
        })
        .to_string(),
    ))
    .unwrap();
    assert_eq!(reply["kind"], "read-board-source", "{reply}");
    let board = &reply["result"];
    let contours = board["pcbSource"]["contours"].as_array().unwrap();
    assert_eq!(contours.len(), 4);
    let hole = contours
        .iter()
        .find(|c| c["id"] == "outline/island-hole")
        .unwrap();
    assert_eq!(hole["outerId"], "outline/island");
    assert_eq!(
        hole["primitives"][0]["curveSupport"]["center"],
        serde_json::json!({"x":-10.,"y":-20.})
    );
    let prepared: serde_json::Value = serde_json::from_str(&boardstudio_core::CoreEngine::new().request(
        &serde_json::json!({"id":"prepare-wasm","kind":"prepare-case","ir":{"revision":1,"bodies":[{
            "revision":1,"contours":board["contours"],"body":{
                "id":"pcb","name":"PCB","boardId":"board","kind":"plate","thickness":1.6,
                "clearance":0.,"pcb_source":board["pcbSource"]
            }
        }]}}).to_string(),
    )).unwrap();
    assert_eq!(prepared["kind"], "case-prepared", "{prepared}");
    let body = &prepared["ir"]["bodies"][0];
    assert_eq!(body["body"]["pcb_source"], board["pcbSource"]);
    let regions = body["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 2);
    assert!(
        regions
            .iter()
            .all(|region| region["holes"].as_array().unwrap().len() == 1)
    );
}

#[wasm_bindgen_test]
async fn mechanical_export_refuses_unverified_or_incomplete_pcb_sources_without_nominal_fallback() {
    let valid = r#"(kicad_pcb (general (thickness 1.6))
        (gr_rect (start 0 0) (end 10 10) (layer "Edge.Cuts") (tstamp board)))"#;
    let incomplete = format!(
        r#"{} (gr_curve (pts (xy 0 0) (xy 1 1) (xy 2 2) (xy 3 3)) (layer "Edge.Cuts") (tstamp extra)))"#,
        valid.strip_suffix(')').unwrap()
    ).into_bytes();
    for (bytes, corrupt_cache, expected) in [
        (valid.as_bytes().to_vec(), true, "accepted SHA-256"),
        (vec![0xff], false, "not valid UTF-8"),
        (
            valid.replace("1.6", "1.2").into_bytes(),
            false,
            "does not match",
        ),
        (incomplete, false, "Unsupported PCB Edge.Cuts"),
        (format!(r#"{} (via blind (at 5 5) (size 0.8) (drill 0.4) (layers "F.Cu" "In1.Cu") (uuid blind-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "blind-via"),
        (format!(r#"{} (via micro (at 5 5) (size 0.8) (drill 0.4) (layers "F.Cu" "In1.Cu") (tstamp micro-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "micro-via"),
        (format!(r#"{} (via blind (at 5 5) (size 0.8) (drill 0.4) (layers "In1.Cu" "In2.Cu") (uuid buried-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "buried-via"),
    ] {
        let runtime = support::new_runtime();
        let mut document = in_process_support::board_document();
        let board_id = document.boards[0].id.clone();
        document.boards[0].outline_ids.push("outline".into());
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0., y: 0. },
            size: Vec2 { x: 20., y: 20. },
            radius: 0.,
            rotation: None,
            operation: Operation::Add,
        });
        document.mechanical = Some(
            serde_json::from_value(serde_json::json!({
                "boardId":board_id,"method":"printed","mount":"rigid",
                "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
                "bottomFoamThickness":0.4,"batteryHeight":4.,"bottomThickness":3.,
                "plateToPcb":3.5,"wallThickness":2.,"clearance":0.3,
                "bottomStyle":"shell","middleFrame":false,"openingAllowance":0.25,
                "mounts":[],"closureMounts":[],"profiles":[]
            }))
            .unwrap(),
        );
        let sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        document.assets.push(Asset {
            id: "pcb".into(),
            name: "source.kicad_pcb".into(),
            media_type: "application/vnd.kicad.pcb".into(),
            sha256: sha256.clone(),
            license: None,
            source: None,
        });
        document.board_references.push(BoardReference {
            id: "reference".into(),
            board_id,
            asset_id: "pcb".into(),
            enabled: true,
            pose: Pose2 {
                at: Vec2 { x: 0., y: 0. },
                rotation: 0.,
            },
            elevation: 0.,
            model_assets: Default::default(),
        });
        support::open_document(&runtime, document).await;
        runtime.assets.borrow_mut().insert(
            sha256,
            if corrupt_cache {
                b"corrupt".to_vec()
            } else {
                bytes
            },
        );
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let contours = accepted
            .scene
            .board_contours
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .unwrap()
            .contours
            .clone();
        let assembly = boardstudio_core::mechanical::resolve(&accepted.document, &contours);
        assert!(!assembly.generation_blocked, "{:?}", assembly.diagnostics);
        // Set only the unrelated CAD-ready precondition; failures must happen before CAD starts.
        let cad_scene = Rc::new(CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted.clone(),
            result: CadResult::default(),
            prepared: boardstudio_core::model::PreparedCaseAssemblyIR {
                revision: accepted.document.revision,
                bodies: vec![],
            },
            physical_fingerprint: None,
            mechanical: Some(assembly),
            exact: true,
            contours,
        });
        let core = runtime.core.borrow().clone();
        let error = runtime
            .prepare_physical_step_assembly(
                core.as_ref(),
                &runtime.electrical_preview_executor_epoch().to_string(),
                "refused-source-operand",
                &accepted,
                &scope,
                Rc::new(|| true),
            )
            .await
            .err()
            .expect("invalid source must not become a physical collision operand");
        assert!(error.contains(expected), "expected {expected}: {error}");
        runtime.submit(Event::StartGeneration {
            operation_id: runtime.operation(),
            scope: scope.clone(),
        });
        let job_id = support::take_held_effects(&runtime)
            .into_iter()
            .find_map(|effect| {
                if let Effect::RunGeneration { job_id, .. } = effect {
                    Some(job_id)
                } else {
                    None
                }
            })
            .expect("generation job is admitted");
        support::deliver_completion(
            &runtime,
            Completion::GenerationFinished {
                job_id,
                scope: scope.clone(),
                exact: true,
            },
        );
        support::run_pending(&runtime).await;
        runtime.set_cad_scene_test(Some(cad_scene));
        let outcome = support::observe_next(&runtime);
        runtime.export_mechanical();
        support::run_pending(&runtime).await;
        assert!(
            matches!(outcome.borrow().as_ref(), Some(TerminalOutcome::ExecutorFailed(reason)) if reason.contains(expected)),
            "source failure must settle the export: {:?}",
            outcome.borrow()
        );
        assert!(
            runtime.status().contains(expected),
            "expected {expected}: {}",
            runtime.status()
        );
        assert!(
            runtime
                .in_process_adapters
                .borrow()
                .as_ref()
                .unwrap()
                .take_deliveries()
                .is_empty(),
            "an invalid raw source must not fall back to a nominal PCB export"
        );
    }
}
