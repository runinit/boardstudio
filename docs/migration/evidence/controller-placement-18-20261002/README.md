# Controller placement source evidence

This evidence applies to the isolated source branch `codex/controller-placement-18-20261002` after commit `fb91af04` plus the current review-fix diff. The shared integration checkout and release browser candidate do not yet contain these changes.

## Focused checks

- `wasm-tests-green.log`: `wasm-pack test --headless --chrome --mode no-install web --no-default-features --features page --bin boardstudio-web -- part_placement` — 19 passed. Mounted production-hook cases cover pending definition ownership and pointer-start exclusion, Escape return, stale preparation retirement and fresh retry, observer registration before synchronous settlement, exact Ready/Saved completion returning to Wiring, PersistenceFailed and terminal-cancel routes, stale board/project non-redirection, and outcome-slot retention after unmount.
- `wasm-clippy.log`: strict page-target all-target Clippy with `-D warnings` — passed.
- `pointer-gate-expected-red.log`: temporary mutant restored the old all-button acceptance (`true`); `placement_commits_only_on_primary_pointer_release` failed because it admitted button 1. The production primary-button predicate was restored before the green run.
- `stale-owner-expected-red.log`: temporary mutant removed stale-preparation clearing from the production hook; `production_hook_retires_stale_preparation_and_accepts_a_fresh_place_request` failed because canvas ownership remained active. The cleanup was restored before the green run.
- `busy-pointer-admission-expected-red.log`: temporary mutant allowed pointer starts while a definition request owned the canvas; the mounted hook test failed at its busy-phase pointer admission assertion. The production ownership guard was restored before the green run.

The paired actual-app browser journey and release-build provenance remain pending root's serial integration/release candidate. No browser acceptance is claimed here.
