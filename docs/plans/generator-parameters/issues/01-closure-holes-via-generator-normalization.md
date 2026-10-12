# 01: Closure clearance holes through generator normalization

Status: resolved
Type: build
Blocked by: none
Map: [map.md](../map.md) · Decision: [ADR-0003](../../../adr/0003-rust-footprint-generators-in-core.md)

## Request

`normalized_mounting_hole` (`core/src/mechanical/clearance.rs:198-222`) writes
`hole_drill`/`hole_size` as `Value::String`. `ceoloide/mounting_hole_npth` declares them
as numbers (`footprints/src/generators/ceoloide/mounting_hole_npth.rs:17-22`), and the
registry rejects them only at render time (`footprints/src/registry.rs:152-158`):
"parameter hole_size must be a number". Any half that keeps its proposed closure mounts
fails preview and export. Lines 223-284 also recompute the pad and courtyard by hand,
duplicating the generator's own normalization. Callers: `clearance.rs:35,48`,
`core/src/mechanical/edit.rs:345-363`.

## What to do

- Write both parameters as JSON numbers, then call
  `crate::generators::normalize_definition` instead of the hand-written pad/courtyard
  code. Keep the template's id, name, kind and models; keep the definition id prefix,
  part ids, MH references and memberships.
- Normalization errors propagate as `Err(String)` before anything changes.
- Update the unit test (`clearance.rs:681-682`) to expect numbers and whatever the
  normalizer produces for pads, courtyard and envelope; record any field that differs.
- Add a regression test in `core/tests/`: open a project, enable a half so closure mounts
  are proposed, keep them, then render every generator-backed definition and part
  (`generators::render_forms`) and run board preview and KiCad export through the
  engine. Cover a boss diameter that is not a clean decimal (2.8000000000000003), a
  flipped instance and a linked pair.
- Re-bless the boundary transcript (`BOUNDARY_BLESS=1`). Only the 8 lines at
  `core/tests/fixtures/boundary/transcript.json:15685-16854` may change; diff to confirm.
  The transcript is checked natively and in WASM, so both must pass.

## Acceptance

- [ ] No `Value::String` is written for numeric generator parameters in `clearance.rs`.
- [ ] Hand-computed pad/courtyard code is removed; normalization is the only source.
- [ ] The regression test fails on the old code and passes on the new.
- [ ] The transcript diff is limited to the hole parameters (plus normalizer fields, if
      any, listed in the Outcome).

## Verification

`python3 scripts/check.py quick`; Core native tests (non-nested checkout); boundary
parity native and WASM. GitNexus `impact` on `normalized_mounting_hole` and
`project_with_optional_template` first.

## Risks / ADR

Normalization could change pad ids or courtyard text, which would mean more transcript
churn. No ADR: this applies ADR-0003. Out of scope (see the map): Core building the
template itself and removing the `mountingHole` wire field.

## Outcome

Resolved. Commit `46610e2a8` on `typed-core-edits/mechanical` (implementation, tests,
transcript); this Outcome follows in a docs commit.

### What changed

- `core/src/mechanical/clearance.rs`: `normalized_mounting_hole` now returns
  `Result<PartDefinition, String>`. It writes `hole_drill` and `hole_size` as JSON numbers
  (a non-finite diameter is an `Err`), keeps the template's id prefix, name, kind and
  models, and calls `generators::normalize_definition`. The hand-written pad rewrite and
  the courtyard corner/bounds code are gone; `project_with_optional_template`
  collects the results with `?`, which happens before the document is touched.
  `Value::String` no longer appears in the file.
- Unit test `projects_canonical_and_flipped_instance_union_and_preserves_other_parts`
  expects `{"hole_drill": 7.6, "hole_size": 7.6}` and nothing else differs: the normalizer
  reproduces the old pad (`pad-0`, circle, size and drill 7.6, unplated, rotation 0),
  courtyard (the 7.6 square), `envelopeSource` and `envelopeNotice` exactly, so there is no
  other field to record at that level.
- New `core/tests/closure_generator_parameters.rs` (registered in `core/tests/integration.rs`).

### Transcript diff scope

`core/tests/fixtures/boundary/transcript.json` changed in 16 lines (32 diff lines), not the
8 the ticket predicted, in exactly the four closure definitions at the ticket's line ranges
(15685, 15739, 16799, 16853). Per definition: `hole_drill` and `hole_size` go from the
string `"2.8000000000000003"` to the number `2.8000000000000003` (the 8 predicted lines),
plus two normalizer fields: the pad's `at.y` and `rotation` go from `0.0` to `-0.0`. The
transcript already held 60 `-0.0` values from other normalized definitions. `-0.0 == 0.0` for every
numeric comparison and no id, courtyard or text changed. Nothing else in the file moved
(`git diff -U0` checked).

