//! Typed outline edit vocabulary and the accepted edit that carries it.
//!
//! One [`OutlineEdit`] names a typed target and the specific user change. Core owns
//! the rule, target eligibility, validation, identity allocation and mutation, and
//! reports only the durable ids whose values or memberships actually changed. UI
//! drafts, selection and focus stay in the adapter, and no adapter prepares a
//! replacement document or pre-plans an entity identity.
//!
//! Composite user actions - creating a linked refinement, copy/edit/activate, and
//! Freeze - are one accepted edit, one revision and one Undo step. Equal field values
//! settle as a no-op ([`OutlineEditDisposition::Unchanged`] and an empty changed-id
//! list) through the same private rules the commit uses. A target that has vanished or
//! become ineligible retires; malformed submitted geometry is `Err` with no mutation.
//! Every mutation is staged on a clone, so a failure leaves the accepted document
//! untouched.
use crate::geometry;
use crate::model::*;
use crate::outline_versions;
use serde::{Deserialize, Serialize};

/// The board outline owner a typed outline edit addresses.
///
/// `Board` is the board's Generated outline and its lifecycle: an edit that addresses
/// the generated source acts on the generated feature, and a lifecycle change acts on
/// the board's versions. `Version` names one fixed or linked version explicitly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OutlineEditTarget {
    Board {
        #[serde(rename = "boardId")]
        board_id: String,
    },
    Version {
        #[serde(rename = "boardId")]
        board_id: String,
        #[serde(rename = "versionId")]
        version_id: String,
    },
}

impl OutlineEditTarget {
    pub fn board_id(&self) -> &str {
        match self {
            Self::Board { board_id } | Self::Version { board_id, .. } => board_id,
        }
    }

    pub fn version_id(&self) -> Option<&str> {
        match self {
            Self::Board { .. } => None,
            Self::Version { version_id, .. } => Some(version_id),
        }
    }
}

/// One named outline change.
///
/// Externally tagged, kebab-case variant tags, like the mechanical vocabulary; a
/// variant's payload fields keep their snake_case names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutlineChange {
    SetMargin {
        margin: f64,
    },
    SetCorners {
        corners: CornerStyle,
    },
    SetSize {
        size: f64,
    },
    SetBridgeWidth {
        bridge_width: f64,
    },
    SetRepairEnabled {
        enabled: bool,
    },
    SetMaximumGapSpan {
        value: f64,
    },
    SetMinimumConnectionWidth {
        value: f64,
    },
    SetEdgeClearance {
        value: f64,
    },
    SetProtectedGap {
        gap_id: String,
        points: Vec<OutlineControlPoint>,
    },
    RemoveProtectedGap {
        gap_id: String,
    },
    AddFeature {
        feature: OutlineFeature,
    },
    SetFeature {
        feature: OutlineFeature,
    },
    RemoveFeature {
        feature_id: String,
    },
    AddConnection {
        connection: OutlineConnection,
    },
    RemoveConnection {
        connection_id: String,
    },
    /// Creates the board's Generated envelope and registers its board membership in one
    /// accepted edit (the permanent outline a board without one gains from Generate).
    CreateGenerated {
        margin: f64,
        settings: OutlineSettings,
        part_ids: Vec<String>,
    },
    /// Creates a linked refinement from the board's current generated projections.
    CreateLinkedRefinement {
        version_id: String,
        name: String,
    },
    /// Copies the current outline (the active version, or the generated source when
    /// Generated is active) into a new fixed version and selects it.
    Copy {
        version_id: String,
        name: String,
    },
    /// Copies the target version's current resolved projection into a new fixed
    /// version and selects it, retaining the source version (linked refinements keep
    /// their link).
    Freeze {
        name: String,
    },
    /// Copies the generated source, replaces one contour and selects the copy - one
    /// accepted edit and one Undo step.
    EditGeneratedPerimeter {
        contour: u32,
        points: Vec<Vec2>,
    },
    /// Edits one feature's perimeter. A fixed version is edited in place; a linked
    /// refinement produces a fixed edited copy and retains the link.
    EditFeaturePerimeter {
        feature_id: String,
        points: Vec<Vec2>,
    },
    Activate,
    Rename {
        name: String,
    },
    Delete,
}

