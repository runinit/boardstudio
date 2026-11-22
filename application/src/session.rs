//! Headless application session for ordered document work and host-owned effects.
//!
//! This crate deliberately contains no browser handles. A host runs each emitted effect and
//! returns its completion through [`Session::complete`].

mod completion;
mod dispatch;
mod queue;

use crate::interactions::{DragSnapOptions, MagneticSnapState, SnapGuide, normalize_drag};
use boardstudio_model::model::*;
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OperationId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExecutorEpoch(pub u64);
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    Empty,
    Opening,
    Ready,
    Applying,
    Saving,
    RecoveryRequired,
    Closing,
    Closed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Durability {
    NoDocument,
    Saved { revision: u64 },
    Saving { revision: u64 },
    Failed { revision: u64, reason: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationStatus {
    Idle,
    Preparing { job_id: JobId },
    Running { job_id: JobId },
    Ready { job_id: JobId, exact: bool },
    Blocked { job_id: JobId, reason: String },
    Failed { job_id: JobId, reason: String },
    Cancelled { job_id: JobId },
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

#[derive(Clone, Debug, PartialEq)]
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
impl Default for ReadModel {
    fn default() -> Self {
        Self {
            lifecycle: Lifecycle::Empty,
            durability: Durability::NoDocument,
            accepted: None,
            display_preview: None,
            selected_part_ids: vec![],
            selection_anchor_id: None,
            selection_mode: SelectionMode::Replace,
            camera: CameraState {
                center: Vec2::default(),
                zoom: 1.0,
            },
            active_board_id: String::new(),
            active_instance_id: None,
            gesture: None,
            snap_guide: None,
            generation: GenerationStatus::Idle,
            last_error: None,
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
    BlockedByRecovery(String),
    ExecutorFailed(String),
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
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
        Self::Submit(EditCommand {
            base_revision: 0,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        })
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
/// once, when the edit reaches the head of the queue with nothing else running — the
/// snapshot it receives is exactly the document Core will apply the resolved command to.
/// A resolver must not read UI state, signals or Runtime.
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

impl Clone for EditResolver {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            resolve: self.resolve.clone(),
        }
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
    /// Explicitly discard unknown engine history and open the supplied durable document.
    RecoverWithDocument {
        operation_id: OperationId,
        document: ProjectDoc,
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
    Core {
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reply: Box<CoreReply>,
    },
    Persist {
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
    },
    CoreFailed {
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reason: String,
    },
    GenerationProgress {
        job_id: JobId,
        scope: Scope,
        message: String,
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
    },
    GenerationBlocked {
        job_id: JobId,
        scope: Scope,
        reason: String,
    },
    ExportFinished {
        operation_id: OperationId,
        token: SnapshotToken,
        scope: Scope,
        artifact_id: String,
    },
    ExportFailed {
        operation_id: OperationId,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Core {
        operation_id: OperationId,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        request: Box<CoreRequest>,
    },
    Persist {
        save_attempt_id: SaveAttemptId,
        document: Arc<ProjectDoc>,
        assets: Vec<AssetRef>,
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
        operation_id: OperationId,
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
    DeliverExport {
        operation_id: OperationId,
        artifact_id: String,
        token: SnapshotToken,
    },
}

#[derive(Clone, Debug)]
enum IntentKind {
    Open(Box<ProjectDoc>),
    Edit(EditCommand),
    ResolveEdit {
        label: String,
        resolver: EditResolver,
    },
    ClearPreview {
        token: SnapshotToken,
        revision: u64,
        board_id: String,
        transaction_id: String,
    },
    ReviewElectricalRemap {
        base_revision: u64,
        board_id: String,
        expected_fingerprint: String,
    },
    Undo,
    Redo,
    GesturePreview(EditCommand),
    GestureCommit(EditCommand),
    Generate(Scope),
    Export(Scope),
    ExportCommit {
        owner: ExportCommitOwner,
        commit: ExportCommitRequest,
    },
}
#[derive(Clone, Debug)]
struct ExportCommitOwner {
    operation_id: OperationId,
    token: SnapshotToken,
    scope: Scope,
}
#[derive(Clone, Debug)]
struct Intent {
    operation_id: OperationId,
    submitted_epoch: SessionEpoch,
    kind: IntentKind,
    strict_revision: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveKind {
    Open,
    Commit,
    ReviewElectricalRemap,
    Preview,
    Undo,
    Redo,
    GestureCommit,
    ExportCommit,
}
#[derive(Clone, Debug)]
struct ActiveCore {
    request_id: RequestId,
    executor_epoch: ExecutorEpoch,
    operation_id: OperationId,
    kind: ActiveKind,
    gesture_generation: Option<u64>,
    preview_transaction_id: Option<String>,
    preview_cancelled: bool,
    export_owner: Option<ExportCommitOwner>,
}
#[derive(Debug)]
struct PendingSave {
    save_attempt_id: SaveAttemptId,
    revision: u64,
    operation: Option<OperationId>,
}
#[derive(Clone, Debug)]
struct Gesture {
    pointer_id: i64,
    generation: u64,
    transaction_id: String,
    base_revision: u64,
    targets: Vec<String>,
    start: Vec<Position>,
    latest: Vec<Position>,
    sent_positions: Option<Vec<Position>>,
    commit_queued: bool,
    operation_id: OperationId,
    pitch: Vec2,
    snap_fraction: f64,
    geometry_snap: bool,
    gap: Option<f64>,
    pixels_per_world: Option<f64>,
    snap_state: MagneticSnapState,
    alt: bool,
    frame_pending: bool,
    changed: bool,
    preview_in_flight: bool,
    final_positions: Option<Vec<Position>>,
}

pub struct Session {
    model: ReadModel,
    queue: VecDeque<Intent>,
    active_core: Option<ActiveCore>,
    display_preview_transaction_id: Option<String>,
    pending_save: Option<PendingSave>,
    next_request: u64,
    next_save: u64,
    next_token: u64,
    next_gesture: u64,
    next_job: u64,
    core_epoch: ExecutorEpoch,
    active_job: Option<(JobId, OperationId, Scope)>,
    exports: Vec<(OperationId, Scope, SnapshotToken)>,
    settled: HashSet<OperationId>,
    close_operation: Option<OperationId>,
    gesture: Option<Gesture>,
    validate_open: OpenValidator,
}

/// Core's side-effect-free admission check for an Open candidate. The host that owns the
/// engine passes `boardstudio_core::CoreEngine::validate_open`, so this crate builds without
/// the engine.
pub type OpenValidator = fn(&ProjectDoc) -> Result<(), String>;

impl Session {
    pub fn new(validate_open: OpenValidator) -> Self {
        Self {
            model: ReadModel::default(),
            queue: VecDeque::new(),
            active_core: None,
            display_preview_transaction_id: None,
            pending_save: None,
            next_request: 1,
            next_save: 1,
            next_token: 1,
            next_gesture: 1,
            next_job: 1,
            core_epoch: ExecutorEpoch(1),
            active_job: None,
            exports: vec![],
            settled: HashSet::new(),
            close_operation: None,
            gesture: None,
            validate_open,
        }
    }
    pub fn read_model(&self) -> &ReadModel {
        &self.model
    }
    pub fn core_executor_epoch(&self) -> ExecutorEpoch {
        self.core_epoch
    }

    /// Check before executing a deferred Core effect. A rejected stale reply alone
    /// cannot protect an in-process engine from a stale request mutating it.
    pub fn core_request_is_current(
        &self,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
    ) -> bool {
        self.core_epoch == executor_epoch
            && self.active_core.as_ref().is_some_and(|active| {
                active.request_id == request_id && active.executor_epoch == executor_epoch
            })
    }
    pub fn scope(&self) -> Option<Scope> {
        let accepted = self.model.accepted.as_ref()?;
        Some(Scope {
            session_epoch: accepted.session_epoch,
            document_id: accepted.document.id.clone(),
            board_id: self.model.active_board_id.clone(),
            instance_id: self.model.active_instance_id.clone(),
        })
    }
    /// Whether an asynchronous export is still owned by this session and its
    /// captured document, board, instance, and snapshot remain current.
    /// Checking membership also closes the cancellation window before a host
    /// has processed the corresponding `CancelExport` effect.
    pub fn export_is_current(
        &self,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> bool {
        self.exports
            .iter()
            .any(|(operation, captured_scope, captured_token)| {
                *operation == operation_id && captured_scope == scope && *captured_token == token
            })
            && self.snapshot_matches(token, scope)
    }

    fn gesture_sample(
        &mut self,
        pointer_id: i64,
        positions: Vec<Position>,
        alt: bool,
        end: bool,
        effects: &mut Vec<Effect>,
    ) {
        let board_id = self.model.active_board_id.clone();
        let Some(g) = &mut self.gesture else {
            return;
        };
        if g.pointer_id != pointer_id {
            return;
        }
        g.alt = alt;
        let (positions, guide) =
            self.model
                .accepted
                .as_ref()
                .map_or((positions.clone(), None), |snapshot| {
                    normalize_drag(
                        &snapshot.document,
                        &g.start,
                        positions,
                        DragSnapOptions {
                            board_id: &board_id,
                            pitch: g.pitch,
                            fraction: g.snap_fraction,
                            geometry_snap: g.geometry_snap,
                            gap: g.gap,
                            targets: &g.targets,
                            alt: g.alt,
                            pixels_per_world: g.pixels_per_world,
                            state: &mut g.snap_state,
                        },
                    )
                });
        self.model.snap_guide = guide;
        g.changed = positions != g.start;
        g.latest = positions.clone();
        if end {
            g.final_positions = Some(positions);
        }
        let generation = g.generation;
        let changed = g.changed;
        if !g.frame_pending && !end {
            g.frame_pending = true;
            effects.push(Effect::RequestFrame { generation });
        }
        let preview_in_flight = g.preview_in_flight;
        let commit_queued = g.commit_queued;
        self.model.gesture = self.gesture.as_ref().map(gesture_view);
        if end && !changed {
            self.invalidate_gesture(effects);
        } else if end && !preview_in_flight && !commit_queued {
            self.dispatch_latest_gesture_preview(effects);
        }
    }
    fn gesture_frame(&mut self, generation: u64, effects: &mut Vec<Effect>) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if g.generation != generation {
            return;
        }
        g.frame_pending = false;
        self.dispatch_latest_gesture_preview(effects);
    }
    fn dispatch_latest_gesture_preview(&mut self, _effects: &mut Vec<Effect>) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if g.preview_in_flight
            || g.commit_queued
            || (!g.changed && g.final_positions.is_none() && g.sent_positions.is_none())
        {
            return;
        }
        let is_commit = g.final_positions.is_some();
        let positions = g
            .final_positions
            .clone()
            .unwrap_or_else(|| g.latest.clone());
        if !is_commit && g.sent_positions.as_ref() == Some(&positions) {
            return;
        }
        let generation = g.generation;
        let operation_id = g.operation_id;
        let command = EditCommand {
            base_revision: g.base_revision,
            transaction_id: g.transaction_id.clone(),
            phase: if is_commit {
                EditPhase::Commit
            } else {
                EditPhase::Preview
            },
            target_ids: g.targets.clone(),
            operation: EditOperation::MoveParts {
                positions: positions.clone(),
            },
        };
        if is_commit {
            g.commit_queued = true;
            g.final_positions = None;
            self.queue.push_front(Intent {
                operation_id,
                submitted_epoch: self.epoch(),
                kind: IntentKind::GestureCommit(command),
                strict_revision: true,
            });
        } else {
            g.preview_in_flight = true;
            g.sent_positions = Some(positions);
            let internal_operation = OperationId(u64::MAX - generation);
            self.queue.push_front(Intent {
                operation_id: internal_operation,
                submitted_epoch: self.epoch(),
                kind: IntentKind::GesturePreview(command),
                strict_revision: true,
            });
        }
    }
    fn invalidate_gesture(&mut self, effects: &mut Vec<Effect>) {
        if let Some(g) = self.gesture.take() {
            if g.frame_pending {
                effects.push(Effect::CancelFrame {
                    generation: g.generation,
                });
            }
            if self
                .active_core
                .as_ref()
                .is_some_and(|active| active.gesture_generation == Some(g.generation))
                && let Some(active) = &self.active_core
            {
                self.settle(active.operation_id, TerminalOutcome::Cancelled, effects);
            }
            let mut retained = VecDeque::new();
            while let Some(intent) = self.queue.pop_front() {
                if matches!(
                    intent.kind,
                    IntentKind::GesturePreview(_) | IntentKind::GestureCommit(_)
                ) {
                    self.settle(intent.operation_id, TerminalOutcome::Cancelled, effects);
                } else {
                    retained.push_back(intent);
                }
            }
            self.queue = retained;
            self.settle(g.operation_id, TerminalOutcome::Cancelled, effects);
            effects.push(Effect::ReleasePointer {
                pointer_id: g.pointer_id,
            });
            self.model.gesture = None;
            if self.display_preview_transaction_id.as_deref() == Some(&g.transaction_id) {
                self.model.display_preview = None;
                self.display_preview_transaction_id = None;
            }
            self.model.snap_guide = None;
            if self.model.lifecycle == Lifecycle::Applying
                && self
                    .active_core
                    .as_ref()
                    .is_some_and(|a| a.gesture_generation == Some(g.generation))
            {
                // The worker result may arrive, but its generation no longer authorizes display.
            }
        }
    }
    fn finish_gesture_commit(
        &mut self,
        generation: Option<u64>,
        succeeded: bool,
        effects: &mut Vec<Effect>,
    ) {
        if generation.is_some_and(|generation| {
            self.gesture
                .as_ref()
                .is_some_and(|g| g.generation != generation)
        }) {
            return;
        }
        if succeeded || self.gesture.is_some() {
            self.invalidate_gesture(effects);
        }
    }
    fn cancel_exports(&mut self, effects: &mut Vec<Effect>) {
        let exports = std::mem::take(&mut self.exports);
        for (operation_id, _, _) in exports {
            effects.push(Effect::CancelExport { operation_id });
            self.settle(operation_id, TerminalOutcome::Cancelled, effects);
        }
    }
    fn cancel_exports_except(&mut self, retained: OperationId, effects: &mut Vec<Effect>) {
        let mut keep = Vec::new();
        for export in std::mem::take(&mut self.exports) {
            if export.0 == retained {
                keep.push(export);
            } else {
                effects.push(Effect::CancelExport {
                    operation_id: export.0,
                });
                self.settle(export.0, TerminalOutcome::Cancelled, effects);
            }
        }
        self.exports = keep;
    }
    fn cancel_job(&mut self, effects: &mut Vec<Effect>) {
        if let Some((job_id, operation_id, _)) = self.active_job.take() {
            effects.push(Effect::CancelJob { job_id });
            self.model.generation = GenerationStatus::Cancelled { job_id };
            self.settle(operation_id, TerminalOutcome::Cancelled, effects);
        }
    }
    fn start_generation_now(
        &mut self,
        operation_id: OperationId,
        scope: Scope,
        effects: &mut Vec<Effect>,
    ) {
        let is_saved = self.model.accepted.as_ref().is_some_and(|s| {
            self.model.durability
                == Durability::Saved {
                    revision: s.document.revision,
                }
        });
        if self.model.lifecycle != Lifecycle::Ready || !is_saved {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery(
                    "generation requires an accepted saved snapshot".into(),
                ),
                effects,
            );
            return;
        }
        let Some(snapshot) = self.model.accepted.clone() else {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("no accepted document".into()),
                effects,
            );
            return;
        };
        if !self.snapshot_matches(snapshot.token, &scope) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("generation scope is stale".into()),
                effects,
            );
            return;
        }
        self.cancel_job(effects);
        let job_id = JobId(self.next_job);
        self.next_job += 1;
        self.active_job = Some((job_id, operation_id, scope.clone()));
        self.model.generation = GenerationStatus::Preparing { job_id };
        effects.push(Effect::RunGeneration {
            operation_id,
            job_id,
            scope,
            snapshot,
        });
    }
    fn start_export_now(
        &mut self,
        operation_id: OperationId,
        scope: Scope,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle != Lifecycle::Ready || self.model.accepted.is_none() {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery(
                    "export requires a ready accepted snapshot".into(),
                ),
                effects,
            );
            return;
        }
        let snapshot = self.model.accepted.clone().unwrap();
        if self.model.scene_is_pending() || !self.snapshot_matches(snapshot.token, &scope) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("export snapshot or scope is not current".into()),
                effects,
            );
            return;
        }
        self.exports
            .push((operation_id, scope.clone(), snapshot.token));
        effects.push(Effect::RunExport {
            operation_id,
            scope,
            snapshot,
        });
    }
    fn scope_is_current(&self, scope: &Scope) -> bool {
        self.model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot.session_epoch == scope.session_epoch
                && snapshot.document.id == scope.document_id
        }) && self.model.active_board_id == scope.board_id
            && self.model.active_instance_id == scope.instance_id
    }
    fn snapshot_matches(&self, token: SnapshotToken, scope: &Scope) -> bool {
        self.model
            .accepted
            .as_ref()
            .is_some_and(|s| s.token == token)
            && self.scope_is_current(scope)
    }
    fn job_is_current(&self, job_id: JobId, scope: &Scope) -> bool {
        self.active_job.as_ref().is_some_and(|(id, _, captured)| {
            *id == job_id
                && captured == scope
                && self.snapshot_matches(
                    self.model
                        .accepted
                        .as_ref()
                        .map_or(SnapshotToken(0), |s| s.token),
                    scope,
                )
        })
    }
    fn close(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        self.invalidate_gesture(effects);
        self.cancel_job(effects);
        self.cancel_exports(effects);
        if self.model.lifecycle == Lifecycle::RecoveryRequired
            || matches!(self.model.durability, Durability::Failed { .. })
        {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("save the project before closing".into()),
                effects,
            );
            return;
        }
        if self.pending_save.is_some() || self.active_core.is_some() {
            self.close_operation = Some(operation_id);
            self.model.lifecycle = Lifecycle::Closing;
            self.block_queue("session is closing", effects);
        } else {
            self.finish_close(operation_id, effects);
        }
    }
    fn finish_close(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        self.model.lifecycle = Lifecycle::Closed;
        self.model.display_preview = None;
        self.display_preview_transaction_id = None;
        self.active_job = None;
        self.settle(operation_id, TerminalOutcome::Completed, effects);
    }
}

