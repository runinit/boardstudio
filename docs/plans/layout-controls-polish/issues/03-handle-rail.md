# 03: Handle rail for the existing tools

Status: resolved
Type: build
Blocked by: none (decision in [01](01-layout-controls-iconography-prototype.md))
Map: [map.md](../map.md)
Prototype: branch `prototype/layout-controls-iconography` (e91865f43), `?variant=D`; its spec sheet holds the exact values.
Review: presentation only; RED/GREEN evidence and a diff read, no combined review.

## What to build

The canvas drag handles that ship today become labelled tokens in one rail above the selection.
Each drag still edits the same field through the same op. The handles are rendered in
`LayoutTransformToolOverlay`
(`web/crates/layout/src/objects/layout_transform_toolbar.rs`, the `m1-transform-handle` /
`m1-spread-handle` / `m1-stagger-handle` / `m1-splay-*` / `m1-matrix-*-handle` nodes), and
`transform_tools` (`web/src/presentation/layout_canvas.rs`) picks them.

- **Tokens:** 18 × 18 px, radius 4, 12 px glyph, a 20 × 24 px hit area, gathered in one rail
  22 px tall (radius 6, 2 px padding, panel fill, 1 px `rule-strong`, soft shadow). The rail sits
  10 px above the selection's top and is centred on it, at screen size at every zoom.
- **States:** at rest the glyph is muted. Hover gives a `selection-surface` fill, a
  `selection-ink` glyph, amber on the keys that would move, and a faint axis guide. Active gives
  an accent fill and a white glyph, the other tokens drop to 35% opacity, and the axis guide
  becomes 1 px accent dashed.
- **Covered here:** Column Spread, Stagger and Splay; Row Offset (`PlacementParameter::RowOffset`) as it ships today;
  Matrix Move and Rotate. Origin stays a point on the canvas: a 6 px ring with a 12 px cross and
  a 24 px hit area. Ctrl-drag quick Offset, keyboard nudge (`aria-label` lookup) and
  hidden-handles behaviour keep working.
- New handles (Row Bend, Key Shift/Bend) are [05](05-row-and-key-shift-bend-handles.md). Parts
  are [07](07-part-rail.md), and the Anchor token is [08](08-canvas-anchor-pick.md).

## Seam and first RED / GREEN slice

Seam: a pure rail layout in `web/crates/layout`. It takes the selection's screen bounds and the
ordered tool list and returns token rectangles plus the rail rectangle. It is native-tested, and
the overlay only draws what it returns.

RED: a native test that three column tools over a known bounding box produce a rail
`3·18 + 2·2 + 4` px wide, centred on the box, with its bottom edge 10 px above the box top. It
also checks that the layout is the same at two zooms in screen space.

## Acceptance

- [x] Native rail-layout tests: width, centring, offset, zoom invariance and an empty tool list.
- [x] The existing mounted handle tests in `layout_transform_toolbar.rs` (the `aria-label`
  queries for "Drag to splay", "Drag to move matrix" and the others) still pass against the
  tokens. They may change only in their geometry assertions, and each such change is listed.
- [x] A mounted test covers hover (selection-surface, amber movers), active (accent, others at
  35%) and leave, with no document revision.
- [ ] Live-app drag pass for each covered tool, including Ctrl-drag quick Offset and the
  keyboard nudge. (Partial: see Answer. Sanctioned smoke blocked pre-existing at Part drag;
  supplemental rail probe green through save for Stagger, Ctrl Offset and Matrix Move;
  Splay/Spread/Rotate/nudge covered by mounted drags only — the smoke has no sections for them.)
- [x] DESIGN.md's "Stagger, Offset, Splay, and Origin activate explicit canvas tools" paragraph
  describes the rail.
- [ ] `python3 scripts/check.py` passes (repo, tooling, lint, build, test, browser), including
  the drag smoke. `check.py test --update-native-baseline` only for deliberate test renames.
  (Partial: see Answer. repo/tooling/build/test/typecheck green; lint + browser blocked
  pre-existing. Baseline JSON updated for the deliberate module-path rename.)

## Carried point

- The prototype is desktop only. DESIGN.md asks for 44 px touch targets in the compact
  layout. Whether tokens grow their hit area at ≤980 px is undecided; ask before building a
  compact variant, and by default leave compact sizing to a follow-up.

## Answer

Implemented on branch `docs/ergogen-layout-comparison` (prototype ref
`prototype/layout-controls-iconography` `e91865f43`, variant D).

What changed (ticket-03 files only):

- `web/crates/layout/src/handle_rail.rs` (new): the pure rail seam. Takes
  the selection's screen geometry plus tool count and returns the rail
  rectangle plus one 18 px token rectangle per peer; `rail_world` is the
  world-units wrapper the overlay draws from. Constants match variant D
  verbatim (token 18/r4, gap 2, pad 2, rail 22/r6, 10 px above, hit 20x24,
  Origin ring r3/cross 12/hit 24, dim 0.35). Registered from
  `matrix_transform_operation.rs` via `#[path]` so the crate root — owned
  this round by the concurrent ticket-02 edit — stays untouched. That move
  renamed the native test paths, so `scripts/check-native-tests-baseline.json`
  carries the same six names under `matrix_transform_operation::handle_rail`
  (sorted position; deliberate rename, no other entries touched).
