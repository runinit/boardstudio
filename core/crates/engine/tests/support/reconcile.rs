use crate::base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::reconcile::*;
use boardstudio_model::model::*;
use std::collections::{BTreeMap, BTreeSet};
pub fn hosted_electrical_fixture() -> Result<(ProjectDoc, Projection), String> {
    let (mut document, projection) = fixed_geometry_fixture()?;
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
    Ok((document, projection))
}

pub type CompactionFixture = (
    ProjectDoc,
    ProjectDoc,
    MembershipContext,
    BTreeMap<String, Option<String>>,
    BTreeMap<String, FinalOwner>,
    BTreeSet<String>,
);

pub fn membership_compaction_fixture() -> Result<CompactionFixture, String> {
    let (mut before, _) = fixed_geometry_fixture()?;
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

    let pair = &before.hardware.as_ref().unwrap().pairs[0];
    let halves = pair.halves.as_ref().unwrap();
    let mut associations = Vec::new();
    for association in &halves.correspondences {
        let is_child = association.source_id.ends_with("/diode");
        associations.push(TransitionAssociation {
            kind: association.kind,
            source_id: association.source_id.clone(),
            mirror_id: association.mirror_id.clone(),
            parent_source_id: is_child
                .then(|| association.source_id.trim_end_matches("/diode").to_owned()),
            parent_mirror_id: is_child
                .then(|| association.mirror_id.trim_end_matches("/diode").to_owned()),
            state: TransitionState::Linked,
            explicit_state: false,
            raw_mode_override: association.companion_mode,
            raw_tweak: association.companion_tweak,
            effective_mode: association.companion_mode.unwrap_or(halves.companion_mode),
            effective_tweak: association.companion_tweak,
        });
    }
    let mut context = MembershipContext {
        pair_id: pair.id.clone(),
        source_board_id: halves.source_board_id.clone(),
        mirror_board_id: halves.mirror_board_id.clone(),
        matrix_pair: Some(
            boardstudio_core_mirrored_half::mirrored_half::reconcile::MatrixPairContext {
                source_id: "source-matrix".into(),
                mirror_id: "mirror-matrix".into(),
            },
        ),
        mapping: pair.mapping,
        pair_right: halves.clone(),
        associations,
        owners: BTreeMap::new(),
        pending_rejoins: BTreeMap::new(),
        pending_pair_transitions: BTreeMap::new(),
        pending_pair_baseline: None,
    };
    let mut context_owners = BTreeMap::new();
    for part in &before.parts {
        let owner = before
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&part.id))
            .unwrap();
        context_owners.insert(part.id.clone(), FinalOwner::Board(owner.id.clone()));
    }
    context.owners = context_owners;

    let source = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mut identity_map = BTreeMap::new();
    for id in &source.part_ids {
        let (row, column, companion) =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                id,
                "source-matrix",
            )
            .unwrap();
        let next = if column == 1 {
            None
        } else {
            let next_column = if column > 1 { column - 1 } else { column };
            let suffix = if companion.is_empty() {
                String::new()
            } else {
                format!("/{companion}")
            };
            Some(format!("matrix/source-matrix/r{row}c{next_column}{suffix}"))
        };
        identity_map.insert(id.clone(), next);
    }
    let mut post_source = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post_source,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )?;
    let mut final_owners = BTreeMap::new();
    let mut identity_universe: BTreeSet<_> =
        before.parts.iter().map(|part| part.id.clone()).collect();
    identity_universe.extend(post_source.parts.iter().map(|part| part.id.clone()));
    let mut final_matrix_ids = BTreeSet::new();
    for id in identity_map.values().flatten() {
        final_matrix_ids.insert(id.clone());
        let (row, column, child) =
            boardstudio_core_mirrored_half::mirrored_half::reconcile::matrix_identity_slot(
                id,
                "source-matrix",
            )
            .unwrap();
        let suffix = if child.is_empty() {
            String::new()
        } else {
            format!("/{child}")
        };
        final_matrix_ids.insert(format!("matrix/mirror-matrix/r{row}c{column}{suffix}"));
    }
    let prior_matrix_ids: BTreeSet<_> = before
        .matrices
        .iter()
        .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
        .flat_map(|matrix| matrix.part_ids.iter().cloned())
        .collect();
    for id in identity_universe {
        let owner = if final_matrix_ids.contains(&id) {
            FinalOwner::Board(if id.starts_with("matrix/source-matrix/") {
                "source-board".into()
            } else {
                "mirror-board".into()
            })
        } else if prior_matrix_ids.contains(&id) {
            FinalOwner::Absent
        } else {
            post_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&id))
                .map(|board| FinalOwner::Board(board.id.clone()))
                .unwrap_or(FinalOwner::Absent)
        };
        final_owners.insert(id, owner);
    }
    let mut explicit_closure = BTreeSet::new();
    for association in &context.associations {
        if identity_map.get(&association.source_id) == Some(&None) {
            explicit_closure.insert(association.source_id.clone());
            explicit_closure.insert(association.mirror_id.clone());
        } else if let Some(mapped_source) = identity_map
            .get(&association.source_id)
            .and_then(Option::as_ref)
        {
            let mapped_mirror = mapped_source
                .strip_prefix("matrix/source-matrix/")
                .map(|suffix| format!("matrix/mirror-matrix/{suffix}"))
                .unwrap();
            if mapped_mirror != association.mirror_id {
                explicit_closure.insert(association.mirror_id.clone());
            }
        }
    }
    Ok((
        before,
        post_source,
        context,
        identity_map,
        final_owners,
        explicit_closure,
    ))
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

