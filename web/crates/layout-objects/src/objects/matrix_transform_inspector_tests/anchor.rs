//! The Inspector's anchor block: Follow, Detach, anchor Pick, Add anchored matrix, aggregate
//! anchors, and the mirrored key Omit.
use super::*;

/// Ticket 23 slice 7a: an anchored matrix that does not follow rotation disables its Rotation
/// field with an accessible reason instead of letting an edit hit Core's anchored-frame
/// refusal; a followed rotation stays editable.
#[wasm_bindgen_test]
async fn rotation_field_follows_the_anchor_rotation_follow() {
    for followed in [false, true] {
        let (document, context) = anchored_matrix_fixture(followed);
        let (_probe, root) = mount_probe(document, context, &[]).await;
        let field: HtmlInputElement = query(&root, "input[aria-label='Rotation']")
            .expect("the Matrix section names Rotation")
            .unchecked_into();
        assert_eq!(
            field.disabled(),
            !followed,
            "Rotation is editable only when followed"
        );
        if !followed {
            assert_eq!(
                text_of(&root, "[role=note]"),
                "Rotation is not followed, so turning it changes nothing.",
                "the reason names the Follow rule"
            );
        }
    }
}

/// Ticket 23 slice 7b: an anchored matrix names its anchor target and Follow subset, and
/// Detach clears the anchor in one atomic Core intent that keeps every pose with one Undo
/// entry.
#[wasm_bindgen_test]
async fn detach_clears_the_anchor_keeping_every_pose_with_one_undo() {
    let (document, context) = anchored_matrix_fixture(true);
    let (probe, root) = mount_probe(document, context, &[]).await;
    assert_eq!(
        text_of(&root, "[data-anchor-target]"),
        "Anchored to matrix · row 0 · column 0",
        "the anchor names its target key"
    );
    assert_eq!(
        text_of(&root, "[data-anchor-follow]"),
        "Follows X, Y and rotation",
        "the anchor names its Follow subset"
    );
    let pose = || fixture::part_named(&probe.runtime, "matrix/matrix-b/r0c0").pose;
    let (pose_before, revision_before) = (pose(), probe.revision());
    click_button(&root, "Detach");
    probe.run().await;
    assert!(
        probe.matrix("matrix-b").anchor.is_none(),
        "Detach clears the anchor"
    );
    assert_eq!(pose(), pose_before, "Detach keeps every pose");
    assert_eq!(
        probe.revision(),
        revision_before + 1,
        "Detach is one atomic Core intent"
    );
    probe.undo().await;
    assert!(
        probe.matrix("matrix-b").anchor.is_some(),
        "one Undo restores the anchor"
    );
}

/// Ticket 23 slice 8: the anchor block edits its Follow subset in place. Unchecking rotation
/// writes one atomic SetMatrixAnchor with one Undo entry.
#[wasm_bindgen_test]
async fn follow_rotation_toggle_updates_the_anchor_with_one_undo() {
    let (document, context) = anchored_matrix_fixture(true);
    let (probe, root) = mount_probe(document, context, &[]).await;
    let rotation = by_role_name(&root, "checkbox", "Follow rotation")
        .expect("the anchor names rotation Follow");
    assert_eq!(
        rotation.get_attribute("aria-checked").as_deref(),
        Some("true")
    );
    let revision_before = probe.revision();
    rotation.click();
    probe.run().await;
    assert!(
        !probe.anchor("matrix-b").follow.rotation,
        "unchecking rotation writes the Follow subset"
    );
    assert_eq!(
        probe.revision(),
        revision_before + 1,
        "the toggle is one atomic Core intent"
    );
    probe.undo().await;
    assert!(
        probe.anchor("matrix-b").follow.rotation,
        "one Undo restores rotation Follow"
    );
}

