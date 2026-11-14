//! Look copied from rust-ui: `app_crates/registry/src/ui/popover.rs` at
//! <https://github.com/rust-ui/ui> commit `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see
//! `web/assets/licenses/rust-ui-MIT.txt`). Upstream changes are pulled in by hand.
//!
//! Behaviour is `dioxus_primitives::popover` (rev `9a758255`); the wrapper shape follows that
//! crate's styled preview component. Edits to the upstream file:
//! - upstream's own state, inline `<script>` and document listeners are replaced by the primitive;
//! - rust-ui classes go on the primitive's parts through `merge_attributes`, so a caller's `class` is
//!   appended, not `tw_merge`d;
//! - every part carries a `data-slot` (theme contract section 9);
//! - upstream's CSS anchor positioning is replaced by placement classes keyed to the primitive's `data-side`;
//! - built-in trigger and floating-content borders removed, cards style (theme contract section 8); focus rings kept;
//! - control height reads the shared theme-independent metric.

use dioxus::prelude::*;
use dioxus_primitives::popover::{
    self, PopoverContentProps, PopoverRootProps, PopoverTriggerProps,
};
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};

#[component]
pub fn Popover(props: PopoverRootProps) -> Element {
    let base = attributes!(div { class: "relative inline-block", "data-slot": "popover" });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        popover::PopoverRoot {
            is_modal: props.is_modal,
            open: props.open,
            default_open: props.default_open,
            on_open_change: props.on_open_change,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn PopoverTrigger(props: PopoverTriggerProps) -> Element {
    let base = attributes!(button {
        class: "px-4 py-2 h-control inline-flex justify-center items-center text-sm font-medium whitespace-nowrap rounded-md transition-colors w-fit focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&_svg:not(:last-child)]:mr-2 [&_svg:not(:first-child)]:ml-2 bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
        "data-slot": "popover-trigger",
    });
    let merged = merge_attributes(vec![base, props.attributes]);
    rsx! {
        popover::PopoverTrigger { attributes: merged, {props.children} }
    }
}

#[component]
pub fn PopoverContent(props: PopoverContentProps) -> Element {
    let base = attributes!(div { "data-slot": "popover-content" });
    let merged = merge_attributes(vec![base, props.attributes]);
    let class = format!(
        "absolute z-50 p-4 rounded-md bg-card text-card-foreground shadow-md w-[250px] data-[side=bottom]:top-full data-[side=bottom]:mt-2 data-[side=top]:bottom-full data-[side=top]:mb-2 data-[side=bottom]:left-1/2 data-[side=bottom]:-translate-x-1/2 data-[side=top]:left-1/2 data-[side=top]:-translate-x-1/2 data-[side=right]:left-full data-[side=right]:ml-2 data-[side=left]:right-full data-[side=left]:mr-2 data-[side=right]:top-1/2 data-[side=right]:-translate-y-1/2 data-[side=left]:top-1/2 data-[side=left]:-translate-y-1/2 transition-opacity duration-150 data-[state=closed]:opacity-0 data-[state=closed]:pointer-events-none data-[state=open]:opacity-100 {}",
        props.class.unwrap_or_default()
    );
    rsx! {
        popover::PopoverContent {
            class,
            id: props.id,
            side: props.side,
            align: props.align,
            attributes: merged,
            {props.children}
        }
    }
}

#[component]
pub fn PopoverTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge::tw_merge!(
        "leading-none font-medium mb-3",
        class.as_deref().unwrap_or("")
    );
    rsx! { h3 { "data-slot": "popover-title", class: "{merged}", {children} } }
}

#[component]
pub fn PopoverDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge::tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! { p { "data-slot": "popover-description", class: "{merged}", {children} } }
}

/// Side margin kept between a lane-anchored dialog and the viewport edge.
const CLAMPED_DIALOG_MARGIN: f64 = 8.0;

/// Entry-settle re-pin bound: the workbench entry runs 280ms (`shell.css`
/// `workbench-tool-enter`). The settle loop stops at this deadline even if the
/// track transform never reports settled (persistent non-`none`/identity).
const SETTLE_MAX_MS: f64 = 280.0;

