//! Ticket 07: the part rail overlay for anchored, detached, mirrored and locked parts.
use super::*;
use fixture::{part_named, undo};

fn part_anchor_of(runtime: &Rc<Runtime>, part_id: &str) -> boardstudio_model::model::PartAnchor {
    part_named(runtime, part_id)
        .anchor
        .expect("the part is anchored")
}

fn popup_text(root: &web_sys::Element) -> String {
    text_of(root, "[data-feedback-popup]")
}

/// Ticket 07: an anchored part's rail draws Move, Rotate, Spread and Stagger left to right
/// whatever the mount order, and hovering the Spread token docks the carried value in the
/// popup.
#[wasm_bindgen_test]
async fn part_rail_orders_move_rotate_spread_stagger_with_live_values() {
    use LayoutTransformTool::{Move, Rotate, Spread, Stagger};
    let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
    wait_until_editable(&runtime).await;
    let rail = mount_part_rail(
        &runtime,
        "mcu",
        vec![Stagger, Move, Spread, Rotate],
        part_snap(),
    )
    .await;
    let mut placed = query_all(&rail.root, "[data-transform-handle]")
        .iter()
        .map(|handle| {
            let label = handle.get_attribute("aria-label").unwrap_or_default();
            (fixture::translate_of(handle).0, label)
        })
        .collect::<Vec<_>>();
    placed.sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap());
    let labels = placed
        .iter()
        .map(|(_, label)| label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        [
            "Drag to move part",
            "Drag to rotate part",
            "Drag to spread",
            "Drag to stagger",
        ]
    );
    let spread = query(&rail.root, "[aria-label='Drag to spread']").expect("the Spread token");
    dispatch_pointer(&spread, "pointerenter", false, false, 0, (100, 100));
    frame().await;
    let text = popup_text(&rail.root);
    assert!(
        text.contains("Spread") && text.contains("19.00 mm"),
        "the popup names the carried value: {text}"
    );
}

/// Ticket 07: dragging the anchored part's Stagger token edits only the carried Stagger, and
/// its Move token only the local Offset; each commit is one revision with its toast, and one
/// Undo restores it. One client pixel is a quarter world mm; Alt skips the grid so the drags
/// land exact.
#[wasm_bindgen_test]
async fn anchored_part_stagger_and_move_drags_commit_their_own_values_with_one_undo() {
    for (tool, to, toast) in [
        (
            LayoutTransformTool::Stagger,
            (38.05, 2.0),
            "Stagger 2.00 mm (+2.00 mm) · 1 Undo step",
        ),
        (
            LayoutTransformTool::Move,
            (40.05, 2.0),
            "Move X 2.00, Y 2.00 mm (+2.00, +2.00 mm) · 1 Undo step",
        ),
    ] {
        let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
        wait_until_editable(&runtime).await;
        let rail = mount_part_tool(&runtime, "mcu", tool).await;
        let handle = rail.handle().expect("the tool draws a handle");
        let client = rail.client();
        let before = revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(38.05, 0.0));
        frame().await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(to.0, to.1));
        frame().await;
        if tool == LayoutTransformTool::Stagger {
            assert!(
                popup_text(&rail.root).contains("+2.00 mm"),
                "the popup follows the drag with the live delta"
            );
        }
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(to.0, to.1));
        pump_past(&runtime, before).await;
        assert_eq!(revision(&runtime), before + 1, "one accepted edit");
        let anchor = part_anchor_of(&runtime, "mcu");
        let (stagger, offset) = match tool {
            LayoutTransformTool::Stagger => (2.0, Vec2::default()),
            _ => (0.0, Vec2 { x: 2.0, y: 2.0 }),
        };
        assert_eq!(
            (anchor.stagger, anchor.spread, anchor.offset),
            (stagger, 19.0, offset),
            "{tool:?} edits only its own value"
        );
        assert_eq!(toast_text(&runtime, &rail.root).await, toast);
        undo(&runtime).await;
        let anchor = part_anchor_of(&runtime, "mcu");
        assert_eq!(
            (anchor.stagger, anchor.offset),
            (0.0, Vec2::default()),
            "one Undo restores the anchor"
        );
    }
}

