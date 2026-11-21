//! The matrix rail overlay through the real pointer path: drags, hover previews, the value
//! popup and the commit toast.
use super::*;
use boardstudio_web_ui_shared::test_wait::{wait_for, wait_for_step, wait_until_future};
use fixture::undo;

fn token_rect_style(rail: &MountedRail, label: &str) -> String {
    query(&rail.token(label), "rect")
        .expect("the token draws its box")
        .get_attribute("style")
        .unwrap_or_default()
}

fn token_opacity(rail: &MountedRail, label: &str) -> String {
    rail.token(label)
        .get_attribute("style")
        .unwrap_or_default()
        .split(';')
        .find_map(|part| {
            let (name, value) = part.split_once(':')?;
            (name.trim() == "opacity").then(|| value.trim().to_owned())
        })
        .unwrap_or_else(|| "1".to_owned())
}

/// Hover and leave `token`, letting the peers re-read the shared mount set.
async fn hover_pass(token: &web_sys::Element, at: (i32, i32)) {
    dispatch_pointer(token, "pointerenter", false, false, 0, at);
    frame().await;
    dispatch_pointer(token, "pointerleave", false, false, 0, at);
    frame().await;
}

/// Ticket 23: the mounted drag harness dispatches script-created pointer events, which the
/// platform never trusts. This pins the coverage split: mounted tests prove drag wiring and
/// math, and the packaged-page mouse smoke (`scripts/test-drag-interactions.cjs`) proves the
/// same gestures with real trusted pointer events.
#[wasm_bindgen_test]
async fn mounted_pointer_dispatch_is_synthetic_never_trusted() {
    let document = web_sys::window().unwrap().document().unwrap();
    let target = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&target).unwrap();
    for kind in ["pointerdown", "pointermove", "pointerup"] {
        let event = dispatch_pointer(&target, kind, false, false, 1, (100, 100));
        assert!(
            !event.is_trusted(),
            "{kind} dispatched from the harness is synthetic, never trusted"
        );
    }
    target.remove();
}

/// Ticket 03: the rail lays its tokens out side by side above the selection at screen size;
/// hover previews the movers in amber with a selection-surface token and leaves nothing
/// behind; while one token drags, the token stays accent and its peers dim to 35%.
#[wasm_bindgen_test]
async fn rail_tokens_share_one_screen_size_row_with_hover_and_dimming() {
    use LayoutTransformTool::{Splay, Stagger};
    let runtime = fixture::open_matrix_runtime().await;
    wait_until_editable(&runtime).await;
    let before = revision(&runtime);
    let rail = mount_matrix_rail(
        &runtime,
        fixture::MATRIX_ID,
        column(1),
        vec![Stagger, Splay],
    )
    .await;
    // The rail draws one backdrop for two 18 px tokens: 2·18 + 2 + 2·2. Peers converge on
    // the post-mount pass, so wait for it.
    wait_until("two tokens share one 42 px rail", || {
        query(&rail.root, "g[aria-hidden] rect")
            .and_then(|backdrop| backdrop.get_attribute("width"))
            .as_deref()
            == Some("42")
    })
    .await;
    // Both tokens sit on one row through the rail centre, symmetric about it, in the peers'
    // canonical order. World offsets scale with the harness fallback until the first hover
    // re-render, so these assert the scale-free invariants; absolute px geometry lives in
    // the native `handle_rail` tests.
    let (stagger, splay) = (rail.token("Drag to stagger"), rail.token("Drag to splay"));
    let (stagger_x, stagger_y) = fixture::translate_of(&stagger);
    let (splay_x, splay_y) = fixture::translate_of(&splay);
    let backdrop = query(&rail.root, "g[aria-hidden]").expect("the rail backdrop group draws");
    let (rail_cx, rail_cy) = fixture::translate_of(&backdrop);
    assert!(
        (stagger_y - rail_cy).abs() < 1e-9 && (splay_y - rail_cy).abs() < 1e-9,
        "the tokens share the rail row, got {stagger_y}, {splay_y} and {rail_cy}"
    );
    assert!(
        ((stagger_x + splay_x) / 2.0 - rail_cx).abs() < 1e-9,
        "the tokens centre on the rail, got {stagger_x}, {splay_x} and {rail_cx}"
    );
    assert!(
        splay_x > stagger_x,
        "the splay token follows in canonical order"
    );
    // Token chrome is px art: an 18 px box plus a 20 by 24 hit area.
    for token in [&stagger, &splay] {
        assert_eq!(
            query_all(token, "rect").len(),
            2,
            "each token draws box and hit area"
        );
    }
    // Hover previews the movers in amber on a selection-surface token and leaves with no
    // document revision.
    let lit = || token_rect_style(&rail, "Drag to stagger").contains("var(--selected)");
    dispatch_pointer(&stagger, "pointerenter", false, false, 0, (100, 100));
    wait_until(
        "stagger hover highlights the token and previews its movers",
        || lit() && rail.preview_parts().is_some_and(|parts| !parts.is_empty()),
    )
    .await;
    dispatch_pointer(&stagger, "pointerleave", false, false, 0, (100, 100));
    wait_until(
        "stagger leave clears the preview and restores the muted token",
        || !lit() && rail.preview_parts().is_none(),
    )
    .await;
    assert_eq!(revision(&runtime), before, "hover and leave write nothing");
    // While the stagger token drags, it stays accent and the splay peer dims to 35%: the drag
    // re-render marks the token, and the peer's own hover pass reads the shared mount set.
    dispatch_pointer(&stagger, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&stagger, "pointermove", false, false, 1, (140, 100));
    pump_preview(&runtime).await;
    assert!(
        token_rect_style(&rail, "Drag to stagger").contains("var(--primary)"),
        "the active token keeps the accent fill"
    );
    hover_pass(&splay, (140, 100)).await;
    assert_eq!(
        token_opacity(&rail, "Drag to splay"),
        "0.35",
        "the peer dims"
    );
    assert_eq!(
        token_opacity(&rail, "Drag to stagger"),
        "1",
        "the active token stays full"
    );
    dispatch_pointer(&stagger, "pointerup", false, false, 0, (140, 100));
    // The release may land or be refused; either way it ends the drag.
    let _ = wait_for_step("the stagger release settles", 2_000.0, async || {
        support::run_pending(&runtime).await;
        revision(&runtime) > before
    })
    .await;
    hover_pass(&splay, (140, 100)).await;
    assert_eq!(
        token_opacity(&rail, "Drag to splay"),
        "1",
        "the peer undims after the drag lands"
    );
}

