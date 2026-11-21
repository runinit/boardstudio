//! Ticket 06: the value popup and commit toast chrome shared by the rail overlays.
use super::*;

/// Ticket 06: the commit toast lingers for 1.8 s, matching the prototype.
pub(super) const TOAST_TIMEOUT_MS: u32 = 1800;

#[cfg(test)]
pub(super) struct ToastExpiryProbe {
    pub(super) release: futures_channel::oneshot::Receiver<()>,
    pub(super) finished: futures_channel::oneshot::Sender<()>,
}

#[cfg(test)]
thread_local! {
    pub(super) static NEXT_TOAST_EXPIRY_PROBE: RefCell<Option<ToastExpiryProbe>> = const { RefCell::new(None) };
}

/// Arm one commit-toast expiry timer. The overlay remounts on every scope and
/// selection change, so the timer can outlive its component: it only touches
/// its signals while they are still alive and stays silent otherwise.
pub(super) fn arm_commit_toast_expiry(
    mut feedback: Signal<FeedbackState>,
    generation: Signal<u64>,
    seen: u64,
) {
    let timeout_ms = TOAST_TIMEOUT_MS;
    #[cfg(test)]
    let mut probe = NEXT_TOAST_EXPIRY_PROBE.with(|next| next.borrow_mut().take());
    #[cfg(test)]
    let timeout_ms = if probe.is_some() { 1 } else { timeout_ms };
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(timeout_ms).await;
        #[cfg(test)]
        if let Some(probe) = probe.as_mut() {
            // The test releases the short timer after observing the overlay unmount.
            if (&mut probe.release).await.is_err() {
                return;
            }
        }
        let live = generation
            .try_peek()
            .map(|current| *current == seen)
            .unwrap_or(false);
        if live && let Ok(mut state) = feedback.try_write() {
            *state = advance_feedback(&state, FeedbackEvent::Expire);
        }
        #[cfg(test)]
        if let Some(probe) = probe {
            let _ = probe.finished.send(());
        }
    });
}

/// Ticket 06: estimated screen-px width of one chrome text line.
pub(super) fn feedback_line_width(text: &str, size_px: f64) -> f64 {
    text.chars().count() as f64 * size_px * 0.58
}

/// Ticket 06: the value popup panel for resolved content, or `None` without
/// a position. `top_left` is world; `px` is world units per screen px.
pub(super) fn feedback_popup_panel(
    popup: &boardstudio_web_ui_model::transform_feedback::ValuePopup,
    follow: Option<(f64, f64)>,
    rail: Option<(f64, f64, f64, f64)>,
    px: f64,
) -> Option<Element> {
    let mut rows: Vec<(String, f64, &'static str, bool)> = vec![
        (popup.name.clone(), 11.0, "var(--muted-foreground)", false),
        (popup.value.clone(), 15.0, "inherit", true),
    ];
    if let Some(delta) = popup.delta.as_ref() {
        rows.push((delta.clone(), 12.0, "var(--primary)", false));
    }
    rows.push((
        "Drag the handle, or type in the Inspector · Esc: close".to_owned(),
        10.5,
        "var(--muted-foreground)",
        false,
    ));
    let panel_w = rows
        .iter()
        .map(|(text, size, _, _)| feedback_line_width(text, *size))
        .fold(0.0_f64, f64::max)
        + 16.0;
    // A following drag anchors its own top-left; a hover or focus centres
    // the panel above the rail.
    let top_left: Option<(f64, f64)> = follow
        .or_else(|| rail.map(|(x, y, w, h)| (x + w / 2.0 - panel_w * px / 2.0, y + h + 8.0 * px)));
    let (panel_x, panel_y) = top_left?;
    Some(rsx! {
        g {
            "data-feedback-popup": "true", class: "fill-foreground",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            "aria-hidden": "true",
            {feedback_panel_contents(panel_w, rows)}
        }
    })
}

