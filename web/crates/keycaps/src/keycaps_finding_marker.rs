//! Projection of the focused Keycaps fit finding onto its accepted Layout scene.

use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{FindingMarker, Vec2};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusedFinding {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub finding_id: String,
    pub navigation_id: u64,
}

pub fn next_navigation_id(current: Option<&FocusedFinding>) -> u64 {
    current.map_or(0, |focused| focused.navigation_id.wrapping_add(1))
}

/// Retire marker state when its accepted Layout owner is no longer current. The Editor and the
/// mounted navigation probe share this effect so a route cannot leave a hidden, stale finding
/// identity behind when switching workspaces or accepted documents.
pub fn use_retire_stale_finding(
    mut finding: Signal<Option<FocusedFinding>>,
    workspace: &'static str,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
) {
    use_effect(use_reactive(
        (&workspace, &scope, &token, &revision, &active_board_id),
        move |(workspace, scope, token, revision, active_board_id)| {
            let current = finding.peek().as_ref().is_some_and(|focused| {
                is_current_finding(
                    focused,
                    workspace,
                    scope.as_ref(),
                    token,
                    revision,
                    &active_board_id,
                )
            });
            if !current {
                finding.set(None);
            }
        },
    ));
}

#[component]
pub fn FocusedFindingMarker(
    workspace: String,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
    finding: Option<FocusedFinding>,
    markers: Rc<[FindingMarker]>,
) -> Element {
    let marker = finding.as_ref().and_then(|finding| {
        is_current_finding(
            finding,
            &workspace,
            scope.as_ref(),
            token,
            revision,
            &active_board_id,
        )
        .then(|| {
            markers.iter().find(|marker| {
                marker.finding_id == finding.finding_id && marker.board_id == finding.scope.board_id
            })
        })
        .flatten()
    });

    if let Some(marker) = marker {
        rsx! {
            g { class: "wb-outline-finding is-focused", "data-finding-id": "{marker.finding_id}",
                for (index, contour) in marker.contours.iter().enumerate() {
                    polygon { key: "focused-finding-contour-{index}", points: contour_points(&contour.points) }
                }
            }
        }
    } else {
        rsx! {}
    }
}

pub fn is_current_finding(
    finding: &FocusedFinding,
    workspace: &str,
    scope: Option<&Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: &str,
) -> bool {
    workspace == "Layout"
        && scope == Some(&finding.scope)
        && token == Some(finding.token)
        && revision == Some(finding.revision)
        && active_board_id == finding.scope.board_id
}

fn contour_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "keycaps_finding_marker_tests.rs"]
mod tests;
