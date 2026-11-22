//! Column, row and carried-field tests for the matrix transform Inspector.
use super::*;

/// A two-key column whose first column carries `splay`/`rotation` layout and an
/// optional `offset_y`, for column-scope Inspector scenarios.
fn column_layout_fixture(splay: f64, rotation: f64, offset_y: f64) -> ProjectDoc {
    let (mut document, _) = two_key_fixture(false);
    let column_layout = &mut document.matrices[0].column_layout;
    column_layout.resize_with(1, Default::default);
    column_layout[0].splay = splay;
    column_layout[0].rotation = rotation;
    column_layout[0].offset = Vec2 {
        x: 0.0,
        y: offset_y,
    };
    document
}

fn angled_column_fixture(splay: f64, rotation: f64) -> ProjectDoc {
    column_layout_fixture(splay, rotation, 0.0)
}

#[wasm_bindgen_test]
async fn column_angle_scope_switches_display_and_discards_the_other_fields_draft() {
    let (probe, root) = mount_probe(angled_column_fixture(7.0, 10.0), column_context(0), &[]).await;
    let input: HtmlInputElement = query(&root, "input[aria-label='Splay']")
        .unwrap()
        .unchecked_into();
    assert_eq!(input.value(), "10", "This column displays Column rotation");
    let before = probe.accepted().document;
    type_value(&input, "99");
    probe.settle().await;
    let mut affect = probe.mount().splay_affect;
    affect.set(boardstudio_model::model::MatrixSplayAffect::Following);
    probe.settle().await;
    assert_eq!(
        input.value(),
        "7",
        "switching fields resets the dirty draft"
    );
    assert_eq!(probe.accepted().document, before);
    affect.set(boardstudio_model::model::MatrixSplayAffect::Column);
    probe.settle().await;
    assert_eq!(input.value(), "10");
    edit_field(&probe, &root, "Splay", "0").await;
    let record = probe.matrix("matrix").column_layout[0];
    assert_eq!((record.rotation, record.splay), (0.0, 7.0));
}

#[wasm_bindgen_test]
async fn column_angle_equality_uses_the_requested_field_in_the_accepted_snapshot() {
    use super::super::super::matrix_transform_controller::transform_edit_resolver;
    use boardstudio_application::Resolution;
    use boardstudio_model::model::MatrixSplayAffect;
    let runtime = support::new_runtime();
    support::open_document(&runtime, angled_column_fixture(0.0, 10.0)).await;
    let accepted = runtime.model().accepted.unwrap();
    let resolve = |value, affect| {
        transform_edit_resolver(
            "matrix".into(),
            "board".into(),
            column_context(0),
            MatrixTransformField::ColumnSplay,
            MatrixTransformValue::Number(value),
            affect,
            vec![],
        )
        .resolve(&accepted)
    };
    assert!(
        matches!(
            resolve(10.0, MatrixSplayAffect::Column),
            Resolution::Unchanged
        ),
        "requesting the current rotation is a no-op even when Splay differs"
    );
    assert!(
        matches!(
            resolve(0.0, MatrixSplayAffect::Column),
            Resolution::Submit(_)
        ),
        "zero Splay must not retire a request to clear nonzero rotation"
    );
    assert!(matches!(
        resolve(0.0, MatrixSplayAffect::Following),
        Resolution::Unchanged
    ));
}

