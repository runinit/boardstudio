pub(super) fn wheel_zoom(delta: f64) -> f64 {
    (delta.signum() * (delta.abs() * 0.001).min(1.0)).exp()
}

pub(super) fn crossed_drag_threshold(start: (f64, f64), point: (f64, f64)) -> bool {
    (point.0 - start.0).abs() + (point.1 - start.1).abs() > 4.0
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn wheel_zoom_is_reversible_and_caps_large_deltas() {
        assert_eq!(wheel_zoom(0.0), 1.0);
        assert_eq!(wheel_zoom(1000.0), wheel_zoom(5000.0));
        assert!((wheel_zoom(250.0) * wheel_zoom(-250.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn click_tolerance_is_measured_from_pointer_down_in_css_pixels() {
        assert!(!crossed_drag_threshold((12.0, 20.0), (14.0, 18.0)));
        assert!(crossed_drag_threshold((12.0, 20.0), (14.0, 17.9)));
    }
}

/// A Case draft and its provisional CAD lease expire together.
pub struct Drag<D, P> {
    pub draft: Option<D>,
    pub preview: Option<P>,
}

impl<D, P> Default for Drag<D, P> {
    fn default() -> Self {
        Self {
            draft: None,
            preview: None,
        }
    }
}

impl<D, P> Drag<D, P> {
    pub fn retire(&mut self) -> (Option<D>, Option<P>) {
        (self.draft.take(), self.preview.take())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn retiring_a_case_drag_releases_its_preview_once_and_drops_pending_edits() {
    let mut drag = Drag {
        draft: Some("pending mount"),
        preview: Some("CAD lease"),
    };
    assert_eq!(drag.retire(), (Some("pending mount"), Some("CAD lease")));
    assert!(
        drag.draft.as_mut().is_none(),
        "a late Move cannot revive the drag"
    );
    assert_eq!(drag.retire(), (None, None), "a late End cannot commit");
}

pub fn current_exact_case(
    owner: &boardstudio_web_runtime::case_generation_lifecycle::CaseGenerationOwner,
    scope: &boardstudio_application::Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    prepared_revision: u64,
    exact: bool,
) -> bool {
    exact
        && scope == &owner.scope
        && token == owner.token
        && revision == owner.revision
        && prepared_revision == owner.revision
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn exact_result_reuse_and_export_require_the_current_owner() {
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_web_runtime::case_generation_lifecycle::CaseGenerationOwner;

    let mut scope = Scope {
        session_epoch: SessionEpoch(1),
        document_id: "keyboard".into(),
        board_id: "left".into(),
        instance_id: Some("primary".into()),
    };
    let owner = CaseGenerationOwner {
        scope: scope.clone(),
        token: SnapshotToken(3),
        revision: 8,
    };
    assert!(current_exact_case(&owner, &scope, owner.token, 8, 8, true));
    assert!(!current_exact_case(&owner, &scope, owner.token, 8, 7, true));
    assert!(!current_exact_case(
        &owner,
        &scope,
        owner.token,
        8,
        8,
        false
    ));
    assert!(!current_exact_case(
        &owner,
        &scope,
        SnapshotToken(2),
        8,
        8,
        true
    ));
    scope.instance_id = Some("flipped".into());
    assert!(!current_exact_case(&owner, &scope, owner.token, 8, 8, true));
}

#[derive(Clone, PartialEq)]
pub(super) struct ProjectionInputs {
    pub source: usize,
    pub preview: usize,
    pub keycaps: Option<u64>,
    pub models: Vec<(String, usize)>,
    pub theme: String,
}

impl ProjectionInputs {
    /// A provisional CAD replacement can carry a captured handle only when the other inputs agree.
    pub fn continues_drag_in(&self, next: &Self) -> bool {
        self.source != next.source
            && self.preview == next.preview
            && self.keycaps == next.keycaps
            && self.models == next.models
            && self.theme == next.theme
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn only_a_provisional_geometry_change_can_carry_a_drag() {
    let before = ProjectionInputs {
        source: 1,
        preview: 2,
        keycaps: Some(3),
        models: vec![("module-model/a/0".into(), 4)],
        theme: "dark".into(),
    };
    let next = ProjectionInputs {
        source: 5,
        ..before.clone()
    };
    assert!(before.continues_drag_in(&next));
    assert!(!before.continues_drag_in(&before));
    assert!(!before.continues_drag_in(&ProjectionInputs {
        keycaps: Some(6),
        ..next.clone()
    }));
    assert!(!before.continues_drag_in(&ProjectionInputs {
        preview: 6,
        ..next.clone()
    }));
    assert!(!before.continues_drag_in(&ProjectionInputs {
        models: vec![],
        ..next.clone()
    }));
    assert!(!before.continues_drag_in(&ProjectionInputs {
        theme: "light".into(),
        ..next
    }));
}
