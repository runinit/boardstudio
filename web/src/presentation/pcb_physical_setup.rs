//! Accepted-source owner and contextual controls for physical setup.
#[path = "pcb_physical_setup/controller.rs"]
pub(in crate::presentation) mod controller;

pub(in crate::presentation) use controller::{
    OwnerContext, OwnerIdentity, PhysicalSetupIntent, PhysicalSetupMount, use_controller,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "pcb_physical_setup/tests.rs"]
mod tests;
