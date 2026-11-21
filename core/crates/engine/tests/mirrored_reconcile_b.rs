// Mirrored-half reconcile integration tests (membership-transition half),
// split into the submodules under `mirrored_reconcile_b/` so no file exceeds
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

#[path = "mirrored_reconcile_b/layout_and_membership_removal.rs"]
mod layout_and_membership_removal;
#[path = "mirrored_reconcile_b/membership_expansion.rs"]
mod membership_expansion;
#[path = "mirrored_reconcile_b/membership_trim_and_repair.rs"]
mod membership_trim_and_repair;
#[path = "mirrored_reconcile_b/repair_fresh_and_durable.rs"]
mod repair_fresh_and_durable;