/// Ticket 23 slice: the anchor block offers Pick anchor. Applying another matrix's key
/// retargets via SetMatrixAnchor as one atomic intent with one Undo entry, keeping Follow;
/// picking the matrix's own key refuses through Core and surfaces the refusal without
/// committing. A second anchor target is another 1x1 matrix in the same Layout.
#[wasm_bindgen_test]
async fn anchor_pick_retargets_the_anchor_or_surfaces_the_core_refusal() {
    for (target, valid) in [("matrix-c", true), ("matrix-b", false)] {
        let (mut document, context) = anchored_matrix_fixture(true);
        document.layouts[0].matrix_ids.push("matrix-c".into());
        let at = Vec2 { x: 120.0, y: 0.0 };
        push_single_key_matrix(&mut document, "matrix-c", "K3", at, None);
        document.matrices[2].name = Some("Other".into());
        let (probe, root) = mount_probe(document, context, &[]).await;
        assert!(
            button_named(&root, "Pick anchor").is_some(),
            "the anchor block offers Pick anchor"
        );
        let revision_before = probe.revision();
        assert!(
            probe.mount().pick_matrix_anchor(target.into(), 0, 0),
            "the picked key reaches the transform request"
        );
        probe.run().await;
        if valid {
            assert_eq!(
                probe.anchor("matrix-b"),
                fixture::key_anchor("matrix-c", true),
                "the pick retargets the anchor keeping Follow"
            );
            assert_eq!(
                probe.revision(),
                revision_before + 1,
                "the retarget is one Core intent"
            );
            probe.undo().await;
            assert_eq!(
                probe.anchor("matrix-b").matrix_id,
                "matrix",
                "one Undo restores it"
            );
        } else {
            assert_eq!(
                probe.revision(),
                revision_before,
                "a refused pick commits nothing"
            );
            assert_eq!(
                probe.anchor("matrix-b").matrix_id,
                "matrix",
                "it keeps the anchor target"
            );
            assert!(
                alert_text(&root)
                    .iter()
                    .any(|text| text.contains("cannot anchor to itself")),
                "the refusal surfaces beside the anchor block"
            );
        }
    }
}

/// Ticket 23 slice: a Key selection offers Add anchored matrix, driving the Core
/// AddAnchoredMatrix intent (1x3, same Layout, Origin (0, -pitch.y), mirrored counterpart per
/// spec section 4) as one atomic intent with one Undo entry. The selected one-key matrix sits
/// in a Layout: the anchor target Core requires.
#[wasm_bindgen_test]
async fn key_selection_adds_an_anchored_matrix_with_one_undo() {
    let (mut document, context) = fixture();
    document.layouts.push(layout("layout", &["matrix"]));
    let (probe, root) = mount_probe(document, context, &["key-part"]).await;
    let revision_before = probe.revision();
    click_button(&root, "Add anchored matrix");
    probe.run().await;
    let after = probe.accepted().document;
    let added = after
        .matrices
        .iter()
        .find(|matrix| matrix.id != "matrix")
        .expect("Add anchored matrix creates a matrix");
    assert_eq!((added.rows, added.columns), (1, 3), "the addition is 1x3");
    assert_eq!(
        added.origin,
        Vec2 { x: 0.0, y: -19.0 },
        "the addition starts at relative Origin (0, -pitch.y)"
    );
    assert_eq!(
        added.anchor,
        Some(fixture::key_anchor("matrix", true)),
        "the addition anchors to the selected key with full Follow"
    );
    assert!(
        after.layouts[0].matrix_ids.contains(&added.id),
        "the addition joins the selected key's Layout"
    );
    assert_eq!(
        probe.revision(),
        revision_before + 1,
        "the addition is one atomic Core intent"
    );
    probe.undo().await;
    assert!(
        probe
            .accepted()
            .document
            .matrices
            .iter()
            .all(|matrix| matrix.id == "matrix"),
        "one Undo removes the added matrix"
    );
}

fn anchor_part(column: u32) -> AnchorPart {
    AnchorPart {
        matrix_id: "matrix".into(),
        row: 0,
        column,
    }
}

/// Ticket 32 slice: a 1x1 matrix with an average aggregate anchor over the first two keys of
/// a `keys`-key matrix. Both matrices join one Layout, which anchor validation requires; the
/// dependant follows the parts' mean. A third key is pickable but not yet a part, so canvas
/// multi-Pick and commit have somewhere to go.
fn aggregate_anchor_fixture(keys: u32) -> (ProjectDoc, TreeContext) {
    let (mut document, _) = if keys == 3 {
        three_key_fixture()
    } else {
        two_key_fixture(false)
    };
    // Non-parallel part headings so the intersection method stays evaluable: the second part
    // key turns 90 degrees.
    document.matrices[0].cells[1].rotation = Some(90.0);
    document
        .layouts
        .push(layout("layout", &["matrix", "matrix-b"]));
    let anchor = MatrixAnchor {
        matrix_id: String::new(),
        aggregate: Some(AggregateAnchor {
            method: AggregateMethod::Average,
            parts: vec![anchor_part(0), anchor_part(1)],
        }),
        ..fixture::key_anchor("", true)
    };
    let at = Vec2 { x: 60.0, y: 0.0 };
    push_single_key_matrix(&mut document, "matrix-b", "K3", at, Some(anchor));
    document.matrices[1].name = Some("Aggregated".into());
    (document, matrix_context("matrix-b"))
}

