//! Accepted-source owner and contextual controls for physical setup.
#[path = "pcb_physical_setup/controller.rs"]
pub mod controller;

pub use controller::{
    OwnerContext, OwnerIdentity, PhysicalSetupIntent, PhysicalSetupMount, use_controller,
};

// The parent module is wasm32-only, so these mounted tests execute in the browser suite;
// a native gate here compiled the module on neither target.
#[cfg(test)]
#[path = "pcb_physical_setup/tests.rs"]
mod tests;
