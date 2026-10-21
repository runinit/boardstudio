# Export admission

Candidate 02 from [export readiness/dispatch research](../../research/2026-10-20-export-readiness-dispatch-deepening.md): align displayed, output-specific prerequisites with click admission while preserving export ownership and delivery checks. “Available” means currently eligible to begin, not guaranteed delivery ([glossary](../../../GLOSSARY.md#mechanical-relationships)).

## Tickets

1. [001: ZMK export admission](issues/001-zmk-export-admission.md) — `ready-for-agent`; establish the admission seam with the demonstrated firmware-prerequisite gap.
2. [002: PCB and design-file export admission](issues/002-pcb-design-file-admission.md) — `ready-for-agent`, blocked by 001; ordinary/Draft KiCad, SVG/DXF and footprints.
3. [003: Geometry-package export admission](issues/003-geometry-package-admission.md) — `ready-for-agent`, blocked by 001; keycap STLs, authored Case STEP and generated mechanical packages. Independent of 002 once 001 establishes the seam.

## Dependencies

- Implement 001 first; 002 and 003 can proceed separately afterward. This work is independent of the parallel Layout architecture candidate; coordinate only if implementation touches a shared presentation seam.
- Accepted document/Session authority and captured export lifetimes remain governed by [ADR-0014](../../adr/0014-document-authority-and-captured-lifetimes.md) and [ADR-0016](../../adr/0016-export-ownership-is-separate-from-preview-currency.md). The ticket must preserve them, not replace them.
