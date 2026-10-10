# 119: Qualify the existing Kailh MX hot-swap socket

Status: claimed
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

## Verification and handoff

Read [component onboarding](../../../hardware/component-onboarding.md), the current central lane registry and the applicable Rust skills before implementation. Use the smallest sufficient Luna implementation effort, followed by a fresh Sol 6.1 xhigh verifier. Only claim and commit under root's granted command lease; stage explicit paths and report complete bound graph delta before commit.

Proposed focused command: `cargo test -p boardstudio-footprints --test mx_socket_qualification`. Run the affected `repo`, `lint` and native `test` gates in scheduled foreground windows; production/shared changes require a revised scope and corresponding checks. No browser/build/golden regeneration is expected for a test-and-evidence-only slice. Root owns generated outputs.

Planning/audit only so far: no tests, builds, browser checks, source edits, index refresh, staging or commits performed for 119. The command window was explicitly released after the audit. Ticket readiness does not grant its implementation lease.
