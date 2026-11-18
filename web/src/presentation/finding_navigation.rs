use super::*;

/// Keep accepted diagnostics stable while exposing only electrical loss notices from a preview.
pub(crate) fn layout_findings_for_display(
    accepted: &[LayoutFinding],
    preview: Option<&[LayoutFinding]>,
) -> Vec<LayoutFinding> {
    let mut findings = accepted.to_vec();
    findings.extend(
        preview
            .into_iter()
            .flatten()
            .filter(|finding| finding.id.starts_with("preview:electrical:"))
            .cloned(),
    );
    findings
}

#[cfg(test)]
mod layout_findings_for_display_tests {
    use super::*;
    use boardstudio_core::model::{Scope as FindingScope, Severity as FindingSeverity};
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn preview_adds_only_electrical_notices_and_keeps_accepted_findings() {
        let accepted = LayoutFinding {
            id: "accepted:layout:warning".into(),
            severity: FindingSeverity::Warning,
            scope: FindingScope::Layout,
            message: "Accepted diagnostic remains visible".into(),
            target_ids: vec!["accepted-part".into()],
        };
        let ordinary_preview = LayoutFinding {
            id: "preview:layout:warning".into(),
            severity: FindingSeverity::Warning,
            scope: FindingScope::Layout,
            message: "Ordinary preview diagnostic stays hidden".into(),
            target_ids: vec!["preview-part".into()],
        };
        let electrical_preview = LayoutFinding {
            id: "preview:electrical:net:1-61".into(),
            severity: FindingSeverity::Info,
            scope: FindingScope::Pcb,
            message: "Electrical preview notice is visible".into(),
            target_ids: vec!["preview-pad".into()],
        };

        assert_eq!(
            layout_findings_for_display(
                std::slice::from_ref(&accepted),
                Some(&[ordinary_preview, electrical_preview.clone()]),
            ),
            vec![accepted, electrical_preview],
        );
    }
}

