# 04: Redo all included keyboards using the settled editing model

Status: wontfix
Type: task
Blocked by: 02, [matrix integrated verification](../../matrix-layout-consistency/issues/11-integrated-verification.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

The user requested this follow-up during mechanical Q8 on 2026-10-08: once the
matrix/mechanical behavior is settled, redo all included keyboards. This work
starts after the new model is implemented and verified. It does not opt existing
user projects into new links automatically.

At `f65ed01a6`, the generator/tests enumerate 20 included project outputs: measured
keyboard recipes, Sofle v2/RGB/Choc variants, the archived original REVIUNG41 and
the VIK review project. Re-inventory at execution time. Existing split recipes use
separate board IDs and do not cover the reused-PCB flipped-instance case; add a
bounded demonstration/verification for that model rather than claim it is covered
by rebuilding the current split examples. Most measured keyboard recipes currently
have no mechanical configuration, so this follow-up includes configuration work.

## What to do

- Inventory every shipped keyboard and review project from the Library catalogue
  and the native `prepare_demo_projects` recipe/output list. Cover all included
  keyboards, not only a selected demo. Record the inventory in the Outcome.
- Rework their reproducible recipes under `core/examples/demo_projects/` and
  measured inputs under `content/layouts/` where needed. Generate projects through
  the public Core editing/resolution/archive interfaces and the existing build
  route; generated archives are not the source of truth.
- Adopt the settled matrix placement meanings and explicit mechanical mirror pairs
  where applicable. Name and retain deliberate half-specific hardware/features;
  preserve measured layouts, legitimate wiring, authored geometry and source facts.
  Use independent configuration for keyboards that are not mirrored pairs.
- Regenerate all shipped project outputs, metadata and any dependent previews.
  Review intentional geometry changes per keyboard rather than silently accepting
  new auto-generated placements as measured/source evidence.
- Verify editable opening, matrix/paired mechanical edits, local exceptions,
  Undo/Redo and portable save/reopen. Check the affected PCB/case/plate output and
  fit findings. Retain unresolved hardware qualification as findings; rebuilding a
  keyboard does not establish physical fit or electrical qualification.

## Acceptance

- [ ] The inventory accounts for every shipped keyboard/review project.
- [ ] Regenerated outputs use the implemented matrix/mechanical model and its
  intended link/local states, with a per-keyboard record of intentional changes.
- [ ] Recipes reproduce the shipped projects; exported archives reopen correctly.
- [ ] Applicable mounted browser and live product checks pass, including paired
  editing and deliberate hardware asymmetry.
- [ ] The production build regenerates the full included-keyboard set successfully.
- [ ] Source/data changes receive the required Standards and Spec reviews.

## Verification

Run affected `python3 scripts/check.py` steps while iterating. Before accepting
the complete set or opening a PR, run the full check, including browser and build,
plus the per-keyboard app review above. Preserve result/source pins in the Outcome.

## Outcome

> Superseded in part by [ADR-0008](../../../adr/0008-linked-halves-stay-linked.md) and
> generator-parameters [06](../../generator-parameters/issues/06-paired-half-closure-proposals.md)
> (resolved). For the seven measured splits the second half's controller
> bay, reset switch and TRRS jack now sit at the reflection of the first half's, so the bay and TRRS
> openings are linked rather than local on the right, and every closure mount is linked to the pair
> from the left half. The "Per-keyboard intentional changes", the openings column of the table and
> "Findings kept open" below describe the state before that change. Keyboard 04 itself stays claimed.

Status stays `claimed`: the data, recipes and Core tests are done and checked, but acceptance
is not complete (see "Not done" below). Commits on `typed-core-edits/mechanical`: `0e6f1a1ad`
(recipes, baseline, archive reopen check, README) and `c4673db3b` (Core tests).

### Inventory (20 shipped projects, matches the Library cards and `demo-projects-baseline.json`)

| Group | Projects | Result |
| --- | --- | --- |
| Symmetric splits, measured recipes | Corne, Ferris Sweep, Chocofi, TOTEM, Cantor | Paired; bay and TRRS local on the right |
| Splits authored from the right half | Lily58, KLOR | Paired with the same map (a reflection is its own inverse); bay and TRRS local on the right |
| Sofle | v2, RGB, Choc | Paired; bay and TRRS linked; v2 carries the gasket as the pair's construction |
| Unibody | REVIUNG41 (measured), GH60, Discipline, Mysterium, Voyager97, Voyager104, Plaid, Lumberjack | Untouched |
| Prebuilt archive | REVIUNG41 original (`content/archives/`) | Untouched (copied, not a recipe) |
| Review | VIK module review | Untouched (a module review with no halves) |

### Per-keyboard intentional changes

Every paired project: both instances `constructionOwnership: pair` (was `legacy`) with the
explicit map `second = (offset - first.x, first.y)`, rotation 0; one shared construction taken
from the left half with Core's defaults (tray mount, 1.5 mm printed plate, 2 mm walls, 0.3 mm
clearance, the default part processes); two Case openings, `opening-1` (controller bay, 20 x 36 mm
around `U1`) and `opening-2` (TRRS jack around the jack part), each authored once by the left
half. Closure mounts: this commit left them empty, and generator-parameters
[ticket 02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md) then
restored them, so each half of the seven measured splits and of Sofle RGB and Choc now stores
the four mounts Core proposes from its own outline (Sofle v2's gasket mount carries none).
`revision` rises because each edit is one revision. No matrix, net, outline or asset value
changed, and no existing part moved (the baseline comparison at 1e-9 passes with the same
`matrices` and every pre-existing `parts` entry). Correction after 02: the
`case-closure/*` parts, the `assembly-closure/*` definitions and the boards' `partIds` for them
were added, and `hardware` carries the closure mounts; the first version of this paragraph said
no part or definition value changed.

| Keyboard | Map offset | Openings (left / right) | Revision |
| --- | --- | --- | --- |
| Corne | 348.125 | linked / local | 2 to 33 |
| Lily58 | 347.8 | linked / local | 2 to 33 |
| KLOR | 386.344 | linked / local | 2 to 33 |
| Ferris Sweep | 284.16 | linked / local | 2 to 33 |
| Chocofi | 296.6 | linked / local | 2 to 33 |
| TOTEM | 329.75 | linked / local | 2 to 33 |
| Cantor | 339.1 | linked / local | 2 to 33 |
| Sofle (v2) | 317.0 | linked / linked | 3 to 23 |
| Sofle RGB | 316.74 | linked / linked | 2 to 23 |
| Sofle Choc | 316.74 | linked / linked | 2 to 23 |

Deliberate half-specific hardware, named: on the seven measured splits the recipe puts the
controller bay, reset switch and TRRS jack at each PCB's right edge on both halves, so the right
half's bay is not the left half's reflection. Its two openings are made local on the right
(`make-feature-local`) and re-placed around the right half's own parts, instead of silently
mirroring a position the right PCB does not have. (The reset switch has no Case opening.) Sofle
places every part from one measured half, so its reflected parts, including the encoder, display
and mounting holes, match the map exactly and its openings stay linked; the recipe asserts this
while building. No Case openings were invented for the Sofle encoder or display: no source fact
gives their window size.

Sofle v2 only: its project-level legacy gasket (`m1-fixture-gasket`, applied to the left board
only; the right half had none) is gone. The pair's shared construction is now the settled
internal-gasket mount (`set-mount gasket`), so both halves resolve it. This is a deliberate
design change: the internal gasket defaults apply (4 supports, 0.1 mm travel against the legacy
0.3 mm), so the v2 gasket geometry is regenerated, not preserved, and needs a fit review. The
baseline now pins `mechanical: null` for it.

Reused-PCB demonstration (not a shipped fixture, so the Library and the 20-project count are
unchanged): `reused_pcb_project` builds one REVIUNG41 board with two physical copies, the second
`flipped: true`, paired with the zero map their opposite flips imply (the same rule the physical
setup uses). The controller bay is linked; the reset access exists only on the unflipped copy and
is `omitted` on the flipped one. Core tests read each copy's frame (the flipped one mirrors X and
walks the polygon the other way).

