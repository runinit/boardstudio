//! The presentation outline settings intent vocabulary.
use boardstudio_core::model::{Board, CornerStyle, OutlineFeature, OutlineSettings, ProjectDoc};

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

pub fn envelope_margin(feature: &OutlineFeature) -> Option<f64> {
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
