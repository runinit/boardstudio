//! Reset and Convert for a departure listed in Relations. They build the same
//! `MatrixTransformRequest` as the Inspector's one-shot buttons and send it through the
//! same `on_edit` path, so there is no second writer of placement parameters.
use super::matrix_transform_controller::MatrixTransformInspectorMount;
use super::matrix_transform_inspector::MatrixTransformRequest;
use crate::matrix_relations::DepartureAction;
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue,
};
use boardstudio_core::model::Vec2;
use dioxus::prelude::*;

/// Whether a departure action can run now, and why not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepartureControl {
    pub label: &'static str,
    pub disabled: bool,
    pub reason: Option<String>,
}

struct Plan {
    field: MatrixTransformField,
    baseline: MatrixTransformValue,
    value: MatrixTransformValue,
    blocked: Option<String>,
}

impl MatrixTransformInspectorMount {
    fn departure_plan(&self, action: DepartureAction) -> Option<Plan> {
        let zero = Vec2 { x: 0.0, y: 0.0 };
        let fields = &self.projection.as_ref()?.fields;
        Some(match (fields, action) {
            (MatrixTransformFields::Row { offset, .. }, DepartureAction::Reset) => Plan {
                field: MatrixTransformField::RowOffsetReset,
                baseline: MatrixTransformValue::Offset(*offset),
                value: MatrixTransformValue::Offset(zero),
                blocked: None,
            },
            (MatrixTransformFields::Column { offset, .. }, DepartureAction::Reset) => Plan {
                field: MatrixTransformField::ColumnOffsetReset,
                baseline: MatrixTransformValue::Offset(*offset),
                value: MatrixTransformValue::Offset(zero),
                blocked: None,
            },
            (
                MatrixTransformFields::Column {
                    offset, convert, ..
                },
                DepartureAction::Convert,
            ) => Plan {
                field: MatrixTransformField::ColumnOffsetConvert,
                baseline: MatrixTransformValue::Number(offset.y),
                value: MatrixTransformValue::Number(0.0),
                blocked: convert.clone().err(),
            },
            (
                MatrixTransformFields::Key {
                    offset, rotation, ..
                },
                DepartureAction::Reset,
            ) => Plan {
                field: MatrixTransformField::KeyTransformReset,
                baseline: MatrixTransformValue::CellTransform {
                    offset: *offset,
                    rotation: *rotation,
                },
                value: MatrixTransformValue::CellTransform {
                    offset: zero,
                    rotation: 0.0,
                },
                blocked: None,
            },
            _ => return None,
        })
    }

    /// The Relations tab stands in for the Inspector form while it is shown, so pending
    /// departure actions settle with feedback rather than retiring.
    pub fn set_inspector_mounted(&self, mounted: bool) {
        let mut signal = self.inspector_mounted;
        if *signal.peek() != mounted {
            signal.set(mounted);
        }
    }

    /// The state of `action` for the selected row, column or key; `None` when the selection
    /// has no such action.
    pub fn departure_control(&self, action: DepartureAction) -> Option<DepartureControl> {
        let plan = self.departure_plan(action)?;
        let label = match plan.field {
            MatrixTransformField::ColumnOffsetConvert => "Convert to stagger",
            // Same wording as the Inspector, so two rows of one key read as one action.
            MatrixTransformField::KeyTransformReset => "Reset local transform",
            _ => "Reset offsets",
        };
        Some(DepartureControl {
            label,
            disabled: !self.editable
                || (self.one_shot_disabled(plan.field))()
                || plan.blocked.is_some(),
            reason: plan.blocked,
        })
    }

    /// The refusal message for the request `submitted` for `action`, if any. The panel
    /// does not yet have a mounted refusal test: every refusal the resolver raises also
    /// removes the selected object, which replaces the panel with its unavailable state.
    pub fn departure_feedback(
        &self,
        action: DepartureAction,
        submitted: Option<u64>,
    ) -> Option<String> {
        let owner = &self.projection.as_ref()?.owner;
        let field = self.departure_plan(action)?.field;
        self.feedback
            .iter()
            .rev()
            .find(|item| {
                item.owner == *owner && item.field == field && submitted == Some(item.request_id)
            })
            .and_then(|item| item.message.clone())
    }

    /// Run `action` through the Inspector's request path. Does nothing when it is disabled;
    /// otherwise returns the submitted request id so the caller can show its feedback.
    pub fn run_departure(&self, action: DepartureAction) -> Option<u64> {
        let plan = self.departure_plan(action)?;
        let projection = self.projection.as_ref()?;
        if self
            .departure_control(action)
            .is_none_or(|control| control.disabled)
        {
            return None;
        }
        let mut sequence = self.request_sequence;
        let id = sequence().checked_add(1)?;
        sequence.set(id);
        self.on_edit.call(MatrixTransformRequest::one_shot(
            projection.owner.clone(),
            (projection.snapshot_token, projection.revision),
            id,
            (plan.field, plan.baseline, plan.value),
            (self.splay_affect)(),
        ));
        Some(id)
    }
}
