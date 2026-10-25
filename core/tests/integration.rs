//! Core's integration tests, linked as one binary instead of one per file.
//!
//! Each module is a former `tests/<name>.rs` test crate, included in place so its
//! relative `include_str!` fixtures and `#[path]` helpers keep resolving.
//! `matrix_performance` stays a separate binary: it times work and should not
//! share the test thread pool with everything else.

#[path = "boundary_parity.rs"]
mod boundary_parity;
#[path = "case.rs"]
mod case;
#[path = "closure_generator_parameters.rs"]
mod closure_generator_parameters;
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
#[path = "generator_parameters.rs"]
mod generator_parameters;
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
#[path = "matrix_kernel.rs"]
mod matrix_kernel;
#[path = "mechanical_identity_monotonic.rs"]
mod mechanical_identity_monotonic;
#[path = "mechanical_integration.rs"]
mod mechanical_integration;
#[path = "mechanical_settings_edits.rs"]
mod mechanical_settings_edits;
#[path = "mechanical_workflow.rs"]
mod mechanical_workflow;
#[path = "mirrored_half.rs"]
mod mirrored_half;
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
#[path = "part_offset_constraint.rs"]
mod part_offset_constraint;
#[path = "splay_reconciliation.rs"]
mod splay_reconciliation;
#[path = "support/mod.rs"]
mod support;
#[path = "thqwgd001_library.rs"]
mod thqwgd001_library;

#[path = "key_move_assemblies.rs"]
mod key_move_assemblies;

#[path = "matrix_placement.rs"]
mod matrix_placement;

#[path = "matrix_companions.rs"]
mod matrix_companions;

#[path = "matrix_anchors.rs"]
mod matrix_anchors;

#[path = "end_to_end_acceptance.rs"]
mod end_to_end_acceptance;

#[path = "splay_companions.rs"]
mod splay_companions;

#[path = "native_matrix_exceptions.rs"]
mod native_matrix_exceptions;

#[path = "matrix_aggregate_anchors.rs"]
mod matrix_aggregate_anchors;

#[path = "matrix_aggregate_reflection.rs"]
mod matrix_aggregate_reflection;

#[path = "split_half_frame.rs"]
mod split_half_frame;

#[path = "split_adaptive_spacing.rs"]
mod split_adaptive_spacing;