/// Self-scheduling entry-settle frame loop, shared with its own callback.
type SettleFrame = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// Live listeners pinning one dialog while it is open.
struct ClampedPlacementListeners {
    scroll_target: Option<web_sys::Element>,
    scroll_callback: Option<Closure<dyn FnMut()>>,
    resize_callback: Option<Closure<dyn FnMut()>>,
    row_observer: Option<web_sys::ResizeObserver>,
    row_callback: Option<Closure<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>>,
    settle_frame: Option<SettleFrame>,
    settle_cancelled: Option<Rc<std::cell::Cell<bool>>>,
    settle_raf: Option<Rc<std::cell::Cell<Option<i32>>>>,
}

fn remove_clamped_placement_listeners(listeners: &Rc<RefCell<Option<ClampedPlacementListeners>>>) {
    if let Some(held) = listeners.borrow_mut().take() {
        if let Some(cancelled) = held.settle_cancelled {
            cancelled.set(true);
        }
        // Cancel the outstanding RAF before dropping its closure: a dropped
        // wasm-bindgen closure throws if JavaScript still calls it.
        if let Some(pending_cell) = held.settle_raf
            && let Some(pending) = pending_cell.take()
            && let Some(window) = web_sys::window()
        {
            let _ = window.cancel_animation_frame(pending);
        }
        // Break the frame Rc -> closure -> Rc cycle so the loop frees.
        if let Some(frame) = held.settle_frame {
            frame.borrow_mut().take();
        }
        if let Some(observer) = held.row_observer {
            observer.disconnect();
        }
        if let (Some(target), Some(callback)) = (held.scroll_target, held.scroll_callback) {
            let _ = target
                .remove_event_listener_with_callback("scroll", callback.as_ref().unchecked_ref());
        }
        if let Some(callback) = held.resize_callback
            && let Some(window) = web_sys::window()
        {
            let _ = window
                .remove_event_listener_with_callback("resize", callback.as_ref().unchecked_ref());
        }
    }
}

