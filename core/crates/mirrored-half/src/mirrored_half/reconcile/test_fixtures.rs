use super::{MatrixLink, PartProjection, Projection};
use boardstudio_core_mechanical::mechanical::{
    MechanicalTarget,
    change::{MechanicalChange, MechanicalEdit, MechanicalFeatureCorrespondence},
};
use boardstudio_model::model::*;
use std::collections::{BTreeMap, BTreeSet};
pub fn fixture_mapping() -> Pose2 {
    Pose2 {
        at: Vec2 { x: 80.0, y: -25.0 },
        rotation: 31.0,
    }
}

pub fn pair_state_bytes(document: &ProjectDoc) -> Vec<u8> {
    serde_json::to_vec(&document.hardware.as_ref().unwrap().pairs[0]).unwrap()
}

pub fn fixture_board(id: &str) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

pub fn fixture_mechanical_configuration(board_id: &str) -> MechanicalConfiguration {
    let mut configuration: MechanicalConfiguration = serde_json::from_value(serde_json::json!({
        "boardId": board_id,
        "method": "printed",
        "mount": "rigid",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4,
        "batteryHeight": 4.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "bottomStyle": "shell",
        "middleFrame": false,
        "openingAllowance": 0.25,
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .unwrap();
    configuration.mounts.push(Mount {
        id: "mount-1".into(),
        at: Vec2 { x: 2.0, y: 3.0 },
        kind: MountKind::Hole,
        hole_diameter: 2.2,
        boss_diameter: None,
        height: None,
    });
    configuration
}

pub fn fixture_instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: "controller".into(),
        flipped: false,
        controller_part_id: None,
        mechanical: Some(fixture_mechanical_configuration(board_id)),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

pub fn fixture_matrix(id: &str, board_id: &str) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows: 1,
        columns: 2,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: Some(board_id.into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: (0..2)
            .map(|column| MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            })
            .collect(),
    }
}

pub fn fixture_mounting_hole() -> PartDefinition {
    serde_json::from_value(serde_json::json!({
        "id":"generator:ceoloide/mounting_hole_npth",
        "name":"mounting hole npth",
        "kind":"custom",
        "pads":[{"id":"pad-0","number":"","at":{"x":0,"y":0},
            "size":{"x":2.2,"y":2.2},"shape":"circle","drill":2.2,"plated":false,"rotation":0}],
        "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
        "envelopeSource":{"courtyard":"generated"},
        "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
        "generator":{"source":"ceoloide/mounting_hole_npth","version":"bundled-1","parameters":{}}
    }))
    .unwrap()
}

pub fn fixture_document() -> Result<ProjectDoc, String> {
    let source_board = "source-board";
    let mirror_board = "mirror-board";
    let mut document = ProjectDoc::empty("reconcile-fixture", "Reconcile fixture");
    document.boards = vec![fixture_board(source_board), fixture_board(mirror_board)];
    document.definitions.push(
        serde_json::from_value(serde_json::json!({
            "id":"switch","name":"Switch","kind":"switch","courtyard":[],"pads":[]
        }))
        .unwrap(),
    );
    document.mechanical = Some(fixture_mechanical_configuration(source_board));
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            fixture_instance("left", source_board),
            fixture_instance("right", mirror_board),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });

    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixture_matrix("source-matrix", source_board),
    )?;
    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixture_matrix("mirror-matrix", mirror_board),
    )?;

    boardstudio_core_mechanical::mechanical::edit::apply(
        &mut document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: source_board.into(),
            },
            change: MechanicalChange::LinkMirrorPair {
                pair_id: "pair-source-mirror".into(),
                members: ["left".into(), "right".into()],
                mapping: fixture_mapping(),
                construction_source: "left".into(),
                correspondences: vec![MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                }],
            },
            mounting_hole: Some(Box::new(fixture_mounting_hole())),
        },
    )?;

    let correspondences = vec![
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: "source-matrix".into(),
            mirror_id: "mirror-matrix".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: "matrix/source-matrix/r0c0".into(),
            mirror_id: "matrix/mirror-matrix/r0c0".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: "matrix/source-matrix/r0c1".into(),
            mirror_id: "matrix/mirror-matrix/r0c1".into(),
        },
    ];
    let halves = serde_json::from_value(serde_json::json!({
        "sourceBoardId": source_board,
        "mirrorBoardId": mirror_board,
        "correspondences": correspondences,
        "exceptions": []
    }))
    .map_err(|error| error.to_string())?;
    document.hardware.as_mut().unwrap().pairs[0].halves = Some(halves);
    Ok(document)
}

