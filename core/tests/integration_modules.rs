//! Core's module integration tests: mounted modules and their supports, wiring,
//! acceptance suites, split halves, splay, key moves and the project library.
//! See `integration.rs` for the split.

#[path = "support/mod.rs"]
#[allow(dead_code)] // This shard uses only some of the shared helpers.
mod support;

#[path = "electrical_wiring.rs"]
mod electrical_wiring;
#[path = "encoder_inputs.rs"]
mod encoder_inputs;
#[path = "end_to_end_acceptance.rs"]
mod end_to_end_acceptance;
#[path = "finding_locations.rs"]
mod finding_locations;
#[path = "key_move_assemblies.rs"]
mod key_move_assemblies;
#[path = "kicad_integration.rs"]
mod kicad_integration;
#[path = "legacy_archive_migration.rs"]
mod legacy_archive_migration;
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
#[path = "splay_companions.rs"]
mod splay_companions;
#[path = "splay_reconciliation.rs"]
mod splay_reconciliation;
#[path = "split_adaptive_spacing.rs"]
mod split_adaptive_spacing;
#[path = "split_half_frame_parts.rs"]
mod split_half_frame_parts;
#[path = "split_part_anchoring.rs"]
mod split_part_anchoring;
#[path = "thqwgd001_library.rs"]
mod thqwgd001_library;
