//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.

pub mod fields;
pub mod hardware_fits;
pub mod pair_controls;
pub mod profiles_mounts;
pub mod stack_controls;
pub mod types;

pub use fields::*;
pub use hardware_fits::*;
pub use pair_controls::*;
pub use profiles_mounts::*;
pub use stack_controls::*;
pub use types::*;

#[cfg(test)]
#[path = "mechanical_settings_migration_tests.rs"]
mod migration_tests;

#[cfg(test)]
pub mod tests_contextual;
#[cfg(test)]
pub mod tests_flows;
#[cfg(test)]
pub mod tests_mounted;
#[cfg(test)]
pub mod tests_support;
