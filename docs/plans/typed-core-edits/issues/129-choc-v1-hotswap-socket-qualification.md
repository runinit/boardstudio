# 129 — Choc v1 socket documented geometry and configuration

Status: resolved
Type: qualification
Parent: [118 hardware contract](118-sofle-adaptation-hardware-contract.md)
Prerequisite: independent plan review and fresh independent verification of the corrected bounded acceptance passed; full asset qualification remains open.

## Scope

Qualify the documented geometry of the existing Kailh CPG135001S30 socket candidate through the public bundled render boundary. Reuse `ceoloide/switch_choc_v1_v2`; do not add a footprint or change production geometry, defaults, models or recipes. This ticket does not qualify the complete switch/keycap/plate assembly. Ticket 118 remains open.

Proposed implementation lease is only new `footprints/tests/choc_socket_qualification.rs` and this ticket. Producer `footprints/src/generators/ceoloide/switch_choc_v1_v2.rs`, registry and model assets remain read-only. A temporary producer mutation requires a separate root lease.

## Independent source evidence

Official source: [Kailh CPG135001S30 datasheet](https://www.kailhswitch.com/Content/upload/pdf/202115927/CPG135001S30-data-sheet.pdf), document KH-PS-1702-35, Rev B. Revision B is dated 2020-10-29 (format revision, ECN-2010-25); PDF modification metadata 2021-08-16 is not a newer drawing revision. Captured PDF has 11 pages, 1,159,992 bytes, SHA-256 `c94a6b45abbbdd456fcc1549a60b4454ad897c027f04dafb26d5797e76061f93`.

Ignored evidence under `.scratch/source-evidence/`:

- `CPG135001S30-data-sheet.pdf` — original bytes.
- `recommended-pcb-layout.png` — printed page 8/10, PDF page 9 (zero-based index 8).
- `profile-dimensions.png` — printed page 3/10, PDF page 4 (zero-based index 3).

The recommended PCB layout states Top View and Single face board T=1.6mm. It shows a pair of holes separated horizontally by 5.00 and vertically by 2.20; the Ø3.00 leader applies only to the lower-left hole. The upper/common hole has no stated diameter, so its emitted drill is a configuration regression only and its manufacturer diameter remains unknown. The LEFT rectangular land has explicit horizontal and vertical 2.60 callouts. The right land has no separate dimension or same-size note. These layout callouts have no stated tolerances and are not parenthesized references. Compare nominal geometry with numerical epsilon only; do not infer manufacturing tolerances or undimensioned symmetry.

The separate profile dimensions (9.55±0.1, 6.85±0.1, 3.05±0.1, 1.80±0.1) describe component geometry, not PCB land/drill tolerances. Its 4.75 and 4.65 callouts have no stated ± tolerance. The PDF does not establish the upper-hole diameter, host terminal numbering/polarity, mounting face, reversible connectivity, absolute switch/socket datum, a PCB thickness range, or assembled clearance/fit.

## Selected configuration and pose

Use existing public parameters: Choc v1 support true, Choc v2 support false, nonreversible hot-swap, solder false, nonplated holes, PCB thickness 1.6 mm, `outer_pad_width_front=2.6`, `outer_pad_width_back=2.6`, and `hotswap_pads_same_side=false`. Explicitly record all additional configuration values. Both-version support and outer-pad widths 2.6 are existing defaults, not independent manufacturer evidence or automatic qualification. Do not change defaults or apply this candidate to recipes.

The stored Sofle Choc SW26 pose is x=96.71 mm, y=133.4 mm, rotation=120 degrees, in `content/layouts/sofle-layouts.json:954`. Upstream repository is `https://github.com/josefadamcik/SofleKeyboard`, pinned revision `fb294f7c58d0f91c379a9ef339159d4d056643e9`, source `Sofle_Choc/PCB/SofleKeyboard.kicad_pcb`, SHA-256 `f1714d5d75b3c6d103e865020a45cb0941ddcd59ac5dbd47a35eb5224bce7189`. This is a stored source pose fixture, not proof of board-origin conversion or physical assembly.

## Acceptance

Independent plan review inspected the actual drawing and narrow producer source. The drawing's LEFT land is beside the lower hole and maps to generated outer pad 2: F at (-8.275,-3.75), B at (+8.275,-3.75), controlled by the respective outer-pad-width parameter. These generator coordinates identify sites; their absolute positions and pad-to-hole gaps are not manufacturer-qualified. Fixed inner pad 1 maps to the undimensioned RIGHT land and receives configuration regression assertions only.

For a consistently ordered vector from the common upper hole to the side-specific lower hole, use independent fixed KiCad-frame expectations below; do not calculate expected vectors with the actual-output helper. Use the stored SW26 pose in both 120° cases and assert placement separately.

| Side | 0° | 90° | 120° |
| --- | --- | --- | --- |
| F | (-5, 2.2) | (2.2, 5) | (4.405255888, 3.230127019) |
| B | (5, 2.2) | (2.2, -5) | (-0.594744112, -5.430127019) |

Model configuration checks pin the emitted socket binding, offset (0,0,-1.6), unit scale and source-confirmed F/B rotations as generator configuration evidence only. The component profile's ±0.1 dimensions do not supply PCB-layout tolerances or establish model alignment.

1. Independently inspect the actual PDF/pixels and justify the drawing-view-to-generator mapping, including which emitted land corresponds to the dimensioned LEFT land. Do not infer host pad numbers from drawing position. If that mapping cannot be established, retain it as an explicit qualification blocker rather than manufacturing an oracle.
2. Through public bundled rendering, exercise F and B at 0°, 90° and the stored 120° Sofle pose. Assert the independently sourced hole-pair separation 5.00×2.20 mm, the Ø3.00 nominal drill on the drawing's lower-left hole, and only the mapped, explicitly dimensioned 2.60×2.60 mm land. Any retained Ø3 assertion on the common upper hole is a configuration regression only; its manufacturer diameter is unknown. Use numerical epsilon, not inferred tolerances. Expected geometry must not copy producer constants.
3. Verify transformed coordinates with independent fixed expected vectors and the actual KiCad down-Y/negative-angle convention. Compose placement and face transforms once; no second face swap. Check distinct terminal/net identities and exact effective copper/paste/mask layer sets. Electrical mapping checks establish configured identities, not manufacturer polarity.
4. Record the undimensioned right land and the upper-hole diameter as explicit unqualified manufacturer-evidence gaps. Any assertion of their emitted sizes is a configuration regression only, visibly distinct from source qualification. Do not claim complete socket fabrication/physical qualification from this bounded evidence.
5. Check emitted `Choc_V1_Hotswap.step` binding, offset, rotation, scale and selected 1.6 mm configuration. Separate generator configuration/binding evidence from unavailable independent model datum, alignment/contact, switch travel, keycap or plate-fit evidence; retain output-specific gates.
6. If a source discrepancy is demonstrated, record an expected failing regression and stop for a separate root producer lease. Do not loosen expectations, infer tolerance, or change production geometry under this ticket's test-only lease.
7. Fresh independent Sol 6.1 xhigh verification must reproduce this complete bounded acceptance and, under a separate explicit temporary producer lease, demonstrate one meaningful pad/drill mutation fails the intended assertion. Restore exact producer bytes, touch, verify hash and restored green. No retained production mutation.

## Audit and verification gates

Read component onboarding, current central lane registry, agent workflow and applicable Rust skills before implementation. Use Luna 6 high for the geometry oracle, then a fresh independent Sol 6.1 xhigh verifier. Independent PLAN REVIEW precedes implementation authorization; it must inspect actual source pixels and mapping, not trust the source planner's summary.

Read-only audit at clean F HEAD `4b97b59ae50d2c3728e64d36442dd8d0bcbf114e` found SPEC/body in `footprints/src/generators/ceoloide/switch_choc_v1_v2.rs:75`/:92, registration in `footprints/src/generators/mod.rs:32`, and dynamic callback in `footprints/src/registry.rs:238`. Bound graph `boardstudio-ts-boardstudio2` returned UNKNOWN/zero resolved upstream callers, lower-bound 25 receiver-typing gaps, and metadata 35 commits behind. Narrow source confirmed registration/dispatch; this is not safety or current-index proof. Root owns refresh and complete change analysis before commits.

Scheduled checks after root grants a foreground window: scoped rustfmt, `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio-footprints --test choc_socket_qualification`, `python3 scripts/check.py repo lint` with the same environment, and `cargo test --locked -p boardstudio-footprints`. No browser/full integration batch is required for this test/evidence slice; root schedules cohort integration checks. Asset qualification is independent of tickets 19/20/44; later recipe/assembly integration retains its dependencies and 118/physical gates.

## Outcome

Implemented as a test-only qualification slice in `footprints/tests/choc_socket_qualification.rs`; generator, registry, defaults, models and recipes remain unchanged. The test covers the datasheet's nominal hole-pair spacing, the lower-left Ø3.00 hole and dimensioned LEFT land on both faces at 0°, 90° and 120°, configured terminal nets and effective copper/mask layers, the selected model binding/transforms, and the explicit V1-only/hotswap configuration. Placement vectors are fixed drawing-derived expectations; the stored Sofle pose is a fixture, not a board-origin conversion. The common upper-hole diameter, right/inner land, configured net identities, model transforms and producer-emitted hole angles are configuration regressions only; no manufacturer pad-number/polarity, mirror-footprint, absolute-origin, contact, plate or full-switch-fit claim is made. Layout tolerances and the upper-hole diameter and undimensioned right land remain unknown.

The selected output contains five NPTHs. The test asserts that count and selects the final two according to the current emitted fragment order (center NPTH, two V1 stabilizers, common upper socket hole, face-specific lower socket hole); this is a producer-order-dependent selector, not manufacturer numbering. The captured producer emits upper angle `r` and lower angle `195° + r` on both faces. A fresh verifier must judge this selector's fragility and the planned drill mutation independently.

An independent implementation verification at `2cff220ec9eeea922ab8c88adeda2b3646c429e5` failed because the initial acceptance and test descriptions attributed the Ø3.00 callout to both holes. The actual drawing's leader specifies only the lower-left hole; the upper/common-hole diameter is not documented. Root accepted this attribution correction: the lower-left drill assertion is manufacturer-dimensioned, while the existing upper-hole drill assertion remains configuration-only. The test and ticket now distinguish those claims without changing geometry assertions or expected values.

Earlier focused runs exposed a moved `Side` value, an overbroad five-hole selection, missing angle components in emitted three-number `(at ...)` fields, and the exact NPTH layer set (`*.Cu` and `*.Mask`). These were corrected without changing fixed geometry expectations or broadening tolerances. The first `repo lint` run then found `clippy::let_and_return` in the new test; returning the render expression directly fixed it. Final focused test passed (2 tests), `python3 scripts/check.py repo lint` passed (repo and lint), and `cargo test --locked -p boardstudio-footprints` passed (all crate tests). Producer files and ticket 118 were not changed; ticket 118 remains open. Fresh independent implementation verification, root-owned GitNexus change detection and commit remain pending.

After the source-attribution correction, scoped `rustfmt --edition 2024 footprints/tests/choc_socket_qualification.rs` passed (exit 0, 0.40 s); the focused qualification test passed (2/2, exit 0, 0.87 s); `python3 scripts/check.py repo lint` passed (exit 0, 3.81 s); and `cargo test --locked -p boardstudio-footprints` passed (exit 0, 6.54 s). Exact command/exit/elapsed records are in ignored `.scratch/source-evidence/129-corrected-validation.log`. These checks did not themselves resolve the failed independent verification; the subsequent independent results are recorded below.

## Verification

Fresh independent Codex Sol 6.1 xhigh verification passed the root-approved corrected bounded acceptance at `0c9a1178d475c80c805ebc776f28068eca291bea`. The earlier `NOT VERIFIED` verdict at `2cff220ec9eeea922ab8c88adeda2b3646c429e5` remains valid history: attributing the Ø3.00 leader to both holes was unsupported. The correction changed the source attribution and acceptance explicitly; both numeric 3.0 mm drill assertions and all other geometry/configuration assertions were preserved. Resolution applies only to the corrected scope, not to the original stronger two-hole manufacturer-diameter claim.

The verifier read the original claim-commit acceptance and inspected the actual recommended-layout and profile pixels. The saved PDF hash remained `c94a6b45abbbdd456fcc1549a60b4454ad897c027f04dafb26d5797e76061f93`. The drawing's lower-left hole and adjacent dimensioned LEFT land map to the selected face-specific lower socket hole and outer pad 2; the common upper-hole diameter is configuration-only. The six fixed F/B vectors, stored 120° placements, explicit V1-only configuration, configured terminal identities, exact effective layer sets and model binding/transforms were reviewed against those evidence distinctions.

Under root's separate temporary lease, the verifier changed only the F lower socket NPTH drill from 3 to 3.2 mm in `footprints/src/generators/ceoloide/switch_choc_v1_v2.rs:646`, at `(-5,-3.75)`. The common upper hole, B lower hole, center hole and stabilizers were untouched. The focused test compiled and failed with exit 101, one test passed and one failed, on the intended dimension assertion: `dimensioned lower-left hole nominal drill: expected 3, got 3.2`. This was not a selector, count, parser or compilation failure.

The producer was backed up byte-for-byte to ignored `.scratch/source-evidence/129-verifier-0c9a1178-producer.preimage.rs`, restored from that backup and touched with `os.utime`. Preimage, backup and final SHA-256 all matched `138134ee039fb538dd36ef8ec1c86b0cc2d538a7b5b8c1f33d6232ed23e19484`; the temporary mutant hash was `31a27f9e0aad632f7a5bdeaf03d827c6948ebc7be6d68c120521cab2b2d34476`. The restored focused run rebuilt and passed. Final `git status --short` was empty and the producer/test/ticket diff against that verification HEAD was empty. No production mutation or test repair was retained.

All check commands used `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1`, sequentially in the exact F worktree:

| Check | Result | Wall time |
| --- | --- | --- |
| `rustfmt --edition 2024 --check footprints/tests/choc_socket_qualification.rs` | exit 0 | 0.030 s |
| `cargo test --locked -p boardstudio-footprints --test choc_socket_qualification` — baseline | exit 0, 2/2 passed | 0.096 s |
| Same focused command — lower-drill mutant | exit 101, intended assertion above | 0.626 s |
| Same focused command — exact restoration | exit 0, 2/2 passed | 0.525 s |
| `python3 scripts/check.py repo lint` | exit 0; vendored cgmath warnings emitted | 8.349 s |
| `cargo test --locked -p boardstudio-footprints` | exit 0, all 79 tests passed | 6.080 s |

Full commands, exits, timings, mutant diff, failure output and restoration proof are in ignored `.scratch/source-evidence/129-independent-reverification.log`. No browser or full integration batch was run for this test/evidence slice; root owns cohort integration checks and graph/commit work.

The selector asserts five NPTHs and takes the final two in the source-confirmed order: center hole, two V1 stabilizers, common upper socket hole, face-specific lower socket hole. It depends on the selected configuration and current producer fragment order, not stable emitted IDs or manufacturer numbering. The mutation demonstrates that it selects the intended lower drill in this configuration; it does not establish general selector robustness. Upper angle `r` and lower angle `195° + r` on both faces remain producer configuration regressions only; circular-hole angles establish no physical fit property.

The current bound `boardstudio-ts-boardstudio2` impact for producer `body` returned UNKNOWN, zero resolved callers/processes and lower-bound coverage with 25 receiver-typing gaps. Actual source confirmed registration and `spec.body` dispatch. Index metadata was at `2cff220e`, one commit behind the corrected verification HEAD; no clean-current-index or unused-symbol claim follows, and the verifier did not refresh it.

The upper-hole manufacturer diameter, undimensioned RIGHT land, host polarity/numbering, mounting face, reversible connectivity, absolute datum and pad gaps remain unqualified. Model checks establish emitted binding/configuration only; independent model datum/alignment/contact, PCB thickness range, switch travel, keycap/plate fit and physical/full asset qualification remain open. Ticket 118 remains open, and later recipe/assembly integration retains its 19/20/44 dependencies. No defaults, models, recipes or production geometry changed. Ticket 129 is resolved only for its corrected bounded acceptance.
