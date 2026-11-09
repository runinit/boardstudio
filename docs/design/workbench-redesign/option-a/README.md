# Option A: sliding workbench toolbar

Status: user-approved visual/interaction reference; **not production UI or an implementation specification**.

Open [toolbar.html](toolbar.html) directly in a browser. It is a self-contained,
disposable presentation prototype with no network dependencies or project mutations.
The appearance toggle offers light and dark; the state selector exercises selection
and availability scenarios. The prototype drawing is illustrative, not Core geometry.

## Preserve this option

`toolbar.html` is the exact approved candidate, copied unchanged from the interactive
design study after the final whole-section motion refinement. Keep it as a frozen
reference. Create subsequent workbench studies in a separate sibling directory;
do not overwrite this file while exploring Objects, Inspector, or the topbar.
Approval applies to the toolbar direction below, not to the prototype's simplified
application shell, sample geometry, or any future implementation.

## Agreed direction

- One horizontal accordion strip, 42px high whether folded or expanded. Only one
  section opens at a time: Select, Transform, Align, Snap, or View.
- Selection scope choices are Matrix, Row, Column, Key, and Part. Changing scope
  does not open another action popup. Relevant tools occupy the open section;
  do not repeat them in a second permanent row or another canvas button rail.
- Show icon-and-text action labels when the actual available width permits.
  Prioritize action names before optional section captions. Narrow layouts retain
  accessible names and hover hints, with sideways scrolling if required.
- Measure label fitting against the final layout before the animation. Do not
  reshuffle labels during the slide.
- Icons and labels move together as one solid section under a stationary clip.
  No text fades, opacity reveals, per-label stagger, or delayed text pop-in.
  The captured candidate uses 280ms entry and 170ms exit, with reduced motion
  switching immediately. Rapid switching cancels stale movement.
- Detailed Position, alignment reference, and snap settings temporarily overlay
  the canvas rather than reserving another toolbar row or a permanent panel.
- Keep camera controls in the footer. Keep project actions, workspace navigation,
  history, and document creation outside this strip.
- Preserve equivalent structure and behavior in light and dark appearances.

## Existing control inventory represented

- Matrix: Move, Rotate, and Anchor when anchored. Rotate is unavailable when
  Follow rotation is off.
- Column: Stagger, Spread, Offset, Splay, Origin. First-column Spread is unavailable.
- Row: Offset, Shift, Bend. Key: Shift, Bend.
- Independent part: Move, Rotate, Anchor; anchored parts additionally offer Spread
  and Stagger, except Spread on first-column anchors. Locked parts cannot manipulate.
- Align: Left, Center X, Right, Top, Center Y, Bottom; reference part and Relationships.
- Snap: Off, 1/8u, 1/4u, 1/2u, 1u, 1mm, 0.5mm, 0.1mm; Geometry snap, Envelope gap,
  gap override.
- View: Keys, Components, Keycaps, Footprints, Board, Canvas handles.

The inventory was checked against `LayoutCommandPill`, `LayoutSelectionToolbar`,
`LayoutTransformTool`, `LayoutContextActionToolbar`, `layout_canvas::transform_tools`,
`LayoutAlignToolbar`, `LayoutSnapOptions`, and `part_rail_tools` in the worktree.
Recheck implementation availability when turning this option into production work.

## Verification and limitations

The design study was exercised at 1731px, 728px, and 390px widths, in light and dark,
including empty/locked selections, rotation and first-column guards, rapid section
switching, and reduced motion. A browser frame trace confirmed constant text opacity
and a constant icon-to-label offset throughout the final unified slide. The strip
remained 42px high. The archived file was compared byte-for-byte with that candidate.

These are prototype checks, not production performance, accessibility, CAD, export,
or fabrication evidence. The mechanical design scan retained a compact 11px text
warning; accessibility compliance has not been established. The surrounding shell
is intentionally simplified. No application code was changed to adopt this option.

## Next discussion, not yet approved

Explore a smaller Objects navigator, a contextual Inspector that belongs to the
same workbench design, and a less crowded topbar. Preserve this toolbar baseline
while those proposals are discussed separately.
