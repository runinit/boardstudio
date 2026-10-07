//! Layout findings presentation state and return-focus lifecycle.

use crate::presentation::SelectionAdapter;
use crate::runtime::Runtime;
use crate::{LayoutOwnerIdentity, objects};
use boardstudio_application::ReadModel;
use boardstudio_web_ui_model::selection;
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnTarget {
    pub owner: LayoutOwnerIdentity,
    pub selection: objects::ScopedTreeContext,
    pub destination: objects::TreeContext,
}

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
    workspace: Signal<&'static str>,
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
        let resume = resume;
        move |(request, owner, current_workspace)| {
            let Some(request) = request else {
                return;
            };
            if current_workspace != "Layout" {
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
            let target_board = crate::presentation::keycaps_fit::target_board_id(&request.target);
            if scope.board_id != target_board {
                if scope.board_id != request.source.scope.board_id {
                    pending.set(None);
                }
                return;
            }
            if request.source.workspace == "Layout"
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
            "Layout"
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
    if !crate::presentation::keycaps_fit::presented_findings(
        &snapshot.scene.findings,
        &snapshot.document,
    )
    .iter()
    .any(|finding| finding == &request.finding)
        || crate::presentation::keycaps_fit::finding_navigation_target(
            &request.finding,
            &snapshot.document,
        )
        .as_ref()
            != Some(&request.target)
        || crate::presentation::keycaps_fit::target_board_id(&request.target) != destination_board
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

pub fn layout_finding_return_is_current(
    model: &ReadModel,
    owner: &LayoutOwnerIdentity,
    current_selection: Option<&objects::ScopedTreeContext>,
    target: &ReturnTarget,
) -> bool {
    if owner != &target.owner || owner.workspace != "Layout" {
        return false;
    }
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let saved_selection_is_live =
        selection::context_is_current(model, scope, &target.selection.context)
            && match &target.selection.context {
                objects::TreeContext::Component {
                    part_id: Some(_), ..
                } => selection::resolve_context(model, &target.selection.context)
                    .is_some_and(|part_ids| !part_ids.is_empty()),
                _ => true,
            };
    target.selection.scope == *scope
        && saved_selection_is_live
        && current_selection.is_some_and(|selected| {
            selected.scope == *scope
                && selected.context == target.destination
                && selection::context_is_current(model, scope, &selected.context)
        })
}

pub fn focus_layout_finding_return_destination() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    for selector in [
        "#m1-inspector-panel-content .m1-layout-component-tabs [role='tab'][aria-selected='true']",
        "#m1-inspector-panel-content .m1-board-inspector :is(input, select, button):not(:disabled)",
        "#m1-inspector-panel-content .m1-outline-inspector :is(input, select, button):not(:disabled)",
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
            "#m1-inspector-panel-content .m1-inspector-body, #m1-inspector-panel-content .m1-selected-context, #m1-inspector-panel-content .m1-board-inspector, #m1-inspector-panel-content .m1-outline-inspector",
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

#[cfg(test)]
mod layout_finding_return_regression_tests {
    use super::ReturnTarget as LayoutFindingReturnTarget;
    use super::*;
    use boardstudio_application::{AcceptedSnapshot, Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        Board, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Readiness, SceneDelta, Side, Vec2,
    };
    use std::sync::Arc;
    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_test::wasm_bindgen_test;

    fn fixture() -> (ReadModel, Scope, objects::ScopedTreeContext) {
        let scope = Scope {
            session_epoch: SessionEpoch(5),
            document_id: "finding-return-doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let mut document = ProjectDoc::empty("finding-return-doc", "Finding return fixture");
        document.revision = 9;
        document.boards.push(Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec!["left-U1".into()],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "controller".into(),
            name: "Controller".into(),
            kind: PartKind::Controller,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![],
            pads: vec![],
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: "left-U1".into(),
            definition_id: "controller".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        let model = ReadModel {
            accepted: Some(AcceptedSnapshot {
                token: SnapshotToken(13),
                session_epoch: scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(SceneDelta {
                    module_scenes: vec![],
                    revision: 9,
                    transaction_id: "accepted-fixture".into(),
                    changed_ids: vec![],
                    transforms: vec![],
                    matrix_scenes: vec![],
                    contours: vec![],
                    board_contours: vec![],
                    board_readiness: vec![],
                    board_outline_scenes: vec![],
                    finding_markers: vec![],
                    findings: vec![],
                    readiness: Readiness {
                        layout: true,
                        outline: true,
                        pcb: true,
                        case_ready: false,
                    },
                }),
            }),
            active_board_id: scope.board_id.clone(),
            selected_part_ids: vec!["left-U1".into()],
            ..ReadModel::default()
        };
        let selection = objects::ScopedTreeContext {
            scope: scope.clone(),
            context: objects::TreeContext::Component {
                part_id: Some("left-U1".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            },
        };
        (model, scope, selection)
    }

    #[wasm_bindgen_test]
    fn outline_return_requires_the_same_live_owner_and_saved_selection() {
        let (model, scope, selection) = fixture();
        let owner = LayoutOwnerIdentity {
            scope: Some(scope.clone()),
            token: Some(SnapshotToken(13)),
            revision: Some(9),
            generation: 2,
            workspace: "Layout",
        };
        let destination = objects::TreeContext::Outline {
            board_id: scope.board_id.clone(),
        };
        let target = LayoutFindingReturnTarget {
            owner: owner.clone(),
            selection: selection.clone(),
            destination: destination.clone(),
        };
        let outline_selection = objects::ScopedTreeContext {
            scope,
            context: destination,
        };

        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&outline_selection),
            &target,
        ));
        let mut non_layout_owner = owner.clone();
        non_layout_owner.workspace = "PCB";
        let non_layout_target = LayoutFindingReturnTarget {
            owner: non_layout_owner.clone(),
            ..target.clone()
        };
        assert!(!layout_finding_return_is_current(
            &model,
            &non_layout_owner,
            Some(&outline_selection),
            &non_layout_target,
        ));

        let empty_outline_selection = objects::ScopedTreeContext {
            scope: outline_selection.scope.clone(),
            context: outline_selection.context.clone(),
        };
        let empty_outline_target = LayoutFindingReturnTarget {
            selection: empty_outline_selection.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&empty_outline_selection),
            &empty_outline_target,
        ));

        let component_target = LayoutFindingReturnTarget {
            destination: selection.context.clone(),
            ..target.clone()
        };
        assert!(layout_finding_return_is_current(
            &model,
            &owner,
            Some(&selection),
            &component_target,
        ));

        let mut changed_owner = owner.clone();
        changed_owner.revision = Some(10);
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_scope_owner = owner.clone();
        changed_scope_owner
            .scope
            .as_mut()
            .expect("fixture scope")
            .board_id = "other-board".into();
        assert!(!layout_finding_return_is_current(
            &model,
            &changed_scope_owner,
            Some(&outline_selection),
            &target,
        ));

        let mut changed_model = model;
        Arc::make_mut(
            &mut changed_model
                .accepted
                .as_mut()
                .expect("fixture snapshot")
                .document,
        )
        .boards[0]
            .part_ids
            .clear();
        assert!(!layout_finding_return_is_current(
            &changed_model,
            &owner,
            Some(&outline_selection),
            &target,
        ));
    }

    #[wasm_bindgen_test]
    fn mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope() {
        let (model, scope, selection) = fixture();
        let owner = LayoutOwnerIdentity {
            scope: Some(scope.clone()),
            token: Some(SnapshotToken(13)),
            revision: Some(9),
            generation: 2,
            workspace: "Layout",
        };
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document
            .create_element("div")
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        root.set_id("m1-inspector-panel-content");
        document.body().unwrap().append_child(&root).unwrap();

        for (inspector, destination, expected_focus) in [
            (
                r#"<button id="back">Back to selection</button><section class="m1-board-inspector"><input id="board-name"></section>"#,
                objects::TreeContext::Board {
                    board_id: scope.board_id.clone(),
                },
                "board-name",
            ),
            (
                r#"<button id="back">Back to selection</button><section class="m1-outline-inspector"><select id="outline-version"><option>Generated</option></select></section>"#,
                objects::TreeContext::Outline {
                    board_id: scope.board_id.clone(),
                },
                "outline-version",
            ),
        ] {
            root.set_inner_html(inspector);
            let back = document
                .get_element_by_id("back")
                .unwrap()
                .dyn_into::<HtmlElement>()
                .unwrap();
            let target = LayoutFindingReturnTarget {
                owner: owner.clone(),
                selection: selection.clone(),
                destination: destination.clone(),
            };
            let current_selection = objects::ScopedTreeContext {
                scope: scope.clone(),
                context: destination,
            };
            let model = model.clone();
            let owner = owner.clone();
            let back_key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == "Enter"
                    && layout_finding_return_is_current(
                        &model,
                        &owner,
                        Some(&current_selection),
                        &target,
                    )
                {
                    event.prevent_default();
                    let _ = focus_layout_finding_return_destination();
                }
            }) as Box<dyn FnMut(_)>);
            back.add_event_listener_with_callback("keydown", back_key.as_ref().unchecked_ref())
                .unwrap();
            back.focus().unwrap();
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Enter");
            init.set_bubbles(true);
            back.dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                document
                    .active_element()
                    .and_then(|element| element.get_attribute("id")),
                Some(expected_focus.to_owned()),
                "Enter returns focus into {expected_focus}"
            );
            back.remove_event_listener_with_callback("keydown", back_key.as_ref().unchecked_ref())
                .unwrap();
        }

        root.set_inner_html(
            r#"<button id="back">Back to selection</button><section class="m1-board-inspector"><input id="board-name"></section>"#,
        );
        let back = document
            .get_element_by_id("back")
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap();
        let target = LayoutFindingReturnTarget {
            owner: owner.clone(),
            selection,
            destination: objects::TreeContext::Board {
                board_id: scope.board_id.clone(),
            },
        };
        let current_selection = objects::ScopedTreeContext {
            scope,
            context: target.destination.clone(),
        };
        let mut stale_owner = owner.clone();
        stale_owner.scope.as_mut().expect("fixture scope").board_id = "other-board".into();
        let model = model.clone();
        let stale_key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Enter"
                && layout_finding_return_is_current(
                    &model,
                    &stale_owner,
                    Some(&current_selection),
                    &target,
                )
            {
                event.prevent_default();
                let _ = focus_layout_finding_return_destination();
            }
        }) as Box<dyn FnMut(_)>);
        back.add_event_listener_with_callback("keydown", stale_key.as_ref().unchecked_ref())
            .unwrap();
        back.focus().unwrap();
        let init = web_sys::KeyboardEventInit::new();
        init.set_key("Enter");
        back.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document
                .active_element()
                .and_then(|element| element.get_attribute("id")),
            Some("back".to_owned()),
            "a stale scope cannot move focus into another board's Inspector"
        );
        back.remove_event_listener_with_callback("keydown", stale_key.as_ref().unchecked_ref())
            .unwrap();
        document.body().unwrap().remove_child(&root).unwrap();
    }
}
