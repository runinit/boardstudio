# Coding standards

Read during review. Mechanical rules live in checks (`python3 scripts/check.py`); this file holds
the judgement calls that no check can make.

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
