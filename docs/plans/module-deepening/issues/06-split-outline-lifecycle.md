# 06: Separate outline version planning from its Inspector and overlays

Status: resolved
Type: build
Blocked by: [edit settlement 20](../../edit-settlement/issues/20-preview-only-direct-edit-event.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Review: [candidate 06](../../../investigations/architecture-review-2026-10-07.html#c6)

## What to build

A move with no behaviour change. `web/crates/layout/src/outline_lifecycle.rs` (3,574
lines at `915d305c0`) mixes four concerns:

- the lifecycle hook `use_outline_lifecycle` (~676) and submission (`submit_action`,
  ~1194);
- action planning: `OutlineAction`, `action_resolver` (~1373), `plan_action` (~1430,
  ~450 lines), version and entity ID allocation (~22-50), perimeter editing helpers;
- the Inspector UI (~1884-2830);
- canvas overlays (~2875-3400).

Split it into sibling modules under `web/crates/layout/src/outline/` (planner, hook,
Inspector, overlays), keeping public paths stable through re-exports as the crate
split did. The planner takes an accepted document and an action and returns an
operation or a retire reason; it has no Dioxus or Runtime dependency, so it is ready
to move into Core in [typed Core edits 03](../../typed-core-edits/issues/03-decide-outline-version-intents.md).

Blocked by edit settlement 20 because that ticket changes the perimeter preview
(~1308) in this file.

## Acceptance criteria

- [x] No file in the new module exceeds ~1,200 lines; the planner module imports no
  Dioxus, `Signal` or `Runtime`.
- [x] `outline_lifecycle_tests.rs` tests the planner module directly; the browser tests
  pass unchanged.
- [x] `git diff --stat` shows moves, not rewrites (`git log --follow` keeps history for
  the planner).

## Verification

```sh
cargo test -p boardstudio-web-layout --locked
python3 scripts/check.py lint typecheck test
wasm-pack test --headless --chrome web/crates/layout --locked --lib
```

## Outcome

Merged commits `2a225d8ae`, `0a29cf710` and `62bc4d449`. A pure move
preserves planner history before the split into planner, hook, Inspector and overlays;
each new module is below 1,200 lines. Existing public paths remain stable. Native
planner tests now directly cover the extracted planner.

Verification: lint, WASM typecheck, Rust workspace tests and footprints passed after
rebasing over TCE-01 and the Editor tracer. Layout browser tests passed (95 tests).
Both Standards and Spec reviews passed against the final worktree diff. The native
CAD test step reaches the previously reproduced rotated-concave/bottom volume
baseline failure (49 passed, 1 failed, 4 ignored).

The old native hook tests were already inactive because the hook is browser-only.
They remain intact in `outline/hook_tests.rs` with their legacy gate documented;
the named `outline_lifecycle_tests.rs` now runs pure planner coverage. No previously
active browser tests were removed.
