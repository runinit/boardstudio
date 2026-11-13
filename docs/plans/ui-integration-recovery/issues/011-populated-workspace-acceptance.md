# 011: Complete populated-workspace human acceptance

Status: resolved
Type: acceptance
Blocked by: [010: Final technical verification](010-final-technical-verification.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), Existing human-review contract

## What to build

Complete and record the user's review of the verified, populated application
across the agreed Themes, Appearances and widths. Tie final product acceptance
to the actual checked candidate rather than automated screenshots or a scoped
implementation-complete record.

## Acceptance criteria

- [x] Identify the checked source and packaged artifact from 010 and the
  representative populated projects used for review. Review the actual
  candidate without replacing the user's existing live preview.
- [x] Review Layout, Case, Parts, PCB, Keymap, Keycaps, Library, the project menu
  and Export in each combination of Original/shadcn Theme, Light/Dark Appearance
  and desktop/mid/narrow width. Use the agreed sizes 1440×900, 1024×900 and 390×844.
- [x] Keep a per-Workspace/surface checklist with the project, Theme, Appearance,
  viewport, screenshot and result for every combination. Populate the checklist
  with observed results, not assumed passes or empty automated capture slots.
- [x] Include real Case geometry and a populated 3D canvas. In Layout, inspect
  the recovered placement and scope workflows, split focus, anchor/reference
  feedback, amber previews, spacing ghosts, Snap and nested disclosures.
- [x] Exercise representative editable fields, menus, dialogs and long Inspector
  content. Review contrast, geometry visibility, overflow/clipping, scrolling,
  keyboard focus, touch targets and selected/disabled states across the matrix.
- [x] Record the user's explicit approval or reported defects. Automated captures
  support the review but cannot stand in for human acceptance.
- [x] Keep unresolved defects and incomplete combinations open. If a correction
  changes the candidate, refresh the applicable 010 verification and affected
  human review before final sign-off.
- [x] Reconcile the acceptance records with the reviewed candidate, actual scope
  and user decision (A8), preserving historical evidence. Do not change or close
  the parent specification or infer permission to push or merge.

## Verification and completion

Human review is the acceptance seam. Agents may prepare the checked candidate,
fixtures, checklist and captures, but this ticket is `awaiting-decision` because
the final result requires the user's decision. Preliminary inspection may occur
earlier; final acceptance depends on 010's verified candidate.

## Comments

- 2026-11-06: Published as the final accepted gate. One checklist per Workspace
  keeps review manageable without creating a ticket for every visual combination.
- **Coordinator transfer 2026-11-08:** candidate `18d4d7f53` plus documentation-only
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
  transfer for human acceptance, not an automated pass or permission to replace
  the live preview. Status remains `awaiting-decision`.
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
  and design-check detector passed. Cargo/Dioxus builds and check.py runs used
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
  **011 remains awaiting-decision, not resolved, and no approval is inferred**.
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
    011 human acceptance remains blocked and `awaiting-decision`, with no approval
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
- 2026-11-09, **Slice E (D7) proof map — targeted implementation complete**:
  B+C committed at `eca34d64`; this slice changes only the dock Canvas visibility
  and Snap triggers to decorative shared ChevronDown icons and 14px medium text.
  Floating-pill triggers, cancelled 007's other sites and reopened 006 motion
  are excluded. The Canvas utilities are scoped to the existing Layout trigger
  `m1-layers-trigger` inside `m1-objects-panel`, not every LayerPanel dock.
  - D7 dock affordance — populated Layout, dock rows open or closed → inspect
    Canvas visibility and Snap, activate disclosures → visible decorative
    ChevronDown, 14px/500 text and preserved expansion/ARIA · mounted + packaged
    · **RED → GREEN**. Reused packaged baseline:
    `.evidence/011-sliceE/packaged-before-shadcn-dark-red.log` (12px/400 Canvas,
    16px/400 Snap, no chevrons). Newly observed mounted RED: `mounted-red.log`
    in that directory (Canvas 12px and Snap 16px vs required 14px); subsequent
    WASM panic-hook failures/timeouts are follow-on poisoning, not extra defects.
    Final mounted GREEN: `mounted-green-scoped.log`; final packaged GREEN:
    `packaged-after-scoped.log` in the same log directory. Both Themes and both
    expansion states retain visible, aria-hidden chevrons with computed Theme
    stroke 1.5px/2px; packaged icons measure 16px. Trusted Enter on Canvas and
    Space on Snap open and close them; existing mounted Snap keyboard/pan-key
    and options-accessibility checks are reused, not duplicated.
  - Non-dock preservation — floating-pill LayerPanel → inspect and toggle →
    existing 12px/400, text-only compact presentation and expansion unchanged
    · mounted · **GREEN**, same scoped mounted log. The same scenario also
    protects other dock headers (PCB layers: 12px/400, no visible chevron);
    packaged PCB inspection confirms that exclusion. The floating variant's
    existing shell metrics, content, owners and Escape logic are unchanged.
  Newly executed commands used the shared FD9 lock and exported
  `BOARDSTUDIO_BUILD_LOCK_FD=9`: `python3 scripts/check.py browser --only ui-shared layout`
  (94 ui-shared + 268 Layout passed) and `python3 scripts/build-web.py --fast --skip-providers --output web/target/site-011-sliceE-after`
  (`build-after-scoped.log`, providers reused). Changed-file rustfmt, diff
  whitespace and design-check detector passed (`rustfmt-scoped.log`,
  `design-check-detect-scoped.log`). No full gate or mutation run is claimed here.
  Before captures reused unchanged from the source-equivalent B+C package
  `web/target/site-011-sliceBC-after`; final after captures use this slice's fresh
  package, Split 42, 1440×900, isolated port 4183. Screenshots:
  `web/target/test-results/011-sliceE/before-original-light-closed.png`,
  `before-original-light-open.png`, `before-shadcn-dark-closed.png`,
  `before-shadcn-dark-open.png`, and matching `after-*` names in that directory.
  Inspected captures show matching row typography and right-aligned disclosure
  chevrons in Original Light and shadcn Dark, open and closed. shadcn open was
  recaptured after the Theme menu finished closing. Fresh final selector markup
  was verified before capture; no service-worker cache replacement was needed.
  Browser errors were empty; own browsers/server stopped; old candidate
  4173/PID 871065 left running unchanged.
  The prior implementer's stale-HEAD blocker is historical: coordinator recovered
  exact Gortex mutation routing at `eca34d64`; all source reads/edits here used
  that exact checkout without fallback. Pre-impact and post detect/tests/guards/
  contract ran: zero graph-mapped tests is not behavioral absence; no guards are
  configured. The shared-renderer reach warning was checked against the scoped
  mounted and packaged preservation evidence; no signature/refactor change.
  Current HEAD remains `eca34d64` plus the owned uncommitted diff. Remaining:
  coordinator independent checks/review and full gate/candidate refresh, then
  affected human re-review under AC7. These captures are NOT full-matrix
  acceptance. **011 remains awaiting-decision, with no approval inferred.**
- 2026-11-09, **Slice E combined Spec + Standards review** — fixed endpoints
  `eca34d64de0f8a9064a845b6bd019f46b43c4f45` → working tree in checkout
  `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378`; HEAD and checkout root verified.
  Exact/fresh Gortex source reads covered every owned changed hunk: imports,
  LayerPanel trigger and mounted fixture/test in
  `web/crates/ui-shared/src/canvas_layers.rs`; import, extended mounted test and
  LayoutSnapDock trigger in `web/crates/layout/src/objects/layout_toolbar.rs`;
  and Slice E proof map/receipt in this ticket. Unread owned source hunks: none.
  External AGENTS, standards modifications and untracked files were not reviewed
  as slice changes; current standards and test economy were consulted as rules.
  - **Standards: PASS.** No required findings in correctness, readability,
    architecture, security, performance or the requested Fowler smell heuristics.
    **Optional maintenance note:** the shared LayerPanel now additionally relies
    on the Layout ancestor/trigger IDs for this presentation, extending an
    existing ancestor-selector coupling. A future owner/layout ID change should
    update the selector and its mounted regression together; the narrow scope is
    preferable here to altering other shared callers or widening the API.
  - **Spec: PASS.** The two requested dock triggers use the shared decorative
    ChevronDown and 14px medium text; their labels, disclosure owners, handlers
    and ARIA remain intact. The shared LayerPanel change is limited to the
    Layout ancestor plus trigger ID, with the floating pill and PCB presentation
    protected; no eye, 007-wide chevron or 006 motion change in the owned source.
    No required spec findings.
  - **Acceptance for committing E: READY**, not final 011 acceptance. Evidence
    inspected: `mounted-red.log` reports Canvas 12px and Snap 16px vs 14px,
    with follow-on panic poisoning; `mounted-green-scoped.log` reports 94
    ui-shared and 268 Layout passes; `build-after-scoped.log` reports successful
    fast package and `packaged-after-scoped.log` records Original Light and
    shadcn Dark open/closed computed 14px/500, decorative 16px icons with
    1.5/2px Theme strokes, trusted keyboard activation and unchanged PCB header.
    Packaged shadcn before RED also inspected. The eight named screenshots exist
    but were not independently viewed in this review. `verification.log` and
    `gortex.log` do not exist in the slice directory; the individual receipts
    above and exact/fresh Gortex reads were used instead. No test, mutation,
    build or browser was run by this reviewer. The fast build reused providers;
    the full gate, refreshed candidate, complete matrix and affected human
    re-review remain coordinator-owned. **011 remains awaiting-decision; human
    acceptance is blocked and no approval is inferred.**
- 2026-11-09, **Coordinator E commit receipt** — pre-commit HEAD `eca34d64`.
  Combined review above is Standards PASS / Spec PASS, commit READY. Coordinator
  re-read every owned changed hunk through exact Gortex (both source imports,
  LayerPanel trigger and full new mounted test; LayoutSnapDock trigger and all
  extended mounted assertions; this ticket's E additions); unread hunks: none.
  Newly executed independent mutation check: scoped regression baseline passed;
  removing the chevron's default hiding was killed by the floating-pill
  text-only assertion, not compilation (1 killed, 0 survived), and source was
  restored byte-for-byte. Logs: `.evidence/011-sliceE/coordinator-mutation.log`
  and `coordinator-mutation/` in that directory; spec lives outside the repo at
  `~/.cache/tmp/011-e-scope-mutation.json`. Changed-file rustfmt and whitespace
  passed. No invalid-input admission guard was added; rejection rerun N/A.
  Reused full targeted GREEN: 94 ui-shared + 268 Layout, plus fresh scoped
  package and trusted keyboard checks above. Coordinator inspected the shadcn
  Dark closed screenshot and packaged metrics; no new source correction needed.
  Remaining full gate, candidate refresh and human acceptance are not waived.
  Next action: commit E, then implement D5's layered scope-strip fill correction.
- 2026-11-09, **Slice D (D5) proof map — targeted implementation complete**:
  - Layered scope fills — mounted Layout scope choices → select a scope → group
    uses `muted`, active item uses `background`, with selected ARIA/state retained
    · mounted · **RED → GREEN**, extending the existing direct-selection/keyboard
    test. Logs: `.evidence/011-sliceD/mounted-red.log` (transparent vs muted;
    later panic-hook failures/timeouts are follow-on poisoning) and
    `mounted-green.log` in that directory (266 passed, 2 setup-guide tests filtered).
    Independent oracle: DESIGN's filled Toggle Group and active `background`
    rule, with known Original Light Theme values; no class-string assertions.
  - Theme paint and geometry — populated Split 42, 1440×900 → select Column and
    inspect both Themes/both Appearances → same semantic roles, unchanged control
    and group dimensions · packaged · **RED** in
    `.evidence/011-sliceD/before-{original,shadcn}-{light,dark}-red.log`.
    Before screenshots: `web/target/test-results/011-sliceD/before-*.png`.
    Reuses source-equivalent Slice E-after package; its scope override and rendered
    group/selected paint were verified. Existing scope selection, keyboard and
    retained-owner scenarios are reused, not duplicated at each Theme combination.
  Scope is CSS fill overrides only plus the focused test/proof: no 005/D10 menu
  deduplication, motion, chevron, tree, dimension or ghost work. Preserve group
  metrics, wrapping and contextual actions. **011 remains awaiting-decision.**
  Packaged **GREEN**: matching `after-{original,shadcn}-{light,dark}-green.log`
  files in the same log directory and `after-*.png` in the same screenshot
  directory. `packaged-comparison.log` confirms group/strip and all five control
  rectangles are identical before/after in all four combinations; inactive
  paint remains transparent and the active Column retains `aria-pressed=true`
  and `data-state=on`. The four before/after pairs were visually inspected:
  muted inset and background active fill now show, including the subtle shadcn
  Light pairing. Captures wait for transient menus/popovers to finish closing;
  initial in-transition captures were replaced, not treated as passing evidence.
  Diagnosis: unlayered group/item transparent backgrounds and the scope item's
  `selected` override defeated the shared layered Tailwind fills. Only these
  scope overrides were removed; non-scope action fills, dimensions, padding,
  wrapping, owners and production Rust are unchanged. Existing direct-selection
  and keyboard, retained-cell and scope-popover tests pass in the targeted run.
  Newly executed under the shared inherited FD9 build lock:
  `python3 scripts/check.py browser --only layout` (RED then GREEN), and
  `python3 scripts/build-web.py --fast --skip-providers --output web/target/site-011-sliceD-after`
  (`build-after.log`, unchanged providers reused). Changed-file rustfmt,
  diff whitespace and design-check detector passed (`rustfmt.log`,
  `design-check-detect.log`). Independent isolated browser sessions used port 4183;
  browser error logs are empty and own browsers/servers are stopped. Existing
  candidate port 4173/PID 871065 was not changed.
  Gortex exact source reads found the owner; initial pre-impact/mutation routing
  refused `view_building` after six 60-second retry cycles (seven impact calls)
  and a refused dry-run. The user-authorized native-edit fallback was disclosed
  and used for the initial test/CSS/proof changes. Automatic routing subsequently
  recovered at checkout HEAD `1cbc191a`, commit generation 6121/dirty 6123;
  exact/fresh impact and post detect/tests/guards/contract ran, and final ticket
  edits used Gortex. Zero graph-mapped tests is not behavioral absence; no guards
  are configured; CSS contract analysis allows the change. No signature changed.
  HEAD remains `1cbc191ac41eb0da3527f8a981a6b11b041c7e3e` plus these uncommitted
  slice changes. Coordinator still owns independent mutation/rejection checks,
  combined review, full gate, candidate refresh and commits; human re-review and
  acceptance remain outstanding. No full gate or mutation run claimed here.
- 2026-11-09, **Slice D combined Spec + Standards review** — fixed endpoints
  `1cbc191ac41eb0da3527f8a981a6b11b041c7e3e` → current uncommitted working
  tree in checkout `01a1b3cf-a8df-75da-ab9e-d3fe3b2d3378`; checkout root and
  HEAD verified. Exact/fresh Gortex reads covered every owned changed hunk:
  `web/assets/layout-toolbar.css` lines 14 and 26–29; mounted selection test
  in `web/crates/layout/src/objects/layout_toolbar.rs` lines 382–476; and the
  Slice D proof map in this ticket lines 363–416. Unread owned hunks: none.
  External AGENTS/standards diffs and untracked files were not reviewed as slice
  changes; current standards and test economy were consulted as rules.
  - **Standards: PASS.** No required findings across correctness, readability,
    architecture, security, performance or the requested Fowler smell heuristics.
    The extended scenario uses an independent DESIGN/known-Theme paint oracle;
    repo test-economy guidance favors this related shared setup.
  - **Spec: PASS.** No required findings. DESIGN calls for filled Toggle Groups
    and active `background` (DESIGN Shapes and Primary); deleting the unlayered
    scope overrides lets the shared `bg-muted` and active `bg-background` win.
    No production Rust, non-scope action styling, geometry, wrapping or owner
    changes in the owned hunks; excluded D10/005, motion, chevrons, tree, popover
    and ghost work is not included.
  - **Commit D: READY**, not final 011 acceptance. Inspected mounted RED identifies
    transparent group vs muted; subsequent panic-hook/timeouts are follow-on.
    GREEN log has **2 setup-guide passes in the separate filtered run plus 266
    other passes**, 268 total, matching E; the two were not omitted. Inspected
    all four packaged RED/GREEN paint summaries, comparison and build receipt:
    desktop Column retains state and geometry across both Themes/Appearances;
    fast package reused providers. The implementer reports inspecting all eight
    screenshots; this reviewer did not independently view them. No tests, build,
    mutation or browser run by this reviewer. Evidence does not establish the
    full viewport matrix, accessibility or human approval. Coordinator's
    independent checks, full gate, candidate refresh and affected human re-review
    remain outstanding; **011 remains awaiting-decision and human acceptance is
    blocked**, with no approval inferred.
- 2026-11-09, **Coordinator D commit receipt** — pre-commit HEAD `1cbc191a`.
  Standards PASS / Spec PASS above; every owned changed hunk independently read
  through exact Gortex (complete CSS, extended mounted selection scenario and
  all D ticket additions), none unread. Newly executed FD9-locked independent
  Layout baseline: 268 passes (2 setup-guide + 266 other); mutation-check then
  reinstated an unlayered transparent scope-item override and was killed by
  Matrix's selected-background assertion (transparent vs required Original Light
  background). No compile failure: 1 killed, 0 survived; source restored byte-for-byte.
  Logs: `.evidence/011-sliceD/coordinator-mutation.log` and
  `coordinator-mutation/` in that directory; spec outside repo at
  `~/.cache/tmp/011-d-fill-mutation.json`. Fresh post-restoration check rebuilt
  test CSS and passed all 268 again (`coordinator-restored-green.log`), ensuring
  generated assets do not retain the mutant. Whitespace passed; no invalid-input
  admission guard added, so rejection rerun N/A. Coordinator inspected Original
  Dark and shadcn Light after captures; all-four-combination packaged comparisons
  and unchanged geometry evidence above reused. No source correction required.
  Commit D READY; next is F's tree recovery. Full gate, candidate refresh and
  affected human re-review remain open; this does not approve or close 011.
