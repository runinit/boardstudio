# 05: Layout Inspector mounted tests reach the real Session and Core

Status: resolved
Type: build
Blocked by: 01
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to build

The 17 mounted Layout Inspector tests currently run in a Runtime mode that records
submitted events and returns before Session sees them. They prove which commands
are emitted, not what the accepted document becomes. Move the suite onto the
in-process Core and persistence adapter from ticket 01, so each test mounts the
form against an opened document and asserts accepted results.

Then add the regression test this whole effort exists for. With Core replies gated,
type X, Tab, type Y, Enter. Release the replies and assert the accepted X and Y,
then Undo once and assert only Y reverted. The test fails today, so register it as
a known failure with a reason that points to ticket 06. Finally, delete the
layout-inspector interception mode from Runtime.

## Background you need

- Read [the investigation](../../../investigations/layout-part-editing.md):
  "What the existing tests establish" and "Constraints on a future deepening". All
  listed behaviours must stay covered: stale selection callbacks, away-and-back
  selection, Inspector removal, unrelated revisions, draft and tab retention,
  Enter/blur counts, untouched precision, Escape, locked/driven positions and group
  translation.
- Interception today: Runtime's `set_layout_component_inspector_test_state`,
  `take_layout_component_inspector_test_events` and
  `settle_layout_component_inspector_test_operation`, plus the matching `#[cfg(test)]`
  fields and the early return in `submit`. Fixture "acceptance" is simulated by
  replacing the test model.
- Ticket 01's adapter can open a real document, gate a Core reply, gate or fail a
  save, and run pending effects.
- Browser tooling:
  - known failures live in `scripts/wasm-known-failures.json` as
    `[{"test": "<full test path>", "reason": "..."}]`. They still run, and the
    runner reports when one starts passing;
  - source-to-test mappings live in `scripts/wasm-test-owners.json`. Keep entries
    for this file's tests correct.
- Starting points (at `9548275`):
  - `web/src/presentation/layout_component_inspector_tests.rs`;
  - `web/crates/runtime/src/runtime.rs` (search `layout_component_inspector_test`).

## Approach

1. Build a shared fixture that opens a small document through the in-process
   adapter: a board, two parts with known coordinates, one locked part and one
   driven part, matching what the current tests need.
2. Port the tests one by one.
   - Assertions about the emitted command become assertions about the accepted
     document, where that's what the test means: group translation, untouched
     precision, a locked part not moving.
   - Assertions that are about event counts (Enter then blur commits once) may
     still count operations. Count settled operations, not intercepted events.
   - Simulate unrelated acceptance with a real edit to another part, not a model
     swap.
3. Add the rapid X/Y accepted-result test with Undo, and register it as a known
   failure (reason: "queued Inspector edits overwrite; fixed by ticket 06").
4. Delete the interception fields, methods and the `submit` early return. Make sure
   nothing else uses them (`grep -rn layout_component_inspector_test web/src`).

## Acceptance criteria

- [x] All previously covered behaviours still have a passing mounted test, now against the real Session and Core.
- [x] The new rapid X/Y plus Undo test exists, fails for the expected reason (X reverted), and is listed as a known failure.
- [x] ~~Runtime has no layout-inspector interception fields, methods or branches.~~
  Deferred to tickets 11/12 with the user's decision (2026-10-06): two more test files in
  `web/crates/layout` still mount through it; see Comments.
- [x] Test-owner mappings are correct (none existed for this file; the runner maps it directly). `python3 scripts/check-wasm-tests.py` passes.

## Verification

```sh
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs
python3 scripts/check.py typecheck
python3 scripts/check-wasm-tests.py
```

## Out of scope

- Fixing the overwrite (ticket 06).
- Definition-name interception ([ticket 07](07-definition-name-tests-reach-core.md)).

## Comments

**2026-10-06, mid-ticket:** The Inspector suite itself is ported (commit e75c49eeb): all
previously covered behaviours now run against the real Session and CoreEngine through the
in-process adapter and assert accepted results, and the rapid queued X/Y + Undo test exists
and is registered as a known failure for the expected reason (the Y edit restores the old
X: `Vec2 { x: 66.675, y: -40.0 }` vs the expected `(60.0, -40.0)`).

The last step — deleting the interception mode — is blocked by users the ticket's grep
(`web/src` only, taken before the web crate split) did not know about:

- `web/crates/layout/src/objects/matrix_transform_inspector_tests.rs` (9 call sites),
- `web/crates/layout/src/outline_lifecycle_browser_tests.rs` (12 call sites).

Both mount inspector hosts over hand-built `ReadModel` fixtures and assert captured
events; through the real Session those assertions must become accepted-document
assertions over Core-generated matrix state, which is the migration work tickets 11
(matrix-and-transform) and 12 (outline) already own. Deleting the interception now would
break their suites (verified: 13 failures). The interception is left in place and these
tests keep passing.

Needs a decision: (a) port those two files onto the adapter inside this ticket, or
(b) move the interception deletion to the cluster tickets (11/12) or cleanup (17) and
adjust this ticket's third acceptance criterion accordingly.

## Outcome

Commits: claim; e75c49eeb (suite port + known failure); this resolution.

- The mounted suite now opens a Core-valid fixture document through ticket 01's in-process
  adapter (memory saves), selects through real `Event::SelectParts`, applies unrelated
  revisions as real edits, and asserts accepted results: positions, group translation from
  the first-selected anchor, margin commits, locked/driven suppression (accepted document
  and revision unchanged), draft survival across acceptance, and Enter/blur committing
  exactly one edit per axis (revision delta). Escape asserts the accepted value. The
  pure-projection tests and the tab-reset test keep their synthetic fixtures (no Runtime).
- Two fixture realities discovered and encoded: Core generates matrix members with
  `matrix/<id>/rRcC` ids (switch-style references), and only matrices owned by mirrored
  layouts are synced at open — the matrix-key test opens a mirrored pair and selects the
  generated member.
- The rapid queued X/Y + Undo test exists and fails for exactly the expected reason
  (X reverted: `(66.675, -40.0)` vs `(60.0, -40.0)`); registered in
  `scripts/wasm-known-failures.json` under its bare test name (the runner matches by bare
  name, not full path) with a reason pointing at ticket 06. The runner reports the suite
  green (exit 0, failed 0) with that one known failure.
- The interception deletion is deferred to tickets 11/12 per the user's decision; their
  acceptance criteria now name it. Ticket 05's third criterion is adjusted above.

Checks (all pass): `python3 scripts/run-wasm-tests.py --files
web/src/presentation/layout_component_inspector_tests.rs` — 18 executed, failed 0 (one
known failure), exit 0; `python3 scripts/check.py typecheck`; `python3 scripts/check-wasm-tests.py`;
`wasm-pack test --headless --chrome web/crates/layout --locked --lib` — 82 passed (the two
dependent files keep working); `wasm-pack test --headless --chrome web/crates/runtime
--locked --lib` — 33 passed.
