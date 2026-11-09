# Coding standards

Read before adding or changing tests, and during review. Mechanical rules live in checks
(`python3 scripts/check.py`); this file holds the judgement calls that no check can make.

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
- **Shared setup, explicit story.** Reuse existing fixtures, builders and mounting/cleanup
  helpers. Keep actions and expected outcomes visible in the test; share substantial repeated
  setup, not coincidentally similar lines or a new configurable testing framework.
- **Parameterize data, not stories.** Table-drive equivalent input variants with clear case
  labels. Keep materially different timing, ownership, admission and failure paths distinct.
- **Synthetic data, production behavior.** Construct small valid fixtures, but exercise the
  production path whose contract is under test. Hand-built projections suit isolated rendering
  tests; they do not substitute for accepted-state/projector integration. Consolidate overlapping
  host plumbing where purposes match instead of keeping parallel fixture implementations.
- **Earn repeated coverage.** Test common mechanics at their agreed seam; repeat coverage for
  owner wiring, canonicalization or lifetime only where it can fail independently. Repeating a
  whole matrix at another layer needs a distinct failure rationale.
- **Review maintenance cost.** Hold fixtures and helpers to the production-code quality bar.
  Large test additions must explain the uncovered risks and why existing setup is insufficient.
  Judge failure coverage and upkeep, not test counts or a test-to-production line quota.
- **Preserve proof.** Consolidation preserves required acceptance cases, named baseline guards
  and RED/GREEN or mutation evidence. Reduce duplication, not meaningful assertions or safety
  coverage; never delete tests merely to improve a ratio.