- 2026-11-09, **Slice F (D8) proof map — targeted implementation complete**:
  - Loaded split depth — real Core CreateMirroredPair result reopened as accepted
    state → project defaults, then expand columns/keys → both halves expose
    selectable columns and keys, companions and every accepted board Part · native
    · **RED → GREEN**: `.evidence/011-sliceF/native-red.log` (missing column),
    `native-green.log` (all 38 UI-model tests). Independent oracle: user's
    keys-as-nodes/all-parts-visible decision and accepted Part membership, not
    copied projection logic or source strings. Fixture reuses synthetic-outline;
    final GREEN also pins Left/Right layout ownership on the default creation axis.
  - Setup Create owner — Objects mounted before actual Matrix Setup Create → create
    split, expand column/select key, collapse an existing layout, create another
    split → new branches open, old layout stays collapsed and setup retains its
    new-matrix selection · mounted · **RED → GREEN**. Logs in the same directory:
    `mounted-depth-red.log` (new split has no columns),
    `mounted-persistence-red.log` (old collapsed layout reopened after seeding all
    defaults), `mounted-green.log` (2 setup-guide + 267 other passes, 269 total).
    RED runs' later panic-hook/timeouts are follow-on poisoning. Existing
    tree-build/disclosure/grouping/commit guard is unchanged and passes.
  - Packaged split — source-equivalent `web/target/site-011-sliceD-after`, then
    fresh `web/target/site-011-sliceF-after` → create 5×6 split and load Split 42
    → columns appear by default, keys/companions remain expandable real nodes
    · packaged · **RED → GREEN**. Before/after screenshots under
    `web/target/test-results/011-sliceF/`: `created-{before,after}.png` and
    `populated-{before,after}.png`; all inspected. Qualitative tree comparisons,
    not a pixel/fit comparison (the later flow explicitly used Fit board).
    `key-and-companion-after.png`, `populated-key-parts-after.png` and
    `right-bridge-after.png` show actual selectable key/diode and MCU/bridge routes.
    `all-parts-reachability.log` in the log directory observes all **122 accepted
    Parts**, 60 primary Key rows + 62 component rows, none missing, after expansion.
    `loaded-split42-after.log` observes all four matrices' 18 default column rows
    with zero default key rows: the dock does not expand every key on load.
  **Bridge ownership confirmed, unchanged:** a read-only observer captured actual
  Core worker Scene replies for the created split with mirrored nice!nano MCUs
  moved to Y=140. `bridge-ownership.log` records Bridge 1's endpoints U1_M and
  right key SW113, `matrixIds=[matrix-1-right]`, and its right-layout tree occurrence;
  the reflected left span is a distinct Bridge 2 (U1 + SW53, `matrixIds=[matrix-1]`).
  This matches GLOSSARY's outline-bridge meaning and Core `bridge_metadata` in
  `core/src/outline_cleanup.rs`: membership follows endpoint Parts, including
  layout-owned independent components. `append_matrix_bridge_occurrences` follows
  that accepted membership, not a hard-coded right-half rule. Do not duplicate
  one span under the other physical half or renumber it as a product decision.
  The existing shared-context/distinct-occurrence bridge guard also passes.
  Production scope: seed visible board layouts' `half:{layout}` disclosures;
  setup Create seeds only new defaults within the same scope, preserving existing
  collapse choices. No projection/Part identities, ownership, selection, fit/dock,
  split-focus or linked-mirror owner was rewritten. Actual linked MCU movement
  and Key selection/Inspector context were observed. No D9, G, 004/005/006 work.
  Newly executed: locked `cargo test -p boardstudio-web-ui-model`; inherited-FD9
  `python3 scripts/check.py browser --only layout` (both REDs then GREEN); inherited-FD9
  `python3 scripts/build-web.py --fast --skip-providers --output web/target/site-011-sliceF-after`
  (`build-after.log`, unchanged providers reused); changed-file rustfmt and
  whitespace checks (`rustfmt.log`, `whitespace.log`). Package identity/cache
  observation: `package-identity.log`; fresh independent browser, Original Light,
  1440×900. Browser error logs are empty; own sessions and port-4183 servers stopped.
  Port 4173 was not acted on. Native first unqualified `--exact` attempt ran zero
  tests and is not RED/GREEN evidence; the qualified rerun above is actual RED.
  Gortex initially refused exact impact after six 60-second retry cycles (seven
  calls); disclosed authorized native fallback covered initial proof/tests and
  the initial default-seeding trial. Routing recovered at HEAD `19786498`, commit
  generation 6130; root/HEAD were verified,
  final production edits used exact/fresh Gortex. Post detect/tests/guards/contract
  ran: native tree tests mapped; mounted Objects is graph-unmapped, not absent;
  no guard rules configured. Contract analysis warns about broad existing Objects
  reach/parameter count; no signature or visibility changed, and no broad refactor
  was made to silence it. Final tree formatting is package-source-equivalent only.
  Coordinator still owns independent rejection/mutation checks, combined review,
  full gate, candidate refresh and commits. Human matrix/re-review and final
  acceptance remain outstanding. **011 remains awaiting-decision; no approval inferred.**
- 2026-11-09, **Slice F/D8 fresh combined Spec + Standards review** — endpoints
  `197864988116e20687269f395f7e3a8b049c7df8` → uncommitted worktree;
  root and HEAD verified. Exact worktree Gortex source reads cover all owned
  hunks: `web/crates/ui-model/src/tree.rs` 136–154, `tree_tests.rs` 15–177;
  `web/crates/layout/src/objects.rs` 242–255, `objects/tree_projection_tests.rs`
  import and 27–221; this ticket's F proof map 467–534. Unread owned hunks:
  none. Output truncation occurred in whole-file reads, but all listed changed
  portions were present in those outputs; other portions are not claimed reviewed.
  Protected AGENTS/standards changes and untracked files are outside F; current
  standards were consulted as rules, not reviewed as this slice's changes.
  - **Standards: PASS.** Required findings: none; optional findings: none.
    Reviewed correctness, readability, architecture, security, performance and
    all twelve Fowler heuristics as judgment calls. The native accepted-Core
    scenario and mounted real setup scenario cover independently failing owners,
    reuse existing fixture/runtime support and use observable membership/state
    oracles; no line-count or test-ratio objection. No new API or dependency.
  - **Spec: PASS.** Required findings: none; optional findings: none. Default
    disclosures match projected visible layouts; created defaults are differenced
    against previous defaults within scope, so the covered old collapse survives.
    The reopened Core test and actual mounted setup exercise distinct paths; columns
    remain collapsed until requested. Existing bridge projection keys occurrences
    from accepted matrix membership, not an invented half rule. Inspected
    `bridge-ownership.log` records U1_M/SW113 and right matrix for Bridge 1 and
    U1/SW53 and left matrix for distinct Bridge 2; this is specific observed
    geometry, not a universal Bridge 1 identity. `all-parts-reachability.log`
    records 122 accepted Parts, none missing, after expansion; this is evidence
    for that packaged fixture, not a proof for every possible accepted document.
    No projection identity, domain ownership, selection, fit, split-focus or mirror
    implementation was edited; excluded D9/G/004/005/006 changes are absent.
  - **Commit F: READY** on this scoped review, subject to coordinator's independent
    guardrail/mutation checks and commit procedure. Independently read native RED
    and GREEN (38), mounted GREEN summary (267 plus separate 2 setup-guide tests),
    packaged 122-Part/bridge and Split 42 summaries, and build receipt; other
    reported RED/GREEN and screenshots are reused from the implementer, not
    independently inspected in full. No tests, builds, mutations, browser or
    server were run in this review. Full gate and refreshed checked candidate,
    complete Theme/Appearance/viewport matrix, affected human re-review and
    explicit user decision remain outstanding: **final 011 acceptance BLOCKED**;
    status stays `awaiting-decision`, with no approval inferred.
- 2026-11-09, **Coordinator F commit receipt** — pre-commit HEAD `19786498`;
  Spec PASS / Standards PASS above. Independently read all five owned files'
  changed hunks via exact Gortex; none unread. Inspected populated-after and
  populated-key-parts-after screenshots; reused the documented all-122-Part and
  actual bridge ownership observations, 269-test Layout GREEN and package build.
  Newly executed locked mutation baseline passed 38 UI-model tests plus both
  mounted tree-projection scenarios. Independently removed effective loaded-half
  disclosure seeding (killed by missing-column assertion) and replaced new-default
  differencing with all-default seeding (killed by old-collapse preservation
  assertion). `scripts/mutation-check.py`: **2 killed, 0 survived, 0 compile
  failures**, source restored byte-for-byte. Spec: outside repo at
  `~/.cache/tmp/011-f-disclosure-mutations.json`; logs:
  `.evidence/011-sliceF/coordinator-mutation.log` and `coordinator-mutation/`.
  No invalid-input admission guard added (rejection N/A); the independently
  failing persistence invariant was exercised. Whitespace/rustfmt passed.
  No source correction required; commit F READY. Next: all-dimension popover D9.
  Full gate/candidate refresh/matrix/human re-review remain open; 011 is not closed.
