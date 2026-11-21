//! Matrix transform gestures: drag state, nudge and drag resolvers, and preview sampling.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TransformGesture {
    /// Edits the column's stagger. `quick_offset` is latched from `ctrlKey` at pointer-down
    /// and turns the drag into a free 2-D column-offset gesture for its whole length.
    Stagger { column: u32, quick_offset: bool },
    /// Edits the column's inherited spread step.
    Spread { column: u32 },
    /// Edits one row's or one column's independent offset (`RowOffset` or `ColumnOffset`).
    Offset { parameter: PlacementParameter },
    /// Turns one row's or one key's running frame, in degrees.
    Bend { parameter: PlacementParameter },
    Splay {
        column: u32,
        start_angle: f64,
        origin: Vec2,
        /// Latched from `ctrlKey` at pointer-down: the drag swings the origin
        /// through `OriginSwing` instead of turning the angle.
        swing: bool,
    },
    Origin {
        column: u32,
        /// Latched from `ctrlKey` at pointer-down: the drag writes the origin
        /// without hold-still compensation.
        swing: bool,
    },
    /// Moves the selected matrix through its Origin. The anchor context is
    /// captured at pointer-down from the accepted document; `anchor` is
    /// `None` exactly when the matrix is unanchored, and an anchored matrix
    /// whose anchor target is not laid out refuses the drag at start.
    MatrixMove { anchor: Option<MatrixDragAnchor> },
    /// Turns the selected matrix through its stored rotation. The anchor
    /// context is captured at pointer-down like the move; the center, start
    /// angle and start rotation follow from the accepted matrix then.
    MatrixRotate {
        anchor: Option<MatrixDragAnchor>,
        center: Vec2,
        start_angle: f64,
        start_rotation: f64,
    },
}

impl TransformGesture {
    /// The placement parameter a Stagger, Spread, Offset or Bend gesture writes.
    pub(super) fn parameter(&self) -> Option<PlacementParameter> {
        match *self {
            Self::Stagger {
                column,
                quick_offset,
            } => Some(stagger_parameter(column, quick_offset)),
            Self::Spread { column } => Some(PlacementParameter::ColumnSpread { column }),
            Self::Offset { parameter } => Some(parameter),
            Self::Bend { parameter } => Some(parameter),
            Self::Splay { .. }
            | Self::Origin { .. }
            | Self::MatrixMove { .. }
            | Self::MatrixRotate { .. } => None,
        }
    }
}

#[derive(Clone)]
pub(super) struct TransformDrag {
    pub(super) pointer_id: i32,
    pub(super) capture: SvgElement,
    pub(super) matrix: Rc<Matrix>,
    pub(super) gesture: TransformGesture,
    pub(super) transaction_id: String,
    pub(super) owner: LayoutOwnerIdentity,
    pub(super) context: TreeContext,
    pub(super) tool: LayoutTransformTool,
    /// The scene frame at pointer-down; the world delta is projected onto it.
    pub(super) basis: Option<SceneBasis>,
    /// The dragged column's world base and world origin at pointer-down: the
    /// snap target and the no-op point for an origin swing.
    pub(super) origin_base: Option<Vec2>,
    pub(super) start_origin: Option<Vec2>,
    pub(super) start_client_x: f64,
    pub(super) start_client_y: f64,
    pub(super) pending: Option<EditOperation>,
    pub(super) preview_submitted: bool,
}

impl RailDrag for TransformDrag {
    fn owner(&self) -> &LayoutOwnerIdentity {
        &self.owner
    }
    fn context(&self) -> &TreeContext {
        &self.context
    }
    fn tool(&self) -> LayoutTransformTool {
        self.tool
    }
}

/// What a handle key press moves, captured as intent. The step is a delta: the resolver
/// applies it to the matrix accepted when the edit runs.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum TransformNudge {
    Stagger {
        column: u32,
        key: ArrowKey,
        big: bool,
    },
    Spread {
        column: u32,
        key: ArrowKey,
        big: bool,
    },
    /// One arrow step of a row or column offset, in screen directions.
    Offset {
        parameter: PlacementParameter,
        key: ArrowKey,
        big: bool,
    },
    /// One arrow step of a row or key bend, in degrees. Only the horizontal
    /// pair is consumed, mirroring the Splay angle nudge.
    Bend {
        parameter: PlacementParameter,
        degrees: f64,
    },
    SplayOrigin {
        column: u32,
    },
    SplayAngle {
        column: u32,
        degrees: f64,
        affect: MatrixSplayAffect,
    },
    /// One arrow step of the selected matrix's Origin; the step arrives as
    /// the nudge delta in world directions.
    MatrixMove,
    /// One arrow step of the selected matrix's stored rotation.
    MatrixRotate {
        degrees: f64,
    },
}

