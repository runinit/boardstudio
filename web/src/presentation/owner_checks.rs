use super::*;

pub(crate) fn case_setup_context_is_current(
    identity: &pcb_physical_setup::OwnerIdentity,
    strict: bool,
    workspace: &str,
    scope: Option<&Scope>,
) -> bool {
    identity.context == pcb_physical_setup::OwnerContext::CaseInspector
        && workspace == "Case"
        && scope.is_some_and(|scope| {
            scope.session_epoch == identity.session_epoch
                && scope.document_id == identity.document_id
                && scope.board_id == identity.board_id
                && (!strict || scope.instance_id == identity.instance_id)
        })
}

pub(crate) fn current_case_layer_ids(
    runtime: &Runtime,
    scope: &Scope,
    snapshot: &AcceptedSnapshot,
) -> Option<Rc<[String]>> {
    let scene = runtime.cad_scene()?;
    if !scene.exact
        || scene.scope != *scope
        || scene.token != snapshot.token
        || scene.prepared.revision != snapshot.document.revision
    {
        return None;
    }
    Some(Rc::from(
        scene
            .mechanical
            .as_ref()
            .map(|assembly| {
                assembly
                    .stack
                    .iter()
                    .map(|layer| layer.id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    ))
}

pub(crate) fn pcb_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if current_layout_owner(runtime, workspace, adapter) != *owner || owner.workspace != "PCB" {
        return false;
    }
    let model = runtime.model();
    owner
        .scope
        .as_ref()
        .is_some_and(|scope| active_board_scope_matches(&model, scope))
}

pub(crate) fn pcb_add_layout_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    if current_layout_owner(runtime, workspace, adapter) != *owner || owner.workspace != "PCB" {
        return false;
    }
    let model = runtime.model();
    let Some(scope) = owner.scope.as_ref() else {
        return false;
    };
    let (Some(token), Some(revision)) = (owner.token, owner.revision) else {
        return false;
    };
    active_board_scope_matches(&model, scope)
        && model.lifecycle == Lifecycle::Ready
        && model.durability == (Durability::Saved { revision })
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.token == token && snapshot.document.revision == revision
        })
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .boards
                .iter()
                .any(|board| board.id == scope.board_id)
        })
}

pub(crate) fn pcb_add_layout_open_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    assembly_3d: Signal<bool>,
    on_open: EventHandler<()>,
) -> EventHandler<()> {
    let mut workspace = workspace;
    let mut assembly_3d = assembly_3d;
    EventHandler::new(move |_| {
        if !pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner) {
            return;
        }
        workspace.set("Layout");
        assembly_3d.set(false);
        on_open.call(());
    })
}

pub(crate) fn pcb_add_outline_select_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    assembly_3d: Signal<bool>,
    on_select: EventHandler<objects::TreeSelectRequest>,
) -> EventHandler<objects::TreeSelectRequest> {
    let mut workspace = workspace;
    let mut assembly_3d = assembly_3d;
    EventHandler::new(move |request: objects::TreeSelectRequest| {
        let objects::TreeContext::Outline { board_id } = &request.context else {
            on_select.call(request);
            return;
        };
        let Some(owner_scope) = owner.scope.as_ref() else {
            return;
        };
        if board_id != &owner_scope.board_id
            || request.scope != *owner_scope
            || !pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        {
            return;
        }
        workspace.set("Layout");
        assembly_3d.set(false);
        on_select.call(request);
    })
}

pub(crate) fn canvas_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    match owner.workspace {
        "Layout" => layout_owner_is_current(runtime, workspace, adapter, owner),
        "PCB" => pcb_owner_is_current(runtime, workspace, adapter, owner),
        _ => false,
    }
}

#[cfg(test)]
mod physical_setup_owner_tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use pcb_physical_setup::{OwnerContext, OwnerIdentity};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn case_transport_owner_does_not_require_a_part_tree_selection() {
        let scope = Scope {
            session_epoch: SessionEpoch(7),
            document_id: "sofle".into(),
            board_id: "right-pcb".into(),
            instance_id: Some("right-half".into()),
        };
        let identity = OwnerIdentity {
            context: OwnerContext::CaseInspector,
            session_epoch: scope.session_epoch,
            document_id: scope.document_id.clone(),
            board_id: scope.board_id.clone(),
            instance_id: scope.instance_id.clone(),
            token: SnapshotToken(23),
            revision: 21,
            generation: 4,
            scope_transition: None,
        };
        assert!(
            case_setup_context_is_current(&identity, true, "Case", Some(&scope)),
            "physical setup belongs to the active Case assembly scope even when no part is selected"
        );
    }
}
