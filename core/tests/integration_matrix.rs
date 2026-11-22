//! Core matrix kernel, anchor, placement, and companion integration tests.

#[allow(dead_code)] // This shard uses only some of the shared helpers.
#[path = "support.rs"]
mod support;

#[path = "matrix_anchors.rs"]
mod matrix_anchors;
#[path = "matrix_companions.rs"]
mod matrix_companions;
#[path = "matrix_kernel.rs"]
mod matrix_kernel;
#[path = "matrix_placement.rs"]
mod matrix_placement;