#[wasm_bindgen_test]
async fn inspector_stagger_lands_as_a_placement_edit_that_moves_later_columns_with_one_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(0), &[]).await;
    assert!(has_exact_text(&root, "Column 1"));
    let second_column_y = || probe.key_at(0, 1).y;
    // The fixture's key poses were never laid out by Core; the first edit syncs them, so
    // measure the second change from that synced state.
    edit_field(&probe, &root, "Stagger", "1").await;
    let before = second_column_y();
    let revision = probe.revision();
    edit_field(&probe, &root, "Stagger", "4").await;
    assert_eq!(
        probe.matrix("matrix").column_layout[0].stagger,
        4.0,
        "the Inspector value is the matrix's column stagger"
    );
    let axis = probe.accepted().scene.matrix_scenes[0].columns[0].stagger_axis;
    let moved = second_column_y() - before;
    assert!(
        (moved - 3.0 * axis.y).abs() < 1e-9 && axis.y != 0.0,
        "the later column moved 3 mm along Core's stagger axis ({moved} vs {})",
        3.0 * axis.y
    );
    assert_eq!(probe.revision(), revision + 1, "one accepted edit");
    probe.undo().await;
    assert_eq!(
        probe.matrix("matrix").column_layout[0].stagger,
        1.0,
        "one Undo reverts the whole second stagger edit"
    );
    assert!((second_column_y() - before).abs() < 1e-9);
}

fn offset_column_fixture(splay: f64) -> ProjectDoc {
    column_layout_fixture(splay, 0.0, -9.525)
}

#[wasm_bindgen_test]
async fn convert_to_stagger_appears_only_for_a_nonzero_offset_y_and_is_one_undo_step() {
    // No offset Y: no action.
    let (document, _) = two_key_fixture(false);
    let (_probe, root) = mount_probe(document, column_context(0), &[]).await;
    assert!(button_named(&root, "Convert to stagger").is_none());
    drop(root);

    let (probe, root) = mount_probe(offset_column_fixture(0.0), column_context(0), &[]).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(!button.has_attribute("disabled"));
    let revision = probe.revision();
    button.click();
    probe.run().await;
    let matrix = probe.matrix("matrix");
    assert_eq!(probe.revision(), revision + 1, "one accepted edit");
    let staggers = |matrix: &Matrix| {
        matrix
            .column_layout
            .iter()
            .map(|record| record.stagger)
            .collect::<Vec<_>>()
    };
    assert_eq!(staggers(&matrix), vec![-9.525, 9.525]);
    assert_eq!(matrix.column_layout[0].offset.y, 0.0);
    // The probe host re-renders only when its version signal moves.
    probe.settle().await;
    assert_eq!(field_number(&root, "Stagger"), -9.525);
    assert_eq!(field_number(&root, "Offset Y"), 0.0);
    assert!(
        button_named(&root, "Convert to stagger").is_none(),
        "nothing left to convert"
    );
    probe.undo().await;
    let undone = probe.matrix("matrix");
    assert_eq!(undone.column_layout[0].offset.y, -9.525);
    assert!(staggers(&undone).iter().all(|value| *value == 0.0));
}

#[wasm_bindgen_test]
async fn convert_to_stagger_is_disabled_with_core_reason_on_a_splayed_column() {
    let (probe, root) = mount_probe(offset_column_fixture(10.0), column_context(0), &[]).await;
    let button = button_named(&root, "Convert to stagger").expect("the action is mounted");
    assert!(
        button.has_attribute("disabled"),
        "a splayed column cannot convert"
    );
    let reason =
        boardstudio_model::model::convert_offset_available(&probe.matrix("matrix"), 0).unwrap_err();
    assert!(has_exact_text(&root, &reason));
    assert_eq!(
        reason,
        "Converting requires zero Splay, Column rotation and carried Bend"
    );
}

