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
| Rectangular host pads | Parenthesized reference rectangle (2.55) × (2.5) mm; a ±0.05 mm tolerance note is printed directly under the recommended PCB layout | Pad 1 is 2.6 × 2.5 mm; pad 2 is 2.55 × 2.5 mm | Preserve the reference-dimension notation and the separate layout tolerance note. The pad-1 width is 2.6 mm against 2.55 mm reference, at the note's upper bound if it applies. The test uses that printed layout tolerance; verifier must confirm this interpretation. |
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

Added four public bundled-render tests for recommended hole spacing and drills on front/back at 0°/90°, the stored SW26 Sofle pose, land dimensions against the printed layout tolerance, distinct from/to nets, effective copper layers and the existing socket model binding. The evidence table records the parenthesized 2.55 × 2.5 mm reference dimensions separately from the ±0.05 mm note printed beneath the recommended layout. Verifier acceptance of applying that layout note to the parenthesized references remains pending.

The focused qualification command, with CARGO_BUILD_JOBS=12 and BOARDSTUDIO_CHECK_JOBS=1, passed all 4 tests: cargo test --locked -p boardstudio-footprints --test mx_socket_qualification (0.12 s). The first run found two fixture compile errors, corrected in the new test. The next run exposed that public rendering serializes the upstream SW26 y=133.41 pose as y=-133.41 while retaining rotation 300°; the fixture now records this observed frame conversion, and the final focused run passed.

Follow-up gates passed: rustfmt ran only on this test file, then CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py repo lint passed both steps (49.9 s total; the configured rustfmt check passed). CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio-footprints passed 77 tests across the crate's targets (7.6 s). Clippy printed existing warnings from vendored cgmath; the configured lint step exited successfully.

No production generator, registry, model, board recipe or shared helper was changed. Browser/build/full-workspace test gates and the fresh-verifier mutation check remain unrun; they are outside this bounded test-and-evidence scope. A fresh verifier must reproduce the source comparison and tests and use its separately granted mutation lease. Absolute socket-to-switch datum, host pad-to-hole offsets, physical contact/net mapping and solid alignment remain open evidence gates. Ticket remains implemented (awaiting verification) pending that independent review.
