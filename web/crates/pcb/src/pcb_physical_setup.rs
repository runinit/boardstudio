//! Accepted-source owner and contextual controls for physical setup.
#[path = "pcb_physical_setup/controller.rs"]
pub mod controller;

pub use controller::{
    OwnerContext, OwnerIdentity, PhysicalSetupIntent, PhysicalSetupMount, use_controller,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "pcb_physical_setup/tests.rs"]
mod tests;
