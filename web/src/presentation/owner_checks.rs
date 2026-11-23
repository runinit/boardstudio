use super::*;
use boardstudio_web_ui_model::Workspace;

pub(crate) fn case_setup_context_is_current(
    id: &pcb_physical_setup::OwnerIdentity,
    strict: bool,
    workspace: Workspace,
    scope: Option<&Scope>,
) -> bool {
    id.context == pcb_physical_setup::OwnerContext::CaseInspector
        && workspace == Workspace::Case
        && scope.is_some_and(|s| {
            s.session_epoch == id.session_epoch
                && s.document_id == id.document_id
                && s.board_id == id.board_id
                && (!strict || s.instance_id == id.instance_id)
        })
}

pub(crate) fn current_case_layer_ids(
    runtime: &Runtime,
    scope: &Scope,
    snap: &AcceptedSnapshot,
) -> Option<Rc<[String]>> {
    let scene = runtime.cad_scene()?;
    (scene.exact
        && scene.scope == *scope
        && scene.token == snap.token
        && scene.prepared.revision == snap.document.revision)
        .then(|| {
            Rc::from(
                scene
                    .mechanical
                    .as_ref()
                    .map_or_else(Vec::new, |a| a.stack.iter().map(|l| l.id.clone()).collect()),
            )
        })
}

pub(crate) fn pcb_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    selection::workspace_owner_is_current(runtime, workspace, adapter, owner, Workspace::Pcb)
}

pub(crate) fn pcb_add_layout_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if !pcb_owner_is_current(runtime, workspace, adapter, owner) {
        return false;
    }
    let (Some(scope), Some(token), Some(rev)) = (owner.scope.as_ref(), owner.token, owner.revision)
    else {
        return false;
    };
    let m = runtime.model();
    m.lifecycle == Lifecycle::Ready
        && matches!(
            m.durability,
            Durability::Saved { revision } | Durability::Saving { revision } if revision == rev
        )
        && m.display_preview.is_none()
        && m.gesture.is_none()
        && m.accepted.as_ref().is_some_and(|s| {
            s.token == token
                && s.document.revision == rev
                && s.document.boards.iter().any(|b| b.id == scope.board_id)
        })
}

pub(crate) fn pcb_add_layout_open_handler(
    runtime: Rc<Runtime>,
    mut workspace: Signal<Workspace>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut assembly_3d: Signal<bool>,
    on_open: EventHandler<()>,
) -> EventHandler<()> {
    EventHandler::new(move |_| {
        if pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            workspace.set(Workspace::Layout);
            assembly_3d.set(false);
            on_open.call(());
        }
    })
}

pub(crate) fn pcb_add_outline_select_handler(
    runtime: Rc<Runtime>,
    mut workspace: Signal<Workspace>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut assembly_3d: Signal<bool>,
    on_select: EventHandler<objects::TreeSelectRequest>,
) -> EventHandler<objects::TreeSelectRequest> {
    EventHandler::new(move |req: objects::TreeSelectRequest| {
        let objects::TreeContext::Outline { board_id } = &req.context else {
            on_select.call(req);
            return;
        };
        if owner
            .scope
            .as_ref()
            .is_some_and(|s| board_id == &s.board_id && req.scope == *s)
            && pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        {
            workspace.set(Workspace::Layout);
            assembly_3d.set(false);
            on_select.call(req);
        }
    })
}

pub(crate) fn canvas_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<Workspace>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    match owner.workspace {
        Workspace::Layout => layout_owner_is_current(runtime, workspace, adapter, owner),
        Workspace::Pcb => pcb_owner_is_current(runtime, workspace, adapter, owner),
        _ => false,
    }
}
