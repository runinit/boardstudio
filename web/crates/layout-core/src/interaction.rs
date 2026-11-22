//! Pointer ownership and preview transitions for Layout transform handles.

use boardstudio_model::model::EditOperation;

#[derive(Clone, Default)]
pub enum CanvasInteraction {
    #[default]
    Idle,
    Transforming {
        pointer: i32,
        preview: Option<EditOperation>,
    },
}

pub struct TransformDrop {
    pub operation: EditOperation,
    pub needs_preview: bool,
}

impl CanvasInteraction {
    pub fn begin_transform(&mut self, pointer: i32) -> bool {
        if !matches!(self, Self::Idle) {
            return false;
        }
        *self = Self::Transforming {
            pointer,
            preview: None,
        };
        true
    }

    pub fn pointer(&self) -> Option<i32> {
        match self {
            Self::Idle => None,
            Self::Transforming { pointer, .. } => Some(*pointer),
        }
    }

    /// Return whether this sample needs to be sent to the preview renderer.
    pub fn sample(&mut self, pointer: i32, operation: &EditOperation) -> bool {
        if let Self::Transforming {
            pointer: owner,
            preview,
        } = self
            && *owner == pointer
            && preview.as_ref() != Some(operation)
        {
            *preview = Some(operation.clone());
            return true;
        }
        false
    }

    /// A release can supply the first sample or reuse the last preview if
    /// the surface no longer provides coordinates.
    pub fn finish(
        &mut self,
        pointer: i32,
        operation: Option<EditOperation>,
    ) -> Option<TransformDrop> {
        if self.pointer() != Some(pointer) {
            return None;
        }
        let preview = self.take_preview();
        let needs_preview = operation.is_some() && operation != preview;
        Some(TransformDrop {
            operation: operation.or(preview)?,
            needs_preview,
        })
    }

    /// Escape, lost capture, and scope or tool changes retire the interaction.
    /// The return value tells the caller whether to clear a submitted preview.
    pub fn cancel(&mut self) -> bool {
        self.take_preview().is_some()
    }

    fn take_preview(&mut self) -> Option<EditOperation> {
        match std::mem::take(self) {
            Self::Idle => None,
            Self::Transforming { preview, .. } => preview,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(id: &str) -> EditOperation {
        EditOperation::RemoveMatrix { id: id.to_owned() }
    }

    #[test]
    fn second_pointer_cannot_change_or_end_a_transform() {
        let mut canvas = CanvasInteraction::default();
        assert!(canvas.begin_transform(1));
        assert!(!canvas.begin_transform(2));
        assert!(canvas.sample(1, &operation("preview")));
        assert!(!canvas.sample(2, &operation("other")));
        assert!(canvas.finish(2, Some(operation("other"))).is_none());
        let committed = canvas.finish(1, None).unwrap();
        assert_eq!(committed.operation, operation("preview"));
        assert!(!committed.needs_preview);
        assert!(canvas.finish(1, None).is_none());
    }

    #[test]
    fn cancel_retires_the_preview_and_allows_a_new_drag() {
        let mut canvas = CanvasInteraction::default();
        canvas.begin_transform(1);
        assert!(canvas.sample(1, &operation("preview")));
        assert!(!canvas.sample(1, &operation("preview")));
        assert!(canvas.cancel());
        assert!(canvas.finish(1, Some(operation("preview"))).is_none());
        assert!(canvas.begin_transform(2));
        let committed = canvas.finish(2, Some(operation("release"))).unwrap();
        assert_eq!(committed.operation, operation("release"));
        assert!(committed.needs_preview);
    }
}
