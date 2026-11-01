# 07: Prototype split editing view

Status: resolved
Type: prototype
Blocked by: [04](04-decide-adaptive-split-gap.md), [05](05-decide-dim-and-focus-interaction.md), [06](06-decide-relationship-highlighting-home.md)
Map: [map.md](../map.md)

## Question

Build a clickable prototype combining the adaptive gap preview, dimmed mirror,
and relationship highlights per the 04/05/06 decisions.

## Outcome

A prototype (branch or scratch, not production) demonstrating gap preview, dimmed
mirror with click-to-focus, and relationship highlights together.

## Prototype review round 1 (2026-10-25, user)

Prototype: `web/crates/layout/prototype/split-board-editing.prototype.html`, archived on the
local branch `prototype/split-board-editing` (ef1c8a008).

- Variant **B** chosen: dimmed half as a grey dashed outline with an inline "click to edit
  here" label; "How it is placed" as a breadcrumb above the Inspector fields; widening leaves a
  dashed ghost of the previous spacing.
- Parts keep half-local coordinates. Display offset is separate from placement: moving a part
  toward the split widens the gap, and the part's and columns' stored coordinates do not change.
- A part cannot be dragged across the split axis.
- An existing board that already touches widens on its first committed edit of any kind.
- No touching: a typed gap below the minimum is raised back to it.
- Clearance should be comfortable but not too wide ("the prototype UIs have a good spacing").

Still open:

- Is clearance outline to outline = 2 × margin (envelopes 4 × margin apart)? This is a proposed
  reading of "comfortable".
- A part at the axis is not mirrored. With symmetric widening and no crossing, the other half
  gets pushed out by the same amount (a 90° Controller gives 27.5 mm of air). The alternative is
  to let a turned part straddle the axis.
- The stored gap is measured at the half-origin column. Should it be measured at the envelope?
- Creation formula: the prototype uses max(40 mm, the minimum gap).

## Prototype review round 2 (2026-10-25, user)

- Spacing adapts in both directions. Moving a part back from the split narrows the spacing
  again, but never below the stored gap. This replaces "never auto-shrink" (04 Q1).
- In this scenario a new controller is mirrored on both halves by default. The Inspector can
  make it local to one half.
- The Layout canvas shows anchor relationships for the selection: an anchor badge with a label
  at what the selection is placed from, a violet dashed highlight on that target, and a dashed
  tie to the selection.

## Prototype review round 3 (2026-10-25, user)

- Parts use the same layout vocabulary as a column:
  - an anchor key with Follow X/Y/rotation, Pick anchor key and Detach;
  - carried from the anchor: Spread, Stagger and Splay;
  - for the part only: Offset and Rotation.
- The canvas anchor badge, the Inspector chain and Relationships cover parts too.
- Proposed (not confirmed): a dropped part anchors to the nearest key of its half and keeps the
  drop point upright. Detach keeps the pose and places the part from the half origin.

## Answer

Approved by the user on 2026-10-25. Primary source: local branch `prototype/split-board-editing`
(ef1c8a008), commit `prototype: archive approved split-board editing view (ticket 07)`.

- **Presentation, variant B:**
  - the dimmed half is a grey dashed outline with an inline "click to edit here" label;
  - "How it is placed" is a breadcrumb above the Inspector fields;
  - spacing changes leave a dashed ghost of the previous spacing.
- **Spacing:**
  - the Split gap is stored and editable;
  - after every committed edit, spacing = max(stored gap, minimum), symmetric about the axis and
    in the same Undo entry;
  - spacing narrows back as well as widens (this replaces 04's "never auto-shrink");
  - outlines never touch;
  - existing boards are not migrated on open.
- **Parts:**
  - stored in their half's own frame, so spacing never changes their coordinates;
  - cannot cross the split axis;
  - a new controller is mirrored on both halves by default, and can be made local;
  - placed with the column vocabulary: an anchor key (Follow, Pick, Detach), Spread, Stagger and
    Splay carried from it, and Offset and Rotation for the part only.
- **Relationships:**
  - the Layout canvas shows what the selection is anchored to: an anchor badge with a label, a
    violet dashed target, and a dashed tie to the selection;
  - the Inspector keeps the clickable chain, the Relationships facts and the amber
    "Moves N keys" field previews.
- **Carried to 08, to confirm or settle while slicing:**
  - clearance is outline to outline = 2 × outline margin (envelopes 4 × margin apart);
  - a dropped part anchors to the nearest key of its half (proposed in the prototype; the user
    approved the prototype as a whole);
  - a local, unmirrored part at the axis pushes the other half out by its overhang;
  - the stored gap is measured at the half-origin column;
  - creation gap uses max(40 mm, minimum).