pub(super) fn accepted_matrix<'a>(
    accepted: &'a AcceptedSnapshot,
    matrix_id: &str,
) -> Option<&'a Matrix> {
    accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
}

/// The anchor context a matrix drag inverse-solves through, or `None` for an
/// unanchored matrix. Refuses when the matrix is anchored but its anchor
/// target is not laid out, so the drag never solves through a stale frame.
pub(super) fn matrix_drag_anchor(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
) -> Result<Option<MatrixDragAnchor>, String> {
    let Some(anchor) = matrix.anchor.as_ref() else {
        return Ok(None);
    };
    let Some(key_pose) = matrix_drag_anchor_pose(&accepted.scene.matrix_scenes, anchor) else {
        return Err(format!(
            "Matrix {} is anchored to {}.r{}c{}, which is not laid out.",
            matrix.id, anchor.matrix_id, anchor.row, anchor.column
        ));
    };
    Ok(Some(MatrixDragAnchor {
        key_pose,
        follow: anchor.follow,
        transport: matrix_drag_transport(&accepted.document, &matrix.id),
    }))
}

pub(super) fn accepted_basis<'a>(
    accepted: &'a AcceptedSnapshot,
    matrix_id: &str,
    column: u32,
) -> Option<&'a boardstudio_model::model::MatrixColumnBasis> {
    accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?
        .columns
        .iter()
        .find(|basis| basis.column == column)
}

pub(super) fn transform_commit(
    transaction_id: String,
    matrix_id: &str,
    operation: EditOperation,
) -> Resolution {
    Resolution::submit_with_transaction_id(vec![matrix_id.to_owned()], transaction_id, operation)
}

/// The accepted frame of a placement parameter, or why the edit has to retire: the column
/// (or, for a row offset, the row) has gone from the accepted matrix.
pub(super) fn accepted_placement_basis(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
    parameter: PlacementParameter,
) -> Result<SceneBasis, &'static str> {
    if let PlacementParameter::RowOffset { row }
    | PlacementParameter::RowShift { row }
    | PlacementParameter::RowBend { row }
    | PlacementParameter::RowPadding { row } = parameter
        && row >= matrix.rows
    {
        return Err("The selected row no longer exists.");
    }
    if let PlacementParameter::Stagger { column }
    | PlacementParameter::ColumnOffset { column }
    | PlacementParameter::ColumnSpread { column }
    | PlacementParameter::KeyShift { column, .. }
    | PlacementParameter::KeyBend { column, .. } = parameter
        && column >= matrix.columns
    {
        return Err("The selected column no longer exists.");
    }
    if let PlacementParameter::KeyShift { row, .. } | PlacementParameter::KeyBend { row, .. } =
        parameter
        && row >= matrix.rows
    {
        return Err("The selected key no longer exists.");
    }
    accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix.id)
        .and_then(|scene| SceneBasis::for_parameter(scene, parameter))
        .ok_or("The selected column no longer exists.")
}

