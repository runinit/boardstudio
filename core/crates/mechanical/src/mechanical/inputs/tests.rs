use super::*;
use crate::model::{HardwareTransport, MechanicalConstructionOwnership, Side, Vec2};
use crate::test_support::*;

/// Test adapter: these configuration cases are independent of the accepted
/// contours, so they resolve without a contour projection.
fn resolve_inputs(
    document: &ProjectDoc,
    target: &MechanicalTarget,
) -> Result<MechanicalInputs, String> {
    effective_inputs(document, &[], target)
}

#[test]
fn canonical_configuration_resolves_to_itself() {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board", vec![], 1.6));
    let mut canonical = configuration("board");
    canonical.clearance = 0.4;
    // The defaulting pass always materializes the stabilizer proposal list.
    canonical.stabilizers = Some(vec![]);
    document.mechanical = Some(canonical.clone());

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();

    assert_eq!(inputs.board_id, "board");
    assert_eq!(inputs.configuration, canonical);
}

#[test]
fn legacy_instance_overlays_the_shared_construction() {
    let mut common = configuration("shared");
    common.method = PlateMethod::Cnc;
    common.mount = MechanicalMount::Rigid;
    common.plate_thickness = 4.0;
    common.profiles = vec![profile("shared")];
    common.hardware = Some(vec![hardware_specification("shared")]);
    common.critical_fits = Some(vec![critical_fit("shared")]);
    common.gasket_layout = Some(gasket_layout(vec![anchor("shared-support")]));

    let mut local = configuration("board");
    local.method = PlateMethod::Printed;
    local.plate_thickness = 1.0;
    local.openings = Some(vec![opening("local-opening")]);
    local.mounts = vec![mount("local-mount", 1.0, 2.0)];
    local.closure_mounts = Some(vec![mount("local-closure", 1.0, 2.0)]);
    local.battery = Some(battery());
    local.stabilizers = Some(vec![stabilizer()]);
    local.gasket_layout = Some(gasket_layout(vec![anchor("local-support")]));

    let document = document_with(
        board("board", vec![], 1.6),
        hardware(
            vec![instance(
                "left",
                "board",
                MechanicalConstructionOwnership::Legacy,
                Some(local),
            )],
            Some(common.clone()),
            HardwareTransport::None,
        ),
    );

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .unwrap();
    let configuration = inputs.configuration;

    // Overlaid shared fields.
    assert_eq!(configuration.method, PlateMethod::Cnc);
    assert_eq!(configuration.mount, MechanicalMount::Rigid);
    assert_eq!(configuration.plate_thickness, 4.0);
    assert_eq!(configuration.profiles, common.profiles);
    assert_eq!(configuration.hardware, common.hardware);
    assert_eq!(configuration.critical_fits, common.critical_fits);
    // The shared layout keeps the instance's own supports.
    let layout = configuration.gasket_layout.clone().unwrap();
    assert_eq!(layout.length, 99.0);
    assert_eq!(layout.supports, vec![anchor("local-support")]);
    // Local-only fields survive the overlay.
    assert_eq!(configuration.openings, Some(vec![opening("local-opening")]));
    assert_eq!(configuration.mounts, vec![mount("local-mount", 1.0, 2.0)]);
    assert_eq!(
        configuration.closure_mounts,
        Some(vec![mount("local-closure", 1.0, 2.0)])
    );
    assert_eq!(configuration.battery, Some(battery()));
    // The defaulting pass recomputes the stabilizer proposal list; this board
    // has no wide MX keycap part, so the list is empty.
    assert_eq!(configuration.stabilizers, Some(vec![]));
    assert_eq!(configuration.board_id, "board");
    assert_eq!(configuration.pcb_thickness, 1.6);
}

#[test]
fn legacy_instance_without_a_shared_source_falls_back_to_the_first_instance() {
    let mut primary = configuration("board");
    primary.method = PlateMethod::Cnc;
    primary.plate_thickness = 3.0;
    primary.openings = Some(vec![opening("primary-opening")]);
    primary.mounts = vec![mount("primary-mount", 1.0, 2.0)];

    let document = document_with(
        board("board", vec![], 1.6),
        hardware(
            vec![
                instance(
                    "primary",
                    "board",
                    MechanicalConstructionOwnership::Legacy,
                    Some(primary),
                ),
                instance(
                    "secondary",
                    "board",
                    MechanicalConstructionOwnership::Legacy,
                    None,
                ),
            ],
            None,
            HardwareTransport::None,
        ),
    );

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "secondary".into(),
        },
    )
    .unwrap();
    let configuration = inputs.configuration;

    assert_eq!(configuration.method, PlateMethod::Cnc);
    assert_eq!(configuration.plate_thickness, 3.0);
    // The target had no local configuration, so its board-local features clear.
    assert!(configuration.openings.is_none());
    assert!(configuration.mounts.is_empty());
    assert!(configuration.closure_mounts.is_none());
    assert!(configuration.battery.is_none());
    assert_eq!(configuration.board_id, "board");
    assert_eq!(configuration.pcb_thickness, 1.6);
}

