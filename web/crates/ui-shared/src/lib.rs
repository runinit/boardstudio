//! Page UI shared by several workspaces: panels, canvas layers, the layout camera,
//! footprint graphics, geometry scripts, model import and board-reference effects.
//! Depends on `ui-model` only, never on a workspace crate or the page shell.
//!
//! Browser-only modules are `wasm32`-only; the rest also compile natively so their
//! unit tests run under `cargo test`.
/// RAII DOM listener: attaches in `listen`, detaches on drop (replaces
/// `Closure::wrap` + manual `use_drop`; mirrors `host::renderer_host::Listener`).
#[cfg(target_arch = "wasm32")]
pub struct DomEventListener {
    target: web_sys::EventTarget,
    name: &'static str,
    callback: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>,
    capture: bool,
}

#[cfg(target_arch = "wasm32")]
impl DomEventListener {
    /// Attach `handler`; `None` when the browser rejects the listener.
    pub fn listen(
        target: &web_sys::EventTarget,
        name: &'static str,
        capture: bool,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Option<Self> {
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::new(handler);
        let function = wasm_bindgen::JsCast::unchecked_ref(callback.as_ref());
        let attached = if capture {
            target.add_event_listener_with_callback_and_bool(name, function, true)
        } else {
            target.add_event_listener_with_callback(name, function)
        };
        attached.ok().map(|()| Self {
            target: target.clone(),
            name,
            callback,
            capture,
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl Drop for DomEventListener {
    fn drop(&mut self) {
        let function = wasm_bindgen::JsCast::unchecked_ref(self.callback.as_ref());
        if self.capture {
            let _ = self
                .target
                .remove_event_listener_with_callback_and_bool(self.name, function, true);
        } else {
            let _ = self
                .target
                .remove_event_listener_with_callback(self.name, function);
        }
    }
}

#[derive(Clone, Copy)]
pub struct DurabilityVersion(pub dioxus::prelude::Signal<u64>);

pub mod board_layer;
pub mod board_reference_effect;
#[cfg(target_arch = "wasm32")]
pub mod canvas_layers;
#[cfg(target_arch = "wasm32")]
pub mod canvas_navigation;
pub mod design_view_toolbar;
pub mod footprint_graphics;
#[cfg(any(test, target_arch = "wasm32"))]
mod generator_preview_defaults;
pub mod inspector_fields;
#[cfg(target_arch = "wasm32")]
pub mod layer_dock;
pub mod layout_camera;
#[cfg(any(test, target_arch = "wasm32"))]
pub mod model_asset_import;
#[cfg(target_arch = "wasm32")]
pub mod object_options;
#[cfg(target_arch = "wasm32")]
pub mod panels;
#[cfg(target_arch = "wasm32")]
pub mod project_menu;
// Vendored components include hooks that are active only in browser builds.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code, unused_imports))]
pub mod ui;

// Bounded waits for browser tests (test-support only).
#[cfg(all(target_arch = "wasm32", any(test, feature = "test-support")))]
pub mod test_wait;
// Finds controls by accessible role and name, as a user or assistive technology would (test-support only).
#[cfg(all(target_arch = "wasm32", any(test, feature = "test-support")))]
pub mod test_role;
// Panel policy is plain logic; compile it natively too so its unit tests run under `cargo test`.
#[cfg(not(target_arch = "wasm32"))]
mod panels {
    #[allow(dead_code)]
    #[path = "policy.rs"]
    mod policy;
}
