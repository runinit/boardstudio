# Coding standards

Code and test-design guidance; installed BMad skills govern the workflow. Mechanical
rules live in checks (`python3 scripts/check.py`).

- **Reuse the helper.** When the module or its neighbours already hold the geometry or the
  arithmetic (`rotate` in `matrix/splay.rs`), call it, widening its visibility if needed. A
  second hand-rolled copy drifts the first time one of them changes.
- **Keep the operation order of saved arithmetic.** Poses are persisted and hashed. When an edit
  rewrites existing arithmetic, keep the order of the operations and pin it with an
  exact-equality test; floating-point addition is not associative, so `a + (b - c)` can differ
  from `(a + b) - c` in the last bit.
- **Say why a pinned contract is pinned.** A variant, flag or branch that exists because a
  golden fixture (`core/tests/fixtures/boundary/transcript.json`) or a baseline pins its
  behaviour carries a comment naming that fixture, so a later tidy-up does not undo it.
- **Match the surrounding tests.** A new test follows its siblings' fixture shape and naming
  before it introduces a helper of its own.

## Test economy

- **Distinct failure.** Each new case catches an observable failure existing cases miss.
  Keep related assertions in one coherent scenario rather than splitting them into new setups.
  Derive expectations from an independent spec, worked example or known-good reference, not
  copied implementation logic. Literals, goldens and compatibility checks earn their place by
  pinning an agreed contract.
- **Shared setup, explicit story.** Reuse existing fixtures, builders and mounting/cleanup
  helpers. Keep actions and expected outcomes visible in the test; share substantial repeated
  setup where fixture purposes match, not coincidentally similar lines or a new configurable
  testing framework. Keep display, static-owner and live-projector purposes distinct.
- **Parameterize data, not stories.** Table-drive equivalent input variants with clear case
  labels. Keep materially different timing, ownership, admission and failure paths distinct.
- **Synthetic data, production behavior.** Construct small valid fixtures, but exercise the
  production path whose contract is under test. Hand-built projections suit isolated rendering
  tests; they do not substitute for accepted-state/projector integration. Use existing dependency
  boundaries; justify and obtain required approval for new test-only APIs, flags or wider
  visibility. Approved dependency injection and diagnostic seams remain valid.
- **Earn repeated coverage.** Test common mechanics at their agreed seam; repeat coverage for
  owner wiring, canonicalization or lifetime only where it can fail independently. Repeating a
  whole matrix at another layer needs a distinct failure rationale. Use the cheapest tier faithful
  to the behavior being checked; preserve accepted functional coverage.
- **Review maintenance cost.** Hold fixtures and helpers to the production-code quality bar.
  New fixtures, helpers or mount paths must explain why existing setup is insufficient.
  Refactor tests for repeated setup edits, diagnosed flakiness or measured runtime cost, not
  test volume or cosmetic line savings. Judge failure coverage and upkeep, not counts or ratios.
- **Preserve coverage.** Consolidation preserves required acceptance cases and meaningful
  assertions. Identify the surviving coverage when retiring or renaming tests; do not silently
  lose regression protection.
- **Protect the oracle.** Fix production code to reach GREEN; never weaken assertions, expected
  values, goldens or baselines merely to pass. Correct a genuinely wrong test only with independent
  evidence (spec, worked example or known-good reference); record that evidence. Contract changes
  require approval, not silent oracle updates.