#[test]
fn local_instance_ignores_the_shared_overlay() {
    let mut common = configuration("shared");
    common.method = PlateMethod::Cnc;
    common.plate_thickness = 9.0;

    let mut local = configuration("board");
    local.method = PlateMethod::Printed;
    local.plate_thickness = 1.0;

    let document = document_with(
        board("board", vec![], 1.6),
        hardware(
            vec![instance(
                "left",
                "board",
                MechanicalConstructionOwnership::Local,
                Some(local),
            )],
            Some(common),
            HardwareTransport::None,
        ),
    );

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .unwrap();

    assert_eq!(inputs.configuration.method, PlateMethod::Printed);
    assert_eq!(inputs.configuration.plate_thickness, 1.0);
}

#[test]
fn pair_instance_uses_the_pairs_shared_construction() {
    let mut local = configuration("board");
    local.method = PlateMethod::Printed;
    local.plate_thickness = 1.0;
    local.openings = Some(vec![opening("local-opening")]);

    let mut hardware_with_pair = hardware(
        vec![instance(
            "left",
            "board",
            MechanicalConstructionOwnership::Pair,
            Some(local),
        )],
        None,
        HardwareTransport::None,
    );
    hardware_with_pair.pairs.push(mirror_pair(
        "pair-left-right",
        ["left", "right"],
        pose(0.0, 0.0, 0.0),
        pair_construction(),
        vec![],
    ));
    let document = document_with(board("board", vec![], 1.6), hardware_with_pair);

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .unwrap();

    assert_eq!(inputs.configuration.method, PlateMethod::Cnc);
    assert_eq!(inputs.configuration.plate_thickness, 7.0);
    assert_eq!(
        inputs.configuration.openings,
        Some(vec![opening("local-opening")])
    );
    assert_eq!(inputs.configuration.board_id, "board");
    assert_eq!(inputs.configuration.pcb_thickness, 1.6);

    // A `Pair` instance with no owning pair behaves local.
    let mut hardware_with_pair = hardware(
        vec![instance(
            "solo",
            "board",
            MechanicalConstructionOwnership::Pair,
            Some(configuration("board")),
        )],
        None,
        HardwareTransport::None,
    );
    hardware_with_pair.pairs.push(mirror_pair(
        "pair-a-b",
        ["a", "b"],
        pose(0.0, 0.0, 0.0),
        pair_construction(),
        vec![],
    ));
    let document = document_with(board("board", vec![], 1.6), hardware_with_pair);
    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "solo".into(),
        },
    )
    .unwrap();
    assert_eq!(inputs.configuration.method, PlateMethod::Printed);
}

#[test]
fn flipped_instance_keeps_its_board_local_reflection() {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    let mut board = board("board", vec![], 1.6);
    board.part_ids = vec!["part-a".into(), "part-b".into()];
    document.boards.push(board);
    document.parts = vec![
        part_at("part-a", "switch", 3.0, 2.0),
        part_at("part-b", "switch", 8.0, 2.0),
    ];
    let mut flipped = instance(
        "left",
        "board",
        MechanicalConstructionOwnership::Legacy,
        Some(configuration("board")),
    );
    flipped.flipped = true;
    document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::Wireless));

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .unwrap();
    let battery = inputs.configuration.battery.expect("wireless default");

    // Board-local part center x is (3 + 8) / 2 = 5.5; the flip reflects it.
    assert!((battery.at.x + 5.5).abs() < 1e-9, "{:?}", battery.at);
    assert!((battery.at.y - 2.0).abs() < 1e-9, "{:?}", battery.at);
    assert!((battery.cable_exit.x - (-5.5 + 17.0)).abs() < 1e-9);
    assert_eq!(inputs.configuration.battery_height, 6.0);
}

