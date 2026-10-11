# Ticket 130: Qualify the EVQ-PUC02K side-reset footprint

Status: ready-for-agent
Type: qualification
Parent: 118

## Scope

Qualify the existing `ceoloide/reset_switch_smd_side` generator for the exact
Panasonic EVQ-PUC02K variant using its current public bundled-generator and
catalogue render boundaries. This is a bounded generator qualification. Do not
change the producer, default model recipe, part API, inventory, board layout,
or case recipe in this ticket.

The future implementation lease is limited to these two new files:

- `footprints/tests/reset_switch_smd_side_qualification.rs`
- `docs/plans/typed-core-edits/issues/130-reset-switch-smd-side-qualification.md`

Use `include_bosses=true`, `reversible=false`, and the generator's existing
default model filename. Exercise synthetic nonzero poses on F and B at 0°, 90°
and 120°. Do not claim these are saved Sofle reset-switch placements. Render
through the public bundled generator, part reference, net allocator and parsed
footprint expression; keep the independent dimension and transform oracle in
the test rather than deriving expected values from generator output.

## Source evidence

The exact manufacturer variant is Panasonic EVQPUC02K (EVQ-PUC02K): straight
terminals, side push, boss present, 2.2 N, 1.65 mm mounting height and 0.3 mm
travel. Panasonic's current EVQ-PU catalog is revision `ANCTB23E 202507`,
printed page 2 (PDF page 3, zero-based page index 2), with the EVQPUC02K
dimensioned views, contact diagram and reference land pattern:

