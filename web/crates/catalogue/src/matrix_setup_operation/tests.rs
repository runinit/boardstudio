use super::{MatrixSetupPreset, MatrixSetupRequest, next_matrix_id, prepare_matrix};
use boardstudio_core::model::{Part, PartKind, Pose2, Side, Vec2};

use super::test_catalogue::{catalogue, definition};

#[test]
fn matrix_setup_builds_the_reference_mx_rgb_cell_recipe_and_owned_definition_snapshots() {
    let prepared = prepare_matrix(
        "matrix-created".into(),
        "board-main".into(),
        MatrixSetupRequest {
            rows: 2,
            columns: 1,
            preset: MatrixSetupPreset::MxRgb,
        },
        false,
        &catalogue(),
    )
    .expect("valid matrix setup");

    assert_eq!(prepared.matrix.id, "matrix-created");
    assert_eq!(prepared.matrix.board_id.as_deref(), Some("board-main"));
    assert_eq!((prepared.matrix.rows, prepared.matrix.columns), (2, 1));
    assert_eq!(prepared.matrix.pitch.x, 19.05);
    assert_eq!(prepared.matrix.pitch.y, 19.05);
    assert_eq!(prepared.matrix.origin.x, 0.0);
    assert_eq!(prepared.matrix.origin.y, 0.0);
    assert_eq!(
        prepared.matrix.edge_gap.as_ref().map(|gap| (gap.x, gap.y)),
        Some((1.0, 1.0))
    );
    assert_eq!(prepared.matrix.cells.len(), 2);
    assert_eq!(
        prepared.matrix.cells[0].variant.as_deref(),
        Some("preset/mx-rgb/south")
    );
    assert_eq!(
        prepared.matrix.cells[0].definition_id,
        Some(prepared.matrix.definition_id.clone())
    );
    assert_eq!(prepared.matrix.cells[0].assemblies_local, Some(true));
    assert_eq!(
        prepared.matrix.cells[0]
            .assemblies
            .iter()
            .map(|member| member.id.as_str())
            .collect::<Vec<_>>(),
        ["diode", "led"]
    );
    assert_eq!(prepared.matrix.cells[0].assemblies[0].offset.x, 7.4);
    assert_eq!(prepared.matrix.cells[0].assemblies[0].offset.y, -1.5);
    assert_eq!(prepared.matrix.cells[0].assemblies[0].rotation, Some(90.0));
    assert_eq!(prepared.matrix.cells[1].row, 1);
    assert_eq!(prepared.matrix.cells[1].column, 0);
    assert_eq!(prepared.definitions.len(), 3);
    assert_eq!(prepared.definitions[0].id, prepared.matrix.definition_id);
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["hotswap"],
        false
    );
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["solder"],
        true
    );
    assert_eq!(
        prepared.definitions[0]
            .generator
            .as_ref()
            .unwrap()
            .parameters["include_keycap"],
        true
    );
}

#[test]
fn matrix_setup_maps_every_reference_preset_without_losing_construction_flags() {
    let cases = [
        (
            MatrixSetupPreset::MxSolder,
            "generator:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            false,
            false,
            "preset/mx-solder/reversible/south",
        ),
        (
            MatrixSetupPreset::MxHotswap,
            "generator:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            true,
            false,
            "preset/mx-hotswap/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocSolder,
            "generator:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            false,
            false,
            "preset/choc-solder/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocHotswap,
            "generator:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            true,
            false,
            "preset/choc-hotswap/reversible/south",
        ),
        (
            MatrixSetupPreset::MxRgb,
            "generator:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            false,
            true,
            "preset/mx-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocRgb,
            "generator:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            false,
            true,
            "preset/choc-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::MxHotswapRgb,
            "generator:ceoloide/switch_mx",
            "ceoloide/switch_mx",
            true,
            true,
            "preset/mx-hotswap-rgb/reversible/south",
        ),
        (
            MatrixSetupPreset::ChocHotswapRgb,
            "generator:ceoloide/switch_choc_v1_v2",
            "ceoloide/switch_choc_v1_v2",
            true,
            true,
            "preset/choc-hotswap-rgb/reversible/south",
        ),
    ];
    for (preset, definition_id, generator_source, hotswap, led, expected_variant) in cases {
        let prepared = prepare_matrix(
            format!("matrix-{}", preset.as_str()),
            "board-main".into(),
            MatrixSetupRequest {
                rows: 1,
                columns: 1,
                preset,
            },
            true,
            &catalogue(),
        )
        .expect("preset is supported by the fixture catalogue");
        assert!(prepared.matrix.definition_id.starts_with(&format!(
            "assembly-preset-{}-south-reversible-matrix-{}-0/definition/switch",
            preset.as_str(),
            preset.as_str()
        )));
        assert_eq!(
            prepared.definitions[0].generator.as_ref().unwrap().source,
            generator_source
        );
        assert!(prepared.definitions[0].id.ends_with("/definition/switch"));
        assert_ne!(prepared.definitions[0].id, definition_id);
        assert_eq!(prepared.matrix.definition_id, prepared.definitions[0].id);
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["hotswap"],
            hotswap
        );
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["solder"],
            !hotswap
        );
        assert_eq!(
            prepared.definitions[0]
                .generator
                .as_ref()
                .unwrap()
                .parameters["reversible"],
            true
        );
        assert_eq!(
            prepared.matrix.cells[0].assemblies.len(),
            1 + usize::from(led)
        );
        assert_eq!(
            prepared.matrix.cells[0].variant.as_deref(),
            Some(expected_variant)
        );
    }
}

#[test]
fn matrix_setup_rejects_invalid_size_or_missing_real_catalogue_entries() {
    let catalogue = catalogue();
    for (rows, columns) in [(0, 1), (1, 0), (65, 64), (u32::MAX, 2)] {
        assert!(
            prepare_matrix(
                "matrix-invalid".into(),
                "board-main".into(),
                MatrixSetupRequest {
                    rows,
                    columns,
                    preset: MatrixSetupPreset::MxSolder
                },
                false,
                &catalogue,
            )
            .is_err(),
            "invalid size {rows}×{columns} must not create a matrix"
        );
    }
    assert!(
        prepare_matrix(
            "matrix-missing".into(),
            "board-main".into(),
            MatrixSetupRequest {
                rows: 1,
                columns: 1,
                preset: MatrixSetupPreset::MxSolder
            },
            false,
            &catalogue[1..2],
        )
        .is_err(),
        "a missing diode must be reported rather than fabricated"
    );
}

#[test]
fn generated_matrix_identity_skips_existing_matrix_definition_and_member_ids() {
    let preset = MatrixSetupPreset::MxSolder;
    let occupied_definition = definition(
        "assembly-preset-mx-solder-south-matrix-1-0/definition/switch",
        "ceoloide/switch_mx",
        PartKind::Switch,
    );
    assert_eq!(
        next_matrix_id(&[], &[occupied_definition], &[], preset.as_str(), false),
        "matrix-2"
    );

    let occupied_part = Part {
        anchor: None,
        keycap: None,
        outline: None,
        id: "matrix/matrix-1/r0c0".into(),
        definition_id: "existing-definition".into(),
        reference: "SW1".into(),
        pose: Pose2 {
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        locked: None,
        properties: None,
        generator_parameters: None,
    };
    assert_eq!(
        next_matrix_id(&[], &[], &[occupied_part], preset.as_str(), false),
        "matrix-2"
    );
}
