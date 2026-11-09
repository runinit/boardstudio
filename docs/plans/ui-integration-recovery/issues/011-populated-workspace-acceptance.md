# 011: Complete populated-workspace human acceptance

Status: ready-for-human
Type: acceptance
Blocked by: [010: Final technical verification](010-final-technical-verification.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Existing human-review contract](../../rust-ui-build/issues/026-merge-gate.md#human-workspace-review-instructions-still-required)

## What to build

Complete and record the user's review of the verified, populated application
across the agreed Themes, Appearances and widths. Tie final product acceptance
to the actual checked candidate rather than automated screenshots or a scoped
implementation-complete record.

## Acceptance criteria

- [ ] Identify the checked source and packaged artifact from 010 and the
  representative populated projects used for review. Review the actual
  candidate without replacing the user's existing live preview.
- [ ] Review Layout, Case, Parts, PCB, Keymap, Keycaps, Library, the project menu
  and Export in each combination of Original/shadcn Theme, Light/Dark Appearance
  and desktop/mid/narrow width. Use the agreed sizes 1440×900, 1024×900 and 390×844.
- [ ] Keep a per-Workspace/surface checklist with the project, Theme, Appearance,
  viewport, screenshot and result for every combination. Populate the checklist
  with observed results, not assumed passes or empty automated capture slots.
- [ ] Include real Case geometry and a populated 3D canvas. In Layout, inspect
  the recovered placement and scope workflows, split focus, anchor/reference
  feedback, amber previews, spacing ghosts, Snap and nested disclosures.
- [ ] Exercise representative editable fields, menus, dialogs and long Inspector
  content. Review contrast, geometry visibility, overflow/clipping, scrolling,
  keyboard focus, touch targets and selected/disabled states across the matrix.
- [ ] Record the user's explicit approval or reported defects. Automated captures
  support the review but cannot stand in for human acceptance.
- [ ] Keep unresolved defects and incomplete combinations open. If a correction
  changes the candidate, refresh the applicable 010 verification and affected
  human review before final sign-off.
- [ ] Reconcile the acceptance records with the reviewed candidate, actual scope
  and user decision (A8), preserving historical evidence. Do not change or close
  the parent specification or infer permission to push or merge.

## Verification and completion

Human review is the acceptance seam. Agents may prepare the checked candidate,
fixtures, checklist and captures, but this ticket is `ready-for-human` because
the final result requires the user's decision. Preliminary inspection may occur
earlier; final acceptance depends on 010's verified candidate.

## Comments

- 2026-11-06: Published as the final accepted gate. One checklist per Workspace
  keeps review manageable without creating a ticket for every visual combination.
- **Coordinator handoff 2026-11-08:** candidate `18d4d7f53` plus documentation-only
  changes thereafter; [010 is resolved](010-final-technical-verification.md#completion-record-coordinator-2026-11-08).
  Its record supplies the full candidate/build/browser identity and fresh full-gate
  and strict-CSS receipts. The restorations in cancelled 004–007 are **ABSENT by
  user cancellation** — do not score their absence as defects. Expect retained
  Relations tabs, duplicate scope popups, abrupt disclosures (restored motion
  unproven) and unrepaired chevrons; verify retained placement/split-focus,
  anchor/reference, amber-preview, spacing-ghost, Snap, save/Undo/reload and
  collision/provider behaviors across the unchanged acceptance matrix above.
  The MCU premise is withdrawn; Class B and worker/cache residual risks remain;
  firmware is not hardware-validated; autosave success is header-only. This is a
  handoff for human acceptance, not an automated pass or permission to replace
  the live preview. Status remains `ready-for-human`.
- 2026-11-08: Human review IN PROGRESS on candidate served from
  `web/target/site-011-candidate` (source `ad76a9bb7`,
  http://127.0.0.1:4173/boardstudio/). First defect reports from the user
  (preview annotations, screenshots in thread attachments
  `preview-annotation-annotation_1bn9b4r2uskpb_*`; combo not yet stated —
  screenshots appear Dark appearance, desktop width, to confirm). Project:
  created via new-keyboard flow (5x7 MX matrix, split linked halves, nice!nano
  MCUs visible in captures). Defects:
  - D1 Layout creation preview is covered by the left pane (assembly preview
    partially hidden behind the dock).
  - D2 No live preview of the matrix being created on the main canvas during the
    New matrix step (rows/columns sliders) — only the small in-panel grid.
  - D3 After matrix creation, the left half is covered by the left pane (canvas
    content under dock; fit/centering does not account for pane).
  - D4 Controller selection MCU previews render messy: MCU footprint tiny inside
    a large black square/circle that looks out of place.
  - D5 Off/mismatched background behind the Matrix/Row/Column/Key/Part scope pill
    group.
  - D6 Missing icons on the view-mode toggle buttons (render as black squares).
  - D7 Canvas-visibility/Snap rows: typography styling inconsistent with
    surrounding UI; rows need disclosure arrows showing they can be opened.
  - D8 Objects tree is missing columns and keys entirely for the halves (only
    Components/U1 shown) while keys render on canvas.
  - D9 (added same session, preview annotation `annotation_1n32ojf1qtqbx5_2`,
    element `g > [aria-label="Drag to stagger"] > rect`): the stagger drag-handle
    popover covers a large part of the canvas and renders terribly (oversized
    'rect' label + drag glyph box) — popover sizing/positioning/styling defect in
    Layout. Same-session thread images `image_a6ec83ab` and its Splay twin show
    both in-layout Stagger and Splay drag popovers ("Stagger/Splay 0.00 mm —
    Drag the handle, or type in the Inspector · Esc: close") rendering as a
    near board-wide solid black bar covering the top rows of keys, rather than a
    small floating chip near the handle. The reviewer observed that it appears
    sized in board/canvas units rather than screen pixels; the root cause is TBD.
    This observation applies to in-canvas dimension-edit popovers generally,
    not only stagger.
  - D10 (same session, thread image `image_afdc1b80`): Row/Column/Key/Part scope
    actions are duplicated — an inline strip (Offset, Stagger, …) and a "Row
    actions" popover (Row offsets, Offset, Stagger, Spread) present the same
    commands simultaneously (dual menu). Note for disposition: this overlaps
    the scope-duplication territory of cancelled 005; the user must decide
    whether this instance is an in-scope defect or the accepted cancellation.
  Status: review continues; no approval recorded; no fixes started (batch
  disposition after the pass; corrections will refresh 010 verification per AC7).
- 2026-11-08: User dispositions during review: D2 = transient ghost preview;
  D8 = keys-as-nodes, all parts visible in tree; D5 = DESIGN-literal layered strip;
  D4 = rounded box card; D10 reopens 005; user also REOPENS 004 and 006. Fix
  slices ordered: A fit (D1/D3), B+C icons/MCU (D6/D4), E dock disclosures (D7,
  fenced from 007), D strip fills (D5), F tree depth (D8), D9 dimension-popover
  sizing, G ghost (D2); then 004/005/006 scoped as their own work. Corrections
  refresh 010 gate + affected human re-review per AC7; 011 stays open (no approval
  yet).
