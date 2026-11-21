pub use crate::component_offset::BoardPartChoice;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    CompanionPlacementTarget, Constraint, Follow, HalfExceptionState, MirroredCompanionMode,
    MirroredCompanionTweak, PartOutline, Vec2,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwner {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub part_id: String,
    pub selected_part_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutComponentInspectorProjection {
    pub owner: LayoutComponentInspectorOwner,
    pub reference: String,
    pub definition_name: String,
    pub definition_kind: String,
    pub envelope_notice: Option<String>,
    pub locked: bool,
    pub position: Vec2,
    pub selection_count: usize,
    pub layout_id: Option<String>,
    pub layouts: Vec<LayoutChoice>,
    pub outline: PartOutline,
    pub board_parts: Vec<BoardPartChoice>,
    pub active_constraint: Option<Constraint>,
    pub relationship_summary: String,
    pub mirrored_half: Option<MirroredHalfProjection>,
    /// Ticket 12: an independent part on a stored-gap half shows a
    /// "Mirror on both halves" toggle instead of the member exception
    /// buttons. `Some(true)` mirrors it today; `Some(false)` keeps it local.
    pub part_mirror: Option<bool>,
    /// Ticket 13: the part's column-vocabulary anchor and the anchor-key
    /// choices on its layout for Pick. `None` is a detached part; parts
    /// without a layout home cannot anchor.
    pub anchor: Option<PartAnchorProjection>,
    pub anchor_choices: Vec<AnchorKeyChoice>,
}

/// Ticket 13: what the part follows (anchor key plus Follow subset), what the
/// anchor carries (Spread/Stagger/Splay) and what stays local to the part
/// (Offset/Rotation).
#[derive(Clone, Debug, PartialEq)]
pub struct PartAnchorProjection {
    pub matrix_id: String,
    pub matrix_name: String,
    pub row: u32,
    pub column: u32,
    pub follow: Follow,
    pub spread: f64,
    pub stagger: f64,
    pub splay: f64,
    pub offset: Vec2,
    pub rotation: f64,
}

impl PartAnchorProjection {
    pub fn key_label(&self) -> String {
        format!("{} R{}C{}", self.matrix_name, self.row + 1, self.column + 1)
    }
}

/// Ticket 13: one anchor-key choice for Pick: a matrix on the part's layout
/// with its live dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorKeyChoice {
    pub matrix_id: String,
    pub matrix_name: String,
    pub rows: u32,
    pub columns: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredHalfProjection {
    pub pair_id: String,
    pub mirror_id: String,
    pub state: HalfExceptionState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionPlacementOwner {
    pub scope: boardstudio_application::Scope,
    pub context: boardstudio_web_ui_model::tree::ScopedTreeContext,
    pub selected_part_ids: Vec<String>,
    pub snapshot_token: boardstudio_application::SnapshotToken,
    pub revision: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompanionPlacementProjection {
    pub owner: CompanionPlacementOwner,
    pub pair_id: String,
    pub targets: Vec<CompanionPlacementTarget>,
    pub mode: Option<MirroredCompanionMode>,
    pub mixed_mode: bool,
    pub tweak: Option<MirroredCompanionTweak>,
    pub mixed_tweak: bool,
    pub disabled_reason: Option<&'static str>,
}
