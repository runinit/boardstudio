#[cfg(target_arch = "wasm32")]
mod host;
#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
fn main() {
    #[cfg(target_arch = "wasm32")]
    dioxus::launch(app);
    #[cfg(not(target_arch = "wasm32"))]
    println!("Isolated P1-CAD browser feasibility probe; use dx build --web.");
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
        })
    });
    rsx! { main { h1 { "P1 CAD package feasibility" } pre { id: "report", "{report}" } } }
}
#[cfg(target_arch = "wasm32")]
async fn run_probe() -> Result<serde_json::Value, String> {
    use boardstudio_p1_cad::{Action, Identity, Payload};
    use js_sys::Uint8Array;
    let path = web_sys::window()
        .ok_or("window missing")?
        .location()
        .pathname()
        .map_err(|e| format!("{e:?}"))?;
    let prefix = if path.starts_with("/boardstudio/") {
        "/boardstudio/"
    } else {
        "/"
    };
    let url = format!("{prefix}worker/worker-entry.js");
    let prepared = include_str!("../fixtures/prepared.json");
    let id = |epoch, operation| Identity { epoch, operation };
    let preview = host::Client::new(&url, 1).map_err(|e| format!("create: {e:?}"))?;
    wait_ready(&preview).await?;
    // A completed older result must populate the cache before stale display rejection.
    preview.send(
        id(1, 1),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    preview.send(
        id(1, 2),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    let (first, buffers, current) = preview.wait(id(1, 1)).await?;
    let Payload::Preview { revision: 7, delta } = first.payload else {
        return Err("first full preview failed".into());
    };
    if current
        || delta.base != 0
        || delta.next != 1
        || delta.changed.len() != 1
        || delta.ids != ["case"]
        || buffers.len() != 2
    {
        return Err("raced full preview identity mismatch".into());
    }
    if delta.changed[0].key != include_str!("../fixtures/key.txt") {
        return Err("public bodyKey contract changed".into());
    }
    let preview_geometry = mesh(&buffers[0], &buffers[1])?;
    let (second, buffers, current) = preview.wait(id(1, 2)).await?;
    let Payload::Preview { revision: 7, delta } = second.payload else {
        return Err("second delta preview failed".into());
    };
    if !current
        || delta.base != 1
        || delta.next != 2
        || !delta.changed.is_empty()
        || !buffers.is_empty()
        || preview.cache_base() != 2
    {
        return Err("reuse delta mismatch".into());
    }
    if preview.body_key("case").as_deref() != Some(include_str!("../fixtures/key.txt")) {
        return Err("host cache key missing".into());
    }
    let cached = preview
        .mesh("case")
        .ok_or("obsolete reply lost cached geometry")?;
    if mesh(&cached.0, &cached.1)? != preview_geometry {
        return Err("raced cache changed geometry".into());
    }
    preview.send(id(1, 3), Action::Status, None)?;
    let (status, _, _) = preview.wait(id(1, 3)).await?;
    let Payload::Status {
        detached: true,
        transferred_bytes,
    } = status.payload
    else {
        return Err("preview transfer not detached".into());
    };
    if transferred_bytes == 0 {
        return Err("preview transfer missing".into());
    }
    // Separate committed executor has no preview-cache dependency.
    let export = host::Client::new(&url, 2).map_err(|e| format!("create export: {e:?}"))?;
    wait_ready(&export).await?;
    export.send(
        id(2, 1),
        Action::Export {
            prepared: prepared.into(),
        },
        None,
    )?;
    let (reply, output, _) = export.wait(id(2, 1)).await?;
    if !matches!(reply.payload, Payload::Export { revision: 7 }) || output.len() != 3 {
        return Err("committed export failed".into());
    }
    if !output[0].starts_with(b"ISO-10303-21;") {
        return Err("missing STEP bytes".into());
    }
    let export_geometry = mesh(&output[1], &output[2])?;
    let bytes = Uint8Array::from(output[0].as_slice());
    let transferred = bytes.buffer();
    export.send(id(2, 2), Action::ReadStep, Some(&bytes))?;
    let detached = transferred.byte_length();
    let (reply, _, _) = export.wait(id(2, 2)).await?;
    let Payload::Model { min, max } = reply.payload else {
        return Err("STEP reopening failed".into());
    };
    bounds(min, max)?;
    if detached != 0 {
        return Err("STEP input transfer did not detach".into());
    }
    export.send(id(2, 3), Action::Status, None)?;
    let (reply, _, _) = export.wait(id(2, 3)).await?;
    let Payload::Status {
        detached: true,
        transferred_bytes: export_bytes,
    } = reply.payload
    else {
        return Err("export transfer not detached".into());
    };
    if export_bytes < output[0].len() as u64 {
        return Err("export transfer bytes missing".into());
    }
    // Cooperative active and queued cancellation cannot advance the completed index.
    let cancel = host::Client::new(&url, 3).map_err(|e| format!("create cancel: {e:?}"))?;
    wait_ready(&cancel).await?;
    cancel.send(
        id(3, 1),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    cancel.send(
        id(3, 2),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    cancel.cancel(id(3, 1))?;
    cancel.cancel(id(3, 2))?;
    for op in [1, 2] {
        let (reply, buffers, _) = cancel.wait(id(3, op)).await?;
        if !matches!(reply.payload, Payload::Cancelled) || !buffers.is_empty() {
            return Err(format!("cancelled preview {op} committed"));
        }
    }
    if cancel.cache_base() != 0 || cancel.mesh("case").is_some() {
        return Err("cancelled preview changed completed cache".into());
    }
    cancel.send(
        id(3, 3),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    let (reply, _, _) = cancel.wait(id(3, 3)).await?;
    if !matches!(reply.payload,Payload::Preview{delta,..} if delta.base==0 && delta.changed.len()==1)
    {
        return Err("fresh preview reused cancelled completed index".into());
    }
    // A failed job settles its caller, leaves completed cache unchanged, and permits retry.
    cancel.send(
        id(3, 4),
        Action::Preview {
            prepared: "{\"revision\":7,\"bodies\":[]}".into(),
        },
        None,
    )?;
    let (reply, _, _) = cancel.wait(id(3, 4)).await?;
    if !matches!(reply.payload, Payload::Error(_)) || cancel.cache_base() != 1 {
        return Err("failed preview corrupted cache".into());
    }
    cancel.send(
        id(3, 5),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    let (reply, _, _) = cancel.wait(id(3, 5)).await?;
    if !matches!(reply.payload,Payload::Preview{delta,..} if delta.base==1 && delta.changed.is_empty())
    {
        return Err("failed preview poisoned retry".into());
    }
    cancel.send(
        id(3, 6),
        Action::Preview {
            prepared: prepared.into(),
        },
        None,
    )?;
    cancel.close();
    if cancel.wait(id(3, 6)).await.is_ok() || cancel.pending() != 0 {
        return Err("close did not settle".into());
    }
    let crash = host::Client::new(&url, 4).map_err(|e| format!("create crash: {e:?}"))?;
    wait_ready(&crash).await?;
    crash.send(id(4, 1), Action::Crash, None)?;
    let failure = crash.wait(id(4, 1)).await.err().ok_or("crash succeeded")?;
    if !failure.contains("worker error") || crash.pending() != 0 {
        return Err(format!("crash not settled via error: {failure}"));
    }
    let missing = host::Client::new(&format!("{prefix}missing-worker.js"), 5)
        .map_err(|e| format!("create missing: {e:?}"))?;
    missing.send(id(5, 1), Action::Status, None)?;
    let failure = missing
        .wait(id(5, 1))
        .await
        .err()
        .ok_or("missing worker succeeded")?;
    if !failure.contains("worker error") || missing.pending() != 0 {
        return Err(format!("init failure not settled: {failure}"));
    }
    let step_hex = hex(&output[0]);
    Ok(
        serde_json::json!({"status":"passed","prefix":prefix,"worker_url":url,"cad_js_url":format!("{prefix}worker/cad/boardstudio_cadrum_wasm.js"),"cad_wasm_url":format!("{prefix}worker/cad/boardstudio_cadrum_wasm_bg.wasm"),"revision":7,"preview":preview_geometry,"export":export_geometry,"preview_mesh":{"positions_hex":hex(&cached.0),"normals_hex":hex(&cached.1)},"export_mesh":{"positions_hex":hex(&output[1]),"normals_hex":hex(&output[2])},"cache":"full then empty delta; obsolete reply cached before display rejection","cancellation":"active and queued settled; completed cache unchanged","failure_retry":"passed","close":"settled","crash":"settled","init_failure":"settled","rejected":preview.rejected(),"preview_transfer_bytes":transferred_bytes,"export_transfer_bytes":export_bytes,"sender_step_bytes_after_transfer":detached,"step_hex":step_hex}),
    )
}
#[cfg(target_arch = "wasm32")]
async fn wait_ready(client: &host::Client) -> Result<(), String> {
    for _ in 0..6000 {
        if client.ready() {
            return Ok(());
        }
        gloo_timers::future::TimeoutFuture::new(5).await;
    }
    client.close();
    Err("initialization deadline exceeded".into())
}
#[cfg(target_arch = "wasm32")]
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(target_arch = "wasm32")]
fn bounds(min: [f64; 3], max: [f64; 3]) -> Result<(), String> {
    if min
        .iter()
        .zip([-0.5, -0.5, 0.0])
        .chain(max.iter().zip([20.5, 20.5, 2.0]))
        .any(|(a, b)| (a - b).abs() > 0.001)
    {
        return Err(format!("wrong bounds: {min:?} {max:?}"));
    }
    Ok(())
}
#[cfg(target_arch = "wasm32")]
fn mesh(positions: &[u8], normals: &[u8]) -> Result<serde_json::Value, String> {
    let floats = |bytes: &[u8]| -> Result<Vec<f64>, String> {
        if !bytes.len().is_multiple_of(4) {
            return Err("invalid mesh byte count".into());
        }
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|n| f32::from_le_bytes(*n) as f64)
            .collect())
    };
    let p = floats(positions)?;
    let n = floats(normals)?;
    if p.is_empty()
        || !p.len().is_multiple_of(9)
        || p.len() != n.len()
        || p.iter().chain(n.iter()).any(|f| !f.is_finite())
    {
        return Err("invalid triangle mesh".into());
    }
    if n.as_chunks::<3>()
        .0
        .iter()
        .any(|v| (v.iter().map(|x| x * x).sum::<f64>().sqrt() - 1.0).abs() > 0.001)
    {
        return Err("non-unit mesh normal".into());
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut volume = 0.0;
    for v in p.as_chunks::<3>().0.iter() {
        for axis in 0..3 {
            min[axis] = min[axis].min(v[axis]);
            max[axis] = max[axis].max(v[axis]);
        }
    }
    for t in p.as_chunks::<9>().0.iter() {
        volume += (t[0] * (t[4] * t[8] - t[5] * t[7]) - t[1] * (t[3] * t[8] - t[5] * t[6])
            + t[2] * (t[3] * t[7] - t[4] * t[6]))
            / 6.0;
    }
    bounds(min, max)?;
    if (volume.abs() - 720.0).abs() > 0.1 {
        return Err(format!("wrong mesh volume: {volume}"));
    }
    Ok(serde_json::json!({"vertices":p.len()/3,"volume":volume.abs(),"min":min,"max":max}))
}
