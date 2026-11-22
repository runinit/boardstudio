use crate::runtime::{CadScene, test_hooks};
use boardstudio_application::{Completion, Effect, Event, TerminalOutcome};
use boardstudio_web_host::cad_jobs::{CadResult, prepare_captured_case};
use boardstudio_web_test_runtime::{
    firmware_export_test_support::take_deliveries, project_name_test_support as support,
};
use std::rc::Rc;
use wasm_bindgen_test::wasm_bindgen_test;

#[path = "../../crates/runtime/src/source_pcb_test_fixture.rs"]
mod fixture;

#[wasm_bindgen_test]
async fn invalid_raw_pcb_source_settles_export_without_delivery() {
    for (bytes, corrupt_cache, expected) in fixture::rejected_sources() {
        let runtime = support::new_runtime();
        let sha256 = boardstudio_core::hash::sha256_hex(&bytes);
        support::open_document(&runtime, fixture::document(&sha256)).await;
        test_hooks::cache_asset(
            &runtime,
            sha256,
            if corrupt_cache {
                b"corrupt".to_vec()
            } else {
                bytes
            },
        );
        let accepted = runtime.model().accepted.unwrap();
        let scope = runtime.scope().unwrap();
        let nominal = prepare_captured_case(
            (runtime.core_services().open_scratch)(),
            "export-ready",
            &accepted,
            &scope,
            true,
        )
        .unwrap();
        let cad_scene = Rc::new(CadScene {
            scope: scope.clone(),
            token: accepted.token,
            snapshot: accepted,
            result: CadResult::default(),
            prepared: nominal.prepared,
            physical_fingerprint: None,
            mechanical: nominal.mechanical_assembly,
            exact: true,
            contours: vec![],
        });
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
            take_deliveries(&runtime).is_empty(),
            "an invalid raw source must not fall back to a nominal PCB export"
        );
    }
}
