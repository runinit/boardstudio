---
status: accepted
---

# Keep edited board outlines fixed

Generated outlines follow the current component layout, but an edited outline
is a separate copy whose geometry remains fixed when components move. The user
selected this behavior on 2026-09-29 to preserve deliberate perimeter edits;
layout changes must produce clearance findings rather than silently reshape
the copy, and the generated source remains available independently.

This trades automatic adaptation for predictable ownership of manual geometry.
Findings must highlight the affected locations on the workbench while leaving
the outline visible and editable; blocking findings prevent export rather than
further outline editing. Advisory findings remain distinguishable from blockers.
Blocking findings affect outline, PCB, plate and case exports that depend on the
invalid active geometry. Project saving stays available; inactive alternatives
and unrelated boards do not block valid output. Invalid or unintentionally
disconnected geometry, missing required pad/drill support, and violations of
configured edge-clearance or connection-width rules are blockers. Keycap overhang
and explicitly permitted component-body overhang remain advisory. Protected gaps
preserve deliberate geometry without waiving required support rules.

Accepted boundary alternatives follow the separate
[linked-refinement decision](0002-linked-outline-refinements.md).
This ADR records the product decisions, not completed implementation.
