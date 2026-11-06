# 009: Verify persistence across reload

Status: ready-for-agent
Type: build
Blocked by: [008: Restore real drag/save/Undo proof](008-restore-drag-save-undo-proof.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Audit A5](../../../research/2026-11-06-dev-ui-integration-audit.md#a5--what-the-existing-green-results-actually-cover)

## What to build

Extend the repaired packaged-app workflow to prove that saved edited geometry
and a saved Undo result survive an actual page reload/reopen. This adds the
missing persistence boundary without inventing a requirement to preserve the
in-memory Undo stack across reload.

## Acceptance criteria

- [ ] Use the real gesture and durable-save setup established by 008. Capture
  the expected document and rendered geometry from the accepted edit, then reload
  or reopen the same saved project and compare the restored result.
- [ ] In a separate scenario where needed, perform an edit, Undo it before
  reloading, verify the Undo result was saved, then reload/reopen and verify the
  restored document and geometry match that saved Undo result.
- [ ] Prove project identity and persistence across the boundary; do not replace
  loading with a direct fixture rewrite or rely only on the save indicator.
  The unrelated Case setup reload is not evidence for these scenarios.
- [ ] Preserve 008's exact document/scene restoration assertions and semantic
  comparison rules. Do not relax numeric tolerances or normalize away an actual
  edit discrepancy to make reload pass.
- [ ] Make the new assertions red-capable and demonstrate that they detect a
  wrong persisted/restored result before recording GREEN (A4). Use the repository
  mutation runner if mutation checks are needed for that evidence.
- [ ] If reload exposes a product defect, diagnose the actual workflow before
  correcting it. Report a separate blocker if the required repair exceeds this
  bounded persistence behavior; do not substitute a weaker assertion.
- [ ] Record current source/build/browser identity, both scenario results and
  the distinction between pre-reload Undo and post-reload restoration (A8).
  Keep the original audit and parent spec unchanged.

## Verification and completion

Run both packaged-app round trips through the existing supported runner and
rerun the affected 008 scenarios after extending the harness. Record exact
invocations, failing controls and passing results. Keep focused round-trip proof
separate from the full reconciled-candidate gate in 010.

## Comments

- 2026-11-06: Published as the accepted follow-on to 008, providing independently
  verifiable reload coverage once the existing gesture/save/Undo path works.
