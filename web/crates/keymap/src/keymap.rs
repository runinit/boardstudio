//! Read-only Keymap projection and page-private canvas/inspector surfaces.
mod binding_controller;
mod binding_editor;
mod canvas;
mod encoder_editor;
mod encoder_inputs;
mod layer_controller;
mod macro_controller;
mod macro_editor;
mod owned_edits;
mod panel;
mod view;
mod view_controls;

pub use binding_controller::{BindingActions, BindingProjectionSources, use_binding_operations};
pub use binding_editor::{BindingEditor, BindingTarget};
pub use canvas::KeymapCanvas;
pub use encoder_editor::EncoderEditor;
pub use encoder_inputs::use_encoder_inputs;
pub use layer_controller::{LayerActions, LayerSource, use_layer_operations};
pub use macro_controller::{MacroActions, use_macro_operations};
pub use macro_editor::MacroEditor;
pub use panel::KeymapPanel;
pub use view::{KeymapView, project};
pub use view_controls::{fit_camera, selected_bounds};

#[cfg(test)]
mod queued_edit_tests;