/// Why an arrow step makes no edit.
pub(super) enum NudgeStop {
    /// The column or row has gone from the accepted matrix.
    Gone(&'static str),
    /// The key is across the stagger axis and is not consumed.
    NotConsumed,
}

impl NudgeStop {
    pub(super) fn resolution(self) -> Resolution {
        match self {
            Self::Gone(reason) => Resolution::Retire(reason.into()),
            Self::NotConsumed => Resolution::Unchanged,
        }
    }
}

/// One arrow step against the accepted matrix.
pub(super) fn nudge_placement(
    accepted: &AcceptedSnapshot,
    matrix: &Matrix,
    parameter: PlacementParameter,
    key: ArrowKey,
    big: bool,
) -> Result<EditOperation, NudgeStop> {
    let basis = accepted_placement_basis(accepted, matrix, parameter).map_err(NudgeStop::Gone)?;
    let value = nudge_value(
        &basis,
        parameter,
        placement_current(matrix, parameter),
        key,
        big,
    )
    .ok_or(NudgeStop::NotConsumed)?;
    Ok(placement_operation(&matrix.id, parameter, value))
}

/// Resolve a handle key press: the step is added to the accepted stagger along the accepted
/// stagger axis, or to the accepted column basis (splay origin and angle). `delta` serves splay.
pub(super) fn transform_nudge_resolver(
    matrix_id: String,
    nudge: TransformNudge,
    delta: Vec2,
) -> EditResolver {
    EditResolver::new(
        "layout-transform-nudge",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted_matrix(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let operation = match &nudge {
                TransformNudge::Stagger { column, key, big } => {
                    let parameter = PlacementParameter::Stagger { column: *column };
                    // The step follows the accepted stagger axis, so a queued press lands on
                    // whatever the earlier presses produced.
                    match nudge_placement(accepted, matrix, parameter, *key, *big) {
                        Ok(operation) => operation,
                        Err(stop) => return stop.resolution(),
                    }
                }
                TransformNudge::Spread { column, key, big } => {
                    let parameter = PlacementParameter::ColumnSpread { column: *column };
                    match nudge_placement(accepted, matrix, parameter, *key, *big) {
                        Ok(operation) => operation,
                        Err(stop) => return stop.resolution(),
                    }
                }
                TransformNudge::Offset {
                    parameter,
                    key,
                    big,
                } => match nudge_placement(accepted, matrix, *parameter, *key, *big) {
                    Ok(operation) => operation,
                    Err(stop) => return stop.resolution(),
                },
                TransformNudge::SplayOrigin { column } => {
                    let Some(basis) = accepted_basis(accepted, &matrix_id, *column) else {
                        return Resolution::Retire("The selected column no longer exists.".into());
                    };
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Origin {
                            world: Some(Vec2 {
                                x: basis.splay_origin.x + delta.x,
                                y: basis.splay_origin.y + delta.y,
                            }),
                        },
                    }
                }
                TransformNudge::Bend { parameter, degrees } => {
                    if let Err(reason) = accepted_placement_basis(accepted, matrix, *parameter) {
                        return Resolution::Retire(reason.into());
                    }
                    let PlacementValue::Bend(current) = placement_current(matrix, *parameter)
                    else {
                        return Resolution::Retire(
                            "This transform no longer matches the selected matrix.".into(),
                        );
                    };
                    placement_operation(
                        &matrix_id,
                        *parameter,
                        PlacementValue::Bend(current + degrees),
                    )
                }
                TransformNudge::SplayAngle {
                    column,
                    degrees,
                    affect,
                } => {
                    let Some(basis) = accepted_basis(accepted, &matrix_id, *column) else {
                        return Resolution::Retire("The selected column no longer exists.".into());
                    };
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Angle {
                            angle: column_angle(basis.splay_angle, basis.rotation, affect)
                                + degrees,
                            affect: affect.clone(),
                        },
                    }
                }
                TransformNudge::MatrixMove => {
                    let anchor = match matrix_drag_anchor(accepted, matrix) {
                        Ok(anchor) => anchor,
                        Err(reason) => return Resolution::Retire(reason),
                    };
                    let mut next = matrix.clone();
                    next.origin = solve_matrix_origin(matrix, anchor, delta);
                    if next.origin == matrix.origin {
                        return Resolution::Retire(
                            "The nudge stays outside the followed directions.".into(),
                        );
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
                TransformNudge::MatrixRotate { degrees } => {
                    let follow_rotation = matrix
                        .anchor
                        .as_ref()
                        .map(|anchor| anchor.follow.rotation)
                        .unwrap_or(true);
                    let Some(rotation) = solve_matrix_rotation(
                        matrix.rotation.unwrap_or(0.0),
                        follow_rotation,
                        *degrees,
                    ) else {
                        return Resolution::Retire(
                            "Rotation is not followed, so turning it changes nothing.".into(),
                        );
                    };
                    let mut next = matrix.clone();
                    next.rotation = Some(rotation);
                    if next.rotation == matrix.rotation {
                        return Resolution::Retire("The nudge turns nothing.".into());
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
            };
            transform_commit(String::new(), &matrix_id, operation)
        },
    )
}

/// The net transform a drag produced, captured when it ends: a stagger or offset and the
/// splay angle as deltas from the drag's start, a splay origin as the point it was dropped on.
/// A matrix move is the constrained world delta from the drag's start, recovered through the
/// inverse of the drag's anchor frame; a matrix rotation is the swept degrees.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum TransformDragResult {
    Placement {
        parameter: PlacementParameter,
        delta: PlacementDelta,
    },
    SplayAngle {
        column: u32,
        delta_degrees: f64,
        affect: MatrixSplayAffect,
    },
    SplayOrigin {
        column: u32,
        world: Option<Vec2>,
    },
    SplaySwing {
        column: u32,
        world: Vec2,
    },
    MatrixMove {
        delta: Vec2,
    },
    MatrixRotate {
        delta_degrees: f64,
    },
}

pub(super) fn transform_drag_result(
    drag: &TransformDrag,
    operation: &EditOperation,
) -> Option<TransformDragResult> {
    if let Some(gesture_parameter) = drag.gesture.parameter() {
        let (parameter, end) = placement_target(operation, &drag.matrix)?;
        if parameter != gesture_parameter {
            return None;
        }
        let delta = placement_delta(placement_current(&drag.matrix, parameter), end)?;
        return Some(TransformDragResult::Placement { parameter, delta });
    }
    match (&drag.gesture, operation) {
        (
            TransformGesture::Splay {
                column,
                swing: true,
                ..
            },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
                ..
            },
        ) => Some(TransformDragResult::SplaySwing {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Angle { angle, affect },
                ..
            },
        ) => Some(TransformDragResult::SplayAngle {
            column: *column,
            delta_degrees: angle - stored_column_angle(&drag.matrix, *column, affect),
            affect: affect.clone(),
        }),
        (
            TransformGesture::Origin {
                column,
                swing: true,
            },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
                ..
            },
        ) => Some(TransformDragResult::SplaySwing {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::Origin { column, .. },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Origin { world },
                ..
            },
        ) => Some(TransformDragResult::SplayOrigin {
            column: *column,
            world: *world,
        }),
        (
            TransformGesture::MatrixMove { anchor },
            EditOperation::SetMatrix { matrix: next, .. },
        ) => {
            // Recover the constrained world delta the preview applied: the
            // commit re-solves it against whatever is accepted when the edit
            // runs, so queued edits compose instead of restoring this pose.
            let origin_delta = Vec2 {
                x: next.origin.x - drag.matrix.origin.x,
                y: next.origin.y - drag.matrix.origin.y,
            };
            let (sin, cos) = anchor
                .map(|anchor| anchor.key_pose.rotation.to_radians().sin_cos())
                .unwrap_or((0.0, 1.0));
            Some(TransformDragResult::MatrixMove {
                delta: Vec2 {
                    x: origin_delta.x * cos - origin_delta.y * sin,
                    y: origin_delta.x * sin + origin_delta.y * cos,
                },
            })
        }
        (TransformGesture::MatrixRotate { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            Some(TransformDragResult::MatrixRotate {
                delta_degrees: next.rotation.unwrap_or(0.0) - drag.matrix.rotation.unwrap_or(0.0),
            })
        }
        _ => None,
    }
}

/// Resolve a finished drag against the accepted matrix: the drag's net transform is applied
/// to whatever is accepted when the edit runs, so a drag queued behind other edits composes.
pub(super) fn transform_drag_resolver(
    matrix_id: String,
    transaction_id: String,
    result: TransformDragResult,
) -> EditResolver {
    EditResolver::new(
        "layout-transform-drag",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted_matrix(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let operation = match &result {
                TransformDragResult::Placement { parameter, delta } => {
                    // A column or row that has gone from the accepted matrix retires the
                    // drag; Core is never asked to edit it.
                    if let Err(reason) = accepted_placement_basis(accepted, matrix, *parameter) {
                        return Resolution::Retire(reason.into());
                    }
                    let Some(value) =
                        apply_placement_delta(placement_current(matrix, *parameter), *delta)
                    else {
                        return Resolution::Retire(
                            "This transform no longer matches the selected matrix.".into(),
                        );
                    };
                    placement_operation(&matrix_id, *parameter, value)
                }
                TransformDragResult::SplayAngle {
                    column,
                    delta_degrees,
                    affect,
                } => EditOperation::SetMatrixSplay {
                    matrix_id: matrix_id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Angle {
                        angle: stored_column_angle(matrix, *column, affect) + delta_degrees,
                        affect: affect.clone(),
                    },
                },
                TransformDragResult::SplayOrigin { column, world } => {
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::Origin { world: *world },
                    }
                }
                TransformDragResult::SplaySwing { column, world } => {
                    EditOperation::SetMatrixSplay {
                        matrix_id: matrix_id.clone(),
                        column: *column,
                        change: MatrixSplayChange::OriginSwing {
                            world: Some(*world),
                        },
                    }
                }
                TransformDragResult::MatrixMove { delta } => {
                    // Re-solve the drag's constrained world delta against the
                    // live matrix: an anchor added, retargeted or detached
                    // while the drag was in flight resolves through the live
                    // frame, so the gesture composes instead of restoring.
                    let anchor = match matrix_drag_anchor(accepted, matrix) {
                        Ok(anchor) => anchor,
                        Err(reason) => return Resolution::Retire(reason),
                    };
                    let mut next = matrix.clone();
                    next.origin = solve_matrix_origin(matrix, anchor, *delta);
                    if next.origin == matrix.origin {
                        return Resolution::Retire(
                            "The drag stays outside the followed directions.".into(),
                        );
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
                TransformDragResult::MatrixRotate { delta_degrees } => {
                    let follow_rotation = matrix
                        .anchor
                        .as_ref()
                        .map(|anchor| anchor.follow.rotation)
                        .unwrap_or(true);
                    let Some(rotation) = solve_matrix_rotation(
                        matrix.rotation.unwrap_or(0.0),
                        follow_rotation,
                        *delta_degrees,
                    ) else {
                        return Resolution::Retire(
                            "Rotation is not followed, so turning it changes nothing.".into(),
                        );
                    };
                    let mut next = matrix.clone();
                    next.rotation = Some(rotation);
                    if next.rotation == matrix.rotation {
                        return Resolution::Retire("The drag turns nothing.".into());
                    }
                    EditOperation::SetMatrix {
                        matrix: next,
                        definitions: None,
                    }
                }
            };
            transform_commit(transaction_id.clone(), &matrix_id, operation)
        },
    )
}

/// A handle key press is a delta intent, so it stays admitted while earlier presses are
/// still applying: only the selection, scope and workspace must be the live owner.
pub(super) fn transform_nudge_admitted(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    let model = runtime.model();
    // Shares the settlement liveness (scope + workspace + generation +
    // selection) so the two cannot drift; nudges additionally need a live
    // Session phase, no preview/gesture and the active board/instance.
    transform_commit_is_live(
        runtime,
        workspace,
        generation,
        selected_context,
        owner,
        context,
    ) && matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.display_preview.is_none()
        && model.gesture.is_none()
        && owner.scope.as_ref().is_some_and(|scope| {
            model.active_board_id == scope.board_id && model.active_instance_id == scope.instance_id
        })
}

pub(super) fn context_matrix_id(context: &TreeContext) -> Option<&str> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id),
        TreeContext::Component { matrix_id, .. } => matrix_id.as_deref(),
        _ => None,
    }
}

