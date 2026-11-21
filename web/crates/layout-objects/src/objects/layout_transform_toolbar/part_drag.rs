//! Ticket 07: part rail drag kinds, guards, resolvers and commits.
use super::*;

/// Ticket 07: which anchor value a part drag writes. Spread and Stagger edit
/// the values carried from the anchor key; Offset and Rotation edit the
/// part's local values. Detached parts have no anchor: Move drives the
/// shipped part-drag gesture, and Rotate commits one `SetPartRotation`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PartAnchorAxis {
    Spread,
    Stagger,
    Offset,
    Rotation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PartRailDragKind {
    AnchorValue(PartAnchorAxis),
    DetachedMove,
    DetachedRotate,
}

pub(super) fn part_drag_kind(
    tool: LayoutTransformTool,
    anchored: bool,
) -> Option<PartRailDragKind> {
    match (tool, anchored) {
        (LayoutTransformTool::Move, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Offset))
        }
        (LayoutTransformTool::Rotate, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Rotation))
        }
        (LayoutTransformTool::Spread, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Spread))
        }
        (LayoutTransformTool::Stagger, true) => {
            Some(PartRailDragKind::AnchorValue(PartAnchorAxis::Stagger))
        }
        (LayoutTransformTool::Move, false) => Some(PartRailDragKind::DetachedMove),
        (LayoutTransformTool::Rotate, false) => Some(PartRailDragKind::DetachedRotate),
        _ => None,
    }
}

/// Ticket 07: the carried (Spread/Stagger) axes rotate with the
/// follow-filtered anchor key; the local Offset frame adds the carried Splay,
/// matching Core's anchor evaluation.
pub(super) fn part_placement_basis(base_degrees: f64, splay_degrees: f64) -> SceneBasis {
    let base = base_degrees.to_radians();
    let frame = (base_degrees + splay_degrees).to_radians();
    SceneBasis {
        stagger_axis: Vec2 {
            x: -base.sin(),
            y: base.cos(),
        },
        axis_x: Vec2 {
            x: frame.cos(),
            y: frame.sin(),
        },
        axis_y: Vec2 {
            x: -frame.sin(),
            y: frame.cos(),
        },
        shift_axis_x: Vec2 {
            x: frame.cos(),
            y: frame.sin(),
        },
        shift_axis_y: Vec2 {
            x: -frame.sin(),
            y: frame.cos(),
        },
        spread_axis: Some(Vec2 {
            x: base.cos(),
            y: base.sin(),
        }),
    }
}

/// Ticket 07: a part Rotate sweep snaps to half degrees; Alt keeps the raw
/// angle.
pub(super) fn part_rotate_value(start: f64, swept_degrees: f64, alt: bool) -> f64 {
    let value = start + swept_degrees;
    if alt {
        value
    } else {
        (value / 0.5).round() * 0.5
    }
}

/// Ticket 07: the member part owning an anchor key, so the drag frame follows
/// the evaluated key (frozen members included) exactly like Core.
pub(super) fn anchor_key_member_id(anchor: &PartAnchor) -> String {
    format!(
        "matrix/{}/r{}c{}",
        anchor.matrix_id, anchor.row, anchor.column
    )
}

/// Ticket 07: the follow-filtered anchor key rotation behind a part drag, or
/// `None` when the anchor key no longer resolves.
pub(super) fn anchor_key_base_rotation(document: &ProjectDoc, anchor: &PartAnchor) -> Option<f64> {
    let member = document
        .parts
        .iter()
        .find(|part| part.id == anchor_key_member_id(anchor))?;
    let rotation = evaluated_part_pose(document, member).rotation;
    Some(if anchor.follow.rotation {
        rotation
    } else {
        0.0
    })
}

/// Ticket 07: a part drag captured at pointer-down, mirroring the matrix
/// `TransformDrag` lifecycle (preview while moving, one composed commit on
/// drop). The anchor snapshot and frames never move mid-drag; the commit
/// re-solves the net delta against whatever is accepted when it runs.
#[derive(Clone)]
pub(super) struct PartDrag {
    pub(super) pointer_id: i64,
    pub(super) capture: SvgElement,
    pub(super) part_id: String,
    pub(super) kind: PartRailDragKind,
    pub(super) tool: LayoutTransformTool,
    pub(super) transaction_id: String,
    pub(super) owner: LayoutOwnerIdentity,
    pub(super) context: TreeContext,
    pub(super) anchor: Option<PartAnchor>,
    pub(super) start_spread: f64,
    pub(super) start_stagger: f64,
    pub(super) start_offset: Vec2,
    pub(super) start_rotation: f64,
    pub(super) start_at: Vec2,
    pub(super) base_rotation: f64,
    pub(super) frame_rotation: f64,
    pub(super) center: Vec2,
    pub(super) start_angle: f64,
    pub(super) start_client_x: f64,
    pub(super) start_client_y: f64,
    /// A detached Move only begins the session gesture past the shipped body
    /// threshold, so a click never churns Core.
    pub(super) gesture_begun: bool,
    pub(super) preview_submitted: bool,
    pub(super) pending: Option<EditOperation>,
}