impl ReadModel {
    fn scene_is_pending(&self) -> bool {
        self.accepted
            .as_ref()
            .is_none_or(|s| s.scene.revision != s.document.revision)
    }
}
fn gesture_view(g: &Gesture) -> GestureView {
    GestureView {
        pointer_id: g.pointer_id,
        generation: g.generation,
        target_ids: g.targets.clone(),
        changed: g.changed,
        snap: g.snap_fraction != 0.0,
        alt: g.alt,
    }
}
fn event_operation(event: &Event) -> Option<OperationId> {
    match event {
        Event::Open { operation_id, .. }
        | Event::GestureBegin { operation_id, .. }
        | Event::RecoverWithDocument { operation_id, .. }
        | Event::PreviewEdit { operation_id, .. }
        | Event::ClearPreview { operation_id, .. }
        | Event::ReviewElectricalRemap { operation_id, .. }
        | Event::Undo { operation_id }
        | Event::Redo { operation_id }
        | Event::RetrySave { operation_id }
        | Event::ResolveEdit { operation_id, .. }
        | Event::SelectParts { operation_id, .. }
        | Event::SelectMatrixCell { operation_id, .. }
        | Event::Navigate { operation_id, .. }
        | Event::SetCamera { operation_id, .. }
        | Event::StartGeneration { operation_id, .. }
        | Event::CancelGeneration { operation_id }
        | Event::StartExport { operation_id, .. }
        | Event::ExportCommit { operation_id, .. }
        | Event::Close { operation_id } => Some(*operation_id),
        _ => None,
    }
}
fn reply_id(reply: &CoreReply) -> String {
    match reply {
        CoreReply::KeycapsResolved { id, .. }
        | CoreReply::ModulesResolved { id, .. }
        | CoreReply::FirmwareGenerated { id, .. }
        | CoreReply::ElectricalResolved { id, .. }
        | CoreReply::ElectricalApplied { id, .. }
        | CoreReply::ElectricalHandoffProtected { id, .. }
        | CoreReply::MechanicalProfile { id, .. }
        | CoreReply::MechanicalResolved { id, .. }
        | CoreReply::Preview { id, .. }
        | CoreReply::Scene { id, .. }
        | CoreReply::MatrixProjections { id, .. }
        | CoreReply::Error { id, .. }
        | CoreReply::CasePrepared { id, .. } => id.clone(),
    }
}