/// One typed outline edit: typed target, typed change, the captured creation seed
/// Core allocates new identities from, and - for a reviewed generated-shape edit -
/// the source shape the user reviewed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineEdit {
    pub target: OutlineEditTarget,
    pub change: OutlineChange,
    /// Captured creation seed for identities Core allocates during this edit.
    pub seed: u64,
    /// The generated source shape reviewed when a generated/perimeter edit was
    /// authored. Settlement retires the queued edit when the current relevant source
    /// geometry differs, instead of comparing whole-document revisions. A component
    /// move, addition or removal changes it; unrelated metadata or another board's
    /// edit does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<Vec<Contour>>,
}

/// The settlement disposition of a submitted outline edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineEditDisposition {
    /// The edit changes durable state.
    Changed,
    /// The edit equals the accepted value and needs no revision step.
    Unchanged,
    /// The target has vanished or is ineligible, so a queued edit retires.
    Retire,
}

/// Applies one typed outline edit, returning the changed durable ids.
///
/// The board id is reported first, then the version, feature, connection or
/// protected-gap ids whose values or memberships actually changed. A staged clone
/// means any failure leaves the accepted document untouched.
pub fn apply(doc: &mut ProjectDoc, edit: &OutlineEdit) -> Result<Vec<String>, String> {
    let mut next = doc.clone();
    let changed = apply_staged(&mut next, edit)?;
    *doc = next;
    Ok(changed)
}

/// Evaluates one typed outline edit against the accepted document.
///
/// This is the read-only settlement half: it shares the private implementation with
/// [`apply`], never exposes a prepared document, and reports a vanished or ineligible
/// target as [`OutlineEditDisposition::Retire`]. Structural problems - malformed
/// geometry or an unsupported change - are `Err` with no mutation.
pub fn evaluate(
    document: &ProjectDoc,
    edit: &OutlineEdit,
) -> Result<OutlineEditDisposition, String> {
    if !document
        .boards
        .iter()
        .any(|board| board.id == *edit.target.board_id())
    {
        return Ok(OutlineEditDisposition::Retire);
    }
    // A reviewed shape edit retires when the relevant source geometry changed, never
    // comparing whole-document revisions: unrelated names and other-board edits do not
    // retire it. A Board target reviews the generated source; a Version target reviews
    // that version's own current projection (a linked refinement's resolved source, or a
    // fixed copy's authored geometry), so an authored cutout is part of what was
    // reviewed rather than a false difference from the generated source.
    if let Some(reviewed) = &edit.reviewed {
        let current = match &edit.target {
            OutlineEditTarget::Board { board_id } => generated_source_shape(document, board_id),
            OutlineEditTarget::Version {
                board_id,
                version_id,
            } => {
                let Some(version) = version(document, board_id, version_id) else {
                    return Ok(OutlineEditDisposition::Retire);
                };
                version_source_shape(document, board_id, version)
            }
        };
        if current != *reviewed {
            return Ok(OutlineEditDisposition::Retire);
        }
    }
    match &edit.target {
        OutlineEditTarget::Board { board_id } => {
            // A generated-shape edit addresses the permanent Generated outline: once
            // another version became active the queued intent retires and is never
            // redirected at the new active version.
            if generated_shape_change(&edit.change)
                && active_version_id(document, board_id).is_some()
            {
                return Ok(OutlineEditDisposition::Retire);
            }
        }
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let Some(version) = version(document, board_id, version_id) else {
                return Ok(OutlineEditDisposition::Retire);
            };
            // A geometry edit for version A retires when the active outline changed to
            // B; it is never redirected. Activation/naming/lifecycle actions keep their
            // own eligibility.
            if geometry_change(&edit.change)
                && active_version_id(document, board_id).as_deref() != Some(version_id.as_str())
            {
                return Ok(OutlineEditDisposition::Retire);
            }
            if addressed_entity_is_gone(version, &edit.change) {
                return Ok(OutlineEditDisposition::Retire);
            }
        }
    }
    let mut next = document.clone();
    match apply_staged(&mut next, edit) {
        Ok(changed) if changed.is_empty() => Ok(OutlineEditDisposition::Unchanged),
        Ok(_) => Ok(OutlineEditDisposition::Changed),
        Err(message) => Err(message),
    }
}

/// Whether the edit changes resolved outline geometry, so dependent outlines and
/// findings must be recomputed. Metadata-only naming does not.
pub(crate) fn affects_outline(change: &OutlineChange) -> bool {
    !matches!(change, OutlineChange::Rename { .. })
}

