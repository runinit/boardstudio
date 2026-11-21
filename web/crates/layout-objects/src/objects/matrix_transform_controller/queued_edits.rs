use super::*;
use crate::presentation::objects::matrix_edit_test_support as fixture;
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use boardstudio_web_test_runtime::project_name_test_support as support;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn origin_ticket(runtime: &Rc<Runtime>, field: MatrixTransformField, value: f64) -> EditTicket {
    EditTicket::begin(
        runtime,
        "layout-matrix-transform",
        Some("transform".into()),
        transform_edit_resolver(
            fixture::MATRIX_ID.into(),
            fixture::BOARD_ID.into(),
            TreeContext::Matrix {
                matrix_id: fixture::MATRIX_ID.into(),
            },
            field,
            MatrixTransformValue::Number(value),
            MatrixSplayAffect::Column,
            Vec::new(),
        ),
    )
}

#[wasm_bindgen_test]
async fn two_transform_fields_committed_back_to_back_both_survive() {
    let runtime = fixture::open_matrix_runtime().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let x = origin_ticket(&runtime, MatrixTransformField::OriginX, 5.0);
    support::drive_pending(&runtime);
    entered.await.expect("the X edit reached Core");
    let y = origin_ticket(&runtime, MatrixTransformField::OriginY, 7.0);
    support::drive_pending(&runtime);
    release.send(()).expect("release the held X reply");
    fixture::settle_ticket(&runtime, &y).await;

    assert!(matches!(x.settlement(true), Settlement::Landed { .. }));
    assert!(matches!(y.settlement(true), Settlement::Landed { .. }));
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(
        matrix.origin,
        Vec2 { x: 5.0, y: 7.0 },
        "the Y edit resolved against the document the X edit produced"
    );
}

#[wasm_bindgen_test]
async fn a_committed_transform_value_wins_over_a_changed_baseline() {
    let runtime = fixture::open_matrix_runtime().await;
    // Another edit moves the origin first; the later commit still lands its own value.
    let first = origin_ticket(&runtime, MatrixTransformField::OriginX, 3.0);
    fixture::settle_ticket(&runtime, &first).await;
    let second = origin_ticket(&runtime, MatrixTransformField::OriginX, 9.0);
    fixture::settle_ticket(&runtime, &second).await;
    assert!(matches!(second.settlement(true), Settlement::Landed { .. }));
    assert_eq!(fixture::accepted_matrix(&runtime).origin.x, 9.0);
    // Committing the value that is already accepted resolves Unchanged: it lands without
    // a new revision.
    let revision = runtime.model().accepted.unwrap().document.revision;
    let same = origin_ticket(&runtime, MatrixTransformField::OriginX, 9.0);
    fixture::settle_ticket(&runtime, &same).await;
    assert!(matches!(same.settlement(true), Settlement::Landed { .. }));
    assert_eq!(
        runtime.model().accepted.unwrap().document.revision,
        revision
    );
}
