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
        serde_json::json!({"status":"passed","prefix":prefix,"worker_url":url,"open":"passed","snapshot":"passed","invalid_stale_unsolicited_rejected":worker.rejected(),"sender_bytes_after_transfer":detached,"received_bytes":received,"close":"settled","crash":"settled","init_failure":"settled","serialization":"serde-wasm-bindgen structured frame; core document copied across boundary; test buffer copied once to JS and once from JS for assertion"}),
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
