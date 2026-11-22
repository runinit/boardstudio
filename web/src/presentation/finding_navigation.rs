use super::*;

pub(crate) fn layout_findings_for_display(
    accepted: &[LayoutFinding],
    preview: Option<&[LayoutFinding]>,
) -> Vec<LayoutFinding> {
    let mut findings = accepted.to_vec();
    findings.extend(
        preview
            .into_iter()
            .flatten()
            .filter(|f| f.id.starts_with("preview:electrical:"))
            .cloned(),
    );
    findings
}

pub(crate) fn layout_finding_context(
    model: &ReadModel,
    target: &keycaps_fit::FindingNavigationTarget,
) -> Option<objects::TreeContext> {
    use keycaps_fit::FindingNavigationTarget as T;
    match target {
        T::Part { part_id, .. } => objects::component_context_for_finding_part(model, part_id),
        T::Matrix { matrix_id, .. } => Some(objects::TreeContext::Matrix {
            matrix_id: matrix_id.clone(),
        }),
        T::Outline { board_id } | T::Board { board_id } => Some(objects::TreeContext::Outline {
            board_id: board_id.clone(),
        }),
        T::Body { .. } | T::MechanicalLayer { .. } => None,
    }
}

pub(crate) fn findings_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    adapter: &SelectionAdapter,
    owner: &LayoutOwnerIdentity,
) -> bool {
    let model = runtime.model();
    owner.workspace != "Export"
        && current_layout_owner(runtime, workspace, adapter) == *owner
        && owner
            .scope
            .as_ref()
            .is_some_and(|s| active_board_scope_matches(&model, s))
}

pub(crate) struct LayoutFindingNavigationContext {
    pub(crate) runtime: Rc<Runtime>,
    pub(crate) adapter: SelectionAdapter,
    pub(crate) owner: LayoutOwnerIdentity,
    pub(crate) workspace: Signal<&'static str>,
    pub(crate) objects_open: Signal<bool>,
    pub(crate) inspect_open: Signal<bool>,
    pub(crate) findings_open: Signal<bool>,
    pub(super) inspector_settings: Signal<PanelSettings>,
    pub(super) keycaps_state: keycaps_workspace::KeycapsWorkspaceState,
    pub(crate) svg: Rc<RefCell<Option<SvgElement>>>,
    pub(crate) alive: Rc<Cell<bool>>,
    pub(crate) body_selection: Signal<Option<case_viewer::BodySelection>>,
    pub(crate) case_selection: case_viewer::CaseSelection,
    pub(crate) select_tree: EventHandler<objects::TreeSelectRequest>,
    pub(crate) resumed_after_board_navigation: bool,
}

pub(crate) fn perform_layout_finding_navigation(
    mut ctx: LayoutFindingNavigationContext,
    req: layout_findings::Request,
) {
    if !layout_owner_is_current(&ctx.runtime, ctx.workspace, &ctx.adapter, &ctx.owner)
        || !source_matches_layout_owner(&req.source, &ctx.owner, ctx.resumed_after_board_navigation)
    {
        return;
    }
    let Some(scope) = ctx.owner.scope.as_ref() else {
        return;
    };
    let model = ctx.runtime.model();
    if !layout_finding_is_live(&model, &req, &scope.board_id) {
        return;
    }
    let Some(snap) = model.accepted.as_ref() else {
        return;
    };
    let Some(tree_ctx) = layout_finding_context(&model, &req.target) else {
        return;
    };
    ctx.select_tree.call(objects::TreeSelectRequest {
        scope: scope.clone(),
        context: tree_ctx.clone(),
        mode: SelectionMode::Replace,
        outline_action: None,
    });
    let rm = ctx.runtime.model();
    let Some(rs) = rm.accepted.as_ref() else {
        return;
    };
    if (ctx.workspace)() != "Layout"
        || ctx.runtime.scope().as_ref() != Some(scope)
        || rs.token != req.source.token
        || rs.document.revision != req.source.revision
    {
        return;
    }
    let generation = (ctx.adapter.generation)();
    let nav_owner = keycaps_navigation::NavigationOwner {
        workspace: "Layout",
        scope: scope.clone(),
        generation,
        token: req.source.token,
        revision: req.source.revision,
        destination: keycaps_navigation::Destination::Layout(tree_ctx),
    };
    if let Some(surface) = ctx.svg.borrow().as_ref() {
        let r = surface.get_bounding_client_rect();
        let sz = (r.width(), r.height());
        if let (Some(base), Some(tb)) = (
            keycaps_fit::layout_camera_basis(&snap.document, &snap.scene, &scope.board_id, sz),
            keycaps_fit::finding_navigation_bounds(
                &snap.document,
                &snap.scene,
                &req.finding.id,
                &req.target,
            ),
        ) {
            let live = keycaps_navigation::LiveNavigationOwner {
                workspace: "Layout",
                scope: Some(scope.clone()),
                generation,
                token: Some(req.source.token),
                revision: Some(req.source.revision),
                destinations: vec![nav_owner.destination.clone()],
            };
            if let Some(cam) =
                keycaps_navigation::destination_camera_fit(&nav_owner, &live, base.bounds, tb, sz)
            {
                ctx.runtime.submit(Event::SetCamera {
                    operation_id: ctx.runtime.operation(),
                    center: cam.center,
                    zoom: cam.zoom,
                });
            }
        }
    }
    ctx.keycaps_state.focus_finding(
        scope.clone(),
        req.source.token,
        req.source.revision,
        req.finding.id,
    );
    ctx.findings_open.set(false);
    ctx.objects_open.set(false);
    ctx.inspect_open.set(true);
    pin_inspector_on_desktop(ctx.inspector_settings);
    let (rt, ad, ws, bs, cs) = (
        ctx.runtime.clone(),
        ctx.adapter.clone(),
        ctx.workspace,
        ctx.body_selection,
        ctx.case_selection,
    );
    focus_first_inspector_control_on_next_frame(ctx.alive, nav_owner, move || {
        current_keycaps_navigation_owner(ws, &rt, &ad, bs, cs)
    });
}

