use crate::interactions::SnapGuide;
use boardstudio_model::model::{
    Asset, EditCommand, EditOperation, EditPhase, Position, ProjectDoc, SceneDelta, Vec2,
};
use std::{fmt, rc::Rc, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SaveAttemptId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionEpoch(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SnapshotToken(pub u64);

/// Where a completed operation landed: the accepted document revision and snapshot token
/// the operation produced or confirmed. Document-changing completions carry one; every
/// other settlement carries none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landing {
    pub revision: u64,
    pub token: SnapshotToken,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRef {
    pub id: String,
    pub sha256: String,
    pub media_type: String,
}
impl From<&Asset> for AssetRef {
    fn from(asset: &Asset) -> Self {
        Self {
            id: asset.id.clone(),
            sha256: asset.sha256.clone(),
            media_type: asset.media_type.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    pub session_epoch: SessionEpoch,
    pub document_id: String,
    pub board_id: String,
    pub instance_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcceptedSnapshot {
    pub token: SnapshotToken,
    pub session_epoch: SessionEpoch,
    pub document: Arc<ProjectDoc>,
    pub scene: Arc<SceneDelta>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Lifecycle {
    #[default]
    Empty,
    Ready,
    Closing,
    Closed,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Durability {
    #[default]
    NoDocument,
    Saved {
        revision: u64,
    },
    Saving {
        revision: u64,
    },
    Failed {
        revision: u64,
        reason: String,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GenerationStatus {
    #[default]
    Idle,
    Preparing {
        job_id: JobId,
    },
    Running {
        job_id: JobId,
    },
    Ready {
        job_id: JobId,
        exact: bool,
    },
    Blocked {
        job_id: JobId,
        reason: String,
    },
    Failed {
        job_id: JobId,
        reason: String,
    },
    Cancelled {
        job_id: JobId,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct GestureView {
    pub pointer_id: i64,
    pub generation: u64,
    pub target_ids: Vec<String>,
    pub changed: bool,
    pub snap: bool,
    pub alt: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraState {
    pub center: Vec2,
    pub zoom: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReadModel {
    pub lifecycle: Lifecycle,
    pub durability: Durability,
    pub accepted: Option<AcceptedSnapshot>,
    pub display_preview: Option<Arc<SceneDelta>>,
    pub selected_part_ids: Vec<String>,
    pub selection_anchor_id: Option<String>,
    pub selection_mode: SelectionMode,
    pub camera: CameraState,
    pub active_board_id: String,
    pub active_instance_id: Option<String>,
    pub gesture: Option<GestureView>,
    pub snap_guide: Option<SnapGuide>,
    pub generation: GenerationStatus,
    pub last_error: Option<String>,
}
impl Default for CameraState {
    fn default() -> Self {
        Self {
            center: Vec2::default(),
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalOutcome {
    Completed,
    Rejected(String),
    Superseded,
    Cancelled,
    PersistenceFailed(String),
    ExecutorFailed(String),
    Closed,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Replace,
    Add,
    Toggle,
    Range,
}

/// The reason Session reports when an intent's document session changed before it ran.
pub const DOCUMENT_SESSION_CHANGED: &str = "document session changed before command began";

/// What a resolver answers when Session hands it the accepted snapshot at execution time.
#[derive(Clone, Debug, PartialEq)]
pub enum Resolution {
    /// Run this commit command. Session sets its base revision from the snapshot it
    /// resolved against, so rapid edits each apply on top of the edits accepted before
    /// them.
    Submit(EditCommand),
    /// Nothing to change: the edit settles completed, landing at the current accepted
    /// snapshot, without a Core request and without a new revision.
    Unchanged,
    /// The edit's target has gone or is ineligible: the edit settles rejected with this
    /// reason and nothing is applied.
    Retire(String),
}

impl Resolution {
    /// Submit a committed edit intent. Session fills in the accepted revision
    /// and a transaction ID when one was not supplied by the resolver.
    pub fn submit(target_ids: Vec<String>, operation: EditOperation) -> Self {
        Self::submit_with_transaction_id(target_ids, "", operation)
    }

    /// Submit a committed edit intent that belongs to a caller-managed transaction.
    pub fn submit_with_transaction_id(
        target_ids: Vec<String>,
        transaction_id: impl Into<String>,
        operation: EditOperation,
    ) -> Self {
        Self::Submit(EditCommand {
            base_revision: 0,
            transaction_id: transaction_id.into(),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        })
    }
}

/// The owner-captured intent of a pending edit: a pure function of the accepted snapshot
/// and the values the owner captured when the user committed. Session calls it exactly
/// once against the current snapshot before Core receives the command. The
/// snapshot it receives is exactly the document Core will apply the resolved command to.
/// A resolver must not read UI state, signals or Runtime.
#[derive(Clone)]
pub struct EditResolver {
    label: String,
    resolve: Rc<dyn Fn(&AcceptedSnapshot) -> Resolution>,
}

impl EditResolver {
    pub fn new(
        label: impl Into<String>,
        resolve: impl Fn(&AcceptedSnapshot) -> Resolution + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            resolve: Rc::new(resolve),
        }
    }

    /// Rebase a project rename onto the accepted snapshot when the edit runs.
    /// The caller admits and trims the field value before creating this resolver.
    pub fn rename_project(label: impl Into<String>, name: String) -> Self {
        Self::new(label, move |accepted| {
            if accepted.document.name == name {
                return Resolution::Unchanged;
            }
            Resolution::submit(
                vec![accepted.document.id.clone()],
                EditOperation::RenameProject { name: name.clone() },
            )
        })
    }

    /// Run the resolver against an accepted snapshot. Session calls this at execution;
    /// presentation tests with a fake runtime call it to stand in for Session.
    pub fn resolve(&self, accepted: &AcceptedSnapshot) -> Resolution {
        (self.resolve)(accepted)
    }
}

impl fmt::Debug for EditResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EditResolver({})", self.label)
    }
}

impl PartialEq for EditResolver {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.resolve, &other.resolve)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Open {
        operation_id: OperationId,
        document: ProjectDoc,
    },
    /// Display a transient edit preview. Commits are submitted as [`Event::ResolveEdit`].
    PreviewEdit {
        operation_id: OperationId,
        transaction_id: String,
        target_ids: Vec<String>,
        operation: EditOperation,
    },
    /// Submit a pending edit as intent rather than as a ready-made command. See
    /// [`EditResolver`] for when the resolver runs and what it may read.
    ResolveEdit {
        operation_id: OperationId,
        label: String,
        resolver: EditResolver,
    },
    /// Retire a transient Core preview only while its captured accepted source is current.
    ClearPreview {
        operation_id: OperationId,
        token: SnapshotToken,
        revision: u64,
        board_id: String,
        transaction_id: String,
    },
    /// Explicitly review a protected PCB handoff before starting a remap revision.
    ReviewElectricalRemap {
        operation_id: OperationId,
        base_revision: u64,
        board_id: String,
        expected_fingerprint: String,
    },
    Undo {
        operation_id: OperationId,
    },
    Redo {
        operation_id: OperationId,
    },
    RetrySave {
        operation_id: OperationId,
    },
    SelectParts {
        operation_id: OperationId,
        part_ids: Vec<String>,
        range_part_ids: Vec<String>,
        mode: SelectionMode,
    },
    /// Select a projected matrix scope from an actual primary-cell hit. Keeping the hit
    /// separate from the projected IDs preserves the clicked cell as the Shift-range anchor.
    SelectMatrixCell {
        operation_id: OperationId,
        scope: Scope,
        matrix_id: String,
        target_part_id: String,
        part_ids: Vec<String>,
        mode: SelectionMode,
    },
    Navigate {
        operation_id: OperationId,
        board_id: String,
        instance_id: Option<String>,
    },
    SetCamera {
        operation_id: OperationId,
        center: Vec2,
        zoom: f64,
    },
    GestureBegin {
        operation_id: OperationId,
        pointer_id: i64,
        target_ids: Vec<String>,
        transaction_id: String,
        start: Vec<Position>,
        pitch: Vec2,
        snap_fraction: f64,
        geometry_snap: bool,
        gap: Option<f64>,
        pixels_per_world: Option<f64>,
        alt: bool,
    },
    GestureSample {
        pointer_id: i64,
        positions: Vec<Position>,
        pixels_per_world: Option<f64>,
        alt: bool,
    },
    GestureFrame {
        generation: u64,
    },
    GestureEnd {
        pointer_id: i64,
        final_positions: Vec<Position>,
        pixels_per_world: Option<f64>,
        alt: bool,
    },
    GestureCancel {
        pointer_id: i64,
    },
    StartGeneration {
        operation_id: OperationId,
        scope: Scope,
    },
    CancelGeneration {
        operation_id: OperationId,
    },
    StartExport {
        operation_id: OperationId,
        scope: Scope,
    },
    /// A document mutation owned by an active export. The child operation is
    /// persisted through the normal Core/Session path; only the matching export
    /// owner may advance its captured snapshot token.
    ExportCommit {
        operation_id: OperationId,
        export_operation_id: OperationId,
        token: SnapshotToken,
        scope: Scope,
        commit: ExportCommitRequest,
    },
    Close {
        operation_id: OperationId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExportCommitRequest {
    ApplyElectrical {
        plan: boardstudio_model::electrical::ElectricalPlan,
        draft: bool,
    },
    ProtectElectricalHandoff {
        plan: boardstudio_model::electrical::ElectricalPlan,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveResult {
    Committed,
    Aborted(String),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Completion {
    Persist {
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
    },
    GenerationFinished {
        job_id: JobId,
        scope: Scope,
        exact: bool,
    },
    GenerationFailed {
        job_id: JobId,
        scope: Scope,
        reason: String,
        blocked: bool,
    },
    Export {
        operation_id: OperationId,
        result: Result<(SnapshotToken, Scope), String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Persist {
        save_attempt_id: SaveAttemptId,
        document: Arc<ProjectDoc>,
    },
    Settled {
        operation_id: OperationId,
        outcome: TerminalOutcome,
        landing: Option<Landing>,
    },
    CapturePointer {
        pointer_id: i64,
    },
    ReleasePointer {
        pointer_id: i64,
    },
    RequestFrame {
        generation: u64,
    },
    CancelFrame {
        generation: u64,
    },
    RunGeneration {
        job_id: JobId,
        scope: Scope,
        snapshot: AcceptedSnapshot,
    },
    CancelJob {
        job_id: JobId,
    },
    CancelExport {
        operation_id: OperationId,
    },
    RunExport {
        operation_id: OperationId,
        scope: Scope,
        snapshot: AcceptedSnapshot,
    },
}
