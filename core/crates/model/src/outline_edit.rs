use crate::model::*;
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