#[wasm_bindgen_test]
async fn collapsing_column_sections_preserves_drafts_nodes_and_edit_owner() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(0), &[]).await;
    let accepted = probe.revision();
    let position = by_role_name(&root, "button", "Position and rotation")
        .expect("position section is a mounted disclosure");
    let splay = by_role_name(&root, "button", "Splay and origin")
        .expect("splay section is a mounted disclosure");
    let expanded = |button: &web_sys::HtmlElement| button.get_attribute("aria-expanded");
    assert_eq!(
        expanded(&splay).as_deref(),
        Some("false"),
        "specialist origin controls start collapsed"
    );
    splay.click();
    let input = numeric_field(&root, "Stagger");
    type_value(&input, "");
    probe.settle().await;
    position.click();
    wait_until("position section collapses", || {
        expanded(&position).as_deref() == Some("false")
    })
    .await;
    assert_eq!(
        expanded(&splay).as_deref(),
        Some("true"),
        "sections can be collapsed independently"
    );
    probe.settle().await;
    assert_eq!(
        expanded(&position).as_deref(),
        Some("false"),
        "unrelated renders retain collapsed state"
    );
    assert!(
        input.is_same_node(
            by_role_name(&root, "spinbutton", "Stagger")
                .as_ref()
                .map(|node| node.as_ref())
        ),
        "collapse never remounts field ownership"
    );
    position.click();
    probe.settle().await;
    assert_eq!(input.value(), "");
    assert_eq!(
        probe.revision(),
        accepted,
        "toggling sections never submits edits"
    );
}

#[wasm_bindgen_test]
async fn canvas_dimensions_preserve_inspector_drafts_and_share_undo() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_with(document, column_context(0), &[], true).await;
    let inspector = query(&root, "#inspector-geometry-test").unwrap();
    let canvas = query(&root, "#canvas-geometry-test").unwrap();
    let draft = numeric_field(&inspector, "Stagger");
    type_value(&draft, "");
    probe.settle().await;
    let before = probe.revision();
    edit_field(&probe, &canvas, "Stagger", "4").await;
    assert_eq!(probe.matrix("matrix").column_layout[0].stagger, 4.0);
    assert_eq!(probe.revision(), before + 1);
    assert_eq!(
        draft.value(),
        "",
        "canvas edits preserve an unfinished inspector draft"
    );
    probe.undo().await;
    let stagger = probe
        .matrix("matrix")
        .column_layout
        .first()
        .map_or(0.0, |record| record.stagger);
    assert_eq!(stagger, 0.0);
    assert_eq!(draft.value(), "");
}

/// What a carried row or key field case edits, and where its value lands.
#[derive(Clone, Copy, Debug)]
enum Carried {
    RowShift,
    KeyShift,
    RowBend,
    KeyBend,
}