pub fn repair_fresh_coherent_compaction_fixture() -> Result<CompactionFixture, String> {
    let (mut before, _, context, source_map, owners, closure) = membership_compaction_fixture()?;
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
    let mut post = before.clone();
    boardstudio_core_kernel::matrix::deletion::remove_axis(
        &mut post,
        "source-matrix",
        1,
        false,
        crate::support::test_dispatch().apply_unscripted,
    )?;
    Ok((before, post, context, source_map, owners, closure))
}

pub type MembershipExpansionFixture = (
    ProjectDoc,
    ProjectDoc,
    MembershipContext,
    BTreeMap<String, Option<String>>,
    BTreeMap<String, Option<String>>,
    Vec<PlannedAddition>,
    BTreeMap<String, FinalOwner>,
);

pub fn membership_expansion_fixture() -> MembershipExpansionFixture {
    let (mut before, _, context, _, _, _) =
        membership_compaction_fixture().expect("the linked matrix fixture is valid");
    before
        .constraints
        .retain(|c| c.id() != "retained-second-key-offset");
    let mut post_source = before.clone();
    let source_matrix = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .unwrap();
    let mut expanded = source_matrix.clone();
    expanded.columns = 5;
    for row in 0..2 {
        expanded.cells.push(MatrixCell {
            shift: None,
            bend: None,
            deleted: false,
            row,
            column: 4,
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
        });
    }
    boardstudio_core_kernel::matrix::set_matrix(&mut post_source, &expanded)
        .expect("the planned source additions materialize");

    let source_identity_map: BTreeMap<_, _> = source_matrix
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mirror_identity_map: BTreeMap<_, _> = before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let mut additions = Vec::new();
    for row in 0..2 {
        let source_key = format!("matrix/source-matrix/r{row}c4");
        let mirror_key = format!("matrix/mirror-matrix/r{row}c4");
        for (source_id, mirror_id, parent_source_id, parent_mirror_id, mirror_reference) in [
            (
                source_key.clone(),
                mirror_key.clone(),
                None,
                None,
                format!("EXP_M_R{row}C4"),
            ),
            (
                format!("{source_key}/diode"),
                format!("{mirror_key}/diode"),
                Some(source_key.clone()),
                Some(mirror_key.clone()),
                format!("EXP_M_D_R{row}C4"),
            ),
        ] {
            let source_part = post_source
                .parts
                .iter()
                .find(|part| part.id == source_id)
                .unwrap();
            additions.push(PlannedAddition {
                source_id,
                mirror_id,
                source_reference: source_part.reference.clone(),
                mirror_reference,
                parent_source_id,
                parent_mirror_id,
            });
        }
    }
    let mut final_owners: BTreeMap<_, _> = post_source
        .parts
        .iter()
        .map(|part| {
            let board = post_source
                .boards
                .iter()
                .find(|board| board.part_ids.contains(&part.id))
                .unwrap();
            (part.id.clone(), FinalOwner::Board(board.id.clone()))
        })
        .collect();
    for addition in &additions {
        final_owners.insert(
            addition.mirror_id.clone(),
            FinalOwner::Board("mirror-board".into()),
        );
    }
    (
        before,
        post_source,
        context,
        source_identity_map,
        mirror_identity_map,
        additions,
        final_owners,
    )
}

