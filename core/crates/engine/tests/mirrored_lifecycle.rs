// Mirrored-half lifecycle integration tests, split into the submodules
// under `mirrored_lifecycle/` so no file exceeds ~1,600 LOC. Shared fixture
// builders live in `support/`; each submodule inherits this crate's imports
// via `use super::*` and cross-imports helpers from sibling submodules.
#[path = "support/base.rs"]
pub mod base_fixtures;
#[path = "mirrored_reconcile_a/fixture_rows.rs"]
pub mod fixture_rows;
pub mod support;
use fixture_rows::{FixturePair, FixtureRows};

use crate::base_fixtures::fixture_document;
use crate::support::linked_endpoint;
use boardstudio_core_kernel::constraints::{
    ConstraintEndpoint, ConstraintRelation, FramedConstraint,
};
use boardstudio_core_mirrored_half::mirrored_half::reconcile::lifecycle::{
    ConstraintLifecycleEffect, lifecycle_context, matrix_pair_for_association,
    prepare_constraint_lifecycle_delta, prepare_lifecycle_transition, retire_replacement_records,
    transition_exception_context,
};
use boardstudio_core_mirrored_half::mirrored_half::reconcile::{
    BaselineKind, FinalOwner, MembershipContext, MembershipTransitionInput, PlannedAddition,
    TransitionAssociation, TransitionState, prepare_membership_transition,
};
use boardstudio_model::model::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[path = "mirrored_lifecycle/constraint_lifecycle.rs"]
mod constraint_lifecycle;
#[path = "mirrored_lifecycle/independent_exceptions.rs"]
mod independent_exceptions;
#[path = "mirrored_lifecycle/matrix_and_circuit_lifecycle.rs"]
mod matrix_and_circuit_lifecycle;
#[path = "mirrored_lifecycle/multi_matrix_and_replacement.rs"]
mod multi_matrix_and_replacement;
#[path = "mirrored_lifecycle/owner_rejoin_electrical.rs"]
mod owner_rejoin_electrical;
