//! Core matrix kernel, anchor, placement, and companion integration tests.

#[path = "support/mod.rs"]
#[allow(dead_code)] // This shard uses only some of the shared helpers.
mod support;

#[path = "support.rs"]
#[allow(dead_code)] // Each shard uses a subset of the fixtures.
mod fixtures;

#[path = "matrix_anchors.rs"]
mod matrix_anchors;
#[path = "matrix_companions.rs"]
mod matrix_companions;
#[path = "matrix_kernel.rs"]
mod matrix_kernel;
#[path = "matrix_placement.rs"]
mod matrix_placement;
