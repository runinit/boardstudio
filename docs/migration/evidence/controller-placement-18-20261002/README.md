# Controller placement source evidence

This evidence applies to the isolated source branch `codex/controller-placement-18-20261002` after commit `72b8c862` plus the current review-fix diff. The shared integration checkout and release browser candidate do not yet contain these changes.

## Focused checks

- `wasm-tests-green.log`: `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- part_placement` — 16 passed. This includes mounted production-hook tests for pending definition ownership, Escape return, stale preparation retirement and fresh placement, registration-before-synchronous-submit, and observer retention after unmount.
- `wasm-clippy.log`: strict page-target all-target Clippy with `-D warnings` — passed.
- `pointer-gate-expected-red.log`: temporary mutant changed the production primary-button predicate to accept non-primary releases; `placement_commits_only_on_primary_pointer_release` failed at its primary assertion as expected. The production predicate was restored before the green test run.
- `stale-owner-expected-red.log`: temporary mutant removed stale-preparation clearing from the production hook; `production_hook_retires_stale_preparation_and_accepts_a_fresh_place_request` failed because canvas ownership remained active. The cleanup was restored before the green test run.

The paired actual-app browser journey and release-build provenance remain pending root's serial integration/release candidate. No browser acceptance is claimed here.
