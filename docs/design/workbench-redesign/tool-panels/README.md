# Toolbar-led workbench template

Status: **user-approved redesign reference**, not production UI or a complete implementation specification.

Open [template.html](template.html) directly in a browser. It is the exact standalone
mockup the user liked and requested we save as a template. No build, installation,
server, network access or temporary source files are required. Use Settings or the
preview appearance button to compare light and dark.

## Use this reference

Keep this snapshot unchanged. Copy it into a separate study when exploring further
changes. Use its hierarchy, placement and interaction rules to guide the redesign;
do not treat its disposable JavaScript as production component architecture.
Production work still requires an explicit implementation brief, current domain
contracts and the repository's normal verification gates.

The earlier [Option A toolbar](../option-a/README.md) remains a separate frozen
reference. This template extends that direction into the surrounding workbench:
Inspect replaces the old Position shortcut, and Objects no longer occupies a dock.
The former pinned-versus-drawer comparison is not the selected direction.

## Direction to retain

- A canvas-first workbench, with no permanent Objects sidebar or object-icon rail.
- One 42px horizontal toolbar, folded or expanded. Select, Transform, Align, Snap
  and View retain the approved accordion behavior. Objects and Inspect belong in
  that same strip; no additional canvas Inspect button or permanent action row.
- Inspect opens an unpinned contextual panel below the toolbar, without a backdrop
  or canvas resize. No pinned alternative. Toggle, Close and Escape dismiss it;
  precise values survive closing and reopening.
- Objects opens a compact, toolbar-anchored searchable popover. Outline comes first.
  Matrix references explain where components are placed from; columns are available
  through an explicit disclosure, not a separate full-hierarchy pane. Selecting an
  object closes the popover without forcing Inspector open.
- One tool panel opens at a time. Switching panels preserves field drafts. Keep
  keyboard focus restoration and remove hidden panels from the focus order.
- Show icon-and-text action labels when actual width permits. Prioritize actions
  over section captions; measure the final layout before movement. Icons and text
  slide together under a stationary clip: no fades, opacity reveals or label stagger.
  Reduced motion switches instantly. Keep all toolbar entry points reachable when narrow.
- Keep project actions, six desktop workspace links, history, Settings and Export
  in the quieter topbar. Camera actions stay in the footer. Maintain both themes.

## Meaning and limits

The user's Objects tasks are selecting Outline and checking parts/components against
matrices. The sample groups components by their key-anchor references; this does not
settle whether "the right side of the matrix" means physical position or association.
A violet anchor line is not a clearance check or a claim of correct placement.

Geometry, field values, hierarchy and counts are illustrative. Edits and preview
Undo/Redo live only in memory. CAD evaluation, manipulation gestures, persistence,
project actions, workspace navigation and export are not implemented. Reconcile the
reference with current application behavior rather than assuming feature completeness.

## Evidence preserved with the choice

The candidate passed 52 interaction checks at 390px, 728px and 1440px, including
panel access, draft retention, keyboard focus, search, selection, availability guards,
themes and a bounded 42px toolbar. Additional checks covered 320px, open-popover
repositioning during resize, and zero running animations with reduced motion.

Light/dark accessibility scans reported no automated violations, but SVG background
contrast checks remained incomplete. This is not accessibility certification or
production CAD, export, performance or fabrication evidence. The archived HTML was
compared byte-for-byte with the shown candidate; saving it did not integrate the design
into application code.
