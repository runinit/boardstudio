//! Deterministic projection of canonical mirror pairs.

pub use boardstudio_model::mirrored_half::*;
pub use boardstudio_model::model;

mod project;
pub use project::{ResolvedDoc, project_document};

#[cfg(test)]
mod invariants_tests;
