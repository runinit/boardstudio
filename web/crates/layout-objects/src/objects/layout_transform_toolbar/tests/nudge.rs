//! Handle nudges and drag commits through the transform resolvers, without mounting a rail.
use super::*;
use crate::matrix_transform_operation::placement_operation;
use boardstudio_model::model::{MatrixPlacementChange, Mirror};
use boardstudio_web_runtime::operation_outcomes::{EditOutcome, Settlement};
use fixture::{matrix_named, redo, undo};

const TRANSACTION: &str = "layout-transform-test";

fn begin_edit(runtime: &Rc<Runtime>, label: &'static str, resolver: EditResolver) -> EditOutcome {
    runtime.resolve_edit(label, Some("transform".into()), resolver)
}

fn nudge_with(runtime: &Rc<Runtime>, nudge: TransformNudge) -> EditOutcome {
    let resolver = transform_nudge_resolver(fixture::MATRIX_ID.into(), nudge, Vec2::default());
    begin_edit(runtime, "layout-transform-nudge", resolver)
}

fn stagger_nudge(runtime: &Rc<Runtime>, key: ArrowKey, big: bool) -> EditOutcome {
    nudge_with(
        runtime,
        TransformNudge::Stagger {
            column: 0,
            key,
            big,
        },
    )
}

fn drag_edit(runtime: &Rc<Runtime>, matrix_id: &str, result: TransformDragResult) -> EditOutcome {
    let resolver = transform_drag_resolver(matrix_id.into(), TRANSACTION.into(), result);
    begin_edit(runtime, "layout-transform-drag", resolver)
}

fn stagger_result(column: u32, along: f64) -> TransformDragResult {
    TransformDragResult::Placement {
        parameter: PlacementParameter::Stagger { column },
        delta: PlacementDelta::Stagger(along),
    }
}

/// An absolute placement edit, as the Inspector sends it.
fn placement_edit(
    runtime: &Rc<Runtime>,
    parameter: PlacementParameter,
    value: PlacementValue,
) -> EditOutcome {
    let resolver = EditResolver::new("inspector-placement", move |_: &AcceptedSnapshot| {
        let operation = placement_operation(fixture::MATRIX_ID, parameter, value);
        transform_commit(String::new(), fixture::MATRIX_ID, operation)
    });
    begin_edit(runtime, "inspector-placement", resolver)
}

/// Drive the runtime until `ticket` settles, then return its settlement.
async fn settle_edit(runtime: &Rc<Runtime>, ticket: &EditOutcome) -> Settlement {
    fixture::settle_ticket(runtime, ticket).await;
    let settlement = ticket.settlement(true);
    assert!(
        !matches!(settlement, Settlement::Pending),
        "transform edit did not settle"
    );
    settlement
}

async fn run(runtime: &Rc<Runtime>, ticket: EditOutcome) {
    settle_edit(runtime, &ticket).await;
}

/// The fixture document with `edit` applied, opened through the real Session and Core.
async fn open_with(edit: impl FnOnce(&mut Matrix)) -> Rc<Runtime> {
    let mut document = fixture::matrix_document();
    edit(&mut document.matrices[0]);
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    runtime
}

fn matrix_and_poses(runtime: &Rc<Runtime>) -> (Matrix, Vec<(String, Vec2, f64)>) {
    let accepted = runtime.model().accepted.expect("an accepted document");
    let mut poses = accepted
        .document
        .parts
        .iter()
        .map(|part| (part.id.clone(), part.pose.at, part.pose.rotation))
        .collect::<Vec<_>>();
    poses.sort_by(|a, b| a.0.cmp(&b.0));
    (fixture::accepted_matrix(runtime), poses)
}

