use super::*;

#[test]
fn source_layout_removal_prunes_linked_projection_and_keeps_local_and_unpaired_members() {
    let mut document = fixed_geometry_fixture().unwrap();
    let source_linked = "layout-linked-source";
    let mirror_linked = "layout-linked-mirror";
    let source_local = "layout-local-source";
    let mirror_local = "layout-local-mirror";
    let mut linked_source_part = document.parts.by_id("matrix/source-matrix/r0c1").clone();
    linked_source_part.id = source_linked.into();
    linked_source_part.reference = "LINKED-SOURCE".into();
    let mut linked_mirror_part = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    linked_mirror_part.id = mirror_linked.into();
    linked_mirror_part.reference = "LINKED-MIRROR".into();
    let mut local_source_part = linked_source_part.clone();
    local_source_part.id = source_local.into();
    local_source_part.reference = "LOCAL-SOURCE".into();
    let mut local_mirror_part = local_source_part.clone();
    local_mirror_part.id = mirror_local.into();
    local_mirror_part.reference = "LOCAL-MIRROR".into();
    document.parts.extend([
        linked_source_part,
        linked_mirror_part,
        local_source_part,
        local_mirror_part,
    ]);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .extend([source_linked.into(), source_local.into()]);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .extend([mirror_linked.into(), mirror_local.into()]);

    let halves = document.first_halves_mut();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_linked.into(),
            mirror_id: mirror_linked.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_local.into(),
            mirror_id: mirror_local.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: "source-layout-removal".into(),
            mirror_id: "mirror-layout-removal".into(),
        },
    ]);
    halves.exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_local, "state": "local"
        }))
        .unwrap(),
    );

    let source_layout = Layout {
        id: "source-layout-removal".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_linked.into(), source_local.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-removal".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_linked.into(),
            mirror_local.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout];
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();

    let request = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![mirror_local.into(), "geometry-probe".into()],
            ..document.layouts[1].clone()
        },
    };
    let canonical = crate::support::canonicalize(&document, &request)
        .expect("the mirror request maps into source membership");
    let expected_source = boardstudio_model::model::EditOperation::SetLayout {
        layout: Layout {
            part_ids: vec![source_local.into()],
            ..source_layout
        },
    };
    assert_eq!(canonical.operation, expected_source);
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the translated source layout removes its linked member");
    reopen(&mut document).expect("the removal and retained target members reconcile");

    assert_eq!(
        document.layouts[1].part_ids,
        vec![mirror_local.to_owned(), "geometry-probe".to_owned()]
    );
}

#[test]
fn mirror_layout_reorder_survives_source_apply_and_reconciliation() {
    let mut document = fixed_geometry_fixture().unwrap();
    let source_header = "layout-header-source";
    let source_u1 = "layout-u1-source";
    let mirror_header = "layout-header-mirror";
    let mirror_u1 = "layout-u1-mirror";
    let mut source_header_part = document.parts.by_id("matrix/source-matrix/r0c0").clone();
    source_header_part.id = source_header.into();
    source_header_part.reference = "LAYOUT-HEADER-SOURCE".into();
    let mut source_u1_part = document.parts.by_id("matrix/source-matrix/r0c1").clone();
    source_u1_part.id = source_u1.into();
    source_u1_part.reference = "LAYOUT-U1-SOURCE".into();
    let mut mirror_header_part = document.parts.by_id("matrix/mirror-matrix/r0c0").clone();
    mirror_header_part.id = mirror_header.into();
    mirror_header_part.reference = "LAYOUT-HEADER-MIRROR".into();
    let mut mirror_u1_part = document.parts.by_id("matrix/mirror-matrix/r0c1").clone();
    mirror_u1_part.id = mirror_u1.into();
    mirror_u1_part.reference = "LAYOUT-U1-MIRROR".into();
    document.parts.extend([
        source_header_part,
        source_u1_part,
        mirror_header_part,
        mirror_u1_part,
    ]);
    document
        .boards
        .by_id_mut("source-board")
        .part_ids
        .extend([source_header.into(), source_u1.into()]);
    document
        .boards
        .by_id_mut("mirror-board")
        .part_ids
        .extend([mirror_header.into(), mirror_u1.into()]);
    document.first_halves_mut().correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_header.into(),
            mirror_id: mirror_header.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_u1.into(),
            mirror_id: mirror_u1.into(),
        },
    ]);
    document.first_halves_mut().exceptions.push(
        serde_json::from_value(serde_json::json!({
            "kind": "part", "mirrorId": mirror_u1, "state": "local"
        }))
        .unwrap(),
    );
    let source_layout = Layout {
        id: "source-layout-reorder".into(),
        name: "Source layout".into(),
        board_id: "source-board".into(),
        matrix_ids: vec!["source-matrix".into()],
        part_ids: vec![source_header.into(), source_u1.into()],
        mirror_link: None,
    };
    let mirror_layout = Layout {
        id: "mirror-layout-reorder".into(),
        name: "Mirror layout".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec![
            mirror_header.into(),
            mirror_u1.into(),
            "geometry-probe".into(),
        ],
        mirror_link: None,
    };
    document.layouts = vec![source_layout.clone(), mirror_layout.clone()];
    document
        .first_halves_mut()
        .correspondences
        .push(MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Layout,
            source_id: source_layout.id.clone(),
            mirror_id: mirror_layout.id.clone(),
        });

    let requested = Layout {
        part_ids: vec![
            mirror_u1.into(),
            mirror_header.into(),
            "geometry-probe".into(),
        ],
        ..mirror_layout
    };
    let canonical = crate::support::canonicalize(
        &document,
        &EditOperation::SetLayout {
            layout: requested.clone(),
        },
    )
    .expect("the reordered mirror request maps to source member order");
    assert_eq!(
        canonical.operation,
        EditOperation::SetLayout {
            layout: Layout {
                part_ids: vec![source_u1.into(), source_header.into()],
                ..source_layout
            },
        }
    );
    crate::support::apply_for_test(&mut document, &[], &canonical.operation)
        .expect("the canonical source layout applies");
    reopen(&mut document).expect("the reordered source layout reconciles");

    assert_eq!(document.layouts[1].part_ids, requested.part_ids);
}