#[wasm_bindgen_test]
async fn column_angle_pointer_preview_starts_at_rotation_and_preserves_mirror_direction() {
    for (mirror, expected) in [
        (None, 100.0),
        (Some(boardstudio_model::model::Mirror::X), -80.0),
    ] {
        let mut document = fixture::matrix_document();
        let matrix = &mut document.matrices[0];
        matrix.mirror = mirror;
        matrix.column_layout.resize_with(2, Default::default);
        matrix.column_layout[1].splay = 7.0;
        matrix.column_layout[1].rotation = 10.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let before = revision(&runtime);
        let rail = mount_column_tool(&runtime, LayoutTransformTool::Splay).await;
        let handle = rail.token("Drag to splay");
        assert_eq!(text_of(&handle, "title"), "Drag to splay · 10°");
        let (_, scene) = matrix_and_scene(&runtime, fixture::MATRIX_ID);
        let origin = scene.columns[1].splay_origin;
        let client = rail.client();
        let (start, end) = (
            client(origin.x + 20.0, origin.y),
            client(origin.x, origin.y + 20.0),
        );
        let previewed = || {
            runtime
                .model()
                .display_preview
                .as_ref()
                .unwrap()
                .matrix_scenes[0]
                .columns[1]
                .rotation
        };
        dispatch_pointer(&handle, "pointerdown", false, false, 1, start);
        dispatch_pointer(&handle, "pointermove", false, false, 1, start);
        support::run_pending(&runtime).await;
        assert_eq!(revision(&runtime), before, "initial preview never commits");
        assert_eq!(
            previewed(),
            10.0,
            "zero pointer delta previews the stored Column rotation"
        );
        dispatch_pointer(&handle, "pointermove", false, false, 1, end);
        support::run_pending(&runtime).await;
        assert_eq!(
            previewed(),
            expected,
            "the pointer preview uses the selected angle and mirror direction"
        );
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].rotation,
            10.0
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, end);
        wait_until_editable(&runtime).await;
        let record = fixture::accepted_matrix(&runtime).column_layout[1];
        assert_eq!((record.rotation, record.splay), (expected, 7.0));
        undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].rotation,
            10.0
        );
    }
}

/// One synthetic drag that may land or be refused. Returns the handle's `data-quick-offset`
/// after the down and after the move.
async fn quick_offset_drag(
    runtime: &Rc<Runtime>,
    handle: &web_sys::Element,
    ctrl_down: bool,
    to: (i32, i32),
) -> (Option<String>, Option<String>) {
    let before = revision(runtime);
    dispatch_pointer(handle, "pointerdown", ctrl_down, false, 1, (100, 100));
    frame().await;
    let after_down = handle.get_attribute("data-quick-offset");
    // Ctrl is released before the first move; the latch must hold.
    dispatch_pointer(handle, "pointermove", false, false, 1, to);
    frame().await;
    let after_move = handle.get_attribute("data-quick-offset");
    dispatch_pointer(handle, "pointerup", false, false, 0, to);
    pump_past(runtime, before).await;
    (after_down, after_move)
}

