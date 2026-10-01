#[cfg(target_arch = "wasm32")]
mod host;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus::launch(app);
    #[cfg(not(target_arch = "wasm32"))]
    println!("P1 is a browser-only feasibility probe. Build the host with dx --web.");
}

#[cfg(target_arch = "wasm32")]
fn app() -> Element {
    let mut report = use_signal(|| "pending".to_owned());
    use_hook(move || {
        spawn(async move {
            let result = run_probe().await;
            report.set(match result {
                Ok(value) => value.to_string(),
                Err(error) => serde_json::json!({"status":"failed","error":error}).to_string(),
            });
        });
    });
    rsx! { main { h1 { "P1 core worker feasibility" } pre { id: "report", "{report}" } } }
}

#[cfg(target_arch = "wasm32")]
async fn run_probe() -> Result<serde_json::Value, String> {
    use boardstudio_core::model::{CoreReply, CoreRequest, ProjectDoc};
    use boardstudio_p1_core::{Action, Identity, ResultPayload};
    use js_sys::Uint8Array;
    let path = web_sys::window()
        .ok_or("window missing")?
        .location()
        .pathname()
        .map_err(|e| format!("location: {e:?}"))?;
    let prefix = if path.starts_with("/boardstudio/") {
        "/boardstudio/"
    } else {
        "/"
    };
    let url = format!("{prefix}worker/worker-entry.js");
    let worker = host::Client::new(&url, 1).map_err(|e| format!("create: {e:?}"))?;
    wait_ready(&worker).await?;
    let first = Identity {
        epoch: 1,
        operation: 1,
    };
    worker.send(
        first,
        Action::Core(Box::new(CoreRequest::Open {
            id: "open-probe".into(),
            document: ProjectDoc::empty("probe-copy", "P1 copied test input"),
        })),
        None,
    )?;
    let (reply, _) = worker.wait(first).await?;
    let ResultPayload::Core(reply) = reply.payload else {
        return Err("expected Open core reply".into());
    };
    let CoreReply::Scene { id, document, .. } = *reply else {
        return Err("expected Open Scene".into());
    };
    if id != "open-probe" || document.id != "probe-copy" || document.revision != 0 {
        return Err("Open identity/revision mismatch".into());
    }
    let second = Identity {
        epoch: 1,
        operation: 2,
    };
    worker.send(
        second,
        Action::Core(Box::new(CoreRequest::Snapshot {
            id: "snapshot-probe".into(),
        })),
        None,
    )?;
    let (reply, _) = worker.wait(second).await?;
    let ResultPayload::Core(reply) = reply.payload else {
        return Err("expected Snapshot core reply".into());
    };
    let CoreReply::Scene {
        id,
        document: snapshot,
        ..
    } = *reply
    else {
        return Err("expected Snapshot Scene".into());
    };
    if id != "snapshot-probe" || snapshot != document {
        return Err("Snapshot changed document".into());
    }
    let mut precision_cases = Vec::new();
    for (index, revision) in [9_007_199_254_740_991_u64, 9_007_199_254_740_993, u64::MAX]
        .into_iter()
        .enumerate()
    {
        let mut expected = ProjectDoc::empty("precision-copy", "P1 public-value oracle");
        expected.revision = revision;
        expected.parameters.insert("integer-boundaries".into(), serde_json::json!({
            "unsigned": [9_007_199_254_740_991_u64, 9_007_199_254_740_993_u64, u64::MAX],
            "signed": [i64::MIN, -9_007_199_254_740_993_i64, i64::MAX],
            "nested": [null, true, {"fraction": 1.25, "whole_float": 1.0, "negative_zero": -0.0, "label": "preserve"}],
        }));
        let opened = Identity {
            epoch: 1,
            operation: 100 + index as u32 * 2,
        };
        worker.send(
            opened,
            Action::Core(Box::new(CoreRequest::Open {
                id: format!("precision-open-{index}"),
                document: expected.clone(),
            })),
            None,
        )?;
        let (reply, _) = worker.wait(opened).await?;
        assert_precision_reply(reply, &format!("precision-open-{index}"), &expected)?;
        let captured = Identity {
            epoch: 1,
            operation: opened.operation + 1,
        };
        worker.send(
            captured,
            Action::Core(Box::new(CoreRequest::Snapshot {
                id: format!("precision-snapshot-{index}"),
            })),
            None,
        )?;
        let (reply, _) = worker.wait(captured).await?;
        assert_precision_reply(reply, &format!("precision-snapshot-{index}"), &expected)?;
        precision_cases.push(revision.to_string());
    }
    let noise = Identity {
        epoch: 1,
        operation: 3,
    };
    worker.send(noise, Action::Noise, None)?;
    worker.wait(noise).await?;
    if worker.rejected() != 3 {
        return Err("invalid/stale/unsolicited replies were accepted".into());
    }
    let echo = Identity {
        epoch: 1,
        operation: 4,
    };
    let bytes = Uint8Array::from(&[3_u8, 1, 4][..]);
    let buffer = bytes.buffer();
    worker.send(echo, Action::Echo, Some(&bytes))?;
    let detached = buffer.byte_length();
    let (_, received) = worker.wait(echo).await?;
    if detached != 0 || received != Some(vec![3, 1, 4]) {
        return Err("transfer did not detach/preserve bytes".into());
    }
    let closed = Identity {
        epoch: 1,
        operation: 5,
    };
    worker.send(
        closed,
        Action::Core(Box::new(CoreRequest::Snapshot {
            id: "close-pending".into(),
        })),
        None,
    )?;
    worker.close();
    if worker.wait(closed).await.is_ok() || worker.pending() != 0 {
        return Err("close did not settle caller".into());
    }
    let crashed = host::Client::new(&url, 2).map_err(|e| format!("create crash: {e:?}"))?;
    wait_ready(&crashed).await?;
    let crash = Identity {
        epoch: 2,
        operation: 1,
    };
    crashed.send(crash, Action::Crash, None)?;
    let error = crashed.wait(crash).await.err().ok_or("crash succeeded")?;
    if !error.contains("worker error") || crashed.pending() != 0 {
        return Err(format!("crash did not settle via error: {error}"));
    }
    let missing = host::Client::new(&format!("{prefix}missing-worker.js"), 3)
        .map_err(|e| format!("create missing: {e:?}"))?;
    let init = Identity {
        epoch: 3,
        operation: 1,
    };
    missing.send(
        init,
        Action::Core(Box::new(CoreRequest::Snapshot {
            id: "init-failure".into(),
        })),
        None,
    )?;
    let error = missing
        .wait(init)
        .await
        .err()
        .ok_or("missing worker succeeded")?;
    if !error.contains("worker error") || missing.pending() != 0 {
        return Err(format!("init did not settle via error: {error}"));
    }
    Ok(
        serde_json::json!({"status":"passed","prefix":prefix,"worker_url":url,"open":"passed","snapshot":"passed","precision_cases":precision_cases,"parameter_values":"preserved","invalid_stale_unsolicited_rejected":worker.rejected(),"sender_bytes_after_transfer":detached,"received_bytes":received,"close":"settled","crash":"settled","init_failure":"settled","serialization":"Rust serde_json text frame; core document encoded/decoded and frame copied across boundary; test buffer copied once to JS and once from JS for assertion"}),
    )
}

