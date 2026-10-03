//! Transform-property navigation for the Layout command pill.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::{ScopedTreeContext, TreeContext};
use crate::presentation::{
    LayoutOwnerIdentity,
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
};
use crate::runtime::Runtime;
use boardstudio_application::{Durability, Event, Lifecycle};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Matrix, MatrixScene, MatrixSplayAffect,
    MatrixSplayChange, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};
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
pub(in crate::presentation) enum LayoutTransformTool {
    Stagger,
    Splay,
    Origin,
}

impl LayoutTransformTool {
    const ALL: [(Self, &'static str); 3] = [
        (Self::Stagger, "Stagger"),
        (Self::Splay, "Splay"),
        (Self::Origin, "Origin"),
    ];
}

#[derive(Clone, PartialEq)]
pub(in crate::presentation) struct LayoutTransformMenuMount {
    pub properties_available: bool,
    pub column_available: bool,
    pub row_available: bool,
    pub pointer_tools_visible: bool,
    pub pointer_tools_available: bool,
    pub active_tool: Option<LayoutTransformTool>,
    pub on_selection_kind: EventHandler<LayoutSelectionKind>,
    pub on_show_properties: EventHandler<()>,
    pub on_transform_tool: EventHandler<LayoutTransformTool>,
}

#[derive(Clone)]
pub(in crate::presentation) struct LayoutTransformRuntime(pub Rc<Runtime>);

impl PartialEq for LayoutTransformRuntime {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
pub(in crate::presentation) struct LayoutTransformSvg(pub Rc<RefCell<Option<SvgElement>>>);

impl PartialEq for LayoutTransformSvg {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TransformGesture {
    Stagger {
        row_axis: bool,
        index: u32,
    },
    Splay {
        column: u32,
        start_angle: f64,
        origin: Vec2,
    },
    Origin {
        column: u32,
    },
}

#[derive(Clone)]
struct TransformDrag {
    pointer_id: i32,
    capture: SvgElement,
    matrix: Matrix,
    gesture: TransformGesture,
    transaction_id: String,
    owner: LayoutOwnerIdentity,
    context: TreeContext,
    tool: LayoutTransformTool,
    start_client_x: f64,
    start_client_y: f64,
    pending: Option<EditOperation>,
    preview_submitted: bool,
}

#[component]
pub(in crate::presentation) fn LayoutTransformToolOverlay(
    runtime: LayoutTransformRuntime,
    svg: LayoutTransformSvg,
    arbiter: CanvasInteractionArbiter,
    owner: LayoutOwnerIdentity,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    matrix: Matrix,
    projection: MatrixScene,
    context: TreeContext,
    tool: LayoutTransformTool,
    snap_settings: super::layout_toolbar::LayoutSnapSettings,
    snap_origins: Vec<super::super::outline_snapping::Origin>,
    splay_affect: Signal<MatrixSplayAffect>,
    view_x: f64,
    view_y: f64,
    width: f64,
    height: f64,
    on_finish: EventHandler<()>,
) -> Element {
    let drag = use_hook(|| Rc::new(RefCell::new(None::<TransformDrag>)));
    let snap_guide = use_signal(|| None::<Vec2>);
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
                event.prevent_default();
                let mut guide = snap_guide;
                cancel_transform_drag(&runtime, &drag, &arbiter, &mut guide);
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
                    );
                }
            }
        },
    ));

    let (handle_point, handle_scale, selected_column) = {
        let selected_column = context_column(&context).unwrap_or(0);
        let selected_row = context_row(&context);
        let point = match tool {
            LayoutTransformTool::Stagger => {
                let row_axis = selected_row.is_some();
                let index = selected_row.unwrap_or(selected_column);
                let cell = if row_axis {
                    projection
                        .cells
                        .iter()
                        .find(|cell| cell.enabled && cell.row == index && cell.column == 0)
                } else {
                    projection
                        .cells
                        .iter()
                        .find(|cell| cell.enabled && cell.row == 0 && cell.column == index)
                };
                cell.map(|cell| Vec2 {
                    x: cell.pose.at.x,
                    y: cell.pose.at.y + 12.0,
                })
            }
            LayoutTransformTool::Splay | LayoutTransformTool::Origin => projection
                .columns
                .iter()
                .find(|basis| basis.column == selected_column)
                .map(|basis| basis.splay_origin),
        };
        let scale = svg.0.borrow().as_ref().map_or(3.0, |surface| {
            let rect = surface.get_bounding_client_rect();
            if rect.width() > 0.0 {
                width / rect.width() * 3.0
            } else {
                3.0
            }
        });
        (point, scale, selected_column)
    };

    let Some(handle_point) = handle_point else {
        return rsx! {};
    };
    let angle_point = angle_handle_point(&projection, selected_column, matrix.pitch.y);
    let angle_degrees = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(0.0, |basis| basis.splay_angle);
    let current_owner = owner.clone();
    let current_context = context.clone();
    let runtime_for_start = runtime.0.clone();
    let svg_for_start = svg.0.clone();
    let drag_for_start = drag.clone();
    let arbiter_for_start = arbiter.clone();
    let guide_for_start = snap_guide;
    let start_drag = Rc::new(move |event: PointerEvent, gesture: TransformGesture| {
        let mut guide_for_start = guide_for_start;
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
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
        ) || context_matrix_id(&current_context) != Some(matrix.id.as_str())
            || !transform_start_is_editable(&runtime_for_start, &current_owner, &matrix)
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
            TransformGesture::Splay { column, origin, .. } => TransformGesture::Splay {
                column,
                origin,
                start_angle,
            },
            other => other,
        };
        let operation = runtime_for_start.operation();
        *drag_for_start.borrow_mut() = Some(TransformDrag {
            pointer_id: pointer.pointer_id(),
            capture,
            matrix: matrix.clone(),
            gesture,
            transaction_id: format!("layout-transform-{}", operation.0),
            owner: current_owner.clone(),
            context: current_context.clone(),
            tool,
            start_client_x: f64::from(pointer.client_x()),
            start_client_y: f64::from(pointer.client_y()),
            pending: None,
            preview_submitted: false,
        });
        guide_for_start.set(None);
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
            submit_transform_edit(
                &runtime_for_move,
                &active,
                operation.clone(),
                EditPhase::Preview,
            );
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
            );
            return;
        }
        if pointer.type_() != "pointerup" {
            cancel_transform_drag(
                &runtime_for_end,
                &drag_for_end,
                &arbiter_for_end,
                &mut guide_for_end,
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
                submit_transform_edit(
                    &runtime_for_end,
                    &active,
                    operation.clone(),
                    EditPhase::Preview,
                );
                active.preview_submitted = true;
            }
            if !same_transform_as_start(&active, &operation) {
                submit_transform_edit(&runtime_for_end, &active, operation, EditPhase::Commit);
            } else if active.preview_submitted {
                clear_transform_preview(&runtime_for_end, &active);
            }
        } else if active.preview_submitted {
            clear_transform_preview(&runtime_for_end, &active);
        }
        guide_for_end.set(None);
        if active.capture.has_pointer_capture(active.pointer_id) {
            let _ = active.capture.release_pointer_capture(active.pointer_id);
        }
        drag_for_end.borrow_mut().take();
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
            );
            on_finish.call(());
        }
    });

    let selected_row = context_row(&context);
    let gesture = if tool == LayoutTransformTool::Stagger {
        TransformGesture::Stagger {
            row_axis: selected_row.is_some(),
            index: selected_row.unwrap_or(selected_column),
        }
    } else {
        TransformGesture::Origin {
            column: selected_column,
        }
    };
    rsx! {
        if let Some(guide) = snap_guide() {
            g { class: "m1-transform-snap-guide", "aria-hidden": "true",
                circle { cx: "{guide.x}", cy: "{guide.y}", r: "{width / 170.0}" }
            }
        }
        if tool == LayoutTransformTool::Stagger {
            g {
                class: "m1-transform-handle m1-stagger-handle",
                transform: "translate({handle_point.x} {handle_point.y})",
                role: "button", tabindex: "0", "aria-label": "Drag to stagger",
                onpointerdown: { let start = start_drag.clone(); move |event| start(event, gesture) },
                onpointermove: { let handler = move_drag.clone(); move |event| handler(event) },
                onpointerup: { let handler = end_drag.clone(); move |event| handler(event) },
                onpointercancel: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onlostpointercapture: { let handler = cancel_drag.clone(); move |event| handler(event) },
                onkeydown: { let handler = on_key_down.clone(); move |event| handler(event) },
                circle { r: "4" }
                path { d: "M0 -3v6M-1.5 -1.5 0 -3l1.5 1.5M-1.5 1.5 0 3l1.5 -1.5" }
            }
        } else {
            g { class: "m1-transform-splay-handles",
                if tool == LayoutTransformTool::Splay {
                    path { class: "m1-transform-splay-guide", d: "M{handle_point.x} {handle_point.y} L{angle_point.x} {angle_point.y}" }
                }
                g {
                    class: "m1-transform-handle m1-splay-origin-handle",
                    transform: "translate({handle_point.x} {handle_point.y}) scale({handle_scale})",
                    role: "button", tabindex: "0", "aria-label": "Move splay origin",
                    onpointerdown: { let start = start_drag.clone(); move |event| start(event, TransformGesture::Origin { column: selected_column }) },
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
                        };
                        rsx! {
                            g {
                                class: "m1-transform-handle m1-splay-angle-handle",
                                transform: "translate({angle_point.x} {angle_point.y}) scale({handle_scale})",
                                role: "button", tabindex: "0", "aria-label": "Drag to splay",
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
        TreeContext::Row { row, .. } | TreeContext::Key { row, .. } => Some(*row),
        TreeContext::Component { row, .. } => *row,
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

fn local_matrix_delta(matrix: &Matrix, delta: Vec2, column: u32) -> Vec2 {
    let angle = -matrix.rotation.unwrap_or(0.0).to_radians();
    let (sin, cos) = angle.sin_cos();
    let mut x = delta.x * cos - delta.y * sin;
    let mut y = delta.x * sin + delta.y * cos;
    match matrix.mirror {
        Some(boardstudio_core::model::Mirror::X) => x = -x,
        Some(boardstudio_core::model::Mirror::Y) => y = -y,
        Some(boardstudio_core::model::Mirror::None) | None => {}
    }
    let splay = -matrix
        .column_splays
        .iter()
        .take(column as usize + 1)
        .sum::<f64>()
        .to_radians();
    let (sin, cos) = splay.sin_cos();
    Vec2 {
        x: x * cos - y * sin,
        y: x * sin + y * cos,
    }
}

fn snap_delta(delta: Vec2, pitch: Vec2, fraction: f64) -> Vec2 {
    if fraction == 0.0 {
        return delta;
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
    Vec2 {
        x: (delta.x / step_x).round() * step_x,
        y: (delta.y / step_y).round() * step_y,
    }
}

fn snap_world_origin(
    point: Vec2,
    matrix: &Matrix,
    pointer: &web_sys::PointerEvent,
    settings: &super::layout_toolbar::LayoutSnapSettings,
    origins: &[super::super::outline_snapping::Origin],
    svg: &Rc<RefCell<Option<SvgElement>>>,
    view_width: f64,
) -> (Vec2, Option<Vec2>) {
    if pointer.alt_key() {
        return (point, None);
    }
    if settings.geometry_snap {
        let tolerance = svg.borrow().as_ref().map_or(1.0, |surface| {
            let width = surface.get_bounding_client_rect().width();
            if width > 0.0 {
                view_width / width * 8.0
            } else {
                1.0
            }
        });
        if let Some((at, _)) =
            super::super::outline_snapping::snap_origin(point, origins, tolerance)
        {
            return (at, Some(at));
        }
    }
    (
        snap_delta(point, matrix.pitch, settings.snap_fraction),
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
        TransformGesture::Stagger { row_axis, index } => {
            let rect = svg.borrow().as_ref()?.get_bounding_client_rect();
            if rect.width() <= 0.0 || rect.height() <= 0.0 {
                return None;
            }
            let delta = local_matrix_delta(
                matrix,
                Vec2 {
                    x: (f64::from(pointer.client_x()) - drag.start_client_x) * view_width
                        / rect.width(),
                    y: -(f64::from(pointer.client_y()) - drag.start_client_y) * view_height
                        / rect.height(),
                },
                if row_axis { 0 } else { index },
            );
            let delta = if pointer.alt_key() {
                delta
            } else {
                snap_delta(delta, matrix.pitch, settings.snap_fraction)
            };
            let mut next = matrix.clone();
            if row_axis {
                let values = &mut next.row_offsets;
                while values.len() <= index as usize {
                    values.push(Vec2 { x: 0.0, y: 0.0 });
                }
                let base = matrix
                    .row_offsets
                    .get(index as usize)
                    .copied()
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                values[index as usize] = Vec2 {
                    x: base.x + delta.x,
                    y: base.y + delta.y,
                };
            } else {
                let values = &mut next.column_offsets;
                while values.len() <= index as usize {
                    values.push(Vec2 { x: 0.0, y: 0.0 });
                }
                let base = matrix
                    .column_offsets
                    .get(index as usize)
                    .copied()
                    .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
                values[index as usize] = Vec2 {
                    x: base.x + delta.x,
                    y: base.y + delta.y,
                };
            }
            guide.set(None);
            Some(EditOperation::SetMatrix {
                matrix: next,
                definitions: None,
            })
        }
        TransformGesture::Splay {
            column,
            start_angle,
            origin,
        } => {
            let angle = (point.y - origin.y).atan2(point.x - origin.x) - start_angle;
            let mut delta = angle.sin().atan2(angle.cos()).to_degrees();
            if matches!(
                matrix.mirror,
                Some(boardstudio_core::model::Mirror::X | boardstudio_core::model::Mirror::Y)
            ) {
                delta = -delta;
            }
            let base = matrix
                .column_splays
                .get(column as usize)
                .copied()
                .unwrap_or(0.0);
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
        TransformGesture::Origin { column } => {
            let (world, snap) =
                snap_world_origin(point, matrix, pointer, settings, origins, svg, view_width);
            guide.set(snap);
            Some(EditOperation::SetMatrixSplay {
                matrix_id: matrix.id.clone(),
                column,
                change: MatrixSplayChange::Origin { world: Some(world) },
            })
        }
    }
}

fn same_transform_as_start(drag: &TransformDrag, operation: &EditOperation) -> bool {
    match (&drag.gesture, operation) {
        (TransformGesture::Stagger { .. }, EditOperation::SetMatrix { matrix, .. }) => {
            matrix == &drag.matrix
        }
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Angle { angle, .. },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && *angle
                    == drag
                        .matrix
                        .column_splays
                        .get(*column as usize)
                        .copied()
                        .unwrap_or(0.0)
        }
        (
            TransformGesture::Origin { column },
            EditOperation::SetMatrixSplay {
                matrix_id,
                column: target,
                change: MatrixSplayChange::Origin { world },
            },
        ) => {
            matrix_id == &drag.matrix.id
                && target == column
                && *world
                    == drag
                        .matrix
                        .column_origins
                        .get(*column as usize)
                        .copied()
                        .flatten()
        }
        _ => false,
    }
}

fn submit_transform_edit(
    runtime: &Rc<Runtime>,
    drag: &TransformDrag,
    operation: EditOperation,
    phase: EditPhase,
) {
    let Some(revision) = drag.owner.revision else {
        return;
    };
    runtime.submit(Event::Edit {
        operation_id: runtime.operation(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: drag.transaction_id.clone(),
            phase,
            target_ids: vec![drag.matrix.id.clone()],
            operation,
        },
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
) {
    let Some(active) = drag.borrow_mut().take() else {
        return;
    };
    clear_transform_preview(runtime, &active);
    if active.capture.has_pointer_capture(active.pointer_id) {
        let _ = active.capture.release_pointer_capture(active.pointer_id);
    }
    guide.set(None);
    arbiter.release(CanvasInteractionOwner::MatrixTransform);
}

fn angle_handle_point(projection: &MatrixScene, column: u32, pitch_y: f64) -> Vec2 {
    let cells: Vec<_> = projection
        .cells
        .iter()
        .filter(|cell| cell.enabled && cell.column == column)
        .collect();
    if cells.is_empty() {
        return Vec2 {
            x: 0.0,
            y: pitch_y * 0.9,
        };
    }
    let x = cells.iter().map(|cell| cell.pose.at.x).sum::<f64>() / cells.len() as f64;
    let y = cells
        .iter()
        .map(|cell| cell.pose.at.y)
        .fold(f64::NEG_INFINITY, f64::max)
        + pitch_y * 0.9;
    Vec2 { x, y }
}

#[component]
pub(in crate::presentation) fn LayoutTransformToolbar(
    mount: LayoutTransformMenuMount,
    open_menu: Signal<Option<LayoutCommandMenu>>,
) -> Element {
    let is_open = open_menu() == Some(LayoutCommandMenu::Transform);
    let position_label = "Position & rotation";

    rsx! {
        details {
            class: "m1-layout-command-menu m1-layout-transform-menu",
            "data-layout-menu": "transform",
            open: is_open,
            onkeydown: move |event: KeyboardEvent| {
                if event.data().key().to_string() == "Escape" && open_menu() == Some(LayoutCommandMenu::Transform) {
                    event.prevent_default();
                    event.stop_propagation();
                    close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                }
            },
            summary {
                id: "m1-layout-transform-trigger",
                "aria-controls": "m1-layout-transform-menu",
                "aria-expanded": "{is_open}",
                onclick: move |event: MouseEvent| {
                    event.prevent_default();
                    let mut open_menu = open_menu;
                    open_menu.set((open_menu() != Some(LayoutCommandMenu::Transform)).then_some(LayoutCommandMenu::Transform));
                },
                svg { class: "m1-layout-command-trigger-icon", view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: "M10 1v18M1 10h18M7 4l3-3 3 3M7 16l3 3 3-3M4 7l-3 3 3 3M16 7l3 3-3 3" }
                }
                span { "Transform" }
            }
            div { id: "m1-layout-transform-menu", class: "m1-layout-transform-popover",
                super::layout_toolbar::LayoutCommandMenuHeader {
                    label: "Transform".to_owned(),
                    close_label: "Close Transform".to_owned(),
                    open_menu,
                    menu: LayoutCommandMenu::Transform,
                }
                for (tool, label) in LayoutTransformTool::ALL {
                    if mount.pointer_tools_visible {
                        button {
                            r#type: "button",
                            disabled: !mount.pointer_tools_available,
                            aria_pressed: mount.active_tool == Some(tool),
                            "data-close-menu": "true",
                            onclick: move |_| {
                                mount.on_transform_tool.call(tool);
                                close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                            },
                            "{label}"
                        }
                    }
                }
                if mount.properties_available {
                    button {
                        r#type: "button",
                        "data-close-menu": "true",
                        onclick: move |_| {
                            mount.on_show_properties.call(());
                            close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                        },
                        "{position_label}"
                    }
                } else {
                    button { r#type: "button", disabled: true, "{position_label}" }
                }
                button {
                    r#type: "button",
                    disabled: !mount.column_available,
                    "data-close-menu": "true",
                    onclick: move |_| {
                        mount.on_selection_kind.call(LayoutSelectionKind::Column);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Splay & origin"
                }
                button {
                    r#type: "button",
                    disabled: !mount.row_available,
                    "data-close-menu": "true",
                    onclick: move |_| {
                        mount.on_selection_kind.call(LayoutSelectionKind::Row);
                        close_layout_command_menu(open_menu, LayoutCommandMenu::Transform);
                    },
                    "Row offsets"
                }
                if !mount.properties_available {
                    p { "Select a matrix, row, column, key or part to open its current transform properties." }
                }
            }
        }
    }
}
