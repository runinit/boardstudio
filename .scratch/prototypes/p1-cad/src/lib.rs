use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Identity {
    pub epoch: u32,
    pub operation: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyStamp {
    pub id: String,
    pub name: String,
    pub key: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    pub base: u32,
    pub next: u32,
    pub ids: Vec<String>,
    pub changed: Vec<BodyStamp>,
}
pub struct Lifecycle {
    epoch: u32,
    closed: bool,
    current: Option<Identity>,
    pending: std::collections::BTreeSet<Identity>,
    base: u32,
    bodies: std::collections::BTreeMap<String, BodyStamp>,
}
impl Lifecycle {
    pub fn new(epoch: u32) -> Self {
        Self {
            epoch,
            closed: false,
            current: None,
            pending: Default::default(),
            base: 0,
            bodies: Default::default(),
        }
    }
    pub fn begin(&mut self, id: Identity) -> Result<(), String> {
        if self.closed || id.epoch != self.epoch || !self.pending.insert(id) {
            return Err("closed, obsolete executor or duplicate operation".into());
        }
        self.current = Some(id);
        Ok(())
    }
    pub fn finish(&mut self, id: Identity) -> Result<bool, String> {
        if self.closed || id.epoch != self.epoch || !self.pending.remove(&id) {
            return Err("unsolicited or obsolete reply".into());
        }
        Ok(self.current == Some(id))
    }
    pub fn finish_preview(&mut self, id: Identity, delta: &Delta) -> Result<bool, String> {
        if !self.pending.contains(&id) || self.closed || id.epoch != self.epoch {
            return Err("unsolicited or obsolete reply".into());
        }
        if delta.base != self.base || self.base.checked_add(1) != Some(delta.next) {
            return Err("wrong cache base".into());
        }
        self.bodies.retain(|key, _| delta.ids.contains(key));
        for body in &delta.changed {
            self.bodies.insert(body.id.clone(), body.clone());
        }
        self.base = delta.next;
        self.finish(id)
    }
    pub fn cache_base(&self) -> u32 {
        self.base
    }
    pub fn body_key(&self, id: &str) -> Option<&str> {
        self.bodies.get(id).map(|b| b.key.as_str())
    }
    pub fn pending(&self) -> usize {
        self.pending.len()
    }
    pub fn fail(&mut self) -> Vec<Identity> {
        self.closed = true;
        std::mem::take(&mut self.pending).into_iter().collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_control_retains_target_identity() {
        let request:Request=serde_json::from_str(r#"{"id":{"epoch":7,"operation":2},"action":{"Cancel":{"target":{"epoch":7,"operation":1}}}}"#).unwrap();
        assert!(matches!(
            request.action,
            Action::Cancel {
                target: Identity {
                    epoch: 7,
                    operation: 1
                }
            }
        ));
    }
    #[test]
    fn wrong_base_delta_keeps_cache_and_caller_unchanged() {
        let mut ledger = Lifecycle::new(7);
        let id = Identity {
            epoch: 7,
            operation: 1,
        };
        ledger.begin(id).unwrap();
        let delta = Delta {
            base: 9,
            next: 10,
            ids: vec![],
            changed: vec![],
        };
        assert!(ledger.finish_preview(id, &delta).is_err());
        assert_eq!(ledger.cache_base(), 0);
        assert_eq!(ledger.pending(), 1);
    }

    #[test]
    fn completed_obsolete_preview_advances_cache_before_display_rejection() {
        let mut ledger = Lifecycle::new(7);
        let old = Identity {
            epoch: 7,
            operation: 1,
        };
        let current = Identity {
            epoch: 7,
            operation: 2,
        };
        ledger.begin(old).unwrap();
        ledger.begin(current).unwrap();
        let delta = Delta {
            base: 0,
            next: 1,
            ids: vec!["case".into()],
            changed: vec![BodyStamp {
                id: "case".into(),
                name: "Plate".into(),
                key: "captured-shape".into(),
            }],
        };
        assert!(!ledger.finish_preview(old, &delta).unwrap());
        assert_eq!(ledger.cache_base(), 1);
        assert_eq!(ledger.body_key("case"), Some("captured-shape"));
        assert_eq!(ledger.pending(), 1);
    }
}

#[derive(Serialize, Deserialize)]
pub struct Request {
    pub id: Identity,
    pub action: Action,
}
#[derive(Serialize, Deserialize)]
pub enum Action {
    Preview { prepared: String },
    Export { prepared: String },
    ReadStep,
    Status,
    Crash,
    Cancel { target: Identity },
}
#[derive(Serialize, Deserialize)]
pub struct Reply {
    pub id: Identity,
    pub payload: Payload,
}
#[derive(Serialize, Deserialize)]
pub enum Payload {
    Preview {
        revision: u64,
        delta: Delta,
    },
    Export {
        revision: u64,
    },
    Model {
        min: [f64; 3],
        max: [f64; 3],
    },
    Status {
        detached: bool,
        transferred_bytes: u64,
    },
    Error(String),
    Cancelled,
}
#[cfg(all(target_arch = "wasm32", feature = "worker"))]
mod worker;
