# UI integration recovery

Recover the approved Layout workflows on the newer shared controls and complete
the missing acceptance evidence for the reconciled application.

## Planning ticket

- [001: UI integration recovery specification](issues/001-ui-integration-recovery-spec.md)
  — `ready-for-agent`; owner `fix/ui-integration-recovery`. Ready for `to-tickets`
  planning, not a claim that repairs or product acceptance are complete.

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
- The source recheck adds explicit requirements for a real split-focus fixture,
  closing the scope hint when selection arrives, and reload verification after
  drag/save/Undo. Existing harnesses are starting points, not completed proof.
- The user reinvoked `to-spec` against the reviewed findings and corrected test
  seams on 2026-11-06. Those seams are carried into 001: mounted production UI,
  real Runtime/Session/Core workflows where relevant, and packaged-app browser
  scenarios. Independent review and human acceptance remain separate gates.

## Next step and boundaries

Run `to-tickets` against 001 and confirm its proposed bounded slices and blocking
edges before publishing implementation tickets. No implementation tickets have
been created. Known UI regressions need not wait for the MCU root cause unless
diagnosis reveals a real dependency; final acceptance requires all recovery
outcomes and their evidence.

Continue in the already-bound `fix/ui-integration-recovery` worktree. Preserve
the original `ci-wall-time-plan` and detached `dev-live-review` checkouts. This
planning scope authorizes no repairs, wholesale history merges, push, merge to
`dev`, or replacement of the user's live preview.