#[cfg(target_arch = "wasm32")]
async fn wait_ready(client: &host::Client) -> Result<(), String> {
    for _ in 0..3000 {
        if client.ready() {
            return Ok(());
        }
        gloo_timers::future::TimeoutFuture::new(5).await;
    }
    client.close();
    Err("initialization deadline exceeded".into())
}

#[cfg(target_arch = "wasm32")]
fn assert_precision_reply(
    reply: boardstudio_p1_core::Reply,
    expected_id: &str,
    expected: &boardstudio_core::model::ProjectDoc,
) -> Result<(), String> {
    use boardstudio_core::model::CoreReply;
    use boardstudio_p1_core::ResultPayload;
    let ResultPayload::Core(reply) = reply.payload else {
        return Err("expected precision core reply".into());
    };
    let CoreReply::Scene {
        id,
        document,
        scene,
    } = *reply
    else {
        return Err("expected precision scene".into());
    };
    if id != expected_id || document != *expected || scene.revision != expected.revision {
        return Err("provider request/reply integer or parameter meaning changed".into());
    }
    // Preserve the provider's serialized JSON value representation too, including -0.0 and 1.0.
    if serde_json::to_string(&document).map_err(|e| e.to_string())?
        != serde_json::to_string(expected).map_err(|e| e.to_string())?
    {
        return Err("provider JSON parameter representation changed".into());
    }
    Ok(())
}
