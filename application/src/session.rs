//! Main-thread document session and host-owned effects.
//!
//! This crate deliberately contains no browser handles. A host runs each emitted effect and
//! returns its completion through [`Session::complete`].

use crate::interactions::{DragSnapOptions, MagneticSnapState, SnapGuide, normalize_drag};
use boardstudio_model::model::*;
use std::collections::HashSet;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

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
    Opening,
    Ready,
    Applying,
    Saving,
    RecoveryRequired,
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
    BlockedByRecovery(String),
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
/// once against the current snapshot before Core receives the command. The
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
    Persist {
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
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
struct ExportCommitOwner {
    operation_id: OperationId,
    token: SnapshotToken,
    scope: Scope,
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
}

pub struct Session {
    model: ReadModel,
    core: boardstudio_core::CoreEngine,
    display_preview_transaction_id: Option<String>,
    pending_save: Option<PendingSave>,
    next_save: u64,
    next_token: u64,
    next_gesture: u64,
    next_job: u64,
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
            core: boardstudio_core::CoreEngine::new(),
            display_preview_transaction_id: None,
            pending_save: None,
            next_save: 1,
            next_token: 1,
            next_gesture: 1,
            next_job: 1,
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

    pub fn project_matrices(
        &mut self,
        id: String,
        base_revision: u64,
        matrices: Vec<boardstudio_model::model::Matrix>,
    ) -> CoreReply {
        self.core.handle(CoreRequest::ProjectMatrices {
            id,
            base_revision,
            matrices,
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
        g.latest = positions;
        let generation = g.generation;
        let changed = g.changed;
        if !g.frame_pending && !end {
            g.frame_pending = true;
            effects.push(Effect::RequestFrame { generation });
        }
        self.model.gesture = self.gesture.as_ref().map(gesture_view);
        if end && !changed {
            self.invalidate_gesture(effects);
        } else if end {
            self.apply_gesture(true, effects);
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
        self.apply_gesture(false, effects);
    }
    fn apply_gesture(&mut self, is_commit: bool, effects: &mut Vec<Effect>) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if !is_commit && !g.changed && g.sent_positions.is_none() {
            return;
        }
        let positions = g.latest.clone();
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
        let operation = if is_commit {
            operation_id
        } else {
            g.sent_positions = Some(positions);
            OperationId(u64::MAX - generation)
        };
        self.apply_request(
            operation,
            CoreRequest::Edit {
                id: wire_id(operation),
                command,
            },
            Some(generation),
            None,
            effects,
        );
    }
    fn invalidate_gesture(&mut self, effects: &mut Vec<Effect>) {
        if let Some(g) = self.gesture.take() {
            if g.frame_pending {
                effects.push(Effect::CancelFrame {
                    generation: g.generation,
                });
            }
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
        if self.pending_save.is_some() {
            self.close_operation = Some(operation_id);
            self.model.lifecycle = Lifecycle::Closing;
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
fn update_selection(selection: &mut Vec<String>, incoming: Vec<String>, mode: SelectionMode) {
    if matches!(mode, SelectionMode::Replace | SelectionMode::Range) {
        *selection = incoming;
        return;
    }
    for id in incoming {
        match selection.iter().position(|current| current == &id) {
            Some(index) if mode == SelectionMode::Toggle => {
                selection.remove(index);
            }
            None => selection.push(id),
            _ => {}
        }
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
pub fn event_operation(event: &Event) -> Option<OperationId> {
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
fn wire_id(operation: OperationId) -> String {
    format!("m1-{}", operation.0)
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

impl Session {
    fn open(&mut self, operation_id: OperationId, document: ProjectDoc, effects: &mut Vec<Effect>) {
        if matches!(
            self.model.durability,
            Durability::Saving { .. } | Durability::Failed { .. }
        ) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("save the current project before opening another".into()),
                effects,
            );
            return;
        }
        if let Err(reason) = (self.validate_open)(&document) {
            self.settle(operation_id, TerminalOutcome::Rejected(reason), effects);
            return;
        }
        self.invalidate_gesture(effects);
        self.cancel_job(effects);
        self.cancel_exports(effects);
        self.apply_request(
            operation_id,
            CoreRequest::Open {
                id: wire_id(operation_id),
                document,
            },
            None,
            None,
            effects,
        );
    }

    fn resolve_edit(
        &mut self,
        operation_id: OperationId,
        label: String,
        resolver: EditResolver,
        effects: &mut Vec<Effect>,
    ) {
        if matches!(
            self.model.lifecycle,
            Lifecycle::Closing | Lifecycle::RecoveryRequired
        ) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("no editable project is open".into()),
                effects,
            );
            return;
        }
        let Some(accepted) = self.model.accepted.as_ref() else {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("open a project before editing".into()),
                effects,
            );
            return;
        };
        let command = match resolver.resolve(accepted) {
            Resolution::Submit(mut command) if command.phase == EditPhase::Commit => {
                command.base_revision = accepted.document.revision;
                if command.transaction_id.is_empty() {
                    command.transaction_id = format!("m1-{label}-{}", operation_id.0);
                }
                command
            }
            Resolution::Unchanged => {
                self.settle_landed(
                    operation_id,
                    Landing {
                        revision: accepted.document.revision,
                        token: accepted.token,
                    },
                    effects,
                );
                return;
            }
            resolution => {
                let reason = match resolution {
                    Resolution::Retire(reason) => reason,
                    _ => "a resolved edit must be a commit".into(),
                };
                self.settle(operation_id, TerminalOutcome::Rejected(reason), effects);
                return;
            }
        };
        self.apply_request(
            operation_id,
            CoreRequest::Edit {
                id: wire_id(operation_id),
                command,
            },
            None,
            None,
            effects,
        );
    }

    fn apply_request(
        &mut self,
        operation_id: OperationId,
        request: CoreRequest,
        gesture_generation: Option<u64>,
        export_owner: Option<ExportCommitOwner>,
        effects: &mut Vec<Effect>,
    ) {
        let opening = matches!(request, CoreRequest::Open { .. });
        if matches!(self.model.lifecycle, Lifecycle::Closing | Lifecycle::Closed)
            || !opening
                && (self.model.accepted.is_none()
                    || self.model.lifecycle == Lifecycle::RecoveryRequired)
        {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("no editable project is open".into()),
                effects,
            );
            return;
        }
        let revision = match &request {
            CoreRequest::Edit { command, .. } => Some(command.base_revision),
            CoreRequest::ReviewElectricalRemap { base_revision, .. }
            | CoreRequest::ApplyElectrical { base_revision, .. }
            | CoreRequest::ProtectElectricalHandoff { base_revision, .. } => Some(*base_revision),
            _ => None,
        };
        if revision.is_some_and(|revision| {
            self.model
                .accepted
                .as_ref()
                .is_none_or(|snapshot| snapshot.document.revision != revision)
        }) {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("captured edit revision is stale".into()),
                effects,
            );
            self.finish_gesture_commit(None, false, effects);
            return;
        }
        let preview_transaction_id = match &request {
            CoreRequest::Edit { command, .. } if command.phase == EditPhase::Preview => {
                Some(command.transaction_id.clone())
            }
            _ => None,
        };
        let reply = self.core.handle(request);
        if reply_id(&reply) != wire_id(operation_id) {
            self.model.lifecycle = Lifecycle::RecoveryRequired;
            self.model.last_error = Some("core reply ID did not match request".into());
            self.settle(
                operation_id,
                TerminalOutcome::ExecutorFailed("core reply ID did not match request".into()),
                effects,
            );
            return;
        }
        match reply {
            CoreReply::Preview { scene, .. } if preview_transaction_id.is_some() => {
                self.model.lifecycle = Lifecycle::Ready;
                self.model.display_preview = Some(Arc::new(scene));
                self.display_preview_transaction_id = preview_transaction_id;
                if gesture_generation.is_none() {
                    self.settle(operation_id, TerminalOutcome::Completed, effects);
                }
            }
            CoreReply::Scene {
                scene, document, ..
            } if preview_transaction_id.is_none() => {
                let document = Arc::new(*document);
                if scene.revision != document.revision {
                    self.model.lifecycle = Lifecycle::RecoveryRequired;
                    self.model.last_error = Some("core scene and document revisions differ".into());
                    self.settle(
                        operation_id,
                        TerminalOutcome::ExecutorFailed(
                            "core scene and document revisions differ".into(),
                        ),
                        effects,
                    );
                    return;
                }
                let is_open = opening;
                let epoch = if is_open {
                    SessionEpoch(self.epoch().0 + 1)
                } else {
                    self.epoch()
                };
                self.cancel_job(effects);
                if let Some(owner) = &export_owner {
                    self.cancel_exports_except(owner.operation_id, effects);
                } else {
                    self.cancel_exports(effects);
                }
                let token = SnapshotToken(self.next_token);
                self.next_token += 1;
                let revision = document.revision;
                let landing = Landing { revision, token };
                self.model.accepted = Some(AcceptedSnapshot {
                    token,
                    session_epoch: epoch,
                    document,
                    scene: Arc::new(scene),
                });
                if let Some(owner) = &export_owner
                    && let Some(export) =
                        self.exports
                            .iter_mut()
                            .find(|(operation, scope, old_token)| {
                                *operation == owner.operation_id
                                    && *scope == owner.scope
                                    && *old_token == owner.token
                            })
                {
                    export.2 = token;
                }
                self.model.display_preview = None;
                self.display_preview_transaction_id = None;
                self.model.durability = Durability::Saving { revision };
                self.model.last_error = None;
                if is_open {
                    self.model.selected_part_ids.clear();
                    self.model.active_instance_id = None;
                }
                self.reconcile_selection_and_board();
                self.model.lifecycle = Lifecycle::Ready;
                if !is_open {
                    self.settle_landed(operation_id, landing, effects);
                }
                self.finish_gesture_commit(None, true, effects);
                self.start_save(is_open.then_some(operation_id), effects);
            }
            CoreReply::Error { message, .. } => {
                self.model.last_error = Some(message.clone());
                self.settle(operation_id, TerminalOutcome::Rejected(message), effects);
                self.model.lifecycle = if self.model.accepted.is_some() {
                    Lifecycle::Ready
                } else {
                    Lifecycle::Empty
                };
                if gesture_generation.is_some() {
                    self.finish_gesture_commit(gesture_generation, false, effects);
                }
            }
            _ => {
                let reason = "core returned an unexpected reply variant".to_string();
                self.model.last_error = Some(reason.clone());
                self.model.lifecycle = Lifecycle::RecoveryRequired;
                self.settle(
                    operation_id,
                    TerminalOutcome::ExecutorFailed(reason.clone()),
                    effects,
                );
            }
        }
    }
}

impl Session {
    pub fn submit(&mut self, event: Event) -> Vec<Effect> {
        let mut effects = Vec::new();
        if self.model.lifecycle == Lifecycle::Closed {
            if let Some(operation_id) = event_operation(&event) {
                self.settle(operation_id, TerminalOutcome::Closed, &mut effects);
            }
            return effects;
        }
        match event {
            Event::Open {
                operation_id,
                document,
            } => {
                if self.model.lifecycle == Lifecycle::RecoveryRequired {
                    self.settle(
                        operation_id,
                        TerminalOutcome::BlockedByRecovery(
                            "explicit recovery choice required".into(),
                        ),
                        &mut effects,
                    );
                } else {
                    self.open(operation_id, document, &mut effects);
                }
            }
            Event::RecoverWithDocument {
                operation_id,
                document,
            } => {
                if self.model.lifecycle != Lifecycle::RecoveryRequired {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("session is not awaiting recovery".into()),
                        &mut effects,
                    );
                } else if let Err(reason) = (self.validate_open)(&document) {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected(reason),
                        &mut effects,
                    );
                } else {
                    // The user chose the last known persisted document. Discard the
                    // failed retained candidate so it cannot block the explicit reopen.
                    self.pending_save = None;
                    self.model.durability = self
                        .model
                        .accepted
                        .as_ref()
                        .map(|snapshot| Durability::Saved {
                            revision: snapshot.document.revision,
                        })
                        .unwrap_or(Durability::NoDocument);
                    self.model.last_error = None;
                    self.open(operation_id, document, &mut effects);
                }
            }
            Event::PreviewEdit {
                operation_id,
                transaction_id,
                target_ids,
                operation,
            } => {
                let base_revision = self
                    .model
                    .accepted
                    .as_ref()
                    .map_or(0, |snapshot| snapshot.document.revision);
                self.apply_request(
                    operation_id,
                    CoreRequest::Edit {
                        id: wire_id(operation_id),
                        command: EditCommand {
                            base_revision,
                            transaction_id,
                            phase: EditPhase::Preview,
                            target_ids,
                            operation,
                        },
                    },
                    None,
                    None,
                    &mut effects,
                );
            }
            Event::ResolveEdit {
                operation_id,
                label,
                resolver,
            } => self.resolve_edit(operation_id, label, resolver, &mut effects),
            Event::ClearPreview {
                operation_id,
                token,
                revision,
                board_id,
                transaction_id,
            } => {
                let current = self.model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == token
                        && accepted.document.revision == revision
                        && self.model.active_board_id == board_id
                });
                if current
                    && self.display_preview_transaction_id.as_deref() == Some(&transaction_id)
                {
                    self.model.display_preview = None;
                    self.model.snap_guide = None;
                    self.display_preview_transaction_id = None;
                }
                self.settle(
                    operation_id,
                    if current {
                        TerminalOutcome::Completed
                    } else {
                        TerminalOutcome::Superseded
                    },
                    &mut effects,
                );
            }
            Event::ReviewElectricalRemap {
                operation_id,
                base_revision,
                board_id,
                expected_fingerprint,
            } => {
                self.apply_request(
                    operation_id,
                    CoreRequest::ReviewElectricalRemap {
                        id: wire_id(operation_id),
                        base_revision,
                        board_id,
                        expected_fingerprint,
                    },
                    None,
                    None,
                    &mut effects,
                );
            }
            Event::Undo { operation_id } => self.apply_request(
                operation_id,
                CoreRequest::Undo {
                    id: wire_id(operation_id),
                },
                None,
                None,
                &mut effects,
            ),
            Event::Redo { operation_id } => self.apply_request(
                operation_id,
                CoreRequest::Redo {
                    id: wire_id(operation_id),
                },
                None,
                None,
                &mut effects,
            ),
            Event::RetrySave { operation_id } => self.retry_save(operation_id, &mut effects),
            Event::SelectParts {
                operation_id,
                part_ids,
                range_part_ids,
                mode,
            } => {
                if let Some(snapshot) = &self.model.accepted {
                    let valid = snapshot
                        .document
                        .parts
                        .iter()
                        .map(|part| part.id.as_str())
                        .collect::<HashSet<_>>();
                    let incoming = part_ids
                        .into_iter()
                        .filter(|id| valid.contains(id.as_str()))
                        .collect::<Vec<_>>();
                    match mode {
                        SelectionMode::Range => {
                            let valid_order = range_part_ids
                                .iter()
                                .filter(|id| valid.contains(id.as_str()))
                                .cloned()
                                .collect::<Vec<_>>();
                            let anchor =
                                self.model.selection_anchor_id.as_ref().or(incoming.first());
                            if let (Some(anchor), Some(last)) = (anchor, incoming.last())
                                && let (Some(a), Some(b)) = (
                                    valid_order.iter().position(|id| id == anchor),
                                    valid_order.iter().position(|id| id == last),
                                )
                            {
                                self.model.selected_part_ids =
                                    valid_order[a.min(b)..=a.max(b)].to_vec();
                            }
                        }
                        _ => update_selection(
                            &mut self.model.selected_part_ids,
                            incoming.clone(),
                            mode,
                        ),
                    }
                    self.model.selection_mode = mode;
                    if let Some(anchor) = incoming.first() {
                        self.model.selection_anchor_id = Some(anchor.clone());
                    }
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
                }
            }
            Event::SelectMatrixCell {
                operation_id,
                scope,
                matrix_id,
                target_part_id,
                part_ids,
                mode,
            } => {
                let Some(snapshot) = self.model.accepted.as_ref() else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
                    return effects;
                };
                let scope_is_current = snapshot.session_epoch == scope.session_epoch
                    && snapshot.document.id == scope.document_id
                    && self.model.active_board_id == scope.board_id
                    && self.model.active_instance_id == scope.instance_id;
                let Some(board) = snapshot
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == scope.board_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let Some(matrix) = snapshot
                    .document
                    .matrices
                    .iter()
                    .find(|matrix| matrix.id == matrix_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let scope_is_current = scope_is_current
                    && matrix
                        .board_id
                        .as_ref()
                        .is_none_or(|board_id| board_id == &scope.board_id);
                let board_parts = board
                    .part_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                let document_parts = snapshot
                    .document
                    .parts
                    .iter()
                    .map(|part| part.id.as_str())
                    .collect::<HashSet<_>>();
                let Some(scene) = snapshot
                    .scene
                    .matrix_scenes
                    .iter()
                    .find(|scene| scene.matrix_id == matrix_id)
                else {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                };
                let mut primary_cells = scene
                    .cells
                    .iter()
                    .filter_map(|cell| {
                        let id = cell.member_id.as_deref()?;
                        (cell.enabled
                            && matrix.part_ids.iter().any(|member| member == id)
                            && board_parts.contains(id)
                            && document_parts.contains(id))
                        .then(|| (cell.row, cell.column, id.to_owned()))
                    })
                    .collect::<Vec<_>>();
                primary_cells.sort_by_key(|(row, column, _)| (*row, *column));
                let target_is_live = primary_cells.iter().any(|(_, _, id)| id == &target_part_id);
                if !scope_is_current || !target_is_live {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                    return effects;
                }
                let incoming = part_ids
                    .into_iter()
                    .filter(|id| {
                        board_parts.contains(id.as_str()) && document_parts.contains(id.as_str())
                    })
                    .collect::<Vec<_>>();
                let mut retained_anchor = self.model.selection_anchor_id.clone();
                let mut effective_mode = mode;
                let selected = if mode == SelectionMode::Range {
                    let anchor = retained_anchor.as_deref().and_then(|anchor| {
                        primary_cells
                            .iter()
                            .find(|(_, _, id)| id == anchor)
                            .map(|(row, column, _)| (*row, *column, anchor.to_owned()))
                    });
                    let target = primary_cells
                        .iter()
                        .find(|(_, _, id)| id == &target_part_id)
                        .map(|(row, column, _)| (*row, *column));
                    if let (
                        Some((anchor_row, anchor_column, _)),
                        Some((target_row, target_column)),
                    ) = (anchor, target)
                    {
                        primary_cells
                            .iter()
                            .filter(|(row, column, _)| {
                                *row >= anchor_row.min(target_row)
                                    && *row <= anchor_row.max(target_row)
                                    && *column >= anchor_column.min(target_column)
                                    && *column <= anchor_column.max(target_column)
                            })
                            .map(|(_, _, id)| id.clone())
                            .collect::<Vec<_>>()
                    } else {
                        effective_mode = SelectionMode::Replace;
                        retained_anchor = Some(target_part_id.clone());
                        incoming
                    }
                } else {
                    retained_anchor = Some(target_part_id.clone());
                    incoming
                };
                update_selection(&mut self.model.selected_part_ids, selected, effective_mode);
                self.model.selection_mode = effective_mode;
                self.model.selection_anchor_id = retained_anchor;
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::Navigate {
                operation_id,
                board_id,
                instance_id,
            } => {
                if let Some(snapshot) = &self.model.accepted {
                    let board_ok = snapshot
                        .document
                        .boards
                        .iter()
                        .any(|board| board.id == board_id);
                    let instance_ok = instance_id.as_ref().is_none_or(|id| {
                        snapshot.document.hardware.as_ref().is_some_and(|h| {
                            h.instances
                                .iter()
                                .any(|i| &i.id == id && i.board_id == board_id)
                        })
                    });
                    if board_ok && instance_ok {
                        self.model.active_board_id = board_id;
                        self.model.active_instance_id = instance_id;
                        self.invalidate_gesture(&mut effects);
                        self.cancel_job(&mut effects);
                        self.cancel_exports(&mut effects);
                        self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                    } else {
                        self.settle(
                            operation_id,
                            TerminalOutcome::Rejected(
                                "board or instance is not in the accepted document".into(),
                            ),
                            &mut effects,
                        );
                    }
                } else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected("no accepted document is open".into()),
                        &mut effects,
                    );
                }
            }
            Event::SetCamera {
                operation_id,
                center,
                zoom,
            } => {
                if center.x.is_finite() && center.y.is_finite() && zoom.is_finite() && zoom > 0.0 {
                    self.model.camera = CameraState { center, zoom };
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.settle(
                        operation_id,
                        TerminalOutcome::Rejected(
                            "camera values must be finite and zoom positive".into(),
                        ),
                        &mut effects,
                    );
                }
            }
            Event::GestureBegin {
                operation_id,
                pointer_id,
                target_ids,
                transaction_id,
                start,
                pitch,
                snap_fraction,
                geometry_snap,
                gap,
                pixels_per_world,
                alt,
            } => {
                if self.gesture.is_none() && self.model.lifecycle == Lifecycle::Ready {
                    let generation = self.next_gesture;
                    self.next_gesture += 1;
                    let base_revision = self
                        .model
                        .accepted
                        .as_ref()
                        .map_or(0, |s| s.document.revision);
                    self.gesture = Some(Gesture {
                        operation_id,
                        pointer_id,
                        generation,
                        transaction_id,
                        base_revision,
                        targets: target_ids,
                        start: start.clone(),
                        latest: start,
                        sent_positions: None,
                        pitch,
                        snap_fraction,
                        geometry_snap,
                        gap,
                        pixels_per_world,
                        snap_state: MagneticSnapState::default(),
                        alt,
                        frame_pending: false,
                        changed: false,
                    });
                    self.model.gesture = self.gesture.as_ref().map(gesture_view);
                    effects.push(Effect::CapturePointer { pointer_id });
                }
            }
            Event::GestureSample {
                pointer_id,
                positions,
                pixels_per_world,
                alt,
            } => {
                if let Some(pixels_per_world) = pixels_per_world
                    && let Some(gesture) = self
                        .gesture
                        .as_mut()
                        .filter(|gesture| gesture.pointer_id == pointer_id)
                {
                    gesture.pixels_per_world = Some(pixels_per_world);
                }
                self.gesture_sample(pointer_id, positions, alt, false, &mut effects)
            }
            Event::GestureFrame { generation } => self.gesture_frame(generation, &mut effects),
            Event::GestureEnd {
                pointer_id,
                final_positions,
                pixels_per_world,
                alt,
            } => {
                if let Some(pixels_per_world) = pixels_per_world
                    && let Some(gesture) = self
                        .gesture
                        .as_mut()
                        .filter(|gesture| gesture.pointer_id == pointer_id)
                {
                    gesture.pixels_per_world = Some(pixels_per_world);
                }
                self.gesture_sample(pointer_id, final_positions, alt, true, &mut effects)
            }
            Event::GestureCancel { pointer_id } => {
                if self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| g.pointer_id == pointer_id)
                {
                    self.invalidate_gesture(&mut effects);
                }
            }
            Event::StartGeneration {
                operation_id,
                scope,
            } => self.start_generation_now(operation_id, scope, &mut effects),
            Event::CancelGeneration { operation_id } => {
                self.cancel_job(&mut effects);
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::StartExport {
                operation_id,
                scope,
            } => self.start_export_now(operation_id, scope, &mut effects),
            Event::ExportCommit {
                operation_id,
                export_operation_id,
                token,
                scope,
                commit,
            } => {
                let owner = ExportCommitOwner {
                    operation_id: export_operation_id,
                    token,
                    scope,
                };
                if !self.export_is_current(owner.operation_id, owner.token, &owner.scope) {
                    self.settle(operation_id, TerminalOutcome::Superseded, &mut effects);
                } else {
                    let id = wire_id(operation_id);
                    let request = match commit {
                        ExportCommitRequest::ApplyElectrical { plan, draft } => {
                            CoreRequest::ApplyElectrical {
                                id,
                                base_revision: plan.revision,
                                plan,
                                draft,
                            }
                        }
                        ExportCommitRequest::ProtectElectricalHandoff { plan } => {
                            CoreRequest::ProtectElectricalHandoff {
                                id,
                                base_revision: plan.revision,
                                board_id: owner.scope.board_id.clone(),
                                plan,
                            }
                        }
                    };
                    self.apply_request(operation_id, request, None, Some(owner), &mut effects);
                }
            }
            Event::Close { operation_id } => self.close(operation_id, &mut effects),
        }
        effects
    }

    pub fn complete(&mut self, completion: Completion) -> Vec<Effect> {
        let mut effects = Vec::new();
        match completion {
            Completion::Persist {
                save_attempt_id,
                result,
            } => self.persist_completed(save_attempt_id, result, &mut effects),
            Completion::GenerationProgress {
                job_id,
                scope,
                message: _,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Running { job_id };
                }
            }
            Completion::GenerationFinished {
                job_id,
                scope,
                exact,
            } => {
                self.finish_generation(
                    job_id,
                    &scope,
                    GenerationStatus::Ready { job_id, exact },
                    TerminalOutcome::Completed,
                    &mut effects,
                );
            }
            Completion::GenerationFailed {
                job_id,
                scope,
                reason,
            } => {
                let status = GenerationStatus::Failed {
                    job_id,
                    reason: reason.clone(),
                };
                self.finish_generation(
                    job_id,
                    &scope,
                    status,
                    TerminalOutcome::ExecutorFailed(reason),
                    &mut effects,
                );
            }
            Completion::GenerationBlocked {
                job_id,
                scope,
                reason,
            } => {
                let status = GenerationStatus::Blocked {
                    job_id,
                    reason: reason.clone(),
                };
                self.finish_generation(
                    job_id,
                    &scope,
                    status,
                    TerminalOutcome::Rejected(reason),
                    &mut effects,
                );
            }
            Completion::ExportFinished {
                operation_id,
                token,
                scope,
                artifact_id,
            } => {
                let valid = self
                    .exports
                    .iter()
                    .any(|(op, expected_scope, expected_token)| {
                        *op == operation_id && *expected_scope == scope && *expected_token == token
                    })
                    && self.snapshot_matches(token, &scope);
                self.exports.retain(|(op, _, _)| *op != operation_id);
                if valid {
                    effects.push(Effect::DeliverExport {
                        operation_id,
                        artifact_id,
                        token,
                    });
                    self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
                } else {
                    self.settle(operation_id, TerminalOutcome::Cancelled, &mut effects);
                }
            }
            Completion::ExportFailed {
                operation_id,
                reason,
            } => {
                self.exports
                    .retain(|(registered_operation, _, _)| *registered_operation != operation_id);
                self.settle(
                    operation_id,
                    TerminalOutcome::ExecutorFailed(reason),
                    &mut effects,
                );
            }
        }
        if self.pending_save.is_none()
            && let Some(operation_id) = self.close_operation.take()
        {
            self.finish_close(operation_id, &mut effects);
        }
        effects
    }
}

