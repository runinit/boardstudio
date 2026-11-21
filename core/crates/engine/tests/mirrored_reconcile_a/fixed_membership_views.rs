use super::*;

#[test]
fn selected_tombstone_survives_prepared_source_and_mirror_edits() {
    fn tombstone(document: &ProjectDoc, matrix_id: &str) -> boardstudio_model::model::MatrixCell {
        document
            .matrices
            .by_id(matrix_id)
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap()
            .clone()
    }

    fn assert_tombstone(document: &ProjectDoc, matrix_id: &str) {
        let matrix = document.matrices.by_id(matrix_id);
        let cell = tombstone(document, matrix_id);
        assert!(!cell.enabled && cell.deleted);
        let key_id = format!("matrix/{matrix_id}/r0c0");
        assert!(!matrix.part_ids.iter().any(|id| id == &key_id));
        assert!(
            !document
                .parts
                .iter()
                .any(|part| { part.id == key_id || part.id.starts_with(&format!("{key_id}/")) })
        );
        assert!(!document.boards.iter().any(|board| {
            board
                .part_ids
                .iter()
                .any(|id| id == &key_id || id.starts_with(&format!("{key_id}/")))
        }));
        let halves = document.first_halves();
        assert!(!halves.correspondences.iter().any(|association| {
            association.source_id == key_id
                || association.mirror_id == key_id
                || association.source_id.starts_with(&format!("{key_id}/"))
                || association.mirror_id.starts_with(&format!("{key_id}/"))
        }));
    }

    let accepted = fixture_document().unwrap();
    let (removed, _) = crate::support::prepare_transaction(
        &accepted,
        &[],
        &boardstudio_model::model::EditOperation::RemoveSelectedMatrixCells {
            cells: BTreeMap::from([("source-matrix".into(), vec![(0, 0)])]),
        },
    )
    .expect("selected-cell removal stores a valid matrix tombstone");
    let removed_tombstones = [
        tombstone(&removed, "source-matrix"),
        tombstone(&removed, "mirror-matrix"),
    ];
    assert_tombstone(&removed, "source-matrix");
    assert_tombstone(&removed, "mirror-matrix");

    let source_move = boardstudio_model::model::EditOperation::MoveParts {
        positions: vec![boardstudio_model::model::Position {
            id: "matrix/source-matrix/r0c1".into(),
            at: Vec2 { x: 43.0, y: 17.0 },
        }],
    };
    let (source_edited, _) = crate::support::prepare_transaction(&removed, &[], &source_move)
        .expect("later source edits reconcile around inactive membership");
    assert_tombstone(&source_edited, "source-matrix");
    assert_tombstone(&source_edited, "mirror-matrix");
    assert_eq!(
        tombstone(&source_edited, "source-matrix"),
        removed_tombstones[0]
    );
    assert_eq!(
        tombstone(&source_edited, "mirror-matrix"),
        removed_tombstones[1]
    );

    let mirror_move = boardstudio_model::model::EditOperation::MoveParts {
        positions: vec![boardstudio_model::model::Position {
            id: "matrix/mirror-matrix/r0c1".into(),
            at: Vec2 { x: 71.0, y: 83.0 },
        }],
    };
    let (mirror_edited, _) = crate::support::prepare_transaction(&source_edited, &[], &mirror_move)
        .expect("later mirror edits preserve inactive membership");
    assert_tombstone(&mirror_edited, "source-matrix");
    assert_tombstone(&mirror_edited, "mirror-matrix");
    assert_eq!(
        tombstone(&mirror_edited, "source-matrix"),
        removed_tombstones[0]
    );
    assert_eq!(
        tombstone(&mirror_edited, "mirror-matrix"),
        removed_tombstones[1]
    );
    let (repeated, repeated_changes) =
        crate::support::prepare_transaction(&mirror_edited, &[], &mirror_move)
            .expect("repeated mirror edit remains prepared");
    assert!(repeated_changes.is_empty());
    assert_eq!(repeated, mirror_edited);

    let (all_inactive, _) = crate::support::prepare_transaction(
        &removed,
        &[],
        &boardstudio_model::model::EditOperation::RemoveSelectedMatrixCells {
            cells: BTreeMap::from([("source-matrix".into(), vec![(0, 1)])]),
        },
    )
    .expect("all-slots-inactive paired matrices remain valid");
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let matrix = all_inactive.matrices.by_id(matrix_id);
        assert_eq!(matrix.part_ids, Vec::<String>::new());
        assert!(
            matrix
                .cells
                .iter()
                .all(|cell| !cell.enabled && cell.deleted)
        );
    }
}

