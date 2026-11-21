use super::*;

#[test]
fn consolidated_fresh_names_a_nonfinite_constraint_of_every_kind() {
    for kind in ["offset", "part-offset", "mirror"] {
        for nonfinite in [f64::INFINITY, f64::NAN] {
            let mut before = compaction_document().unwrap();
            let existing = before
                .constraints
                .iter_mut()
                .find(|c| c.id() == "retained-second-key-offset")
                .unwrap();
            let (id, source_part_id, target_part_id) = (
                existing.id().to_owned(),
                existing.source().to_owned(),
                existing.target().to_owned(),
            );
            *existing = match kind {
                "offset" => Constraint::Offset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 {
                        x: nonfinite,
                        y: 1.0,
                    },
                    rotation: 0.0,
                },
                "part-offset" => Constraint::PartOffset {
                    id,
                    source_part_id,
                    target_part_id,
                    offset: Vec2 { x: 1.0, y: 1.0 },
                    rotation: nonfinite,
                },
                _ => Constraint::Mirror {
                    id,
                    source_part_id,
                    target_part_id,
                    axis: MirrorAxis::Vertical,
                    coordinate: nonfinite,
                },
            };
            let message = crate::support::prepare_transaction(
                &before,
                &[],
                &EditOperation::RemoveMatrixColumn {
                    matrix_id: "source-matrix".into(),
                    column: 1,
                },
            )
            .unwrap_err();
            assert!(
                message.contains("retained-second-key-offset") && message.contains("non-finite"),
                "{kind} {nonfinite}: {message}"
            );
        }
    }
}
