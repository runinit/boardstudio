//! Layout findings presentation state and return-focus lifecycle.

use crate::LayoutOwnerIdentity;
use crate::presentation::SelectionAdapter;
use crate::runtime::Runtime;
use boardstudio_application::ReadModel;
use boardstudio_web_ui_model::Workspace;
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

pub use crate::findings_return::{ReturnTarget, layout_finding_return_is_current};

#[derive(Clone, Copy)]
pub struct LayoutFindingsState {
    pub open: Signal<bool>,
    pub return_target: Signal<Option<ReturnTarget>>,
    pub return_focus: Signal<bool>,
    pub pending: Signal<Option<super::layout_findings::Request>>,
    pub resume: EventHandler<(super::layout_findings::Request, LayoutOwnerIdentity)>,
}

pub fn use_layout_findings_state(
    runtime: Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
) -> LayoutFindingsState {
    let open = use_signal(|| false);
    let return_target = use_signal(|| None::<ReturnTarget>);
    let return_focus = use_signal(|| false);
    let pending = use_signal(|| None::<super::layout_findings::Request>);
    let resume = use_hook(|| {
        EventHandler::new(|_: (super::layout_findings::Request, LayoutOwnerIdentity)| {})
    });

    use_effect(use_reactive(
        (&return_target(), &owner, &(adapter.selected_context)()),
        {
            let runtime = runtime.clone();
            let mut target_state = return_target;
            move |(target, current_owner, current_selection)| {
                let Some(target) = target.as_ref() else {
                    return;
                };
                if !layout_finding_return_is_current(
                    &runtime.model(),
                    &current_owner,
                    current_selection.as_ref(),
                    target,
                ) {
                    target_state.set(None);
                }
            }
        },
    ));
    use_effect(use_reactive((&return_focus(),), {
        let mut focus_state = return_focus;
        move |(pending,)| {
            if !pending {
                return;
            }
            focus_state.set(false);
            let _ = focus_layout_finding_return_destination();
        }
    }));
    use_effect(use_reactive((&pending(), &owner, &workspace()), {
        let mut pending = pending;
        move |(request, owner, current_workspace)| {
            let Some(request) = request else {
                return;
            };
            if current_workspace != Workspace::Layout {
                if current_workspace != request.source.workspace {
                    pending.set(None);
                }
                return;
            }
            if !source_matches_layout_owner(&request.source, &owner, true) {
                pending.set(None);
                return;
            }
            let Some(scope) = owner.scope.as_ref() else {
                pending.set(None);
                return;
            };
            let target_board =
                boardstudio_web_ui_model::finding_navigation::target_board_id(&request.target);
            if scope.board_id != target_board {
                if scope.board_id != request.source.scope.board_id {
                    pending.set(None);
                }
                return;
            }
            if request.source.workspace == Workspace::Layout
                && scope.board_id == request.source.scope.board_id
            {
                return;
            }
            if !layout_finding_is_live(&runtime.model(), &request, target_board) {
                pending.set(None);
                return;
            }
            pending.set(None);
            resume.call((request, owner));
        }
    }));

    LayoutFindingsState {
        open,
        return_target,
        return_focus,
        pending,
        resume,
    }
}

pub fn source_matches_layout_owner(
    source: &super::layout_findings::Source,
    owner: &LayoutOwnerIdentity,
    allow_board_hop: bool,
) -> bool {
    owner.workspace
        == if allow_board_hop {
            Workspace::Layout
        } else {
            source.workspace
        }
        && owner.token == Some(source.token)
        && owner.revision == Some(source.revision)
        && owner.scope.as_ref().is_some_and(|scope| {
            scope.session_epoch == source.scope.session_epoch
                && scope.document_id == source.scope.document_id
                && (allow_board_hop || scope.board_id == source.scope.board_id)
        })
        && (allow_board_hop || owner.generation == source.generation)
}

pub fn layout_finding_is_live(
    model: &ReadModel,
    request: &super::layout_findings::Request,
    destination_board: &str,
) -> bool {
    let Some(snapshot) = model.accepted.as_ref() else {
        return false;
    };
    if !boardstudio_web_ui_model::finding_navigation::presented_findings(
        &snapshot.scene.findings,
        &snapshot.document,
    )
    .iter()
    .any(|finding| finding == &request.finding)
        || boardstudio_web_ui_model::finding_navigation::finding_navigation_target(
            &request.finding,
            &snapshot.document,
        )
        .as_ref()
            != Some(&request.target)
        || boardstudio_web_ui_model::finding_navigation::target_board_id(&request.target)
            != destination_board
    {
        return false;
    }
    snapshot
        .document
        .boards
        .iter()
        .any(|board| board.id == destination_board)
        && crate::presentation::layout_findings::target_has_live_layout_destination(
            &request.target,
            &snapshot.document,
        )
}

pub fn focus_layout_finding_return_destination() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    for selector in [
        "#m1-inspector-panel-content .layout-inspector-tabs [role='tab'][aria-selected='true']",
        "#m1-inspector-panel-content [aria-label='Board Inspector'] :is(input, select, button):not(:disabled)",
        "#m1-inspector-panel-content [aria-label='Board outline'] :is(input, select, button):not(:disabled)",
    ] {
        if let Some(element) = document
            .query_selector(selector)
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlElement>().ok())
        {
            let _ = element.focus();
            return true;
        }
    }
    if let Some(element) = document
        .query_selector(
            "#m1-inspector-panel-content > div, #m1-inspector-panel-content [aria-label='Board Inspector'], #m1-inspector-panel-content [aria-label='Board outline']",
        )
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
    {
        let _ = element.set_attribute("tabindex", "-1");
        let _ = element.focus();
        return true;
    }
    false
}
