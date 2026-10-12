# 135: Persist the qualified EC11 configuration in the Sofle v2 recipe

Status: resolved
Type: build
Blocked by: 133
Parent: [118](118-sofle-adaptation-hardware-contract.md)

## Problem and result

The Sofle v2 recipe currently selects `rotary_encoder_ec11_ec12` for SW25 with empty generator overrides. Persist the explicit configuration qualified in ticket 133 so the v2 encoder pattern does not depend on producer defaults. Preserve existing SW25 identities, poses, side and wiring, plus measured key positions, tilts and saved widths. Do not repair pre-existing key-width normalization in this ticket.

Use this v2-only configuration for the encoder definition:

| Parameter | Value |
| --- | --- |
| `side` | `"F"` |
| `reversible` | `false` |
| `include_plated_mounting_holes` | `false` |
| `signal_hole_width` | `1.0` |
| `signal_hole_height` | `1.0` |
| `encoder_pads_position` | `7.5` |
| `include_momentary_switch_pads` | `true` |
| `mounting_holes_position` | `6.25` |
| `mounting_holes_height` | `2.6` |
| `mounting_holes_width` | `1.5` |

This reuses ticket 133's front, nonreversible, NPTH fixture. NPTH is an explicit configuration choice, not a plating recommendation. Preserve the producer's separate push row at y=-7 and lug relative angle of -90 degrees. Do not copy test sentinel nets, override A/B/C/S1/S2 nets, or introduce an input profile. Electrical role rewiring and direction/firmware tuning remain separate.

## Exact two-file lease

1. `core/examples/demo_projects/recipes.rs`: only the parameter-selection expression inside `recipes::sofle`, in the existing definitions loop near the `mcu_nice_nano` `include_extra_pins` branch. Add the v2-only `rotary_encoder_ec11_ec12` case while preserving MCU and fallback cases. Do not edit placement/add calls, `physical::append`, measurements, other variants, or other recipes.
2. `core/tests/included_keyboards.rs`: add focused regression coverage, for example `sofle_v2_ec11_explicit_configuration_persists_and_exports`, with strictly test-local helpers if required. Preserve existing tests and shared test helpers. Bind insertion anchors during the granted source read; line numbers are not a lease.

All other paths and hunks are excluded: footprint producer and ticket 133 test, Core mirror/native assembly engine, `mechanical.rs`, electrical/firmware contracts, schema/API visibility, UI, catalogue/assets/models, RGB/Choc recipe changes, ticket/map/registry/ADR edits, generated goldens and generated outputs. Root owns generated-output updates, graph refresh, staging, commit, integration and tracker allocation. No broad refactor or dependency is included.

## Acceptance

- Before production editing, run source-bound GitNexus impact and resolve UNKNOWN/caller gaps with targeted source evidence; report HIGH/CRITICAL risk. The preparation packet is not graph clearance.
- Add a public recipe regression that fails on the old empty-override recipe because the explicit parameter contract is absent. Observe the expected failure before editing production code. Do not claim a geometry regression if existing defaults already produce the same geometry.
- Assert every selected value on the v2 SW25 definition, preserving generator source/version/definition identity and both existing SW25 instances' identity, pose, side and wiring. Assert RGB/Choc configuration is unchanged.
- Through existing public portable serialization/reopen facilities, assert the parameter map survives save/reopen and normalization without dropping or replacing it. If this requires a third production/test file or new public seam, stop and request the smallest lease amendment.
- Inspect public v2 PCB/footprint export: five electrical pads and two unnetted NPTH lug pads; five round 1.0 drills; lug centers at +/-6.25; oval drill axes 2.6 x 1.5 with producer relative orientation. Preserve terminal/net identity through export and use real recipe nets, not test sentinels. Distinguish the front-view qualified pattern from any second-half output; unchanged local geometry does not prove rear installed fit.
- Assert existing key geometry/width manifest, unrelated component definitions/placements and source metadata remain unchanged. Preserve current legacy two-half composition; do not migrate native correspondence.

## Checks and verification

Run the new `included_keyboards` regression and existing `encoder_ec11e15244g1_qualification` target, then affected `repo`, `lint` and `test` checks in the explicitly granted sequential foreground window. Find exact Cargo package/selectors from current repository conventions; this ticket assumes no invocation. Fresh independent verification must reproduce the initial RED, change one explicit override back to old behavior and observe the intended assertion fail, restore exact bytes (touching as required), then rerun focused GREEN. Reuse ticket 133's completed manufacturer/producer evidence; do not repeat its hardware audit or producer mutations. Root owns generated-output updates/build and the required full check before PR. This source-only recipe change requires no browser step.

## Parent 118 remains open

This ticket encodes and persists the already qualified nominal fixture; it does not close ticket 118 or qualify the system electrically or mechanically. Encoder pullups/filtering/minimum current, diode-matrix push current, 60 transitions/30 detents, firmware direction and bench behavior, lug profile/plating/terminal tolerance, shaft/knob/plate fit, rear physical mounting and model alignment remain open. Broader UART, lighting, power, firmware and installed-geometry work retains its existing ownership.

## Evidence

- Accepted proposal: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/v2-encoder-explicit-configuration-proposal.md`.
- Qualification: `/home/chris/.cache/boardstudio-baselines/ticket133/verification/verification-report.md`.
- Retained comparison: `/home/chris/.cache/boardstudio-baselines/sofle-encoder-audit/comparison/comparison-report.md`.
- Preparation draft: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/proposed-child-118-v2-ec11-recipe.md`.

## Outcome

Implemented in source commit `4689c99b`, integrated at `78eb5069`. Fresh independent verification passed: recipe source `3d620d10…`, included-keyboards test `e5b768de…`; public archive/export and preservation assertions passed, and reverting one explicit override reproduced the intended compile-time RED before exact restoration and GREEN. The verifier's affected checks passed in 63.3 s; report: `/home/chris/.cache/boardstudio-baselines/ticket135/verification/verification-report.md`.

The E12+F135 integration cohort passed at `78eb50695708686fbb65ea962aae7c67825e5871` in 372.4 s: repo 0.1, tooling 3.0, lint 6.1, build 55.5, test 60.0 and browser 247.7. Log: `/tmp/boardstudio-cohort-inspector12-sofle135.log`. The exact two-file implementation lease is released. Parent 118 remains open for electrical, firmware and physical-fit gates; this ticket does not establish system hardware acceptance.
