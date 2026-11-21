use super::*;

pub(crate) fn local_transition_fixture() -> Result<ProjectDoc, String> {
    let mut document = fixture_document()?;
    let mut diode = document.definitions[0].clone();
    diode.id = "diode".into();
    diode.name = "Diode".into();
    document.definitions.push(diode);

    let matrix_cells = || {
        (0..2)
            .flat_map(|row| {
                (0..4).map(move |column| MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row,
                    column,
                    enabled: true,
                    definition_id: None,
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![MatrixAssembly {
                        id: "diode".into(),
                        definition_id: "diode".into(),
                        offset: Vec2 { x: 2.0, y: 3.0 },
                        rotation: Some(30.0),
                        side: Some(Side::Back),
                    }],
                    assemblies_local: None,
                })
            })
            .collect::<Vec<_>>()
    };
    let mut source_matrix = document.matrices.by_id("source-matrix").clone();
    source_matrix.rows = 2;
    source_matrix.columns = 4;
    source_matrix.pitch = Vec2 { x: 10.0, y: 20.0 };
    source_matrix.cells = matrix_cells();
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &source_matrix)?;

    let mut mirror_matrix = document.matrices.by_id("mirror-matrix").clone();
    mirror_matrix.rows = 2;
    mirror_matrix.columns = 4;
    mirror_matrix.pitch = source_matrix.pitch;
    mirror_matrix.cells = matrix_cells();
    let matrix_correspondence = document
        .first_halves()
        .correspondences
        .iter()
        .find(|record| record.kind == MirroredItemKind::Matrix)
        .unwrap()
        .clone();
    mirror_matrix = boardstudio_core_mirrored_half::mirrored_half::derive::project_matrix(
        &source_matrix,
        &mirror_matrix,
        &matrix_correspondence,
        document.hardware.as_ref().unwrap().pairs[0].mapping,
    )?;
    boardstudio_core_kernel::matrix::set_matrix(&mut document, &mirror_matrix)?;

    let halves = document.first_halves_mut();
    halves.correspondences.clear();
    halves.exceptions.clear();
    halves.correspondences.push(MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
        companion_mode: None,
        companion_tweak: None,
    });
    for row in 0..2 {
        for column in 0..4 {
            let source = format!("matrix/source-matrix/r{row}c{column}");
            let mirror = format!("matrix/mirror-matrix/r{row}c{column}");
            halves.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Part,
                source_id: source.clone(),
                mirror_id: mirror.clone(),
                companion_mode: None,
                companion_tweak: None,
            });
            halves.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Part,
                source_id: format!("{source}/diode"),
                mirror_id: format!("{mirror}/diode"),
                companion_mode: None,
                companion_tweak: None,
            });
        }
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document)?;

    Ok(document)
}

pub(crate) fn independent_lifecycle_fixture() -> ProjectDoc {
    let mut document = local_transition_fixture().unwrap();
    for name in ["screen", "encoder"] {
        for (half, board_id, x) in [
            ("source", "source-board", 8.0),
            ("mirror", "mirror-board", 90.0),
        ] {
            let id = format!("{half}-{name}");
            let mut part = document.parts[0].clone();
            part.id = id.clone();
            part.reference = format!("{half}_{name}");
            part.pose.at = Vec2 { x, y: 42.0 };
            part.properties = Some(BTreeMap::from([("revision".into(), json!(1))]));
            document.parts.push(part);
            document
                .boards
                .by_id_mut(board_id)
                .part_ids
                .push(id.clone());
        }
        add_link(
            &mut document,
            MirroredItemKind::Part,
            &format!("source-{name}"),
            &format!("mirror-{name}"),
        );
    }
    document
}
