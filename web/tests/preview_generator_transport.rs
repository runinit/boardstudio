#![cfg(target_arch = "wasm32")]

#[path = "../src/preview_generator.rs"]
mod preview_generator;

use preview_generator::PreviewGeneratorClient;
use serde_json::{Value, json};
use std::{future::Future, task::Poll};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

struct WorkerSource(String);

impl WorkerSource {
    fn new(source: &str) -> Self {
        let parts = js_sys::Array::of1(&source.into());
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("text/javascript");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options).unwrap();
        Self(web_sys::Url::create_object_url_with_blob(&blob).unwrap())
    }
}

impl Drop for WorkerSource {
    fn drop(&mut self) {
        web_sys::Url::revoke_object_url(&self.0).unwrap();
    }
}

async fn bounded_reply(client: &PreviewGeneratorClient, request: &Value) -> Result<Value, String> {
    let mut reply = std::pin::pin!(client.generate(7, request));
    let mut timeout = std::pin::pin!(gloo_timers::future::TimeoutFuture::new(3000));
    std::future::poll_fn(|cx| {
        if let Poll::Ready(result) = reply.as_mut().poll(cx) {
            return Poll::Ready(result);
        }
        if timeout.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err("test deadline: worker reply remained pending".into()));
        }
        Poll::Pending
    })
    .await
}

#[wasm_bindgen_test]
async fn actual_packaged_module_worker_accepts_plain_object_and_correlates_reply() {
    let Some(url) = option_env!("BOARDSTUDIO_TEST_PREVIEW_WORKER_URL") else {
        panic!("run scripts/web/test-preview-generator-transport.mjs");
    };
    let worker = WorkerSource::new(&format!("import {};", serde_json::to_string(url).unwrap()));
    let client = PreviewGeneratorClient::new(&worker.0).unwrap();
    let request = json!({
        "kind": "generate-preview-jobs", "worker_generation": 3, "request_id": 7,
        "owner": {"scope": {"docId": "doc", "boardId": "board", "instanceId": "left"},
            "token": "accepted-43", "viewer_instance": 2, "projection_generation": 4},
        "batch": {"accepted_revision": 12, "batch_generation": 9},
        "plan_key": {"snapshot_token": "plan-7", "revision": 12, "job_ids": []},
        "jobs": [], "reserved_nets": [], "next_net_index": 1, "paths": []
    });
    let reply = bounded_reply(&client, &request)
        .await
        .expect("real worker must settle");
    assert_eq!(reply["kind"], "generated-preview-jobs");
    for identity in [
        "request_id",
        "worker_generation",
        "owner",
        "batch",
        "plan_key",
    ] {
        assert_eq!(reply[identity], request[identity], "{identity}");
    }
    assert_eq!(reply["results"], json!([]));
}

#[wasm_bindgen_test]
async fn malformed_response_identity_rejects_pending_request() {
    for id in ["undefined", "7.5", "0", "9007199254740992"] {
        let worker = WorkerSource::new(&format!(
            "self.onmessage = () => self.postMessage({{request_id: {id}}});"
        ));
        let client = PreviewGeneratorClient::new(&worker.0).unwrap();
        let error = bounded_reply(&client, &json!({"request_id": 7}))
            .await
            .expect_err("malformed correlation cannot settle successfully");
        assert!(error.contains("reply identity"), "{id}: {error}");
    }
}

#[wasm_bindgen_test]
async fn obsolete_valid_response_cannot_settle_a_new_request() {
    let worker = WorkerSource::new(
        "self.onmessage = () => { self.postMessage({request_id: 6, value: 'old'}); \
         self.postMessage({request_id: 7, value: 'current'}); };",
    );
    let client = PreviewGeneratorClient::new(&worker.0).unwrap();
    let reply = bounded_reply(&client, &json!({"request_id": 7}))
        .await
        .unwrap();
    assert_eq!(reply["value"], "current");
}