impl Session {
    fn finish_generation(
        &mut self,
        job_id: JobId,
        scope: &Scope,
        status: GenerationStatus,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        if self.job_is_current(job_id, scope)
            && let Some((_, operation, _)) = self.active_job.take()
        {
            if let TerminalOutcome::ExecutorFailed(reason) | TerminalOutcome::Rejected(reason) =
                &outcome
            {
                self.model.last_error = Some(reason.clone());
            }
            self.model.generation = status;
            self.settle(operation, outcome, effects);
        }
    }

    fn start_save(&mut self, operation: Option<OperationId>, effects: &mut Vec<Effect>) {
        if self.pending_save.is_some() {
            return;
        }
        let Some(accepted) = self.model.accepted.as_ref() else {
            return;
        };
        let document = accepted.document.clone();
        let save_attempt_id = SaveAttemptId(self.next_save);
        self.next_save += 1;
        self.pending_save = Some(PendingSave {
            save_attempt_id,
            revision: document.revision,
            operation,
        });
        self.model.durability = Durability::Saving {
            revision: document.revision,
        };
        effects.push(Effect::Persist {
            save_attempt_id,
            document,
        });
    }

    pub(super) fn persist_completed(
        &mut self,
        save_attempt_id: SaveAttemptId,
        result: SaveResult,
        effects: &mut Vec<Effect>,
    ) {
        let Some(pending) = self.pending_save.take() else {
            return;
        };
        if pending.save_attempt_id != save_attempt_id {
            self.pending_save = Some(pending);
            return;
        }
        let accepted = self
            .model
            .accepted
            .as_ref()
            .expect("saving an open project");
        let revision = accepted.document.revision;
        match result {
            SaveResult::Committed if pending.revision != revision => {
                self.start_save(pending.operation, effects);
            }
            SaveResult::Committed => {
                self.model.durability = Durability::Saved { revision };
                if let Some(operation_id) = pending.operation {
                    self.settle(operation_id, TerminalOutcome::Completed, effects);
                }
            }
            SaveResult::Aborted(reason) => {
                self.model.durability = Durability::Failed {
                    revision,
                    reason: reason.clone(),
                };
                if let Some(operation_id) = pending.operation {
                    self.settle(
                        operation_id,
                        TerminalOutcome::PersistenceFailed(reason),
                        effects,
                    );
                }
                if let Some(operation) = self.close_operation.take() {
                    self.model.lifecycle = Lifecycle::Ready;
                    self.settle(
                        operation,
                        TerminalOutcome::BlockedByRecovery("save failed before close".into()),
                        effects,
                    );
                }
            }
        }
    }

