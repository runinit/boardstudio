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
| [002: Diagnose MCU rotation](issues/002-diagnose-mcu-rotation.md) | resolved | none | Closed on the user's stale-build report; original symptom not reproduced |
| [003: Resolve the diagnosed MCU symptom](issues/003-resolve-mcu-orientation.md) | wontfix | 002 | Not applicable; closed by the user (see 002). Evidence-backed correction and original-scenario verification |
| [004: Restore placement and Relationships](issues/004-restore-placement-relationships.md) | wontfix | none | Cancelled by the user; no completion evidence |
| [005: Restore nonduplicated scope actions](issues/005-restore-scope-actions.md) | wontfix | none | Cancelled by the user; no completion evidence |
| [006: Restore shared disclosure motion](issues/006-restore-disclosure-motion.md) | wontfix | none | Cancelled by the user; no completion evidence |
| [007: Correct disclosure chevrons](issues/007-correct-disclosure-chevrons.md) | wontfix | none | Cancelled by the user; no completion evidence |
| [008: Restore real drag/save/Undo proof](issues/008-restore-drag-save-undo-proof.md) | resolved | none | Drag, save, cancel and exact Undo pass on a pinned build; the complete runner still fails later at the unrelated Case collision control (see Answer) |
| [009: Verify persistence across reload](issues/009-verify-reload-persistence.md) | resolved | 008 | Saved edit and saved Undo result survive an actual reload with exact document/scene/identity/revision proof; the complete runner still fails later at the unrelated Case collision control (see Answer) |
| [010: Final technical verification and independent review](issues/010-final-technical-verification.md) | ready-for-agent | 009, 022 | Full combined gates and independent spec+standards review |
| [011: Populated-workspace human acceptance](issues/011-populated-workspace-acceptance.md) | ready-for-human | 010 | User review of the checked candidate across the complete review matrix |

Dependencies cover 008 through 009. 002 is resolved and 003 is `wontfix`.
The user cancelled 004–007 on 2026-11-06; they no longer block 010. Their
cancellation is not evidence the original behaviors were recovered or a waiver
of findings in final review. Reconcile acceptance scope with that decision.

## Added after the accepted breakdown

The user added these on 2026-11-06 after 002 and 003 closed. They do not block
010 or 011 unless explicitly stated; the user approved 022 as a blocker for 010.

| Ticket | Status | Blocked by | Delivers |
| --- | --- | --- | --- |
| [012: Review the mirrored-heading ADRs](issues/012-review-mirrored-heading-adrs.md) | resolved | none | ADR-0010 amended with one heading table and the controller rule; ADR-0039 pointer; Core unchanged (carries the A8 controller-heading item) |
| [013: Make a stale or updated build visible](issues/013-surface-build-updates.md) | ready-for-agent | none for coordination | Umbrella for specification 017 and implementation children; retains the user decisions |
| [017: Build update notification specification](issues/017-build-update-spec.md) | ready-for-agent | none for planning | Source-grounded behavior, test seams, assumptions and open questions for 013 |
| [018: Notify the editor when a build is waiting](issues/018-notify-waiting-build.md) | resolved | none | Real waiting installation to a persistent dismissible Toast, with mounted and packaged RED/GREEN |
| [019: Adopt the waiting build through protected Refresh](issues/019-adopt-build-safely.md) | ready-for-agent | 018 | "Update Now" in the Toast and a persistent header notice, with work/cache protection and real A-to-B adoption evidence |
| [014: Correct controller heading comments](issues/014-correct-controller-heading-comments.md) | resolved | none | Describe plain reflection accurately without changing Core behavior |
| [015: Decide headings for other y-symmetric parts](issues/015-decide-y-symmetric-part-headings.md) | resolved | none | Decided 2026-11-06: keep `180 - r`; no change |
| [016: Audit ADR decisions](issues/016-audit-adr-decisions.md) | resolved | none | All 40 indexed primary decisions audited; delivery gaps and uncertainties recorded |
| [020: Export split firmware with either half as central](issues/020-split-firmware-either-half-central.md) | ready-for-agent | none | ADR-0026 delivery: right or left central without reordering logical keys |
| [021: Stop repeated save Toasts](issues/021-repeated-save-toasts.md) | ready-for-agent | none | Reproduced zoom flood; agreed truthful save-feedback behavior and test seams in 023 |
| [022: Restore collision/provider browser proof](issues/022-restore-collision-provider-browser-proof.md) | ready-for-agent | none | Complete required collision/provider smoke and validate the success receipt; blocks 010 |
| [023: Truthful and quiet save feedback specification](issues/023-truthful-save-feedback-spec.md) | ready-for-agent | none | Ordinary autosave status in the header; no save notification for camera/navigation completion |

