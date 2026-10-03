//! Read-only Keymap projection and page-private canvas/inspector surfaces.
mod binding_controller;
mod binding_editor;
mod canvas;
mod encoder_editor;
mod encoder_inputs;
mod layer_controller;
mod layer_edit;
mod macro_controller;
mod macro_editor;
mod panel;
mod view;
mod view_controls;

pub(in crate::presentation) use binding_controller::{
    BindingActions, BindingProjectionSources, use_binding_operations,
};
pub(in crate::presentation) use binding_editor::{BindingEditor, BindingTarget};
pub(super) use canvas::KeymapCanvas;
pub(in crate::presentation) use encoder_editor::EncoderEditor;
pub(in crate::presentation) use encoder_inputs::use_encoder_inputs;
pub(in crate::presentation) use layer_controller::{
    LayerActions, LayerSource, use_layer_operations,
};
pub(in crate::presentation) use macro_controller::{MacroActions, use_macro_operations};
pub(in crate::presentation) use macro_editor::MacroEditor;
pub(in crate::presentation) use panel::KeymapPanel;
pub(super) use view::{KeymapView, project};
pub(super) use view_controls::{fit_camera, selected_bounds};