impl RailDrag for PartDrag {
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

/// Ticket 07: the net change a finished anchored drag applies, re-solved
/// against the accepted anchor when the commit runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PartAxisDelta {
    Spread(f64),
    Stagger(f64),
    Offset(Vec2),
    Rotation(f64),
}

/// Ticket 07: why an anchored part-drag edit has to retire: the part is gone,
/// locked, driven, a member, or no longer anchored. Mirrors the Inspector's
/// anchor guard so rail drags and typed edits agree.
pub(super) fn part_anchor_drag_guard(document: &ProjectDoc, part_id: &str) -> Option<String> {
    let part = document.parts.iter().find(|part| part.id == part_id)?;
    if part.locked == Some(true) {
        return Some("This part is locked.".into());
    }
    if part.anchor.is_none() {
        return Some("The part is no longer anchored.".into());
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(&part.id))
    {
        return Some("This part follows its matrix.".into());
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship.".into()),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified.".into()),
    }
}

/// Ticket 07: resolve a finished anchored part drag against the accepted
/// document: the net delta applies to whatever the anchor holds when the
/// edit runs, so a drag queued behind other edits composes.
pub(super) fn part_anchor_drag_resolver(
    part_id: String,
    transaction_id: String,
    delta: PartAxisDelta,
) -> EditResolver {
    EditResolver::new(
        "layout-part-rail-drag",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if let Some(reason) = part_anchor_drag_guard(document, &part_id) {
                return Resolution::Retire(reason);
            }
            let anchor = document
                .parts
                .iter()
                .find(|part| part.id == part_id)
                .and_then(|part| part.anchor.clone())
                .expect("guarded above");
            let change = match delta {
                PartAxisDelta::Spread(delta) => {
                    PartAnchorValueChange::Spread(anchor.spread + delta)
                }
                PartAxisDelta::Stagger(delta) => {
                    PartAnchorValueChange::Stagger(anchor.stagger + delta)
                }
                PartAxisDelta::Offset(delta) => PartAnchorValueChange::Offset(Vec2 {
                    x: anchor.offset.x + delta.x,
                    y: anchor.offset.y + delta.y,
                }),
                PartAxisDelta::Rotation(delta) => {
                    PartAnchorValueChange::Rotation(anchor.rotation + delta)
                }
            };
            Resolution::submit_with_transaction_id(
                vec![part_id.clone()],
                transaction_id.clone(),
                EditOperation::SetPartAnchorValues {
                    part_id: part_id.clone(),
                    change,
                },
            )
        },
    )
}

pub(super) fn submit_part_preview(
    runtime: &Rc<Runtime>,
    drag: &PartDrag,
    operation: EditOperation,
) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: drag.transaction_id.clone(),
        target_ids: vec![drag.part_id.clone()],
        operation,
    });
}

pub(super) fn clear_part_preview(runtime: &Rc<Runtime>, drag: &PartDrag) {
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

/// Ticket 07: cancel a part drag (Escape, lost capture, owner change): clear
/// the anchored preview or the detached session gesture, release the pointer
/// and the canvas, and land nothing. Returns whether a drag was active.
pub(super) fn cancel_part_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<PartDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    svg: &LayoutTransformSvg,
) -> bool {
    let Some(active) = drag.borrow_mut().take() else {
        return false;
    };
    match active.kind {
        PartRailDragKind::AnchorValue(_) | PartRailDragKind::DetachedRotate => {
            clear_part_preview(runtime, &active)
        }
        PartRailDragKind::DetachedMove => {
            if active.gesture_begun {
                runtime.submit(Event::GestureCancel {
                    pointer_id: active.pointer_id,
                });
            }
        }
    }
    if active.capture.has_pointer_capture(active.pointer_id as i32) {
        let _ = active
            .capture
            .release_pointer_capture(active.pointer_id as i32);
    }
    arbiter.release(CanvasInteractionOwner::MatrixTransform);
    set_rail_active(Rail::Part, svg, None);
    true
}

/// Ticket 07: why a detached Rotate edit has to retire: the part is gone,
/// locked, anchored, a member, or driven. Mirrors the Inspector's guards so
/// rail drags and typed edits agree.
pub(super) fn part_detached_rotate_guard(document: &ProjectDoc, part_id: &str) -> Option<String> {
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return Some("The part no longer exists.".into());
    };
    if part.locked == Some(true) {
        return Some("This part is locked.".into());
    }
    if part.anchor.is_some() {
        return Some("The part is now anchored.".into());
    }
    if document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.contains(&part.id))
    {
        return Some("This part follows its matrix.".into());
    }
    match document.constraint_driving_part(part_id) {
        Ok(Some(_)) => Some("This part is driven by a relationship.".into()),
        Ok(None) => None,
        Err(_) => Some("The saved placement relationship could not be verified.".into()),
    }
}

