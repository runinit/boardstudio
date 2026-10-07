//! Outline lifecycle planning and presentation.

pub mod planner;

#[cfg(target_arch = "wasm32")]
pub(crate) mod hook;
#[cfg(target_arch = "wasm32")]
pub(crate) mod inspector;
#[cfg(target_arch = "wasm32")]
pub(crate) mod overlays;

#[cfg(target_arch = "wasm32")]
pub use hook::{
    OutlineDrawTool, OutlineFeedback, OutlineInspectorProjection, OutlineVersionChoice,
    use_outline_lifecycle,
};
#[cfg(target_arch = "wasm32")]
pub use inspector::OutlineVersionInspector;
#[cfg(target_arch = "wasm32")]
pub use overlays::{OutlineDraftCanvasOverlay, OutlinePointCanvasOverlay, OutlineRuntimeHandle};
pub use planner::{OutlineAction, OutlinePointTarget};

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
// Child presentation modules import these shared layout modules through `super::*`.
pub(crate) use crate::{coordinates, keycaps_fit, objects, selection};

#[cfg(all(test, target_arch = "wasm32"))]
use hook::{OutlineActionContext, action_resolver, editable_perimeter};
#[cfg(all(test, target_arch = "wasm32"))]
use overlays::{OutlineCoordinate, OutlineDimension};
#[cfg(all(test, target_arch = "wasm32"))]
use planner::{move_connection_point, unique_outline_entity_id, unique_outline_version_id};

#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use crate::canvas_interaction::{CanvasInteractionArbiter, CanvasInteractionOwner};
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use crate::outline_settings::{
    OutlineEdit, apply_outline_edit, generated_feature, generated_margin, generated_settings,
};
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports this through `super::*`.
use crate::runtime::Runtime;
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, OperationId, Resolution, Scope,
};
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use boardstudio_core::model::{
    Contour, CornerStyle, EditCommand, EditOperation, EditPhase, Operation, OutlineConnection,
    OutlineContourEdit, OutlineControlPoint, OutlineFeature, OutlineGap, OutlineRepairSettings,
    OutlineSettings, Part, Side, Vec2,
};
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)]
// The included browser suite imports Dioxus test helpers through `super::*`.
use dioxus::prelude::*;
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports this through `super::*`.
use dioxus_web::WebEventExt;
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use std::cell::RefCell;
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports these through `super::*`.
use std::rc::Rc;
#[cfg(all(test, target_arch = "wasm32"))]
#[allow(unused_imports)] // The included browser suite imports this through `super::*`.
use wasm_bindgen::JsCast;
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../outline_lifecycle_browser_tests.rs"]
mod browser_tests;
