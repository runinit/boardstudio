//! Native tests for the matrix transform operation mapping: catalogue admission, Inspector
//! field edits, placement gestures and nudges, matrix drag solving and per-key Omit/Rejoin.

use super::*;
use boardstudio_model::model::{
    EditOperation, Follow, KicadSource, Matrix, MatrixAnchor, MatrixAssembly, MatrixCell,
    MatrixPlacementChange, MatrixScene, MatrixSplayAffect, MatrixSplayChange, Mirror,
    PartDefinition, PartGenerator, Pose2, Vec2, convert_offset_available,
};
use serde_json::json;

mod key_omit;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

fn component_definition(id: String, generator: Option<&str>, kicad: bool) -> PartDefinition {
    let mut definition: PartDefinition = serde_json::from_value(json!({
        "id": id, "name": format!("Label {id}"), "kind": "passive", "courtyard": [], "pads": []
    }))
    .unwrap();
    definition.kicad_source = kicad.then(|| KicadSource {
        format_version: 1,
        source: "fixture".into(),
    });
    definition.generator = generator.map(|source| PartGenerator {
        source: source.into(),
        version: "1".into(),
        parameters: Default::default(),
    });
    definition
}

/// A 2x3 matrix with row and column layouts and one edited key at row 1, column 2.
fn matrix() -> Matrix {
    serde_json::from_value(json!({
        "id": "matrix-1", "name": "Fixture", "rows": 2, "columns": 3,
        "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": 12.0, "y": -4.0},
        "definitionId": "switch", "partIds": ["p0"], "boardId": "board-1", "rotation": 0.0,
        "rowLayout": [{"offset": {"x": 2.0, "y": 3.0}}, {"offset": {"x": 4.0, "y": 5.0}}],
        "columnLayout": [
            {"offset": {"x": 1.0, "y": 2.0}, "stagger": 0.25},
            {"stagger": 0.5, "splay": 4.0},
            {"stagger": 0.75, "splay": 8.0}
        ],
        "cells": [{
            "row": 1, "column": 2, "enabled": true, "definitionId": "switch", "variant": "north",
            "offset": {"x": 6.0, "y": 7.0}, "rotation": 9.0, "assembliesLocal": true,
            "assemblies": [{"id": "led", "definitionId": "led-def",
                "offset": {"x": 1.0, "y": 0.0}, "rotation": 90.0}]
        }]
    }))
    .unwrap()
}

fn matrix_fields(mirror_y_locked: bool, anchor: Option<MatrixAnchor>) -> MatrixTransformFields {
    MatrixTransformFields::Matrix {
        origin: v(12.0, -4.0),
        rotation: 0.0,
        mirror: None,
        mirror_y_locked,
        rotation_followed: true,
        anchor,
    }
}

fn row_fields(row: u32, offset: Vec2, bend: f64) -> MatrixTransformFields {
    MatrixTransformFields::Row {
        row,
        offset,
        shift: Vec2::default(),
        bend,
        padding: None,
        pitch: 19.05,
    }
}

/// Column fields with zero rotation, no spread and Core's convert verdict `Ok`.
fn column_fields(column: u32, offset: Vec2, stagger: f64, splay: f64) -> MatrixTransformFields {
    MatrixTransformFields::Column {
        column,
        offset,
        stagger,
        spread: None,
        pitch: 19.05,
        splay_angle: splay,
        rotation: 0.0,
        splay_origin: Vec2::default(),
        custom_origin: false,
        convert: Ok(()),
    }
}

fn with_splay_origin(
    mut fields: MatrixTransformFields,
    at: Vec2,
    custom: bool,
) -> MatrixTransformFields {
    if let MatrixTransformFields::Column {
        splay_origin,
        custom_origin,
        ..
    } = &mut fields
    {
        (*splay_origin, *custom_origin) = (at, custom);
    }
    fields
}

fn key_at(row: u32, column: u32, offset: Vec2, rotation: f64) -> MatrixTransformFields {
    MatrixTransformFields::Key {
        enabled: true,
        definition_id: "switch".into(),
        choices: Vec::new(),
        assemblies: Vec::new(),
        component_choices: Vec::new(),
        mirror_target: false,
        assemblies_local: false,
        half_exception: None,
        row,
        column,
        offset,
        rotation,
        shift: Vec2::default(),
        bend: 0.0,
    }
}

/// The fixture's edited key (row 1, column 2) as the Inspector projects it.
fn key_fields() -> MatrixTransformFields {
    key_at(1, 2, v(6.0, 7.0), 9.0)
}

fn paired_key_fields(target: bool, local: bool) -> MatrixTransformFields {
    let mut fields = key_fields();
    if let MatrixTransformFields::Key {
        mirror_target,
        assemblies_local,
        ..
    } = &mut fields
    {
        (*mirror_target, *assemblies_local) = (target, local);
    }
    fields
}

fn build(
    matrix: &Matrix,
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    value: MatrixTransformValue,
) -> Result<EditOperation, String> {
    build_operation(matrix, fields, field, value, MatrixSplayAffect::Following)
}

