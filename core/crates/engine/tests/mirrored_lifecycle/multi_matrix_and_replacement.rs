use super::*;
use crate::constraint_lifecycle::{part, valid_omitted_document};
use crate::independent_exceptions::independent_lifecycle_fixture;
use crate::matrix_and_circuit_lifecycle::circuit_lifecycle_fixture;

pub(crate) fn add_second_matrix_pair(document: &mut ProjectDoc) -> Result<(), String> {
    let mut source = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "source-matrix")
        .ok_or("fixture source Matrix missing")?
        .clone();
    source.id = "source-matrix-2".into();
    source.name = Some("second source Matrix".into());
    source.part_ids.clear();
    boardstudio_core_kernel::matrix::set_matrix(document, &source)?;

    let mut mirror = document
        .matrices
        .iter()
        .find(|matrix| matrix.id == "mirror-matrix")
        .ok_or("fixture mirror Matrix missing")?
        .clone();
    mirror.id = "mirror-matrix-2".into();
    mirror.name = Some("second mirror Matrix".into());
    mirror.part_ids.clear();
    let correspondence = MirroredCorrespondence {
        kind: MirroredItemKind::Matrix,
        source_id: source.id.clone(),
        mirror_id: mirror.id.clone(),
        companion_mode: None,
        companion_tweak: None,
    };
    mirror = boardstudio_core_mirrored_half::mirrored_half::derive::project_matrix(
        &source,
        &mirror,
        &correspondence,
        document.hardware.as_ref().unwrap().pairs[0].mapping,
    )?;
    boardstudio_core_kernel::matrix::set_matrix(document, &mirror)?;

    let halves = document.first_halves_mut();
    halves.correspondences.push(correspondence);
    let original_children: Vec<_> = halves
        .correspondences
        .iter()
        .filter(|record| {
            record.kind == MirroredItemKind::Part
                && record.source_id.starts_with("matrix/source-matrix/")
        })
        .cloned()
        .collect();
    for original in original_children {
        let source_id =
            original
                .source_id
                .replacen("matrix/source-matrix/", "matrix/source-matrix-2/", 1);
        let mirror_id =
            original
                .mirror_id
                .replacen("matrix/mirror-matrix/", "matrix/mirror-matrix-2/", 1);
        let record = MirroredCorrespondence {
            kind: MirroredItemKind::Part,
            source_id: source_id.clone(),
            mirror_id: mirror_id.clone(),
            companion_mode: original.companion_mode,
            companion_tweak: original.companion_tweak,
        };
        halves.correspondences.push(record);
    }
    boardstudio_core_mechanical::mechanical::halves::validate_document(document)?;
    Ok(())
}

#[test]
fn independent_part_exception_intents_keep_exact_matrix_owners_and_children() {
    for matrix_count in [1, 2] {
        let mut before = independent_lifecycle_fixture();
        if matrix_count == 2 {
            add_second_matrix_pair(&mut before).unwrap();
        }
        let matrices_before = before.matrices.clone();
        let children_before: Vec<_> = before
            .parts
            .iter()
            .filter(|part| part.id.starts_with("matrix/"))
            .cloned()
            .collect();

        let local = exception(
            &before,
            MirroredItemKind::Part,
            "mirror-screen",
            HalfExceptionState::Local,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"));
        assert_eq!(local.matrices, matrices_before);
        for child in &children_before {
            assert_eq!(
                local.parts.iter().find(|part| part.id == child.id),
                Some(child)
            );
        }
        assert!(local.first_halves().exceptions.iter().any(|exception| {
            exception.kind == MirroredItemKind::Part && exception.mirror_id == "mirror-screen"
        }));

        let mut edited_source = local.clone();
        let source = edited_source.parts.by_id_mut("source-encoder");
        source.pose.at.x += 23.0;
        source.properties = Some(BTreeMap::from([("revision".into(), json!(2))]));
        let edited = support::prepare_open(&edited_source, &[]).unwrap().0;
        let source = edited_source.parts.by_id("source-encoder");
        let mirror = edited.parts.by_id("mirror-encoder");
        assert_eq!(
            mirror,
            &boardstudio_core_mirrored_half::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                before.hardware.as_ref().unwrap().pairs[0].mapping,
                boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
                    &edited_source.definitions,
                    &source.definition_id,
                ),
            )
        );
        assert_eq!(edited.matrices, matrices_before);

        let omitted = exception(
            &edited,
            MirroredItemKind::Part,
            "mirror-screen",
            HalfExceptionState::Omitted,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"));
        assert!(omitted.parts.iter().all(|part| part.id != "mirror-screen"));
        assert_eq!(omitted.matrices, matrices_before);
        let rejoined = exception(
            &omitted,
            MirroredItemKind::Part,
            "mirror-screen",
            HalfExceptionState::Linked,
        )
        .unwrap_or_else(|error| panic!("{matrix_count} Matrix pairs: {error}"));
        let source = rejoined.parts.by_id("source-screen");
        let mirror = rejoined.parts.by_id("mirror-screen");
        assert_eq!(
            mirror,
            &boardstudio_core_mirrored_half::mirrored_half::derive::part(
                source,
                &mirror.id,
                &mirror.definition_id,
                &mirror.reference,
                before.hardware.as_ref().unwrap().pairs[0].mapping,
                boardstudio_core_mirrored_half::mirrored_half::derive::is_controller(
                    &rejoined.definitions,
                    &source.definition_id,
                ),
            )
        );
        assert_eq!(rejoined.matrices, matrices_before);
        assert!(rejoined.first_halves().exceptions.is_empty());
    }
}