- `web/crates/layout/src/objects/layout_transform_toolbar.rs`: every
  transform overlay registers its tool in a per-svg mount set (keyed by the
  shared svg handle, canonical peer order Spread/Stagger/Offset/Splay/
  Move/Rotate, Origin excluded) and draws its token from the shared layout:
  rail backdrop (first peer only, panel fill, rule-strong 1 px, drop shadow,
  pointer-events none), 18 px token with 20x24 hit area and prototype glyph
  paths, 12 px Origin point (ring + 12 px cross + 24 px hit) at the splay
  origin for Splay/Origin. Hover/focus fills selection-surface with
  selection-ink glyph, previews the movers in amber through the existing
  `FieldPreview` context (transient view state, never written) and shows a
  faint axis guide; a live drag turns the token accent with on-accent glyph
  and a 1 px accent-dashed guide while peers dim to 35% (shared active-tool
  slot read on every Editor re-render, which each preview submit triggers).
  Leave/focus-out/Escape/drop/cancel clears preview and active. Gestures,
  drag math, Ctrl quick-offset, keyboard nudge, aria-labels/titles and the
  hidden-handles rule are unchanged. All chrome is inline SVG
  presentation attributes (no stylesheet or mount-site edits, keeping the
  ticket-02-owned files untouched).
- `DESIGN.md`: canvas paragraph records the 18 px / 22 px / 10 px rail,
  the Origin point, and the rest/hover/active states.
- New mounted test `rail_tokens_share_one_screen_size_row_with_hover_and_dimming`
  (two overlays, one svg): 42 px backdrop for two tokens, shared row,
  rail-centred, canonical order, 18 px boxes with hit areas; hover asserts
  selection-surface + non-empty amber preview and leave asserts cleared
  preview, muted token and unchanged revision; drag asserts accent token,
  35% peer, full opacity on self, and undim after landing.

Geometry assertion changes: none to existing tests (all mounted handle
tests pass unmodified). Behaviour deltas vs the old chrome: splay axis
guides now show on hover/active only (they were always on); the Origin
point is restyled to the spec ring/cross/hit; handle art moved from
geometry points into the rail. Judgement calls: active glyph uses
`--wb-on-accent` (white in light, dark-ink in dark) instead of literal
`#fff` for themed contrast; dimming appears with the first preview
submit (pointerdown-to-first-move shows only the active token).

Carried open point: compact-screen 44 px touch targets (≤980 px) remain
UNDECIDED and unimplemented — desktop behaviour only, hit areas stay
20x24 per the prototype.

Evidence:

- RED then GREEN: `cargo test -p boardstudio-web-layout --lib
  matrix_transform_operation::handle_rail` — 1 failure observed (my own
  test arithmetic: absolute world coords converted to px without a camera
  origin), corrected to relative offsets, then 6/6 green.
- Mounted: `wasm-pack test --headless --chrome web/crates/layout
  --locked --lib -- --skip setup_guide::` → 147 pass incl. the new rail
  test and every pre-existing `tool_tests` drag/nudge/escape test. 22
  failures, all in
  `matrix_transform_inspector::mounted_tests` (detach/aggregate/pitch
  badge), reproduce byte-identical on the stashed baseline — pre-existing,
  unrelated (detach/anchor territory). Chrome here needs a matched
  binary + `--no-sandbox` (`/tmp` shim + temporary `webdriver.json`
  flags, reverted afterwards; system chromium 156 exits on launch).
- Live: sanctioned `run-drag-browser.py` fails pre-existing at 'Part
  drag: Undo restored the complete saved document', byte-identical with
  and without this diff (baseline rebuild + rerun). Supplemental
  rail-only probe (`/tmp/drag-rail-probe.cjs`, same selectors/asserts,
  pre-transform drags and collision fixtures cut): Stagger, Ctrl Offset
  and Matrix move each drive trusted-mouse drag → visible preview →
  durable save green; each stops at the same pre-existing post-Undo
  full-document signature compare. Splay, Spread, Rotate and nudge have
  no smoke sections and are covered by the mounted drag tests.
- Gates: `check.py` repo/tooling/build/test/typecheck green (test:
  "2367 baselined test(s) executed"). Lint: `cargo fmt -p
  boardstudio-web-layout --check` clean except the sibling-committed
  `lib.rs` ordering (pre-existing at HEAD, reverted out of this diff);
  `cargo clippy -p boardstudio-web-layout` green natively and for
  wasm32 with `-D warnings`. Full-workspace `fmt --all` and the
  `browser` step cannot pass on this tree for pre-existing reasons
  (above + system-chromium launch).

Two-axis review (inline): Spec — 18/22/2/10 px geometry, origin point,
rest/hover/active/dim/leave, kept behaviours (hidden handles, Ctrl
quick offset, nudge, aria lookups, no labels/re-anchor/Shift-Bend/
popup/part-rail/pick), compact point carried open. Standards — exact
ticket-03 file list, no cross-file edits (registry + inline styles
instead of `presentation.rs`/`m1.css`), verbatim prototype numbers,
no dead code (`stagger_arrow_degrees` removed with its last use),
`cargo fmt` + Clippy `-D warnings` green on both targets.

Map pointer: not added (`docs/plans/cad-backend-replacement/map.md` is
not a ticket-03 file) — left for the orchestrator.
