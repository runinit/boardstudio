# 133: Qualify the EC11E15244G1 encoder pattern

Status: claimed  
Type: build  
Lane: F  
Blocked by: none

This is an isolated numeric pattern and terminal-role qualification child of [118](118-sofle-adaptation-hardware-contract.md).
It does not depend on resolving 118; ticket 118 remains open for its electrical, firmware, functional and installed-fit gates.
The F coordinator owns a Luna 6 medium implementer followed by a fresh Sol 6.1 xhigh verifier. Source work has not started.

## Proposed source lease and gate

The sole proposed source path is the new file `footprints/tests/encoder_ec11e15244g1_qualification.rs`. Producer, PARAMS,
registration, Core, recipes, assets, existing tests and goldens stay read-only. The source lease is held until root commits this
claim, explicitly grants the complete implementation-to-verification cycle, and finishes the cross-effort delivery queue.
No temporary producer mutation is included; any such experiment needs a separate explicit root grant.

## Qualification scope

Use explicit parameters: `signal_hole_width=1.0`, `signal_hole_height=1.0`, `encoder_pads_position=7.5`,
`include_momentary_switch_pads=true`, `mounting_holes_position=6.25`, `mounting_holes_height=2.6`, and
`mounting_holes_width=1.5`. Configure the front-only mounting-side pattern at placement angles 0°, 90° and 120°.
Configure the documented input roles independently: manufacturer A→generator C, B→A, common C→B, D→S1 and E→S2;
use distinct net sentinels and verify fresh pad IDs remain associated with their literal pad identities.

Assertions use the independent manufacturer coordinates and roles, never values derived from producer output. In the mounting-side
view with x right and y down, check rotary A/C/B at (−2.5, 7.5)/(0, 7.5)/(2.5, 7.5), push D/E at (−2.5, −7)/(2.5, −7),
and mounting tabs at (−6.25, 0)/(6.25, 0). Check generator pad IDs C/A/B/S1/S2 = pad-2/pad-0/pad-1/pad-3/pad-4 and
distinct tab IDs pad-5/pad-6. Verify the 5 mm row spans, the 7 versus 7.5 mm row offsets, 12.5 mm tab span,
Ø1.0 electrical-hole nominal and 1.5 × 2.6 mm total tab-oval extents. Assert normalization's Y and angle conversion once,
then check each rendered placement against an independently calculated translated/rotated expectation without double-applying
the Y or angle transform. Preserve separate identities, layers and nets through rendering and export.

The oval is the observed producer profile; the manufacturer's rectangular depiction does not specify corner profile. Plating,
back-side physical fit, terminal clearance and STEP/model fit remain unknown. No hardware acceptance, manufacturing readiness,
or resolution of ticket 118 follows from numeric qualification.

## Verification

Run the focused qualification test, affected crate checks, lint and `python3 scripts/check.py repo`; run no per-ticket full batch,
browser or build. A fresh verifier reviews the literal oracle and reproduces the focused and required checks. Record exactly what
was and was not established in the Outcome. Root owns staging, commit and GitNexus refresh.
