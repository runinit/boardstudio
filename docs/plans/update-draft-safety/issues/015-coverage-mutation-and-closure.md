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

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above and add none. They cite 008–014 evidence and do not repeat any owner matrix. The
optimized and full gates are `pending`; every other row `planned`.

- P1 All 14 — the settled census of 14 Class A owners ([006 handoff](006-agree-verification-and-implementation-handoff.md#answer))
  against the adapters registered at the closing `HEAD` → one audit row per owner naming its passing
  mounted deferral evidence from 008–014. Oracle: the census, not the registry's own list; an owner
  without passing evidence is a blocker. Native source counts and test-graph reach are not behavioural
  proof. Class B: no adapter added, risk acceptance and revocation trigger intact. Tier `native` audit
  over `mounted` evidence.
- P2 Copy remedies — the per-(owner, admission key) label and remedy table → compared with the
  settled copy rule → every key names its consuming action, initial and installed copy stay
  distinct. Oracle: the copy rule and the table, reviewed. Tier `native`.
- P3 Packaged Layout constraint — a real unapplied Layout constraint draft in the packaged build →
  trusted Update Now, then Cancel → the update defers and the draft stays intact. Oracle: packaged
  deferral and unchanged draft. The coordinator decides whether the accepted 010 Layout leg already
  meets this oracle (`reused`) before adding a leg (`planned`). Tier `packaged`.
- P4 Mutation — weakened-gate mutants over registry polling, pruning and the per-key unresolved
  branches → `scripts/mutation-check.py` → each mutant fails a test. Oracle: the tool's killed and
  survived report; a survivor is not caught and blocks. Tier `native`/`mounted`.
- P5 Optimized and full gate — coordinator-owned: rebuild the disposable packages deleted after 010 →
  optimized validation, then the full `python3 scripts/check.py` gate → receipt recorded. Oracle: the
  gate's own result. `pending`.
- P6 Final review — one combined Standards and Spec review per the
  [review-report protocol](../../../agents/issue-tracker.md#review-reports) on fixed endpoints with every
  changed file read. Acceptance is `READY` only when P1–P5 and the evidence criteria hold, including
  the dead-entry pruning diagnostic surface and the packaged-leg cleanup attestations in a small
  completion record.
