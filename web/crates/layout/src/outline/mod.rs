//! Outline lifecycle planning and presentation.

pub mod adapter;
pub use boardstudio_web_layout_core::outline::planner;

#[cfg(target_arch = "wasm32")]
pub(crate) mod hook;
#[cfg(target_arch = "wasm32")]
pub(crate) mod inspector;
#[cfg(target_arch = "wasm32")]
pub(crate) mod overlays;

#[cfg(target_arch = "wasm32")]
pub use hook::{
    OutlineDrawTool, OutlineFeedback, OutlineInspectorProjection, OutlineVersionChoice,
    OutlineVersionOwnership, outline_action_is_current, use_outline_lifecycle,
};
#[cfg(target_arch = "wasm32")]
pub use inspector::OutlineVersionInspector;
#[cfg(target_arch = "wasm32")]
pub use overlays::{OutlineDraftCanvasOverlay, OutlinePointCanvasOverlay, OutlineRuntimeHandle};
pub use planner::{OutlineAction, OutlineActionEnvelope, OutlineActionKind, OutlinePointTarget};

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
// Child presentation modules import these shared layout modules through `super::*`.
pub(crate) use crate::{coordinates, keycaps_fit, objects, selection};