### Test evidence

Red-then-green. The tests were written before the fix and run against the unfixed code
(`cargo test -p boardstudio_core --locked --test integration closure_generator_parameters`
in the non-nested checkout `/tmp/claude-1000/boardstudio-tce-verify` at `4559e2ca8`):

- `proposed_closure_mounts_render_preview_and_export` failed on the type assertion
  (`assembly-closure/definition/board/-38.18182/-30.00000 hole_drill: "6.6"`). With that
  assertion disabled it failed at the render the ticket names:
  `ceoloide/mounting_hole_npth: parameter hole_size must be a number.`
- `a_boss_diameter_that_is_not_a_clean_decimal_renders_on_a_flipped_linked_pair` failed on
  `left: String("2.8000000000000003") right: Number(2.8000000000000003)`.

After the fix both pass. They cover: closure mounts proposed and kept by an edit on a
configuration with none; a boss mount whose diameter is `2.2 + 2 * 0.3 = 2.8000000000000003`
(and a second boss at `3.1 + 0.6`); a flipped instance; a linked mirror pair; and for each
case `generators::render_forms` on every generator-backed definition and part, then
`kicad::preview` and `kicad::export` of the board, asserting every generated hole appears
in the exported `.kicad_pcb`.

### Command results

All at `46610e2a8` unless stated. GitNexus `impact` (CLI, upstream, **index stale**:
last indexed at `4b372b2`) on `normalized_mounting_hole` and `project_with_optional_template`:
LOW, 5 and 17 impacted symbols, all in the Mechanical module, no processes.
`detect-changes --scope all`: low risk, 6 symbols, 0 processes (also stale-index).

- `python3 scripts/check.py quick` in the non-nested checkout: `repo`, `tooling`, `lint`
  (fmt and clippy `-D warnings`), `typecheck` and `test` (workspace 476 integration tests
  plus the lib and footprint suites) pass. Its `browser` step stops at
  `cad/scripts/test-cadrum-browser.py` because that checkout has no `cad/wasm/pkg`, which
  is an untracked build output, not a code failure.
- `python3 scripts/check.py quick` in the nested worktree: every step through `browser`
  passes (`run-wasm-tests.py --all`: executed 70, failed 0); the run stops at
  `cad/scripts/test-cadrum.py`, where cargo refuses the nested `cad/step-oracle`
  workspace, as the map's note on non-nested checkouts predicts.
- Boundary parity: native passes against the re-blessed transcript (`BOUNDARY_BLESS=1`);
  WASM passes: `wasm-pack test --headless --chrome web/crates/host --locked --features page
  --lib -- boundary_parity::` ran `wasm_exports_reproduce_the_native_transcript` ok.

### Not verified

- `check.py browser` was not run end to end in a single pass: the cadrum steps fail in the
  environments above for the stated reasons, and only the boundary and presentation WASM
  tests were run.
- `check.py deny` (no dependency changed) and `release` (not allowed) were not run.

### For tickets 02 and 03

- 02: the projected closure definitions now carry `-0.0` for the pad `at.y` and `rotation`
  (and numeric parameters). Re-recording `demo-projects-baseline.json` will show them in the
  `assembly-closure/definition/*` entries; that is expected and comes from the normalizer,
  not from the recipes. Closure holes still land wherever Core proposes them; this ticket
  did not change positions, de-duplication or counts.
- 02: observed while writing the flipped-pair test, unchanged by this ticket: a closure
  mount linked through a mirror pair, with the second instance `flipped: true`, projected
  one `case-closure/*` hole (at the authored position) instead of two mirrored ones. The
  likely cause is that the pair map and the instance flip both reflect the mount so the
  halves coincide and de-duplicate; I did not confirm it. Check the hole count per shipped
  pair before accepting it. (The test therefore adds a mount owned by the flipped half to
  get two distinct holes.)
- 03: `normalize_definition` regenerates a closure definition's pads and courtyard from its
  parameters, so a migrated saved definition (strings turned into numbers) has the right
  pads already; the next projection rebuilds it through the normalizer anyway. 03's
  count of 53 transcript lines was taken before this commit: the 8 closure parameter lines
  are numbers now, so recount before editing, and start any re-bless from this commit's
  transcript.
