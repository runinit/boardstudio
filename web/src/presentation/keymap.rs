//! Read-only Keymap projection and page-private canvas/inspector surfaces.
mod binding_controller;
mod binding_editor;
mod canvas;
mod layer_controller;
mod layer_edit;
mod macro_controller;
mod macro_editor;
mod panel;
mod view;

pub(in crate::presentation) use binding_controller::use_binding_operations;
pub(in crate::presentation) use binding_editor::BindingEditor;
pub(super) use canvas::KeymapCanvas;
pub(in crate::presentation) use layer_controller::{LayerSource, use_layer_operations};
pub(in crate::presentation) use macro_controller::use_macro_operations;
pub(in crate::presentation) use macro_editor::MacroEditor;
pub(in crate::presentation) use panel::KeymapPanel;
pub(super) use view::{KeymapView, project};