/// Sweep a Rotate token from `from` to `to` with Alt, and return the live popup degrees.
async fn sweep_rotation(
    runtime: &Rc<Runtime>,
    rail: &MountedPartRail,
    from: (i32, i32),
    to: (i32, i32),
) -> f64 {
    let handle = rail.handle().expect("the Rotate token draws");
    let before = revision(runtime);
    dispatch_pointer(&handle, "pointerdown", false, false, 1, from);
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, true, 1, to);
    frame().await;
    let text = popup_text(&rail.root);
    assert!(
        text.contains("Rotate"),
        "the popup follows the sweep: {text}"
    );
    let live = degrees_in(&text);
    assert!(live.abs() > 1.0, "the sweep moved the rotation: {text}");
    dispatch_pointer(&handle, "pointerup", false, true, 0, to);
    pump_past(runtime, before).await;
    assert_eq!(revision(runtime), before + 1, "one accepted edit");
    live
}

/// Ticket 07: dragging the anchored part's Rotate token edits the local Rotation; the commit
/// matches the live popup value within its tenth-degree display and one Undo restores it.
#[wasm_bindgen_test]
async fn part_rotate_drag_commits_local_rotation() {
    let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
    wait_until_editable(&runtime).await;
    let rail = mount_part_tool(&runtime, "mcu", LayoutTransformTool::Rotate).await;
    let client = rail.client();
    let live = sweep_rotation(&runtime, &rail, client(38.05, 0.0), client(48.05, 10.0)).await;
    let rotation = part_anchor_of(&runtime, "mcu").rotation;
    assert!(
        (rotation - live).abs() < 0.06,
        "{rotation} vs the live {live}"
    );
    undo(&runtime).await;
    assert_eq!(
        part_anchor_of(&runtime, "mcu").rotation,
        0.0,
        "one Undo restores the Rotation"
    );
}

/// Ticket 07: a detached Rotate sweep previews the popup and commits one `SetPartRotation`:
/// the exact rotation persists, one toast, one Undo restores it. User approval keeps the token
/// visible and working.
#[wasm_bindgen_test]
async fn detached_rotate_drag_commits_exact_rotation() {
    let runtime = fixture::open_detached_part_runtime().await;
    wait_until_editable(&runtime).await;
    let rail = mount_part_tool(&runtime, "mcu", LayoutTransformTool::Rotate).await;
    let handle = rail.handle().expect("the Rotate token draws");
    dispatch_pointer(&handle, "pointerenter", false, false, 0, (100, 100));
    frame().await;
    assert!(
        popup_text(&rail.root).contains("0.0°"),
        "the popup shows the detached rotation"
    );
    let live = sweep_rotation(&runtime, &rail, (100, 100), (140, 60)).await;
    let rotation = part_named(&runtime, "mcu").pose.rotation;
    assert!(
        (rotation - live).abs() < 0.06,
        "{rotation} vs the live {live}"
    );
    let toast = toast_text(&runtime, &rail.root).await;
    assert!(
        toast.contains("Rotate"),
        "the toast names the rotation: {toast}"
    );
    undo(&runtime).await;
    assert_eq!(
        part_named(&runtime, "mcu").pose.rotation,
        0.0,
        "one Undo restores the Rotation"
    );
}