/// Ticket 23: the Row and Key Inspectors name the carried Shift and Bend fields. Setting one
/// moves (Shift X) or turns (Bend) the edited row or key and the later keys it carries to
/// through Core's running frame as one accepted edit; a key's neighbouring column stays
/// still, and one Undo restores every key exactly.
#[wasm_bindgen_test]
async fn carried_row_and_key_fields_move_their_keys_through_cores_frame_with_one_undo() {
    for (case, label, value) in [
        (Carried::RowShift, "Shift X", 4.0),
        (Carried::KeyShift, "Shift X", 3.0),
        (Carried::RowBend, "Bend", 10.0),
        (Carried::KeyBend, "Bend", 10.0),
    ] {
        let (document, key) = match case {
            Carried::RowShift | Carried::RowBend => two_row_fixture(),
            Carried::KeyShift | Carried::KeyBend => shift_grid_fixture(),
        };
        let context = match case {
            Carried::RowShift | Carried::RowBend => row_context(0),
            Carried::KeyShift | Carried::KeyBend => key,
        };
        let (probe, root) = mount_probe(document, context, &[]).await;
        let keys: &[(u32, u32)] = match case {
            Carried::RowShift | Carried::RowBend => &[(0, 0), (1, 0)],
            Carried::KeyShift | Carried::KeyBend => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        };
        let poses = || {
            keys.iter()
                .map(|&(row, column)| probe.scene_pose("matrix", row, column))
                .collect::<Vec<_>>()
        };
        let before = poses();
        let revision = probe.revision();
        edit_field(&probe, &root, label, &value.to_string()).await;
        // The stored field as (x, y) for a Shift and (degrees, 0) for a Bend.
        let stored = || {
            let matrix = probe.matrix("matrix");
            let row = matrix.row_layout.first().copied().unwrap_or_default();
            let cell = matrix
                .cells
                .into_iter()
                .find(|cell| cell.row == 0 && cell.column == 0)
                .expect("the edited key keeps its cell");
            match case {
                Carried::RowShift => (row.shift.x, row.shift.y),
                Carried::KeyShift => cell.shift.map_or((0.0, 0.0), |shift| (shift.x, shift.y)),
                Carried::RowBend => (row.bend, 0.0),
                Carried::KeyBend => (cell.bend.unwrap_or_default(), 0.0),
            }
        };
        assert_eq!(
            stored(),
            (value, 0.0),
            "{case:?}: the Inspector value is the carried field"
        );
        assert_eq!(
            probe.revision(),
            revision + 1,
            "{case:?}: one accepted edit"
        );
        for ((&(row, column), from), to) in keys.iter().zip(&before).zip(poses()) {
            // Column 0 holds the edited row or key and everything it carries to.
            let carried = if column == 0 { value } else { 0.0 };
            let ok = match case {
                Carried::RowShift | Carried::KeyShift => moved_by(from.at, to.at, carried, 0.0),
                Carried::RowBend | Carried::KeyBend => (to.rotation - carried).abs() < 1e-9,
            };
            assert!(
                ok,
                "{case:?}: key ({row}, {column}) carries {carried}, got {to:?}"
            );
        }
        probe.undo().await;
        assert_eq!(
            stored(),
            (0.0, 0.0),
            "{case:?}: one Undo clears the carried field"
        );
        for ((&(row, column), from), to) in keys.iter().zip(&before).zip(poses()) {
            assert!(
                close(from.at, to.at) && (from.rotation - to.rotation).abs() < 1e-9,
                "{case:?}: one Undo restores key ({row}, {column}) exactly"
            );
        }
    }
}

/// Ticket 23: the Column Inspector names the inherited Spread step and the Row Inspector the
/// Padding step. Setting one moves its own column or row past the 19 mm pitch as one accepted
/// edit without moving column or row 0, and one Undo restores both exactly.
#[wasm_bindgen_test]
async fn spread_and_padding_move_only_their_own_column_or_row_with_one_undo() {
    for (label, spread) in [("Spread", true), ("Padding", false)] {
        let (document, context, moved_key) = if spread {
            (two_key_fixture(false).0, column_context(1), (0, 1))
        } else {
            (two_row_fixture().0, row_context(1), (1, 0))
        };
        let (probe, root) = mount_probe(document, context, &[]).await;
        let (moved, still) = (
            || probe.key_at(moved_key.0, moved_key.1),
            || probe.key_at(0, 0),
        );
        let (before_moved, before_still) = (moved(), still());
        let revision = probe.revision();
        edit_field(&probe, &root, label, "24").await;
        let matrix = probe.matrix("matrix");
        let stored = if spread {
            matrix.column_layout.get(1).and_then(|record| record.spread)
        } else {
            matrix.row_layout.get(1).and_then(|record| record.padding)
        };
        assert_eq!(
            stored,
            Some(24.0),
            "the Inspector value is the explicit {label}"
        );
        assert_eq!(probe.revision(), revision + 1, "one accepted edit");
        let (dx, dy) = if spread { (5.0, 0.0) } else { (0.0, 5.0) };
        assert!(
            moved_by(before_moved, moved(), dx, dy),
            "its own {label} steps out the full value past the 19 mm pitch"
        );
        assert!(
            close(before_still, still()),
            "index 0 does not follow a later {label}"
        );
        probe.undo().await;
        assert!(
            close(before_moved, moved()) && close(before_still, still()),
            "one Undo restores both exactly"
        );
    }
}

