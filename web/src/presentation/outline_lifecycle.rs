use super::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
use crate::outline_settings::{
    OutlineEdit, OutlineExpectation, apply_outline_edit, expectation_applied, generated_feature,
    generated_margin, generated_settings,
};
use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, TerminalOutcome,
};
use boardstudio_core::model::{
    Contour, CornerStyle, EditCommand, EditOperation, EditPhase, Operation, OutlineConnection,
    OutlineContourEdit, OutlineControlPoint, OutlineFeature, OutlineGap, OutlineRepairSettings,
    OutlineSettings, Part, Side, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::SvgElement;

fn unique_outline_entity_id(
    prefix: &str,
    operation_id: u64,
    existing_ids: impl IntoIterator<Item = String>,
) -> String {
    let existing_ids = existing_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let base = format!("{prefix}-{operation_id}");
    if !existing_ids.contains(&base) {
        return base;
    }

    for suffix in 2usize.. {
        let candidate = format!("{base}-{suffix}");
        if !existing_ids.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("a finite saved ID set cannot exhaust the suffix sequence")
}

fn unique_outline_version_id(
    operation_id: u64,
    existing_ids: impl IntoIterator<Item = String>,
) -> String {
    unique_outline_entity_id("outline-version", operation_id, existing_ids)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlineAction {
    Activate {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        context: super::objects::TreeContext,
        require_selected_version: bool,
        board_id: String,
        version_id: Option<String>,
    },
    Copy {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
    },
    Delete {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        version_id: String,
    },
    Update {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        edit: OutlineEdit,
    },
    EditPerimeter {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        target: OutlinePointTarget,
        points: Vec<Vec2>,
        phase: EditPhase,
        transaction_id: String,
    },
    AddFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        feature: OutlineFeature,
    },
    AddConnection {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        points: Vec<Vec2>,
    },
    SetFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        version_id: Option<String>,
        before: OutlineFeature,
        after: OutlineFeature,
    },
    RemoveFeature {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        version_id: String,
        feature_id: String,
    },
    FocusGap {
        scope: Scope,
        token: boardstudio_application::SnapshotToken,
        revision: u64,
        generation: u64,
        board_id: String,
        context: super::objects::TreeContext,
        gap_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum OutlinePointTarget {
    Generated {
        contour: u32,
    },
    Fixed {
        version_id: String,
        feature_id: String,
        anchor_part_id: Option<String>,
        operation: Operation,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct EditablePerimeter {
    target: OutlinePointTarget,
    points: Vec<Vec2>,
    canvas_points: Vec<Vec2>,
    anchor: Option<PerimeterAnchor>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PerimeterAnchor {
    at: Vec2,
    rotation: f64,
    back: bool,
}

impl PerimeterAnchor {
    fn world(self, point: Vec2) -> Vec2 {
        let x = point.x * if self.back { -1.0 } else { 1.0 };
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        Vec2 {
            x: self.at.x + x * cos - point.y * sin,
            y: self.at.y + x * sin + point.y * cos,
        }
    }

    fn local(self, point: Vec2) -> Vec2 {
        let (sin, cos) = (-self.rotation).to_radians().sin_cos();
        let x = point.x - self.at.x;
        let y = point.y - self.at.y;
        Vec2 {
            x: (x * cos - y * sin) * if self.back { -1.0 } else { 1.0 },
            y: x * sin + y * cos,
        }
    }
}

fn outline_connection_control(at: Vec2, eligible: &[&Part]) -> OutlineControlPoint {
    let nearest = eligible.iter().copied().min_by(|left, right| {
        let distance = |part: &Part| (part.pose.at.x - at.x).hypot(part.pose.at.y - at.y);
        distance(left).total_cmp(&distance(right))
    });
    if let Some(part) =
        nearest.filter(|part| (part.pose.at.x - at.x).hypot(part.pose.at.y - at.y) <= 10.0)
    {
        let anchor = PerimeterAnchor {
            at: part.pose.at,
            rotation: part.pose.rotation,
            back: part.side == Side::Back,
        };
        OutlineControlPoint {
            at: anchor.local(at),
            part_id: Some(part.id.clone()),
        }
    } else {
        OutlineControlPoint { at, part_id: None }
    }
}

fn connection_point_world(point: &OutlineControlPoint, parts: &[Part]) -> Vec2 {
    point
        .part_id
        .as_deref()
        .and_then(|id| parts.iter().find(|part| part.id == id))
        .map(|part| {
            PerimeterAnchor {
                at: part.pose.at,
                rotation: part.pose.rotation,
                back: part.side == Side::Back,
            }
            .world(point.at)
        })
        .unwrap_or(point.at)
}

fn move_connection_point(
    feature: &OutlineFeature,
    connection_id: &str,
    point_index: usize,
    world: Vec2,
    parts: &[Part],
) -> OutlineFeature {
    let mut next = feature.clone();
    if let OutlineFeature::PartEnvelope { connections, .. } = &mut next
        && let Some(point) = connections
            .iter_mut()
            .find(|connection| connection.id == connection_id)
            .and_then(|connection| connection.points.get_mut(point_index))
    {
        point.at = point
            .part_id
            .as_deref()
            .and_then(|id| parts.iter().find(|part| part.id == id))
            .map(|part| {
                PerimeterAnchor {
                    at: part.pose.at,
                    rotation: part.pose.rotation,
                    back: part.side == Side::Back,
                }
                .local(world)
            })
            .unwrap_or(world);
    }
    next
}

fn feature_anchor(part_id: Option<&str>, parts: &[Part]) -> Option<PerimeterAnchor> {
    part_id
        .and_then(|id| parts.iter().find(|part| part.id == id))
        .map(|part| PerimeterAnchor {
            at: part.pose.at,
            rotation: part.pose.rotation,
            back: part.side == Side::Back,
        })
}

fn attach_connection_point(
    feature: &OutlineFeature,
    connection_id: &str,
    point_index: usize,
    next_part_id: Option<&str>,
    parts: &[Part],
) -> OutlineFeature {
    let mut next = feature.clone();
    if let OutlineFeature::PartEnvelope { connections, .. } = &mut next
        && let Some(point) = connections
            .iter_mut()
            .find(|connection| connection.id == connection_id)
            .and_then(|connection| connection.points.get_mut(point_index))
    {
        let world = connection_point_world(point, parts);
        point.at = feature_anchor(next_part_id, parts).map_or(world, |anchor| anchor.local(world));
        point.part_id = next_part_id.map(str::to_owned);
    }
    next
}

#[derive(Clone, Debug, PartialEq)]
enum PendingKind {
    Activate {
        version_id: Option<String>,
    },
    Copy {
        version_id: String,
        edited_contour: Option<(u32, Vec<Vec2>)>,
    },
    AddedFeature {
        version_id: String,
        feature_id: String,
    },
    UpdatedFeature {
        version_id: String,
        feature_id: String,
    },
    RemovedFeature {
        version_id: String,
        feature_id: String,
    },
    Delete {
        version_id: String,
    },
    Edit(OutlineExpectation),
}

#[derive(Clone)]
struct Pending {
    scope: Scope,
    snapshot: AcceptedSnapshot,
    generation: u64,
    kind: PendingKind,
    outcome: OutcomeSlot,
}

#[derive(Clone, Copy)]
struct ActionState {
    pending: Signal<Option<Pending>>,
    feedback: Signal<Option<OutlineFeedback>>,
    selected_point: Signal<usize>,
    editing_points: Signal<bool>,
    drawing_operation: Signal<Option<OutlineDrawTool>>,
    drawing_points: Signal<Vec<Vec2>>,
    selected_feature_id: Signal<Option<String>>,
    selected_connection_id: Signal<Option<String>>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    captured_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum OutlineDrawTool {
    Polygon(Operation),
    Connect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutlineFeedback {
    scope: Scope,
    pub(super) generation: u64,
    pub(super) board_id: String,
    pub(super) state: &'static str,
    pub(super) message: Option<String>,
}

#[derive(Clone, PartialEq)]
pub(super) struct OutlineInspectorProjection {
    pub(super) board_id: String,
    pub(super) board_name: String,
    pub(super) version_name: String,
    pub(super) active_version_id: Option<String>,
    pub(super) contours: Vec<Contour>,
    perimeter: Option<EditablePerimeter>,
    snap_paths: Vec<Vec<Vec2>>,
    pub(super) selected_point: Signal<usize>,
    pub(super) editing_points: Signal<bool>,
    pub(super) drawing_operation: Signal<Option<OutlineDrawTool>>,
    pub(super) drawing_points: Signal<Vec<Vec2>>,
    selected_feature_id: Signal<Option<String>>,
    selected_connection_id: Signal<Option<String>>,
    pub(super) geometry_features: Vec<OutlineFeature>,
    pub(super) connections: Vec<OutlineConnection>,
    connection_feature: Option<OutlineFeature>,
    outline_parts: Vec<Part>,
    pub(super) versions: Vec<OutlineVersionChoice>,
    pub(super) settings: OutlineSettings,
    pub(super) repair: OutlineRepairSettings,
    pub(super) generated_margin: Option<f64>,
    pub(super) has_generated: bool,
    pub(super) gaps: Vec<OutlineGap>,
    action_context: Rc<OutlineActionContext>,
    pub(super) enabled: bool,
    pub(super) feedback: Option<OutlineFeedback>,
    pub(super) on_action: EventHandler<OutlineAction>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    pub(super) scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    generation: u64,
}

impl OutlineInspectorProjection {
    pub(super) fn canvas_edit_key(&self) -> String {
        format!(
            "{:?}-{}-{:?}-{}-{}-{}",
            self.scope,
            self.board_id,
            self.active_version_id,
            self.token.0,
            self.revision,
            self.generation
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OutlineActionContext {
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    generation: u64,
    board_id: String,
    selection_context: super::objects::TreeContext,
}

impl OutlineActionContext {
    fn action(&self, edit: OutlineEdit) -> OutlineAction {
        OutlineAction::Update {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            edit,
        }
    }

    fn activate_action(&self, version_id: Option<String>) -> OutlineAction {
        OutlineAction::Activate {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            context: self.selection_context.clone(),
            require_selected_version: false,
            board_id: self.board_id.clone(),
            version_id,
        }
    }

    fn edit_perimeter(
        &self,
        target: OutlinePointTarget,
        points: Vec<Vec2>,
        phase: EditPhase,
        transaction_id: String,
    ) -> OutlineAction {
        OutlineAction::EditPerimeter {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            target,
            points,
            phase,
            transaction_id,
        }
    }

    fn add_feature(&self, feature: OutlineFeature) -> OutlineAction {
        OutlineAction::AddFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            feature,
        }
    }

    fn add_connection(&self, points: Vec<Vec2>) -> OutlineAction {
        OutlineAction::AddConnection {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            points,
        }
    }

    fn set_feature(
        &self,
        version_id: Option<String>,
        before: OutlineFeature,
        after: OutlineFeature,
    ) -> OutlineAction {
        OutlineAction::SetFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            version_id,
            before,
            after,
        }
    }

    fn remove_feature(&self, version_id: String, feature_id: String) -> OutlineAction {
        OutlineAction::RemoveFeature {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            version_id,
            feature_id,
        }
    }

    fn focus_gap(&self, gap_id: String) -> OutlineAction {
        OutlineAction::FocusGap {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
            context: self.selection_context.clone(),
            gap_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OutlineVersionChoice {
    pub(super) id: String,
    pub(super) name: String,
}

impl OutlineAction {
    fn envelope(
        &self,
    ) -> (
        &Scope,
        &boardstudio_application::SnapshotToken,
        u64,
        u64,
        &String,
    ) {
        match self {
            OutlineAction::Activate {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Copy {
                scope,
                token,
                revision,
                generation,
                board_id,
            }
            | OutlineAction::Delete {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::Update {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::EditPerimeter {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::AddFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::AddConnection {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::SetFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::RemoveFeature {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            }
            | OutlineAction::FocusGap {
                scope,
                token,
                revision,
                generation,
                board_id,
                ..
            } => (scope, token, *revision, *generation, board_id),
        }
    }

    pub(super) fn is_current(&self, runtime: &Runtime, generation: u64) -> bool {
        let (scope, token, revision, captured_generation, board_id) = self.envelope();
        let model = runtime.model();
        let perimeter_edit = matches!(
            self,
            OutlineAction::EditPerimeter {
                phase: EditPhase::Preview | EditPhase::Commit,
                ..
            }
        );
        let lifecycle_current = model.lifecycle == Lifecycle::Ready
            || (perimeter_edit && model.lifecycle == Lifecycle::Applying);
        generation == captured_generation
            && runtime.scope().as_ref() == Some(scope)
            && model.accepted.as_ref().is_some_and(|snapshot| {
                snapshot.token == *token
                    && snapshot.document.revision == revision
                    && snapshot.document.id == scope.document_id
                    && snapshot.session_epoch == scope.session_epoch
            })
            && scope.board_id == *board_id
            && lifecycle_current
            && model.durability == (Durability::Saved { revision })
            && (model.display_preview.is_none()
                || matches!(
                    self,
                    OutlineAction::EditPerimeter {
                        phase: EditPhase::Preview | EditPhase::Commit,
                        ..
                    }
                ))
            && model.gesture.is_none()
    }

    pub(super) fn for_tree(
        snapshot: &AcceptedSnapshot,
        scope: &Scope,
        generation: u64,
        context: &super::objects::TreeContext,
    ) -> Option<Self> {
        let super::objects::TreeContext::OutlineVersion {
            board_id,
            version_id,
        } = context
        else {
            return None;
        };
        Some(Self::Activate {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation,
            context: context.clone(),
            require_selected_version: true,
            board_id: board_id.clone(),
            version_id: version_id.clone(),
        })
    }
}

pub(super) fn use_outline_lifecycle(
    runtime: Rc<Runtime>,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
) -> (
    Option<OutlineInspectorProjection>,
    EventHandler<OutlineAction>,
) {
    let version = use_context::<Signal<u64>>()();
    let captured_generation = scope_generation();
    let pending = use_signal(|| None::<Pending>);
    let feedback = use_signal(|| None::<OutlineFeedback>);
    let selected_point = use_signal(|| 0usize);
    let editing_points = use_signal(|| false);
    let drawing_operation = use_signal(|| None::<OutlineDrawTool>);
    let drawing_points = use_signal(Vec::<Vec2>::new);
    let selected_feature_id = use_signal(|| None::<String>);
    let selected_connection_id = use_signal(|| None::<String>);
    let draft_owner = selected_context.read().clone();
    let draft_workspace = workspace();
    let draft_active_version = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|snapshot| {
            let board_id = draft_owner
                .as_ref()
                .and_then(|owner| match &owner.context {
                    super::objects::TreeContext::Outline { board_id }
                    | super::objects::TreeContext::OutlineVersion { board_id, .. } => {
                        Some(board_id)
                    }
                    _ => None,
                })?;
            snapshot
                .document
                .board_outlines
                .iter()
                .find(|state| &state.board_id == board_id)
        })
        .and_then(|state| state.active_version_id.clone());
    let draft_scope = runtime.scope();
    use_effect(use_reactive(
        (
            &version,
            &draft_owner,
            &draft_workspace,
            &captured_generation,
        ),
        {
            let mut drawing_operation = drawing_operation;
            let mut drawing_points = drawing_points;
            move |_| {
                drawing_operation.set(None);
                drawing_points.set(Vec::new());
            }
        },
    ));
    use_effect(use_reactive(
        (
            &draft_owner,
            &draft_workspace,
            &draft_active_version,
            &draft_scope,
            &captured_generation,
        ),
        {
            let mut selected_feature_id = selected_feature_id;
            let mut selected_connection_id = selected_connection_id;
            move |_| {
                selected_feature_id.set(None);
                selected_connection_id.set(None);
            }
        },
    ));
    let action_state = ActionState {
        pending,
        feedback,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
    };
    let on_action = use_callback({
        let runtime = runtime.clone();
        move |action| {
            submit_action(&runtime, action_state, action);
        }
    });

    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow().clone() else {
                return;
            };
            match outcome {
                TerminalOutcome::Completed => {
                    let model = runtime.model();
                    let Some(snapshot) = model.accepted.as_ref().filter(|snapshot| {
                        snapshot.document.id == waiting.snapshot.document.id
                            && snapshot.session_epoch == waiting.snapshot.session_epoch
                            && runtime.scope().as_ref() == Some(&waiting.scope)
                    }) else {
                        pending.set(None);
                        feedback.set(Some(OutlineFeedback {
                            scope: waiting.scope.clone(),
                            generation: waiting.generation,
                            board_id: waiting.scope.board_id.clone(),
                            state: "source-changed",
                            message: Some(
                                "The outline source changed before its result could be confirmed."
                                    .into(),
                            ),
                        }));
                        return;
                    };
                    if matches!(model.durability, Durability::Failed { .. })
                        || matches!(
                            model.lifecycle,
                            Lifecycle::RecoveryRequired | Lifecycle::Closed
                        )
                    {
                        pending.set(None);
                        feedback.set(Some(OutlineFeedback {
                            scope: waiting.scope.clone(),
                            generation: waiting.generation,
                            board_id: waiting.scope.board_id.clone(),
                            state: "recovery-required",
                            message: Some("The outline operation completed, but the accepted document is not durably available. Retry after recovery.".into()),
                        }));
                        return;
                    }
                    if snapshot.token == waiting.snapshot.token
                        || snapshot.document.revision <= waiting.snapshot.document.revision
                    {
                        return;
                    }
                    if model.lifecycle != Lifecycle::Ready
                        || model.durability
                            != (Durability::Saved {
                                revision: snapshot.document.revision,
                            })
                    {
                        return;
                    }
                    let board_state = snapshot
                        .document
                        .board_outlines
                        .iter()
                        .find(|state| state.board_id == waiting.scope.board_id);
                    let applied = match &waiting.kind {
                        PendingKind::Activate { version_id } => {
                            board_state.and_then(|state| state.active_version_id.as_ref())
                                == version_id.as_ref()
                        }
                        PendingKind::Copy {
                            version_id,
                            edited_contour,
                        } => board_state.is_some_and(|state| {
                            state.active_version_id.as_deref() == Some(version_id)
                                && state.versions.iter().any(|version| {
                                    version.id == *version_id
                                        && edited_contour.as_ref().is_none_or(
                                            |(contour, points)| {
                                                matches!(
                                                    version.geometry.features.get(*contour as usize),
                                                    Some(OutlineFeature::Polygon {
                                                        points: accepted,
                                                        ..
                                                    }) if accepted == points
                                                )
                                            },
                                        )
                                })
                        }),
                        PendingKind::AddedFeature { version_id, feature_id }
                        | PendingKind::UpdatedFeature { version_id, feature_id } => board_state
                            .and_then(|state| state.versions.iter().find(|version| version.id == *version_id))
                            .is_some_and(|version| {
                                version.geometry.features.iter().any(|feature| feature.id() == *feature_id)
                                    && (matches!(&waiting.kind, PendingKind::UpdatedFeature { .. })
                                        || board_state.is_some_and(|state| state.active_version_id.as_deref() == Some(version_id)))
                            }),
                        PendingKind::RemovedFeature { version_id, feature_id } => board_state
                            .is_some_and(|state| state.active_version_id.as_deref() == Some(version_id.as_str())
                                && state.versions.iter().find(|version| version.id == *version_id)
                                    .is_some_and(|version| version.geometry.features.iter().all(|feature| feature.id() != *feature_id))),
                        PendingKind::Delete { version_id } => board_state.is_none_or(|state| {
                            state.active_version_id.is_none()
                                && state
                                    .versions
                                    .iter()
                                    .all(|version| version.id != *version_id)
                        }),
                        PendingKind::Edit(expectation) => expectation_applied(
                            &snapshot.document,
                            &waiting.scope.board_id,
                            expectation,
                        ),
                    };
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: if applied { "saved" } else { "failed" },
                        message: (!applied).then(|| "The saved outline no longer matches this action. Review the accepted version and retry.".into()),
                    }));
                }
                TerminalOutcome::Rejected(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "rejected",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::PersistenceFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "persistence-failed",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::BlockedByRecovery(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "recovery-required",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::ExecutorFailed(message) => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "executor-failed",
                        message: Some(message),
                    }));
                }
                TerminalOutcome::Superseded
                | TerminalOutcome::Cancelled
                | TerminalOutcome::Closed => {
                    pending.set(None);
                    feedback.set(Some(OutlineFeedback {
                        scope: waiting.scope.clone(),
                        generation: waiting.generation,
                        board_id: waiting.scope.board_id.clone(),
                        state: "cancelled",
                        message: Some(
                            "The outline change did not complete in the active session.".into(),
                        ),
                    }));
                }
            }
        }
    }));

    (
        project_inspector(&runtime, action_state, on_action),
        on_action,
    )
}

fn project_inspector(
    runtime: &Runtime,
    state: ActionState,
    on_action: EventHandler<OutlineAction>,
) -> Option<OutlineInspectorProjection> {
    let ActionState {
        pending,
        feedback,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        selected_context,
        workspace,
        scope_generation,
        captured_generation,
    } = state;
    let model = runtime.model();
    let scope = runtime.scope()?;
    if workspace() != "Layout" || scope_generation() != captured_generation {
        return None;
    }
    let snapshot = model.accepted.as_ref()?;
    let selected = selected_context.read().clone()?;
    if selected.scope != scope
        || !matches!(
            selected.context,
            super::objects::TreeContext::Outline { .. }
                | super::objects::TreeContext::OutlineVersion { .. }
        )
        || !super::selection::context_is_current(&model, &scope, &selected.context)
    {
        return None;
    }
    let board_id = match &selected.context {
        super::objects::TreeContext::Outline { board_id }
        | super::objects::TreeContext::OutlineVersion { board_id, .. } => board_id.clone(),
        _ => return None,
    };
    if board_id != scope.board_id {
        return None;
    }
    let board_name = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)?
        .name
        .clone();
    let outline = snapshot
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == board_id);
    let active_version_id = outline.and_then(|state| state.active_version_id.clone());
    let version_name = outline
        .and_then(|state| {
            state
                .versions
                .iter()
                .find(|version| Some(&version.id) == active_version_id.as_ref())
        })
        .map_or_else(|| "Generated".to_owned(), |version| version.name.clone());
    let board = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == board_id)?;
    let versions = outline
        .into_iter()
        .flat_map(|state| state.versions.iter())
        .map(|version| OutlineVersionChoice {
            id: version.id.clone(),
            name: version.name.clone(),
        })
        .collect();
    let generated = generated_feature(&snapshot.document, board);
    let settings = outline
        .and_then(|state| {
            active_version_id.as_ref().and_then(|active_id| {
                state
                    .versions
                    .iter()
                    .find(|version| &version.id == active_id)
                    .map(|version| version.geometry.settings.clone())
            })
        })
        .or_else(|| generated.and_then(generated_settings))
        .unwrap_or_default();
    let repair = settings.repair.clone().unwrap_or_default();
    let gaps = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map_or_else(Vec::new, |scene| scene.gaps.clone());
    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map_or_else(Vec::new, |board| board.contours.clone());
    let geometry_features = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .map(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                        .cloned()
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_else(|| {
            board
                .outline_ids
                .iter()
                .filter_map(|id| {
                    snapshot
                        .document
                        .outline
                        .iter()
                        .find(|feature| feature.id() == id)
                })
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .cloned()
                .collect()
        });
    let connections = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .and_then(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .find_map(|feature| match feature {
                            OutlineFeature::PartEnvelope { connections, .. } => {
                                Some(connections.clone())
                            }
                            _ => None,
                        })
                })
        })
        .or_else(|| {
            generated.and_then(|feature| match feature {
                OutlineFeature::PartEnvelope { connections, .. } => Some(connections.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();
    let connection_feature = active_version_id
        .as_deref()
        .and_then(|version_id| {
            outline
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == version_id)
                .and_then(|version| {
                    version
                        .geometry
                        .features
                        .iter()
                        .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
                        .cloned()
                })
        })
        .or_else(|| generated.cloned());
    let outline_parts = board
        .part_ids
        .iter()
        .filter_map(|id| snapshot.document.parts.iter().find(|part| &part.id == id))
        .cloned()
        .collect::<Vec<_>>();
    let perimeter = editable_perimeter(
        snapshot,
        &board_id,
        active_version_id.as_deref(),
        selected_feature_id.read().as_deref(),
    );
    let snap_paths = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)
        .map(|scene| {
            scene
                .source_contours
                .iter()
                .map(|contour| contour.points.clone())
                .collect()
        })
        .unwrap_or_default();
    let editable = model.durability
        == Durability::Saved {
            revision: snapshot.document.revision,
        }
        && model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && pending.read().is_none();
    let visible_feedback = feedback
        .read()
        .as_ref()
        .filter(|state| {
            state.board_id == board_id
                && state.scope == scope
                && state.generation == captured_generation
        })
        .cloned();
    let pending_feedback = pending
        .read()
        .as_ref()
        .filter(|waiting| waiting.scope == scope && waiting.generation == captured_generation)
        .map(|waiting| OutlineFeedback {
            scope: waiting.scope.clone(),
            generation: waiting.generation,
            board_id: waiting.scope.board_id.clone(),
            state: "pending",
            message: None,
        });
    Some(OutlineInspectorProjection {
        board_id: board_id.clone(),
        board_name,
        version_name,
        active_version_id,
        contours,
        perimeter,
        snap_paths,
        selected_point,
        editing_points,
        drawing_operation,
        drawing_points,
        selected_feature_id,
        selected_connection_id,
        geometry_features,
        connections,
        connection_feature,
        outline_parts,
        versions,
        settings,
        repair,
        generated_margin: generated.and_then(generated_margin),
        has_generated: generated.is_some(),
        gaps,
        action_context: Rc::new(OutlineActionContext {
            scope: scope.clone(),
            token: snapshot.token,
            revision: snapshot.document.revision,
            generation: captured_generation,
            board_id: board_id.clone(),
            selection_context: selected.context,
        }),
        enabled: editable,
        feedback: pending_feedback.or(visible_feedback),
        on_action,
        selected_context,
        scope,
        token: snapshot.token,
        revision: snapshot.document.revision,
        generation: captured_generation,
    })
}

fn editable_perimeter(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    active_version_id: Option<&str>,
    selected_feature_id: Option<&str>,
) -> Option<EditablePerimeter> {
    if let Some(version_id) = active_version_id {
        let feature = snapshot
            .document
            .board_outlines
            .iter()
            .find(|state| state.board_id == board_id)?
            .versions
            .iter()
            .find(|version| version.id == version_id)?
            .geometry
            .features
            .iter()
            .find(|feature| {
                matches!(feature, OutlineFeature::Polygon { .. })
                    && selected_feature_id.is_none_or(|id| feature.id() == id)
            })?;
        let OutlineFeature::Polygon {
            id,
            points,
            anchor_part_id,
            operation,
        } = feature
        else {
            return None;
        };
        return Some(EditablePerimeter {
            target: OutlinePointTarget::Fixed {
                version_id: version_id.to_owned(),
                feature_id: id.clone(),
                anchor_part_id: anchor_part_id.clone(),
                operation: *operation,
            },
            points: points.clone(),
            canvas_points: points
                .iter()
                .map(|point| {
                    anchor_part_id
                        .as_deref()
                        .and_then(|id| snapshot.document.parts.iter().find(|part| part.id == id))
                        .map(|part| PerimeterAnchor {
                            at: part.pose.at,
                            rotation: part.pose.rotation,
                            back: part.side == Side::Back,
                        })
                        .map_or(*point, |anchor| anchor.world(*point))
                })
                .collect(),
            anchor: anchor_part_id
                .as_deref()
                .and_then(|id| snapshot.document.parts.iter().find(|part| part.id == id))
                .map(|part| PerimeterAnchor {
                    at: part.pose.at,
                    rotation: part.pose.rotation,
                    back: part.side == Side::Back,
                }),
        });
    }

    let source = snapshot
        .scene
        .board_outline_scenes
        .iter()
        .find(|scene| scene.board_id == board_id)?
        .source_contours
        .first()?;
    Some(EditablePerimeter {
        target: OutlinePointTarget::Generated { contour: 0 },
        points: source.points.clone(),
        canvas_points: source.points.clone(),
        anchor: None,
    })
}

fn submit_action(runtime: &Rc<Runtime>, state: ActionState, action: OutlineAction) {
    let mut pending = state.pending;
    let mut feedback = state.feedback;
    let mut selected_context = state.selected_context;
    let workspace = state.workspace;
    let scope_generation = state.scope_generation;
    let captured_generation = state.captured_generation;
    if pending.read().is_some()
        || workspace() != "Layout"
        || scope_generation() != captured_generation
    {
        return;
    }
    if !action.is_current(runtime, captured_generation) {
        return;
    }
    let (action_scope, _, expected_revision, _, board_id) = action.envelope();
    let model = runtime.model();
    let Some(snapshot) = model.accepted.as_ref() else {
        return;
    };
    let selected = selected_context.read().clone();
    let selection_is_current = selected.as_ref().is_some_and(|selected| {
        selected.scope == *action_scope
            && matches!(&selected.context,
                super::objects::TreeContext::Outline { board_id: selected_board }
                | super::objects::TreeContext::OutlineVersion { board_id: selected_board, .. }
                if selected_board == board_id)
            && super::selection::context_is_current(&model, action_scope, &selected.context)
    });
    if !selection_is_current {
        return;
    }
    if let OutlineAction::FocusGap { gap_id, .. } = &action {
        let Some(gap) = snapshot
            .scene
            .board_outline_scenes
            .iter()
            .find(|scene| scene.board_id == *board_id)
            .and_then(|scene| scene.gaps.iter().find(|gap| gap.id == *gap_id))
        else {
            return;
        };
        let points = gap
            .points
            .iter()
            .map(|point| connection_point_world(point, &snapshot.document.parts))
            .filter(|point| point.x.is_finite() && point.y.is_finite())
            .collect::<Vec<_>>();
        let Some((min_x, max_x, min_y, max_y)) =
            points
                .iter()
                .fold(None::<(f64, f64, f64, f64)>, |bounds, point| {
                    Some(match bounds {
                        Some((min_x, max_x, min_y, max_y)) => (
                            min_x.min(point.x),
                            max_x.max(point.x),
                            min_y.min(point.y),
                            max_y.max(point.y),
                        ),
                        None => (point.x, point.x, point.y, point.y),
                    })
                })
        else {
            return;
        };
        let Some((base_min_x, base_max_x, base_min_y, base_max_y)) =
            super::keycaps_fit::layout_canvas_bounds(&snapshot.document, &snapshot.scene, board_id)
        else {
            return;
        };
        // Match React's canvas fit around a contour: include its 12 mm breathing room and
        // preserve the target offset relative to the stable Layout camera basis.
        let target_width = (max_x - min_x + 24.0).max(1.0);
        let target_height = (max_y - min_y + 24.0).max(1.0);
        let base_width = (base_max_x - base_min_x).max(50.0);
        let base_height = (base_max_y - base_min_y).max(50.0);
        let (surface_width, surface_height) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.query_selector("svg.m1-canvas").ok().flatten())
            .map(|surface| {
                let bounds = surface.get_bounding_client_rect();
                (bounds.width().max(1.0), bounds.height().max(1.0))
            })
            .unwrap_or((800.0, 600.0));
        let usable_width = (surface_width - 32.0).max(1.0);
        let usable_height = (surface_height - 72.0 - 24.0).max(1.0);
        let zoom = (base_width / target_width * usable_width / surface_width)
            .min(base_height / target_height * usable_height / surface_height)
            .clamp(0.15, 8.0);
        let center = Vec2 {
            x: (min_x + max_x - base_min_x - base_max_x) * 0.5,
            y: (min_y + max_y - base_min_y - base_max_y) * 0.5
                + (72.0 - 24.0) * base_height / zoom / surface_height * 0.5,
        };
        runtime.submit(Event::SetCamera {
            operation_id: runtime.operation(),
            center,
            zoom,
        });
        return;
    }
    let state = snapshot
        .document
        .board_outlines
        .iter()
        .find(|state| state.board_id == *board_id);
    let generated = snapshot
        .document
        .boards
        .iter()
        .find(|board| board.id == *board_id)
        .and_then(|board| generated_feature(&snapshot.document, board));
    let operation_id = runtime.operation();
    let (operation, kind, target_ids) = match &action {
        OutlineAction::Activate {
            version_id,
            context,
            require_selected_version,
            ..
        } => {
            if !selected.as_ref().is_some_and(|selected| {
                if selected.context != *context {
                    return false;
                }
                match (&selected.context, version_id) {
                    (
                        super::objects::TreeContext::Outline {
                            board_id: selected_board,
                        },
                        _,
                    ) => selected_board == board_id,
                    (
                        super::objects::TreeContext::OutlineVersion {
                            board_id: selected_board,
                            version_id: selected_version,
                        },
                        version_id,
                    ) => {
                        selected_board == board_id
                            && (!require_selected_version || selected_version == version_id)
                    }
                    _ => false,
                }
            }) || version_id.as_ref().is_some_and(|id| {
                !state.is_some_and(|state| state.versions.iter().any(|version| version.id == *id))
            }) || state.and_then(|state| state.active_version_id.as_ref()) == version_id.as_ref()
            {
                return;
            }
            (
                EditOperation::SelectOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                PendingKind::Activate {
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::Copy { .. } => {
            let next_number = state
                .into_iter()
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
            let name = format!("Edited outline {next_number}");
            let version_id = unique_outline_version_id(
                operation_id.0,
                snapshot
                    .document
                    .board_outlines
                    .iter()
                    .flat_map(|state| &state.versions)
                    .map(|version| version.id.clone()),
            );
            (
                EditOperation::CopyOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                    name,
                    edit: None,
                    feature: None,
                },
                PendingKind::Copy {
                    version_id,
                    edited_contour: None,
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::Delete { version_id, .. } => {
            if state.and_then(|state| state.active_version_id.as_deref())
                != Some(version_id.as_str())
                || !state
                    .into_iter()
                    .flat_map(|state| &state.versions)
                    .any(|version| version.id == *version_id)
            {
                return;
            }
            selected_context.set(Some(super::objects::ScopedTreeContext {
                scope: action_scope.clone(),
                context: super::objects::TreeContext::Outline {
                    board_id: board_id.clone(),
                },
            }));
            (
                EditOperation::RemoveOutline {
                    board_id: board_id.clone(),
                    version_id: version_id.clone(),
                },
                PendingKind::Delete {
                    version_id: version_id.clone(),
                },
                vec![board_id.clone()],
            )
        }
        OutlineAction::AddFeature {
            context, feature, ..
        } => {
            if !selected
                .as_ref()
                .is_some_and(|selected| selected.context == *context)
            {
                return;
            }
            if selected
                .as_ref()
                .is_some_and(|selected| match &selected.context {
                    super::objects::TreeContext::OutlineVersion { version_id, .. } => {
                        state.and_then(|state| state.active_version_id.as_ref())
                            != version_id.as_ref()
                    }
                    _ => false,
                })
            {
                return;
            }
            let OutlineFeature::Polygon { id, points, .. } = feature else {
                return;
            };
            if points.len() < 3
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
                || snapshot.document.outline.iter().any(|item| item.id() == id)
                || snapshot
                    .document
                    .board_outlines
                    .iter()
                    .flat_map(|item| &item.versions)
                    .flat_map(|version| &version.geometry.features)
                    .any(|item| item.id() == id)
            {
                return;
            }
            if let Some(version_id) = state.and_then(|state| state.active_version_id.as_deref()) {
                let mut document = snapshot.document.as_ref().clone();
                let Some(version) = document
                    .board_outlines
                    .iter_mut()
                    .find(|outline| outline.board_id == *board_id)
                    .and_then(|outline| {
                        outline
                            .versions
                            .iter_mut()
                            .find(|version| version.id == version_id)
                    })
                else {
                    return;
                };
                version.geometry.features.push(feature.clone());
                (
                    EditOperation::ReplaceDocument {
                        document: Box::new(document),
                    },
                    PendingKind::UpdatedFeature {
                        version_id: version_id.to_owned(),
                        feature_id: id.clone(),
                    },
                    vec![board_id.clone(), id.clone()],
                )
            } else {
                let next_number = state
                    .into_iter()
                    .flat_map(|state| &state.versions)
                    .filter_map(|version| {
                        version
                            .name
                            .strip_prefix("Edited outline ")
                            .and_then(|n| n.parse::<u32>().ok())
                    })
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1);
                let version_id = unique_outline_version_id(
                    operation_id.0,
                    snapshot
                        .document
                        .board_outlines
                        .iter()
                        .flat_map(|state| &state.versions)
                        .map(|version| version.id.clone()),
                );
                (
                    EditOperation::CopyOutline {
                        board_id: board_id.clone(),
                        version_id: version_id.clone(),
                        name: format!("Edited outline {next_number}"),
                        edit: None,
                        feature: Some(feature.clone()),
                    },
                    PendingKind::AddedFeature {
                        version_id,
                        feature_id: id.clone(),
                    },
                    vec![board_id.clone(), id.clone()],
                )
            }
        }
        OutlineAction::AddConnection {
            context, points, ..
        } => {
            if selected
                .as_ref()
                .is_none_or(|selected| selected.context != *context)
                || state
                    .and_then(|state| state.active_version_id.as_ref())
                    .is_some()
                || points.len() < 2
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return;
            }
            let Some(OutlineFeature::PartEnvelope {
                part_ids, settings, ..
            }) = generated
            else {
                return;
            };
            let eligible = snapshot
                .document
                .parts
                .iter()
                .filter(|part| {
                    part_ids.contains(&part.id)
                        && !part
                            .outline
                            .as_ref()
                            .is_some_and(|outline| outline.excluded)
                })
                .collect::<Vec<_>>();
            let mut feature = generated.expect("matched generated feature").clone();
            let OutlineFeature::PartEnvelope { connections, .. } = &mut feature else {
                return;
            };
            connections.push(OutlineConnection {
                id: unique_outline_entity_id(
                    "outline-connection",
                    operation_id.0,
                    connections.iter().map(|connection| connection.id.clone()),
                ),
                width: settings.bridge_width,
                points: points
                    .iter()
                    .enumerate()
                    .map(|(index, point)| {
                        if index == 0 || index + 1 == points.len() {
                            outline_connection_control(*point, &eligible)
                        } else {
                            OutlineControlPoint {
                                at: *point,
                                part_id: None,
                            }
                        }
                    })
                    .collect(),
            });
            let id = feature.id().to_owned();
            (
                EditOperation::SetOutline {
                    feature: feature.clone(),
                },
                PendingKind::Edit(OutlineExpectation::GeneratedFeature(feature)),
                vec![board_id.clone(), id],
            )
        }
        OutlineAction::SetFeature {
            context,
            version_id,
            before,
            after,
            ..
        } => {
            if selected
                .as_ref()
                .is_none_or(|selected| selected.context != *context)
                || before.id() != after.id()
                || state.and_then(|state| state.active_version_id.as_deref())
                    != version_id.as_deref()
            {
                return;
            }
            let accepted = if let Some(version_id) = version_id {
                state
                    .into_iter()
                    .flat_map(|state| &state.versions)
                    .find(|version| version.id == *version_id)
                    .and_then(|version| {
                        version
                            .geometry
                            .features
                            .iter()
                            .find(|feature| feature.id() == before.id())
                    })
            } else {
                snapshot.document.outline.iter().find(|feature| {
                    feature.id() == before.id()
                        && snapshot
                            .document
                            .boards
                            .iter()
                            .find(|board| board.id == *board_id)
                            .is_some_and(|board| {
                                board.outline_ids.iter().any(|id| id == feature.id())
                            })
                })
            };
            if accepted != Some(before) {
                return;
            }
            let expectation = version_id.as_ref().map_or_else(
                || OutlineExpectation::GeneratedFeature(after.clone()),
                |version_id| OutlineExpectation::VersionFeature {
                    version_id: version_id.clone(),
                    feature: after.clone(),
                },
            );
            let operation = if let Some(version_id) = version_id {
                let mut document = snapshot.document.as_ref().clone();
                let Some(target) = document
                    .board_outlines
                    .iter_mut()
                    .find(|outline| outline.board_id == *board_id)
                    .and_then(|outline| {
                        outline
                            .versions
                            .iter_mut()
                            .find(|version| version.id == *version_id)
                    })
                else {
                    return;
                };
                let Some(feature) = target
                    .geometry
                    .features
                    .iter_mut()
                    .find(|feature| feature.id() == before.id())
                else {
                    return;
                };
                *feature = after.clone();
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                }
            } else {
                EditOperation::SetOutline {
                    feature: after.clone(),
                }
            };
            (
                operation,
                PendingKind::Edit(expectation),
                vec![board_id.clone(), before.id().to_owned()],
            )
        }
        OutlineAction::RemoveFeature {
            context,
            version_id,
            feature_id,
            ..
        } => {
            if selected
                .as_ref()
                .is_none_or(|selected| selected.context != *context)
                || state.and_then(|state| state.active_version_id.as_deref())
                    != Some(version_id.as_str())
            {
                return;
            }
            let Some(version) = state
                .into_iter()
                .flat_map(|state| &state.versions)
                .find(|version| version.id == *version_id)
            else {
                return;
            };
            let authored_index = version
                .geometry
                .features
                .iter()
                .filter(|feature| !matches!(feature, OutlineFeature::PartEnvelope { .. }))
                .position(|feature| feature.id() == feature_id);
            if authored_index.is_none_or(|index| index == 0) {
                return;
            }
            let mut document = snapshot.document.as_ref().clone();
            let Some(target) = document
                .board_outlines
                .iter_mut()
                .find(|outline| outline.board_id == *board_id)
                .and_then(|outline| {
                    outline
                        .versions
                        .iter_mut()
                        .find(|version| version.id == *version_id)
                })
            else {
                return;
            };
            target
                .geometry
                .features
                .retain(|feature| feature.id() != feature_id);
            (
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
                PendingKind::RemovedFeature {
                    version_id: version_id.clone(),
                    feature_id: feature_id.clone(),
                },
                vec![board_id.clone(), feature_id.clone()],
            )
        }
        OutlineAction::FocusGap { .. } => return,
        OutlineAction::Update { edit, .. } => {
            let Some((operation, expectation, target_ids)) = apply_outline_edit(
                &snapshot.document,
                &snapshot.scene,
                board_id,
                edit,
                operation_id,
            ) else {
                return;
            };
            (operation, PendingKind::Edit(expectation), target_ids)
        }
        OutlineAction::EditPerimeter {
            context,
            target,
            points,
            ..
        } => {
            if !selected
                .as_ref()
                .is_some_and(|selected| selected.context == *context)
            {
                return;
            }
            if points.len() < 3
                || points
                    .iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
            {
                return;
            }
            match target {
                OutlinePointTarget::Generated { contour } => {
                    let Some(source_points) = snapshot
                        .scene
                        .board_outline_scenes
                        .iter()
                        .find(|scene| scene.board_id == *board_id)
                        .and_then(|scene| scene.source_contours.get(*contour as usize))
                        .map(|source| &source.points)
                    else {
                        return;
                    };
                    if state
                        .and_then(|state| state.active_version_id.as_ref())
                        .is_some()
                        || source_points == points
                    {
                        return;
                    }
                    let version_id = unique_outline_version_id(
                        operation_id.0,
                        snapshot
                            .document
                            .board_outlines
                            .iter()
                            .flat_map(|state| &state.versions)
                            .map(|version| version.id.clone()),
                    );
                    let next_number = state
                        .into_iter()
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
                    let name = format!("Edited outline {next_number}");
                    (
                        EditOperation::CopyOutline {
                            board_id: board_id.clone(),
                            version_id: version_id.clone(),
                            name,
                            edit: Some(OutlineContourEdit {
                                contour: *contour,
                                points: points.clone(),
                            }),
                            feature: None,
                        },
                        PendingKind::Copy {
                            version_id,
                            edited_contour: Some((*contour, points.clone())),
                        },
                        vec![board_id.clone()],
                    )
                }
                OutlinePointTarget::Fixed {
                    version_id,
                    feature_id,
                    anchor_part_id,
                    operation,
                } => {
                    if state.and_then(|state| state.active_version_id.as_deref())
                        != Some(version_id.as_str())
                    {
                        return;
                    }
                    let Some(OutlineFeature::Polygon {
                        id,
                        points: current,
                        anchor_part_id: current_anchor,
                        operation: current_operation,
                    }) = state
                        .into_iter()
                        .flat_map(|state| &state.versions)
                        .find(|version| version.id == *version_id)
                        .and_then(|version| {
                            version
                                .geometry
                                .features
                                .iter()
                                .find(|feature| feature.id() == *feature_id)
                        })
                    else {
                        return;
                    };
                    if current == points
                        || current_anchor != anchor_part_id
                        || current_operation != operation
                    {
                        return;
                    }
                    let feature = OutlineFeature::Polygon {
                        id: id.clone(),
                        points: points.clone(),
                        anchor_part_id: anchor_part_id.clone(),
                        operation: *operation,
                    };
                    (
                        EditOperation::SetOutline {
                            feature: feature.clone(),
                        },
                        PendingKind::Edit(OutlineExpectation::VersionFeature {
                            version_id: version_id.clone(),
                            feature,
                        }),
                        vec![board_id.clone(), feature_id.clone()],
                    )
                }
            }
        }
    };
    let (phase, transaction_id) = match &action {
        OutlineAction::EditPerimeter {
            phase,
            transaction_id,
            ..
        } => (*phase, transaction_id.clone()),
        _ => (
            EditPhase::Commit,
            format!("outline-lifecycle-{}", operation_id.0),
        ),
    };
    if phase == EditPhase::Preview {
        runtime.submit(Event::Edit {
            operation_id,
            command: EditCommand {
                base_revision: expected_revision,
                transaction_id,
                phase,
                target_ids,
                operation,
            },
        });
        return;
    }
    let outcome = runtime.observe_operation(operation_id);
    pending.set(Some(Pending {
        scope: action_scope.clone(),
        snapshot: snapshot.clone(),
        generation: captured_generation,
        kind,
        outcome: outcome.clone(),
    }));
    feedback.set(Some(OutlineFeedback {
        scope: action_scope.clone(),
        generation: captured_generation,
        board_id: board_id.clone(),
        state: "pending",
        message: None,
    }));
    runtime.submit(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: expected_revision,
            transaction_id,
            phase,
            target_ids,
            operation,
        },
    });
    // Keep exact observation alive independently of the Editor's signal lifetime.
    // This task never reads or writes a component signal after an await.
    wasm_bindgen_futures::spawn_local(async move {
        while outcome.borrow().is_none() {
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

impl OutlineInspectorProjection {
    fn copy_action(&self) -> OutlineAction {
        OutlineAction::Copy {
            scope: self.scope.clone(),
            token: self.token,
            revision: self.revision,
            generation: self.generation,
            board_id: self.board_id.clone(),
        }
    }

    fn delete_action(&self) -> Option<OutlineAction> {
        self.active_version_id
            .as_ref()
            .map(|version_id| OutlineAction::Delete {
                scope: self.scope.clone(),
                token: self.token,
                revision: self.revision,
                generation: self.generation,
                board_id: self.board_id.clone(),
                version_id: version_id.clone(),
            })
    }
}

#[component]
pub(super) fn OutlineVersionInspector(projection: OutlineInspectorProjection) -> Element {
    let contour_view = contour_preview(&projection.contours);
    let copy = projection.copy_action();
    let delete = projection.delete_action();
    let copy_handler = projection.on_action;
    let delete_handler = projection.on_action;
    let mut version_name = use_signal(|| None::<(String, String, String)>);
    let mut perimeter_open = projection.editing_points;
    let mut selected_point = projection.selected_point;
    let mut selected_feature_id = projection.selected_feature_id;
    let mut selected_connection_id = projection.selected_connection_id;
    let name_draft = version_name()
        .filter(|(id, baseline, _)| {
            Some(id) == projection.active_version_id.as_ref()
                && baseline == &projection.version_name
        })
        .map(|(_, _, draft)| draft)
        .unwrap_or_else(|| projection.version_name.clone());
    let active_version = projection.active_version_id.clone();
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let enabled = projection.enabled;
    let mut drawing_operation = projection.drawing_operation;
    let mut drawing_points = projection.drawing_points;
    let mut draft_serial = use_signal(|| 0u64);
    let active_value = projection.active_version_id.clone().unwrap_or_default();
    let perimeter = projection.perimeter.clone();
    let corner_value = match projection.settings.corners {
        CornerStyle::Sharp => "sharp",
        CornerStyle::Fillet => "fillet",
        CornerStyle::Chamfer => "chamfer",
    };
    let size_label: &'static str = match projection.settings.corners {
        CornerStyle::Fillet => "Fillet radius",
        _ => "Chamfer size",
    };
    let point_index = perimeter
        .as_ref()
        .map(|perimeter| selected_point().min(perimeter.points.len().saturating_sub(1)))
        .unwrap_or_default();
    let point = perimeter
        .as_ref()
        .and_then(|perimeter| perimeter.canvas_points.get(point_index).copied())
        .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
    let point_count = perimeter
        .as_ref()
        .map(|perimeter| perimeter.points.len())
        .unwrap_or_default();
    rsx! {
                if perimeter_open() {
            if let Some(perimeter) = perimeter.as_ref() {
                section { class: "m1-outline-inspector m1-outline-point-editor", "aria-label": "Perimeter",
                    onkeydown: {
                        let mut selected_context = projection.selected_context;
                        let scope = projection.scope.clone();
                        let board_id = projection.board_id.clone();
                        move |event: KeyboardEvent| {
                            if event.data().key().to_string() != "Escape" { return; }
                            event.prevent_default();
                            event.stop_propagation();
                            let Some(selected) = selected_context.read().clone() else { return; };
                            if selected.scope != scope
                                || !matches!(selected.context, super::objects::TreeContext::Outline { board_id: selected_board } if selected_board == board_id)
                            { return; }
                            selected_context.set(Some(super::objects::ScopedTreeContext {
                                scope: scope.clone(),
                                context: super::objects::TreeContext::Board { board_id: board_id.clone() },
                            }));
                        }
                    },
                    div { class: "m1-outline-inspector-heading",
                        h2 { "Perimeter" }
                        button { r#type: "button", disabled: !enabled, onclick: move |_| perimeter_open.set(false), "Done" }
                    }
                    p { if projection.active_version_id.is_some() { "This outline stays fixed when components move. Changes save as you edit." } else { "The first point change creates and activates a fixed copy. Generated stays available." } }
                    h3 { class: "m1-outline-point-heading", "Point {point_index + 1} of {point_count}" }
                    div { class: "m1-outline-coordinate-fields",
                        OutlineCoordinate {
                            key: format!("{:?}:{:?}:{point_index}:x", action_context.scope, perimeter.target),
                            label: format!("Point {} X mm", point_index + 1),
                            value: point.x,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                let world_points = perimeter.canvas_points.clone();
                                let anchor = perimeter.anchor;
                                move |value| {
                                    if let (Some(point), Some(world)) = (points.get_mut(point_index), world_points.get(point_index)) {
                                        let next = Vec2 { x: value, y: world.y };
                                        *point = anchor.map_or(next, |anchor| anchor.local(next));
                                    }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                        }
                        OutlineCoordinate {
                            key: format!("{:?}:{:?}:{point_index}:y", action_context.scope, perimeter.target),
                            label: format!("Point {} Y mm", point_index + 1),
                            value: point.y,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                let world_points = perimeter.canvas_points.clone();
                                let anchor = perimeter.anchor;
                                move |value| {
                                    if let (Some(point), Some(world)) = (points.get_mut(point_index), world_points.get(point_index)) {
                                        let next = Vec2 { x: world.x, y: value };
                                        *point = anchor.map_or(next, |anchor| anchor.local(next));
                                    }
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                        }
                    }
                    div { class: "m1-outline-point-actions",
                        button {
                            class: "m1-outline-insert-point",
                            r#type: "button",
                            disabled: !enabled,
                            aria_label: "Insert after {point_index + 1}",
                            onclick: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |_| {
                                    if point_count < 3 { return; }
                                    let next = (point_index + 1) % point_count;
                                    let a = points[point_index];
                                    let b = points[next];
                                    points.insert(point_index + 1, Vec2 { x: (a.x + b.x) / 2.0, y: (a.y + b.y) / 2.0 });
                                    selected_point.set(point_index + 1);
                                    on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                            "Insert after"
                        }
                        button {
                            class: "m1-outline-remove-point",
                            r#type: "button",
                            disabled: !enabled || point_count <= 3,
                            aria_label: "Remove point {point_index + 1}",
                            title: if point_count <= 3 { "Keep at least three points." } else { "Remove the selected point" },
                            onclick: {
                                let action_context = action_context.clone();
                                let target = perimeter.target.clone();
                                let mut points = perimeter.points.clone();
                                move |_| {
                                    if point_count <= 3 { return; }
                                    points.remove(point_index);
                                    selected_point.set(point_index.saturating_sub(1));
                                        on_action.call(action_context.edit_perimeter(target.clone(), points.clone(), EditPhase::Commit, format!("outline-point-{}-{point_index}", action_context.revision)));
                                }
                            },
                            "Remove point"
                        }
                    }
                    div { class: "m1-outline-point-list", role: "group", aria_label: "Outline points",
                        div { class: "m1-outline-point-columns", aria_hidden: "true",
                            span { "Point" } span { "X · mm" } span { "Y · mm" }
                        }
                        for (index, point) in perimeter.canvas_points.iter().enumerate() {
                            button {
                                key: "outline-point-{index}",
                                r#type: "button",
                                aria_label: "Select outline point {index + 1}",
                                aria_pressed: "{index == point_index}",
                                onclick: move |_| selected_point.set(index),
                                span { "{index + 1}" }
                                span { "{point.x:.3}" }
                                span { "{point.y:.3}" }
                            }
                        }
                    }
                    if let Some(feedback) = projection.feedback.as_ref() {
                        p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                            if feedback.state == "pending" { "Saving outline…" }
                            else if feedback.state == "saved" { "Saved" }
                            else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                        }
                    }
                }
            }
        } else {
        section { class: "m1-outline-inspector", "aria-label": "Board outline",
            div { class: "m1-outline-inspector-heading", h2 { "Board outline" } span { class: "m1-outline-board-name", "{projection.board_name}" } }
            p { if projection.active_version_id.is_some() { "A fixed outline; component placement is shared with every version." } else { "Generated follows your keycaps and included components." } }
            p { "{projection.contours.len()} accepted contours" }
            if let Some((view_box, paths)) = contour_view {
                svg { class: "m1-outline-preview", role: "img", "aria-label": "Accepted active board outline", view_box: "{view_box}",
                    g { transform: "scale(1,-1)",
                        for (index, (points, hole)) in paths.iter().enumerate() {
                            polygon { key: "outline-preview-{index}", class: if *hole { "m1-outline is-hole" } else { "m1-outline" }, points: "{points}" }
                        }
                    }
                }
            }
            div { class: "m1-outline-settings",
                label { class: "m1-outline-field",
                    span { "Active outline" }
                    select {
                        aria_label: "Active outline",
                        value: "{active_value}",
                        disabled: !enabled,
                        onchange: {
                            let action_context = action_context.clone();
                            move |event: FormEvent| {
                                let value = event.value();
                                on_action.call(action_context.activate_action((!value.is_empty()).then_some(value)));
                            }
                        },
                        option { value: "", selected: active_value.is_empty(), "Generated" }
                        for item in projection.versions.iter() {
                            option { key: "{item.id}", value: "{item.id}", selected: item.id == active_value, "{item.name}" }
                        }
                    }
                }
                if let Some(version_id) = active_version.as_ref() {
                    label { class: "m1-outline-field",
                        span { "Version name" }
                        input {
                            aria_label: "Outline version name",
                            maxlength: "120",
                            value: "{name_draft}",
                            disabled: !enabled,
                            aria_invalid: name_draft.trim().is_empty(),
                            oninput: {
                                let version_id = version_id.clone();
                                let baseline = projection.version_name.clone();
                                move |event: FormEvent| version_name.set(Some((version_id.clone(), baseline.clone(), event.value())))
                            },
                            onblur: {
                                let version_id = version_id.clone();
                                let accepted_name = projection.version_name.clone();
                                let action_context = action_context.clone();
                                move |_| {
                                    let Some((draft_id, baseline, draft)) = version_name() else { return; };
                                    if draft_id != version_id || baseline != accepted_name { return; }
                                    let name = draft.trim().to_owned();
                                    if !name.is_empty() && name.len() <= 120 && name != accepted_name {
                                        on_action.call(action_context.action(OutlineEdit::RenameVersion { version_id: version_id.clone(), name }));
                                    }
                                }
                            },
                            onkeydown: {
                                move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                    "Enter" => {
                                        event.prevent_default();
                                        if let Some(input) = event.data().try_as_web_event()
                                            .and_then(|event| event.target())
                                            .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                        { let _ = input.blur(); }
                                    }
                                    "Escape" => {
                                        event.prevent_default();
                                        version_name.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if !projection.has_generated && projection.active_version_id.is_none() {
                    button {
                        class: "m1-outline-action",
                        disabled: !enabled,
                        onclick: {
                            let action_context = action_context.clone();
                            move |_| on_action.call(action_context.action(OutlineEdit::CreateAutomatic))
                        },
                        "Generate automatic outline"
                    }
                }
                if projection.has_generated || projection.active_version_id.is_some() {
                    label { class: "m1-outline-field",
                        span { "Corners" }
                        select {
                            aria_label: "Outline corners",
                            value: "{corner_value}",
                            disabled: !enabled,
                            onchange: {
                                let action_context = action_context.clone();
                                move |event: FormEvent| {
                                let corner = match event.value().as_str() {
                                    "fillet" => CornerStyle::Fillet,
                                    "chamfer" => CornerStyle::Chamfer,
                                    _ => CornerStyle::Sharp,
                                };
                                on_action.call(action_context.action(OutlineEdit::SetCorners(corner)));
                            }},
                            option { value: "sharp", "Sharp" }
                            option { value: "fillet", "Fillet" }
                            option { value: "chamfer", "Chamfer" }
                        }
                    }
                    if projection.settings.corners != CornerStyle::Sharp {
                        OutlineDimension {
                            label: size_label,
                            value: projection.settings.size,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetSize(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() {
                        if let Some(margin) = projection.generated_margin {
                            OutlineDimension {
                                label: "Outline margin",
                                value: margin,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMargin(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Bridge width",
                            value: projection.settings.bridge_width,
                            minimum: 0.001,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetBridgeWidth(value)))
                            },
                        }
                    }
                    if projection.active_version_id.is_none() && !projection.gaps.is_empty() {
                        fieldset { class: "m1-outline-controls", disabled: !enabled,
                            legend { "Gap repair" }
                            p { "Keep gap preserves an intentional recess and follows its source components." }
                            for (index, gap) in projection.gaps.iter().enumerate() {
                                div { class: "m1-outline-gap-row",
                                    button {
                                        class: "m1-outline-gap-focus",
                                        r#type: "button",
                                        aria_label: "Show gap {index + 1}",
                                        disabled: !enabled,
                                        onclick: {
                                            let action_context = action_context.clone();
                                            let gap_id = gap.id.clone();
                                            move |_| on_action.call(action_context.focus_gap(gap_id.clone()))
                                        },
                                        "Gap {index + 1} · {gap.span:.1} mm span"
                                    }
                                    label { class: "m1-outline-field m1-outline-gap",
                                        input {
                                            r#type: "checkbox",
                                            aria_label: "Keep gap {index + 1}",
                                            checked: gap.protected,
                                            onchange: {
                                                let gap_id = gap.id.clone();
                                                let action_context = action_context.clone();
                                                move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetProtectedGap { gap_id: gap_id.clone(), protected: event.checked() }))
                                            }
                                        }
                                        span { "Keep gap" }
                                    }
                                }
                            }
                        }
                    }
                    fieldset { class: "m1-outline-controls", disabled: !enabled,
                        legend { "Advanced cleanup and clearance" }
                        if projection.active_version_id.is_none() {
                            label { class: "m1-outline-field m1-outline-gap",
                                input {
                                    r#type: "checkbox",
                                    aria_label: "Automatic gap cleanup",
                                    checked: projection.repair.enabled,
                                    onchange: {
                                        let action_context = action_context.clone();
                                        move |event: FormEvent| on_action.call(action_context.action(OutlineEdit::SetRepairEnabled(event.checked())))
                                    },
                                }
                                span { "Automatic cleanup" }
                            }
                            OutlineDimension {
                                label: "Maximum gap span",
                                value: projection.repair.maximum_gap_span,
                                minimum: 0.0,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    move |value| on_action.call(action_context.action(OutlineEdit::SetMaximumGapSpan(value)))
                                },
                            }
                        }
                        OutlineDimension {
                            label: "Minimum connection width",
                            value: projection.repair.minimum_connection_width,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetMinimumConnectionWidth(value)))
                            },
                        }
                        OutlineDimension {
                            label: "PCB edge clearance",
                            value: projection.repair.edge_clearance,
                            minimum: 0.0,
                            editable: enabled,
                            on_commit: {
                                let action_context = action_context.clone();
                                move |value| on_action.call(action_context.action(OutlineEdit::SetEdgeClearance(value)))
                            },
                        }
                        p { "Support and clearance findings block affected fabrication exports. Editing and project saving stay available." }
                        if projection.active_version_id.is_none() {
                            for (index, gap) in projection.repair.keep_gaps.iter().enumerate() {
                                button {
                                    class: "m1-outline-remove-gap",
                                    aria_label: "Remove protected gap {index + 1}",
                                    onclick: {
                                        let gap_id = gap.id.clone();
                                        let action_context = action_context.clone();
                                        move |_| on_action.call(action_context.action(OutlineEdit::RemoveProtectedGap { gap_id: gap_id.clone() }))
                                    },
                                    "Remove protected gap {index + 1}"
                                }
                            }
                        }
                    }
                }
            }
            if projection.perimeter.as_ref().is_some_and(|perimeter| perimeter.points.len() >= 3) {
                button {
                    class: "m1-outline-action",
                    r#type: "button",
                    disabled: !enabled,
                    onclick: move |_| {
                        selected_point.set(0);
                        perimeter_open.set(true);
                    },
                    "Edit perimeter points"
                }
            }
            section { class: "m1-outline-manual", "aria-label": "Manual geometry",
                h3 { "Manual geometry" }
                if !projection.geometry_features.is_empty() {
                    div { class: "m1-outline-feature-list", role: "group", aria_label: "Saved outline geometry",
                        h4 { "Saved geometry" }
                        for (index, feature) in projection.geometry_features.iter().enumerate() {
                            {
                                let feature_id = feature.id().to_owned();
                                let selected = selected_feature_id().as_deref() == Some(feature_id.as_str());
                                let feature_name = match feature {
                                    OutlineFeature::Polygon { operation, .. } => match operation { Operation::Add => "Addition", Operation::Subtract => "Cutout" },
                                    OutlineFeature::Rect { operation, .. } => match operation { Operation::Add => "Rectangle addition", Operation::Subtract => "Rectangle cutout" },
                                    OutlineFeature::PartEnvelope { .. } => "Generated perimeter",
                                };
                                rsx! {
                                    div { class: "m1-outline-feature-row", key: "outline-feature-{feature_id}",
                                        button {
                                            r#type: "button",
                                            aria_pressed: "{selected}",
                                            disabled: !enabled,
                                            onclick: {
                                                let feature_id = feature_id.clone();
                                                move |_| selected_feature_id.set(Some(feature_id.clone()))
                                            },
                                            "{feature_name} {index + 1}"
                                        }
                                        if matches!(feature, OutlineFeature::Polygon { .. }) {
                                            button {
                                                r#type: "button",
                                                disabled: !enabled,
                                                onclick: {
                                                    let feature_id = feature_id.clone();
                                                    move |_| {
                                                        selected_feature_id.set(Some(feature_id.clone()));
                                                        selected_point.set(0);
                                                        perimeter_open.set(true);
                                                    }
                                                },
                                                "Edit points"
                                            }
                                        }
                                        if let Some(version_id) = active_version.as_ref().filter(|_| index > 0) {
                                            button {
                                                r#type: "button",
                                                class: "m1-outline-remove-feature",
                                                disabled: !enabled,
                                                aria_label: "Remove {feature_name} {index + 1}",
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let version_id = version_id.clone();
                                                    let feature_id = feature_id.clone();
                                                    move |_| on_action.call(action_context.remove_feature(version_id.clone(), feature_id.clone()))
                                                },
                                                "Remove"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(feature) = projection.geometry_features.iter().find(|feature| Some(feature.id()) == selected_feature_id().as_deref()) {
                    if let OutlineFeature::Rect { center, size, anchor_part_id, .. } = feature {
                        fieldset { class: "m1-outline-feature-editor", disabled: !enabled,
                            legend { "Selected rectangle" }
                            div { class: "m1-outline-coordinate-fields",
                                OutlineCoordinate {
                                    key: "rectangle-{feature.id()}-center-x",
                                    label: "Rectangle center X mm",
                                    value: anchor_part_id.as_deref()
                                        .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                        .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back }.world(*center))
                                        .unwrap_or(*center).x,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        let anchor = anchor_part_id.as_deref()
                                            .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                            .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back });
                                        let world_center = anchor.map_or(*center, |anchor| anchor.world(*center));
                                        move |value| {
                                            let local = anchor.map_or(Vec2 { x: value, y: world_center.y }, |anchor| anchor.local(Vec2 { x: value, y: world_center.y }));
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { center, .. } = &mut after { *center = local; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                                OutlineCoordinate {
                                    key: "rectangle-{feature.id()}-center-y",
                                    label: "Rectangle center Y mm",
                                    value: anchor_part_id.as_deref()
                                        .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                        .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back }.world(*center))
                                        .unwrap_or(*center).y,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        let anchor = anchor_part_id.as_deref()
                                            .and_then(|id| projection.outline_parts.iter().find(|part| part.id == id))
                                            .map(|part| PerimeterAnchor { at: part.pose.at, rotation: part.pose.rotation, back: part.side == Side::Back });
                                        let world_center = anchor.map_or(*center, |anchor| anchor.world(*center));
                                        move |value| {
                                            let local = anchor.map_or(Vec2 { x: world_center.x, y: value }, |anchor| anchor.local(Vec2 { x: world_center.x, y: value }));
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { center, .. } = &mut after { *center = local; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                            }
                            OutlineDimension {
                                label: "Rectangle width",
                                value: size.x,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = feature.clone();
                                    let version_id = active_version.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::Rect { size, .. } = &mut after { size.x = value; }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            OutlineDimension {
                                label: "Rectangle height",
                                value: size.y,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = feature.clone();
                                    let version_id = active_version.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::Rect { size, .. } = &mut after { size.y = value; }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            if let OutlineFeature::Rect { radius, .. } = feature {
                                OutlineDimension {
                                    label: "Rectangle corner radius",
                                    value: *radius,
                                    minimum: 0.0,
                                    editable: enabled,
                                    on_commit: {
                                        let action_context = action_context.clone();
                                        let before = feature.clone();
                                        let version_id = active_version.clone();
                                        move |value| {
                                            let mut after = before.clone();
                                            if let OutlineFeature::Rect { radius, .. } = &mut after { *radius = value; }
                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
                if !projection.connections.is_empty() {
                    div { class: "m1-outline-connection-list", role: "group", aria_label: "Outline connections",
                        h4 { "Connections" }
                        for (index, connection) in projection.connections.iter().enumerate() {
                            div { class: "m1-outline-feature-row", key: "outline-connection-{connection.id}",
                                button {
                                    r#type: "button",
                                    aria_pressed: "{selected_connection_id().as_deref() == Some(connection.id.as_str())}",
                                    onclick: {
                                        let id = connection.id.clone();
                                        move |_| selected_connection_id.set(Some(id.clone()))
                                    },
                                    "Connection {index + 1} · {connection.points.len()} points · {connection.width:.2} mm"
                                }
                                if let Some(before) = projection.connection_feature.as_ref() {
                                    button {
                                        r#type: "button",
                                        class: "m1-outline-remove-feature",
                                        disabled: !enabled,
                                        aria_label: "Remove connection {index + 1}",
                                        onclick: {
                                            let action_context = action_context.clone();
                                            let before = before.clone();
                                            let version_id = active_version.clone();
                                            let connection_id = connection.id.clone();
                                            move |_| {
                                                let mut after = before.clone();
                                                if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                                    connections.retain(|item| item.id != connection_id);
                                                }
                                                on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                selected_connection_id.set(None);
                                            }
                                        },
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                }
                if let (Some(connection_id), Some(before)) = (selected_connection_id().as_ref(), projection.connection_feature.as_ref()) {
                    if let OutlineFeature::PartEnvelope { connections, .. } = before {
                        if let Some(connection) = connections.iter().find(|connection| &connection.id == connection_id) {
                            OutlineDimension {
                                label: "Connection width",
                                value: connection.width,
                                minimum: 0.001,
                                editable: enabled,
                                on_commit: {
                                    let action_context = action_context.clone();
                                    let before = before.clone();
                                    let version_id = active_version.clone();
                                    let connection_id = connection_id.clone();
                                    move |value| {
                                        let mut after = before.clone();
                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after {
                                            if let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id) {
                                                connection.width = value;
                                            }
                                        }
                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                    }
                                },
                            }
                            if !connection.points.is_empty() {
                                {
                                    let index = selected_point().min(connection.points.len() - 1);
                                    let point = &connection.points[index];
                                    let world = connection_point_world(point, &projection.outline_parts);
                                    rsx! {
                                        h4 { "Connection point {index + 1} of {connection.points.len()}" }
                                        div { class: "m1-outline-coordinate-fields",
                                            OutlineCoordinate {
                                                key: "connection-{connection.id}-{index}-x",
                                                label: format!("Point {} X mm", index + 1),
                                                value: world.x,
                                                editable: enabled,
                                                on_commit: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |value| {
                                                        let next = move_connection_point(&before, &connection_id, index, Vec2 { x: value, y: world.y }, &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                            }
                                            OutlineCoordinate {
                                                key: "connection-{connection.id}-{index}-y",
                                                label: format!("Point {} Y mm", index + 1),
                                                value: world.y,
                                                editable: enabled,
                                                on_commit: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |value| {
                                                        let next = move_connection_point(&before, &connection_id, index, Vec2 { x: world.x, y: value }, &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                            }
                                        }
                                        label { class: "m1-outline-field",
                                            span { "Point attachment" }
                                            select {
                                                aria_label: "Point {index + 1} attachment",
                                                value: "{point.part_id.as_deref().unwrap_or_default()}",
                                                disabled: !enabled,
                                                onchange: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |event: FormEvent| {
                                                        let value = event.value();
                                                        let next = attach_connection_point(&before, &connection_id, index, (!value.is_empty()).then_some(value.as_str()), &parts);
                                                        on_action.call(action_context.set_feature(version_id.clone(), before.clone(), next));
                                                    }
                                                },
                                                option { value: "", "Fixed on board" }
                                                if let Some(id) = point.part_id.as_ref().filter(|id| !projection.outline_parts.iter().any(|part| &part.id == *id)) {
                                                    option { value: "{id}", "Missing component" }
                                                }
                                                for part in projection.outline_parts.iter() {
                                                    option { key: "{part.id}", value: "{part.id}", "{part.reference}" }
                                                }
                                            }
                                        }
                                        div { class: "m1-outline-point-actions",
                                            button {
                                                r#type: "button",
                                                disabled: !enabled || index + 1 == connection.points.len(),
                                                aria_label: "Insert after connection point {index + 1}",
                                                title: if index + 1 == connection.points.len() { "A connection needs its final endpoint." } else { "Insert a fixed point after this point." },
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    let parts = projection.outline_parts.clone();
                                                    move |_| {
                                                        let mut after = before.clone();
                                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                                            && index + 1 < connection.points.len()
                                                        {
                                                            let a = connection_point_world(&connection.points[index], &parts);
                                                            let b = connection_point_world(&connection.points[index + 1], &parts);
                                                            connection.points.insert(index + 1, OutlineControlPoint {
                                                                at: Vec2 { x: (a.x + b.x) * 0.5, y: (a.y + b.y) * 0.5 },
                                                                part_id: None,
                                                            });
                                                            selected_point.set(index + 1);
                                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                        }
                                                    }
                                                },
                                                "Insert after"
                                            }
                                            button {
                                                r#type: "button",
                                                disabled: !enabled || connection.points.len() <= 2,
                                                aria_label: "Remove connection point {index + 1}",
                                                title: if connection.points.len() <= 2 { "Keep at least two points." } else { "Remove the selected point" },
                                                onclick: {
                                                    let action_context = action_context.clone();
                                                    let before = before.clone();
                                                    let version_id = active_version.clone();
                                                    let connection_id = connection.id.clone();
                                                    move |_| {
                                                        let mut after = before.clone();
                                                        if let OutlineFeature::PartEnvelope { connections, .. } = &mut after
                                                            && let Some(connection) = connections.iter_mut().find(|connection| connection.id == connection_id)
                                                            && connection.points.len() > 2 && index < connection.points.len()
                                                        {
                                                            connection.points.remove(index);
                                                            selected_point.set(index.saturating_sub(1));
                                                            on_action.call(action_context.set_feature(version_id.clone(), before.clone(), after));
                                                        }
                                                    }
                                                },
                                                "Remove point"
                                            }
                                        }
                                        div { class: "m1-outline-point-list", role: "group", aria_label: "Connection points",
                                            for (index, control) in connection.points.iter().enumerate() {
                                                {
                                                    let point = connection_point_world(control, &projection.outline_parts);
                                                    rsx! {
                                                        button {
                                                            key: "connection-point-{index}",
                                                            r#type: "button",
                                                            aria_label: "Select connection point {index + 1}",
                                                            aria_pressed: "{index == selected_point()}",
                                                            onclick: move |_| selected_point.set(index),
                                                            span { "{index + 1}" }
                                                            span { "{point.x:.3}" }
                                                            span { "{point.y:.3}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                        }
                    }
                }
                }
                if let Some(tool) = drawing_operation() {
                    p { role: "status", "Click the canvas to add points. Enter finishes; Escape cancels." }
                    p { "{drawing_points.read().len()} points" }
                    div { class: "m1-outline-actions",
                        button {
                            r#type: "button",
                            disabled: drawing_points.read().is_empty(),
                            onclick: move |_| { let mut points = drawing_points.read().clone(); points.pop(); drawing_points.set(points); },
                            "Undo point"
                        }
                        button { r#type: "button", onclick: move |_| { drawing_operation.set(None); drawing_points.set(Vec::new()); }, "Cancel drawing" }
                        button {
                            r#type: "button",
                            disabled: !enabled || drawing_points.read().len() < if tool == OutlineDrawTool::Connect { 2 } else { 3 }
                                || matches!(tool, OutlineDrawTool::Polygon(_)) && polygon_area(&drawing_points.read()).abs() < 1e-6,
                            onclick: {
                                let action_context = action_context.clone();
                                let board_id = projection.board_id.clone();
                                let revision = projection.revision;
                                move |_| {
                                    let minimum_points = if tool == OutlineDrawTool::Connect { 2 } else { 3 };
                                    let points = drawing_points.read().clone();
                                    if points.len() < minimum_points { return; }
                                    match tool {
                                        OutlineDrawTool::Polygon(operation) => {
                                            if polygon_area(&points).abs() < 1e-6 { return; }
                                            let serial = draft_serial() + 1;
                                            draft_serial.set(serial);
                                            let feature = OutlineFeature::Polygon {
                                                id: format!("outline-manual-{board_id}-{}-{serial}", revision),
                                                points,
                                                anchor_part_id: None,
                                                operation,
                                            };
                                            on_action.call(action_context.add_feature(feature));
                                        }
                                        OutlineDrawTool::Connect => on_action.call(action_context.add_connection(points)),
                                    }
                                    drawing_operation.set(None);
                                    drawing_points.set(Vec::new());
                                }
                            },
                            "Finish drawing"
                        }
                    }
                } else {
                    div { class: "m1-outline-actions",
                        button {
                            r#type: "button", disabled: !enabled,
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Add))); },
                            "Draw addition"
                        }
                        button {
                            r#type: "button", disabled: !enabled,
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Polygon(Operation::Subtract))); },
                            "Draw cutout"
                        }
                        button {
                            r#type: "button",
                            disabled: !enabled || projection.active_version_id.is_some() || !projection.has_generated,
                            title: if projection.active_version_id.is_some() { "Select Generated to add a linked connection." } else if !projection.has_generated { "Generate an automatic outline first." } else { "" },
                            onclick: move |_| { perimeter_open.set(false); drawing_points.set(Vec::new()); drawing_operation.set(Some(OutlineDrawTool::Connect)); },
                            "Connect points"
                        }
                    }
                }
            }
            div { class: "m1-outline-actions",
                button { r#type: "button", disabled: !projection.enabled, onclick: move |_| copy_handler.call(copy.clone()), "Copy outline" }
                if let Some(delete) = delete {
                    button { r#type: "button", disabled: !projection.enabled, onclick: move |_| delete_handler.call(delete.clone()), "Delete outline" }
                }
            }
            if let Some(feedback) = projection.feedback.as_ref() {
                p { role: if feedback.state == "pending" || feedback.state == "saved" { "status" } else { "alert" }, "data-state": feedback.state,
                    if feedback.state == "pending" { "Saving outline…" }
                    else if feedback.state == "saved" { "Saved" }
                    else { "Outline change failed: {feedback.message.as_deref().unwrap_or_default()}" }
                }
            }
        }
        }
    }
}

#[derive(Clone)]
struct PointDrag {
    pointer_id: i32,
    point_index: usize,
    points: Vec<Vec2>,
    pending: Vec<Vec2>,
    preview_points: Vec<Vec2>,
    original_world: Vec2,
    target: OutlinePointTarget,
    anchor: Option<PerimeterAnchor>,
    action_context: Rc<OutlineActionContext>,
    transaction_id: String,
    capture: SvgElement,
    snap: Option<crate::presentation::outline_snapping::Snap>,
    preview_submitted: bool,
    moved: bool,
}

#[derive(Clone)]
struct PointSnapInputs {
    paths: Vec<Vec<Vec2>>,
    origins: Rc<Vec<crate::presentation::outline_snapping::Origin>>,
    grid: Vec2,
    geometry_snap: bool,
    svg: Rc<RefCell<Option<SvgElement>>>,
    width: f64,
}

#[derive(Clone)]
pub(super) struct OutlineRuntimeHandle(Rc<Runtime>);

impl OutlineRuntimeHandle {
    pub(super) fn new(runtime: Rc<Runtime>) -> Self {
        Self(runtime)
    }
}

impl PartialEq for OutlineRuntimeHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
pub(super) fn OutlinePointCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    arbiter: CanvasInteractionArbiter,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: super::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let drag = use_hook(|| Rc::new(RefCell::new(None::<PointDrag>)));
    let mut preview = use_signal(|| None::<Vec<Vec2>>);
    let mut guides = use_signal(|| None::<crate::presentation::outline_snapping::Snap>);
    let token = projection.token;
    let revision = projection.revision;
    let clear_board_id = projection.board_id.clone();
    let drop_drag = drag.clone();
    let drop_runtime = runtime.0.clone();
    let drop_arbiter = arbiter.clone();
    let drop_board_id = clear_board_id.clone();
    use_drop(move || {
        if let Some(active) = drop_drag.borrow_mut().take() {
            drop_arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
            if active.capture.has_pointer_capture(active.pointer_id) {
                let _ = active.capture.release_pointer_capture(active.pointer_id);
            }
            if active.preview_submitted {
                drop_runtime.submit(Event::ClearPreview {
                    operation_id: drop_runtime.operation(),
                    token,
                    revision,
                    board_id: drop_board_id,
                    transaction_id: active.transaction_id,
                });
            }
        }
    });
    let Some(perimeter) = projection
        .perimeter
        .as_ref()
        .filter(|_| (projection.editing_points)())
    else {
        return rsx! {};
    };
    if perimeter.canvas_points.len() < 3 {
        return rsx! {};
    }
    let on_action = projection.on_action;
    let action_context = projection.action_context.clone();
    let selected_point = projection.selected_point;
    let saved_points = perimeter.points.clone();
    let canvas_points = perimeter.canvas_points.clone();
    let target = perimeter.target.clone();
    let anchor = perimeter.anchor;
    let snap_paths = projection.snap_paths.clone();
    let origins = Rc::new(origins);
    let grid = outline_grid(snap_settings.snap_fraction, pitch);
    let geometry_snap = snap_settings.geometry_snap;
    let enabled = projection.enabled;
    let snap_inputs = PointSnapInputs {
        paths: snap_paths.clone(),
        origins: origins.clone(),
        grid,
        geometry_snap,
        svg: svg.clone(),
        width,
    };

    let finish_drag = {
        let drag = drag.clone();
        let runtime = runtime.0.clone();
        let arbiter = arbiter.clone();
        let snap_inputs = snap_inputs.clone();
        move |commit: bool, final_at: Option<Vec2>, free: bool| {
            let Some(mut active) = drag.borrow_mut().take() else {
                return;
            };
            if let Some(at) = final_at {
                apply_point_sample(&mut active, at, &snap_inputs, free);
            }
            preview.set(None);
            guides.set(None);
            arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
            if active.capture.has_pointer_capture(active.pointer_id) {
                let _ = active.capture.release_pointer_capture(active.pointer_id);
            }
            if commit && active.moved {
                on_action.call(active.action_context.edit_perimeter(
                    active.target,
                    active.pending,
                    EditPhase::Commit,
                    active.transaction_id,
                ));
            } else if active.preview_submitted {
                runtime.submit(Event::ClearPreview {
                    operation_id: runtime.operation(),
                    token,
                    revision,
                    board_id: clear_board_id.clone(),
                    transaction_id: active.transaction_id,
                });
            }
        }
    };

    let rendered_points = preview().unwrap_or(canvas_points.clone());
    let screen_width = svg
        .borrow()
        .as_ref()
        .map(|surface| surface.get_bounding_client_rect().width())
        .filter(|value| *value > 0.0)
        .unwrap_or(800.0);
    let handle_radius = width / screen_width * 12.0;
    let rendered_point_string = rendered_points
        .iter()
        .chain(rendered_points.first())
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        g { class: "m1-outline-point-controls", "aria-label": "Outline perimeter points",
            if let Some(snap) = guides().as_ref().filter(|snap| !snap.guides.is_empty()) {
                g { class: "m1-outline-snap-guides", "aria-label": "Outline alignment guides",
                    for (index, guide) in snap.guides.iter().enumerate() {
                        line {
                            key: "guide-{index}-{guide.id}",
                            class: "m1-outline-snap-guide",
                            x1: "{guide.from.x - guide.direction.x * width * 2.0}",
                            y1: "{guide.from.y - guide.direction.y * width * 2.0}",
                            x2: "{guide.from.x + guide.direction.x * width * 2.0}",
                            y2: "{guide.from.y + guide.direction.y * width * 2.0}",
                            "data-guide": "{guide.label}"
                        }
                        text {
                            class: "m1-outline-snap-label",
                            transform: "translate({snap.at.x - handle_radius} {snap.at.y - handle_radius * (2.0 + index as f64)}) scale(1,-1)",
                            text_anchor: "end",
                            "{guide.label}"
                        }
                    }
                }
            }
            polyline {
                class: "m1-outline-control-path",
                points: "{rendered_point_string}",
            }
            for (index, point) in rendered_points.iter().copied().enumerate() {
                {
                    let is_selected = selected_point() == index;
                    let point_drag = drag.clone();
                    let point_preview = preview;
                    let point_guides = guides;
                    let point_action = on_action;
                    let point_context = action_context.clone();
                    let point_runtime = runtime.0.clone();
                    let point_arbiter = arbiter.clone();
                    let point_target = target.clone();
                    let point_points = saved_points.clone();
                    let point_canvas_points = canvas_points.clone();
                    let point_anchor = anchor;
                    let point_snap_inputs = snap_inputs.clone();
                    let point_svg = svg.clone();
                    let mut point_selected = selected_point;
                    let point_id = format!("outline-point-{index}");
                    let point_label = format!("Outline point {}", index + 1);
                    let point_x = point.x;
                    let point_y = point.y;
                    let point_commit = finish_drag.clone();
                    let mut pointerup_commit = point_commit.clone();
                    let mut key_commit = point_commit.clone();
                    let mut cancel_commit = point_commit.clone();
                    let mut lost_capture_commit = point_commit.clone();
                    let down_drag = point_drag.clone();
                    let down_arbiter = point_arbiter.clone();
                    let mut down_selected = point_selected;
                    let down_runtime = point_runtime.clone();
                    let down_points = point_points.clone();
                    let down_canvas_points = point_canvas_points.clone();
                    let down_target = point_target.clone();
                    let down_context = point_context.clone();
                    let move_drag = point_drag.clone();
                    let move_svg = point_svg.clone();
                    let move_snap_inputs = point_snap_inputs.clone();
                    let mut move_preview = point_preview;
                    let mut move_guides = point_guides;
                    let move_action = point_action;
                    let up_drag = point_drag.clone();
                    let up_svg = point_svg.clone();
                    let key_drag = point_drag.clone();
                    let key_points = point_points.clone();
                    let key_anchor = point_anchor;
                    let key_context = point_context.clone();
                    let key_target = point_target.clone();
                    let key_runtime = point_runtime.clone();
                    let mut key_selected = point_selected;
                    let key_action = point_action;
                    rsx! {
                        circle {
                            key: "{point_id}",
                            class: if is_selected { "m1-outline-point-handle is-selected" } else { "m1-outline-point-handle" },
                            role: "button",
                            tabindex: "0",
                            "aria-label": "{point_label}",
                            "aria-pressed": "{is_selected}",
                            cx: "{point_x}", cy: "{point_y}", r: "{handle_radius}",
                            onfocus: move |_| point_selected.set(index),
                            onpointerdown: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                pointer.stop_propagation();
                                if !enabled || pointer.button() != 0 || !down_arbiter.try_acquire(CanvasInteractionOwner::OutlinePerimeter) { return; }
                                let Some(capture) = pointer.target().and_then(|target| target.dyn_into::<SvgElement>().ok()).filter(|target| target.get_attribute("class").is_some_and(|classes| classes.split_whitespace().any(|class| class == "m1-outline-point-handle"))) else { down_arbiter.release(CanvasInteractionOwner::OutlinePerimeter); return; };
                                if capture.set_pointer_capture(pointer.pointer_id()).is_err() || !capture.has_pointer_capture(pointer.pointer_id()) {
                                    down_arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
                                    return;
                                }
                                pointer.prevent_default();
                                down_selected.set(index);
                                let transaction_id = format!("outline-drag-{}", down_runtime.operation().0);
                                *down_drag.borrow_mut() = Some(PointDrag {
                                    pointer_id: pointer.pointer_id(), point_index: index,
                                    points: down_points.clone(), pending: down_points.clone(), preview_points: down_canvas_points.clone(), original_world: down_canvas_points[index],
                                    target: down_target.clone(), anchor: point_anchor, action_context: down_context.clone(), transaction_id,
                                    capture, snap: None, preview_submitted: false, moved: false,
                                });
                            },
                            onpointermove: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                let mut active = move_drag.borrow_mut();
                                let Some(active) = active.as_mut().filter(|active| active.pointer_id == pointer.pointer_id()) else { return; };
                                pointer.stop_propagation();
                                let Some(at) = super::coordinates(&move_svg, &pointer, view_x, view_y, width, height) else { return; };
                                apply_point_sample(active, at, &move_snap_inputs, pointer.alt_key());
                                let preview_world = active.preview_points.clone();
                                let pending = active.pending.clone();
                                let context = active.action_context.clone();
                                let target = active.target.clone();
                                let transaction = active.transaction_id.clone();
                                let changed = active.moved;
                                if changed {
                                    active.preview_submitted = true;
                                }
                                let snap = active.snap.clone();
                                let _ = active;
                                move_preview.set(Some(preview_world)); move_guides.set(snap);
                                if changed {
                                    move_action.call(context.edit_perimeter(target, pending, EditPhase::Preview, transaction));
                                }
                            },
                            onpointerup: move |event: PointerEvent| {
                                let Some(pointer) = event.data().try_as_web_event() else { return; };
                                if !up_drag.borrow().as_ref().is_some_and(|drag| drag.pointer_id == pointer.pointer_id()) { return; }
                                pointer.stop_propagation();
                                let at = super::coordinates(&up_svg, &pointer, view_x, view_y, width, height);
                                pointerup_commit(true, at, pointer.alt_key());
                            },
                            onpointercancel: move |_| cancel_commit(false, None, false),
                            onlostpointercapture: move |_| lost_capture_commit(false, None, false),
                            onkeydown: move |event: KeyboardEvent| {
                                let Some(key) = event.data().try_as_web_event() else { return; };
                                if key.key() == "Escape" && key_drag.borrow().is_some() {
                                    key.prevent_default(); key.stop_propagation(); key_commit(false, None, false); return;
                                }
                                if key_drag.borrow().is_some() || !enabled { return; }
                                let mut points = key_points.clone();
                                let step_x = grid.x.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let step_y = grid.y.max(0.1) * if key.shift_key() { 10.0 } else { 1.0 };
                                let world = key_anchor.map_or(Vec2 { x: point_x, y: point_y }, |a| a.world(key_points[index]));
                                let (dx, dy) = match key.key().as_str() {
                                    "ArrowLeft" => (-step_x, 0.0), "ArrowRight" => (step_x, 0.0),
                                    "ArrowUp" => (0.0, step_y), "ArrowDown" => (0.0, -step_y),
                                    _ if key.key() == "Delete" || key.key() == "Backspace" => {
                                        key.prevent_default(); key.stop_propagation();
                                        if points.len() <= 3 { return; }
                                        points.remove(index); key_selected.set(index.saturating_sub(1));
                                        key_action.call(key_context.edit_perimeter(key_target.clone(), points, EditPhase::Commit, format!("outline-delete-{}-{}", key_runtime.operation().0, index)));
                                        return;
                                    },
                                    _ => return,
                                };
                                key.prevent_default(); key.stop_propagation();
                                let next = Vec2 { x: world.x + dx, y: world.y + dy };
                                points[index] = key_anchor.map_or(next, |a| a.local(next));
                                key_action.call(key_context.edit_perimeter(key_target.clone(), points, EditPhase::Commit, format!("outline-nudge-{}-{}", key_runtime.operation().0, index)));
                            }
                        }
                        text { class: "m1-outline-point-label", x: "{point_x + 2.0}", y: "{point_y + 2.0}", "aria-hidden": "true", "{index + 1}" }
                    }
                }
            }
        }
    }
}

fn outline_grid(fraction: f64, pitch: Vec2) -> Vec2 {
    let spacing = |axis: f64| {
        if fraction > 0.0 {
            axis * fraction
        } else if fraction < 0.0 {
            -fraction
        } else {
            0.0
        }
    };
    Vec2 {
        x: spacing(pitch.x),
        y: spacing(pitch.y),
    }
}

fn polygon_area(points: &[Vec2]) -> f64 {
    points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let next = points[(index + 1) % points.len()];
            point.x * next.y - next.x * point.y
        })
        .sum::<f64>()
        * 0.5
}

#[component]
pub(super) fn OutlineDraftCanvasOverlay(
    projection: OutlineInspectorProjection,
    runtime: OutlineRuntimeHandle,
    arbiter: CanvasInteractionArbiter,
    svg: Rc<RefCell<Option<SvgElement>>>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    snap_settings: super::objects::LayoutSnapSettings,
    pitch: Vec2,
    origins: Vec<crate::presentation::outline_snapping::Origin>,
) -> Element {
    let Some(tool) = (projection.drawing_operation)() else {
        return rsx! {};
    };
    let minimum_points = if tool == OutlineDrawTool::Connect {
        2
    } else {
        3
    };
    let mut drawing_operation = projection.drawing_operation;
    let points = projection.drawing_points;
    let grid = outline_grid(snap_settings.snap_fraction, pitch);
    let paths = projection.snap_paths.clone();
    let origins = Rc::new(origins);
    let geometry_snap = snap_settings.geometry_snap;
    let draft = points.read().clone();
    let point_string = draft
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");
    let runtime = runtime.0.clone();
    let action_context = projection.action_context.clone();
    let on_action = projection.on_action;
    let enabled = projection.enabled;
    let board_id = projection.board_id.clone();
    let revision = projection.revision;
    let finish = {
        let mut points = points;
        let runtime = runtime.clone();
        move || {
            let draft = points.read().clone();
            if draft.len() < minimum_points
                || matches!(tool, OutlineDrawTool::Polygon(_)) && polygon_area(&draft).abs() < 1e-6
            {
                return;
            }
            match tool {
                OutlineDrawTool::Polygon(operation) => {
                    let feature = OutlineFeature::Polygon {
                        id: format!(
                            "outline-manual-{board_id}-{}-{}",
                            revision,
                            runtime.operation().0
                        ),
                        points: draft,
                        anchor_part_id: None,
                        operation,
                    };
                    on_action.call(action_context.add_feature(feature));
                }
                OutlineDrawTool::Connect => on_action.call(action_context.add_connection(draft)),
            }
            points.set(Vec::new());
            drawing_operation.set(None);
        }
    };
    rsx! {
        g { class: "m1-outline-draft-controls", "aria-label": match tool { OutlineDrawTool::Polygon(Operation::Add) => "Draw addition", OutlineDrawTool::Polygon(Operation::Subtract) => "Draw cutout", OutlineDrawTool::Connect => "Connect points" },
            rect {
                transform: "scale(1,-1)",
                x: "{view_x}", y: "{view_y}", width: "{width}", height: "{height}",
                fill: "transparent", tabindex: "0", role: "application",
                "aria-label": "Outline drawing canvas. Click to add points. Enter finishes; Escape cancels.",
                onpointerdown: move |event: PointerEvent| {
                    let mut points = points;
                    let Some(pointer) = event.data().try_as_web_event() else { return; };
                    pointer.stop_propagation();
                    if !enabled || pointer.button() != 0 || !arbiter.try_acquire(CanvasInteractionOwner::OutlinePerimeter) { return; }
                    let Some(at) = super::coordinates(&svg, &pointer, view_x, view_y, width, height) else { arbiter.release(CanvasInteractionOwner::OutlinePerimeter); return; };
                    let path_points = points.read().clone();
                    let context = crate::presentation::outline_snapping::Context { anchor: path_points.last().copied(), previous: path_points.iter().rev().nth(1).copied(), exclude: None, neighbor: None };
                    let px = svg.borrow().as_ref().map(|surface| surface.get_bounding_client_rect().width()).unwrap_or(1.0).max(1.0);
                    let tolerance = width / px * 7.0;
                    let mut snapped = crate::presentation::outline_snapping::snap_outline_point(at, context, &paths, crate::presentation::outline_snapping::Options { grid, tolerance, enabled: geometry_snap, free: pointer.alt_key() }, None);
                    if !pointer.alt_key() && geometry_snap && snapped.guides.is_empty()
                        && let Some((at, _)) = crate::presentation::outline_snapping::snap_origin(at, &origins, tolerance)
                    {
                        snapped.at = at;
                    }
                    if !path_points.iter().any(|point| (point.x-snapped.at.x).abs() < 1e-7 && (point.y-snapped.at.y).abs() < 1e-7) { points.set(path_points.into_iter().chain([snapped.at]).collect()); }
                    arbiter.release(CanvasInteractionOwner::OutlinePerimeter);
                    pointer.prevent_default();
                },
                onkeydown: {
                    let mut finish = finish.clone();
                    let mut points = points;
                    let mut drawing_operation = drawing_operation;
                    move |event: KeyboardEvent| {
                        let Some(key) = event.data().try_as_web_event() else { return; };
                        match key.key().as_str() {
                            "Enter" => { key.prevent_default(); key.stop_propagation(); finish(); },
                            "Escape" => { key.prevent_default(); key.stop_propagation(); points.set(Vec::new()); drawing_operation.set(None); },
                            "Backspace" | "Delete" => { key.prevent_default(); key.stop_propagation(); let mut next = points.read().clone(); next.pop(); points.set(next); },
                            _ => {}
                        }
                    }
                },
            }
            if !draft.is_empty() {
                polyline { class: "m1-outline-draft-path", points: "{point_string}" }
                for (index, point) in draft.iter().enumerate() {
                    circle { key: "outline-draft-point-{index}", class: "m1-outline-draft-point", cx: "{point.x}", cy: "{point.y}", r: "1.5" }
                }
            }
        }
    }
}

fn apply_point_sample(drag: &mut PointDrag, world: Vec2, inputs: &PointSnapInputs, free: bool) {
    let count = drag.points.len();
    let index = drag.point_index;
    let context = crate::presentation::outline_snapping::Context {
        anchor: Some(drag.preview_points[(index + count - 1) % count]),
        previous: Some(drag.original_world),
        exclude: Some(drag.original_world),
        neighbor: Some(drag.preview_points[(index + 1) % count]),
    };
    let pixel_width = inputs
        .svg
        .borrow()
        .as_ref()
        .map(|surface| surface.get_bounding_client_rect().width())
        .unwrap_or(1.0)
        .max(1.0);
    let tolerance = inputs.width / pixel_width * 7.0;
    let mut result = crate::presentation::outline_snapping::snap_outline_point(
        world,
        context,
        &inputs.paths,
        crate::presentation::outline_snapping::Options {
            grid: inputs.grid,
            tolerance,
            enabled: inputs.geometry_snap,
            free,
        },
        drag.snap.as_ref(),
    );
    if !free
        && inputs.geometry_snap
        && result.guides.is_empty()
        && let Some((at, _)) =
            crate::presentation::outline_snapping::snap_origin(world, &inputs.origins, tolerance)
    {
        result.at = at;
    }
    let mut preview_points = drag.preview_points.clone();
    preview_points[index] = result.at;
    let mut pending = drag.points.clone();
    pending[index] = drag
        .anchor
        .map_or(result.at, |anchor| anchor.local(result.at));
    drag.moved = pending != drag.points;
    drag.pending = pending;
    drag.preview_points = preview_points;
    drag.snap = Some(result);
}

#[component]
fn OutlineDimension(
    label: &'static str,
    value: f64,
    minimum: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    use_effect(use_reactive!(|value| {
        if field.peek().0 != value {
            field.set((value, value.to_string()));
        }
    }));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let parsed = draft.trim().parse::<f64>();
    let valid = !draft.trim().is_empty()
        && parsed.is_ok_and(|parsed| parsed.is_finite() && parsed >= minimum);
    let error = !valid;
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    min: "{minimum}",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: label,
                    aria_invalid: error,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next >= minimum && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            let has_draft = field().1 != value.to_string();
                            field.set((value, value.to_string()));
                            if has_draft { event.stop_propagation(); }
                        }
                        _ => {}
                    }
                }
                small { "mm" }
            }
            if error { small { role: "alert", "Enter a finite value greater than or equal to {minimum} mm." } }
        }
    }
}

#[component]
fn OutlineCoordinate(
    label: String,
    value: f64,
    editable: bool,
    on_commit: EventHandler<f64>,
) -> Element {
    let mut field = use_signal(|| (value, value.to_string()));
    use_effect(use_reactive!(|value| {
        if field.peek().0 != value {
            field.set((value, value.to_string()));
        }
    }));
    let draft = if field().0 == value {
        field().1
    } else {
        value.to_string()
    };
    let valid = !draft.trim().is_empty()
        && draft
            .trim()
            .parse::<f64>()
            .is_ok_and(|parsed| parsed.is_finite());
    rsx! {
        label { class: "m1-outline-field",
            span { "{label}" }
            span { class: "m1-outline-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    value: "{draft}",
                    disabled: !editable,
                    aria_label: "{label}",
                    aria_invalid: !valid,
                    oninput: move |event: FormEvent| field.set((value, event.value())),
                    onblur: move |_| {
                        if field().0 != value {
                            field.set((value, value.to_string()));
                        } else if let Ok(next) = field().1.trim().parse::<f64>()
                            && next.is_finite() && next != value
                        { on_commit.call(next); }
                    },
                    onkeydown: move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                        "Enter" => {
                            event.prevent_default();
                            if let Some(input) = event.data().try_as_web_event()
                                .and_then(|event| event.target())
                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                            { let _ = input.blur(); }
                        }
                        "Escape" => {
                            event.prevent_default();
                            let has_draft = field().1 != value.to_string();
                            field.set((value, value.to_string()));
                            if has_draft { event.stop_propagation(); }
                        }
                        _ => {}
                    }
                }
                span { "mm" }
            }
        }
    }
}

fn contour_preview(contours: &[Contour]) -> Option<(String, Vec<(String, bool)>)> {
    let mut points = contours.iter().flat_map(|contour| &contour.points);
    let first = points.next()?;
    let (min_x, max_x, min_y, max_y) = points.fold(
        (first.x, first.x, first.y, first.y),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point.x),
                max_x.max(point.x),
                min_y.min(point.y),
                max_y.max(point.y),
            )
        },
    );
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let padding = width.max(height) * 0.08;
    let view_box = format!(
        "{} {} {} {}",
        min_x - padding,
        -(max_y + padding),
        width + padding * 2.0,
        height + padding * 2.0,
    );
    let paths = contours
        .iter()
        .map(|contour| {
            let points = contour
                .points
                .iter()
                .map(|point| format!("{},{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" ");
            (points, contour.hole)
        })
        .collect();
    Some((view_box, paths))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "outline_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "outline_lifecycle_browser_tests.rs"]
mod browser_tests;
