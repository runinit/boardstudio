//! Apply the exact current accepted board plan through the normal Session edit owner.
use super::mode::{
    BoardWiringFailure, BoardWiringModeIdentity, board_mode, current_edit_snapshot, mode_identity,
    settle_board_commit,
};
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::EditResolver;
use boardstudio_core_mechanical::electrical::{self, ElectricalPlan};
use boardstudio_model::model::ProjectDoc;
use boardstudio_web_ui_model::Workspace;
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
    workspace: Signal<Workspace>,
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
            settle_board_commit(
                &runtime,
                "pcb-wiring-apply",
                apply_resolver(identity, (*plan).clone(), review),
                target,
                failure,
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
    let failure = BoardWiringFailure::for_identity(failure(), identity.as_ref());
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
    super::mode::replace_document_resolver("pcb-wiring-apply", move |accepted, replacement| {
        let board_id = &identity.ui_scope.board_id;
        if !replacement.boards.iter().any(|board| &board.id == board_id) {
            return Err("The board was deleted.".into());
        }
        if plan
            .controller_part_id
            .as_ref()
            .is_some_and(|id| !replacement.parts.iter().any(|part| &part.id == id))
            || plan
                .nets
                .iter()
                .flat_map(|net| &net.pins)
                .any(|pin| !replacement.parts.iter().any(|part| part.id == pin.part_id))
        {
            return Err(
                "A part used by this wiring plan was deleted. Resolve the plan again.".into(),
            );
        }
        if board_mode(&accepted.document, board_id) != plan.mode {
            return Err("The board wiring mode changed. Resolve the plan again.".into());
        }
        if let Some(review) = &review {
            if review
                .nets
                .iter()
                .any(|reviewed| !replacement.nets.iter().any(|net| net.id == reviewed.id))
            {
                return Err("A reviewed connection net was deleted.".into());
            }
            *replacement =
                super::connections::release_reviewed_connections(&accepted.document, review);
        } else {
            let mut plan = plan.clone();
            plan.revision = accepted.document.revision;
            electrical::materialize(replacement, &plan)?;
        }
        Ok(vec![board_id.clone()])
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
    super::mode::current_plan_for(resolution, identity)
        .filter(|plan| plan.mode == board_mode(document, &identity.scope.board_id))
}
