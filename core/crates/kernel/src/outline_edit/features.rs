//! Feature, connection and version lifecycle mutations for outline edits.
use super::*;

pub(super) fn allocated_feature_id(doc: &ProjectDoc, seed: u64, requested: &str) -> String {
    let identities = persisted_identities(doc);
    if identities.iter().any(|id| id == requested) {
        unique_id("outline-feature", seed, &identities)
    } else {
        requested.to_owned()
    }
}

pub(super) fn named_polygon(
    feature: &OutlineFeature,
    id: String,
) -> Result<OutlineFeature, String> {
    let OutlineFeature::Polygon {
        points,
        anchor_part_id,
        operation,
        ..
    } = feature
    else {
        return Err("Only polygons can be added here".into());
    };
    polygon_points(points)?;
    Ok(OutlineFeature::Polygon {
        id,
        points: points.clone(),
        anchor_part_id: anchor_part_id.clone(),
        operation: *operation,
    })
}

pub(super) fn add_feature(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    feature: &OutlineFeature,
) -> Result<Vec<String>, String> {
    match &edit.target {
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let id = allocated_feature_id(doc, edit.seed, feature.id());
            let feature = named_polygon(feature, id.clone())?;
            let version = version_mut(doc, board_id, version_id)?;
            version.geometry.features.push(feature);
            Ok(vec![board_id.clone(), version_id.clone(), id])
        }
        OutlineEditTarget::Board { board_id } => {
            let id = allocated_feature_id(doc, edit.seed, feature.id());
            let feature = named_polygon(feature, id.clone())?;
            if let Some(version_id) = active_version_id(doc, board_id) {
                let version = version_mut(doc, board_id, &version_id)?;
                version.geometry.features.push(feature);
                return Ok(vec![board_id.clone(), version_id, id]);
            }
            // Adding a feature without a fixed active version creates and selects the
            // version in the same accepted edit: one revision and one Undo step.
            let version_id = unique_id("outline-version", edit.seed, &persisted_identities(doc));
            let prefix = format!("board:{board_id}:version:{version_id}");
            let mut geometry = outline_versions::snapshot(doc, board_id, &prefix)?;
            geometry.features.push(feature);
            let name = default_version_name(doc, board_id);
            push_fixed_version(doc, board_id, &version_id, &name, geometry, None)?;
            Ok(vec![board_id.clone(), version_id, id])
        }
    }
}

pub(super) fn set_feature(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
    feature: &OutlineFeature,
) -> Result<Vec<String>, String> {
    let id = feature.id().to_owned();
    match target {
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let version = version_mut(doc, board_id, version_id)?;
            let Some(current) = version
                .geometry
                .features
                .iter_mut()
                .find(|current| current.id() == id)
            else {
                return Err(format!("Outline feature {id} is missing"));
            };
            if current == feature {
                return Ok(vec![]);
            }
            *current = feature.clone();
            Ok(vec![board_id.clone(), version_id.clone(), id])
        }
        OutlineEditTarget::Board { board_id } => {
            if !belongs_to_generated(doc, board_id, &id) {
                return Err(format!("Outline feature {id} is not the generated outline"));
            }
            let Some(current) = doc.outline.iter_mut().find(|current| current.id() == id) else {
                return Err(format!("Outline feature {id} is missing"));
            };
            if current == feature {
                return Ok(vec![]);
            }
            *current = feature.clone();
            Ok(vec![board_id.clone(), id])
        }
    }
}

pub(super) fn remove_feature(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
    feature_id: &str,
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Version {
        board_id,
        version_id,
    } = target
    else {
        return Err("The generated outline cannot lose its base feature".into());
    };
    let version = version_mut(doc, board_id, version_id)?;
    let Some(index) = version
        .geometry
        .features
        .iter()
        .position(|feature| feature.id() == feature_id)
    else {
        return Err(format!("Outline feature {feature_id} is missing"));
    };
    if index == 0
        || matches!(
            version.geometry.features[index],
            OutlineFeature::PartEnvelope { .. }
        )
    {
        return Err("The base outline feature cannot be removed".into());
    }
    version.geometry.features.remove(index);
    Ok(vec![
        board_id.clone(),
        version_id.clone(),
        feature_id.to_owned(),
    ])
}