fn placement_result(operation: EditOperation) -> MatrixPlacementChange {
    let EditOperation::SetMatrixPlacement { change, .. } = operation else {
        panic!("this field maps to SetMatrixPlacement");
    };
    change
}

fn edited_matrix(
    matrix: &Matrix,
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    value: MatrixTransformValue,
) -> Matrix {
    let EditOperation::SetMatrix { matrix, .. } = build(matrix, fields, field, value).unwrap()
    else {
        panic!("this field maps to SetMatrix");
    };
    matrix
}

fn edited_cell(
    matrix: &Matrix,
    fields: &MatrixTransformFields,
    field: MatrixTransformField,
    value: MatrixTransformValue,
) -> MatrixCell {
    edited_matrix(matrix, fields, field, value).cells.remove(0)
}

// --- Catalogue admission ----------------------------------------------------------------

#[test]
fn attached_component_choices_keep_only_this_members_snapshot_and_preserve_order() {
    let current = "assembly-preset-mx-hotswap-south-left-keys-0/definition/diode".to_owned();
    let catalogue = |index: u32| component_definition(format!("catalogue-{index:02}"), None, false);
    let mut definitions = (0..30)
        .map(|index| match index {
            12 => component_definition(
                "assembly-imported-board/definition/diode".into(),
                None,
                true,
            ),
            _ => catalogue(index),
        })
        .collect::<Vec<_>>();
    definitions.extend((0..7).map(|index| {
        component_definition(
            format!("assembly-placement-{index}/definition/switch"),
            None,
            false,
        )
    }));
    definitions.push(component_definition(current.clone(), None, false));
    definitions.extend((30..51).map(catalogue));

    let choices = attachment_component_choices(&definitions, &current);

    let label = |definition: &PartDefinition| (definition.id.clone(), definition.name.clone());
    let mut expected = definitions[..30].iter().map(label).collect::<Vec<_>>();
    assert_eq!(definitions[37].id, current);
    expected.extend(definitions[37..].iter().map(label));
    assert_eq!(choices, expected);
    assert_eq!(choices.len(), 52);
    let has = |id: &str| choices.iter().any(|(choice, _)| choice == id);
    assert!(has(&current) && has("assembly-imported-board/definition/diode"));
    assert!(
        !choices
            .iter()
            .any(|(id, _)| id.starts_with("assembly-placement-"))
    );

    let retired = component_definition(
        "retired-nice-nano".into(),
        Some("infused-kim/nice_nano_pretty"),
        false,
    );
    assert!(attachment_component_choices(&[retired], "retired-nice-nano").is_empty());
}

#[test]
fn assembly_edits_wait_for_a_matching_catalogue_only_when_they_need_one() {
    use MatrixTransformField::{KeyAssembly, KeyAttached};
    use MatrixTransformValue::{Attached, Text};
    let ids = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
    let switch = Text("switch".into());
    let assembly = |built_for, reversible, to: &str, document: &[&str]| {
        let to = Text(to.into());
        component_edit_admitted_by_catalogue(
            built_for,
            reversible,
            KeyAssembly,
            &switch,
            &to,
            &ids(document),
        )
    };
    let attachment = |reversible, from: &[&str], to: &[&str], document: &[&str]| {
        let attached = |definitions: &[&str]| {
            let pairs = definitions
                .iter()
                .map(|definition| ("led".into(), definition.to_string()));
            Attached(pairs.collect())
        };
        let (from, to) = (attached(from), attached(to));
        component_edit_admitted_by_catalogue(
            None,
            reversible,
            KeyAttached,
            &from,
            &to,
            &ids(document),
        )
    };

    // A failed catalogue does not block removing a document-owned attachment.
    assert!(attachment(false, &["led-def"], &[], &["led-def"]));
    // An assembly that needs the catalogue waits for the matching construction's catalogue.
    assert!(!assembly(None, false, "bundled-switch", &["switch"]));
    assert!(assembly(Some(false), false, "bundled-switch", &["switch"]));
    // Document-owned assembly changes and attachment replacements need no catalogue.
    assert!(assembly(
        None,
        true,
        "alternate-switch",
        &["switch", "alternate-switch"]
    ));
    let led = ["led-original", "led-replacement"];
    assert!(attachment(true, &led[..1], &led[1..], &led));

    // The admitted removal uses the existing accepted edit operation.
    let mut fields = key_fields();
    if let MatrixTransformFields::Key { assemblies, .. } = &mut fields {
        *assemblies = vec![("led".into(), "led-def".into())];
    }
    let next = edited_matrix(&matrix(), &fields, KeyAttached, Attached(Vec::new()));
    assert!(next.cells[0].assemblies.is_empty());
}

#[test]
fn failed_catalogue_can_be_retried_for_the_same_construction() {
    let due =
        |requested, loading, built_for| catalogue_retry_due(requested, loading, built_for, false);
    assert!(!due(None, false, None), "nothing is requested yet");
    // Initial effect request, followed by a failed load.
    assert!(!due(Some(false), true, None));
    assert!(due(Some(false), false, None));
    // A user assembly action clears the failed request; the effect starts it again.
    assert!(!due(None, false, None));
    assert!(!due(Some(false), true, None));
    // A successful retry tags the construction as ready and prevents another retry.
    assert!(!due(Some(false), false, Some(false)));
}