/// A drag as pointer-down would record it, for the matrix as it is accepted now.
fn begin_gesture(
    runtime: &Rc<Runtime>,
    matrix_id: &str,
    gesture: TransformGesture,
) -> TransformDrag {
    let column = match gesture {
        TransformGesture::Stagger { column, .. }
        | TransformGesture::Offset {
            parameter: PlacementParameter::ColumnOffset { column },
        } => column,
        _ => 0,
    };
    let (matrix, scene) = matrix_and_scene(runtime, matrix_id);
    let capture = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
        .unwrap()
        .unchecked_into::<SvgElement>();
    TransformDrag {
        rail: RailDrag {
            pointer_id: 1,
            capture,
            transaction_id: TRANSACTION.into(),
            owner: owner_of(runtime),
            context: TreeContext::Column {
                matrix_id: matrix_id.into(),
                column,
            },
            tool: LayoutTransformTool::Stagger,
            preview: DragPreview::default(),
        },
        matrix: Rc::new(matrix),
        gesture,
        basis: gesture
            .parameter()
            .and_then(|parameter| SceneBasis::for_parameter(&scene, parameter)),
        origin_base: None,
        start_origin: None,
        start_client_x: 0.0,
        start_client_y: 0.0,
    }
}

fn stagger_drag(runtime: &Rc<Runtime>, matrix_id: &str, column: u32) -> TransformDrag {
    let gesture = TransformGesture::Stagger {
        column,
        quick_offset: false,
    };
    begin_gesture(runtime, matrix_id, gesture)
}

fn offset_drag(
    runtime: &Rc<Runtime>,
    matrix_id: &str,
    parameter: PlacementParameter,
) -> TransformDrag {
    begin_gesture(runtime, matrix_id, TransformGesture::Offset { parameter })
}

async fn preview(runtime: &Rc<Runtime>, drag: &mut TransformDrag, world: Vec2) -> EditOperation {
    let operation = placement_preview(drag, world, None).expect("a preview");
    assert!(drag.rail.preview.sample(&operation));
    drag.rail
        .submit_preview(runtime, &drag.matrix.id, operation.clone());
    support::run_pending(runtime).await;
    operation
}

async fn commit(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: &EditOperation) {
    let result = transform_drag_result(drag, operation).expect("a drag result");
    run(runtime, drag_edit(runtime, &drag.matrix.id, result)).await;
}

/// Preview `world` on `drag` and commit it.
async fn drag_to(runtime: &Rc<Runtime>, mut drag: TransformDrag, world: Vec2) {
    let last = preview(runtime, &mut drag, world).await;
    commit(runtime, &drag, &last).await;
}

fn all_zero_offsets<'a>(offsets: impl IntoIterator<Item = &'a Vec2>) -> bool {
    offsets.into_iter().all(|offset| *offset == Vec2::default())
}

fn staggers(matrix: &Matrix) -> Vec<f64> {
    matrix
        .column_layout
        .iter()
        .map(|record| record.stagger)
        .collect()
}

fn key_shift(matrix: &Matrix, row: u32, column: u32) -> Option<Vec2> {
    matrix
        .cells
        .iter()
        .find(|cell| cell.row == row && cell.column == column)
        .and_then(|cell| cell.shift)
}

fn angle_nudge(affect: MatrixSplayAffect) -> TransformNudge {
    TransformNudge::SplayAngle {
        column: 0,
        degrees: 1.0,
        affect,
    }
}

async fn open_angled_column() -> Rc<Runtime> {
    open_with(|matrix| {
        matrix.column_layout.resize_with(1, Default::default);
        matrix.column_layout[0].splay = 7.0;
        matrix.column_layout[0].rotation = 10.0;
    })
    .await
}

#[wasm_bindgen_test]
async fn column_angle_nudges_compose_without_overwriting_splay() {
    let runtime = open_angled_column().await;
    let column_layout = |runtime: &Rc<Runtime>| fixture::accepted_matrix(runtime).column_layout[0];
    for (affect, splay, rotation) in [
        (MatrixSplayAffect::Column, 7.0, 11.0),
        (MatrixSplayAffect::Column, 7.0, 12.0),
        (MatrixSplayAffect::Following, 8.0, 12.0),
    ] {
        run(&runtime, nudge_with(&runtime, angle_nudge(affect.clone()))).await;
        let record = column_layout(&runtime);
        assert_eq!(
            (record.splay, record.rotation),
            (splay, rotation),
            "{affect:?}"
        );
    }
    undo(&runtime).await;
    assert_eq!(column_layout(&runtime).splay, 7.0);
}

