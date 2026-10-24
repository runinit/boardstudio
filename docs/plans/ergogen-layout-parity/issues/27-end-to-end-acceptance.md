# 27: Verify the complete cutover and release the integration gate

Status: claimed
Type: build
Blocked by: [26](26-rebuild-examples-and-fixtures.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)
Ownership: Owns only the end-to-end acceptance checklist and release gate; fixture regeneration and cutover gates are left to [17e](17e-fixtures-and-final-gate.md) and the example rebuilds to [26](26-rebuild-examples-and-fixtures.md).

## Outcome

Prove the whole accepted scope through production paths and permit promotion of the
version-7 implementation only after the coordinated lane is complete. Planning map
closure is not product completion, and aggregate anchors remain explicitly deferred.

## First RED / GREEN slice

Start with an end-to-end traced case from public edit/preview through materialization,
scene, saved archive and reopen, including an anchored mirrored group. If it exposes
a gap, add the smallest behavioral regression at the relevant agreed seam and fix
that path before taking another scenario. Do not replace real routes with test doubles.

## Release acceptance

- Account for every decision/spec scenario and every preserved dependent contract;
  check primitive golden parity and real prepared edit/lifecycle coverage separately.
- Verify strict v7 gates at all entries, no old-data writes/migrations/wipes, stale-tab
  and storage-failure safeguards, raw-data truthfulness and history/draft lifecycle.
- Verify curved/rotated/anchored/mirrored placement, origin compensation, Local/Omitted
  graph/resize/removal/confirmation, constraints/companions, column Align and key sizes.
- Verify source/receiver reference highlighting versus amber actual preview, hidden
  contributors, keyboard routes, real trusted drags, cancellation and exact Undo.
- Verify the three editable examples, member/scan/keymap identity, scene/materialized
  consistency and affected outline/PCB/CAD/export consumers. No fabrication claim.
- Remove staging leftovers, old vector/formula readers and legacy oracle activation;
  retain deliberate negative fixtures and unrelated behavior. Do not merge prototype
  code, flags, duplicate geometry or bulk-regenerated unrelated snapshots.
- Run one combined spec/standards closure review of the actual cross-crate change.
  Route concrete findings to their owning build tickets, not silent spec relaxation.

## Checks and evidence

Run `python3 scripts/check.py` for the full pre-PR scope, including its checked build,
native/contract/provider and mounted browser coverage. Use README prerequisites and
the scheduler; native-only or `browser --only` evidence is partial. Collect the
supported check receipt under `.evidence/`, observed test counts/names, trusted-input
engine and any failures/unrun environments. Never claim planned checks passed. Hardware
verification remains downstream; unresolved required acceptance blocks this ticket.