#[wasm_bindgen_test]
async fn real_pointer_events_with_stubbed_capture_latch_ctrl_at_pointer_down_and_show_the_offset_glyph()
 {
    let runtime = fixture::open_matrix_runtime().await;
    // The first edit syncs the fixture's unlaid poses.
    let mut sync = boardstudio_web_runtime::pending_edits::PendingEdits::<()>::default();
    sync.begin(
        &runtime,
        (),
        "sync",
        Some("transform".into()),
        EditResolver::new("sync", |_: &AcceptedSnapshot| {
            let operation = crate::matrix_transform_operation::placement_operation(
                fixture::MATRIX_ID,
                PlacementParameter::ColumnOffset { column: 2 },
                PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
            );
            transform_commit(String::new(), fixture::MATRIX_ID, operation)
        }),
    );
    let sync = RefCell::new(sync);
    pump_until(&runtime, "the sync edit settles", || {
        !sync.borrow_mut().settle(true).is_empty()
    })
    .await;
    wait_until_editable(&runtime).await;
    let shifted = Vec2 {
        x: 9.525,
        y: 4.7625,
    };

    // Ctrl at pointer-down: a free 2-D column offset with the offset glyph, though Ctrl is
    // released before the first move. The grid step is 4.7625 mm; 4 px is 1 mm.
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Stagger).await;
    let handle = rail.handle();
    assert_eq!(
        handle.get_attribute("data-quick-offset").as_deref(),
        Some("false")
    );
    assert_eq!(
        handle.get_attribute("aria-label").as_deref(),
        Some("Drag to stagger")
    );
    let latched = quick_offset_drag(&runtime, &handle, true, (140, 80)).await;
    assert_eq!(
        latched,
        (Some("true".into()), Some("true".into())),
        "Ctrl at pointer-down shows the glyph, and releasing it does not switch it"
    );
    let matrix = fixture::accepted_matrix(&runtime);
    assert_eq!(matrix.column_layout[1].offset, shifted);
    assert!(
        matrix
            .column_layout
            .iter()
            .all(|record| record.stagger == 0.0)
    );
    frame().await;
    assert_eq!(
        handle.get_attribute("data-quick-offset").as_deref(),
        Some("false"),
        "the glyph returns when the drag ends"
    );
    drop(rail);
    wait_until_editable(&runtime).await;

    // No Ctrl: the same gesture is a one-dimensional stagger.
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Stagger).await;
    let plain = quick_offset_drag(&runtime, &rail.handle(), false, (140, 60)).await;
    assert_eq!(plain, (Some("false".into()), Some("false".into())));
    let record = fixture::accepted_matrix(&runtime).column_layout[1];
    assert!((record.stagger - 9.525).abs() < 1e-9);
    assert_eq!(record.offset, shifted, "the offset is untouched");
}

/// Ticket 23: the Spread tool draws its own horizontal handle that commits an explicit Spread
/// through the real Runtime/Core seam. A 40 px drag is 10 world mm; the 4.7625 mm grid lands
/// six steps out, so column 1 steps 28.575 mm and column 2 follows.
#[wasm_bindgen_test]
async fn spread_handle_drag_commits_an_explicit_spread_with_one_undo() {
    let runtime = fixture::open_matrix_runtime().await;
    wait_until_editable(&runtime).await;
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Spread).await;
    let handle = rail.handle();
    assert_eq!(
        handle.get_attribute("aria-label").as_deref(),
        Some("Drag to spread")
    );
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
    // The drag previews actual later-column movement before commit.
    pump_preview(&runtime).await;
    let before = revision(&runtime);
    dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
    pump_past(&runtime, before).await;
    assert_eq!(revision(&runtime), before + 1, "one accepted edit");
    let spread = fixture::accepted_matrix(&runtime).column_layout[1].spread;
    assert!(
        spread.is_some_and(|spread| (spread - 28.575).abs() < 1e-9),
        "the drag commits an explicit Spread, got {spread:?}"
    );
    let (_, scene) = matrix_and_scene(&runtime, fixture::MATRIX_ID);
    let at = |column| {
        scene
            .cells
            .iter()
            .find(|cell| cell.row == 0 && cell.column == column)
            .expect("the key is laid out")
            .pose
            .at
    };
    assert!(
        (at(1).x - 28.575).abs() < 1e-9 && (at(2).x - 47.625).abs() < 1e-9,
        "later columns step out the full Spread"
    );
    assert!(
        at(0).x.abs() < 1e-9,
        "column 0 does not follow a later Spread"
    );
    undo(&runtime).await;
    assert!(
        fixture::accepted_matrix(&runtime)
            .column_layout
            .get(1)
            .and_then(|record| record.spread)
            .is_none(),
        "one Undo restores the default step"
    );
}

