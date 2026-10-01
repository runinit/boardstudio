use boardstudio_core::CoreEngine;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TRANSACTION: AtomicU64 = AtomicU64::new(1);

pub struct ProbeSession {
    engine: CoreEngine,
    snapshot: Value,
}

impl ProbeSession {
    pub fn open(document_json: &str) -> Result<Self, String> {
        let document: Value =
            serde_json::from_str(document_json).map_err(|error| error.to_string())?;
        let mut engine = CoreEngine::new();
        let reply = engine.request(
            &json!({ "kind": "open", "id": "probe-open", "document": document }).to_string(),
        );
        let snapshot = committed_document(&reply)?;
        Ok(Self { engine, snapshot })
    }

    pub fn part_position(&self, id: &str) -> Option<(f64, f64)> {
        self.snapshot
            .get("parts")?
            .as_array()?
            .iter()
            .find(|part| part.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|part| part.pointer("/pose/at"))
            .and_then(|at| Some((at.get("x")?.as_f64()?, at.get("y")?.as_f64()?)))
    }

    pub fn commit_drag(&mut self, id: &str, delta: (f64, f64)) -> Result<(), String> {
        let (x, y) = self
            .part_position(id)
            .ok_or_else(|| format!("Unknown fixture part {id}"))?;
        let revision = self
            .snapshot
            .get("revision")
            .and_then(Value::as_u64)
            .ok_or_else(|| "Engine snapshot omitted revision".to_owned())?;
        let transaction = NEXT_TRANSACTION.fetch_add(1, Ordering::Relaxed);
        let request = json!({
            "kind": "edit",
            "id": format!("probe-edit-{transaction}"),
            "command": {
                "baseRevision": revision,
                "transactionId": format!("probe-drag-{transaction}"),
                "phase": "commit",
                "targetIds": [id],
                "operation": {
                    "kind": "move-parts",
                    "positions": [{ "id": id, "at": { "x": x + delta.0, "y": y + delta.1 } }]
                }
            }
        });
        self.apply_committed_request(&request.to_string())
    }

    pub fn undo(&mut self) -> Result<(), String> {
        self.apply_committed_request(&json!({ "kind": "undo", "id": "probe-undo" }).to_string())
    }

    pub fn redo(&mut self) -> Result<(), String> {
        self.apply_committed_request(&json!({ "kind": "redo", "id": "probe-redo" }).to_string())
    }

    fn apply_committed_request(&mut self, request: &str) -> Result<(), String> {
        let reply = self.engine.request(request);
        self.snapshot = committed_document(&reply)?;
        Ok(())
    }
}

fn committed_document(reply: &str) -> Result<Value, String> {
    let reply: Value = serde_json::from_str(reply).map_err(|error| error.to_string())?;
    match reply.get("kind").and_then(Value::as_str) {
        Some("scene") => reply
            .get("document")
            .cloned()
            .ok_or_else(|| "Engine scene omitted committed document".to_owned()),
        Some("error") => Err(reply
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("CoreEngine rejected probe request")
            .to_owned()),
        _ => Err("CoreEngine returned a non-committed reply".to_owned()),
    }
}
