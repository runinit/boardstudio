//! Apply the exact current accepted board plan through the normal Session edit owner.
use super::mode::{BoardWiringFailure, current_edit_snapshot, mode_identity};
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::pcb_wiring_mode_operation::{BoardWiringModeFeedbackTarget, BoardWiringModeIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core_mechanical::electrical::{self, ElectricalPlan};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
use dioxus::prelude::*;
use std::rc::Rc;

/// The two one-shot action kinds this panel runs. Their controls disable while the
/// matching edit is pending, so a double click cannot apply or release twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BoardWiringApplyKey {
    Apply,
    ReleaseReviewedConnections,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringApplyActions {
    pub identity: Option<BoardWiringModeIdentity>,
    /// The Apply control is available: no apply edit is pending and the current plan
    /// still matches the accepted document.
    pub editable: bool,
    /// The reviewed-connection release control is unavailable while its edit is pending.
    pub(crate) release_disabled: bool,
    /// The current board target's failure; landing and retirement are silent.
    pub(crate) failure: Option<String>,
    pub on_apply: EventHandler<BoardWiringModeIdentity>,
    pub on_release_reviewed_connections: EventHandler<BoardWiringModeIdentity>,
}

pub fn use_board_wiring_apply(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> BoardWiringApplyActions {
    let mut edits =
        use_signal(PendingEdits::<BoardWiringApplyKey, BoardWiringModeFeedbackTarget>::default);
    let latest = use_signal(|| None::<BoardWiringApplyKey>);
    let failure = use_signal(|| None::<BoardWiringFailure>);
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let mut failure = failure;
        move |_| {
            let live = workspace() == "PCB";
            if live && !edits.peek().has_terminal() {
                return;
            }
            for (result, target) in edits.write().settle_with(live) {
                let (key, message) = match result {
                    PendingEditResult::Failed { key, message } => (key, Some(message)),
                    PendingEditResult::Landed { key, .. } | PendingEditResult::Retired { key } => {
                        (key, None)
                    }
                };
                if latest.peek().as_ref() == Some(&key) {
                    failure.set(message.map(|message| BoardWiringFailure { target, message }));
                }
            }
        }
    }));
    let submit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut latest = latest;
        let mut failure = failure;
        move |(identity, release): (BoardWiringModeIdentity, bool)| {
            let key = if release {
                BoardWiringApplyKey::ReleaseReviewedConnections
            } else {
                BoardWiringApplyKey::Apply
            };
            if edits.peek().is_pending(&key) {
                return;
            }
            let Some(snapshot) = current_edit_snapshot(
                &runtime,
                &identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let plan = if release {
                current_review_plan(&identity.plan, &resolution.read(), &snapshot.document)
            } else {
                current_plan(&identity.plan, &resolution.read(), &snapshot.document)
            };
            let Some(plan) = plan else {
                return;
            };
            let review = if release {
                super::connections::existing_connection_review(&snapshot.document, &plan)
            } else {
                None
            };
            if release && review.is_none() {
                return;
            }
            latest.set(Some(key));
            // One-shot actions stay quiet while their own ticket is pending.
            failure.set(None);
            let target = identity.feedback_target();
            edits.write().begin_with(
                &runtime,
                key,
                "pcb-wiring-apply",
                Some("wiring plan".into()),
                apply_resolver(identity, (*plan).clone(), review),
                target,
            );
        }
    });
    let on_apply = use_callback(move |identity| submit.call((identity, false)));
    let on_release_reviewed_connections =
        use_callback(move |identity| submit.call((identity, true)));
    let identity = source.as_ref().map(mode_identity);
    let editable = !edits.read().is_pending(&BoardWiringApplyKey::Apply)
        && identity.as_ref().is_some_and(|identity| {
            current_edit_snapshot(
                &runtime,
                identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            )
            .is_some_and(|snapshot| {
                current_plan(&identity.plan, &resolution.read(), &snapshot.document).is_some()
            })
        });
    let target = identity
        .as_ref()
        .map(BoardWiringModeIdentity::feedback_target);
    let failure = failure()
        .filter(|failure| target.as_ref() == Some(&failure.target))
        .map(|failure| failure.message);
    BoardWiringApplyActions {
        identity,
        editable,
        release_disabled: edits
            .read()
            .is_pending(&BoardWiringApplyKey::ReleaseReviewedConnections),
        failure,
        on_apply,
        on_release_reviewed_connections,
    }
}

fn apply_resolver(
    identity: BoardWiringModeIdentity,
    plan: ElectricalPlan,
    review: Option<super::connections::ExistingConnectionReview>,
) -> EditResolver {
    EditResolver::new("pcb-wiring-apply", move |accepted: &AcceptedSnapshot| {
        let board_id = &identity.ui_scope.board_id;
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| &board.id == board_id)
        {
            return Resolution::Retire("The board was deleted.".into());
        }
        if plan
            .controller_part_id
            .as_ref()
            .is_some_and(|id| !accepted.document.parts.iter().any(|part| &part.id == id))
            || plan.nets.iter().flat_map(|net| &net.pins).any(|pin| {
                !accepted
                    .document
                    .parts
                    .iter()
                    .any(|part| part.id == pin.part_id)
            })
        {
            return Resolution::Retire(
                "A part used by this wiring plan was deleted. Resolve the plan again.".into(),
            );
        }
        if board_mode(&accepted.document, board_id) != plan.mode {
            return Resolution::Retire(
                "The board wiring mode changed. Resolve the plan again.".into(),
            );
        }
        let proposal = if let Some(review) = &review {
            if review.nets.iter().any(|reviewed| {
                !accepted
                    .document
                    .nets
                    .iter()
                    .any(|net| net.id == reviewed.id)
            }) {
                return Resolution::Retire("A reviewed connection net was deleted.".into());
            }
            super::connections::release_reviewed_connections(&accepted.document, review)
        } else {
            let mut proposal = (*accepted.document).clone();
            let mut plan = plan.clone();
            plan.revision = accepted.document.revision;
            if let Err(reason) = electrical::materialize(&mut proposal, &plan) {
                return Resolution::Retire(reason);
            }
            proposal
        };
        if proposal == *accepted.document {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![board_id.clone()],
            EditOperation::ReplaceDocument {
                document: Box::new(proposal),
            },
        )
    })
}

fn current_plan(
    identity: &WiringPlanIdentity,
    resolution: &PcbWiringResolution,
    document: &ProjectDoc,
) -> Option<Rc<ElectricalPlan>> {
    current_review_plan(identity, resolution, document).filter(|plan| {
        !plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == "error")
    })
}

fn current_review_plan(
    identity: &WiringPlanIdentity,
    resolution: &PcbWiringResolution,
    document: &ProjectDoc,
) -> Option<Rc<ElectricalPlan>> {
    let PcbWiringResolution::Current {
        identity: current,
        plan,
    } = resolution
    else {
        return None;
    };
    if current != identity
        || plan.revision != identity.revision
        || plan.board_id.as_deref() != Some(identity.scope.board_id.as_str())
        || plan.instance_id.is_some()
        || plan.mode != board_mode(document, &identity.scope.board_id)
    {
        return None;
    }
    Some(plan.clone())
}

fn board_mode(
    document: &ProjectDoc,
    board_id: &str,
) -> boardstudio_core_mechanical::electrical::ElectricalMode {
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        })
        .map_or(
            boardstudio_core_mechanical::electrical::ElectricalMode::Matrix,
            |configuration| configuration.mode,
        )
}