/// A commit-toast timer that fires after the overlay unmounts must stay silent: scope
/// switches remount the overlay constantly, and the 1.8 s timer touching dropped signals
/// panicked the page (WASM `unreachable`), leaving the Dioxus panic toast over the toolbar.
/// The timer reads its generation signal after an await, so a dropped owner must retire the
/// toast silently, whether the overlay unmounts before or after the commit lands.
#[wasm_bindgen_test]
async fn commit_toast_timer_survives_overlay_unmount() {
    type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send + 'static>;
    /// Restores the panic hook it replaces, even when the test fails first. A trapped timer
    /// task can leave this thread unwinding; restoring the hook then would trap again, so
    /// it only restores on a clean drop.
    struct RestorePanicHook(Option<PanicHook>);
    impl Drop for RestorePanicHook {
        fn drop(&mut self) {
            if !std::thread::panicking()
                && let Some(hook) = self.0.take()
            {
                std::panic::set_hook(hook);
            }
        }
    }
    fn observe_next_toast_expiry() -> (
        futures_channel::oneshot::Sender<()>,
        futures_channel::oneshot::Receiver<()>,
    ) {
        let (release, released) = futures_channel::oneshot::channel();
        let (finished, expired) = futures_channel::oneshot::channel();
        NEXT_TOAST_EXPIRY_PROBE.with(|next| {
            next.replace(Some(ToastExpiryProbe {
                release: released,
                finished,
            }));
        });
        (release, expired)
    }
    let panics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    for unmount_before_landing in [true, false] {
        let restore = RestorePanicHook(Some(std::panic::take_hook()));
        let recorded = panics.clone();
        std::panic::set_hook(Box::new(move |info| {
            recorded.lock().unwrap().push(info.to_string());
        }));
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let mut rail = mount_column_tool(&runtime, LayoutTransformTool::Stagger).await;
        let handle = rail.handle();
        // A real pointer drag commits a stagger and arms the observed timer.
        let (release_expiry, expired) = observe_next_toast_expiry();
        let before = revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        frame().await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (100, 40));
        frame().await;
        dispatch_pointer(&handle, "pointerup", false, false, 0, (100, 40));
        let landed = |runtime: &Rc<Runtime>| revision(runtime) > before;
        let has_toast = || query(&rail.root, "[data-feedback-toast]").is_some();
        if unmount_before_landing {
            // Retire the overlay the moment its toast appears: triggering on the toast (not
            // the revision bump) keeps the unmount ahead of the expiry however slow the
            // commit is, and the commit below does not need the overlay alive.
            pump_until(&runtime, "the toast has armed its expiry", has_toast).await;
        } else {
            pump_past(&runtime, before).await;
            assert!(has_toast(), "the commit arms its toast timer");
        }
        rail.show.set(false);
        pump_until(&runtime, "the toast's overlay unmounts", || {
            query(&rail.root, "[data-transform-handle]").is_none()
        })
        .await;
        pump_until(&runtime, "the stagger revision lands", || landed(&runtime)).await;
        assert_eq!(revision(&runtime), before + 1, "one accepted edit");
        release_expiry
            .send(())
            .expect("the commit toast armed the timer");
        wait_until_future("the expiry callback finishes after unmount", expired)
            .await
            .expect("the expiry callback did not finish");
        // Restore the previous hook before asserting, so a failure here never masks its own
        // message behind a re-entrant hook error.
        drop(restore);
        assert!(
            panics.lock().unwrap().is_empty(),
            "the toast timer trapped after unmount: {:?}",
            panics.lock().unwrap()
        );
        let document = web_sys::window().unwrap().document().unwrap();
        assert!(
            document.query_selector("#__dx-toast").unwrap().is_none(),
            "no framework panic toast is showing after the timer fired"
        );
        assert!(
            runtime.model().accepted.is_some(),
            "the session keeps its document"
        );
    }
}

/// Ticket 05: the row Bend and key Shift tokens drag through the real pointer path and commit
/// one Undo entry that changes only their own row or key. Forty pixels are ten world
/// millimetres: the row frame turns five degrees, and the key shift steps two grid steps.
#[wasm_bindgen_test]
async fn row_and_key_token_drags_change_only_their_own_target_with_one_undo() {
    fn row_bend(matrix: &Matrix, row: usize) -> f64 {
        matrix.row_layout.get(row).map_or(0.0, |record| record.bend)
    }
    fn key_shift(matrix: &Matrix, row: u32, column: u32) -> Vec2 {
        matrix
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .and_then(|cell| cell.shift)
            .unwrap_or_default()
    }
    let mut document = fixture::matrix_document();
    document.matrices[0].rows = 2;
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    for (tool, context, label) in [
        (LayoutTransformTool::Bend, row(1), "Drag to bend row"),
        (LayoutTransformTool::Shift, key(0, 1), "Drag to shift key"),
    ] {
        wait_until_editable(&runtime).await;
        let rail = mount_matrix_rail(&runtime, fixture::MATRIX_ID, context, vec![tool]).await;
        let handle = rail.handle();
        assert_eq!(handle.get_attribute("aria-label").as_deref(), Some(label));
        drag_commit(&runtime, &handle, (100, 100), (140, 100), false, false).await;
        let matrix = fixture::accepted_matrix(&runtime);
        if tool == LayoutTransformTool::Bend {
            assert_eq!(
                row_bend(&matrix, 1),
                -5.0,
                "the drag turns the row's running frame"
            );
            assert_eq!(row_bend(&matrix, 0), 0.0, "the other row holds still");
        } else {
            assert_eq!(key_shift(&matrix, 0, 1), Vec2 { x: 9.525, y: 0.0 });
            assert!(
                matrix
                    .cells
                    .iter()
                    .filter(|cell| !(cell.row == 0 && cell.column == 1))
                    .all(|cell| cell.shift.is_none()),
                "only the dragged key carries a shift"
            );
        }
        undo(&runtime).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            row_bend(&matrix, 1),
            0.0,
            "one Undo restores the running frame"
        );
        assert_eq!(
            key_shift(&matrix, 0, 1),
            Vec2::default(),
            "one Undo restores the shift"
        );
    }
}