/// Mount the average-aggregate fixture and capture its dependant pose and
/// revision. Aggregate edit scenarios share this instead of repeating the
/// fixture/mount/capture sequence.
async fn aggregate_mount() -> (Probe, Root, Pose2, u64) {
    let (document, context) = aggregate_anchor_fixture(2);
    let (probe, root) = mount_probe(document, context, &[]).await;
    let pose = probe.scene_pose("matrix-b", 0, 0);
    let revision = probe.revision();
    (probe, root, pose, revision)
}

/// The current laid-out dependant pose of an aggregate fixture mount.
fn aggregate_pose(probe: &Probe) -> Pose2 {
    probe.scene_pose("matrix-b", 0, 0)
}

/// Ticket 32 slice: opening the Anchor section of an average-anchored matrix lists its two
/// part links, and activating a part link navigates to the part key with the revision
/// unchanged and no Undo entry.
#[wasm_bindgen_test]
async fn aggregate_anchor_section_lists_part_links_that_navigate_without_writing() {
    let (document, context) = aggregate_anchor_fixture(2);
    let (probe, root) = mount_probe(document, context, &[]).await;
    let first = by_role_name(&root, "button", "matrix · row 0 · column 0")
        .expect("the first aggregate part is named");
    assert!(by_role_name(&root, "button", "matrix · row 0 · column 1").is_some());
    let revision_before = probe.revision();
    first.click();
    probe.run().await;
    assert_eq!(
        probe.navigated.borrow().as_slice(),
        &[key_context("matrix", 0, 0)],
        "activating a part link navigates to the part key"
    );
    assert_eq!(
        probe.revision(),
        revision_before,
        "navigation writes nothing: no new revision, no Undo entry"
    );
}

/// Ticket 32 and ticket 36: switching the aggregate method through the mounted Inspector
/// writes one Core intent keeping parts and Follow; Undo restores the prior method with the
/// Core-published dependant pose unchanged, and Redo restores the committed method and pose.
#[wasm_bindgen_test]
async fn aggregate_method_switch_commits_with_undo_redo_and_pose_round_trip() {
    let (probe, root, pose_before, revision_before) = aggregate_mount().await;
    let method = by_role_name(&root, "button", "Method").expect("aggregate method");
    assert!(
        method
            .text_content()
            .unwrap_or_default()
            .contains("Average")
    );
    method.click();
    wait_until("aggregate method options open", || {
        by_role_name(&root, "option", "Intersection").is_some()
    })
    .await;
    choose_option(&by_role_name(&root, "option", "Intersection").unwrap());
    probe.run().await;
    let aggregate = probe.aggregate();
    assert_eq!(
        aggregate.method,
        AggregateMethod::Intersect,
        "the switch writes the method"
    );
    assert_eq!(
        aggregate.parts,
        vec![anchor_part(0), anchor_part(1)],
        "it keeps every part"
    );
    assert_eq!(
        probe.revision(),
        revision_before + 1,
        "the switch is one atomic Core intent"
    );
    let pose_after = aggregate_pose(&probe);
    probe.undo().await;
    assert_eq!(
        probe.aggregate().method,
        AggregateMethod::Average,
        "Undo restores the method"
    );
    assert_eq!(
        aggregate_pose(&probe),
        pose_before,
        "Undo restores the Core-published pose exactly"
    );
    probe.redo().await;
    assert_eq!(
        probe.aggregate().method,
        AggregateMethod::Intersect,
        "Redo restores the switch"
    );
    assert_eq!(
        aggregate_pose(&probe),
        pose_after,
        "Redo restores the Core-published committed pose exactly"
    );
}

/// Ticket 32 and ticket 36: removing a part writes one Core intent with one Undo entry and
/// moves the Core-published dependant pose to the surviving part; the last remaining part
/// cannot be removed, and Undo restores the parts and the pose.
#[wasm_bindgen_test]
async fn aggregate_part_remove_updates_the_dependant_pose_with_one_undo() {
    let (probe, root, pose_before, revision_before) = aggregate_mount().await;
    let removes: Vec<_> = [0, 1]
        .into_iter()
        .filter_map(|column| {
            by_role_name(
                &root,
                "button",
                &format!("Remove matrix · row 0 · column {column}"),
            )
        })
        .collect();
    assert_eq!(
        removes.len(),
        2,
        "each part offers Remove while more than one part remains"
    );
    removes[0].click();
    probe.run().await;
    assert_eq!(
        probe.aggregate().parts,
        vec![anchor_part(1)],
        "Remove drops the part"
    );
    assert_eq!(
        probe.revision(),
        revision_before + 1,
        "the removal is one atomic Core intent"
    );
    let pose_after = aggregate_pose(&probe);
    assert_ne!(
        (pose_after.at.x, pose_after.at.y, pose_after.rotation),
        (pose_before.at.x, pose_before.at.y, pose_before.rotation),
        "the Core-published dependant pose follows the surviving part"
    );
    let remaining = by_role_name(&root, "button", "Remove matrix · row 0 · column 1")
        .expect("one part link remains");
    assert!(
        remaining.has_attribute("disabled"),
        "the last remaining part cannot be removed"
    );
    probe.undo().await;
    assert_eq!(
        probe.aggregate().parts.len(),
        2,
        "one Undo restores the removed part"
    );
    assert_eq!(
        aggregate_pose(&probe),
        pose_before,
        "one Undo restores the Core-published pose exactly"
    );
}

