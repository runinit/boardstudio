//! Core's matrix integration tests: the matrix kernel, anchors, placement, companions,
//! native exceptions and the mirrored-half suites. See `integration.rs` for the split.

#[path = "support/mod.rs"]
#[allow(dead_code)] // This shard uses only some of the shared helpers.
mod support;

#[path = "matrix_anchors.rs"]
mod matrix_anchors;
#[path = "matrix_companions.rs"]
mod matrix_companions;
#[path = "matrix_kernel.rs"]
mod matrix_kernel;
#[path = "matrix_placement.rs"]
mod matrix_placement;
#[path = "mirrored_half.rs"]
mod mirrored_half;
#[path = "native_matrix_exceptions.rs"]
mod native_matrix_exceptions;