fn geometry_change(change: &OutlineChange) -> bool {
    matches!(
        change,
        OutlineChange::AddFeature { .. }
            | OutlineChange::SetFeature { .. }
            | OutlineChange::RemoveFeature { .. }
            | OutlineChange::AddConnection { .. }
            | OutlineChange::RemoveConnection { .. }
            | OutlineChange::SetProtectedGap { .. }
            | OutlineChange::RemoveProtectedGap { .. }
            | OutlineChange::EditGeneratedPerimeter { .. }
            | OutlineChange::EditFeaturePerimeter { .. }
            | OutlineChange::Delete
    )
}

/// A change that only the generated source can answer.
fn generated_shape_change(change: &OutlineChange) -> bool {
    matches!(
        change,
        OutlineChange::AddConnection { .. }
            | OutlineChange::RemoveConnection { .. }
            | OutlineChange::EditGeneratedPerimeter { .. }
    )
}

fn active_version_id(doc: &ProjectDoc, board_id: &str) -> Option<String> {
    doc.board_outlines
        .iter()
        .find(|state| state.board_id == board_id)
        .and_then(|state| state.active_version_id.clone())
}

/// The board's current generated source contours, from the authored generated features.
fn generated_source_shape(doc: &ProjectDoc, board_id: &str) -> Vec<Contour> {
    let Some(board) = doc.boards.iter().find(|board| board.id == board_id) else {
        return vec![];
    };
    let (cache, _, _) = geometry::outlines(doc, None, &[]);
    geometry::generated_source(doc, &cache, board).source
}

/// One version's current source contours: a linked refinement's resolved projection from
/// the board's current components, or a fixed copy's authored geometry.
fn version_source_shape(
    doc: &ProjectDoc,
    board_id: &str,
    version: &OutlineVersion,
) -> Vec<Contour> {
    let features = if version.ownership.is_fixed() {
        version.geometry.features.clone()
    } else {
        outline_versions::resolved_features(doc, board_id, version)
    };
    let (cache, _, _) = geometry::outlines(doc, None, &[]);
    geometry::feature_source_contours(doc, &cache, &features)
}

fn version<'a>(
    doc: &'a ProjectDoc,
    board_id: &str,
    version_id: &str,
) -> Option<&'a OutlineVersion> {
    doc.board_outlines
        .iter()
        .find(|state| state.board_id == board_id)?
        .versions
        .iter()
        .find(|version| version.id == version_id)
}

fn version_mut<'a>(
    doc: &'a mut ProjectDoc,
    board_id: &str,
    version_id: &str,
) -> Result<&'a mut OutlineVersion, String> {
    doc.board_outlines
        .iter_mut()
        .find(|state| state.board_id == board_id)
        .and_then(|state| state.versions.iter_mut().find(|v| v.id == version_id))
        .ok_or_else(|| format!("Outline version {version_id} is missing"))
}

/// The board's outline state, creating the empty state when the board has none yet.
fn state_mut<'a>(doc: &'a mut ProjectDoc, board_id: &str) -> Result<&'a mut BoardOutline, String> {
    if !doc.boards.iter().any(|board| board.id == board_id) {
        return Err(format!("Unknown board {board_id}"));
    }
    let index = match doc
        .board_outlines
        .iter()
        .position(|state| state.board_id == board_id)
    {
        Some(index) => index,
        None => {
            doc.board_outlines.push(BoardOutline {
                board_id: board_id.to_owned(),
                active_version_id: None,
                versions: vec![],
                generated_last_valid: None,
                retired_version_ids: vec![],
            });
            doc.board_outlines.len() - 1
        }
    };
    Ok(&mut doc.board_outlines[index])
}

/// The board's generated envelope feature (the permanent Generated outline source).
fn generated_feature_mut<'a>(
    doc: &'a mut ProjectDoc,
    board_id: &str,
) -> Result<&'a mut OutlineFeature, String> {
    let ids = doc
        .boards
        .iter()
        .find(|board| board.id == board_id)
        .map(|board| board.outline_ids.clone())
        .ok_or_else(|| format!("Unknown board {board_id}"))?;
    doc.outline
        .iter_mut()
        .find(|feature| ids.iter().any(|id| id == feature.id()))
        .ok_or_else(|| format!("Board {board_id} has no generated outline"))
}

fn belongs_to_generated(doc: &ProjectDoc, board_id: &str, id: &str) -> bool {
    doc.boards
        .iter()
        .find(|board| board.id == board_id)
        .is_some_and(|board| board.outline_ids.iter().any(|outline_id| outline_id == id))
}