pub(super) fn context_column(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Column { column, .. } | TreeContext::Key { column, .. } => Some(*column),
        TreeContext::Component { column, .. } => *column,
        _ => None,
    }
}

pub(super) fn context_row(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Row { row, .. } => Some(*row),
        _ => None,
    }
}

/// The selected key's (row, column), when the selection is a key.
pub(super) fn context_key(context: &TreeContext) -> Option<(u32, u32)> {
    match context {
        TreeContext::Key { row, column, .. } => Some((*row, *column)),
        _ => None,
    }
}

/// The carried-shift parameter for a row or key selection, or `None` when the
/// context has no row or key (a hidden Shift token on a column or matrix
/// selection draws nothing rather than editing the wrong field).
pub(super) fn shift_parameter(context: &TreeContext) -> Option<PlacementParameter> {
    match context {
        TreeContext::Row { row, .. } => Some(PlacementParameter::RowShift { row: *row }),
        TreeContext::Key { row, column, .. } => Some(PlacementParameter::KeyShift {
            row: *row,
            column: *column,
        }),
        _ => None,
    }
}

/// The carried-bend parameter for a row or key selection, or `None` when the
/// context has no row or key.
pub(super) fn bend_parameter(context: &TreeContext) -> Option<PlacementParameter> {
    match context {
        TreeContext::Row { row, .. } => Some(PlacementParameter::RowBend { row: *row }),
        TreeContext::Key { row, column, .. } => Some(PlacementParameter::KeyBend {
            row: *row,
            column: *column,
        }),
        _ => None,
    }
}