/// Ticket 06: the value popup docks above the rail on hover, follows the pointer during a drag
/// with the live value and delta, and the drop raises one commit toast with no extra revision;
/// the toast is gone after its timeout. Sixty pixels up are fifteen world millimetres, so the
/// snapped stagger lands three grid steps out.
#[wasm_bindgen_test]
async fn value_popup_follows_the_drag_and_commit_toast_expires() {
    let runtime = fixture::open_matrix_runtime().await;
    wait_until_editable(&runtime).await;
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Stagger).await;
    let handle = rail.token("Drag to stagger");
    let popup = || query(&rail.root, "[data-feedback-popup]");
    dispatch_pointer(&handle, "pointerenter", false, false, 0, (100, 100));
    wait_until("stagger hover opens the value popup", || popup().is_some()).await;
    let docked = popup().unwrap();
    assert!(
        fixture::translate_of(&docked).1 > fixture::translate_of(&handle).1,
        "the docked popup sits above the rail token"
    );
    assert!(
        docked
            .text_content()
            .unwrap_or_default()
            .contains("Stagger"),
        "the popup names the field"
    );

    let before = revision(&runtime);
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    let start_y = fixture::translate_of(&popup().unwrap()).1;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (100, 40));
    // The popup follows the pointer up by the dragged world distance with the live delta.
    wait_until(
        "stagger popup follows the drag and shows its live delta",
        || {
            popup().is_some_and(|popup| {
                (fixture::translate_of(&popup).1 - start_y - 15.0).abs() < 1.5
                    && popup
                        .text_content()
                        .unwrap_or_default()
                        .contains("+14.29 mm")
            })
        },
    )
    .await;
    dispatch_pointer(&handle, "pointerup", false, false, 0, (100, 40));
    pump_past(&runtime, before).await;
    assert_eq!(revision(&runtime), before + 1, "one accepted edit");
    let toast = query(&rail.root, "[data-feedback-toast]").expect("the commit raises a toast");
    assert_eq!(toast.get_attribute("role").as_deref(), Some("status"));
    assert_eq!(
        toast.get_attribute("tabindex"),
        None,
        "the toast never takes focus"
    );
    assert_eq!(
        toast.text_content().unwrap_or_default(),
        "Stagger 14.29 mm (+14.29 mm) · 1 Undo step"
    );
    wait_for(
        "commit toast expires without an extra revision",
        2_500.0,
        || {
            assert_eq!(
                revision(&runtime),
                before + 1,
                "the toast writes no extra revision"
            );
            query(&rail.root, "[data-feedback-toast]").is_none()
        },
    )
    .await
    .unwrap();
}

/// Ticket 05: hovering a row or key token previews, in amber, the same key set the Inspector
/// field preview carries for that field. The fixture's parts all sit in row 0, so the row
/// rail previews row 0: a part-less row would give the Inspector field no movers either.
#[wasm_bindgen_test]
async fn row_and_key_tokens_preview_the_same_movers_as_the_inspector_fields() {
    use crate::matrix_transform_operation::MatrixTransformField as Field;
    use LayoutTransformTool::{Bend, Offset, Shift};
    for (context, tools, tokens) in [
        (
            row(0),
            vec![Offset, Shift, Bend],
            vec![
                ("Drag to offset row", Field::RowOffsetX),
                ("Drag to shift row", Field::RowShiftX),
                ("Drag to bend row", Field::RowBend),
            ],
        ),
        (
            key(0, 1),
            vec![Shift, Bend],
            vec![
                ("Drag to shift key", Field::KeyShiftX),
                ("Drag to bend key", Field::KeyBend),
            ],
        ),
    ] {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let rail = mount_matrix_rail(&runtime, fixture::MATRIX_ID, context.clone(), tools).await;
        for (label, field) in tokens {
            let token = rail.token(label);
            dispatch_pointer(&token, "pointerenter", false, false, 0, (100, 100));
            let accepted = runtime.model().accepted.expect("an accepted document");
            let expected = matrix_relations::field_preview(
                &accepted.document,
                &accepted.scene.matrix_scenes,
                &context,
                field,
                MatrixSplayAffect::Column,
            )
            .expect("the Inspector field previews its movers");
            assert!(
                !expected.parts.is_empty(),
                "{label} previews a non-empty mover set"
            );
            wait_until(&format!("{label} previews the {field:?} movers"), || {
                rail.preview_parts().as_ref() == Some(&expected.parts)
            })
            .await;
            dispatch_pointer(&token, "pointerleave", false, false, 0, (100, 100));
            wait_until(&format!("{label} clears the preview on leave"), || {
                rail.preview_parts().is_none()
            })
            .await;
        }
    }
}

fn dep_origin(runtime: &Rc<Runtime>) -> Vec2 {
    fixture::matrix_named(runtime, "dep").origin
}

