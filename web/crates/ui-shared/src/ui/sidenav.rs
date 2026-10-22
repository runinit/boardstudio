//! In-flow (`collapsible: None`) look adapted from rust-ui `app_crates/registry/src/ui/sidenav.rs`
//! at `7120636e68556ddd023a4ec8e083663c7b745318` (MIT, see `web/assets/licenses/rust-ui-MIT.txt`).
//! No fixed viewport wrapper, scripts or private preference store. The caller supplies the existing
//! `panels::use_panel_settings` signal; resize changes only its width, preserving saved modes.

#[derive(Clone, Copy, Debug)]
struct ResizeDrag {
    pointer_id: i32,
    start_x: f64,
    start_width: f64,
}

impl ResizeDrag {
    fn requested_width(self, pointer_id: i32, x: f64, grows_right: bool) -> Option<f64> {
        (self.pointer_id == pointer_id && x.is_finite())
            .then(|| self.start_width + (x - self.start_x) * if grows_right { 1.0 } else { -1.0 })
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::*;

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::ResizeDrag;
    use crate::panels::{PanelSettings, PanelSide};
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use wasm_bindgen::JsCast;

    /// The in-flow Sidenav form. Visibility/collapsing and available workspace geometry belong to
    /// the integrating shell. `available_width` may reserve space for its canvas and opposite dock.
    #[component]
    pub fn Sidenav(
        side: PanelSide,
        mut settings: Signal<PanelSettings>,
        aria_label: String,
        #[props(default = ReadSignal::new(Signal::new(f64::MAX)))] available_width: ReadSignal<f64>,
        #[props(into, optional)] class: Option<String>,
        children: Element,
    ) -> Element {
        let fallback = if side == PanelSide::Objects {
            260.0
        } else {
            360.0
        };
        let mut preview = use_signal(|| None::<f64>);
        let drag = use_hook(|| std::rc::Rc::new(std::cell::Cell::new(None::<ResizeDrag>)));
        let handle =
            use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(None::<web_sys::Element>)));
        let width = side.resize_width(
            preview().or(settings().width).unwrap_or(fallback),
            available_width(),
        );
        let maximum = side.resize_width(f64::MAX, available_width());
        let minimum = side.minimum_width();
        let grows_right = side == PanelSide::Objects;
        let mut commit = move |width| {
            let mut next = settings();
            next.width = Some(side.resize_width(width, available_width()));
            settings.set(next);
        };
        let mut finish = {
            let drag = drag.clone();
            let handle = handle.clone();
            move |pointer_id| {
                if drag
                    .get()
                    .is_some_and(|active| active.pointer_id == pointer_id)
                {
                    drag.set(None);
                    if let Some(width) = preview() {
                        commit(width);
                    }
                    preview.set(None);
                    if let Some(handle) = handle.borrow().as_ref()
                        && handle.has_pointer_capture(pointer_id)
                    {
                        let _ = handle.release_pointer_capture(pointer_id);
                    }
                }
            }
        };
        use_drop({
            let drag = drag.clone();
            let handle = handle.clone();
            move || {
                if let Some(active) = drag.take()
                    && let Some(handle) = handle.borrow().as_ref()
                {
                    let _ = handle.release_pointer_capture(active.pointer_id);
                }
            }
        });
        let class = tw_merge::tw_merge!(
            "relative flex flex-col shrink-0 min-h-0 h-full rounded-xl bg-sidenav text-sidenav-foreground shadow-sm",
            class.as_deref().unwrap_or("")
        );
        let down_drag = drag.clone();
        let move_drag = drag.clone();
        let mut up = finish.clone();
        let mut cancel = finish.clone();
        rsx! {
            aside {
                "data-slot": "sidenav", "data-side": side.storage_side(), "data-collapsible": "none",
                role: "complementary", aria_label: aria_label.clone(), class,
                style: "width:{width}px",
                div { "data-slot": "sidenav-content", class: "flex flex-col min-h-0 flex-1 overflow-auto p-4", {children} }
                div {
                    "data-slot": "sidenav-resize", role: "separator", aria_label: "Resize {aria_label}",
                    aria_orientation: "vertical", aria_valuemin: "{minimum}", aria_valuemax: "{maximum}", aria_valuenow: "{width}",
                    tabindex: 0,
                    class: "absolute inset-y-0 w-3 cursor-col-resize touch-none select-none rounded-md hover:bg-sidenav-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidenav-ring [@media(pointer:coarse)]:w-control-touch",
                    style: if grows_right { "right:0" } else { "left:0" },
                    onkeydown: move |event: KeyboardEvent| {
                        let step = if event.modifiers().shift() { 40.0 } else { 20.0 };
                        let requested = match event.key() {
                            Key::ArrowRight => width + if grows_right { step } else { -step },
                            Key::ArrowLeft => width + if grows_right { -step } else { step },
                            Key::Home => minimum,
                            Key::End => maximum,
                            _ => return,
                        };
                        event.prevent_default(); event.stop_propagation();
                        commit(side.resize_width(requested, available_width()));
                    },
                    onpointerdown: move |event: PointerEvent| {
                        let Some(raw) = event.data().try_as_web_event() else { return; };
                        if raw.button() != 0 || down_drag.get().is_some() { return; }
                        let Some(target) = raw.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) else { return; };
                        if target.set_pointer_capture(raw.pointer_id()).is_err() { return; }
                        event.prevent_default(); event.stop_propagation();
                        let _ = target.dyn_ref::<web_sys::HtmlElement>().map(|target| target.focus());
                        *handle.borrow_mut() = Some(target);
                        down_drag.set(Some(ResizeDrag { pointer_id: raw.pointer_id(), start_x: f64::from(raw.client_x()), start_width: width }));
                    },
                    onpointermove: move |event: PointerEvent| {
                        let Some(raw) = event.data().try_as_web_event() else { return; };
                        if let Some(requested) = move_drag.get().and_then(|active| active.requested_width(raw.pointer_id(), f64::from(raw.client_x()), grows_right)) {
                            preview.set(Some(side.resize_width(requested, available_width())));
                        }
                    },
                    onpointerup: move |event: PointerEvent| up(event.pointer_id()),
                    onpointercancel: move |event: PointerEvent| cancel(event.pointer_id()),
                    onlostpointercapture: move |event: PointerEvent| finish(event.pointer_id()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ResizeDrag;

    #[test]
    fn resize_delta_is_local_to_the_active_pointer_and_side() {
        let drag = ResizeDrag {
            pointer_id: 7,
            start_x: 700.0,
            start_width: 300.0,
        };
        assert_eq!(drag.requested_width(7, 750.0, true), Some(350.0));
        assert_eq!(drag.requested_width(7, 750.0, false), Some(250.0));
        assert_eq!(drag.requested_width(8, 750.0, true), None);
        assert_eq!(drag.requested_width(7, f64::NAN, true), None);
        assert_eq!(drag.requested_width(7, f64::INFINITY, false), None);
    }
}
