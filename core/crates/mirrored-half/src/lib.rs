//! Deterministic projection of canonical mirror pairs.

#[path = "model.rs"]
mod mirror_model;
pub use boardstudio_model::model;
pub use mirror_model::*;

mod project;
pub use project::{ResolvedDoc, project_document};

#[cfg(test)]
mod invariants_tests;

mod ops;
pub use ops::*;

mod reflect;
