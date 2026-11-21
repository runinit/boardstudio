// Mirrored-half reconcile integration tests (membership-transition half),
// split into the submodules under `mirrored_reconcile_b/` so no file exceeds
// ~1,600 LOC. Shared fixtures live in `support/` (see `support/reconcile.rs`);
// each submodule inherits this crate's imports via `use super::*`.
#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "mirrored_reconcile_a/fixture_rows.rs"]
pub mod fixture_rows;
#[path = "support/reconcile.rs"]
pub mod reconcile_fixtures;
pub mod support;
use fixture_rows::{FixturePair, FixtureRows};

use base_fixtures::*;
use boardstudio_core_mirrored_half::mirrored_half::derive::{project_matrix, project_part};
use boardstudio_model::model::*;
use reconcile_fixtures::*;
use std::collections::BTreeSet;

#[path = "mirrored_reconcile_b/layout_and_membership_removal.rs"]
mod layout_and_membership_removal;
#[path = "mirrored_reconcile_b/membership_trim_and_repair.rs"]
mod membership_trim_and_repair;
#[path = "mirrored_reconcile_b/repair_fresh_and_durable.rs"]
mod repair_fresh_and_durable;