pub(crate) fn current_keycaps_navigation_owner(
    workspace: Signal<&'static str>,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    body_selection: Signal<Option<case_viewer::BodySelection>>,
    case_selection: case_viewer::CaseSelection,
) -> keycaps_navigation::LiveNavigationOwner {
    let (scope, model) = (runtime.scope(), runtime.model());
    let (token, revision) = model
        .accepted
        .as_ref()
        .map_or((None, None), |s| (Some(s.token), Some(s.document.revision)));
    let destinations = match (workspace(), scope.as_ref()) {
        ("Layout", Some(sc)) => adapter
            .selected_context
            .read()
            .as_ref()
            .filter(|s| &s.scope == sc)
            .map(|s| vec![keycaps_navigation::Destination::Layout(s.context.clone())])
            .unwrap_or_default(),
        ("Case", Some(sc)) => {
            let lid = case_selection.layer_id(sc);
            let bid = body_selection
                .read()
                .as_ref()
                .filter(|s| &s.scope == sc)
                .map(|b| b.body_id.clone());
            keycaps_navigation::active_case_destination(&lid, bid.as_deref())
                .into_iter()
                .collect()
        }
        _ => vec![],
    };
    keycaps_navigation::LiveNavigationOwner {
        workspace: workspace(),
        scope,
        generation: (adapter.generation)(),
        token,
        revision,
        destinations,
    }
}

fn points_bounds(pts: impl Iterator<Item = Vec2>) -> Option<(f64, f64, f64, f64)> {
    pts.fold(None, |b, p| {
        Some(b.map_or(
            (p.x, p.x, p.y, p.y),
            |(x0, x1, y0, y1): (f64, f64, f64, f64)| {
                (x0.min(p.x), x1.max(p.x), y0.min(p.y), y1.max(p.y))
            },
        ))
    })
}

pub(crate) fn fit_selected_bridge(runtime: &Rc<Runtime>, bridge_id: Option<&str>) {
    let Some(bridge_id) = bridge_id else {
        return;
    };
    let model = runtime.model();
    let Some(snap) = model.accepted.as_ref() else {
        return;
    };
    let Some(bridge) = snap
        .scene
        .board_outline_scenes
        .iter()
        .find(|s| s.board_id == model.active_board_id)
        .and_then(|s| s.bridges.iter().find(|b| b.id == bridge_id))
    else {
        return;
    };
    let Some((x0, x1, y0, y1)) = points_bounds(bridge.points.iter().copied()) else {
        return;
    };
    let board_bounds = points_bounds(
        snap.scene
            .board_contours
            .iter()
            .filter(|b| b.board_id == model.active_board_id)
            .flat_map(|b| &b.contours)
            .flat_map(|c| &c.points)
            .copied(),
    )
    .or_else(|| {
        let b = snap
            .document
            .boards
            .iter()
            .find(|b| b.id == model.active_board_id)?;
        points_bounds(
            snap.document
                .parts
                .iter()
                .filter(|p| b.part_ids.contains(&p.id))
                .map(|p| p.pose.at),
        )
    })
    .unwrap_or((x0, x1, y0, y1));
    let viewport_bounds = snap
        .document
        .boards
        .iter()
        .find(|b| b.id == model.active_board_id)
        .and_then(|b| {
            points_bounds(
                snap.document
                    .parts
                    .iter()
                    .filter(|p| b.part_ids.contains(&p.id))
                    .map(|p| {
                        snap.scene
                            .transforms
                            .iter()
                            .find(|t| t.id == p.id)
                            .map_or(p.pose.at, |t| t.pose.at)
                    }),
            )
        })
        .unwrap_or(board_bounds);
    let (bw, bh) = (
        (x1 - x0).max(bridge.width).max(1.0),
        (y1 - y0).max(bridge.width).max(1.0),
    );
    let (ow, oh) = (
        (board_bounds.1 - board_bounds.0).max(50.0) + 40.0,
        (board_bounds.3 - board_bounds.2).max(50.0) + 40.0,
    );
    let zoom = (0.72 * (ow / bw).min(oh / bh)).clamp(0.15, 8.0);
    runtime.submit(Event::SetCamera {
        operation_id: runtime.operation(),
        center: layout_camera::bridge_camera_offset(
            viewport_bounds,
            Vec2 {
                x: (x0 + x1) * 0.5,
                y: (y0 + y1) * 0.5,
            },
        ),
        zoom,
    });
}
