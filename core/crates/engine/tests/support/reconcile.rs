use crate::base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::derive::project_matrix;
use boardstudio_model::model::*;
use std::collections::BTreeMap;
pub fn reopen(document: &mut ProjectDoc) -> Result<Vec<String>, String> {
    let (draft, findings) = crate::support::prepare_open(document, &[])?;
    let changed = findings
        .into_iter()
        .flat_map(|finding| finding.target_ids)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    *document = draft;
    Ok(changed)
}

pub fn hosted_electrical_fixture() -> Result<ProjectDoc, String> {
    let mut document = fixed_geometry_fixture()?;
    document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .retain(|overlay| overlay.mirror_id != "matrix/mirror-matrix/r0c0");
    let source_connector = "matrix/source-matrix/r0c0";
    let mirror_connector = "matrix/mirror-matrix/r0c0";
    let source_member = "embedded/source-circuit/U1";
    let mirror_member = "embedded/mirror-circuit/U1";
    let source_definition = "embedded/source-circuit/definition/u";
    let mirror_definition = "embedded/mirror-circuit/definition/u";
    let source_private_net = "embedded/source-circuit/N$1";
    let mirror_private_net = "embedded/mirror-circuit/N$1";
    let source_joined_net = "host-source-net";
    let mirror_joined_net = "host-mirror-net";

    for (id, reference, board_id) in [
        (source_member, "U1", "source-board"),
        (mirror_member, "U1", "mirror-board"),
    ] {
        let mut part: Part = serde_json::from_value(serde_json::json!({
            "id": id,
            "definitionId": if id == source_member { source_definition } else { mirror_definition },
            "reference": reference,
            "pose": { "at": { "x": 1.0, "y": 2.0 }, "rotation": 0.0 },
            "side": "front",
            "properties": { "fixturePayload": { "preserve": true } },
            "generatorParameters": { "fixtureMode": "literal" }
        }))
        .map_err(|error| error.to_string())?;
        part.pose.at = if id == source_member {
            Vec2 { x: 7.0, y: 9.0 }
        } else {
            Vec2 { x: -7.0, y: 9.0 }
        };
        document.parts.push(part);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .part_ids
            .push(id.into());
    }
    document.definitions.extend([
        serde_json::from_value(serde_json::json!({
            "id": source_definition, "name": "Source U", "kind": "custom",
            "courtyard": [],
            "pads": [{"id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
                "size":{"x":1.0,"y":1.0},"shape":"circle"}]
        }))
        .map_err(|error| error.to_string())?,
        serde_json::from_value(serde_json::json!({
            "id": mirror_definition, "name": "Mirror U", "kind": "custom",
            "courtyard": [],
            "pads": [{"id":"pad-1","number":"1","at":{"x":0.0,"y":0.0},
                "size":{"x":1.0,"y":1.0},"shape":"circle"}]
        }))
        .map_err(|error| error.to_string())?,
    ]);
    document.module_definitions.push(
        serde_json::from_value(serde_json::json!({
            "id": "module-definition",
            "name": "Fixture module",
            "family": "fixture",
            "variant": "source",
            "source": {
                "repository": "https://example.invalid/fixture",
                "revision": "fixture-revision",
                "path": "module.kicad_pcb",
                "license": "MIT"
            },
            "board": { "contours": [] },
            "mounts": [], "volumes": [], "openings": [], "models": [], "gates": [],
            "interfaces": [],
            "electrical": { "protocol": "pass-through", "requiredSignals": [], "logicVoltage": 3.3 },
            "constituents": []
        }))
        .map_err(|error| error.to_string())?,
    );
    for (id, definition_id, board_id, connector_id, at_x) in [
        (
            "source-module",
            "module-definition",
            "source-board",
            source_connector,
            4.0,
        ),
        (
            "mirror-module",
            "module-definition",
            "mirror-board",
            mirror_connector,
            -4.0,
        ),
    ] {
        document.modules.push(
            serde_json::from_value(serde_json::json!({
                "id": id,
                "definitionId": definition_id,
                "hostBoardId": board_id,
                "hostFace": "front",
                "facingFace": "back",
                "at": { "x": at_x, "y": 3.0 },
                "rotation": 12.0,
                "gap": 0.4,
                "attachment": "board",
                "connection": {
                    "hostConnectorPartId": connector_id,
                    "modulePortId": "host",
                    "busId": if id == "source-module" { "source-bus" } else { "mirror-bus" },
                    "assignments": {},
                    "cableType": "fixture-cable",
                    "railVoltages": {}
                }
            }))
            .map_err(|error| error.to_string())?,
        );
    }
    document.embedded_circuits = vec![
        EmbeddedCircuit {
            id: "source-circuit".into(),
            definition_id: "module-definition".into(),
            host_board_id: "source-board".into(),
            part_ids: vec![source_member.into()],
            net_ids: vec![source_private_net.into(), source_joined_net.into()],
            ports: BTreeMap::from([("signal".into(), source_joined_net.into())]),
        },
        EmbeddedCircuit {
            id: "mirror-circuit".into(),
            definition_id: "module-definition".into(),
            host_board_id: "mirror-board".into(),
            part_ids: vec![mirror_member.into()],
            net_ids: vec![mirror_private_net.into(), mirror_joined_net.into()],
            ports: BTreeMap::from([("signal".into(), mirror_joined_net.into())]),
        },
    ];
    for (id, name, pin_id, host_id, board_id) in [
        (
            source_private_net,
            "PRIVATE",
            source_member,
            None,
            "source-board",
        ),
        (
            mirror_private_net,
            "PRIVATE MIRROR",
            mirror_member,
            None,
            "mirror-board",
        ),
        (
            source_joined_net,
            "HOST",
            source_member,
            Some(source_connector),
            "source-board",
        ),
        (
            mirror_joined_net,
            "HOST MIRROR",
            mirror_member,
            Some(mirror_connector),
            "mirror-board",
        ),
    ] {
        let mut pins = vec![Pin {
            part_id: pin_id.into(),
            pad_id: "pad-1".into(),
        }];
        if let Some(host_id) = host_id {
            pins.push(Pin {
                part_id: host_id.into(),
                pad_id: "pad-two".into(),
            });
        }
        document.nets.push(Net {
            id: id.into(),
            name: name.into(),
            pins,
        });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == board_id)
            .unwrap()
            .net_ids
            .push(id.into());
    }
    for (board_id, controller_part_id, key_binding) in [
        ("source-board", source_member, "source-only-binding"),
        ("mirror-board", mirror_member, "mirror-local-binding"),
    ] {
        let mut electrical = ElectricalBoardConfiguration {
            board_id: board_id.into(),
            controller_part_id: Some(controller_part_id.into()),
            mode: boardstudio_core_mechanical::electrical::ElectricalMode::Direct,
            ..ElectricalBoardConfiguration::default()
        };
        electrical.locks.insert(
            format!("direct/{controller_part_id}"),
            if board_id == "source-board" {
                "embedded/source-circuit/N$1".into()
            } else {
                "mirror-local-lock".into()
            },
        );
        electrical.assignments.insert(
            format!("peripheral/{controller_part_id}/signal"),
            if board_id == "source-board" {
                "host-source-net".into()
            } else {
                "mirror-local-assignment".into()
            },
        );
        electrical
            .key_bindings
            .insert("local-function".into(), key_binding.into());
        document.hardware.as_mut().unwrap().boards.push(electrical);
    }
    document.board_references.push(BoardReference {
        id: "source-board-reference".into(),
        board_id: "source-board".into(),
        asset_id: "source-only-model".into(),
        enabled: true,
        pose: Pose2 {
            at: Vec2 { x: 3.0, y: 4.0 },
            rotation: 20.0,
        },
        elevation: 1.0,
        model_assets: BTreeMap::new(),
    });
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "source-board")
        .unwrap()
        .traces
        .push(CopperTrace {
            id: "source-only-copper".into(),
            start: Vec2 { x: 1.0, y: 0.0 },
            end: Vec2 { x: 2.0, y: 0.0 },
            width: 0.25,
            layer: Side::Front,
            net_id: Some("host-source-net".into()),
        });

    let halves = document.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.correspondences.extend([
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source_member.into(),
            mirror_id: mirror_member.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Module,
            source_id: "source-module".into(),
            mirror_id: "mirror-module".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::EmbeddedCircuit,
            source_id: "source-circuit".into(),
            mirror_id: "mirror-circuit".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::ElectricalBoard,
            source_id: "source-board".into(),
            mirror_id: "mirror-board".into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            source_id: source_private_net.into(),
            mirror_id: mirror_private_net.into(),
        },
        MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Net,
            source_id: source_joined_net.into(),
            mirror_id: mirror_joined_net.into(),
        },
    ]);
    Ok(document)
}

