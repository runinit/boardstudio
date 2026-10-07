# 06: Tracer bullet: Layout Inspector edits land through resolution

Status: resolved
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

- [x] The rapid X/Y plus Undo mounted test passes and is no longer a known failure.
- [x] Revert-on-failure and retire-on-vanished-target mounted tests pass.
- [x] All other Inspector tests from ticket 05 still pass.
- [x] The Inspector no longer sends `Event::Edit` or `ReplaceDocument` at dispatch. Every Inspector edit is an intent.
- [x] Investigation and backlog entries are updated. Doc links pass.

## Verification

```sh
python3 scripts/run-wasm-tests.py --files web/src/presentation/layout_component_inspector_tests.rs
python3 scripts/check.py repo typecheck test
python3 scripts/check-wasm-tests.py
```

## Out of scope

- Other panels (cluster tickets 08 to 16).
- New Core operations: the assign-layout and outline resolvers may still produce
  `ReplaceDocument`, built from the accepted document at execution time.

## Pitfalls

- Enter followed by blur must still commit once. Untouched fields must still not
  commit on blur.
- Don't let the draft become a second document store. Once a ticket settles, the
  field shows the accepted document again.

## Outcome

Commits: claim; "Land Layout Inspector edits through resolution".

- `web/src/presentation/layout_component_edits.rs` is the pattern the cluster tickets
  copy: pure resolver builders for position (single anchor axis + group delta
  translation), assign-layout, outline, constraint and constraint removal — each
  capturing only ids and the user's value, re-checking existence/anchor/eligibility
  against the accepted snapshot at execution, resolving `Unchanged` when the value
  already equals the accepted one, and retiring vanished or ineligible targets with a
  user-readable reason ("The selected part no longer exists.", "This part is locked.",
  "This part is driven by a relationship.", …). It also holds one `EditTicket` per
  committed field and the settle helper: pending keeps the draft, failure restores the
  accepted value with the standard message inline, landed/retired drop the ticket.
- `dispatch_layout_component_inspector_action` keeps every admission check (owner
  currency, board membership, anchor first-selected, finite values) and now begins a
  ticket per action instead of building commands from the render-time snapshot. The
  assign-layout and outline resolvers build `ReplaceDocument` from the snapshot they are
  handed at execution — explicitly allowed by this ticket's scope note.
- The form (`layout_component_inspector.rs`) gains the `pending_edits` signal prop and
  settles tickets each render; drafts and Enter/blur/Escape/precision behaviour are
  untouched. The production host, workspace input and test host thread the signal.
- Tests: the ticket-05 known failure is removed from `scripts/wasm-known-failures.json`
  and `mounted_rapid_xy_queued_edits_keep_both_coordinates_and_undo_removes_only_y`
  passes — both coordinates survive and one Undo removes only Y. New:
  `mounted_failed_save_reverts_the_field_with_an_inline_message` (adapter save failure →
  field reverts, inline message names the reason) and
  `mounted_edit_retires_when_the_part_becomes_locked_before_execution` (a lock edit
  queued ahead retires the position edit with "locked" explained inline).
- The investigation doc is marked resolved with links to ADR-0005, the map and the new
  module; the "Layout queued coordinate edits" backlog entry is removed.

Checks (all pass): `python3 scripts/run-wasm-tests.py --files
web/src/presentation/layout_component_inspector_tests.rs` — 20 executed, 0 failed;
`python3 scripts/check.py typecheck`; `python3 scripts/check-wasm-tests.py`;
`wasm-pack test --headless --chrome web/crates/runtime --locked --lib` — 33 passed;
`wasm-pack test --headless --chrome web/crates/layout --locked --lib` — 82 passed;
`python3 scripts/check-doc-links.py`.

Follow-ups for the cluster tickets: one-shot Inspector controls do not yet disable while
pending (no one-shot control exists in this panel; the amendment applies where they do),
and the settle helper currently answers owner-liveness `true` (a departed owner unmounts
the Inspector).