#[wasm_bindgen_test]
async fn column_angle_drag_delta_composes_with_the_latest_rotation() {
    let runtime = open_angled_column().await;
    let gesture = TransformGesture::Splay {
        column: 0,
        start_angle: 0.0,
        origin: Vec2::default(),
        swing: false,
    };
    let drag = begin_gesture(&runtime, fixture::MATRIX_ID, gesture);
    let angle = |angle| EditOperation::SetMatrixSplay {
        matrix_id: fixture::MATRIX_ID.into(),
        column: 0,
        change: MatrixSplayChange::Angle {
            angle,
            affect: MatrixSplayAffect::Column,
        },
    };
    let result = transform_drag_result(&drag, &angle(12.0)).unwrap();
    assert_eq!(
        result,
        TransformDragResult::SplayAngle {
            column: 0,
            delta_degrees: 2.0,
            affect: MatrixSplayAffect::Column,
        }
    );
    assert!(same_transform_as_start(&drag, &angle(10.0)));
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let other = nudge_with(&runtime, angle_nudge(MatrixSplayAffect::Column));
    support::drive_pending(&runtime);
    entered.await.expect("the nudge reached Core");
    let queued = drag_edit(&runtime, fixture::MATRIX_ID, result);
    release.send(()).unwrap();
    settle_edit(&runtime, &other).await;
    settle_edit(&runtime, &queued).await;
    let rotation =
        |runtime: &Rc<Runtime>| fixture::accepted_matrix(runtime).column_layout[0].rotation;
    assert_eq!(rotation(&runtime), 13.0);
    assert_eq!(
        fixture::accepted_matrix(&runtime).column_layout[0].splay,
        7.0
    );
    undo(&runtime).await;
    assert_eq!(rotation(&runtime), 11.0);
    redo(&runtime).await;
    assert_eq!(rotation(&runtime), 13.0);
}

#[wasm_bindgen_test]
async fn an_offset_drag_moves_one_column_by_the_pointer_delta() {
    let runtime = fixture::open_matrix_runtime().await;
    let drag = offset_drag(
        &runtime,
        fixture::MATRIX_ID,
        PlacementParameter::ColumnOffset { column: 0 },
    );
    drag_to(&runtime, drag, Vec2 { x: 2.0, y: 1.0 }).await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.column_layout[0].offset, Vec2 { x: 2.0, y: 1.0 });
    assert!(staggers(&matrix).iter().all(|stagger| *stagger == 0.0));
    assert!(all_zero_offsets(
        matrix.row_layout.iter().map(|record| &record.offset)
    ));
}

#[wasm_bindgen_test]
async fn a_row_offset_drag_and_nudge_change_only_that_rows_offset() {
    let runtime = open_with(|matrix| matrix.rows = 2).await;
    let row = PlacementParameter::RowOffset { row: 1 };
    drag_to(
        &runtime,
        offset_drag(&runtime, fixture::MATRIX_ID, row),
        Vec2 { x: 1.0, y: 2.0 },
    )
    .await;
    assert_eq!(
        fixture::accepted_matrix(&runtime).row_layout[1].offset,
        Vec2 { x: 1.0, y: 2.0 }
    );
    let nudge = TransformNudge::Offset {
        parameter: row,
        key: ArrowKey::Up,
        big: false,
    };
    run(&runtime, nudge_with(&runtime, nudge)).await;
    let matrix = fixture::accepted_matrix(&runtime);
    let offset = matrix.row_layout[1].offset;
    assert!(
        (offset.x - 1.0).abs() < 1e-9 && (offset.y - 2.1).abs() < 1e-9,
        "ArrowUp raises the row by 0.1 mm: {offset:?}"
    );
    assert_eq!(matrix.row_layout[0].offset, Vec2::default());
    assert!(all_zero_offsets(
        matrix.column_layout.iter().map(|record| &record.offset)
    ));
    assert!(staggers(&matrix).iter().all(|stagger| *stagger == 0.0));
}