/// Ticket 07: a detached part draws Move and Rotate but no Spread or Stagger; a locked part
/// draws no part token; and a matrix member draws none either, keeping its matrix's tools.
#[wasm_bindgen_test]
async fn only_independent_unlocked_parts_draw_their_own_tokens() {
    use LayoutTransformTool::{Move, Rotate, Spread, Stagger};
    let mut locked = fixture::part_document((19.0, 0.0));
    locked
        .parts
        .iter_mut()
        .find(|part| part.id == "mcu")
        .expect("the mcu part")
        .locked = Some(true);
    for (document, part_id, drawn, hidden) in [
        (
            fixture::detached_part_document(),
            "mcu",
            vec![Move, Rotate],
            vec![Spread, Stagger],
        ),
        (locked, "mcu", vec![], vec![Move, Rotate, Spread, Stagger]),
        (
            fixture::matrix_document(),
            "matrix/matrix/r0c1",
            vec![],
            vec![Move],
        ),
    ] {
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        for (tool, draws) in drawn
            .into_iter()
            .map(|tool| (tool, true))
            .chain(hidden.into_iter().map(|tool| (tool, false)))
        {
            let rail = mount_part_tool(&runtime, part_id, tool).await;
            assert_eq!(rail.handle().is_some(), draws, "{tool:?} on {part_id}");
        }
    }
}

/// Ticket 07: a detached Move token drives the shipped part-drag path: the gesture commits one
/// revision that moves the pose, raises a toast, and one Undo restores it.
#[wasm_bindgen_test]
async fn detached_move_drag_drives_the_shipped_part_drag_path() {
    let runtime = fixture::open_detached_part_runtime().await;
    wait_until_editable(&runtime).await;
    let start = part_named(&runtime, "mcu").pose.at;
    let rail = mount_part_tool(&runtime, "mcu", LayoutTransformTool::Move).await;
    let handle = rail.handle().expect("Move draws a handle");
    let client = rail.client();
    // Past the shipped body threshold, so the session gesture begins.
    let to = client(start.x + 3.0, start.y);
    let before = revision(&runtime);
    dispatch_pointer(
        &handle,
        "pointerdown",
        false,
        false,
        1,
        client(start.x, start.y),
    );
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, to);
    frame().await;
    // The token drag drives the shipped session gesture, like a body drag.
    pump_until(
        &runtime,
        "the token drag drives the session gesture",
        || runtime.model().gesture.is_some(),
    )
    .await;
    dispatch_pointer(&handle, "pointerup", false, false, 0, to);
    pump_past(&runtime, before).await;
    assert_eq!(revision(&runtime), before + 1, "one accepted edit");
    assert!(
        runtime.model().gesture.is_none(),
        "the session gesture settles on drop"
    );
    let toast = toast_text(&runtime, &rail.root).await;
    assert!(
        toast.starts_with("Move X "),
        "the toast names the move: {toast}"
    );
    undo(&runtime).await;
    let restored = part_named(&runtime, "mcu").pose.at;
    assert!(
        (restored.x - start.x).abs() < 1e-9 && (restored.y - start.y).abs() < 1e-9,
        "one Undo restores the pose"
    );
}

