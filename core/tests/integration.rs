//! Core's integration tests, linked as four balanced binaries instead of one per file.
//!
//! Each module is a former `tests/<name>.rs` test crate, included in place so its
//! relative `include_str!` fixtures and `#[path]` helpers keep resolving.
//! `performance` and `matrix_performance` stay separate binaries: they time work
//! and should not share the test thread pool with everything else.
//!
//! This root holds the core engine, document integrity, contracts, schema, firmware,
//! keymap, keycap and outline suites; the matrix, mechanical and module suites live in
//! `integration_matrix.rs`, `integration_mechanical.rs` and `integration_modules.rs`.

#[allow(dead_code)] // This shard uses only some of the shared helpers.
#[path = "support.rs"]
mod support;

#[path = "constraint_depth.rs"]
mod constraint_depth;
#[path = "contracts.rs"]
mod contracts;
#[path = "core.rs"]
mod core_engine;
#[path = "current_schema.rs"]
mod current_schema;
#[path = "document_integrity.rs"]
mod document_integrity;
#[path = "firmware_hardware.rs"]
mod firmware_hardware;
#[path = "firmware_local_build.rs"]
mod firmware_local_build;
#[path = "keycaps.rs"]
mod keycaps;
#[path = "keymap_edits.rs"]
mod keymap_edits;
#[path = "keymap_workflow.rs"]
mod keymap_workflow;
#[path = "outline_clearance.rs"]
mod outline_clearance;
#[path = "outline_edits.rs"]
mod outline_edits;
#[path = "outline_linked_resolution.rs"]
mod outline_linked_resolution;
#[path = "outline_ownership.rs"]
mod outline_ownership;
#[path = "outline_persistence.rs"]
mod outline_persistence;
#[path = "outline_repairs.rs"]
mod outline_repairs;
#[path = "outline_versions.rs"]
mod outline_versions;
#[path = "outlines.rs"]
mod outlines;