#[test]
fn fixed_membership_view_rejects_tombstone_mismatches_and_stale_members() {
    fn tombstone_slot(document: &mut ProjectDoc, matrix_id: &str) {
        let key_id = format!("matrix/{matrix_id}/r0c1");
        let matrix = document.matrices.by_id_mut(matrix_id);
        let cell = matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        cell.enabled = false;
        cell.deleted = true;
        cell.assemblies.clear();
        matrix.part_ids.retain(|id| id != &key_id);
    }

    fn fixed_view_error(document: &ProjectDoc) -> String {
        match reopen(&mut document.clone()) {
            Err(error) => error,
            Ok(_) => panic!("expected malformed fixed membership to be refused"),
        }
    }

    let mut document = fixed_geometry_fixture().unwrap();
    tombstone_slot(&mut document, "source-matrix");
    assert!(fixed_view_error(&document).contains("inconsistent inactive slots"));

    let mut document = fixed_geometry_fixture().unwrap();
    tombstone_slot(&mut document, "mirror-matrix");
    assert!(fixed_view_error(&document).contains("inconsistent inactive slots"));

    fn clean_tombstone_fixture() -> ProjectDoc {
        let mut document = fixed_geometry_fixture().unwrap();
        tombstone_slot(&mut document, "source-matrix");
        tombstone_slot(&mut document, "mirror-matrix");
        let source_id = "matrix/source-matrix/r0c1";
        let mirror_id = "matrix/mirror-matrix/r0c1";
        document
            .parts
            .retain(|part| part.id != source_id && part.id != mirror_id);
        for board in &mut document.boards {
            board
                .part_ids
                .retain(|id| id != source_id && id != mirror_id);
        }
        document
            .first_halves_mut()
            .correspondences
            .retain(|association| {
                !(association.kind == MirroredItemKind::Part
                    && association.source_id == source_id
                    && association.mirror_id == mirror_id)
            });

        document
    }

    for stale_kind in ["part", "board owner", "correspondence"] {
        let mut document = clean_tombstone_fixture();
        let source_id = "matrix/source-matrix/r0c1";
        let mirror_id = "matrix/mirror-matrix/r0c1";
        match stale_kind {
            "part" => {
                let mut part = document.parts.by_id("matrix/mirror-matrix/r0c2").clone();
                part.id = mirror_id.into();
                document.parts.push(part);
            }
            "board owner" => document
                .boards
                .by_id_mut("mirror-board")
                .part_ids
                .push(mirror_id.into()),
            "correspondence" => {
                document
                    .first_halves_mut()
                    .correspondences
                    .push(MirroredCorrespondence {
                        companion_mode: None,
                        companion_tweak: None,
                        kind: MirroredItemKind::Part,
                        source_id: source_id.into(),
                        mirror_id: mirror_id.into(),
                    })
            }
            _ => unreachable!(),
        }
        assert!(
            fixed_view_error(&document).contains(if stale_kind == "correspondence" {
                "source part \"matrix/source-matrix/r0c1\" is unavailable"
            } else {
                "materialized or associated members"
            }),
            "stale {stale_kind} was not refused"
        );
    }

    let mut document = fixed_geometry_fixture().unwrap();
    let source_cell = document
        .matrices
        .by_id_mut("source-matrix")
        .cells
        .iter_mut()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    source_cell.enabled = false;
    source_cell.deleted = false;
    assert!(fixed_view_error(&document).contains("fixed active membership"));
}