/// Ticket 23: column 0 has no Spread step and row 0 no Padding step, and the Inspector says
/// why instead of leaving a silent gap.
#[wasm_bindgen_test]
async fn index_zero_hides_spread_and_padding_with_an_accessible_reason() {
    for (document, context, label, reason) in [
        (
            two_key_fixture(false).0,
            column_context(0),
            "Spread",
            "Spread starts at column 1. Column 0 steps by pitch.",
        ),
        (
            two_row_fixture().0,
            row_context(0),
            "Padding",
            "Padding starts at row 1. Row 0 sits at the column base.",
        ),
    ] {
        let (_probe, root) = mount_probe(document, context, &[]).await;
        assert!(
            query(&root, &format!("input[aria-label='{label}']")).is_none(),
            "index 0 has no {label} field"
        );
        assert_eq!(
            text_of(&root, "[role=note]"),
            reason,
            "the reason names the rule"
        );
    }
}

/// Ticket 23: a default Spread reads the pitch with a pitch badge; an explicit Spread reads
/// its own value with no badge.
#[wasm_bindgen_test]
async fn default_spread_shows_the_pitch_badge_and_explicit_spread_does_not() {
    let (document, _) = two_key_fixture(false);
    let (probe, root) = mount_probe(document, column_context(1), &[]).await;
    let italic = || {
        query(&root, "input[aria-label='Spread']")
            .expect("column 1 has a Spread field")
            .get_attribute("class")
            .unwrap_or_default()
            .split_whitespace()
            .any(|class| class == "italic")
    };
    assert_eq!(
        numeric_field(&root, "Spread").value(),
        "19",
        "the default Spread reads the pitch"
    );
    assert!(
        italic(),
        "the inherited pitch is visually distinguished from an explicit value"
    );
    assert!(
        query(&root, "[data-pitch-badge]").is_some(),
        "the default carries a pitch badge"
    );
    edit_field(&probe, &root, "Spread", "24").await;
    assert!(
        query(&root, "[data-pitch-badge]").is_none(),
        "an explicit Spread drops the badge"
    );
    assert!(!italic(), "an explicit value is not styled as inherited");
}

/// Ticket 23: Carried and This-only sections group the named fields with scope explanations.
#[wasm_bindgen_test]
async fn carried_and_this_only_sections_group_the_named_fields() {
    let (document, _) = two_key_fixture(false);
    let (_probe, root) = mount_probe(document, column_context(1), &[]).await;
    let headings = query_all(&root, "summary, h3")
        .iter()
        .filter_map(|heading| heading.text_content())
        .collect::<Vec<_>>();
    for section in ["Carried", "This column only"] {
        assert!(
            headings.iter().any(|heading| heading == section),
            "{section} is mounted"
        );
    }
    for field in ["Spread", "Stagger"] {
        assert!(
            query(&root, &format!("input[aria-label='{field}']")).is_some(),
            "{field} stays a named field"
        );
    }
}

/// Ticket 23: a committed edit names its changed field and promises one Undo entry; the line
/// clears when the next edit submits.
#[wasm_bindgen_test]
async fn committed_shift_names_the_changed_field_with_one_undo() {
    let (document, _) = two_row_fixture();
    let (probe, root) = mount_probe(document, row_context(0), &[]).await;
    let status = || query(&root, "[data-matrix-change-status]");
    assert!(
        status().is_none(),
        "no change is announced before the first commit"
    );
    edit_field(&probe, &root, "Shift X", "4").await;
    assert_eq!(
        status().and_then(|status| status.text_content()).as_deref(),
        Some("Changed Shift X · one Undo entry"),
        "the commit names the changed field"
    );
    set_numeric_field(&root, "Shift X", "5");
    probe.run().await;
    assert_eq!(probe.matrix("matrix").row_layout[0].shift.x, 5.0);
    let status_text = status()
        .and_then(|element| element.text_content())
        .unwrap_or_default();
    assert!(
        status_text.contains("Changed Shift X · one Undo entry"),
        "expected Shift X announcement, got {status_text:?}"
    );
}