// --- Inspector field edits --------------------------------------------------------------

#[test]
fn convert_offset_builds_the_one_shot_placement_and_follows_core_availability() {
    let mut matrix = matrix();
    let fields = column_fields(0, v(0.0, -9.525), 0.0, 0.0);
    let convert = |matrix: &Matrix| {
        let value = MatrixTransformValue::Number(0.0);
        build(
            matrix,
            &fields,
            MatrixTransformField::ColumnOffsetConvert,
            value,
        )
    };
    assert_eq!(
        placement_result(convert(&matrix).unwrap()),
        MatrixPlacementChange::ColumnOffsetToStagger { column: 0 }
    );
    // A splayed column with a default origin is refused with Core's own reason.
    for (index, record) in matrix.column_layout.iter_mut().enumerate() {
        record.splay = if index == 0 { 12.0 } else { 0.0 };
    }
    assert_eq!(
        convert(&matrix),
        Err(convert_offset_available(&matrix, 0).unwrap_err())
    );
}

/// Every field that maps to one typed placement, splay or anchor operation, with the exact
/// operation it sends: only the edited component changes and nothing else is sent.
#[test]
fn fields_map_to_their_exact_typed_operations() {
    use MatrixPlacementChange as Change;
    use MatrixTransformField::*;
    use MatrixTransformValue::{Bool, Number, Offset, OriginMode, Point, Text};
    let matrix = matrix();
    let check = |fields: &MatrixTransformFields, field, value: MatrixTransformValue, expected| {
        let actual = build(&matrix, fields, field, value.clone());
        assert_eq!(actual, Ok(expected), "{field:?} = {value:?}");
    };
    let place = |change| EditOperation::SetMatrixPlacement {
        matrix_id: "matrix-1".into(),
        change,
    };
    let splay = |column, change| EditOperation::SetMatrixSplay {
        matrix_id: "matrix-1".into(),
        column,
        change,
    };
    let origin = |column, x, y| {
        splay(
            column,
            MatrixSplayChange::Origin {
                world: Some(v(x, y)),
            },
        )
    };
    let row_offset = |row, x, y| {
        place(Change::RowOffset {
            row,
            value: v(x, y),
        })
    };
    let column_offset = |x, y| {
        place(Change::ColumnOffset {
            column: 2,
            value: v(x, y),
        })
    };

    let zero = || Offset(Vec2::default());
    let row1 = row_fields(1, v(4.0, 5.0), 2.0);
    let row_bend = Change::RowBend { row: 1, value: 7.5 };
    check(&row1, RowBend, Number(7.5), place(row_bend));
    let padding = Change::RowPadding {
        row: 1,
        value: Some(24.0),
    };
    check(&row1, RowPadding, Number(24.0), place(padding));
    // A row edit sends the absolute row offset and nothing else.
    check(&row1, RowOffsetX, Number(-2.5), row_offset(1, -2.5, 5.0));
    let row0 = row_fields(0, v(2.0, 3.0), 0.0);
    check(&row0, RowOffsetY, Number(-6.0), row_offset(0, 2.0, -6.0));
    check(&row0, RowOffsetReset, zero(), row_offset(0, 0.0, 0.0));
    let bend = Change::KeyBend {
        row: 1,
        column: 2,
        value: -3.0,
    };
    check(&key_fields(), KeyBend, Number(-3.0), place(bend));

    let spread = Change::ColumnSpread {
        column: 1,
        value: Some(24.0),
    };
    let spread_column = column_fields(1, Vec2::default(), 0.5, 0.0);
    check(&spread_column, ColumnSpread, Number(24.0), place(spread));
    let col = column_fields(2, v(1.0, 2.0), 0.75, 8.0);
    let col = with_splay_origin(col, v(11.0, 12.0), true);
    check(&col, ColumnOffsetX, Number(5.0), column_offset(5.0, 2.0));
    check(&col, ColumnOffsetY, Number(-3.0), column_offset(1.0, -3.0));
    check(&col, ColumnOffsetReset, zero(), column_offset(0.0, 0.0));
    let stagger = Change::ColumnStagger {
        column: 2,
        value: -1.5,
    };
    check(&col, ColumnStagger, Number(-1.5), place(stagger));
    check(&col, SplayOriginX, Number(13.0), origin(2, 13.0, 12.0));
    check(&col, SplayOriginY, Number(-2.0), origin(2, 11.0, -2.0));
    let unset = column_fields(2, v(0.0, 0.0), 0.0, 8.0);
    let unset = with_splay_origin(unset, v(11.0, 12.0), false);
    let custom = OriginMode(true);
    check(&unset, SplayOriginMode, custom, origin(2, 11.0, 12.0));

    // Picking the splay origin is one atomic origin edit; a Ctrl-click pick writes an
    // uncompensated swing.
    let picked = column_fields(1, v(0.0, 0.0), 0.5, 4.0);
    let picked = with_splay_origin(picked, v(10.0, 20.0), true);
    let point = v(33.5, -12.0);
    let pick = || Point(point);
    check(&picked, SplayOriginPoint, pick(), origin(1, 33.5, -12.0));
    let swing = MatrixSplayChange::OriginSwing { world: Some(point) };
    check(&picked, SplayOriginSwingPoint, pick(), splay(1, swing));

    // Detach maps to one atomic Core intent; the confirmed one names the local exception.
    let unanchored = matrix_fields(false, None);
    let detach = EditOperation::DetachMatrixAnchor {
        matrix_id: "matrix-1".into(),
    };
    check(&unanchored, MatrixDetach, Bool(true), detach);
    let confirmed = EditOperation::DetachMatrixAnchorConfirmed {
        matrix_id: "matrix-1".into(),
        local_matrix_id: "partner-mirror".into(),
    };
    let partner = Text("partner-mirror".into());
    check(&unanchored, MatrixDetachConfirm, partner, confirmed);

    // Splay sends the Inspector's affect choice unchanged.
    let affect = || MatrixSplayAffect::Column;
    let angle = MatrixSplayChange::Angle {
        angle: 12.5,
        affect: affect(),
    };
    let actual = build_operation(&matrix, &unset, ColumnSplay, Number(12.5), affect());
    assert_eq!(actual, Ok(splay(2, angle)));
}

