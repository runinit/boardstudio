//! Accepted board-level mode edits for PCB wiring.
use super::{PcbWiringResolution, PcbWiringSource, WiringPlanIdentity};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope,
};
use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlan};
use boardstudio_model::model::{EditOperation, ProjectDoc};
use boardstudio_web_ui_model::Workspace;
#[cfg(target_arch = "wasm32")]
use boardstudio_web_ui_model::editor::{EditGuard, use_guarded_edit};
use dioxus::prelude::*;
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
    use boardstudio_core_mechanical::electrical::{ElectricalMode, ElectricalPlanRequest, resolve};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeIdentity {
    /// Selection-independent accepted board-plan identity.
    pub plan: WiringPlanIdentity,
    /// Selection and UI-scope identity of the rendered control.
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

impl BoardWiringModeIdentity {
    pub fn feedback_target(&self) -> BoardWiringModeFeedbackTarget {
        BoardWiringModeFeedbackTarget {
            ui_scope: self.ui_scope.clone(),
            selected_part_id: self.selected_part_id.clone(),
            scope_generation: self.scope_generation,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeFeedbackTarget {
    pub ui_scope: Scope,
    pub selected_part_id: Option<String>,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardWiringModeEditRequest {
    pub identity: BoardWiringModeIdentity,
    pub mode: ElectricalMode,
}

/// The label used in the wiring panel header.
pub(crate) fn mode_text(mode: ElectricalMode) -> String {
    match mode {
        ElectricalMode::Matrix => "matrix".to_owned(),
        ElectricalMode::Direct => "direct".to_owned(),
    }
}

/// The board's accepted wiring mode; a board without configuration is a matrix.
pub(crate) fn board_mode(
    document: &boardstudio_model::model::ProjectDoc,
    board_id: &str,
) -> ElectricalMode {
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

/// The board panel's latest action failure, attributed to the target it belongs to so a
/// navigated-away board never shows another board's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoardWiringFailure {
    pub(crate) target: BoardWiringModeFeedbackTarget,
    pub(crate) message: String,
}

#[derive(Clone, PartialEq)]
pub struct BoardWiringModeActions {
    pub identity: Option<BoardWiringModeIdentity>,
    pub editable: bool,
    pub(crate) mode: ElectricalMode,
    pub(crate) failure: Option<String>,
    pub on_change: EventHandler<BoardWiringModeEditRequest>,
}

/// The shared `ReplaceDocument` resolver for board-wiring edits: clone the
/// accepted document, apply `edit` (submit targets or retire reason), resolve
/// `Unchanged` when the clone still equals the accepted document (ADR-0005).
pub(super) fn replace_document_resolver(
    name: &'static str,
    edit: impl Fn(&AcceptedSnapshot, &mut ProjectDoc) -> Result<Vec<String>, String> + 'static,
) -> EditResolver {
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

/// The one scoped board-wiring action hook: a target-filtered failure signal, a
/// guarded edit admitted against the exact current wiring plan, and a
/// `runtime.commit` settled onto that target. `admit` builds the resolver for an
/// admitted request (`None` silently drops it, as before).
#[cfg(target_arch = "wasm32")]
pub(super) fn use_board_wiring_edit<Req: 'static>(
    guard: EditGuard,
    source: &Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
    label: &'static str,
    admit: impl Fn(&AcceptedSnapshot, &ElectricalPlan, Req) -> Option<EditResolver> + 'static,
    identity_of: impl Fn(&Req) -> BoardWiringModeIdentity + 'static,
) -> (
    Option<BoardWiringModeIdentity>,
    bool,
    Option<String>,
    EventHandler<Req>,
) {
    let failure = use_signal(|| None::<BoardWiringFailure>);
    let on_change = use_guarded_edit(guard, move |runtime, request: Req| {
        let identity = identity_of(&request);
        let current_resolution = resolution.read();
        let Some((snapshot, plan)) = current_wiring_edit(
            runtime,
            &identity,
            (guard.editor.scope_generation)(),
            &current_resolution,
        ) else {
            return;
        };
        let target = identity.feedback_target();
        if let Some(resolver) = admit(&snapshot, plan, request) {
            settle_board_commit(runtime, label, resolver, target, failure);
        }
    });
    let identity = source.as_ref().map(mode_identity);
    let editable = wiring_editable(guard, identity.as_ref(), resolution);
    let failure = BoardWiringFailure::for_identity(failure(), identity.as_ref());
    (identity, editable, failure, on_change)
}

#[cfg(target_arch = "wasm32")]
pub fn use_board_wiring_mode_edits(
    guard: EditGuard,
    source: Option<PcbWiringSource>,
    resolution: Signal<PcbWiringResolution>,
) -> BoardWiringModeActions {
    let mode = source.as_ref().map_or(ElectricalMode::Matrix, |source| {
        board_mode(&source.document, &source.ui_scope.board_id)
    });
    let (identity, editable, failure, on_change) = use_board_wiring_edit(
        guard,
        &source,
        resolution,
        "pcb-wiring-mode",
        |_, _, request| Some(mode_resolver(request)),
        |request: &BoardWiringModeEditRequest| request.identity.clone(),
    );
    BoardWiringModeActions {
        identity,
        editable,
        mode,
        failure,
        on_change,
    }
}

fn mode_resolver(request: BoardWiringModeEditRequest) -> EditResolver {
    EditResolver::new("pcb-wiring-mode", move |accepted: &AcceptedSnapshot| {
        let board_id = &request.identity.plan.scope.board_id;
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| &board.id == board_id)
        {
            return Resolution::Retire("The board was deleted.".into());
        }
        let current_mode = accepted
            .document
            .hardware
            .as_ref()
            .and_then(|hardware| {
                hardware
                    .boards
                    .iter()
                    .find(|configuration| configuration.board_id == *board_id)
            })
            .map_or(ElectricalMode::Matrix, |configuration| configuration.mode);
        if current_mode == request.mode {
            return Resolution::Unchanged;
        }
        Resolution::submit(
            vec![board_id.clone()],
            EditOperation::SetWiringMode {
                board_id: board_id.clone(),
                mode: request.mode,
            },
        )
    })
}

pub fn mode_identity(source: &PcbWiringSource) -> BoardWiringModeIdentity {
    BoardWiringModeIdentity {
        plan: source.identity.clone(),
        ui_scope: source.ui_scope.clone(),
        selected_part_id: source.active_part_id.clone(),
        scope_generation: source.scope_generation,
    }
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

impl BoardWiringFailure {
    /// The failure message when it belongs to the currently rendered board target.
    pub(super) fn for_identity(
        failure: Option<BoardWiringFailure>,
        identity: Option<&BoardWiringModeIdentity>,
    ) -> Option<String> {
        let target = identity.map(BoardWiringModeIdentity::feedback_target);
        failure
            .filter(|failure| Some(&failure.target) == target.as_ref())
            .map(|failure| failure.message)
    }
}

/// Commit one board-wiring resolver and settle its failure onto the board target.
pub(super) fn settle_board_commit(
    runtime: &std::rc::Rc<Runtime>,
    label: &'static str,
    resolver: EditResolver,
    target: BoardWiringModeFeedbackTarget,
    mut failure: Signal<Option<BoardWiringFailure>>,
) {
    failure.set(
        runtime
            .commit(label, resolver)
            .err()
            .map(|message| BoardWiringFailure { target, message }),
    );
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

pub(super) fn current_edit_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    workspace: Workspace,
    scope_generation: u64,
    instance_is_current: bool,
) -> Option<AcceptedSnapshot> {
    if workspace != Workspace::Pcb || !instance_is_current {
        return None;
    }
    matching_mode_snapshot(runtime, identity, scope_generation)
}

fn matching_mode_snapshot(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    generation: u64,
) -> Option<AcceptedSnapshot> {
    let (snapshot, source) = pcb_source(runtime, generation)?;
    (*identity == mode_identity(&source)).then_some(snapshot)
}

pub(super) fn current_wiring_edit<'a>(
    runtime: &Runtime,
    identity: &BoardWiringModeIdentity,
    generation: u64,
    resolution: &'a PcbWiringResolution,
) -> Option<(AcceptedSnapshot, &'a ElectricalPlan)> {
    let snapshot = matching_mode_snapshot(runtime, identity, generation)?;
    match resolution {
        PcbWiringResolution::Current {
            identity: current,
            plan,
        } if current == &identity.plan => Some((snapshot, plan)),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn wiring_editable(
    guard: EditGuard,
    identity: Option<&BoardWiringModeIdentity>,
    resolution: Signal<PcbWiringResolution>,
) -> bool {
    identity.is_some_and(|identity| {
        let runtime = guard.editor.runtime();
        guard.workspace_is_current()
            && guard.instance_is_current(&runtime)
            && current_wiring_edit(
                &runtime,
                identity,
                (guard.editor.scope_generation)(),
                &resolution.read(),
            )
            .is_some()
    })
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
