//! Transform-property navigation for the Layout command pill.
use super::layout_toolbar::LayoutSelectionKind;
use super::layout_toolbar::{LayoutCommandMenu, close_layout_command_menu};
use super::{ScopedTreeContext, TreeContext};
use crate::presentation::{
    LayoutOwnerIdentity,
    canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner},
};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution,
};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, Matrix, MatrixScene, MatrixSplayAffect,
    MatrixSplayChange, Vec2,
};
use boardstudio_web_runtime::edit_ticket::EditTicket;
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
pub enum LayoutTransformTool {
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
pub struct LayoutTransformMenuMount {
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
pub fn LayoutTransformToolOverlay(
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

    let selected_points = {
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
    let angle_point = angle_handle_point(
        &projection,
        selected_column,
        matrix.pitch.y,
        &selected_points,
    );
    let angle_degrees = projection
        .columns
        .iter()
        .find(|basis| basis.column == selected_column)
        .map_or(0.0, |basis| basis.splay_angle);
    let current_owner = owner.clone();
    let current_context = context.clone();
    let matrix_for_key = matrix.clone();
    let owner_for_key = owner.clone();
    let context_for_key = context.clone();
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
        let nudge = match target.get_attribute("aria-label").as_deref() {
            Some("Drag to stagger") => {
                let row = context_row(&context_for_key);
                TransformNudge::Stagger {
                    row_axis: row.is_some(),
                    index: row.unwrap_or(selected_column),
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
            _ => return,
        };
        key.prevent_default();
        key.stop_propagation();
        // A held key queues one step per repeat; each resolves against the matrix the
        // previous steps produced.
        let _ = EditTicket::begin(
            &runtime_for_key,
            "layout-transform-nudge",
            Some("transform".into()),
            transform_nudge_resolver(matrix_for_key.id.clone(), nudge, delta),
        );
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

/// What a handle key press moves, captured as intent. The step is a delta: the resolver
/// applies it to the matrix accepted when the edit runs.
#[derive(Clone, Debug, PartialEq)]
enum TransformNudge {
    Stagger {
        row_axis: bool,
        index: u32,
    },
    SplayOrigin {
        column: u32,
    },
    SplayAngle {
        column: u32,
        degrees: f64,
        affect: MatrixSplayAffect,
    },
}

fn accepted_matrix<'a>(accepted: &'a AcceptedSnapshot, matrix_id: &str) -> Option<&'a Matrix> {
    accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == matrix_id)
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

fn stagger_by(matrix: &Matrix, row_axis: bool, index: u32, delta: Vec2) -> Matrix {
    let mut next = matrix.clone();
    let offsets = if row_axis {
        &mut next.row_offsets
    } else {
        &mut next.column_offsets
    };
    offsets.resize(offsets.len().max(index as usize + 1), Vec2::default());
    offsets[index as usize].x += delta.x;
    offsets[index as usize].y += delta.y;
    next
}

fn transform_commit(
    transaction_id: String,
    matrix_id: &str,
    operation: EditOperation,
) -> Resolution {
    Resolution::submit_with_transaction_id(vec![matrix_id.to_owned()], transaction_id, operation)
}

/// Resolve a handle key press: the step is added to the accepted matrix (stagger offset) or
/// to the accepted column basis (splay origin and angle).
fn transform_nudge_resolver(matrix_id: String, nudge: TransformNudge, delta: Vec2) -> EditResolver {
    EditResolver::new(
        "layout-transform-nudge",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted_matrix(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let operation = match &nudge {
                TransformNudge::Stagger { row_axis, index } => EditOperation::SetMatrix {
                    matrix: stagger_by(matrix, *row_axis, *index, delta),
                    definitions: None,
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
                            angle: basis.splay_angle + degrees,
                            affect: affect.clone(),
                        },
                    }
                }
            };
            transform_commit(String::new(), &matrix_id, operation)
        },
    )
}

/// The net transform a drag produced, captured when it ends: stagger and angle as deltas
/// from the drag's start, a splay origin as the point it was dropped on.
#[derive(Clone, Debug, PartialEq)]
enum TransformDragResult {
    Stagger {
        row_axis: bool,
        index: u32,
        delta: Vec2,
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
}

fn transform_drag_result(
    drag: &TransformDrag,
    operation: &EditOperation,
) -> Option<TransformDragResult> {
    match (&drag.gesture, operation) {
        (
            TransformGesture::Stagger { row_axis, index },
            EditOperation::SetMatrix { matrix, .. },
        ) => {
            let offset = |matrix: &Matrix| {
                let offsets = if *row_axis {
                    &matrix.row_offsets
                } else {
                    &matrix.column_offsets
                };
                offsets.get(*index as usize).copied().unwrap_or_default()
            };
            let (start, end) = (offset(&drag.matrix), offset(matrix));
            Some(TransformDragResult::Stagger {
                row_axis: *row_axis,
                index: *index,
                delta: Vec2 {
                    x: end.x - start.x,
                    y: end.y - start.y,
                },
            })
        }
        (
            TransformGesture::Splay { column, .. },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Angle { angle, affect },
                ..
            },
        ) => Some(TransformDragResult::SplayAngle {
            column: *column,
            delta_degrees: angle
                - drag
                    .matrix
                    .column_splays
                    .get(*column as usize)
                    .copied()
                    .unwrap_or(0.0),
            affect: affect.clone(),
        }),
        (
            TransformGesture::Origin { column },
            EditOperation::SetMatrixSplay {
                change: MatrixSplayChange::Origin { world },
                ..
            },
        ) => Some(TransformDragResult::SplayOrigin {
            column: *column,
            world: *world,
        }),
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
                TransformDragResult::Stagger {
                    row_axis,
                    index,
                    delta,
                } => EditOperation::SetMatrix {
                    matrix: stagger_by(matrix, *row_axis, *index, *delta),
                    definitions: None,
                },
                TransformDragResult::SplayAngle {
                    column,
                    delta_degrees,
                    affect,
                } => EditOperation::SetMatrixSplay {
                    matrix_id: matrix_id.clone(),
                    column: *column,
                    change: MatrixSplayChange::Angle {
                        angle: matrix
                            .column_splays
                            .get(*column as usize)
                            .copied()
                            .unwrap_or(0.0)
                            + delta_degrees,
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
            };
            transform_commit(transaction_id.clone(), &matrix_id, operation)
        },
    )
}

fn commit_transform_drag(runtime: &Rc<Runtime>, drag: &TransformDrag, operation: &EditOperation) {
    let Some(result) = transform_drag_result(drag, operation) else {
        return;
    };
    let _ = EditTicket::begin(
        runtime,
        "layout-transform-drag",
        Some("transform".into()),
        transform_drag_resolver(drag.matrix.id.clone(), drag.transaction_id.clone(), result),
    );
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
    let x = points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64;
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max)
        + pitch_y * 0.9;
    Vec2 { x, y }
}

#[component]
pub fn LayoutTransformToolbar(
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
                        mount.on_show_properties.call(());
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
                        mount.on_show_properties.call(());
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
    use crate::presentation::objects::matrix_edit_test_support as fixture;
    use crate::runtime::project_name_test_support as support;
    use boardstudio_web_runtime::edit_ticket::Settlement;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn nudge_ticket(runtime: &Rc<Runtime>, delta: Vec2) -> EditTicket {
        EditTicket::begin(
            runtime,
            "layout-transform-nudge",
            Some("transform".into()),
            transform_nudge_resolver(
                fixture::MATRIX_ID.into(),
                TransformNudge::Stagger {
                    row_axis: false,
                    index: 0,
                },
                delta,
            ),
        )
    }

    #[wasm_bindgen_test]
    async fn three_handle_nudges_queued_behind_a_gated_reply_move_the_matrix_three_steps() {
        let runtime = fixture::open_matrix_runtime().await;
        let step = Vec2 { x: 0.0, y: 0.1 };
        let (entered, release) = support::gate_next_core_reply(&runtime);
        let first = nudge_ticket(&runtime, step);
        support::drive_pending(&runtime);
        entered.await.expect("the first nudge reached Core");
        let second = nudge_ticket(&runtime, step);
        let third = nudge_ticket(&runtime, step);
        support::drive_pending(&runtime);
        release.send(()).expect("release the held reply");
        fixture::settle_ticket(&runtime, &third).await;

        for ticket in [&first, &second, &third] {
            assert!(matches!(ticket.settlement(true), Settlement::Landed { .. }));
        }
        let offset = fixture::accepted_matrix(&runtime).column_offsets[0];
        assert!(
            (offset.y - 0.3).abs() < 1e-9,
            "three steps of 0.1 compose to 0.3, got {}",
            offset.y
        );
        assert_eq!(offset.x, 0.0);
    }

    #[wasm_bindgen_test]
    async fn a_drag_commit_applies_its_net_transform_to_the_accepted_matrix() {
        let runtime = fixture::open_matrix_runtime().await;
        // Another stagger edit lands while the drag was in flight; the drag's net delta
        // composes with it instead of restoring the matrix the drag started from.
        let other = nudge_ticket(&runtime, Vec2 { x: 0.0, y: 1.0 });
        fixture::settle_ticket(&runtime, &other).await;
        let drag = EditTicket::begin(
            &runtime,
            "layout-transform-drag",
            Some("transform".into()),
            transform_drag_resolver(
                fixture::MATRIX_ID.into(),
                "layout-transform-test".into(),
                TransformDragResult::Stagger {
                    row_axis: false,
                    index: 0,
                    delta: Vec2 { x: 2.0, y: 0.0 },
                },
            ),
        );
        fixture::settle_ticket(&runtime, &drag).await;
        assert!(matches!(drag.settlement(true), Settlement::Landed { .. }));
        let offset = fixture::accepted_matrix(&runtime).column_offsets[0];
        assert_eq!(offset, Vec2 { x: 2.0, y: 1.0 });
    }
}
