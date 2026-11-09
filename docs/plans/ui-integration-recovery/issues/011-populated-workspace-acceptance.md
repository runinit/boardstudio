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
- 2026-11-09, **Slice B+C (D6+D4) proof map — targeted implementation complete**:
  user approved toolbar/icon corrections and rounded-box controller cards; this
  is not final acceptance. Rendered DOM/CSS geometry is the agreed regression
  seam (no source-class-string oracle).
  - D6 — populated Layout → inspect 2D, 3D assembly and Footprints icons, plus
    both visibility-eye states → unfilled current-color strokes at normal icon
    size · packaged + mounted · RED: `.evidence/011-sliceBC/packaged-toolbar-red.log`
    and `packaged-eye-red.log`; GREEN: `packaged-toolbar-green.log`,
    `packaged-eye-green.log` and `mounted-green.log` in the same log directory.
    Before/after screenshots: `web/target/test-results/011-sliceBC/toolbar-before.png`,
    `toolbar-after.png` and `eyes-after.png` in the same screenshot directory.
  - D4 — Controller & wiring picker loaded → inspect compiled MCU footprint and
    card → footprint fills its 152×116px inner viewport; rounded rectangle,
    not capsule · packaged · RED: `.evidence/011-sliceBC/packaged-mcu-red.log`
    (16×16px footprint, pill radius); GREEN: `packaged-mcu-green.log` in the
    same log directory (both MCU previews 152×116px, card radius 14px).
    Before/after screenshots: `web/target/test-results/011-sliceBC/mcu-before.png`
    and `mcu-after.png` in the same screenshot directory.
  - Shared selector safety — Button/button-link and ToggleItem with direct
    default/explicit icons and nested geometry → defaults remain 16px, explicit
    icon sizing and nested viewport dimensions survive · mounted · RED:
    `.evidence/011-sliceBC/mounted-size-red.log`; GREEN: `mounted-green.log`
    in the same log directory. Regression tests:
    `action_icons_keep_default_and_explicit_sizes_without_resizing_nested_geometry`
    and `visibility_eyes_use_unfilled_current_color_strokes_in_both_states`.
    The first mounted attempt had a test-fixture type error (corrected); the next
    observed the eye-paint RED, whose panic poisoned later tests. The isolated
    size run independently observed the 16×16px nested-geometry RED.
  Source baseline `ea890dade`; fresh before package:
  `web/target/site-011-sliceBC-before`, port 4183; build log
  `.evidence/011-sliceBC/build-before.log`. Exact/fresh Gortex worktree route
  refused with `view_building` after six 60-second retry cycles (seven calls);
  native fallback disclosed. No D7/chevron/type or other slice changes made.
  Completion receipt: unchanged HEAD `ea890dade` plus this slice's uncommitted
  diff; fresh after package `web/target/site-011-sliceBC-after`, same isolated
  port 4183 (`build-after.log`). Browser initially retained the old service-worker
  cache; applying Update Now and waiting for the new SVG markup preceded final
  captures. Stale-cache attempts are labelled separately, not counted as GREEN.
  Original Light/Dark and shadcn Dark were visually inspected at 1440×900;
  supplemental `mcu-after-original-dark.png`, `mcu-after-shadcn-dark.png` and
  `toolbar-after-shadcn-dark.png` are in the screenshot directory above.
  Newly executed targeted checks: `check.py browser --only ui-shared parts`
  passed 93 ui-shared and 106 Parts tests; changed-file rustfmt, diff whitespace
  and Impeccable detector passed. Cargo/Dioxus builds and check.py runs used
  the shared FD9 lock.
  Consumer audit: direct SVG icons keep the 16px default; explicit sizes remain
  caller-owned; carousel arrows stay 20px. Nested Library previews have their
  own full-viewport sizing, empty-state icons 30px and Current mark 12px; shared
  icon components already supply size classes. Only Button/ToggleItem default
  size selectors were narrowed, not other controls or descendant paint rules.
  Source/browser audit logs: `selector-source-audit.log` and
  `selector-consumer-browser-audit.log` in the log directory above.
  Gortex later recovered exact/fresh worktree routing: source was verified
  against this checkout; detect/tests/guards/contract ran. It found no mapped
  test coverage or configured guards and warned about shared-control reach;
  mounted/rendered evidence above supplies behavioral proof. No signature changes.
  Remaining gates: coordinator full gate/build identity and affected user
  re-review under AC7. The complete acceptance matrix is NOT claimed;
  **011 remains ready-for-human, not resolved, and no approval is inferred**.
