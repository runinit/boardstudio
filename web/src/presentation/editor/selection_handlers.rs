use super::super::*;
use boardstudio_web_ui_model::Workspace;

pub(in crate::presentation) struct SelectionKindInput {
    pub runtime: Rc<Runtime>,
    pub adapter: SelectionAdapter,
    pub owner: LayoutOwnerIdentity,
    pub tree_cell_anchor: Rc<RefCell<Option<OwnedTreeCellAnchor>>>,
    pub objects_open: Signal<bool>,
    pub inspect_open: Signal<bool>,
    pub selection_kind: Signal<objects::LayoutSelectionKind>,
    pub workspace: Signal<Workspace>,
    pub inspector_panel_settings: Signal<PanelSettings>,
}

pub(in crate::presentation) fn on_selection_kind_handler(
    input: SelectionKindInput,
    expected_workspace: Workspace,
) -> EventHandler<objects::LayoutSelectionKind> {
    let SelectionKindInput {
        runtime,
        adapter,
        owner,
        tree_cell_anchor,
        mut objects_open,
        mut inspect_open,
        mut selection_kind,
        workspace,
        inspector_panel_settings,
    } = input;
    EventHandler::new(move |kind: objects::LayoutSelectionKind| {
        if !selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            return;
        }
        selection_kind.set(kind);
        let model = runtime.model();
        let Some(scope) = owner.scope.as_ref() else {
            tree_cell_anchor.borrow_mut().take();
            return;
        };
        let Some(selected) = adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == *scope)
        else {
            tree_cell_anchor.borrow_mut().take();
            return;
        };
        if !selection::context_is_current(&model, scope, &selected.context) {
            tree_cell_anchor.borrow_mut().take();
            return;
        }
        let retained = tree_cell_anchor_for_owner(&tree_cell_anchor, &owner);
        let Some(projection) =
            objects::context_for_selection_kind(&model, &selected.context, kind, retained.as_ref())
        else {
            return;
        };
        if !selection::context_is_current(&model, scope, &projection.context) {
            return;
        }
        update_tree_cell_anchor(&tree_cell_anchor, &owner, &model, Some(&projection.context));
        selection::submit_context(
            &runtime,
            &adapter,
            objects::TreeSelectRequest {
                scope: scope.clone(),
                context: projection.context,
                mode: SelectionMode::Replace,
                outline_action: None,
            }
            .into(),
        );
        if selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    })
}

pub(in crate::presentation) fn on_snap_intent_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut snap_settings: Signal<objects::LayoutSnapSettings>,
    workspace: Signal<Workspace>,
    expected_workspace: Workspace,
) -> EventHandler<objects::LayoutSnapIntent> {
    EventHandler::new(move |intent: objects::LayoutSnapIntent| {
        if !selection::workspace_owner_is_current(
            &runtime,
            workspace,
            &adapter,
            &owner,
            expected_workspace,
        ) {
            return;
        }
        let mut settings = snap_settings.read().clone();
        match intent {
            objects::LayoutSnapIntent::Fraction(value)
                if [0.0, 0.125, 0.25, 0.5, 1.0, -1.0, -0.5, -0.1].contains(&value) =>
            {
                settings.snap_fraction = value;
            }
            objects::LayoutSnapIntent::GeometrySnap(enabled) => {
                settings.geometry_snap = enabled;
            }
            objects::LayoutSnapIntent::GapSnap(enabled) => {
                settings.gap_snap = enabled;
            }
            objects::LayoutSnapIntent::GapOverride(value) => {
                settings.gap_override = value;
            }
            objects::LayoutSnapIntent::Fraction(_) => return,
        }
        snap_settings.set(settings);
    })
}