/// Every persisted outline identity: the single-namespace collision set.
fn persisted_identities(doc: &ProjectDoc) -> Vec<String> {
    let mut ids: Vec<String> = doc
        .outline
        .iter()
        .map(|feature| feature.id().to_owned())
        .collect();
    for feature in &doc.outline {
        if let OutlineFeature::PartEnvelope { connections, .. } = feature {
            ids.extend(connections.iter().map(|connection| connection.id.clone()));
        }
    }
    for state in &doc.board_outlines {
        ids.extend(state.retired_version_ids.iter().cloned());
        for version in &state.versions {
            ids.push(version.id.clone());
            ids.extend(
                version
                    .geometry
                    .settings
                    .repair
                    .iter()
                    .flat_map(|repair| repair.keep_gaps.iter().map(|gap| gap.id.clone())),
            );
            ids.extend(
                version
                    .geometry
                    .protected_gaps
                    .iter()
                    .map(|gap| gap.id.clone()),
            );
            ids.extend(
                version
                    .geometry
                    .bridges
                    .iter()
                    .map(|bridge| bridge.id.clone()),
            );
            for feature in &version.geometry.features {
                ids.push(feature.id().to_owned());
                if let OutlineFeature::PartEnvelope { connections, .. } = feature {
                    ids.extend(connections.iter().map(|connection| connection.id.clone()));
                }
            }
        }
    }
    ids
}

fn unique_id(prefix: &str, seed: u64, existing: &[String]) -> String {
    let base = format!("{prefix}-{seed}");
    if !existing.iter().any(|id| id == &base) {
        return base;
    }
    let mut index = 2;
    loop {
        let candidate = format!("{base}-{index}");
        if !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn finite(points: &[Vec2]) -> bool {
    points
        .iter()
        .all(|point| point.x.is_finite() && point.y.is_finite())
}

fn outline_points_finite(points: &[OutlineControlPoint]) -> bool {
    points
        .iter()
        .all(|point| point.at.x.is_finite() && point.at.y.is_finite())
}

/// Malformed submitted geometry fails with a Core error and changes nothing. A
/// self-intersection is *not* rejected here: it stays a located finding.
fn polygon_points(points: &[Vec2]) -> Result<(), String> {
    if points.len() < 3 || !finite(points) {
        return Err("A polygon needs at least three finite points".into());
    }
    Ok(())
}

fn addressed_entity_is_gone(version: &OutlineVersion, change: &OutlineChange) -> bool {
    let has_feature = |id: &str| {
        version
            .geometry
            .features
            .iter()
            .any(|feature| feature.id() == id)
    };
    let has_connection = |id: &str| {
        version.geometry.features.iter().any(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { connections, .. }
                if connections.iter().any(|connection| connection.id == id))
        })
    };
    match change {
        OutlineChange::SetFeature { feature } => !has_feature(feature.id()),
        OutlineChange::RemoveFeature { feature_id }
        | OutlineChange::EditFeaturePerimeter { feature_id, .. } => !has_feature(feature_id),
        OutlineChange::RemoveConnection { connection_id } => !has_connection(connection_id),
        _ => false,
    }
}

/// The settings field a named settings change carries, if any.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SettingsField {
    Margin(f64),
    Corners(CornerStyle),
    Size(f64),
    BridgeWidth(f64),
    RepairEnabled(bool),
    MaximumGapSpan(f64),
    MinimumConnectionWidth(f64),
    EdgeClearance(f64),
}

fn settings_field(change: &OutlineChange) -> Option<SettingsField> {
    match change {
        OutlineChange::SetMargin { margin } => Some(SettingsField::Margin(*margin)),
        OutlineChange::SetCorners { corners } => Some(SettingsField::Corners(*corners)),
        OutlineChange::SetSize { size } => Some(SettingsField::Size(*size)),
        OutlineChange::SetBridgeWidth { bridge_width } => {
            Some(SettingsField::BridgeWidth(*bridge_width))
        }
        OutlineChange::SetRepairEnabled { enabled } => Some(SettingsField::RepairEnabled(*enabled)),
        OutlineChange::SetMaximumGapSpan { value } => Some(SettingsField::MaximumGapSpan(*value)),
        OutlineChange::SetMinimumConnectionWidth { value } => {
            Some(SettingsField::MinimumConnectionWidth(*value))
        }
        OutlineChange::SetEdgeClearance { value } => Some(SettingsField::EdgeClearance(*value)),
        _ => None,
    }
}