pub(super) fn transform_owner_is_current(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    let model = runtime.model();
    let current = LayoutOwnerIdentity {
        scope: runtime.scope(),
        token: model.accepted.as_ref().map(|snapshot| snapshot.token),
        revision: model
            .accepted
            .as_ref()
            .map(|snapshot| snapshot.document.revision),
        generation: generation(),
        workspace: workspace(),
    };
    current == *owner
        && owner.workspace == "Layout"
        && owner.scope.as_ref().is_some_and(|scope| {
            model.active_board_id == scope.board_id
                && model.active_instance_id == scope.instance_id
                && model.accepted.as_ref().is_some_and(|snapshot| {
                    snapshot.document.id == scope.document_id
                        && snapshot.session_epoch == scope.session_epoch
                })
                && selected_context().is_some_and(|selected| {
                    selected.scope == *scope && selected.context == *context
                })
        })
}

pub(super) fn transform_start_is_editable(
    runtime: &Runtime,
    owner: &LayoutOwnerIdentity,
    matrix: &Matrix,
) -> bool {
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && owner
            .revision
            .is_some_and(|revision| model.durability == (Durability::Saved { revision }))
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && model.accepted.as_ref().is_some_and(|snapshot| {
            snapshot
                .document
                .matrices
                .iter()
                .any(|accepted| accepted == matrix)
        })
}