/// Ticket 23 slice: dragging an anchored matrix writes its Origin through the inverse of the
/// anchor frame as one atomic edit, constrained to the Follow directions: an unfollowed
/// component never moves the frame.
#[wasm_bindgen_test]
async fn anchored_matrix_move_drag_writes_origin_through_the_anchor_inverse_along_followed_directions()
 {
    use boardstudio_model::model::Follow;
    let followed = |x| Follow {
        x,
        y: true,
        rotation: true,
    };
    // Followed x and y: forty pixels right are ten world mm through the anchor inverse.
    let runtime = open_anchored_drag_runtime(followed(true)).await;
    let rail = mount_matrix_rail(
        &runtime,
        "dep",
        dep_context(),
        vec![LayoutTransformTool::Move],
    )
    .await;
    let handle = rail.handle();
    assert_eq!(
        handle.get_attribute("aria-label").as_deref(),
        Some("Drag to move matrix")
    );
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
    pump_preview(&runtime).await;
    let before = revision(&runtime);
    dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
    pump_past(&runtime, before).await;
    assert_eq!(revision(&runtime), before + 1, "one accepted edit");
    let origin = dep_origin(&runtime);
    assert!(
        (origin.x - 10.0).abs() < 1e-9 && (origin.y + 19.05).abs() < 1e-9,
        "the drag writes the Origin through the anchor inverse, got {origin:?}"
    );
    undo(&runtime).await;
    let origin = dep_origin(&runtime);
    assert!(
        origin.x.abs() < 1e-9 && (origin.y + 19.05).abs() < 1e-9,
        "one Undo restores the dragged Origin"
    );
    drop(rail);

    // Unfollowed x: a horizontal drag previews and commits nothing, a diagonal one moves y.
    let runtime = open_anchored_drag_runtime(followed(false)).await;
    let rail = mount_matrix_rail(
        &runtime,
        "dep",
        dep_context(),
        vec![LayoutTransformTool::Move],
    )
    .await;
    let handle = rail.handle();
    let before = revision(&runtime);
    let idle = async || {
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
    };
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
    idle().await;
    assert!(
        runtime.model().display_preview.is_none(),
        "a drag outside every followed direction previews nothing"
    );
    dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
    idle().await;
    assert_eq!(
        revision(&runtime),
        before,
        "a drag outside every followed direction commits nothing"
    );
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 80));
    pump_preview(&runtime).await;
    dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 80));
    pump_past(&runtime, before).await;
    let origin = dep_origin(&runtime);
    assert!(
        origin.x.abs() < 1e-9 && (origin.y + 14.05).abs() < 1e-9,
        "only the followed direction moves the frame, got {origin:?}"
    );
}

/// Ticket 23 follow-up: each matrix tool mount draws only its own handle. The packaged-page
/// smoke caught the Move and Rotate mounts each rendering both handles, so two identical
/// "Drag to move matrix" copies fought over the gesture.
#[wasm_bindgen_test]
async fn matrix_tool_mounts_draw_only_their_own_handle() {
    let runtime = open_anchored_drag_runtime(full_follow()).await;
    for (tool, own, other) in [
        (
            LayoutTransformTool::Move,
            "Drag to move matrix",
            "Drag to rotate matrix",
        ),
        (
            LayoutTransformTool::Rotate,
            "Drag to rotate matrix",
            "Drag to move matrix",
        ),
    ] {
        let rail = mount_matrix_rail(&runtime, "dep", dep_context(), vec![tool]).await;
        let labels = query_all(&rail.root, "[data-transform-handle]")
            .iter()
            .filter_map(|handle| handle.get_attribute("aria-label"))
            .collect::<Vec<_>>();
        assert!(
            labels.iter().any(|label| label == own),
            "{tool:?} draws its handle"
        );
        assert!(
            !labels.iter().any(|label| label == other),
            "{tool:?} draws no {other}"
        );
    }
}

/// Ticket 23 slice: the Rotate tool turns the selected matrix: a quarter turn about its handle
/// point sweeps 90 degrees.
#[wasm_bindgen_test]
async fn matrix_rotate_drag_turns_the_matrix() {
    let runtime = open_anchored_drag_runtime(full_follow()).await;
    let rail = mount_matrix_rail(
        &runtime,
        "dep",
        dep_context(),
        vec![LayoutTransformTool::Rotate],
    )
    .await;
    let rotate = rail.token("Drag to rotate matrix");
    let (_, scene) = matrix_and_scene(&runtime, "dep");
    let center = scene
        .cells
        .iter()
        .find(|cell| cell.enabled)
        .expect("the key is laid out")
        .pose
        .at;
    let client = rail.client();
    let before = revision(&runtime);
    dispatch_pointer(
        &rotate,
        "pointerdown",
        false,
        false,
        1,
        client(center.x + 20.0, center.y),
    );
    frame().await;
    let to = client(center.x, center.y + 20.0);
    dispatch_pointer(&rotate, "pointermove", false, false, 1, to);
    pump_preview(&runtime).await;
    dispatch_pointer(&rotate, "pointerup", false, false, 0, to);
    pump_past(&runtime, before).await;
    assert_eq!(fixture::matrix_named(&runtime, "dep").rotation, Some(90.0));
}

