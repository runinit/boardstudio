//! Private, deterministic projection of accepted Case closure mounts into PCB parts.
//!
//! This is document planning only. Callers provide the already normalized
//! mounting-hole catalogue definition; geometry resolution, validation and edit
//! admission remain with their existing owners.

pub use boardstudio_core::mechanical::clearance::project_owned_closure_clearance;