#[test]
fn inspector_sets_compose_per_component_against_the_resolved_current_value() {
    use PlacementSet as Set;
    use PlacementValue::{Bend, Padding, Spread};
    let row_bend = PlacementParameter::RowBend { row: 1 };
    let spread = PlacementParameter::ColumnSpread { column: 1 };
    let padding = PlacementParameter::RowPadding { row: 1 };
    assert_eq!(
        placement_value(row_bend, Bend(2.0), Set::Bend(7.5)),
        Bend(7.5)
    );
    assert_eq!(
        placement_value(spread, Spread(19.05), Set::Spread(24.0)),
        Spread(24.0)
    );
    assert_eq!(
        placement_value(padding, Padding(19.05), Set::Padding(24.0)),
        Padding(24.0)
    );
}

#[test]
fn matrix_fields_rewrite_only_their_own_matrix_property() {
    use MatrixTransformField::{MatrixRotation, OriginX, OriginY};
    let matrix = matrix();
    for (field, value, origin, rotation) in [
        (OriginX, 20.0, v(20.0, -4.0), 0.0),
        (OriginY, -8.0, v(12.0, -8.0), 0.0),
        (MatrixRotation, 33.0, v(12.0, -4.0), 33.0),
    ] {
        let number = MatrixTransformValue::Number(value);
        let next = edited_matrix(&matrix, &matrix_fields(false, None), field, number);
        assert_eq!(
            (next.origin, next.rotation),
            (origin, Some(rotation)),
            "{field:?}"
        );
        assert_eq!(next.cells, matrix.cells);
    }
}

#[test]
fn linked_layout_rejects_y_mirror_at_the_operation_boundary() {
    let mirror = MatrixTransformValue::Mirror(Some(Mirror::Y));
    let fields = matrix_fields(true, None);
    let error = build(
        &matrix(),
        &fields,
        MatrixTransformField::MatrixMirror,
        mirror,
    )
    .expect_err("linked layout mirrors must retain the restriction");
    assert!(error.contains("linked layout"));
}

#[test]
fn follow_toggles_rewrite_only_their_own_component() {
    use MatrixTransformField::{MatrixFollowRotation, MatrixFollowX, MatrixFollowY};
    let anchor = MatrixAnchor {
        matrix_id: "other".into(),
        row: 0,
        column: 1,
        follow: Follow::default(),
        aggregate: None,
    };
    let fields = matrix_fields(false, Some(anchor));
    for (field, expected) in [
        (MatrixFollowX, (false, true, true)),
        (MatrixFollowY, (true, false, true)),
        (MatrixFollowRotation, (true, true, false)),
    ] {
        let off = MatrixTransformValue::Bool(false);
        let EditOperation::SetMatrixAnchor { matrix_id, anchor } =
            build(&matrix(), &fields, field, off).unwrap()
        else {
            panic!("a Follow toggle reaches SetMatrixAnchor")
        };
        assert_eq!(matrix_id, "matrix-1");
        assert_eq!(
            (anchor.matrix_id.as_str(), anchor.row, anchor.column),
            ("other", 0, 1)
        );
        let follow = (anchor.follow.x, anchor.follow.y, anchor.follow.rotation);
        assert_eq!(follow, expected, "only the toggled component changes");
    }
}

// --- Key cell edits ---------------------------------------------------------------------

#[test]
fn key_transform_edits_preserve_assembly_and_cell_identity() {
    use MatrixTransformField::{KeyOffsetY, KeyRotation, KeyTransformReset};
    use MatrixTransformValue::{CellTransform, Number};
    let matrix = matrix();
    let reset = CellTransform {
        offset: v(0.0, 0.0),
        rotation: 0.0,
    };
    for (field, value, offset, rotation) in [
        (KeyTransformReset, reset, v(0.0, 0.0), 0.0),
        (KeyOffsetY, Number(-9.0), v(6.0, -9.0), 9.0),
        (KeyRotation, Number(45.0), v(6.0, 7.0), 45.0),
    ] {
        let next = edited_matrix(&matrix, &key_fields(), field, value);
        assert_eq!(next.cells.len(), 1);
        let cell = &next.cells[0];
        assert_eq!((cell.row, cell.column), (1, 2));
        assert_eq!(
            (cell.offset, cell.rotation),
            (Some(offset), Some(rotation)),
            "{field:?}"
        );
        assert_eq!(cell.definition_id.as_deref(), Some("switch"));
        assert_eq!(cell.variant.as_deref(), Some("north"));
        assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
        assert_eq!(cell.assemblies_local, Some(true));
    }
}

