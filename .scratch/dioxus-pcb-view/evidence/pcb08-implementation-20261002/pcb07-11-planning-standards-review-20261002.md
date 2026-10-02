# PCB07–11 independent Standards planning review — 2026-10-02

Read-only snapshot at `/home/chris/.local/share/boardstudio/worktrees/f56-physical-intents-ticket06-20261002`.

Exact SHA256:
- F5.2-5.3 spec: 021fbb7d2f7a14f3ef5bd2e6517ba35c39fe5774cbb3cfa06dfb909096963687
- F5.6 spec: d5580158d480071f2a4935cbada7885986fb1296df91b10c58fc6f6ea9eddab1
- 07: b9c67771cd2c1000b97e8bf2150d213df4ecf83ab003c219bb1e5a2aa142b5c2
- 08: ac34f3b6cfe2191171db1164856e132f69150a1a42119c31b81e76520b9a1da8
- 09: 823ee411e339cee0ff0813429fa7d820c8a922c971c807f6c957310e95cd9df7
- 10: 81db0d42128134b1300ab726726cc00d677ad0540e2d1af7d034deb2ecd3e07d
- 11: f7dc3cea5530e701ee0ba840e1c0951d05c28fd18d3cbfe57ec66cabeff8da76
- publication manifest: f85a91cc1b5818173a72ebea7f00309a3c161e0ca58ef38a0df6b10eff53d813

## Disposition
07/09/10/11 bounded planning Standards clear. 08 held for the eligibility correction below. This is not implementation, publication, browser or parent acceptance. No source edits or tests performed for this document review.

## Findings
1. P2 — Ticket08 must preserve the actual reference branch predicate. `app/src/ui/usePcbWorkspace.tsx:55–61` returns the inherited/read-only panel for every `definition.kind === 'switch'` (and routes controllers to board Wiring) before the Press input/generator inspector. Ticket08 instead says selecting a supported switch exposes scan controls and calls out “eligible switch/rotary definitions.” Narrow the contract to reachable non-switch/non-controller generic parts with the actual press-profile or EC11/EC12 generator predicate. The broader spec already preserves inherited switch routing; do not introduce a new editable switch surface under parity.
2. P3 — Ticket07's part-Inspector-journey link uses `../evidence` from `drafts/tickets`; use `../../evidence`. The evidence itself was reviewed at its actual path, so this does not block the bounded07 implementation start.

## Boundaries checked
07 truthfully requires a new narrow private edit owner at the accepted Inspector mount; it does not assume an existing callback. Captured document/session/token/revision/board/part identity, board instance=None scope, exact operation outcome registration/retention, immutable ReplaceDocument and ordinary validation/history remain required. Net creation must update the selected board's netIds rather than accidentally taking Core SetNet's first-board behavior. Terminal/pad reassignment preserves unrelated pins and nets. Mapping and net creation are separate edits.

09 retains the unresolved reference controller-selector discrepancy rather than inventing a new control or closing the parent. 10 retains exact revision/fingerprint guarded apply/remap, accepted history and protected-handoff distinction. 11 matches HardwareInstancesPanel's actual boardId plus mechanical.boardId update and instance-only flip, including onSelect routing; it creates no second selection authority.

Existing planning evidence explicitly lacks final exact-candidate, same-document public parity. Those limitations and F5.1 scene selection, save/reopen, asynchronous currentness, accessibility and parent acceptance remain open. Existing RF-001 composition, RF-006 scope and RF-009 provenance handoffs remain applicable; no new ledger category needed.

## Corrected07/08 recheck

07 SHA256 `2320b1404e4432d77547504d26c036e43bd932e52bdd73da4a2d59633ad8cd92`: journey link fixed to../../evidence; clear.
08 `6e03ea9691d3ef4614f98d746b0aa1ca38ccbac9d2cdf0f1490770286cc8c7a6`: inherited switch/controller branch correction is accurate. One newly added boolean sentence remains wrong: “disabled unless ... matrix contains ... or ... profile does not forbid” suggests OR eligibility. Actual source disables on `!membership || press.independent === false`, so enabled requires membership AND independence not forbidden. Author requested exact correction. Remaining08 hold is only this sentence; previous broader source routing finding is resolved.

Final08 reack: SHA256 `4fb6ccc399efdc4dc176a1b78cb0383b54d0e891c2711923b160291ce5117296` fixes the AND eligibility exactly. Standards planning clear for corrected07 `2320b140…8cd92`, corrected08 `4fb6ccc3…17296` and previously reviewed09/10/11/specs. Capability receipts, second-axis review/publication and all implementation/browser/parent gates remain distinct; this review closes only its documented planning findings.
