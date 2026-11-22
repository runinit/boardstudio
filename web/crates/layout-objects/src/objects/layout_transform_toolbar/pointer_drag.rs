//! Browser wiring for the matrix and part transform drags.
use super::*;

#[derive(Clone)]
pub(super) struct RailDrag {
    pub(super) pointer_id: i32,
    pub(super) capture: SvgElement,
    pub(super) transaction_id: String,
    pub(super) owner: LayoutOwnerIdentity,
    pub(super) context: TreeContext,
    pub(super) tool: LayoutTransformTool,
    pub(super) interaction: CanvasInteraction,
}

impl RailDrag {
    pub(super) fn interaction(pointer: i32) -> CanvasInteraction {
        let mut interaction = CanvasInteraction::default();
        interaction.begin_transform(pointer);
        interaction
    }

    pub(super) fn submit_preview(
        &self,
        runtime: &Rc<Runtime>,
        target_id: &str,
        operation: EditOperation,
    ) {
        runtime.submit(Event::PreviewEdit {
            operation_id: runtime.operation(),
            transaction_id: self.transaction_id.clone(),
            target_ids: vec![target_id.to_owned()],
            operation,
        });
    }

    pub(super) fn clear_preview(&self, runtime: &Rc<Runtime>) {
        if let (Some(token), Some(revision), Some(scope)) = (
            self.owner.token,
            self.owner.revision,
            self.owner.scope.as_ref(),
        ) {
            runtime.submit(Event::ClearPreview {
                operation_id: runtime.operation(),
                token,
                revision,
                board_id: scope.board_id.clone(),
                transaction_id: self.transaction_id.clone(),
            });
        }
    }

    /// Take the drag out of its slot before calling this: releasing capture can
    /// dispatch lostpointercapture.
    pub(super) fn release(&self, arbiter: &CanvasInteractionArbiter) {
        if self.capture.has_pointer_capture(self.pointer_id) {
            let _ = self.capture.release_pointer_capture(self.pointer_id);
        }
        arbiter.release(CanvasInteractionOwner::MatrixTransform);
    }
}

pub(super) fn capture_rail_pointer(
    pointer: &web_sys::PointerEvent,
    selector: &str,
    arbiter: &CanvasInteractionArbiter,
) -> Option<SvgElement> {
    let capture = pointer
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|target| target.closest(selector).ok().flatten())
        .and_then(|target| target.dyn_into::<SvgElement>().ok())
        .filter(|capture| capture.set_pointer_capture(pointer.pointer_id()).is_ok());
    let Some(capture) = capture else {
        arbiter.release(CanvasInteractionOwner::MatrixTransform);
        return None;
    };
    pointer.prevent_default();
    pointer.stop_propagation();
    let focus_options = web_sys::FocusOptions::new();
    focus_options.set_prevent_scroll(true);
    let _ = capture.focus_with_options(&focus_options);
    Some(capture)
}

/// Route move and up to the captured pointer. A stale owner cancels before
/// either handler can sample or commit.
pub(super) fn rail_drag_events<D: AsRef<RailDrag> + Clone + 'static>(
    drag: &Rc<RefCell<Option<D>>>,
    runtime: &Rc<Runtime>,
    workspace: Signal<&'static str>,
    generation: Signal<u64>,
    selection: Signal<Option<ScopedTreeContext>>,
    tool: LayoutTransformTool,
    cancel: impl Fn() + Clone + 'static,
) -> impl Fn(PointerEvent, bool) -> Option<(web_sys::PointerEvent, D)> + Clone + 'static {
    let drag = drag.clone();
    let runtime = runtime.clone();
    move |event: PointerEvent, finishing| {
        let pointer = event.data().try_as_web_event()?;
        let active = drag
            .borrow()
            .as_ref()
            .filter(|active| active.as_ref().pointer_id == pointer.pointer_id())
            .cloned()?;
        pointer.prevent_default();
        pointer.stop_propagation();
        let held = active.as_ref();
        if !transform_owner_is_current(
            &runtime,
            workspace,
            generation,
            selection,
            &held.owner,
            &held.context,
        ) || held.tool != tool
            || (finishing && pointer.type_() != "pointerup")
        {
            cancel();
            return None;
        }
        Some((pointer, active))
    }
}

pub(super) fn cancel_rail_pointer(
    event: PointerEvent,
    drag: &RefCell<Option<impl AsRef<RailDrag>>>,
    feedback: Signal<FeedbackState>,
    popup_at: Signal<Option<(f64, f64)>>,
    cancel: impl FnOnce(),
) {
    let Some(pointer) = event.data().try_as_web_event() else {
        return;
    };
    if drag
        .borrow()
        .as_ref()
        .is_some_and(|drag| drag.as_ref().pointer_id == pointer.pointer_id())
    {
        pointer.prevent_default();
        pointer.stop_propagation();
        dismiss_drag_feedback(feedback, popup_at);
        cancel();
    }
}

fn dismiss_drag_feedback(
    mut feedback: Signal<FeedbackState>,
    mut popup_at: Signal<Option<(f64, f64)>>,
) {
    feedback.set(advance_feedback(&feedback(), FeedbackEvent::Dismiss));
    popup_at.set(None);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn use_rail_drag_cancel<D: AsRef<RailDrag> + 'static>(
    drag: &Rc<RefCell<Option<D>>>,
    owner: &LayoutOwnerIdentity,
    context: &TreeContext,
    tool: LayoutTransformTool,
    selected_context: Signal<Option<ScopedTreeContext>>,
    scope_generation: Signal<u64>,
    svg: &LayoutTransformSvg,
    arbiter: &CanvasInteractionArbiter,
    feedback: Signal<FeedbackState>,
    popup_at: Signal<Option<(f64, f64)>>,
    on_finish: EventHandler<()>,
    cancel: impl Fn() + Clone + 'static,
) -> impl Fn() + Clone + 'static {
    use_drop(cancel.clone());
    let escape = {
        let cancel = cancel.clone();
        move || {
            dismiss_drag_feedback(feedback, popup_at);
            cancel();
            on_finish.call(());
        }
    };
    use_canvas_escape(svg, {
        let drag = drag.clone();
        let arbiter = arbiter.clone();
        let escape = escape.clone();
        move |event| {
            let claims = rail_escape_claims(event, drag.borrow().is_some(), &arbiter);
            if claims {
                escape();
            }
            claims
        }
    });
    let drag = drag.clone();
    use_effect(use_reactive(
        (owner, context, &tool),
        move |(owner, context, tool)| {
            let _ = selected_context();
            let _ = scope_generation();
            if drag.borrow().as_ref().is_some_and(|active| {
                let held = active.as_ref();
                held.owner != owner || held.context != context || held.tool != tool
            }) {
                cancel();
            }
        },
    ));
    escape
}