#[test]
fn fixed_membership_view_only_allows_exactly_omitted_matrix_companions_to_be_absent() {
    let mut document = fixed_membership_assembly_fixture(90.0).unwrap();
    let source_id = "matrix/source-matrix/r0c1/diode";
    let mirror_id = "matrix/mirror-matrix/r0c1/diode";
    let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
    let halves = pair.halves.as_mut().unwrap();
    halves.exceptions.push(MirroredException {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Part,
        mirror_id: mirror_id.into(),
        state: MirroredExceptionState::Omitted,
        removed_copper: None,
    });
    document.parts.retain(|part| part.id != mirror_id);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != mirror_id);
    document
        .matrices
        .by_id_mut("mirror-matrix")
        .part_ids
        .retain(|id| id != mirror_id);

    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)
        .expect("Omitted keeps the exact correspondence while its target is absent");
    let pair_before = serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
    reopen(&mut document).expect("the exact Omitted overlay authenticates the absent child");
    assert!(document.parts.iter().all(|part| part.id != mirror_id));
    let mirror = document.matrices.by_id("mirror-matrix");
    assert!(!mirror.part_ids.iter().any(|id| id == mirror_id));
    assert!(mirror.cells.iter().any(|cell| {
        (cell.row, cell.column) == (0, 1)
            && cell
                .assemblies
                .iter()
                .any(|assembly| assembly.id == "diode")
    }));
    let pair_after = serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap();
    assert_eq!(
        pair_after, pair_before,
        "Omitted retains durable provenance"
    );

    let mut document = fixed_membership_assembly_fixture(90.0).unwrap();
    document.parts.retain(|part| part.id != mirror_id);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .retain(|id| id != mirror_id);
    document
        .matrices
        .by_id_mut("mirror-matrix")
        .part_ids
        .retain(|id| id != mirror_id);

    let error = match reopen(&mut document.clone()) {
        Err(error) => error,
        Ok(_) => panic!("an unrecorded missing assembly must remain a refusal"),
    };
    assert!(
        error.contains(mirror_id) && error.contains("not on board"),
        "{error}"
    );
    assert!(
        document
            .first_halves()
            .correspondences
            .iter()
            .any(|item| item.source_id == source_id && item.mirror_id == mirror_id)
    );
}

#[test]
fn fixed_membership_view_preserves_authored_cell_defaults_and_variants() {
    let mut document = fixed_membership_assembly_fixture(90.0).unwrap();
    let matrix_default = document
        .matrices
        .by_id("source-matrix")
        .definition_id
        .clone();
    let mut alternate_definition = document.definitions[0].clone();
    alternate_definition.id = "alternate-switch".into();
    document.definitions.push(alternate_definition);
    {
        let source_matrix = document.matrices.by_id_mut("source-matrix");
        let cell = source_matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 1))
            .unwrap();
        // The definition override is redundant today but remains an authored per-cell
        // choice if the matrix default changes later.
        cell.definition_id = Some(matrix_default.clone());
        cell.variant = Some("preset/authored-red".into());
        cell.assemblies_local = Some(false);
        source_matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 2))
            .unwrap()
            .definition_id = Some("alternate-switch".into());
    }
    let source_matrix = document.matrices.by_id("source-matrix").clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)
        .expect("the source materializes its explicit and alternate definitions");

    let source_before_reconcile = document.matrices.by_id("source-matrix").clone();
    reopen(&mut document).expect("authored cell definitions materialize through reconciliation");
    assert_eq!(
        document.matrices.by_id("source-matrix"),
        &source_before_reconcile,
        "reconciliation preserves the complete authored source matrix"
    );
    let cell = document
        .matrices
        .by_id("mirror-matrix")
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .unwrap();
    assert_eq!(cell.definition_id.as_deref(), Some(matrix_default.as_str()));
    assert_eq!(cell.variant.as_deref(), Some("preset/authored-red"));
    assert_eq!(cell.assemblies_local, Some(false));
    for (matrix_id, coordinate, expected) in [
        ("source-matrix", (0, 1), matrix_default.as_str()),
        ("mirror-matrix", (0, 1), matrix_default.as_str()),
        ("source-matrix", (0, 2), "alternate-switch"),
        ("mirror-matrix", (0, 2), "alternate-switch"),
    ] {
        let key_id =
            boardstudio_core_kernel::matrix::member_id(matrix_id, coordinate.0, coordinate.1);
        let key = document
            .parts
            .iter()
            .find(|part| part.id == key_id)
            .unwrap_or_else(|| panic!("generated key {key_id} exists"));
        assert_eq!(key.definition_id, expected, "{key_id}");
    }
}