/// The grid step: `snap_fraction` of the pitch (a negative fraction is an absolute step in
/// millimetres). `None` when snapping is off.
pub(super) fn grid_step(pitch: Vec2, fraction: f64) -> Option<Vec2> {
    if fraction == 0.0 {
        return None;
    }
    let step_x = if fraction < 0.0 {
        -fraction
    } else {
        pitch.x * fraction
    }
    .max(0.001);
    let step_y = if fraction < 0.0 {
        -fraction
    } else {
        pitch.y * fraction
    }
    .max(0.001);
    Some(Vec2 {
        x: step_x,
        y: step_y,
    })
}

pub(super) fn snap_delta(delta: Vec2, pitch: Vec2, fraction: f64) -> Vec2 {
    let Some(step) = grid_step(pitch, fraction) else {
        return delta;
    };
    Vec2 {
        x: (delta.x / step.x).round() * step.x,
        y: (delta.y / step.y).round() * step.y,
    }
}

pub(super) fn snap_tolerance(svg: &Rc<RefCell<Option<SvgElement>>>, view_width: f64) -> f64 {
    svg.borrow().as_ref().map_or(1.0, |surface| {
        let width = surface.get_bounding_client_rect().width();
        if width > 0.0 {
            view_width / width * 8.0
        } else {
            1.0
        }
    })
}

/// The snap inputs for an origin drop: part geometry, the dragged column's
/// world base and the grid fraction. The base is checked first: Ctrl-dragging
/// the origin onto it is the ergogen-style reset.
pub(super) struct OriginSnapTarget<'a> {
    pub(super) settings: &'a super::super::layout_toolbar::LayoutSnapSettings,
    pub(super) origins: &'a [crate::outline_snapping::Origin],
    pub(super) svg: &'a Rc<RefCell<Option<SvgElement>>>,
    pub(super) view_width: f64,
    pub(super) column_base: Option<Vec2>,
}

pub(super) fn snap_world_origin(
    point: Vec2,
    matrix: &Matrix,
    pointer: &web_sys::PointerEvent,
    target: &OriginSnapTarget<'_>,
) -> (Vec2, Option<Vec2>) {
    if pointer.alt_key() {
        return (point, None);
    }
    if let Some(base) = target.column_base
        && (point.x - base.x).hypot(point.y - base.y)
            <= snap_tolerance(target.svg, target.view_width)
    {
        return (base, Some(base));
    }
    if target.settings.geometry_snap {
        let tolerance = snap_tolerance(target.svg, target.view_width);
        if let Some((at, _)) =
            crate::outline_snapping::snap_origin(point, target.origins, tolerance)
        {
            return (at, Some(at));
        }
    }
    (
        snap_delta(point, matrix.pitch, target.settings.snap_fraction),
        None,
    )
}

pub(super) struct TransformSampleContext<'a> {
    pub(super) view_width: f64,
    pub(super) view_height: f64,
    pub(super) settings: &'a super::super::layout_toolbar::LayoutSnapSettings,
    pub(super) origins: &'a [crate::outline_snapping::Origin],
    pub(super) affect: MatrixSplayAffect,
    pub(super) svg: &'a Rc<RefCell<Option<SvgElement>>>,
    pub(super) guide: &'a mut Signal<Option<Vec2>>,
}

/// The preview of a Stagger, Offset, Shift or Bend gesture: an absolute placement value
/// for the gesture's parameter, built by the placement module from the world delta since
/// pointer-down. A Bend ignores the grid step's magnitude (it snaps to half degrees) and
/// only reads whether snapping is on, so Alt still keeps the raw angle.
///
/// A stagger snaps to the grid step of the stagger axis' dominant component: pitch.y for an
/// upright axis, pitch.x when a splay has turned the axis nearer to horizontal. The gesture
/// value reads `y` of the step it is given, so the dominant step is handed over there.
pub(super) fn placement_preview(
    drag: &TransformDrag,
    world_delta: Vec2,
    snap: Option<Vec2>,
) -> Option<EditOperation> {
    let parameter = drag.gesture.parameter()?;
    let basis = drag.basis?;
    let snap = match parameter {
        PlacementParameter::Stagger { .. }
            if basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs() =>
        {
            snap.map(|step| Vec2 {
                x: step.x,
                y: step.x,
            })
        }
        _ => snap,
    };
    let value = gesture_value(
        &basis,
        parameter,
        placement_current(&drag.matrix, parameter),
        world_delta,
        snap,
    );
    Some(placement_operation(&drag.matrix.id, parameter, value))
}