#[test]
fn independent_circuit_exception_intents_work_beside_multiple_matrix_owners() {
    let mut before = circuit_lifecycle_fixture();
    add_second_matrix_pair(&mut before).unwrap();
    let matrices_before = before.matrices.clone();
    let explicit_children: Vec<_> = before
        .parts
        .iter()
        .filter(|part| part.id.starts_with("matrix/"))
        .cloned()
        .collect();

    let local = exception(
        &before,
        MirroredItemKind::EmbeddedCircuit,
        "mirror-circuit",
        HalfExceptionState::Local,
    )
    .unwrap_or_else(|error| panic!("circuit Local: {error}"));
    let omitted = exception(
        &local,
        MirroredItemKind::EmbeddedCircuit,
        "mirror-circuit",
        HalfExceptionState::Omitted,
    )
    .unwrap_or_else(|error| panic!("circuit Omit: {error}"));
    assert!(
        omitted
            .embedded_circuits
            .iter()
            .all(|circuit| circuit.id != "mirror-circuit")
    );
    let rejoined = exception(
        &omitted,
        MirroredItemKind::EmbeddedCircuit,
        "mirror-circuit",
        HalfExceptionState::Linked,
    )
    .unwrap_or_else(|error| panic!("circuit Rejoin: {error}"));
    assert!(
        rejoined
            .embedded_circuits
            .iter()
            .any(|circuit| circuit.id == "mirror-circuit")
    );
    assert_eq!(rejoined.matrices, matrices_before);
    for child in &explicit_children {
        assert_eq!(
            rejoined.parts.iter().find(|part| part.id == child.id),
            Some(child)
        );
    }
    assert!(rejoined.first_halves().exceptions.iter().all(|exception| {
        exception.kind != MirroredItemKind::EmbeddedCircuit
            || exception.mirror_id != "mirror-circuit"
    }));
}

