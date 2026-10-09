---
status: accepted
---

# Keep accepted outline refinements linked

Accepted boundary suggestions retain their relationship to the source components
and follow layout changes. The user accepted this behavior on 2026-09-29, with an
explicit Freeze action and a fixed copy for manual editing. This preserves the
convenience of generated geometry while making the transition to deliberate,
fixed geometry explicit.

A linked refinement follows the board's current included components, including
placement, geometry, additions and removals. Its own settings remain editable
without breaking the link. Removing a component removes only connections,
protected gaps and cutouts that require it; the outline and bridges recalculate
from the remaining components. Other authored choices remain intact.

If resolution cannot produce valid manufacturing geometry, keep the link active,
retain its clearly identified last valid display where necessary, highlight the
problem and block affected exports until it is resolved. Saving and further
editing remain available. A normal source change or component removal is not a
reason to freeze the refinement or keep stale geometry instead of recalculating.

Freeze creates a fixed copy while retaining the linked version. Each version owns
its outline choices independently; source references on copies record origin
rather than granting the source control over their shape. Save/reopen preserves
explicit linked/unlinked ownership; existing fixed copies do not gain links.

The recalculation, source-removal and persistence rules were confirmed on
2026-10-09 in [typed Core edits 03](../plans/typed-core-edits/issues/03-decide-outline-version-intents.md#answer).
This ADR records the product decisions, not completed implementation.
