# 02: Restore closure mounts in the shipped split keyboards

Status: resolved
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

- [x] Nine splits ship with closure mounts on both halves and every half exports. The tenth,
  Sofle v2, ships none: its gasket mount carries no closure mounts. The criterion said "all
  ten" and cannot be met for v2 without changing its mount; the exception is recorded in the
  Outcome as a decision for the user.
- [x] Baseline changes limited to the listed fields; no key moved (1e-9 comparison).
- [x] Per-keyboard closure and fit record in the Outcome.

## Verification

`build`, Core tests, `quick`. Keyboard 04 is still `claimed` on the same recipe,
baseline and fixtures: land this before 04's mounted/live checks and Standards/Spec
reviews so they review the final data once. This Outcome must correct 04's claim that
no part values changed. No ADR.

## Outcome

Status: resolved, with one exception and open fit findings (below). Commits on
`typed-core-edits/mechanical`: `aadded82b`, `c0bbfdb9c` and `7e6115e32` (a first attempt,
superseded), then `e5f253fab` (the recipe, tests and baseline as shipped) and the commit that
rewrites this Outcome.

### What the first attempt did, and why it was replaced

The first attempt only removed the `no-closures-*` edits. It reported acceptance item 1 as
unmet, and an independent check reproduced all of its claims:

