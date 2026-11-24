//! Pure board-level wiring helpers and synchronous edit resolvers.
//!
//! Every resolver revalidates against the accepted document it is handed, so
//! the declarative inspector can commit plain board ids directly through
//! `Runtime::commit` without identity or settlement plumbing.
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core_mechanical::electrical::{self, ElectricalMode, ElectricalPlan};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use std::rc::Rc;

/// The accepted board plan matching `identity`; ADR-0005 `Unchanged` stays in `replace_document_resolver`.
pub(super) fn current_plan_for(
    resolution: &PcbWiringResolution,
    identity: &WiringPlanIdentity,
) -> Option<Rc<ElectricalPlan>> {
    let PcbWiringResolution::Current {
        identity: current,
        plan,
    } = resolution
    else {
        return None;
    };
    (current == identity
        && plan.revision == identity.revision
        && plan.board_id.as_deref() == Some(identity.scope.board_id.as_str())
        && plan.instance_id.is_none())
    .then(|| plan.clone())
}

/// One electrical preview-then-resolve shared by firmware-position and pin locks.
pub(super) fn resolve_preview_plan(document: &ProjectDoc, board_id: &str) -> ElectricalPlan {
    use boardstudio_core_mechanical::electrical::{ElectricalPlanRequest, resolve};

    let configuration = document.hardware.as_ref().and_then(|hardware| {
        hardware
            .boards
            .iter()
            .find(|item| item.board_id == board_id)
    });
    resolve(ElectricalPlanRequest {
        document: document.clone(),
        instance_id: None,
        mode: configuration.map_or(ElectricalMode::Matrix, |item| item.mode),
        locks: configuration.map_or_else(Default::default, |item| item.locks.clone()),
        controller_profile: None,
        board_id: Some(board_id.to_owned()),
        controller_part_id: configuration.and_then(|item| item.controller_part_id.clone()),
    })
}

/// The label used in the wiring panel header.
pub(super) fn mode_text(mode: ElectricalMode) -> &'static str {
    match mode {
        ElectricalMode::Matrix => "matrix",
        ElectricalMode::Direct => "direct",
    }
}

/// The board's accepted wiring mode; a board without configuration is a matrix.
pub(super) fn board_mode(document: &ProjectDoc, board_id: &str) -> ElectricalMode {
    document
        .hardware
        .as_ref()
        .and_then(|hardware| {
            hardware
                .boards
                .iter()
                .find(|configuration| configuration.board_id == board_id)
        })
        .map_or(ElectricalMode::Matrix, |configuration| configuration.mode)
}

type DocumentEdit = Box<dyn Fn(&AcceptedSnapshot, &mut ProjectDoc) -> Result<Vec<String>, String>>;

/// The shared `ReplaceDocument` resolver for board-wiring edits: clone the
/// accepted document, apply `edit` (submit targets or retire reason), resolve
/// `Unchanged` when the clone still equals the accepted document (ADR-0005).
pub(super) fn replace_document_resolver(
    name: &'static str,
    edit: impl Fn(&AcceptedSnapshot, &mut ProjectDoc) -> Result<Vec<String>, String> + 'static,
) -> EditResolver {
    replace_document_resolver_boxed(name, Box::new(edit))
}

fn replace_document_resolver_boxed(name: &'static str, edit: DocumentEdit) -> EditResolver {
    EditResolver::new(name, move |accepted: &AcceptedSnapshot| {
        let mut replacement = accepted.document.as_ref().clone();
        match edit(accepted, &mut replacement) {
            Err(reason) => Resolution::Retire(reason),
            Ok(_) if replacement == *accepted.document => Resolution::Unchanged,
            Ok(target_ids) => Resolution::submit(
                target_ids,
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
            ),
        }
    })
}

pub(super) fn mode_resolver(board_id: String, mode: ElectricalMode) -> EditResolver {
    EditResolver::new("pcb-wiring-mode", move |accepted: &AcceptedSnapshot| {
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == board_id)
        {
            return Resolution::Retire("The board was deleted.".into());
        }
        if board_mode(&accepted.document, &board_id) == mode {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![board_id.clone()],
            EditOperation::SetWiringMode {
                board_id: board_id.clone(),
                mode,
            },
        )
    })
}

/// The plan an Apply click may use: current for the board's accepted mode and,
/// unless releasing reviewed connections, free of error diagnostics.
pub(super) fn apply_plan(
    source: &PcbWiringSource,
    resolution: &PcbWiringResolution,
    release: bool,
) -> Option<Rc<ElectricalPlan>> {
    current_plan_for(resolution, &source.identity).filter(|plan| {
        plan.mode == board_mode(&source.document, &source.identity.scope.board_id)
            && (release
                || !plan
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == "error"))
    })
}

pub(super) fn apply_resolver(
    board_id: String,
    plan: ElectricalPlan,
    review: Option<super::connections::ExistingConnectionReview>,
) -> EditResolver {
    replace_document_resolver("pcb-wiring-apply", move |accepted, replacement| {
        if !replacement.boards.iter().any(|board| board.id == board_id) {
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
        if board_mode(&accepted.document, &board_id) != plan.mode {
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

/// The accepted snapshot while the editor is Ready, durable (Saved/Saving) and
/// free of gestures and display previews — the shared liveness floor for every
/// scoped PCB action owner.
pub fn ready_snapshot(runtime: &Runtime) -> Option<AcceptedSnapshot> {
    let model = runtime.model();
    if model.lifecycle != Lifecycle::Ready
        || model.gesture.is_some()
        || model.display_preview.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
    {
        return None;
    }
    model.accepted
}

pub(super) fn pcb_source(
    runtime: &Runtime,
    generation: u64,
) -> Option<(AcceptedSnapshot, PcbWiringSource)> {
    let snapshot = ready_snapshot(runtime)?;
    let model = runtime.model();
    let scope = runtime.scope()?;
    if model.active_board_id != scope.board_id || model.active_instance_id != scope.instance_id {
        return None;
    }
    let board_scope = Scope {
        instance_id: None,
        ..scope.clone()
    };
    let source = PcbWiringSource::new(
        &snapshot,
        &board_scope,
        &scope,
        model.selected_part_ids.first().map(String::as_str),
        generation,
    )?;
    Some((snapshot, source))
}

#[cfg(test)]
mod board_mode_tests {
    use super::*;

    #[test]
    fn a_board_without_configuration_is_a_matrix_and_mode_text_round_trips() {
        let document = boardstudio_model::model::ProjectDoc::empty("project", "Mode text");
        assert_eq!(board_mode(&document, "left"), ElectricalMode::Matrix);
        assert_eq!(mode_text(ElectricalMode::Matrix), "matrix");
        assert_eq!(mode_text(ElectricalMode::Direct), "direct");
    }
}