#[test]
fn wireless_transport_supplies_the_battery_default_when_none_is_stored() {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    let mut board = board("board", vec![], 1.6);
    board.part_ids = vec!["part-a".into(), "part-b".into()];
    document.boards.push(board);
    document.parts = vec![
        part_at("part-a", "switch", 3.0, 2.0),
        part_at("part-b", "switch", 8.0, 2.0),
    ];
    document.mechanical = Some(configuration("board"));
    document.hardware = Some(hardware(vec![], None, HardwareTransport::Wireless));

    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    let default_battery = inputs.configuration.battery.expect("wireless default");

    assert!((default_battery.at.x - 5.5).abs() < 1e-9);
    assert_eq!(default_battery.size.z, 6.0);
    assert_eq!(inputs.configuration.battery_height, 6.0);

    // A stored battery is never replaced by the default.
    document.mechanical.as_mut().unwrap().battery = Some(battery());
    let inputs = resolve_inputs(
        &document,
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert_eq!(inputs.configuration.battery, Some(battery()));
}

#[test]
fn ineligible_targets_return_an_error() {
    // No canonical configuration at all.
    let empty = ProjectDoc::empty("doc", "Fixture");
    assert!(
        resolve_inputs(
            &empty,
            &MechanicalTarget::Board {
                board_id: "board".into(),
            },
        )
        .is_err()
    );

    // An eligible board resolves the Core-owned defaults when it has no stored
    // canonical configuration (so the adapter can drop its own defaults builder).
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board", vec![], 1.6));
    let defaults = resolve_inputs(
        &document,
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert_eq!(defaults.board_id, "board");
    assert_eq!(defaults.configuration.board_id, "board");
    assert!(
        defaults.configuration.mounts.is_empty(),
        "a defaulted board keeps no authored mounts"
    );

    // A canonical configuration that belongs to another board is not borrowed:
    // the requested board still resolves its own defaults.
    document.mechanical = Some(configuration("other"));
    let other = resolve_inputs(
        &document,
        &MechanicalTarget::Board {
            board_id: "board".into(),
        },
    )
    .unwrap();
    assert_eq!(other.configuration.board_id, "board");

    // No hardware configuration at all.
    assert!(
        resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .is_err()
    );

    // Hardware without the requested instance.
    document.hardware = Some(hardware(vec![], None, HardwareTransport::None));
    assert!(
        resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .is_err()
    );

    // Instance whose board is not in the document.
    document.hardware = Some(hardware(
        vec![instance(
            "left",
            "ghost",
            MechanicalConstructionOwnership::Legacy,
            Some(configuration("ghost")),
        )],
        None,
        HardwareTransport::None,
    ));
    assert!(
        resolve_inputs(
            &document,
            &MechanicalTarget::Instance {
                instance_id: "left".into(),
            },
        )
        .is_err()
    );
}

#[test]
fn a_flipped_instance_mirrors_parts_references_and_contours() {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    let mut board = board("board", vec![], 1.6);
    board.outline_ids = vec!["outline".into()];
    board.part_ids = vec!["left-switch".into(), "right-switch".into()];
    document.boards.push(board);
    document
        .parts
        .push(part_at("left-switch", "switch", 3.0, 2.0));
    document
        .parts
        .push(part_at("right-switch", "switch", 40.0, 2.0));
    document.board_references.push(
        serde_json::from_value(serde_json::json!({
            "id": "reference", "boardId": "board", "assetId": "asset",
            "enabled": true,
            "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 },
            "elevation": 0.0, "modelAssets": {}
        }))
        .unwrap(),
    );
    let mut flipped = instance(
        "left",
        "board",
        MechanicalConstructionOwnership::Local,
        Some(configuration("board")),
    );
    flipped.flipped = true;
    document.hardware = Some(hardware(vec![flipped], None, HardwareTransport::None));

    let (effective, contours) = effective_document(
        &document,
        &MechanicalTarget::Instance {
            instance_id: "left".into(),
        },
    )
    .unwrap();

    assert_eq!(effective.physical_instance_id.as_deref(), Some("left"));
    let mirrored = effective
        .parts
        .iter()
        .find(|part| part.id == "left-switch")
        .unwrap();
    assert_eq!(mirrored.pose.at.x, -3.0);
    assert_eq!(mirrored.side, Side::Back);
    assert_eq!(
        mirrored.generator_parameters.as_ref().unwrap()["side"],
        serde_json::Value::String("B".into())
    );
    assert_eq!(
        effective
            .parts
            .iter()
            .find(|part| part.id == "right-switch")
            .unwrap()
            .pose
            .at
            .x,
        -40.0
    );
    assert!(!effective.board_references[0].enabled);

    // The returned target contours negate X and reverse the point order.
    let (cache, _, _) = crate::geometry::outlines(&document, None, &[]);
    let (per_board, _, _) = crate::geometry::board_contours(&document, &cache);
    let mut expected = per_board
        .into_iter()
        .find(|entry| entry.board_id == "board")
        .unwrap()
        .contours;
    for contour in &mut expected {
        for point in &mut contour.points {
            point.x = -point.x;
        }
        contour.points.reverse();
    }
    assert_eq!(contours, expected);
    assert!(!contours.is_empty(), "the board outline resolves");
}