#[wasm_bindgen_test]
async fn a_column_offset_on_a_splayed_rotated_matrix_follows_the_pointer_in_world_space() {
    let runtime = open_with(|matrix| {
        matrix.rotation = Some(30.0);
        matrix.column_layout.resize_with(3, Default::default);
        matrix.column_layout[1].splay = 20.0;
    })
    .await;
    // The first edit syncs the fixture's unlaid poses; the drag is measured after it.
    let sync = placement_edit(
        &runtime,
        PlacementParameter::ColumnOffset { column: 2 },
        PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
    );
    run(&runtime, sync).await;
    let (before_matrix, before) = matrix_and_poses(&runtime);
    let world = Vec2 { x: 3.0, y: 2.0 };
    let drag = offset_drag(
        &runtime,
        fixture::MATRIX_ID,
        PlacementParameter::ColumnOffset { column: 1 },
    );
    drag_to(&runtime, drag, world).await;
    let (after_matrix, after) = matrix_and_poses(&runtime);
    for ((id, from, _), (_, to, _)) in before.iter().zip(&after) {
        let expected = if id == "matrix/matrix/r0c1" {
            world
        } else {
            Vec2::default()
        };
        assert!(
            (to.x - from.x - expected.x).abs() < 1e-6 && (to.y - from.y - expected.y).abs() < 1e-6,
            "{id} moved by ({}, {}), expected {expected:?}",
            to.x - from.x,
            to.y - from.y
        );
    }
    assert_eq!(staggers(&after_matrix), staggers(&before_matrix));
    assert_eq!(after_matrix.row_layout, before_matrix.row_layout);
}

#[wasm_bindgen_test]
async fn a_drag_for_a_column_or_row_that_has_gone_is_refused_before_core() {
    let runtime = fixture::open_matrix_runtime().await;
    let revision_before = revision(&runtime);
    for parameter in [
        PlacementParameter::ColumnOffset { column: 9 },
        PlacementParameter::Stagger { column: 9 },
        PlacementParameter::RowOffset { row: 5 },
    ] {
        let delta = match parameter {
            PlacementParameter::Stagger { .. } => PlacementDelta::Stagger(1.0),
            _ => PlacementDelta::Offset(Vec2 { x: 1.0, y: 0.0 }),
        };
        let result = TransformDragResult::Placement { parameter, delta };
        let ticket = drag_edit(&runtime, fixture::MATRIX_ID, result);
        let outcome = settle_edit(&runtime, &ticket).await;
        assert!(
            matches!(
                &outcome,
                Settlement::Failed { message } if message.contains("no longer exists")
            ),
            "{parameter:?} must be refused by the resolver, got {outcome:?}"
        );
    }
    assert_eq!(revision(&runtime), revision_before, "nothing was written");
}

#[wasm_bindgen_test]
async fn a_stagger_snaps_to_the_grid_step_of_its_axis_dominant_component() {
    let runtime = fixture::open_matrix_runtime().await;
    let mut drag = stagger_drag(&runtime, fixture::MATRIX_ID, 1);
    let step = Some(Vec2 { x: 4.0, y: 9.0 });
    let value = |drag: &TransformDrag, world: Vec2| {
        let Some(EditOperation::SetMatrixPlacement {
            change: MatrixPlacementChange::ColumnStagger { value, .. },
            ..
        }) = placement_preview(drag, world, step)
        else {
            panic!("a ColumnStagger preview");
        };
        value
    };
    assert!(
        (value(&drag, Vec2 { x: 0.0, y: 5.0 }) - 9.0).abs() < 1e-9,
        "an upright axis snaps to the y step"
    );
    let (x, y) = (Vec2 { x: 1.0, y: 0.0 }, Vec2 { x: 0.0, y: 1.0 });
    drag.basis = Some(SceneBasis {
        stagger_axis: x,
        axis_x: x,
        axis_y: y,
        shift_axis_x: x,
        shift_axis_y: y,
        spread_axis: None,
    });
    assert!(
        (value(&drag, Vec2 { x: 5.0, y: 0.0 }) - 4.0).abs() < 1e-9,
        "a horizontal axis snaps to the x step"
    );
}

