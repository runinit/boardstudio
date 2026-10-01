use super::*;

fn contour() -> Vec<Contour> {
    vec![Contour {
        points: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 10.0, y: 0.0 },
            Vec2 { x: 10.0, y: 5.0 },
            Vec2 { x: 0.0, y: 5.0 },
        ],
        hole: false,
    }]
}

fn board_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Document");
    document.boards.push(Board {
        id: "board".into(),
        name: "Main board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document
}

fn prepare(document: ProjectDoc, target: ExportTarget) -> ExportPlan {
    prepare_export(PrepareExportRequest {
        snapshot_token: "snapshot-1".into(),
        expected_revision: document.revision,
        document,
        target,
        contours: contour(),
        model_paths: BTreeMap::new(),
    })
    .expect("prepare")
}

fn ergogen_document() -> ProjectDoc {
    let mut document = board_document();
    document.definitions = ["d1", "d2"]
        .into_iter()
        .map(|id| PartDefinition {
            hardware_profile: None,
            input_profile: None,
            mechanical_profile: None,
            id: id.into(),
            name: format!("Part {id}"),
            kind: PartKind::Utility,
            keycap: None,
            envelope_source: None,
            kicad_source: None,
            terminals: BTreeMap::new(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: vec![
                Vec2 { x: -1.0, y: -1.0 },
                Vec2 { x: 1.0, y: -1.0 },
                Vec2 { x: 1.0, y: 1.0 },
            ],
            pads: vec![],
            models: None,
            generator: Some(PartGenerator {
                source: "ceoloide/utility_text".into(),
                version: "1".into(),
                parameters: BTreeMap::new(),
            }),
        })
        .collect();
    for (index, id) in ["p1", "p2"].into_iter().enumerate() {
        document.parts.push(Part {
            id: id.into(),
            definition_id: format!("d{}", index + 1),
            reference: format!("U{}", index + 1),
            pose: Pose2 {
                at: Vec2 {
                    x: index as f64,
                    y: 0.0,
                },
                rotation: 0.0,
            },
            side: Side::Front,
            keycap: None,
            outline: None,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.boards[0].part_ids.push(id.into());
    }
    document
}

fn placed_native_document(side: Side) -> ProjectDoc {
    let mut document = board_document();
    let definition = asymmetric_definition();
    document.boards[0].part_ids.push("placed".into());
    document.definitions.push(definition);
    document.parts.push(Part {
        id: "placed".into(),
        definition_id: "asym".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        },
        side,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document
}

fn asymmetric_definition() -> PartDefinition {
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        mechanical_profile: None,
        id: "asym".into(),
        name: "Asymmetric".into(),
        kind: PartKind::Custom,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: BTreeMap::new(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 4.0, y: 0.0 },
            Vec2 { x: 4.0, y: 3.0 },
            Vec2 { x: 0.0, y: 3.0 },
        ],
        pads: vec![Pad {
            id: "pad1".into(),
            number: "1".into(),
            at: Vec2 { x: 2.0, y: 3.0 },
            size: Vec2 { x: 2.0, y: 1.0 },
            shape: PadShape::Oval,
            drill: None,
            plated: None,
            side: None,
            rotation: Some(30.0),
            net_id: None,
        }],
        models: None,
        generator: None,
    }
}

fn result(
    plan: &ExportPlan,
    index: usize,
    source: &str,
    nets: Vec<ReservedNet>,
) -> ErgogenJobResult {
    ErgogenJobResult {
        snapshot_token: plan.snapshot_token.clone(),
        revision: plan.revision,
        job_id: plan.jobs[index].job_id.clone(),
        source: source.into(),
        nets,
    }
}

#[test]
fn numbers_match_javascript_fixed_six_decimal_semantics() {
    assert_eq!(num(1.2345675).unwrap(), "1.234568");
    assert_eq!(num(-1.2345675).unwrap(), "-1.234568");
    assert_eq!(num(0.0000005).unwrap(), "0");
    assert_eq!(num(-0.0000005).unwrap(), "0");
    assert_eq!(num(-0.0).unwrap(), "0");
}

#[test]
fn unicode_uuid_is_stable_and_matches_javascript_codepoints() {
    assert_eq!(
        uuid("definition:café🚀"),
        "5d140fed-df0d-59c4-a661-76ff4c486855"
    );
}

#[test]
fn rotated_oblong_uses_absolute_angle_and_back_side_canonical_mirror() {
    let definition = asymmetric_definition();
    let part = Part {
        id: "placed".into(),
        definition_id: definition.id.clone(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2 { x: 11.0, y: 13.0 },
            rotation: 37.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    let front = native_footprint(
        &definition,
        "board:part:placed",
        Some(&part),
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    assert!(front.contains("(at 11 -13 37)"));
    assert!(front.contains("(at 2 -3 67) (size 2 1)"));

    let mut back_part = part;
    back_part.side = Side::Back;
    let back = native_footprint(
        &definition,
        "board:part:placed",
        Some(&back_part),
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    assert!(back.contains("(at 11 -13 -143)"));
    assert!(back.contains("(at 2 3 187) (size 2 1)"));
}

#[test]
fn native_models_use_safe_project_paths_and_kicad_axis_transforms() {
    let mut definition = asymmetric_definition();
    definition.models = Some(vec![PartModel {
        asset_id: "mesh".into(),
        offset: Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        rotation: Vec3 {
            x: 10.0,
            y: 20.0,
            z: 30.0,
        },
        scale: Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    }]);
    let paths = BTreeMap::from([("mesh".into(), "models/body.step".into())]);
    let footprint =
        native_footprint(&definition, "model-test", None, &BTreeMap::new(), &paths).unwrap();
    assert!(footprint.contains("${KIPRJMOD}/models/body.step"));
    assert!(footprint.contains("(offset (xyz 1 -2 3))"));
    assert!(footprint.contains("(scale (xyz 1 2 3))"));
    assert!(footprint.contains("(rotate (xyz 10 20 -30))"));

    for unsafe_path in ["../escape.step", "/absolute.step", "models\\body.step"] {
        let paths = BTreeMap::from([("mesh".into(), unsafe_path.into())]);
        assert!(
            native_footprint(&definition, "model-test", None, &BTreeMap::new(), &paths,).is_err()
        );
    }
}

#[test]
fn imported_source_rejects_nets_bound_to_removed_pad_ids() {
    let source_text = "(footprint \"Imported\" (layer \"F.Cu\") (pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\")))";
    let imported = source::import_footprint(source_text, "imported").unwrap();
    let mut document = board_document();
    document.definitions.push(imported.definition);
    document.parts.push(Part {
        id: "imported-part".into(),
        definition_id: "imported".into(),
        reference: "U1".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.boards[0].part_ids.push("imported-part".into());
    document.nets.push(Net {
        id: "net".into(),
        name: "SIGNAL".into(),
        pins: vec![Pin {
            part_id: "imported-part".into(),
            pad_id: "pad-1".into(),
        }],
    });
    document.boards[0].net_ids.push("net".into());
    let plan = prepare(
        document,
        ExportTarget::Board {
            board_id: "board".into(),
        },
    );
    let error = finish_export(FinishExportRequest {
        plan,
        results: vec![],
    })
    .unwrap_err();
    assert_eq!(error.code, ArtifactErrorCode::Validation);
    assert!(
        error
            .message
            .contains("Net references missing pad on part imported-part")
    );
}

#[test]
fn native_board_output_can_be_written_for_kicad_cli_oracle() {
    let Some(base_path) = std::env::var_os("BOARDSTUDIO_KICAD_ORACLE_PATH") else {
        return;
    };
    let base_path = std::path::PathBuf::from(base_path);
    for (suffix, side) in [("front", Side::Front), ("back", Side::Back)] {
        let document = placed_native_document(side);
        let plan = prepare_export(PrepareExportRequest {
            snapshot_token: format!("oracle-{suffix}"),
            expected_revision: document.revision,
            document,
            target: ExportTarget::Board {
                board_id: "board".into(),
            },
            contours: contour(),
            model_paths: BTreeMap::new(),
        })
        .unwrap();
        let artifact = finish_export(FinishExportRequest {
            plan,
            results: vec![],
        })
        .unwrap();
        let path = base_path.with_file_name(format!(
            "{}-{suffix}.kicad_pcb",
            base_path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("boardstudio-oracle")
        ));
        std::fs::write(path, &artifact.files[0].content).unwrap();
    }
}

#[test]
fn prepared_plan_is_deterministic_and_rejects_mutated_snapshot() {
    let target = ExportTarget::Board {
        board_id: "board".into(),
    };
    let one = prepare(board_document(), target.clone());
    let two = prepare(board_document(), target);
    let output = finish_export(FinishExportRequest {
        plan: one.clone(),
        results: vec![],
    })
    .unwrap();
    let repeated = finish_export(FinishExportRequest {
        plan: two,
        results: vec![],
    })
    .unwrap();
    assert_eq!(output.files, repeated.files);

    let mut altered = one.clone();
    altered.captured_document.name.push('!');
    assert_eq!(
        finish_export(FinishExportRequest {
            plan: altered,
            results: vec![]
        })
        .unwrap_err()
        .code,
        ArtifactErrorCode::MismatchedResults
    );

    let mut altered = one.clone();
    altered
        .model_paths
        .insert("asset".into(), "models/asset.step".into());
    assert_eq!(
        finish_export(FinishExportRequest {
            plan: altered,
            results: vec![]
        })
        .unwrap_err()
        .code,
        ArtifactErrorCode::MismatchedResults
    );
    let mut altered = one.clone();
    altered.contours[0].points[0].x = 3.0;
    assert_eq!(
        finish_export(FinishExportRequest {
            plan: altered,
            results: vec![]
        })
        .unwrap_err()
        .code,
        ArtifactErrorCode::MismatchedResults
    );
    let mut altered = one;
    altered.target = ExportTarget::Board {
        board_id: "other".into(),
    };
    assert_eq!(
        finish_export(FinishExportRequest {
            plan: altered,
            results: vec![]
        })
        .unwrap_err()
        .code,
        ArtifactErrorCode::MismatchedResults
    );
}

#[test]
fn ergogen_results_require_complete_ordered_snapshot_bound_jobs() {
    let plan = prepare(
        ergogen_document(),
        ExportTarget::Board {
            board_id: "board".into(),
        },
    );
    let footprint = "(footprint \"part\" (layer \"F.Cu\") (at 0 0) (uuid \"00000000-0000-5000-a000-000000000000\"))";
    let missing = finish_export(FinishExportRequest {
        plan: plan.clone(),
        results: vec![result(&plan, 0, footprint, vec![])],
    })
    .unwrap_err();
    assert_eq!(missing.code, ArtifactErrorCode::MismatchedResults);

    let reordered = finish_export(FinishExportRequest {
        plan: plan.clone(),
        results: vec![
            result(&plan, 1, footprint, vec![]),
            result(&plan, 0, footprint, vec![]),
        ],
    })
    .unwrap_err();
    assert_eq!(reordered.code, ArtifactErrorCode::MismatchedResults);

    let stale = ErgogenJobResult {
        snapshot_token: "another-snapshot".into(),
        ..result(&plan, 0, footprint, vec![])
    };
    assert_eq!(
        finish_export(FinishExportRequest {
            plan: plan.clone(),
            results: vec![stale, result(&plan, 1, footprint, vec![])]
        })
        .unwrap_err()
        .code,
        ArtifactErrorCode::StaleResult
    );

    let complete = finish_export(FinishExportRequest {
        plan: plan.clone(),
        results: vec![
            result(&plan, 0, footprint, vec![]),
            result(&plan, 1, footprint, vec![]),
        ],
    })
    .unwrap();
    assert_eq!(complete.files.len(), 1);
}

#[test]
fn ergogen_net_references_must_match_reserved_allocator_state() {
    let nets = vec![ReservedNet {
        name: "ROW".into(),
        index: 1,
    }];
    assert!(validate_generated_forms("(segment (net 2 \"ROW\"))", &nets).is_err());
    assert!(validate_generated_forms("(segment (net 1 \"COL\"))", &nets).is_err());
    assert!(
        validate_net_transition(
            &nets,
            &[
                ReservedNet {
                    name: "ROW".into(),
                    index: 1
                },
                ReservedNet {
                    name: "ROW".into(),
                    index: 2
                }
            ]
        )
        .is_err()
    );
}

#[test]
fn ergogen_validation_preserves_escaped_source_spans() {
    let source = r#"(gr_text "line\\nnext" (at 0 0) (layer "F.SilkS"))"#;
    assert_eq!(validate_generated_forms(source, &[]).unwrap(), vec![source]);
}

#[test]
fn standalone_bundles_report_board_utility_skips_but_single_export_errors() {
    let document = ergogen_document();
    let target = ExportTarget::StandaloneFootprints {
        definition_ids: vec!["d1".into(), "d2".into()],
    };
    let plan = prepare(document.clone(), target);
    let utility = r#"(gr_text "utility" (at 0 0) (layer "F.SilkS"))"#;
    let bundle = finish_export(FinishExportRequest {
        plan: plan.clone(),
        results: vec![
            result(&plan, 0, "", vec![]),
            result(&plan, 1, utility, vec![]),
        ],
    })
    .unwrap();
    assert!(bundle.files.is_empty());
    assert_eq!(bundle.skipped_utilities, vec!["Part d1", "Part d2"]);

    let plan = prepare(
        document,
        ExportTarget::StandaloneFootprints {
            definition_ids: vec!["d1".into()],
        },
    );
    let error = finish_export(FinishExportRequest {
        plan: plan.clone(),
        results: vec![result(&plan, 0, utility, vec![])],
    })
    .unwrap_err();
    assert!(
        error
            .message
            .contains("Part d1 emits board objects and must be exported on a board")
    );
}