pub(super) fn sample_transform(
    drag: &TransformDrag,
    pointer: &web_sys::PointerEvent,
    point: Vec2,
    context: TransformSampleContext<'_>,
) -> Option<EditOperation> {
    let TransformSampleContext {
        view_width,
        view_height,
        settings,
        origins,
        affect,
        svg,
        guide,
    } = context;
    let matrix = &drag.matrix;
    match drag.gesture {
        TransformGesture::Stagger { .. }
        | TransformGesture::Spread { .. }
        | TransformGesture::Offset { .. }
        | TransformGesture::Bend { .. } => {
            let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return None;
            }
            let world_delta = Vec2 {
                x: (f64::from(pointer.client_x()) - drag.start_client_x) * view_width
                    / rect.width(),
                y: -(f64::from(pointer.client_y()) - drag.start_client_y) * view_height
                    / rect.height(),
            };
            let snap = if pointer.alt_key() {
                None
            } else {
                grid_step(matrix.pitch, settings.snap_fraction)
            };
            guide.set(None);
            placement_preview(drag, world_delta, snap)
        }
        TransformGesture::Splay {
            column,
            start_angle,
            origin,
            swing,
        } => {
            if swing {
                // A Ctrl pointer-down swings the keys: the origin moves with no
                // hold-still compensation, exactly like a Ctrl-click pick.
                let (world, snap) = snap_world_origin(
                    point,
                    matrix,
                    pointer,
                    &OriginSnapTarget {
                        settings,
                        origins,
                        svg,
                        view_width,
                        column_base: drag.origin_base,
                    },
                );
                guide.set(snap);
                return Some(EditOperation::SetMatrixSplay {
                    matrix_id: matrix.id.clone(),
                    column,
                    change: MatrixSplayChange::OriginSwing { world: Some(world) },
                });
            }
            let angle = (point.y - origin.y).atan2(point.x - origin.x) - start_angle;
            let mut delta = angle.sin().atan2(angle.cos()).to_degrees();
            if matches!(
                matrix.mirror,
                Some(boardstudio_model::model::Mirror::X | boardstudio_model::model::Mirror::Y)
            ) {
                delta = -delta;
            }
            let base = stored_column_angle(matrix, column, &affect);
            let value = base + delta;
            let value = if pointer.alt_key() {
                value
            } else {
                value.round()
            };
            guide.set(None);
            Some(EditOperation::SetMatrixSplay {
                matrix_id: matrix.id.clone(),
                column,
                change: MatrixSplayChange::Angle {
                    angle: value,
                    affect,
                },
            })
        }
        TransformGesture::Origin { column, swing } => {
            let (world, snap) = snap_world_origin(
                point,
                matrix,
                pointer,
                &OriginSnapTarget {
                    settings,
                    origins,
                    svg,
                    view_width,
                    column_base: drag.origin_base,
                },
            );
            guide.set(snap);
            let change = if swing {
                MatrixSplayChange::OriginSwing { world: Some(world) }
            } else {
                MatrixSplayChange::Origin { world: Some(world) }
            };
            Some(EditOperation::SetMatrixSplay {
                matrix_id: matrix.id.clone(),
                column,
                change,
            })
        }
        TransformGesture::MatrixMove { anchor } => {
            let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return None;
            }
            let world_delta = Vec2 {
                x: (f64::from(pointer.client_x()) - drag.start_client_x) * view_width
                    / rect.width(),
                y: -(f64::from(pointer.client_y()) - drag.start_client_y) * view_height
                    / rect.height(),
            };
            guide.set(None);
            let origin = solve_matrix_origin(matrix, anchor, world_delta);
            if origin == matrix.origin {
                // A drag outside every Follow direction changes nothing.
                return None;
            }
            let mut next = matrix.as_ref().clone();
            next.origin = origin;
            Some(EditOperation::SetMatrix {
                matrix: next,
                definitions: None,
            })
        }
        TransformGesture::MatrixRotate {
            anchor,
            center,
            start_angle,
            start_rotation,
        } => {
            let rotation_followed = match anchor {
                Some(anchor) => anchor.follow.rotation,
                None => true,
            };
            if !rotation_followed {
                return None;
            }
            let angle = (point.y - center.y).atan2(point.x - center.x) - start_angle;
            let mut delta = angle.sin().atan2(angle.cos()).to_degrees();
            if matches!(matrix.mirror, Some(Mirror::X | Mirror::Y)) {
                delta = -delta;
            }
            let mut rotation = start_rotation + delta;
            if !pointer.alt_key() {
                rotation = rotation.round();
            }
            if Some(rotation) == matrix.rotation {
                return None;
            }
            let mut next = matrix.as_ref().clone();
            next.rotation = Some(rotation);
            Some(EditOperation::SetMatrix {
                matrix: next,
                definitions: None,
            })
        }
    }
}