#[wasm_bindgen_test]
async fn three_handle_nudges_queued_behind_a_gated_reply_move_the_matrix_three_steps() {
    let runtime = fixture::open_matrix_runtime().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let first = stagger_nudge(&runtime, ArrowKey::Up, false);
    support::drive_pending(&runtime);
    entered.await.expect("the first nudge reached Core");
    let second = stagger_nudge(&runtime, ArrowKey::Up, false);
    let third = stagger_nudge(&runtime, ArrowKey::Up, false);
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    for edits in [&third, &first, &second] {
        settle_edit(&runtime, edits).await;
    }
    let matrix = fixture::accepted_matrix(&runtime);
    assert!(
        (matrix.column_layout[0].stagger - 0.3).abs() < 1e-9,
        "three steps of 0.1 compose to a stagger of 0.3, got {}",
        matrix.column_layout[0].stagger
    );
    assert!(
        all_zero_offsets(matrix.column_layout.iter().map(|record| &record.offset)),
        "the offsets are untouched"
    );
}

#[wasm_bindgen_test]
async fn a_drag_commit_applies_its_net_transform_to_the_accepted_matrix() {
    let runtime = fixture::open_matrix_runtime().await;
    // Another stagger edit lands while the drag was in flight; the drag's net change
    // composes with it instead of restoring the matrix the drag started from.
    let other = placement_edit(
        &runtime,
        PlacementParameter::Stagger { column: 0 },
        PlacementValue::Stagger(1.0),
    );
    run(&runtime, other).await;
    run(
        &runtime,
        drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(0, 2.0)),
    )
    .await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.column_layout[0].stagger, 3.0);
    assert!(all_zero_offsets(
        matrix.column_layout.iter().map(|record| &record.offset)
    ));
}

#[wasm_bindgen_test]
async fn cancel_and_escape_land_nothing() {
    let runtime = fixture::open_matrix_runtime().await;
    run(&runtime, stagger_nudge(&runtime, ArrowKey::Up, true)).await;
    let before = matrix_and_poses(&runtime);
    let revision_before = revision(&runtime);
    let mut drag = stagger_drag(&runtime, fixture::MATRIX_ID, 1);
    preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 3.0 }).await;
    assert!(
        runtime.model().display_preview.is_some(),
        "the drag is previewing"
    );
    let arbiter = CanvasInteractionArbiter::default();
    assert!(arbiter.try_acquire(CanvasInteractionOwner::MatrixTransform));
    // Escape, pointer cancel and lost capture all end in this.
    let cell = Rc::new(RefCell::new(Some(drag)));
    assert!(abandon_transform_drag(&runtime, &cell, &arbiter));
    support::run_pending(&runtime).await;
    assert!(runtime.model().display_preview.is_none());
    assert_eq!(arbiter.current(), None, "the canvas is released");
    assert_eq!(revision(&runtime), revision_before);
    assert_eq!(matrix_and_poses(&runtime), before);
    // The Undo depth is unchanged: one Undo reverts only the earlier nudge.
    undo(&runtime).await;
    assert!(
        staggers(&fixture::accepted_matrix(&runtime))
            .iter()
            .all(|stagger| *stagger == 0.0),
        "the cancelled drag left no Undo step behind"
    );
}

#[wasm_bindgen_test]
async fn one_drag_is_one_undo_step_and_redo_restores_it() {
    let runtime = fixture::open_matrix_runtime().await;
    let before = matrix_and_poses(&runtime);
    let revision_before = revision(&runtime);
    let mut drag = stagger_drag(&runtime, fixture::MATRIX_ID, 1);
    preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 1.0 }).await;
    let last = preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 4.0 }).await;
    commit(&runtime, &drag, &last).await;
    let after = matrix_and_poses(&runtime);
    assert_eq!(after.0.column_layout[1].stagger, 4.0);
    assert_eq!(revision(&runtime), revision_before + 1, "one accepted edit");
    undo(&runtime).await;
    assert_eq!(staggers(&matrix_and_poses(&runtime).0), staggers(&before.0));
    redo(&runtime).await;
    assert_eq!(matrix_and_poses(&runtime), after, "Redo restores the drag");
}

