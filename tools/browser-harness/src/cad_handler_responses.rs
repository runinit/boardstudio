//! Collect ordered responses from the page's own flattened worker sessions.
use crate::cad_responses::{self, Resource};
use boardstudio_browser_harness::{Error, Result};
use chromiumoxide::{
    cdp::CustomEvent,
    types::{MethodId, MethodType},
    Page,
};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
struct NetworkEvent {
    session: String,
    method: String,
    params: Value,
}
impl MethodType for NetworkEvent {
    fn method_id() -> MethodId {
        "BoardStudio.NetworkEvent".into()
    }
}
impl CustomEvent for NetworkEvent {}

fn response_url_matches(request_url: &str, response_url: Option<&str>, handoff: bool) -> bool {
    response_url.is_some_and(|url| handoff || url == request_url)
}

pub async fn start(page: &Page) -> Result<tokio::task::JoinHandle<Result<Vec<Resource>>>> {
    let mut events = page
        .event_listener::<NetworkEvent>()
        .await
        .map_err(|e| Error(e.to_string()))?;
    Ok(tokio::spawn(async move {
        let mut resources = Vec::new();
        let mut requests = HashMap::new();
        let mut parents = HashMap::new();
        let mut saw_worker = false;
        while let Some(event) = events.next().await {
            if event.method == "Target.attachedToTarget" {
                let child = event.params["sessionId"]
                    .as_str()
                    .ok_or_else(|| Error("worker attachment missing session".into()))?;
                let target = event.params["targetInfo"]["targetId"]
                    .as_str()
                    .ok_or_else(|| Error("worker attachment missing target ID".into()))?;
                if parents
                    .insert(child.to_owned(), (event.session.clone(), target.to_owned()))
                    .is_some()
                {
                    return Err(Error("duplicate worker session".into()));
                }
                saw_worker = true;
                continue;
            }
            let params = &event.params;
            let id = params["requestId"]
                .as_str()
                .ok_or_else(|| Error("network event missing request ID".into()))?;
            let key = (event.session.clone(), id.to_owned());
            let response;
            let favicon;
            if event.method == "Network.requestWillBeSent" {
                let url = params["request"]["url"]
                    .as_str()
                    .ok_or_else(|| Error("request missing URL".into()))?;
                response = params.get("redirectResponse").filter(|r| !r.is_null());
                let previous = requests.get(&key).or_else(|| {
                    parents.get(&event.session).and_then(|(parent, target)| {
                        (target == id)
                            .then(|| requests.get(&(parent.clone(), id.to_owned())))
                            .flatten()
                    })
                });
                if let Some(redirect) = response {
                    let prior: &(bool, String) = previous
                        .ok_or_else(|| Error("redirect missing request provenance".into()))?;
                    if redirect["url"].as_str() != Some(prior.1.as_str()) {
                        return Err(Error(
                            "redirect URL does not match request provenance".into(),
                        ));
                    }
                }
                favicon = previous.map(|(favicon, _)| *favicon).unwrap_or(false);
                requests.insert(
                    key.clone(),
                    (
                        url.ends_with("/favicon.ico") || (response.is_some() && favicon),
                        url.to_owned(),
                    ),
                );
            } else if event.method == "Network.responseReceived" {
                let handoff = !requests.contains_key(&key)
                    && parents.get(&event.session).is_some_and(|(parent, target)| {
                        target == id && requests.contains_key(&(parent.clone(), id.to_owned()))
                    });
                if handoff {
                    let (parent, _) = &parents[&event.session];
                    requests.insert(
                        key.clone(),
                        requests[&(parent.clone(), id.to_owned())].clone(),
                    );
                }
                let request = requests.get(&key).ok_or_else(|| {
                    Error(format!(
                        "response missing request provenance: session={} id={} url={}",
                        event.session, id, params["response"]["url"]
                    ))
                })?;
                if !response_url_matches(&request.1, params["response"]["url"].as_str(), handoff) {
                    return Err(Error(
                        "response URL does not match request provenance".into(),
                    ));
                }
                favicon = request.0;
                response = Some(&params["response"]);
            } else {
                return Err(Error("unexpected network observation".into()));
            }
            if let Some(response) = response {
                let url = response["url"]
                    .as_str()
                    .ok_or_else(|| Error("response missing URL".into()))?;
                let status = response["status"]
                    .as_i64()
                    .ok_or_else(|| Error("response missing status".into()))?;
                let worker = match response.get("fromServiceWorker") {
                    Some(Value::Bool(value)) => Some(*value),
                    None | Some(Value::Null) => None,
                    _ => return Err(Error("invalid service worker response field".into())),
                };
                if !favicon {
                    let url = if event.method == "Network.responseReceived" {
                        &requests[&key].1
                    } else {
                        url
                    };
                    resources.push(cad_responses::resource(url.into(), status, worker));
                }
            }
        }
        if !saw_worker {
            return Err(Error("no dedicated worker target observed".into()));
        }
        Ok(resources)
    }))
}

#[cfg(test)]
mod tests {
    use super::response_url_matches;

    #[test]
    fn only_handoff_accepts_a_different_response_url() {
        assert!(response_url_matches("entry", Some("final"), true));
        assert!(!response_url_matches("entry", Some("final"), false));
        assert!(!response_url_matches("entry", None, true));
        assert!(response_url_matches("entry", Some("entry"), false));
    }
}