### Tests and baseline

- `core/tests/included_keyboards.rs` (6 tests, public requests plus the effective-input read):
  all ten pairs open and reflect every measured key with mirrored orientation (a mutation that
  moved the map by 1 mm fails it); reopening changes nothing; the per-keyboard link states; a
  paired value (plate thickness) edited from the right half changes both in one Undo step, with
  Undo/Redo; a linked opening edited from the right half moves the left by the reflected value;
  a local exception stays local both ways while the construction stays shared and Undo walks back
  to the opened project; the reused flipped PCB (mirrored frame, omitted opening, edit from the
  flipped copy, Undo, reopen).
- `core/tests/demo_projects.rs`: every exported archive now also reopens in a fresh engine with its
  pairs unchanged. Baseline re-recorded for `revision` and `hardware` of the ten paired
  fixtures and `mechanical` of Sofle v2; the other ten fixtures and all `matrices` and `assets`
  are untouched. Ticket 02 later re-recorded `parts`, `definitions` and the boards' `partIds`
  for the added closure holes (no existing part moved).
- The earlier agent draft was kept in approach but corrected: it stored prose notes in
  `parameters.mechanicalPair` (dropped; the pair itself is the record), left the Sofle v2 gasket as
  a project setting on one board that the paired halves do not read (now the pair's construction),
  and had a `Bay` model with no way to express an omitted opening or a flipped copy (now a
  `Counterpart` of linked, local or omitted, which the reused-PCB demonstration needs).

### Commands and results

- `python3 scripts/check.py repo`: passed.
- `python3 scripts/check.py lint`: passed (rustfmt and Clippy, warnings denied).
- `python3 scripts/check.py build`: passed in the worktree with the sandbox lifted for wasm-pack;
  regenerated all 20 fixtures with their archives (`web/assets/fixtures`, 20 entries, Corne at
  revision 34 with its pair; 33 after ticket 02).
- `python3 scripts/check.py test`: passed at `c4673db3b` in a non-nested validation worktree
  (`/tmp/claude-1000/boardstudio-tce-verify`, because `/tmp/boardstudio-tce-verify` is read-only
  from the sandbox); Core integration 474 passed (the 468 existing plus these 6), CAD 51 passed,
  4 ignored.
- Not run: `typecheck`, `browser`, `tooling`, `deny`; no web or presentation source changed.
  `release` was never run.

### Findings kept open

- Core defect (resolved by generator-parameters
  [01](../../generator-parameters/issues/01-closure-holes-via-generator-normalization.md) and
  [02](../../generator-parameters/issues/02-restore-closure-mounts-in-recipes.md)):
  `normalized_mounting_hole` wrote the generated closure hole's `hole_drill` and `hole_size`
  as text, which the footprint generator rejects, so this ticket's recipes first set an empty
  closure collection after enabling each half. 01 fixed the type; 02 removed the workaround.
  The shipped splits now carry closure screws, except Sofle v2 (gasket mount). Open fit
  findings (closure holes inside the controller bay, tight switch gaps) are in 02's Outcome.
- Rebuilding does not establish physical fit or electrical qualification. The regenerated Sofle v2
  gasket and every new bay opening need a fit review; the bay sizes (20 x 36 mm, jack 12 x 14 or
  14 x 12) are recipe values, not measurements.
- Superseded by ADR-0008 (generator-parameters 06): the right-half bay was local because the recipe
  placed it at the PCB's outer edge. It now sits at the reflection of the left half's bay, and the
  bay, reset and TRRS openings link.

### Not done (why Status is still `claimed`)

- Mounted browser tests and a live-product pass (open each paired fixture in the app, edit a paired
  value and a local exception in the Case workspace): not run.
- Independent Standards and Spec reviews of this source/data change: not run.
- The full `python3 scripts/check.py` (including `browser`, `tooling`, `typecheck`) before a PR.
- Not pushed, not merged to `dev`.

### Decisions recorded

- Pair all seven split keyboards, not only the symmetric ones: the map is a reflection and holds for
  the right-authored KLOR and Lily58 too, which the key-reflection test shows. What differs is the
  hardware placement, which stays a local exception.
- Unibody keyboards stay without mechanical configuration. A default there would invent geometry
  with no source fact, and the ticket only asks for independent configuration where it is clear.
- The reused-PCB case is a Core-verified demonstration, not a 21st shipped project: the Library
  card list, the 20-project baseline counts and the provenance would change with no product need.
- The draft was kept after review and repaired, not restarted: its approach (typed edits through
  `apply-mechanical-edit`) was right.

## Comments

- Unclaimed 2026-10-12 at the user's request; status back to `ready-for-agent`. Existing work on this ticket's branches or worktrees was not touched.
- 2026-10-14: User deferred built-in board work under [138](138-retire-shipped-demos.md). This ticket's board-specific work is out of scope for now; re-triage only if the user brings built-in boards back into scope. Existing findings remain historical evidence, not implementation instructions.