- 2026-11-09, **D9 proof map — targeted implementation complete** — approved
  scope is every in-canvas dimension-edit chip, not a Stagger-only correction.
  - Shared size/paint — accepted matrix tools → hover Stagger, Spread, Offset,
    Splay, Shift, Bend, Move and Rotate; Stagger additionally at 2/4/8 screen
    px per world unit → compact chip (100–280px wide, 40–110px high), contained
    readable text, full hint and Theme icon paint · mounted + packaged · RED:
    `.evidence/011-D9/mounted-red.log`; GREEN: `mounted-green.log` and
    `layout-browser-green.log` in that directory. The latter includes the
    added Theme-stroke assertion. Test:
    `dimension_popups_stay_compact_above_handles_at_screen_size`.
  - Anchor/hit target — hover a dimension handle → chip entirely above handle,
    4–14px clear gap; original 20×24px drag target and pointer-transparent chip
    unchanged · mounted + packaged · same mounted RED/GREEN; packaged RED:
    `packaged-before-geometry.log` in `.evidence/011-D9/` (344.86px width and
    −60.1px gap). `packaged-after-*.log` records 265.69×81.8px with a 7px gap
    at fit and zoomed out, in Original Light/Dark and shadcn Dark. Actual
    text contrast minima: 5.87, 7.65 and 6.94 respectively (`contrast-*.log`).
  - Interaction — newly reran the unchanged accepted-state popup-follow,
    live-delta, one-commit/expiry test and existing capture/cancel/Esc/Undo,
    independent-part and Inspector-draft guards in the 270-test Layout subset
    (2 setup-guide + 268 other tests), all GREEN: `layout-browser-green.log`.
    No edit ownership, units, hit-target transform or viewport code changed.
  Before package reused: `web/target/site-011-sliceF-after`, source-equivalent
  to `b8ae4a88`. Actual current render is already correctly inverse-scaled and
  Theme-colored, not the earlier black board-wide bar. It still covers the
  rail/keys: the y-down panel's **top**, not its bottom, anchors 8px above the
  rail; its unbroken footer also overallocates width. This observed current
  defect, rather than the unconfirmed board-unit hypothesis, governs the fix.
  Shared `feedback_popup_panel` now anchors using the rendered panel's height
  and retains the full instruction on two lines. Existing Theme fill/stroke
  colors remain; measurement font and glyph stroke now consume Theme tokens.
  Producer audit: `LayoutTransformToolOverlay` supplies matrix/column/row/key
  dimension chips; `LayoutPartToolOverlay` supplies independent-part
  Move/Rotate/Spread/Stagger chips; `LayoutAnchorPickOverlay` hover uses the
  same renderer. Armed Anchor pick stays top-right using its separate panel
  (only the shared measurement font follows the Theme). Origin's point has
  no hover chip. `LayoutCanvasControls` is an HTML measurement grid currently
  mounted only in tests, not another shipped popup; `OutlineDimension` is an
  Inspector field, not transformed canvas chrome. No producer was special-cased.
  Fresh AFTER package: `web/target/site-011-D9-after`, fast build with unchanged
  providers reused (`build-after.log`), isolated port 4183. Applied Update Now
  before final captures; two footer rows and new glyph paint confirm fresh
  markup. Screenshots in `web/target/test-results/011-D9/`:
  `stagger-before.png` / `stagger-after.png`, `splay-before.png` /
  `splay-after.png` (Original Light); paired
  `stagger-{before,after}-original-dark-zoom-out.png` and
  `splay-{before,after}-shadcn-dark-zoom-out.png`; supplemental
  `stagger-after-shadcn-dark-fit.png`, `splay-after-shadcn-dark-fit.png`.
  Full-frame captures show the chip just above its handle, no longer over the
  rail/top key row; other geometry can still lie behind this small floating chip.
  Newly executed: locked `check.py browser --only layout` (including CSS build),
  targeted WASM Clippy `--lib --tests -D warnings`, rustfmt, whitespace and doc
  links passed. `scripts/mutation-check.py` killed all three mutants (bad
  vertical anchor, board-unit panel, fixed icon stroke), zero survivors or
  compile failures; source restored byte-for-byte. Spec:
  `~/.cache/tmp/011-D9-mutations.json`; logs: `.evidence/011-D9/mutation.log`
  and `mutations/`. No input admission/rejection guard was added (N/A).
  Gortex exact worktree reads/edits, impact, detect/tests/guards/contract ran;
  initial change routing recovered after six 60-second retry cycles, with no
  native source fallback. No mapped coverage/configured graph guards; actual
  mounted tests above supply behavioral proof. Contract warns about shared
  caller reach, not a broken signature. design-check reports only pre-existing
  shadow-color advisories (`design-check.log`); no new CSS file or tokens needed.
  HEAD remains `b8ae4a88` plus this uncommitted two-file slice. Coordinator owns
  independent mutation/rejection reruns, combined Spec/Standards review, full
  gate and candidate refresh. Historical black-bar cause was not reproduced
  on the current baseline; no claim about its original cause or the complete
  Theme/Appearance/viewport matrix is made. Affected human re-review and explicit
  acceptance remain outstanding: **011 stays awaiting-decision, not resolved**.
- 2026-11-09, **D9 fresh combined Spec + Standards review** (user-authorized
  single reviewer): fixed endpoints `b8ae4a88b3670fed31f30ec467341e2232aeebfb`
  to its uncommitted worktree on `fix/ui-integration-recovery`; root and HEAD
  verified. D9 owned diff files/hunks read via exact worktree source: this ticket
  591–659; `web/crates/layout/src/objects/layout_transform_toolbar.rs` 296–319,
  369–394, 428–432 and 10303–10465. No owned changed hunk unread. Protected
  external changes excluded. Commit-ref baseline source view was building;
  current exact worktree source and mechanical hunk inventory were available.
  - **Standards: PASS**, bounded to D9. No required or optional findings.
    Reviewed correctness, readability, architecture, security and performance;
    the existing large file is a maintenance concern, but this focused shared
    renderer change does not justify an unrelated decomposition here. Considered
    all 12 Fowler heuristics (mysterious names, duplication, feature envy, data
    clumps, primitive obsession, repeated switches, shotgun surgery, divergent
    change, speculative generality, message chains, middle man, refused bequest):
    no actionable new smell. Test economy: one shared mounted scenario exercises
    equivalent tools and representative zooms with rendered geometry oracles.
  - **Spec: PASS**, for the bounded current-baseline correction, not historical
    black-bar resolution. No required or optional code findings. Shared y-down
    panel now accounts for its height over the rail; height and width remain
    screen-pixel-scaled, hint retained, hit geometry unchanged. Common renderer
    reaches dimension and Anchor-hover producers; independent-part wiring and
    interaction preservation rely on existing guards and producer audit, not a
    new packaged matrix of every owner. Toast is independently top-centred; no
    toast positioning change is required by this hover-anchor correction.
  - Independently inspected existing mounted RED/GREEN logs, mutation summary,
    packaged before and Original Light after geometry, shadcn Dark contrast,
    Layout browser log boundaries, and Stagger light / Splay dark before-after
    screenshots. RED demonstrates current 344.86px chip and −60.1px gap, not the
    historical black bar; after log gives 265.69×81.8px and 7px gap. Reused
    reports for other Theme/zoom cases, complete 270-test result, source restore,
    packaging provenance and remaining checks; no commands were rerun here.
    Historical black-bar cause/reproduction remains unresolved; no causal claim.
  - **D9 commit readiness: BLOCKED** pending coordinator-owned independent
    mutation/fail-path reruns and final scope/commit checks; no new input
    rejection guard was introduced. **011 acceptance: BLOCKED** pending full
    gate and refreshed candidate, complete required Theme/Appearance/viewport
    matrix, affected human re-review and explicit approval, plus remaining
    ordered slices. This review does not authorize closing 011.
- 2026-11-09, **Coordinator D9 commit receipt** — pre-commit HEAD `b8ae4a88`.
  All owned changed source/test/ticket hunks independently read through exact
  Gortex, none unread; no source correction required. Spec/Standards PASS above
  remain bounded to the observed current-baseline chip defect, not proof of the
  historical black bar's cause. Inspected Stagger Light and Splay shadcn Dark
  zoom-out after captures; reused other packaged comparisons/producer audit.
  Newly executed independent locked mutation-check baseline passed; all three
  existing mutants independently killed: top-vs-bottom anchor (negative gap),
  board-unit panel (screen-size text), fixed icon stroke (shadcn token mismatch).
  **3 killed, 0 survived, 0 compile failures**, source restored byte-for-byte.
  Spec reused from `~/.cache/tmp/011-D9-mutations.json`; new logs:
  `.evidence/011-D9/coordinator-mutation.log` and `coordinator-mutation/`.
  Fresh restored `check.py browser --only layout` rebuilt test CSS and passed
  **270 tests** (2 setup-guide + 268 other), including interaction guards:
  `coordinator-restored-green.log` in the same directory. Rustfmt/whitespace
  passed; no invalid-input admission guard added (rejection N/A).
  Independent rerun/final scope prerequisites now met: **D9 commit READY**.
  Next G's transient matrix ghost. Historical reproduction limitation, full
  gate/refreshed candidate/required matrix/human re-review remain open; no 011
  acceptance inferred.
- 2026-11-09, **Slice G (D2) — approved presentation seam implemented;
  focused proof GREEN**, uncommitted over `11a3513d`:
  The user approved only the narrow public presentation provider/outlet between
  private Matrix Setup sliders and the page canvas, with private transient paint
  storage. This resolved the previous blocker, not other visibility/API changes.
  - Live draft — actual New matrix form open over the real accepted Editor →
    rows/columns keyboard input before Create → candidate key footprints update
    on the main canvas; accepted document/scene/revision, runtime preview,
    camera and selection remain unchanged · mounted · **GREEN**:
    `.evidence/011-sliceG/mounted-lifecycle.log`, test
    `new_matrix_sliders_preview_on_canvas_without_changing_accepted_state`.
    Original **RED** retained in `.evidence/011-sliceG/mounted-red.log`: the
    real Add menu/sliders mounted but 30 main-canvas footprints were absent.
    Its three panic-poisoned followers were not independent REDs. The regression
    now also checks visible separation from the form and each dimension's extent.
  - Lifetime and writes — Cancel, stage exit and actual Create → no stale
    candidate; Cancel/stage exit write nothing, Create keeps its normal single
    accepted commit and selection behavior · mounted + packaged · **GREEN**.
    The same Editor scenario checks complete ReadModel equality during input,
    Cancel and workspace exit, one revision/Create, selected created matrix and
    one Undo restoring prior matrices/Parts. Its host now subscribes to runtime
    delivery (previous hover-only cases manually advanced the version signal).
    `.evidence/011-sliceG/layout-mounted.log` extends the existing real Create
    fixture: form-only unmount clears paint while the controller/outlet remain
    live; remount starts at defaults; pending Create clears paint.
  - Separate wiring — empty new-keyboard Layout step versus Add matrix on an
    accepted board → both show the live footprint only during their real setup
    lifetime, preserving existing board visibility and linked-half semantics ·
    mounted + packaged · **GREEN**. Packaged before evidence independently
    reproduces the empty split flow: trusted slider keys change 5×6 to 6×7;
    the panel shows 42 keys while the main canvas has zero key rectangles.
    Logs: `.evidence/011-sliceG/packaged-before.log`, `packaged-red.log`;
    full-frame captures: `web/target/test-results/011-sliceG/maincanvas-before.png`
    and `maincanvas-before-6x7.png` in that directory (Original Light, 1440×900).
  Fresh after package: `web/target/site-011-sliceG-after` (fast, skip-providers;
  existing provider artifacts reused, not rebuilt), isolated port 4183. Logs:
  `.evidence/011-sliceG/packaged-after.log`, `packaged-live.log`. Final captures
  under `web/target/test-results/011-sliceG/`: `empty-after-final-{5x6,6x7,created}.png`
  (Original Light) and `add-after-visible-{5x6,6x7,created}.png` (shadcn Dark),
  full-frame 1440×900. The final rebuild/adoption is also captured in
  `final-package-smoke-6x7.png` with visible, pointer-transparent paint and clean
  Cancel. Trusted inputs change both dimensions live; Cancel and
  workspace exit clear the ghost; Create leaves accepted linked halves with no
  ghost. Add-existing keeps accepted key markup and canvas viewBox unchanged.
  A stale worker was observed and adopted via trusted Update Now; old full-width
  form captures are not final GREEN. The existing form is now narrow, with its
  measured area reserved only for the schematic; accepted fit/camera are unchanged.
  The only new public interface is `use_matrix_setup_preview_provider` and
  `MatrixSetupCanvasPreview` using the existing setup mount. Dimensions are copied
  read-only; private paint cleanup is scoped to its mounted publisher. Core's
  read-only matrix projection and existing footprint adapter supply geometry.
  No runtime geometry requests, draft registry/owner, accepted scene or history
  writes were added. Existing input admission guards are unchanged.
  Focused Layout native/mounted, WASM Clippy, rustfmt, whitespace and doc links
  are recorded in `.evidence/011-sliceG/`; no full gate or mutation run is claimed.
  All source reads/edits used the exact Gortex checkout; no native source fallback.
  Coordinator full gate, independent rejections/mutations, fresh combined review,
  candidate refresh and human acceptance remain outstanding; **011 remains
  awaiting-decision**, not accepted.