pub(super) fn stored_column_angle(matrix: &Matrix, column: u32, affect: &MatrixSplayAffect) -> f64 {
    matrix
        .column_layout
        .get(column as usize)
        .map_or(0.0, |record| {
            column_angle(record.splay, record.rotation, affect)
        })
}

pub(super) fn same_transform_as_start(drag: &TransformDrag, operation: &EditOperation) -> bool {
    if let Some(gesture_parameter) = drag.gesture.parameter() {
        return placement_target(operation, &drag.matrix).is_some_and(|(parameter, value)| {
            parameter == gesture_parameter && value == placement_current(&drag.matrix, parameter)
        });
    }
    match (&drag.gesture, operation) {
        (
            TransformGesture::Splay {
                column,
                swing: true,
                ..
            },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
            },
        )
        | (
            TransformGesture::Origin {
                column,
                swing: true,
            },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::OriginSwing { world: Some(world) },
            },
        ) => matrix_id == &drag.matrix.id && target == column && Some(*world) == drag.start_origin,
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Angle { angle, affect },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && *angle == stored_column_angle(&drag.matrix, *column, affect)
        }
        (
            TransformGesture::Origin { column, .. },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Origin { world },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && world.is_none()
                && drag
                    .matrix
                    .column_layout
                    .get(*column as usize)
                    .is_none_or(|record| record.splay_origin == Vec2::default())
        }
        (TransformGesture::MatrixMove { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            next.id == drag.matrix.id && next.origin == drag.matrix.origin
        }
        (TransformGesture::MatrixRotate { .. }, EditOperation::SetMatrix { matrix: next, .. }) => {
            next.id == drag.matrix.id && next.rotation == drag.matrix.rotation
        }
        _ => false,
    }
}

pub(super) fn submit_transform_edit(
    runtime: &Rc<Runtime>,
    drag: &TransformDrag,
    operation: EditOperation,
) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: drag.transaction_id.clone(),
        target_ids: vec![drag.matrix.id.clone()],
        operation,
    });
}

pub(super) fn clear_transform_preview(runtime: &Rc<Runtime>, drag: &TransformDrag) {
    if !drag.preview_submitted {
        return;
    }
    if let (Some(token), Some(revision), Some(scope)) = (
        drag.owner.token,
        drag.owner.revision,
        drag.owner.scope.as_ref(),
    ) {
        runtime.submit(Event::ClearPreview {
            operation_id: runtime.operation(),
            token,
            revision,
            board_id: scope.board_id.clone(),
            transaction_id: drag.transaction_id.clone(),
        });
    }
}

pub(super) fn cancel_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<TransformDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    guide: &mut Signal<Option<Vec2>>,
    mut quick_glyph: Signal<bool>,
    mut swing_glyph: Signal<bool>,
    svg: &LayoutTransformSvg,
) {
    if abandon_transform_drag(runtime, drag, arbiter) {
        guide.set(None);
        quick_glyph.set(false);
        swing_glyph.set(false);
    }
    set_rail_active(Rail::Matrix, svg, None);
}

/// Cancel, Escape, lost capture or an owner change: clear the preview, release the pointer
/// and the canvas, and land nothing. Returns whether a drag was active.
pub(super) fn abandon_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<TransformDrag>>>,
    arbiter: &CanvasInteractionArbiter,
) -> bool {
    let Some(active) = drag.borrow_mut().take() else {
        return false;
    };
    clear_transform_preview(runtime, &active);
    if active.capture.has_pointer_capture(active.pointer_id) {
        let _ = active.capture.release_pointer_capture(active.pointer_id);
    }
    arbiter.release(CanvasInteractionOwner::MatrixTransform);
    true
}
