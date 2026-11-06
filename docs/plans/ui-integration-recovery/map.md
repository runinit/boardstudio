# UI integration recovery

Recover the approved Layout workflows on the newer shared controls and complete
the missing acceptance evidence for the reconciled application.

## Planning ticket

- [001: UI integration recovery specification](issues/001-ui-integration-recovery-spec.md)
  — `ready-for-agent`; owner `fix/ui-integration-recovery`. The parent specification
  remains unchanged by ticket publication; repairs and product acceptance are open.

## Accepted child tickets

The user accepted this ten-ticket breakdown on 2026-11-06. A4 regression
assertions and A8 documentation updates travel with the relevant behavior;
neither is deferred to a separate cleanup ticket.

| Ticket | Status | Blocked by | Delivers |
| --- | --- | --- | --- |
| [002: Diagnose MCU rotation](issues/002-diagnose-mcu-rotation.md) | resolved | none | Actual reproduction, minimized fixture and demonstrated cause |
| [003: Resolve the diagnosed MCU symptom](issues/003-resolve-mcu-orientation.md) | ready-for-agent | 002 | Evidence-backed correction and original-scenario verification |
| [004: Restore placement and Relationships](issues/004-restore-placement-relationships.md) | ready-for-agent | none | Facts beside fields, navigation/highlighting and meaningful split-focus proof |
| [005: Restore nonduplicated scope actions](issues/005-restore-scope-actions.md) | ready-for-agent | none | Matrix/Row/Column actions and guidance transitions without competing popups |
| [006: Restore shared disclosure motion](issues/006-restore-disclosure-motion.md) | ready-for-agent | none | Open/close motion, reduced motion, draft retention and focus handling |
| [007: Correct disclosure chevrons](issues/007-correct-disclosure-chevrons.md) | ready-for-agent | none | Each affected arrow follows its own disclosure state |
| [008: Restore real drag/save/Undo proof](issues/008-restore-drag-save-undo-proof.md) | ready-for-agent | none | Current-control gestures, durable saves, cancellation and exact Undo |
| [009: Verify persistence across reload](issues/009-verify-reload-persistence.md) | ready-for-agent | 008 | Explicit reload of saved edits and saved Undo results |
| [010: Final technical verification and independent review](issues/010-final-technical-verification.md) | ready-for-agent | 003–007, 009 | Full combined gates and independent spec+standards review |
| [011: Populated-workspace human acceptance](issues/011-populated-workspace-acceptance.md) | ready-for-human | 010 | User review of the checked candidate across the complete review matrix |

Dependencies cover 002 transitively through 003 and 008 through 009. The
independent UI repairs do not wait on MCU diagnosis. Coordinate overlapping
shared-file changes without inventing a blocking edge. Ticket 003 may be claimed
only after 002 establishes its bounded correction; a missing reproduction keeps
that chain open.

## Evidence and decisions

- Preserve the [2026-11-06 integration audit](../../research/2026-11-06-dev-ui-integration-audit.md)
  unchanged with this plan. Its findings A1–A8 retain their identifiers.
- The source candidate is `1fda4f48a964568b98dde9c82556350c4bc50911`;
  `510861b11` is the integrated behavioral reference, `c79de4708` the problematic
  reconciliation, and `ef1c8a008` the approved split-board prototype.
- Retain the newer controls and deliberate shell changes. Restore placement and
  nonduplicated scope actions, with behavioral assertions (A4) and documentation
  (A8) included in the relevant work. Shared disclosure motion (A6) and chevron
  binding (A7) are separate behaviors.
- MCU rotation (A1) remains an unresolved user-observed symptom. The heading fix
  is already present; diagnosis requires a real reproduction. Source-present
  engine work is integrated but has no new browser acceptance from this plan.
- 002 (2026-11-06): the user attributes the observed MCU orientation to a stale cached
  build, not the integrated candidate; no defect was reproduced and no Core or renderer
  change is indicated. A1 remains open until 003 is re-scoped. See
  [002](issues/002-diagnose-mcu-rotation.md#answer).
- The source recheck adds explicit requirements for a real split-focus fixture,
  closing the scope hint when selection arrives, and reload verification after
  drag/save/Undo. Existing harnesses are starting points, not completed proof.
- The user reinvoked `to-spec` against the reviewed findings and corrected test
  seams on 2026-11-06. Those seams are carried into 001: mounted production UI,
  real Runtime/Session/Core workflows where relevant, and packaged-app browser
  scenarios. Independent review and human acceptance remain separate gates.

## Next step and boundaries

Ticket publication is complete; no child has been claimed or implemented. The
next frontier ticket is 002. Tickets 004–008 are also unblocked if diagnosis needs
missing project/build input. Claim the selected ticket before working it, keep
execution bounded to that ticket, and retain the next blockers and stopping point
in its completion record. Final acceptance requires all recovery outcomes and
their evidence; a focused pass does not close 010 or 011.

Continue in the already-bound `fix/ui-integration-recovery` worktree. Preserve
the original `ci-wall-time-plan` and detached `dev-live-review` checkouts. This
publication step starts no repairs and authorizes no wholesale history merges,
push, merge to `dev`, or replacement of the user's live preview. Skills, lock
metadata and T3 setup remain local and Git-ignored.