#[test]
fn key_enabled_toggle_preserves_cell_and_creates_missing_cell() {
    let matrix = matrix();
    let disable = |matrix: &Matrix| {
        let off = MatrixTransformValue::Bool(false);
        edited_matrix(matrix, &key_fields(), MatrixTransformField::KeyEnabled, off)
    };
    let next = disable(&matrix);
    assert_eq!(next.cells.len(), 1);
    assert!(!next.cells[0].enabled);
    assert_eq!(next.cells[0].offset, matrix.cells[0].offset);
    assert_eq!(next.cells[0].assemblies, matrix.cells[0].assemblies);

    let mut empty = matrix.clone();
    empty.cells.clear();
    let cell = &disable(&empty).cells[0];
    assert_eq!((cell.row, cell.column, cell.enabled), (1, 2, false));
}

#[test]
fn key_assembly_sets_definition_and_variant_without_losing_cell_state() {
    let matrix = matrix();
    let value = MatrixTransformValue::Text("alt-switch".into());
    let cell = edited_cell(
        &matrix,
        &key_fields(),
        MatrixTransformField::KeyAssembly,
        value,
    );
    assert_eq!(cell.definition_id.as_deref(), Some("alt-switch"));
    assert_eq!(cell.variant.as_deref(), Some("alt-switch"));
    assert_eq!(cell.offset, matrix.cells[0].offset);
    assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
}

#[test]
fn attached_components_replace_and_remove_by_id() {
    let mut matrix = matrix();
    let assembly = |id: &str, definition_id: &str, offset| MatrixAssembly {
        id: id.into(),
        definition_id: definition_id.into(),
        offset,
        rotation: None,
        side: None,
    };
    matrix.cells[0].assemblies = vec![
        assembly("diode", "diode-a", v(1.0, 2.0)),
        assembly("led", "led-a", v(3.0, 4.0)),
    ];
    let value = MatrixTransformValue::Attached(vec![("diode".into(), "diode-b".into())]);
    let cell = edited_cell(
        &matrix,
        &key_fields(),
        MatrixTransformField::KeyAttached,
        value,
    );
    assert_eq!(
        cell.assemblies,
        vec![assembly("diode", "diode-b", v(1.0, 2.0))]
    );
}

/// Only a linked mirror target keeps a changed assembly local to its key; the canonical half
/// keeps sharing, an unchanged edit never localizes, and Use mirrored clears only the flag.
#[test]
fn assembly_edits_localize_only_a_changed_linked_target_key() {
    use MatrixTransformField::{KeyAssembliesLocal, KeyAssembly, KeyAttached};
    use MatrixTransformValue::{Attached, Bool, Text};
    let mut shared = matrix();
    shared.cells[0].assemblies_local = None;
    let mut unchanged = shared.clone();
    unchanged.cells[0].variant = Some("switch".into());
    let mut created = shared.clone();
    created.cells.clear();
    let led = |definition: &str| Attached(vec![("led".into(), definition.into())]);
    let alt = || Text("alt-switch".into());
    let local = |matrix: &Matrix, target, field, value| {
        edited_cell(matrix, &paired_key_fields(target, false), field, value).assemblies_local
    };

    // The canonical half keeps sharing.
    let attached = edited_cell(
        &shared,
        &paired_key_fields(false, false),
        KeyAttached,
        led("led-b"),
    );
    assert_eq!(attached.assemblies[0].definition_id, "led-b");
    assert_eq!(attached.assemblies_local, None);
    assert_eq!(local(&shared, false, KeyAssembly, alt()), None);
    // A changed edit on a linked target key, or a new cell there, makes it local.
    assert_eq!(local(&shared, true, KeyAttached, led("led-b")), Some(true));
    assert_eq!(local(&shared, true, KeyAssembly, alt()), Some(true));
    assert_eq!(local(&created, true, KeyAssembly, alt()), Some(true));
    // An unchanged assembly does not localize a linked target key.
    assert_eq!(
        local(&unchanged, true, KeyAssembly, Text("switch".into())),
        None
    );
    assert_eq!(local(&unchanged, true, KeyAttached, led("led-def")), None);

    // Use mirrored components clears the local flag only.
    let matrix = matrix();
    let cell = edited_cell(
        &matrix,
        &paired_key_fields(true, true),
        KeyAssembliesLocal,
        Bool(false),
    );
    assert_eq!(cell.assemblies_local, Some(false));
    assert_eq!(cell.assemblies, matrix.cells[0].assemblies);
    assert_eq!(cell.definition_id, matrix.cells[0].definition_id);
}

