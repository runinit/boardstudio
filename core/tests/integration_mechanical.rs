//! Core's mechanical integration tests: mechanical workflows and settings, case, gasket,
//! part offsets, boundary parity, generators and the matrix aggregate suites.
//! See `integration.rs` for the split.

#[allow(dead_code)] // This shard uses only some of the shared helpers.
#[path = "support.rs"]
mod support;

#[path = "boundary_parity.rs"]
mod boundary_parity;
#[path = "case.rs"]
mod case;
#[path = "closure_generator_parameters.rs"]
mod closure_generator_parameters;
#[path = "gasket_features.rs"]
mod gasket_features;
#[path = "generator_export.rs"]
mod generator_export;
#[path = "generator_parameters.rs"]
mod generator_parameters;
#[path = "matrix_aggregate_anchors.rs"]
mod matrix_aggregate_anchors;
#[path = "mechanical_identity_monotonic.rs"]
mod mechanical_identity_monotonic;
#[path = "mechanical_integration.rs"]
mod mechanical_integration;
#[path = "mechanical_settings_edits.rs"]
mod mechanical_settings_edits;
#[path = "mechanical_workflow.rs"]
mod mechanical_workflow;
#[path = "part_offset_constraint.rs"]
mod part_offset_constraint;
