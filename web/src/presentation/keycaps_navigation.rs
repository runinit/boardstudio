//! Private owner for Keycaps finding navigation effects.
//!
//! The root supplies a request only after validating its accepted snapshot and live target.
//! This module keeps route-specific behavior and the delayed destination-camera calculation
//! behind the same production seam exercised by its tests.

use super::{keycaps_fit, objects};
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_core::model::{ProjectDoc, Vec2};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Destination {
    Layout(objects::TreeContext),
    CaseLayer(String),
    CaseBody(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RouteEffects {
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

/// Accepted authority passed by the root at request time. This keeps the shared admission seam
/// explicit without transferring snapshot or workspace ownership out of the root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AcceptedNavigationSource {
    pub scope: Scope,
    pub session_epoch: SessionEpoch,
    pub token: SnapshotToken,
    pub revision: u64,
    pub active_board_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AdmittedNavigation {
    pub target: keycaps_fit::FindingNavigationTarget,
    pub effects: RouteEffects,
    pub owner: NavigationOwner,
}

/// Publish a finding identity only for a route that was admitted to the Layout workspace.
/// Editor and the mounted navigation probe use this shared projection so marker state follows
/// the same accepted request that owns selection and destination fitting.
pub(super) fn focused_finding_for_admitted_route(
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

pub(super) struct NavigationAdmission<'a> {
    pub current_workspace: &'a str,
    pub owner: OwnerIdentity<'a>,
    pub live_scope: Option<&'a Scope>,
    pub live_generation: u64,
    pub accepted: &'a AcceptedNavigationSource,
    pub fit_state: &'a keycaps_fit::KeycapsFitState,
    pub document: &'a ProjectDoc,
    pub live_mechanical_layers: Option<&'a [String]>,
}

/// Identity retained by post-navigation work. Scope/generation alone are insufficient: a
/// selection can be replaced without changing either, so the intended destination is part of
/// the owner as well.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NavigationOwner {
    pub workspace: &'static str,
    pub scope: Scope,
    pub generation: u64,
    pub token: SnapshotToken,
    pub revision: u64,
    pub destination: Destination,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LiveNavigationOwner {
    pub workspace: &'static str,
    pub scope: Option<Scope>,
    pub generation: u64,
    pub token: Option<SnapshotToken>,
    pub revision: Option<u64>,
    pub destinations: Vec<Destination>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingLayoutFit {
    pub request: keycaps_fit::FindingNavigationRequest,
    pub owner: NavigationOwner,
    /// Camera basis captured by the Keycaps owner before routing. React's finding handler
    /// computes `fitParts` from that source render, then changes workspace; it does not
    /// recompute the fit against Layout's later canvas dimensions.
    pub source_basis: Option<keycaps_fit::CameraBasis>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct DestinationFitGeometry {
    pub base: (f64, f64, f64, f64),
    pub target: (f64, f64, f64, f64),
    pub surface: (f64, f64),
}

/// Shared reactive owner for the post-route destination fit. Editor and the mounted production
/// probe use this same hook, including pending-request settlement and owner revalidation.
pub(super) fn use_pending_layout_fit<T>(
    mut pending: Signal<Option<PendingLayoutFit>>,
    observed_owner: T,
    alive: Rc<Cell<bool>>,
    current_owner: impl Fn() -> LiveNavigationOwner + 'static,
    resolve_geometry: impl Fn(&PendingLayoutFit) -> Option<DestinationFitGeometry> + 'static,
    perform: impl Fn(FitAction, NavigationOwner) + 'static,
) where
    T: Clone + PartialEq + 'static,
{
    use_effect(use_reactive(
        (&pending(), &observed_owner),
        move |(pending_fit, _)| {
            if !alive.get() {
                return;
            }
            let Some(pending_fit) = pending_fit.clone() else {
                return;
            };
            let live = current_owner();
            if !navigation_owner_is_current(&pending_fit.owner, &live) {
                pending.set(None);
                return;
            }
            let Some(geometry) = resolve_geometry(&pending_fit) else {
                pending.set(None);
                return;
            };
            let live = current_owner();
            let did_fit = finish_destination_fit(
                &pending_fit.owner,
                &live,
                geometry.base,
                geometry.target,
                geometry.surface,
                |action| perform(action, pending_fit.owner.clone()),
            );
            if !did_fit {
                pending.set(None);
                return;
            }
            pending.set(None);
        },
    ));
}

pub(super) fn owner_for_request(
    request: &keycaps_fit::FindingNavigationRequest,
    destination: Destination,
    generation: u64,
) -> NavigationOwner {
    let workspace = match &destination {
        Destination::Layout(_) => "Layout",
        Destination::CaseLayer(_) | Destination::CaseBody(_) => "Case",
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

pub(super) fn navigation_owner_is_current(
    expected: &NavigationOwner,
    live: &LiveNavigationOwner,
) -> bool {
    live.workspace == expected.workspace
        && live.scope.as_ref() == Some(&expected.scope)
        && live.generation == expected.generation
        && live.token == Some(expected.token)
        && live.revision == Some(expected.revision)
        && live.destinations.contains(&expected.destination)
}

pub(super) fn active_case_destination(
    selected_layer_id: &str,
    selected_body_id: Option<&str>,
) -> Option<Destination> {
    if !selected_layer_id.is_empty() {
        Some(Destination::CaseLayer(selected_layer_id.to_owned()))
    } else {
        selected_body_id.map(|body_id| Destination::CaseBody(body_id.to_owned()))
    }
}

/// Owner lifetime shared by the production Editor and the mounted lifecycle probe.
pub(super) fn use_navigation_lifetime() -> Rc<Cell<bool>> {
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    alive
}

/// Schedule an owner-checked focus operation. The lifetime check deliberately precedes the
/// live-owner closure, since that closure can read scoped Dioxus Signals.
pub(super) fn queue_owner_focus(
    alive: Rc<Cell<bool>>,
    expected: NavigationOwner,
    current_owner: impl Fn() -> LiveNavigationOwner + 'static,
    focus: impl Fn() + 'static,
    schedule: impl FnOnce(Box<dyn FnOnce()>),
) {
    schedule(Box::new(move || {
        if !alive.get() {
            return;
        }
        if navigation_owner_is_current(&expected, &current_owner()) {
            focus();
        }
    }));
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum RouteAction {
    SetWorkspace(&'static str),
    SelectTree {
        scope: Scope,
        context: objects::TreeContext,
    },
    SelectCaseLayer {
        scope: Scope,
        layer_id: String,
    },
    SelectCaseBody {
        scope: Scope,
        body_id: String,
    },
    CloseObjects,
    OpenInspector,
    PinInspector,
    QueueLayoutFit(Box<PendingLayoutFit>),
    Report(String),
    FocusInspector,
}

#[derive(Clone, Copy)]
pub(super) struct OwnerIdentity<'a> {
    pub scope: &'a Scope,
    pub generation: u64,
}

pub(super) fn request_owner_is_current(
    workspace: &str,
    owner: OwnerIdentity<'_>,
    live_scope: Option<&Scope>,
    live_generation: u64,
    request_scope: &Scope,
) -> bool {
    workspace == "Keycaps"
        && live_scope == Some(owner.scope)
        && live_generation == owner.generation
        && request_scope == owner.scope
}

/// Compute effects only for the live request admitted by the root's accepted-source checks.
/// Mechanical-layer navigation intentionally follows React's early return and does not pin.
pub(super) fn route_effects(
    request: &keycaps_fit::FindingNavigationRequest,
    target: &keycaps_fit::FindingNavigationTarget,
    part_context: Option<objects::TreeContext>,
    generation: u64,
) -> Option<RouteEffects> {
    if request.target != *target {
        return None;
    }
    let (destination, fit_after_layout, pin_inspector) = match target {
        keycaps_fit::FindingNavigationTarget::MechanicalLayer { layer_id, .. } => {
            (Destination::CaseLayer(layer_id.clone()), false, false)
        }
        keycaps_fit::FindingNavigationTarget::Body { body_id, .. } => {
            (Destination::CaseBody(body_id.clone()), false, true)
        }
        keycaps_fit::FindingNavigationTarget::Part { .. } => {
            (Destination::Layout(part_context?), true, true)
        }
        keycaps_fit::FindingNavigationTarget::Matrix { matrix_id, .. } => (
            Destination::Layout(objects::TreeContext::Matrix {
                matrix_id: matrix_id.clone(),
            }),
            true,
            true,
        ),
        keycaps_fit::FindingNavigationTarget::Outline { board_id }
        | keycaps_fit::FindingNavigationTarget::Board { board_id } => (
            Destination::Layout(objects::TreeContext::Outline {
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

/// Admit a Keycaps finding against the root's live scope and accepted snapshot, then resolve its
/// route through the same production path used by the Editor. The caller supplies the already
/// resolved part context because the root's selection adapter owns that projection.
pub(super) fn admit_accepted_request(
    request: &keycaps_fit::FindingNavigationRequest,
    admission: NavigationAdmission<'_>,
    part_context_for_target: impl FnOnce(
        &keycaps_fit::FindingNavigationTarget,
    ) -> Option<objects::TreeContext>,
) -> Option<AdmittedNavigation> {
    if !request_owner_is_current(
        admission.current_workspace,
        admission.owner,
        admission.live_scope,
        admission.live_generation,
        &request.source.scope,
    ) || request.source.scope != admission.accepted.scope
        || request.source.scope.session_epoch != admission.accepted.session_epoch
        || request.source.token != admission.accepted.token
        || request.source.revision != admission.accepted.revision
        || request.source.scope.document_id != admission.document.id
        || request.source.scope.board_id != admission.accepted.active_board_id
    {
        return None;
    }
    let live_mechanical_layers =
        if admission.live_mechanical_layers.is_none() && request.source.case_preview_current {
            return None;
        } else {
            admission.live_mechanical_layers.unwrap_or(&[])
        };
    let target = keycaps_fit::accepted_navigation_target(
        admission.fit_state,
        request,
        admission.document,
        live_mechanical_layers,
    )?;
    if matches!(
        &request.target,
        keycaps_fit::FindingNavigationTarget::MechanicalLayer { .. }
    ) && target != request.target
    {
        return None;
    }
    let target_board = match &target {
        keycaps_fit::FindingNavigationTarget::MechanicalLayer { board_id, .. }
        | keycaps_fit::FindingNavigationTarget::Outline { board_id }
        | keycaps_fit::FindingNavigationTarget::Part { board_id, .. }
        | keycaps_fit::FindingNavigationTarget::Matrix { board_id, .. }
        | keycaps_fit::FindingNavigationTarget::Body { board_id, .. }
        | keycaps_fit::FindingNavigationTarget::Board { board_id } => board_id,
    };
    if target_board != &request.source.scope.board_id
        || !target_exists(admission.document, &target, live_mechanical_layers)
    {
        return None;
    }
    let part_context = part_context_for_target(&target);
    let effects = route_effects(request, &target, part_context, admission.live_generation)?;
    let destination = effects.destination.clone();
    let owner = owner_for_request(request, destination, admission.live_generation);
    Some(AdmittedNavigation {
        target,
        effects,
        owner,
    })
}

fn target_exists(
    document: &ProjectDoc,
    target: &keycaps_fit::FindingNavigationTarget,
    live_mechanical_layers: &[String],
) -> bool {
    use keycaps_fit::FindingNavigationTarget as Target;
    match target {
        Target::MechanicalLayer { layer_id, .. } => {
            live_mechanical_layers.iter().any(|id| id == layer_id)
        }
        Target::Body { board_id, body_id } => document
            .case_bodies
            .iter()
            .any(|body| body.id == *body_id && body.board_id == *board_id),
        Target::Part { board_id, part_id } => {
            document
                .boards
                .iter()
                .any(|board| board.id == *board_id && board.part_ids.contains(part_id))
                && document.parts.iter().any(|part| part.id == *part_id)
        }
        Target::Matrix {
            board_id,
            matrix_id,
        } => document.matrices.iter().any(|matrix| {
            matrix.id == *matrix_id
                && matrix.part_ids.iter().all(|id| {
                    document
                        .boards
                        .iter()
                        .any(|board| board.id == *board_id && board.part_ids.contains(id))
                })
        }),
        Target::Outline { board_id } | Target::Board { board_id } => {
            document.boards.iter().any(|board| board.id == *board_id)
        }
    }
}

#[cfg(test)]
pub(super) fn dispatch_route(
    effects: RouteEffects,
    request: &keycaps_fit::FindingNavigationRequest,
    perform: impl FnMut(RouteAction),
) {
    dispatch_route_with_camera_basis(effects, request, None, perform);
}

pub(super) fn dispatch_route_with_camera_basis(
    effects: RouteEffects,
    request: &keycaps_fit::FindingNavigationRequest,
    source_basis: Option<keycaps_fit::CameraBasis>,
    mut perform: impl FnMut(RouteAction),
) {
    let generation = effects.generation;
    match effects.destination {
        Destination::Layout(context) => {
            perform(RouteAction::SetWorkspace("Layout"));
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
            perform(RouteAction::SetWorkspace("Case"));
            perform(RouteAction::SelectCaseLayer {
                scope: effects.scope.clone(),
                layer_id,
            });
        }
        Destination::CaseBody(body_id) => {
            perform(RouteAction::SetWorkspace("Case"));
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
pub(super) struct CameraFit {
    pub center: Vec2,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum FitAction {
    SetCamera(CameraFit),
    FocusInspector,
}

/// Compute a camera update from destination-workbench geometry after the Layout render.
/// A stale request returns no effects, so the caller cannot accidentally move the new owner's
/// camera or focus its Inspector.
pub(super) fn destination_camera_fit(
    expected: &NavigationOwner,
    live: &LiveNavigationOwner,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
) -> Option<CameraFit> {
    if !navigation_owner_is_current(expected, live) {
        return None;
    }
    let (min_x, max_x, min_y, max_y) = base;
    let (target_min_x, target_max_x, target_min_y, target_max_y) = target;
    let (surface_width, surface_height) = surface;
    let surface_width = surface_width.max(1.0);
    let surface_height = surface_height.max(1.0);
    let base_width = (max_x - min_x).max(50.0);
    let base_height = (max_y - min_y).max(50.0);
    let target_width = (target_max_x - target_min_x).max(1.0);
    let target_height = (target_max_y - target_min_y).max(1.0);
    let top = 48.0 + 24.0;
    let bottom = 24.0;
    let usable_width = (surface_width - 32.0).max(1.0);
    let usable_height = (surface_height - top - bottom).max(1.0);
    let zoom = (base_width / target_width * usable_width / surface_width)
        .min(base_height / target_height * usable_height / surface_height)
        .clamp(0.15, 8.0);
    let center = Vec2 {
        x: (target_min_x + target_max_x - min_x - max_x) * 0.5,
        y: (target_min_y + target_max_y - min_y - max_y) * 0.5
            + (top - bottom) * base_height / zoom / surface_height * 0.5,
    };
    Some(CameraFit { center, zoom })
}

pub(super) fn finish_destination_fit(
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

#[cfg(test)]
mod tests {
    use super::super::keycaps_finding_marker::{FocusedFindingMarker, use_retire_stale_finding};
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{
        Contour, Finding, FindingMarker, Scope as FindingScope, Severity, Vec2,
    };
    use std::{
        cell::RefCell,
        task::{Context, Waker},
    };
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;
    use web_sys::{Element as DomElement, HtmlElement};

    wasm_bindgen_test_configure!(run_in_browser);

    fn request(
        target: keycaps_fit::FindingNavigationTarget,
    ) -> keycaps_fit::FindingNavigationRequest {
        keycaps_fit::FindingNavigationRequest {
            source: keycaps_fit::KeycapsFitSource {
                scope: Scope {
                    session_epoch: SessionEpoch(2),
                    document_id: "doc".into(),
                    board_id: "left".into(),
                    instance_id: None,
                },
                token: SnapshotToken(7),
                revision: 11,
                case_preview_current: false,
            },
            finding: Finding {
                id: "fit/overlap".into(),
                severity: Severity::Warning,
                scope: FindingScope::Layout,
                message: "Overlap found".into(),
                target_ids: vec!["part-a".into()],
            },
            target,
        }
    }

    fn live(owner: &NavigationOwner) -> LiveNavigationOwner {
        LiveNavigationOwner {
            workspace: owner.workspace,
            scope: Some(owner.scope.clone()),
            generation: owner.generation,
            token: Some(owner.token),
            revision: Some(owner.revision),
            destinations: vec![owner.destination.clone()],
        }
    }

    type Deferred = Box<dyn FnOnce()>;

    #[derive(Clone)]
    struct MountedProbe {
        expected: NavigationOwner,
        request: keycaps_fit::FindingNavigationRequest,
        fit_state: keycaps_fit::KeycapsFitState,
        document: boardstudio_core::model::ProjectDoc,
        initial_live: LiveNavigationOwner,
        source_basis: keycaps_fit::CameraBasis,
        workspace: Rc<RefCell<Option<Signal<&'static str>>>>,
        focused: Rc<
            RefCell<Option<Signal<Option<super::super::keycaps_finding_marker::FocusedFinding>>>>,
        >,
        markers: Rc<[FindingMarker]>,
        live: Rc<RefCell<Option<Signal<LiveNavigationOwner>>>>,
        schedule: Rc<RefCell<Option<EventHandler<()>>>>,
        pending: Rc<RefCell<Option<Signal<Option<PendingLayoutFit>>>>>,
        frame_effects: Rc<RefCell<Vec<Deferred>>>,
        route_actions: Rc<RefCell<Vec<RouteAction>>>,
        effects: Rc<RefCell<Vec<FitAction>>>,
        focus_count: Rc<Cell<usize>>,
    }

    fn mounted_owner_host() -> Element {
        let probe = use_context::<MountedProbe>();
        let mut live = use_signal(|| probe.initial_live.clone());
        *probe.live.borrow_mut() = Some(live);
        let mut workspace = use_signal(|| probe.initial_live.workspace);
        *probe.workspace.borrow_mut() = Some(workspace);
        let mut focused =
            use_signal(|| None::<super::super::keycaps_finding_marker::FocusedFinding>);
        *probe.focused.borrow_mut() = Some(focused);
        use_retire_stale_finding(
            focused,
            workspace(),
            Some(probe.request.source.scope.clone()),
            Some(probe.request.source.token),
            Some(probe.request.source.revision),
            "left".into(),
        );
        let pending = use_signal(|| None::<PendingLayoutFit>);
        *probe.pending.borrow_mut() = Some(pending);
        let alive = use_navigation_lifetime();
        use_pending_layout_fit(
            pending,
            live(),
            alive.clone(),
            move || live.read().clone(),
            |pending_fit| {
                let destination_surface = (900.0, 600.0);
                let basis = pending_fit
                    .source_basis
                    .unwrap_or(keycaps_fit::CameraBasis {
                        bounds: (-100.0, 100.0, -60.0, 60.0),
                        surface: destination_surface,
                    });
                Some(DestinationFitGeometry {
                    base: basis.bounds,
                    target: (-69.07, 87.43, -99.148, -56.948),
                    surface: basis.surface,
                })
            },
            {
                let probe = probe.clone();
                move |action, expected| match action {
                    FitAction::SetCamera(fit) => {
                        probe.effects.borrow_mut().push(FitAction::SetCamera(fit))
                    }
                    FitAction::FocusInspector => {
                        let focus_probe = probe.clone();
                        let focus_live = focus_probe.live.borrow().expect("mounted owner signal");
                        let current_focus_owner = move || focus_live.read().clone();
                        let scheduled_probe = focus_probe.clone();
                        queue_owner_focus(
                            alive.clone(),
                            expected,
                            current_focus_owner,
                            move || {
                                scheduled_probe
                                    .focus_count
                                    .set(scheduled_probe.focus_count.get() + 1);
                            },
                            move |frame| {
                                scheduled_probe.frame_effects.borrow_mut().push(frame);
                            },
                        );
                        probe.effects.borrow_mut().push(FitAction::FocusInspector);
                    }
                }
            },
        );
        let on_schedule = use_callback({
            let probe = probe.clone();
            move |_: ()| {
                let request = probe.request.clone();
                let live_now = live.read().clone();
                let current_workspace = workspace();
                let accepted = AcceptedNavigationSource {
                    scope: request.source.scope.clone(),
                    session_epoch: request.source.scope.session_epoch,
                    token: request.source.token,
                    revision: request.source.revision,
                    active_board_id: "left".into(),
                };
                let admitted = admit_accepted_request(
                    &request,
                    NavigationAdmission {
                        current_workspace,
                        owner: OwnerIdentity {
                            scope: &request.source.scope,
                            generation: probe.expected.generation,
                        },
                        live_scope: live_now.scope.as_ref(),
                        live_generation: live_now.generation,
                        accepted: &accepted,
                        fit_state: &probe.fit_state,
                        document: &probe.document,
                        live_mechanical_layers: Some(&[]),
                    },
                    |_| None,
                )
                .expect("mounted production admission accepts the current fixture");
                focused.set(focused_finding_for_admitted_route(&request, &admitted));
                let mut pending = probe.pending.borrow().expect("mounted pending fit");
                dispatch_route_with_camera_basis(
                    admitted.effects,
                    &request,
                    Some(probe.source_basis),
                    |action| {
                        match &action {
                            RouteAction::SetWorkspace(name) => {
                                workspace.set(*name);
                                let mut owner = live.read().clone();
                                owner.workspace = *name;
                                live.set(owner);
                            }
                            RouteAction::SelectTree { scope, context } => {
                                let mut owner = live.read().clone();
                                owner.workspace = "Layout";
                                owner.scope = Some(scope.clone());
                                owner.destinations = vec![Destination::Layout(context.clone())];
                                live.set(owner);
                            }
                            RouteAction::QueueLayoutFit(fit) => pending.set(Some(*fit.clone())),
                            _ => {}
                        }
                        probe.route_actions.borrow_mut().push(action);
                    },
                );
            }
        });
        *probe.schedule.borrow_mut() = Some(on_schedule);
        let clear_route = move |_| {
            workspace.set("Keycaps");
            let mut owner = live.read().clone();
            owner.workspace = "Keycaps";
            owner.destinations.clear();
            live.set(owner);
        };
        let marker_scope = probe.request.source.scope.clone();
        let marker_token = probe.request.source.token;
        let marker_revision = probe.request.source.revision;
        rsx! {
            button { id: "schedule-navigation", onclick: move |_| on_schedule.call(()), "Schedule destination fit" }
            button { id: "leave-layout", onclick: clear_route, "Leave Layout" }
            FocusedFindingMarker {
                workspace: workspace().to_owned(),
                scope: Some(marker_scope),
                token: Some(marker_token),
                revision: Some(marker_revision),
                active_board_id: "left".to_owned(),
                finding: focused(),
                markers: probe.markers.clone(),
            }
        }
    }

    fn make_mounted_probe(owner: NavigationOwner) -> MountedProbe {
        let (request, fit_state, document) = keycaps_fit::browser_navigation_fixture();
        let markers = Rc::from([FindingMarker {
            finding_id: request.finding.id.clone(),
            board_id: "left".into(),
            contours: vec![Contour {
                points: vec![
                    Vec2 { x: 1.0, y: 2.0 },
                    Vec2 { x: 4.0, y: 2.0 },
                    Vec2 { x: 4.0, y: 5.0 },
                ],
                hole: false,
            }],
        }]);
        let mut initial_live = live(&owner);
        initial_live.workspace = "Keycaps";
        initial_live.destinations.clear();
        MountedProbe {
            request,
            fit_state,
            document,
            initial_live,
            source_basis: keycaps_fit::CameraBasis {
                bounds: (-100.0, 126.915_439_560_439_52, -126.867, 15.542),
                surface: (725.0, 455.0),
            },
            expected: owner,
            workspace: Rc::default(),
            focused: Rc::default(),
            markers,
            live: Rc::default(),
            schedule: Rc::default(),
            pending: Rc::default(),
            frame_effects: Rc::default(),
            route_actions: Rc::default(),
            effects: Rc::default(),
            focus_count: Rc::new(Cell::new(0)),
        }
    }

    fn mounted_probe(owner: NavigationOwner) -> (MountedProbe, VirtualDom) {
        let probe = make_mounted_probe(owner);
        let mut dom = VirtualDom::new(mounted_owner_host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        (probe, dom)
    }

    fn owner_fixture() -> NavigationOwner {
        let (request, _, _) = keycaps_fit::browser_navigation_fixture();
        owner_for_request(
            &request,
            Destination::Layout(objects::TreeContext::Outline {
                board_id: "left".into(),
            }),
            4,
        )
    }

    fn flush(dom: &mut VirtualDom) {
        for _ in 0..4 {
            dom.render_immediate_to_vec();
            let mut work = std::pin::pin!(dom.wait_for_work());
            let _ = work.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        }
    }

    #[wasm_bindgen_test]
    fn current_part_route_selects_layout_and_pins_inspector() {
        let target = keycaps_fit::FindingNavigationTarget::Part {
            board_id: "left".into(),
            part_id: "part-a".into(),
        };
        let request = request(target.clone());
        let effects = route_effects(
            &request,
            &target,
            Some(objects::TreeContext::Component {
                part_id: Some("part-a".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            }),
            4,
        )
        .expect("current accepted target routes");
        assert_eq!(
            effects.destination,
            Destination::Layout(objects::TreeContext::Component {
                part_id: Some("part-a".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
            })
        );
        assert!(effects.pin_inspector);
        assert!(effects.fit_after_layout);
        assert!(!effects.focus_inspector_now);
        let mut performed = Vec::new();
        dispatch_route(effects, &request, |effect| performed.push(effect));
        assert_eq!(
            performed,
            vec![
                RouteAction::SetWorkspace("Layout"),
                RouteAction::SelectTree {
                    scope: request.source.scope.clone(),
                    context: objects::TreeContext::Component {
                        part_id: Some("part-a".into()),
                        matrix_id: None,
                        row: None,
                        column: None,
                        assembly_id: None,
                    },
                },
                RouteAction::QueueLayoutFit(Box::new(PendingLayoutFit {
                    request: request.clone(),
                    owner: owner_for_request(
                        &request,
                        Destination::Layout(objects::TreeContext::Component {
                            part_id: Some("part-a".into()),
                            matrix_id: None,
                            row: None,
                            column: None,
                            assembly_id: None,
                        }),
                        4,
                    ),
                    source_basis: None,
                })),
                RouteAction::CloseObjects,
                RouteAction::OpenInspector,
                RouteAction::PinInspector,
                RouteAction::Report("Overlap found".into()),
            ]
        );
    }

    #[wasm_bindgen_test]
    fn stale_route_and_changed_target_produce_no_ui_effects() {
        let target = keycaps_fit::FindingNavigationTarget::Body {
            board_id: "left".into(),
            body_id: "body-a".into(),
        };
        let request = request(target.clone());
        let owner = OwnerIdentity {
            scope: &request.source.scope,
            generation: 4,
        };
        assert!(!request_owner_is_current(
            "Layout",
            owner,
            Some(&request.source.scope),
            4,
            &request.source.scope,
        ));
        assert!(!request_owner_is_current(
            "Keycaps",
            owner,
            Some(&request.source.scope),
            5,
            &request.source.scope,
        ));
        let expected = owner_for_request(&request, Destination::CaseBody("body-a".into()), 4);
        let replacement = LiveNavigationOwner {
            destinations: vec![Destination::CaseLayer("generated/plate".into())],
            ..live(&expected)
        };
        assert!(!navigation_owner_is_current(&expected, &replacement));
        assert!(
            route_effects(
                &request,
                &keycaps_fit::FindingNavigationTarget::Board {
                    board_id: "left".into(),
                },
                None,
                4,
            )
            .is_none()
        );
    }

    #[wasm_bindgen_test]
    fn active_case_layer_replaces_retained_body_as_inspector_owner() {
        let body_request = request(keycaps_fit::FindingNavigationTarget::Body {
            board_id: "left".into(),
            body_id: "body-a".into(),
        });
        let body_owner =
            owner_for_request(&body_request, Destination::CaseBody("body-a".into()), 4);
        let layer_request = request(keycaps_fit::FindingNavigationTarget::MechanicalLayer {
            board_id: "left".into(),
            layer_id: "generated/plate".into(),
        });
        let layer_owner = owner_for_request(
            &layer_request,
            Destination::CaseLayer("generated/plate".into()),
            4,
        );
        let live_layer = LiveNavigationOwner {
            destinations: active_case_destination("generated/plate", Some("body-a"))
                .into_iter()
                .collect(),
            ..live(&layer_owner)
        };

        assert!(!navigation_owner_is_current(&body_owner, &live_layer));
        assert!(navigation_owner_is_current(&layer_owner, &live_layer));
    }

    #[wasm_bindgen_test]
    fn generated_layer_uses_case_early_route_without_desktop_pin() {
        let target = keycaps_fit::FindingNavigationTarget::MechanicalLayer {
            board_id: "left".into(),
            layer_id: "generated/plate".into(),
        };
        let request = request(target.clone());
        let effects = route_effects(&request, &target, None, 4).expect("live layer target routes");
        assert_eq!(
            effects.destination,
            Destination::CaseLayer("generated/plate".into())
        );
        assert!(!effects.pin_inspector);
        assert!(!effects.fit_after_layout);
        assert!(effects.focus_inspector_now);
        let mut performed = Vec::new();
        dispatch_route(effects, &request, |effect| performed.push(effect));
        assert!(performed.contains(&RouteAction::FocusInspector));
        assert!(!performed.contains(&RouteAction::PinInspector));
    }

    #[wasm_bindgen_test]
    fn destination_fit_uses_layout_bounds_and_suppresses_stale_side_effects() {
        let owner = owner_fixture();
        let current_owner = live(&owner);
        let current = destination_camera_fit(
            &owner,
            &current_owner,
            (-100.0, 100.0, -60.0, 60.0),
            (20.0, 40.0, 10.0, 30.0),
            (900.0, 600.0),
        )
        .expect("current request fits destination Layout geometry");
        assert!(current.zoom > 0.0);
        assert!((current.center.x - 30.0).abs() < 0.001);
        let base = (-100.0, 100.0, -60.0, 60.0);
        let target = (20.0, 40.0, 10.0, 30.0);
        let mut current_effects = Vec::new();
        assert!(finish_destination_fit(
            &owner,
            &current_owner,
            base,
            target,
            (900.0, 600.0),
            |effect| { current_effects.push(effect) }
        ));
        assert!(matches!(current_effects[0], FitAction::SetCamera(_)));
        assert_eq!(current_effects[1], FitAction::FocusInspector);
        let mut stale_effects = Vec::new();
        let mut stale_owner = current_owner.clone();
        stale_owner.destinations = vec![Destination::Layout(objects::TreeContext::Matrix {
            matrix_id: "replacement-matrix".into(),
        })];
        assert!(!finish_destination_fit(
            &owner,
            &stale_owner,
            base,
            target,
            (900.0, 600.0),
            |effect| stale_effects.push(effect),
        ));
        assert!(stale_effects.is_empty());
    }

    #[wasm_bindgen_test]
    fn mounted_layout_fit_drops_when_selection_changes_without_scope_change() {
        let owner = owner_fixture();
        let (probe, mut dom) = mounted_probe(owner.clone());
        probe.schedule.borrow().as_ref().unwrap().call(());
        let replacement = LiveNavigationOwner {
            destinations: vec![Destination::Layout(objects::TreeContext::Matrix {
                matrix_id: "replacement-matrix".into(),
            })],
            ..live(&owner)
        };
        probe.live.borrow().unwrap().set(replacement);
        flush(&mut dom);
        assert!(probe.effects.borrow().is_empty());
        assert!(probe.frame_effects.borrow().is_empty());
        assert!(probe.pending.borrow().unwrap()().is_none());
    }

    #[wasm_bindgen_test]
    fn mounted_delayed_focus_rechecks_same_scope_selection_before_focusing() {
        let owner = owner_fixture();
        let (probe, mut dom) = mounted_probe(owner.clone());
        probe.schedule.borrow().as_ref().unwrap().call(());
        flush(&mut dom);
        assert!(matches!(probe.effects.borrow()[0], FitAction::SetCamera(_)));
        assert_eq!(probe.effects.borrow()[1], FitAction::FocusInspector);
        assert_eq!(probe.frame_effects.borrow().len(), 1);

        // This is a same-scope replacement. The captured callback must not focus the next
        // Inspector owner even though the Runtime generation is unchanged.
        let replacement = LiveNavigationOwner {
            destinations: vec![Destination::Layout(objects::TreeContext::Matrix {
                matrix_id: "other-matrix".into(),
            })],
            ..live(&owner)
        };
        probe.live.borrow().unwrap().set(replacement);
        probe.frame_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.focus_count.get(), 0);
        drop(dom);
    }

    #[wasm_bindgen_test]
    fn mounted_delayed_focus_runs_for_current_owner_and_stops_after_unmount() {
        let owner = owner_fixture();
        let (probe, mut dom) = mounted_probe(owner);
        assert_eq!(
            probe.workspace.borrow().expect("mounted workspace")(),
            "Keycaps"
        );
        probe.schedule.borrow().as_ref().unwrap().call(());
        flush(&mut dom);
        probe.frame_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.focus_count.get(), 1);

        // A second queued callback from this mounted owner must stop before reading its scoped
        // live-owner Signal when the component is removed. Return to Keycaps before submitting
        // a second finding request; accepted navigation correctly rejects requests from Layout.
        probe
            .workspace
            .borrow()
            .expect("mounted workspace")
            .set("Keycaps");
        let mut live = probe.live.borrow().expect("mounted owner signal");
        let mut keycaps_owner = live.read().clone();
        keycaps_owner.workspace = "Keycaps";
        keycaps_owner.destinations.clear();
        live.set(keycaps_owner);
        probe.schedule.borrow().as_ref().unwrap().call(());
        flush(&mut dom);
        assert_eq!(probe.frame_effects.borrow().len(), 1);
        drop(dom);
        probe.frame_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.focus_count.get(), 1);
    }

    #[wasm_bindgen_test]
    async fn mounted_accepted_route_publishes_layout_marker_then_retires_it_on_workspace_change() {
        let probe = make_mounted_probe(owner_fixture());
        let root = mount_navigation_probe(probe.clone());
        settle_navigation_probe().await;

        click_probe("schedule-navigation");
        settle_navigation_probe().await;

        let marker = navigation_marker().expect("accepted Layout route renders its finding marker");
        assert_eq!(
            marker.get_attribute("data-finding-id").as_deref(),
            Some("board:left:invalid-settings")
        );
        assert!(matches!(
            probe.focused.borrow().unwrap().read().as_ref(),
            Some(finding) if finding.finding_id == "board:left:invalid-settings"
        ));
        assert_eq!(*probe.workspace.borrow().unwrap().read(), "Layout");
        assert!(probe.route_actions.borrow().iter().any(|action| matches!(
            action,
            RouteAction::SelectTree {
                context: objects::TreeContext::Outline { board_id },
                ..
            } if board_id == "left"
        )));
        assert!(matches!(
            probe.effects.borrow().first(),
            Some(FitAction::SetCamera(_))
        ));
        let Some(FitAction::SetCamera(camera)) = probe.effects.borrow().first().copied() else {
            panic!("accepted route must fit from its captured Keycaps basis");
        };
        assert!((camera.zoom - 1.385_94).abs() < 0.001);
        assert!(probe.pending.borrow().unwrap()().is_none());

        click_probe("leave-layout");
        settle_navigation_probe().await;
        assert!(probe.focused.borrow().unwrap().read().is_none());
        assert!(navigation_marker().is_none());

        root.remove();
    }

    fn mount_navigation_probe(probe: MountedProbe) -> DomElement {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("keycaps-navigation-marker-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(mounted_owner_host);
        dom.provide_root_context(probe);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        root
    }

    fn navigation_marker() -> Option<DomElement> {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#keycaps-navigation-marker-test-root g.wb-outline-finding.is-focused")
            .unwrap()
    }

    fn click_probe(id: &str) {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(id)
            .unwrap()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .click();
    }

    async fn settle_navigation_probe() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }
}
