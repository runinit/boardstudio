//! Projection of the focused Keycaps fit finding onto its accepted Layout scene.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{FindingMarker, Vec2};
use boardstudio_web_ui_model::Workspace;
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

pub fn use_retire_stale_finding(
    mut finding: Signal<Option<FocusedFinding>>,
    workspace: Workspace,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
) {
    use_effect(use_reactive(
        (&workspace, &scope, &token, &revision, &active_board_id),
        move |(workspace, scope, token, revision, active_board_id)| {
            if !finding.peek().as_ref().is_some_and(|f| {
                is_current_finding(
                    f,
                    workspace,
                    scope.as_ref(),
                    token,
                    revision,
                    &active_board_id,
                )
            }) {
                finding.set(None);
            }
        },
    ));
}

#[component]
pub fn FocusedFindingMarker(
    workspace: Workspace,
    scope: Option<Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: String,
    finding: Option<FocusedFinding>,
    markers: Rc<[FindingMarker]>,
) -> Element {
    let marker = finding.as_ref().and_then(|f| {
        is_current_finding(
            f,
            workspace,
            scope.as_ref(),
            token,
            revision,
            &active_board_id,
        )
        .then(|| {
            markers
                .iter()
                .find(|m| m.finding_id == f.finding_id && m.board_id == f.scope.board_id)
        })
        .flatten()
    });
    if let Some(marker) = marker {
        rsx! {
            g { class: "wb-outline-finding is-focused", "data-finding-id": "{marker.finding_id}",
                for (i, c) in marker.contours.iter().enumerate() {
                    polygon { key: "focused-finding-contour-{i}", points: contour_points(&c.points) }
                }
            }
        }
    } else {
        rsx! {}
    }
}

pub fn is_current_finding(
    finding: &FocusedFinding,
    workspace: Workspace,
    scope: Option<&Scope>,
    token: Option<SnapshotToken>,
    revision: Option<u64>,
    active_board_id: &str,
) -> bool {
    workspace == Workspace::Layout
        && scope == Some(&finding.scope)
        && token == Some(finding.token)
        && revision == Some(finding.revision)
        && active_board_id == finding.scope.board_id
}

fn contour_points(points: &[Vec2]) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

    #[test]
    fn current_finding_requires_matching_layout_owner_and_advances_id() {
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "b1".into(),
            instance_id: None,
        };
        let f = FocusedFinding {
            scope: scope.clone(),
            token: SnapshotToken(3),
            revision: 5,
            finding_id: "f1".into(),
            navigation_id: next_navigation_id(None),
        };
        assert_eq!(f.navigation_id, 0);
        assert_eq!(next_navigation_id(Some(&f)), 1);
        assert!(is_current_finding(
            &f,
            Workspace::Layout,
            Some(&scope),
            Some(SnapshotToken(3)),
            Some(5),
            "b1"
        ));
        assert!(!is_current_finding(
            &f,
            Workspace::Keycaps,
            Some(&scope),
            Some(SnapshotToken(3)),
            Some(5),
            "b1"
        ));
        assert!(!is_current_finding(
            &f,
            Workspace::Layout,
            Some(&scope),
            Some(SnapshotToken(4)),
            Some(5),
            "b1"
        ));
    }
}