/// Ticket 06: the value popup (when `show_popup`) docked above `rail` or following the drag
/// at `follow`, then the commit toast at the top centre of the canvas `view` (x, y, width).
/// Both are view-only: `pointer-events: none`, no tabindex, no document write.
pub(super) fn feedback_chrome(
    state: &FeedbackState,
    show_popup: bool,
    follow: Option<(f64, f64)>,
    rail: Option<&handle_rail::WorldRailLayout>,
    px: f64,
    (view_x, view_y, view_width): (f64, f64, f64),
) -> Element {
    let popup = state
        .popup
        .as_ref()
        .filter(|_| show_popup)
        .and_then(|popup| {
            feedback_popup_panel(
                popup,
                follow,
                rail.map(|layout| (layout.rail.x, layout.rail.y, layout.rail.w, layout.rail.h)),
                px,
            )
        });
    let toast = state
        .toast
        .as_ref()
        .map(|toast| feedback_toast_panel(toast, view_x, view_y, view_width, px));
    rsx! {
        {popup}
        {toast}
    }
}

/// Ticket 06: the commit toast panel at the top centre of the canvas view.
/// The value renders bold, the rest plain, inside one `role="status"` group
/// that never takes focus.
pub(super) fn feedback_toast_panel(
    toast: &boardstudio_web_ui_model::transform_feedback::CommitToast,
    view_x: f64,
    view_y: f64,
    view_width: f64,
    px: f64,
) -> Element {
    let width_px = feedback_line_width(&toast.text(), 12.5) + 24.0;
    let panel_x = view_x + view_width / 2.0 - width_px * px / 2.0;
    let panel_y = -view_y - 8.0 * px;
    let value = toast.value.clone();
    let rest = format!(" ({}) · 1 Undo step", toast.delta);
    rsx! {
        g {
            "data-feedback-toast": "true", class: "fill-foreground",
            role: "status",
            transform: "translate({panel_x} {panel_y}) scale({px} {-px})",
            style: "pointer-events:none",
            rect {
                x: "0",
                y: "0",
                width: "{width_px}",
                height: "33.5",
                rx: "8",
                style: "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 6px 18px rgb(24 35 49 / 18%))",
            }
            text {
                x: "12",
                y: "20.5",
                style: "font-size:12.5px",
                tspan { style: "font-weight:700", "{value}" }
                tspan { "{rest}" }
            }
        }
    }
}

/// Ticket 06: the rect plus left-aligned text rows of one view-only chrome
/// panel, drawn in a y-down px group the caller positions. Rows carry
/// (text, size px, fill, mono-600). Static panels only: reduced motion drops
/// travel, and there is no travel to drop.
pub(super) fn feedback_panel_contents(
    width_px: f64,
    rows: Vec<(String, f64, &'static str, bool)>,
) -> Element {
    let pad = 8.0;
    let height_px: f64 = rows.iter().map(|(_, size, _, _)| size * 1.4).sum::<f64>() + pad * 2.0;
    let mut baseline = pad;
    let placed: Vec<(String, f64, &'static str, String, f64)> = rows
        .into_iter()
        .map(|(text, size, fill, mono)| {
            baseline += size;
            let y = baseline;
            baseline += size * 0.4;
            let style = if mono {
                "font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-weight:600".to_owned()
            } else {
                String::new()
            };
            (text, size, fill, style, y)
        })
        .collect();
    rsx! {
        rect {
            x: "0",
            y: "0",
            width: "{width_px}",
            height: "{height_px}",
            rx: "8",
            style: "fill:var(--card);stroke:var(--border-strong);stroke-width:1px;vector-effect:non-scaling-stroke;filter:drop-shadow(0 2px 4px rgb(24 35 49 / 12%))",
        }
        for (text, size, fill, style, y) in placed {
            text {
                x: "{pad}",
                y: "{y}",
                style: "font-size:{size}px;fill:{fill};{style}",
                "{text}"
            }
        }
    }
}