pub(super) fn add_connection(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    connection: &OutlineConnection,
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Board { board_id } = &edit.target else {
        return Err("Connections apply to the generated outline".into());
    };
    if active_version_id(doc, board_id).is_some() {
        return Err("Connections apply to the generated outline".into());
    }
    if connection.points.len() < 2 || !outline_points_finite(&connection.points) {
        return Err("A connection needs at least two finite points".into());
    }
    // A non-positive or non-finite width is malformed submitted input: it fails with a
    // Core error and no mutation instead of silently defaulting to the bridge width.
    // The adapter supplies the board's bridge width itself.
    if !connection.width.is_finite() || connection.width <= 0.0 {
        return Err("A connection width must be a positive finite number".into());
    }
    for point in &connection.points {
        crate::outline_controls::point(doc, point)?;
    }
    let connection_id = {
        let identities = persisted_identities(doc);
        if identities.iter().any(|identity| identity == &connection.id) {
            unique_id("outline-connection", edit.seed, &identities)
        } else {
            connection.id.clone()
        }
    };
    let feature = generated_feature_mut(doc, board_id)?;
    let OutlineFeature::PartEnvelope {
        connections, id, ..
    } = feature
    else {
        return Err(format!("Board {board_id} has no generated envelope"));
    };
    let id = id.clone();
    connections.push(OutlineConnection {
        id: connection_id.clone(),
        width: connection.width,
        points: connection.points.clone(),
    });
    Ok(vec![board_id.clone(), id, connection_id])
}

pub(super) fn remove_connection(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
    connection_id: &str,
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Board { board_id } = target else {
        return Err("Connections apply to the generated outline".into());
    };
    let feature = generated_feature_mut(doc, board_id)?;
    let OutlineFeature::PartEnvelope {
        connections, id, ..
    } = feature
    else {
        return Err(format!("Board {board_id} has no generated envelope"));
    };
    let before = connections.len();
    connections.retain(|connection| connection.id != connection_id);
    if connections.len() == before {
        return Ok(vec![]);
    }
    Ok(vec![board_id.clone(), id.clone(), connection_id.to_owned()])
}

/// Creates the board's Generated envelope and its board membership in one accepted edit.
///
/// The payload mirrors what the presentation Generate control authors today: the
/// envelope margin, its settings and the included part list, with an empty connection
/// list and an additive operation. Core derives the envelope identity from the captured
/// creation seed (`board:{board}:automatic:outline:{seed}`, the seed being the adapter's
/// operation id) and registers `board.outline_ids` membership itself, so one accepted
/// edit and one Undo own both. A board that already has a Generated envelope - or a
/// re-submission carrying the same captured seed - settles Unchanged, exactly as the
/// previous landing path's guards did.
pub(super) fn create_generated(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    margin: f64,
    settings: &OutlineSettings,
    part_ids: &[String],
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Board { board_id } = &edit.target else {
        return Err("The generated outline belongs to the board".into());
    };
    let board_id = board_id.clone();
    if !margin.is_finite() {
        return Err("The generated outline margin must be a finite number".into());
    }
    if !settings.size.is_finite() || !settings.bridge_width.is_finite() {
        return Err("The generated outline settings must be finite numbers".into());
    }
    if let Some(repair) = &settings.repair
        && !(repair.maximum_gap_span.is_finite()
            && repair.minimum_connection_width.is_finite()
            && repair.edge_clearance.is_finite())
    {
        return Err("The generated outline repair settings must be finite numbers".into());
    }
    let membership = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.part_ids.clone())
        .ok_or_else(|| format!("Unknown board {board_id}"))?;
    for id in part_ids {
        if !doc.parts.iter().any(|part| &part.id == id) {
            return Err(format!("Unknown part {id}"));
        }
        if !membership.iter().any(|member| member == id) {
            return Err(format!("Part {id} is not on board {board_id}"));
        }
    }
    // No second Generated envelope exists: an existing one settles Unchanged.
    if membership.iter().any(|id| {
        doc.outline.iter().any(|feature| {
            feature.id() == id && matches!(feature, OutlineFeature::PartEnvelope { .. })
        })
    }) {
        return Ok(vec![]);
    }
    let feature_id = format!("board:{board_id}:automatic:outline:{}", edit.seed);
    // The same captured seed is the same creation intent, so an already-authored
    // identity settles Unchanged rather than colliding.
    if persisted_identities(doc).iter().any(|id| id == &feature_id) {
        return Ok(vec![]);
    }
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: settings.clone(),
        id: feature_id.clone(),
        part_ids: part_ids.to_vec(),
        margin,
        operation: Operation::Add,
    });
    let board = doc
        .boards
        .iter_mut()
        .find(|board| board.id == board_id)
        .ok_or_else(|| format!("Unknown board {board_id}"))?;
    board.outline_ids.push(feature_id.clone());
    Ok(vec![board_id, feature_id])
}