#[test]
fn source_and_mirror_local_assemblies_have_no_correspondence_and_survive_reconcile() {
    let mut document = fixed_membership_assembly_fixture(90.0).unwrap();
    let mut target_definition = document.definitions[0].clone();
    target_definition.id = "target-local-switch".into();
    document.definitions.push(target_definition);
    for matrix_id in ["source-matrix", "mirror-matrix"] {
        let cell = document
            .matrices
            .by_id_mut(matrix_id)
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap();
        cell.assemblies_local = Some(true);
        cell.variant = Some(if matrix_id == "source-matrix" {
            "source-local-variant".into()
        } else {
            "target-local-variant".into()
        });
        if matrix_id == "mirror-matrix" {
            cell.definition_id = Some("target-local-switch".into());
        }
    }
    let mut source_matrix = document.matrices.by_id("source-matrix").clone();
    source_matrix.cells[0].assemblies.push(MatrixAssembly {
        id: "source-only".into(),
        definition_id: "diode".into(),
        offset: Vec2 { x: 9.0, y: 1.0 },
        rotation: Some(0.0),
        side: Some(Side::Back),
    });
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix).unwrap();
    let target_matrix = document.matrices.by_id("mirror-matrix").clone();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &target_matrix).unwrap();
    document
        .first_halves_mut()
        .correspondences
        .retain(|association| {
            association.source_id != "matrix/source-matrix/r0c0/diode"
                && association.mirror_id != "matrix/mirror-matrix/r0c0/diode"
        });
    let mirror_assembly = document.matrices.by_id("mirror-matrix").cells[0].assemblies[0].clone();
    let target_cell = document
        .matrices
        .by_id("mirror-matrix")
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap()
        .clone();
    let source_assemblies = document.matrices.by_id("source-matrix").cells[0]
        .assemblies
        .clone();
    let mirror_part = document
        .parts
        .by_id("matrix/mirror-matrix/r0c0/diode")
        .clone();

    reopen(&mut document).expect("independent local companion lists do not need pair associations");

    assert_eq!(
        document.matrices.by_id("source-matrix").cells[0].assemblies,
        source_assemblies
    );
    let source_cell = document
        .matrices
        .by_id("source-matrix")
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap();
    assert_eq!(source_cell.definition_id, None);
    assert_eq!(source_cell.variant.as_deref(), Some("source-local-variant"));
    let source_key = document.parts.by_id("matrix/source-matrix/r0c0");
    assert_eq!(source_key.definition_id, "switch");
    assert_eq!(
        source_key
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("source-local-variant"))
    );
    assert_eq!(
        document.matrices.by_id("mirror-matrix").cells[0].assemblies[0],
        mirror_assembly
    );
    let mirrored_cell = document
        .matrices
        .by_id("mirror-matrix")
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .unwrap();
    assert_eq!(mirrored_cell.definition_id, target_cell.definition_id);
    assert_eq!(mirrored_cell.variant, target_cell.variant);
    assert_eq!(mirrored_cell.assemblies_local, target_cell.assemblies_local);
    assert_eq!(mirrored_cell.assemblies, target_cell.assemblies);
    assert_eq!(
        mirrored_cell.definition_id.as_deref(),
        Some("target-local-switch")
    );
    assert_eq!(
        mirrored_cell.variant.as_deref(),
        Some("target-local-variant")
    );
    let target_key = document.parts.by_id("matrix/mirror-matrix/r0c0");
    assert_eq!(target_key.definition_id, "target-local-switch");
    assert_eq!(
        target_key
            .properties
            .as_ref()
            .and_then(|properties| properties.get("variant")),
        Some(&serde_json::json!("target-local-variant"))
    );
    assert_eq!(document.parts.by_id(&mirror_part.id), &mirror_part);
    let after_first = document.clone();
    assert!(reopen(&mut document).unwrap().is_empty());
    assert_eq!(document, after_first);
}
