//! Read-only Keymap projection and page-private canvas/inspector surfaces.
mod canvas;
mod panel;
mod view;

pub(super) use canvas::KeymapCanvas;
pub(super) use panel::KeymapPanel;
pub(super) use view::{KeymapView, project};
