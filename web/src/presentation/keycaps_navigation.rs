//! Private owner for Keycaps finding navigation effects.
//!
//! The root supplies a request only after validating its accepted snapshot and live target.
//! This module keeps route-specific behavior and the delayed destination-camera calculation
//! behind the same production seam exercised by its tests.

use super::{keycaps_fit, objects};
use boardstudio_application::Scope;
use boardstudio_core::model::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Destination {
    Layout(objects::TreeContext),
    CaseLayer(String),
    CaseBody(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RouteEffects {
    pub destination: Destination,
    pub scope: Scope,
    pub finding_message: String,
    pub close_objects: bool,
    pub open_inspector: bool,
    pub pin_inspector: bool,
    pub fit_after_layout: bool,
    pub focus_inspector_now: bool,
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
    QueueLayoutFit(keycaps_fit::FindingNavigationRequest),
    Report(String),
    FocusInspector,
}

#[derive(Clone, Copy)]
pub(super) struct OwnerIdentity<'a> {
    pub scope: &'a Scope,
    pub generation: u64,
}

pub(super) fn request_owner_is_current(
    in_expected_workspace: bool,
    owner: OwnerIdentity<'_>,
    live_scope: Option<&Scope>,
    live_generation: u64,
    request_scope: &Scope,
) -> bool {
    in_expected_workspace
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
    is_current: bool,
) -> Option<RouteEffects> {
    if !is_current || request.target != *target {
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
    match effects.destination {
        Destination::Layout(context) => {
            perform(RouteAction::SetWorkspace("Layout"));
            perform(RouteAction::SelectTree {
                scope: effects.scope.clone(),
                context,
            });
            if effects.fit_after_layout {
                perform(RouteAction::QueueLayoutFit(request.clone()));
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
    is_current: bool,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
) -> Option<CameraFit> {
    if !is_current {
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
    is_current: bool,
    base: (f64, f64, f64, f64),
    target: (f64, f64, f64, f64),
    surface: (f64, f64),
    mut perform: impl FnMut(FitAction),
) -> bool {
    let Some(fit) = destination_camera_fit(is_current, base, target, surface) else {
        return false;
    };
    perform(FitAction::SetCamera(fit));
    perform(FitAction::FocusInspector);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{Finding, Scope as FindingScope, Severity};
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
            true,
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
                RouteAction::QueueLayoutFit(request.clone()),
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
            false,
            owner,
            Some(&request.source.scope),
            4,
            &request.source.scope,
        ));
        assert!(!request_owner_is_current(
            true,
            owner,
            Some(&request.source.scope),
            5,
            &request.source.scope,
        ));
        assert!(route_effects(&request, &target, None, false).is_none());
        assert!(
            route_effects(
                &request,
                &keycaps_fit::FindingNavigationTarget::Board {
                    board_id: "left".into(),
                },
                None,
                true,
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
        let effects =
            route_effects(&request, &target, None, true).expect("live layer target routes");
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
        let current = destination_camera_fit(
            true,
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
            true,
            base,
            target,
            (900.0, 600.0),
            |effect| { current_effects.push(effect) }
        ));
        assert!(matches!(current_effects[0], FitAction::SetCamera(_)));
        assert_eq!(current_effects[1], FitAction::FocusInspector);
        let mut stale_effects = Vec::new();
        assert!(!finish_destination_fit(
            false,
            base,
            target,
            (900.0, 600.0),
            |effect| stale_effects.push(effect),
        ));
        assert!(stale_effects.is_empty());
    }
}
