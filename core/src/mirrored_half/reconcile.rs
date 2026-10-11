//! Forward reconciliation building blocks for already-corresponded mirrored values.

use crate::model::{
    Matrix, MirroredCorrespondence, MirroredItemKind, Part, Pose2, ProjectDoc, Vec2,
};

struct PartProjection {
    source_id: String,
    mirror_id: String,
}

struct Projection {
    source_board_id: String,
    mirror_board_id: String,
    source_matrix_id: String,
    mirror_matrix_id: String,
    mapping: Pose2,
    parts: Vec<PartProjection>,
    locally_protected_mirror_parts: std::collections::BTreeSet<String>,
    final_owners: std::collections::BTreeMap<String, String>,
}

#[allow(dead_code)] // Ticket 111 activates this private seam after the reconciliation slices are verified.
fn reconcile(
    document: &mut ProjectDoc,
    pair_id: &str,
    projection: &Projection,
) -> Result<Vec<String>, String> {
    fn matrix_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Matrix, String> {
        let mut matches = document.matrices.iter().filter(|matrix| matrix.id == id);
        let matrix = matches
            .next()
            .ok_or_else(|| format!("The mirrored matrix {id:?} is unavailable"))?;
        if matches.next().is_some() {
            return Err(format!("The mirrored matrix {id:?} is ambiguous"));
        }
        Ok(matrix)
    }

    fn part_by_id<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a Part, String> {
        let mut matches = document.parts.iter().filter(|part| part.id == id);
        let part = matches
            .next()
            .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
        if matches.next().is_some() {
            return Err(format!("The mirrored part {id:?} is ambiguous"));
        }
        Ok(part)
    }

    fn owner_for<'a>(document: &'a ProjectDoc, id: &str) -> Result<&'a str, String> {
        let mut owners = document
            .boards
            .iter()
            .filter(|board| board.part_ids.iter().any(|part_id| part_id == id));
        let board = owners
            .next()
            .ok_or_else(|| format!("The mirrored part {id:?} has no board owner"))?;
        if owners.next().is_some() {
            return Err(format!(
                "The mirrored part {id:?} has ambiguous board ownership"
            ));
        }
        Ok(&board.id)
    }

    fn snapshot<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, String> {
        serde_json::to_value(value).map_err(|error| error.to_string())
    }

    let hardware = document
        .hardware
        .as_ref()
        .ok_or_else(|| "The mirrored hardware configuration is unavailable".to_owned())?;
    let mut pairs = hardware.pairs.iter().filter(|pair| pair.id == pair_id);
    let pair = pairs
        .next()
        .ok_or_else(|| format!("The mirrored pair {pair_id:?} is unavailable"))?;
    if pairs.next().is_some() {
        return Err(format!("The mirrored pair {pair_id:?} is ambiguous"));
    }
    let halves = pair
        .halves
        .as_ref()
        .ok_or_else(|| format!("The mirrored pair {pair_id:?} has no halves record"))?;
    if halves.source_board_id != projection.source_board_id
        || halves.mirror_board_id != projection.mirror_board_id
        || pair.mapping != projection.mapping
    {
        return Err("The reconciliation input does not match its canonical pair".into());
    }

    let mut matrix_correspondences = halves.correspondences.iter().filter(|correspondence| {
        correspondence.kind == MirroredItemKind::Matrix
            && (correspondence.source_id == projection.source_matrix_id
                || correspondence.mirror_id == projection.mirror_matrix_id)
    });
    let matrix_correspondence = matrix_correspondences
        .next()
        .ok_or_else(|| "The matrix correspondence is unavailable".to_owned())?;
    if matrix_correspondences.next().is_some()
        || matrix_correspondence.source_id != projection.source_matrix_id
        || matrix_correspondence.mirror_id != projection.mirror_matrix_id
    {
        return Err("The matrix correspondence is ambiguous or does not match the input".into());
    }

    let source_matrix = matrix_by_id(document, &projection.source_matrix_id)?;
    let mirror_matrix = matrix_by_id(document, &projection.mirror_matrix_id)?;
    if source_matrix.board_id.as_deref() != Some(halves.source_board_id.as_str())
        || mirror_matrix.board_id.as_deref() != Some(halves.mirror_board_id.as_str())
    {
        return Err(
            "The matrix correspondence does not belong to its source and mirror boards".into(),
        );
    }

    for id in &source_matrix.part_ids {
        part_by_id(document, id)?;
        if owner_for(document, id)? != halves.source_board_id {
            return Err(format!(
                "Source matrix part {id:?} has an unexpected board owner"
            ));
        }
    }
    for id in &mirror_matrix.part_ids {
        part_by_id(document, id)?;
        if owner_for(document, id)? != halves.mirror_board_id {
            return Err(format!(
                "Mirror matrix part {id:?} has an unexpected board owner"
            ));
        }
    }

    for part in &projection.parts {
        let mut correspondences = halves.correspondences.iter().filter(|correspondence| {
            correspondence.kind == MirroredItemKind::Part
                && correspondence.source_id == part.source_id
                && correspondence.mirror_id == part.mirror_id
        });
        if correspondences.next().is_none() || correspondences.next().is_some() {
            return Err(format!(
                "The part correspondence {:?} → {:?} is missing or ambiguous",
                part.source_id, part.mirror_id
            ));
        }
        if !source_matrix.part_ids.contains(&part.source_id)
            || !mirror_matrix.part_ids.contains(&part.mirror_id)
        {
            return Err("A projected part does not belong to the selected matrices".into());
        }
        part_by_id(document, &part.source_id)?;
        part_by_id(document, &part.mirror_id)?;
        if owner_for(document, &part.source_id)? != halves.source_board_id
            || owner_for(document, &part.mirror_id)? != halves.mirror_board_id
        {
            return Err("A projected part has an unexpected board owner".into());
        }
    }
    for id in &projection.locally_protected_mirror_parts {
        if !mirror_matrix.part_ids.contains(id) {
            return Err(format!(
                "Locally protected mirror part {id:?} is not a matrix member"
            ));
        }
        part_by_id(document, id)?;
        if owner_for(document, id)? != halves.mirror_board_id {
            return Err(format!(
                "Locally protected mirror part {id:?} has an unexpected owner"
            ));
        }
        if projection.parts.iter().any(|part| part.mirror_id == *id) {
            return Err(format!(
                "Mirror part {id:?} is both linked and locally protected"
            ));
        }
    }
    for (id, expected_owner) in &projection.final_owners {
        if owner_for(document, id)? != expected_owner {
            return Err(format!("The final owner input for part {id:?} is stale"));
        }
    }

    // This forward slice supports the reviewed unsplayed 1x2 matrix with one linked
    // member and one locally retained X-mirrored member. Reject other shapes before
    // calling the materializer so its cleanup cannot partially affect the document.
    let linked_source_id = format!("matrix/{}/r0c1", source_matrix.id);
    let linked_mirror_id = format!("matrix/{}/r0c1", mirror_matrix.id);
    let retained_mirror_id = format!("matrix/{}/r0c0", mirror_matrix.id);
    if projection.parts.len() != 1
        || projection.parts[0].source_id != linked_source_id
        || projection.parts[0].mirror_id != linked_mirror_id
        || projection.locally_protected_mirror_parts.len() != 1
        || !projection
            .locally_protected_mirror_parts
            .contains(&retained_mirror_id)
    {
        return Err("The reconciliation input has an unsupported matrix membership shape".into());
    }
    let simple_matrix = |matrix: &Matrix| {
        let coordinates: std::collections::BTreeSet<_> = matrix
            .cells
            .iter()
            .map(|cell| (cell.row, cell.column))
            .collect();
        let expected_part_ids = std::collections::BTreeSet::from([
            format!("matrix/{}/r0c0", matrix.id),
            format!("matrix/{}/r0c1", matrix.id),
        ]);
        matrix.rows == 1
            && matrix.columns == 2
            && matrix.cells.len() == 2
            && coordinates == std::collections::BTreeSet::from([(0, 0), (0, 1)])
            && matrix.part_ids.len() == 2
            && matrix
                .part_ids
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                == expected_part_ids
            && matrix.row_offsets.is_empty()
            && matrix.column_offsets.is_empty()
            && matrix.column_staggers.is_empty()
            && matrix.column_splays.is_empty()
            && matrix.column_origins.is_empty()
            && matrix.cells.iter().all(|cell| {
                cell.enabled
                    && !cell.deleted
                    && cell.definition_id.is_none()
                    && cell.variant.is_none()
                    && cell.assemblies.is_empty()
                    && cell.assemblies_local.is_none()
            })
    };
    if !simple_matrix(source_matrix) || !simple_matrix(mirror_matrix) {
        return Err("The reconciliation input has an unsupported matrix layout".into());
    }

    let projected_matrix = project_matrix(
        source_matrix,
        mirror_matrix,
        matrix_correspondence,
        pair.mapping,
    )?;
    let mirror_axis = format!("{:?}", projected_matrix.mirror).to_ascii_lowercase();
    if projected_matrix.mirror.is_none() || !mirror_axis.contains('x') {
        return Err("The reconciliation input does not use the supported X mirror axis".into());
    }
    let retained = part_by_id(document, &retained_mirror_id)?;
    let matrix_rotation = projected_matrix.rotation.unwrap_or(0.0);
    let radians = matrix_rotation.to_radians();
    let dx = retained.pose.at.x - projected_matrix.origin.x;
    let dy = retained.pose.at.y - projected_matrix.origin.y;
    // Invert origin + R(rotation) * MirrorX * cell_position for cell r0c0.
    let retained_offset = Vec2 {
        x: -(radians.cos() * dx + radians.sin() * dy),
        y: -radians.sin() * dx + radians.cos() * dy,
    };
    let mut retained_cell = projected_matrix
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?
        .clone();
    retained_cell.offset = Some(retained_offset);
    retained_cell.rotation = Some(retained.pose.rotation - matrix_rotation);

    // Materialize the linked neighbor while excluding the retained ID from the
    // materializer's old generated-part ownership view. Its Part and references stay
    // in the staged document; the final matrix cell is installed after normalization.
    let mut incoming_matrix = projected_matrix.clone();
    let incoming_retained_cell = incoming_matrix
        .cells
        .iter_mut()
        .find(|cell| cell.row == 0 && cell.column == 0)
        .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?;
    incoming_retained_cell.enabled = false;
    incoming_matrix
        .part_ids
        .retain(|id| id != &retained_mirror_id);
    let mut staged = document.clone();
    let staged_old_matrix = staged
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == mirror_matrix.id)
        .ok_or_else(|| "The mirrored matrix disappeared from the staged document".to_owned())?;
    staged_old_matrix
        .part_ids
        .retain(|id| id != &retained_mirror_id);
    crate::matrix::set_matrix(&mut staged, &incoming_matrix)?;

    {
        let normalized_mirror_matrix = staged
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == mirror_matrix.id)
            .ok_or_else(|| "The normalized mirrored matrix is unavailable".to_owned())?;
        let normalized_retained_cell = normalized_mirror_matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == 0 && cell.column == 0)
            .ok_or_else(|| "The normalized retained matrix cell is unavailable".to_owned())?;
        *normalized_retained_cell = retained_cell;
        let mut ordered_cells: Vec<_> = normalized_mirror_matrix.cells.iter().collect();
        ordered_cells.sort_by_key(|cell| (cell.row, cell.column));
        normalized_mirror_matrix.part_ids = ordered_cells
            .into_iter()
            .filter(|cell| cell.enabled && !cell.deleted)
            .map(|cell| format!("matrix/{}/r{}c{}", mirror_matrix.id, cell.row, cell.column))
            .collect();
    }
    let final_matrix = matrix_by_id(&staged, &projection.mirror_matrix_id)?.clone();
    crate::matrix::valid_matrix(&final_matrix, &staged)?;
    for id in &final_matrix.part_ids {
        if owner_for(&staged, id)? != halves.mirror_board_id {
            return Err(format!(
                "Projected mirror matrix part {id:?} has an unexpected owner"
            ));
        }
    }
    crate::mechanical::features::validate(&staged)?;
    crate::mechanical::halves::validate_document(&staged)?;

    let mut candidates: std::collections::BTreeSet<String> =
        mirror_matrix.part_ids.iter().cloned().collect();
    candidates.extend(final_matrix.part_ids.iter().cloned());
    candidates.insert(mirror_matrix.id.clone());
    let mut changed_ids = std::collections::BTreeSet::new();
    if snapshot(mirror_matrix)? != snapshot(&final_matrix)? {
        changed_ids.insert(mirror_matrix.id.clone());
    }
    for id in candidates {
        if id == mirror_matrix.id {
            continue;
        }
        let before = document.parts.iter().find(|part| part.id == id);
        let after = staged.parts.iter().find(|part| part.id == id);
        let before = before.map(snapshot).transpose()?;
        let after = after.map(snapshot).transpose()?;
        if before != after {
            changed_ids.insert(id);
        }
    }

    *document = staged;
    Ok(changed_ids.into_iter().collect())
}

