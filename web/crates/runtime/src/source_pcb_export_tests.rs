use crate::physical_pcb::{install_source_pcb, verify_source_digest};
use boardstudio_application::CoreEngine;
use boardstudio_application::Event;
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_model::model::CaseAssemblyIR;
#[path = "source_pcb_test_fixture.rs"]
mod fixture;
use boardstudio_web_host::cad_jobs::{
    CadSnapshotIdentity, PreparedCadSnapshot, captured_pcb_source,
};

#[test]
fn source_pcb_nested_curve_ownership_survives_core_preparation() {
    let reply: serde_json::Value = serde_json::from_str(&boardstudio_core::artifact::request(
        &serde_json::json!({"id":"nested-source","kind":"read-board-source",
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
    let ir = serde_json::from_value(serde_json::json!({"revision":1,"bodies":[{
        "revision":1,"contours":board["contours"],"body":{
            "id":"pcb","name":"PCB","boardId":"board","kind":"plate","thickness":1.6,
            "clearance":0.,"pcb_source":board["pcbSource"]
        }
    }]}))
    .unwrap();
    let prepared = serde_json::to_value(CoreEngine::prepare_case(&ir).unwrap()).unwrap();
    let body = &prepared["bodies"][0];
    assert_eq!(body["body"]["pcb_source"], board["pcbSource"]);
    let regions = body["regions"].as_array().unwrap();
    assert_eq!(regions.len(), 2);
    assert!(
        regions
            .iter()
            .all(|region| region["holes"].as_array().unwrap().len() == 1)
    );
}

#[test]
fn mechanical_export_refuses_unverified_or_incomplete_pcb_sources_without_nominal_fallback() {
    for (bytes, corrupt_cache, expected) in fixture::rejected_sources() {
        let runtime = crate::runtime::Runtime::new();
        let document = fixture::document(&sha256_hex(&bytes));
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let bytes = if corrupt_cache {
            b"corrupt".to_vec()
        } else {
            bytes
        };
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let contours = &accepted
            .scene
            .board_contours
            .iter()
            .find(|item| item.board_id == scope.board_id)
            .unwrap()
            .contours;
        let assembly =
            boardstudio_core_mechanical::mechanical::resolve(&accepted.document, contours);
        assert!(!assembly.generation_blocked, "{:?}", assembly.diagnostics);
        let revision = accepted.document.revision;
        let reference = assembly
            .pcb_reference
            .clone()
            .expect("nominal PCB reference");
        let ir = CoreEngine::prepare_case(&CaseAssemblyIR {
            revision,
            bodies: vec![reference],
        })
        .expect("nominal PCB can be prepared before reading its source");
        let mut prepared = PreparedCadSnapshot {
            identity: CadSnapshotIdentity {
                token: accepted.token.0,
                session_epoch: scope.session_epoch.0,
                document_id: scope.document_id.clone(),
                board_id: scope.board_id.clone(),
                instance_id: scope.instance_id.clone(),
                revision,
            },
            prepared: ir,
            mechanical_assembly: Some(assembly),
        };
        let source = captured_pcb_source(&accepted, &scope).unwrap().unwrap();
        let nominal = prepared.prepared.clone();
        let result = verify_source_digest(&source.asset, &bytes)
            .and_then(|()| install_source_pcb(expected, &source, bytes, &mut prepared));
        let error = result
            .expect_err("invalid source must not become a physical collision or export operand");
        assert!(error.contains(expected), "expected {expected}: {error}");
        assert_eq!(
            prepared.prepared, nominal,
            "failed preparation must never publish a partial PCB"
        );
    }
}
