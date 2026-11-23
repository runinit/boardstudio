#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
//! Matrix layout kernel, constraint resolution and outline geometry for BoardStudio Core.

pub use boardstudio_model::model;

pub mod case;
pub mod constraints;
pub mod generators;
pub mod geometry;
pub mod hardware;
pub mod hash;
pub mod inputs;
pub mod matrix;
pub use boardstudio_model::document_format;
pub mod outline_controls;
pub mod outline_edit;
pub mod outline_versions;
