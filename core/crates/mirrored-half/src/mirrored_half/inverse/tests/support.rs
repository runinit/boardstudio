use super::{Canonical, Refusal, canonicalize_with_dispatch};
use crate::constraints::{ConstraintEndpoint, FramedConstraint};
use crate::model::{EditOperation, Position, ProjectDoc, Vec2};

pub(super) fn canonicalize(
    document: &ProjectDoc,
    operation: &EditOperation,
) -> Result<Canonical, Refusal> {
    canonicalize_with_dispatch(
        document,
        operation,
        super::super::EditDispatch {
            apply_with_ledger: |_, _, _, _| {
                panic!("engine-backed translation belongs in the engine integration tests")
            },
            apply_unscripted: |_, _| {
                panic!("engine-backed translation belongs in the engine integration tests")
            },
        },
    )
}

pub(super) fn only_upsert(result: &Canonical) -> &FramedConstraint {
    let [super::constraints::PreparedConstraintEdit::Upsert(record)] =
        result.prepared_constraints.as_slice()
    else {
        panic!(
            "expected one prepared upsert, got {:?}",
            result.prepared_constraints
        );
    };
    record
}

pub(super) fn move_positions<const N: usize>(positions: [(&str, f64, f64); N]) -> EditOperation {
    EditOperation::MoveParts {
        positions: positions
            .into_iter()
            .map(|(id, x, y)| Position {
                id: id.into(),
                at: Vec2 { x, y },
            })
            .collect(),
    }
}

pub(super) fn local_endpoint(id: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::Part { part_id: id.into() }
}

pub(super) fn linked_endpoint(source: &str, mirror: &str) -> ConstraintEndpoint {
    ConstraintEndpoint::LinkedMirror {
        pair_id: "pair".into(),
        source_part_id: source.into(),
        mirror_part_id: mirror.into(),
    }
}
