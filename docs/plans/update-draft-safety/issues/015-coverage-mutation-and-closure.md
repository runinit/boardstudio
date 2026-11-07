# 015: Complete owner coverage, mutation controls and verification closure

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Packaged A→B draft-safety proof](010-packaged-a-to-b-proof.md), [Layout explicit-submit owners](011-layout-explicit-submit-owners.md), [Parts single-draft owners](012-parts-single-draft-owners.md), [PCB placement, attach, assembly and new-net owners](013-pcb-placement-attach-assembly-net-owners.md), [Creation-form owners](014-creation-form-owners.md)

## What to build

The verification closure for draft-safe Update Now: prove every one of the
fourteen Class A owners from the census is protected, complete the
per-(owner, admission-key) label and remedy table, run the mutation controls
over the safety gate, add the packaged Layout-constraint leg, and close with
the full project gate and the combined spec-and-standards review. Closing this
ticket completes this spec only; it does not close the parent adoption
umbrella or claim its final acceptance.

## Acceptance criteria

- [ ] Coverage audit: every Class A owner in the settled census maps to a registered adapter with passing mounted deferral evidence; any owner found unprotected is a blocker, not a waiver.
- [ ] The per-(owner, admission-key) label and remedy table is complete and reviewed against the settled copy rule.
- [ ] Packaged Layout-constraint leg passes: a real unapplied Layout constraint draft defers packaged Update Now and survives cancellation intact.
- [ ] Mutation and rejection controls rerun over the registry polling, pruning and per-key unresolved branches using the project's mutation-check tooling; weakened-gate mutants are caught.
- [ ] Class B remains an explicit, revocable risk acceptance with its recorded revocation trigger; no Class B adapter is added and no Class B loss is waived by this closure.
- [ ] Full project check gate passes with its receipt recorded; one combined spec-and-standards review is completed and its findings resolved.
- [ ] Evidence is logs-only in the evidence directory, reporting results and relevant failures; the completion record names the diagnostic surface chosen for dead-entry pruning and the cleanup attestations from the packaged legs.
