//! Session admission and preview progression for the canvas transform rails.

use boardstudio_application::{Durability, Lifecycle, ReadModel};

pub fn drag_session_is_ready(model: &ReadModel, revision: Option<u64>) -> bool {
    model.lifecycle == Lifecycle::Ready
        && revision.is_some_and(|revision| model.durability == Durability::Saved { revision })
        && pointer_edit_is_idle(model)
}

/// A nudge is queued intent and can compose with an edit already saving.
pub fn nudge_session_is_ready(model: &ReadModel) -> bool {
    model.lifecycle == Lifecycle::Ready && pointer_edit_is_idle(model)
}

fn pointer_edit_is_idle(model: &ReadModel) -> bool {
    model.display_preview.is_none() && model.gesture.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        model.durability = Durability::Saving { revision: 7 };
        assert!(!drag_session_is_ready(&model, Some(7)));
        assert!(nudge_session_is_ready(&model));
        model.durability = Durability::Saved { revision: 7 };
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
