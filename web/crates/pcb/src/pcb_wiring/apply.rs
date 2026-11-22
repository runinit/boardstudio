//! Apply the exact current accepted board plan through the normal Session edit owner.
use super::mode::{
    BoardWiringFailure, BoardWiringModeIdentity, board_mode, current_edit_snapshot, mode_identity,
};
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_core_mechanical::electrical::{self, ElectricalPlan};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct BoardWiringApplyActions {
    pub identity: Option<BoardWiringModeIdentity>,
    /// The current plan matches the accepted document.
    pub editable: bool,
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
    let _ = version();
    let failure = use_signal(|| None::<BoardWiringFailure>);
    let submit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let mut failure = failure;
        move |(identity, release): (BoardWiringModeIdentity, bool)| {
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
            let target = identity.feedback_target();
            let resolver = apply_resolver(identity, (*plan).clone(), review);
            failure.set(
                runtime
                    .commit("pcb-wiring-apply", resolver)
                    .err()
                    .map(|message| BoardWiringFailure { target, message }),
            );
        }
    });
    let on_apply = use_callback(move |identity| submit.call((identity, false)));
    let on_release_reviewed_connections =
        use_callback(move |identity| submit.call((identity, true)));
    let identity = source.as_ref().map(mode_identity);
    let editable = identity.as_ref().is_some_and(|identity| {
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
