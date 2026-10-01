use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Identity {
    pub epoch: u32,
    pub operation: u32,
}

pub struct Callers {
    epoch: u32,
    closed: bool,
    pending: BTreeSet<Identity>,
}

impl Callers {
    pub fn new(epoch: u32) -> Self {
        Self {
            epoch,
            closed: false,
            pending: BTreeSet::new(),
        }
    }

    pub fn begin(&mut self, id: Identity) -> Result<(), String> {
        if self.closed || id.epoch != self.epoch || !self.pending.insert(id) {
            return Err("closed, obsolete executor or duplicate operation".into());
        }
        Ok(())
    }

    pub fn complete(&mut self, id: Identity) -> bool {
        !self.closed && id.epoch == self.epoch && self.pending.remove(&id)
    }

    pub fn fail(&mut self) -> Vec<Identity> {
        self.closed = true;
        std::mem::take(&mut self.pending).into_iter().collect()
    }

    pub fn pending(&self) -> usize {
        self.pending.len()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub id: Identity,
    pub action: Action,
}

#[derive(Serialize, Deserialize)]
pub enum Action {
    Core(Box<boardstudio_core::model::CoreRequest>),
    Noise,
    Echo,
    Crash,
}

#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub id: Identity,
    pub payload: ResultPayload,
}

#[derive(Serialize, Deserialize)]
pub enum ResultPayload {
    Core(Box<boardstudio_core::model::CoreReply>),
    Ack,
}

pub fn execute(engine: &mut boardstudio_core::CoreEngine, request: Request) -> Option<Reply> {
    let payload = match request.action {
        Action::Core(request) => ResultPayload::Core(Box::new(engine.handle(*request))),
        Action::Noise | Action::Echo => ResultPayload::Ack,
        Action::Crash => return None,
    };
    Some(Reply {
        id: request.id,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_provider_roundtrip_keeps_document_and_request_identity() {
        use boardstudio_core::{
            CoreEngine,
            model::{CoreReply, CoreRequest, ProjectDoc},
        };
        let id = Identity {
            epoch: 9,
            operation: 20,
        };
        let mut engine = CoreEngine::new();
        let request = Request {
            id,
            action: Action::Core(Box::new(CoreRequest::Open {
                id: "open-20".into(),
                document: ProjectDoc::empty("probe-copy", "P1 copied test input"),
            })),
        };
        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: Request = serde_json::from_str(&encoded).unwrap();
        let reply = execute(&mut engine, decoded).unwrap();
        assert_eq!(reply.id, id);
        let ResultPayload::Core(opened) = reply.payload else {
            panic!("expected core reply")
        };
        let CoreReply::Scene {
            id: request_id,
            document,
            ..
        } = *opened
        else {
            panic!("expected scene")
        };
        assert_eq!(request_id, "open-20");
        assert_eq!(document.id, "probe-copy");
        let reply = execute(
            &mut engine,
            Request {
                id,
                action: Action::Core(Box::new(CoreRequest::Snapshot {
                    id: "snapshot-21".into(),
                })),
            },
        )
        .unwrap();
        let ResultPayload::Core(snapshot) = reply.payload else {
            panic!("expected snapshot")
        };
        let CoreReply::Scene { document: next, .. } = *snapshot else {
            panic!("expected scene")
        };
        assert_eq!(next, document);
    }

    #[test]
    fn obsolete_executor_and_unsolicited_reply_leave_real_caller_pending() {
        let mut callers = Callers::new(7);
        let id = Identity {
            epoch: 7,
            operation: 11,
        };
        assert!(callers.begin(id).is_ok());
        assert!(!callers.complete(Identity {
            epoch: 6,
            operation: 11
        }));
        assert!(!callers.complete(Identity {
            epoch: 7,
            operation: 99
        }));
        assert_eq!(callers.pending(), 1);
        assert!(callers.complete(id));
        assert!(!callers.complete(id));
        assert_eq!(callers.pending(), 0);
    }

    #[test]
    fn duplicate_operation_never_replaces_an_existing_caller() {
        let mut callers = Callers::new(1);
        let id = Identity {
            epoch: 1,
            operation: 2,
        };
        assert!(callers.begin(id).is_ok());
        assert!(callers.begin(id).is_err());
        assert!(
            callers
                .begin(Identity {
                    epoch: 0,
                    operation: 3
                })
                .is_err()
        );
        assert_eq!(callers.pending(), 1);
    }

    #[test]
    fn failure_and_close_drain_callers_and_reject_new_work_without_replay() {
        let mut callers = Callers::new(5);
        let a = Identity {
            epoch: 5,
            operation: 1,
        };
        let b = Identity {
            epoch: 5,
            operation: 2,
        };
        callers.begin(a).unwrap();
        callers.begin(b).unwrap();
        assert_eq!(callers.fail(), vec![a, b]);
        assert_eq!(callers.pending(), 0);
        assert!(!callers.complete(a));
        assert!(
            callers
                .begin(Identity {
                    epoch: 5,
                    operation: 3
                })
                .is_err()
        );
        assert!(callers.fail().is_empty());
    }
}

#[cfg(all(target_arch = "wasm32", feature = "worker"))]
mod worker;
