use super::projection::transform_fields;
use super::*;

pub(super) fn field_value(
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    splay_affect: &MatrixSplayAffect,
) -> Option<MatrixTransformValue> {
    use MatrixTransformField as Field;
    use MatrixTransformValue as Value;
    match (fields, field) {
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginX) => {
            Some(Value::Number(origin.x))
        }
        (MatrixTransformFields::Matrix { origin, .. }, Field::OriginY) => {
            Some(Value::Number(origin.y))
        }
        (MatrixTransformFields::Matrix { rotation, .. }, Field::MatrixRotation) => {
            Some(Value::Number(*rotation))
        }
        (MatrixTransformFields::Matrix { mirror, .. }, Field::MatrixMirror) => {
            Some(Value::Mirror(*mirror))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Row { offset, .. }, Field::RowOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        (MatrixTransformFields::Row { shift, .. }, Field::RowShiftX) => {
            Some(Value::Number(shift.x))
        }
        (MatrixTransformFields::Row { shift, .. }, Field::RowShiftY) => {
            Some(Value::Number(shift.y))
        }
        (MatrixTransformFields::Row { bend, .. }, Field::RowBend) => Some(Value::Number(*bend)),
        (MatrixTransformFields::Row { padding, pitch, .. }, Field::RowPadding) => {
            Some(Value::Number(padding.unwrap_or(*pitch)))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetReset) => {
            Some(Value::Offset(*offset))
        }
        // The action is done once the offset Y is zero, so the resolver reports
        // `Unchanged` against a button value of 0.
        (MatrixTransformFields::Column { offset, .. }, Field::ColumnOffsetConvert) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Column { stagger, .. }, Field::ColumnStagger) => {
            Some(Value::Number(*stagger))
        }
        (MatrixTransformFields::Column { spread, pitch, .. }, Field::ColumnSpread) => {
            Some(Value::Number(spread.unwrap_or(*pitch)))
        }
        (
            MatrixTransformFields::Column {
                splay_angle,
                rotation,
                ..
            },
            Field::ColumnSplay,
        ) => Some(Value::Number(column_angle(
            *splay_angle,
            *rotation,
            splay_affect,
        ))),
        (MatrixTransformFields::Column { custom_origin, .. }, Field::SplayOriginMode) => {
            Some(Value::OriginMode(*custom_origin))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginX) => {
            Some(Value::Number(splay_origin.x))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginY) => {
            Some(Value::Number(splay_origin.y))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginPoint) => {
            Some(Value::Point(*splay_origin))
        }
        (MatrixTransformFields::Column { splay_origin, .. }, Field::SplayOriginSwingPoint) => {
            Some(Value::Point(*splay_origin))
        }
        (
            MatrixTransformFields::Matrix {
                anchor: Some(anchor),
                ..
            },
            Field::MatrixAnchorMethod,
        ) => anchor
            .aggregate
            .as_ref()
            .map(|aggregate| Value::AnchorMethod(aggregate.method)),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(anchor),
                ..
            },
            Field::MatrixAnchorParts,
        ) => anchor
            .aggregate
            .as_ref()
            .map(|aggregate| Value::AnchorParts(aggregate.parts.clone())),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(anchor),
                ..
            },
            Field::MatrixFollowX,
        ) => Some(Value::Bool(anchor.follow.x)),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(anchor),
                ..
            },
            Field::MatrixFollowY,
        ) => Some(Value::Bool(anchor.follow.y)),
        (
            MatrixTransformFields::Matrix {
                anchor: Some(anchor),
                ..
            },
            Field::MatrixFollowRotation,
        ) => Some(Value::Bool(anchor.follow.rotation)),
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetX) => {
            Some(Value::Number(offset.x))
        }
        (MatrixTransformFields::Key { offset, .. }, Field::KeyOffsetY) => {
            Some(Value::Number(offset.y))
        }
        (MatrixTransformFields::Key { rotation, .. }, Field::KeyRotation) => {
            Some(Value::Number(*rotation))
        }
        (MatrixTransformFields::Key { shift, .. }, Field::KeyShiftX) => {
            Some(Value::Number(shift.x))
        }
        (MatrixTransformFields::Key { shift, .. }, Field::KeyShiftY) => {
            Some(Value::Number(shift.y))
        }
        (MatrixTransformFields::Key { bend, .. }, Field::KeyBend) => Some(Value::Number(*bend)),
        (MatrixTransformFields::Key { enabled, .. }, Field::KeyEnabled) => {
            Some(Value::Bool(*enabled))
        }
        (MatrixTransformFields::Key { half_exception, .. }, Field::KeyOmit) => half_exception
            .as_ref()
            .map(|exception| Value::Bool(exception.omitted)),
        (MatrixTransformFields::Key { assemblies, .. }, Field::KeyAttached) => {
            Some(Value::Attached(assemblies.clone()))
        }
        (MatrixTransformFields::Key { definition_id, .. }, Field::KeyAssembly) => {
            Some(Value::Text(definition_id.clone()))
        }
        (
            MatrixTransformFields::Key {
                assemblies_local, ..
            },
            Field::KeyAssembliesLocal,
        ) => Some(Value::Bool(*assemblies_local)),
        (
            MatrixTransformFields::Key {
                offset, rotation, ..
            },
            Field::KeyTransformReset,
        ) => Some(Value::CellTransform {
            offset: *offset,
            rotation: *rotation,
        }),
        _ => None,
    }
}

pub(super) fn accepted_field_text(runtime: &Runtime, request: &MatrixTransformRequest) -> String {
    let Some(snapshot) = runtime.model().accepted else {
        return String::new();
    };
    let Some(matrix) = snapshot
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == request.owner.matrix_id)
    else {
        return String::new();
    };
    transform_fields(
        &snapshot,
        &request.owner.scope.board_id,
        &request.owner.context,
        matrix,
    )
    .and_then(|fields| field_value(&fields, request.field, &request.splay_affect))
    .and_then(|value| match value {
        MatrixTransformValue::Number(value) => Some(value.to_string()),
        _ => None,
    })
    .unwrap_or_default()
}

pub(super) fn publish_request_failure(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    request: &MatrixTransformRequest,
    message: Option<String>,
) {
    if let Some(mut failure) = request.failure {
        failure.set(message);
    } else {
        publish_feedback(
            feedback,
            &request.owner,
            request.request_id,
            request.field,
            message,
        );
    }
}

pub(super) fn publish_feedback(
    feedback: &mut Signal<Vec<MatrixTransformFeedback>>,
    owner: &MatrixTransformInspectorOwner,
    request_id: u64,
    field: MatrixTransformField,
    message: Option<String>,
) {
    let next = MatrixTransformFeedback {
        owner: owner.clone(),
        request_id,
        field,
        message,
    };
    let mut entries = feedback.read().clone();
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.owner == next.owner && entry.request_id == next.request_id)
    {
        *existing = next;
    } else {
        entries.push(next);
    }
    while entries.len() > 8 {
        let Some(index) = entries.iter().position(|entry| entry.message.is_some()) else {
            break;
        };
        entries.remove(index);
    }
    feedback.set(entries);
}
