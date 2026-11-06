# 09: Prototype the Inspector and toolbar for the new parameters

Status: resolved
Type: prototype
Blocked by: [01](01-name-the-carried-parameters.md), [04](04-layout-kernel-order-of-operations.md)
Map: [map.md](../map.md) · Decision: [ADR-0036](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Make a rough, clickable prototype of how users edit:

- column spread;
- row padding;
- Shift and Bend, at row and key level;
- independent offsets;
- the relative splay origin, its Reset button and the Ctrl swing
  ([ticket 06](06-origin-edits-hold-keys-still.md#answer));
- Column rotation, and how the Splay drag's "This column" choice looks
  ([ticket 05](05-this-column-splay-representation.md#answer));
- the matrix Anchor section, its canvas Pick, Follow X/Y/rotation, Detach, and Add anchored
  matrix ([ticket 07](07-matrix-anchors.md#answer)).

Cover the Inspector fields for Matrix, Row, Column and Key selections, and the toolbar tools and
drags, including a Bend gesture. Cover keyboard nudges and how a carried value
visibly moves the keys above it during preview. Existing tools stay: Stagger, Offset, Splay,
Origin and the Shift quick offset. React to the prototype with the user and record the chosen
layout and wording.

## Answer

Accepted by the user on 2026-10-23: **variant D**, the combination of A and C, with
separate Spread and Stagger handles.

1. **Toolbar:** keep A's parameter-specific tools for Matrix, Row, Column and Key
   selections. Splay retains its **This and following / This column** choice,
   writing Splay or Column rotation respectively.
2. **Inspector:** keep A's grouped **Carried** and **This column/row/key only**
   sections, with scope explanations. Column selections also show **Splay origin**;
   matrix selections show Placement and Anchor. Keep the established parameter names:
   Spread, Stagger, Padding, Shift, Bend, Offset, Column rotation and Key rotation.
3. **Canvas handles:** use C's direct manipulation alongside the toolbar, not instead
   of it. **Handles** is a view-only visibility toggle. Spread has a horizontal handle
   and Stagger a separate vertical handle; neither writes the other field. Column 1
   has no Spread handle. Alt disables snapping, rather than switching the handle's
   meaning. Explicit Origin and Splay tools still expose their required origin handle
   when passive handles are hidden.
4. **Placement chain:** include C's clickable chain under a collapsible **How it is
   placed** section. Earlier steps select their source; fields remain in the selected
   item's Inspector. This is the accepted Inspector presentation, not closure of
   [ticket 12's Relations and highlight question](12-relations-and-highlight.md).
5. **Feedback and defaults:** amber marks affected keys on field/handle hover and
   during drag; dashed outlines show pre-drag poses. Status names the changed fields,
   and each edit is one Undo entry. Default Spread and Padding display the pitch
   value in muted italics with a **pitch** badge.
6. **Origin and anchors:** retain Reset to column base, hold-still edits and Ctrl
   swing from ticket 06; retain Pick anchor key, Follow X/Y/Rotation, Detach and
   Add anchored matrix from ticket 07. The prototype does not replace the kernel,
   frame or snapping contracts already settled in tickets 04–08.

Primary source: local throwaway branch `prototype/ergogen-parity-inspector`, commit
`44c80fa984035cf0113e9d7922ccc0bbb7710959`, file
`web/crates/layout/prototype/ergogen-parity-inspector.prototype.html`, `?variant=D`.
That branch preserves all four variants; the plan branch records the decision only.
The prototype is not production code and must not be merged as the implementation.

Validation: browser-generated Chromium mouse checks passed for all 16 handle types
in C and D, including hover, drag, a single Undo entry and exact restoration. Spread
and Stagger were checked independently, plus view-only visibility and chain navigation.
Firefox was checked visually; touch and production integration were not checked.

## Comments

### 2026-10-23 — prototype feedback and combined variant

- User prefers A's parameter-specific toolbar and grouped Inspector together with C's
  canvas handles and placement chain, with a visibility toggle for the handles.
- Working variant **D** combines these: A's toolbar and sections, toggleable handles,
  and a collapsible **How it is placed** chain. A–C remain available for comparison.
- User requested separate Spread and Stagger control. Both C and D now have distinct
  horizontal Spread and vertical Stagger handles; Alt still disables snapping.
  Column 1 has no Spread handle.
- Fixed handle hover replacing its SVG target and passive hints intercepting clicks.
  Handles off also hides the passive origin handle; choosing Origin or Splay still
  exposes the origin needed by that explicit tool.
- Browser-generated Chromium mouse checks passed for all 16 handle types in both C
  and D: hover, drag, one Undo entry and exact restoration. Spread and Stagger write
  only their own field. Visibility and chain navigation create no Undo entries.
  Evidence: `.evidence/ergogen-layout-parity-09/handles.log` (gitignored).
- The user subsequently accepted the prototype. The final choice and archived source
  are recorded in the Answer above.