/// Refuses a value outside the field's range: every length is finite and non-negative,
/// and a bridge needs a positive width.
fn validate_settings_field(field: SettingsField) -> Result<(), String> {
    let (label, value, positive) = match field {
        SettingsField::Margin(value) => ("margin", value, false),
        SettingsField::Size(value) => ("size", value, false),
        SettingsField::BridgeWidth(value) => ("bridge width", value, true),
        SettingsField::MaximumGapSpan(value) => ("maximum gap span", value, false),
        SettingsField::MinimumConnectionWidth(value) => ("minimum connection width", value, false),
        SettingsField::EdgeClearance(value) => ("edge clearance", value, false),
        SettingsField::Corners(_) | SettingsField::RepairEnabled(_) => return Ok(()),
    };
    match (positive, value.is_finite()) {
        (true, true) if value > 0.0 => Ok(()),
        (false, true) if value >= 0.0 => Ok(()),
        (true, _) => Err(format!("The outline {label} must be a positive number")),
        (false, _) => Err(format!("The outline {label} must be zero or more")),
    }
}

/// Applies a settings field to the outline settings owner.
fn write_settings(settings: &mut OutlineSettings, field: SettingsField) {
    match field {
        SettingsField::Margin(_) => {}
        SettingsField::Corners(value) => settings.corners = value,
        SettingsField::Size(value) => settings.size = value,
        SettingsField::BridgeWidth(value) => settings.bridge_width = value,
        SettingsField::RepairEnabled(value) => {
            settings.repair.get_or_insert_with(Default::default).enabled = value
        }
        SettingsField::MaximumGapSpan(value) => {
            settings
                .repair
                .get_or_insert_with(Default::default)
                .maximum_gap_span = value
        }
        SettingsField::MinimumConnectionWidth(value) => {
            settings
                .repair
                .get_or_insert_with(Default::default)
                .minimum_connection_width = value
        }
        SettingsField::EdgeClearance(value) => {
            settings
                .repair
                .get_or_insert_with(Default::default)
                .edge_clearance = value
        }
    }
}

fn apply_staged(doc: &mut ProjectDoc, edit: &OutlineEdit) -> Result<Vec<String>, String> {
    let board_id = edit.target.board_id().to_owned();
    if !doc.boards.iter().any(|board| board.id == board_id) {
        return Err(format!("Unknown board {board_id}"));
    }
    if let Some(field) = settings_field(&edit.change) {
        return apply_settings(doc, &edit.target, field);
    }
    match &edit.change {
        OutlineChange::SetProtectedGap { gap_id, points } => {
            set_protected_gap(doc, edit, gap_id, points)
        }
        OutlineChange::RemoveProtectedGap { gap_id } => remove_protected_gap(doc, edit, gap_id),
        OutlineChange::AddFeature { feature } => add_feature(doc, edit, feature),
        OutlineChange::SetFeature { feature } => set_feature(doc, &edit.target, feature),
        OutlineChange::RemoveFeature { feature_id } => {
            remove_feature(doc, &edit.target, feature_id)
        }
        OutlineChange::AddConnection { connection } => add_connection(doc, edit, connection),
        OutlineChange::RemoveConnection { connection_id } => {
            remove_connection(doc, &edit.target, connection_id)
        }
        OutlineChange::CreateGenerated {
            margin,
            settings,
            part_ids,
        } => create_generated(doc, edit, *margin, settings, part_ids),
        OutlineChange::CreateLinkedRefinement { version_id, name } => {
            create_linked_refinement(doc, &board_id, version_id, name)
        }
        OutlineChange::Copy { version_id, name } => copy(doc, &board_id, version_id, name),
        OutlineChange::Freeze { name } => freeze(doc, edit, name),
        OutlineChange::EditGeneratedPerimeter { contour, points } => {
            edit_generated_perimeter(doc, &board_id, edit, *contour, points)
        }
        OutlineChange::EditFeaturePerimeter { feature_id, points } => {
            edit_feature_perimeter(doc, edit, feature_id, points)
        }
        OutlineChange::Activate => activate(doc, &edit.target),
        OutlineChange::Rename { name } => rename(doc, &edit.target, name),
        OutlineChange::Delete => delete(doc, &edit.target),
        _ => unreachable!("settings changes are handled above"),
    }
}