- Core refused the export of the left half of all seven measured splits ("The linked closure
  mount at (x, y) lies outside this half's board outline").
- Sofle RGB and Choc had no right-half closure mounts.
- Sofle v2 had none on either half.

The cause is in the recipe order, not in the closure check:

1. `enable` on the second half is a no-op. A legacy second half already resolves from the shared
   source, so its stored closure list stays absent (`enable` returns early on
   `already_effective`).
2. The recipe then links the pair and edits the right half (placing its local controller-bay and
   TRRS openings). The first settings edit on a half with an absent list initializes it, from the
   right half's own outline.
3. `persist_edit` links every mount added by an edit on a paired half to the pair, authored by
   the edited half. The four proposals became linked closure mounts in right-half coordinates.
4. The pair map places them on the left half. The measured splits put the controller and TRRS at
   each PCB's right edge on both halves (`MEASURED_BAYS`), so the right outline is not the left's
   mirror image, and its outermost proposals map beyond the left outline. `pair_findings` reports
   them as a `Pcb` error, which blocks that half's export. This is designed behaviour: a linked
   feature is kept as placed and never relocated or unlinked.

Whether Core should link initialization proposals to the pair, and whether `enable` should
initialize a legacy second half, were Core policy questions. They were decided by
[ADR-0008](../../../adr/0008-linked-halves-stay-linked.md) and are implemented by
[06](06-paired-half-closure-proposals.md) (resolved). The "What changed (as shipped)"
recipe below is superseded: the explicit initialize step and its comment are gone, and the shipped
closure mounts are linked. Core was not changed by this ticket.

### What changed (as shipped)

> Superseded by [ADR-0008](../../../adr/0008-linked-halves-stay-linked.md) and
> [06](06-paired-half-closure-proposals.md): `link_pair` no longer initializes closures explicitly,
> the known-gap test described under "Verified" was flipped, and the closure mounts of the shipped
> pairs are linked. The rest of this list records the 02 state as it was shipped.

- `core/examples/demo_projects/mechanical.rs`: `link_pair` no longer sets empty closure
  collections. After `enable`, a half whose closure list is still absent is initialized with the
  explicit `initialize-closures` change, before the pair is linked. Each half then stores the
  mounts Core proposes from its own outline, and nothing is linked. A gasket pair (Sofle v2) is
  skipped: initialization does not apply to a gasket mount.
- `core/tests/fixtures/demo-projects-baseline.json`: re-recorded for the ten paired fixtures.
  Changed fields: `revision`, `hardware`, `parts` (`case-closure/*` added: 4 per half on every
  measured split, Sofle RGB and Choc; none on Sofle v2), `definitions` (the matching
  `assembly-closure/definition/*`) and `boards[].partIds` (only the new ids). The ten unpaired
  fixtures and `provenance` are unchanged. Sofle v2 changes `revision` and `hardware` only: its
  right half stores `mount: gasket` (was `tray`) and no closure list (was `[]`); its effective
  mount is the gasket on both halves.
- `core/tests/included_keyboards.rs`: four tests added. Every half exports to KiCad with its own
  closure holes (counts pinned); the shipped pairs link no closure mount; closure holes clear
  Sofle's authored mounting holes; no closure hole cuts a switch cutout; and a known-gap test
  that reproduces the order the recipe avoids (the right half's list removed, then one edit on
  it): four proposals join the pair, they land on their mirror, the right half exports and the
  left is refused on all seven measured splits. It fails if Core changes this.
- `core/tests/demo_projects.rs`: header comment records this re-record.

### Verified

Against the pre-02 baseline (`aadded82b~1`): the ten unpaired fixtures are byte-identical, and
all 1,176 existing part entries of the ten paired fixtures are still present and match within
1e-9, including every key matrix part (2,484 entries across the 20 fixtures). The only new parts are
`case-closure/*`; `boards[].partIds` gained nothing else. Revisions: measured splits 34 to 33,
Sofle RGB and Choc 24 to 23 (two `no-closures` edits removed, one `initialize-closures` added
on the right half), Sofle v2 25 to 23 (two removed, none added).

### Per-keyboard record

Holes are the `case-closure/*` parts on each board. Every hole comes from a closure mount stored
on that half. Gaps are edge to edge from the 6.6 mm closure hole (the boss diameter plus
2 x 0.3 mm clearance) to the nearest key's 14 mm switch cutout; the cutout size is an
assumption, not a measured value. All 20 halves export to KiCad.

| Keyboard | Holes left / right | Export left / right | Authored TH overlap | Closest switch gap left / right (mm) |
| --- | --- | --- | --- | --- |
| Corne | 4 / 4 | exported / exported | none (no TH parts) | 0.664 / 1.700 |
| Lily58 | 4 / 4 | exported / exported | none | 1.118 / 1.214 |
| KLOR | 4 / 4 | exported / exported | none | 0.295 / 1.324 |
| Ferris Sweep | 4 / 4 | exported / exported | none | 0.495 / 0.751 |
| Chocofi | 4 / 4 | exported / exported | none | 1.381 / 1.443 |
| TOTEM | 4 / 4 | exported / exported | none | 0.048 / 0.875 |
| Cantor | 4 / 4 | exported / exported | none | 0.031 / 1.608 |
| Sofle v2 (gasket) | 0 / 0 | exported / exported (7 TH each) | no closure holes | n/a |
| Sofle RGB | 4 / 4 | exported / exported (7 TH each) | closest closure to TH 19.74 / 21.94 mm | 1.066 / 0.009 |
| Sofle Choc | 4 / 4 | exported / exported (7 TH each) | closest closure to TH 20.52 / 23.23 mm | 0.109 / 1.028 |

### Fit findings and decisions for the user

Proposals are computed from the PCB outline before the recipes add the Case openings, and no
mount was moved to hide a conflict.

1. Sofle v2 has no closure mounts. A gasket mount does not take them. "Closure mounts on both
   halves" for v2 needs a mount decision: keep the gasket (none) or change to tray (closure
   mounts would then have to be proposed; the left half's list is an explicit empty one).
2. Closure holes inside the controller-bay opening (`opening-1`), measured from the hole edge to
   the opening rectangle, negative meaning overlap: Lily58 right -1.35 mm, KLOR left -1.42 and
   right -1.35, Ferris Sweep right -1.10, Chocofi right -0.54, Sofle RGB and Choc right -3.30
   (the mirrored linked bay). Sofle RGB and Choc left sit 0.10 mm from it. Corne, TOTEM and
   Cantor have none within 0.5 mm. Core raises no finding for a half's own closure mount
   against a Case opening. The first attempt's data already overlapped the bay for Lily58
   (right), KLOR (both halves) and Ferris Sweep (right).
3. Tight switch clearances: Sofle RGB right 0.009 mm, Cantor left 0.031 mm, TOTEM left
   0.048 mm, Sofle Choc left 0.109 mm, KLOR left 0.295 mm, Ferris Sweep left 0.495 mm. These
   come from the geometry above, not a measured fit. The test only requires a positive gap.
4. Closure positions are de-duplicated only against each other. No Sofle closure hole overlaps
   an authored mounting hole (the closest is 19.74 mm away).
5. Ticket [06](06-paired-half-closure-proposals.md) asks the user to choose how Core handles
   closure proposals on a paired half. Superseded by ADR-0008 and
   06: the known-gap test is flipped and the recipe no longer initializes explicitly.
6. The flipped-instance hole-collapse that ticket 01 suspected is still not exercised: no
   shipped pair is flipped. For all ten, holes per board equal the mounts stored on that board.

### Correction to keyboard ticket 04's Outcome

04 said no part value changed and that closure mounts were left empty. Both are false for nine
of the ten pairs and for Sofle v2's `hardware`. The correction is applied in
[04's Outcome](../../typed-core-edits/issues/04-refresh-included-keyboards.md) (per-keyboard
changes, revision table, Findings).

### Commands and results

- `python3 scripts/check.py repo`, `lint`: passed in the worktree after `e5f253fab`.
- `python3 scripts/check.py test`, non-nested checkout `/tmp/claude-1000/boardstudio-tce-verify`
  at `e5f253fab`: passed (92 s). All 51 test binaries ok, 1,277 tests, 0 failed; the Core
  integration binary ran 480 tests, including the ten included-keyboard tests and the baseline
  test.
- `python3 scripts/check.py build`, worktree, sandbox lifted for wasm-pack and the CAD container: passed (65 s). The
  regenerated fixtures in `web/assets/fixtures` (gitignored) show 4 closure holes on every half
  of the nine splits and none on Sofle v2.
- `python3 scripts/check.py typecheck`, worktree: passed (18 s).
- GitNexus: the index is stale (last indexed `4b372b2`) and does not cover the examples or these
  tests. `impact link_pair` resolved to an unrelated test helper in
  `core/tests/mechanical_integration.rs` (`UNKNOWN`, no callers); a text search shows the only
  callers of the recipe's `link_pair` are the three in `core/examples/demo_projects/mod.rs`.
  `detect-changes --scope all` reported 4 files, 1 symbol (the baseline test's `equivalent`
  helper, from a header comment), risk low, 0 processes. Stale, so not a clean check.

Not run: `browser` (no web or presentation source changed), `deny`, `quick`, `release` (never
run), mounted or live browser checks (they belong to keyboard 04), Standards and Spec reviews.
Nothing was pushed or merged.
