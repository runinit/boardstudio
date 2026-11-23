use super::*;
use boardstudio_web_ui_model::Workspace;

pub(crate) fn design_consumer_view_mode_handler(
    runtime: Rc<Runtime>,
    workspace: Signal<Workspace>,
    adapter: SelectionAdapter,
    owner: LayoutOwnerIdentity,
    mut is_assembly_3d: Signal<bool>,
) -> EventHandler<bool> {
    EventHandler::new(move |assembly_3d| {
        if owner.workspace != Workspace::Layout
            && current_layout_owner(&runtime, workspace, &adapter) == owner
            && owner.workspace == workspace()
            && owner.scope.as_ref().is_some_and(|s| {
                active_board_scope_matches(&runtime.model(), s)
                    && runtime.scope().as_ref() == Some(s)
            })
        {
            is_assembly_3d.set(assembly_3d);
        }
    })
}

pub(crate) fn matrix_snap_parameters(
    model: &boardstudio_application::ReadModel,
    scope: &Scope,
    adapter: &SelectionAdapter,
    retained_cell: Option<&objects::TreeCellAnchor>,
) -> (Option<Vec2>, Option<f64>) {
    let Some(snap) = model.accepted.as_ref() else {
        return (None, None);
    };
    let ctx = adapter
        .selected_context
        .read()
        .clone()
        .filter(|s| s.scope == *scope && selection::context_is_current(model, scope, &s.context))
        .map(|s| s.context)
        .or_else(|| {
            model
                .selected_part_ids
                .first()
                .and_then(|id| objects::context_for_part(model, id))
        });
    let Some(ctx) = ctx else {
        return (None, None);
    };
    let matrix_id = match &ctx {
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
            let Some(cell) = retained_cell.filter(|c| {
                objects::context_for_selection_kind(
                    model,
                    &ctx,
                    objects::LayoutSelectionKind::Key,
                    Some(c),
                )
                .is_some()
            }) else {
                return (None, None);
            };
            &cell.matrix_id
        }
        _ => return (None, None),
    };
    snap.document
        .matrices
        .iter()
        .find(|m| m.id == *matrix_id)
        .map_or((None, None), |m| (Some(m.pitch), m.edge_gap.map(|g| g.x)))
}

pub(crate) fn pin_inspector_on_desktop(mut settings: Signal<PanelSettings>) {
    if !is_compact_viewport() {
        let mut cur = settings();
        if cur.mode != PanelMode::Pinned {
            cur.mode = PanelMode::Pinned;
            settings.set(cur);
        }
    }
}

pub(crate) fn is_compact_viewport() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(width < 980px)").ok().flatten())
        .is_some_and(|q| q.matches())
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
            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                if let Some(body) = doc
                    .query_selector("#m1-inspector-panel-content > div")
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<HtmlElement>().ok())
                {
                    body.set_scroll_top(0);
                }
                if let Some(el) = doc
                    .query_selector("#m1-inspector-panel-content :is(button, input):not(:disabled)")
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<HtmlElement>().ok())
                {
                    let _ = el.focus();
                }
            }
        },
        move |task| {
            let cb = Closure::once_into_js(task);
            let _ = window.request_animation_frame(cb.unchecked_ref());
        },
    );
}
