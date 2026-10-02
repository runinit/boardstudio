# PCB07–11 independent Spec planning review — 2026-10-02

Disposition: bounded planning Spec clear. Read-only review in `f56-physical-intents-ticket06-20261002`; no source edits, parent graph edits, RF ledger edits or implementation acceptance.

Exact SHA256:
- Wiring spec: 021fbb7d2f7a14f3ef5bd2e6517ba35c39fe5774cbb3cfa06dfb909096963687
- Case physical-instance spec: d5580158d480071f2a4935cbada7885986fb1296df91b10c58fc6f6ea9eddab1
- 07: 2320b1404e4432d77547504d26c036e43bd932e52bdd73da4a2d59633ad8cd92
- 08: 4fb6ccc399efdc4dc176a1b78cb0383b54d0e891c2711923b160291ce5117296
- 09: 823ee411e339cee0ff0813429fa7d820c8a922c971c807f6c957310e95cd9df7
- 10: 81db0d42128134b1300ab726726cc00d677ad0540e2d1af7d034deb2ecd3e07d
- 11: f7dc3cea5530e701ee0ba840e1c0951d05c28fd18d3cbfe57ec66cabeff8da76

Reviewed both specs and all five child contracts against React usePcbWorkspace, WiringPanel and HardwareInstancesPanel, canonical parent requirements/joins and the current accepted PCB Inspector capability receipts. The start gate for07 explicitly scopes implementation of the missing private accepted-context callback rather than assuming one exists or requiring whole-parent completion. Exact observed outcomes and immutable accepted-document proposals remain mandatory.

Corrected08 preserves the real controller/inherited-switch branch ordering and limits editable input/generator controls to reachable generic definitions. Matrix eligibility correctly requires membership AND independence not forbidden.07 preserves board-specific net ownership, unrelated pin/net membership, separate create/map edits and accepted history.09 explicitly keeps the controller-selector discrepancy unresolved: React accepts the prop but exposes no selector. This review does not waive the parent criterion.10 preserves current plan fingerprint/revision protection and explicit remap review.11 matches actual selected instance boardId plus mechanical.boardId mutation, instance-only flipped mutation, and existing selection routing.

No remaining blocking planning findings at these hashes. Capability receipt, implementation source review, exact candidate paired public browser/history/reopen and F5.1/F5.2/F5.3/F5.6 acceptance joins remain separate gates. RF-001/RF-006/RF-009 carry-forward remains required. No new ledger category proposed.