fn apply_settings(
    doc: &mut ProjectDoc,
    target: &OutlineEditTarget,
    field: SettingsField,
) -> Result<Vec<String>, String> {
    validate_settings_field(field)?;
    match target {
        OutlineEditTarget::Board { board_id } => {
            let board_id = board_id.clone();
            let feature = generated_feature_mut(doc, &board_id)?;
            let OutlineFeature::PartEnvelope {
                settings,
                margin,
                id,
                ..
            } = feature
            else {
                return Err(format!("Board {board_id} has no generated envelope"));
            };
            let before = settings.clone();
            let before_margin = *margin;
            write_settings(settings, field);
            if let SettingsField::Margin(value) = field {
                *margin = value;
            }
            if *settings == before && *margin == before_margin {
                return Ok(vec![]);
            }
            Ok(vec![board_id, id.clone()])
        }
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let board_id = board_id.clone();
            let version_id = version_id.clone();
            let version = version_mut(doc, &board_id, &version_id)?;
            let mut changed = false;
            if let SettingsField::Margin(value) = field {
                let Some(margin) =
                    version
                        .geometry
                        .features
                        .iter_mut()
                        .find_map(|feature| match feature {
                            OutlineFeature::PartEnvelope { margin, .. } => Some(margin),
                            _ => None,
                        })
                else {
                    return Err("This outline has no envelope to set a margin on".into());
                };
                changed = *margin != value;
                *margin = value;
            }
            let before = version.geometry.settings.clone();
            write_settings(&mut version.geometry.settings, field);
            changed |= version.geometry.settings != before;
            if !changed {
                return Ok(vec![]);
            }
            // The envelope features carry the version settings as projection
            // bookkeeping; the version snapshot's settings are authoritative.
            let settings = version.geometry.settings.clone();
            for feature in &mut version.geometry.features {
                if let OutlineFeature::PartEnvelope {
                    settings: feature_settings,
                    ..
                } = feature
                {
                    *feature_settings = settings.clone();
                }
            }
            Ok(vec![board_id, version_id])
        }
    }
}

fn set_protected_gap(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    gap_id: &str,
    points: &[OutlineControlPoint],
) -> Result<Vec<String>, String> {
    if points.len() < 3 || !outline_points_finite(points) {
        return Err("A protected gap needs at least three finite points".into());
    }
    for point in points {
        if point.part_id.is_some() {
            crate::outline_controls::point(doc, point)?;
        }
    }
    let target = edit.target.clone();
    // A gap the target already holds keeps its identity (this is an update); only an
    // identity owned by another persisted namespace is allocated a fresh free id.
    let gap_id = if target_holds_gap(doc, &target, gap_id) {
        gap_id.to_owned()
    } else {
        let identities = persisted_identities(doc);
        if identities.iter().any(|id| id == gap_id) {
            unique_id("outline-gap", edit.seed, &identities)
        } else {
            gap_id.to_owned()
        }
    };
    let gap = ProtectedOutlineGap {
        id: gap_id.clone(),
        points: points.to_vec(),
    };
    let owner = match &target {
        OutlineEditTarget::Board { board_id } => {
            let board_id = board_id.clone();
            let feature = generated_feature_mut(doc, &board_id)?;
            let OutlineFeature::PartEnvelope { settings, id, .. } = feature else {
                return Err(format!("Board {board_id} has no generated envelope"));
            };
            if settings
                .repair
                .as_ref()
                .is_some_and(|repair| gap_is_accepted(&repair.keep_gaps, &gap))
            {
                return Ok(vec![]);
            }
            let repair = settings.repair.get_or_insert_with(Default::default);
            upsert_gap(&mut repair.keep_gaps, gap);
            Some((board_id, vec![id.clone()]))
        }
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let board_id = board_id.clone();
            let version_id = version_id.clone();
            let version = version_mut(doc, &board_id, &version_id)?;
            if version
                .geometry
                .settings
                .repair
                .as_ref()
                .is_some_and(|repair| gap_is_accepted(&repair.keep_gaps, &gap))
            {
                return Ok(vec![]);
            }
            let repair = version
                .geometry
                .settings
                .repair
                .get_or_insert_with(Default::default);
            upsert_gap(&mut repair.keep_gaps, gap.clone());
            upsert_gap(&mut version.geometry.protected_gaps, gap);
            Some((board_id, vec![version_id]))
        }
    };
    let Some((board_id, mut changed)) = owner else {
        return Ok(vec![]);
    };
    changed.insert(0, board_id);
    changed.push(gap_id);
    Ok(changed)
}

/// Whether the target (the generated envelope or one version) already holds this gap id.
fn target_holds_gap(doc: &ProjectDoc, target: &OutlineEditTarget, gap_id: &str) -> bool {
    let holds = |settings: &OutlineSettings| {
        settings
            .repair
            .as_ref()
            .is_some_and(|repair| repair.keep_gaps.iter().any(|gap| gap.id == gap_id))
    };
    match target {
        OutlineEditTarget::Board { board_id } => doc
            .boards
            .iter()
            .find(|board| &board.id == board_id)
            .is_some_and(|board| {
                board
                    .outline_ids
                    .iter()
                    .filter_map(|id| doc.outline.iter().find(|feature| feature.id() == id))
                    .any(|feature| {
                        matches!(feature, OutlineFeature::PartEnvelope { settings, .. } if holds(settings))
                    })
            }),
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => version(doc, board_id, version_id)
            .is_some_and(|version| holds(&version.geometry.settings)),
    }
}

