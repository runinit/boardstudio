//! The accepted wiring source and plan resolution that the PCB wiring owner projects and the
//! keymap and firmware export read.
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::{electrical::ElectricalPlan, model::ProjectDoc};
pub use boardstudio_web_runtime::firmware_position_projection::FirmwarePlanIdentity as WiringPlanIdentity;
use std::{rc::Rc, sync::Arc};

/// A cheap view of the accepted source. Arc identity avoids deep document comparisons in the
/// Dioxus props diff; the leaf never receives writable Session access.
#[derive(Clone)]
pub struct PcbWiringSource {
    pub identity: WiringPlanIdentity,
    pub ui_scope: Scope,
    pub scope_generation: u64,
    pub document: Arc<ProjectDoc>,
    pub active_part_id: Option<String>,
}

impl PcbWiringSource {
    pub fn new(
        accepted: &AcceptedSnapshot,
        scope: &Scope,
        ui_scope: &Scope,
        active_part_id: Option<&str>,
        executor_epoch: u64,
        scope_generation: u64,
    ) -> Option<Self> {
        if scope.instance_id.is_some()
            || (Scope {
                instance_id: None,
                ..ui_scope.clone()
            }) != *scope
            || scope.session_epoch != accepted.session_epoch
            || scope.document_id != accepted.document.id
            || accepted.scene.revision != accepted.document.revision
            || !accepted
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        {
            return None;
        }
        Some(Self {
            identity: WiringPlanIdentity {
                scope: scope.clone(),
                token: accepted.token,
                revision: accepted.document.revision,
                executor_epoch,
            },
            ui_scope: ui_scope.clone(),
            scope_generation,
            document: accepted.document.clone(),
            active_part_id: active_part_id.map(str::to_owned),
        })
    }
}

impl PartialEq for PcbWiringSource {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
            && Arc::ptr_eq(&self.document, &other.document)
            && self.active_part_id == other.active_part_id
            && self.ui_scope == other.ui_scope
            && self.scope_generation == other.scope_generation
    }
}

#[derive(Clone, Debug)]
pub enum PcbWiringResolution {
    Idle,
    Pending {
        identity: WiringPlanIdentity,
    },
    Current {
        identity: WiringPlanIdentity,
        plan: Rc<ElectricalPlan>,
    },
    Failed {
        identity: WiringPlanIdentity,
        message: String,
    },
}

impl PartialEq for PcbWiringResolution {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Idle, Self::Idle) => true,
            (Self::Pending { identity: a }, Self::Pending { identity: b }) => a == b,
            (
                Self::Current {
                    identity: a_identity,
                    plan: a_plan,
                },
                Self::Current {
                    identity: b_identity,
                    plan: b_plan,
                },
            ) => a_identity == b_identity && Rc::ptr_eq(a_plan, b_plan),
            (
                Self::Failed {
                    identity: a_identity,
                    message: a_message,
                },
                Self::Failed {
                    identity: b_identity,
                    message: b_message,
                },
            ) => a_identity == b_identity && a_message == b_message,
            _ => false,
        }
    }
}
