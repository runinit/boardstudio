use p2_lifecycle_probe::gesture::{GestureCoordinator, GestureEffect, Point};

#[test]
fn pointerup_coordinates_are_the_committed_final_sample() {
    let mut gesture = GestureCoordinator::default();
    assert!(
        gesture
            .pointer_down(
                7,
                "matrix/main-right-keys/r0c0",
                Point { x: 100.0, y: 80.0 }
            )
            .is_empty()
    );
    assert!(matches!(
        gesture.pointer_up(7, Point { x: 147.5, y: 102.0 }).as_slice(),
        [GestureEffect::Commit { id, at: Point { x, y }, .. }, GestureEffect::Release { pointer_id: 7 }]
            if id == "matrix/main-right-keys/r0c0" && *x == 47.5 && *y == 22.0
    ));
}

#[test]
fn another_pointer_cannot_preview_commit_or_cancel_the_active_drag() {
    let mut gesture = GestureCoordinator::default();
    gesture.pointer_down(7, "switch-1", Point { x: 10.0, y: 15.0 });
    assert!(
        gesture
            .pointer_move(8, Point { x: 30.0, y: 40.0 })
            .is_empty()
    );
    assert!(gesture.pointer_up(8, Point { x: 35.0, y: 45.0 }).is_empty());
    assert!(gesture.pointer_cancel(8).is_empty());
    assert!(gesture.is_active_for(7));
    assert!(matches!(
        gesture.pointer_cancel(7).as_slice(),
        [
            GestureEffect::Cancel { .. },
            GestureEffect::Release { pointer_id: 7 }
        ]
    ));
}

#[test]
fn escape_cancels_and_releases_the_captured_pointer() {
    let mut gesture = GestureCoordinator::default();
    gesture.pointer_down(12, "switch-2", Point { x: 1.0, y: 2.0 });
    assert!(
        matches!(gesture.escape().as_slice(), [GestureEffect::Cancel { id, .. }, GestureEffect::Release { pointer_id: 12 }] if id == "switch-2")
    );
    assert!(!gesture.is_active_for(12));
}

#[test]
fn drag_commit_keeps_pointerdown_revision_transaction_and_origin() {
    let mut gesture = GestureCoordinator::default();
    gesture.pointer_down_with_base(
        44,
        "switch-captured",
        Point { x: 10.0, y: 12.0 },
        71,
        "stable-transaction".into(),
        Point { x: 4.5, y: -8.0 },
    );
    assert!(matches!(
        gesture.pointer_up(44, Point { x: 25.0, y: 32.0 }).as_slice(),
        [GestureEffect::Commit { id, at: Point { x: 15.0, y: 20.0 }, base_revision: 71, transaction_id, origin: Point { x: 4.5, y: -8.0 } }, GestureEffect::Release { pointer_id: 44 }]
            if id == "switch-captured" && transaction_id == "stable-transaction"
    ));
}

#[test]
fn move_then_pointerup_emits_one_commit_using_the_up_sample() {
    let mut gesture = GestureCoordinator::default();
    gesture.pointer_down(2, "switch-3", Point { x: 20.0, y: 30.0 });
    assert!(
        matches!(gesture.pointer_move(2, Point { x: 29.0, y: 37.0 }).as_slice(), [GestureEffect::Preview { at: Point { x, y }, .. }] if *x == 9.0 && *y == 7.0)
    );
    let up = gesture.pointer_up(2, Point { x: 33.0, y: 42.0 });
    assert!(
        matches!(up.as_slice(), [GestureEffect::Commit { at: Point { x, y }, .. }, GestureEffect::Release { pointer_id: 2 }] if *x == 13.0 && *y == 12.0)
    );
    assert!(gesture.pointer_up(2, Point { x: 99.0, y: 99.0 }).is_empty());
}

#[test]
fn stationary_pointerup_releases_without_committing_a_click_as_a_drag() {
    let mut gesture = GestureCoordinator::default();
    gesture.pointer_down(3, "switch-4", Point { x: 5.0, y: 7.0 });
    assert_eq!(
        gesture.pointer_up(3, Point { x: 5.0, y: 7.0 }),
        vec![GestureEffect::Release { pointer_id: 3 }]
    );
}
