use super::{MatrixLink, PartProjection, Projection, reconcile};
#[path = "test_fixtures.rs"]
mod fixtures;
use fixtures::*;
mod outlines_tests {
    include!("outlines/tests.rs");
}
