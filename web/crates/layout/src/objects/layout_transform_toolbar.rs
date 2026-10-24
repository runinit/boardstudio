//! Transform-property navigation for the Layout command pill.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::{ScopedTreeContext, TreeContext};
use crate::matrix_transform_operation::{
    ArrowKey, MatrixDragAnchor, PlacementParameter, PlacementValue, SceneBasis, column_angle,
    gesture_value, matrix_drag_anchor_pose, matrix_drag_transport, nudge_value,
    placement_operation, solve_matrix_origin, solve_matrix_rotation,
};
use crate::presentation::{
    LayoutOwnerIdentity,
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution,
};
use boardstudio_core::model::{
    EditOperation, Matrix, MatrixPlacementChange, MatrixScene, MatrixSplayAffect,
    MatrixSplayChange, Mirror, Vec2,
};
use boardstudio_web_runtime::pending_edits::PendingEdits;
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardButtonDensity, BoardPopover, BoardToolbar, BoardToolbarButton,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::SvgElement;

type TransformEscapeListener = Rc<
    RefCell<
        Option<(
            web_sys::Document,
            Closure<dyn FnMut(web_sys::KeyboardEvent)>,
        )>,
    >,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutTransformTool {
    Stagger,
    Spread,
    Offset,
    Splay,
    Origin,
    /// Moves the selected matrix through its Origin.
    Move,
    /// Turns the selected matrix through its stored rotation.
    Rotate,
}

impl LayoutTransformTool {
    const ALL: [(Self, &'static str); 7] = [
        (Self::Stagger, "Stagger"),
        (Self::Spread, "Spread"),
        (Self::Offset, "Offset"),
        (Self::Splay, "Splay"),
        (Self::Origin, "Origin"),
        (Self::Move, "Move"),
        (Self::Rotate, "Rotate"),
    ];
}

/// What choosing a menu tool does: the tool that is active afterwards and the selection
/// kind to switch to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolActivation {
    pub active: Option<LayoutTransformTool>,
    pub selection_kind: Option<LayoutSelectionKind>,
}

/// The activation rules for a menu tool. Choosing the active tool again turns it off and
/// leaves the selection alone. Offset keeps a Row selection and turns every other matrix
/// selection into its Column. Stagger is a column parameter: a Row selection becomes the
/// column of the retained key when there is one (the conversion reads the retained cell)
/// and otherwise column 1, and every other selection becomes its Column. Splay and Origin
/// turn the selection into its Column. Move and Rotate keep the Matrix selection: they
/// edit the matrix itself, not one of its parameters.
pub fn activate_transform_tool(
    active: Option<LayoutTransformTool>,
    tool: LayoutTransformTool,
    context: &TreeContext,
) -> ToolActivation {
    if active == Some(tool) {
        return ToolActivation {
            active: None,
            selection_kind: None,
        };
    }
    match (tool, context) {
        (LayoutTransformTool::Offset, TreeContext::Row { .. }) => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Row),
        },
        (LayoutTransformTool::Move | LayoutTransformTool::Rotate, _) => ToolActivation {
            active: Some(tool),
            selection_kind: None,
        },
        _ => ToolActivation {
            active: Some(tool),
            selection_kind: Some(LayoutSelectionKind::Column),
        },
    }
}

#[derive(Clone, PartialEq)]
pub struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub pointer_tools_visible: bool,
    pub pointer_tools_available: bool,
    pub active_tool: Option<LayoutTransformTool>,
    /// `None` when the selection is not a Matrix; otherwise whether the
    /// matrix follows rotation, which decides whether the Rotate entry is
    /// offered enabled or disabled with an explanation.
    pub matrix_rotate_followed: Option<bool>,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
    pub on_transform_tool: EventHandler<LayoutTransformTool>,
}

#[derive(Clone)]
pub struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub struct LayoutTransformSvg(pub Rc<RefCell<Option<SvgElement>>>);

