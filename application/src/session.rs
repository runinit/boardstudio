//! Headless application session for ordered document work and host-owned effects.
//!
//! This crate deliberately contains no browser handles. A host runs each emitted effect and
//! returns its completion through [`Session::complete`].

use crate::interactions::{DragSnapOptions, SnapGuide, normalize_drag};
use boardstudio_core::{CoreEngine, model::*};
use std::collections::{HashSet, VecDeque};
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

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Open {
        operation_id: OperationId,
        document: ProjectDoc,
    },
    Edit {
        operation_id: OperationId,
        command: EditCommand,
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
        alt: bool,
    },
    GestureSample {
        pointer_id: i64,
        positions: Vec<Position>,
        alt: bool,
    },
    GestureFrame {
        generation: u64,
    },
    GestureEnd {
        pointer_id: i64,
        final_positions: Vec<Position>,
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
    Close {
        operation_id: OperationId,
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
    ExecutorRestarted {
        executor_epoch: ExecutorEpoch,
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
    RestartCoreExecutor {
        executor_epoch: ExecutorEpoch,
    },
    Persist {
        operation_id: OperationId,
        save_attempt_id: SaveAttemptId,
        document: Arc<ProjectDoc>,
        assets: Vec<AssetRef>,
    },
    Settled {
        operation_id: OperationId,
        outcome: TerminalOutcome,
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
    Undo,
    Redo,
    GesturePreview(EditCommand),
    GestureCommit(EditCommand),
    Generate(Scope),
    Export(Scope),
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
    Preview,
    Undo,
    Redo,
    GestureCommit,
}
#[derive(Clone, Debug)]
struct ActiveCore {
    request_id: RequestId,
    executor_epoch: ExecutorEpoch,
    operation_id: OperationId,
    kind: ActiveKind,
    gesture_generation: Option<u64>,
}
#[derive(Clone, Debug)]
struct PendingSave {
    operation_id: OperationId,
    document: Arc<ProjectDoc>,
    scene: Arc<SceneDelta>,
    kind: ActiveKind,
    save_attempt_id: SaveAttemptId,
    retry_operation_id: Option<OperationId>,
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
}
impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
impl Session {
    pub fn new() -> Self {
        Self {
            model: ReadModel::default(),
            queue: VecDeque::new(),
            active_core: None,
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
        }
    }
    pub fn read_model(&self) -> &ReadModel {
        &self.model
    }
    pub fn core_executor_epoch(&self) -> ExecutorEpoch {
        self.core_epoch
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
                    self.queue_open(operation_id, document, &mut effects);
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
                    self.queue_open(operation_id, document, &mut effects);
                }
            }
            Event::Edit {
                operation_id,
                command,
            } => self.enqueue(operation_id, IntentKind::Edit(command), false, &mut effects),
            Event::Undo { operation_id } => {
                self.enqueue(operation_id, IntentKind::Undo, false, &mut effects)
            }
            Event::Redo { operation_id } => {
                self.enqueue(operation_id, IntentKind::Redo, false, &mut effects)
            }
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
                        SelectionMode::Replace => self.model.selected_part_ids = incoming.clone(),
                        SelectionMode::Add => {
                            for id in &incoming {
                                if !self.model.selected_part_ids.contains(id) {
                                    self.model.selected_part_ids.push(id.clone());
                                }
                            }
                        }
                        SelectionMode::Toggle => {
                            for id in &incoming {
                                if let Some(index) = self
                                    .model
                                    .selected_part_ids
                                    .iter()
                                    .position(|current| current == id)
                                {
                                    self.model.selected_part_ids.remove(index);
                                } else {
                                    self.model.selected_part_ids.push(id.clone());
                                }
                            }
                        }
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
                        commit_queued: false,
                        pitch,
                        snap_fraction,
                        geometry_snap,
                        gap,
                        alt,
                        frame_pending: false,
                        changed: false,
                        preview_in_flight: false,
                        final_positions: None,
                    });
                    self.model.gesture = self.gesture.as_ref().map(gesture_view);
                    effects.push(Effect::CapturePointer { pointer_id });
                }
            }
            Event::GestureSample {
                pointer_id,
                positions,
                alt,
            } => self.gesture_sample(pointer_id, positions, alt, false, &mut effects),
            Event::GestureFrame { generation } => self.gesture_frame(generation, &mut effects),
            Event::GestureEnd {
                pointer_id,
                final_positions,
                alt,
            } => self.gesture_sample(pointer_id, final_positions, alt, true, &mut effects),
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
            } => self.enqueue_aux(operation_id, IntentKind::Generate(scope), &mut effects),
            Event::CancelGeneration { operation_id } => {
                self.cancel_job(&mut effects);
                self.settle(operation_id, TerminalOutcome::Completed, &mut effects);
            }
            Event::StartExport {
                operation_id,
                scope,
            } => self.enqueue_aux(operation_id, IntentKind::Export(scope), &mut effects),
            Event::Close { operation_id } => self.close(operation_id, &mut effects),
        }
        if self.active_core.is_none()
            && self.pending_save.is_none()
            && self.model.lifecycle != Lifecycle::RecoveryRequired
        {
            self.pump(&mut effects);
        }
        effects
    }

    pub fn complete(&mut self, completion: Completion) -> Vec<Effect> {
        let mut effects = Vec::new();
        match completion {
            Completion::Core {
                request_id,
                executor_epoch,
                reply,
            } => self.core_completed(request_id, executor_epoch, *reply, &mut effects),
            Completion::Persist {
                save_attempt_id,
                result,
            } => self.persist_completed(save_attempt_id, result, &mut effects),
            Completion::CoreFailed {
                request_id,
                executor_epoch,
                reason,
            } => self.core_failed(request_id, executor_epoch, reason, &mut effects),
            Completion::ExecutorRestarted { executor_epoch } => {
                if executor_epoch == self.core_epoch {
                    self.core_epoch = executor_epoch;
                }
            }
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
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Ready { job_id, exact };
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(operation, TerminalOutcome::Completed, &mut effects);
                    }
                }
            }
            Completion::GenerationFailed {
                job_id,
                scope,
                reason,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Failed {
                        job_id,
                        reason: reason.clone(),
                    };
                    self.model.last_error = Some(reason.clone());
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(
                            operation,
                            TerminalOutcome::ExecutorFailed(reason),
                            &mut effects,
                        );
                    }
                }
            }
            Completion::GenerationBlocked {
                job_id,
                scope,
                reason,
            } => {
                if self.job_is_current(job_id, &scope) {
                    self.model.generation = GenerationStatus::Blocked {
                        job_id,
                        reason: reason.clone(),
                    };
                    self.model.last_error = Some(reason.clone());
                    if let Some((_, operation, _)) = self.active_job.take() {
                        self.settle(operation, TerminalOutcome::Rejected(reason), &mut effects);
                    }
                }
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
            } => self.settle(
                operation_id,
                TerminalOutcome::ExecutorFailed(reason),
                &mut effects,
            ),
        }
        if self.active_core.is_none()
            && self.pending_save.is_none()
            && self.model.lifecycle != Lifecycle::RecoveryRequired
        {
            if let Some(operation_id) = self.close_operation.take() {
                self.finish_close(operation_id, &mut effects);
            } else {
                self.pump(&mut effects);
            }
        }
        effects
    }

    fn queue_open(
        &mut self,
        operation_id: OperationId,
        document: ProjectDoc,
        effects: &mut Vec<Effect>,
    ) {
        self.invalidate_gesture(effects);
        self.cancel_job(effects);
        self.cancel_exports(effects);
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: self.epoch(),
            kind: IntentKind::Open(Box::new(document)),
            strict_revision: false,
        });
        self.model.lifecycle = Lifecycle::Opening;
    }
    fn enqueue(
        &mut self,
        operation_id: OperationId,
        kind: IntentKind,
        strict_revision: bool,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle == Lifecycle::Closing {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("session is closing".into()),
                effects,
            );
            return;
        }
        if self.model.lifecycle == Lifecycle::RecoveryRequired {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("save recovery is required".into()),
                effects,
            );
            return;
        }
        if self.model.accepted.is_none() {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("no accepted document is open".into()),
                effects,
            );
            return;
        }
        let epoch = self.epoch();
        if let IntentKind::Edit(command) = &kind
            && strict_revision
            && command.base_revision != self.model.accepted.as_ref().unwrap().document.revision
        {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("captured edit revision is stale".into()),
                effects,
            );
            return;
        }
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: epoch,
            kind,
            strict_revision,
        });
    }
    fn enqueue_aux(
        &mut self,
        operation_id: OperationId,
        kind: IntentKind,
        effects: &mut Vec<Effect>,
    ) {
        if self.model.lifecycle == Lifecycle::RecoveryRequired
            || self.model.lifecycle == Lifecycle::Closing
        {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery(
                    "session cannot start jobs during recovery or close".into(),
                ),
                effects,
            );
            return;
        }
        if let Some(scope) = match &kind {
            IntentKind::Generate(scope) | IntentKind::Export(scope) => Some(scope),
            _ => None,
        } && !self.scope_is_current(scope)
        {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("job scope is stale".into()),
                effects,
            );
            return;
        }
        self.queue.push_back(Intent {
            operation_id,
            submitted_epoch: self.epoch(),
            kind,
            strict_revision: false,
        });
    }
    fn pump(&mut self, effects: &mut Vec<Effect>) {
        if self.active_core.is_some()
            || self.pending_save.is_some()
            || self.model.lifecycle == Lifecycle::RecoveryRequired
            || self.model.lifecycle == Lifecycle::Closed
        {
            return;
        }
        while let Some(intent) = self.queue.pop_front() {
            if intent.submitted_epoch != self.epoch() && !matches!(intent.kind, IntentKind::Open(_))
            {
                self.settle(
                    intent.operation_id,
                    TerminalOutcome::Rejected(
                        "document session changed before command began".into(),
                    ),
                    effects,
                );
                continue;
            }
            if let IntentKind::Generate(scope) = &intent.kind {
                if self.model.lifecycle != Lifecycle::Ready {
                    self.queue.push_front(intent);
                    return;
                }
                self.start_generation_now(intent.operation_id, scope.clone(), effects);
                continue;
            }
            if let IntentKind::Export(scope) = &intent.kind {
                if self.model.lifecycle != Lifecycle::Ready {
                    self.queue.push_front(intent);
                    return;
                }
                self.start_export_now(intent.operation_id, scope.clone(), effects);
                continue;
            }
            let captured_revision = match &intent.kind {
                IntentKind::Edit(command)
                | IntentKind::GesturePreview(command)
                | IntentKind::GestureCommit(command)
                    if intent.strict_revision =>
                {
                    Some(command.base_revision)
                }
                _ => None,
            };
            if captured_revision.is_some_and(|revision| {
                self.model
                    .accepted
                    .as_ref()
                    .is_none_or(|s| s.document.revision != revision)
            }) {
                self.settle(
                    intent.operation_id,
                    TerminalOutcome::Rejected("captured edit revision is stale".into()),
                    effects,
                );
                self.finish_gesture_commit(None, false, effects);
                continue;
            }
            let request_id = RequestId(self.next_request);
            self.next_request += 1;
            let request_wire_id = format!("m1-{}", request_id.0);
            let (request, kind, gesture_generation) = match intent.kind {
                IntentKind::Open(document) => (
                    CoreRequest::Open {
                        id: request_wire_id.clone(),
                        document: *document,
                    },
                    ActiveKind::Open,
                    None,
                ),
                IntentKind::Edit(mut command) => {
                    if !intent.strict_revision
                        && let Some(accepted) = &self.model.accepted
                    {
                        command.base_revision = accepted.document.revision;
                    }
                    let kind = if command.phase == EditPhase::Preview {
                        ActiveKind::Preview
                    } else {
                        ActiveKind::Commit
                    };
                    (
                        CoreRequest::Edit {
                            id: request_wire_id.clone(),
                            command,
                        },
                        kind,
                        None,
                    )
                }
                IntentKind::Undo => (
                    CoreRequest::Undo {
                        id: request_wire_id.clone(),
                    },
                    ActiveKind::Undo,
                    None,
                ),
                IntentKind::Redo => (
                    CoreRequest::Redo {
                        id: request_wire_id.clone(),
                    },
                    ActiveKind::Redo,
                    None,
                ),
                IntentKind::GesturePreview(command) => (
                    CoreRequest::Edit {
                        id: request_wire_id.clone(),
                        command,
                    },
                    ActiveKind::Preview,
                    self.gesture.as_ref().map(|g| g.generation),
                ),
                IntentKind::Generate(_) | IntentKind::Export(_) => return,
                IntentKind::GestureCommit(mut command) => {
                    if let Some(g) = &self.gesture {
                        command.base_revision = g.base_revision;
                        command.transaction_id = g.transaction_id.clone();
                    }
                    (
                        CoreRequest::Edit {
                            id: request_wire_id.clone(),
                            command,
                        },
                        ActiveKind::GestureCommit,
                        self.gesture.as_ref().map(|g| g.generation),
                    )
                }
            };
            self.model.lifecycle = if kind == ActiveKind::Open {
                Lifecycle::Opening
            } else {
                Lifecycle::Applying
            };
            self.active_core = Some(ActiveCore {
                request_id,
                executor_epoch: self.core_epoch,
                operation_id: intent.operation_id,
                kind,
                gesture_generation,
            });
            effects.push(Effect::Core {
                operation_id: intent.operation_id,
                request_id,
                executor_epoch: self.core_epoch,
                request: Box::new(request),
            });
            return;
        }
        if self.model.lifecycle != Lifecycle::RecoveryRequired && self.close_operation.is_none() {
            self.model.lifecycle = if self.model.accepted.is_some() {
                Lifecycle::Ready
            } else {
                Lifecycle::Empty
            };
        }
    }
    fn core_completed(
        &mut self,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reply: CoreReply,
        effects: &mut Vec<Effect>,
    ) {
        let Some(active) = self.active_core.take() else {
            return;
        };
        if active.request_id != request_id
            || active.executor_epoch != executor_epoch
            || executor_epoch != self.core_epoch
        {
            self.active_core = Some(active);
            return;
        }
        if reply_id(&reply) != format!("m1-{}", request_id.0) {
            self.model.lifecycle = Lifecycle::RecoveryRequired;
            self.model.last_error = Some("core reply ID did not match request".into());
            self.settle(
                active.operation_id,
                TerminalOutcome::ExecutorFailed("core reply ID did not match request".into()),
                effects,
            );
            self.block_queue("core reply ID mismatch", effects);
            return;
        }
        match (active.kind, reply) {
            (ActiveKind::Preview, CoreReply::Preview { scene, .. }) => {
                if active.gesture_generation.is_none_or(|generation| {
                    self.gesture
                        .as_ref()
                        .is_some_and(|g| g.generation == generation)
                }) {
                    self.model.display_preview = Some(Arc::new(scene));
                }
                if active.gesture_generation.is_none() {
                    self.settle(active.operation_id, TerminalOutcome::Completed, effects);
                }
                if let Some(g) = &mut self.gesture {
                    g.preview_in_flight = false;
                }
                self.dispatch_latest_gesture_preview(effects);
            }
            (
                ActiveKind::Open
                | ActiveKind::Commit
                | ActiveKind::Undo
                | ActiveKind::Redo
                | ActiveKind::GestureCommit,
                CoreReply::Scene {
                    scene, document, ..
                },
            ) => {
                let document = Arc::new(*document);
                if scene.revision != document.revision {
                    self.model.lifecycle = Lifecycle::RecoveryRequired;
                    self.model.last_error = Some("core scene and document revisions differ".into());
                    self.settle(
                        active.operation_id,
                        TerminalOutcome::ExecutorFailed(
                            "core scene and document revisions differ".into(),
                        ),
                        effects,
                    );
                    return;
                }
                let save_attempt_id = SaveAttemptId(self.next_save);
                self.next_save += 1;
                let assets = document.assets.iter().map(AssetRef::from).collect();
                let scene = Arc::new(scene);
                self.pending_save = Some(PendingSave {
                    operation_id: active.operation_id,
                    document: document.clone(),
                    scene,
                    kind: active.kind,
                    save_attempt_id,
                    retry_operation_id: None,
                });
                self.model.lifecycle = Lifecycle::Saving;
                self.model.durability = Durability::Saving {
                    revision: document.revision,
                };
                effects.push(Effect::Persist {
                    operation_id: active.operation_id,
                    save_attempt_id,
                    document,
                    assets,
                });
            }
            (_, CoreReply::Error { message, .. }) => {
                self.model.last_error = Some(message.clone());
                self.settle(
                    active.operation_id,
                    TerminalOutcome::Rejected(message),
                    effects,
                );
                self.model.lifecycle = if self.model.accepted.is_some() {
                    Lifecycle::Ready
                } else {
                    Lifecycle::Empty
                };
                self.finish_gesture_commit(active.gesture_generation, false, effects);
            }
            _ => {
                let reason = "core returned an unexpected reply variant".to_string();
                self.model.last_error = Some(reason.clone());
                self.model.lifecycle = Lifecycle::RecoveryRequired;
                self.settle(
                    active.operation_id,
                    TerminalOutcome::ExecutorFailed(reason.clone()),
                    effects,
                );
                self.block_queue(&reason, effects);
            }
        }
    }
    fn persist_completed(
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
        let attempt_operation = pending.retry_operation_id.unwrap_or(pending.operation_id);
        match result {
            SaveResult::Aborted(reason) => {
                self.model.lifecycle = Lifecycle::RecoveryRequired;
                self.model.durability = Durability::Failed {
                    revision: pending.document.revision,
                    reason: reason.clone(),
                };
                self.model.last_error = Some(reason.clone());
                self.pending_save = Some(PendingSave {
                    save_attempt_id,
                    ..pending.clone()
                });
                self.settle(
                    attempt_operation,
                    TerminalOutcome::PersistenceFailed(reason.clone()),
                    effects,
                );
                self.block_queue(&reason, effects);
                if let Some(close_operation) = self.close_operation.take() {
                    self.settle(
                        close_operation,
                        TerminalOutcome::BlockedByRecovery(
                            "pending save failed; resolve recovery before closing".into(),
                        ),
                        effects,
                    );
                }
            }
            SaveResult::Committed => {
                let is_open = pending.kind == ActiveKind::Open;
                let epoch = if is_open {
                    SessionEpoch(self.epoch().0 + 1)
                } else {
                    self.epoch()
                };
                self.cancel_job(effects);
                self.cancel_exports(effects);
                let token = SnapshotToken(self.next_token);
                self.next_token += 1;
                self.model.accepted = Some(AcceptedSnapshot {
                    token,
                    session_epoch: epoch,
                    document: pending.document.clone(),
                    scene: pending.scene.clone(),
                });
                self.model.display_preview = None;
                self.model.durability = Durability::Saved {
                    revision: pending.document.revision,
                };
                self.model.last_error = None;
                if is_open {
                    self.model.selected_part_ids.clear();
                    self.model.active_instance_id = None;
                }
                self.reconcile_selection_and_board();
                self.model.lifecycle = Lifecycle::Ready;
                self.settle(pending.operation_id, TerminalOutcome::Completed, effects);
                if let Some(retry_operation_id) = pending.retry_operation_id {
                    self.settle(retry_operation_id, TerminalOutcome::Completed, effects);
                }
                self.finish_gesture_commit(None, true, effects);
                if let Some(operation) = self.close_operation.take() {
                    self.finish_close(operation, effects);
                }
            }
        }
    }
    fn retry_save(&mut self, operation_id: OperationId, effects: &mut Vec<Effect>) {
        let Some(mut pending) = self.pending_save.take() else {
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("there is no retained save to retry".into()),
                effects,
            );
            return;
        };
        if self.model.lifecycle != Lifecycle::RecoveryRequired {
            self.pending_save = Some(pending);
            self.settle(
                operation_id,
                TerminalOutcome::Rejected("session is not in save recovery".into()),
                effects,
            );
            return;
        }
        let save_attempt_id = SaveAttemptId(self.next_save);
        self.next_save += 1;
        pending.save_attempt_id = save_attempt_id;
        pending.retry_operation_id = Some(operation_id);
        self.model.lifecycle = Lifecycle::Saving;
        self.model.durability = Durability::Saving {
            revision: pending.document.revision,
        };
        let document = pending.document.clone();
        let assets = document.assets.iter().map(AssetRef::from).collect();
        self.pending_save = Some(pending);
        effects.push(Effect::Persist {
            operation_id,
            save_attempt_id,
            document,
            assets,
        });
    }
    fn core_failed(
        &mut self,
        request_id: RequestId,
        executor_epoch: ExecutorEpoch,
        reason: String,
        effects: &mut Vec<Effect>,
    ) {
        let Some(active) = self.active_core.take() else {
            return;
        };
        if active.request_id != request_id || active.executor_epoch != executor_epoch {
            self.active_core = Some(active);
            return;
        }
        self.core_epoch = ExecutorEpoch(self.core_epoch.0 + 1);
        effects.push(Effect::RestartCoreExecutor {
            executor_epoch: self.core_epoch,
        });
        self.model.lifecycle = Lifecycle::RecoveryRequired;
        self.model.last_error = Some(reason.clone());
        self.settle(
            active.operation_id,
            TerminalOutcome::ExecutorFailed(reason.clone()),
            effects,
        );
        self.block_queue("core outcome is unknown; explicit reopen required", effects);
        if let Some(close_operation) = self.close_operation.take() {
            self.settle(
                close_operation,
                TerminalOutcome::BlockedByRecovery(
                    "core outcome is unknown; explicit reopen required".into(),
                ),
                effects,
            );
        }
        self.finish_gesture_commit(active.gesture_generation, false, effects);
    }
    fn block_queue(&mut self, reason: &str, effects: &mut Vec<Effect>) {
        while let Some(intent) = self.queue.pop_front() {
            self.settle(
                intent.operation_id,
                TerminalOutcome::BlockedByRecovery(reason.into()),
                effects,
            );
        }
    }
    fn settle(
        &mut self,
        operation_id: OperationId,
        outcome: TerminalOutcome,
        effects: &mut Vec<Effect>,
    ) {
        if self.settled.insert(operation_id) {
            effects.push(Effect::Settled {
                operation_id,
                outcome,
            });
        }
    }
    fn epoch(&self) -> SessionEpoch {
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
        if g.preview_in_flight || g.commit_queued || (!g.changed && g.final_positions.is_none()) {
            return;
        }
        let is_commit = g.final_positions.is_some();
        let positions = g
            .final_positions
            .clone()
            .unwrap_or_else(|| g.latest.clone());
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
            self.model.display_preview = None;
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
        if self.model.lifecycle == Lifecycle::RecoveryRequired {
            self.settle(
                operation_id,
                TerminalOutcome::BlockedByRecovery("resolve retained work before closing".into()),
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
        | Event::Edit { operation_id, .. }
        | Event::Undo { operation_id }
        | Event::Redo { operation_id }
        | Event::RetrySave { operation_id }
        | Event::SelectParts { operation_id, .. }
        | Event::Navigate { operation_id, .. }
        | Event::SetCamera { operation_id, .. }
        | Event::StartGeneration { operation_id, .. }
        | Event::CancelGeneration { operation_id }
        | Event::StartExport { operation_id, .. }
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

/// Synchronous adapter used by native callers that want to exercise the actual engine.
/// Browser hosts should execute effects asynchronously and call [`Session::complete`].
pub fn handle_core(engine: &mut CoreEngine, request: CoreRequest) -> CoreReply {
    engine.handle(request)
}