/// Pin one fixed lane-anchored dialog under its trigger, clamped to the viewport.
///
/// CSS `anchor()` follows the trigger's viewport position, but lane-scroll
/// movement reaches the used position post-clamp (observed: computed `left`
/// stays clamped while the used position tracks scroll). Worse, a live
/// `position-anchor` keeps the dialog glued to its scrolling trigger even
/// with inline pixel offsets (observed: used x drifts by exactly the scroll
/// delta past the inline `left`; hit-testing then reports canvas content
/// over the dialog and stays stale until the next reposition). While `open`,
/// this hook therefore bypasses anchoring entirely: it writes inline `left`
/// (untransformed trigger x clamped to 8px side margins), inline `top`
/// (untransformed trigger bottom plus `gap`) and inline `position-anchor:
/// none`, refreshing on lane scroll, window resize, row resize (parent-only
/// reflow) and entry settle. On close it removes all three inline styles so
/// the stylesheet anchor rules apply again. Triggers outside a
/// `.workbench-tool-row` (dock/floating LayerPanel) are left alone: their
/// content is not fixed-positioned, so the hook is a no-op for them.
pub fn use_trigger_clamped_placement(trigger_id: String, popup_id: String, open: bool, gap: f64) {
    let listeners: Rc<RefCell<Option<ClampedPlacementListeners>>> =
        use_hook(|| Rc::new(RefCell::new(None)));
    {
        let listeners = listeners.clone();
        use_effect(use_reactive((&open,), move |(open,)| {
            remove_clamped_placement_listeners(&listeners);
            if !open {
                clear_dialog_position(&popup_id);
                return;
            }
            // Lane-only: dock/floating LayerPanel content is not fixed, so the
            // hook stays a no-op there (review scoping for the shared seam).
            let lane_row = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(&trigger_id))
                .and_then(|trigger| trigger.closest(".workbench-tool-row").ok().flatten());
            if lane_row.is_none() {
                clear_dialog_position(&popup_id);
                return;
            }
            position_clamped_dialog(&trigger_id, &popup_id, gap);
            let scroll_target = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id(&trigger_id))
                .and_then(|trigger| trigger.closest(".workbench-tool-track").ok().flatten());
            let mut held = ClampedPlacementListeners {
                scroll_target: None,
                scroll_callback: None,
                resize_callback: None,
                row_observer: None,
                row_callback: None,
                settle_frame: None,
                settle_cancelled: None,
                settle_raf: None,
            };
            if let Some(target) = scroll_target {
                let refresh_trigger_id = trigger_id.clone();
                let refresh_popup_id = popup_id.clone();
                let callback = Closure::wrap(Box::new(move || {
                    position_clamped_dialog(&refresh_trigger_id, &refresh_popup_id, gap);
                }) as Box<dyn FnMut()>);
                if target
                    .add_event_listener_with_callback("scroll", callback.as_ref().unchecked_ref())
                    .is_ok()
                {
                    held.scroll_target = Some(target);
                    held.scroll_callback = Some(callback);
                }
            }
            if let Some(window) = web_sys::window() {
                let refresh_trigger_id = trigger_id.clone();
                let refresh_popup_id = popup_id.clone();
                let callback = Closure::wrap(Box::new(move || {
                    position_clamped_dialog(&refresh_trigger_id, &refresh_popup_id, gap);
                }) as Box<dyn FnMut()>);
                if window
                    .add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref())
                    .is_ok()
                {
                    held.resize_callback = Some(callback);
                }
            }
            // Parent-only reflow moves the trigger with no window resize (the
            // row shrinks; the trigger slides). Observe the row's size so the
            // open dialog follows row/parent movement.
            if let Some(row) = lane_row {
                let refresh_trigger_id = trigger_id.clone();
                let refresh_popup_id = popup_id.clone();
                let callback = Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(
                    move |_, _| {
                        position_clamped_dialog(&refresh_trigger_id, &refresh_popup_id, gap);
                    },
                );
                if let Ok(observer) =
                    web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref())
                {
                    observer.observe(&row);
                    held.row_observer = Some(observer);
                    held.row_callback = Some(callback);
                }
            }
            // Entry settle: the open effect can run while the track still
            // carries its entry transform. Re-pin every frame until the track
            // settles so a transformed first write cannot stick past cancel.
            // The write itself uses untransformed coordinates, so this loop is
            // a fallback bounded by the 280ms entry deadline, not the anchoring
            // itself. The outstanding RAF ID is retained and cancelled before
            // its closure is dropped (close/reopen/unmount).
            {
                use std::cell::Cell;
                let cancelled = Rc::new(Cell::new(false));
                let loop_cancelled = cancelled.clone();
                let pending: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
                let loop_pending = pending.clone();
                let loop_trigger_id = trigger_id.clone();
                let loop_popup_id = popup_id.clone();
                let frame: SettleFrame = Rc::new(RefCell::new(None));
                let frame_next = frame.clone();
                let start_ms = js_sys::Date::now();
                let closure = Closure::wrap(Box::new(move || {
                    // This frame fired; no longer outstanding.
                    loop_pending.set(None);
                    if loop_cancelled.get() {
                        return;
                    }
                    position_clamped_dialog(&loop_trigger_id, &loop_popup_id, gap);
                    if track_entry_settled(&loop_trigger_id) {
                        return;
                    }
                    // Re-pin bound: stop at the entry deadline even if the
                    // transform never settles (persistent non-`none`/identity).
                    if js_sys::Date::now() - start_ms >= SETTLE_MAX_MS {
                        return;
                    }
                    if let Some(window) = web_sys::window()
                        && let Some(next) = frame_next.borrow().as_ref()
                        && let Ok(id) =
                            window.request_animation_frame(next.as_ref().unchecked_ref())
                    {
                        loop_pending.set(Some(id));
                    }
                }) as Box<dyn FnMut()>);
                if let Some(window) = web_sys::window()
                    && let Ok(id) = window.request_animation_frame(closure.as_ref().unchecked_ref())
                {
                    pending.set(Some(id));
                }
                *frame.borrow_mut() = Some(closure);
                held.settle_frame = Some(frame);
                held.settle_cancelled = Some(cancelled);
                held.settle_raf = Some(pending);
            }
            *listeners.borrow_mut() = Some(held);
        }));
    }
    use_drop(move || remove_clamped_placement_listeners(&listeners));
}