fn upsert_gap(gaps: &mut Vec<ProtectedOutlineGap>, gap: ProtectedOutlineGap) {
    match gaps.iter_mut().find(|entry| entry.id == gap.id) {
        Some(entry) => *entry = gap,
        None => gaps.push(gap),
    }
}

/// Whether the accepted gaps already hold this exact authored gap, so setting it
/// again settles Unchanged instead of adding a revision step.
fn gap_is_accepted(gaps: &[ProtectedOutlineGap], gap: &ProtectedOutlineGap) -> bool {
    gaps.iter().any(|accepted| accepted == gap)
}

/// The authored keep-gap identities that protect one derived gap region.
///
/// A derived gap candidate is protected by its own id and by every authored keep gap
/// whose region protects it (`OutlineGap::protected_ids`). After the geometry moves, an
/// authored gap's id can stop matching the candidate it protects, so removal clears the
/// protection by the candidate's identity rather than one alias string.
fn protecting_gap_ids(doc: &ProjectDoc, board_id: &str, gap_id: &str) -> Option<Vec<String>> {
    let (cache, _, _) = geometry::outlines(doc, None, &[]);
    let scenes = geometry::board_outline_scenes(doc, &cache);
    let gap = scenes
        .iter()
        .find(|scene| scene.board_id == board_id)?
        .gaps
        .iter()
        .find(|gap| gap.id == gap_id)?;
    let mut ids = vec![gap.id.clone()];
    ids.extend(gap.protected_ids.iter().cloned());
    ids.sort();
    ids.dedup();
    Some(ids)
}

fn remove_protected_gap(
    doc: &mut ProjectDoc,
    edit: &OutlineEdit,
    gap_id: &str,
) -> Result<Vec<String>, String> {
    let target = edit.target.clone();
    let owner = match &target {
        OutlineEditTarget::Board { board_id } => {
            let board_id = board_id.clone();
            // A submitted id may be a derived gap candidate; removing that gap clears
            // the protection its identity carries, including every alias protecting it.
            let mut targets = protecting_gap_ids(doc, &board_id, gap_id).unwrap_or_default();
            if !targets.iter().any(|id| id == gap_id) {
                targets.push(gap_id.to_owned());
            }
            let feature = generated_feature_mut(doc, &board_id)?;
            let OutlineFeature::PartEnvelope { settings, id, .. } = feature else {
                return Err(format!("Board {board_id} has no generated envelope"));
            };
            let removed: Vec<String> = match settings.repair.as_mut() {
                Some(repair) => {
                    let before: Vec<String> =
                        repair.keep_gaps.iter().map(|gap| gap.id.clone()).collect();
                    repair.keep_gaps.retain(|gap| !targets.contains(&gap.id));
                    before
                        .into_iter()
                        .filter(|id| !repair.keep_gaps.iter().any(|gap| &gap.id == id))
                        .collect()
                }
                None => vec![],
            };
            if removed.is_empty() {
                return Ok(vec![]);
            }
            let mut changed = vec![board_id.clone(), id.clone()];
            changed.extend(removed);
            return Ok(changed);
        }
        OutlineEditTarget::Version {
            board_id,
            version_id,
        } => {
            let board_id = board_id.clone();
            let version_id = version_id.clone();
            let version = version_mut(doc, &board_id, &version_id)?;
            let mut removed = false;
            if let Some(repair) = version.geometry.settings.repair.as_mut() {
                let before = repair.keep_gaps.len();
                repair.keep_gaps.retain(|gap| gap.id != gap_id);
                removed |= before != repair.keep_gaps.len();
            }
            let before = version.geometry.protected_gaps.len();
            version
                .geometry
                .protected_gaps
                .retain(|gap| gap.id != gap_id);
            removed |= before != version.geometry.protected_gaps.len();
            if !removed {
                return Ok(vec![]);
            }
            (board_id, vec![version_id])
        }
    };
    let (board_id, mut changed) = owner;
    changed.insert(0, board_id);
    changed.push(gap_id.to_owned());
    Ok(changed)
}

