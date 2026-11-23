use super::*;
use boardstudio_application::SessionEpoch;
use std::{
    cell::Cell,
    future::Future,
    task::{Context, Waker},
};

type Probe = Rc<
    Cell<
        Option<(
            LayoutWorkspaceState,
            Signal<Option<ScopedTreeContext>>,
            Signal<Workspace>,
        )>,
    >,
>;

fn host() -> Element {
    let probe = use_context::<Probe>();
    let selected = use_signal(|| None);
    let workspace = use_signal(|| Workspace::Layout);
    let state = use_layout_workspace_state(selected, workspace, use_signal(|| None));
    probe.set(Some((state, selected, workspace)));
    // Subscribe the probe to the state exposed by the former browser readouts.
    let _ = (
        state.selection_kind()(),
        state.inspector_tab()(),
        selected(),
    );
    let _ = (
        state.pending_splay_origin_pick()(),
        state.pending_anchor_pick()(),
        state.transform_tool()(),
    );
    rsx! {}
}

fn flush(dom: &mut VirtualDom) {
    for _ in 0..8 {
        let work = {
            let mut pending = std::pin::pin!(dom.wait_for_work());
            pending
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
        };
        if work.is_pending() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("workspace state did not settle");
}

#[test]
fn handle_resets_layout_interactions_when_selection_or_owner_changes() {
    let probe = Probe::default();
    let mut dom = VirtualDom::new(host);
    dom.provide_root_context(probe.clone());
    dom.rebuild_in_place();
    flush(&mut dom);
    let (state, mut selected, mut workspace) = probe.get().unwrap();
    let owner = MatrixTransformInspectorOwner {
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "layout-state".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        matrix_id: "matrix".into(),
        context: TreeContext::Matrix {
            matrix_id: "matrix".into(),
        },
    };

    state.inspector_tab().set(LayoutInspectorTab::Relations);
    flush(&mut dom);
    assert_eq!(*state.inspector_tab().peek(), LayoutInspectorTab::Relations);
    selected.set(Some(ScopedTreeContext {
        scope: owner.scope.clone(),
        context: owner.context.clone(),
    }));
    state.set_selection_kind_from_context(&owner.context);
    flush(&mut dom);
    assert!(selected.peek().is_some());
    assert_eq!(*state.selection_kind().peek(), LayoutSelectionKind::Matrix);
    assert_eq!(
        *state.inspector_tab().peek(),
        LayoutInspectorTab::Properties
    );

    state.pending_splay_origin_pick().set(Some(owner.clone()));
    flush(&mut dom);
    assert_eq!(
        *state.pending_splay_origin_pick().peek(),
        Some(owner.clone())
    );
    state.pending_anchor_pick().set(Some(owner.clone()));
    flush(&mut dom);
    assert_eq!(*state.pending_anchor_pick().peek(), Some(owner));
    state.set_assembly_3d(true);
    flush(&mut dom);
    assert!(state.pending_splay_origin_pick().peek().is_none());
    assert!(state.pending_anchor_pick().peek().is_none());

    state
        .transform_tool()
        .set(Some(LayoutTransformTool::Stagger));
    flush(&mut dom);
    assert_eq!(
        *state.transform_tool().peek(),
        Some(LayoutTransformTool::Stagger)
    );
    workspace.set(Workspace::Keycaps);
    flush(&mut dom);
    assert!(state.transform_tool().peek().is_none());
}
