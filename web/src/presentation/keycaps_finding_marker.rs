//! Projection of the focused Keycaps fit finding onto its accepted Layout scene.

use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{FindingMarker, Vec2};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FocusedFinding {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub finding_id: String,
}

#[component]
pub(super) fn FocusedFindingMarker(
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

pub(super) fn is_current_finding(
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
