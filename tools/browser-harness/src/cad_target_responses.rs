//! Observe a page and its dedicated workers on a separate flattened CDP session.
use crate::cad_responses::{self, Resource};
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use boardstudio_browser_harness::{Error, Result};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tokio::sync::oneshot;

pub async fn start(
    address: &str,
    target: &str,
) -> Result<tokio::task::JoinHandle<Result<Vec<Resource>>>> {
    let (mut socket, _) = connect_async(address)
        .await
        .map_err(|e| Error(format!("response CDP connection: {e}")))?;
    let (ready_tx, ready_rx) = oneshot::channel();
    let target = target.to_owned();
    let task = tokio::spawn(async move {
        let mut ready = Some(ready_tx);
        let result = async {
            let mut next_id = 1_u64;
            let mut pending = HashMap::new();
            let mut workers = HashSet::new();
            let mut responses = Vec::new();
            let mut page = String::new();
            send(&mut socket, &mut next_id, &mut pending, "Target.attachToTarget", json!({"targetId":target,"flatten":true}), None, "attach").await?;
            while let Some(message) = socket.next().await {
                let message = message.map_err(|e| Error(format!("response CDP transport: {e}")))?;
                if !message.is_text() { return Err(Error("response CDP ended before page boundary".into())); }
                let value: Value = serde_json::from_str(message.to_text().map_err(|e| Error(e.to_string()))?).map_err(|e| Error(format!("invalid response CDP message: {e}")))?;
                if let Some(id) = value.get("id").and_then(Value::as_u64) {
                    let (kind, session) = pending.remove(&id).ok_or_else(|| Error("unexpected response CDP reply".into()))?;
                    if value.get("error").is_some() || !value.get("result").is_some_and(Value::is_object) { return Err(Error(format!("response CDP {kind} failed: {value}"))); }
                    match kind {
                        "attach" => {
                            page = value["result"]["sessionId"].as_str().ok_or_else(|| Error("missing page session".into()))?.to_owned();
                            send(&mut socket, &mut next_id, &mut pending, "Network.enable", json!({}), Some(&page), "network").await?;
                        }
                        "network" if session.as_deref() == Some(&page) => {
                            send(&mut socket, &mut next_id, &mut pending, "Target.setAutoAttach", json!({"autoAttach":true,"waitForDebuggerOnStart":true,"flatten":true}), Some(&page), "autoattach").await?;
                        }
                        "autoattach" => { if let Some(tx) = ready.take() { let _ = tx.send(Ok(())); } }
                        "network" => {
                            let session = session.ok_or_else(|| Error("missing worker session".into()))?;
                            send(&mut socket, &mut next_id, &mut pending, "Runtime.runIfWaitingForDebugger", json!({}), Some(&session), "resume").await?;
                        }
                        "resume" => {}
                        _ => return Err(Error("unexpected response CDP command".into())),
                    }
                    continue;
                }
                let method = value["method"].as_str().ok_or_else(|| Error("invalid response CDP event".into()))?;
                let session = value["sessionId"].as_str();
                match method {
                    "Target.attachedToTarget" if session == Some(&page) => {
                        let params = &value["params"];
                        if params["targetInfo"]["type"] != "worker" || params["waitingForDebugger"] != true { return Err(Error(format!("unsupported attached target: {params}"))); }
                        let child = params["sessionId"].as_str().ok_or_else(|| Error("missing worker session".into()))?.to_owned();
                        if !workers.insert(child.clone()) { return Err(Error("duplicate worker session".into())); }
                        send(&mut socket, &mut next_id, &mut pending, "Network.enable", json!({}), Some(&child), "network").await?;
                    }
                    "Network.responseReceived" | "Network.requestWillBeSent" if session == Some(&page) || session.is_some_and(|s| workers.contains(s)) => {
                        let response = if method == "Network.responseReceived" {
                            Some(&value["params"]["response"])
                        } else {
                            value["params"].get("redirectResponse")
                        };
                        if let Some(r) = response {
                            let url = r["url"].as_str().ok_or_else(|| Error("response missing URL".into()))?;
                            let status = r["status"].as_i64().ok_or_else(|| Error("response missing status".into()))?;
                            let service_worker = match r.get("fromServiceWorker") { Some(v) => Some(v.as_bool().ok_or_else(|| Error("invalid service worker response field".into()))?), None => None };
                            responses.push(cad_responses::resource(url.to_owned(), status, service_worker));
                        }
                    }
                    "Target.detachedFromTarget" if value["params"]["sessionId"].as_str() == Some(&page) => {
                        if !pending.is_empty() { return Err(Error("response CDP commands incomplete at page boundary".into())); }
                        if workers.is_empty() { return Err(Error("no dedicated worker target observed".into())); }
                        return Ok(responses);
                    }
                    _ => {}
                }
            }
            Err(Error("response CDP stream ended before page boundary".into()))
        }.await;
        if let Some(tx) = ready {
            let _ = tx.send(Err(Error(
                result
                    .as_ref()
                    .err()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "response capture ended before setup".into()),
            )));
        }
        result
    });
    match tokio::time::timeout(std::time::Duration::from_secs(60), ready_rx).await {
        Ok(Ok(Ok(()))) => Ok(task),
        Ok(Ok(Err(error))) => {
            task.abort();
            Err(error)
        }
        Ok(Err(_)) => {
            task.abort();
            Err(Error("response capture setup terminated".into()))
        }
        Err(_) => {
            task.abort();
            Err(Error("response capture setup timed out".into()))
        }
    }
}

async fn send<S>(
    socket: &mut S,
    next_id: &mut u64,
    pending: &mut HashMap<u64, (&'static str, Option<String>)>,
    method: &str,
    params: Value,
    session: Option<&str>,
    kind: &'static str,
) -> Result<()>
where
    S: futures::Sink<Message> + Unpin,
    S::Error: std::fmt::Display,
{
    let id = *next_id;
    *next_id += 1;
    let mut message = json!({"id":id,"method":method,"params":params});
    if let Some(session) = session {
        message["sessionId"] = json!(session);
    }
    pending.insert(id, (kind, session.map(str::to_owned)));
    socket
        .send(Message::Text(message.to_string().into()))
        .await
        .map_err(|e| Error(format!("response CDP send: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejected_or_invalid_protocol_fails_capture_setup() {
        for reply in [
            r#"{"id":1,"error":{"code":-32601,"message":"unsupported"}}"#,
            "not json",
        ] {
            let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = format!("ws://{}", server.local_addr().unwrap());
            let peer = tokio::spawn(async move {
                let (stream, _) = server.accept().await.unwrap();
                let mut socket = async_tungstenite::tokio::accept_async(stream)
                    .await
                    .unwrap();
                socket.next().await.unwrap().unwrap();
                socket.send(Message::Text(reply.into())).await.unwrap();
            });
            let error = start(&address, "target")
                .await
                .err()
                .expect("capture must fail")
                .to_string();
            assert!(
                error.contains(if reply == "not json" {
                    "invalid response CDP message"
                } else {
                    "unsupported"
                }),
                "{error}"
            );
            peer.await.unwrap();
        }
    }
}