#[test]
fn empty_key_cell_edit_adds_a_semantic_cell_without_part_ids() {
    let mut matrix = matrix();
    matrix.cells.clear();
    matrix.part_ids = vec!["existing-primary".into()];
    let fields = key_at(0, 1, v(0.0, 0.0), 0.0);
    let value = MatrixTransformValue::Number(-1.25);
    let next = edited_matrix(&matrix, &fields, MatrixTransformField::KeyOffsetX, value);
    assert_eq!(next.part_ids, vec!["existing-primary"]);
    assert_eq!(next.cells.len(), 1);
    let cell = &next.cells[0];
    assert_eq!(
        (cell.row, cell.column, cell.offset),
        (0, 1, Some(v(-1.25, 0.0)))
    );
    assert!(cell.enabled && cell.definition_id.is_none() && cell.assemblies.is_empty());
}

#[test]
fn cell_edit_preserves_order_and_untouched_cell_fields() {
    let mut matrix = matrix();
    matrix.cells.push(
        serde_json::from_value(json!({
            "row": 0, "column": 1, "enabled": false, "definitionId": "alternate",
            "variant": "south", "offset": {"x": -3.0, "y": 2.0}, "rotation": -4.0
        }))
        .unwrap(),
    );
    let value = MatrixTransformValue::Number(-1.0);
    let next = edited_matrix(
        &matrix,
        &key_fields(),
        MatrixTransformField::KeyOffsetX,
        value,
    );
    assert_eq!(next.cells.len(), 2);
    assert_eq!(next.cells[0].offset, Some(v(-1.0, 7.0)));
    assert_eq!(next.cells[0].assemblies, matrix.cells[0].assemblies);
    assert_eq!(next.cells[1], matrix.cells[1]);
}

// --- Placement: one module builds every placement edit -----------------------------------

/// A rotated, mirrored, splayed (custom origin) matrix, opened in a real `CoreEngine`.
fn transformed_scene() -> MatrixScene {
    use super::super::engine_test_support::{edit_engine, open_engine};
    use boardstudio_model::model::ProjectDoc;
    let mut document = serde_json::to_value(ProjectDoc::empty("p", "P")).unwrap();
    document["definitions"] = json!([{
        "id": "switch", "name": "switch", "kind": "switch",
        "courtyard": [{ "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
                      { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 2.0, "y": 2.0 }, "shape": "rect" }],
    }]);
    document["boards"] = json!([{ "id": "board", "name": "Board",
        "outlineIds": [], "partIds": [], "netIds": [], "thickness": 1.6 }]);
    let (mut engine, _, opened) =
        open_engine(serde_json::from_value(document).unwrap());
    let matrix: Matrix = serde_json::from_value(json!({
        "id": "m", "rows": 3, "columns": 4,
        "pitch": { "x": 19.0, "y": 19.0 }, "origin": { "x": 10.0, "y": 5.0 },
        "definitionId": "switch", "boardId": "board", "partIds": [],
        "rotation": 20.0, "mirror": "x",
        "columnLayout": [{"stagger": 0.0, "splay": 0.0},
            {"stagger": 3.0, "splay": 8.0, "splayOrigin": {"x": 30.0, "y": 12.0}},
            {"stagger": -2.0, "splay": 0.0}, {"stagger": 1.0, "splay": -6.0}]
    }))
    .unwrap();
    let (scene, _) = edit_engine(
        &mut engine,
        &opened,
        EditOperation::SetMatrix {
            matrix,
            definitions: None,
        },
    );
    scene.matrix_scenes.into_iter().next().unwrap()
}

/// The kind, indices and values of a placement change, for a tolerant comparison.
fn placement_parts(change: MatrixPlacementChange) -> (&'static str, Vec<u32>, Vec<f64>) {
    use MatrixPlacementChange as Change;
    match change {
        Change::ColumnStagger { column, value } => ("stagger", vec![column], vec![value]),
        Change::ColumnOffset { column, value } => ("column", vec![column], vec![value.x, value.y]),
        Change::RowOffset { row, value } => ("row", vec![row], vec![value.x, value.y]),
        Change::ColumnSpread { column, value } => {
            ("spread", vec![column], value.into_iter().collect())
        }
        Change::RowShift { row, value } => ("row-shift", vec![row], vec![value.x, value.y]),
        Change::KeyShift { row, column, value } => {
            ("key-shift", vec![row, column], vec![value.x, value.y])
        }
        other => panic!("{other:?} is not compared here"),
    }
}

