//! Core's integration tests, linked as one binary instead of one per file.
//!
//! Each module is a former `tests/<name>.rs` test crate, included in place so its
//! relative `include_str!` fixtures and `#[path]` helpers keep resolving.
//! `performance` and `matrix_performance` stay separate binaries: they time work
//! and should not share the test thread pool with everything else.

#[path = "boundary_parity.rs"]
mod boundary_parity;
#[path = "case.rs"]
mod case;
#[path = "contracts.rs"]
mod contracts;
#[path = "core.rs"]
mod core_engine;
#[path = "current_schema.rs"]
mod current_schema;
#[path = "demo_projects.rs"]
mod demo_projects;
#[path = "electrical_wiring.rs"]
mod electrical_wiring;
#[path = "encoder_inputs.rs"]
mod encoder_inputs;
#[path = "finding_locations.rs"]
mod finding_locations;
#[path = "firmware_hardware.rs"]
mod firmware_hardware;
#[path = "firmware_local_build.rs"]
mod firmware_local_build;
#[path = "gasket_features.rs"]
mod gasket_features;
#[path = "generator_export.rs"]
mod generator_export;
#[path = "keycaps.rs"]
mod keycaps;
#[path = "keymap_edits.rs"]
mod keymap_edits;
#[path = "keymap_workflow.rs"]
mod keymap_workflow;
#[path = "kicad_integration.rs"]
mod kicad_integration;
#[path = "legacy_archive_migration.rs"]
mod legacy_archive_migration;
#[path = "mechanical_workflow.rs"]
mod mechanical_workflow;
#[path = "mixed_project_acceptance.rs"]
mod mixed_project_acceptance;
#[path = "module_board_supports.rs"]
mod module_board_supports;
#[path = "module_case_supports.rs"]
mod module_case_supports;
#[path = "module_circuit_electrical.rs"]
mod module_circuit_electrical;
#[path = "module_host_connector.rs"]
mod module_host_connector;
#[path = "module_import.rs"]
mod module_import;
#[path = "module_preview_footprints.rs"]
mod module_preview_footprints;
#[path = "module_service_clearance.rs"]
mod module_service_clearance;
#[path = "mounted_modules.rs"]
mod mounted_modules;
#[path = "outline_clearance.rs"]
mod outline_clearance;
#[path = "outline_repairs.rs"]
mod outline_repairs;
#[path = "outline_versions.rs"]
mod outline_versions;
#[path = "outlines.rs"]
mod outlines;
#[path = "splay_reconciliation.rs"]
mod splay_reconciliation;
#[path = "thqwgd001_library.rs"]
mod thqwgd001_library;