- [Panasonic product page](https://na.industrial.panasonic.com/products/switches-encoders-interface-devices/switches/series/79279/model/79357)
- [Panasonic EVQ-PU catalog PDF](https://api.pim.na.industrial.panasonic.com/file_stream/main/fileversion/245509)
- PDF SHA-256: `cbdebcddea0fc6c64a64e8e1a53f97acca292fd8e44af49af544f140fc67021a` (194,144 bytes; 11 pages).
- Ignored local captures: `.scratch/source-evidence/Panasonic-EVQ-PU-ANCTB23E-202507.pdf`, `.scratch/source-evidence/Panasonic-EVQ-PU-ANCTB23E-202507-page-2.png`, and `.scratch/source-evidence/EVQ-PU-boss-land-detail.png`.

The manufacturer reference land pattern gives an outside pad span of `6.8 ±
0.1 mm`, inner gap `3.7 ± 0.1 mm`, left-column row gap `0.7 ± 0.1 mm`, and
right-column overall span `2.7 ± 0.1 mm`. The drawing labels the exact part's
terminal positions in its external view as upper-right 1, lower-right 2,
upper-left 3 and lower-left 4. Its circuit diagram identifies contact groups
`{1, 3}` and `{2, 4}` and shows a normally open contact.

The boss land drawing labels the distance between the two boss-hole centers as
`2.75 ± 0.05 mm`. Its diameter leader applies to the lower hole only:
`Ø0.75 +0.1/−0 mm`. The upper hole's diameter is not dimensioned in that
drawing. Do not assert both manufacturer hole diameters from the shared
appearance of the two holes.

The generator is currently registered at
`footprints/src/generators/mod.rs:36`; its `PARAMS` declaration is at
`footprints/src/generators/ceoloide/reset_switch_smd_side.rs:15` and `body` at
line 47. Its declared license is MIT, author `@ceoloide`; the generator NOTICE
pins upstream `ceoloide/ergogen-footprints` at
`48935f54b456ff1503d78d6b17d9d146b54e8ade`.

With boss holes enabled, the existing body emits two Ø0.75 mm non-plated holes
at Y ±1.375 mm. This is a producer configuration regression assertion, not a
claim that the drawing dimensions both holes. The nominal pad dimensions and
symmetry are also producer configuration assertions unless independently
labelled by the manufacturer drawing.

Boss mode with the default model filename selects the existing KiCad STEP
asset `Panasonic_EVQPUL_EVQPUC.step` (SHA-256
`c30e15d8da5c3856f0a5b19ab62f22eef9b06d69161963e77cdf6c0b09a7ac8c`). Its
header declares KiCad's CC-BY-SA 4.0 license with the electronic-design
exception. This is a KiCad model, not manufacturer CAD. Its presence and
binding do not prove absolute datum, solid alignment or physical fit. The
generator itself is MIT; keep these two provenance records distinct.

GitNexus query/context/impact results for `boardstudio-ts-boardstudio2` were
UNKNOWN/lower-bound: the index was 62 commits behind HEAD and reported 25
receiver-typing gaps for `body`. Source inspection confirmed current
registration and text references; the stale graph is not evidence of isolation.
Do not refresh the index from this lane; root owns refreshes.

## Acceptance

1. Render the exact bundled generator on F and B for each of 0°, 90° and 120°
   at a synthetic nonzero translation, with distinct `from` and `to` nets.
   Assert exactly four numbered rectangular SMD pads and two unnumbered
   circular NPTH boss holes. Assert exact face copper/paste/mask layers on the
   SMD pads, wildcard copper/mask layers on the holes, and no electrical nets
   on the holes.
2. From the parsed emitted pads, calculate the manufacturer-labelled spans
   independently. On the unrotated local land pattern, compare outside pad
   span, inner gap, left-column edge gap and right-column overall span against
   the published nominal values above using `1e-6` only as numeric comparison
   epsilon. Keep source tolerances explicit and separate; do not treat the
   epsilon as a manufacturing tolerance. Confirm the same invariant after
   pose rotation and translation through independent fixed expectations:
   upper-to-lower vectors are `(0, 2.75)` at 0°, `(2.75, 0)` at 90°, and
   `(2.381569860407, -1.375)` at 120°, transformed once by the placement
   `(x, -y, rotation)` convention. Check pad angle and face transforms are
   applied once.
3. Map pad identities from the manufacturer's external view and contact
   diagram. Assert groups `{1, 3}` and `{2, 4}` receive distinct configured
   nets on both faces, with the expected copper side. Treat `GND`/`RST` as a
   design choice for `from`/`to`, not manufacturer polarity.
4. Compare the lower boss-hole center spacing and lower-hole drill against
   their actual manufacturer leaders. Assert the upper hole's current Ø0.75
   drill only as a generator regression and record the missing manufacturer
   diameter as an open fact. Do not filter holes by electrical number when
   identifying their parsed emitted order.
5. With bosses enabled and the default filename, assert exactly one model
   binding resolves to `Panasonic_EVQPUL_EVQPUC.step`, offset zero, unit scale,
   F rotation `[0, 0, 0]` and B rotation `[0, 0, 180]`. Verify explicit
   filename/rotation overrides win and configured offset/scale are preserved.
   Optionally confirm boss-off selects the unbossed EVQPUJ/EVQPUA model and
   emits no holes. Do not infer solid fit from these bindings.
6. A temporary mutation changing only the lower boss drill from `0.75` to
   `0.95` mm must fail the intended lower-hole manufacturer assertion. Capture
   exact producer preimage bytes and hash before mutation, restore and touch
   the exact preimage, then rerun the focused qualification green. A `0.8 mm`
   mutation is inside the published lower-hole tolerance and is not a valid
   failing mutant. Obtain a separate root mutation lease before making any
   producer mutation.

## Gates and limits

The generator's actuator marker is at negative local Y while the Panasonic
external view depicts the push plate on the opposite side of its terminal
view. This ticket may qualify the emitted land pattern, contacts, layers and
model binding; it cannot qualify actuator orientation against an absolute
board datum, case opening/access, usable actuation travel in the assembled
keyboard, solder/contact reliability, actual boss fit, STEP solid alignment,
or physical behavior. Record these as open gates. Do not loosen or weaken an
assertion to make a mismatch pass: capture the failing evidence and ask root
for a separate, bounded producer investigation lease.

Ticket 118 and whole-footprint/source-catalogue qualification remain open.
Passing this ticket does not resolve 118 or establish fabrication readiness.

## Verification plan

After a distinct implementer completes the two-path lease, a fresh Sol 6.1
xhigh verifier must independently review the manufacturer leaders and
transform oracle, reproduce focused green and the restored lower-hole mutation
red/green, then run the affected checks in an explicit root command window:

- `cargo test --locked -p boardstudio-footprints --test reset_switch_smd_side_qualification`
- `python3 scripts/check.py repo lint`
- `cargo test --locked -p boardstudio-footprints`

Use one foreground command at a time and the repository's configured 12 Cargo
jobs; do not serialize a 12-job check by overriding it. Root retains the
index refresh and graph-change analysis ownership.

## Outcome

Planning-only draft based on the read-only source audit and independent Sol
6.1 max plan review. No qualification tests have been authored or run, no
producer bytes changed, and no source or product model fit has been claimed.