#[test]
fn an_overlay_with_saved_copper_round_trips_and_a_legacy_exception_is_not_linked() {
    let value = json!({
        "sourceBoardId": "source-board", "mirrorBoardId": "mirror-board",
        "correspondences": [{ "kind": "part", "sourceId": "source-part", "mirrorId": "mirror-part",
            "companionMode": "duplicate-position" }],
        "exceptions": [
            { "kind": "part", "mirrorId": "mirror-part", "state": "omitted", "removedCopper": {
                "traces": [{ "id": "t", "start": {"x": 0.0, "y": 0.0}, "end": {"x": 1.0, "y": 0.0},
                    "width": 0.2, "layer": "front", "netId": "n" }],
                "vias": [{ "id": "v", "at": {"x": 2.0, "y": 0.0}, "size": 0.8, "drill": 0.4 }],
                "pads": [{ "partId": "mirror-part", "padId": "1", "at": {"x": 3.0, "y": 4.0} }] } },
            { "kind": "part", "mirrorId": "legacy-part", "state": "local" }
        ]
    });
    let halves: MirroredHalves = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&halves).unwrap(),
        value,
        "the shape is stable"
    );
    let omitted = &halves.exceptions[0];
    assert_eq!(halves.overlay_source(omitted), Some("source-part"));
    assert_eq!(
        omitted.removed_copper.as_ref().unwrap().pads[0].at,
        Vec2 { x: 3.0, y: 4.0 }
    );
    let legacy = &halves.exceptions[1];
    assert_eq!(
        halves.overlay_source(legacy),
        None,
        "a legacy record infers no source"
    );
    assert!(legacy.removed_copper.is_none());
    assert_eq!(
        halves.correspondences.len(),
        1,
        "and no correspondence is invented for it"
    );
}
pub(crate) fn replacement_retirement_fixture() -> ProjectDoc {
    use boardstudio_model::model::MirroredItemKind as Kind;

    let mut document = valid_omitted_document();
    let halves = document.first_halves_mut();
    let mut rows = vec![(Kind::Part, "source-part", "mirror-part")];
    for (kind, source_id, mirror_id) in [
        (Kind::Matrix, "source-matrix", "mirror-matrix"),
        (Kind::Layout, "source-layout", "mirror-layout"),
        (Kind::Net, "source-net", "mirror-net"),
        (Kind::EmbeddedCircuit, "source-circuit", "mirror-circuit"),
    ] {
        rows.push((kind, source_id, mirror_id));
    }
    halves.correspondences = rows
        .iter()
        .map(|(kind, source_id, mirror_id)| MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: *kind,
            source_id: (*source_id).into(),
            mirror_id: (*mirror_id).into(),
        })
        .collect();
    halves.exceptions = rows
        .iter()
        .map(|(kind, _, mirror_id)| MirroredException {
            companion_mode: None,
            companion_tweak: None,
            kind: *kind,
            mirror_id: (*mirror_id).into(),
            state: MirroredExceptionState::Omitted,
            removed_copper: None,
        })
        .collect();
    document.matrices.push(
        serde_json::from_value(json!({
            "id":"source-matrix","name":"Source","rows":1,"columns":1,
            "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
            "definitionId":"test-footprint","partIds":[],"boardId":"source-board"
        }))
        .unwrap(),
    );
    document.layouts.push(
        serde_json::from_value(json!({
            "id":"source-layout","name":"Source","boardId":"source-board",
            "matrixIds": ["source-matrix"],"partIds":[]
        }))
        .unwrap(),
    );
    document.nets.push(Net {
        id: "source-net".into(),
        name: "Source net".into(),
        pins: Vec::new(),
    });
    document.boards[0].net_ids.push("source-net".into());
    document.embedded_circuits.push(
        serde_json::from_value(json!({
            "id":"source-circuit","definitionId":"test-circuit",
            "hostBoardId":"source-board","partIds":[],"netIds":[],"ports":{}
        }))
        .unwrap(),
    );
    document
}

pub(crate) fn remove_replacement_retirement_sources(mut replacement: ProjectDoc) -> ProjectDoc {
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    replacement
        .matrices
        .retain(|matrix| matrix.id != "source-matrix");
    replacement
        .layouts
        .retain(|layout| layout.id != "source-layout");
    replacement.nets.retain(|net| net.id != "source-net");
    replacement.boards[0]
        .net_ids
        .retain(|net_id| net_id != "source-net");
    replacement
        .embedded_circuits
        .retain(|circuit| circuit.id != "source-circuit");
    replacement
}

#[test]
fn replacement_batch_retires_typed_part_matrix_layout_net_and_circuit_rows() {
    let accepted = replacement_retirement_fixture();
    boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).unwrap();
    let replacement = remove_replacement_retirement_sources(accepted.clone());

    let (replacement, _) = support::prepare_transaction(
        &accepted,
        &[],
        &EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    )
    .unwrap();

    let halves = replacement.first_halves();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
    assert!(
        boardstudio_core_mechanical::mechanical::halves::validate_document(&replacement).is_ok()
    );
}