fn picked_text(root: &web_sys::Element) -> Option<String> {
    query_all(root, "[data-anchor-pick-note]")
        .iter()
        .filter_map(|note| note.text_content())
        .find(|text| text.ends_with("picked"))
}

/// Pick canvas key r0c`column` while the aggregate pick is armed.
async fn canvas_pick(probe: &Probe, root: &web_sys::Element, column: u32) {
    let key = query_all(root, "[data-part-id]")
        .into_iter()
        .nth(column as usize)
        .expect("the canvas stub renders the picked key");
    let init = web_sys::PointerEventInit::new();
    init.set_bubbles(true);
    key.dispatch_event(
        &web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init).unwrap(),
    )
    .unwrap();
    probe.settle().await;
}

/// Ticket 32 slice: canvas multi-Pick adds parts to the pending aggregate without writing —
/// duplicates and existing parts are a no-op — and Esc cancels with nothing written;
/// committing the pending pick writes one Core intent with one Undo entry.
#[wasm_bindgen_test]
async fn aggregate_multi_pick_holds_pending_parts_until_one_commit() {
    for commit in [false, true] {
        let (document, context) = aggregate_anchor_fixture(3);
        let (probe, root) = mount_probe(document, context, &[]).await;
        click_button(&root, "Pick parts");
        probe.settle().await;
        let revision_before = probe.revision();
        let picks: &[u32] = if commit { &[2] } else { &[2, 2, 0] };
        for &column in picks {
            canvas_pick(&probe, &root, column).await;
        }
        assert_eq!(
            picked_text(&root).as_deref(),
            Some("1 picked"),
            "one fresh key is pending; repeats and existing parts are a no-op"
        );
        assert_eq!(
            probe.revision(),
            revision_before,
            "pending picks write nothing"
        );
        if !commit {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key("Escape");
            let escape =
                web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap();
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .dispatch_event(&escape)
                .unwrap();
            probe.settle().await;
            assert!(picked_text(&root).is_none(), "Esc drops the pending draft");
            click_button(&root, "Pick parts");
            probe.settle().await;
            assert_eq!(
                probe.revision(),
                revision_before,
                "Esc cancels with nothing written"
            );
            continue;
        }
        click_button(&root, "Add 1 part");
        probe.run().await;
        assert_eq!(
            probe.aggregate().parts,
            vec![anchor_part(0), anchor_part(1), anchor_part(2)],
            "the commit appends the picked part in one Core intent"
        );
        assert_eq!(
            probe.revision(),
            revision_before + 1,
            "the commit is one atomic Core intent"
        );
        assert!(
            picked_text(&root).is_none(),
            "the commit clears the pending draft"
        );
        probe.undo().await;
        assert_eq!(
            probe.aggregate().parts.len(),
            2,
            "one Undo restores the parts"
        );
    }
}

/// Ticket 32 slice: committing a part Core refuses surfaces the Core reason inline with
/// nothing written.
#[wasm_bindgen_test]
async fn aggregate_commit_with_self_part_refuses_through_core() {
    let (document, context) = aggregate_anchor_fixture(2);
    let (probe, root) = mount_probe(document, context, &[]).await;
    click_button(&root, "Pick parts");
    probe.settle().await;
    assert!(
        probe.mount().pick_aggregate_part("matrix-b".into(), 0, 0),
        "the self part reaches Core validation"
    );
    probe.settle().await;
    let revision_before = probe.revision();
    click_button(&root, "Add 1 part");
    probe.run().await;
    assert_eq!(
        probe.revision(),
        revision_before,
        "a refused commit writes nothing"
    );
    assert_eq!(
        probe.aggregate().parts.len(),
        2,
        "a refused commit keeps the parts"
    );
    assert!(
        alert_text(&root)
            .iter()
            .any(|text| text.contains("cannot anchor to itself")),
        "the Core refusal surfaces inline"
    );
}