#[wasm_bindgen_test]
async fn a_drag_composes_with_an_unrelated_edit_held_in_flight() {
    let runtime = fixture::open_matrix_runtime().await;
    let (entered, release) = support::gate_next_core_reply(&runtime);
    let unrelated = placement_edit(
        &runtime,
        PlacementParameter::ColumnOffset { column: 2 },
        PlacementValue::Offset(Vec2 { x: 1.0, y: 0.0 }),
    );
    support::drive_pending(&runtime);
    entered.await.expect("the unrelated edit reached Core");
    let drag = drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(1, 2.0));
    support::drive_pending(&runtime);
    release.send(()).expect("release the held reply");
    settle_edit(&runtime, &drag).await;
    settle_edit(&runtime, &unrelated).await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.column_layout[1].stagger, 2.0);
    assert_eq!(matrix.column_layout[2].offset, Vec2 { x: 1.0, y: 0.0 });
}

#[wasm_bindgen_test]
async fn arrow_up_moves_the_column_up_on_screen_on_a_mirror_y_matrix() {
    let runtime = open_with(|matrix| matrix.mirror = Some(Mirror::Y)).await;
    run(&runtime, stagger_nudge(&runtime, ArrowKey::Up, false)).await;
    let key_y = fixture::part_named(&runtime, "matrix/matrix/r0c1")
        .pose
        .at
        .y;
    assert!(
        (key_y - 0.1).abs() < 1e-9,
        "ArrowUp raises the column by 0.1 mm on screen, got {key_y}"
    );
    assert!(
        fixture::accepted_matrix(&runtime).column_layout[0].stagger < 0.0,
        "the stagger value falls because the matrix is mirrored"
    );
    // The pair across the stagger axis is not consumed.
    let (_, scene) = matrix_and_scene(&runtime, fixture::MATRIX_ID);
    let stagger = PlacementParameter::Stagger { column: 0 };
    let basis = SceneBasis::for_parameter(&scene, stagger).expect("column 0 has a basis");
    let value = PlacementValue::Stagger(0.0);
    assert_eq!(
        nudge_value(&basis, stagger, value, ArrowKey::Left, false),
        None
    );
}

/// A linked left/right pair on the fixture board, for the mirror-sync drag tests.
async fn open_mirrored_pair_runtime() -> Rc<Runtime> {
    let mut document = fixture::matrix_document();
    document.matrices.clear();
    document.parts.clear();
    document.boards[0].part_ids.clear();
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    let operation: EditOperation = serde_json::from_value(serde_json::json!({
        "kind": "create-mirrored-pair",
        "matrix": {
            "id": "left-matrix", "rows": 1, "columns": 3,
            "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": -20.0, "y": 0.0},
            "mirror": "x", "definitionId": "switch:base",
            "boardId": fixture::BOARD_ID, "partIds": []
        },
        "left": {"id": "left-layout", "name": "Left", "boardId": fixture::BOARD_ID,
                 "matrixIds": ["left-matrix"], "partIds": []},
        "right": {"id": "right-layout", "name": "Right", "boardId": fixture::BOARD_ID,
                  "matrixIds": ["right-matrix"], "partIds": [],
                  "mirrorLink": {"sourceId": "left-layout", "axisX": 10.0}}
    }))
    .expect("a mirrored pair operation");
    let resolver = EditResolver::new("pair", move |_: &AcceptedSnapshot| {
        Resolution::submit(vec![], operation.clone())
    });
    run(&runtime, begin_edit(&runtime, "pair", resolver)).await;
    runtime
}

