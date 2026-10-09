//! The presentation outline settings intent vocabulary, and the one remaining
//! whole-document landing path (creating the Generated envelope), which the typed Core
//! outline vocabulary does not cover yet.
use boardstudio_application::OperationId;
use boardstudio_core::model::{
    Board, CornerStyle, EditOperation, Operation, OutlineFeature, OutlineSettings, ProjectDoc,
};

#[derive(Clone, Debug, PartialEq)]
pub enum OutlineEdit {
    RenameVersion { version_id: String, name: String },
    CreateAutomatic,
    SetMargin(f64),
    SetCorners(CornerStyle),
    SetSize(f64),
    SetBridgeWidth(f64),
    SetRepairEnabled(bool),
    SetMaximumGapSpan(f64),
    SetMinimumConnectionWidth(f64),
    SetEdgeClearance(f64),
    SetProtectedGap { gap_id: String, protected: bool },
    RemoveProtectedGap { gap_id: String },
}

pub fn generated_feature<'a>(
    document: &'a ProjectDoc,
    board: &Board,
) -> Option<&'a OutlineFeature> {
    board
        .outline_ids
        .iter()
        .filter_map(|id| document.outline.iter().find(|feature| feature.id() == id))
        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
}

pub fn generated_settings(feature: &OutlineFeature) -> Option<OutlineSettings> {
    match feature {
        OutlineFeature::PartEnvelope { settings, .. } => Some(settings.clone()),
        _ => None,
    }
}

pub fn generated_margin(feature: &OutlineFeature) -> Option<f64> {
    match feature {
        OutlineFeature::PartEnvelope { margin, .. } => Some(*margin),
        _ => None,
    }
}

pub fn reference_outline_settings() -> OutlineSettings {
    OutlineSettings {
        corners: CornerStyle::Fillet,
        size: 2.0,
        bridge_width: 10.0,
        repair: None,
    }
}

/// The one remaining whole-document landing path: creating the board's Generated
/// envelope registers the feature and the board membership in one accepted edit. Core's
/// typed outline vocabulary has no change for this yet (reported to the parent), so the
/// adapter keeps this single, narrow exception instead of inventing Core policy.
pub fn create_generated_outline(
    document: &ProjectDoc,
    board_id: &str,
    operation_id: OperationId,
) -> Option<EditOperation> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    if generated_feature(document, board).is_some() {
        return None;
    }
    let feature_id = format!("board:{board_id}:automatic:outline:{}", operation_id.0);
    if document
        .outline
        .iter()
        .any(|feature| feature.id() == feature_id)
    {
        return None;
    }
    let feature = OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: reference_outline_settings(),
        id: feature_id.clone(),
        part_ids: board.part_ids.clone(),
        margin: 4.0,
        operation: Operation::Add,
    };
    let mut replacement = document.clone();
    replacement.outline.push(feature);
    if let Some(target) = replacement
        .boards
        .iter_mut()
        .find(|item| item.id == board_id)
    {
        target.outline_ids.push(feature_id);
    }
    Some(EditOperation::ReplaceDocument {
        document: Box::new(replacement),
    })
}
