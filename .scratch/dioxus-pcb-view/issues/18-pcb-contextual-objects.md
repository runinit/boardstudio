# 18: Restore the contextual PCB Objects inventory

**What to build:** PCB shows the selected board and Outline hierarchy followed by flat actual part rows with definition/side details. Selecting a primary switch or other part opens its existing PCB wiring Inspector and updates the reference/path/count footer.

**Blocked by:** None at source start: accepted selected-board scene, explicit actual-part selection, outline tree and PCB Inspector are already mounted. Root serial integration and the one combined candidate qualification gate public completion. Full F5.1/F3.1 parent joins remain open.

**Status:** implementation-in-progress

- [x] PCB inventory matches the reference Board→Outline/versions/bridges→actual parts hierarchy and document order.
- [x] Each actual board part appears once with reference, definition name and front/back side; matrix primaries are explicit parts.
- [x] Selecting SW3 and an independent component reaches the existing scoped PCB Inspector and selection footer; selection kind changes to Part.
- [x] Layout physical-instance/grouping/project-heading controls are absent in PCB; Layout behavior remains contextual.
- [x] Outline disclosure and board collapse preserve accepted data/history and use the existing ownership guards.
- [ ] One combined affected check, changed paired browser receipt and consolidated candidate review are retained; all parent criteria and RF history remain.

Spec: [PCB contextual Objects](../drafts/F5.1b-pcb-contextual-objects.md). Flat inventory, SW3/R1 selection/footer and Outline/board disclosure are paired GREEN on34758. The non-reference Position section from34758 is repaired; actual34759 SW3/R1 delta is GREEN in [receipt](../evidence/18-contextual-objects-20261003/public-34759.json). The last combined-review criterion remains open. No full-parent acceptance is claimed. No new tests for this reversible UI projection.

- [x] Selected switch/component Inspector preserves the TypeScript wiring controls without Layout position/matrix authoring prepended. Actual34758 RED is retained in [receipt](../evidence/18-contextual-objects-20261003/public-34758.json).