#[test]
fn replacement_batch_keeps_local_target_geometry_byte_for_byte() {
    let mut accepted = valid_omitted_document();
    let mirror = part("mirror-part");
    accepted.parts.push(mirror.clone());
    accepted.boards[1].part_ids.push("mirror-part".into());
    let halves = accepted.first_halves_mut();
    halves.exceptions[0].state = MirroredExceptionState::Local;
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).is_ok());
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");

    let (replacement, _) = support::prepare_transaction(
        &accepted,
        &[],
        &EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    )
    .unwrap();

    assert_eq!(
        replacement
            .parts
            .iter()
            .find(|part| part.id == "mirror-part"),
        Some(&mirror)
    );
    assert!(
        replacement.boards[1]
            .part_ids
            .iter()
            .any(|id| id == "mirror-part")
    );
    let halves = replacement.first_halves();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
}

#[test]
fn replacement_batch_retires_exact_record_with_unrelated_halves_metadata_change() {
    let accepted = valid_omitted_document();
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    let halves = replacement.first_halves_mut();
    halves.companion_mode = MirroredCompanionMode::DuplicatePosition;

    let (replacement, _) = support::prepare_transaction(
        &accepted,
        &[],
        &EditOperation::ReplaceDocument {
            document: Box::new(replacement),
        },
    )
    .unwrap();

    let halves = replacement.first_halves();
    assert!(halves.correspondences.is_empty());
    assert!(halves.exceptions.is_empty());
    assert_eq!(
        halves.companion_mode,
        MirroredCompanionMode::DuplicatePosition
    );
}

#[test]
fn replacement_batch_refuses_incomplete_matrix_owned_part_closure_atomically() {
    let mut accepted = valid_omitted_document();
    accepted.matrices.push(
        serde_json::from_value(json!({
            "id":"owner-matrix","rows":1,"columns":1,
            "pitch":{"x":19.0,"y":19.0},"origin":{"x":0.0,"y":0.0},
            "definitionId":"test-footprint","partIds":["source-part"],"boardId":"source-board"
        }))
        .unwrap(),
    );
    let mut replacement = accepted.clone();
    replacement.parts.retain(|part| part.id != "source-part");
    replacement.boards[0]
        .part_ids
        .retain(|part_id| part_id != "source-part");
    let halves_before = replacement.first_halves().clone();

    let refusal = support::prepare_transaction(
        &accepted,
        &[],
        &EditOperation::ReplaceDocument {
            document: Box::new(replacement.clone()),
        },
    )
    .unwrap_err();

    assert!(
        refusal.contains("incomplete ownership closure"),
        "{refusal}"
    );
    assert_eq!(
        replacement.first_halves(),
        &halves_before,
        "a refused batch does not partially retire any metadata"
    );
}

#[test]
fn replacement_batch_leaves_unowned_kind_to_existing_document_validation() {
    use boardstudio_model::model::MirroredItemKind;

    let mut accepted = valid_omitted_document();
    let halves = accepted.first_halves_mut();
    halves.correspondences[0].kind = MirroredItemKind::Outline;
    halves.correspondences[0].source_id = "source-outline".into();
    halves.correspondences[0].mirror_id = "mirror-outline".into();
    halves.exceptions[0].kind = MirroredItemKind::Outline;
    halves.exceptions[0].mirror_id = "mirror-outline".into();
    accepted.boards[0].outline_ids.push("source-outline".into());
    accepted.outline.push(
        serde_json::from_value(json!({
            "kind":"rect","id":"source-outline","center":{"x":0.0,"y":0.0},
            "size":{"x":10.0,"y":10.0},"radius":0.0,"operation":"add"
        }))
        .unwrap(),
    );
    assert!(boardstudio_core_mechanical::mechanical::halves::validate_document(&accepted).is_ok());
    let mut replacement = accepted.clone();
    replacement
        .outline
        .retain(|outline| outline.id() != "source-outline");
    replacement.boards[0]
        .outline_ids
        .retain(|outline_id| outline_id != "source-outline");
    let halves_before = replacement.first_halves().clone();

    let refusal = support::prepare_transaction(
        &accepted,
        &[],
        &EditOperation::ReplaceDocument {
            document: Box::new(replacement.clone()),
        },
    )
    .unwrap_err();

    assert!(
        refusal.contains("names outline"),
        "existing halves validation owns unsupported-kind refusal: {refusal}"
    );
    assert_eq!(replacement.first_halves(), &halves_before);
}
