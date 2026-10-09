# 02: Restore closure mounts in the shipped split keyboards

Status: implemented (awaiting verification)
Type: task
Blocked by: 01
Map: [map.md](../map.md) · Related: [typed-core-edits 04](../../typed-core-edits/issues/04-refresh-included-keyboards.md)

## Request

Keyboard ticket 04's recipes set an empty closure collection after enabling each half
(`core/examples/demo_projects/mechanical.rs:~244-255`) because of the 01 defect, so the
ten shipped splits have no closure screws.

## What to do

- Remove the `no-closures-*` edits; keep Core's proposed closure mounts.
- Regenerate fixtures (`build`), then re-record
  `core/tests/fixtures/demo-projects-baseline.json` for the ten paired fixtures only
  (`revision`, `hardware`, plus the generated `case-closure/*` parts and
  `assembly-closure/definition/*` definitions). Unibody, review and archive fixtures do
  not change.
- For each keyboard, record the closure count, any overlap with authored mounting holes
  (Sofle has measured holes; closure holes are de-duplicated only against each other) and
  any new fit findings, since proposed positions can land on switches. Do not hide a
  conflict by moving mounts without a source fact.
- Extend `core/tests/included_keyboards.rs` to export each pair (KiCad) with the closure
  holes present.

## Acceptance

- [ ] All ten splits ship with closure mounts on both halves; exports succeed.
- [ ] Baseline changes limited to the listed fields; no key moved (1e-9 comparison).
- [ ] Per-keyboard closure and fit record in the Outcome.

## Verification

`build`, Core tests, `quick`. Keyboard 04 is still `claimed` on the same recipe,
baseline and fixtures: land this before 04's mounted/live checks and Standards/Spec
reviews so they review the final data once. This Outcome must correct 04's claim that
no part values changed. No ADR.

## Outcome

Status: implemented (awaiting verification). The ticket is not resolved: acceptance item 1
is not met (see below). Commits on `typed-core-edits/mechanical`: `aadded82b` (recipe and
baseline), `c0bbfdb9c` (export and closure tests), and the commit that records this Outcome.

### What changed

- `core/examples/demo_projects/mechanical.rs`: `link_pair` no longer applies the
  `no-closures-*` edits. Each half keeps the closure mounts Core proposes on enable.
- `core/tests/fixtures/demo-projects-baseline.json`: re-recorded for the ten paired fixtures
  only. Changed fields: `revision`; `hardware`; `parts` (`case-closure/*` added: 11 per
  measured split, 4 on each of Sofle RGB and Choc, none on Sofle v2); `definitions`
  (the matching `assembly-closure/definition/*`); and `boards[].partIds`, which gained only
  the new `case-closure/*` ids. The ten unpaired fixtures are unchanged. `provenance` is
  unchanged; it predates the Rust recipes.
- `core/tests/included_keyboards.rs`: four tests added (export and pinned refusals per half,
  mirror placement, clearance from authored holes, clearance from switch cutouts).

Keys: all 2,484 existing part entries of the 20 fixtures are still present, matched by id,
and their poses match the old baseline within 1e-9. That includes 1,048 key matrix parts in
the ten paired fixtures. None moved.

### Acceptance

- [ ] All ten splits ship with closure mounts on both halves; exports succeed. **Not met.**
  Sofle v2 has no closure mounts (its mount is the gasket). Sofle RGB and Choc have none on
  the right. Core refuses the export of every measured split's left half. Removing the
  workaround alone does not give closure mounts on both halves.
- [x] Baseline changes limited to the listed fields, plus `boards[].partIds` (above). No key
  moved (1e-9).
- [x] Per-keyboard closure and fit record below.

### Per-keyboard record

Holes are the `case-closure/*` parts on each board, which is what Core projects. "Local" is a
closure mount stored on that half. "Linked" is a closure mount owned by the pair and
projected on both halves. Gaps are edge to edge, from a 6.6 mm closure hole (the boss
diameter plus 2 x 0.3 mm clearance) to the nearest key's 14 mm switch cutout. The cutout size
is an assumption, not a measured value.

| Keyboard | Left holes (local + linked) | Right holes (linked) | Left export | Right export | Authored TH overlap | Closest switch gap, left / right (mm) |
| --- | --- | --- | --- | --- | --- | --- |
| Corne | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none (no TH parts) | 0.66 / 1.70 |
| Lily58 | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 1.12 / 1.37 |
| KLOR | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 0.30 / 0.36 |
| Ferris Sweep | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 0.49 / 1.42 |
| Chocofi | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 1.38 / 1.46 |
| TOTEM | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 0.05 / 1.54 |
| Cantor | 7 (3 + 4) | 4 (0 + 4) | refused | exported, 4 holes | none | 0.03 / 1.61 |
| Sofle v2 (gasket) | 0 | 0 | exported, 7 TH | exported, 7 TH | none (no closure holes) | n/a |
| Sofle RGB | 4 (4 + 0) | 0 | exported, 4 holes + 7 TH | exported, 7 TH only | closest closure to TH 19.74 mm | 1.07 / n/a |
| Sofle Choc | 4 (4 + 0) | 0 | exported, 4 holes + 7 TH | exported, 7 TH only | closest closure to TH 20.52 mm | 0.11 / n/a |

The refusal text is "The linked closure mount at (x, y) lies outside this half's board outline."
The left refusal positions (x, y) are: Corne (-23.66, -47.61), Lily58 (-27.95, -34.91), KLOR
(-23.11, -34.88), Ferris Sweep (-27.95, -34.03), Chocofi (-22.25, -37.99), TOTEM (-21.77,
-37.93), Cantor (-23.72, -37.70). Core names only the first mount it finds outside each
outline, so others may also lie outside.

