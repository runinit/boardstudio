# UI integration recovery

Recover the approved Layout workflows on the newer shared controls and complete
the missing acceptance evidence for the reconciled application.

## Planning ticket

- [001: UI integration recovery specification](issues/001-ui-integration-recovery-spec.md)
  — `ready-for-agent`; owner `fix/ui-integration-recovery`. The parent specification
  remains unchanged; bounded technical verification is resolved in 010 and product
  acceptance remains open in 011.

## Accepted child tickets

The user accepted this ten-ticket breakdown on 2026-11-06. A4 regression
assertions and A8 documentation updates travel with the relevant behavior;
neither is deferred to a separate cleanup ticket.

| Ticket | Status | Blocked by | Delivers |
| --- | --- | --- | --- |
| [002: Diagnose MCU rotation](issues/002-diagnose-mcu-rotation.md) | resolved | none | Closed on the user's stale-build report; original symptom not reproduced |
| [003: Resolve the diagnosed MCU symptom](issues/003-resolve-mcu-orientation.md) | wontfix | 002 | Not applicable; closed by the user (see 002). Evidence-backed correction and original-scenario verification |
| [004: Restore placement and Relationships](issues/004-restore-placement-relationships.md) | ready-for-agent (reopened 2026-11-08) | none | Placement/Relationships; scope to follow 011 defect slices |
| [005: Restore nonduplicated scope actions](issues/005-restore-scope-actions.md) | ready-for-agent (reopened 2026-11-08) | none | Scope actions including D10; scope to follow 011 defect slices |
| [006: Restore shared disclosure motion](issues/006-restore-disclosure-motion.md) | ready-for-agent (reopened 2026-11-08) | none | Disclosure motion; scope to follow 011 defect slices |
| [007: Correct disclosure chevrons](issues/007-correct-disclosure-chevrons.md) | wontfix | none | Cancelled by the user; no completion evidence |
| [008: Restore real drag/save/Undo proof](issues/008-restore-drag-save-undo-proof.md) | resolved | none | Drag, save, cancel and exact Undo pass on a pinned build; the complete runner still failed later at the unrelated Case collision control at that time (see Answer) |
| [009: Verify persistence across reload](issues/009-verify-reload-persistence.md) | resolved | 008 | Saved edit and saved Undo result survive an actual reload with exact document/scene/identity/revision proof; the complete runner still failed later at the unrelated Case collision control at that time (see Answer) |
| [010: Final technical verification and independent review](issues/010-final-technical-verification.md#answer) | resolved | 009, 022 | Fresh full gate/drag/floor and strict CSS pass; required Standards finding resolved, Spec PARTIAL with explicit aggregate-coverage acceptance; 011 remains the human gate |
| [011: Populated-workspace human acceptance](issues/011-populated-workspace-acceptance.md) | ready-for-human | 010 | User review of the checked candidate across the complete review matrix |

Dependencies cover 008 through 009. 002 is resolved and 003 is `wontfix`.
The user cancelled 004–007 on 2026-11-06; they did not block the completed 010.
On 2026-11-08 the user reopened 004–006 during 011 review; 007 remains cancelled.
Cancellation is not evidence the original behaviors were recovered or a waiver
of findings in final review. Scope reopened work after the 011 defect slices.

## Added after the accepted breakdown

The user added these on 2026-11-06 after 002 and 003 closed. They do not block
010 or 011 unless explicitly stated; the user approved 022 as a blocker for 010.

| Ticket | Status | Blocked by | Delivers |
| --- | --- | --- | --- |
| [012: Review the mirrored-heading ADRs](issues/012-review-mirrored-heading-adrs.md) | resolved | none | ADR-0010 amended with one heading table and the controller rule; ADR-0039 pointer; Core unchanged (carries the A8 controller-heading item) |
| [013: Make a stale or updated build visible](issues/013-surface-build-updates.md#answer) | resolved | none for coordination | Umbrella reconciled against resolved 018/019 and draft-safety proof; user decisions and residual risks retained |
| [017: Build update notification specification](issues/017-build-update-spec.md#answer) | resolved | none for planning | Stories and seams discharged by reviewed 018/019 and draft-safety; historical source limits explicitly superseded |
| [018: Notify the editor when a build is waiting](issues/018-notify-waiting-build.md) | resolved | none | Real waiting installation to a persistent dismissible Toast, with mounted and packaged RED/GREEN |
| [019: Adopt the waiting build through protected Refresh](issues/019-adopt-build-safely.md#answer) | resolved | 018 | Protected Update Now reviewed Spec/Standards PASS; combined packaged draft + held-save proof, exact reopening and retained risks recorded |
| [014: Correct controller heading comments](issues/014-correct-controller-heading-comments.md) | resolved | none | Describe plain reflection accurately without changing Core behavior |
| [015: Decide headings for other y-symmetric parts](issues/015-decide-y-symmetric-part-headings.md) | resolved | none | Decided 2026-11-06: keep `180 - r`; no change |
| [016: Audit ADR decisions](issues/016-audit-adr-decisions.md) | resolved | none | All 40 indexed primary decisions audited; delivery gaps and uncertainties recorded |
| [020: Export split firmware with either half as central](issues/020-split-firmware-either-half-central.md#answer) | resolved | none | Either central side preserves physical key/sensor order; gate green and wired/encoder Runtime proof gaps closed 2026-11-08 (see Answer) |
| [021: Stop repeated save Toasts](issues/021-repeated-save-toasts.md#answer) | resolved | none | Header-only autosave success and silent view completion; packaged checks and full-gate rerun pass; initial gate cause unproven (see Answer) |
| [022: Restore collision/provider browser proof](issues/022-restore-collision-provider-browser-proof.md#answer) | resolved | none | Closed on user approval: 17 packaged smoke checks and validated receipt; independent review has 0 required and 2 optional findings; unblocks 010 |
| [023: Truthful and quiet save feedback specification](issues/023-truthful-save-feedback-spec.md#answer) | resolved | none | Policy implemented by 021; DESIGN autosave exception documented; decisions unchanged |

For the build-update addition, 013 remains the decision-owning umbrella; 017 is
the specification. 018 and reviewed 019 are resolved, and 013/017 are reconciled
and resolved with their evidence and residual risks preserved. 020/021/023 are
also resolved as recorded above. These closures do not establish human acceptance
in 011 or close parent 001.

## Evidence and decisions

- 010 (2026-11-08): Source `18d4d7f53` passed the fresh full combined gate (442.9s,
  including 17 drag/reload checks and the accessibility floor) and strict old-CSS
  0/0. Required Standards evidence-hygiene finding resolved; optional debt accepted.
  Spec remains PARTIAL: aggregate per-ticket reviews, sampled cross-checks and gate
  evidence were explicitly accepted without claiming a full 445-path line-read.
  Candidate identities, retained-proof rationale, cancellations and 011 handoff
  are in [010's Answer](issues/010-final-technical-verification.md#answer).

- 009: Saved accepted edits and saved Undo results survive an actual reload of the same saved project (exact document, revision, project identity, rendered scene and mounted-document re-save/Undo proof); nine reload/Undo mutants killed by their intended messages. The complete runner still failed later at the unrelated Case collision control at that time, so no full smoke pass was claimed. See [009](issues/009-verify-reload-persistence.md#answer).

- 016: Completed the bounded 40-record source audit; four acknowledged delivery gaps and explicit unverified areas remain. No ADR or implementation changes. See [016](issues/016-audit-adr-decisions.md#answer).

- 014: Controller comments now describe plain reflection; focused checks passed. See [014](issues/014-correct-controller-heading-comments.md#answer).

- Preserve the [2026-11-06 integration audit](../../research/2026-11-06-dev-ui-integration-audit.md)
  unchanged with this plan. Its findings A1–A8 retain their identifiers.
- The original planning source candidate was `1fda4f48a964568b98dde9c82556350c4bc50911`;
  `510861b11` is the integrated behavioral reference, `c79de4708` the problematic
  reconciliation, and `ef1c8a008` the approved split-board prototype.
- The original plan retained the newer controls and deliberate shell changes,
  and proposed restoring placement, scope actions, disclosure motion and chevron
  binding with behavioral assertions and documentation. The user subsequently
  cancelled 004–007; those proposed repairs are not completed work. The user
  reopened 004–006 on 2026-11-08; 007 remains cancelled.
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

010 is resolved with the combined technical evidence and explicit review
coverage disposition in its completion record. 013/017/019/020/021/022/023 are
resolved; their historical limitations and retained risks remain in their tickets.
011 human acceptance remains open across its complete surfaces ×
Themes/Appearances × widths matrix. Address the ordered 011 defect slices, then
scope reopened 004/005/006 as their own work. Corrections require refreshed 010
verification and affected human re-review per 011 AC7; no approval is recorded.
011 remains `ready-for-human`; technical completion does not close it or parent 001.

Continue in the already-bound `fix/ui-integration-recovery` worktree. Preserve
the original `ci-wall-time-plan` and detached `dev-live-review` checkouts. This
publication step starts no repairs and authorizes no wholesale history merges,
push, merge to `dev`, or replacement of the user's live preview. Skills, lock
metadata and T3 setup remain local and Git-ignored.
