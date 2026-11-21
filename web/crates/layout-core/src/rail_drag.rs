//! Session admission and preview progression for the canvas transform rails.

use boardstudio_application::{Durability, Lifecycle, ReadModel};

pub fn drag_session_is_ready(model: &ReadModel, revision: Option<u64>) -> bool {
    model.lifecycle == Lifecycle::Ready
        && revision.is_some_and(|revision| model.durability == Durability::Saved { revision })
        && pointer_edit_is_idle(model)
}

/// A nudge is queued intent and can compose with an edit already applying or saving.
pub fn nudge_session_is_ready(model: &ReadModel) -> bool {
    matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && pointer_edit_is_idle(model)
}

fn pointer_edit_is_idle(model: &ReadModel) -> bool {
    model.display_preview.is_none() && model.gesture.is_none()
}

/// The latest submitted sample of a preview-backed drag. A drag can start and end
/// before it receives a move event, or lose its coordinates at release.
#[derive(Clone)]
pub struct DragPreview<T> {
    last: Option<T>,
}

impl<T> Default for DragPreview<T> {
    fn default() -> Self {
        Self { last: None }
    }
}

pub struct DropSample<T> {
    pub operation: T,
    pub submit_preview: bool,
}

impl<T: Clone + PartialEq> DragPreview<T> {
    pub fn sample(&mut self, operation: &T) -> bool {
        if self.last.as_ref() == Some(operation) {
            return false;
        }
        self.last = Some(operation.clone());
        true
    }

    pub fn has_preview(&self) -> bool {
        self.last.is_some()
    }

    pub fn finish(&self, final_operation: Option<T>) -> Option<DropSample<T>> {
        let operation = final_operation.or_else(|| self.last.clone())?;
        Some(DropSample {
            submit_preview: self.last.as_ref() != Some(&operation),
            operation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_can_supply_the_first_sample() {
        let drag = DragPreview::default();
        assert!(drag.finish(None).is_none());
        let drop = drag.finish(Some(3)).unwrap();
        assert_eq!(drop.operation, 3);
        assert!(drop.submit_preview);
        assert!(!drag.has_preview(), "release does not record a move sample");
    }

    #[test]
    fn release_prefers_fresh_coordinates_and_falls_back_to_last_preview() {
        let mut drag = DragPreview::default();
        assert!(drag.sample(&2));
        assert!(!drag.sample(&2));
        assert!(drag.sample(&4));
        assert!(drag.has_preview());
        for final_operation in [None, Some(4)] {
            let drop = drag.finish(final_operation).unwrap();
            assert_eq!(drop.operation, 4);
            assert!(!drop.submit_preview);
        }
        let drop = drag.finish(Some(2)).unwrap();
        assert_eq!(drop.operation, 2, "a release can revisit an earlier sample");
        assert!(drop.submit_preview);
    }

    #[test]
    fn queued_nudges_and_pointer_drags_have_distinct_admission() {
        let mut model = ReadModel {
            lifecycle: Lifecycle::Ready,
            durability: Durability::Saved { revision: 7 },
            ..ReadModel::default()
        };
        assert!(drag_session_is_ready(&model, Some(7)));
        assert!(!drag_session_is_ready(&model, Some(6)));
        assert!(nudge_session_is_ready(&model));
        for lifecycle in [Lifecycle::Applying, Lifecycle::Saving] {
            model.lifecycle = lifecycle;
            assert!(!drag_session_is_ready(&model, Some(7)));
            assert!(nudge_session_is_ready(&model));
        }
        model.lifecycle = Lifecycle::Ready;
        model.gesture = Some(boardstudio_application::GestureView {
            pointer_id: 1,
            generation: 1,
            target_ids: vec![],
            changed: false,
            snap: false,
            alt: false,
        });
        assert!(!drag_session_is_ready(&model, Some(7)));
        assert!(!nudge_session_is_ready(&model));
    }
}
