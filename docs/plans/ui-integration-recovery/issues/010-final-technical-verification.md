# 010: Complete final technical verification and independent review

Status: ready-for-agent
Type: task
Blocked by: [009](009-verify-reload-persistence.md), [022](022-restore-collision-provider-browser-proof.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Migration gate record](../../rust-ui-build/issues/026-merge-gate.md)

## What to build

Establish technical acceptance of the combined recovery candidate: complete
final-source checks and independent spec+standards review of the reconciled
migration, including inherited integration behavior. Deliver a precisely
identified checked candidate for populated-workspace human review in 011.

## Acceptance criteria

- [ ] All listed blockers are resolved with their required evidence. 003 is `wontfix` (not applicable)
  and 008 is covered through 009. Do not infer completion from source
  presence, diagnosis alone or a ticket title/status without its acceptance.
- [ ] Run the full repository default check, including the full browser gate,
  against the final source. Record exact invocation, source/build/browser
  identity and all terminal results; selected-crate and quick plans are not
  substitutes. A completed full run need not be duplicated without a reason.
- [ ] The real drag/save/Undo and reload scenarios run against the combined
  candidate, with persistence/scene assertions and receipt checks intact.
  Retained older or individually focused runs do not establish this result.
- [ ] Complete the existing accessibility floor and strict old-CSS check. State
  what they establish and any unrun broader accessibility validation.
- [ ] Obtain independent spec and standards review through the repository's
  two-axis review workflow. Cover the reconciled migration candidate relative
  to the integrated behavioral reference, not merely the last repair diff.
- [ ] Resolve or explicitly disposition review findings and recheck corrections.
  Outstanding acceptance blockers keep this ticket open. New unrelated work is
  separately scoped rather than absorbed as unlimited final-gate cleanup.
- [ ] Ensure evidence still applies to the delivered source after corrections.
  Record which checks were rerun and why any retained evidence remains valid;
  do not silently promote results from a different candidate.
- [ ] Reconcile the migration/recovery records with the actual gate and review
  results (A8), preserving historical receipts and keeping human acceptance open.
  Do not change or close the parent specification.

## Verification and completion

The required checks are the full default/browser plan, the accessibility floor,
the strict old-CSS check and independent spec+standards review. Document their
commands, results and source identities in the ticket's completion record, with
long logs kept in the repository's evidence area. No merge, push or live-preview
replacement is authorized by this gate. Product acceptance still requires 011.

## Comments

- 2026-11-06: The user approved 022 as an explicit blocker for this gate. 008 and
  009 are resolved within their bounded scopes, but the complete smoke runner
  still fails at Case mounting style and has no success receipt.
- 2026-11-06: Published with direct dependencies only. This ticket separates
  combined technical evidence from the user's populated-workspace acceptance.
- 2026-11-06: The user cancelled 004–007; they are no longer dependencies.
  Cancellation does not establish their behaviors or waive outstanding review
  findings. Reconcile final acceptance scope with that decision when running
  the independent review and human acceptance.
