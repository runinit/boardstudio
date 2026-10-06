# 06: Tracer bullet: Layout Inspector edits land through resolution

Status: ready-for-agent
Type: build
Blocked by: 04, 05
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Decision: [ADR-0005](../../../adr/0005-resolve-queued-edits-at-execution.md)

## What to build

Make the Layout Inspector the first real user of the new path. Every Inspector
action that edits the document submits intent through the edit ticket (ticket 04).
Each action's resolver reads the accepted part, layout or constraint at execution
time instead of the snapshot from when the user pressed Enter.

Rapid X then Y now keeps both coordinates, and one Undo reverts only Y. A failed
or retired edit puts the field back to the accepted value with an inline message.
This ticket sets the pattern every later cluster ticket copies, so keep it tidy and
well commented.

## Background you need

- Read ADR-0005, the spec, the investigation's constraints, and tickets 03 to 05.
- Inspector dispatch today builds `MoveParts` absolute positions from the accepted
  part position plus the user's axis value. For a group it applies the first
  selected part's delta to every selected part. Assign-layout and outline actions
  clone the whole document and send `ReplaceDocument`. Constraint actions send
  their own operations. None of these observe their outcome.
- The resolver must re-check at execution what dispatch checks at admission today:
  - the selected parts still exist on the board;
  - the first-selected anchor is unchanged;
  - locked or relationship-driven parts are still ineligible.
  If any check fails, retire with a reason the user can understand, such as "The
  selected part no longer exists" or "This part is locked".
- If the value already equals the accepted value, resolve **Unchanged**.
- Starting points (at `9548275`):
  - `web/src/presentation.rs`: `submit_layout_component_edit` and
    `dispatch_layout_component_inspector_action` (~2121-2380);
  - `web/src/presentation/inspector/layout_component_inspector.rs` (drafts,
    Enter/blur, Escape).

## Approach

1. Move the per-action command construction into pure resolver-building functions
   (owner + captured value → resolver). Keep the admission checks at dispatch, so
   stale callbacks are still ignored early.
2. Keep one ticket per committed field in the Inspector's state. Render the field
   from the draft while Pending. On Failed, clear the draft so the accepted value
   shows, and show the message inline. On Landed or Retired, drop the ticket.
3. Make the ticket 05 regression test pass and remove it from known failures. Add a
   mounted revert-on-failure test (fail the save through the adapter) and a
   retire test (delete the part between commit and execution with gated replies).
4. Update the investigation doc's status and the backlog entry ("Layout queued
   coordinate edits") to resolved, linking this ticket and ADR-0005.

## Acceptance criteria

- [ ] The rapid X/Y plus Undo mounted test passes and is no longer a known failure.
- [ ] Revert-on-failure and retire-on-vanished-target mounted tests pass.
- [ ] All other Inspector tests from ticket 05 still pass.
- [ ] The Inspector no longer sends `Event::Edit` or `ReplaceDocument`. Every Inspector edit is an intent.
- [ ] Investigation and backlog entries are updated. Doc links pass (`python3 scripts/check-doc-links.py`).

## Verification

```sh
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs
python3 scripts/check.py repo typecheck test
python3 scripts/check-wasm-tests.py
```

## Out of scope

- Other panels (cluster tickets 07 to 13).
- New Core operations: the assign-layout and outline resolvers may still produce
  `ReplaceDocument`, built from the accepted document at execution time.

## Pitfalls

- Enter followed by blur must still commit once. Untouched fields must still not
  commit on blur.
- Don't let the draft become a second document store. Once a ticket settles, the
  field shows the accepted document again.