/// The mounted rail uses the four-pixel-per-mm viewbox for magnetic grid snapping: a move
/// inside the snap radius lands on the grid, one outside it is preserved, and Alt bypasses
/// the snap. The preview and the commit agree.
#[wasm_bindgen_test]
async fn detached_move_rail_snaps_inside_the_radius_only_without_alt() {
    let snap = GestureSnapInputs {
        pitch: Vec2 { x: 10.0, y: 10.0 },
        snap_fraction: 1.0,
        geometry_snap: false,
        gap: None,
    };
    for (delta, alt, expected) in [(10.5, false, 10.0), (7.5, false, 7.5), (10.5, true, 10.5)] {
        let runtime = fixture::open_detached_part_runtime().await;
        wait_until_editable(&runtime).await;
        let rail = mount_part_rail(&runtime, "mcu", vec![LayoutTransformTool::Move], snap).await;
        let handle = rail.handle().expect("Move draws a handle");
        let svg = query(&rail.root, "svg").unwrap();
        svg.set_attribute("width", "600").unwrap();
        let client = world_to_client(&svg);
        let client = |x: f64| client(x, 40.0);
        let lands =
            |at: Vec2| (at.x - (60.0 + expected)).abs() < 1e-6 && (at.y - 40.0).abs() < 1e-6;
        let case = format!("delta {delta}, alt {alt}");
        let before = revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, alt, 1, client(60.0));
        frame().await;
        dispatch_pointer(&handle, "pointermove", false, alt, 1, client(63.0));
        frame().await;
        pump_until(
            &runtime,
            "the first qualifying move samples the gesture",
            || {
                runtime
                    .model()
                    .gesture
                    .as_ref()
                    .is_some_and(|gesture| gesture.changed)
            },
        )
        .await;
        dispatch_pointer(&handle, "pointermove", false, alt, 1, client(60.0 + delta));
        pump_until(&runtime, &format!("the preview lands, {case}"), || {
            runtime
                .model()
                .display_preview
                .as_ref()
                .is_some_and(|scene| {
                    scene
                        .transforms
                        .iter()
                        .any(|transform| transform.id == "mcu" && lands(transform.pose.at))
                })
        })
        .await;
        assert_eq!(revision(&runtime), before, "the preview does not commit");
        // Pointer-up supplies the final position for the snap decision.
        dispatch_pointer(&handle, "pointerup", false, alt, 0, client(60.0 + delta));
        pump_past(&runtime, before).await;
        assert_eq!(revision(&runtime), before + 1);
        let at = part_named(&runtime, "mcu").pose.at;
        assert!(lands(at), "commit {case}: {at:?}");
    }
}

/// Ticket 07: a drag on one half of a mirrored pair updates both halves per ADR-0038: the
/// Splay-carrying Stagger edit fans out to the partner.
#[wasm_bindgen_test]
async fn part_stagger_drag_on_one_half_updates_both_halves() {
    let runtime = fixture::open_part_pair_runtime().await;
    wait_until_editable(&runtime).await;
    let rail = mount_part_tool(&runtime, "mcu", LayoutTransformTool::Stagger).await;
    let handle = rail.handle().expect("the Stagger token draws");
    let client = rail.client();
    // The left mcu evaluates at its anchor key plus the carried spread.
    let center = {
        let accepted = runtime.model().accepted.expect("accepted");
        let part = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "mcu")
            .unwrap();
        evaluated_part_pose(&accepted.document, part).at
    };
    let (from, to) = (client(center.x, center.y), client(center.x, center.y + 2.0));
    drag_commit(&runtime, &handle, from, to, false, true).await;
    let (source, partner) = (
        part_anchor_of(&runtime, "mcu"),
        part_anchor_of(&runtime, "mcu-mirror"),
    );
    assert_eq!(source.stagger, 2.0, "the dragged half carries +2");
    assert_eq!(partner.stagger, 2.0, "the partner mirrors the edit");
    assert_eq!(source.offset, Vec2::default(), "Offset is untouched");
    undo(&runtime).await;
    for part_id in ["mcu", "mcu-mirror"] {
        assert_eq!(
            part_anchor_of(&runtime, part_id).stagger,
            0.0,
            "one Undo restores {part_id}"
        );
    }
}

/// Ticket 07: pressing a part token focuses the part's half, like a body click
/// (split-board-editing 14), so a drag on the dimmed half keeps editing that half.
#[wasm_bindgen_test]
async fn part_token_press_focuses_the_parts_half() {
    let runtime = fixture::open_part_runtime((19.0, 0.0)).await;
    wait_until_editable(&runtime).await;
    let rail = mount_part_tool(&runtime, "mcu", LayoutTransformTool::Stagger).await;
    let handle = rail.handle().expect("the Stagger token draws");
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    assert_eq!(
        *rail.focus_half.borrow(),
        1,
        "the token press focuses the part's half"
    );
}