fn allocated_feature_id(doc: &ProjectDoc, seed: u64, requested: &str) -> String {
    let identities = persisted_identities(doc);
    if identities.iter().any(|id| id == requested) {
        unique_id("outline-feature", seed, &identities)
    } else {
        requested.to_owned()
    }
}

fn named_polygon(feature: &OutlineFeature, id: String) -> Result<OutlineFeature, String> {
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

fn add_feature(
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

fn set_feature(
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

fn remove_feature(
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

fn add_connection(
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

fn remove_connection(
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
fn create_generated(
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

fn default_version_name(doc: &ProjectDoc, board_id: &str) -> String {
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

fn version_identity(doc: &ProjectDoc, version_id: &str) -> Result<(), String> {
    if let Some(problem) = outline_versions::version_identity_problem(doc, version_id) {
        return Err(problem);
    }
    Ok(())
}

fn push_fixed_version(
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

fn create_linked_refinement(
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

fn copy(
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

fn freeze(doc: &mut ProjectDoc, edit: &OutlineEdit, name: &str) -> Result<Vec<String>, String> {
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

fn edit_generated_perimeter(
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

fn edit_feature_perimeter(
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

fn activate(doc: &mut ProjectDoc, target: &OutlineEditTarget) -> Result<Vec<String>, String> {
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

fn rename(
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

fn delete(doc: &mut ProjectDoc, target: &OutlineEditTarget) -> Result<Vec<String>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_create_generated_change_round_trips_through_its_wire_shape() {
        let operation = EditOperation::ApplyOutlineEdit {
            edit: Box::new(OutlineEdit {
                target: OutlineEditTarget::Board {
                    board_id: "board".into(),
                },
                change: OutlineChange::CreateGenerated {
                    margin: 4.0,
                    settings: OutlineSettings {
                        corners: CornerStyle::Fillet,
                        size: 2.0,
                        bridge_width: 10.0,
                        repair: None,
                    },
                    part_ids: vec!["a".into(), "b".into()],
                },
                seed: 91,
                reviewed: None,
            }),
        };
        let value = serde_json::to_value(&operation).unwrap();
        assert_eq!(value["kind"], "apply-outline-edit");
        let change = &value["edit"]["change"]["create-generated"];
        // Struct-variant fields stay snake_case; the settings struct is camelCase and
        // an absent repair block is omitted.
        assert_eq!(change["margin"], json!(4.0));
        assert_eq!(change["part_ids"], json!(["a", "b"]));
        assert_eq!(change["settings"]["corners"], "fillet");
        assert_eq!(change["settings"]["size"], json!(2.0));
        assert_eq!(change["settings"]["bridgeWidth"], json!(10.0));
        assert!(change["settings"].get("repair").is_none());
        let parsed: EditOperation = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, operation);
    }

    #[test]
    fn an_outline_edit_round_trips_through_its_wire_shape() {
        let edit = OutlineEdit {
            target: OutlineEditTarget::Version {
                board_id: "board".into(),
                version_id: "refined".into(),
            },
            change: OutlineChange::SetMargin { margin: 2.5 },
            seed: 7,
            reviewed: None,
        };
        let value = serde_json::to_value(&edit).unwrap();
        assert_eq!(value["target"]["kind"], "version");
        assert_eq!(value["target"]["boardId"], "board");
        assert_eq!(value["target"]["versionId"], "refined");
        // The change vocabulary is externally tagged: one kebab-case key names it, and
        // its payload fields stay snake_case, like the mechanical vocabulary.
        assert_eq!(value["change"]["set-margin"]["margin"], 2.5);
        assert_eq!(value["seed"], 7);
        assert!(value.get("reviewed").is_none());
        let parsed: OutlineEdit = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, edit);

        let unit = serde_json::to_value(OutlineChange::Activate).unwrap();
        assert_eq!(unit, json!("activate"));
    }

    #[test]
    fn an_outline_edit_serializes_inside_an_edit_operation() {
        let operation = EditOperation::ApplyOutlineEdit {
            edit: Box::new(OutlineEdit {
                target: OutlineEditTarget::Board {
                    board_id: "board".into(),
                },
                change: OutlineChange::Freeze {
                    name: "Frozen".into(),
                },
                seed: 3,
                reviewed: None,
            }),
        };
        let value = serde_json::to_value(&operation).unwrap();
        assert_eq!(value["kind"], "apply-outline-edit");
        assert_eq!(value["edit"]["target"]["kind"], "board");
        assert_eq!(value["edit"]["change"]["freeze"]["name"], "Frozen");
        let parsed: EditOperation = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, operation);
    }
}
