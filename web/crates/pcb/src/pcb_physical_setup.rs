//! Accepted-source owner and contextual controls for physical setup.
mod reassignment_owner;

#[cfg(target_arch = "wasm32")]
#[path = "pcb_physical_setup/controller.rs"]
pub mod controller;

#[cfg(target_arch = "wasm32")]
pub use controller::{
    OwnerContext, OwnerIdentity, PhysicalSetupIntent, PhysicalSetupMount, use_controller,
};