impl PartialEq for LayoutTransformSvg {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransformGesture {
    /// Edits the column's stagger. `quick_offset` is latched from `ctrlKey` at pointer-down
    /// and turns the drag into a free 2-D column-offset gesture for its whole length.
    Stagger { column: u32, quick_offset: bool },
    /// Edits the column's inherited spread step.
    Spread { column: u32 },
    /// Edits one row's or one column's independent offset (`RowOffset` or `ColumnOffset`).
    Offset { parameter: PlacementParameter },
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
    /// The placement parameter a Stagger, Spread or Offset gesture writes.
    fn parameter(&self) -> Option<PlacementParameter> {
        match *self {
            Self::Stagger {
                column,
                quick_offset,
            } => Some(stagger_parameter(column, quick_offset)),
            Self::Spread { column } => Some(PlacementParameter::ColumnSpread { column }),
            Self::Offset { parameter } => Some(parameter),
            Self::Splay { .. }
            | Self::Origin { .. }
            | Self::MatrixMove { .. }
            | Self::MatrixRotate { .. } => None,
        }
    }
}

#[derive(Clone)]
struct TransformDrag {
    pointer_id: i32,
    capture: SvgElement,
    matrix: Rc<Matrix>,
    gesture: TransformGesture,
    transaction_id: String,
    owner: LayoutOwnerIdentity,
    context: TreeContext,
    tool: LayoutTransformTool,
    /// The scene frame at pointer-down; the world delta is projected onto it.
    basis: Option<SceneBasis>,
    /// The dragged column's world base and world origin at pointer-down: the
    /// snap target and the no-op point for an origin swing.
    origin_base: Option<Vec2>,
    start_origin: Option<Vec2>,
    start_client_x: f64,
    start_client_y: f64,
    pending: Option<EditOperation>,
    preview_submitted: bool,
}

#[component]
pub fn LayoutTransformToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    #[props(into)] matrix: Rc<Matrix>,
    #[props(into)] projection: Rc<MatrixScene>,
    context: TreeContext,
    tool: LayoutTransformTool,
    #[props(default)] surface_size: Option<Signal<(f64, f64)>>,
    snap_settings: super::layout_toolbar::LayoutSnapSettings,
    #[props(into)] snap_origins: Rc<Vec<super::super::outline_snapping::Origin>>,
    splay_affect: Signal<MatrixSplayAffect>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
) -> Element {
    let control = use_signal(|| false);
    let _control_listener = use_hook(move || {
        Rc::new(
            if matches!(
                tool,
                LayoutTransformTool::Stagger
                    | LayoutTransformTool::Splay
                    | LayoutTransformTool::Origin
            ) {
                CtrlModifierListener::install(control)
            } else {
                None
            },
        )
    });
    let _ = surface_size.map(|size| size());
    let drag = use_hook(|| Rc::new(RefCell::new(None::<TransformDrag>)));
    let snap_guide = use_signal(|| None::<Vec2>);
    // True from a Ctrl pointer-down until release: the handle shows the offset glyph.
    let quick_glyph = use_signal(|| false);
    // True from a Ctrl pointer-down on a Splay or Origin handle until release.
    let swing_glyph = use_signal(|| false);
    let runtime_inner = runtime.0.clone();
    let arbiter_for_drop = arbiter.clone();
    let drag_for_drop = drag.clone();
    let mut guide_for_drop = snap_guide;
    use_drop(move || {
        cancel_transform_drag(
            &runtime_inner,
            &drag_for_drop,
            &arbiter_for_drop,
            &mut guide_for_drop,
            quick_glyph,
            swing_glyph,
        );
    });
    let escape_listener = use_hook(TransformEscapeListener::default);
    use_effect({
        let listener = escape_listener.clone();
        let runtime = runtime.0.clone();
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        move || {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            let runtime = runtime.clone();
            let drag = drag.clone();
            let arbiter = arbiter.clone();
            let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() != "Escape" || event.default_prevented() {
                    return;
                }
                if event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                    .is_some_and(|target| {
                        target
                            .closest("input, textarea, select, [contenteditable='true']")
                            .ok()
                            .flatten()
                            .is_some()
                    })
                {
                    return;
                }
                // Another visible handle may own the live gesture. Its listener cancels it.
                if drag.borrow().is_none() && arbiter.current().is_some() {
                    return;
                }
                event.prevent_default();
                let mut guide = snap_guide;
                cancel_transform_drag(
                    &runtime,
                    &drag,
                    &arbiter,
                    &mut guide,
                    quick_glyph,
                    swing_glyph,
                );
                on_finish.call(());
            }) as Box<dyn FnMut(_)>);
            let _ = document
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
            *listener.borrow_mut() = Some((document, callback));
        }
    });
    use_drop({
        let listener = escape_listener.clone();
        move || {
            if let Some((document, callback)) = listener.borrow_mut().take() {
                let _ = document.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            }
        }
    });
    let active_owner = owner.clone();
    let active_context = context.clone();
    let active_tool = tool;
    let drag_for_owner = drag.clone();
    let runtime_for_owner = runtime.0.clone();
    let arbiter_for_owner = arbiter.clone();
    let mut guide_for_owner = snap_guide;
    use_effect(use_reactive(
        (&active_owner, &active_context, &active_tool),
        {
            move |(active_owner, active_context, active_tool)| {
                if drag_for_owner.borrow().as_ref().is_some_and(|active| {
                    active.owner != active_owner
                        || active.context != active_context
                        || active.tool != active_tool
                }) {
                    cancel_transform_drag(
                        &runtime_for_owner,
                        &drag_for_owner,
                        &arbiter_for_owner,
                        &mut guide_for_owner,
                        quick_glyph,
                        swing_glyph,
                    );
                }
            }
        },
    ));

    let mut selected_points = {
        let model = runtime.0.model();
        model.accepted.as_ref().map_or_else(Vec::new, |snapshot| {
            model
                .selected_part_ids
                .iter()
                .filter_map(|id| {
                    snapshot
                        .document
                        .parts
                        .iter()
                        .find(|part| &part.id == id)
                        .map(|part| part.pose.at)
                })
                .collect::<Vec<_>>()
        })
    };
    match &context {
        TreeContext::Column { column, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.column == *column)
                .map(|cell| cell.pose.at)
                .collect()
        }
        TreeContext::Row { row, .. } => {
            selected_points = projection
                .cells
                .iter()
                .filter(|cell| cell.enabled && cell.row == *row)
                .map(|cell| cell.pose.at)
                .collect()
        }
        _ => {}
    }
    // The matrix Move/Rotate handles share one anchor point on the drawn
    // matrix: the first enabled key's pose. The rotation center captured at
    // pointer-down uses the same point from the live scene, so the two cannot
    // drift apart.
    let matrix_handle_point = projection
        .cells
        .iter()
        .find(|cell| cell.enabled)
        .or(projection.cells.first())
        .map(|cell| cell.pose.at)
        .unwrap_or(matrix.origin);
    let matrix_rotation_followed = matrix
        .anchor
        .as_ref()
        .map(|anchor| anchor.follow.rotation)
        .unwrap_or(true);
    let (handle_point, handle_scale, selected_column) = {
        let selected_column = context_column(&context).unwrap_or(0);
        let selected_row = context_row(&context);
        let point = match tool {
            // Stagger is a column parameter. Activation turns a Row selection into a Column,
            // so a Row here (selected while the tool was active) renders no Stagger handle.
            LayoutTransformTool::Stagger if selected_row.is_some() => None,
            // An Offset handle on a Row sits at the row's first key (column 0 when unselected).
            LayoutTransformTool::Offset if let Some(row) = selected_row => {
                let cell = projection
                    .cells
                    .iter()
                    .find(|cell| cell.enabled && cell.row == row && cell.column == 0)
                    .or_else(|| {
                        projection
                            .cells
                            .iter()
                            .find(|cell| cell.enabled && cell.row == row)
                    });
                selected_points
                    .first()
                    .copied()
                    .or_else(|| cell.map(|cell| cell.pose.at))
                    .map(|at| Vec2 {
                        x: at.x,
                        y: at.y + 12.0,
                    })
            }
            // Spread is a column step: a Row selection renders no handle, and
            // column 0 follows the pitch, so it has no step to drag.
            LayoutTransformTool::Spread if selected_row.is_some() || selected_column == 0 => None,
            LayoutTransformTool::Stagger
            | LayoutTransformTool::Offset
            | LayoutTransformTool::Spread => {
                let cell = projection
                    .cells
                    .iter()
                    .find(|cell| cell.enabled && cell.row == 0 && cell.column == selected_column);
                selected_points
                    .first()
                    .copied()
                    .or_else(|| cell.map(|cell| cell.pose.at))
                    .map(|at| Vec2 {
                        x: at.x,
                        y: at.y + 12.0,
                    })
            }
            LayoutTransformTool::Splay | LayoutTransformTool::Origin => projection
                .columns
                .iter()
                .find(|basis| basis.column == selected_column)
                .map(|basis| basis.splay_origin),
            LayoutTransformTool::Move | LayoutTransformTool::Rotate => Some(matrix_handle_point),
        };
        let scale = svg.0.borrow().as_ref().map_or(3.0, |surface| {
            let rect = surface.get_bounding_client_rect();
            if rect.width() > 0.0 {
                (width / rect.width()).max(height / rect.height().max(1.0)) * 3.0
            } else {
                3.0
            }
        });
        let point = if matches!(
            tool,
            LayoutTransformTool::Stagger
                | LayoutTransformTool::Offset
                | LayoutTransformTool::Spread
        ) && !selected_points.is_empty()
        {
            Some(Vec2 {
                x: selected_points.iter().map(|p| p.x).sum::<f64>() / selected_points.len() as f64,
                y: selected_points
                    .iter()
                    .map(|p| p.y)
                    .fold(f64::INFINITY, f64::min)
                    - matrix.pitch.y / 2.0
                    - 6.0 * scale,
            })
        } else {
            point
        };
        (point, scale, selected_column)
    };

    let Some(handle_point) = handle_point else {
        return rsx! {};
    };
    let fallback_angle_point = angle_handle_point(
        &projection,
        selected_column,
        matrix.pitch.y,
        &selected_points,
    );
    let angle_point = if selected_points.is_empty() {
        fallback_angle_point
    } else {
        Vec2 {
            x: selected_points.iter().map(|p| p.x).sum::<f64>() / selected_points.len() as f64,
            y: selected_points
                .iter()
                .map(|p| p.y)
                .fold(f64::INFINITY, f64::min)
                - matrix.pitch.y / 2.0
                - 18.0 * handle_scale,
        }
    };
    let angle_degrees = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(0.0, |basis| {
            column_angle(basis.splay_angle, basis.rotation, &splay_affect())
        });
    let arc = splay_arc(handle_point, angle_point, angle_degrees);
    let current_owner = owner.clone();
    let current_context = context.clone();
    let matrix_for_key = matrix.clone();
    let matrix_for_spread = matrix.clone();
    let offset_parameter = match context_row(&context) {
        Some(row) => PlacementParameter::RowOffset { row },
        None => PlacementParameter::ColumnOffset {
            column: selected_column,
        },
    };
    let offset_now = match placement_current(&matrix, offset_parameter) {
        PlacementValue::Offset(offset) => offset,
        _ => Vec2::default(),
    };
    let stagger_now = matrix
        .column_layout
        .get(selected_column as usize)
        .map(|record| &record.stagger)
        .copied()
        .unwrap_or(0.0);
    let projection_for_key = projection.clone();
    let projection_for_start = projection.clone();
    let matrix_for_start = matrix.clone();
    let owner_for_key = owner.clone();
    let context_for_key = context.clone();
    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.0.clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let guide_for_start = snap_guide;
    let start_drag = Rc::new(move |event: PointerEvent, gesture: TransformGesture| {
        let mut guide_for_start = guide_for_start;
        let mut quick_for_start = quick_glyph;
        let mut swing_for_start = swing_glyph;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        // The modifier is read here and nowhere else, so a drag never switches parameter.
        let gesture = match gesture {
            TransformGesture::Stagger { column, .. } => TransformGesture::Stagger {
                column,
                quick_offset: pointer.ctrl_key(),
            },
            TransformGesture::Splay {
                column,
                start_angle,
                origin,
                ..
            } => TransformGesture::Splay {
                column,
                start_angle,
                origin,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::Origin { column, .. } => TransformGesture::Origin {
                column,
                swing: pointer.ctrl_key(),
            },
            TransformGesture::MatrixMove { .. } => {
                // Capture the anchor context now; the preview and the commit
                // compose this drag against whatever the matrix becomes. An
                // anchored matrix whose anchor target is not laid out refuses
                // the drag before it acquires anything.
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixMove { anchor }
            }
            TransformGesture::MatrixRotate { .. } => {
                let Some(accepted) = runtime_for_start.model().accepted.clone() else {
                    return;
                };
                let Ok(anchor) = matrix_drag_anchor(&accepted, &matrix_for_start) else {
                    return;
                };
                TransformGesture::MatrixRotate {
                    anchor,
                    center: Vec2::default(),
                    start_angle: 0.0,
                    start_rotation: 0.0,
                }
            }
            other => other,
        };
        let basis = gesture
            .parameter()
            .and_then(|parameter| SceneBasis::for_parameter(&projection_for_start, parameter));
        if pointer.button() != 0 || drag_for_start.borrow().is_some() {
            return;
        }
        if !transform_owner_is_current(
            &runtime_for_start,
            workspace,
            scope_generation,
            selected_context,
            &current_owner,
            &current_context,
        ) || context_matrix_id(&current_context) != Some(matrix_for_start.id.as_str())
            || !transform_start_is_editable(&runtime_for_start, &current_owner, &matrix_for_start)
            || !arbiter_for_start.try_acquire(CanvasInteractionOwner::MatrixTransform)
        {
            return;
        }
        let Some(capture) = pointer
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".m1-transform-handle").ok().flatten())
            .and_then(|target| target.dyn_into::<SvgElement>().ok())
        else {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        if capture.set_pointer_capture(pointer.pointer_id()).is_err() {
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        }
        pointer.prevent_default();
        pointer.stop_propagation();
        let focus_options = web_sys::FocusOptions::new();
        focus_options.set_prevent_scroll(true);
        let _ = capture.focus_with_options(&focus_options);
        let angle_origin = match gesture {
            TransformGesture::Splay { origin, .. } => origin,
            _ => Vec2::default(),
        };
        let Some(start_point) = crate::presentation::coordinates(
            &svg_for_start,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            let _ = capture.release_pointer_capture(pointer.pointer_id());
            arbiter_for_start.release(CanvasInteractionOwner::MatrixTransform);
            return;
        };
        let start_angle = (start_point.y - angle_origin.y).atan2(start_point.x - angle_origin.x);
        let gesture = match gesture {
            TransformGesture::Splay {
                column,
                origin,
                swing,
                ..
            } => TransformGesture::Splay {
                column,
                origin,
                start_angle,
                swing,
            },
            TransformGesture::MatrixRotate { anchor, .. } => {
                // The rotation center is the shared handle point: the first
                // enabled key's pose from the drag's own scene.
                let center = projection_for_start
                    .cells
                    .iter()
                    .find(|cell| cell.enabled)
                    .or(projection_for_start.cells.first())
                    .map(|cell| cell.pose.at)
                    .unwrap_or(matrix_for_start.origin);
                TransformGesture::MatrixRotate {
                    anchor,
                    center,
                    start_angle: (start_point.y - center.y).atan2(start_point.x - center.x),
                    start_rotation: matrix_for_start.rotation.unwrap_or(0.0),
                }
            }
            other => other,
        };
        let operation = runtime_for_start.operation();
        let swing_basis = match gesture {
            TransformGesture::Splay { column, .. } | TransformGesture::Origin { column, .. } => {
                projection_for_start
                    .columns
                    .iter()
                    .find(|basis| basis.column == column)
            }
            _ => None,
        };
        let origin_base = swing_basis.map(|basis| basis.base);
        let start_origin = swing_basis.map(|basis| basis.splay_origin);
        *drag_for_start.borrow_mut() = Some(TransformDrag {
            pointer_id: pointer.pointer_id(),
            capture,
            matrix: matrix_for_start.clone(),
            gesture,
            transaction_id: format!("layout-transform-{}", operation.0),
            owner: current_owner.clone(),
            context: current_context.clone(),
            tool,
            basis,
            origin_base,
            start_origin,
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            pending: None,
            preview_submitted: false,
        });
        guide_for_start.set(None);
        quick_for_start.set(matches!(
            gesture,
            TransformGesture::Stagger {
                quick_offset: true,
                ..
            }
        ));
        swing_for_start.set(matches!(
            gesture,
            TransformGesture::Splay { swing: true, .. }
                | TransformGesture::Origin { swing: true, .. }
        ));
    });

    let runtime_for_move = runtime.0.clone();
    let svg_for_move = svg.0.clone();
    let drag_for_move = drag.clone();
    let arbiter_for_move = arbiter.clone();
    let guide_for_move = snap_guide;
    let snap_settings_for_move = snap_settings.clone();
    let snap_origins_for_move = snap_origins.clone();
    let move_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_move = guide_for_move;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_move
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_move,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_transform_drag(
                &runtime_for_move,
                &drag_for_move,
                &arbiter_for_move,
                &mut guide_for_move,
                quick_glyph,
                swing_glyph,
            );
            return;
        }
        let Some(point) = crate::presentation::coordinates(
            &svg_for_move,
            &pointer,
            view_x,
            view_y,
            width,
            height,
        ) else {
            return;
        };
        let operation = sample_transform(
            &active,
            &pointer,
            point,
            TransformSampleContext {
                view_width: width,
                view_height: height,
                settings: &snap_settings_for_move,
                origins: &snap_origins_for_move,
                affect: splay_affect(),
                svg: &svg_for_move,
                guide: &mut guide_for_move,
            },
        );
        if let Some(operation) = operation
            && active.pending.as_ref() != Some(&operation)
        {
            submit_transform_edit(&runtime_for_move, &active, operation.clone());
            active.preview_submitted = true;
            active.pending = Some(operation);
            *drag_for_move.borrow_mut() = Some(active);
        }
    });

    let runtime_for_end = runtime.0.clone();
    let svg_for_end = svg.0.clone();
    let drag_for_end = drag.clone();
    let arbiter_for_end = arbiter.clone();
    let guide_for_end = snap_guide;
    let end_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_end = guide_for_end;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let Some(mut active) = drag_for_end
            .borrow()
            .clone()
            .filter(|drag| drag.pointer_id == pointer.pointer_id())
        else {
            return;
        };
        pointer.prevent_default();
        pointer.stop_propagation();
        if !transform_owner_is_current(
            &runtime_for_end,
            workspace,
            scope_generation,
            selected_context,
            &active.owner,
            &active.context,
        ) || active.tool != tool
        {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
            );
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &mut guide_for_end,
                quick_glyph,
                swing_glyph,
            );
            return;
        }
        let point =
            crate::presentation::coordinates(&svg_for_end, &pointer, view_x, view_y, width, height);
        let operation = point
            .and_then(|point| {
                sample_transform(
                    &active,
                    &pointer,
                    point,
                    TransformSampleContext {
                        view_width: width,
                        view_height: height,
                        settings: &snap_settings,
                        origins: &snap_origins,
                        affect: splay_affect(),
                        svg: &svg_for_end,
                        guide: &mut guide_for_end,
                    },
                )
            })
            .or_else(|| active.pending.clone());
        if let Some(operation) = operation {
            if active.pending.as_ref() != Some(&operation) {
                submit_transform_edit(&runtime_for_end, &active, operation.clone());
                active.preview_submitted = true;
            }
            if !same_transform_as_start(&active, &operation) {
                commit_transform_drag(&runtime_for_end, &active, &operation);
            } else if active.preview_submitted {
                clear_transform_preview(&runtime_for_end, &active);
            }
        } else if active.preview_submitted {
            clear_transform_preview(&runtime_for_end, &active);
        }
        guide_for_end.set(None);
        let mut quick_for_end = quick_glyph;
        quick_for_end.set(false);
        let mut swing_for_end = swing_glyph;
        swing_for_end.set(false);
        // Releasing capture fires `lostpointercapture`; retire the drag before it can cancel the drop.
        drag_for_end.borrow_mut().take();
        if active.capture.has_pointer_capture(active.pointer_id) {
            let _ = active.capture.release_pointer_capture(active.pointer_id);
        }
        arbiter_for_end.release(CanvasInteractionOwner::MatrixTransform);
    });

    let runtime_for_cancel = runtime.0.clone();
    let drag_for_cancel = drag.clone();
    let arbiter_for_cancel = arbiter.clone();
    let guide_for_cancel = snap_guide;
    let cancel_drag = Rc::new(move |event: PointerEvent| {
        let mut guide_for_cancel = guide_for_cancel;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        if drag_for_cancel
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer_id == pointer.pointer_id())
        {
            pointer.prevent_default();
            pointer.stop_propagation();
            cancel_transform_drag(
                &runtime_for_cancel,
                &drag_for_cancel,
                &arbiter_for_cancel,
                &mut guide_for_cancel,
                quick_glyph,
                swing_glyph,
            );
        }
    });

    let runtime_for_key = runtime.0.clone();
    let drag_for_key = drag.clone();
    let arbiter_for_key = arbiter.clone();
    let guide_for_key = snap_guide;
    let on_key_down = Rc::new(move |event: KeyboardEvent| {
        let mut guide_for_key = guide_for_key;
        let Some(key) = event.data().try_as_web_event() else {
            return;
        };
        if key.key() == "Escape" {
            key.prevent_default();
            key.stop_propagation();
            cancel_transform_drag(
                &runtime_for_key,
                &drag_for_key,
                &arbiter_for_key,
                &mut guide_for_key,
                quick_glyph,
                swing_glyph,
            );
            on_finish.call(());
            return;
        }
        if !matches!(
            key.key().as_str(),
            "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
        ) || !transform_nudge_admitted(
            &runtime_for_key,
            workspace,
            scope_generation,
            selected_context,
            &owner_for_key,
            &context_for_key,
        ) {
            return;
        }
        let Some(target) = key
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest(".m1-transform-handle").ok().flatten())
        else {
            return;
        };
        let step = if key.shift_key() { 1.0 } else { 0.1 };
        let delta = Vec2 {
            x: match key.key().as_str() {
                "ArrowLeft" => -step,
                "ArrowRight" => step,
                _ => 0.0,
            },
            y: match key.key().as_str() {
                "ArrowDown" => -step,
                "ArrowUp" => step,
                _ => 0.0,
            },
        };
        let arrow = match key.key().as_str() {
            "ArrowLeft" => ArrowKey::Left,
            "ArrowRight" => ArrowKey::Right,
            "ArrowUp" => ArrowKey::Up,
            _ => ArrowKey::Down,
        };
        let nudge = match target.get_attribute("aria-label").as_deref() {
            Some("Drag to spread") => {
                let parameter = PlacementParameter::ColumnSpread {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the spread axis is not consumed; column 0 has
                // no Spread handle, so its basis never reaches this mapping.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if nudge_value(
                    &basis,
                    parameter,
                    placement_current(&matrix_for_key, parameter),
                    arrow,
                    big,
                )
                .is_none()
                {
                    return;
                }
                TransformNudge::Spread {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some("Drag to stagger") => {
                let parameter = PlacementParameter::Stagger {
                    column: selected_column,
                };
                let big = key.shift_key();
                // The pair across the stagger axis is not consumed.
                let Some(basis) = SceneBasis::for_parameter(&projection_for_key, parameter) else {
                    return;
                };
                if context_row(&context_for_key).is_some()
                    || nudge_value(
                        &basis,
                        parameter,
                        placement_current(&matrix_for_key, parameter),
                        arrow,
                        big,
                    )
                    .is_none()
                {
                    return;
                }
                TransformNudge::Stagger {
                    column: selected_column,
                    key: arrow,
                    big,
                }
            }
            Some(label @ ("Drag to offset row" | "Drag to offset column")) => {
                let parameter = match (label, context_row(&context_for_key)) {
                    ("Drag to offset row", Some(row)) => PlacementParameter::RowOffset { row },
                    ("Drag to offset column", None) => PlacementParameter::ColumnOffset {
                        column: selected_column,
                    },
                    _ => return,
                };
                TransformNudge::Offset {
                    parameter,
                    key: arrow,
                    big: key.shift_key(),
                }
            }
            Some("Move splay origin") => TransformNudge::SplayOrigin {
                column: selected_column,
            },
            Some("Drag to splay") if delta.x != 0.0 => TransformNudge::SplayAngle {
                column: selected_column,
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
                affect: splay_affect(),
            },
            Some("Drag to move matrix") => TransformNudge::MatrixMove,
            Some("Drag to rotate matrix") if delta.x != 0.0 => TransformNudge::MatrixRotate {
                degrees: delta.x.signum() * if key.shift_key() { 5.0 } else { 1.0 },
            },
            _ => return,
        };
        key.prevent_default();
        key.stop_propagation();
        // A held key queues one step per repeat; each resolves against the matrix the
        // previous steps produced.
        let resolver = transform_nudge_resolver(matrix_for_key.id.clone(), nudge, delta);
        let mut edits = PendingEdits::default();
        edits.begin(
            &runtime_for_key,
            (),
            "layout-transform-nudge",
            Some("transform".into()),
            resolver,
        );
        let runtime = runtime_for_key.clone();
        let owner = owner_for_key.clone();
        let context = context_for_key.clone();
        spawn_local(async move {
            loop {
                let is_live = transform_nudge_admitted(
                    &runtime,
                    workspace,
                    scope_generation,
                    selected_context,
                    &owner,
                    &context,
                );
                if !edits.settle(is_live).is_empty() {
                    break;
                }
                gloo_timers::future::TimeoutFuture::new(16).await;
            }
        });
    });

    let gesture = match tool {
        LayoutTransformTool::Stagger => TransformGesture::Stagger {
            column: selected_column,
            quick_offset: false,
        },
        LayoutTransformTool::Spread => TransformGesture::Spread {
            column: selected_column,
        },
        LayoutTransformTool::Offset => TransformGesture::Offset {
            parameter: offset_parameter,
        },
        LayoutTransformTool::Splay | LayoutTransformTool::Origin => TransformGesture::Origin {
            column: selected_column,
            swing: false,
        },
        LayoutTransformTool::Move => TransformGesture::MatrixMove { anchor: None },
        LayoutTransformTool::Rotate => TransformGesture::MatrixRotate {
            anchor: None,
            center: Vec2::default(),
            start_angle: 0.0,
            start_rotation: 0.0,
        },
    };
    let stagger_axis = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(Vec2 { x: 0.0, y: 1.0 }, |basis| basis.stagger_axis);
    let arrow_rotation = stagger_arrow_degrees(stagger_axis);
    let spread_now = match placement_current(
        &matrix_for_spread,
        PlacementParameter::ColumnSpread {
            column: selected_column,
        },
    ) {
        PlacementValue::Spread(value) => value,
        _ => matrix_for_spread.pitch.x,
    };
    let handle_label = match (tool, context_row(&context)) {
        (LayoutTransformTool::Stagger, None) if control() || quick_glyph() => {
            "Drag to offset column"
        }
        (LayoutTransformTool::Offset, Some(_)) => "Drag to offset row",
        (LayoutTransformTool::Offset, None) => "Drag to offset column",
        (LayoutTransformTool::Spread, _) => "Drag to spread",
        _ => "Drag to stagger",
    };
    // The matrix Move/Rotate handles share one anchor point on the drawn
    // matrix: the first enabled key's pose. The rotation center captured at
    // pointer-down uses the same point from the live scene, so the two cannot
    // drift apart.
    let matrix_handle_point = projection
        .cells
        .iter()
        .find(|cell| cell.enabled)
        .or(projection.cells.first())
        .map(|cell| cell.pose.at)
        .unwrap_or(matrix.origin);
    let handle_value = match tool {
        LayoutTransformTool::Offset => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        LayoutTransformTool::Spread => format!("{spread_now:.2} mm"),
        _ if control() || quick_glyph() => {
            format!("X {:.2} mm, Y {:.2} mm", offset_now.x, offset_now.y)
        }
        _ => format!("{stagger_now:.2} mm"),
    };
    let quick_offset_shown = quick_glyph() || control() || tool == LayoutTransformTool::Offset;
    rsx! {
        if let Some(guide) = snap_guide() {
            g { class: "m1-transform-snap-guide", "aria-hidden": "true",
                circle { cx: "{guide.x}", cy: "{guide.y}", r: "{width / 170.0}" }
            }
        }
        if matches!(context, TreeContext::Matrix { .. })
            && matches!(tool, LayoutTransformTool::Move | LayoutTransformTool::Rotate)
        {
            g { class: "m1-transform-matrix-handles",
                path {
                    class: "m1-transform-rotate-guide",
                    d: "M{matrix_handle_point.x} {matrix_handle_point.y + 12.0} L{matrix_handle_point.x + 28.0} {matrix_handle_point.y + 12.0}"
                }
                g {
                    class: "m1-transform-handle m1-matrix-move-handle",
                    transform: "translate({matrix_handle_point.x} {matrix_handle_point.y + 12.0}) scale({handle_scale})",
                    role: "button", tabindex: "0", "aria-label": "Drag to move matrix",
                    "aria-description": "X {matrix.origin.x:.2} mm, Y {matrix.origin.y:.2} mm",
                    onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                    onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                    onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                    onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                    rect { x: "-8", y: "-8", width: "16", height: "16" }
                    path { d: "M0 -5v10M-5 0h10M-2.5 -2.5 0 -5l2.5 2.5M-2.5 2.5 0 5l2.5 -2.5" }
                    title { "Drag to move matrix · X {matrix.origin.x:.2} mm, Y {matrix.origin.y:.2} mm" }
                }
                if matrix_rotation_followed {
                    g {
                        class: "m1-transform-handle m1-matrix-rotate-handle",
                        transform: "translate({matrix_handle_point.x + 28.0} {matrix_handle_point.y + 12.0}) scale({handle_scale})",
                        role: "button", tabindex: "0", "aria-label": "Drag to rotate matrix",
                        "aria-description": "{matrix.rotation.unwrap_or(0.0):.2} degrees",
                        onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                        onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                        onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                        onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                        onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                        onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                        circle { r: "8" }
                        path { d: "M5.5 -2.5 A6 6 0 1 1 -2.5 -5.5M5.5 -5.5v3h-3" }
                        title { "Drag to rotate matrix · {matrix.rotation.unwrap_or(0.0):.2} degrees" }
                    }
                } else {
                    g {
                        class: "m1-transform-handle m1-matrix-rotate-handle m1-handle-disabled",
                        transform: "translate({matrix_handle_point.x + 28.0} {matrix_handle_point.y + 12.0}) scale({handle_scale})",
                        role: "button", tabindex: "0", "aria-label": "Drag to rotate matrix",
                        "aria-disabled": "true",
                        "aria-description": "Follow rotation is off, so turning changes nothing.",
                        circle { r: "8" }
                        path { d: "M5.5 -2.5 A6 6 0 1 1 -2.5 -5.5M5.5 -5.5v3h-3" }
                        title { "Rotate matrix (unavailable: Follow rotation is off)" }
                    }
                }
            }
        } else if tool == LayoutTransformTool::Spread {
            g {
                class: "m1-transform-handle m1-spread-handle",
                transform: "translate({handle_point.x} {handle_point.y}) scale({handle_scale})",
                role: "button", tabindex: "0", "aria-label": "{handle_label}",
                "aria-description": "{handle_value}",
                onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                circle { r: "4" }
                g { transform: "rotate(90)",
                    path { d: "M0 -3v6M-1.5 -1.5 0 -3l1.5 1.5M-1.5 1.5 0 3l1.5 -1.5" }
                }
                title { "Drag to spread · {handle_value}" }
            }
        } else if matches!(tool, LayoutTransformTool::Stagger | LayoutTransformTool::Offset) {
            g {
                class: "m1-transform-handle m1-stagger-handle",
                transform: "translate({handle_point.x} {handle_point.y}) scale({handle_scale})",
                // The value is a description, not `aria-valuenow`: that belongs to range
                // roles, and a handle that also edits a 2-D offset is a button.
                role: "button", tabindex: "0", "aria-label": "{handle_label}",
                "aria-description": "{handle_value}. Hold Ctrl while dragging to offset the column.",
                "data-quick-offset": "{quick_offset_shown}",
                onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                circle { r: "4" }
                if quick_offset_shown {
                    path { d: "M0 -3.2v6.4M-3.2 0h6.4M-1.4 -1.8 0 -3.2l1.4 1.4M-1.4 1.8 0 3.2l1.4 -1.4M-1.8 -1.4-3.2 0l1.4 1.4M1.8 -1.4 3.2 0l-1.4 1.4" }
                } else {
                    // Drawn along `staggerAxis`, the direction the column's keys move per +1 mm.
                    g { transform: "rotate({arrow_rotation})",
                        path { d: "M0 -3v6M-1.5 -1.5 0 -3l1.5 1.5M-1.5 1.5 0 3l1.5 -1.5" }
                    }
                }
            }
        } else {
            g { class: "m1-transform-splay-handles",
                if tool == LayoutTransformTool::Splay {
                    path { class: "m1-transform-splay-guide", d: "M{handle_point.x} {handle_point.y} L{angle_point.x} {angle_point.y}" }
                    path { class: "m1-transform-splay-guide", d: "{arc}" }

                }
                g {
                    class: "m1-transform-handle m1-splay-origin-handle",
                    transform: "translate({handle_point.x} {handle_point.y}) scale({handle_scale})",
                    role: "button", tabindex: "0", "aria-label": "Move splay origin",
                    "aria-description": "Hold Ctrl while dragging to swing the origin without moving the keys.",
                    "data-swing": "{swing_glyph()}",
                    onpointerdown: { let start = start_drag.clone(); move |event| start(event, TransformGesture::Origin { column: selected_column, swing: false }) },
                    onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                    onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                    onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                    onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                    circle { class: "m1-transform-handle-target", r: "4" }
                    circle { r: "2" }
                    path { d: "M-3.5 0h7M0 -3.5v7" }
                    title { "Move splay origin" }
                }
                if tool == LayoutTransformTool::Splay {
                    {
                        let origin = handle_point;
                        let gesture = TransformGesture::Splay {
                            column: selected_column,
                            start_angle: 0.0,
                            origin,
                            swing: false,
                        };
                        rsx! {
                            g {
                                class: "m1-transform-handle m1-splay-angle-handle",
                                transform: "translate({angle_point.x} {angle_point.y}) scale({handle_scale})",
                                role: "button", tabindex: "0", "aria-label": "Drag to splay",
                                "aria-description": "Hold Ctrl while dragging to swing the origin instead.",
                                "data-swing": "{swing_glyph()}",
                                onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                                onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                                onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                                onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                                onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                                onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                                rect { class: "m1-transform-handle-target", x: "-11", y: "-5", width: "22", height: "10" }
                                path { d: "M-9 0Q0 6 9 0M-9 0l1 3M-9 0l3-.5M9 0l-1 3M9 0l-3-.5" }
                                title { "Drag to splay · {angle_degrees:.0}°" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What a handle key press moves, captured as intent. The step is a delta: the resolver
/// applies it to the matrix accepted when the edit runs.
#[derive(Clone, Debug, PartialEq)]
enum TransformNudge {
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

fn accepted_matrix<'a>(accepted: &'a AcceptedSnapshot, matrix_id: &str) -> Option<&'a Matrix> {
    accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
}

/// The anchor context a matrix drag inverse-solves through, or `None` for an
/// unanchored matrix. Refuses when the matrix is anchored but its anchor
/// target is not laid out, so the drag never solves through a stale frame.
fn matrix_drag_anchor(
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

fn accepted_basis<'a>(
    accepted: &'a AcceptedSnapshot,
    matrix_id: &str,
    column: u32,
) -> Option<&'a boardstudio_core::model::MatrixColumnBasis> {
    accepted
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)?
        .columns
        .iter()
        .find(|basis| basis.column == column)
}

/// Rotation, in degrees, that turns the upright arrow glyph (drawn along +y) onto `axis`.
fn stagger_arrow_degrees(axis: Vec2) -> f64 {
    (-axis.x).atan2(axis.y).to_degrees()
}

fn stagger_parameter(column: u32, quick_offset: bool) -> PlacementParameter {
    if quick_offset {
        PlacementParameter::ColumnOffset { column }
    } else {
        PlacementParameter::Stagger { column }
    }
}

/// The parameter's value in `matrix`; an index past the saved vector reads as zero,
/// and an absent Spread or Padding reads the pitch so a drag that starts from a
/// default writes an explicit step.
fn placement_current(matrix: &Matrix, parameter: PlacementParameter) -> PlacementValue {
    match parameter {
        PlacementParameter::Stagger { column } => PlacementValue::Stagger(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.stagger)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnOffset { column } => PlacementValue::Offset(
            matrix
                .column_layout
                .get(column as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowOffset { row } => PlacementValue::Offset(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::RowShift { row } => PlacementValue::Shift(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.shift)
                .copied()
                .unwrap_or_default(),
        ),
        PlacementParameter::KeyShift { row, column } => PlacementValue::Shift(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.shift)
                .unwrap_or_default(),
        ),
        PlacementParameter::RowBend { row } => PlacementValue::Bend(
            matrix
                .row_layout
                .get(row as usize)
                .map(|record| &record.bend)
                .copied()
                .unwrap_or(0.0),
        ),
        PlacementParameter::KeyBend { row, column } => PlacementValue::Bend(
            matrix
                .cells
                .iter()
                .find(|cell| cell.row == row && cell.column == column)
                .and_then(|cell| cell.bend)
                .unwrap_or(0.0),
        ),
        PlacementParameter::ColumnSpread { column } => PlacementValue::Spread(
            matrix
                .column_layout
                .get(column as usize)
                .and_then(|record| record.spread)
                .unwrap_or(matrix.pitch.x),
        ),
        PlacementParameter::RowPadding { row } => PlacementValue::Padding(
            matrix
                .row_layout
                .get(row as usize)
                .and_then(|record| record.padding)
                .unwrap_or(matrix.pitch.y),
        ),
    }
}

/// The parameter and absolute value a placement operation sets. Absent Spread
/// and Padding resolve against the matrix pitch, matching `placement_current`,
/// so a drag that lands on a default still compares and deltas exactly.
fn placement_target(
    operation: &EditOperation,
    matrix: &Matrix,
) -> Option<(PlacementParameter, PlacementValue)> {
    let EditOperation::SetMatrixPlacement {
        matrix_id: target,
        change,
    } = operation
    else {
        return None;
    };
    if target != &matrix.id {
        return None;
    }
    match change {
        MatrixPlacementChange::ColumnStagger { column, value } => Some((
            PlacementParameter::Stagger { column: *column },
            PlacementValue::Stagger(*value),
        )),
        MatrixPlacementChange::ColumnOffset { column, value } => Some((
            PlacementParameter::ColumnOffset { column: *column },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowOffset { row, value } => Some((
            PlacementParameter::RowOffset { row: *row },
            PlacementValue::Offset(*value),
        )),
        MatrixPlacementChange::RowShift { row, value } => Some((
            PlacementParameter::RowShift { row: *row },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::KeyShift { row, column, value } => Some((
            PlacementParameter::KeyShift {
                row: *row,
                column: *column,
            },
            PlacementValue::Shift(*value),
        )),
        MatrixPlacementChange::RowBend { row, value } => Some((
            PlacementParameter::RowBend { row: *row },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::KeyBend { row, column, value } => Some((
            PlacementParameter::KeyBend {
                row: *row,
                column: *column,
            },
            PlacementValue::Bend(*value),
        )),
        MatrixPlacementChange::ColumnSpread { column, value } => Some((
            PlacementParameter::ColumnSpread { column: *column },
            PlacementValue::Spread(value.unwrap_or(matrix.pitch.x)),
        )),
        MatrixPlacementChange::RowPadding { row, value } => Some((
            PlacementParameter::RowPadding { row: *row },
            PlacementValue::Padding(value.unwrap_or(matrix.pitch.y)),
        )),
        _ => None,
    }
}

/// The net change of a placement parameter over a drag, from start to snapped value.
#[derive(Clone, Copy, Debug, PartialEq)]
enum PlacementDelta {
    Stagger(f64),
    Offset(Vec2),
    Shift(Vec2),
    Spread(f64),
    Padding(f64),
}

fn placement_delta(start: PlacementValue, end: PlacementValue) -> Option<PlacementDelta> {
    match (start, end) {
        (PlacementValue::Stagger(start), PlacementValue::Stagger(end)) => {
            Some(PlacementDelta::Stagger(end - start))
        }
        (PlacementValue::Offset(start), PlacementValue::Offset(end)) => {
            Some(PlacementDelta::Offset(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Shift(start), PlacementValue::Shift(end)) => {
            Some(PlacementDelta::Shift(Vec2 {
                x: end.x - start.x,
                y: end.y - start.y,
            }))
        }
        (PlacementValue::Spread(start), PlacementValue::Spread(end)) => {
            Some(PlacementDelta::Spread(end - start))
        }
        (PlacementValue::Padding(start), PlacementValue::Padding(end)) => {
            Some(PlacementDelta::Padding(end - start))
        }
        _ => None,
    }
}

fn apply_placement_delta(current: PlacementValue, delta: PlacementDelta) -> Option<PlacementValue> {
    match (current, delta) {
        (PlacementValue::Stagger(value), PlacementDelta::Stagger(delta)) => {
            Some(PlacementValue::Stagger(value + delta))
        }
        (PlacementValue::Offset(value), PlacementDelta::Offset(delta)) => {
            Some(PlacementValue::Offset(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Shift(value), PlacementDelta::Shift(delta)) => {
            Some(PlacementValue::Shift(Vec2 {
                x: value.x + delta.x,
                y: value.y + delta.y,
            }))
        }
        (PlacementValue::Spread(value), PlacementDelta::Spread(delta)) => {
            Some(PlacementValue::Spread(value + delta))
        }
        (PlacementValue::Padding(value), PlacementDelta::Padding(delta)) => {
            Some(PlacementValue::Padding(value + delta))
        }
        _ => None,
    }
}

fn transform_commit(
    transaction_id: String,
    matrix_id: &str,
    operation: EditOperation,
) -> Resolution {
    Resolution::submit_with_transaction_id(vec![matrix_id.to_owned()], transaction_id, operation)
}

/// The accepted frame of a placement parameter, or why the edit has to retire: the column
/// (or, for a row offset, the row) has gone from the accepted matrix.
fn accepted_placement_basis(
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
enum NudgeStop {
    /// The column or row has gone from the accepted matrix.
    Gone(&'static str),
    /// The key is across the stagger axis and is not consumed.
    NotConsumed,
}

impl NudgeStop {
    fn resolution(self) -> Resolution {
        match self {
            Self::Gone(reason) => Resolution::Retire(reason.into()),
            Self::NotConsumed => Resolution::Unchanged,
        }
    }
}

/// One arrow step against the accepted matrix.
fn nudge_placement(
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
fn transform_nudge_resolver(matrix_id: String, nudge: TransformNudge, delta: Vec2) -> EditResolver {
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
enum TransformDragResult {
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

fn transform_drag_result(
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
fn transform_drag_resolver(
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

fn commit_transform_drag(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: &EditOperation) {
    let Some(result) = transform_drag_result(drag, operation) else {
        return;
    };
    let mut edits = PendingEdits::default();
    edits.begin(
        runtime,
        (),
        "layout-transform-drag",
        Some("transform".into()),
        transform_drag_resolver(drag.matrix.id.clone(), drag.transaction_id.clone(), result),
    );
    spawn_local(async move {
        loop {
            if !edits.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(16).await;
        }
    });
}

/// A handle key press is a delta intent, so it stays admitted while earlier presses are
/// still applying: only the selection, scope and workspace must be the live owner.
fn transform_nudge_admitted(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selected_context: Signal<Option<ScopedTreeContext>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
) -> bool {
    let model = runtime.model();
    owner.workspace == "Layout"
        && workspace() == owner.workspace
        && generation() == owner.generation
        && runtime.scope() == owner.scope
        && matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && owner.scope.as_ref().is_some_and(|scope| {
            model.active_board_id == scope.board_id
                && model.active_instance_id == scope.instance_id
                && selected_context().is_some_and(|selected| {
                    selected.scope == *scope && selected.context == *context
                })
        })
}

fn context_matrix_id(context: &TreeContext) -> Option<&str> {
    match context {
        TreeContext::Matrix { matrix_id }
        | TreeContext::Row { matrix_id, .. }
        | TreeContext::Column { matrix_id, .. }
        | TreeContext::Key { matrix_id, .. } => Some(matrix_id),
        TreeContext::Component { matrix_id, .. } => matrix_id.as_deref(),
        _ => None,
    }
}

fn context_column(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Column { column, .. } | TreeContext::Key { column, .. } => Some(*column),
        TreeContext::Component { column, .. } => *column,
        _ => None,
    }
}

fn context_row(context: &TreeContext) -> Option<u32> {
    match context {
        TreeContext::Row { row, .. } => Some(*row),
        _ => None,
    }
}

fn transform_owner_is_current(
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

fn transform_start_is_editable(
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
fn grid_step(pitch: Vec2, fraction: f64) -> Option<Vec2> {
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

fn snap_delta(delta: Vec2, pitch: Vec2, fraction: f64) -> Vec2 {
    let Some(step) = grid_step(pitch, fraction) else {
        return delta;
    };
    Vec2 {
        x: (delta.x / step.x).round() * step.x,
        y: (delta.y / step.y).round() * step.y,
    }
}

fn snap_tolerance(svg: &Rc<RefCell<Option<SvgElement>>>, view_width: f64) -> f64 {
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
struct OriginSnapTarget<'a> {
    settings: &'a super::layout_toolbar::LayoutSnapSettings,
    origins: &'a [super::super::outline_snapping::Origin],
    svg: &'a Rc<RefCell<Option<SvgElement>>>,
    view_width: f64,
    column_base: Option<Vec2>,
}

fn snap_world_origin(
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
            super::super::outline_snapping::snap_origin(point, target.origins, tolerance)
        {
            return (at, Some(at));
        }
    }
    (
        snap_delta(point, matrix.pitch, target.settings.snap_fraction),
        None,
    )
}

struct TransformSampleContext<'a> {
    view_width: f64,
    view_height: f64,
    settings: &'a super::layout_toolbar::LayoutSnapSettings,
    origins: &'a [super::super::outline_snapping::Origin],
    affect: MatrixSplayAffect,
    svg: &'a Rc<RefCell<Option<SvgElement>>>,
    guide: &'a mut Signal<Option<Vec2>>,
}

/// The preview of a Stagger or Offset gesture: an absolute placement value for the gesture's
/// parameter, built by the placement module from the world delta since pointer-down.
///
/// A stagger snaps to the grid step of the stagger axis' dominant component: pitch.y for an
/// upright axis, pitch.x when a splay has turned the axis nearer to horizontal. The gesture
/// value reads `y` of the step it is given, so the dominant step is handed over there.
fn placement_preview(
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

fn sample_transform(
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
        | TransformGesture::Offset { .. } => {
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
                Some(boardstudio_core::model::Mirror::X | boardstudio_core::model::Mirror::Y)
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

fn stored_column_angle(matrix: &Matrix, column: u32, affect: &MatrixSplayAffect) -> f64 {
    matrix
        .column_layout
        .get(column as usize)
        .map_or(0.0, |record| {
            column_angle(record.splay, record.rotation, affect)
        })
}

fn same_transform_as_start(drag: &TransformDrag, operation: &EditOperation) -> bool {
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

fn submit_transform_edit(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: EditOperation) {
    runtime.submit(Event::PreviewEdit {
        operation_id: runtime.operation(),
        transaction_id: drag.transaction_id.clone(),
        target_ids: vec![drag.matrix.id.clone()],
        operation,
    });
}

fn clear_transform_preview(runtime: &Rc<Runtime>, drag: &TransformDrag) {
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

fn cancel_transform_drag(
    runtime: &Rc<Runtime>,
    drag: &Rc<RefCell<Option<TransformDrag>>>,
    arbiter: &CanvasInteractionArbiter,
    guide: &mut Signal<Option<Vec2>>,
    mut quick_glyph: Signal<bool>,
    mut swing_glyph: Signal<bool>,
) {
    if abandon_transform_drag(runtime, drag, arbiter) {
        guide.set(None);
        quick_glyph.set(false);
        swing_glyph.set(false);
    }
}

/// Cancel, Escape, lost capture or an owner change: clear the preview, release the pointer
/// and the canvas, and land nothing. Returns whether a drag was active.
fn abandon_transform_drag(
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

// Presentation-only angular annotation: the accepted Core scene supplies origin and angle.
fn splay_arc(origin: Vec2, end: Vec2, degrees: f64) -> String {
    let dx = end.x - origin.x;
    let dy = end.y - origin.y;
    let radius = dx.hypot(dy);
    let angle = dy.atan2(dx);
    let sweep_degrees = degrees.clamp(-359.9, 359.9);
    let start = angle - sweep_degrees.to_radians();
    let x = origin.x + radius * start.cos();
    let y = origin.y + radius * start.sin();
    let large = u8::from(sweep_degrees.abs() > 180.0);
    let sweep = u8::from(sweep_degrees >= 0.0);
    format!(
        "M{x} {y} A{radius} {radius} 0 {large} {sweep} {} {}",
        end.x, end.y
    )
}

fn angle_handle_point(
    projection: &MatrixScene,
    column: u32,
    pitch_y: f64,
    points: &[Vec2],
) -> Vec2 {
    if points.is_empty() {
        let origin = projection
            .columns
            .iter()
            .find(|basis| basis.column == column)
            .map_or(Vec2::default(), |basis| basis.splay_origin);
        return Vec2 {
            x: origin.x,
            y: origin.y + 20.0,
        };
    }
    let x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min)
        - pitch_y * 0.9;
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        + pitch_y * 0.9;
    Vec2 { x, y }
}

#[component]
pub fn LayoutContextActionToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    selection_kind: LayoutSelectionKind,
) -> Element {
    let property_action = mount.pointer_tools_visible
        && mount.properties_available
        && (matches!(
            selection_kind,
            LayoutSelectionKind::Matrix
                | LayoutSelectionKind::Column
                | LayoutSelectionKind::Key
                | LayoutSelectionKind::Part
        ) || (selection_kind == LayoutSelectionKind::Row && mount.row_available));
    let selection_label = match selection_kind {
        LayoutSelectionKind::Matrix => "Matrix",
        LayoutSelectionKind::Row => "Row",
        LayoutSelectionKind::Column => "Column",
        LayoutSelectionKind::Key => "Key",
        LayoutSelectionKind::Part => "Part",
    };
    let rotate_followed = mount.matrix_rotate_followed.unwrap_or(true);
    let rotate_accessible_label = if rotate_followed {
        "Activate Rotate for the selected matrix"
    } else {
        "Rotate matrix (unavailable: Follow rotation is off)"
    };
    let tools = match selection_kind {
        LayoutSelectionKind::Matrix => vec![
            (
                LayoutTransformTool::Move,
                "Move matrix",
                "Activate Move for the selected matrix",
                true,
                false,
            ),
            (
                LayoutTransformTool::Rotate,
                "Rotate matrix",
                rotate_accessible_label,
                true,
                !rotate_followed,
            ),
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Key | LayoutSelectionKind::Part => vec![
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Row => vec![
            (
                LayoutTransformTool::Offset,
                "Offset row",
                "Activate Offset for the selected row",
                mount.row_available,
                false,
            ),
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger column; selection changes to Column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread column; selection changes to Column",
                mount.column_available,
                false,
            ),
        ],
        LayoutSelectionKind::Column => vec![
            (
                LayoutTransformTool::Stagger,
                "Stagger column",
                "Activate Stagger for the selected column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Offset,
                "Offset column",
                "Activate Offset for the selected column",
                mount.column_available,
                false,
            ),
            (
                LayoutTransformTool::Spread,
                "Spread column",
                "Activate Spread for the selected column",
                mount.column_available,
                false,
            ),
        ],
    };
    let mut visible_tools = if mount.pointer_tools_visible && mount.pointer_tools_available {
        tools
            .into_iter()
            .filter(|(_, _, _, available, _)| *available)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if mount.pointer_tools_visible
        && mount.pointer_tools_available
        && mount.column_available
        && matches!(
            selection_kind,
            LayoutSelectionKind::Column | LayoutSelectionKind::Matrix
        )
    {
        visible_tools.push((
            LayoutTransformTool::Splay,
            "Splay",
            "Activate Splay for the selected column",
            true,
            false,
        ));
    }
    if !property_action && visible_tools.is_empty() {
        return rsx! {};
    }
    let on_show_properties = mount.on_show_properties;
    let on_selection_kind = mount.on_selection_kind;
    let position_menu = open_menu;
    let property_label = if selection_kind == LayoutSelectionKind::Row {
        "Row offsets"
    } else {
        "Position"
    };
    let property_accessible_label = if selection_kind == LayoutSelectionKind::Row {
        "Open Row offsets properties for Row".to_owned()
    } else {
        format!("Open Position and rotation properties for {selection_label}")
    };

    rsx! {
        BoardToolbar {
            class: "layout-context-actions flex items-center gap-1 px-2",
            label: "Actions for {selection_label}",
            if property_action {
                BoardToolbarButton {
                    index: 0,
                    class: "layout-context-action",
                    aria_label: property_accessible_label.clone(),
                    title: property_accessible_label,
                    on_activate: move |_| {
                        on_show_properties.call(());
                        if selection_kind == LayoutSelectionKind::Row {
                            on_selection_kind.call(LayoutSelectionKind::Row);
                        }
                        close_layout_command_menu(position_menu, LayoutCommandMenu::Transform);
                    },
                    svg { class: "m1-context-tool-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                        path { d: "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3M5 7l-3 3 3 3M15 7l3 3-3 3" }
                    }
                    span { class: "layout-action-label", "{property_label}" }
                }
            }
            for (index, (tool, label, accessible_label, _, disabled)) in visible_tools.into_iter().enumerate() {
                LayoutContextToolButton {
                    index: index + usize::from(property_action),
                    mount: mount.clone(),
                    open_menu,
                    tool,
                    label: label.to_owned(),
                    accessible_label: accessible_label.to_owned(),
                    pressed: mount.active_tool == Some(tool),
                    disabled,
                }
            }
        }
    }
}

#[component]
fn LayoutContextToolButton(
    index: usize,
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    tool: LayoutTransformTool,
    label: String,
    accessible_label: String,
    pressed: bool,
    #[props(default)] disabled: bool,
) -> Element {
    let on_transform_tool = mount.on_transform_tool;
    let glyph = match tool {
        LayoutTransformTool::Stagger => "M10 2v16M6 6l4-4 4 4M6 14l4 4 4-4",
        LayoutTransformTool::Spread => "M2 10h16M4 6l-2 4 2 4M16 6l2 4-2 4",
        LayoutTransformTool::Offset => "M2 10h16M6 6l-4 4 4 4M14 6l4 4-4 4",
        LayoutTransformTool::Splay => "M4 16h13M4 16l8-12M7 16a7 7 0 0 0 3-6",
        LayoutTransformTool::Origin => "M10 1v18M1 10h18M5 5h10v10H5z",
        LayoutTransformTool::Move => "M10 2v16M2 10h16M7 5l3-3 3 3M7 15l3 3 3-3",
        LayoutTransformTool::Rotate => "M17 10a7 7 0 1 1-2-4.9M17 2v5h-5",
    };
    rsx! {
        BoardToolbarButton {
            index,
            class: "layout-context-action",
            aria_label: accessible_label.clone(),
            title: accessible_label,
            pressed,
            disabled,
            on_activate: move |_| {
                on_transform_tool.call(tool);
                close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
            },
            svg { class: "m1-context-tool-icon", view_box: "0 0 20 20", "aria-hidden": "true", path { d: glyph } }
            span { class: "layout-action-label", {match label.as_str() {
                "Stagger column" => "Stagger",
                "Offset column" | "Offset row" => "Offset",
                "Spread column" => "Spread",
                "Move matrix" => "Move",
                "Rotate matrix" => "Rotate",
                _ => label.as_str(),
            }} }
        }
    }
}

#[component]
pub fn LayoutTransformToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
    #[props(default)] selection_kind: Option<LayoutSelectionKind>,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Transform);
    let position_label = "Position & rotation";

    rsx! {
        BoardPopover {
            open: is_open,
            on_open_change: move |next: bool| {
                let mut open_menu = open_menu;
                if next {
                    open_menu.set(Some(LayoutCommandMenu::Transform));
                } else if open_menu() == Some(LayoutCommandMenu::Transform) {
                    open_menu.set(None);
                }
            },
            trigger_id: "m1-layout-transform-trigger",
            content_id: "m1-layout-transform-menu",
            trigger_label: "Transform",
            root_class: "m1-layout-command-menu m1-layout-transform-menu",
            menu_kind: "transform",
            content_class: "m1-layout-transform-popover grid gap-1",
            content_role: "group",
            content_label: "Transform tools and properties",
            trigger: rsx! {
                svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    circle { cx: "4", cy: "10", r: "1.2", fill: "currentColor" }
                    circle { cx: "10", cy: "10", r: "1.2", fill: "currentColor" }
                    circle { cx: "16", cy: "10", r: "1.2", fill: "currentColor" }
                }
                span { class: "layout-action-label", "Transform" }
            },
            super::layout_toolbar::LayoutCommandMenuHeader {
                label: "Transform".to_owned(),
                close_label: "Close Transform".to_owned(),
                open_menu,
                menu: LayoutCommandMenu::Transform,
            }
            if let Some(selection_kind) = selection_kind {
                LayoutContextActionToolbar { mount: mount.clone(), open_menu, selection_kind }
                BoardButton {
                    density: BoardButtonDensity::Compact,
                    disabled: !mount.pointer_tools_available || !mount.column_available,
                    pressed: mount.active_tool == Some(LayoutTransformTool::Origin),
                    onclick: move |_| {
                        mount.on_transform_tool.call(LayoutTransformTool::Origin);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Origin"
                }
            } else {
            for (tool, label) in LayoutTransformTool::ALL {
                if mount.pointer_tools_visible {
                    BoardButton {
                        density: BoardButtonDensity::Compact,
                        disabled: !mount.pointer_tools_available,
                        pressed: mount.active_tool == Some(tool),
                        onclick: move |_| {
                            mount.on_transform_tool.call(tool);
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                        },
                        "{label}"
                    }
                }
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.properties_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "{position_label}"
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.column_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    mount.on_selection_kind.call(LayoutSelectionKind::Column);
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "Splay & origin"
            }
            BoardButton {
                density: BoardButtonDensity::Compact,
                disabled: !mount.row_available,
                onclick: move |_| {
                    mount.on_show_properties.call(());
                    mount.on_selection_kind.call(LayoutSelectionKind::Row);
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                },
                "Row offsets"
            }
            }
            if !mount.properties_available {
                p { "Select a matrix, row, column, key or part to open its current transform properties." }
            }
        }
    }
}

#[cfg(test)]
mod shortcut_tests {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn shortcut_test_host() -> Element {
        let mut properties_tab = use_signal(|| false);
        let mut owner_current = use_signal(|| true);
        let mut selection_changes = use_signal(|| 0_u32);
        let mut open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
        let on_show_properties = EventHandler::new(move |()| {
            if owner_current() {
                properties_tab.set(true);
            }
        });
        let on_selection_kind = EventHandler::new(move |_: LayoutSelectionKind| {
            owner_current.set(false);
            selection_changes += 1;
        });
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: false,
            pointer_tools_available: false,
            active_tool: None,
            matrix_rotate_followed: None,
            on_selection_kind,
            on_show_properties,
            on_transform_tool: EventHandler::new(|_: LayoutTransformTool| {}),
        };

        rsx! {
            LayoutTransformToolbar { mount, open_menu }
            button {
                id: "reset-shortcut-probe",
                onclick: move |_| {
                    properties_tab.set(false);
                    owner_current.set(true);
                    selection_changes.set(0);
                    open_menu.set(Some(LayoutCommandMenu::Transform));
                },
                "Reset"
            }
            output { id: "shortcut-probe-state", "{properties_tab()}:{selection_changes()}" }
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct ContextHostProps {
        selection_kind: LayoutSelectionKind,
        pointer_tools_visible: bool,
    }

    #[component]
    fn context_test_host(props: ContextHostProps) -> Element {
        let mut active_tool = use_signal(|| None::<LayoutTransformTool>);
        let mut selection_kind = use_signal(|| props.selection_kind);
        let open_menu = use_signal(|| None::<LayoutCommandMenu>);
        let on_transform_tool = EventHandler::new(move |tool| {
            let context = match selection_kind() {
                LayoutSelectionKind::Row => TreeContext::Row {
                    matrix_id: "matrix-1".to_owned(),
                    row: 0,
                },
                _ => TreeContext::Column {
                    matrix_id: "matrix-1".to_owned(),
                    column: 0,
                },
            };
            let activation = activate_transform_tool(active_tool(), tool, &context);
            active_tool.set(activation.active);
            if let Some(next_kind) = activation.selection_kind {
                selection_kind.set(next_kind);
            }
        });
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: props.pointer_tools_visible,
            pointer_tools_available: true,
            active_tool: active_tool(),
            matrix_rotate_followed: None,
            on_selection_kind: EventHandler::new(move |kind| selection_kind.set(kind)),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool,
        };
        let selection_label = match selection_kind() {
            LayoutSelectionKind::Matrix => "Matrix",
            LayoutSelectionKind::Row => "Row",
            LayoutSelectionKind::Column => "Column",
            LayoutSelectionKind::Key => "Key",
            LayoutSelectionKind::Part => "Part",
        };
        let active_label = match active_tool() {
            Some(LayoutTransformTool::Stagger) => "Stagger",
            Some(LayoutTransformTool::Spread) => "Spread",
            Some(LayoutTransformTool::Offset) => "Offset",
            Some(LayoutTransformTool::Splay) => "Splay",
            Some(LayoutTransformTool::Origin) => "Origin",
            Some(LayoutTransformTool::Move) => "Move",
            Some(LayoutTransformTool::Rotate) => "Rotate",
            None => "None",
        };

        rsx! {
            LayoutContextActionToolbar { mount, open_menu, selection_kind: selection_kind() }
            output { id: "context-action-state", "{selection_label}:{active_label}" }
        }
    }

    /// Ticket 23: the Handles toggle flips view state and reports the intent,
    /// so hiding handles never touches the document. The host owns no Runtime
    /// and no edit machinery: the intent carries a plain bool to workspace state.
    #[component]
    fn snap_dock_host() -> Element {
        let mut handles = use_signal(|| true);
        let mut toggled = use_signal(|| None::<bool>);
        rsx! {
            super::super::layout_toolbar::LayoutSnapDock {
                settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                open_menu: use_signal(|| None),
                on_snap_intent: EventHandler::new(|_| {}),
                handles_visible: handles(),
                on_toggle_handles: EventHandler::new(move |visible| {
                    toggled.set(Some(visible));
                    handles.set(visible);
                }),
            }
            output { id: "snap-dock-toggled", "{toggled():?}" }
        }
    }

    fn handles_checkbox(root: &web_sys::Element) -> web_sys::HtmlInputElement {
        let labels = root.query_selector_all("label").unwrap();
        let toggle: web_sys::Element = (0..labels.length())
            .filter_map(|index| labels.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .find(|node| node.text_content().as_deref() == Some("Canvas handles"))
            .expect("the dock names the Canvas handles toggle");
        toggle
            .query_selector("input")
            .unwrap()
            .expect("the toggle is a checkbox")
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    #[wasm_bindgen_test]
    async fn handles_toggle_reports_intent_and_flips_view_only() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(snap_dock_host),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root.query_selector("#m1-layout-snap-trigger")
            .unwrap()
            .expect("the snap dock trigger is mounted")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert!(handles_checkbox(&root).checked(), "handles start visible");
        handles_checkbox(&root).click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#snap-dock-toggled")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Some(false)"),
            "the toggle reports the flipped view state"
        );
        assert!(
            !handles_checkbox(&root).checked(),
            "the checkbox follows view state"
        );
        handles_checkbox(&root).click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#snap-dock-toggled")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Some(true)"),
            "toggling back restores the handles"
        );
        root.remove();
    }

    async fn mount_context_actions(
        selection_kind: LayoutSelectionKind,
        pointer_tools_visible: bool,
    ) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(
                context_test_host,
                ContextHostProps {
                    selection_kind,
                    pointer_tools_visible,
                },
            ),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn contextual_tools_show_for_layout_selection_and_follow_existing_activation_scope() {
        let root = mount_context_actions(LayoutSelectionKind::Row, true).await;
        let buttons = root.query_selector_all(".layout-context-action").unwrap();
        let labels = (0..buttons.length())
            .filter_map(|index| buttons.item(index))
            .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
            .map(|button| button.text_content().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(labels, ["Row offsets", "Offset", "Stagger", "Spread"]);
        root.query_selector(
            "button[aria-label='Activate Stagger column; selection changes to Column']",
        )
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            root.query_selector("#context-action-state")
                .unwrap()
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Column:Stagger")
        );
        assert_eq!(
            root.query_selector("button[aria-label='Activate Stagger for the selected column']")
                .unwrap()
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        let stagger = root
            .query_selector("button[aria-label='Activate Stagger for the selected column']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        stagger.focus().unwrap();
        let key = web_sys::KeyboardEventInit::new();
        key.set_key("ArrowRight");
        key.set_bubbles(true);
        stagger
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &key)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .active_element()
                .unwrap()
                .get_attribute("aria-label")
                .as_deref(),
            Some("Activate Offset for the selected column"),
            "toolbar keyboard navigation must follow the updated selection's action order"
        );
        root.remove();

        let pcb = mount_context_actions(LayoutSelectionKind::Matrix, false).await;
        assert!(
            pcb.query_selector(".layout-context-actions")
                .unwrap()
                .is_none(),
            "PCB must not inherit Layout-only contextual quick actions"
        );
        pcb.remove();
    }

    #[wasm_bindgen_test]
    async fn all_transform_property_shortcuts_route_to_properties_before_selection_changes_owner() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(shortcut_test_host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;

        for (label, expected) in [
            ("Position & rotation", "true:0"),
            ("Splay & origin", "true:1"),
            ("Row offsets", "true:1"),
        ] {
            let button = root.query_selector_all("button").unwrap();
            let target = (0..button.length())
                .filter_map(|index| button.item(index))
                .find(|element| {
                    element
                        .text_content()
                        .is_some_and(|text| text.trim() == label)
                })
                .expect("transform shortcut button exists")
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            target.click();
            gloo_timers::future::TimeoutFuture::new(40).await;
            assert_eq!(
                root.query_selector("#shortcut-probe-state")
                    .unwrap()
                    .unwrap()
                    .text_content()
                    .as_deref(),
                Some(expected),
                "{label} must select Properties before any selection transition"
            );
            root.query_selector("#reset-shortcut-probe")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap()
                .click();
            gloo_timers::future::TimeoutFuture::new(40).await;
        }

        root.remove();
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod nudge_tests {
    use super::*;
    use crate::matrix_transform_operation::{
        PlacementParameter, PlacementValue, placement_operation,
    };
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_core::model::Mirror;
    use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    const TRANSACTION: &str = "layout-transform-test";

    fn nudge_edit(runtime: &Rc<Runtime>, key: ArrowKey, big: bool) -> PendingEdits<()> {
        nudge_with(
            runtime,
            TransformNudge::Stagger {
                column: 0,
                key,
                big,
            },
        )
    }

    fn nudge_with(runtime: &Rc<Runtime>, nudge: TransformNudge) -> PendingEdits<()> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "layout-transform-nudge",
            Some("transform".into()),
            transform_nudge_resolver(fixture::MATRIX_ID.into(), nudge, Vec2::default()),
        );
        edits
    }

    fn stagger_result(column: u32, along: f64) -> TransformDragResult {
        TransformDragResult::Placement {
            parameter: PlacementParameter::Stagger { column },
            delta: PlacementDelta::Stagger(along),
        }
    }

    #[wasm_bindgen_test]
    async fn column_angle_nudges_compose_without_overwriting_splay() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(1, Default::default);
        document.matrices[0].column_layout[0].splay = 7.0;
        document.matrices[0].column_layout[0].rotation = 10.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let nudge = TransformNudge::SplayAngle {
            column: 0,
            degrees: 1.0,
            affect: MatrixSplayAffect::Column,
        };
        let mut first = nudge_with(&runtime, nudge.clone());
        settle_edit(&runtime, &mut first).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            11.0
        );
        let mut second = nudge_with(&runtime, nudge);
        settle_edit(&runtime, &mut second).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].rotation, 12.0);
        assert_eq!(matrix.column_layout[0].splay, 7.0);
        let mut following = nudge_with(
            &runtime,
            TransformNudge::SplayAngle {
                column: 0,
                degrees: 1.0,
                affect: MatrixSplayAffect::Following,
            },
        );
        settle_edit(&runtime, &mut following).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].splay, 8.0);
        assert_eq!(matrix.column_layout[0].rotation, 12.0);
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].splay,
            7.0
        );
    }

    #[wasm_bindgen_test]
    async fn column_angle_drag_delta_composes_with_the_latest_rotation() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(1, Default::default);
        document.matrices[0].column_layout[0].splay = 7.0;
        document.matrices[0].column_layout[0].rotation = 10.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            TransformGesture::Splay {
                column: 0,
                start_angle: 0.0,
                origin: Vec2::default(),
                swing: false,
            },
        );
        let operation = EditOperation::SetMatrixSplay {
            matrix_id: fixture::MATRIX_ID.into(),
            column: 0,
            change: MatrixSplayChange::Angle {
                angle: 12.0,
                affect: MatrixSplayAffect::Column,
            },
        };
        let result = transform_drag_result(&drag, &operation).unwrap();
        assert_eq!(
            result,
            TransformDragResult::SplayAngle {
                column: 0,
                delta_degrees: 2.0,
                affect: MatrixSplayAffect::Column,
            }
        );
        assert!(same_transform_as_start(
            &drag,
            &EditOperation::SetMatrixSplay {
                matrix_id: fixture::MATRIX_ID.into(),
                column: 0,
                change: MatrixSplayChange::Angle {
                    angle: 10.0,
                    affect: MatrixSplayAffect::Column
                },
            }
        ));
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut other = nudge_with(
            &runtime,
            TransformNudge::SplayAngle {
                column: 0,
                degrees: 1.0,
                affect: MatrixSplayAffect::Column,
            },
        );
        support::drive_pending(&runtime);
        entered.await.expect("the nudge reached Core");
        let mut queued = drag_edit(&runtime, fixture::MATRIX_ID, result);
        release.send(()).unwrap();
        settle_edit(&runtime, &mut other).await;
        settle_edit(&runtime, &mut queued).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].rotation, 13.0);
        assert_eq!(matrix.column_layout[0].splay, 7.0);
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            11.0
        );
        redo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[0].rotation,
            13.0
        );
    }

    /// The first terminal result of an edit, for tests that care how it ended.
    async fn settle_outcome(
        runtime: &Rc<Runtime>,
        edits: &mut PendingEdits<()>,
    ) -> PendingEditResult<()> {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if let Some(result) = edits.settle(true).into_iter().find(|result| {
                matches!(
                    result,
                    PendingEditResult::Landed { .. }
                        | PendingEditResult::Failed { .. }
                        | PendingEditResult::Retired { .. }
                )
            }) {
                return result;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("transform edit did not settle");
    }

    fn drag_edit(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        result: TransformDragResult,
    ) -> PendingEdits<()> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "layout-transform-drag",
            Some("transform".into()),
            transform_drag_resolver(matrix_id.into(), TRANSACTION.into(), result),
        );
        edits
    }

    /// An absolute placement edit, as the Inspector sends it.
    fn placement_edit(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        parameter: PlacementParameter,
        value: PlacementValue,
    ) -> PendingEdits<()> {
        let matrix_id = matrix_id.to_owned();
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            (),
            "inspector-placement",
            Some("transform".into()),
            EditResolver::new("inspector-placement", move |_: &AcceptedSnapshot| {
                transform_commit(
                    String::new(),
                    &matrix_id,
                    placement_operation(&matrix_id, parameter, value),
                )
            }),
        );
        edits
    }

    async fn settle_edit(runtime: &Rc<Runtime>, edits: &mut PendingEdits<()>) {
        for _ in 0..100 {
            support::run_pending(runtime).await;
            if edits.settle(true).into_iter().any(|result| {
                matches!(
                    result,
                    PendingEditResult::Landed { .. }
                        | PendingEditResult::Failed { .. }
                        | PendingEditResult::Retired { .. }
                )
            }) {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("transform edit did not settle");
    }

    async fn redo(runtime: &Rc<Runtime>) {
        runtime.submit(Event::Redo {
            operation_id: runtime.operation(),
        });
        support::run_pending(runtime).await;
    }

    fn matrix_named(runtime: &Runtime, id: &str) -> Matrix {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == id)
            .cloned()
            .expect("the matrix exists")
    }

    fn matrix_and_poses(runtime: &Rc<Runtime>) -> (Matrix, Vec<(String, Vec2, f64)>) {
        let accepted = runtime.model().accepted.expect("an accepted document");
        let mut poses = accepted
            .document
            .parts
            .iter()
            .map(|part| (part.id.clone(), part.pose.at, part.pose.rotation))
            .collect::<Vec<_>>();
        poses.sort_by(|a, b| a.0.cmp(&b.0));
        (fixture::accepted_matrix(runtime), poses)
    }

    fn revision(runtime: &Runtime) -> u64 {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision
    }

    /// A drag as pointer-down would record it, for the matrix as it is accepted now.
    fn begin_drag(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        column: u32,
        quick_offset: bool,
    ) -> TransformDrag {
        begin_gesture(
            runtime,
            matrix_id,
            TransformGesture::Stagger {
                column,
                quick_offset,
            },
        )
    }

    fn begin_gesture(
        runtime: &Rc<Runtime>,
        matrix_id: &str,
        gesture: TransformGesture,
    ) -> TransformDrag {
        let column = match gesture {
            TransformGesture::Stagger { column, .. } => column,
            TransformGesture::Offset {
                parameter: PlacementParameter::ColumnOffset { column },
            } => column,
            _ => 0,
        };
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let scene = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix_id)
            .expect("the matrix has a scene");
        let capture = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| {
                document
                    .create_element_ns(Some("http://www.w3.org/2000/svg"), "g")
                    .ok()
            })
            .and_then(|element| element.dyn_into::<SvgElement>().ok())
            .expect("an SVG element");
        TransformDrag {
            pointer_id: 1,
            capture,
            matrix: Rc::new(matrix_named(runtime, matrix_id)),
            gesture,
            transaction_id: TRANSACTION.into(),
            owner: LayoutOwnerIdentity {
                scope: runtime.scope(),
                token: Some(accepted.token),
                revision: Some(accepted.document.revision),
                generation: 0,
                workspace: "Layout",
            },
            context: TreeContext::Column {
                matrix_id: matrix_id.into(),
                column,
            },
            tool: LayoutTransformTool::Stagger,
            basis: gesture
                .parameter()
                .and_then(|parameter| SceneBasis::for_parameter(scene, parameter)),
            origin_base: None,
            start_origin: None,
            start_client_x: 0.0,
            start_client_y: 0.0,
            pending: None,
            preview_submitted: false,
        }
    }

    fn preview(runtime: &Rc<Runtime>, drag: &mut TransformDrag, world: Vec2) -> EditOperation {
        let operation = placement_preview(drag, world, None).expect("a preview");
        submit_transform_edit(runtime, drag, operation.clone());
        drag.preview_submitted = true;
        operation
    }

    async fn commit(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: &EditOperation) {
        let result = transform_drag_result(drag, operation).expect("a drag result");
        let mut edits = drag_edit(runtime, &drag.matrix.id, result);
        settle_edit(runtime, &mut edits).await;
    }

    fn offset_of(parameter: PlacementParameter) -> TransformGesture {
        TransformGesture::Offset { parameter }
    }

    #[wasm_bindgen_test]
    async fn an_offset_drag_moves_one_column_by_the_pointer_delta() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            offset_of(PlacementParameter::ColumnOffset { column: 0 }),
        );
        let last = preview(&runtime, &mut drag, Vec2 { x: 2.0, y: 1.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.first().map(|record| &record.offset),
            Some(&Vec2 { x: 2.0, y: 1.0 })
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
        assert!(
            matrix
                .row_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
    }

    #[wasm_bindgen_test]
    async fn a_row_offset_drag_and_nudge_change_only_that_rows_offset() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rows = 2;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let row = PlacementParameter::RowOffset { row: 1 };
        let mut drag = begin_gesture(&runtime, fixture::MATRIX_ID, offset_of(row));
        let last = preview(&runtime, &mut drag, Vec2 { x: 1.0, y: 2.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.row_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 { x: 1.0, y: 2.0 })
        );

        let mut nudge = nudge_with(
            &runtime,
            TransformNudge::Offset {
                parameter: row,
                key: ArrowKey::Up,
                big: false,
            },
        );
        settle_edit(&runtime, &mut nudge).await;
        let matrix = fixture::accepted_matrix(&runtime);
        let offset = matrix.row_layout[1].offset;
        assert!(
            (offset.x - 1.0).abs() < 1e-9 && (offset.y - 2.1).abs() < 1e-9,
            "ArrowUp raises the row by 0.1 mm: {offset:?}"
        );
        assert_eq!(
            matrix
                .row_layout
                .first()
                .map(|record| &record.offset)
                .copied()
                .unwrap_or_default(),
            Vec2::default()
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
    }

    #[wasm_bindgen_test]
    async fn a_column_offset_on_a_splayed_rotated_matrix_follows_the_pointer_in_world_space() {
        let mut document = fixture::matrix_document();
        document.matrices[0].rotation = Some(30.0);
        {
            let values = vec![0.0, 20.0, 0.0];
            let count = values.len().max(document.matrices[0].column_layout.len());
            document.matrices[0]
                .column_layout
                .resize_with(count, Default::default);
            for record in &mut document.matrices[0].column_layout {
                record.splay = 0.0;
            }
            for (record, value) in document.matrices[0].column_layout.iter_mut().zip(values) {
                record.splay = value;
            }
        }
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        // The first edit syncs the fixture's unlaid poses; the drag is measured after it.
        let mut sync = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::ColumnOffset { column: 2 },
            PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
        );
        settle_edit(&runtime, &mut sync).await;
        let (before_matrix, before) = matrix_and_poses(&runtime);

        let mut drag = begin_gesture(
            &runtime,
            fixture::MATRIX_ID,
            offset_of(PlacementParameter::ColumnOffset { column: 1 }),
        );
        let world = Vec2 { x: 3.0, y: 2.0 };
        let last = preview(&runtime, &mut drag, world);
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let (after_matrix, after) = matrix_and_poses(&runtime);
        for ((id, from, _), (_, to, _)) in before.iter().zip(&after) {
            let expected = if id == "matrix/matrix/r0c1" {
                Vec2 {
                    x: world.x,
                    y: world.y,
                }
            } else {
                Vec2::default()
            };
            assert!(
                (to.x - from.x - expected.x).abs() < 1e-6
                    && (to.y - from.y - expected.y).abs() < 1e-6,
                "{id} moved by ({}, {}), expected {expected:?}",
                to.x - from.x,
                to.y - from.y
            );
        }
        assert_eq!(
            after_matrix
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            before_matrix
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            after_matrix
                .row_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>(),
            before_matrix
                .row_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>()
        );
    }

    #[wasm_bindgen_test]
    async fn a_drag_for_a_column_or_row_that_has_gone_is_refused_before_core() {
        let runtime = fixture::open_matrix_runtime().await;
        let revision_before = revision(&runtime);
        for parameter in [
            PlacementParameter::ColumnOffset { column: 9 },
            PlacementParameter::Stagger { column: 9 },
            PlacementParameter::RowOffset { row: 5 },
        ] {
            let delta = match parameter {
                PlacementParameter::Stagger { .. } => PlacementDelta::Stagger(1.0),
                _ => PlacementDelta::Offset(Vec2 { x: 1.0, y: 0.0 }),
            };
            let mut edits = drag_edit(
                &runtime,
                fixture::MATRIX_ID,
                TransformDragResult::Placement { parameter, delta },
            );
            let outcome = settle_outcome(&runtime, &mut edits).await;
            assert!(
                matches!(
                    &outcome,
                    PendingEditResult::Failed { message, .. } if message.contains("no longer exists")
                ),
                "{parameter:?} must be refused by the resolver, got {outcome:?}"
            );
        }
        assert_eq!(revision(&runtime), revision_before, "nothing was written");
    }

    #[wasm_bindgen_test]
    async fn a_stagger_snaps_to_the_grid_step_of_its_axis_dominant_component() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        let step = Some(Vec2 { x: 4.0, y: 9.0 });
        let value = |drag: &TransformDrag, world: Vec2| {
            let Some(EditOperation::SetMatrixPlacement {
                change: MatrixPlacementChange::ColumnStagger { value, .. },
                ..
            }) = placement_preview(drag, world, step)
            else {
                panic!("a ColumnStagger preview");
            };
            value
        };
        assert!(
            (value(&drag, Vec2 { x: 0.0, y: 5.0 }) - 9.0).abs() < 1e-9,
            "an upright axis snaps to the y step"
        );
        drag.basis = Some(SceneBasis {
            stagger_axis: Vec2 { x: 1.0, y: 0.0 },
            axis_x: Vec2 { x: 1.0, y: 0.0 },
            axis_y: Vec2 { x: 0.0, y: 1.0 },
            shift_axis_x: Vec2 { x: 1.0, y: 0.0 },
            shift_axis_y: Vec2 { x: 0.0, y: 1.0 },
            spread_axis: None,
        });
        assert!(
            (value(&drag, Vec2 { x: 5.0, y: 0.0 }) - 4.0).abs() < 1e-9,
            "a horizontal axis snaps to the x step"
        );
    }

    #[wasm_bindgen_test]
    async fn three_handle_nudges_queued_behind_a_gated_reply_move_the_matrix_three_steps() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut first = nudge_edit(&runtime, ArrowKey::Up, false);
        support::drive_pending(&runtime);
        entered.await.expect("the first nudge reached Core");
        let mut second = nudge_edit(&runtime, ArrowKey::Up, false);
        let mut third = nudge_edit(&runtime, ArrowKey::Up, false);
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        settle_edit(&runtime, &mut third).await;
        settle_edit(&runtime, &mut first).await;
        settle_edit(&runtime, &mut second).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            (matrix.column_layout[0].stagger - 0.3).abs() < 1e-9,
            "three steps of 0.1 compose to a stagger of 0.3, got {}",
            matrix.column_layout[0].stagger
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default()),
            "the offsets are untouched"
        );
    }

    #[wasm_bindgen_test]
    async fn a_drag_commit_applies_its_net_transform_to_the_accepted_matrix() {
        let runtime = fixture::open_matrix_runtime().await;
        // Another stagger edit lands while the drag was in flight; the drag's net change
        // composes with it instead of restoring the matrix the drag started from.
        let mut other = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::Stagger { column: 0 },
            PlacementValue::Stagger(1.0),
        );
        settle_edit(&runtime, &mut other).await;
        let mut drag = drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(0, 2.0));
        settle_edit(&runtime, &mut drag).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(matrix.column_layout[0].stagger, 3.0);
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.offset)
                .all(|offset| *offset == Vec2::default())
        );
    }

    #[wasm_bindgen_test]
    async fn cancel_and_escape_land_nothing() {
        let runtime = fixture::open_matrix_runtime().await;
        let mut settled = nudge_edit(&runtime, ArrowKey::Up, true);
        settle_edit(&runtime, &mut settled).await;
        let before = matrix_and_poses(&runtime);
        let revision_before = revision(&runtime);

        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 3.0 });
        support::run_pending(&runtime).await;
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing"
        );
        let arbiter = CanvasInteractionArbiter::default();
        assert!(arbiter.try_acquire(CanvasInteractionOwner::MatrixTransform));
        // Escape, pointer cancel and lost capture all end in this.
        let cell = Rc::new(RefCell::new(Some(drag)));
        assert!(abandon_transform_drag(&runtime, &cell, &arbiter));
        support::run_pending(&runtime).await;

        assert!(runtime.model().display_preview.is_none());
        assert_eq!(arbiter.current(), None, "the canvas is released");
        assert_eq!(revision(&runtime), revision_before);
        assert_eq!(matrix_and_poses(&runtime), before);
        // The Undo depth is unchanged: one Undo reverts only the earlier nudge.
        fixture::undo(&runtime).await;
        assert!(
            fixture::accepted_matrix(&runtime)
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0),
            "the cancelled drag left no Undo step behind"
        );
    }

    #[wasm_bindgen_test]
    async fn one_drag_is_one_undo_step_and_redo_restores_it() {
        let runtime = fixture::open_matrix_runtime().await;
        let before = matrix_and_poses(&runtime);
        let revision_before = revision(&runtime);
        let mut drag = begin_drag(&runtime, fixture::MATRIX_ID, 1, false);
        preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 1.0 });
        support::run_pending(&runtime).await;
        let last = preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 4.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let after = matrix_and_poses(&runtime);
        assert_eq!(
            after.0.column_layout.get(1).map(|record| &record.stagger),
            Some(&4.0)
        );
        assert_eq!(revision(&runtime), revision_before + 1, "one accepted edit");

        fixture::undo(&runtime).await;
        assert_eq!(
            matrix_and_poses(&runtime)
                .0
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            before
                .0
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        redo(&runtime).await;
        assert_eq!(matrix_and_poses(&runtime), after, "Redo restores the drag");
    }

    #[wasm_bindgen_test]
    async fn a_drag_composes_with_an_unrelated_edit_held_in_flight() {
        let runtime = fixture::open_matrix_runtime().await;
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let mut unrelated = placement_edit(
            &runtime,
            fixture::MATRIX_ID,
            PlacementParameter::ColumnOffset { column: 2 },
            PlacementValue::Offset(Vec2 { x: 1.0, y: 0.0 }),
        );
        support::drive_pending(&runtime);
        entered.await.expect("the unrelated edit reached Core");
        let mut drag = drag_edit(&runtime, fixture::MATRIX_ID, stagger_result(1, 2.0));
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        settle_edit(&runtime, &mut drag).await;
        settle_edit(&runtime, &mut unrelated).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.stagger),
            Some(&2.0)
        );
        assert_eq!(
            matrix.column_layout.get(2).map(|record| &record.offset),
            Some(&Vec2 { x: 1.0, y: 0.0 })
        );
    }

    #[wasm_bindgen_test]
    async fn arrow_up_moves_the_column_up_on_screen_on_a_mirror_y_matrix() {
        let mut document = fixture::matrix_document();
        document.matrices[0].mirror = Some(Mirror::Y);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let mut up = nudge_edit(&runtime, ArrowKey::Up, false);
        settle_edit(&runtime, &mut up).await;
        let accepted = runtime.model().accepted.expect("an accepted document");
        let key_y = accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "matrix/matrix/r0c1")
            .expect("the second key")
            .pose
            .at
            .y;
        assert!(
            (key_y - 0.1).abs() < 1e-9,
            "ArrowUp raises the column by 0.1 mm on screen, got {key_y}"
        );
        assert!(
            fixture::accepted_matrix(&runtime).column_layout[0].stagger < 0.0,
            "the stagger value falls because the matrix is mirrored"
        );
        // The pair across the stagger axis is not consumed.
        let scene = &accepted.scene.matrix_scenes[0];
        let basis = SceneBasis::for_parameter(scene, PlacementParameter::Stagger { column: 0 })
            .expect("column 0 has a basis");
        assert_eq!(
            nudge_value(
                &basis,
                PlacementParameter::Stagger { column: 0 },
                PlacementValue::Stagger(0.0),
                ArrowKey::Left,
                false,
            ),
            None
        );
    }

    #[wasm_bindgen_test]
    async fn a_drag_on_the_right_half_of_a_linked_pair_updates_both_halves_equally() {
        let mut document = fixture::matrix_document();
        document.matrices.clear();
        document.parts.clear();
        document.boards[0].part_ids.clear();
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let operation: EditOperation = serde_json::from_value(serde_json::json!({
            "kind": "create-mirrored-pair",
            "matrix": {
                "id": "left-matrix", "rows": 1, "columns": 3,
                "pitch": {"x": 19.05, "y": 19.05}, "origin": {"x": -20.0, "y": 0.0},
                "mirror": "x", "definitionId": "switch:base",
                "boardId": fixture::BOARD_ID, "partIds": []
            },
            "left": {"id": "left-layout", "name": "Left", "boardId": fixture::BOARD_ID,
                     "matrixIds": ["left-matrix"], "partIds": []},
            "right": {"id": "right-layout", "name": "Right", "boardId": fixture::BOARD_ID,
                      "matrixIds": ["right-matrix"], "partIds": [],
                      "mirrorLink": {"sourceId": "left-layout", "axisX": 10.0}}
        }))
        .expect("a mirrored pair operation");
        let mut pair = PendingEdits::default();
        pair.begin(
            &runtime,
            (),
            "pair",
            Some("pair".into()),
            EditResolver::new("pair", move |_: &AcceptedSnapshot| {
                Resolution::submit(vec![], operation.clone())
            }),
        );
        settle_edit(&runtime, &mut pair).await;

        let mut drag = begin_drag(&runtime, "right-matrix", 1, false);
        let last = preview(&runtime, &mut drag, Vec2 { x: 0.0, y: 2.0 });
        support::run_pending(&runtime).await;
        commit(&runtime, &drag, &last).await;
        let (left, right) = (
            matrix_named(&runtime, "left-matrix"),
            matrix_named(&runtime, "right-matrix"),
        );
        assert_eq!(
            right.column_layout.get(1).map(|record| &record.stagger),
            Some(&2.0)
        );
        assert_eq!(
            left.column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>(),
            right
                .column_layout
                .iter()
                .map(|record| record.stagger)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            left.column_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>(),
            right
                .column_layout
                .iter()
                .map(|record| record.offset)
                .collect::<Vec<_>>()
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tool_tests {
    use super::*;
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_application::Scope;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn context_of(kind: &str) -> TreeContext {
        let matrix_id = "matrix".to_owned();
        match kind {
            "row" => TreeContext::Row { matrix_id, row: 1 },
            "column" => TreeContext::Column {
                matrix_id,
                column: 2,
            },
            "key" => TreeContext::Key {
                matrix_id,
                row: 1,
                column: 2,
            },
            _ => TreeContext::Matrix { matrix_id },
        }
    }

    #[wasm_bindgen_test]
    fn activation_changes_the_selection_kind_per_tool() {
        use LayoutSelectionKind::{Column, Row};
        use LayoutTransformTool::{Move, Offset, Origin, Rotate, Splay, Stagger};
        for (tool, selection, expected) in [
            // Offset keeps a Row and turns the rest into their Column.
            (Offset, "row", Row),
            (Offset, "column", Column),
            (Offset, "key", Column),
            (Offset, "matrix", Column),
            // Stagger is a column parameter, so a Row asks for a Column selection. Which
            // column it resolves to (the retained key's, or column 1) is the selection
            // conversion's rule (`context_for_selection_kind`), not covered here.
            (Stagger, "row", Column),
            (Stagger, "column", Column),
            (Stagger, "key", Column),
            (Splay, "row", Column),
            (Origin, "key", Column),
        ] {
            let activation = activate_transform_tool(None, tool, &context_of(selection));
            assert_eq!(
                activation,
                ToolActivation {
                    active: Some(tool),
                    selection_kind: Some(expected),
                },
                "{tool:?} on a {selection} selection"
            );
        }
        // Move and Rotate edit the matrix itself, so every selection stays a
        // Matrix selection: no selection change is requested.
        for tool in [Move, Rotate] {
            for selection in ["row", "column", "key", "matrix"] {
                assert_eq!(
                    activate_transform_tool(None, tool, &context_of(selection)),
                    ToolActivation {
                        active: Some(tool),
                        selection_kind: None,
                    },
                    "{tool:?} on a {selection} selection"
                );
            }
        }
        // Choosing the active tool again turns it off and leaves the selection alone.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Offset),
                LayoutTransformTool::Offset,
                &context_of("row")
            ),
            ToolActivation {
                active: None,
                selection_kind: None
            }
        );
        // Switching tool replaces the active one.
        assert_eq!(
            activate_transform_tool(
                Some(LayoutTransformTool::Stagger),
                LayoutTransformTool::Offset,
                &context_of("row")
            )
            .active,
            Some(LayoutTransformTool::Offset)
        );
    }

    #[derive(Props, Clone, PartialEq)]
    struct MenuHostProps {
        available: bool,
    }

    #[component]
    fn menu_host(props: MenuHostProps) -> Element {
        let open_menu = use_signal(|| Some(LayoutCommandMenu::Transform));
        let mut chosen = use_signal(String::new);
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: true,
            pointer_tools_available: props.available,
            active_tool: Some(LayoutTransformTool::Offset),
            matrix_rotate_followed: None,
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool: EventHandler::new(move |tool: LayoutTransformTool| {
                chosen.set(format!("{tool:?}"));
            }),
        };
        rsx! {
            LayoutTransformToolbar { mount, open_menu }
            output { id: "chosen-tool", "{chosen()}" }
        }
    }

    async fn mount_menu(available: bool) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(menu_host, MenuHostProps { available }),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn transform_popover_preserves_escape_and_outside_focus() {
        let root = mount_menu(true).await;
        let document = web_sys::window().unwrap().document().unwrap();
        let trigger = root
            .query_selector("#m1-layout-transform-trigger")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let content = root
            .query_selector("#m1-layout-transform-menu")
            .unwrap()
            .unwrap();
        assert_eq!(content.get_attribute("role").as_deref(), Some("group"));
        let label_id = content.get_attribute("aria-labelledby").unwrap();
        assert_eq!(
            document
                .get_element_by_id(&label_id)
                .unwrap()
                .text_content()
                .as_deref(),
            Some("Transform tools and properties")
        );
        let escape = web_sys::KeyboardEventInit::new();
        escape.set_key("Escape");
        escape.set_bubbles(true);
        content
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                    .unwrap(),
            )
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(document.active_element().unwrap().id(), trigger.id());

        trigger.click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        let outside = document.create_element("button").unwrap();
        outside.set_id("transform-outside-target");
        root.append_child(&outside).unwrap();
        outside
            .dyn_ref::<web_sys::HtmlElement>()
            .unwrap()
            .focus()
            .unwrap();
        gloo_timers::future::TimeoutFuture::new(80).await;
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            document.active_element().unwrap().id(),
            "transform-outside-target"
        );
        root.remove();
    }

    #[derive(Props, Clone, PartialEq)]
    struct MatrixMenuHostProps {
        followed: bool,
    }

    #[component]
    fn matrix_menu_host(props: MatrixMenuHostProps) -> Element {
        let mount = LayoutTransformMenuMount {
            properties_available: true,
            column_available: true,
            row_available: true,
            pointer_tools_visible: true,
            pointer_tools_available: true,
            active_tool: None,
            matrix_rotate_followed: Some(props.followed),
            on_selection_kind: EventHandler::new(|_: LayoutSelectionKind| {}),
            on_show_properties: EventHandler::new(|()| {}),
            on_transform_tool: EventHandler::new(|_: LayoutTransformTool| {}),
        };
        rsx! {
            LayoutTransformToolbar {
                mount,
                open_menu: Signal::new(Some(LayoutCommandMenu::Transform)),
                selection_kind: Some(LayoutSelectionKind::Matrix),
            }
        }
    }

    /// Ticket 23 slice: the Matrix menu offers Move always, and Rotate
    /// enabled only when the matrix follows rotation; otherwise Rotate stays
    /// visible but disabled with the reason.
    #[wasm_bindgen_test]
    async fn the_matrix_menu_disables_rotate_without_follow_rotation() {
        for (followed, rotate_enabled) in [(true, true), (false, false)] {
            let document = web_sys::window().unwrap().document().unwrap();
            let root = document.create_element("div").unwrap();
            document.body().unwrap().append_child(&root).unwrap();
            dioxus_web::launch::launch_virtual_dom(
                VirtualDom::new_with_props(matrix_menu_host, MatrixMenuHostProps { followed }),
                dioxus_web::Config::new().rootnode(root.clone().into()),
            );
            gloo_timers::future::TimeoutFuture::new(40).await;
            let buttons = root.query_selector_all("button").unwrap();
            let labels = (0..buttons.length())
                .filter_map(|index| buttons.item(index))
                .filter_map(|node| node.text_content())
                .collect::<Vec<_>>();
            assert!(
                labels.iter().any(|label| label == "Move"),
                "the Matrix menu offers Move, got {labels:?}"
            );
            let rotate = (0..buttons.length())
                .filter_map(|index| buttons.item(index))
                .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
                .find(|button| button.text_content().as_deref() == Some("Rotate"))
                .expect("the Matrix menu offers Rotate");
            assert_eq!(
                rotate.has_attribute("disabled"),
                !rotate_enabled,
                "Rotate follows the rotation Follow"
            );
            assert_eq!(
                rotate.get_attribute("aria-label").as_deref(),
                Some(if rotate_enabled {
                    "Activate Rotate for the selected matrix"
                } else {
                    "Rotate matrix (unavailable: Follow rotation is off)"
                }),
                "the Rotate entry explains itself"
            );
            root.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn the_menu_lists_stagger_offset_splay_origin_and_follows_availability() {
        for available in [true, false] {
            let root = mount_menu(available).await;
            let buttons = root.query_selector_all("button").unwrap();
            let tools = (0..buttons.length())
                .filter_map(|index| buttons.item(index))
                .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
                .filter(|button| {
                    matches!(
                        button.text_content().as_deref(),
                        Some("Stagger" | "Spread" | "Offset" | "Splay" | "Origin")
                    )
                })
                .collect::<Vec<_>>();
            let labels = tools
                .iter()
                .map(|button| button.text_content().unwrap_or_default())
                .collect::<Vec<_>>();
            assert_eq!(labels, ["Stagger", "Spread", "Offset", "Splay", "Origin"]);
            for button in &tools {
                assert_eq!(button.has_attribute("disabled"), !available);
                let pressed = button.get_attribute("aria-pressed");
                assert_eq!(
                    pressed.as_deref(),
                    Some(if button.text_content().as_deref() == Some("Offset") {
                        "true"
                    } else {
                        "false"
                    })
                );
            }
            if available {
                tools
                    .iter()
                    .find(|button| button.text_content().as_deref() == Some("Offset"))
                    .expect("the Offset tool stays listed")
                    .clone()
                    .dyn_into::<web_sys::HtmlElement>()
                    .unwrap()
                    .click();
                gloo_timers::future::TimeoutFuture::new(40).await;
                assert_eq!(
                    root.query_selector("#chosen-tool")
                        .unwrap()
                        .unwrap()
                        .text_content()
                        .as_deref(),
                    Some("Offset")
                );
            }
            root.remove();
        }
    }

    #[derive(Props, Clone, PartialEq)]
    struct OverlayHostProps {
        runtime: LayoutTransformRuntime,
        svg: LayoutTransformSvg,
        matrix: Matrix,
        projection: MatrixScene,
        owner: LayoutOwnerIdentity,
        scope: Scope,
        tool: LayoutTransformTool,
        context: TreeContext,
    }

    #[component]
    fn overlay_host(props: OverlayHostProps) -> Element {
        let context = props.context.clone();
        let selected_context = use_signal(|| {
            Some(ScopedTreeContext {
                scope: props.scope.clone(),
                context: context.clone(),
            })
        });
        let scope_generation = use_signal(|| props.owner.generation);
        let workspace = use_signal(|| "Layout");
        let splay_affect = use_signal(|| MatrixSplayAffect::Column);
        let arbiter = use_hook(CanvasInteractionArbiter::default);
        rsx! {
            svg { id: "overlay-host-svg", view_box: "0 0 100 100", width: "400", height: "400",
                LayoutTransformToolOverlay {
                    runtime: props.runtime.clone(),
                    svg: props.svg.clone(),
                    arbiter,
                    owner: props.owner.clone(),
                    selected_context,
                    scope_generation,
                    workspace,
                    matrix: props.matrix.clone(),
                    projection: props.projection.clone(),
                    context: props.context.clone(),
                    tool: props.tool,
                    snap_settings: super::super::layout_toolbar::LayoutSnapSettings::default(),
                    snap_origins: Vec::new(),
                    splay_affect,
                    view_x: 0.0,
                    view_y: 0.0,
                    width: 100.0,
                    height: 100.0,
                    on_finish: EventHandler::new(|()| {}),
                }
            }
        }
    }

    /// Mount the overlay for the runtime's accepted matrix as it is now.
    async fn mount_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
    ) -> (web_sys::Element, web_sys::Element) {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let matrix = fixture::accepted_matrix(runtime);
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
            .cloned()
            .expect("the matrix scene");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let context = TreeContext::Column {
            matrix_id: matrix.id.clone(),
            column: 1,
        };
        let props = OverlayHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            matrix,
            projection,
            scope: owner.scope.clone().expect("a scope"),
            owner,
            tool,
            context,
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(overlay_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        let handle = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the tool draws a handle");
        // A synthetic pointer id is not an active pointer, so the browser refuses real capture.
        // The handle's capture methods are stubbed on this element; everything else is the
        // production pointer path.
        let stub = js_sys::Function::new_no_args("return true;");
        for method in [
            "setPointerCapture",
            "hasPointerCapture",
            "releasePointerCapture",
        ] {
            js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
        }
        (root, handle)
    }

    /// Mount the overlay for a named matrix with a Matrix selection, for the
    /// matrix Move and Rotate tools.
    async fn mount_matrix_overlay(
        runtime: &Rc<Runtime>,
        tool: LayoutTransformTool,
        matrix_id: &str,
    ) -> (web_sys::Element, web_sys::Element) {
        let model = runtime.model();
        let accepted = model.accepted.as_ref().expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == matrix_id)
            .cloned()
            .expect("the named matrix");
        let projection = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
            .cloned()
            .expect("the matrix scene");
        let owner = LayoutOwnerIdentity {
            scope: runtime.scope(),
            token: Some(accepted.token),
            revision: Some(accepted.document.revision),
            generation: 0,
            workspace: "Layout",
        };
        let svg = LayoutTransformSvg(Rc::new(RefCell::new(None)));
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let context = TreeContext::Matrix {
            matrix_id: matrix.id.clone(),
        };
        let props = OverlayHostProps {
            runtime: LayoutTransformRuntime(runtime.clone()),
            svg: svg.clone(),
            matrix,
            projection,
            scope: owner.scope.clone().expect("a scope"),
            owner,
            tool,
            context,
        };
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new_with_props(overlay_host, props),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        *svg.0.borrow_mut() = root
            .query_selector("#overlay-host-svg")
            .unwrap()
            .and_then(|element| element.dyn_into::<SvgElement>().ok());
        // Every handle needs the capture stubs: synthetic pointer events are
        // never real active pointers, so the unstubbed capture throws and the
        // drag refuses to start.
        let handles = root.query_selector_all(".m1-transform-handle").unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for index in 0..handles.length() {
            let handle = handles.item(index).expect("the stubbed handle");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
        }
        let handle = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the tool draws a handle");
        (root, handle)
    }

    fn dispatch_pointer(
        target: &web_sys::Element,
        kind: &str,
        shift: bool,
        alt: bool,
        buttons: u16,
        client: (i32, i32),
    ) {
        let init = js_sys::Object::new();
        for (name, value) in [
            ("bubbles", wasm_bindgen::JsValue::TRUE),
            ("cancelable", wasm_bindgen::JsValue::TRUE),
            ("pointerId", 17.into()),
            ("pointerType", "mouse".into()),
            ("isPrimary", wasm_bindgen::JsValue::TRUE),
            ("button", 0.into()),
            ("buttons", buttons.into()),
            ("ctrlKey", shift.into()),
            ("altKey", alt.into()),
            ("clientX", client.0.into()),
            ("clientY", client.1.into()),
        ] {
            js_sys::Reflect::set(init.as_ref(), &name.into(), &value).unwrap();
        }
        let constructor = js_sys::Reflect::get(&js_sys::global(), &"PointerEvent".into())
            .unwrap()
            .dyn_into::<js_sys::Function>()
            .unwrap();
        let arguments = js_sys::Array::new();
        arguments.push(&kind.into());
        arguments.push(init.as_ref());
        let event = js_sys::Reflect::construct(&constructor, &arguments)
            .unwrap()
            .dyn_into::<web_sys::PointerEvent>()
            .unwrap();
        target.dispatch_event(&event).unwrap();
    }

    async fn wait_until_editable(runtime: &Rc<Runtime>) {
        for _ in 0..200 {
            support::run_pending(runtime).await;
            let model = runtime.model();
            if model.lifecycle == Lifecycle::Ready
                && model.accepted.as_ref().is_some_and(|accepted| {
                    model.durability
                        == (Durability::Saved {
                            revision: accepted.document.revision,
                        })
                })
            {
                return;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        panic!("the runtime did not become editable");
    }

    /// One real pointer drag on the handle: down, move, up. Returns the handle's
    /// `data-quick-offset` after the down and after the move.
    async fn drag_handle(
        runtime: &Rc<Runtime>,
        handle: &web_sys::Element,
        shift_down: bool,
        to: (i32, i32),
    ) -> (Option<String>, Option<String>) {
        let revision = fixture_revision(runtime);
        dispatch_pointer(handle, "pointerdown", shift_down, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        let after_down = handle.get_attribute("data-quick-offset");
        // Shift is released before the first move; the latch must hold.
        dispatch_pointer(handle, "pointermove", false, false, 1, to);
        gloo_timers::future::TimeoutFuture::new(40).await;
        let after_move = handle.get_attribute("data-quick-offset");
        dispatch_pointer(handle, "pointerup", false, false, 0, to);
        for _ in 0..200 {
            support::run_pending(runtime).await;
            if fixture_revision(runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        (after_down, after_move)
    }

    fn fixture_revision(runtime: &Runtime) -> u64 {
        runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .revision
    }

    #[wasm_bindgen_test]
    async fn column_angle_pointer_preview_starts_at_rotation_and_preserves_mirror_direction() {
        for (mirror, expected) in [
            (None, 100.0),
            (Some(boardstudio_core::model::Mirror::X), -80.0),
        ] {
            let mut document = fixture::matrix_document();
            let matrix = &mut document.matrices[0];
            matrix.mirror = mirror;
            matrix.column_layout.resize_with(2, Default::default);
            matrix.column_layout[1].splay = 7.0;
            matrix.column_layout[1].rotation = 10.0;
            let runtime = support::new_runtime();
            support::open_document(&runtime, document).await;
            let before = fixture_revision(&runtime);
            let (root, _) = mount_overlay(&runtime, LayoutTransformTool::Splay).await;
            let handle = root
                .query_selector("[aria-label='Drag to splay']")
                .unwrap()
                .unwrap();
            assert_eq!(
                handle
                    .query_selector("title")
                    .unwrap()
                    .unwrap()
                    .text_content()
                    .as_deref(),
                Some("Drag to splay · 10°")
            );
            let stub = js_sys::Function::new_no_args("return true;");
            for method in [
                "setPointerCapture",
                "hasPointerCapture",
                "releasePointerCapture",
            ] {
                js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
            }
            let accepted = runtime.model().accepted.unwrap();
            let origin = accepted.scene.matrix_scenes[0].columns[1].splay_origin;
            let rect = root
                .query_selector("svg")
                .unwrap()
                .unwrap()
                .get_bounding_client_rect();
            let client = |x: f64, y: f64| {
                (
                    (rect.left() + x * 4.0).round() as i32,
                    (rect.top() - y * 4.0).round() as i32,
                )
            };
            let start = client(origin.x + 20.0, origin.y);
            let end = client(origin.x, origin.y + 20.0);
            dispatch_pointer(&handle, "pointerdown", false, false, 1, start);
            dispatch_pointer(&handle, "pointermove", false, false, 1, start);
            support::run_pending(&runtime).await;
            assert_eq!(
                fixture_revision(&runtime),
                before,
                "initial preview never commits"
            );
            assert_eq!(
                runtime
                    .model()
                    .display_preview
                    .as_ref()
                    .unwrap()
                    .matrix_scenes[0]
                    .columns[1]
                    .rotation,
                10.0,
                "zero pointer delta previews the stored Column rotation"
            );
            dispatch_pointer(&handle, "pointermove", false, false, 1, end);
            support::run_pending(&runtime).await;
            assert_eq!(
                runtime
                    .model()
                    .display_preview
                    .as_ref()
                    .unwrap()
                    .matrix_scenes[0]
                    .columns[1]
                    .rotation,
                expected,
                "the pointer preview uses the selected angle and mirror direction"
            );
            assert_eq!(
                fixture::accepted_matrix(&runtime).column_layout[1].rotation,
                10.0
            );
            dispatch_pointer(&handle, "pointerup", false, false, 0, end);
            wait_until_editable(&runtime).await;
            let matrix = fixture::accepted_matrix(&runtime);
            assert_eq!(matrix.column_layout[1].rotation, expected);
            assert_eq!(matrix.column_layout[1].splay, 7.0);
            fixture::undo(&runtime).await;
            assert_eq!(
                fixture::accepted_matrix(&runtime).column_layout[1].rotation,
                10.0
            );
            root.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn real_pointer_events_with_stubbed_capture_latch_ctrl_at_pointer_down_and_show_the_offset_glyph()
     {
        let runtime = fixture::open_matrix_runtime().await;
        // The first edit syncs the fixture's unlaid poses.
        let mut sync = PendingEdits::<()>::default();
        sync.begin(
            &runtime,
            (),
            "sync",
            Some("transform".into()),
            EditResolver::new("sync", |_: &AcceptedSnapshot| {
                transform_commit(
                    String::new(),
                    fixture::MATRIX_ID,
                    placement_operation(
                        fixture::MATRIX_ID,
                        PlacementParameter::ColumnOffset { column: 2 },
                        PlacementValue::Offset(Vec2 { x: 0.5, y: 0.0 }),
                    ),
                )
            }),
        );
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if !sync.settle(true).is_empty() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        wait_until_editable(&runtime).await;

        // Shift at pointer-down: a free 2-D column offset with the offset glyph, though Shift
        // is released before the first move. The grid step is 4.7625 mm; 4 px is 1 mm.
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        assert_eq!(
            handle.get_attribute("data-quick-offset").as_deref(),
            Some("false")
        );
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to stagger")
        );
        let (down, moved) = drag_handle(&runtime, &handle, true, (140, 80)).await;
        assert_eq!(
            down.as_deref(),
            Some("true"),
            "Shift at pointer-down shows the glyph"
        );
        assert_eq!(
            moved.as_deref(),
            Some("true"),
            "releasing Shift does not switch it"
        );
        let matrix = fixture::accepted_matrix(&runtime);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 {
                x: 9.525,
                y: 4.7625
            })
        );
        assert!(
            matrix
                .column_layout
                .iter()
                .map(|record| &record.stagger)
                .all(|stagger| *stagger == 0.0)
        );
        gloo_timers::future::TimeoutFuture::new(60).await;
        assert_eq!(
            handle.get_attribute("data-quick-offset").as_deref(),
            Some("false"),
            "the glyph returns when the drag ends"
        );
        root.remove();
        wait_until_editable(&runtime).await;

        // No Shift: the same gesture is a one-dimensional stagger.
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let (down, moved) = drag_handle(&runtime, &handle, false, (140, 60)).await;
        assert_eq!(
            (down.as_deref(), moved.as_deref()),
            (Some("false"), Some("false"))
        );
        let matrix = fixture::accepted_matrix(&runtime);
        assert!((matrix.column_layout[1].stagger - 9.525).abs() < 1e-9);
        assert_eq!(
            matrix.column_layout.get(1).map(|record| &record.offset),
            Some(&Vec2 {
                x: 9.525,
                y: 4.7625
            }),
            "the offset is untouched"
        );
        root.remove();
    }

    /// Ticket 23, RED: the Spread tool draws its own horizontal handle that
    /// commits an explicit Spread through the real Runtime/Core seam.
    /// A 40 px drag is 10 world mm; the 4.7625 mm grid lands six steps out,
    /// so column 1 steps 28.575 mm and column 2 follows.
    #[wasm_bindgen_test]
    async fn spread_handle_drag_commits_an_explicit_spread_with_one_undo() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Spread).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to spread"),
            "the Spread tool draws its own horizontal handle"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag previews actual later-column movement before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            matrix
                .column_layout
                .get(1)
                .and_then(|record| record.spread)
                .is_some_and(|spread| (spread - 28.575).abs() < 1e-9),
            "the drag commits an explicit Spread"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        let scene = runtime.model().accepted.unwrap().scene.clone();
        let at = |column| {
            scene.matrix_scenes[0]
                .cells
                .iter()
                .find(|cell| cell.row == 0 && cell.column == column)
                .expect("the key is laid out")
                .pose
                .at
        };
        assert!(
            (at(1).x - 28.575).abs() < 1e-9 && (at(2).x - 47.625).abs() < 1e-9,
            "later columns step out the full Spread"
        );
        assert!(
            at(0).x.abs() < 1e-9,
            "column 0 does not follow a later Spread"
        );
        fixture::undo(&runtime).await;
        let matrix = fixture::accepted_matrix(&runtime);
        assert!(
            matrix
                .column_layout
                .get(1)
                .and_then(|record| record.spread)
                .is_none(),
            "one Undo restores the default step"
        );
        root.remove();
    }

    /// A second 1x1 matrix anchored to the fixture matrix's first key, with a
    /// live key part, for the matrix Move and Rotate drags.
    fn anchored_drag_document(
        follow: boardstudio_core::model::Follow,
    ) -> boardstudio_core::model::ProjectDoc {
        use boardstudio_core::model::*;
        let mut document = fixture::matrix_document();
        document.layouts.push(Layout {
            id: "layout".into(),
            name: "Main".into(),
            board_id: fixture::BOARD_ID.into(),
            matrix_ids: vec![fixture::MATRIX_ID.into(), "dep".into()],
            part_ids: vec![],
            mirror_link: None,
        });
        let key_id = "matrix/dep/r0c0";
        document.boards[0].part_ids.push(key_id.into());
        document.parts.push(Part {
            keycap: None,
            outline: None,
            id: key_id.into(),
            definition_id: "switch:base".into(),
            reference: "K3".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: -19.05 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: Some(false),
            properties: None,
            generator_parameters: None,
        });
        document.matrices.push(Matrix {
            id: "dep".into(),
            name: Some("Dependent".into()),
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.05, y: 19.05 },
            origin: Vec2 { x: 0.0, y: -19.05 },
            definition_id: "switch:base".into(),
            part_ids: vec![key_id.into()],
            board_id: Some(fixture::BOARD_ID.into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![ColumnLayout::default()],
            anchor: Some(MatrixAnchor {
                matrix_id: fixture::MATRIX_ID.into(),
                row: 0,
                column: 0,
                follow,
            }),
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: Some("switch:base".into()),
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        document
    }

    async fn open_anchored_drag_runtime(follow: boardstudio_core::model::Follow) -> Rc<Runtime> {
        let runtime = support::new_runtime();
        support::open_document(&runtime, anchored_drag_document(follow)).await;
        wait_until_editable(&runtime).await;
        runtime
    }

    /// Ticket 23 slice, RED: dragging an anchored matrix writes its Origin
    /// through the inverse of the anchor frame as one atomic edit.
    #[wasm_bindgen_test]
    async fn anchored_matrix_move_drag_writes_origin_through_anchor_inverse() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let (root, handle) = mount_matrix_overlay(&runtime, LayoutTransformTool::Move, "dep").await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Drag to move matrix"),
            "the Move tool draws its own handle"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag previews actual matrix movement before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let accepted = runtime.model().accepted.expect("an accepted document");
        let matrix = accepted
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dragged matrix");
        assert!(
            (matrix.origin.x - 10.0).abs() < 1e-9 && (matrix.origin.y + 19.05).abs() < 1e-9,
            "the drag writes the Origin through the anchor inverse, got {:?}",
            matrix.origin
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "one accepted edit"
        );
        fixture::undo(&runtime).await;
        let matrix = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .cloned()
            .expect("the dragged matrix");
        assert!(
            matrix.origin.x.abs() < 1e-9 && (matrix.origin.y + 19.05).abs() < 1e-9,
            "one Undo restores the dragged Origin"
        );
        root.remove();
    }

    /// Ticket 23 slice, RED: the matrix drag is constrained to the Follow
    /// directions, so an unfollowed component never moves the frame.
    #[wasm_bindgen_test]
    async fn anchored_matrix_move_drag_ignores_unfollowed_directions() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: false,
            y: true,
            rotation: true,
        })
        .await;
        let (root, handle) = mount_matrix_overlay(&runtime, LayoutTransformTool::Move, "dep").await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_none(),
            "a drag outside every followed direction previews nothing"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        for _ in 0..50 {
            support::run_pending(&runtime).await;
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision,
            "a drag outside every followed direction commits nothing"
        );
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 80));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the followed direction previews before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 80));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let matrix = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .cloned()
            .expect("the dragged matrix");
        assert!(
            matrix.origin.x.abs() < 1e-9 && (matrix.origin.y + 14.05).abs() < 1e-9,
            "only the followed direction moves the frame, got {:?}",
            matrix.origin
        );
        root.remove();
    }

    /// Ticket 23 slice, RED: the Rotate tool turns the selected matrix, and
    /// the menu disables Rotate with an explanation when Follow rotation is
    /// off.
    #[wasm_bindgen_test]
    async fn matrix_rotate_drag_turns_the_matrix() {
        use boardstudio_core::model::Follow;
        let runtime = open_anchored_drag_runtime(Follow {
            x: true,
            y: true,
            rotation: true,
        })
        .await;
        let (root, _) = mount_matrix_overlay(&runtime, LayoutTransformTool::Rotate, "dep").await;
        let rotate = root
            .query_selector("[aria-label='Drag to rotate matrix']")
            .unwrap()
            .expect("the Rotate tool draws its own handle");
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let accepted = runtime.model().accepted.expect("an accepted document");
        let center = accepted
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == "dep")
            .expect("the dragged scene")
            .cells
            .iter()
            .find(|cell| cell.enabled)
            .expect("the key is laid out")
            .pose
            .at;
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(
            &rotate,
            "pointerdown",
            false,
            false,
            1,
            client(center.x + 20.0, center.y),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(
            &rotate,
            "pointermove",
            false,
            false,
            1,
            client(center.x, center.y + 20.0),
        );
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the rotation previews before commit"
        );
        dispatch_pointer(
            &rotate,
            "pointerup",
            false,
            false,
            0,
            client(center.x, center.y + 20.0),
        );
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let rotation = runtime
            .model()
            .accepted
            .expect("an accepted document")
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == "dep")
            .expect("the dragged matrix")
            .rotation;
        assert_eq!(rotation, Some(90.0), "a quarter turn sweeps 90 degrees");
        root.remove();
    }

    /// Ticket 23, RED: hovering the Spread handle neither replaces its SVG
    /// target nor lets a passive hint intercept the pointer-down that follows.
    #[wasm_bindgen_test]
    async fn spread_handle_hover_keeps_its_svg_target() {
        let runtime = fixture::open_matrix_runtime().await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Spread).await;
        dispatch_pointer(&handle, "pointerover", false, false, 0, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        let live = root
            .query_selector(".m1-transform-handle")
            .unwrap()
            .expect("the handle survives hover");
        assert!(
            live == handle,
            "hover does not replace the handle's SVG target"
        );
        let rect = handle.get_bounding_client_rect();
        // No passive hint may cover the handle: every painted element in the
        // overlay that contains the handle's center and paints above it must
        // opt out of pointer events. Box math is viewport-independent, so the
        // check holds wherever the canvas scrolled.
        let center = (
            rect.x() + rect.width() / 2.0,
            rect.y() + rect.height() / 2.0,
        );
        let window = web_sys::window().unwrap();
        let painted = root.query_selector_all("svg *").unwrap();
        let mut interceptors = Vec::new();
        for index in 0..painted.length() {
            let element: web_sys::Element = painted.item(index).unwrap().dyn_into().unwrap();
            if element == handle || handle.contains(Some(element.unchecked_ref())) {
                continue;
            }
            let position = handle.compare_document_position(element.unchecked_ref());
            if position & web_sys::Node::DOCUMENT_POSITION_FOLLOWING == 0 {
                continue;
            }
            let area = element.get_bounding_client_rect();
            if center.0 < area.x()
                || center.0 > area.x() + area.width()
                || center.1 < area.y()
                || center.1 > area.y() + area.height()
            {
                continue;
            }
            let events = window
                .get_computed_style(&element)
                .ok()
                .flatten()
                .and_then(|style| style.get_property_value("pointer-events").ok())
                .unwrap_or_default();
            if events != "none" {
                interceptors.push(format!("{}.{}", element.tag_name(), element.class_name()));
            }
        }
        assert!(
            interceptors.is_empty(),
            "no passive hint intercepts the handle: {interceptors:?}"
        );
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 100));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision || runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            fixture_revision(&runtime) > revision || runtime.model().display_preview.is_some(),
            "the pointer-down after hover still starts the drag"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 100));
        root.remove();
    }

    /// Ticket 23 slice 11: releasing the pointer outside the handle still
    /// commits the drag. Capture retargets the up to the handle, so the drop
    /// path must not re-hit-test the handle geometry: an up far outside it
    /// commits the previewed operation with one Undo step.
    #[wasm_bindgen_test]
    async fn pointerup_outside_the_handle_still_commits_the_drag() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing before the outside release"
        );
        // The pointer left the handle: the up arrives with a client point far
        // outside the handle geometry, as capture retargeting delivers it.
        dispatch_pointer(&handle, "pointerup", false, false, 0, (900, 900));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the outside release commits one accepted edit"
        );
        assert!(
            runtime.model().display_preview.is_none(),
            "the preview clears on release"
        );
        let stagger = fixture::accepted_matrix(&runtime).column_layout[1].stagger;
        assert!(stagger != 0.0, "the committed drag moved the column");
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger,
            0.0,
            "one Undo restores the column"
        );
        root.remove();
    }

    /// Ticket 23 slice 12: losing pointer capture mid-drag cancels the
    /// preview with nothing committed, and a subsequent drag on the same
    /// handle recovers and commits.
    #[wasm_bindgen_test]
    async fn lost_capture_cancels_and_a_subsequent_drag_recovers() {
        let mut document = fixture::matrix_document();
        document.matrices[0]
            .column_layout
            .resize_with(2, Default::default);
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        wait_until_editable(&runtime).await;
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Stagger).await;
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the drag is previewing before capture is lost"
        );
        dispatch_pointer(&handle, "lostpointercapture", false, false, 1, (140, 60));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_none() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_none(),
            "lost capture clears the preview"
        );
        assert_eq!(
            fixture_revision(&runtime),
            revision,
            "the cancelled drag commits nothing"
        );
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger,
            0.0,
            "the cancelled drag moves nothing"
        );
        // A subsequent drag on the same handle recovers and commits.
        dispatch_pointer(&handle, "pointerdown", false, false, 1, (100, 100));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, (140, 60));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointerup", false, false, 0, (140, 60));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            revision + 1,
            "the subsequent drag commits one accepted edit"
        );
        assert!(
            fixture::accepted_matrix(&runtime).column_layout[1].stagger != 0.0,
            "the subsequent drag moves the column"
        );
        root.remove();
    }

    /// Ticket 23 slice 6, RED: Ctrl at pointer-down on the Splay handle latches a
    /// swing. The drag writes only the origin through `OriginSwing` (no step
    /// compensation), never the angle; one Undo restores exactly.
    #[wasm_bindgen_test]
    async fn ctrl_splay_drag_swings_the_origin_without_step_compensation() {
        let mut document = fixture::matrix_document();
        let matrix = &mut document.matrices[0];
        matrix.column_layout.resize_with(2, Default::default);
        matrix.column_layout[1].splay = 17.0;
        matrix.column_layout[1].stagger = 3.0;
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let before = fixture_revision(&runtime);
        let origin = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .scene
            .matrix_scenes[0]
            .columns[1]
            .splay_origin;
        let (root, _) = mount_overlay(&runtime, LayoutTransformTool::Splay).await;
        let handle = root
            .query_selector("[aria-label='Drag to splay']")
            .unwrap()
            .unwrap();
        let stub = js_sys::Function::new_no_args("return true;");
        for method in [
            "setPointerCapture",
            "hasPointerCapture",
            "releasePointerCapture",
        ] {
            js_sys::Reflect::set(handle.as_ref(), &method.into(), stub.as_ref()).unwrap();
        }
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let start = client(origin.x + 20.0, origin.y);
        let end = client(origin.x, origin.y + 20.0);
        // Ctrl is down for the pointer-down only; the latch must hold the swing.
        dispatch_pointer(&handle, "pointerdown", true, false, 1, start);
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert_eq!(
            handle.get_attribute("data-swing").as_deref(),
            Some("true"),
            "Ctrl at pointer-down shows the swing"
        );
        dispatch_pointer(&handle, "pointermove", false, false, 1, end);
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(
            fixture_revision(&runtime),
            before,
            "the swing preview never commits"
        );
        let preview = runtime
            .model()
            .display_preview
            .as_ref()
            .expect("the swing previews")
            .matrix_scenes[0]
            .columns[1]
            .clone();
        assert_eq!(
            preview.splay_angle, 17.0,
            "the swing preview never turns the angle"
        );
        assert!(
            preview.splay_origin != origin,
            "the swing preview moves the origin"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, end);
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > before {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert_eq!(fixture_revision(&runtime), before + 1, "one accepted edit");
        let matrix = fixture::accepted_matrix(&runtime);
        let record = &matrix.column_layout[1];
        assert_eq!(record.splay, 17.0, "the swing writes no angle");
        assert!(
            record.spread.is_none(),
            "the swing writes no compensating Spread"
        );
        assert_eq!(
            record.stagger, 3.0,
            "the swing writes no compensating Stagger"
        );
        assert!(
            record.splay_origin != Vec2::default(),
            "the swing moves the origin"
        );
        fixture::undo(&runtime).await;
        let matrix = fixture::accepted_matrix(&runtime);
        let record = &matrix.column_layout[1];
        assert_eq!(
            record.splay_origin,
            Vec2::default(),
            "one Undo restores the origin exactly"
        );
        assert_eq!(record.splay, 17.0, "one Undo keeps the angle");
        assert!(
            record.spread.is_none() && record.stagger == 3.0,
            "one Undo keeps the steps uncompensated"
        );
        root.remove();
    }

    /// Ticket 23 slice 6, RED: an Origin drag snaps to its column base (the
    /// ergogen-style reset target). Alt disables the snap, and a subsequent
    /// drag recovers.
    #[wasm_bindgen_test]
    async fn origin_drag_snaps_to_the_column_base_without_alt() {
        let mut document = fixture::matrix_document();
        let matrix = &mut document.matrices[0];
        matrix.column_layout.resize_with(2, Default::default);
        // An off-grid base: the grid alone would land this drop at 23.8125,
        // never exactly on the base.
        matrix.column_layout[1].spread = Some(22.0);
        matrix.column_layout[1].splay_origin = Vec2 { x: 5.0, y: 0.0 };
        let runtime = support::new_runtime();
        support::open_document(&runtime, document).await;
        let scene = runtime
            .model()
            .accepted
            .as_ref()
            .expect("an accepted document")
            .scene
            .clone();
        let basis = &scene.matrix_scenes[0].columns[1];
        assert_eq!(basis.base, Vec2 { x: 22.0, y: 0.0 });
        assert_eq!(basis.splay_origin, Vec2 { x: 27.0, y: 0.0 });
        let before = fixture_revision(&runtime);
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Origin).await;
        assert_eq!(
            handle.get_attribute("aria-label").as_deref(),
            Some("Move splay origin")
        );
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        // Raw drop (23, 1): 1.4 mm from the base, off the grid.
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(27.0, 0.0));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, false, 1, client(23.0, 1.0));
        for _ in 0..100 {
            support::run_pending(&runtime).await;
            if runtime.model().display_preview.is_some() {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        assert!(
            runtime.model().display_preview.is_some(),
            "the origin drag previews before commit"
        );
        dispatch_pointer(&handle, "pointerup", false, false, 0, client(23.0, 1.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > before {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let record = &fixture::accepted_matrix(&runtime).column_layout[1];
        assert!(
            record.splay_origin.x.abs() < 1e-9 && record.splay_origin.y.abs() < 1e-9,
            "the drag snaps to the column base, not the grid: {:?}",
            record.splay_origin
        );
        fixture::undo(&runtime).await;
        assert_eq!(
            fixture::accepted_matrix(&runtime).column_layout[1].splay_origin,
            Vec2 { x: 5.0, y: 0.0 },
            "one Undo restores the custom origin"
        );

        // Alt disables the snap: the same gesture lands on the raw drop, and
        // the recovered drag still commits. The overlay remounts because the
        // commit moved the owner revision on.
        root.remove();
        let (root, handle) = mount_overlay(&runtime, LayoutTransformTool::Origin).await;
        let rect = root
            .query_selector("svg")
            .unwrap()
            .unwrap()
            .get_bounding_client_rect();
        let client = |x: f64, y: f64| {
            (
                (rect.left() + x * 4.0).round() as i32,
                (rect.top() - y * 4.0).round() as i32,
            )
        };
        let revision = fixture_revision(&runtime);
        dispatch_pointer(&handle, "pointerdown", false, false, 1, client(27.0, 0.0));
        gloo_timers::future::TimeoutFuture::new(40).await;
        dispatch_pointer(&handle, "pointermove", false, true, 1, client(23.0, 1.0));
        dispatch_pointer(&handle, "pointerup", false, true, 0, client(23.0, 1.0));
        for _ in 0..200 {
            support::run_pending(&runtime).await;
            if fixture_revision(&runtime) > revision {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(10).await;
        }
        let record = &fixture::accepted_matrix(&runtime).column_layout[1];
        // One client pixel is a quarter world mm, and Alt skips the snap that
        // would absorb the roundtrip: the drop lands on the raw point, which
        // is neither the base nor a grid step.
        assert!(
            (record.splay_origin.x - 1.0).abs() < 0.25
                && (record.splay_origin.y - 1.0).abs() < 0.25,
            "Alt lands the raw drop without snapping: {:?}",
            record.splay_origin
        );
        root.remove();
    }
}

// The modifier changes presentation immediately; pointer-down latches the edit intent.
struct CtrlModifierListener {
    document: web_sys::Document,
    window: web_sys::Window,
    down: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    up: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    blur: Closure<dyn FnMut(web_sys::Event)>,
}
impl CtrlModifierListener {
    fn install(mut pressed: Signal<bool>) -> Option<Self> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let down = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(true);
            }
        }) as Box<dyn FnMut(_)>);
        let up = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.key() == "Control" {
                pressed.set(false);
            }
        }) as Box<dyn FnMut(_)>);
        let blur = Closure::wrap(
            Box::new(move |_: web_sys::Event| pressed.set(false)) as Box<dyn FnMut(_)>
        );
        let listener = Self {
            document,
            window,
            down,
            up,
            blur,
        };
        listener
            .document
            .add_event_listener_with_callback("keydown", listener.down.as_ref().unchecked_ref())
            .ok()?;
        listener
            .document
            .add_event_listener_with_callback("keyup", listener.up.as_ref().unchecked_ref())
            .ok()?;
        listener
            .window
            .add_event_listener_with_callback("blur", listener.blur.as_ref().unchecked_ref())
            .ok()?;
        Some(listener)
    }
}
impl Drop for CtrlModifierListener {
    fn drop(&mut self) {
        let _ = self
            .document
            .remove_event_listener_with_callback("keydown", self.down.as_ref().unchecked_ref());
        let _ = self
            .document
            .remove_event_listener_with_callback("keyup", self.up.as_ref().unchecked_ref());
        let _ = self
            .window
            .remove_event_listener_with_callback("blur", self.blur.as_ref().unchecked_ref());
    }
}