/// Project one source matrix into its already assigned mirror identity.
///
/// Production dispatch stays dormant until the verified slices are activated together.
#[allow(dead_code)] // Ticket 111 owns activation after the reconciliation slices are verified.
fn project_matrix(
    source: &Matrix,
    mirror: &Matrix,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
) -> Result<Matrix, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Matrix
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The matrix correspondence does not match its source and mirror".into());
    }

    crate::matrix::layout::reflected(source, mirror, mapping, false)
}

/// Project one source part through its established correspondence, retaining target identifiers.
#[allow(dead_code)] // Ticket 111 owns production activation after the reconciliation slices are verified.
fn project_part(
    source: &Part,
    mirror: &Part,
    correspondence: &MirroredCorrespondence,
    mapping: Pose2,
) -> Result<Part, String> {
    if correspondence.kind != crate::model::MirroredItemKind::Part
        || correspondence.source_id != source.id
        || correspondence.mirror_id != mirror.id
    {
        return Err("The part correspondence does not match its source and mirror".into());
    }

    Ok(super::derive::part(
        source,
        &mirror.id,
        &mirror.definition_id,
        &mirror.reference,
        mapping,
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{PartProjection, Projection, reconcile};
    use crate::mirrored_half::{
        MechanicalChange, MechanicalEdit, MechanicalFeatureCorrespondence, MechanicalTarget,
    };
    use crate::model::*;

    fn fixture_mapping() -> Pose2 {
        Pose2 {
            at: Vec2 { x: 80.0, y: -25.0 },
            rotation: 31.0,
        }
    }

    fn fixture_board(id: &str) -> Board {
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

    fn fixture_mechanical_configuration(board_id: &str) -> MechanicalConfiguration {
        let mut configuration: MechanicalConfiguration =
            serde_json::from_value(serde_json::json!({
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

    fn fixture_instance(id: &str, board_id: &str) -> PhysicalBoardInstance {
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

    fn fixture_matrix(id: &str, board_id: &str) -> Matrix {
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
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: (0..2)
                .map(|column| MatrixCell {
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

    fn fixture_mounting_hole() -> PartDefinition {
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

    fn fixture_document() -> Result<ProjectDoc, String> {
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

        crate::matrix::set_matrix(
            &mut document,
            &fixture_matrix("source-matrix", source_board),
        )?;
        crate::matrix::set_matrix(
            &mut document,
            &fixture_matrix("mirror-matrix", mirror_board),
        )?;

        crate::mechanical::edit::apply(
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

    #[test]
    fn valid_pair_fixture_materializes_both_matrices_and_passes_validators() {
        let document = fixture_document().unwrap();

        assert_eq!(document.matrices.len(), 2);
        for matrix_id in ["source-matrix", "mirror-matrix"] {
            let matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == matrix_id)
                .unwrap();
            assert_eq!(matrix.part_ids.len(), 2);
            assert!(
                matrix
                    .part_ids
                    .iter()
                    .all(|id| document.parts.iter().any(|part| &part.id == id))
            );
        }
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/source-matrix/r0c0")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/source-matrix/r0c1")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/mirror-matrix/r0c0")
        );
        assert!(
            document
                .parts
                .iter()
                .any(|part| part.id == "matrix/mirror-matrix/r0c1")
        );
        crate::mechanical::features::validate(&document).unwrap();
        crate::mechanical::halves::validate_document(&document).unwrap();
    }

    #[test]
    fn changed_source_matrix_projects_linked_neighbor_after_entry_snapshot() {
        let mut document = fixture_document().unwrap();
        let mut moved_source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        moved_source.origin = Vec2 { x: 27.0, y: -14.0 };
        moved_source.rotation = Some(43.0);
        crate::matrix::set_matrix(&mut document, &moved_source).unwrap();

        let definition = document
            .definitions
            .iter_mut()
            .find(|definition| definition.id == "switch")
            .unwrap();
        definition.pads = vec![
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2 { x: 0.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
            crate::model::Pad {
                id: "pad-two".into(),
                number: "2".into(),
                at: Vec2 { x: 1.0, y: 0.0 },
                size: Vec2 { x: 1.0, y: 1.0 },
                shape: crate::model::PadShape::Circle,
                drill: None,
                plated: None,
                side: None,
                rotation: None,
                net_id: None,
            },
        ];

        let probe_id = "projection-probe";
        let mut probe = document
            .parts
            .iter()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1")
            .unwrap()
            .clone();
        probe.id = probe_id.into();
        probe.reference = "PROBE".into();
        probe.pose.at = Vec2 { x: 4.0, y: 2.0 };
        probe.pose.rotation = 0.0;
        document.parts.push(probe);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(probe_id.into());

        let retained_net_id = "retained-two-pin-net";
        document.nets.push(crate::model::Net {
            id: retained_net_id.into(),
            name: "RETAINED".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r0c0".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
                    part_id: probe_id.into(),
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
            .push(retained_net_id.into());

        let retained_offset = crate::model::Constraint::Offset {
            id: "retained-stationary-offset".into(),
            source_part_id: probe_id.into(),
            target_part_id: "matrix/mirror-matrix/r0c0".into(),
            offset: Vec2 { x: -4.0, y: -2.0 },
            rotation: 0.0,
        };
        document.constraints.push(retained_offset.clone());

        let retained_envelope_id = "retained-global-envelope";
        document
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
                connections: vec![],
                settings: Default::default(),
                id: retained_envelope_id.into(),
                part_ids: vec![
                    "matrix/mirror-matrix/r0c0".into(),
                    "matrix/mirror-matrix/r0c1".into(),
                    probe_id.into(),
                ],
                margin: 2.0,
                operation: crate::model::Operation::Add,
            });
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .outline_ids
            .push(retained_envelope_id.into());

        let assert_retained_net_is_valid = |document: &ProjectDoc| {
            let net = document
                .nets
                .iter()
                .find(|net| net.id == retained_net_id)
                .unwrap();
            assert_eq!(
                net.pins,
                vec![
                    crate::model::Pin {
                        part_id: "matrix/mirror-matrix/r0c0".into(),
                        pad_id: "pad-one".into(),
                    },
                    crate::model::Pin {
                        part_id: probe_id.into(),
                        pad_id: "pad-two".into(),
                    },
                ]
            );
            let owner = document
                .boards
                .iter()
                .find(|board| board.net_ids.contains(&net.id))
                .unwrap();
            assert_eq!(owner.id, "mirror-board");
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.net_ids.contains(&net.id))
                    .count(),
                1
            );
            for pin in &net.pins {
                assert!(owner.part_ids.contains(&pin.part_id));
                let part = document
                    .parts
                    .iter()
                    .find(|part| part.id == pin.part_id)
                    .unwrap();
                let definition = document
                    .definitions
                    .iter()
                    .find(|definition| definition.id == part.definition_id)
                    .unwrap();
                assert!(
                    definition
                        .pads
                        .iter()
                        .any(|pad| pad.id == pin.pad_id && !pad.id.is_empty())
                );
            }

            let retained_mirror = document
                .parts
                .iter()
                .find(|part| part.id == "matrix/mirror-matrix/r0c0")
                .unwrap();
            assert_eq!(
                retained_mirror.pose,
                crate::model::Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                }
            );
            let probe = document
                .parts
                .iter()
                .find(|part| part.id == probe_id)
                .unwrap();
            assert_eq!(
                probe.pose,
                crate::model::Pose2 {
                    at: Vec2 { x: 4.0, y: 2.0 },
                    rotation: 0.0,
                }
            );

            let envelope = document
                .outline
                .iter()
                .find(|feature| {
                    matches!(feature, crate::model::OutlineFeature::PartEnvelope { id, .. } if id == retained_envelope_id)
                })
                .unwrap();
            let crate::model::OutlineFeature::PartEnvelope { part_ids, .. } = envelope else {
                unreachable!("the retained feature was found as a part envelope");
            };
            assert_eq!(
                part_ids,
                &vec![
                    "matrix/mirror-matrix/r0c0".to_owned(),
                    "matrix/mirror-matrix/r0c1".to_owned(),
                    probe_id.to_owned(),
                ]
            );
            assert_eq!(
                document
                    .boards
                    .iter()
                    .filter(|board| board.outline_ids.contains(&retained_envelope_id.to_owned()))
                    .map(|board| board.id.as_str())
                    .collect::<Vec<_>>(),
                vec!["mirror-board"]
            );
            assert!(
                document
                    .boards
                    .iter()
                    .find(|board| board.id == "source-board")
                    .unwrap()
                    .net_ids
                    .iter()
                    .all(|net_id| net_id != retained_net_id)
            );
        };
        assert_retained_net_is_valid(&document);
        crate::mechanical::halves::validate_document(&document).unwrap();

        // The entry snapshot is deliberately taken after the source's ordinary edit/materialization.
        let entry = document.clone();
        let entry_source_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        let entry_mirror_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap()
            .clone();
        let matrix_correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: entry_source_matrix.id.clone(),
            mirror_id: entry_mirror_matrix.id.clone(),
        };
        let expected_mirror_matrix = project_matrix(
            &entry_source_matrix,
            &entry_mirror_matrix,
            &matrix_correspondence,
            fixture_mapping(),
        )
        .unwrap();
        let mut expected_document = entry.clone();
        crate::matrix::set_matrix(&mut expected_document, &expected_mirror_matrix).unwrap();

        let protected_local_id = "matrix/mirror-matrix/r0c0";
        let linked_source_id = "matrix/source-matrix/r0c1";
        let linked_mirror_id = "matrix/mirror-matrix/r0c1";
        let entry_probe = entry
            .parts
            .iter()
            .find(|part| part.id == probe_id)
            .unwrap()
            .clone();
        let entry_probe_owner = entry
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&probe_id.to_owned()))
            .unwrap()
            .id
            .clone();
        let entry_local = entry
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();

        let mut final_owners = BTreeMap::new();
        for id in [
            "matrix/source-matrix/r0c0",
            linked_source_id,
            protected_local_id,
            linked_mirror_id,
        ] {
            let owner = entry
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .map(|board| board.id.clone())
                .unwrap();
            final_owners.insert(id.to_owned(), owner);
        }
        let projection = Projection {
            source_board_id: "source-board".into(),
            mirror_board_id: "mirror-board".into(),
            source_matrix_id: entry_source_matrix.id.clone(),
            mirror_matrix_id: entry_mirror_matrix.id.clone(),
            mapping: fixture_mapping(),
            parts: vec![PartProjection {
                source_id: linked_source_id.into(),
                mirror_id: linked_mirror_id.into(),
            }],
            locally_protected_mirror_parts: BTreeSet::from([protected_local_id.into()]),
            final_owners: final_owners.clone(),
        };

        let entry_source_matrix_json = serde_json::to_value(&entry_source_matrix).unwrap();
        let entry_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
            .into_iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == id).unwrap())
                    .unwrap()
            })
            .collect();
        let entry_mirror_ids = entry_mirror_matrix.part_ids.clone();
        let entry_local_json = serde_json::to_value(entry_local).unwrap();
        let protected_references = serde_json::json!({
            "boards": &entry.boards,
            "nets": &entry.nets,
            "constraints": &entry.constraints,
            "boardOutlines": &entry.board_outlines,
            "definitions": &entry.definitions,
            "outline": &entry.outline,
        });
        let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();

        assert_retained_net_is_valid(&document);
        crate::mechanical::halves::validate_document(&document).unwrap();
        assert_eq!(
            document.constraints.iter().find(|constraint| {
                matches!(constraint, crate::model::Constraint::Offset { id, .. } if id == "retained-stationary-offset")
            }),
            Some(&retained_offset)
        );
        let actual_probe = document
            .parts
            .iter()
            .find(|part| part.id == probe_id)
            .unwrap();
        assert_eq!(actual_probe, &entry_probe);
        let actual_probe_owner = document
            .boards
            .iter()
            .find(|board| board.part_ids.contains(&probe_id.to_owned()))
            .unwrap()
            .id
            .clone();
        assert_eq!(actual_probe_owner, entry_probe_owner);
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == entry_probe_owner)
                .unwrap()
                .part_ids
                .contains(&probe_id.to_owned())
        );

        let actual_neighbor = document
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        assert!(
            (actual_neighbor.pose.at.x - 45.48221151584141).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected x position"
        );
        assert!(
            (actual_neighbor.pose.at.y - -46.956048106863605).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected y position"
        );
        assert!(
            (actual_neighbor.pose.rotation - -12.0).abs() < 1e-9,
            "the linked neighbor must receive the independently calculated reflected heading"
        );

        assert_eq!(
            serde_json::to_value(
                document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == "source-matrix")
                    .unwrap()
            )
            .unwrap(),
            entry_source_matrix_json
        );
        let actual_source_parts_json: Vec<_> = ["matrix/source-matrix/r0c0", linked_source_id]
            .into_iter()
            .map(|id| {
                serde_json::to_value(document.parts.iter().find(|part| part.id == id).unwrap())
                    .unwrap()
            })
            .collect();
        assert_eq!(actual_source_parts_json, entry_source_parts_json);
        let actual_mirror_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(actual_mirror_matrix.id, entry_mirror_matrix.id);
        assert_eq!(actual_mirror_matrix.part_ids, entry_mirror_ids);
        let mut rematerialized = entry.clone();
        crate::matrix::set_matrix(&mut rematerialized, actual_mirror_matrix).unwrap();
        let rematerialized_matrix = rematerialized
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual_mirror_matrix).unwrap(),
            serde_json::to_value(rematerialized_matrix).unwrap(),
            "the final active matrix must already be a complete normalized materializer input"
        );
        let actual_retained = document
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();
        let rematerialized_retained = rematerialized
            .parts
            .iter()
            .find(|part| part.id == protected_local_id)
            .unwrap();
        for pose in [&actual_retained.pose, &rematerialized_retained.pose] {
            assert!((pose.at.x - 0.0).abs() < 1e-9);
            assert!((pose.at.y - 0.0).abs() < 1e-9);
            assert!((pose.rotation - 0.0).abs() < 1e-9);
        }
        let mut actual_non_pose = serde_json::to_value(actual_retained).unwrap();
        let mut rematerialized_non_pose = serde_json::to_value(rematerialized_retained).unwrap();
        actual_non_pose.as_object_mut().unwrap().remove("pose");
        rematerialized_non_pose
            .as_object_mut()
            .unwrap()
            .remove("pose");
        assert_eq!(
            actual_non_pose, rematerialized_non_pose,
            "the retained member's identity and non-pose data must agree with rematerialization"
        );
        assert_eq!(
            serde_json::to_value(
                document
                    .parts
                    .iter()
                    .find(|part| part.id == protected_local_id)
                    .unwrap()
            )
            .unwrap(),
            entry_local_json
        );
        assert_eq!(
            serde_json::json!({
                "boards": &document.boards,
                "nets": &document.nets,
                "constraints": &document.constraints,
                "boardOutlines": &document.board_outlines,
                "definitions": &document.definitions,
                "outline": &document.outline,
            }),
            protected_references
        );

        let actual_final_owners: BTreeMap<_, _> = final_owners
            .keys()
            .map(|id| {
                let owner = document
                    .boards
                    .iter()
                    .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                    .map(|board| board.id.clone())
                    .unwrap();
                (id.clone(), owner)
            })
            .collect();
        assert_eq!(actual_final_owners, final_owners);

        let mut expected_changed_ids = BTreeSet::new();
        if serde_json::to_value(&expected_mirror_matrix).unwrap()
            != serde_json::to_value(&entry_mirror_matrix).unwrap()
        {
            expected_changed_ids.insert(entry_mirror_matrix.id.clone());
        }
        let before = entry
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        let after = expected_document
            .parts
            .iter()
            .find(|part| part.id == linked_mirror_id)
            .unwrap();
        if serde_json::to_value(before).unwrap() != serde_json::to_value(after).unwrap() {
            expected_changed_ids.insert(linked_mirror_id.into());
        }
        assert_eq!(
            changed_ids,
            expected_changed_ids.into_iter().collect::<Vec<_>>()
        );
        let before_second_reconcile = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, before_second_reconcile);

        let before_rejected = serde_json::to_value(&document).unwrap();
        let unsupported = Projection {
            source_board_id: projection.source_board_id.clone(),
            mirror_board_id: projection.mirror_board_id.clone(),
            source_matrix_id: projection.source_matrix_id.clone(),
            mirror_matrix_id: projection.mirror_matrix_id.clone(),
            mapping: projection.mapping,
            parts: vec![],
            locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
            final_owners: projection.final_owners.clone(),
        };
        assert!(reconcile(&mut document, "pair-source-mirror", &unsupported).is_err());
        assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);

        let mismatched_pair = Projection {
            mapping: Pose2 {
                at: projection.mapping.at,
                rotation: projection.mapping.rotation + 1.0,
            },
            source_board_id: projection.source_board_id.clone(),
            mirror_board_id: projection.mirror_board_id.clone(),
            source_matrix_id: projection.source_matrix_id.clone(),
            mirror_matrix_id: projection.mirror_matrix_id.clone(),
            parts: projection
                .parts
                .iter()
                .map(|part| PartProjection {
                    source_id: part.source_id.clone(),
                    mirror_id: part.mirror_id.clone(),
                })
                .collect(),
            locally_protected_mirror_parts: projection.locally_protected_mirror_parts.clone(),
            final_owners: projection.final_owners.clone(),
        };
        assert!(reconcile(&mut document, "pair-source-mirror", &mismatched_pair).is_err());
        assert_eq!(serde_json::to_value(&document).unwrap(), before_rejected);
    }

    use super::{project_matrix, project_part};
    use crate::model::Part;
    use crate::model::{Matrix, MirroredCorrespondence, MirroredItemKind, Pose2, Vec2};

    fn matrix(id: &str, board_id: &str, rows: u32, columns: u32, origin: Vec2) -> Matrix {
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
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![],
        }
    }

    #[test]
    fn changed_source_matrix_reprojects_existing_mirror_geometry_and_keeps_its_identity() {
        let source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 20.0, y: 7.0 });
        let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_matrix(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .expect("the existing matrix correspondence can be projected");

        assert_eq!(projected.id, "right-matrix");
        assert_eq!(projected.board_id.as_deref(), Some("right-board"));
        assert_eq!(projected.rows, 2);
        assert_eq!(projected.columns, 3);
        assert_eq!(projected.origin, Vec2 { x: 180.0, y: 7.0 });
    }

    #[test]
    fn a_rotated_pair_map_reprojects_matrix_origin_and_heading_once() {
        let mut source = matrix("left-matrix", "left-board", 2, 3, Vec2 { x: 5.0, y: 2.0 });
        source.rotation = Some(15.0);
        let mirror = matrix("right-matrix", "right-board", 1, 1, Vec2 { x: 5.0, y: 1.0 });
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Matrix,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_matrix(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 20.0, y: -3.0 },
                rotation: 90.0,
            },
        )
        .expect("the existing matrix correspondence can be projected");

        assert_eq!(projected.id, "right-matrix");
        assert_eq!(projected.rows, 2);
        assert_eq!(projected.columns, 3);
        assert_eq!(projected.origin, Vec2 { x: 18.0, y: -8.0 });
        assert_eq!(projected.rotation, Some(75.0));
    }

    #[test]
    fn changed_source_part_pose_keeps_existing_mirror_identity_and_source_parameters() {
        let source = part(
            "left/part",
            "source-definition",
            "U1",
            Pose2 {
                at: Vec2 { x: 20.0, y: 7.0 },
                rotation: 35.0,
            },
            "back",
            9,
        );
        let mirror = part(
            "right/part",
            "mirror-definition",
            "U9",
            Pose2 {
                at: Vec2 { x: 5.0, y: 1.0 },
                rotation: 0.0,
            },
            "front",
            2,
        );
        let correspondence = MirroredCorrespondence {
            companion_mode: None,
            companion_tweak: None,
            kind: MirroredItemKind::Part,
            source_id: source.id.clone(),
            mirror_id: mirror.id.clone(),
        };

        let projected = project_part(
            &source,
            &mirror,
            &correspondence,
            Pose2 {
                at: Vec2 { x: 200.0, y: 0.0 },
                rotation: 0.0,
            },
        )
        .expect("the existing part correspondence can be projected");

        assert_eq!(projected.id, "right/part");
        assert_eq!(projected.definition_id, "mirror-definition");
        assert_eq!(projected.reference, "U9");
        assert_eq!(
            projected.pose,
            Pose2 {
                at: Vec2 { x: 180.0, y: 7.0 },
                rotation: 145.0,
            }
        );
        assert_eq!(projected.side, source.side);
        assert_eq!(projected.generator_parameters, source.generator_parameters);
        assert_eq!(projected.properties, source.properties);
    }

    fn part(
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
}
