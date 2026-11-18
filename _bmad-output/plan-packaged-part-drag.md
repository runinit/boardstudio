---
title: Restore packaged Part drag verification
type: bugfix
ticket: ''
created: '2026-11-17'
status: built
baseline_revision: 'c9653233ba1c8d4f5b1b486c63501fa405b868ad'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent">

## Intent

Resolve the Part drag failure on the packaged dev baseline while preserving real pointer input, the saving-to-saved transition, persisted matrix edits and exact Undo.

</frozen-after-approval>

## Implementation Notes

Two local runs failed before the first drag input. The harness polled the former save-indicator class while the current header emits `.header-save[data-state]`. Updated the drag harness to read the current save indicator and assert capture on the layout SVG or a transform handle. The first run after the save-selector correction reached the live preview and exposed the retired pointer-capture selector.

## Verification

Run `python3 scripts/run-drag-browser.py` against the packaged dev site and inspect its named checks. The original gate passed Part drag, including the durable revision change from 2 to 3, exact document and scene restoration on Undo, and a newer saved Undo revision. Column body drag and exact Undo also passed. The subsequent Stagger cancel check failed its separate exact-scene restoration assertion, so the gate exited 1 without a complete receipt. See [local run log](../.evidence/part-drag/final.log).

## Review Triage Log

- Medium, deferred. Both final runs passed Part drag and failed the subsequent Stagger cancel exact-scene assertion. The cancellation branch samples the scene once. Whether it later settles remains unverified.
- Medium, deferred. The cancellation branch discards the observed capture flag and would write a literal true into a successful receipt. The Part drag path checks capture before it releases the pointer.