- 2026-11-09, **Slice G (D2) fresh combined Spec + Standards review**
  (one reviewer per explicit user instruction): fixed `11a3513dfebf0b23c8ee5a1e4deb25b29fd684a2`
  to the uncommitted worktree on `fix/ui-integration-recovery`; root and HEAD
  verified. Exact Gortex worktree source read: new
  `web/crates/layout/src/objects/matrix_setup_preview.rs` (entire file),
  `web/crates/layout/src/objects.rs` 51, 89;
  `web/crates/layout/src/objects/matrix_setup.rs` 120–124, 258, 317–452;
  `web/src/presentation.rs` 3267, 8744–8773, 9788–9800;
  `web/src/presentation/layout_field_preview_tests.rs` 28, 179–198, 230–238,
  254, 325–513; this ticket 719–777. **No owned changed hunk unread.**
  Protected external work was not reviewed as part of the diff.
  - **Standards: PASS**, bounded to this slice; no required or optional
    findings. Correctness, readability, architecture, security and performance
    reviewed. The provider exposes only the approved presentation seam; paint and
    source identity stay private. Dimensions are bounded by existing input
    parsing; projection uses Core and the existing footprint adapter. The long
    Editor test uses the real owner and existing fixture to check distinct live,
    cleanup, Create and Undo promises; its cost is justified, not an assertion
    count rule. Considered the twelve baseline smells as judgment calls, not
    automatic violations: mysterious names, duplication, feature envy, data
    clumps, primitive obsession, repeated switches, shotgun surgery, divergent
    change, speculative generality, message chains, middle man and refused
    bequest; no actionable new smell found.
  - **Spec: PARTIAL**. Required verification: `web/src/presentation.rs:9789–9798`
    clamps the measured form inset to 80% of the canvas, while
    `web/crates/layout/src/objects/matrix_setup.rs:258` allows a form nearly as
    wide as its containing area. When the measured form extends beyond that
    clamp, the computed preview region includes covered space; the 1440px
    separation assertion does not establish visibility at narrow widths. D2
    requires a live main-canvas ghost, and the approved change must be dock-safe.
    Independently exercise the real narrow Editor and setup-guide layouts,
    including 390×844 and changed dock/form geometry; if obscured, reserve the
    actual unobscured region or adapt the form without changing camera/fit, and
    add a faithful regression. This is an unverified risk, not a claim that a
    particular 390px screenshot failed. Otherwise the reviewed publisher reacts
    to local sliders, filters by editable current owner and clears on publisher
    drop; the mounted Editor scenario checks no accepted-state writes, normal
    Create/selection and one Undo. No extra runtime geometry or draft ownership
    changes are in this slice. No conclusion about historical D9 black bars.
  - **Evidence audited, not rerun:** `.evidence/011-sliceG/mounted-red.log`
    shows the missing 30-key ghost first; its three followers are panic-poisoned,
    not independent REDs. `mounted-lifecycle.log` reports 5 passing, and
    `layout-mounted.log` 6 passing, including pending Create and form-only drop.
    `packaged-live.log` records 1440×900 trusted-slider geometry, unchanged
    viewBox and pointer-events none; provider artifacts were reused. Other
    producer evidence and screenshots were not independently inspected here.
    No tests, builds, mutations, browser or server were executed by this reviewer.
  - **Slice G commit readiness: BLOCKED** pending the narrow-visibility proof
    above and coordinator's independent disconnected/frozen publisher, drop,
    editable-filter and form-width mutation/fail-path reruns and scope checks.
    **011 acceptance: BLOCKED** pending full gate, refreshed candidate, required
    Theme/Appearance/viewport matrix, affected human re-review and explicit
    approval. 011 remains `awaiting-decision`, parent 001 open; no acceptance or
    permission to merge is inferred.
- 2026-11-09, **Coordinator G follow-up and commit receipt**, pre-commit HEAD
  `11a3513d`. All seven owned files' source/test/doc changed hunks read through
  exact Gortex, including the new module and added `web/assets/shell.css`; none
  unread. The review's narrow-screen risk was independently reproduced at
  **390×844**: the full guide hid the ghost, then Add-existing's compact Sheet
  and stationary controls covered it. Bounded correction reserves the measured
  whole guide/form and view-control region, observes their size changes, and
  paints beside or below them without changing camera/fit or accepted geometry.
  Narrow authoring surfaces scroll within part of the canvas; the compact Sheet
  is temporarily hidden, keeping its owners mounted and preference unchanged.
  Other setup stages and desktop dock ownership remain unchanged.
  - Proof row — real Editor/guide → narrow canvas, resize, two subsequent
    guide-only geometry changes and floating-form transfer → visible footprint
    outside measured chrome with complete ReadModel equality · mounted ·
    **RED→GREEN**: `coordinator-narrow-red.log`, `coordinator-final-green.log`
    under `.evidence/011-sliceG/`. The fixture collapses/restores real desktop
    docks before narrowing its container; a zero-width pinned-dock fixture was
    diagnosed, not used as product evidence. Restoring live dock state and saved
    preferences avoids leaking that scenario into keyboard-focus siblings.
  - Newly executed independent mutation baselines passed. Disconnected and frozen
    publisher, missing painter cleanup, pending-Create editable-filter removal,
    full-width form and full-height narrow guide were killed. A missing guide
    observer initially survived; a second settled resize strengthened the same
    scenario. Final observer rerun killed it: **7 distinct targets killed,
    0 outstanding survivors, 0 compile failures**, source restored. Logs:
    `coordinator-{web,layout}-mutation.log` plus their directories and
    `coordinator-final-observer-mutation.log`/directory in the same evidence area;
    mutation specifications under `~/.cache/tmp/011-g-*.json`.
  - Newly executed restored checks: **6 field/spacing tests GREEN**, fresh fast
    package in `coordinator-final-green.log`; **270 Layout tests GREEN** in
    `coordinator-restored-green.log`. Native 168-pass/1-ignored and producer
    targeted Clippy evidence above reused; full gate still next. Rustfmt,
    whitespace and design-check detector passed. No new input-admission guard;
    pending paint rejection and form-only cleanup independently rerun.
  - Packaged trusted-input proof at 390×844, Original Light and shadcn Dark,
    plus 1440×900 resized docks: `coordinator-packaged-narrow.log`. A correctly
    hit slider track updated the ghost while the mouse remained held before
    pointerup. Final package clears both form and stationary controls; final
    capture is **7×6**, `coordinator-narrow-final-clear-controls-7x6.png`, under
    `web/target/test-results/011-sliceG/`; earlier 6×7 and guide/Create/Cancel
    captures are there too. Cancel restores the compact Sheet. Stale cached CSS
    was detected and excluded; final proof used a clean isolated browser.
    Chromium viewport/keyboard/mouse evidence, not Safari or physical-touch proof.
  Coordinator delta review: **Standards PASS; Spec PASS for G**, resolving the
  required visibility verification above; the original reviewer report remains
  intact. **G scoped commit READY**. **011 acceptance BLOCKED** on the full gate,
  refreshed candidate, complete required matrix, affected human re-review and
  explicit approval. Parent 001 open; D9 historical black-bar cause still unproven.