/// Ticket 07: resolve a finished detached Rotate drag against the accepted
/// document: the net delta applies to whatever the pose holds when the edit
/// runs, so a drag queued behind other edits composes.
pub(super) fn part_detached_rotate_resolver(
    part_id: String,
    transaction_id: String,
    delta: f64,
) -> EditResolver {
    EditResolver::new(
        "layout-part-rail-drag",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            if let Some(reason) = part_detached_rotate_guard(document, &part_id) {
                return Resolution::Retire(reason);
            }
            let rotation = document
                .parts
                .iter()
                .find(|part| part.id == part_id)
                .map(|part| evaluated_part_pose(document, part).rotation)
                .expect("guarded above")
                + delta;
            Resolution::submit_with_transaction_id(
                vec![part_id.clone()],
                transaction_id.clone(),
                EditOperation::SetPartRotation {
                    part_id: part_id.clone(),
                    rotation,
                },
            )
        },
    )
}

/// Ticket 07: one live sample of an anchored part drag: the absolute
/// `SetPartAnchorValues` change for the preview, the net change for the popup,
/// and the composed delta for the commit.
pub(super) fn sample_anchor_value(
    drag: &PartDrag,
    axis: PartAnchorAxis,
    world_delta: Vec2,
    point: Vec2,
    alt: bool,
    pitch: Vec2,
    snap_fraction: f64,
) -> Option<(EditOperation, FeedbackDelta, PartAxisDelta, String)> {
    let anchor = drag.anchor.as_ref()?;
    let basis = part_placement_basis(drag.base_rotation, drag.frame_rotation - drag.base_rotation);
    if let PartAnchorAxis::Rotation = axis {
        let angle = (point.y - drag.center.y).atan2(point.x - drag.center.x) - drag.start_angle;
        let swept = angle.sin().atan2(angle.cos()).to_degrees();
        let value = part_rotate_value(drag.start_rotation, swept, alt);
        let delta = value - drag.start_rotation;
        return Some((
            EditOperation::SetPartAnchorValues {
                part_id: drag.part_id.clone(),
                change: PartAnchorValueChange::Rotation(value),
            },
            FeedbackDelta::Deg(delta),
            PartAxisDelta::Rotation(delta),
            format!("{value:.1}°"),
        ));
    }
    let snap = if alt {
        None
    } else {
        grid_step(pitch, snap_fraction)
    };
    // A Stagger drag steps along the dominant screen axis, like the matrix
    // Stagger preview; Spread steps the column pitch directly.
    let snap = match axis {
        PartAnchorAxis::Stagger if basis.stagger_axis.x.abs() > basis.stagger_axis.y.abs() => snap
            .map(|step| Vec2 {
                x: step.x,
                y: step.x,
            }),
        _ => snap,
    };
    let (parameter, start) = match axis {
        PartAnchorAxis::Spread => (
            PlacementParameter::ColumnSpread {
                column: anchor.column,
            },
            PlacementValue::Spread(drag.start_spread),
        ),
        PartAnchorAxis::Stagger => (
            PlacementParameter::Stagger {
                column: anchor.column,
            },
            PlacementValue::Stagger(drag.start_stagger),
        ),
        PartAnchorAxis::Offset => (
            PlacementParameter::ColumnOffset {
                column: anchor.column,
            },
            PlacementValue::Offset(drag.start_offset),
        ),
        PartAnchorAxis::Rotation => return None,
    };
    let value = gesture_value(&basis, parameter, start, world_delta, snap);
    let (change, feedback, delta, value_text) = match (axis, value) {
        (PartAnchorAxis::Spread, PlacementValue::Spread(value)) => (
            PartAnchorValueChange::Spread(value),
            FeedbackDelta::Mm(value - drag.start_spread),
            PartAxisDelta::Spread(value - drag.start_spread),
            format_placement_value(PlacementValue::Spread(value)),
        ),
        (PartAnchorAxis::Stagger, PlacementValue::Stagger(value)) => (
            PartAnchorValueChange::Stagger(value),
            FeedbackDelta::Mm(value - drag.start_stagger),
            PartAxisDelta::Stagger(value - drag.start_stagger),
            format_placement_value(PlacementValue::Stagger(value)),
        ),
        (PartAnchorAxis::Offset, PlacementValue::Offset(value)) => (
            PartAnchorValueChange::Offset(value),
            FeedbackDelta::Mm2(value.x - drag.start_offset.x, value.y - drag.start_offset.y),
            PartAxisDelta::Offset(Vec2 {
                x: value.x - drag.start_offset.x,
                y: value.y - drag.start_offset.y,
            }),
            format_placement_value(PlacementValue::Offset(value)),
        ),
        _ => return None,
    };
    Some((
        EditOperation::SetPartAnchorValues {
            part_id: drag.part_id.clone(),
            change,
        },
        feedback,
        delta,
        value_text,
    ))
}

/// Ticket 07: whether a part rail drag may start: the session is Ready and
/// saved at the owner's revision, no preview or gesture is in flight, and the
/// part is still in the accepted document.
pub(super) fn part_start_is_editable(
    runtime: &Rc<Runtime>,
    owner: &LayoutOwnerIdentity,
    part_id: &str,
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
                .parts
                .iter()
                .any(|part| part.id == part_id)
        })
}
