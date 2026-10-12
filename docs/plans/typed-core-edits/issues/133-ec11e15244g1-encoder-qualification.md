# 133: Qualify the EC11E15244G1 encoder pattern

Status: resolved
Type: build  
Lane: F  
Blocked by: none

This is an isolated numeric pattern and terminal-role qualification child of [118](118-sofle-adaptation-hardware-contract.md).
It does not depend on resolving 118; ticket 118 remains open for its electrical, firmware, functional and installed-fit gates.
The F coordinator completed the Luna 6 medium implementation followed by fresh Sol 6.1 xhigh verification. The source and temporary mutation leases are released; bounded acceptance and the full integration cohort passed.

## Completed source lease and gate

The sole lasting source addition is `footprints/tests/encoder_ec11e15244g1_qualification.rs`. Root committed the claim and
granted the complete cycle. The initial lease excluded producer mutations; root later separately authorized two bounded
verifier-only geometry/common-net probes in the encoder body. Both compiled, failed as intended and were restored exactly.
Producer, PARAMS, registration, Core, recipes, assets, existing tests and goldens have no lasting change from this ticket.

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

## Outcome (2026-10-11)

Fresh independent Sol6.1 xhigh PASS for the complete bounded front-only numeric-pattern and terminal-role qualification. Sole lasting source addition: footprints/tests/encoder_ec11e15244g1_qualification.rs, final formatted SHA256 7fc260b0c6c80009da62a4b112cfb09f6f67fb61a9979c01d24c69fd0d0b4e4e. Verifier formatting was whitespace-only relative to independently reviewed467a09b3.... Producer bytes restored exactly to SHA256 d83e2011b3bb24154082abaaa34aa48b8225ef6ba9f2d93a9d061521adbcf756; no producer change remains.

Independent runtime: focused3/3 PASS; full footprints crate90 PASS,0 failed,3 existing ignored; repo/lint, rustfmt and diff checks PASS. Authorized geometry and common-role/net producer probes separately compiled and failed at their intended literal assertions. Each was restored byte-for-byte, hash-bound and touched, followed by focused3/3 GREEN. Front0/90/120 placements, independent nominal pattern/identity/net crosswalk, drill syntax/extents, one-time normalization and render/export preservation are covered. Physical corner profile, plating/backside/insertion/STEP fit, electrical/firmware and installed-fit acceptance remain outside this qualification;118 stays OPEN.

Authority: /home/chris/.cache/boardstudio-baselines/ticket133/verification/verification-report.md. Root source commit `4a40faff`, merged with `--no-ff`. One full default cohort check passed at integration `6eeef607adc186b162e38ff22ac54521de42d809` in 510.0 s (repo 0.1, tooling 7.5, lint 10.1, build 142.4, test 70.3, browser 279.5); all 70 presentation tests passed. Log: `/tmp/boardstudio-cohort-133-134.log`.