- 2026-11-09, **Refreshed human-review transfer**: all implementation slices are
  committed through `2694b18913821b08c5b79da8414ef13f29c26863`.
  [010's refreshed record](010-final-technical-verification.md#candidate-refresh-coordinator-2026-11-09)
  supplies the new full-default PASS (657.7s), bound 17-check drag/reload receipt,
  strict old-CSS 0/0 and separate optimized root/subpath build/smoke identities.
  Candidate `web/target/site-011-candidate` is now served at
  http://127.0.0.1:4173/boardstudio/; the previous artifact is retained as
  `web/target/site-011-candidate-pre-2694b189`. No user browser/storage was touched.
  A cold promoted-page check verified the new script and narrow matrix CSS.
  **Scoped implementation complete; human acceptance BLOCKED pending actual
  affected re-review, status of observations below and explicit approval.**
  The checklist below is prepared evidence, not a user decision. All acceptance
  checkboxes and `awaiting-decision` remain unchanged; parent 001 stays open.

## Candidate review matrix — 2694b189

**Project:** `011 recovery review`, created through the public new-project/setup
flow: 5×7 linked split, 70 accepted keys, compiled MX switch/diode footprints and
four mounting holes (144 placed parts). The controller placement was not
completed; wiring, keymap and keycap assignments remain incomplete. Export's
controller/wiring blockers are expected fixture state, not a fabrication or
firmware-success claim. The public portable-project download is
`web/target/test-results/011-final-candidate/011-recovery-review.boardstudio`
for import into another browser. The initial Split 42 demo lacks a required
switch courtyard and was excluded from Case proof. The valid review project's
battery and cable were moved inside its left assembly via actual Inspector
fields (X −86 and −69 respectively), after which Case reported exact geometry
ready with enabled geometry export and populated 3D plate/foam/battery/tray/PCB.
This fixture correction is not a claimed product repair or hardware validation.

**Evidence/result key:** every cell marked **O / pending** has an actual capture
and correct observed Theme/Appearance/viewport, visually inspected by the
coordinator in its labelled contact sheet. It means **observed / human decision
pending**, never PASS or approval. All 108 cells were populated, with no horizontal
page overflow; no empty capture slot or assumed pass is counted. The complete
per-cell project, screenshot and observed DOM record is
`.evidence/011-full-gate/matrix/observations.json`; snapshots/command logs are in
that directory. Capture driver outcome: `matrix-capture.log` one directory up.

**Screenshot for each cell:** under `web/target/test-results/011-final-candidate/`,
use `<column-prefix>-<viewport>-<section-slug>.png`, where column prefixes are
`original-light`, `original-dark`, `shadcn-light`, `shadcn-dark`, viewport uses
ASCII `x`, and the section slug is stated below. E.g.
`original-light-1440x900-layout.png`. All 108 named images exist. Each combo's
nine images also appear in `contact-<theme>-<appearance>-<width>.png`.

### Layout (`layout`)

Observed: both populated halves inside the canvas, nested half/column tree,
visible view-mode icons and scope-role fills. Reopened 005's duplicate actions
remain deferred. Dense labels/contrast/interaction fit still require the user's
judgment, not a geometry-only automatic pass.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Case (`case`)

Observed: exact geometry ready, populated interactive 3D assembly, construction
and long Inspector content; compact Inspector Sheet visible on phone. **R1**
below is not waived by the valid geometry.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Parts (`parts`)

Observed: controller-library results, actual nice!nano footprint viewport and
component Inspector. This is an unapplied generator preview, not an accepted
controller/wiring assignment or a manufacturing-ready footprint claim.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### PCB (`pcb`)

Observed: populated split footprint scene, Objects and routing Inspector;
controller/wiring errors are visible for this deliberately incomplete fixture.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Keymap (`keymap`)

Observed: base layer, 70-key scene, assignment controls and unassigned-key list;
no configured or hardware-validated firmware is claimed.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Keycaps (`keycaps`)

Observed: 70-key scene, profile/legend/color controls, selection Inspector and
0/70 assigned state; no finished keycap assignment is claimed.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Library (`library`)

Observed: populated project library with two saved projects; phone's saved cards
are below the introductory actions. Additional actual scroll capture:
`shadcn-dark-390x844-library-saved-projects.png`.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Project menu (`project-menu`)

Observed: New/Open/Save-copy/Browse/Setup/Undo/Redo/Export/settings menu remains
within each viewport; menu commands used for fixture setup and navigation.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Export (`export`)

Observed: current project/revision/local-save state, portable download and
readiness-gated design files. Portable-project download succeeded; final KiCad
and firmware remain controller/wiring-blocked. Additional actual long-page
scroll capture: `shadcn-dark-390x844-export-scrolled.png`.

| Viewport | Original Light | Original Dark | shadcn Light | shadcn Dark |
| --- | --- | --- | --- | --- |
| 1440x900 | O / pending | O / pending | O / pending | O / pending |
| 1024x900 | O / pending | O / pending | O / pending | O / pending |
| 390x844 | O / pending | O / pending | O / pending | O / pending |

### Supplemental affected checks and open observations

- Final optimized G check: actual Columns keyboard input at 390×844 shadcn Dark
  showed **5×7 / 35 ghost keys**, while accepted scene stayed at 70 keys. Ghost
  bounds `(138.38,559.18)–(251.62,639.82)` clear the form bottom 510 and view
  controls top 689. Cancel clears ghost/form without changing accepted keys.
  Captures `shadcn-dark-390x844-new-matrix-5x7.png` and
  `shadcn-dark-390x844-cancelled.png`; `.evidence/011-full-gate/candidate-smoke.log`.
  This desktop-start/phone-resize case does not establish that a previously
  closed Sheet must open on Cancel; its unconditional Sheet-open wait was
  inapplicable. Prior phone-start G checks separately prove preservation of an
  already-open Sheet. No false success is inferred from that wait.
- **Agent observation R1 — human status pending:** the footer still reads
  `mechanical findings block case generation` after the corrected fixture's
  Case workspace says `Exact case geometry ready`, has visible geometry and
  enabled Export geometry. The stale wording also persists in other Workspace
  captures. Cause not diagnosed, no repair or acceptance claimed. Do not assign
  it a user-reported D-number or silently absorb it into the completed slices.
- All 12 contact sheets were inspected, not just generated. These captures
  support review of geometry/overflow/scrolling/selected and disabled states;
  they do not certify all contrast, keyboard/touch or application sequences.
  The full gate reran the bounded accessibility floor. Safari, screen readers,
  physical touch and broad accessibility certification remain unrun.
- Existing user D10 belongs to reopened 005; 004/005/006 still need their
  separately approved questioning sessions and work. D9's historical black-bar cause remains
  unproven. **Every human result above remains pending; 011 and 001 stay open.**

## Human re-review — second reported batch, 2026-11-09

The user reports “getting a bit better”, followed by the defects below on
http://127.0.0.1:4173/boardstudio/. **This is not approval.** The delivered
implementation is still `2694b189`, with documentation through `4edad33` before
these reports. Dark Appearance is visible in the attachments; exact Theme,
viewport and the reviewing client's loaded asset/cache identity are not yet
independently confirmed. Do not dismiss the reports as stale assets or promote
agent captures to human acceptance.

- **D11 — Split keyboard feedback delay.** Selecting `Split keyboard` under
  `[aria-label="Keyboard configuration"]` has a large delay before the assembly
  preview appears. Annotation `annotation_57l58y1y0h2ym_4`, screenshot
  `image_annotation_57l58y1y0h2ym_4`. Actual elapsed time, cold/warm distinction
  and cause have not been measured; no performance improvement is claimed.
- **D12 — Setup guide shape and separation.** The
  `[aria-label="Project setup guide"]` pane touches the upper pill. User asks
  for a rounded box with margin around it and clear separation, not a flush
  rectangular continuation. Annotation `annotation_57l58y1y0h2ym_6`, screenshot
  `image_annotation_57l58y1y0h2ym_6`.
- **D13 — Missing slider drag affordance.** New matrix Rows/Columns sliders
  lack the visible drag icon/handle. The reported Rows target is
  `[aria-label="Rows"][role="slider"]`, `data-slot="slider-thumb"`; the
  annotation also identifies both `data-slot="slider-range"` fills and values
  5/6. Annotation `annotation_57l58y1y0h2ym_c`, screenshot
  `image_annotation_57l58y1y0h2ym_c`. Do not equate a functional invisible thumb
  or retained keyboard control with satisfying this visual defect.
- **D14 — Canvas background still has full-width bands.** User identifies a
  broad bar at the top and another at the bottom, and says the workspace is not
  one solid background. Drawing `annotation_57l58y1y0h2ym_f` and selected
  `#m1-workspace-panel` in `annotation_57l58y1y0h2ym_k`; corresponding screenshots
  `image_annotation_57l58y1y0h2ym_f` / `image_annotation_57l58y1y0h2ym_k`.
  Track these visible canvas/toolbar-lane bands separately from the bounded
  dimension-popover correction. Their cause and relation to historical D9's
  black-bar report remain unproven; no shared-cause assertion is made.
- **D15 — Split New matrix preview shows only one half.** When Split keyboard
  was selected, the transient preview must show both left and right sides,
  not one 5×7 grid. Drawing `annotation_57l58y1y0h2ym_i`, screenshot
  `image_annotation_57l58y1y0h2ym_i`. Current G's single-grid visibility proof
  does not satisfy this split-configuration expectation. Preserve transient
  paint ownership and no accepted-state/history/camera writes when correcting it.
- **D16 — Align/Transform separated too far from scope toolbar.** Keep these
  controls visually associated with the Matrix/Row/Column/Key/Part strip;
  current secondary pill floats far below it. Annotation
  `annotation_57l58y1y0h2ym_15`, screenshot `image_annotation_57l58y1y0h2ym_15`;
  targets include `#m1-layout-align-trigger`, `#m1-layout-transform-trigger`,
  `#m1-layout-scope-Key`, `#m1-layout-scope-Part` and `#m1-layout-scope-Row`.
- **D10 reiterated — Row actions popup still present.** The accompanying
  `image_eae1dd72-770a-4823-8d92-287f03efca16` shows the `Row actions` popup,
  `Close Row actions` control and `Open Inspector` beside the scope selector.
  This is the already-reopened [005](005-restore-scope-actions.md) duplication
  issue, not a delivered restoration. Route its toolbar overlap through 005's
  required documented interview before implementation; do not silently fold the
  deferred behavior into an unrelated styling correction.

### Next-correction proof map (planned, not executed)

Each row needs a faithful failing loop before its production correction. These
are user-reported symptoms and supplied visual evidence, not newly run automated
RED/GREEN or timing results.

| Requirement | Preconditions → exact action → independent oracle | Tier / status |
| --- | --- | --- |
| D11 response latency | Empty project, providers cold then warm → select Split keyboard → measure trusted click to visible assembly preview and distinguish first feedback from completed geometry; record actual before/after timings | Packaged / planned |
| D12 guide presentation | Setup guide open → inspect against actual upper pill and viewport → visible margin on all intended edges, rounded guide surface and non-touching bounds | Packaged/rendered / planned |
| D13 slider affordance | New matrix open → inspect Rows/Columns and drag each → visible themed handle/icon with usable hit region, live dimensions and retained keyboard behavior | Mounted + packaged / planned |
| D14 solid canvas | Layout before and after creation → inspect top/center/bottom away from controls and annotation overlays → one intended canvas paint with no full-width contrasting bands | Packaged/rendered / planned |
| D15 split ghost | Select Split keyboard, New matrix still uncommitted → change Rows/Columns → both left/right transient footprints update, while complete accepted model/history/selection/camera remain unchanged; single-keyboard case stays single | Existing real-Editor scenario + packaged / planned |
| D16 toolbar grouping | Populated Layout at agreed widths → inspect scope, Align and Transform, then open their menus → compact deliberate inter-group gap, no separated lane/overlap or lost keyboard/focus behavior | Rendered + packaged / planned |
| D10 / 005 | Selected Row, with scope selector → open the current actions surface → reproduce duplication and use 005's approved questioning/spec to define the single intended action path | Existing 005 protocol / planned |

The human review remains in progress; batch closure and correction disposition
have not been stated in this message. No new correction, rebuilt candidate or
acceptance is claimed by this report. R1's stale mechanical footer and earlier
unresolved observations remain open. **011 stays awaiting-decision; 001 stays open.**

### Agent investigation — second batch D10–D16 (2026-11-09, investigation only)

Research sub-agent run. **No production, test, build or candidate change; nothing
here is acceptance.** HEAD `9d642fa`; the candidate
(`web/target/site-011-candidate`, built from `2694b189`, server PID 1511513) was
used unchanged and left running. All browser work used one isolated
agent-browser session (`r2inv-011-opus`, headless Chromium 155, 1440×900 unless
stated, dark scheme → `data-appearance=dark`, `data-theme=original`) and
disposable projects created there; the session was closed afterwards. The
user's browser and storage were not touched. Counterfactuals were injected CSS
in that session only, removed after measurement and re-measured restored.
Source facts come from the exact Gortex worktree view at `9d642fa`.

Provenance key: **N** = newly executed in this run; **A** = user attachment
(`_4`, `_6`, `_c`, `_f`, `_i`, `_k`, `_15`, `image_eae1dd72`); no earlier logs
were reused as evidence. Logs: `.evidence/011-round2-investigation/`. Images:
`web/target/test-results/011-round2-investigation/`. The temporary probe
helpers (`d11-seq.js`, `d11-arm-fn.js`, `geom.js`) are in `~/.cache/tmp/011r2/`.
The shadcn theme was not exercised. D12–D16 are layout-structure causes and do
not depend on colour, but D14's contrast was measured only for Original/dark.
The user's client, viewport and CPU remain unconfirmed. From the attachment
scale, `_k` is consistent with a panel about 670 px wide (reproduced at
`vw=1322` with the tree and Inspector open).

| ID | Reproduced (N) | Cause | Smallest proposed correction (not implemented) |
| --- | --- | --- | --- |
| D11 | Yes | **Ordering proven:** the visible pressed state and Assembly preview publish only after the durable IndexedDB write. The cold +≈140 ms main-thread task is **not attributed**. | Paint pending feedback from the submitted intent; durability semantics are a decision |
| D12 | Yes | **Proven:** setup overlay `section` is flush (`absolute inset-y-0 left-0`, no radius) | Inset the overlay by the 8 px gutter, add `rounded-xl` and a border |
| D13 | Yes | **Proven:** shared `Slider` thumb is static inside the `overflow-hidden` track, so it is clipped and not hit-testable | Absolutely centre the thumb and make the track overflow visible, at the call site or in the shared Slider (decision) |
| D14 | Yes | **Proven:** `bg-card` panel lanes around an inset `bg-canvas` svg | Paint the Layout panel with the canvas colour, keeping lane geometry |
| D15 | Yes | **Proven:** the ghost paints one grid and receives no topology | Reuse `project_mirrored_pair` for Split, using topology from the accepted snapshot |
| D16 | Yes | **Proven:** the context-strip grid collapses and an empty `.layout-action-slot` row stays reserved | Order or collapse the rows so Align/Transform sit directly under the scope strip |
| D10/005 | Yes | **Proven:** every scope click opens the `{Scope} actions` dialog, which duplicates the inline action toolbar | **Questioning 005 first**; boundaries below |

#### D11 — Split keyboard delay (N)

- **Loop.** New project → Setup guide → trusted CDP click (`isTrusted:true`) on
  `[aria-label="Keyboard configuration"] > button:nth-of-type(2)`. A
  MutationObserver plus a `longtask` PerformanceObserver and wrapped
  `IDBDatabase.transaction` recorded times from `pointerdown`. Run it with:
  `agent-browser eval "($(cat ~/.cache/tmp/011r2/d11-seq.js))('Split keyboard')"`,
  then `agent-browser find role button click --name "Split keyboard"`, then read
  `window.__seq`.
- **Warm (alternating One/Split, at least 16 clicks):**
  - Buttons become `disabled` at 6.5–7.9 ms. This is the only immediate
    feedback; the pressed state does not change yet.
  - The `projects,assets` readwrite transaction opens at about 19–25 ms and
    commits at 20–39 ms.
  - `aria-pressed`, `data-state=on`, the preview text and the second schematic
    `rect` land in one mutation at 33–56 ms (one 113 ms outlier). No long tasks.
  - Logs: `d11-warm-toggle.log`, `d11-sequence-warm.log`,
    `d11-usertiming-counterfactual.log`.
- **Cold:**
  - First Split in a fresh project: DOM at 103 ms, then a 168 ms long task, with
    the first frame after it at about 245 ms. Repeated under the eager-wasm flag:
    DOM at 75 ms, 150 ms long task.
  - First Split after reload, 2 sessions × 3 runs: the DOM lands at 67–97 ms,
    inside a 133–157 ms long task that starts at the IndexedDB commit. The first
    frame after the mutation comes at 190–235 ms.
  - Logs: `d11-cold.log`, `d11-sequence-cold.log`,
    `d11-eager-wasm-counterfactual.log`.
  - **The trigger is not pinned.** Six later reload → first-Split runs had no
    long task, while the first Reversible-on edit in each reloaded
    page had one of 136–143 ms in 4 of 4 runs (`d11-observer-discriminator.log`,
    `d11-first-edit-discriminator.log`).
- **Proven ordering (one-variable probe, `d11-idb-hold-probe.log`).** Holding a
  competing `projects` readwrite transaction for about 590 ms (gets only, no
  writes) delayed pressed state + preview to 606–608 ms, about 17 ms after the
  app's commit. Without the hold it was 41 ms. Visible latency therefore tracks
  storage latency one to one.
- **Matching source.** In `web/crates/pcb/src/pcb_physical_setup/controller.rs`,
  `use_controller` sets only `busy` on click (line 236). `project_setup_controls`
  derives `ToggleGroup.pressed` from the accepted `projection.topology` (around
  lines 331–337). Settlement is polled with `TimeoutFuture::new(16)` (line 306),
  which matches the 13–20 ms post-commit lag.
- **Hypothesis, not isolated:** the runtime's `PendingEdits`/persistence path
  publishes the accepted snapshot only after durability (`DocumentPersistence`,
  `web/crates/runtime/src/runtime.rs:251-258`).
- **Falsified (N):**
  1. dioxus `performance.mark`/`measure`. They were 60 % of profiled main-thread
     samples (`d11-cold-profile-summary.log`), but stubbing them left the long
     task at 133–140 ms against 139–142 ms, so the profile share was a profiler
     artifact.
  2. V8 lazy wasm compilation. With `--js-flags=--no-wasm-lazy-compilation`
     (verified on `chrome://version`) the long task stayed at 139–150 ms.
  3. A persisted derived cache. The `assets` store has 0 rows
     (`d11-idb-stores.log`).
- **Caveats.** These absolute numbers come from this fast headless host. No
  latency threshold and no backend-versus-UI split is claimed.
- **Smallest correction (needs a decision).** While `busy`, paint the requested
  topology's pressed state and schematic preview from the submitted intent,
  reverting on Retire. The accepted, history and durability semantics stay
  unchanged. The alternative, publishing accepted state before durability, is
  architectural and needs an ADR check.
- **Regression oracle.** Packaged probe with the IndexedDB hold: visible Split
  feedback must appear within about one frame of `pointerdown` while a 600 ms
  hold is active. This is red today at 606 ms. A retired edit must restore One.
- **Next experiment for the cold task.** Capture a symbolized wasm profile
  (name section kept; the build is coordinator-owned) of the two repeatable
  triggers: new project → first Split (2 of 2) and reload → first Reversible-on
  (4 of 4).

#### D12 — Setup guide flush rectangle (N)

- **Bounds at 1440×900** (`d12-geometry-1440x900.log`,
  `d12-guide-dark-1440.png`):
  - The header pill ends at bottom=64. The `aside[aria-label="Project setup guide"]`
    and its parent `section[aria-label="Keyboard setup"]` start at x=0, y=64,
    radius 0, margin 0, with bottom=860, which is the shell-footer top.
  - The gap to the pill is therefore **0 px**, and both surfaces use the same
    rgb(36,40,49).
  - The adjacent workspace panel has an 8 px gutter and a 14 px radius.
  - Matches `_6` (A).
- **Cause.** `web/src/presentation.rs:9666` gives the guide overlay
  `absolute inset-y-0 left-0 … border-r … bg-card`, with no inset or radius.
  The supplied aside class (`web/crates/layout/src/setup_guide.rs:328`) is the
  inner content, not the box.
- **Counterfactual (N, restored).** `top/bottom/left: 8px`, `border-radius: 14px`
  and a 1 px border give the overlay y=72–852, x=8, radius 14. That is an 8 px
  gap to the header, the footer and the viewport edge
  (`d12-counterfactual-geometry.log`, `d12-counterfactual-inset8-radius14.png`).
- **Proposal.** Change the overlay to `top-2 bottom-2 left-2 rounded-xl border
  border-border-strong` and give it the matching width or max-width budget. Then
  decide the `max-[760px]` phone variant separately.
- **Oracle.** Rendered bounds at 1440, 1024 and phone width: the gap to the
  header pill and to the shell footer is ≥ 8 px, left ≥ 8 px, radius > 0, and
  internal scrolling and the sticky footer are unchanged.

#### D13 — Slider thumb not painted (N)

- **Computed** (`d13-slider-computed.log`):
  - Each Rows and Columns `[data-slot=slider-thumb]` is `position: static`, so
    its inline `left: 36.36%` has no effect. It sits at x=22, the track start.
  - It is 16 × 44 px because the guide's `[&_button]:min-h-control-touch` applies
    to it, and the 6 px `overflow:hidden` track clips it.
  - The screenshot shows only the range fills (`d13-new-matrix-dark.png`, which
    matches `_c` (A)).
- **Interaction** (`d13-drag-live.log`):
  - At the thumb's nominal centre the hit target is the `LABEL`, and a drag
    there changes nothing.
  - Keyboard still works: ArrowLeft takes Rows 5→4. That does not satisfy D13.
- **Cause.** The shared
  `web/crates/ui-shared/src/ui/slider.rs::Slider` (lines 22–94) nests
  `SliderThumb` inside the `overflow-hidden` `SliderTrack` and gives it no
  absolute or centering classes.
- **Precedent.** The Case sliders patch this per call site
  (`web/crates/case/src/shared_viewer.rs:3102/3124/3149/3172`). The 020 lane
  decision recorded "no shared Slider edit".
- **Counterfactual (N, restored).** Thumb absolute at `top:50%` with
  `translate(-50%,-50%)`, 16 px tall, and the track `overflow:visible`:
  - The thumb appears at the correct fraction (Rows x=105.4 at 4 of 1–12).
  - `elementFromPoint` returns the thumb.
  - A real +80 px drag moved Rows 4→7 and the thumb followed.
  - The counterfactual also showed the thumb crowding the next label, so the row
    needs an activation height, as in the Case precedent's `min-h-control`
    (`d13-counterfactual-thumb.png`).
- **Proposal.** Either add the Case class set plus a row height to the New
  matrix sliders (`MatrixDimensionSliders`, in
  `web/crates/layout/src/objects/matrix_setup_fields.rs`), or fix the shared
  Slider once. The shared fix touches every slider and makes the Case overrides
  redundant; that is a scope decision. The Inspector Rows slider
  (`matrix_inspector.rs:444`) probably shares the defect; not verified.
- **Open question for the user.** Is a visible round thumb enough, or does "drag
  icon" mean an added grip glyph?
- **Oracle.** Mounted plus packaged checks:
  - A 16 × 16 thumb, centred on the track at the value fraction.
  - The hit-test lands on the thumb.
  - Pointer drag changes `aria-valuenow`, the `strong` label and the ghost.
  - Keyboard and touch target are retained.

#### D14 — Full-width top/bottom bands (N)

- **Layers** (`d14-panel-layers-setup-split.log`, `d16-toolbar-geometry-1440.log`):
  - `#m1-workspace-panel` is `bg-card` rgb(36,40,49).
  - Its canvas `svg` is `bg-canvas` rgb(24,27,33) with
    `mt-[76px] h-[calc(100%-120px)]`.
  - In a 780 px panel that leaves a 76 px top band and a 44 px bottom band of
    panel colour.
  - The bands are present with no annotation tint or overlay, during setup with
    the toolbar hidden (`d15-split-ghost-full.png`) and in a populated Layout
    (`d14-d16-populated-1440.png`). They match `_f`/`_k` (A).
  - They are unrelated to the dimension popovers.
- **Cause.** `web/src/presentation.rs:9764` (svg class) inside the panel at
  `presentation.rs:9666` context. The bands are the toolbar and view-dock lanes.
- **Counterfactual (N, restored).** Painting the panel with the canvas colour
  removes both bands (`d14-1322-cf-panel-canvas-bg.png`). The thin blue lines in
  that image are the svg focus outline left by the probe, not bands. The
  grid-dot pattern still covers only the svg area.
- **Options.**
  - (a) Paint the Layout panel `bg-canvas`, or give the lanes `bg-canvas`.
    Lowest risk: lane geometry, camera and fit are unchanged. Recommended.
  - (b) Make the svg full height with the toolbars floating over it. This
    changes the viewBox, fit, hit-testing and setup-ghost clear space, so it
    needs camera and fit regressions.
- **D9 link.** No black bar was observed. The relation to historical D9 remains
  **unproven**.
- **Oracle.** Sample rendered pixels at the top, centre and bottom lane, away
  from controls: one canvas colour in both appearances and in Original and
  shadcn.

#### D15 — Split ghost shows one half (N)

- **Reproduced in a Split project** (`d15-split-ghost-full.png`). The canvas
  ghost `g[aria-label="New matrix preview"]` and the guide thumbnail both show
  one 7 × 6 grid. Create then commits a Left half plus a
  "Right half · linked mirror" (`d15-after-create-split.png`). Matches `_i` (A).
- **Cause.**
  - `MatrixSetupCanvasPreview`
    (`web/crates/layout/src/objects/matrix_setup_preview.rs`) paints only the
    private `Paint{owner, rows, columns}` and has no topology input.
  - The Create resolver
    (`web/crates/layout/src/objects/matrix_setup_controller.rs:71-100`) reads the
    accepted `document.hardware.topology` and, for Split, calls the pure
    `mirrored_pair_geometry::project_mirrored_pair(…, gap_mm: 40.0)`.
- **Proposed narrow seam:**
  - Carry `split` from the same accepted snapshot. The Layout call site
    (`presentation.rs:9870`) already has
    `physical_setup_mount.projection.topology` in scope (used at
    `presentation.rs:9743`). Pass it as either a `MatrixSetupProjection` field or
    a `MatrixSetupCanvasPreview` prop.
  - Inside the component, project both halves with the same
    `project_mirrored_pair`/`preview_cells` geometry as Create, and include the
    40 mm gap and both widths in the fit scale.
  - Keep the paint store private. Add no runtime request and no accepted-state,
    history, selection, camera or fit writes.
  - Either option widens a `pub` layout-crate type or prop, so the coordinator
    must choose against the "no public API widening" constraint.
- **Oracle.** Extend
  `web/src/presentation/layout_field_preview_tests.rs::new_matrix_sliders_preview_on_canvas_without_changing_accepted_state`
  (line 521) with Split and One cases:
  - Two mirrored footprints for Split, one for One, updating with Rows/Columns.
  - The full accepted model, history, selection and camera stay unchanged.
  - Add a packaged visual check.

#### D16 — Align/Transform floating away (N)

- **Reproduced at user-like geometry** (`vw=1322`, panel 670 px, Key scope, no
  selection after reload): Align is 128 px from the panel top (the user's `_k`
  is about 124 at its capture scale) and 68 px below the scope strip
  (`d16-noselect-counterfactual.log`, `d16-1322-baseline-noselect.png`).
- **Width sweep** (`d16-width-sweep.log`): at 1440 the gap is 12 px, but Align
  is centred in the right grid column, away from the scope strip. At a 372 px
  panel the scope strip wraps to 90 px and the gap is 110 px.
- **Cause:**
  - `web/assets/layout-toolbar.css:6`: `.layout-context-strip` uses
    `grid-template-columns: repeat(auto-fit, minmax(min(100%, 22rem), 1fr))`.
    Below about 720 px it collapses to one column, so `.layout-action-slot`
    takes row 1 and `.layout-menu-actions` takes row 2.
  - `layout-toolbar.css:7` and `:16-19` deliberately reserve the slot's height
    "including when selection has no actions".
  - `web/crates/layout/src/objects/layout_toolbar.rs:1326-1334` always renders
    `.layout-action-slot` when pointer tools are visible, even without
    `has_selection_context`.
- **Counterfactual (N, restored).** `.layout-action-slot:empty{display:none}`
  brings Align to 72 px from the panel top, 12 px under the scope strip.
- **Trade-off.** The reservation exists to stop the toolbar shifting when a
  selection appears.
- **Options.**
  - (a) In single-column mode, place `.layout-menu-actions` before the action
    slot.
  - (b) Collapse the empty slot and accept the shift.
  - (c) Move Align/Transform into the scope-strip row.
  - Coarse-pointer rules (`layout-toolbar.css` `@media (pointer: coarse)`)
    reserve the same way.
- **Relation to D14.** The causes are independent but share a region. The
  svg's 76 px top lane covers only the scope strip (y 84–132); the context strip
  (y 140–244) overlaps the canvas top (148). Fixing either defect does not fix
  the other.
- **Oracle.** Rendered bounds at 1440, 1322, 1024 and phone width, with and
  without selection: `align.top − scope.bottom` stays within the agreed compact
  gap, nothing overlaps, and the menus, keyboard access and focus behave as
  before.

#### D10 / 005 — Scope popup duplication (N; no implementation)

- **Reproduced at 1322** (`d10-row-actions.log`, `d10-row-actions-1322.png`):
  `#m1-layout-scope-Row` press → `role=dialog` "Row actions" at x473 y140,
  276 × 173, with the hint, "Close Row actions" and "Open Inspector". It covers
  the context-strip lane where Align/Transform sit, and the Inspector was
  already open. Matches `image_eae1dd72` (A).
- With a Key selected, the Key scope press opened "Key actions" containing
  Position, Stagger, Offset and Spread above the identical inline strip
  (`d16-key-noselect-1322.png`), which duplicates interactive controls.
- **Cause.** `web/crates/layout/src/objects/layout_toolbar.rs::LayoutSelectionToolbar`
  (line 1373; popover around lines 1393–1447):
  - Each scope `ToggleItem` click both sets the scope and runs
    `open_menu.set(Some(LayoutCommandMenu::Select))`.
  - With actions, the popover renders the same `LayoutContextActionToolbar` used
    inline at lines 1328–1331.
- **Proposed boundary:**
  - D16 owns only strip geometry and order. It must not change the popup's open
    semantics, the Select menu or action availability, and it must keep the
    popup anchoring and focus.
  - 005 owns whether, and when, a scope press opens a popup, plus duplicate
    removal.
  - If 005 moves the empty-selection hint inline, it would occupy the reserved
    slot that causes D16's empty row. Agree the sequencing first.
- **Questions for 005's required documented interview (do not implement before it):**
  1. With inline tools present, should a scope press ever open a popup?
  2. With no selection, should the hint stay a popup, or render inline in the
     action slot?
  3. Are Key and Part in scope? 005 names Matrix/Row/Column, but Key duplicates
     too.
  4. Is "Open Inspector" wanted when the Inspector is already open?
  5. Should pressing the active scope again keep toggling the popup?
- 004 and 006 remain deferred and outside these findings.

#### Unresolved and next experiments

- **D11 cold long task:** capture the symbolized profile described above, and
  check whether Reversible-on and fresh-project Split share one first-time
  derivation. Two hypotheses were falsified, so no third blind probe was run.
- **Theme and client:** repeat the D14 sampling under shadcn, and confirm the
  reviewing client's viewport and scale.
- **D13:** confirm the Inspector slider defect.
- **Fix decisions:** D15's seam owner and D13's shared-versus-call-site fix need
  coordinator or user decisions.

No correction, rebuilt candidate or acceptance is claimed. **011 stays
awaiting-decision; 001 stays open.**

**Coordinator receipt:** all 321 appended investigation lines were read through
exact Gortex, plus the raw IndexedDB-hold, slider-drag and empty-slot measurement
logs. The visual causes have evidence; D11's ordering is demonstrated, but the
cold long-task cause and the user's full perceived latency remain unresolved.
Only 011 changed in the assigned work; production/test source and candidate
remain unchanged. Documentation links (741 files) and whitespace passed; no
product gate was rerun for this report. Implementation remains a separate phase.
Execution caveats: the delegate also read the ticket through the shell at startup
and during append verification, contrary to the required Gortex-only source
route. The coordinator's complete report read used Gortex. The delegate left a
filesystem checkpoint at `~/.cache/tmp/011r2/notes-checkpoint.md`, not the required
virtual notes transfer; the coordinator's virtual checkpoint is authoritative.
These deviations are recorded, not described as compliant tool use.

## Second-batch correction execution plan — 2026-11-09

User decision after investigation: **round slider thumb; fix all identified
issues using Sol medium subagents via Pi**. This authorizes the correction
phase and the necessary read-only split-mode input on the existing presentation
preview; it does not authorize broader API visibility, accepted-state-before-save
semantics or new draft/runtime ownership. New delegates use Pi
`openai-codex/gpt-6.1-sol`, `thinking=medium`, including independent re-checks.

The existing planned proof rows above remain the starting map; each owner must
record actual RED before production correction, reuse adequate existing tests,
and return GREEN evidence and owned-file inventories. No screenshot or prior
source presence stands in for acceptance. The coordinator owns this ticket's
tracking and all final gates/commits/candidate promotion; children must not edit
this ticket or each other's files.

| Lane | Exclusive initial ownership | Required result / sequencing |
| --- | --- | --- |
| A — D13 round thumbs | `web/crates/ui-shared/src/ui/slider.rs`, `web/crates/ui-shared/src/ui/primitives_mount_tests.rs`; existing Case slider consumer inspected read-only | Implementation returned: 95 shared browser / 23 native checks and 16 packaged interaction cases reported GREEN; fresh combined review Standards PASS / Spec PASS; coordinator final-fixture baseline/restored GREEN and all four meaningful mutants killed; acceptance BLOCKED on integrated gate and human review, no new matrix/page edits or Case cleanup |
| B — D15 split ghost | `web/crates/layout/src/objects/matrix_setup_preview.rs`, `web/crates/layout/src/objects/matrix_setup.rs`, `web/crates/layout/src/objects/matrix_setup_fields.rs`, `web/src/presentation.rs`, `web/src/presentation/layout_field_preview_tests.rs` | Implementation returned and page ownership released: approved provider input `accepted_split: bool`, pure mirrored geometry and both-half canvas/thumbnail; producer RED 30/60 cells, then 5 field-preview + 1 spacing + 7 Layout mounted + 168 native checks GREEN (1 ignored); fresh combined source Standards PASS / Spec PASS; coordinator's first five mutations killed three and exposed two proof gaps; only the existing Editor story was extended (+72) and now kills the fit/topology survivors; 23 affected browser checks, formatting and WASM Clippy GREEN; test-delta review Standards PASS / Spec PASS (task 3ffba…, read-only receipt reuse, recorded in coordinator notes); B closed (kills + diff read also satisfy the re-review rule) |
| C1 — D11 pending feedback | Initial PCB physical-setup scope returned unchanged; further implementation waits for the narrow preview accessor approval and page wiring after lane B releases it | Hold RED recorded (601 ms IDB → 659 ms DOM / ~826 ms RAF opportunity, not presentation proof); user approved R4 new phase. Planner boundary + Builder shipped narrow busy-gated pending_topology() (accepted-only preserved): P1a/P1b 627.8→14.8 ms, P2–P4 GREEN, 5/5 mutants killed, T1 recorded; fresh Spec PASS after display-proof close. Reload→Reversible stays follow-up F2. Detail in the C1 section below |
| C2 — D11 cold catalogue work | `web/crates/layout/src/objects/matrix_transform_controller.rs`, including its existing regression seam; other files require allocation | Implementation and fresh review returned: Standards PASS / Spec PASS for the actual Key-projection demand guard; same-path debug RED/GREEN and later-Key positive demand, 168 native / 50 targeted browser checks reported GREEN; remove direction RED-covered; invert mutant killed after same-story strengthening (R2 closed); optimized timings have shared-checkout comparability limits; C1 done (see below), reload→Reversible is follow-up F2 |
| D0 — D16 and D10/005 | Toolbar/CSS returned/frozen; private `presentation.rs` correction returned/released; coordinator corrected only existing page-test host teardown, preserving all scope assertions and B/D1; 005 tracking remains coordinator-owned | Original popup-focus/grouping/Part-action RED then 13 toolbar / 268 Layout plus 2 guide / 168 native and packaged transfers GREEN; persistent Matrix-clear RED corrected without a new owner; all five unchanged scope cycles now pass, including managed teardown after independently evidenced host correction; 23 affected browser checks / formatting / WASM Clippy / whitespace GREEN; fresh combined Standards PASS / Spec PASS, all 71 hunks read and no findings; both page-guard mutants killed with restored GREEN; trigger-focus mutant killed after same-story strengthening (R2 closed); extra toolbar guards are follow-up F1; integrated/HITL gates pending |
| D1 — D12/D14 | `web/src/presentation.rs`, `web/src/presentation/layout_field_preview_tests.rs`, `web/crates/layout/src/setup_guide.rs`, `web/assets/shell.css`; B's fresh review completed, exclusive page writing now released to this lane | Implementation returned and page ownership released to the coordinator: only the two page files changed, guide/stylesheet files unchanged; actual gutter and paint RED, then 6 page / 268 Layout / 168 native checks plus formatting/Clippy/fast-package GREEN; fresh combined Standards PASS / Spec PASS, no unread hunks and owned postimages unchanged; review hold released, only the existing page-test file passes to D0's regression owner; coordinator sensitivity and integrated/HITL gates remain |
| E — remaining R1 footer | Implementation returned and ownership released: only `web/crates/runtime/src/runtime.rs`, private bounded report provenance/retirement plus existing in-file lifecycle seam; no presentation/CSS/public API changes | Faithful Runtime rebind RED then 62 Runtime browser / 138 native checks reported GREEN; original same-owner battery −86 → 0 → −86 / Update preview loop GREEN with populated exact geometry, enabled Export and no stale footer; fresh combined Standards PASS / Spec PASS, no unread hunks or required findings; coordinator sensitivity closed (provenance-removal + indiscriminate-clear both killed); integrated/HITL gates pending |

Lane A must request ownership before changing any unlisted consumer. Lane B may
use only the narrow approved presentation input, not widen unrelated members.
Lane C2 now owns only the additional matrix-transform controller file; it must
request ownership before any other runtime/Core/page edits or architectural
proposal. C1's proposed pending-topology accessor is not yet approved.
All builds/tests share inherited FD9 lock; no overlapping Cargo or
wasm-pack, no per-agent full gate. Temporary mutation experiments are
coordinator-owned so one lane cannot measure another lane's in-flight edits.

**005 questioning completed:** user answered **“yes, yes”** to one inline action/hint
surface for all five scopes (no scope popup on choice or reselection), and
Align/Transform beside the scope strip when space permits or directly beneath
it otherwise, without the empty reserved row. [005's current criteria and proof
map](005-restore-scope-actions.md#approved-proof-map) explicitly replace its old
empty-selection hint-panel requirement while preserving useful guidance,
selection transitions and focus. Lane D0 can proceed independently of lane B;
lane D1 received exclusive page-file ownership after B's completed review. Historical cancellation
and investigation records remain intact; no result or acceptance is inferred.

Lane A transfer limits are explicit: its mounted RED preceded correction of the
synthetic selector to the real guide utility, so final-fixture sensitivity against
unfixed styling remains a coordinator gate. Lane C1 made no tracked changes and
claims no GREEN; its debug catalogue attribution is not optimized before/after
proof or an explanation of the separate reload→Reversible trace. Transfer logs:
`.evidence/011-round2-A/` and `.evidence/011-round2-C/transfer.log`.
A's reviewer specifically identified a possible clipping-proof gap: the final
fixture probes only the thumb centre, so restoring track clipping might survive.
The coordinator must measure it, then add upper/lower thumb hit probes in the
same scenario if needed; absolute positioning, guide sizing protection and the
activation-row minimum also need independent sensitivity checks.

Coordinator resolution: the corrected final fixture passed its unmutated run,
then **all four** mutants were killed by the intended rendered oracle, with zero
compile failures or survivors and exact source restoration. Track clipping failed
at End/value 90 because the thumb centre hit BODY rather than BUTTON; removing
absolute positioning misplaced the centre; removing the guide exception stretched
the thumb to 16 × 44 px; removing the activation minimum shrank the row to 6 px.
The restored scenario passed again. Thus the suspected clipping survivor was
falsified; extra upper/lower probes were not added without a distinct uncovered
failure. Logs: `.evidence/011-round2-A/coordinator-sensitivity/`. The earlier
synthetic-fixture RED limitation remains historical, not silently relabelled.

B's fresh fast-package proof reused providers and included concurrent Slider
changes; logs/captures are in the `011-round2-B` evidence directories. Broader
Theme verification and integrated gates remain outstanding. B's fresh combined
review is complete: Standards **PASS**, Spec **PASS**, acceptance **BLOCKED** on
coordinator sensitivity, integration/build, broader capture checks and human
review. All five owned postimages returned `not_modified` against their initial
ETags despite parallel global graph generations changing 6331 → 6334. Optional
memo-transition/caption coverage is not represented as an executed test.

R1 planned proof: one real Editor owner with a blocked generation → repair the
accepted physical inputs (or Undo to a reusable valid result), then await current
exact geometry → no obsolete mechanical-blocker footer while geometry export
remains correctly enabled; an unrelated live error must remain visible. Tier:
production mounted/packaged; the original public loop is now GREEN in
`.evidence/011-round2-E-fix/original-public-loop.log`, with the bounded fresh
E review below. The existing `case_generation_title` native regression remains
insufficient alone for this separate global-footer behavior.

### A/B fresh review scope receipts

Both compare fixed base `bb70ba0df640521291720cfe7a86ac9531e7d773` with their
reserved working postimages; HEAD did not move. Each independently reports
Standards **PASS** and Spec **PASS**, **no unread changed hunks**, and acceptance
**BLOCKED** on the remaining coordinator/human gates. This is not an aggregate
re-review or a waiver of the earlier 445-path cap. Paths below are under `web/`.

| Slice | File | Changed postimage ranges read (baseline/postimage recovered exactly) |
| --- | --- | --- |
| A | `crates/ui-shared/src/ui/slider.rs` | Full baseline/postimage; 27–28, 30, 79, 82, 90 |
| A | `crates/ui-shared/src/ui/primitives_mount_tests.rs` | Baseline 1–55, postimage 1–160; insertion 38–113 complete |
| B | `crates/layout/src/objects/matrix_setup.rs` | 317 |
| B | `crates/layout/src/objects/matrix_setup_fields.rs` | 153–158, 166–183, 187 |
| B | `crates/layout/src/objects/matrix_setup_preview.rs` | 33–36, 40–46, 106–117, 119–120, 125–126, 134–138, 150, 160–180, 184, 186–194, 198–218 |
| B | `src/presentation.rs` | Deleted baseline 3267; 3589–3594 |
| B | `src/presentation/layout_field_preview_tests.rs` | 246–249, 251, 280–284, 290, deletion seam 292, 530–549, 554, 584–669, 696, 699, 737, 766, 784, 792–813, 821 |

A independently executed scoped whitespace verification and reused producer
checks; B executed no behavioral checks and reused its producer receipts. Neither
review mutated source. A recorded commit generation 6286 / dirty 6320; B recorded
commit 6286 / dirty 6331 → 6334 with owned content unchanged. Their source/evidence
judgments do not establish final-fixture sensitivity or a new full-gate result.

C2's returned correction is 20 added / 12 removed lines in the single allocated
controller, mostly moving the existing projection before the effect and adding
a reactive actual-Key demand guard. Its public same-path profiling and positive
later-Key experiment are the performance proof, not a fake private-helper unit
test. Logs: `.evidence/011-round2-C2/transfer.log`. Fresh independent combined
review: Standards **PASS**, Spec **PASS**, no required source findings or unread
changed hunks; baseline/postimage ranges were 715 → 715–729, 718 → 732,
after 719 → 734–736, and 748–757 → 765. Owned content returned `not_modified`
while foreign dirty generations changed 6344 → 6346. The reviewer newly checked
whitespace and independently reanalyzed the three existing RED/GREEN/later-Key
traces; it reused the producer's build/test receipts. Sampled absence is not
absolute no-call proof, and optimized observations are paint-opportunity proxies,
not actual presentation timings. Acceptance remains **BLOCKED**: the coordinator
must remove the guard and reject first-Split absence, invert it and reject later-Key
positive demand, restore and verify both, then complete integration. C2's earlier
blocked broader Layout run and follow-on WASM panic-hook failures are not called
GREEN; D0's later Layout-only success below does not retroactively change that run.

D0 returned its toolbar/CSS implementation without page edits. Logs:
`.evidence/011-round2-D0/`; packaged measurements put Align/Transform 8 px beside
the scope strip at 1322/1440 px and 8 px below at 1024 px, unchanged by selection,
across both Themes/Appearances. Five real canvas transfers replaced inline guidance
with actions, including Part Position, without scope popups. Existing production
focus/selection checks passed separately. Its final Layout run passed 268 general
plus 2 setup-guide tests, but the page field-preview run encountered D1's in-flight
guide-gutter RED and is **not GREEN**. Persistent production-mounted guidance
coverage still needs the existing page-test seam after D1 releases it; toolbar
projection fixtures and packaged captures are not substituted for that planned
regression. Independent combined review and coordinator sensitivity remain pending.

D1 returned only `presentation.rs` and its field-preview test file; no guide
component or stylesheet edit was needed. Its existing resized-guide story gained
51 computed geometry/paint assertion lines, without a new fixture or API. Actual
REDs were the flush 0 px/radius-0 guide and unequal Layout panel/SVG paint;
corrected page-plus-spacing checks passed 6/6. Logs:
`.evidence/011-round2-D1/transfer.log`. Twelve guide and twelve populated Layout
captures covered both Themes/Appearances at 1440/1024/390 px: 8 px guide gutters,
14 px Theme radius, no horizontal overflow, and identical top/centre/bottom canvas
paint. The existing phone 60% form budget, hidden-but-mounted compact Sheet,
B's accepted split preview and Export/setup overflow precedence were retained.
Its fast package reused providers and is not an optimized/full-gate transfer.
Fresh independent review returned Standards **PASS**, Spec **PASS**, no required
source findings and no unread changed hunks. It read all baseline/current page
hunks and independently viewed the eight contact sheets plus full desktop/phone
captures. D1 deltas: presentation 9666 → 9671 and 9719 → 9724–9727;
field test insertion after 358 → 367–417. Both full-file ETags returned
`not_modified` at generations 6356/6366, epoch 4068. New scoped whitespace passed;
behavioral/build receipts were reused. Readiness remains **BLOCKED** on independent
sensitivity/integrated/HITL gates. Container-width assertions do not change browser
media queries: phone-width sensitivity must use a real 390 px packaged viewport;
non-Layout card preservation needs its own oracle. The review hold is released.
D0's test owner initially returned **only** `layout_field_preview_tests.rs`, adding
one five-scope real Editor story (249 lines); B/D1 hunks were preserved. Parent
read the story and both RED logs. Matrix guidance → real canvas selection →
usable inline actions → Inspector reopening passed. Ctrl-click demonstrably
cleared Runtime selection, but inline guidance did not return; the independent
Runtime-empty assertion passed before the guidance timeout. Four other scopes
were then unreached, not passing. Initial field/spacing/focus run: **7 passed,
1 failed**; WASM Clippy, rustfmt and whitespace pass. Logs:
`.evidence/011-round2-D0-mounted/final-affected.log` and
`clear-selection-diagnostic.log`; test-owner task
`25c2948e-44cf-4b1e-9185-28275949bed8`. PAGE-TEST ownership released; new file
SHA-256 `ba8bab0423e6e311bee806980835123a73f344b6aba549f9d0e36d5bfa3233d7`.
The fresh corrective owner returned **only** `presentation.rs` (+16/−5, six
hunks), preserving B/D1: retained Inspector context no longer admits Layout
commands or reselects objects after canvas deselection. Parent read all six
baseline/current hunks. All **five** unchanged scope cycles then passed, reaching
a separate Close cleanup RED twice: the test host mounted Editor unconditionally,
but `application/src/session.rs:2560–2585` deliberately retains accepted state
while setting lifecycle Closed. The coordinator independently reproduced that
precise timeout, then corrected only the existing test host: read its existing
version signal and conditionally unmount Editor at Closed before restoring pane
preferences. No scope assertion, production lifecycle, API or selection owner
changed. Fixture-only delta: +7/−1; test-file SHA after correction
`2ccfb5756df63d84bd5ddf5dd0fbc361838a02eeb5dba0d4ba092d76e7894746`.
Parent read the exact new hunks before checks. All **23** affected browser checks
passed: six field stories including the full five-scope/cleanup story, relation,
spacing and focus coverage. Formatting, WASM Clippy and whitespace also passed.
RED/GREEN logs: `.evidence/011-round2-D0-cleanup/{baseline-real-red,green-affected}.log`.
The earlier `baseline-red.log` is a helper-argument invocation failure, not a
semantic RED. Native Gortex mutation succeeded on the fifth spaced attempt after
four `view_building` responses; no CLI/reset/fallback was used. Host graph coverage
is unmapped and contract risk remains WARN/high lower-bound; actual mounted proof
above is not replaced by the graph.

### D0 fresh combined review — 2026-11-09

Standards **PASS** / Spec **PASS**; no required or optional findings. One fresh
Pi Sol medium reviewer read all **71/71** baseline/current hunks, unread **NONE**:
`layout_toolbar.rs` 36, `layout-toolbar.css` 6, `presentation.rs` 10 and
`layout_field_preview_tests.rs` 19. BASE
`bb70ba0df640521291720cfe7a86ac9531e7d773` → unstaged working tree; HEAD stayed
`ec26e46d1fca0e38f5f690d368d73edf7704dc3f`. B/D1 page/test hunks were read for
preservation; their prior bounded reviews stand. R1 and foreign changes are
excluded. The [005 guard mapping](005-restore-scope-actions.md#retired-guard-mapping)
records every replaced/retained popup, guidance, selection and focus guard.
All four identical-shape ETags returned `not_modified`; additional physical reads
confirmed full-file pins:

| File | SHA-256 |
| --- | --- |
| `layout_toolbar.rs` | `809032f972d6226261ae19388da1e32afe10fa8406a2cb8d97e30fc2e158bf41` |
| `layout-toolbar.css` | `95a0c544a28a7fe42631e6ccd7db27870d98c10013e60655761d95d98278c768` |
| `presentation.rs` | `afc6cfc412ad9ab5fa17e0abc8c27dcd4cf6d178682f4806b4300cc293992ad1` |
| `layout_field_preview_tests.rs` | `2ccfb5756df63d84bd5ddf5dd0fbc361838a02eeb5dba0d4ba092d76e7894746` |

New review work: mechanical inventory, endpoint checks, exact source review and
physical pins only; no tests/builds/mutations or tracked edits. Actual RED/GREEN,
23 affected browser checks, 22 production-selection checks, packaged transfers and
format/Clippy receipts above were reused. The reviewer did not view captures or
rerun browser automation; empty-log exit statuses rely on the coordinator's
recorded results. Synthetic DOM input over real production behavior does not
prove trusted input or optimized/full-gate behavior, and sensitivity is not
established by inspection. All twelve structural heuristics and five quality
axes had no actionable findings. Task
`48e458af-8d1f-4e4f-9e5b-a11be510d3b0`; review hold **released**.
Acceptance readiness **BLOCKED**: DESIGN update, coordinator sensitivity
(including separate removal of the empty-selection filter and handler return),
integrated/full/strict-CSS gates, optimized replacement verification and human
acceptance. No aggregate/candidate approval; the 445-path PARTIAL cap remains.
C1 stays unapproved and unwired.

E's read-only diagnosis reproduced the exact R1 footer twice in fresh isolated
sessions: battery X −86 → 0 → −86, then Update preview, retaining instance
`physical-primary-22`; populated 3D, exact-ready title and enabled Export geometry
coexisted with the raw global blocker. Command:
`python3 /home/chris/.cache/tmp/011-r1-E-loop.py --fresh`, exit 1 in approximately
34 seconds; `.evidence/011-round2-E/fresh-repeat-red.log`. Timing/owner steps were
minimized, but the supplied geometry fixture is not claimed irreducible. Parent
read the RED and capture, and the real Runtime settlement/report path: rejected
or executor-failed unleased outcomes call `report`, whereas completion only
notifies views. Fresh E implementation receives **only** `runtime.rs` for private
provenance and retirement, with no new lifecycle authority or public/test API.
Proof extension: real generation rejection → admitted current success/reusable
exact result → obsolete generation-owned global status retires; an intervening
unrelated report (including identical text), save failure or current block must
survive stale/unrelated completion. Use the existing Runtime lifecycle seam plus
the original packaged footer loop; a `case_generation_title` helper test alone
is insufficient. E returned only `runtime.rs` (+335/−4), private bounded
provenance carried by Session-admitted generation settlements and retirement at
current success or canonical exact-result rebind. Five lifecycle tests extend the
existing Runtime seam: rejection/rebind, real Core preparation plus controlled
worker success, unrelated identical-text reports, stale completion/current block,
and failed-save/retry. Producer reports 62 Runtime browser / 138 native checks,
native/WASM Clippy, rustfmt and whitespace GREEN. Parent read the new lifecycle
stories, current production hunks, `red-rebind.log`, the 62-test final receipt and
`original-public-loop.log`. Original public reproduction now passes: same
`physical-primary-22`, populated exact geometry, enabled Export geometry and no
stale global footer. The intermediate repaired-before-update stage also has no
stale footer, but Export remains disabled until the explicit update. Pre-fix
Runtime failure: `.evidence/011-round2-E-fix/red-rebind.log`; all final logs are
in that directory. Controlled-worker geometry is synthetic; the public loop
uses a FAST/unoptimized package with existing providers, not an optimized/full
gate. E ownership is released. Fresh single combined review returned Standards
**PASS**, Spec **PASS**, no required/optional findings and **no unread hunks**.
Only `runtime.rs` was reviewed against base `bb70ba0d` and HEAD `ec0a6165`:
base→current after137→138, after153→155, after162→165, after1122→1126–1164,
after2207→2250–2252, after2241→2287–2293, after2409→2462–2466,
2424→2481–2493, after2698→2768, 2702–2703→2772, after6677→6747,
6697→6767–6772, after6715→6791–6806, after6716→6808, after9934→10027–10265.
Identical physical reads at 20:39:46Z / 20:40:51Z pinned full SHA
`198d30f47d209c0d79f67d137e5c2eb4157d7a40536c16c0f2c5e71e48c41bc5`
(467,335 bytes / 11,523 lines; ETag `ef8d33e0ec46d5e9`); all seven changed-source
windows separately returned `not_modified`. New review verification was scoped
whitespace and mechanical inventory; 62 browser / 138 native / Clippy / formatting
and actual public RED/GREEN receipts were read and reused, not rerun. No behavioral
reruns, mutations or full gate were executed by the reviewer. Existing admission,
observation/export leases and notification behavior were reviewed as preserved;
all twelve structural heuristics and five quality axes had no actionable finding.
Coordinator sensitivity remains pending: remove provenance, independently remove
success/rebind retirement, indiscriminately clear unrelated identical-text status.
Integrated/optimized/HITL gates and the legacy aggregate PARTIAL cap remain.

A separate process advanced HEAD to `ec0a61652c688a02b25671c4b2b80fa7b51eeddc`
with only the two `docs/design/workbench-redesign/option-a/` mockup documents.
Those unrelated committed changes are preserved and excluded from recovery
source review; a mechanical committed diff confirmed no recovery source changed.
A later foreign commit advanced HEAD to
`ec26e46d1fca0e38f5f690d368d73edf7704dc3f`, adding only
`docs/design/workbench-redesign/tool-panels/{README.md,template.html}` (486 lines).
Mechanical committed inventory confirmed recovery source again unchanged; these
design documents are preserved and excluded. Fixed review base
`bb70ba0df640521291720cfe7a86ac9531e7d773` remains valid. Neither foreign HEAD
movement is a coordinator commit, source-review invalidation or push.

Each completed slice gets a fresh independent combined Spec + Standards review
through a new Pi Sol medium delegate, then coordinator full-hunk read and
independent rejection/sensitivity checks. The coordinator reruns the full gate,
strict CSS, optimized build and affected Theme/Appearance/viewport checks before
promoting a replacement candidate. Keep the current 4173 candidate intact until
then. **011 remains awaiting-decision; parent 001 open; no push authorized.**

### Coordinator D0/B sensitivity — 2026-11-09

D0: the unchanged real Editor story independently killed both page-guard mutants:
removing the empty-selection toolbar filter failed Matrix clear→guidance;
removing the scope-handler return failed the next scope's initial guidance
assertion. Both built, neither runner timed out, and reviewed page bytes restored
exactly. Baseline/restored five-scope GREEN and canonical results:
`.evidence/011-round2-D0/coordinator-page-sensitivity/`. Remaining toolbar guards
are not claimed sensitive yet.

B: the first five canonical mutations killed force-One, unmirrored-source and
zero-40-mm-gap at their existing count/orientation/gap assertions. Fit-from-only-
one-half and frozen provider topology **survived**; original receipts remain in
`.evidence/011-round2-B/coordinator-sensitivity/`. Their six-test restored run was
interrupted before a verdict (exit 137); process absence and an unchanged native
ETag were verified before the resumed run passed all six. Only the existing
`new_matrix_preview_case` story then gained **72 lines**, no production edits,
new mount, helper, API or owner: the public Columns End input exercises 6×18
bounds outside the form and view controls; the existing `replace_document`
helper drives real Session/Core accepted One↔Split edits within the same session,
then both canvas and thumbnail must follow the newly accepted topology. This is
accepted-state integration, not a fabricated projection; it does not test the
physical-setup checkbox's pending feedback or approve C1's accessor.
The added proof passed unchanged production: a **proof gap**, not a product RED.
Native test-file SHA after mutation 179/reindex 204:
`fa88ad2b32a7014126344c55ceabff8cb6286f3472105e927b312d88a72dd6ec`.
This supersedes only the historical field-test pin in D0's completed review.

The two survivors were rerun against the stronger story and both **killed** at
the new maximum-width bounds and accepted-topology assertions; zero compile
failures, exact production restoration, six-test baseline GREEN. Canonical logs:
`.evidence/011-round2-B/coordinator-strengthened-sensitivity/`. The three original
killed mutants' unchanged oracles are reused, not described as rerun on the new
fixture. A subsequent **23 affected browser checks**, formatting, WASM Clippy and
whitespace passed: `.evidence/011-round2-B/proof-strengthening/`. Failure cascades
after each primary WASM assertion are not independent guardrail proofs. Native
coverage remains unmapped and the helper's contract risk WARN/high lower-bound;
actual behavioral receipts supply the proof. Fresh combined test-delta review returned Standards PASS / Spec PASS (task 3ffba…, read-only reuse of the receipts above; no .evidence report); B closed. Prior unchanged production reviews stand. DESIGN, R5/R6 and human gates remain open.

### C1 pending-topology accessor — 2026-11-10 (R4 approved)

User approved the pending accessor as a new phase; accepted projection stays accepted-only. Planner task 8a78d26c wrote the boundary: private `pending_topology` Signal + public `pending_topology()` (only new public item) on `PhysicalSetupMount`, set in `on_intent` beside `busy`, returned only while busy is Some (any id — seed yields to begin id), displayed as `pending.unwrap_or(projection)` in setup controls + the one `GuideHardwarePreview split:` argument. Proof rows P1a/P1b (packaged RED→GREEN), P2 (no flicker), P3/P4 (mounted accepted-only/retire), P5 (B-story rerun), T1 (before/after timings), each new oracle with one boundary mutation.

Builder task ec51fe58 (3 files, SHAs `de2beb8a…` controller / `beab38bd…` page / `128c8a23…` host): P1a/P1b 627.8 ms → **14.8/14.8 ms** pressed/preview under the 600 ms hold (×3 pre-runs 628.5/628.7/627.8; trace shows feedback at +15 ms with the hold tx open); P2 firstFeedback 15.1 ms, unpress [] / oneRect [], landing 658.4 ms accepted Split; P3/P4 GREEN; P5 rerun GREEN. T1 same-profile before→after: cold btnDom 76.2→23.4, btnRaf 133.7→57.9; warm n=16 btnDom median 65.8→13.25 (11.1/16.2), btnRaf median 68.15→26.35 (21.9/31.1); after-warm longtasks count=14 max=52 (Builder attributes to fresh-profile warmup; warm2 n=8 shows lt:[] at the same fast timings, supporting it). All 5 boundary mutants KILLED at intended assertions with exact restoration. PCB browser suite 66/66 GREEN; coordinator-verified filtered field-preview 6/6 GREEN with C1 present; full-page 19 FAILs match the no-C1 baseline modulo one flake swap each side (canvas story fails both ways — pre-existing full-suite instability, known-failures file empty); layout crate has zero references to the changed types (30 full-lib FAILs outside blast radius, non-canonical profile). Logs: `.evidence/011-round2-C1/`.

Process: Fable compliance review (task 9083bee1, MINOR DRIFT) found stash use (balanced, verified), one hand-applied Python mutant (residue excluded via SHAs), missing FD9 nesting (no concurrent builds, no collision) and partial change protocol (coordinator reran: tests 0/3 graph-unmapped as expected for wasm, guards none, contract WARN reviewed). No damage; rules made explicit in later briefs.

Review: first pass (task 04fb9585) Spec CHANGES REQUIRED on one valid BLOCKS (P3/P4 never observed rendered pressed state) + Standards PARTIAL on breadth. Follow-up Builder (task 69bd3378, test-only, same host file + same two tests) rendered the production controls and asserted pressed/retirement display; P4 mutant now kills AT the One-pressed assert, P3 establishes Split-pressed before its accepted-only checks. Delta re-review (task d38094b1) Spec PASS, no new findings; Standards no-violation with a preservation evidence limitation that the coordinator closed by byte-proof (current page minus the C1 hunk hashes exactly to pre-C1 reviewed SHA `afc6cfc4…`). C1 review-complete.

### R8 completion record — 2026-11-10 (close-out)

Status set to `resolved`; acceptance boxes checked per evidence + user waiver.

- Reviewed slices A/B/C1/C2/D0/D1/E with fresh combined Spec + Standards
  reviews, boundary/sensitivity kills, gate and promotion as recorded in this
  ticket.
- R5 source `83d81100`; R6 gate GREEN.
- Candidate promoted to `:4173` from `web/target/site-011-round2`.
- 24-cell human matrix defined on disk at `.evidence/011-round2-full-gate/matrix/`
  (all `human_result` pending).
- User approval 2026-11-10 ordering close-out, WITHOUT itemized per-cell
  verdicts — recorded honestly; no cell reviews fabricated. AC2–AC5 closed by
  this waiver, not by observed per-cell results. AC1/AC6/AC7/AC8 by direct
  evidence: source/artifact identified, approval recorded, unresolved items
  carried, records reconciled; parent 001 stays OPEN, no push/merge inferred.
- Follow-ups F1–F4 explicitly CARRIED (not closed):
  - F1: C2 remove-guard already RED-covered; extra D0 toolbar sensitivity as recorded.
  - F2: D1 real-390/card; reload→Reversible.
  - F3: 004/006 questioning sessions + the retrospective follow-ups.
  - F4: 445-path cap + D9 cause as recorded.