pub fn consolidated_fresh_identity_fixture() -> Result<CompactionFixture, String> {
    let (before, _, mut context, _, _, _) = repair_fresh_coherent_compaction_fixture()?;
    for association in &mut context.associations {
        if association.source_id == "matrix/source-matrix/r1c1" {
            association.state = TransitionState::Local;
            association.explicit_state = true;
        } else if association.source_id == "matrix/source-matrix/r1c1/diode" {
            association.state = TransitionState::Local;
            association.explicit_state = false;
        }
    }
    let source_map = before
        .matrices
        .iter()
        .find(|m| m.id == "source-matrix")
        .unwrap()
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect();
    let owners = context.owners.clone();
    Ok((
        before.clone(),
        before,
        context,
        source_map,
        owners,
        BTreeSet::new(),
    ))
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

/// Mirror identity map derived from one source identity map.
///
/// Every mirrored membership refusal test must translate the source operation
/// into mirror slots the same way; deriving it here keeps the operation
/// sequence sourced from the input instead of re-typed per test.
pub fn mirror_map_from_source(
    source_map: &BTreeMap<String, Option<String>>,
) -> BTreeMap<String, Option<String>> {
    source_map
        .iter()
        .map(|(old, new)| {
            (
                old.replace("matrix/source-matrix/", "matrix/mirror-matrix/"),
                new.as_ref()
                    .map(|id| id.replace("matrix/source-matrix/", "matrix/mirror-matrix/")),
            )
        })
        .collect()
}

/// Identity mirror map derived from the source document's actual matrix members.
///
/// Fresh-identity transitions keep every historical member, so the expected map
/// is exactly the document's own part list; deriving it proves the test checks
/// exact row/column identities instead of a hand-typed subset.
pub fn fresh_identity_mirror_map(before: &ProjectDoc) -> BTreeMap<String, Option<String>> {
    before
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .expect("the mirror matrix exists")
        .part_ids
        .iter()
        .map(|id| (id.clone(), Some(id.clone())))
        .collect()
}

/// Exact row/column identity invariant for one matrix, derived from its dimensions.
///
/// Every member id must parse to an in-bounds `(row, column, child)` slot, every
/// slot present in the matrix must resolve to a live part registered on exactly
/// one board, and the member set must equal the full source-derived slot
/// universe (one key plus one diode child per cell). A single mistyped row or
/// column fails either the parse, the bound, or the exact-set comparison.
pub fn assert_exact_matrix_identities(document: &ProjectDoc, matrix_id: &str) {
    let matrix = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
        .unwrap_or_else(|| panic!("{matrix_id} exists"));
    let mut expected = BTreeSet::new();
    for row in 0..matrix.rows {
        for column in 0..matrix.columns {
            expected.insert(format!("matrix/{matrix_id}/r{row}c{column}"));
            expected.insert(format!("matrix/{matrix_id}/r{row}c{column}/diode"));
        }
    }
    let actual: BTreeSet<_> = matrix.part_ids.iter().cloned().collect();
    assert_eq!(
        actual, expected,
        "{matrix_id}: member set is exactly the source-derived row/column universe"
    );
    for id in &actual {
        let (row, column, _) = matrix_identity_slot(id, matrix_id)
            .unwrap_or_else(|| panic!("{id} parses as a {matrix_id} slot"));
        assert!(
            row < matrix.rows && column < matrix.columns,
            "{id} is in bounds for {matrix_rows}x{matrix_columns}",
            matrix_rows = matrix.rows,
            matrix_columns = matrix.columns,
        );
        assert!(
            document.parts.iter().any(|part| part.id == *id),
            "{id} resolves to a live part"
        );
        assert_eq!(
            document
                .boards
                .iter()
                .filter(|board| board.part_ids.contains(id))
                .count(),
            1,
            "{id} is registered on exactly one board"
        );
    }
}

/// History invariant: a refused transition must leave caller inputs untouched.
pub fn assert_membership_inputs_unchanged(
    before: &ProjectDoc,
    snapshot: &ProjectDoc,
    context: &MembershipContext,
    context_snapshot: &MembershipContext,
) {
    assert_eq!(before, snapshot, "caller document remains unchanged");
    assert_eq!(
        context, context_snapshot,
        "caller context remains unchanged"
    );
}

/// Operation-sequence invariant: an exact repeat stages nothing new.
pub fn assert_repeat_is_noop(
    repeated_draft: &ProjectDoc,
    draft: &ProjectDoc,
    repeated_context: &MembershipContext,
    context: &MembershipContext,
    changed_ids: &[String],
    changed_paths: &[String],
    phase: &str,
) {
    assert_eq!(repeated_draft, draft, "{phase}: repeated draft is identical");
    assert_eq!(
        repeated_context, context,
        "{phase}: repeated context is identical"
    );
    assert!(
        changed_ids.is_empty(),
        "{phase}: no changed document ids"
    );
    assert!(
        changed_paths.is_empty(),
        "{phase}: no changed field paths"
    );
}

/// Run one fresh-identity refusal case: apply `mutate`, derive the mirror map
/// from the mutated source, snapshot history, run the transition, assert the
/// refusal and that history is preserved, and return the refusal message.
///
/// Centralizing the sequence keeps every row sourced from the same operation
/// instead of duplicating fixture/mirror-map/snapshot assertions per refusal.
pub fn run_fresh_identity_refusal(
    mutate: impl FnOnce(&mut ProjectDoc, &mut MembershipContext),
    expect: &str,
) -> String {
    let (mut before, _, mut context, source_map, owners, _) =
        consolidated_fresh_identity_fixture().expect("the fresh identity fixture is valid");
    mutate(&mut before, &mut context);
    let mirror_map = fresh_identity_mirror_map(&before);
    let snapshot = before.clone();
    let context_snapshot = context.clone();
    let result = prepare_membership_transition(MembershipTransitionInput {
        dispatch: crate::support::test_dispatch(),
        valid_before: &before,
        post_source: &before,
        context: &context,
        baseline_kind: BaselineKind::MaterializedCurrentSchema,
        source_identity_map: &source_map,
        mirror_identity_map: &mirror_map,
        planned_additions: &[],
        final_owners: &owners,
        explicit_removal_closure: &BTreeSet::new(),
    });
    assert_membership_inputs_unchanged(&before, &snapshot, &context, &context_snapshot);
    let message = result.unwrap_err();
    assert!(
        message.contains(expect),
        "expected refusal containing {expect:?}, got {message:?}"
    );
    message
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