pub fn compaction_document() -> Result<ProjectDoc, String> {
    let mut before = fixed_geometry_fixture()?;
    before.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap()
        .exceptions
        .clear();
    let mut diode = before.definitions[0].clone();
    diode.id = "diode".into();
    diode.name = "Diode".into();
    before.definitions.push(diode);
    let matrix_association = before.hardware.as_ref().unwrap().pairs[0]
        .halves
        .as_ref()
        .unwrap()
        .correspondences
        .iter()
        .find(|association| association.kind == MirroredItemKind::Matrix)
        .unwrap()
        .clone();
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
    let mut source_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap()
        .clone();
    source_matrix.rows = 2;
    source_matrix.columns = 4;
    source_matrix.pitch = Vec2 { x: 10.0, y: 20.0 };
    source_matrix.origin = Vec2 { x: 5.0, y: 1.0 };
    source_matrix
        .row_layout
        .iter_mut()
        .for_each(|record| record.offset = Vec2::default());
    source_matrix
        .column_layout
        .iter_mut()
        .for_each(|record| record.offset = Vec2::default());
    source_matrix
        .column_layout
        .iter_mut()
        .for_each(|record| record.stagger = 0.0);
    source_matrix
        .column_layout
        .iter_mut()
        .for_each(|record| record.splay = 0.0);
    source_matrix
        .column_layout
        .iter_mut()
        .for_each(|record| record.splay_origin = Vec2::default());
    source_matrix.cells = matrix_cells();
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &source_matrix)?;
    let mut mirror_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .clone();
    mirror_matrix.rows = 2;
    mirror_matrix.columns = 4;
    mirror_matrix.cells = matrix_cells();
    let mapping = before.hardware.as_ref().unwrap().pairs[0].mapping;
    let mut mirror_matrix =
        project_matrix(&source_matrix, &mirror_matrix, &matrix_association, mapping)?;
    // Keep the materialized baseline consistent with the per-association mode data below:
    // B duplicates the source placement; D mirrors it and then applies its tweak.
    for cell in &mut mirror_matrix.cells {
        let Some(diode) = cell
            .assemblies
            .iter_mut()
            .find(|assembly| assembly.id == "diode")
        else {
            continue;
        };
        match cell.column {
            1 => {
                diode.offset = Vec2 { x: 2.0, y: 3.0 };
                diode.rotation = Some(30.0);
            }
            3 => {
                diode.offset = Vec2 { x: -1.0, y: 1.0 };
                diode.rotation = Some(-15.0);
            }
            _ => {}
        }
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut before, &mirror_matrix)?;
    let halves = before.hardware.as_mut().unwrap().pairs[0]
        .halves
        .as_mut()
        .unwrap();
    halves.companion_mode = MirroredCompanionMode::LiteralMirror;
    halves.correspondences = vec![MirroredCorrespondence {
        companion_mode: None,
        companion_tweak: None,
        kind: MirroredItemKind::Matrix,
        source_id: "source-matrix".into(),
        mirror_id: "mirror-matrix".into(),
    }];
    for row in 0..2 {
        for column in 0..4 {
            let source_key = format!("matrix/source-matrix/r{row}c{column}");
            let mirror_key = format!("matrix/mirror-matrix/r{row}c{column}");
            halves.correspondences.push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Part,
                source_id: source_key.clone(),
                mirror_id: mirror_key.clone(),
            });
            let (mode, tweak) = if column == 1 {
                (Some(MirroredCompanionMode::DuplicatePosition), None)
            } else if column == 2 {
                (Some(MirroredCompanionMode::LiteralMirror), None)
            } else if column == 3 {
                (
                    Some(MirroredCompanionMode::LiteralMirrorWithTweak),
                    Some(MirroredCompanionTweak {
                        x: 1.0,
                        y: -2.0,
                        rotation: 15.0,
                    }),
                )
            } else {
                (None, None)
            };
            halves.correspondences.push(MirroredCorrespondence {
                companion_mode: mode,
                companion_tweak: tweak,
                kind: MirroredItemKind::Part,
                source_id: format!("{source_key}/diode"),
                mirror_id: format!("{mirror_key}/diode"),
            });
        }
    }
    for part in &mut before.parts {
        part.properties = Some(BTreeMap::from([(
            "logical-payload".into(),
            serde_json::json!(part.id.clone()),
        )]));
        part.generator_parameters = Some(BTreeMap::from([(
            "logical-payload".into(),
            serde_json::json!(part.id.clone()),
        )]));
        part.locked = Some(true);
        part.keycap = Some(Vec2 { x: 18.0, y: 17.0 });
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(&before)?;

    let Constraint::Offset {
        offset, rotation, ..
    } = before
        .constraints
        .iter_mut()
        .find(|c| c.id() == "retained-second-key-offset")
        .unwrap()
    else {
        unreachable!()
    };
    *offset = Vec2 { x: 75.0, y: -47.0 };
    *rotation = 90.0;
    Ok(before)
}