pub(crate) fn layout_finding_context(
    model: &ReadModel,
    target: &keycaps_fit::FindingNavigationTarget,
) -> Option<objects::TreeContext> {
    use keycaps_fit::FindingNavigationTarget as Target;
    match target {
        Target::Part { part_id, .. } => objects::component_context_for_finding_part(model, part_id),
        Target::Matrix { matrix_id, .. } => Some(objects::TreeContext::Matrix {
            matrix_id: matrix_id.clone(),
        }),
        Target::Outline { board_id } => Some(objects::TreeContext::Outline {
            board_id: board_id.clone(),
        }),
        Target::Board { board_id } => Some(objects::TreeContext::Outline {
            board_id: board_id.clone(),
        }),
        Target::Body { .. } | Target::MechanicalLayer { .. } => None,
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
            .is_some_and(|scope| active_board_scope_matches(&model, scope))
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
    context: LayoutFindingNavigationContext,
    request: layout_findings::Request,
) {
    let LayoutFindingNavigationContext {
        runtime,
        adapter,
        owner,
        workspace,
        mut objects_open,
        mut inspect_open,
        mut findings_open,
        inspector_settings,
        keycaps_state,
        svg,
        alive,
        body_selection,
        case_selection,
        select_tree,
        resumed_after_board_navigation,
    } = context;
    if !layout_owner_is_current(&runtime, workspace, &adapter, &owner)
        || !source_matches_layout_owner(&request.source, &owner, resumed_after_board_navigation)
    {
        return;
    }
    let Some(scope) = owner.scope.as_ref() else {
        return;
    };
    let model = runtime.model();
    if !layout_finding_is_live(&model, &request, &scope.board_id) {
        return;
    }
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let Some(context) = layout_finding_context(&model, &request.target) else {
        return;
    };
    select_tree.call(objects::TreeSelectRequest {
        scope: scope.clone(),
        context: context.clone(),
        mode: SelectionMode::Replace,
        outline_action: None,
    });
    let route_model = runtime.model();
    let Some(route_snapshot) = route_model.accepted.as_ref() else {
        return;
    };
    if workspace() != "Layout"
        || runtime.scope().as_ref() != Some(scope)
        || route_snapshot.token != request.source.token
        || route_snapshot.document.revision != request.source.revision
    {
        return;
    }
    let route_generation = (adapter.generation)();

    let destination = keycaps_navigation::Destination::Layout(context);
    let navigation_owner = keycaps_navigation::NavigationOwner {
        workspace: "Layout",
        scope: scope.clone(),
        generation: route_generation,
        token: request.source.token,
        revision: request.source.revision,
        destination,
    };
    if let Some(surface) = svg.borrow().as_ref() {
        let rect = surface.get_bounding_client_rect();
        let surface_size = (rect.width(), rect.height());
        if let (Some(base), Some(target_bounds)) = (
            keycaps_fit::layout_camera_basis(
                &snapshot.document,
                &snapshot.scene,
                &scope.board_id,
                surface_size,
            ),
            keycaps_fit::finding_navigation_bounds(
                &snapshot.document,
                &snapshot.scene,
                &request.finding.id,
                &request.target,
            ),
        ) {
            let live_owner = keycaps_navigation::LiveNavigationOwner {
                workspace: "Layout",
                scope: Some(scope.clone()),
                generation: route_generation,
                token: Some(request.source.token),
                revision: Some(request.source.revision),
                destinations: vec![navigation_owner.destination.clone()],
            };
            if let Some(camera) = keycaps_navigation::destination_camera_fit(
                &navigation_owner,
                &live_owner,
                base.bounds,
                target_bounds,
                surface_size,
            ) {
                runtime.submit(Event::SetCamera {
                    operation_id: runtime.operation(),
                    center: camera.center,
                    zoom: camera.zoom,
                });
            }
        }
    }

    keycaps_state.focus_finding(
        scope.clone(),
        request.source.token,
        request.source.revision,
        request.finding.id,
    );
    findings_open.set(false);
    objects_open.set(false);
    inspect_open.set(true);
    pin_inspector_on_desktop(inspector_settings);
    let focus_runtime = runtime.clone();
    let focus_adapter = adapter.clone();
    focus_first_inspector_control_on_next_frame(alive, navigation_owner, move || {
        current_keycaps_navigation_owner(
            workspace,
            &focus_runtime,
            &focus_adapter,
            body_selection,
            case_selection,
        )
    });
}

pub(crate) fn current_keycaps_navigation_owner(
    workspace: Signal<&'static str>,
    runtime: &Rc<Runtime>,
    adapter: &SelectionAdapter,
    body_selection: Signal<Option<case_viewer::BodySelection>>,
    case_selection: case_viewer::CaseSelection,
) -> keycaps_navigation::LiveNavigationOwner {
    let scope = runtime.scope();
    let model = runtime.model();
    let (token, revision) = model
        .accepted
        .as_ref()
        .map(|snapshot| (Some(snapshot.token), Some(snapshot.document.revision)))
        .unwrap_or((None, None));
    let destinations = match (workspace(), scope.as_ref()) {
        ("Layout", Some(scope)) => adapter
            .selected_context
            .read()
            .as_ref()
            .filter(|selected| &selected.scope == scope)
            .map(|selected| {
                vec![keycaps_navigation::Destination::Layout(
                    selected.context.clone(),
                )]
            })
            .unwrap_or_default(),
        ("Case", Some(scope)) => {
            let layer_id = case_selection.layer_id(scope);
            let body_id = body_selection
                .read()
                .as_ref()
                .filter(|selected| &selected.scope == scope)
                .map(|body| body.body_id.clone());
            keycaps_navigation::active_case_destination(&layer_id, body_id.as_deref())
                .into_iter()
                .collect()
        }
        _ => Vec::new(),
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

pub(crate) fn fit_selected_bridge(runtime: &Rc<Runtime>, bridge_id: Option<&str>) {
    let Some(bridge_id) = bridge_id else { return };
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let Some(bridge) = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == model.active_board_id)
        .and_then(|scene| scene.bridges.iter().find(|bridge| bridge.id == bridge_id))
    else {
        return;
    };
    let Some((min_x, max_x, min_y, max_y)) = bridge.points.iter().fold(None, |bounds, point| {
        Some(bounds.map_or(
            (point.x, point.x, point.y, point.y),
            |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                (
                    min_x.min(point.x),
                    max_x.max(point.x),
                    min_y.min(point.y),
                    max_y.max(point.y),
                )
            },
        ))
    }) else {
        return;
    };
    let board_bounds = snapshot
        .scene
        .board_contours
        .iter()
        .filter(|board| board.board_id == model.active_board_id)
        .flat_map(|board| &board.contours)
        .flat_map(|contour| &contour.points)
        .fold(None, |bounds, point| {
            Some(bounds.map_or(
                (point.x, point.x, point.y, point.y),
                |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                    (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_y.min(point.y),
                        max_y.max(point.y),
                    )
                },
            ))
        })
        .or_else(|| {
            let board = snapshot
                .document
                .boards
                .iter()
                .find(|board| board.id == model.active_board_id)?;
            let mut parts = snapshot
                .document
                .parts
                .iter()
                .filter(|part| board.part_ids.contains(&part.id));
            let first = parts.next()?.pose.at;
            Some(parts.fold(
                (first.x, first.x, first.y, first.y),
                |(min_x, max_x, min_y, max_y), part| {
                    (
                        min_x.min(part.pose.at.x),
                        max_x.max(part.pose.at.x),
                        min_y.min(part.pose.at.y),
                        max_y.max(part.pose.at.y),
                    )
                },
            ))
        })
        .unwrap_or((min_x, max_x, min_y, max_y));
    let viewport_bounds = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == model.active_board_id)
        .and_then(|board| {
            snapshot
                .document
                .parts
                .iter()
                .filter(|part| board.part_ids.contains(&part.id))
                .map(|part| {
                    // Ticket 12: fit the evaluated board pose of half-frame
                    // parts, like the canvas does.
                    snapshot
                        .scene
                        .transforms
                        .iter()
                        .find(|transform| transform.id == part.id)
                        .map(|transform| transform.pose.at)
                        .unwrap_or(part.pose.at)
                })
                .fold(None, |bounds, point| {
                    Some(bounds.map_or(
                        (point.x, point.x, point.y, point.y),
                        |(min_x, max_x, min_y, max_y): (f64, f64, f64, f64)| {
                            (
                                min_x.min(point.x),
                                max_x.max(point.x),
                                min_y.min(point.y),
                                max_y.max(point.y),
                            )
                        },
                    ))
                })
        })
        .unwrap_or(board_bounds);
    let bridge_width = (max_x - min_x).max(bridge.width).max(1.0);
    let bridge_height = (max_y - min_y).max(bridge.width).max(1.0);
    let board_width = (board_bounds.1 - board_bounds.0).max(50.0) + 40.0;
    let board_height = (board_bounds.3 - board_bounds.2).max(50.0) + 40.0;
    let zoom =
        (0.72 * (board_width / bridge_width).min(board_height / bridge_height)).clamp(0.15, 8.0);
    runtime.submit(Event::SetCamera {
        operation_id: runtime.operation(),
        center: layout_camera::bridge_camera_offset(
            viewport_bounds,
            Vec2 {
                x: (min_x + max_x) * 0.5,
                y: (min_y + max_y) * 0.5,
            },
        ),
        zoom,
    });
}
