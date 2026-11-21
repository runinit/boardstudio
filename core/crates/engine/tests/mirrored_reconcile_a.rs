// Mirrored-half reconcile integration tests (hosted/fixed-membership half),
// split into the submodules under `mirrored_reconcile_a/` so no file exceeds
// ~1,600 LOC. Shared fixtures live in `support/` (see `support/reconcile.rs`);
// each submodule inherits this crate's imports via `use super::*`.
#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "support/reconcile.rs"]
pub mod reconcile_fixtures;
pub mod support;

use base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::reconcile::*;
use boardstudio_model::model::*;
use reconcile_fixtures::*;
use std::collections::{BTreeMap, BTreeSet};

#[path = "mirrored_reconcile_a/fixed_membership_rejections.rs"]
mod fixed_membership_rejections;
#[path = "mirrored_reconcile_a/fixed_membership_views.rs"]
mod fixed_membership_views;
#[path = "mirrored_reconcile_a/hosted_correspondences.rs"]
mod hosted_correspondences;
#[path = "mirrored_reconcile_a/hosted_omitted_and_fixed_membership.rs"]
mod hosted_omitted_and_fixed_membership;
#[path = "mirrored_reconcile_a/projection_and_layouts.rs"]
mod projection_and_layouts;