pub fn add_membership_cleanup_sentinels(document: &mut ProjectDoc) {
    let removed = "matrix/mirror-matrix/r0c1/diode";
    let shifted = "matrix/mirror-matrix/r0c2/diode";
    let mut keymap = KeymapConfiguration::default();
    keymap.layers[0].bindings.insert(
        removed.into(),
        KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    );
    keymap.layers[0].bindings.insert(
        shifted.into(),
        KeyBinding::KeyPress {
            keycode: "B".into(),
        },
    );
    keymap.layers[0].sensors.insert(
        removed.into(),
        EncoderBinding {
            clockwise: KeyBinding::KeyPress {
                keycode: "C".into(),
            },
            counterclockwise: KeyBinding::KeyPress {
                keycode: "D".into(),
            },
        },
    );
    keymap.layers[0].sensors.insert(
        shifted.into(),
        EncoderBinding {
            clockwise: KeyBinding::KeyPress {
                keycode: "E".into(),
            },
            counterclockwise: KeyBinding::KeyPress {
                keycode: "F".into(),
            },
        },
    );
    document.keymap = Some(keymap);
    let mut keycaps = KeycapConfiguration::default();
    keycaps.keys.insert(
        removed.into(),
        KeycapKeySettings {
            legend: Some("removed".into()),
            ..KeycapKeySettings::default()
        },
    );
    keycaps.keys.insert(
        shifted.into(),
        KeycapKeySettings {
            legend: Some("shifted sentinel".into()),
            ..KeycapKeySettings::default()
        },
    );
    document.keycaps = Some(keycaps);
    let hardware = document.hardware.as_mut().unwrap();
    hardware.instances[1].controller_part_id = Some(removed.into());
    let mut electrical = ElectricalBoardConfiguration {
        board_id: "mirror-board".into(),
        ..ElectricalBoardConfiguration::default()
    };
    electrical
        .locks
        .insert(format!("direct/{removed}"), "lock sentinel".into());
    electrical
        .assignments
        .insert(format!("link/{shifted}"), "assignment sentinel".into());
    electrical
        .key_bindings
        .insert(shifted.into(), "binding sentinel".into());
    hardware.boards.push(electrical);
    document.embedded_circuits.push(EmbeddedCircuit {
        id: "closure-circuit".into(),
        definition_id: "closure-definition".into(),
        host_board_id: "mirror-board".into(),
        part_ids: vec![removed.into(), shifted.into()],
        net_ids: vec![],
        ports: BTreeMap::new(),
    });
    let settings = OutlineSettings {
        size: 4.25,
        bridge_width: 12.5,
        repair: Some(OutlineRepairSettings {
            enabled: true,
            maximum_gap_span: 23.0,
            minimum_connection_width: 2.75,
            edge_clearance: 0.4,
            keep_gaps: vec![ProtectedOutlineGap {
                id: "closure-settings-gap".into(),
                points: vec![OutlineControlPoint {
                    at: Vec2 { x: 1.0, y: 2.0 },
                    part_id: Some(removed.into()),
                }],
            }],
        }),
        ..OutlineSettings::default()
    };
    let feature = OutlineFeature::PartEnvelope {
        id: "closure-envelope".into(),
        part_ids: vec![removed.into(), shifted.into()],
        margin: 2.25,
        operation: Operation::Add,
        settings: settings.clone(),
        connections: vec![OutlineConnection {
            id: "closure-connection".into(),
            width: 3.5,
            points: vec![
                OutlineControlPoint {
                    at: Vec2 { x: 3.0, y: 4.0 },
                    part_id: Some(removed.into()),
                },
                OutlineControlPoint {
                    at: Vec2 { x: 5.0, y: 6.0 },
                    part_id: Some(shifted.into()),
                },
            ],
        }],
    };
    document.outline.push(feature.clone());
    document
        .boards
        .iter_mut()
        .find(|board| board.id == "mirror-board")
        .unwrap()
        .outline_ids
        .push("closure-envelope".into());
    let snapshot = OutlineSnapshot {
        features: vec![feature],
        settings,
        expected_regions: 1,
        bridges: vec![OutlineBridge {
            id: "closure-bridge".into(),
            width: 2.75,
            points: vec![Vec2 { x: 7.0, y: 8.0 }],
            part_ids: vec![removed.into(), shifted.into()],
            matrix_ids: vec!["mirror-matrix".into()],
            authored: true,
        }],
        protected_gaps: vec![ProtectedOutlineGap {
            id: "closure-snapshot-gap".into(),
            points: vec![OutlineControlPoint {
                at: Vec2 { x: 9.0, y: 10.0 },
                part_id: Some(shifted.into()),
            }],
        }],
    };
    document.board_outlines.push(BoardOutline {
        board_id: "mirror-board".into(),
        active_version_id: Some("closure-version".into()),
        versions: vec![OutlineVersion {
            id: "closure-version".into(),
            name: "Closure sentinel".into(),
            source: OutlineProvenance {
                revision: 12,
                version_id: None,
            },
            geometry: snapshot.clone(),
            ownership: OutlineOwnership::Fixed,
        }],
        generated_last_valid: Some(snapshot),
        retired_version_ids: vec![],
    });
    for (id, connector) in [("closure-module", removed), ("shifted-module", shifted)] {
        document.modules.push(
            serde_json::from_value(serde_json::json!({
                "id":id, "definitionId":"closure-definition", "hostBoardId":"mirror-board",
                "hostFace":"front", "facingFace":"back", "at":{"x":1.0,"y":2.0}, "rotation":17.0,
                "gap":0.3, "attachment":"board", "connection":{
                    "hostConnectorPartId":connector, "modulePortId":"port", "busId":"bus",
                    "assignments":{}, "cableType":"A", "railVoltages":{}
                }
            }))
            .unwrap(),
        );
    }
}

pub fn matrix(id: &str, board_id: &str, rows: u32, columns: u32, origin: Vec2) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin,
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
        cells: vec![],
    }
}

pub fn part(
    id: &str,
    definition_id: &str,
    reference: &str,
    pose: Pose2,
    side: &str,
    custom: u64,
) -> Part {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "definitionId": definition_id,
        "reference": reference,
        "pose": pose,
        "side": side,
        "properties": {"custom": custom},
        "generatorParameters": {"side": side, "custom": custom}
    }))
    .expect("the compact part fixture is valid")
}