    pub(super) fn retry_save(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        if matches!(self.model.durability, Durability::Failed { .. }) {
            self.start_save(Some(operation_id), effects);
        } else {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("there is no failed save to retry".into()),
                effects,
            );
        }
    }

    pub(super) fn settle(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        if self.settled.insert(operation_id) {
            self.push_settled(operation_id, outcome, None, effects);
        }
    }

    /// Settle a completed operation at the accepted snapshot it just installed, so callers
    /// read exactly where it landed. A retried save re-reports the original operation with
    /// its eventual landing after the earlier failure report, so the dedup set is updated
    /// without suppressing this report.
    pub(super) fn settle_landed(
        &mut self,
        operation_id: OperationId,
        landing: Landing,
        effects: &mut Vec<Effect>,
    ) {
        self.settled.insert(operation_id);
        self.push_settled(
            operation_id,
            TerminalOutcome::Completed,
            Some(landing),
            effects,
        );
    }

    fn push_settled(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        landing: Option<Landing>,
        effects: &mut Vec<Effect>,
    ) {
        effects.push(Effect::Settled {
            operation_id,
            outcome,
            landing,
        });
    }
    pub(super) fn epoch(&self) -> SessionEpoch {
        self.model
            .accepted
            .as_ref()
            .map_or(SessionEpoch(0), |s| s.session_epoch)
    }
    fn reconcile_selection_and_board(&mut self) {
        let Some(snapshot) = &self.model.accepted else {
            return;
        };
        let parts = snapshot
            .document
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .collect::<HashSet<_>>();
        self.model
            .selected_part_ids
            .retain(|id| parts.contains(id.as_str()));
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == self.model.active_board_id)
        {
            self.model.active_board_id = snapshot
                .document
                .boards
                .first()
                .map_or_else(String::new, |board| board.id.clone());
        }
        if self.model.active_instance_id.as_ref().is_some_and(|id| {
            !snapshot.document.hardware.as_ref().is_some_and(|h| {
                h.instances
                    .iter()
                    .any(|i| &i.id == id && i.board_id == self.model.active_board_id)
            })
        }) {
            self.model.active_instance_id = None;
        }
    }
}