/// Translation carried by the trigger's entry track, if any. The entry
/// animates `translateX` on `.workbench-tool-track`; the trigger's bounding
/// rect includes that transform while the entry runs. Subtracting it pins the
/// dialog to the settled (untransformed) trigger at the first frame, before
/// the cancellation observer finishes the entry.
fn track_translation(trigger: &web_sys::Element) -> (f64, f64) {
    let Some(track) = trigger.closest(".workbench-tool-track").ok().flatten() else {
        return (0.0, 0.0);
    };
    let transform = web_sys::window()
        .and_then(|window| window.get_computed_style(&track).ok().flatten())
        .and_then(|style| style.get_property_value("transform").ok())
        .unwrap_or_default();
    let transform = transform.trim();
    if transform.is_empty() || transform == "none" {
        return (0.0, 0.0);
    }
    let numbers: Vec<f64> = transform
        .trim_start_matches("matrix3d(")
        .trim_start_matches("matrix(")
        .trim_end_matches(')')
        .split(',')
        .filter_map(|part| part.trim().parse::<f64>().ok())
        .collect();
    if transform.starts_with("matrix3d(") && numbers.len() == 16 {
        (numbers[12], numbers[13])
    } else if transform.starts_with("matrix(") && numbers.len() == 6 {
        (numbers[4], numbers[5])
    } else {
        (0.0, 0.0)
    }
}

fn track_entry_settled(trigger_id: &str) -> bool {
    let Some(trigger) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(trigger_id))
    else {
        return true;
    };
    let Some(track) = trigger.closest(".workbench-tool-track").ok().flatten() else {
        return true;
    };
    web_sys::window()
        .and_then(|window| window.get_computed_style(&track).ok().flatten())
        .and_then(|style| style.get_property_value("transform").ok())
        .is_none_or(|transform| {
            let transform = transform.trim();
            transform.is_empty() || transform == "none"
        })
}

fn position_clamped_dialog(trigger_id: &str, popup_id: &str, gap: f64) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(trigger) = document.get_element_by_id(trigger_id) else {
        return;
    };
    let Some(popup) = document.get_element_by_id(popup_id) else {
        return;
    };
    let viewport = window
        .inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .unwrap_or(0.0);
    if viewport <= 0.0 {
        return;
    }
    let trigger_box = trigger.get_bounding_client_rect();
    let (tx, ty) = track_translation(&trigger);
    let anchor_x = trigger_box.x() - tx;
    let anchor_bottom = trigger_box.bottom() - ty;
    let popup_width = popup.get_bounding_client_rect().width();
    let max_left = (viewport - popup_width - CLAMPED_DIALOG_MARGIN).max(CLAMPED_DIALOG_MARGIN);
    let left = anchor_x.clamp(CLAMPED_DIALOG_MARGIN, max_left);
    let top = anchor_bottom + gap;
    if let Ok(popup) = popup.dyn_into::<web_sys::HtmlElement>() {
        let _ = popup.style().set_property("left", &format!("{left}px"));
        let _ = popup.style().set_property("top", &format!("{top}px"));
        let _ = popup.style().set_property("position-anchor", "none");
    }
}

fn clear_dialog_position(popup_id: &str) {
    let Some(popup) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id(popup_id))
    else {
        return;
    };
    if let Ok(popup) = popup.dyn_into::<web_sys::HtmlElement>() {
        let _ = popup.style().remove_property("left");
        let _ = popup.style().remove_property("top");
        let _ = popup.style().remove_property("position-anchor");
    }
}
