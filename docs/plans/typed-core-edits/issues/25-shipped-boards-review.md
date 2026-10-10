# 25: Full review of the 20 shipped boards, Sofle first

Status: ready-for-agent
Type: task
Blocked by: [22](22-mirrored-half-integrated-verification.md)
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## Request

User, 2026-10-10: the shipped keyboards were imported and recreated but never cleaned up.
They need a full review covering footprints, positioning and the rest, "lots of them need
tweaks outside of these changes". **Sofle is the focus** and goes first.

## What to do

Per board (20 shipped: 15 measured, 3 Sofle, REVIUNG41 archive, VIK review), starting with
Sofle v2, RGB and Choc:

- **Footprints:** each part's footprint, library source, pad and drill parameters, model
  assets, courtyard, side and `reversible` settings against the real part; import leftovers
  and placeholders (generated bay, reset, TRRS) that should be real components.
- **Positioning:** switch pitch and rotation against the measured source, controller,
  connector, diode, LED and encoder placement, mounting holes and their keep-outs, and the
  tight fits already recorded (closure holes vs controller bay on Lily58, KLOR, Sweep, Chocofi,
  Sofle RGB/Choc; switch-cutout gaps on Cantor, TOTEM, Sofle RGB; Sofle v2 gasket has no
  closure mounts).
- **Structure:** one measured half plus the native mirror; recorded exceptions only; outlines
  and gap repair (the KLOR 6.4 mm non-equivariant pocket); matrices and wiring.
- **Output:** PCB, plate and case results and fit findings; record unresolved hardware
  qualification as findings, since rebuilding does not establish physical fit.

Method: a planning pass producing a per-board checklist and severity list (read-only
research with the real fixtures), an AI review against the recorded outcomes and ADR rules,
then user review of the ranked list. No board fixes start before the user review; afterwards,
fixes proceed board by board with the implement/verify process. Each board's fixes are their
own ticket under this one.

## Acceptance

- [x] A written review for every shipped board with findings ranked by severity.
- [x] AI review completed against tickets 02, 06 and 30 and ADR-0010; corrected current linked-fit values and recorded scope limits.
- [x] User reviews the ranked list and supplies the rebuild direction before any board fix starts.
- [x] Rebuild direction recorded: configurable standard key spacing, native board-specific key assemblies, real component matches, native outlines/mount generation, regenerated closure clearances, and VIK deferred.
- [ ] Sofle variants fixed and verified first; user sign-off before the rest.
- [x] Decisions needing hardware knowledge are asked of the user, not guessed.

Keyboard [04](04-refresh-included-keyboards.md) is redone on the native model in [23](23-redo-keyboard-04-on-native-halves.md); this review feeds it and may supersede part of it.

## Outcome

Phase 1 research, AI review and human direction: [shipped boards review](../audit/shipped-boards-review.md). The audit now records the user's rebuild rules: repository-default key assembly spacing with a global configurable option; duplicate-position diodes for the current shipped data followed by BoardStudio-native, board-specific assemblies; real component footprints; native outline conversion and mounting-system/clearance generation for splits and unibodies; safe-only footprint flipping and a separate reversible-variant option; and VIK deferred. No recipes, fixtures, footprints or code were changed; no build/export or hardware fit check was run. Exact component matches, Choc cutout geometry, hardware tolerances and actual physical fit remain to be checked in the implementation tickets. The report also records the parts of the footprint/output review that were not performed. This lane stops before implementation; per-board tickets should begin with Sofle after the mirrored-half dependencies are ready.

### Verification (Sonnet 5.5 reviewer, 2026-10-10, `lane-boards-review`)

Read-only: no build, cargo, export or hardware check. Reproduced from `core/tests/fixtures/demo-projects-baseline.json`, the layout JSON, the REVIUNG41 archive and the tickets: all 20 part counts; the closure-to-bay and closure-to-switch values (KLOR 1.42 mm overlap, Cantor 0.031, TOTEM 0.048, Sofle Choc 0.109, and the rest, for both halves); the diode offset; the ticket 30 headings; the Sofle peripheral provenance; the 15 measured and three Sofle switch layouts (distance and rotation test). Wrong in the draft and corrected in the document: Sofle RGB 0.009 mm and the Sofle −3.30 mm bay overlap (both superseded by ticket 06, the baseline gives 1.066 mm and 0.10 mm on both halves); Sweep, Chocofi, TOTEM and Cantor are Choc boards at 18 x 17 mm pitch, not MX at 19.05 mm; key counts for Sweep, Chocofi and Cantor; the Lily58 and Sweep sources; stabiliser counts for Discipline, Mysterium and REVIUNG41; the ticket 30 heading difference applies to all seven measured splits. Decisions 1 (diodes) and 4 (pads) were already answered in ADR-0010 and ticket 31; reworded to the physical question that remains. A "Proposed tickets per board" list (Sofle first) is in the document. Not checked: pads, drills, courtyards, model assets, wiring and pin assignments, exported PCB, plate and case output, any real-hardware fit. Coverage gaps and the full claim table are in the document's Verification section. Status stays `ready-for-human`.