/// Ticket 23: hovering the Spread handle neither replaces its SVG target nor lets a passive
/// hint intercept the pointer-down that follows.
#[wasm_bindgen_test]
async fn spread_handle_hover_keeps_its_svg_target() {
    let runtime = fixture::open_matrix_runtime().await;
    wait_until_editable(&runtime).await;
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Spread).await;
    let handle = rail.handle();
    dispatch_pointer(&handle, "pointerover", false, false, 0, (100, 100));
    frame().await;
    assert!(
        rail.handle() == handle,
        "hover does not replace the handle's SVG target"
    );
    // No passive hint may cover the handle: every painted element in the overlay that
    // contains the handle's center and paints above it must opt out of pointer events. Box
    // math is viewport-independent, so the check holds wherever the canvas scrolled.
    let rect = handle.get_bounding_client_rect();
    let (cx, cy) = (
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    );
    let window = web_sys::window().unwrap();
    let interceptors = query_all(&rail.root, "svg *")
        .into_iter()
        .filter(|element| *element != handle && !handle.contains(Some(element.unchecked_ref())))
        .filter(|element| {
            let position = handle.compare_document_position(element.unchecked_ref());
            position & web_sys::Node::DOCUMENT_POSITION_FOLLOWING != 0
        })
        .filter(|element| {
            let area = element.get_bounding_client_rect();
            (area.x()..=area.x() + area.width()).contains(&cx)
                && (area.y()..=area.y() + area.height()).contains(&cy)
        })
        .filter(|element| {
            window
                .get_computed_style(element)
                .ok()
                .flatten()
                .and_then(|style| style.get_property_value("pointer-events").ok())
                .unwrap_or_default()
                != "none"
        })
        .map(|element| format!("{}.{}", element.tag_name(), element.class_name()))
        .collect::<Vec<_>>();
    assert!(
        interceptors.is_empty(),
        "no passive hint intercepts the handle: {interceptors:?}"
    );
    dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
    pump_preview(&runtime).await;
    dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
}

async fn open_two_column_runtime(
    edit: impl FnOnce(&mut boardstudio_model::model::ColumnLayout),
) -> Rc<Runtime> {
    let mut document = fixture::matrix_document();
    let matrix = &mut document.matrices[0];
    matrix.column_layout.resize_with(2, Default::default);
    edit(&mut matrix.column_layout[1]);
    let runtime = support::new_runtime();
    support::open_document(&runtime, document).await;
    runtime
}

/// Ticket 23 slices 11 and 12: releasing the pointer far outside the handle still commits the
/// previewed drag with one Undo step, because capture retargets the up and the drop path must
/// not re-hit-test the handle; losing capture mid-drag cancels the preview with nothing
/// committed, and a subsequent drag on the same handle recovers and commits.
#[wasm_bindgen_test]
async fn outside_release_commits_and_lost_capture_cancels_then_recovers() {
    for lose_capture in [false, true] {
        let runtime = open_two_column_runtime(|_| {}).await;
        wait_until_editable(&runtime).await;
        let rail = mount_column_tool(&runtime, LayoutTransformTool::Stagger).await;
        let handle = rail.handle();
        let stagger = || fixture::accepted_matrix(&runtime).column_layout[1].stagger;
        let before = revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        frame().await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        pump_preview(&runtime).await;
        if lose_capture {
            dispatch_pointer(&handle, "lostpointercapture", false, false, 1, (140, 60));
            pump_until(&runtime, "lost capture clears the preview", || {
                runtime.model().display_preview.is_none()
            })
            .await;
            assert_eq!(
                revision(&runtime),
                before,
                "the cancelled drag commits nothing"
            );
            assert_eq!(stagger(), 0.0, "the cancelled drag moves nothing");
            drag_commit(&runtime, &handle, (100, 100), (140, 60), false, false).await;
            assert!(stagger() != 0.0, "the subsequent drag moves the column");
        } else {
            // The up arrives with a client point far outside the handle geometry, as capture
            // retargeting delivers it.
            dispatch_pointer(&handle, "pointerup", false, false, 0, (900, 900));
            pump_past(&runtime, before).await;
            assert_eq!(
                revision(&runtime),
                before + 1,
                "the outside release commits one edit"
            );
            assert!(
                runtime.model().display_preview.is_none(),
                "the preview clears on release"
            );
            assert!(stagger() != 0.0, "the committed drag moved the column");
            undo(&runtime).await;
            assert_eq!(stagger(), 0.0, "one Undo restores the column");
        }
    }
}