pub fn fixed_geometry_matrix(id: &str, board_id: &str, source_geometry: bool) -> Matrix {
    let mut matrix = Matrix {
        id: id.into(),
        name: None,
        rows: 2,
        columns: 3,
        pitch: Vec2 { x: 10.0, y: 20.0 },
        origin: if source_geometry {
            Vec2 { x: 3.0, y: -2.0 }
        } else {
            Vec2::default()
        },
        definition_id: "switch".into(),
        part_ids: vec![],
        board_id: Some(board_id.into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: (0..2)
            .flat_map(|row| {
                (0..3).map(move |column| MatrixCell {
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
                    assemblies: vec![],
                    assemblies_local: None,
                })
            })
            .collect(),
    };
    if source_geometry {
        {
            let values = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
            matrix
                .row_layout
                .resize_with(values.len().max(matrix.row_layout.len()), Default::default);
            for record in &mut matrix.row_layout {
                record.offset = Vec2::default();
            }
            for (record, value) in matrix.row_layout.iter_mut().zip(values) {
                record.offset = value;
            }
        }
        {
            let values = vec![
                Vec2::default(),
                Vec2 { x: 3.0, y: 4.0 },
                Vec2 { x: -2.0, y: 1.0 },
            ];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.offset = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.offset = value;
            }
        }
        {
            let values = vec![1.0, 2.0, -1.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.stagger = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.stagger = value;
            }
        }
        {
            let values = vec![90.0, -90.0, 90.0];
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay = 0.0;
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        {
            let values = (vec![
                Some(Vec2 { x: 2.0, y: 2.0 }),
                None,
                Some(Vec2 { x: -3.0, y: -6.0 }),
            ])
            .into_iter()
            .map(|value| value.unwrap_or_default())
            .collect::<Vec<_>>();
            matrix.column_layout.resize_with(
                values.len().max(matrix.column_layout.len()),
                Default::default,
            );
            for record in &mut matrix.column_layout {
                record.splay_origin = Vec2::default();
            }
            for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                record.splay_origin = value;
            }
        }
        let final_cell = matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == 0 && cell.column == 2)
            .unwrap();
        final_cell.offset = Some(Vec2 { x: 2.0, y: -3.0 });
        final_cell.rotation = Some(90.0);
    }
    matrix
}

