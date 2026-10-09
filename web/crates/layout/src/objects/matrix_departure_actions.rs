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
        Some(DepartureControl {
            disabled: !self.editable
                || (self.one_shot_disabled(plan.field))()
                || plan.blocked.is_some(),
            reason: plan.blocked,
        })
    }

    /// Run `action` through the Inspector's request path. Does nothing when it is disabled.
    pub fn run_departure(&self, action: DepartureAction) {
        let Some(plan) = self.departure_plan(action) else {
            return;
        };
        let Some(projection) = self.projection.as_ref() else {
            return;
        };
        if self
            .departure_control(action)
            .is_none_or(|control| control.disabled)
        {
            return;
        }
        let mut sequence = self.request_sequence;
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        self.on_edit.call(MatrixTransformRequest {
            owner: projection.owner.clone(),
            request_id: id,
            snapshot_token: projection.snapshot_token,
            revision: projection.revision,
            field: plan.field,
            baseline: plan.baseline,
            value: plan.value,
            splay_affect: (self.splay_affect)(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: true,
        });
    }
}
