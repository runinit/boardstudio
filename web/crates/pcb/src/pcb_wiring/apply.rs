//! Apply the exact current accepted board plan through the normal Session edit owner.
use super::mode::{current_edit_snapshot, mode_identity};
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::pcb_wiring_mode_operation::BoardWiringModeIdentity;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core::{
    electrical::{self, ElectricalPlan},
    model::{EditOperation, ProjectDoc},
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardWiringApplyFeedback {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringApplyFeedbackView {
    pub target: crate::pcb_wiring_mode_operation::BoardWiringModeFeedbackTarget,
    pub request_plan: WiringPlanIdentity,
    pub state: BoardWiringApplyFeedback,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringApplyActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub feedback: Option<BoardWiringApplyFeedbackView>,
    pub on_apply: EventHandler<BoardWiringModeIdentity>,
    pub on_release_reviewed_connections: EventHandler<BoardWiringModeIdentity>,
}

use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
#[derive(Clone, Copy)]
struct ApplyTickets(Signal<Vec<(BoardWiringModeIdentity, EditTicket, bool)>>);

pub(super) fn release_pending() -> bool {
    try_consume_context::<ApplyTickets>().is_some_and(|tickets| {
        tickets
            .0
            .read()
            .iter()
            .any(|(_, ticket, release)| *release && ticket.is_pending())
    })
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
    let tickets = use_signal(Vec::<(BoardWiringModeIdentity, EditTicket, bool)>::new);
    use_context_provider(|| ApplyTickets(tickets));
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let feedback = use_signal(|| None::<BoardWiringApplyFeedbackView>);
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut tickets = tickets;
        let mut feedback = feedback;
        move |_| {
            let mut entries = tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(identity, ticket, _)| {
                let live = workspace() == "PCB"
                    && identity.feedback_target().is_visible(
                        runtime.scope().as_ref(),
                        runtime
                            .model()
                            .selected_part_ids
                            .first()
                            .map(String::as_str),
                        scope_generation(),
                    );
                let state = match ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(BoardWiringApplyFeedback::Saved),
                    Settlement::Failed { message } => {
                        Some(BoardWiringApplyFeedback::Failed(message))
                    }
                    Settlement::Retired => None,
                };
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(state.map(|state| BoardWiringApplyFeedbackView {
                        target: identity.feedback_target(),
                        request_plan: identity.plan.clone(),
                        state,
                    }));
                }
                false
            });
            if before != entries.len() {
                tickets.set(entries);
            }
        }
    }));
    let submit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut tickets = tickets;
        let mut latest = latest;
        let mut feedback = feedback;
        move |(identity, release): (BoardWiringModeIdentity, bool)| {
            if tickets
                .peek()
                .iter()
                .any(|(_, ticket, kind)| *kind == release && ticket.is_pending())
            {
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
            let ticket = EditTicket::begin(
                &runtime,
                "pcb-wiring-apply",
                Some("wiring plan".into()),
                apply_resolver(identity.clone(), (*plan).clone(), review),
            );
            latest.set(Some(ticket.operation()));
            // One-shot actions stay quiet while their own ticket is pending.
            feedback.set(None);
            tickets.write().push((identity, ticket, release));
        }
    });
    let on_apply = use_callback(move |identity| submit.call((identity, false)));
    let on_release_reviewed_connections =
        use_callback(move |identity| submit.call((identity, true)));
    let identity = source.as_ref().map(mode_identity);
    let editable = !tickets()
        .iter()
        .any(|(_, ticket, release)| !release && ticket.is_pending())
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
    let feedback = feedback().filter(|item| target.as_ref() == Some(&item.target));
    BoardWiringApplyActions {
        identity,
        editable,
        feedback,
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
) -> boardstudio_core::electrical::ElectricalMode {
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
            boardstudio_core::electrical::ElectricalMode::Matrix,
            |configuration| configuration.mode,
        )
}
