use super::*;

/// Keymap and Keycaps only change views; their shared 3D viewer has no Layout
/// placement or transform interaction to cancel. Keep these requests bound to
/// the exact workspace, scope, accepted snapshot, and selection generation.
pub(crate) fn design_consumer_view_mode_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    is_assembly_3d: Signal<bool>,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if owner.workspace == "Layout"
            || current_layout_owner(&runtime, workspace, &adapter) != owner
            || owner.workspace != workspace()
        {
            return;
        }
        let model = runtime.model();
        if !owner.scope.as_ref().is_some_and(|scope| {
            active_board_scope_matches(&model, scope) && runtime.scope().as_ref() == Some(scope)
        }) {
            return;
        }
        let mut is_assembly_3d = is_assembly_3d;
        is_assembly_3d.set(assembly_3d);
    })
}

pub(crate) fn matrix_snap_parameters(
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    adapter: &SelectionAdapter,
    retained_cell: Option<&objects::TreeCellAnchor>,
) -> (Option<Vec2>, Option<f64>) {
    let Some(snapshot) = model.accepted.as_ref() else {
        return (None, None);
    };
    let context = adapter
        .selected_context
        .read()
        .clone()
        .filter(|selected| {
            selected.scope == *scope
                && selection::context_is_current(model, scope, &selected.context)
        })
        .map(|selected| selected.context)
        .or_else(|| {
            model
                .selected_part_ids
                .first()
                .and_then(|part_id| objects::context_for_part(model, part_id))
        });
    let Some(context) = context else {
        return (None, None);
    };
    let matrix_id = match &context {
        objects::TreeContext::Matrix { matrix_id }
        | objects::TreeContext::Row { matrix_id, .. }
        | objects::TreeContext::Column { matrix_id, .. }
        | objects::TreeContext::Key { matrix_id, .. }
        | objects::TreeContext::Component {
            matrix_id: Some(matrix_id),
            ..
        } => matrix_id,
        objects::TreeContext::Component {
            part_id: Some(_),
            matrix_id: None,
            ..
        } => {
            let Some(cell) = retained_cell.filter(|cell| {
                objects::context_for_selection_kind(
                    model,
                    &context,
                    objects::LayoutSelectionKind::Key,
                    Some(cell),
                )
                .is_some()
            }) else {
                return (None, None);
            };
            &cell.matrix_id
        }
        objects::TreeContext::Outline { .. }
        | objects::TreeContext::OutlineVersion { .. }
        | objects::TreeContext::Bridge { .. }
        | objects::TreeContext::MountedModule { .. }
        | objects::TreeContext::Board { .. }
        | objects::TreeContext::LayoutGroup { .. }
        | objects::TreeContext::Component { .. } => return (None, None),
    };
    let Some(matrix) = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == *matrix_id)
    else {
        return (None, None);
    };
    (Some(matrix.pitch), matrix.edge_gap.map(|gap| gap.x))
}

pub(crate) fn pin_inspector_on_desktop(mut settings: Signal<PanelSettings>) {
    if is_compact_viewport() {
        return;
    }
    let mut current = settings();
    if current.mode != PanelMode::Pinned {
        current.mode = PanelMode::Pinned;
        settings.set(current);
    }
}

pub(crate) fn is_compact_viewport() -> bool {
    web_sys::window()
        .and_then(|window| window.match_media("(width < 980px)").ok().flatten())
        .is_some_and(|query| query.matches())
}

pub(crate) fn focus_first_inspector_control_on_next_frame(
    alive: Rc<Cell<bool>>,
    owner: keycaps_navigation::NavigationOwner,
    current_owner: impl Fn() -> keycaps_navigation::LiveNavigationOwner + 'static,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    keycaps_navigation::queue_owner_focus(
        alive,
        owner,
        current_owner,
        || {
            if let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| {
                    if let Some(body) = document
                        .query_selector("#m1-inspector-panel-content > div")
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<HtmlElement>().ok())
                    {
                        body.set_scroll_top(0);
                    }
                    document
                        .query_selector(
                            "#m1-inspector-panel-content :is(button, input):not(:disabled)",
                        )
                        .ok()
                        .flatten()
                })
                .and_then(|element| element.dyn_into::<HtmlElement>().ok())
            {
                let _ = element.focus();
            }
        },
        move |task| {
            let callback = Closure::once_into_js(task);
            let _ = window.request_animation_frame(callback.unchecked_ref());
        },
    );
}
