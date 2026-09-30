---
status: accepted
---

# Keep accepted outline refinements linked

Accepted boundary suggestions retain their relationship to the source components
and follow layout changes. The user accepted this behavior on 2026-09-29, with an
explicit Freeze action and a fixed copy for manual editing. This preserves the
convenience of generated geometry while making the transition to deliberate,
fixed geometry explicit.

If a source disappears or the refinement becomes invalid, retain its last valid
shape, highlight the problem on the workbench, and block affected exports until
the problem is resolved. Freeze creates a fixed copy while retaining the linked
version. Each version owns its outline choices independently; source references
on copies record origin rather than granting the source control over their shape.
This ADR records the product decisions, not completed implementation.
