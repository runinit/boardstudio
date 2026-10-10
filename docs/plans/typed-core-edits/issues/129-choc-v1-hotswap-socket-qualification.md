# 129 — Choc v1 socket documented geometry and configuration

Status: claimed
Type: qualification
Parent: [118 hardware contract](118-sofle-adaptation-hardware-contract.md)
Prerequisite: independent plan review passed; root implementation lease not yet granted.

## Scope

Qualify the documented geometry of the existing Kailh CPG135001S30 socket candidate through the public bundled render boundary. Reuse `ceoloide/switch_choc_v1_v2`; do not add a footprint or change production geometry, defaults, models or recipes. This ticket does not qualify the complete switch/keycap/plate assembly. Ticket 118 remains open.

Proposed implementation lease is only new `footprints/tests/choc_socket_qualification.rs` and this ticket. Producer `footprints/src/generators/ceoloide/switch_choc_v1_v2.rs`, registry and model assets remain read-only. A temporary producer mutation requires a separate root lease.

## Independent source evidence

Official source: [Kailh CPG135001S30 datasheet](https://www.kailhswitch.com/Content/upload/pdf/202115927/CPG135001S30-data-sheet.pdf), document KH-PS-1702-35, Rev B. Revision B is dated 2020-10-29 (format revision, ECN-2010-25); PDF modification metadata 2021-08-16 is not a newer drawing revision. Captured PDF has 11 pages, 1,159,992 bytes, SHA-256 `c94a6b45abbbdd456fcc1549a60b4454ad897c027f04dafb26d5797e76061f93`.

Ignored evidence under `.scratch/source-evidence/`:

- `CPG135001S30-data-sheet.pdf` — original bytes.
- `recommended-pcb-layout.png` — printed page 8/10, PDF page 9 (zero-based index 8).
- `profile-dimensions.png` — printed page 3/10, PDF page 4 (zero-based index 3).

The recommended PCB layout states Top View and Single face board T=1.6mm. It dimensions two Ø3.00 holes, horizontal separation 5.00 and vertical separation 2.20. The LEFT rectangular land has explicit horizontal and vertical 2.60 callouts. The right land has no separate dimension or same-size note. These layout callouts have no stated tolerances and are not parenthesized references. Compare nominal geometry with numerical epsilon only; do not infer manufacturing tolerances or undimensioned symmetry.

The separate profile dimensions (9.55±0.1, 6.85±0.1, 3.05±0.1, 1.80±0.1) describe component geometry, not PCB land/drill tolerances. Its 4.75 and 4.65 callouts have no stated ± tolerance. The PDF does not establish host terminal numbering/polarity, mounting face, reversible connectivity, absolute switch/socket datum, a PCB thickness range, or assembled clearance/fit.

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
2. Through public bundled rendering, exercise F and B at 0°, 90° and the stored 120° Sofle pose. Assert the independently sourced hole-pair separation 5.00×2.20 mm and Ø3.00 drills, and only the mapped, explicitly dimensioned 2.60×2.60 mm land. Use numerical epsilon, not inferred tolerances. Expected geometry must not copy producer constants.
3. Verify transformed coordinates with independent fixed expected vectors and the actual KiCad down-Y/negative-angle convention. Compose placement and face transforms once; no second face swap. Check distinct terminal/net identities and exact effective copper/paste/mask layer sets. Electrical mapping checks establish configured identities, not manufacturer polarity.
4. Record the undimensioned right land as an explicit unqualified manufacturer-evidence gap. Any assertion of its emitted size is a configuration regression only, visibly distinct from source qualification. Do not claim complete socket fabrication/physical qualification from this bounded evidence.
5. Check emitted `Choc_V1_Hotswap.step` binding, offset, rotation, scale and selected 1.6 mm configuration. Separate generator configuration/binding evidence from unavailable independent model datum, alignment/contact, switch travel, keycap or plate-fit evidence; retain output-specific gates.
6. If a source discrepancy is demonstrated, record an expected failing regression and stop for a separate root producer lease. Do not loosen expectations, infer tolerance, or change production geometry under this ticket's test-only lease.
7. Fresh independent Sol 6.1 xhigh verification must reproduce this complete bounded acceptance and, under a separate explicit temporary producer lease, demonstrate one meaningful pad/drill mutation fails the intended assertion. Restore exact producer bytes, touch, verify hash and restored green. No retained production mutation.

## Audit and verification gates

Read component onboarding, current central lane registry, agent workflow and applicable Rust skills before implementation. Use Luna 6 high for the geometry oracle, then a fresh independent Sol 6.1 xhigh verifier. Independent PLAN REVIEW precedes implementation authorization; it must inspect actual source pixels and mapping, not trust the source planner's summary.

Read-only audit at clean F HEAD `4b97b59ae50d2c3728e64d36442dd8d0bcbf114e` found SPEC/body in `footprints/src/generators/ceoloide/switch_choc_v1_v2.rs:75`/:92, registration in `footprints/src/generators/mod.rs:32`, and dynamic callback in `footprints/src/registry.rs:238`. Bound graph `boardstudio-ts-boardstudio2` returned UNKNOWN/zero resolved upstream callers, lower-bound 25 receiver-typing gaps, and metadata 35 commits behind. Narrow source confirmed registration/dispatch; this is not safety or current-index proof. Root owns refresh and complete change analysis before commits.

Scheduled checks after root grants a foreground window: scoped rustfmt, `CARGO_BUILD_JOBS=12 BOARDSTUDIO_CHECK_JOBS=1 cargo test --locked -p boardstudio-footprints --test choc_socket_qualification`, `python3 scripts/check.py repo lint` with the same environment, and `cargo test --locked -p boardstudio-footprints`. No browser/full integration batch is required for this test/evidence slice; root schedules cohort integration checks. Asset qualification is independent of tickets 19/20/44; later recipe/assembly integration retains its dependencies and 118/physical gates.

## Outcome

Planning only. Official evidence was captured and inspected. Independent Sol 6.1 xhigh PLAN REVIEW passed after inspecting the actual PDF/pixels, hash/revision, narrow producer blocks and stored pose; its mapping and configuration clarifications are recorded above. No tracked implementation, tests, builds, production mutations or commits have run for 129. Exact implementation lease, checks, graph/commit gate and fresh implementation verification remain pending. Source capture/read and plan-review commands were explicitly released. Ticket 118 remains open.
