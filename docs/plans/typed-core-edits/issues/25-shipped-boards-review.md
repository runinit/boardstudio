# 25: Full review of the 20 shipped boards, Sofle first

Status: claimed
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
research with the real fixtures), then user review of the list, then fixes board by board
with the implement/verify process. Each board's fixes are their own ticket under this one.

## Acceptance

- [ ] A written review for every shipped board with findings ranked by severity.
- [ ] Sofle variants fixed and verified first; user sign-off before the rest.
- [ ] Decisions needing hardware knowledge are asked of the user, not guessed.

Keyboard [04](04-refresh-included-keyboards.md) is redone on the native model in [23](23-redo-keyboard-04-on-native-halves.md); this review feeds it and may supersede part of it.

## Outcome

Phase 1 review: [shipped boards review](../audit/shipped-boards-review.md). This was read-only research; no recipes, fixtures, footprints or code were changed and no builds/exports or hardware fit checks were run. The REVIUNG41 archive JSON was inspected in memory; hardware-dependent component and clearance decisions remain for the user.