/// A drag on the right half of a linked pair updates both halves equally: Core mirrors the
/// carried stagger and offsets, and the carried row shift, onto the left half.
#[wasm_bindgen_test]
async fn drags_on_the_right_half_of_a_linked_pair_update_both_halves_equally() {
    let runtime = open_mirrored_pair_runtime().await;
    let both = || {
        (
            matrix_named(&runtime, "left-matrix"),
            matrix_named(&runtime, "right-matrix"),
        )
    };
    drag_to(
        &runtime,
        stagger_drag(&runtime, "right-matrix", 1),
        Vec2 { x: 0.0, y: 2.0 },
    )
    .await;
    let (left, right) = both();
    assert_eq!(right.column_layout[1].stagger, 2.0);
    assert_eq!(staggers(&left), staggers(&right));
    let offsets = |matrix: &Matrix| {
        matrix
            .column_layout
            .iter()
            .map(|record| record.offset)
            .collect::<Vec<_>>()
    };
    assert_eq!(offsets(&left), offsets(&right));

    let shift = PlacementParameter::RowShift { row: 0 };
    drag_to(
        &runtime,
        offset_drag(&runtime, "right-matrix", shift),
        Vec2 { x: 4.7625, y: 0.0 },
    )
    .await;
    let (left, right) = both();
    assert_eq!(right.row_layout[0].shift, Vec2 { x: 4.7625, y: 0.0 });
    let shifts = |matrix: &Matrix| {
        matrix
            .row_layout
            .iter()
            .map(|record| record.shift)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        shifts(&left),
        shifts(&right),
        "Core mirrors the carried shift onto the linked half"
    );
}

/// Ticket 05: a row Shift drag carries the row through the shift axes and a row Bend drag
/// turns its running frame; a Bend nudge adds degrees like the Splay nudge. Only the dragged
/// row changes. The resolver-level preview carries no grid step, so the raw world delta
/// lands; the mounted pointer path covers snapping.
#[wasm_bindgen_test]
async fn a_row_shift_drag_and_a_row_bend_drag_change_only_that_row() {
    let runtime = open_with(|matrix| matrix.rows = 2).await;
    let shift = PlacementParameter::RowShift { row: 1 };
    drag_to(
        &runtime,
        offset_drag(&runtime, fixture::MATRIX_ID, shift),
        Vec2 { x: 8.0, y: 4.0 },
    )
    .await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.row_layout[1].shift, Vec2 { x: 8.0, y: 4.0 });
    assert_eq!(matrix.row_layout[0].shift, Vec2::default());

    let bend = PlacementParameter::RowBend { row: 1 };
    let drag = begin_gesture(
        &runtime,
        fixture::MATRIX_ID,
        TransformGesture::Bend { parameter: bend },
    );
    // Ten millimetres right turns the running frame five degrees.
    drag_to(&runtime, drag, Vec2 { x: 10.0, y: 0.0 }).await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.row_layout[1].bend, -5.0);
    assert_eq!(matrix.row_layout[0].bend, 0.0);

    // A Bend nudge adds degrees without touching the shift.
    let nudge = TransformNudge::Bend {
        parameter: bend,
        degrees: 1.0,
    };
    run(&runtime, nudge_with(&runtime, nudge)).await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.row_layout[1].bend, -4.0);
    assert_eq!(matrix.row_layout[1].shift, Vec2 { x: 8.0, y: 4.0 });
}

/// Ticket 05: a key Shift drag moves only that key's carried shift.
#[wasm_bindgen_test]
async fn a_key_shift_drag_moves_only_that_keys_shift() {
    let runtime = fixture::open_matrix_runtime().await;
    let parameter = PlacementParameter::KeyShift { row: 0, column: 1 };
    drag_to(
        &runtime,
        offset_drag(&runtime, fixture::MATRIX_ID, parameter),
        Vec2 { x: 8.0, y: 4.0 },
    )
    .await;
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(key_shift(&matrix, 0, 1), Some(Vec2 { x: 8.0, y: 4.0 }));
    assert!(
        matrix
            .cells
            .iter()
            .filter(|cell| !(cell.row == 0 && cell.column == 1))
            .all(|cell| cell.shift.is_none()),
        "only the dragged key carries a shift"
    );
}