- 2026-11-09, **Independent combined B+C Spec + Standards review** — endpoints:
  `ea890dadeb925ece27c8b7a6884e75715d7536a4` → current uncommitted worktree,
  excluding unrelated external changes. Exact Gortex worktree
  `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378` returned disk-verified source and
  checkout-local paths, not main. All changed hunks read: `web/src/presentation/layout_workspace.rs`
  (toolbar), `web/crates/ui-shared/src/canvas_layers.rs` (eye),
  `web/crates/ui-shared/src/ui/button.rs`, `toggle_group.rs`,
  `primitives_mount_tests.rs` (all three under `web/crates/ui-shared/src/ui/`),
  `web/crates/parts/src/parts.rs` (picker), and this ticket's B+C record.
  Unread changed slice hunks: none. External AGENTS, standards and untracked
  documentation changes were not reviewed as part of this slice.
  - **Standards: CHANGES REQUIRED. Required — theme stroke contract:** the four
    restored glyphs hardcode `stroke-[1.5]` in `layout_workspace.rs:528,552,581`
    and `canvas_layers.rs:110`. `DESIGN.md` Colors/Shapes says the Theme supplies
    icon stroke (Original 1.5, shadcn 2), and
    `docs/plans/rust-ui-migration/theme-contract.md` section 1 names
    `--icon-stroke`; `web/tailwind.css:53,111` sets those values. These icons
    therefore stay 1.5 in shadcn instead of following the active Theme. Use the
    existing token for these strokes and verify computed width under both Themes;
    the eye test currently checks only positive width
    (`primitives_mount_tests.rs:133–141`). This does not require a global icon
    cleanup. No other correctness, readability, architecture, security,
    performance or Fowler-smell finding in the owned diff.
  - **Spec: PASS** for the requested B+C scope, conditional on the standards fix:
    the three view icons and both eye states gain visible unfilled current-color
    strokes, shared Button/ToggleItem sizing is narrowed to direct SVGs, and the
    controller card is a rounded box. No D7, scope strip, tree, dimension-popover
    or ghost work appears in the owned source diff. D4/D6 and the user disposition
    are recorded above; `DESIGN.md` Shapes specifies `rounded-xl` for cards.
  - **Acceptance for committing B+C: BLOCKED** pending the required theme-stroke
    correction and its targeted verification; this is not a demand to close 011.
    The coordinator owns the full gate/build identity and affected re-review;
    011 human acceptance remains blocked and `ready-for-human`, with no approval
    inferred.
  - Evidence reviewed: `verification.log`, mounted GREEN and independent size
    RED, packaged toolbar/MCU RED and GREEN, eye GREEN, source and browser
    selector audits, and Gortex receipt. They support the recorded 16→152×116
    geometry, 14px card, toolbar/eye paint, 93+106 targeted tests and 20px carousel
    arrows. The logs report eye RED and other captures; screenshots were not
    independently inspected by this reviewer, and no tests/builds/browser were
    run here. Full matrix and full gate remain pending; the fast package reused
    providers. Source audit and mounted fixtures reduce, but do not eliminate,
    shared-selector risk: wrapped bare icons that relied on inherited default
    sizing elsewhere may now need explicit size; verify affected consumers during
    coordinator's broader gate/re-review. No mutation test was run in this review.
- 2026-11-09, **Coordinator B+C correction and commit readiness** — baseline
  `ea890dade` → corrected working tree. The required review finding above is
  addressed in all four glyphs with `stroke_width: "var(--icon-stroke, 2)"`,
  matching shared icons. The existing eye regression now checks both Themes
  and both visibility states against DESIGN's 1.5/2 widths, not merely >0.
  Actual RED: `.evidence/011-sliceBC/coordinator-theme-red.log` reports shadcn
  1.5 vs required 2.0. GREEN: `coordinator-theme-green.log` in that directory,
  fresh 93 ui-shared + 106 Parts passes; changed-file rustfmt and whitespace pass.
  Independent sensitivity experiment using `scripts/mutation-check.py`:
  `coordinator-mutation.log` and `coordinator-mutation/` in that directory;
  unmutated filtered eye baseline passed, fixed-1.5 mutant killed by shadcn's
  assertion, source restored byte-for-byte (1 killed, 0 survived, 0 compile failures).
  No invalid-input admission guard was added by B+C; rejection rerun is not applicable.
  Fresh corrected fast package `web/target/site-011-sliceBC-after` rebuilt
  (`coordinator-theme-build.log`, providers reused). Agent-browser verified
  visible 2D/3D/Footprints glyphs in both Themes: no fill, current-color stroke,
  16px width, computed stroke 1.5px/2px (`coordinator-packaged-theme.log`).
  Screenshots `coordinator-toolbar-theme-original.png` and
  `coordinator-toolbar-theme-shadcn.png` under the B+C screenshot directory;
  coordinator inspected the latter and MCU after capture. Earlier B+C screenshots
  remain historical paint/geometry evidence; they precede the Theme-stroke correction.
  Full owned hunk inventory re-read through exact Gortex: this ticket additions,
  parts.rs:1251, canvas_layers.rs:110, button.rs:964, toggle_group.rs:60,
  primitives_mount_tests.rs:38–159, layout_workspace.rs:528/552/581.
  Unread changed slice hunks: none. Gortex detect/tests/guards/contract rerun:
  graph test mapping unavailable, no guards configured, shared-renderer reach warning
  reviewed against rendered evidence; no signature change or extraction needed.
  **Coordinator delta review: Standards PASS; Spec PASS; B+C commit READY**.
  This closes the small review correction, not 011 or its human acceptance matrix.
  Current HEAD before commit remains `ea890dade`; full gate/candidate promotion
  awaits remaining slices, followed by affected human re-review. Old candidate
  server was down and restarted unchanged on 4173 (PID 871065); isolated B+C
  browser/server stopped. Next action: commit B+C and implement fenced D7 Slice E.
