# 25: Full review of the 20 shipped boards, Sofle first

Status: ready-for-human
Type: [22](22-mirrored-half-integrated-verification.md)
Blocked by: 
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
- [ ] User reviews the ranked list and hardware-dependent decisions before any board fix starts.
- [ ] Sofle variants fixed and verified first; user sign-off before the rest.
- [x] Decisions needing hardware knowledge are asked of the user, not guessed.

Keyboard [04](04-refresh-included-keyboards.md) is redone on the native model in [23](23-redo-keyboard-04-on-native-halves.md); this review feeds it and may supersede part of it.

## Outcome

Phase 1 research and AI review: [shipped boards review](../audit/shipped-boards-review.md). The review pass corrected the Corne, Sofle RGB and Sofle Choc linked switch-gap values from ticket 06, removed the superseded Sofle RGB 0.009 mm current-gap claim, and ranked the native-mirror recipe mismatch from ticket 20 among the top findings. No recipes, fixtures, footprints or code were changed; no build/export or hardware fit check was run. The report does not independently inventory every pad, drill, courtyard, model asset, wiring assignment or generated PCB/plate/case output. The REVIUNG41 archive JSON was inspected in memory. Hardware-dependent component and clearance decisions remain for the user. Ready for human review before any fix begins.