For the build-update addition, 013 remains the decision-owning umbrella; 017 is
the specification, 018 is complete, and 019 is now unblocked. Both children must
complete before 013 can close. These additions do not
block 010/011. Publication does not claim implementation or browser evidence.

## Evidence and decisions

- 009: Saved accepted edits and saved Undo results survive an actual reload of the same saved project (exact document, revision, project identity, rendered scene and mounted-document re-save/Undo proof); nine reload/Undo mutants killed by their intended messages. The complete runner still fails later at the unrelated Case collision control, so no full smoke pass is claimed. See [009](issues/009-verify-reload-persistence.md#answer).

- 016: Completed the bounded 40-record source audit; four acknowledged delivery gaps and explicit unverified areas remain. No ADR or implementation changes. See [016](issues/016-audit-adr-decisions.md#answer).

- 014: Controller comments now describe plain reflection; focused checks passed. See [014](issues/014-correct-controller-heading-comments.md#answer).

- Preserve the [2026-11-06 integration audit](../../research/2026-11-06-dev-ui-integration-audit.md)
  unchanged with this plan. Its findings A1–A8 retain their identifiers.
- The source candidate is `1fda4f48a964568b98dde9c82556350c4bc50911`;
  `510861b11` is the integrated behavioral reference, `c79de4708` the problematic
  reconciliation, and `ef1c8a008` the approved split-board prototype.
- The original plan retained the newer controls and deliberate shell changes,
  and proposed restoring placement, scope actions, disclosure motion and chevron
  binding with behavioral assertions and documentation. The user subsequently
  cancelled 004–007; those proposed repairs are not completed work.
- The original audit recorded MCU rotation (A1) as an unresolved user-observed
  symptom. The heading fix is present. Source-present engine work is integrated
  but has no new browser acceptance from this plan.
- 002 (2026-11-06): the user attributes the observed MCU orientation to a stale cached
  build, not the integrated candidate; no defect was reproduced and no Core or renderer
  change is indicated. The user closed 003 as not applicable (`wontfix`). See
  [002](issues/002-diagnose-mcu-rotation.md#answer).
- The source recheck adds explicit requirements for a real split-focus fixture,
  closing the scope hint when selection arrives, and reload verification after
  drag/save/Undo. Existing harnesses are starting points, not completed proof.
- The user reinvoked `to-spec` against the reviewed findings and corrected test
  seams on 2026-11-06. Those seams are carried into 001: mounted production UI,
  real Runtime/Session/Core workflows where relevant, and packaged-app browser
  scenarios. Independent review and human acceptance remain separate gates.

## Next step and boundaries

022 is the next unblocked recovery implementation ticket; 019 is also unblocked,
and 020 remains open pending its recorded full-gate limitation. Claim the selected
ticket before working it, keep execution bounded to that ticket, and retain the
next blockers and stopping point in its completion record. Final acceptance
remains open; a focused pass does not close 010 or 011.

Continue in the already-bound `fix/ui-integration-recovery` worktree. Preserve
the original `ci-wall-time-plan` and detached `dev-live-review` checkouts. This
publication step starts no repairs and authorizes no wholesale history merges,
push, merge to `dev`, or replacement of the user's live preview. Skills, lock
metadata and T3 setup remain local and Git-ignored.