#[test]
fn inspector_and_gesture_send_the_same_operation() {
    use PlacementParameter as P;
    use PlacementValue::{Offset, Shift, Spread, Stagger};
    let scene = transformed_scene();
    for (parameter, start, target) in [
        (P::Stagger { column: 1 }, Stagger(3.0), Stagger(7.5)),
        (
            P::ColumnOffset { column: 3 },
            Offset(v(1.0, -2.0)),
            Offset(v(-4.0, 6.5)),
        ),
        (
            P::RowOffset { row: 2 },
            Offset(v(0.0, 0.0)),
            Offset(v(2.5, -3.0)),
        ),
        (P::ColumnSpread { column: 1 }, Spread(19.0), Spread(24.0)),
        (
            P::RowShift { row: 2 },
            Shift(v(1.0, -2.0)),
            Shift(v(-4.0, 6.5)),
        ),
        (
            P::KeyShift { row: 1, column: 2 },
            Shift(v(0.5, 1.5)),
            Shift(v(-2.0, 3.0)),
        ),
    ] {
        let basis = SceneBasis::for_parameter(&scene, parameter).unwrap();
        // The world delta that moves the parameter from `start` to `target` is worked from
        // the scene axes, not from the function under test.
        let along = |delta: f64, axis: Vec2| v(delta * axis.x, delta * axis.y);
        let across = |from: Vec2, to: Vec2, x_axis: Vec2, y_axis: Vec2| {
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            v(dx * x_axis.x + dy * y_axis.x, dx * x_axis.y + dy * y_axis.y)
        };
        let set = |current, set| placement_value(parameter, current, set);
        let (world_delta, from_inspector) = match (start, target) {
            (Stagger(from), Stagger(to)) => (
                along(to - from, basis.stagger_axis),
                set(start, PlacementSet::Stagger(to)),
            ),
            (Offset(from), Offset(to)) => (
                across(from, to, basis.axis_x, basis.axis_y),
                set(
                    set(start, PlacementSet::OffsetX(to.x)),
                    PlacementSet::OffsetY(to.y),
                ),
            ),
            (Shift(from), Shift(to)) => (
                across(from, to, basis.shift_axis_x, basis.shift_axis_y),
                set(
                    set(start, PlacementSet::ShiftX(to.x)),
                    PlacementSet::ShiftY(to.y),
                ),
            ),
            (Spread(from), Spread(to)) => (
                along(to - from, basis.spread_axis.unwrap_or(basis.axis_x)),
                set(start, PlacementSet::Spread(to)),
            ),
            _ => unreachable!(),
        };
        let gesture = gesture_value(&basis, parameter, start, world_delta, None);
        let parts =
            |value| placement_parts(placement_result(placement_operation("m", parameter, value)));
        let ((kind, ids, a), (kind_b, ids_b, b)) = (parts(gesture), parts(from_inspector));
        assert_eq!((kind, &ids), (kind_b, &ids_b), "{parameter:?}");
        let close = a.len() == b.len() && a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 1e-9);
        // A Spread step is compared exactly.
        assert!(
            if kind == "spread" { a == b } else { close },
            "{parameter:?}: {a:?} vs {b:?}"
        );
    }
}

fn axis_basis(stagger: Vec2, axis_x: Vec2, axis_y: Vec2) -> SceneBasis {
    SceneBasis {
        stagger_axis: stagger,
        axis_x,
        axis_y,
        shift_axis_x: axis_x,
        shift_axis_y: axis_y,
        spread_axis: None,
    }
}

fn assert_close(actual: Option<PlacementValue>, expected: PlacementValue) {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-12;
    let close = match (actual, expected) {
        (Some(PlacementValue::Stagger(a)), PlacementValue::Stagger(b))
        | (Some(PlacementValue::Bend(a)), PlacementValue::Bend(b)) => near(a, b),
        (Some(PlacementValue::Offset(a)), PlacementValue::Offset(b)) => {
            near(a.x, b.x) && near(a.y, b.y)
        }
        _ => false,
    };
    assert!(close, "{actual:?} is not {expected:?}");
}

/// Grid-snapped gestures land exactly; Alt (no snap) gestures return the raw value.
#[test]
fn gesture_values_snap_staggers_to_the_grid_and_bends_to_half_degrees() {
    use PlacementValue::{Bend, Stagger};
    let basis = axis_basis(v(0.0, 1.0), v(1.0, 0.0), v(0.0, 1.0));
    let step = Some(v(4.7625, 4.7625));
    let stagger = PlacementParameter::Stagger { column: 0 };
    let row_bend = PlacementParameter::RowBend { row: 0 };
    let key_bend = PlacementParameter::KeyBend { row: 0, column: 1 };
    let drag =
        |parameter, start, x, y, snap| gesture_value(&basis, parameter, start, v(x, y), snap);

    // Off the grid, the first snap brings the value onto it; on the grid, the change is a
    // whole number of steps; motion across the stagger axis is ignored; Alt skips the snap.
    assert_eq!(drag(stagger, Stagger(0.3), 0.0, 4.5, step), Stagger(4.7625));
    assert_eq!(
        drag(stagger, Stagger(4.7625), 0.0, 4.7, step),
        Stagger(2.0 * 4.7625)
    );
    assert_eq!(drag(stagger, Stagger(0.0), 50.0, 0.0, step), Stagger(0.0));
    assert_close(
        Some(drag(stagger, Stagger(0.3), 0.0, 4.5, None)),
        Stagger(4.8),
    );

    // Ticket 05: a Bend drag turns horizontal world travel into degrees at the prototype
    // bend handle's half-degree rate: ten millimetres right turns the running frame five
    // degrees down. The grid step never reaches a bend: it snaps to half degrees, Alt keeps
    // the raw angle, and vertical-only travel holds the angle still. Row and key bends share
    // the mapping.
    assert_eq!(drag(row_bend, Bend(0.0), 10.0, 0.0, step), Bend(-5.0));
    assert_eq!(drag(row_bend, Bend(2.0), 3.3, 0.0, step), Bend(0.5));
    assert_close(Some(drag(key_bend, Bend(2.0), 3.3, 0.0, None)), Bend(0.35));
    assert_eq!(drag(row_bend, Bend(2.0), 0.0, 50.0, step), Bend(2.0));
}