pub fn fixed_geometry_fixture() -> Result<(ProjectDoc, Projection), String> {
    let mut document = fixture_document()?;
    let mapping = Pose2 {
        at: Vec2 { x: 100.0, y: -30.0 },
        rotation: 90.0,
    };
    document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixed_geometry_matrix("source-matrix", "source-board", true),
    )?;
    boardstudio_core_kernel::matrix::set_matrix(
        &mut document,
        &fixed_geometry_matrix("mirror-matrix", "mirror-board", false),
    )?;

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences = vec![MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
    }];
    for row in 0..2 {
        for column in 0..3 {
            halves.correspondences.push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: format!("matrix/source-matrix/r{row}c{column}"),
                mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
            });
        }
    }

    let definition = document
        .definitions
        .iter_mut()
        .find(|definition| definition.id == "switch")
        .unwrap();
    definition.pads = vec![
        boardstudio_model::model::Pad {
            id: "pad-one".into(),
            number: "1".into(),
            at: Vec2::default(),
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
        boardstudio_model::model::Pad {
            id: "pad-two".into(),
            number: "2".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
    ];
    let mut probe = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c2")
        .unwrap()
        .clone();
    probe.id = "geometry-probe".into();
    probe.reference = "PROBE".into();
    probe.pose = Pose2 {
        at: Vec2 { x: 4.0, y: 2.0 },
        rotation: 0.0,
    };
    document.parts.push(probe);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("geometry-probe".into());
    document.nets.push(boardstudio_model::model::Net {
        id: "retained-second-key-net".into(),
        name: "RETAINED SECOND KEY".into(),
        pins: vec![
            boardstudio_model::model::Pin {
                part_id: "matrix/mirror-matrix/r1c1".into(),
                pad_id: "pad-one".into(),
            },
            boardstudio_model::model::Pin {
                part_id: "geometry-probe".into(),
                pad_id: "pad-two".into(),
            },
        ],
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .net_ids
        .push("retained-second-key-net".into());
    document
        .constraints
        .push(boardstudio_model::model::Constraint::Offset {
            id: "retained-second-key-offset".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r1c1".into(),
            offset: Vec2 { x: 6.0, y: 18.0 },
            rotation: 0.0,
        });
    document
        .outline
        .push(boardstudio_model::model::OutlineFeature::PartEnvelope {
            connections: vec![],
            settings: Default::default(),
            id: "retained-keys-envelope".into(),
            part_ids: vec![
                "matrix/mirror-matrix/r0c0".into(),
                "matrix/mirror-matrix/r0c1".into(),
                "matrix/mirror-matrix/r0c2".into(),
                "matrix/mirror-matrix/r1c0".into(),
                "matrix/mirror-matrix/r1c1".into(),
                "matrix/mirror-matrix/r1c2".into(),
                "geometry-probe".into(),
            ],
            margin: 2.0,
            operation: boardstudio_model::model::Operation::Add,
        });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("retained-keys-envelope".into());

    let linked_slots = [(0, 1), (0, 2), (1, 0), (1, 2)];
    let parts = linked_slots
        .into_iter()
        .map(|(row, column)| PartProjection {
            source_id: format!("matrix/source-matrix/r{row}c{column}"),
            mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
        })
        .collect();
    let protected = BTreeSet::from([
        "matrix/mirror-matrix/r0c0".to_owned(),
        "matrix/mirror-matrix/r1c1".to_owned(),
    ]);
    let mut final_owners = BTreeMap::new();
    for id in document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
        .flat_map(|matrix| matrix.part_ids.iter())
    {
        let owner = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(id))
            .unwrap();
        final_owners.insert(id.clone(), owner.id.clone());
    }
    let projection = Projection {
        matrix: Some(MatrixLink {
            source_matrix_id: "source-matrix".into(),
            mirror_matrix_id: "mirror-matrix".into(),
        }),
        preserve_matrix_geometry: false,
        source_board_id: "source-board".into(),
        mirror_board_id: "mirror-board".into(),
        mapping,
        parts,
        independent_part_transitions: vec![],
        hosted_transitions: vec![],
        applied_independent_additions: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_operations: std::cell::RefCell::new(BTreeSet::new()),
        applied_hosted_circuit_additions: std::cell::RefCell::new(BTreeMap::new()),
        hosted_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        hosted_omission_baselines: std::cell::RefCell::new(BTreeMap::new()),
        outline_removal_baselines: std::cell::RefCell::new(BTreeMap::new()),
        locally_protected_mirror_parts: protected,
        final_owners,
    };
    Ok((document, projection))
}

pub fn fixed_membership_assembly_fixture(
    mapping_rotation: f64,
) -> Result<(ProjectDoc, Projection), String> {
    let (mut document, mut projection) = fixed_geometry_fixture()?;
    let mut diode = document.definitions[0].clone();
    diode.id = "diode".into();
    diode.name = "Diode".into();
    diode.pads = vec![
        boardstudio_model::model::Pad {
            id: "pad-one".into(),
            number: "1".into(),
            at: Vec2::default(),
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
        boardstudio_model::model::Pad {
            id: "pad-two".into(),
            number: "2".into(),
            at: Vec2 { x: 1.0, y: 0.0 },
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: boardstudio_model::model::PadShape::Circle,
            drill: None,
            plated: None,
            side: None,
            rotation: None,
            net_id: None,
        },
    ];
    let mut stabiliser = diode.clone();
    stabiliser.id = "kicad:marbastlib/STAB_MX_2u".into();
    stabiliser.name = "Stabiliser".into();
    document.definitions.extend([diode, stabiliser]);

    let make_assemblies = |matrix: &mut Matrix| {
        for cell in &mut matrix.cells {
            cell.assemblies = vec![MatrixAssembly {
                id: "diode".into(),
                definition_id: "diode".into(),
                offset: Vec2 { x: 2.0, y: 3.0 },
                rotation: Some(30.0),
                side: Some(Side::Back),
            }];
            if (cell.row, cell.column) == (0, 1) {
                cell.assemblies.push(MatrixAssembly {
                    id: "fallback".into(),
                    definition_id: "diode".into(),
                    offset: Vec2 { x: 5.0, y: -2.0 },
                    rotation: Some(45.0),
                    side: Some(Side::Back),
                });
            }
            if (cell.row, cell.column) == (1, 2) {
                cell.assemblies.push(MatrixAssembly {
                    id: "stab".into(),
                    definition_id: "kicad:marbastlib/STAB_MX_2u".into(),
                    offset: Vec2 { x: 4.0, y: 0.0 },
                    rotation: Some(0.0),
                    side: Some(Side::Front),
                });
            }
        }
    };
    for (id, board_id) in [
        ("source-matrix", "source-board"),
        ("mirror-matrix", "mirror-board"),
    ] {
        let mut matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == id)
            .unwrap()
            .clone();
        make_assemblies(&mut matrix);
        if id == "source-matrix" {
            matrix.origin = Vec2 { x: 3.0, y: -2.0 };
            {
                let values = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
                matrix
                    .row_layout
                    .resize_with(values.len().max(matrix.row_layout.len()), Default::default);
                for record in &mut matrix.row_layout {
                    record.offset = Vec2::default();
                }
                for (record, value) in matrix.row_layout.iter_mut().zip(values) {
                    record.offset = value;
                }
            }
            {
                let values = vec![
                    Vec2::default(),
                    Vec2 { x: 3.0, y: 4.0 },
                    Vec2 { x: -2.0, y: 1.0 },
                ];
                matrix.column_layout.resize_with(
                    values.len().max(matrix.column_layout.len()),
                    Default::default,
                );
                for record in &mut matrix.column_layout {
                    record.offset = Vec2::default();
                }
                for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                    record.offset = value;
                }
            }
            {
                let values = vec![1.0, 2.0, -1.0];
                matrix.column_layout.resize_with(
                    values.len().max(matrix.column_layout.len()),
                    Default::default,
                );
                for record in &mut matrix.column_layout {
                    record.stagger = 0.0;
                }
                for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                    record.stagger = value;
                }
            }
            {
                let values = vec![90.0, -90.0, 90.0];
                matrix.column_layout.resize_with(
                    values.len().max(matrix.column_layout.len()),
                    Default::default,
                );
                for record in &mut matrix.column_layout {
                    record.splay = 0.0;
                }
                for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                    record.splay = value;
                }
            }
            {
                let values = (vec![
                    Some(Vec2 { x: 2.0, y: 2.0 }),
                    None,
                    Some(Vec2 { x: -3.0, y: -6.0 }),
                ])
                .into_iter()
                .map(|value| value.unwrap_or_default())
                .collect::<Vec<_>>();
                matrix.column_layout.resize_with(
                    values.len().max(matrix.column_layout.len()),
                    Default::default,
                );
                for record in &mut matrix.column_layout {
                    record.splay_origin = Vec2::default();
                }
                for (record, value) in matrix.column_layout.iter_mut().zip(values) {
                    record.splay_origin = value;
                }
            }
            let final_cell = matrix
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == (0, 2))
                .unwrap();
            final_cell.offset = Some(Vec2 { x: 2.0, y: -3.0 });
            final_cell.rotation = Some(90.0);
        }
        boardstudio_core_kernel::matrix::set_matrix(&mut document, &matrix)?;
        debug_assert_eq!(matrix.board_id.as_deref(), Some(board_id));
    }

    let pair = &mut document.hardware.as_mut().unwrap().pairs[0];
    pair.mapping = Pose2 {
        at: Vec2 { x: 100.0, y: -30.0 },
        rotation: mapping_rotation,
    };
    let halves = pair.halves.as_mut().unwrap();
    halves.companion_mode = MirroredCompanionMode::DuplicatePosition;
    halves.correspondences = vec![MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
    }];
    for row in 0..2 {
        for column in 0..3 {
            let key_source = format!("matrix/source-matrix/r{row}c{column}");
            let key_mirror = format!("matrix/mirror-matrix/r{row}c{column}");
            halves.correspondences.push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: key_source.clone(),
                mirror_id: key_mirror.clone(),
            });
            let mut companion_ids = vec!["diode"];
            if (row, column) == (0, 1) {
                companion_ids.push("fallback");
            }
            if (row, column) == (1, 2) {
                companion_ids.push("stab");
            }
            for assembly_id in companion_ids {
                let mode = match (row, column, assembly_id) {
                    (0, 1, "diode") | (1, 2, "diode") => Some(MirroredCompanionMode::LiteralMirror),
                    (1, 0, "diode") => Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                    (0, 1, "fallback") => Some(MirroredCompanionMode::LiteralMirror),
                    _ => None,
                };
                let tweak = ((row, column, assembly_id) == (1, 0, "diode")).then_some(
                    MirroredCompanionTweak {
                        x: 1.0,
                        y: -2.0,
                        rotation: 15.0,
                    },
                );
                halves.correspondences.push(MirroredCorrespondence {
                    companion_mode: mode,
                    companion_tweak: tweak,
                    kind: MirroredItemKind::Part,
                    source_id: format!("{key_source}/{assembly_id}"),
                    mirror_id: format!("{key_mirror}/{assembly_id}"),
                });
            }
        }
    }
    if mapping_rotation == 0.0 {
        projection.mapping.rotation = 0.0;
    } else {
        projection.mapping.rotation = mapping_rotation;
    }
    projection.parts = [(0, 1), (0, 2), (1, 0), (1, 2)]
        .into_iter()
        .map(|(row, column)| PartProjection {
            source_id: format!("matrix/source-matrix/r{row}c{column}"),
            mirror_id: format!("matrix/mirror-matrix/r{row}c{column}"),
        })
        .collect();
    projection.final_owners.clear();
    for id in document
        .matrices
        .iter()
        .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
        .flat_map(|matrix| matrix.part_ids.iter())
    {
        let owner = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(id))
            .unwrap();
        projection.final_owners.insert(id.clone(), owner.id.clone());
    }

    let mut probe = document
        .parts
        .iter()
        .find(|part| part.id == "matrix/mirror-matrix/r0c2")
        .unwrap()
        .clone();
    probe.id = "assembly-probe".into();
    probe.reference = "ASSEMBLY-PROBE".into();
    document.parts.push(probe);
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .part_ids
        .push("assembly-probe".into());
    for (index, retained_id) in [
        "matrix/mirror-matrix/r0c0/diode",
        "matrix/mirror-matrix/r1c1/diode",
    ]
    .into_iter()
    .enumerate()
    {
        let net_id = format!("retained-diode-net-{index}");
        document.nets.push(Net {
            id: net_id.clone(),
            name: net_id.clone(),
            pins: vec![
                Pin {
                    part_id: retained_id.into(),
                    pad_id: "pad-one".into(),
                },
                Pin {
                    part_id: "assembly-probe".into(),
                    pad_id: "pad-two".into(),
                },
            ],
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .net_ids
            .push(net_id);
        document.constraints.push(Constraint::Offset {
            id: format!("retained-diode-constraint-{index}"),
            source_part_id: "assembly-probe".into(),
            target_part_id: retained_id.into(),
            offset: Vec2 { x: 2.0, y: -1.0 },
            rotation: 0.0,
        });
    }
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: Default::default(),
        id: "retained-diodes-envelope".into(),
        part_ids: vec![
            "matrix/mirror-matrix/r0c0/diode".into(),
            "matrix/mirror-matrix/r0c0".into(),
            "matrix/mirror-matrix/r0c1".into(),
            "matrix/mirror-matrix/r0c1/diode".into(),
            "matrix/mirror-matrix/r0c1/fallback".into(),
            "matrix/mirror-matrix/r0c2".into(),
            "matrix/mirror-matrix/r0c2/diode".into(),
            "matrix/mirror-matrix/r1c0".into(),
            "matrix/mirror-matrix/r1c0/diode".into(),
            "matrix/mirror-matrix/r1c1".into(),
            "matrix/mirror-matrix/r1c1/diode".into(),
            "matrix/mirror-matrix/r1c2".into(),
            "matrix/mirror-matrix/r1c2/diode".into(),
            "matrix/mirror-matrix/r1c2/stab".into(),
            "assembly-probe".into(),
        ],
        margin: 2.0,
        operation: Operation::Add,
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("retained-diodes-envelope".into());
    for part in document
        .parts
        .iter_mut()
        .filter(|part| projection.final_owners.contains_key(&part.id))
    {
        part.locked = Some(true);
        part.properties = Some(BTreeMap::from([(
            "custom".into(),
            serde_json::json!({"part": part.id}),
        )]));
        part.generator_parameters = Some(BTreeMap::from([(
            "custom".into(),
            serde_json::json!({"part": part.id}),
        )]));
    }
    Ok((document, projection))
}