### Hole count per shipped pair (ticket 01's note)

No shipped pair is flipped. For all ten, holes per board equal local mounts plus two per
linked mount; the test asserts this, and no holes collapsed. The flipped-instance collapse
that ticket 01 suspected is not exercised by any shipped pair, so it stays unconfirmed.

### Why the halves differ

A temporary trace of each recipe edit showed (instrumentation removed, not committed):

- Enabling the left half stores its four proposed closure mounts.
- Enabling the right half is a no-op. `enable` returns early when an instance's effective
  inputs already resolve. A legacy second half resolves from the shared source, so its
  stored configuration stays null and no proposals are made for it.
- Linking adds the right half's stored configuration without a closure list. No closure
  correspondences are passed, so the left keeps its four.
- The TRRS opening edit on the left, after linking, re-initialises closure proposals. The
  left drops to three stored mounts, four closure mounts join the pair as linked features
  (in right-half coordinates), and the right half's list becomes empty.
- Sofle v2: `set-mount gasket` clears the left half's four proposals. The gasket mount
  carries its own fasteners and no closure mounts.
- Sofle RGB and Choc: no later edit re-initialises the right half, so it has none.

### Fit findings and decisions for the user

1. Export refusal on all seven measured left halves. The four linked mounts are right-half
   proposals mapped through the pair. The first one Core reports lies 22 to 28 mm beyond the
   left outline, so the right outline extends further than the left half's mirror image.
   Options: keep these proposals local to the right half, reposition them from a source
   fact, or change the outline. None is chosen here. Moving the mounts to pass export would
   hide the conflict.
2. The second-half enable no-op (Core) leaves Sofle RGB and Choc's right halves with no
   closure mounts. Fixing it is Core work, not recipe work.
3. Sofle v2 keeps the gasket mount, which carries no closure mounts. "Closure mounts on both
   halves" for v2 needs a mount decision: keep the gasket and accept none, or change to tray.
4. Tight switch clearances on left halves: cantor 0.03 mm, TOTEM 0.05 mm, Sofle Choc 0.11 mm,
   KLOR 0.30 mm, Corne 0.66 mm. These come from the geometry check above, not a measured fit.
5. Sofle v2's right half now stores `mount: gasket` (was `tray`) and no closure list (was
   `[]`). Its effective mount is still gasket on both halves, which the existing test checks.
   Sofle RGB and Choc's right halves store no closure list at all, so Core will initialise
   proposals on their next edit.

### Correction to keyboard ticket 04's Outcome

04 said no part value changed and that closure mounts were left empty. Both are now false for
the ten pairs. `parts`, `definitions` and `boards[].partIds` gained closure entries;
`hardware` changed (closure mounts, pair features, and Sofle v2's right-half mount); `revision`
fell (34 to 32 on the measured splits, 25 to 23 on Sofle v2, 24 to 22 on Sofle RGB and Choc).
Keys, matrices, nets, outlines and assets are unchanged, so 04's claim that no key moved still
holds.

### Commands and results

- `rustfmt --edition 2024 --check` on the two changed Rust files: clean after one
  formatting fix to the test file.
- Worktree (nested checkout, debug), `cargo test --manifest-path core/Cargo.toml --locked
  --test integration -- included_keyboards demo_projects`: 10 and 1 passed. Native
  `test` must run in the non-nested checkout, so this is a pre-check only.
- Non-nested checkout `/tmp/claude-1000/boardstudio-tce-verify`, detached at `c0bbfdb9c`:
  - `python3 scripts/check.py repo`: passed.
  - `python3 scripts/check.py lint`: passed (`cargo fmt --all --check` and
    `cargo clippy -D warnings`).
  - `python3 scripts/check.py build`: **failed in that checkout.** Its `cad/.cache` is a
    symlink to the worktree's cache, which the CAD container cannot see, so
    `cad/scripts/build-cadrum-wasm.py` tried to download prebuilt OCCT and the sandbox has no
    network. A real copy of the cache would not fit: `/tmp` is a 16 GB tmpfs with about
    600 MB free. I removed the partial copy and restored the symlink.
  - `python3 scripts/check.py test`: **passed** (`TEST_RC=0`, 90 s). Every cargo test
    binary reported `ok`; the Core integration binary ran 480 tests, 0 failed, including the
    10 included-keyboard tests and the baseline test. The CAD step inside `test` also passed.
- Worktree (real CAD cache), `python3 scripts/check.py build` at `c0bbfdb9c`: **passed**
  (`BUILD_RC=0`, 87 s). Run here because of the failure above. The 20 fixtures it wrote to
  `web/assets/fixtures` (gitignored) agree with the committed baseline, 20 of 20.
- GitNexus: the index is stale (18 commits behind HEAD, and it does not cover the examples or
  these tests). `impact link_pair` resolved to an unrelated test helper with no callers
  (`UNKNOWN`). A text search shows the only callers are the three in
  `core/examples/demo_projects/mod.rs`. `detect-changes --scope all` reported "Diff touched 3
  file(s) but no indexed symbols overlap those hunks". That is not a clean check.

Not run: `typecheck`, `browser`, `deny`, `quick`, mounted or live browser checks (no
presentation source changed; the live checks belong to keyboard 04), Standards and Spec
reviews, and `release` (never run). Nothing has been pushed or merged.