pub(super) fn default_version_name(doc: &ProjectDoc, board_id: &str) -> String {
    let next_number = doc
        .board_outlines
        .iter()
        .filter(|state| state.board_id == board_id)
        .flat_map(|state| &state.versions)
        .filter_map(|version| {
            version
                .name
                .strip_prefix("Edited outline ")
                .and_then(|number| number.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    format!("Edited outline {next_number}")
}

pub(super) fn version_identity(doc: &ProjectDoc, version_id: &str) -> Result<(), String> {
    if let Some(problem) = outline_versions::version_identity_problem(doc, version_id) {
        return Err(problem);
    }
    Ok(())
}

pub(super) fn push_fixed_version(
    doc: &mut ProjectDoc,
    board_id: &str,
    version_id: &str,
    name: &str,
    mut geometry: OutlineSnapshot,
    provenance: Option<String>,
) -> Result<(), String> {
    version_identity(doc, version_id)?;
    let prefix = format!("board:{board_id}:version:{version_id}");
    for (index, feature) in geometry.features.iter_mut().enumerate() {
        let id = match feature {
            OutlineFeature::Polygon { id, .. }
            | OutlineFeature::Rect { id, .. }
            | OutlineFeature::PartEnvelope { id, .. } => id,
        };
        *id = format!("{prefix}:contour:{index}");
    }
    let revision = doc.revision;
    let state = state_mut(doc, board_id)?;
    state.versions.push(OutlineVersion {
        id: version_id.to_owned(),
        name: name.to_owned(),
        source: OutlineProvenance {
            revision,
            version_id: provenance,
        },
        geometry,
        ownership: OutlineOwnership::Fixed,
    });
    state.active_version_id = Some(version_id.to_owned());
    Ok(())
}

pub(super) fn create_linked_refinement(
    doc: &mut ProjectDoc,
    board_id: &str,
    version_id: &str,
    name: &str,
) -> Result<Vec<String>, String> {
    let name = outline_versions::name(name)?;
    version_identity(doc, version_id)?;
    let prefix = format!("board:{board_id}:version:{version_id}");
    let (cache, _, _) = geometry::outlines(doc, None, &[]);
    let board = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .ok_or_else(|| format!("Unknown board {board_id}"))?;
    // A linked refinement follows the board's current components, so it owns the
    // generated envelope recipe (and any authored generated features) and resolves
    // live; the composed projection stays the permanent Generated version's business.
    let composed = geometry::generated_source(doc, &cache, board);
    if composed.source.is_empty()
        || composed
            .findings
            .iter()
            .any(|finding| finding.severity == Severity::Error)
    {
        return Err("Resolve the generated outline before making a linked refinement".into());
    }
    let mut features: Vec<OutlineFeature> = doc
        .outline
        .iter()
        .filter(|feature| board.outline_ids.iter().any(|id| id == feature.id()))
        .cloned()
        .collect();
    if features.is_empty() {
        return Err("Resolve the generated outline before making a linked refinement".into());
    }
    for (index, feature) in features.iter_mut().enumerate() {
        let id = match feature {
            OutlineFeature::Polygon { id, .. }
            | OutlineFeature::Rect { id, .. }
            | OutlineFeature::PartEnvelope { id, .. } => id,
        };
        *id = format!("{prefix}:contour:{index}");
    }
    let settings = features
        .iter()
        .find_map(|feature| match feature {
            OutlineFeature::PartEnvelope { settings, .. } => Some(settings.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let geometry = OutlineSnapshot {
        expected_regions: composed
            .source
            .iter()
            .filter(|contour| !contour.hole)
            .count() as u32,
        features,
        settings,
        bridges: composed.bridges,
        protected_gaps: vec![],
    };
    let provenance = active_version_id(doc, board_id);
    let revision = doc.revision;
    let state = state_mut(doc, board_id)?;
    state.versions.push(OutlineVersion {
        id: version_id.to_owned(),
        name,
        source: OutlineProvenance {
            revision,
            version_id: provenance,
        },
        geometry,
        ownership: OutlineOwnership::Linked,
    });
    state.active_version_id = Some(version_id.to_owned());
    Ok(vec![board_id.to_owned(), version_id.to_owned()])
}

pub(super) fn copy(
    doc: &mut ProjectDoc,
    board_id: &str,
    version_id: &str,
    name: &str,
) -> Result<Vec<String>, String> {
    let name = outline_versions::name(name)?;
    version_identity(doc, version_id)?;
    let prefix = format!("board:{board_id}:version:{version_id}");
    let geometry = outline_versions::snapshot(doc, board_id, &prefix)?;
    let provenance = active_version_id(doc, board_id);
    push_fixed_version(doc, board_id, version_id, &name, geometry, provenance)?;
    Ok(vec![board_id.to_owned(), version_id.to_owned()])
}

pub(super) fn freeze(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    name: &str,
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Version {
        board_id,
        version_id,
    } = &edit.target
    else {
        return Err("Freeze needs an outline version".into());
    };
    let board_id = board_id.clone();
    let version_id = version_id.clone();
    let name = outline_versions::name(name)?;
    let source = version(doc, &board_id, &version_id)
        .ok_or_else(|| format!("Outline version {version_id} is missing"))?
        .clone();
    let new_id = unique_id("outline-version", edit.seed, &persisted_identities(doc));
    // Freeze copies the version's *current resolved projection* into a fixed copy and
    // retains the source version, so a linked refinement keeps its link.
    let mut geometry = source.geometry.clone();
    if !source.ownership.is_fixed() {
        let resolved = outline_versions::resolved_features(doc, &board_id, &source);
        geometry.features = resolved;
    }
    let prefix = format!("board:{board_id}:version:{new_id}");
    for (index, feature) in geometry.features.iter_mut().enumerate() {
        let id = match feature {
            OutlineFeature::Polygon { id, .. }
            | OutlineFeature::Rect { id, .. }
            | OutlineFeature::PartEnvelope { id, .. } => id,
        };
        *id = format!("{prefix}:contour:{index}");
    }
    let revision = doc.revision;
    let state = state_mut(doc, &board_id)?;
    state.versions.push(OutlineVersion {
        id: new_id.clone(),
        name,
        source: OutlineProvenance {
            revision,
            version_id: Some(source.id.clone()),
        },
        geometry,
        ownership: OutlineOwnership::Fixed,
    });
    state.active_version_id = Some(new_id.clone());
    Ok(vec![board_id, new_id])
}

pub(super) fn edit_generated_perimeter(
    doc: &mut ProjectDoc,
    board_id: &str,
    edit: &OutlineEdit,
    contour: u32,
    points: &[Vec2],
) -> Result<Vec<String>, String> {
    polygon_points(points)?;
    if active_version_id(doc, board_id).is_some() {
        return Err("The outline version changed before the perimeter was edited".into());
    }
    let version_id = unique_id("outline-version", edit.seed, &persisted_identities(doc));
    let prefix = format!("board:{board_id}:version:{version_id}");
    let mut geometry = outline_versions::snapshot(doc, board_id, &prefix)?;
    let Some(feature) = geometry.features.get_mut(contour as usize) else {
        return Err("The perimeter no longer exists".into());
    };
    let operation = feature.operation();
    let id = feature.id().to_owned();
    *feature = OutlineFeature::Polygon {
        id,
        anchor_part_id: None,
        points: points.to_vec(),
        operation,
    };
    let provenance = active_version_id(doc, board_id);
    let name = default_version_name(doc, board_id);
    push_fixed_version(doc, board_id, &version_id, &name, geometry, provenance)?;
    Ok(vec![board_id.to_owned(), version_id])
}

pub(super) fn edit_feature_perimeter(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    feature_id: &str,
    points: &[Vec2],
) -> Result<Vec<String>, String> {
    polygon_points(points)?;
    let OutlineEditTarget::Version {
        board_id,
        version_id,
    } = &edit.target
    else {
        return Err("Editing a feature perimeter needs an outline version".into());
    };
    let board_id = board_id.clone();
    let version_id = version_id.clone();
    let source = version(doc, &board_id, &version_id)
        .ok_or_else(|| format!("Outline version {version_id} is missing"))?
        .clone();
    let replace = |features: &mut Vec<OutlineFeature>| -> Result<bool, String> {
        let Some(current) = features
            .iter_mut()
            .find(|current| current.id() == feature_id)
        else {
            return Err(format!("Outline feature {feature_id} is missing"));
        };
        let operation = current.operation();
        let anchor = match current {
            OutlineFeature::Polygon { anchor_part_id, .. }
            | OutlineFeature::Rect { anchor_part_id, .. } => anchor_part_id.clone(),
            _ => None,
        };
        let next = OutlineFeature::Polygon {
            id: feature_id.to_owned(),
            anchor_part_id: anchor,
            points: points.to_vec(),
            operation,
        };
        if *current == next {
            return Ok(false);
        }
        *current = next;
        Ok(true)
    };
    if source.ownership.is_fixed() {
        // A fixed edited outline is edited in place.
        let version = version_mut(doc, &board_id, &version_id)?;
        if !replace(&mut version.geometry.features)? {
            return Ok(vec![]);
        }
        return Ok(vec![board_id, version_id, feature_id.to_owned()]);
    }
    // Manual perimeter editing of a linked refinement creates a fixed edited copy and
    // retains the linked version.
    let new_id = unique_id("outline-version", edit.seed, &persisted_identities(doc));
    let mut geometry = source.geometry.clone();
    geometry.features = outline_versions::resolved_features(doc, &board_id, &source);
    replace(&mut geometry.features)?;
    let prefix = format!("board:{board_id}:version:{new_id}");
    for (index, feature) in geometry.features.iter_mut().enumerate() {
        let id = match feature {
            OutlineFeature::Polygon { id, .. }
            | OutlineFeature::Rect { id, .. }
            | OutlineFeature::PartEnvelope { id, .. } => id,
        };
        *id = format!("{prefix}:contour:{index}");
    }
    let name = default_version_name(doc, &board_id);
    let revision = doc.revision;
    let state = state_mut(doc, &board_id)?;
    state.versions.push(OutlineVersion {
        id: new_id.clone(),
        name,
        source: OutlineProvenance {
            revision,
            version_id: Some(version_id),
        },
        geometry,
        ownership: OutlineOwnership::Fixed,
    });
    state.active_version_id = Some(new_id.clone());
    Ok(vec![board_id, new_id])
}

pub(super) fn activate(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
) -> Result<Vec<String>, String> {
    let board_id = target.board_id().to_owned();
    let requested = target.version_id().map(|id| id.to_owned());
    let state = doc
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    if let Some(id) = &requested
        && !state.is_some_and(|state| state.versions.iter().any(|version| &version.id == id))
    {
        return Err(format!("Unknown outline version {id}"));
    }
    if state.and_then(|state| state.active_version_id.clone()) == requested {
        return Ok(vec![]);
    }
    let state = state_mut(doc, &board_id)?;
    state.active_version_id = requested.clone();
    let mut changed = vec![board_id];
    changed.extend(requested);
    Ok(changed)
}

pub(super) fn rename(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
    name: &str,
) -> Result<Vec<String>, String> {
    let name = outline_versions::name(name)?;
    let OutlineEditTarget::Version {
        board_id,
        version_id,
    } = target
    else {
        return Err("Renaming needs an outline version".into());
    };
    let version = version_mut(doc, board_id, version_id)?;
    if version.name == name {
        return Ok(vec![]);
    }
    version.name = name;
    Ok(vec![board_id.clone(), version_id.clone()])
}

pub(super) fn delete(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
) -> Result<Vec<String>, String> {
    let OutlineEditTarget::Version {
        board_id,
        version_id,
    } = target
    else {
        return Err("Only a named outline version can be deleted".into());
    };
    let state = state_mut(doc, board_id)?;
    if !state
        .versions
        .iter()
        .any(|version| &version.id == version_id)
    {
        return Err(format!("Outline version {version_id} is missing"));
    }
    state.versions.retain(|version| &version.id != version_id);
    if !state.retired_version_ids.iter().any(|id| id == version_id) {
        state.retired_version_ids.push(version_id.clone());
    }
    if state.active_version_id.as_ref() == Some(version_id) {
        state.active_version_id = None;
    }
    Ok(vec![board_id.clone(), version_id.clone()])
}
