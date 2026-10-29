use std::collections::HashSet;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::ready;

use async_tungstenite::tungstenite::Message as WsMessage;
use async_tungstenite::{WebSocketStream, tungstenite::protocol::WebSocketConfig};
use futures::stream::Stream;
use futures::task::{Context, Poll};
use futures::{SinkExt, StreamExt};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

use async_tungstenite::tokio::ConnectStream;
use chromiumoxide_cdp::cdp::browser_protocol::target::SessionId;
use chromiumoxide_types::{CallId, EventMessage, Message, MethodCall, MethodId};

use crate::error::CdpError;
use crate::error::Result;

// Retain raw values so translating a field cannot alter unrelated numbers or fields.
struct RawObject<'a>(Vec<(String, &'a RawValue)>);

impl<'de> Deserialize<'de> for RawObject<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = RawObject<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(RawObject(entries))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

fn reject_duplicates(raw: &str) -> serde_json::Result<()> {
    match raw.trim_start().as_bytes().first() {
        Some(b'{') => {
            let object: RawObject<'_> = serde_json::from_str(raw)?;
            let mut keys = HashSet::new();
            for (key, value) in object.0 {
                if !keys.insert(key) {
                    return Err(<serde_json::Error as serde::de::Error>::custom(
                        "duplicate JSON key",
                    ));
                }
                reject_duplicates(value.get())?;
            }
        }
        Some(b'[') => {
            let values: Vec<&RawValue> = serde_json::from_str(raw)?;
            for value in values {
                reject_duplicates(value.get())?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn decode_message<T: EventMessage>(raw: &str) -> serde_json::Result<Message<T>> {
    let Ok(root) = serde_json::from_str::<RawObject<'_>>(raw) else {
        return serde_json::from_str(raw);
    };
    let target = root.0.iter().any(|(key, value)| {
        key == "method"
            && serde_json::from_str::<String>(value.get()).ok().as_deref()
                == Some("Network.requestWillBeSentExtraInfo")
    });
    if !target {
        return serde_json::from_str(raw);
    }
    if root.0.iter().any(|(key, _)| key == "id")
        && serde_json::from_str::<chromiumoxide_types::Response>(raw).is_ok()
    {
        return serde_json::from_str(raw);
    }
    reject_duplicates(raw)?;
    let Some(params) = root
        .0
        .iter()
        .find(|(key, _)| key == "params")
        .map(|(_, value)| *value)
    else {
        return serde_json::from_str(raw);
    };
    let Ok(params) = serde_json::from_str::<RawObject<'_>>(params.get()) else {
        return serde_json::from_str(raw);
    };
    let Some(state) = params
        .0
        .iter()
        .find(|(key, _)| key == "clientSecurityState")
        .map(|(_, value)| *value)
    else {
        return serde_json::from_str(raw);
    };
    let Ok(state_fields) = serde_json::from_str::<RawObject<'_>>(state.get()) else {
        return serde_json::from_str(raw);
    };
    let new = state_fields
        .0
        .iter()
        .any(|(key, _)| key == "localNetworkAccessRequestPolicy");
    let old = state_fields
        .0
        .iter()
        .any(|(key, _)| key == "privateNetworkRequestPolicy");
    if new && old {
        return Err(<serde_json::Error as serde::de::Error>::custom(
            "conflicting network policy fields",
        ));
    }
    if !new {
        return serde_json::from_str(raw);
    }
    let mut translated = String::from("{");
    for (index, (key, value)) in state_fields.0.iter().enumerate() {
        if index > 0 {
            translated.push(',');
        }
        let key = if key == "localNetworkAccessRequestPolicy" {
            "privateNetworkRequestPolicy"
        } else {
            key
        };
        translated.push_str(&serde_json::to_string(key)?);
        translated.push(':');
        translated.push_str(value.get());
    }
    translated.push('}');
    let offset = state.get().as_ptr() as usize - raw.as_ptr() as usize;
    let rewritten = format!(
        "{}{}{}",
        &raw[..offset],
        translated,
        &raw[offset + state.get().len()..]
    );
    serde_json::from_str(&rewritten)
}

/// Exchanges the messages with the websocket
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
pub struct Connection<T: EventMessage> {
    /// Queue of commands to send.
    pending_commands: VecDeque<MethodCall>,
    /// The websocket of the chromium instance
    ws: WebSocketStream<ConnectStream>,
    /// The identifier for a specific command
    next_id: usize,
    needs_flush: bool,
    /// The message that is currently being proceessed
    pending_flush: Option<MethodCall>,
    _marker: PhantomData<T>,
}

impl<T: EventMessage + Unpin> Connection<T> {
    pub async fn connect(debug_ws_url: impl AsRef<str>) -> Result<Self> {
        let config = WebSocketConfig::default()
            .max_message_size(None)
            .max_frame_size(None);

        let (ws, _) = async_tungstenite::tokio::connect_async_with_config(
            debug_ws_url.as_ref(),
            Some(config),
        )
        .await?;

        Ok(Self {
            pending_commands: Default::default(),
            ws,
            next_id: 0,
            needs_flush: false,
            pending_flush: None,
            _marker: Default::default(),
        })
    }
}

impl<T: EventMessage> Connection<T> {
    fn next_call_id(&mut self) -> CallId {
        let id = CallId::new(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Queue in the command to send over the socket and return the id for this
    /// command
    pub fn submit_command(
        &mut self,
        method: MethodId,
        session_id: Option<SessionId>,
        params: serde_json::Value,
    ) -> serde_json::Result<CallId> {
        let id = self.next_call_id();
        let call = MethodCall {
            id,
            method,
            session_id: session_id.map(Into::into),
            params,
        };
        self.pending_commands.push_back(call);
        Ok(id)
    }

    /// flush any processed message and start sending the next over the conn
    /// sink
    fn start_send_next(&mut self, cx: &mut Context<'_>) -> Result<()> {
        if self.needs_flush {
            match self.ws.poll_flush_unpin(cx) {
                Poll::Ready(Ok(())) => self.needs_flush = false,
                Poll::Ready(Err(err)) => return Err(err.into()),
                Poll::Pending => {}
            }
        }
        if self.pending_flush.is_none() && !self.needs_flush {
            if let Some(cmd) = self.pending_commands.pop_front() {
                tracing::trace!("Sending {:?}", cmd);
                let msg = serde_json::to_string(&cmd)?;
                self.ws.start_send_unpin(msg.into())?;
                self.pending_flush = Some(cmd);
            }
        }
        Ok(())
    }
}

impl<T: EventMessage + Unpin> Stream for Connection<T> {
    type Item = Result<Message<T>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pin = self.get_mut();

        loop {
            // queue in the next message if not currently flushing
            if let Err(err) = pin.start_send_next(cx) {
                return Poll::Ready(Some(Err(err)));
            }

            // send the message
            if let Some(call) = pin.pending_flush.take() {
                match pin.ws.poll_ready_unpin(cx) {
                    Poll::Ready(Ok(())) => {
                        pin.needs_flush = true;
                        continue;
                    }
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err.into()))),
                    Poll::Pending => pin.pending_flush = Some(call),
                }
            }

            break;
        }

        // read from the ws
        match ready!(pin.ws.poll_next_unpin(cx)) {
            Some(Ok(WsMessage::Text(text))) => {
                let ready = match decode_message::<T>(&text) {
                    Ok(msg) => {
                        tracing::trace!("Received {:?}", msg);
                        Ok(msg)
                    }
                    Err(err) => {
                        let msg = text.as_str().to_string();
                        tracing::debug!(target: "chromiumoxide::conn::raw_ws::parse_errors", msg, "Failed to parse raw WS message {}", err);
                        Err(CdpError::InvalidMessage(text.as_str().to_string(), err))
                    }
                };
                Poll::Ready(Some(ready))
            }
            Some(Ok(WsMessage::Close(_))) => Poll::Ready(None),
            // ignore ping and pong
            Some(Ok(WsMessage::Ping(_))) | Some(Ok(WsMessage::Pong(_))) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Some(Ok(msg)) => Poll::Ready(Some(Err(CdpError::UnexpectedWsMessage(msg)))),
            Some(Err(err)) => Poll::Ready(Some(Err(CdpError::Ws(err)))),
            None => {
                // ws connection closed
                Poll::Ready(None)
            }
        }
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    use chromiumoxide_cdp::cdp::{CdpEventMessage, events::CdpEvent};
    use tokio::net::TcpListener;

    // Redacted shape of a Chrome 156 event captured from the face runner.
    const CAPTURED: &str = r#"{"method":"Network.requestWillBeSentExtraInfo","params":{"requestId":"REDACTED","associatedCookies":[],"headers":{"Accept":"REDACTED","Accept-Encoding":"REDACTED","Accept-Language":"REDACTED","Connection":"REDACTED","Host":"REDACTED","Referer":"REDACTED","Sec-Fetch-Dest":"REDACTED","Sec-Fetch-Mode":"REDACTED","Sec-Fetch-Site":"REDACTED","User-Agent":"REDACTED","sec-ch-ua":"REDACTED","sec-ch-ua-mobile":"REDACTED","sec-ch-ua-platform":"REDACTED"},"connectTiming":{"requestTime":213466.237742},"clientSecurityState":{"initiatorIsSecureContext":true,"initiatorIPAddressSpace":"Loopback","localNetworkAccessRequestPolicy":"PermissionBlock"},"siteHasCookieInOtherPartition":false},"sessionId":"REDACTED"}"#;

    async fn replay(messages: Vec<String>) -> Vec<Result<Message<CdpEventMessage>>> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let count = messages.len();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            for message in messages {
                ws.send(message.into()).await.unwrap();
            }
        });
        let mut conn = Connection::<CdpEventMessage>::connect(format!("ws://{addr}"))
            .await
            .unwrap();
        let mut received = Vec::new();
        for _ in 0..count {
            received.push(
                tokio::time::timeout(std::time::Duration::from_secs(5), conn.next())
                    .await
                    .unwrap()
                    .unwrap(),
            );
        }
        server.await.unwrap();
        received
    }

    #[tokio::test]
    async fn policy_variants_and_legacy_events_keep_typed_values() {
        let mut inputs: Vec<String> = [
            "Allow",
            "BlockFromInsecureToMorePrivate",
            "WarnFromInsecureToMorePrivate",
            "PermissionBlock",
            "PermissionWarn",
        ]
        .iter()
        .map(|policy| CAPTURED.replace("PermissionBlock", policy))
        .collect();
        inputs.push(CAPTURED.replace(
            "localNetworkAccessRequestPolicy",
            "privateNetworkRequestPolicy",
        ));
        inputs.push(CAPTURED.replace(",\"clientSecurityState\":{\"initiatorIsSecureContext\":true,\"initiatorIPAddressSpace\":\"Loopback\",\"localNetworkAccessRequestPolicy\":\"PermissionBlock\"}", ""));
        let received = replay(inputs.clone()).await;
        for (raw, actual) in inputs.iter().zip(received) {
            let expected: Message<CdpEventMessage> = serde_json::from_str(&raw.replace(
                "localNetworkAccessRequestPolicy",
                "privateNetworkRequestPolicy",
            ))
            .unwrap();
            let (Message::Event(actual), Message::Event(expected)) = (actual.unwrap(), expected)
            else {
                panic!("expected events")
            };
            assert_eq!(actual.session_id, expected.session_id);
            let (
                CdpEvent::NetworkRequestWillBeSentExtraInfo(actual),
                CdpEvent::NetworkRequestWillBeSentExtraInfo(expected),
            ) = (actual.params, expected.params)
            else {
                panic!("expected network events")
            };
            assert_eq!(actual, expected);
        }
    }

    #[tokio::test]
    async fn ambiguous_or_invalid_events_remain_invalid_with_original_payload() {
        let key = "\"localNetworkAccessRequestPolicy\":\"PermissionBlock\"";
        let inputs = vec![
            CAPTURED.replace(key, "\"localNetworkAccessRequestPolicy\":42"),
            CAPTURED.replace(key, "\"localNetworkAccessRequestPolicy\":\"UnknownPolicy\""),
            CAPTURED.replace(",\"localNetworkAccessRequestPolicy\":\"PermissionBlock\"", ""),
            CAPTURED.replace("\"clientSecurityState\":{\"initiatorIsSecureContext\":true,\"initiatorIPAddressSpace\":\"Loopback\",\"localNetworkAccessRequestPolicy\":\"PermissionBlock\"}", "\"clientSecurityState\":42"),
            CAPTURED.replace(",\"connectTiming\":{\"requestTime\":213466.237742}", ""),
            CAPTURED.replace("\"initiatorIsSecureContext\":true,", ""),
            CAPTURED.replace(
                key,
                &format!("{key},\"privateNetworkRequestPolicy\":\"PermissionBlock\""),
            ),
            CAPTURED.replace(
                key,
                &format!("{key},\"privateNetworkRequestPolicy\":\"Allow\""),
            ),
            CAPTURED.replace(key, &format!("{key},{key}")),
            CAPTURED.replace(
                key,
                &format!("{key},\"localNetworkAccessRequestPol\\u0069cy\":\"Allow\""),
            ),
            CAPTURED.replace(
                "\"clientSecurityState\":{",
                "\"clientSecurityState\":{},\"clientSecurityState\":{",
            ),
            CAPTURED.replace("\"params\":{", "\"params\":{},\"params\":{"),
            CAPTURED.replace(
                "\"requestTime\":213466.237742",
                "\"requestTime\":1,\"requestTime\":2",
            ),
            CAPTURED.replace(
                "\"method\":\"Network.requestWillBeSentExtraInfo\"",
                "\"method\":\"Network.requestWillBeSentExtraInfo\",\"method\":\"Unknown.event\"",
            ),
            CAPTURED.replace(
                "\"sessionId\":\"REDACTED\"",
                "\"sessionId\":\"REDACTED\",\"sessionId\":\"other\"",
            ),
            CAPTURED.replace(
                "\"Accept\":\"REDACTED\"",
                "\"Accept\":\"one\",\"Accept\":\"two\"",
            ),
            CAPTURED[..CAPTURED.len() - 1].to_string(),
            r#"{"method":"Runtime.exceptionThrown","params":{}}"#.into(),
            r#"{"method":"Page.frameNavigated","params":{}}"#.into(),
            r#"{"method":"Target.attachedToTarget","params":{}}"#.into(),
            r#"{"method":"Network.responseReceived","params":{}}"#.into(),
        ];
        for (raw, result) in inputs.iter().zip(replay(inputs.clone()).await) {
            match result {
                Err(CdpError::InvalidMessage(payload, _)) => assert_eq!(&payload, raw),
                other => panic!("expected InvalidMessage for {raw}: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn unrelated_messages_keep_original_decoder_behavior() {
        let inputs = vec![
            r#"{"method":"Custom.unrecognized","params":{"clientSecurityState":{"localNetworkAccessRequestPolicy":"PermissionBlock"}}}"#.to_string(),
            r#"{"id":1,"result":{}}"#.into(),
            r#"{"id":2,"error":{"code":-32000,"message":"denied"}}"#.into(),
            r#"{"id":3,"result":{},"method":"Network.requestWillBeSentExtraInfo","params":{"clientSecurityState":{"localNetworkAccessRequestPolicy":"Allow","privateNetworkRequestPolicy":"BlockFromInsecureToMorePrivate"}}}"#.into(),
            r#"{"method":"Runtime.executionContextsCleared","params":{}}"#.into(),
        ];
        for (raw, actual) in inputs.iter().zip(replay(inputs.clone()).await) {
            let expected = serde_json::from_str::<Message<CdpEventMessage>>(raw);
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{raw}");
        }
    }

    #[tokio::test]
    async fn audits_event_does_not_get_network_alias() {
        let raw = r#"{"method":"Audits.issueAdded","params":{"issue":{"code":"CorsIssue","details":{"corsIssueDetails":{"corsErrorStatus":{"corsError":"LocalNetworkAccessPermissionDenied","failedParameter":""},"isWarning":false,"request":{"url":"https://example.test/"},"clientSecurityState":{"initiatorIsSecureContext":true,"initiatorIPAddressSpace":"Loopback","localNetworkAccessRequestPolicy":"Allow"}}}}}}"#.to_string();
        let expected = serde_json::from_str::<Message<CdpEventMessage>>(&raw);
        let actual = replay(vec![raw.clone()]).await.pop().unwrap();
        assert!(expected.is_err());
        assert!(
            serde_json::from_str::<Message<CdpEventMessage>>(&raw.replace(
                "localNetworkAccessRequestPolicy",
                "privateNetworkRequestPolicy"
            ))
            .is_ok()
        );
        assert!(matches!(actual, Err(CdpError::InvalidMessage(payload, _)) if payload == raw));
    }

    #[tokio::test]
    async fn chrome_156_captured_event_reaches_typed_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut ws = async_tungstenite::tokio::accept_async(socket)
                .await
                .unwrap();
            ws.send(CAPTURED.into()).await.unwrap();
        });
        let mut conn = Connection::<CdpEventMessage>::connect(format!("ws://{addr}"))
            .await
            .unwrap();
        let message = tokio::time::timeout(std::time::Duration::from_secs(5), conn.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let Message::Event(event) = message else {
            panic!("expected event")
        };
        assert_eq!(event.session_id.as_deref(), Some("REDACTED"));
        let CdpEvent::NetworkRequestWillBeSentExtraInfo(params) = event.params else {
            panic!("expected Network.requestWillBeSentExtraInfo")
        };
        let state = params.client_security_state.unwrap();
        assert_eq!(
            serde_json::to_value(state).unwrap()["privateNetworkRequestPolicy"],
            "PermissionBlock"
        );
        assert_eq!(params.site_has_cookie_in_other_partition, Some(false));
        server.await.unwrap();
    }
}