#[test]
fn nudge_pairs_follow_the_axis() {
    use ArrowKey::{Down, Left, Right, Up};
    use PlacementValue::{Offset, Stagger};
    // An axis rotated 90 degrees on screen consumes Left/Right, not Up/Down.
    let rotated = axis_basis(v(1.0, 0.0), v(0.0, 1.0), v(-1.0, 0.0));
    let stagger = |key, big| {
        nudge_value(
            &rotated,
            PlacementParameter::Stagger { column: 0 },
            Stagger(0.0),
            key,
            big,
        )
    };
    assert_eq!(stagger(Up, false), None);
    assert_eq!(stagger(Down, false), None);
    assert_close(stagger(Right, false), Stagger(0.1));
    // Shift steps 1 mm.
    assert_close(stagger(Left, true), Stagger(-1.0));

    // A mirror-Y frame's ArrowUp increases the on-screen Y, which lowers its local Y.
    let mirrored = axis_basis(v(0.0, -1.0), v(1.0, 0.0), v(0.0, -1.0));
    let offset = PlacementParameter::ColumnOffset { column: 0 };
    let nudge = |key| nudge_value(&mirrored, offset, Offset(v(0.0, 0.0)), key, false);
    assert_close(nudge(Up), Offset(v(0.0, -0.1)));
    assert_close(nudge(Right), Offset(v(0.1, 0.0)));
}

// --- Whole-matrix drag ---------------------------------------------------------------------

#[test]
fn matrix_drag_solves_the_origin_through_the_anchor_frame_and_follow_basis() {
    let follow = |x, y| Follow {
        x,
        y,
        rotation: true,
    };
    let world = MatrixDragTransport::World;
    let reflected = MatrixDragTransport::Reflected {
        translation: v(100.0, 0.0),
        rotation: 0.0,
    };
    let anchored = |at, rotation, follow, transport| {
        Some(MatrixDragAnchor {
            key_pose: Pose2 { at, rotation },
            follow,
            transport,
        })
    };
    let solve = |origin, anchor: Option<MatrixDragAnchor>, delta| {
        let mut matrix = matrix();
        matrix.origin = origin;
        matrix.anchor = anchor.map(|anchor| MatrixAnchor {
            matrix_id: "matrix".into(),
            row: 0,
            column: 0,
            follow: anchor.follow,
            aggregate: None,
        });
        solve_matrix_origin(&matrix, anchor, delta)
    };
    let check = |solved: Vec2, x: f64, y: f64| {
        assert!(
            (solved.x - x).abs() < 1e-9 && (solved.y - y).abs() < 1e-9,
            "{solved:?}"
        );
    };
    let zero = Vec2::default();
    let full = Follow::default();

    // Unanchored matrices move one to one; anchored ones inverse through the anchor frame.
    check(solve(v(5.0, 7.0), None, v(10.0, -3.0)), 15.0, 4.0);
    check(
        solve(v(3.0, 4.0), anchored(zero, 0.0, full, world), v(10.0, 5.0)),
        13.0,
        9.0,
    );
    // Anchor key at (100, 200) turned 90 degrees, matrix origin (0, -19.05): the frame sits
    // at (119.05, 200). Dragging the frame +10 in world Y lands the origin at (10, -19.05).
    let turned = anchored(v(100.0, 200.0), 90.0, full, world);
    check(solve(v(0.0, -19.05), turned, v(0.0, 10.0)), 10.0, -19.05);
    // Unfollowed directions are ignored.
    let unfollowed_x = anchored(zero, 0.0, follow(false, true), world);
    check(solve(v(3.0, 4.0), unfollowed_x, v(10.0, 5.0)), 3.0, 9.0);
    // Linked partner mirrored about x = 50 in the source layout: with full Follow the world
    // drag applies exactly, round-tripping through the transported source-world axes; a
    // source-unfollowed direction is ignored.
    let transported = anchored(v(100.0, 0.0), 0.0, full, reflected);
    check(solve(zero, transported, v(10.0, 5.0)), 10.0, 5.0);
    let transported = anchored(v(100.0, 0.0), 0.0, follow(false, true), reflected);
    check(solve(zero, transported, v(10.0, 5.0)), 0.0, 5.0);
    // A drag outside every Follow direction solves to the same origin.
    let unfollowed = anchored(zero, 0.0, follow(false, false), world);
    check(solve(v(3.0, 4.0), unfollowed, v(10.0, 5.0)), 3.0, 4.0);
}

#[test]
fn rotate_solves_only_when_rotation_is_followed() {
    assert_eq!(solve_matrix_rotation(12.0, true, 30.0), Some(42.0));
    assert_eq!(solve_matrix_rotation(12.0, false, 30.0), None);
}
