//! Private owner for Keycaps finding navigation effects.
use super::keycaps_fit;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_model::model::{ProjectDoc, Vec2};
use boardstudio_web_ui_model::{Workspace, tree::TreeContext};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Layout(TreeContext),
    CaseLayer(String),
    CaseBody(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouteEffects {
    pub destination: Destination,
    pub scope: Scope,
    pub generation: u64,
    pub finding_message: String,
    pub close_objects: bool,
    pub open_inspector: bool,
    pub pin_inspector: bool,
    pub fit_after_layout: bool,
    pub focus_inspector_now: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedNavigationSource {
    pub scope: Scope,
    pub session_epoch: SessionEpoch,
    pub token: SnapshotToken,
    pub revision: u64,
    pub active_board_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdmittedNavigation {
    pub target: keycaps_fit::FindingNavigationTarget,
    pub effects: RouteEffects,
    pub owner: NavigationOwner,
}

pub fn focused_finding_for_admitted_route(
    request: &keycaps_fit::FindingNavigationRequest,
    admitted: &AdmittedNavigation,
) -> Option<super::keycaps_finding_marker::FocusedFinding> {
    matches!(&admitted.owner.destination, Destination::Layout(_)).then(|| {
        super::keycaps_finding_marker::FocusedFinding {
            scope: request.source.scope.clone(),
            token: request.source.token,
            revision: request.source.revision,
            finding_id: request.finding.id.clone(),
            navigation_id: 0,
        }
    })
}

pub struct NavigationAdmission<'a> {
    pub current_workspace: Workspace,
    pub owner: OwnerIdentity<'a>,
    pub live_scope: Option<&'a Scope>,
    pub live_generation: u64,
    pub accepted: &'a AcceptedNavigationSource,
    pub fit_state: &'a keycaps_fit::KeycapsFitState,
    pub document: &'a ProjectDoc,
    pub live_mechanical_layers: Option<&'a [String]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationOwner {
    pub workspace: Workspace,
    pub scope: Scope,
    pub generation: u64,
    pub token: SnapshotToken,
    pub revision: u64,
    pub destination: Destination,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveNavigationOwner {
    pub workspace: Workspace,
    pub scope: Option<Scope>,
    pub generation: u64,
    pub token: Option<SnapshotToken>,
    pub revision: Option<u64>,
    pub destinations: Vec<Destination>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingLayoutFit {
    pub request: keycaps_fit::FindingNavigationRequest,
    pub owner: NavigationOwner,
    pub source_basis: Option<keycaps_fit::CameraBasis>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DestinationFitGeometry {
    pub base: (f64, f64, f64, f64),
    pub target: (f64, f64, f64, f64),
    pub surface: (f64, f64),
}

pub fn use_pending_layout_fit<T: Clone + PartialEq + 'static>(
    mut pending: Signal<Option<PendingLayoutFit>>,
    observed_owner: T,
    alive: Rc<Cell<bool>>,
    current_owner: impl Fn() -> LiveNavigationOwner + 'static,
    resolve_geometry: impl Fn(&PendingLayoutFit) -> Option<DestinationFitGeometry> + 'static,
    perform: impl Fn(FitAction, NavigationOwner) + 'static,
) {
    use_effect(use_reactive(
        (&pending(), &observed_owner),
        move |(pending_fit, _)| {
            if !alive.get() {
                return;
            }
            let Some(pf) = pending_fit.clone() else {
                return;
            };
            if !navigation_owner_is_current(&pf.owner, &current_owner()) {
                pending.set(None);
                return;
            }
            if let Some(g) = resolve_geometry(&pf) {
                finish_destination_fit(
                    &pf.owner,
                    &current_owner(),
                    g.base,
                    g.target,
                    g.surface,
                    |a| perform(a, pf.owner.clone()),
                );
            }
            pending.set(None);
        },
    ));
}

pub fn owner_for_request(
    request: &keycaps_fit::FindingNavigationRequest,
    destination: Destination,
    generation: u64,
) -> NavigationOwner {
    let workspace = match &destination {
        Destination::Layout(_) => Workspace::Layout,
        Destination::CaseLayer(_) | Destination::CaseBody(_) => Workspace::Case,
    };
    NavigationOwner {
        workspace,
        scope: request.source.scope.clone(),
        generation,
        token: request.source.token,
        revision: request.source.revision,
        destination,
    }
}

pub fn navigation_owner_is_current(expected: &NavigationOwner, live: &LiveNavigationOwner) -> bool {
    live.workspace == expected.workspace
        && live.scope.as_ref() == Some(&expected.scope)
        && live.generation == expected.generation
        && live.token == Some(expected.token)
        && live.revision == Some(expected.revision)
        && live.destinations.contains(&expected.destination)
}

pub fn active_case_destination(
    selected_layer_id: &str,
    selected_body_id: Option<&str>,
) -> Option<Destination> {
    if !selected_layer_id.is_empty() {
        Some(Destination::CaseLayer(selected_layer_id.to_owned()))
    } else {
        selected_body_id.map(|id| Destination::CaseBody(id.to_owned()))
    }
}

pub fn use_navigation_lifetime() -> Rc<Cell<bool>> {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let a = alive.clone();
        move || a.set(false)
    });
    alive
}

pub fn queue_owner_focus(
    alive: Rc<Cell<bool>>,
    expected: NavigationOwner,
    current_owner: impl Fn() -> LiveNavigationOwner + 'static,
    focus: impl Fn() + 'static,
    schedule: impl FnOnce(Box<dyn FnOnce()>),
) {
    schedule(Box::new(move || {
        if alive.get() && navigation_owner_is_current(&expected, &current_owner()) {
            focus();
        }
    }));
}

#[derive(Clone, Debug, PartialEq)]
pub enum RouteAction {
    SetWorkspace(Workspace),
    SelectTree { scope: Scope, context: TreeContext },
    SelectCaseLayer { scope: Scope, layer_id: String },
    SelectCaseBody { scope: Scope, body_id: String },
    CloseObjects,
    OpenInspector,
    PinInspector,
    QueueLayoutFit(Box<PendingLayoutFit>),
    Report(String),
    FocusInspector,
}

#[derive(Clone, Copy)]
pub struct OwnerIdentity<'a> {
    pub scope: &'a Scope,
    pub generation: u64,
}

pub fn request_owner_is_current(
    workspace: Workspace,
    owner: OwnerIdentity<'_>,
    live_scope: Option<&Scope>,
    live_generation: u64,
    request_scope: &Scope,
) -> bool {
    workspace == Workspace::Keycaps
        && live_scope == Some(owner.scope)
        && live_generation == owner.generation
        && request_scope == owner.scope
}

pub fn route_effects(
    request: &keycaps_fit::FindingNavigationRequest,
    target: &keycaps_fit::FindingNavigationTarget,
    part_context: Option<TreeContext>,
    generation: u64,
) -> Option<RouteEffects> {
    if request.target != *target {
        return None;
    }
    use keycaps_fit::FindingNavigationTarget as T;
    let (destination, fit_after_layout, pin_inspector) = match target {
        T::MechanicalLayer { layer_id, .. } => {
            (Destination::CaseLayer(layer_id.clone()), false, false)
        }
        T::Body { body_id, .. } => (Destination::CaseBody(body_id.clone()), false, true),
        T::Part { .. } => (Destination::Layout(part_context?), true, true),
        T::Matrix { matrix_id, .. } => (
            Destination::Layout(TreeContext::Matrix {
                matrix_id: matrix_id.clone(),
            }),
            true,
            true,
        ),
        T::Outline { board_id } | T::Board { board_id } => (
            Destination::Layout(TreeContext::Outline {
                board_id: board_id.clone(),
            }),
            true,
            true,
        ),
    };
    Some(RouteEffects {
        destination,
        scope: request.source.scope.clone(),
        generation,
        finding_message: request.finding.message.clone(),
        close_objects: true,
        open_inspector: true,
        pin_inspector,
        fit_after_layout,
        focus_inspector_now: !fit_after_layout,
    })
}

pub fn admit_accepted_request(
    request: &keycaps_fit::FindingNavigationRequest,
    a: NavigationAdmission<'_>,
    part_context_for_target: impl FnOnce(&keycaps_fit::FindingNavigationTarget) -> Option<TreeContext>,
) -> Option<AdmittedNavigation> {
    if !request_owner_is_current(
        a.current_workspace,
        a.owner,
        a.live_scope,
        a.live_generation,
        &request.source.scope,
    ) || request.source.scope != a.accepted.scope
        || request.source.scope.session_epoch != a.accepted.session_epoch
        || request.source.token != a.accepted.token
        || request.source.revision != a.accepted.revision
        || request.source.scope.document_id != a.document.id
        || request.source.scope.board_id != a.accepted.active_board_id
    {
        return None;
    }
    let layers = if a.live_mechanical_layers.is_none() && request.source.case_preview_current {
        return None;
    } else {
        a.live_mechanical_layers.unwrap_or(&[])
    };
    let target = keycaps_fit::accepted_navigation_target(a.fit_state, request, a.document, layers)?;
    if matches!(
        &request.target,
        keycaps_fit::FindingNavigationTarget::MechanicalLayer { .. }
    ) && target != request.target
    {
        return None;
    }
    if keycaps_fit::target_board_id(&target) != request.source.scope.board_id
        || !target_exists(a.document, &target, layers)
    {
        return None;
    }
    let effects = route_effects(
        request,
        &target,
        part_context_for_target(&target),
        a.live_generation,
    )?;
    let owner = owner_for_request(request, effects.destination.clone(), a.live_generation);
    Some(AdmittedNavigation {
        target,
        effects,
        owner,
    })
}

fn target_exists(
    doc: &ProjectDoc,
    target: &keycaps_fit::FindingNavigationTarget,
    layers: &[String],
) -> bool {
    use keycaps_fit::FindingNavigationTarget as T;
    match target {
        T::MechanicalLayer { layer_id, .. } => layers.contains(layer_id),
        T::Body { board_id, body_id } => doc
            .case_bodies
            .iter()
            .any(|b| b.id == *body_id && b.board_id == *board_id),
        T::Part { board_id, part_id } => {
            doc.boards
                .iter()
                .any(|b| b.id == *board_id && b.part_ids.contains(part_id))
                && doc.parts.iter().any(|p| p.id == *part_id)
        }
        T::Matrix {
            board_id,
            matrix_id,
        } => doc.matrices.iter().any(|m| {
            m.id == *matrix_id
                && m.part_ids.iter().all(|id| {
                    doc.boards
                        .iter()
                        .any(|b| b.id == *board_id && b.part_ids.contains(id))
                })
        }),
        T::Outline { board_id } | T::Board { board_id } => {
            doc.boards.iter().any(|b| b.id == *board_id)
        }
    }
}

pub fn dispatch_route_with_camera_basis(
    effects: RouteEffects,
    request: &keycaps_fit::FindingNavigationRequest,
    source_basis: Option<keycaps_fit::CameraBasis>,
    mut perform: impl FnMut(RouteAction),
) {
    let generation = effects.generation;
    match effects.destination {
        Destination::Layout(context) => {
            perform(RouteAction::SetWorkspace(Workspace::Layout));
            perform(RouteAction::SelectTree {
                scope: effects.scope.clone(),
                context: context.clone(),
            });
            if effects.fit_after_layout {
                perform(RouteAction::QueueLayoutFit(Box::new(PendingLayoutFit {
                    request: request.clone(),
                    owner: owner_for_request(request, Destination::Layout(context), generation),
                    source_basis,
                })));
            }
        }
        Destination::CaseLayer(layer_id) => {
            perform(RouteAction::SetWorkspace(Workspace::Case));
            perform(RouteAction::SelectCaseLayer {
                scope: effects.scope.clone(),
                layer_id,
            });
        }
        Destination::CaseBody(body_id) => {
            perform(RouteAction::SetWorkspace(Workspace::Case));
            perform(RouteAction::SelectCaseBody {
                scope: effects.scope.clone(),
                body_id,
            });
        }
    }
    if effects.close_objects {
        perform(RouteAction::CloseObjects);
    }
    if effects.open_inspector {
        perform(RouteAction::OpenInspector);
    }
    if effects.pin_inspector {
        perform(RouteAction::PinInspector);
    }
    perform(RouteAction::Report(effects.finding_message));
    if effects.focus_inspector_now {
        perform(RouteAction::FocusInspector);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFit {
    pub center: Vec2,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FitAction {
    SetCamera(CameraFit),
    FocusInspector,
}

pub fn destination_camera_fit(
    expected: &NavigationOwner,
    live: &LiveNavigationOwner,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
) -> Option<CameraFit> {
    if !navigation_owner_is_current(expected, live) {
        return None;
    }
    let (sw, sh) = (surface.0.max(1.0), surface.1.max(1.0));
    let (bw, bh) = ((base.1 - base.0).max(50.0), (base.3 - base.2).max(50.0));
    let (tw, th) = (
        (target.1 - target.0).max(1.0),
        (target.3 - target.2).max(1.0),
    );
    let (top, bottom) = (72.0, 24.0);
    let zoom = (bw / tw * (sw - 32.0).max(1.0) / sw)
        .min(bh / th * (sh - top - bottom).max(1.0) / sh)
        .clamp(0.15, 8.0);
    Some(CameraFit {
        center: Vec2 {
            x: (target.0 + target.1 - base.0 - base.1) * 0.5,
            y: (target.2 + target.3 - base.2 - base.3) * 0.5
                + (top - bottom) * bh / zoom / sh * 0.5,
        },
        zoom,
    })
}

pub fn finish_destination_fit(
    expected: &NavigationOwner,
    live: &LiveNavigationOwner,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
    mut perform: impl FnMut(FitAction),
) -> bool {
    let Some(fit) = destination_camera_fit(expected, live, base, target, surface) else {
        return false;
    };
    perform(FitAction::SetCamera(fit));
    perform(FitAction::FocusInspector);
    true
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn destination_fit_rejects_stale_owner_and_clamps_zoom() {
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "d".into(),
            board_id: "b".into(),
            instance_id: None,
        };
        let dest = Destination::Layout(TreeContext::Outline {
            board_id: "b".into(),
        });
        let expected = NavigationOwner {
            workspace: Workspace::Layout,
            scope: scope.clone(),
            generation: 1,
            token: SnapshotToken(2),
            revision: 3,
            destination: dest.clone(),
        };
        let live = LiveNavigationOwner {
            workspace: Workspace::Layout,
            scope: Some(scope),
            generation: 1,
            token: Some(SnapshotToken(2)),
            revision: Some(3),
            destinations: vec![dest],
        };
        let fit = destination_camera_fit(
            &expected,
            &live,
            (-50.0, 50.0, -50.0, 50.0),
            (-10.0, 10.0, -10.0, 10.0),
            (800.0, 600.0),
        )
        .expect("fit");
        assert!((0.15..=8.0).contains(&fit.zoom));
        let mut stale = live;
        stale.generation = 2;
        assert!(
            destination_camera_fit(
                &expected,
                &stale,
                (-50.0, 50.0, -50.0, 50.0),
                (-10.0, 10.0, -10.0, 10.0),
                (800.0, 600.0)
            )
            .is_none()
        );
    }
}
