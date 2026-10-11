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

struct RetainedCell {
    member_ids: Vec<String>,
    cell: crate::model::MatrixCell,
}

struct FixedMembershipView {
    incoming_matrix: Matrix,
    retained_cells: Vec<RetainedCell>,
}

fn build_fixed_membership_view(
    document: &ProjectDoc,
    halves: &crate::model::MirroredHalves,
    source: &Matrix,
    mirror: &Matrix,
    projected: &Matrix,
    projection: &Projection,
) -> Result<FixedMembershipView, String> {
    type Slot = (u32, u32);
    type SlotIds = std::collections::BTreeMap<Slot, String>;
    let mut adjusted_projected = projected.clone();

    fn point_is_finite(point: Vec2) -> bool {
        point.x.is_finite() && point.y.is_finite()
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

    fn canonical_slots(matrix: &Matrix) -> Result<SlotIds, String> {
        if matrix.rows == 0 || matrix.columns == 0 {
            return Err("The reconciliation matrix has empty dimensions".into());
        }
        let count = matrix
            .rows
            .checked_mul(matrix.columns)
            .ok_or_else(|| "The reconciliation matrix dimensions overflow".to_owned())?;
        if matrix.cells.len() != count as usize {
            return Err("The reconciliation matrix does not have fixed active membership".into());
        }
        let mut slots = SlotIds::new();
        for cell in &matrix.cells {
            if !cell.enabled
                || cell.deleted
                || cell.row >= matrix.rows
                || cell.column >= matrix.columns
            {
                return Err(
                    "The reconciliation matrix does not have fixed active membership".into(),
                );
            }
            let id = format!("matrix/{}/r{}c{}", matrix.id, cell.row, cell.column);
            if slots.insert((cell.row, cell.column), id).is_some() {
                return Err("The reconciliation matrix contains aliased cells".into());
            }
        }
        if slots.len() != count as usize {
            return Err("The reconciliation matrix has incomplete active membership".into());
        }
        Ok(slots)
    }

    fn canonical_members(
        matrix: &Matrix,
        slots: &SlotIds,
    ) -> Result<std::collections::BTreeSet<String>, String> {
        let mut members: std::collections::BTreeSet<_> = slots.values().cloned().collect();
        for cell in &matrix.cells {
            let key_id = slots
                .get(&(cell.row, cell.column))
                .ok_or_else(|| "The reconciliation matrix has an unknown active slot".to_owned())?;
            let mut assembly_ids = std::collections::BTreeSet::new();
            for assembly in &cell.assemblies {
                if assembly.id.is_empty()
                    || !assembly_ids.insert(assembly.id.as_str())
                    || !assembly.offset.x.is_finite()
                    || !assembly.offset.y.is_finite()
                    || assembly
                        .rotation
                        .is_some_and(|rotation| !rotation.is_finite())
                {
                    return Err("The reconciliation matrix has invalid assembly membership".into());
                }
                let id = crate::matrix::companion_id(key_id, &assembly.id);
                if !members.insert(id) {
                    return Err("The reconciliation matrix has aliased assembly membership".into());
                }
            }
        }
        let actual: std::collections::BTreeSet<_> = matrix.part_ids.iter().cloned().collect();
        if actual.len() != matrix.part_ids.len() || actual != members {
            return Err("The reconciliation matrix membership is not canonical".into());
        }
        Ok(members)
    }

    if source.id == mirror.id
        || source.rows != mirror.rows
        || source.columns != mirror.columns
        || source.rows != projected.rows
        || source.columns != projected.columns
    {
        return Err("The source and mirror matrices do not have matching fixed dimensions".into());
    }
    let source_slots = canonical_slots(source)?;
    let mirror_slots = canonical_slots(mirror)?;
    let projected_slots = canonical_slots(projected)?;
    if source_slots.keys().ne(mirror_slots.keys()) || source_slots.keys().ne(projected_slots.keys())
    {
        return Err("The source and mirror matrices do not have matching active slots".into());
    }
    let source_members = canonical_members(source, &source_slots)?;
    let mirror_members = canonical_members(mirror, &mirror_slots)?;
    let projected_members = canonical_members(projected, &projected_slots)?;
    if source_members.len() != mirror_members.len()
        || source_members.len() != projected_members.len()
    {
        return Err("The source and mirror matrices do not have matching fixed members".into());
    }

    let mut linked_slots = std::collections::BTreeMap::<Slot, String>::new();
    let mut linked_source_ids = std::collections::BTreeSet::new();
    let mut linked_mirror_ids = std::collections::BTreeSet::new();
    for part in &projection.parts {
        let source_slot = source_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.source_id).then_some(*slot));
        let mirror_slot = mirror_slots
            .iter()
            .find_map(|(slot, id)| (id == &part.mirror_id).then_some(*slot));
        let Some(slot) = source_slot.filter(|slot| Some(*slot) == mirror_slot) else {
            return Err("The linked matrix associations are missing or inconsistent".into());
        };
        if !linked_source_ids.insert(part.source_id.clone())
            || !linked_mirror_ids.insert(part.mirror_id.clone())
            || linked_slots.insert(slot, part.mirror_id.clone()).is_some()
        {
            return Err("The linked matrix associations contain aliases".into());
        }
    }

    let mut retained_slots = std::collections::BTreeMap::<Slot, String>::new();
    for id in &projection.locally_protected_mirror_parts {
        let Some((slot, _)) = mirror_slots.iter().find(|(_, member)| *member == id) else {
            return Err(format!(
                "Locally protected mirror part {id:?} is not a matrix member"
            ));
        };
        if retained_slots.insert(*slot, id.clone()).is_some() {
            return Err("The retained matrix associations contain aliases".into());
        }
    }
    let partition_slots: std::collections::BTreeSet<_> = linked_slots
        .keys()
        .chain(retained_slots.keys())
        .copied()
        .collect();
    if linked_slots
        .keys()
        .any(|slot| retained_slots.contains_key(slot))
        || partition_slots != source_slots.keys().copied().collect()
        || projection.parts.len() != linked_slots.len()
        || projection.locally_protected_mirror_parts.len() != retained_slots.len()
        || linked_mirror_ids
            .iter()
            .any(|id| projection.locally_protected_mirror_parts.contains(id))
    {
        return Err(
            "Linked and retained matrix memberships must form a disjoint complete partition".into(),
        );
    }

    let affected_ids: std::collections::BTreeSet<_> =
        source_members.union(&mirror_members).cloned().collect();
    let owner_ids: std::collections::BTreeSet<_> =
        projection.final_owners.keys().cloned().collect();
    if owner_ids != affected_ids || projection.final_owners.len() != affected_ids.len() {
        return Err("The final owner input is incomplete for matrix membership".into());
    }
    for (slot, source_id) in &source_slots {
        let mirror_id = &mirror_slots[slot];
        let source_owner = owner_for(document, source_id)?;
        let mirror_owner = owner_for(document, mirror_id)?;
        if source_owner != halves.source_board_id
            || mirror_owner != halves.mirror_board_id
            || projection.final_owners.get(source_id).map(String::as_str) != Some(source_owner)
            || projection.final_owners.get(mirror_id).map(String::as_str) != Some(mirror_owner)
        {
            return Err("A matrix part has an incomplete or stale owner input".into());
        }
        for id in [source_id, mirror_id] {
            let mut matches = document.parts.iter().filter(|part| part.id == *id);
            let part = matches
                .next()
                .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
            if matches.next().is_some() {
                return Err(format!("The mirrored part {id:?} is ambiguous"));
            }
            if !point_is_finite(part.pose.at) || !part.pose.rotation.is_finite() {
                return Err(format!("The mirrored part {id:?} has non-finite geometry"));
            }
            if linked_slots.contains_key(slot)
                && crate::matrix::layout::companion_mode(document, id)
                    != crate::model::MatrixCompanionMode::Follows
            {
                return Err(
                    "Linked matrix keys cannot have authored or constrained placement".into(),
                );
            }
        }
        let mut associations = halves.correspondences.iter().filter(|association| {
            association.kind == MirroredItemKind::Part
                && (association.source_id == *source_id
                    || association.mirror_id == *mirror_id
                    || association.source_id == *mirror_id
                    || association.mirror_id == *source_id)
        });
        let association = associations
            .next()
            .ok_or_else(|| "A canonical matrix part association is missing".to_owned())?;
        if associations.next().is_some()
            || association.source_id != *source_id
            || association.mirror_id != *mirror_id
        {
            return Err("A canonical matrix part association is aliased or inconsistent".into());
        }
        if linked_slots.get(slot).is_some_and(|id| id == mirror_id)
            == retained_slots.get(slot).is_some_and(|id| id == mirror_id)
        {
            return Err("A canonical matrix slot must be linked or retained exactly once".into());
        }

        let source_cell = source
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A source matrix cell is unavailable".to_owned())?;
        let mirror_cell = mirror
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A mirror matrix cell is unavailable".to_owned())?;
        let projected_cell = projected
            .cells
            .iter()
            .find(|cell| (cell.row, cell.column) == *slot)
            .ok_or_else(|| "A projected matrix cell is unavailable".to_owned())?;
        let source_assemblies: std::collections::BTreeMap<_, _> = source_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        let mirror_assemblies: std::collections::BTreeMap<_, _> = mirror_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        let projected_assemblies: std::collections::BTreeMap<_, _> = projected_cell
            .assemblies
            .iter()
            .map(|assembly| (assembly.id.as_str(), assembly))
            .collect();
        if source_assemblies.len() != source_cell.assemblies.len()
            || mirror_assemblies.len() != mirror_cell.assemblies.len()
            || projected_assemblies.len() != projected_cell.assemblies.len()
            || source_assemblies.keys().ne(mirror_assemblies.keys())
            || source_assemblies.keys().ne(projected_assemblies.keys())
        {
            return Err(
                "The linked matrix assemblies do not have matching fixed membership".into(),
            );
        }
        let source_key = document
            .parts
            .iter()
            .find(|part| part.id == *source_id)
            .ok_or_else(|| format!("The mirrored part {source_id:?} is unavailable"))?;
        for assembly_id in source_assemblies.keys() {
            let source_assembly = source_assemblies[assembly_id];
            let mirror_assembly = mirror_assemblies[assembly_id];
            if source_assembly.definition_id != mirror_assembly.definition_id
                || source_assembly.side != mirror_assembly.side
            {
                return Err("The linked matrix assemblies have inconsistent definitions".into());
            }
            let source_companion_id = crate::matrix::companion_id(source_id, assembly_id);
            let mirror_companion_id = crate::matrix::companion_id(mirror_id, assembly_id);
            let mut matches = document
                .parts
                .iter()
                .filter(|part| part.id == source_companion_id);
            let source_companion = matches.next().ok_or_else(|| {
                format!("The mirrored part {source_companion_id:?} is unavailable")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "The mirrored part {source_companion_id:?} is ambiguous"
                ));
            }
            let mut matches = document
                .parts
                .iter()
                .filter(|part| part.id == mirror_companion_id);
            let mirror_companion = matches.next().ok_or_else(|| {
                format!("The mirrored part {mirror_companion_id:?} is unavailable")
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "The mirrored part {mirror_companion_id:?} is ambiguous"
                ));
            }
            let source_owner = owner_for(document, &source_companion_id)?;
            let mirror_owner = owner_for(document, &mirror_companion_id)?;
            if source_owner != halves.source_board_id
                || mirror_owner != halves.mirror_board_id
                || projection
                    .final_owners
                    .get(&source_companion_id)
                    .map(String::as_str)
                    != Some(source_owner)
                || projection
                    .final_owners
                    .get(&mirror_companion_id)
                    .map(String::as_str)
                    != Some(mirror_owner)
            {
                return Err("A matrix companion has an incomplete or stale owner input".into());
            }
            for part in [source_companion, mirror_companion] {
                if !point_is_finite(part.pose.at) || !part.pose.rotation.is_finite() {
                    return Err(format!(
                        "The mirrored part {:?} has non-finite geometry",
                        part.id
                    ));
                }
            }
            let mut companion_associations = halves.correspondences.iter().filter(|association| {
                association.kind == MirroredItemKind::Part
                    && (association.source_id == source_companion_id
                        || association.mirror_id == mirror_companion_id
                        || association.source_id == mirror_companion_id
                        || association.mirror_id == source_companion_id)
            });
            let association = companion_associations
                .next()
                .ok_or_else(|| "A canonical matrix companion association is missing".to_owned())?;
            if companion_associations.next().is_some()
                || association.source_id != source_companion_id
                || association.mirror_id != mirror_companion_id
            {
                return Err(
                    "A canonical matrix companion association is aliased or inconsistent".into(),
                );
            }
            crate::mechanical::halves::validate_companion(
                association.companion_mode.unwrap_or(halves.companion_mode),
                association.companion_tweak,
            )?;
            if linked_slots.contains_key(slot)
                && (crate::matrix::layout::companion_mode(document, &source_companion_id)
                    != crate::model::MatrixCompanionMode::Follows
                    || crate::matrix::layout::companion_mode(document, &mirror_companion_id)
                        != crate::model::MatrixCompanionMode::Follows)
            {
                return Err(
                    "Linked matrix companions cannot have authored or constrained placement".into(),
                );
            }
            if !point_is_finite(source_assembly.offset)
                || !point_is_finite(mirror_assembly.offset)
                || source_assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
                || mirror_assembly
                    .rotation
                    .is_some_and(|rotation| !rotation.is_finite())
            {
                return Err(
                    "The linked matrix assembly placement contains non-finite values".into(),
                );
            }
            if !linked_slots.contains_key(slot) {
                continue;
            }
            let projected_assembly = projected_cell
                .assemblies
                .iter()
                .find(|assembly| assembly.id == *assembly_id)
                .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
            let derived =
                super::derive::companion_assembly(super::derive::CompanionAssemblyInput {
                    offset: projected_assembly.offset,
                    rotation: projected_assembly.rotation,
                    key: Some(source_key),
                    companion: Some(source_companion),
                    mapping: projection.mapping,
                    mode: association.companion_mode.unwrap_or(halves.companion_mode),
                    tweak: association.companion_tweak,
                    stabiliser: super::SYMMETRIC_DEFINITIONS
                        .contains(&source_assembly.definition_id.as_str()),
                    fallback: false,
                })
                .map_err(str::to_owned)?;
            if !point_is_finite(derived.0)
                || derived.1.is_some_and(|rotation| !rotation.is_finite())
            {
                return Err("The projected matrix assembly contains non-finite values".into());
            }
            let projected_assembly = adjusted_projected
                .cells
                .iter_mut()
                .find(|cell| (cell.row, cell.column) == *slot)
                .and_then(|cell| {
                    cell.assemblies
                        .iter_mut()
                        .find(|assembly| assembly.id == *assembly_id)
                })
                .ok_or_else(|| "A projected matrix assembly is unavailable".to_owned())?;
            projected_assembly.offset = derived.0;
            projected_assembly.rotation = derived.1;
        }
    }

    if source.cells.iter().chain(&mirror.cells).any(|cell| {
        cell.definition_id.is_some() || cell.variant.is_some() || cell.assemblies_local.is_some()
    }) {
        return Err("The reconciliation matrix uses an unsupported cell representation".into());
    }
    let finite_matrix = |matrix: &Matrix| {
        point_is_finite(matrix.origin)
            && point_is_finite(matrix.pitch)
            && matrix.rotation.is_none_or(f64::is_finite)
            && matrix
                .row_offsets
                .iter()
                .chain(&matrix.column_offsets)
                .chain(matrix.column_origins.iter().flatten())
                .all(|point| point_is_finite(*point))
            && matrix
                .column_staggers
                .iter()
                .chain(&matrix.column_splays)
                .all(|angle| angle.is_finite())
            && matrix.cells.iter().all(|cell| {
                cell.offset.is_none_or(point_is_finite) && cell.rotation.is_none_or(f64::is_finite)
            })
    };
    if !finite_matrix(source) || !finite_matrix(mirror) || !finite_matrix(&adjusted_projected) {
        return Err("The reconciliation matrix geometry contains non-finite values".into());
    }

    let mirror_axis = adjusted_projected
        .mirror
        .unwrap_or(crate::model::Mirror::None);
    let matrix_rotation = adjusted_projected.rotation.unwrap_or(0.0);
    let mut projected_key_poses = std::collections::BTreeMap::new();
    for cell in &adjusted_projected.cells {
        let row = cell.row as usize;
        let column = cell.column as usize;
        let row_offset = adjusted_projected
            .row_offsets
            .get(row)
            .copied()
            .unwrap_or_default();
        let column_offset = adjusted_projected
            .column_offsets
            .get(column)
            .copied()
            .unwrap_or_default();
        let cell_offset = cell.offset.unwrap_or_default();
        let stagger = adjusted_projected
            .column_staggers
            .iter()
            .take(column + 1)
            .sum::<f64>();
        let mut local = Vec2 {
            x: column as f64 * adjusted_projected.pitch.x
                + row_offset.x
                + column_offset.x
                + cell_offset.x,
            y: row as f64 * adjusted_projected.pitch.y
                + row_offset.y
                + column_offset.y
                + cell_offset.y
                + stagger,
        };
        if !point_is_finite(local) {
            return Err("The projected matrix produced non-finite cell geometry".into());
        }
        let mut splay_sum = 0.0;
        for index in (0..=column).rev() {
            let angle = adjusted_projected
                .column_splays
                .get(index)
                .copied()
                .unwrap_or(0.0);
            splay_sum += angle;
            let pivot = adjusted_projected
                .column_origins
                .get(index)
                .copied()
                .flatten()
                .unwrap_or(Vec2 {
                    x: index as f64 * adjusted_projected.pitch.x,
                    y: adjusted_projected
                        .column_staggers
                        .iter()
                        .take(index + 1)
                        .sum(),
                });
            if !point_is_finite(pivot) || !splay_sum.is_finite() {
                return Err("The projected matrix produced non-finite splay geometry".into());
            }
            let (sin, cos) = angle.to_radians().sin_cos();
            let dx = local.x - pivot.x;
            let dy = local.y - pivot.y;
            local = Vec2 {
                x: pivot.x + dx * cos - dy * sin,
                y: pivot.y + dx * sin + dy * cos,
            };
            if !point_is_finite(local) {
                return Err("The projected matrix produced non-finite splay geometry".into());
            }
        }
        match mirror_axis {
            crate::model::Mirror::X => local.x = -local.x,
            crate::model::Mirror::Y => local.y = -local.y,
            crate::model::Mirror::None => {}
        }
        let (sin, cos) = matrix_rotation.to_radians().sin_cos();
        let world = Vec2 {
            x: projected.origin.x + local.x * cos - local.y * sin,
            y: projected.origin.y + local.x * sin + local.y * cos,
        };
        let signed_splay = if matches!(
            mirror_axis,
            crate::model::Mirror::X | crate::model::Mirror::Y
        ) {
            -splay_sum
        } else {
            splay_sum
        };
        let rotation = matrix_rotation + signed_splay + cell.rotation.unwrap_or(0.0);
        if !point_is_finite(world) || !rotation.is_finite() {
            return Err("The projected matrix produced non-finite part geometry".into());
        }
        projected_key_poses.insert(
            (cell.row, cell.column),
            Pose2 {
                at: world,
                rotation,
            },
        );
    }
    for cell in &adjusted_projected.cells {
        let key_pose = projected_key_poses
            .get(&(cell.row, cell.column))
            .ok_or_else(|| "The projected matrix cell pose is unavailable".to_owned())?;
        for assembly in &cell.assemblies {
            let (sin, cos) = key_pose.rotation.to_radians().sin_cos();
            let companion_at = Vec2 {
                x: key_pose.at.x + assembly.offset.x * cos - assembly.offset.y * sin,
                y: key_pose.at.y + assembly.offset.x * sin + assembly.offset.y * cos,
            };
            let companion_rotation = key_pose.rotation + assembly.rotation.unwrap_or(0.0);
            if !point_is_finite(companion_at) || !companion_rotation.is_finite() {
                return Err("The projected matrix companion has non-finite geometry".into());
            }
        }
    }

    let (global_sin, global_cos) = (-matrix_rotation.to_radians()).sin_cos();
    let mut retained_cells = Vec::with_capacity(retained_slots.len());
    for ((row, column), id) in retained_slots {
        let retained = document
            .parts
            .iter()
            .find(|part| part.id == id)
            .ok_or_else(|| format!("The mirrored part {id:?} is unavailable"))?;
        let mut local = Vec2 {
            x: global_cos * (retained.pose.at.x - projected.origin.x)
                - global_sin * (retained.pose.at.y - projected.origin.y),
            y: global_sin * (retained.pose.at.x - projected.origin.x)
                + global_cos * (retained.pose.at.y - projected.origin.y),
        };
        match mirror_axis {
            crate::model::Mirror::X => local.x = -local.x,
            crate::model::Mirror::Y => local.y = -local.y,
            crate::model::Mirror::None => {}
        }
        for index in 0..=column as usize {
            let angle = -projected
                .column_splays
                .get(index)
                .copied()
                .unwrap_or(0.0)
                .to_radians();
            let pivot = projected
                .column_origins
                .get(index)
                .copied()
                .flatten()
                .unwrap_or(Vec2 {
                    x: index as f64 * projected.pitch.x,
                    y: projected.column_staggers.iter().take(index + 1).sum(),
                });
            let (sin, cos) = angle.sin_cos();
            let dx = local.x - pivot.x;
            let dy = local.y - pivot.y;
            local = Vec2 {
                x: pivot.x + dx * cos - dy * sin,
                y: pivot.y + dx * sin + dy * cos,
            };
            if !point_is_finite(local) {
                return Err("The retained cell inverse produced non-finite geometry".into());
            }
        }
        let row_offset = adjusted_projected
            .row_offsets
            .get(row as usize)
            .copied()
            .unwrap_or_default();
        let column_offset = adjusted_projected
            .column_offsets
            .get(column as usize)
            .copied()
            .unwrap_or_default();
        let stagger = adjusted_projected
            .column_staggers
            .iter()
            .take(column as usize + 1)
            .sum::<f64>();
        let cell_offset = Vec2 {
            x: local.x - column as f64 * projected.pitch.x - row_offset.x - column_offset.x,
            y: local.y - row as f64 * projected.pitch.y - row_offset.y - column_offset.y - stagger,
        };
        let splay_sum = adjusted_projected
            .column_splays
            .iter()
            .take(column as usize + 1)
            .sum::<f64>();
        let signed_splay = if matches!(
            mirror_axis,
            crate::model::Mirror::X | crate::model::Mirror::Y
        ) {
            -splay_sum
        } else {
            splay_sum
        };
        let cell_rotation = retained.pose.rotation - matrix_rotation - signed_splay;
        if !point_is_finite(cell_offset) || !cell_rotation.is_finite() {
            return Err("The retained cell inverse produced non-finite geometry".into());
        }
        let mut cell = adjusted_projected
            .cells
            .iter()
            .find(|cell| cell.row == row && cell.column == column)
            .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?
            .clone();
        cell.offset = Some(cell_offset);
        cell.rotation = Some(cell_rotation);
        let source_cell = source
            .cells
            .iter()
            .find(|source_cell| (source_cell.row, source_cell.column) == (row, column))
            .ok_or_else(|| "The source matrix cell is unavailable".to_owned())?;
        for assembly in &mut cell.assemblies {
            let source_assembly = source_cell
                .assemblies
                .iter()
                .find(|source_assembly| source_assembly.id == assembly.id)
                .ok_or_else(|| "The retained source assembly is unavailable".to_owned())?;
            *assembly = mirror
                .cells
                .iter()
                .find(|mirror_cell| (mirror_cell.row, mirror_cell.column) == (row, column))
                .and_then(|mirror_cell| {
                    mirror_cell
                        .assemblies
                        .iter()
                        .find(|mirror_assembly| mirror_assembly.id == source_assembly.id)
                })
                .ok_or_else(|| "The retained mirror assembly is unavailable".to_owned())?
                .clone();
        }
        let mut member_ids = vec![id.clone()];
        member_ids.extend(
            cell.assemblies
                .iter()
                .map(|assembly| crate::matrix::companion_id(&id, &assembly.id)),
        );
        retained_cells.push(RetainedCell { member_ids, cell });
    }
    retained_cells.sort_by_key(|retained| (retained.cell.row, retained.cell.column));

    let mut incoming_matrix = adjusted_projected;
    for retained in &retained_cells {
        let cell = incoming_matrix
            .cells
            .iter_mut()
            .find(|cell| cell.row == retained.cell.row && cell.column == retained.cell.column)
            .ok_or_else(|| "The retained matrix cell is unavailable".to_owned())?;
        cell.enabled = false;
        incoming_matrix
            .part_ids
            .retain(|id| !retained.member_ids.contains(id));
    }
    Ok(FixedMembershipView {
        incoming_matrix,
        retained_cells,
    })
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
                || correspondence.mirror_id == projection.mirror_matrix_id
                || correspondence.source_id == projection.mirror_matrix_id
                || correspondence.mirror_id == projection.source_matrix_id)
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

    crate::matrix::valid_matrix(source_matrix, document)?;
    crate::matrix::valid_matrix(mirror_matrix, document)?;
    if !projection.mapping.at.x.is_finite()
        || !projection.mapping.at.y.is_finite()
        || !projection.mapping.rotation.is_finite()
    {
        return Err("The reconciliation mapping contains non-finite geometry".into());
    }

    let projected_matrix = project_matrix(
        source_matrix,
        mirror_matrix,
        matrix_correspondence,
        pair.mapping,
    )?;
    crate::matrix::valid_matrix(&projected_matrix, document)?;
    let view = build_fixed_membership_view(
        document,
        halves,
        source_matrix,
        mirror_matrix,
        &projected_matrix,
        projection,
    )?;

    // Keep both retained parts out of the incoming pass and the previous ownership
    // view, so the destructive setter cannot remove their parts or references.
    let incoming_matrix = view.incoming_matrix;
    let mut staged = document.clone();
    let staged_old_matrix = staged
        .matrices
        .iter_mut()
        .find(|matrix| matrix.id == mirror_matrix.id)
        .ok_or_else(|| "The mirrored matrix disappeared from the staged document".to_owned())?;
    let retained_ids: std::collections::BTreeSet<_> = view
        .retained_cells
        .iter()
        .flat_map(|retained| retained.member_ids.iter().cloned())
        .collect();
    staged_old_matrix
        .part_ids
        .retain(|id| !retained_ids.contains(id));
    crate::matrix::set_matrix(&mut staged, &incoming_matrix)?;

    {
        let normalized_mirror_matrix = staged
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == mirror_matrix.id)
            .ok_or_else(|| "The normalized mirrored matrix is unavailable".to_owned())?;
        for retained in &view.retained_cells {
            let normalized_cell = normalized_mirror_matrix
                .cells
                .iter_mut()
                .find(|cell| cell.row == retained.cell.row && cell.column == retained.cell.column)
                .ok_or_else(|| "The normalized retained matrix cell is unavailable".to_owned())?;
            *normalized_cell = retained.cell.clone();
        }
        let mut ordered_cells: Vec<_> = normalized_mirror_matrix.cells.iter().collect();
        ordered_cells.sort_by_key(|cell| (cell.row, cell.column));
        normalized_mirror_matrix.part_ids = ordered_cells
            .into_iter()
            .filter(|cell| cell.enabled && !cell.deleted)
            .flat_map(|cell| {
                let key_id = format!("matrix/{}/r{}c{}", mirror_matrix.id, cell.row, cell.column);
                std::iter::once(key_id.clone()).chain(
                    cell.assemblies
                        .iter()
                        .map(move |assembly| crate::matrix::companion_id(&key_id, &assembly.id)),
                )
            })
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

    fn fixed_geometry_matrix(id: &str, board_id: &str, source_geometry: bool) -> Matrix {
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
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: (0..2)
                .flat_map(|row| {
                    (0..3).map(move |column| MatrixCell {
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
            matrix.row_offsets = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
            matrix.column_offsets = vec![
                Vec2::default(),
                Vec2 { x: 3.0, y: 4.0 },
                Vec2 { x: -2.0, y: 1.0 },
            ];
            matrix.column_staggers = vec![1.0, 2.0, -1.0];
            matrix.column_splays = vec![90.0, -90.0, 90.0];
            matrix.column_origins = vec![
                Some(Vec2 { x: 2.0, y: 3.0 }),
                None,
                Some(Vec2 { x: 17.0, y: -4.0 }),
            ];
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

    fn fixed_geometry_fixture() -> Result<(ProjectDoc, Projection), String> {
        let mut document = fixture_document()?;
        let mapping = Pose2 {
            at: Vec2 { x: 100.0, y: -30.0 },
            rotation: 90.0,
        };
        document.hardware.as_mut().unwrap().pairs[0].mapping = mapping;
        crate::matrix::set_matrix(
            &mut document,
            &fixed_geometry_matrix("source-matrix", "source-board", true),
        )?;
        crate::matrix::set_matrix(
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
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2::default(),
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
        document.nets.push(crate::model::Net {
            id: "retained-second-key-net".into(),
            name: "RETAINED SECOND KEY".into(),
            pins: vec![
                crate::model::Pin {
                    part_id: "matrix/mirror-matrix/r1c1".into(),
                    pad_id: "pad-one".into(),
                },
                crate::model::Pin {
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
        document.constraints.push(crate::model::Constraint::Offset {
            id: "retained-second-key-offset".into(),
            source_part_id: "geometry-probe".into(),
            target_part_id: "matrix/mirror-matrix/r1c1".into(),
            offset: Vec2 { x: 6.0, y: 18.0 },
            rotation: 0.0,
        });
        document
            .outline
            .push(crate::model::OutlineFeature::PartEnvelope {
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
                operation: crate::model::Operation::Add,
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
            source_board_id: "source-board".into(),
            mirror_board_id: "mirror-board".into(),
            source_matrix_id: "source-matrix".into(),
            mirror_matrix_id: "mirror-matrix".into(),
            mapping,
            parts,
            locally_protected_mirror_parts: protected,
            final_owners,
        };
        Ok((document, projection))
    }

    fn fixed_membership_assembly_fixture(
        mapping_rotation: f64,
    ) -> Result<(ProjectDoc, Projection), String> {
        let (mut document, mut projection) = fixed_geometry_fixture()?;
        let mut diode = document.definitions[0].clone();
        diode.id = "diode".into();
        diode.name = "Diode".into();
        diode.pads = vec![
            crate::model::Pad {
                id: "pad-one".into(),
                number: "1".into(),
                at: Vec2::default(),
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
                matrix.row_offsets = vec![Vec2::default(), Vec2 { x: 1.0, y: 2.0 }];
                matrix.column_offsets = vec![
                    Vec2::default(),
                    Vec2 { x: 3.0, y: 4.0 },
                    Vec2 { x: -2.0, y: 1.0 },
                ];
                matrix.column_staggers = vec![1.0, 2.0, -1.0];
                matrix.column_splays = vec![90.0, -90.0, 90.0];
                matrix.column_origins = vec![
                    Some(Vec2 { x: 2.0, y: 3.0 }),
                    None,
                    Some(Vec2 { x: 17.0, y: -4.0 }),
                ];
                let final_cell = matrix
                    .cells
                    .iter_mut()
                    .find(|cell| (cell.row, cell.column) == (0, 2))
                    .unwrap();
                final_cell.offset = Some(Vec2 { x: 2.0, y: -3.0 });
                final_cell.rotation = Some(90.0);
            }
            crate::matrix::set_matrix(&mut document, &matrix)?;
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
                        (0, 1, "diode") | (1, 2, "diode") => {
                            Some(MirroredCompanionMode::LiteralMirror)
                        }
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

    #[test]
    fn fixed_membership_assembly_fixture_reconciles_existing_companions() {
        let expected_changed = vec![
            "matrix/mirror-matrix/r0c1",
            "matrix/mirror-matrix/r0c1/diode",
            "matrix/mirror-matrix/r0c1/fallback",
            "matrix/mirror-matrix/r0c2",
            "matrix/mirror-matrix/r0c2/diode",
            "matrix/mirror-matrix/r1c0",
            "matrix/mirror-matrix/r1c0/diode",
            "matrix/mirror-matrix/r1c2",
            "matrix/mirror-matrix/r1c2/diode",
            "matrix/mirror-matrix/r1c2/stab",
            "mirror-matrix",
        ];
        let source_keys = [
            ("matrix/source-matrix/r0c0", (7.0, -1.0, 90.0)),
            ("matrix/source-matrix/r0c1", (8.0, 13.0, 0.0)),
            ("matrix/source-matrix/r0c2", (8.0, 5.0, 180.0)),
            ("matrix/source-matrix/r1c0", (-15.0, 0.0, 90.0)),
            ("matrix/source-matrix/r1c1", (9.0, 35.0, 0.0)),
            ("matrix/source-matrix/r1c2", (-17.0, 4.0, 90.0)),
        ];
        let source_companions = [
            ("matrix/source-matrix/r0c0/diode", (4.0, 1.0, 120.0)),
            ("matrix/source-matrix/r0c1/diode", (10.0, 16.0, 30.0)),
            ("matrix/source-matrix/r0c2/diode", (6.0, 2.0, 210.0)),
            ("matrix/source-matrix/r1c0/diode", (-18.0, 2.0, 120.0)),
            ("matrix/source-matrix/r1c1/diode", (11.0, 38.0, 30.0)),
            ("matrix/source-matrix/r1c2/diode", (-20.0, 6.0, 120.0)),
            ("matrix/source-matrix/r1c2/stab", (-17.0, 8.0, 90.0)),
            ("matrix/source-matrix/r0c1/fallback", (13.0, 11.0, 45.0)),
        ];
        let expected_rotated = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (87.0, -38.0, 90.0)),
            ("matrix/mirror-matrix/r0c2", (95.0, -38.0, -90.0)),
            ("matrix/mirror-matrix/r1c0", (100.0, -15.0, 0.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (96.0, -13.0, 0.0)),
            ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
            ("matrix/mirror-matrix/r0c1/diode", (84.0, -40.0, 60.0)),
            ("matrix/mirror-matrix/r0c2/diode", (98.0, -40.0, -60.0)),
            ("matrix/mirror-matrix/r1c0/diode", (99.0, -14.0, -15.0)),
            ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
            ("matrix/mirror-matrix/r1c2/diode", (94.0, -10.0, -30.0)),
            ("matrix/mirror-matrix/r1c2/stab", (92.0, -13.0, 90.0)),
            ("matrix/mirror-matrix/r0c1/fallback", (89.0, -43.0, 45.0)),
        ];
        let expected_axis = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (92.0, -17.0, 0.0)),
            ("matrix/mirror-matrix/r0c2", (92.0, -25.0, -180.0)),
            ("matrix/mirror-matrix/r1c0", (115.0, -30.0, -90.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (117.0, -26.0, -90.0)),
            ("matrix/mirror-matrix/r0c0/diode", (2.0, 3.0, 30.0)),
            ("matrix/mirror-matrix/r0c1/diode", (90.0, -14.0, -30.0)),
            ("matrix/mirror-matrix/r0c2/diode", (90.0, -28.0, -150.0)),
            ("matrix/mirror-matrix/r1c0/diode", (116.0, -29.0, -105.0)),
            ("matrix/mirror-matrix/r1c1/diode", (12.0, 23.0, 30.0)),
            ("matrix/mirror-matrix/r1c2/diode", (120.0, -24.0, -120.0)),
            ("matrix/mirror-matrix/r1c2/stab", (117.0, -22.0, 90.0)),
            ("matrix/mirror-matrix/r0c1/fallback", (87.0, -19.0, -45.0)),
        ];
        let assert_pose = |document: &ProjectDoc, id: &str, expected: (f64, f64, f64)| {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            let heading_delta = (part.pose.rotation - expected.2 + 180.0).rem_euclid(360.0) - 180.0;
            assert!(
                heading_delta.abs() < 1e-9,
                "{id} rotation: {}",
                part.pose.rotation
            );
        };
        let non_pose = |part: &Part| {
            let mut value = serde_json::to_value(part).unwrap();
            value.as_object_mut().unwrap().remove("pose");
            value
        };

        for (mapping_rotation, expected_members, frame, local_cells) in [
            (
                90.0,
                &expected_rotated[..],
                (102.0, -33.0, 90.0),
                [((0, 0), (101.0, 37.0, 0.0)), ((1, 1), (-59.0, 55.0, -90.0))],
            ),
            (
                0.0,
                &expected_axis[..],
                (97.0, -32.0, 0.0),
                [((0, 0), (31.0, -93.0, 90.0)), ((1, 1), (81.0, 15.0, 0.0))],
            ),
        ] {
            let (mut document, projection) =
                fixed_membership_assembly_fixture(mapping_rotation).unwrap();
            for id in ["source-matrix", "mirror-matrix"] {
                let matrix = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == id)
                    .unwrap();
                crate::matrix::valid_matrix(matrix, &document).unwrap();
            }
            crate::mechanical::features::validate(&document).unwrap();
            crate::mechanical::halves::validate_document(&document).unwrap();
            assert_eq!(projection.parts.len(), 4);
            assert_eq!(projection.locally_protected_mirror_parts.len(), 2);
            assert_eq!(projection.final_owners.len(), 28);
            let member_parts: Vec<_> = document
                .parts
                .iter()
                .filter(|part| projection.final_owners.contains_key(&part.id))
                .collect();
            assert_eq!(member_parts.len(), 28);
            assert_eq!(
                member_parts
                    .iter()
                    .map(|part| part.reference.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
                28
            );
            assert!(member_parts.iter().all(|part| {
                part.locked == Some(true)
                    && part.properties.as_ref().unwrap()["custom"]
                        == serde_json::json!({"part": part.id})
                    && part.generator_parameters.as_ref().unwrap()["custom"]
                        == serde_json::json!({"part": part.id})
            }));
            for matrix_id in ["source-matrix", "mirror-matrix"] {
                let matrix = document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_id)
                    .unwrap();
                assert_eq!(matrix.part_ids.len(), 14);
            }
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0]
                    .halves
                    .as_ref()
                    .unwrap()
                    .correspondences
                    .iter()
                    .filter(|association| association.kind == MirroredItemKind::Part)
                    .count(),
                14
            );
            for (id, expected) in source_keys.iter().chain(source_companions.iter()) {
                assert_pose(&document, id, *expected);
            }
            for (retained_id, expected_net, expected_constraint) in [
                (
                    "matrix/mirror-matrix/r0c0/diode",
                    "retained-diode-net-0",
                    "retained-diode-constraint-0",
                ),
                (
                    "matrix/mirror-matrix/r1c1/diode",
                    "retained-diode-net-1",
                    "retained-diode-constraint-1",
                ),
            ] {
                assert!(document.nets.iter().any(|net| {
                    net.id == expected_net
                        && net
                            .pins
                            .iter()
                            .any(|pin| pin.part_id == retained_id && pin.pad_id == "pad-one")
                }));
                assert!(
                    document
                        .constraints
                        .iter()
                        .any(|constraint| constraint.id() == expected_constraint)
                );
                assert!(document.outline.iter().any(|feature| matches!(feature, OutlineFeature::PartEnvelope { part_ids, .. } if part_ids.iter().any(|id| id == retained_id))));
                assert!(document.boards.iter().any(|board| {
                    board.id == "mirror-board"
                        && board.part_ids.iter().any(|id| id == retained_id)
                        && board.net_ids.contains(&expected_net.into())
                        && board
                            .outline_ids
                            .contains(&"retained-diodes-envelope".into())
                }));
            }

            let entry = document.clone();
            let source_ids = entry
                .matrices
                .iter()
                .find(|matrix| matrix.id == "source-matrix")
                .unwrap()
                .part_ids
                .clone();
            let retained_ids = [
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c0/diode",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c1/diode",
            ];
            let source_parts: Vec<_> = source_ids
                .iter()
                .map(|id| {
                    entry
                        .parts
                        .iter()
                        .find(|part| &part.id == id)
                        .unwrap()
                        .clone()
                })
                .collect();
            let retained_parts: Vec<_> = retained_ids
                .iter()
                .map(|id| {
                    entry
                        .parts
                        .iter()
                        .find(|part| part.id == *id)
                        .unwrap()
                        .clone()
                })
                .collect();
            let affected_ids: Vec<_> = entry
                .matrices
                .iter()
                .filter(|matrix| matrix.id == "source-matrix" || matrix.id == "mirror-matrix")
                .flat_map(|matrix| matrix.part_ids.iter())
                .cloned()
                .collect();
            let non_pose_before: Vec<_> = affected_ids
                .iter()
                .map(|id| non_pose(entry.parts.iter().find(|part| &part.id == id).unwrap()))
                .collect();
            let references_before = serde_json::json!({
                "boards": entry.boards,
                "nets": entry.nets,
                "constraints": entry.constraints,
                "boardOutlines": entry.board_outlines,
                "outline": entry.outline,
            });
            let correspondences_before = entry.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .clone();

            let changed = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
            assert_eq!(changed, expected_changed);
            assert_eq!(
                document.hardware.as_ref().unwrap().pairs[0]
                    .halves
                    .as_ref()
                    .unwrap()
                    .correspondences,
                correspondences_before
            );
            for (id, expected) in expected_members {
                assert_pose(&document, id, *expected);
            }
            let final_matrix = document
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            assert_eq!(
                final_matrix.origin,
                Vec2 {
                    x: frame.0,
                    y: frame.1
                }
            );
            assert!((final_matrix.rotation.unwrap_or(0.0) - frame.2).abs() < 1e-9);
            assert_eq!(final_matrix.mirror, Some(Mirror::X));
            let expected_part_ids = [
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c0/diode",
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c1/diode",
                "matrix/mirror-matrix/r0c1/fallback",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r0c2/diode",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c0/diode",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c1/diode",
                "matrix/mirror-matrix/r1c2",
                "matrix/mirror-matrix/r1c2/diode",
                "matrix/mirror-matrix/r1c2/stab",
            ];
            assert_eq!(final_matrix.part_ids, expected_part_ids);
            for ((row, column), (x, y, rotation)) in local_cells {
                let cell = final_matrix
                    .cells
                    .iter()
                    .find(|cell| (cell.row, cell.column) == (row, column))
                    .unwrap();
                assert!((cell.offset.unwrap().x - x).abs() < 1e-9);
                assert!((cell.offset.unwrap().y - y).abs() < 1e-9);
                assert!((cell.rotation.unwrap() - rotation).abs() < 1e-9);
            }
            for (id, expected) in source_ids.iter().zip(&source_parts) {
                assert_eq!(
                    document.parts.iter().find(|part| &part.id == id).unwrap(),
                    expected
                );
            }
            for (id, expected) in retained_ids.iter().zip(&retained_parts) {
                assert_eq!(
                    document.parts.iter().find(|part| part.id == *id).unwrap(),
                    expected
                );
            }
            let non_pose_after: Vec<_> = affected_ids
                .iter()
                .map(|id| non_pose(document.parts.iter().find(|part| &part.id == id).unwrap()))
                .collect();
            assert_eq!(non_pose_after, non_pose_before);
            assert_eq!(
                serde_json::json!({
                    "boards": document.boards,
                    "nets": document.nets,
                    "constraints": document.constraints,
                    "boardOutlines": document.board_outlines,
                    "outline": document.outline,
                }),
                references_before
            );
            for (id, owner) in &projection.final_owners {
                assert_eq!(
                    document
                        .boards
                        .iter()
                        .filter(|board| board.part_ids.contains(id))
                        .map(|board| board.id.as_str())
                        .collect::<Vec<_>>(),
                    vec![owner.as_str()],
                    "owner for {id}"
                );
            }

            let mut disposable = document.clone();
            crate::matrix::set_matrix(&mut disposable, final_matrix).unwrap();
            let rematerialized = disposable
                .matrices
                .iter()
                .find(|matrix| matrix.id == "mirror-matrix")
                .unwrap();
            assert_eq!(rematerialized, final_matrix);
            for (id, expected) in expected_members {
                assert_pose(&disposable, id, *expected);
            }
            let before_repeat = document.clone();
            assert!(
                reconcile(&mut document, "pair-source-mirror", &projection)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(document, before_repeat);
        }
    }

    #[test]
    fn fixed_membership_assembly_rejections_are_atomic() {
        fn assert_rejected(mut document: ProjectDoc, projection: &Projection, reason: &str) {
            let before = serde_json::to_value(&document).unwrap();
            let Err(error) = reconcile(&mut document, "pair-source-mirror", projection) else {
                panic!("expected atomic rejection containing {reason:?}");
            };
            assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
        }

        for (part_id, authored) in [
            ("matrix/mirror-matrix/r0c1", true),
            ("matrix/source-matrix/r0c1", true),
            ("matrix/mirror-matrix/r0c1", false),
            ("matrix/source-matrix/r0c1", false),
        ] {
            let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
            if authored {
                document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == part_id)
                    .unwrap()
                    .properties
                    .as_mut()
                    .unwrap()
                    .insert("layoutOverride".into(), serde_json::json!(1));
            } else {
                document.constraints.push(Constraint::Offset {
                    id: "linked-key-constraint".into(),
                    source_part_id: "assembly-probe".into(),
                    target_part_id: part_id.into(),
                    offset: Vec2 { x: 1.0, y: 2.0 },
                    rotation: 0.0,
                });
            }
            assert_rejected(
                document,
                &projection,
                "Linked matrix keys cannot have authored or constrained placement",
            );
        }

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "unavailable");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "companion association is missing");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap();
        association.source_id = "matrix/mirror-matrix/r0c0/diode".into();
        association.mirror_id = "matrix/source-matrix/r0c0/diode".into();
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap()
            .clone();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(association);
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r0c0/diode")
            .unwrap();
        association.mirror_id = "matrix/mirror-matrix/r0c1/diode".into();
        assert_rejected(
            document,
            &projection,
            "companion association is aliased or inconsistent",
        );

        let (document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        projection
            .final_owners
            .remove("matrix/mirror-matrix/r1c1/diode");
        assert_rejected(document, &projection, "final owner input is incomplete");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/mirror-matrix/r1c1/diode");
        assert_rejected(document, &projection, "no board owner");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("matrix/mirror-matrix/r1c1/diode".into());
        assert_rejected(document, &projection, "ambiguous board ownership");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/mirror-matrix/r0c1/diode");
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .push("matrix/mirror-matrix/r0c1/diode".into());
        assert_rejected(
            document,
            &projection,
            "companion has an incomplete or stale owner input",
        );

        let (mut document, mut projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let matrix = document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        matrix
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (0, 0))
            .unwrap()
            .assemblies
            .clear();
        matrix
            .part_ids
            .retain(|id| id != "matrix/source-matrix/r0c0/diode");
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0/diode");
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .retain(|id| id != "matrix/source-matrix/r0c0/diode");
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r0c0/diode");
        projection
            .final_owners
            .remove("matrix/source-matrix/r0c0/diode");
        assert_rejected(document, &projection, "membership is not canonical");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .assemblies[0]
            .offset
            .x = f64::NAN;
        assert_rejected(document, &projection, "Matrix assembly is invalid");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .assemblies_local = Some(true);
        assert_rejected(document, &projection, "unsupported cell representation");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
            .unwrap();
        association.companion_tweak.as_mut().unwrap().rotation = f64::NAN;
        assert_rejected(document, &projection, "finite");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let mut source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .clone();
        source
            .cells
            .iter_mut()
            .find(|cell| (cell.row, cell.column) == (1, 0))
            .unwrap()
            .assemblies[0]
            .offset
            .x = f64::MAX * 0.9;
        crate::matrix::set_matrix(&mut document, &source).unwrap();
        let association = document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .iter_mut()
            .find(|association| association.source_id == "matrix/source-matrix/r1c0/diode")
            .unwrap();
        association.companion_tweak.as_mut().unwrap().x = -f64::MAX * 0.9;
        assert_rejected(document, &projection, "non-finite values");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        document.constraints.push(Constraint::Offset {
            id: "linked-companion-constraint".into(),
            source_part_id: "assembly-probe".into(),
            target_part_id: "matrix/mirror-matrix/r0c1/diode".into(),
            offset: Vec2 { x: 1.0, y: 2.0 },
            rotation: 0.0,
        });
        assert_rejected(document, &projection, "authored or constrained placement");

        let (mut document, projection) = fixed_membership_assembly_fixture(90.0).unwrap();
        let companion = document
            .parts
            .iter_mut()
            .find(|part| part.id == "matrix/mirror-matrix/r0c1/diode")
            .unwrap();
        companion
            .properties
            .get_or_insert_with(BTreeMap::new)
            .insert("layoutOverride".into(), serde_json::json!(1));
        assert_rejected(document, &projection, "authored or constrained placement");
    }

    #[test]
    fn fixed_membership_geometry_reconciles_multiple_retained_keys() {
        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let source = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        let mirror = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        crate::matrix::valid_matrix(source, &document).unwrap();
        crate::matrix::valid_matrix(mirror, &document).unwrap();
        crate::mechanical::features::validate(&document).unwrap();
        crate::mechanical::halves::validate_document(&document).unwrap();
        assert_eq!(source.part_ids.len(), 6);
        assert_eq!(mirror.part_ids.len(), 6);
        assert_eq!(projection.parts.len(), 4);
        assert_eq!(projection.locally_protected_mirror_parts.len(), 2);
        assert_eq!(projection.final_owners.len(), 12);
        assert_eq!(
            document.hardware.as_ref().unwrap().pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .correspondences
                .iter()
                .filter(|association| association.kind == MirroredItemKind::Part)
                .count(),
            6
        );
        for (id, expected) in [
            ("matrix/source-matrix/r0c0", (7.0, -1.0, 90.0)),
            ("matrix/source-matrix/r0c1", (8.0, 13.0, 0.0)),
            ("matrix/source-matrix/r0c2", (8.0, 5.0, 180.0)),
            ("matrix/source-matrix/r1c0", (-15.0, 0.0, 90.0)),
            ("matrix/source-matrix/r1c1", (9.0, 35.0, 0.0)),
            ("matrix/source-matrix/r1c2", (-17.0, 4.0, 90.0)),
        ] {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rotation"
            );
        }

        let entry = document.clone();
        let entry_source_matrix = source.clone();
        let entry_source_parts: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        let entry_mirror_matrix = entry
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        let affected_part_ids: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .chain(&entry_mirror_matrix.part_ids)
            .cloned()
            .collect();
        let non_pose = |part: &Part| {
            let mut value = serde_json::to_value(part).unwrap();
            value.as_object_mut().unwrap().remove("pose");
            value
        };
        let entry_non_pose: Vec<_> = affected_part_ids
            .iter()
            .map(|id| non_pose(entry.parts.iter().find(|part| part.id == *id).unwrap()))
            .collect();
        let retained_ids = ["matrix/mirror-matrix/r0c0", "matrix/mirror-matrix/r1c1"];
        let retained_parts: Vec<_> = retained_ids
            .iter()
            .map(|id| {
                serde_json::to_value(entry.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        let unchanged_references = serde_json::json!({
            "boards": entry.boards,
            "nets": entry.nets,
            "constraints": entry.constraints,
            "boardOutlines": entry.board_outlines,
            "outline": entry.outline,
        });

        let changed_ids = reconcile(&mut document, "pair-source-mirror", &projection).unwrap();
        assert_eq!(
            changed_ids,
            vec![
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c2",
                "mirror-matrix",
            ]
        );
        let source_after = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap();
        assert_eq!(source_after, &entry_source_matrix);
        let source_parts_after: Vec<_> = entry_source_matrix
            .part_ids
            .iter()
            .map(|id| {
                serde_json::to_value(document.parts.iter().find(|part| part.id == *id).unwrap())
                    .unwrap()
            })
            .collect();
        assert_eq!(source_parts_after, entry_source_parts);
        let non_pose_after: Vec<_> = affected_part_ids
            .iter()
            .map(|id| non_pose(document.parts.iter().find(|part| part.id == *id).unwrap()))
            .collect();
        assert_eq!(non_pose_after, entry_non_pose);

        let expected_final = [
            ("matrix/mirror-matrix/r0c0", (0.0, 0.0, 0.0)),
            ("matrix/mirror-matrix/r0c1", (87.0, -38.0, 90.0)),
            ("matrix/mirror-matrix/r0c2", (95.0, -38.0, -90.0)),
            ("matrix/mirror-matrix/r1c0", (100.0, -15.0, 0.0)),
            ("matrix/mirror-matrix/r1c1", (10.0, 20.0, 0.0)),
            ("matrix/mirror-matrix/r1c2", (96.0, -13.0, 0.0)),
        ];
        for (id, expected) in expected_final {
            let part = document.parts.iter().find(|part| part.id == id).unwrap();
            assert!((part.pose.at.x - expected.0).abs() < 1e-9, "{id} x");
            assert!((part.pose.at.y - expected.1).abs() < 1e-9, "{id} y");
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rotation"
            );
        }
        for (id, expected) in retained_ids.iter().zip(&retained_parts) {
            let retained = document.parts.iter().find(|part| part.id == *id).unwrap();
            assert_eq!(serde_json::to_value(retained).unwrap(), *expected);
        }

        let final_matrix = document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(final_matrix.origin, Vec2 { x: 102.0, y: -33.0 });
        assert_eq!(final_matrix.rotation, Some(90.0));
        assert_eq!(final_matrix.mirror, Some(Mirror::X));
        assert_eq!(
            final_matrix.part_ids,
            vec![
                "matrix/mirror-matrix/r0c0",
                "matrix/mirror-matrix/r0c1",
                "matrix/mirror-matrix/r0c2",
                "matrix/mirror-matrix/r1c0",
                "matrix/mirror-matrix/r1c1",
                "matrix/mirror-matrix/r1c2",
            ]
        );
        for (row, column, expected_offset, expected_rotation) in [
            (0, 0, Vec2 { x: 101.0, y: 37.0 }, 0.0),
            (1, 1, Vec2 { x: -59.0, y: 55.0 }, -90.0),
        ] {
            let cell = final_matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .unwrap();
            let offset = cell.offset.unwrap();
            assert!((offset.x - expected_offset.x).abs() < 1e-9);
            assert!((offset.y - expected_offset.y).abs() < 1e-9);
            assert!((cell.rotation.unwrap() - expected_rotation).abs() < 1e-9);
        }
        assert!(
            final_matrix
                .cells
                .iter()
                .all(|cell| cell.enabled && !cell.deleted)
        );

        let mut disposable = document.clone();
        crate::matrix::set_matrix(&mut disposable, final_matrix).unwrap();
        let rematerialized = disposable
            .matrices
            .iter()
            .find(|matrix| matrix.id == "mirror-matrix")
            .unwrap();
        assert_eq!(rematerialized, final_matrix);
        for (id, expected) in expected_final {
            let part = disposable.parts.iter().find(|part| part.id == id).unwrap();
            assert!(
                (part.pose.at.x - expected.0).abs() < 1e-9,
                "{id} rematerialized x"
            );
            assert!(
                (part.pose.at.y - expected.1).abs() < 1e-9,
                "{id} rematerialized y"
            );
            assert!(
                (part.pose.rotation - expected.2).abs() < 1e-9,
                "{id} rematerialized rotation"
            );
        }
        assert_eq!(
            serde_json::json!({
                "boards": document.boards,
                "nets": document.nets,
                "constraints": document.constraints,
                "boardOutlines": document.board_outlines,
                "outline": document.outline,
            }),
            unchanged_references
        );
        for (id, expected_owner) in &projection.final_owners {
            let owner = document
                .boards
                .iter()
                .find(|board| board.part_ids.iter().any(|part_id| part_id == id))
                .unwrap();
            assert_eq!(&owner.id, expected_owner, "owner for {id}");
        }
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .net_ids
                .is_empty()
        );
        assert!(
            document
                .boards
                .iter()
                .find(|board| board.id == "source-board")
                .unwrap()
                .outline_ids
                .is_empty()
        );
        let mirror_board = document
            .boards
            .iter()
            .find(|board| board.id == "mirror-board")
            .unwrap();
        assert_eq!(mirror_board.net_ids, vec!["retained-second-key-net"]);
        assert_eq!(mirror_board.outline_ids, vec!["retained-keys-envelope"]);
        assert!(mirror_board.part_ids.contains(&"geometry-probe".to_owned()));

        let before_repeat = document.clone();
        assert!(
            reconcile(&mut document, "pair-source-mirror", &projection)
                .unwrap()
                .is_empty()
        );
        assert_eq!(document, before_repeat);
    }

    #[test]
    fn fixed_membership_rejections_are_atomic_and_reach_the_intended_guard() {
        fn assert_rejected(
            mut document: ProjectDoc,
            projection: &Projection,
            expected_reason: &str,
        ) {
            let before = serde_json::to_value(&document).unwrap();
            let error = reconcile(&mut document, "pair-source-mirror", projection).unwrap_err();
            assert!(
                error.contains(expected_reason),
                "expected {expected_reason:?}, got {error:?}"
            );
            assert_eq!(serde_json::to_value(&document).unwrap(), before);
        }

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .push(MirroredCorrespondence {
                companion_mode: None,
                companion_tweak: None,
                kind: MirroredItemKind::Matrix,
                source_id: "mirror-matrix".into(),
                mirror_id: "source-matrix".into(),
            });
        assert_rejected(document, &projection, "matrix correspondence is ambiguous");

        let (document, mut projection) = fixed_geometry_fixture().unwrap();
        projection.parts.push(PartProjection {
            source_id: "matrix/source-matrix/r0c1".into(),
            mirror_id: "matrix/mirror-matrix/r0c1".into(),
        });
        assert_rejected(
            document,
            &projection,
            "linked matrix associations contain aliases",
        );

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .parts
            .retain(|part| part.id != "matrix/source-matrix/r0c0");
        assert_rejected(document, &projection, "unavailable");

        let (document, mut projection) = fixed_geometry_fixture().unwrap();
        projection.final_owners.remove("matrix/mirror-matrix/r1c1");
        assert_rejected(document, &projection, "final owner input is incomplete");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        let part_id = "matrix/source-matrix/r0c1";
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "source-board")
            .unwrap()
            .part_ids
            .retain(|id| id != part_id);
        document
            .boards
            .iter_mut()
            .find(|board| board.id == "mirror-board")
            .unwrap()
            .part_ids
            .push(part_id.into());
        assert_rejected(document, &projection, "owner input");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[1]
            .enabled = false;
        assert_rejected(document, &projection, "fixed active membership");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .part_ids
            .push("matrix/source-matrix/r0c0".into());
        assert_rejected(document, &projection, "membership is not canonical");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .column_splays[1] = f64::NAN;
        assert_rejected(document, &projection, "finite");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .column_splays = vec![f64::MAX, f64::MAX, 0.0];
        assert_rejected(document, &projection, "non-finite splay geometry");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document
            .matrices
            .iter_mut()
            .find(|matrix| matrix.id == "source-matrix")
            .unwrap()
            .cells[0]
            .variant = Some("unsupported-variant".into());
        assert_rejected(document, &projection, "unsupported cell representation");

        let (mut document, projection) = fixed_geometry_fixture().unwrap();
        document.hardware.as_mut().unwrap().pairs[0]
            .halves
            .as_mut()
            .unwrap()
            .correspondences
            .retain(|association| association.source_id != "matrix/source-matrix/r1c1");
        assert_rejected(
            document,
            &projection,
            "canonical matrix part association is missing",
        );
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
