# Map: Layout part-move pointer transaction

## Destination

Concentrate the ordinary Layout 2D part-move pointer transaction at the Layout
canvas handoff without taking ownership of selection, pan navigation, or Session
commit semantics away from their existing authorities.

## Ticket

- [001: Own the Layout part-move pointer transaction](issues/001-layout-part-move-pointer-transaction.md) — `ready-for-agent`, the single implementation ticket. Its real-page drag-to-durable-save-and-Undo test is the primary acceptance seam.

## Dependencies and coordination

- Specification input: [architecture candidate 01 research](../../research/2026-10-20-layout-pointer-transaction-deepening.md), [glossary](../../../GLOSSARY.md), [ADR-0005](../../adr/0005-resolve-queued-edits-at-execution.md), [ADR-0014](../../adr/0014-document-authority-and-captured-lifetimes.md), and [ADR-0021](../../adr/0021-test-real-workflows-natively-and-browser-behavior-mounted.md).
- No prerequisite ticket in this plan. Coordinate any overlapping Layout canvas work with [Editor workspace extraction 012](../editor-workspace-extraction/issues/012-layout-canvas-and-gesture-ownership.md); that broader ticket is related work, not authorization to expand 001 into a canvas-wide extraction. Refresh the current composition before implementing either overlapping slice.
- Test dependencies: a freshly built mounted page and browser storage for the trusted-pointer, durable-save, exact-Undo flow; existing Session/native gesture tests supplement, but cannot replace, browser evidence.
