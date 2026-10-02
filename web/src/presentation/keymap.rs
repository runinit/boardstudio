//! Read-only Keymap projection and page-private canvas/inspector surfaces.
mod canvas;
mod layer_controller;
mod layer_edit;
mod panel;
mod view;

pub(super) use canvas::KeymapCanvas;
pub(in crate::presentation) use layer_controller::{LayerSource, use_layer_operations};
pub(in crate::presentation) use panel::KeymapPanel;
pub(super) use view::{KeymapView, project};
