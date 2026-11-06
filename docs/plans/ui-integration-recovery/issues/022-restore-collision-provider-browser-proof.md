# 022: Restore collision/provider browser proof

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [008 results](008-restore-drag-save-undo-proof.md#answer), [009 results](009-verify-reload-persistence.md#answer), [010 final gate](010-final-technical-verification.md)

## What to build

Restore the required packaged-app collision/provider smoke workflow against the
current shared controls. Complete the scenarios left unreached after the Case
mounting-style control failure and obtain a valid success receipt without
weakening geometry, qualification, uncertainty or provenance assertions.

008 and 009 already prove their bounded drag/save/Undo/reload outcomes; their
complete runner still fails because Case mounting style is no longer a native
select. Preserve those outcomes while repairing the remaining harness boundary.

## Acceptance criteria

- [ ] Reproduce the recorded native-select failure on an identified packaged
  build using the supported runner. Distinguish harness assumptions from actual
  product defects; diagnose any newly exposed failure before prescribing a fix.
- [ ] Drive the real accessible Case mounting-style control and any other stale
  control or save-status observations required by this same gate. Required
  preconditions fail explicitly; no direct fixture rewrite substitutes for the
  user interaction under test.
- [ ] Complete the default collision PCB pair, Case viewer with overflowing
  findings, qualified collision product fixture and unavailable-PCB uncertainty
  scenario, retaining the existing meaningful assertions. Record terminal
  results for each scenario rather than treating an unreached check as passed.
- [ ] Keep the existing trusted drag, cancellation, durable save, exact Undo and
  reload checks green. Preserve document/scene equality, revision checks,
  qualification semantics and uncertainty handling; do not loosen tolerances or
  remove checks to obtain a passing run.
- [ ] Demonstrate RED/GREEN for the restored harness behavior and run focused
  harness/consumer controls. Preserve failure behavior: a failed or incomplete
  scenario must produce no success receipt.
- [ ] Run the complete supported smoke workflow to success and validate its
  receipt against the actual source, packaged bytes and browser identity. Keep
  the complete smoke result distinct from 010's broader full default gate.
- [ ] Update the relevant evidence and historical gate records with exact
  commands, tested identities, results and remaining limits, preserving earlier
  failures as history. Use a dedicated browser profile/origin, not the user's
  live preview.

## Verification and scope

Use the existing supported Rust browser harness and runner, existing receipt
validators and repository mutation runner where mutation controls are needed.
Run applicable formatting, lint and harness tests, followed by the complete
packaged smoke workflow. Obtain independent spec and standards review before
closure.

Unrelated opt-in scripts and wholesale browser-tooling migration are outside
this slice. Change another consumer only when evidence shows it is required by
this gate. If a product defect or additional independent repair exceeds this
bounded workflow, record a separate blocker instead of expanding silently.
Ticket 021's save-Toast policy and 019's update-adoption behavior are separate.

## Comments

- 2026-11-06: The user approved this single-ticket scope and making it a blocker
  for 010. No implementation or new browser run was performed during publication.