/// Ticket 23 slice 6: Ctrl at pointer-down on the Splay handle latches a swing. The drag writes
/// only the origin through `OriginSwing` (no step compensation), never the angle; one Undo
/// restores exactly.
#[wasm_bindgen_test]
async fn ctrl_splay_drag_swings_the_origin_without_step_compensation() {
    let runtime = open_two_column_runtime(|record| {
        record.splay = 17.0;
        record.stagger = 3.0;
    })
    .await;
    let before = revision(&runtime);
    let origin = matrix_and_scene(&runtime, fixture::MATRIX_ID).1.columns[1].splay_origin;
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Splay).await;
    let handle = rail.token("Drag to splay");
    let client = rail.client();
    let end = client(origin.x, origin.y + 20.0);
    // Ctrl is down for the pointer-down only; the latch must hold the swing.
    dispatch_pointer(
        &handle,
        "pointerdown",
        true,
        false,
        1,
        client(origin.x + 20.0, origin.y),
    );
    frame().await;
    assert_eq!(
        handle.get_attribute("data-swing").as_deref(),
        Some("true"),
        "Ctrl shows the swing"
    );
    dispatch_pointer(&handle, "pointermove", false, false, 1, end);
    pump_preview(&runtime).await;
    assert_eq!(
        revision(&runtime),
        before,
        "the swing preview never commits"
    );
    let preview = runtime.model().display_preview.unwrap().matrix_scenes[0].columns[1].clone();
    assert_eq!(
        preview.splay_angle, 17.0,
        "the swing preview never turns the angle"
    );
    assert!(
        preview.splay_origin != origin,
        "the swing preview moves the origin"
    );
    dispatch_pointer(&handle, "pointerup", false, false, 0, end);
    pump_past(&runtime, before).await;
    assert_eq!(revision(&runtime), before + 1, "one accepted edit");
    let record = fixture::accepted_matrix(&runtime).column_layout[1];
    assert_eq!(record.splay, 17.0, "the swing writes no angle");
    assert!(
        record.spread.is_none(),
        "the swing writes no compensating Spread"
    );
    assert_eq!(
        record.stagger, 3.0,
        "the swing writes no compensating Stagger"
    );
    assert!(
        record.splay_origin != Vec2::default(),
        "the swing moves the origin"
    );
    undo(&runtime).await;
    let record = fixture::accepted_matrix(&runtime).column_layout[1];
    assert_eq!(
        record.splay_origin,
        Vec2::default(),
        "one Undo restores the origin exactly"
    );
    assert_eq!(
        (record.splay, record.spread, record.stagger),
        (17.0, None, 3.0),
        "one Undo keeps the angle and the uncompensated steps"
    );
}

/// Ticket 23 slice 6: an Origin drag snaps to its column base (the ergogen-style reset
/// target). Alt disables the snap, and a subsequent drag recovers.
#[wasm_bindgen_test]
async fn origin_drag_snaps_to_the_column_base_without_alt() {
    // An off-grid base: the grid alone would land this drop at 23.8125, never exactly on the
    // base.
    let runtime = open_two_column_runtime(|record| {
        record.spread = Some(22.0);
        record.splay_origin = Vec2 { x: 5.0, y: 0.0 };
    })
    .await;
    let basis = matrix_and_scene(&runtime, fixture::MATRIX_ID).1.columns[1].clone();
    assert_eq!(basis.base, Vec2 { x: 22.0, y: 0.0 });
    assert_eq!(basis.splay_origin, Vec2 { x: 27.0, y: 0.0 });
    let splay_origin = || fixture::accepted_matrix(&runtime).column_layout[1].splay_origin;
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Origin).await;
    let handle = rail.handle();
    assert_eq!(
        handle.get_attribute("aria-label").as_deref(),
        Some("Move splay origin")
    );
    let client = rail.client();
    // Raw drop (23, 1): 1.4 mm from the base, off the grid.
    let (from, to) = (client(27.0, 0.0), client(23.0, 1.0));
    let before = revision(&runtime);
    dispatch_pointer(&handle, "pointerdown", false, false, 1, from);
    frame().await;
    dispatch_pointer(&handle, "pointermove", false, false, 1, to);
    pump_preview(&runtime).await;
    dispatch_pointer(&handle, "pointerup", false, false, 0, to);
    pump_past(&runtime, before).await;
    let snapped = splay_origin();
    assert!(
        snapped.x.abs() < 1e-9 && snapped.y.abs() < 1e-9,
        "the drag snaps to the column base, not the grid: {snapped:?}"
    );
    undo(&runtime).await;
    assert_eq!(
        splay_origin(),
        Vec2 { x: 5.0, y: 0.0 },
        "one Undo restores the custom origin"
    );
    // Alt disables the snap: the same gesture lands on the raw drop, and the recovered drag
    // still commits. The overlay remounts because the commit moved the owner revision on.
    drop(rail);
    let rail = mount_column_tool(&runtime, LayoutTransformTool::Origin).await;
    let client = rail.client();
    drag_commit(
        &runtime,
        &rail.handle(),
        client(27.0, 0.0),
        client(23.0, 1.0),
        false,
        true,
    )
    .await;
    // One client pixel is a quarter world mm, and Alt skips the snap that would absorb the
    // roundtrip: the drop lands on the raw point, which is neither the base nor a grid step.
    let raw = splay_origin();
    assert!(
        (raw.x - 1.0).abs() < 0.25 && (raw.y - 1.0).abs() < 0.25,
        "Alt lands the raw drop without snapping: {raw:?}"
    );
}
