//! Private owner for Keycaps finding navigation effects.
//!
//! The root supplies a request only after validating its accepted snapshot and live target.
//! This module keeps route-specific behavior and the delayed destination-camera calculation
//! behind the same production seam exercised by its tests.

use super::{keycaps_fit, objects};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::Vec2;
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
    QueueLayoutFit(PendingLayoutFit),
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

pub(super) fn dispatch_route(
    effects: RouteEffects,
    request: &keycaps_fit::FindingNavigationRequest,
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
                perform(RouteAction::QueueLayoutFit(PendingLayoutFit {
                    request: request.clone(),
                    owner: owner_for_request(request, Destination::Layout(context), generation),
                }));
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

pub(super) fn apply_destination_fit(
    expected: &NavigationOwner,
    current_owner: impl FnOnce() -> LiveNavigationOwner,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
    perform: impl FnMut(FitAction),
) -> bool {
    let live = current_owner();
    finish_destination_fit(expected, &live, base, target, surface, perform)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Finding, Scope as FindingScope, Severity};
    use std::{
        cell::RefCell,
        task::{Context, Waker},
    };
    use wasm_bindgen_test::*;

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
        initial_live: LiveNavigationOwner,
        live: Rc<RefCell<Option<Signal<LiveNavigationOwner>>>>,
        schedule: Rc<RefCell<Option<EventHandler<()>>>>,
        fit_effects: Rc<RefCell<Vec<Deferred>>>,
        frame_effects: Rc<RefCell<Vec<Deferred>>>,
        effects: Rc<RefCell<Vec<FitAction>>>,
        focus_count: Rc<Cell<usize>>,
    }

    fn mounted_owner_host() -> Element {
        let probe = use_context::<MountedProbe>();
        let live = use_signal(|| probe.initial_live.clone());
        *probe.live.borrow_mut() = Some(live);
        let alive = use_navigation_lifetime();
        let on_schedule = use_callback({
            let probe = probe.clone();
            move |_: ()| {
                let expected = probe.expected.clone();
                let fit_probe = probe.clone();
                let fit_alive = alive.clone();
                probe.fit_effects.borrow_mut().push(Box::new(move || {
                    if !fit_alive.get() {
                        return;
                    }
                    let owner_signal = fit_probe.live.borrow().expect("mounted owner signal");
                    let current_owner = move || owner_signal.read().clone();
                    let frame_probe = fit_probe.clone();
                    let frame_alive = fit_alive.clone();
                    let focus_expected = expected.clone();
                    apply_destination_fit(
                        &expected,
                        current_owner,
                        (-100.0, 100.0, -60.0, 60.0),
                        (20.0, 40.0, 10.0, 30.0),
                        (900.0, 600.0),
                        move |effect| match effect {
                            FitAction::SetCamera(fit) => {
                                frame_probe
                                    .effects
                                    .borrow_mut()
                                    .push(FitAction::SetCamera(fit));
                            }
                            FitAction::FocusInspector => {
                                let focus_probe = frame_probe.clone();
                                let focus_signal =
                                    focus_probe.live.borrow().expect("mounted owner signal");
                                let current_focus_owner = move || focus_signal.read().clone();
                                let scheduled_probe = focus_probe.clone();
                                queue_owner_focus(
                                    frame_alive.clone(),
                                    focus_expected.clone(),
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
                                frame_probe
                                    .effects
                                    .borrow_mut()
                                    .push(FitAction::FocusInspector);
                            }
                        },
                    );
                }));
            }
        });
        *probe.schedule.borrow_mut() = Some(on_schedule);
        rsx! { button { onclick: move |_| on_schedule.call(()), "Schedule destination fit" } }
    }

    fn mounted_probe(owner: NavigationOwner) -> (MountedProbe, VirtualDom) {
        let probe = MountedProbe {
            initial_live: live(&owner),
            expected: owner,
            live: Rc::default(),
            schedule: Rc::default(),
            fit_effects: Rc::default(),
            frame_effects: Rc::default(),
            effects: Rc::default(),
            focus_count: Rc::new(Cell::new(0)),
        };
        let mut dom = VirtualDom::new(mounted_owner_host);
        dom.provide_root_context(probe.clone());
        dom.rebuild_to_vec();
        (probe, dom)
    }

    fn owner_fixture() -> NavigationOwner {
        owner_for_request(
            &request(keycaps_fit::FindingNavigationTarget::Part {
                board_id: "left".into(),
                part_id: "part-a".into(),
            }),
            Destination::Layout(objects::TreeContext::Component {
                part_id: Some("part-a".into()),
                matrix_id: None,
                row: None,
                column: None,
                assembly_id: None,
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
                RouteAction::QueueLayoutFit(PendingLayoutFit {
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
                }),
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
        stale_owner.destinations = vec![Destination::Layout(objects::TreeContext::Outline {
            board_id: "left".into(),
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
            destinations: vec![Destination::Layout(objects::TreeContext::Outline {
                board_id: "left".into(),
            })],
            ..live(&owner)
        };
        probe.live.borrow().unwrap().set(replacement);
        flush(&mut dom);
        probe.fit_effects.borrow_mut().pop().unwrap()();
        assert!(probe.effects.borrow().is_empty());
        assert!(probe.frame_effects.borrow().is_empty());
    }

    #[wasm_bindgen_test]
    fn mounted_delayed_focus_rechecks_same_scope_selection_before_focusing() {
        let owner = owner_fixture();
        let (probe, dom) = mounted_probe(owner.clone());
        probe.schedule.borrow().as_ref().unwrap().call(());
        probe.fit_effects.borrow_mut().pop().unwrap()();
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
        let (probe, dom) = mounted_probe(owner);
        probe.schedule.borrow().as_ref().unwrap().call(());
        probe.fit_effects.borrow_mut().pop().unwrap()();
        probe.frame_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.focus_count.get(), 1);

        // A second queued callback from this mounted owner must stop before reading its scoped
        // live-owner Signal when the component is removed.
        probe.schedule.borrow().as_ref().unwrap().call(());
        probe.fit_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.frame_effects.borrow().len(), 1);
        drop(dom);
        probe.frame_effects.borrow_mut().pop().unwrap()();
        assert_eq!(probe.focus_count.get(), 1);
    }
}
