//! Private project-scoped placement planning for catalogue components.
//!
//! The browser controller in `hook` owns the interaction lifetime; `owner` keeps the
//! identity checks and `plan` keeps the atomic document proposal independent of the
//! canvas DOM. `plan` also compiles natively so its unit tests run under `cargo test`.

pub mod plan;

#[cfg(target_arch = "wasm32")]
mod hook;
#[cfg(any(target_arch = "wasm32", test))]
mod owner;

#[cfg(target_arch = "wasm32")]
pub use hook::{DefinitionLoader, PartPlacementHost, PartPlacementMount, use_controller_placement};
#[cfg(any(target_arch = "wasm32", test))]
pub use owner::{
    ActivePartPlacement, ComponentPlacementAction, PlacementOwner, PlacementWorkflow,
    selected_key_component_operation,
};
pub use plan::{
    PendingPart, PlacementSnapOptions, canvas_world_center, component_part, controller_part,
    next_component_reference, placement_gap, placement_operation, pointer_release_commits,
    snap_placement_at, update_pending_part,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use owner::PlacementCapture;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) use plan::PlacementIntent;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod key_component_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod plan_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_support;
