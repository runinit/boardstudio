# 119: Qualify the existing Kailh MX hot-swap socket

Status: implemented (awaiting verification)
Type: build
Blocked by: none
Parent: [118](118-sofle-adaptation-hardware-contract.md) · Evidence: [hardware candidates](../audit/sofle-adaptation-hardware-candidates.md)

## Bounded concern

Qualify only the existing `ceoloide/switch_mx` hot-swap socket configuration against Kailh **CPG151101S11-16**, drawing **KHPS220643 Rev A**. This is independent of mirror 19, recipe 20 and profile 44. It does not qualify the complete MX switch/keycap assembly, either complete Sofle half, electrical operation or physical manufacture. Preserve every measured Sofle key pose and width.

Primary source: [Kailh drawing](https://www.kailhswitch.com/Content/upload/pdf/202215927/CPG151101S11-16.pdf?rnd=109). At execution, record the drawing revision, units and retrieved asset hash; distinguish manufacturer dimensions/tolerances from implementation values. Do not assume an existing model filename establishes an exact manufacturer match.

## Proposed file lease and exclusions

Root must grant the exact implementation lease and command window before claim or dispatch:

- New focused test: `footprints/tests/mx_socket_qualification.rs`.
- This ticket and its Outcome/evidence table.

Read-only source: `footprints/src/generators/ceoloide/switch_mx.rs`, `footprints/src/registry.rs`, `footprints/src/generators/mod.rs`, existing public render/geometry contracts and the test pattern in `footprints/tests/library_assertions.rs`. The existing generator remains read-only in this slice. If a manufacturer discrepancy is demonstrated, reproduce it with a failing test and request a bounded geometry/model follow-up lease before changing production code. No new public/member API, registry entry, recipe, model asset, golden, browser component or shared test helper belongs here.

## Acceptance

- [ ] Record exact manufacturer recommended host pad/drill geometry, terminal identities and relevant body/datum dimensions with a source-to-implementation evidence table. Identify which dimensions are recommendations, tolerances or unknowns.
- [ ] Through the public bundled render/resolution boundary, test the selected nonreversible hot-swap configuration on F and B at zero, orthogonal and a representative nonorthogonal Sofle rotation. Assert manufacturer-derived pad/drill dimensions, transformed coordinates, distinct terminal/net identities and effective layers. Do not copy generator constants as independent expected evidence or introduce a second face transform.
- [ ] Check the emitted socket model binding, side/datum and 1.6 mm PCB configuration against available independent evidence. Distinguish a binding assertion from actual solid contact/alignment qualification. Missing source/model evidence remains an explicit output-specific gate.
- [ ] Reuse the existing entry. If all covered dimensions agree, make no production geometry change. If a discrepancy exists, record the expected failing regression and stop for the follow-up lease; do not loosen assertions to preserve the old result.
- [ ] A fresh verifier reproduces the source comparison and tests. In a separately granted temporary mutation lease, perturb one emitted critical pad or drill site and confirm the qualification test fails for the expected reason, then restore and rerun. Changing only an expected assertion is insufficient. No production mutation belongs in the final change. Report any unavailable mutation gate; no whole-board or physical-fit claim follows from a passing test.

## Audit and graph evidence (2026-10-10)

Managed worktree: `/home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2`, branch `codex/sofle-adaptations`, audited HEAD `b9e93fab40e552bf4490d0a812ff57ca3a9c5965`. Central registry read first. Expected dirty paths were the report amendment and new 118 contract; no other changes were observed.

Bound `boardstudio-ts-boardstudio2` query/context identified `switch_mx.rs::body`, `PARAMS` and `SPEC`; process hints were “Body → Lookup” and “Body → Misuse”. Upstream impact returned **UNKNOWN**, zero resolved callers and a lower-bound warning for 25 unresolved receiver call sites. The tool reported the index 30 commits behind; root identified documentation drift, but this is not a clean impact verdict. Direct source confirms dynamic `(spec.body)(&context)` in `Registry::render` and registration through `&ceoloide::switch_mx::SPEC`. Recheck bound impact/freshness before any future symbol edit; root alone refreshes.

The generator already exposes side, PCB thickness, hot-swap/reversibility, pad dimensions and model transforms. The existing `model_frames_follow_the_side_and_explicit_transforms_win` test covers switch model frames, not this exact manufacturer socket comparison. No defect or need for a production source change was established in the read-only audit.

### Independent manufacturer evidence

The manufacturer's official 10-page PDF contains a cover drawing and a nine-page specification; both are Rev A. The drawing title block identifies part CPG151101S11-16, drawing KHA-PG1511-388EN, dated 2022-06-01; the specification identifies KH-PS2206-43, Rev A. Dimensions are millimetres. The retrieved official PDF is 1,003,871 bytes, SHA-256 e41ada68576e65b61fd1a2c34c22ccc549f37a736d1cec9eb2e2aff491d19350. See the cover drawing's recommended PCB layout and the specification's printed page 8/9, section 9 (PDF zero-based page index 8).

| Source fact | Manufacturer evidence | Existing ceoloide/switch_mx output | Qualification treatment |
| --- | --- | --- | --- |
| Board/layout | Recommended PCB layout is top view for a single-face board, T=1.6 mm | pcb_thickness default is 1.6 mm | Confirms the selected config; recommendation is not a board stack qualification. |
| Socket holes | Pair centres are 6.35 mm horizontally and 2.54 mm vertically apart; drill Ø3.00 mm; recommended layout tolerance ±0.05 mm | Two 3 mm NPTH sites; F-side relative vector is (6.35, 2.54) mm and B-side vector is (-6.35, 2.54) mm | Assert drill and pair spacing against independent drawing values; preserve the side reflection as generated. |
| Rectangular host pads | Parenthesized reference rectangle (2.55) × (2.5) mm; a separate ±0.05 mm note is printed beneath the recommended PCB layout | The qualification selects public parameters `outer_pad_width_front=2.55` and `outer_pad_width_back=2.55`; height remains 2.5 mm. Generator defaults remain 2.6 mm for both widths. | Assert the selected output against the drawn nominal with numeric comparison epsilon only. Do not use the nearby layout note as reference-dimension acceptance. The unchanged 2.6 mm defaults remain unqualified against the 2.55 mm reference. |
| Housing | Cover drawing gives body width 11.30 ±0.15 mm, overall length 14.50 ±0.15 mm, and height 1.85 ±0.1 mm | Bundled socket model is named SW_Hotswap_Kailh_MX.stp; no independent source asset/datum match is established | Dimensions do not qualify the bundled solid's alignment, contact, or clearance. Keep those as open mechanical gates. |
| Terminal identities | BOM identifies one nylon base and two gold-plated copper-alloy contacts; drawing has contact/base item callouts, but no host pad numbering or contact-to-net map | Generator emits separate pads "1" and "2" with distinct from and to nets | Test that the public output preserves two distinct terminal/net identities; do not claim that drawing item numbers establish a from/to polarity map. |
| Absolute placement datum / pad-to-hole offset | No dimension ties the recommended layout to an MX switch origin, and the host pad-to-hole gap is not dimensioned | Generator emits absolute local socket coordinates | Absolute socket placement relative to the switch and solid alignment remain unqualified. |

The cover drawing and its dimension callouts are primary evidence; the specification's printed page 8/9 confirms the 1.6 mm single-face layout note. A page-8 screenshot alone is blurred, so the dimension readings above use the clearer cover drawing's recommended PCB layout. The manufacturer does not publish an asset hash for the retrieved PDF; the recorded SHA-256 identifies the downloaded bytes. No test/build/check has yet confirmed the implementation claims in this table.

### Selected Sofle pose evidence

The selected source component is SW26 from the Sofle v2 PCB at pinned upstream commit fb294f7c58d0f91c379a9ef339159d4d056643e9, source path Sofle_v2/PCB/SofleKeyboard.kicad_pcb (SHA-256 e34c4981c039bd5c0ab8613c7454422cae117f0dc320b99cb37fd26592d36cce). Its stored position is x=96.7 mm, y=133.41 mm, rotation=300 degrees. The public-render test supplies that stored pose directly as PartRef; it does not infer a board-origin or heading conversion from the upstream KiCad file.

## Verification and handoff

Read [component onboarding](../../../hardware/component-onboarding.md), the current central lane registry and the applicable Rust skills before implementation. Use the smallest sufficient Luna implementation effort, followed by a fresh Sol 6.1 xhigh verifier. Only claim and commit under root's granted command lease; stage explicit paths and report complete bound graph delta before commit.

Proposed focused command: `cargo test -p boardstudio-footprints --test mx_socket_qualification`. Run the affected `repo`, `lint` and native `test` gates in scheduled foreground windows; production/shared changes require a revised scope and corresponding checks. No browser/build/golden regeneration is expected for a test-and-evidence-only slice. Root owns generated outputs.

### Outcome (Codex Luna 6)

Initial implementation checks passed at `a0ec8456`, but the fresh review rejected the rotation oracle and the application of the layout tolerance to parenthesized land references. The bounded revision now selects the approved public parameters `outer_pad_width_front=2.55` and `outer_pad_width_back=2.55`; tests compare emitted pad sizes and Ø3 mm drills to manufacturer nominals with numeric epsilon only. Generator defaults remain 2.6 mm and are explicitly unqualified against the 2.55 mm reference.

The revised public-render assertions cover front and back at 0°, 90° and 300°, including the pinned SW26 pose x=96.7 mm, y=133.41 mm, rotation=300°. The actual emitted hole vector is transformed with KiCad's -theta/down-Y convention and compared to independent, precomputed vectors. The existing renderer frame assertion remains: stored source y=133.41 is emitted as y=-133.41 while the rotation remains 300°. There is no second face transform in the oracle. Exact pad layer sets, shared F&B copper/mask layers for the NPTH holes, distinct from/to net identities, and the socket model path, offset, rotation and scale at an explicitly selected 1.6 mm PCB are asserted. The land-test name's upstream impact returned UNKNOWN/zero callers on an index one commit behind; `rg` confirmed only the test definition. I initially changed the identifier directly, then restored the indexed spelling and completed the rename through GitNexus after reviewing a preview limited to this test file. The final test bytes match the previously tested draft exactly.

The model assertions qualify the emitted generator binding/configuration only. No manufacturer solid asset/datum evidence establishes exact model identity, absolute model datum, pad-to-hole offset, physical contact or solid fit; those gates remain open. The drawing has no host pad-to-terminal polarity map, so the test preserves distinct net assignments without claiming manufacturer polarity.

Revision validation ran `rustfmt --edition 2024 footprints/tests/mx_socket_qualification.rs`; `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio-footprints --test mx_socket_qualification` passed all 4 tests (0.21 s on the final run); `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo lint` passed both steps in 3.4 s; and `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio-footprints` passed all 77 tests across crate targets (about 3.8 s). Clippy emitted existing vendored cgmath warnings; the configured lint step exited successfully. No production generator, registry, model asset, board recipe or shared helper was changed. The fresh verifier must reproduce the source comparison and tests and use its separately granted mutation lease. Ticket remains `implemented (awaiting verification)`.
