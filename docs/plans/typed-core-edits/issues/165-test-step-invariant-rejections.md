# 165: Test STEP invariant rejection with corrupt imports

Status: claimed
Type: build
Priority: P2
Blocked by: None (can start immediately)
Related: [STEP validation 03](../../step-validation-simplification/issues/03-post-import-invariants.md), [CAD 007](../../cad-backend-replacement/issues/007-strict-step-import.md)

## What to build

The post-import invariant suite must have tests that detect broken rejection
behavior. A successful tetrahedron import remains a smoke check; it must not be
presented as proof that corrupt imports are rejected or that the import-time guard
is enforced.

The 2026-10-18 audit at `f955050c3` found two new tests using only a valid
tetrahedron. Neither exercises rejection, including the recently corrected scan
for a nonfinite coordinate beyond the first ten positions.

## Acceptance criteria

- [ ] Replace the duplicate happy-path claim with focused negative cases using
  private test access. Do not widen member or API visibility for the tests.
- [ ] Cover NaN and infinity beyond the first ten position entries, including a
  finite prefix, so truncating the scan by position cannot pass unnoticed.
- [ ] Cover an inconsistent triangle-to-face metadata count and the other existing
  invariant rejection branches with controlled corrupt values and explicit reasons.
  Keep assertions tied to the current contract; any newly discovered production
  defect is reported explicitly before broadening this test-only slice.
- [ ] Retain a valid-import control. Rename or rewrite the successful-import test
  so its stated guarantee matches what it actually observes.
- [ ] Establish a meaningful RED/GREEN result for the late nonfinite-coordinate
  regression. Use the project's mutation checker to demonstrate that bypassing the
  relevant invariant condition is detected, and restore source exactly afterward.
- [ ] Check that the production import boundary still invokes validation. Claim
  integration enforcement coverage only if a test actually exercises that boundary
  with a failing invariant; record the limit otherwise.
- [ ] Run the affected native invariant/import tests in release mode. For this
  test-only slice, retain RED/GREEN and mutation evidence plus a final diff read
  under the CAD review policy.

## Comments

Created from audit finding 4 at the user's request. This adds rejection evidence
for existing invariants and does not reopen the broader STEP validation plan.

## CURRENT

2026-10-18: Implementation complete; status remains claimed pending parent integration,
full verification and combined spec/standards review. Added eight private tests with
explicit rejection reasons: NaN/+Inf/-Inf at position 35 after a finite prefix,
inconsistent nonempty triangle-face tags, empty mesh, missing source faces, all
bounds endpoints/axes with nonfinite values, and equal/reversed degenerate bounds.
No production implementation, API or visibility changes. Retained
`import_invariants_pass_for_valid_step`; removed the misleading duplicate
`import_invariants_are_checked_during_import` (eight added, one removed).

Fresh checks in this worktree, with logs under `.evidence/ticket-165/`:

- `python3 scripts/mutation-check.py .evidence/ticket-165/late-position-scan.json --command 'cargo test --manifest-path cad/monstertruck/Cargo.toml --release --locked --lib step::invariant_tests' --evidence-dir .evidence/ticket-165/mutation --timeout 1800`: unmodified baseline 8 passed; mutation RED killed by all three late-coordinate tests (exit 101, no compile failure/timeout), `restored_exact: true`. The single exact edit changed `self.mesh.positions.iter().enumerate()` to `self.mesh.positions.iter().take(10).enumerate()` in the validator loop. Each failure returned `Ok(())` instead of the explicit nonfinite-position reason. This is mutation RED, not an unmodified baseline failure.
- `cargo test --manifest-path cad/monstertruck/Cargo.toml --release --locked --lib step::invariant_tests`: restored GREEN, 8 passed (`restored-invariants.log`).
- `cargo test --manifest-path cad/monstertruck/Cargo.toml --release --locked --test step_import`: 60 passed, 1 existing ignored vendor-corpus test (`step-import.log`).
- `rustfmt --check --edition 2024 cad/monstertruck/src/step/invariant_tests.rs` and `git diff --check`: passed after correcting initial wrapping differences. Final diff read completed.

Before builds, `readlink -f` inspection showed root and CAD targets absent and
resolving locally to this worktree; all builds used `cad/monstertruck/target`.
Source inspection confirms `import_step_with_options` invokes
`imported.validate_invariants()` and returns its failures before `Ok(imported)`.
The negative tests corrupt an already imported model: they prove validator
rejection, not import-boundary rejection of an injected invariant failure. No
injectable import hook was introduced. No production defect discovered in this
slice. No installed Rust-specific skill was found; TDD and project Rust guidance
were read.

Parent gates: integrate with 162's separate production function changes preserved,
run the final full suite (`python3 scripts/check.py`) under the shared root-target
lock when applicable, and complete the combined spec/standards review before
resolving. The existing ignored corpus test was not run in this focused stream.
