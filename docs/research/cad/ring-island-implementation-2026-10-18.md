# Connected blind ring-island implementation —006/013

Continuation from the [executed Core reproduction](mechanical-prototype-reachability-2026-10-18.md), `a5571b508`. `232b6560` checkpoints the native expected-reason RED; the earlier actual-provider RED remains in `.evidence/b1-reachability/ring-product-red.log`. Prior PCB fixes, Core requests, emitted bodies and independent analytic oracles are preserved. Only the reproduced opening interaction is implemented; no ticket closes here.

## Exact construction and ownership

[contours.rs](../../../cad/monstertruck/src/layered/contours.rs):383 now retains both positive and negative rings of a void union. It passes that complete void profile through the **existing** `subtract_profile`/`intersection` composition, then applies the complete authored cutter. `from_rings` still validates the rings and assigns their nearest owners; source-line/arc lineage still passes through `pair`/`retain_lineage`. No trimming, surface, native assembler, exporter, API, dependency or tolerance change is needed.

For material region `R = outer − existing holes`, an intermediate void union `U` contains only existing holes and possibly the new cutter `C`. Thus `(R − U) − C = R − C`. Crucially, `U`'s negative rings are retained holes **in the void profile**, so the existing profile subtractor restores `R ∩ each void-hole` as material, preserving any prior cuts there. They are not discarded or emitted as inward void shells. The source proof is the Boolean result's ordered closed rings/lineage and validated ownership, not a bounding-box inference. Existing coincident/tangent/source-consistency checks remain unchanged.

The existing Z-band assembler connects the island to the floor through shared cap/wall edges. There is no new3D Boolean, polygon approximation, rasterization, mesh substitution, kernel fallback or new sealed-void support. Every authored feature remains applied in its original order.

## Independent geometry controls

[Native tests](../../../cad/monstertruck/src/layered/tests.rs), `adoption_ring_island_*`, use the unchanged [bare fixture/oracle](../../../cad/monstertruck/tests/fixtures/adoption/expectations.json) and [literal actual-Core body](../../../core/tests/fixtures/adoption-reachability.json):

| Input | Expected and checked geometry |
| --- | --- |
| Bare20×16×4mm blank, U+cap blind opening | Cutter union area `48+42−8=82mm²`, depth2mm, material volume1116mm³. One native solid/one consistently oriented closed shell; material probes/bounds, strict tessellation and the existing independent planar Part21 volume/material checker pass. |
| Real gasket bottom, unchanged source-emitted IR | Same82mm²×2mm=164mm³ removal versus the same baseline, Z−3.75→−1.75mm. Floor remains0.5mm. Material at(10,8,−2.75), void at(5,8,−2.75), material under the cut at(5,8,−4)mm. One native solid/one closed shell, geometric consistency and exact STEP with the original eight cylindrical walls retained. Bounds stay(−7.502,−7.502,−4.25)→(107.502,77.502,8.2)mm. Whole-body absolute analytic volume is not newly claimed: the independently known value is the164mm³ difference. |
| Same bare cutters through all4mm | Two **explicit** valid material solids,128mm³ central island and824mm³ outside body; STEP contains two solid/shell records. This follows the existing legitimate post-opening split contract. It does not weaken the separate pre-opening disconnected-feature guard. |
| Additional4×2×2mm pocket inside the island, before or after the ring cutters | Material volume1100mm³, correct pocket/island/floor probes and exact planar STEP. Confirms restoring the island does not refill an already owned void or omit a later cut. |

Only expected-output metadata changes for this ring fixture. All old independent volume/bounds/material oracles, literal requests and emitted `PreparedCaseIR` geometry stay unchanged. The generic actual-provider adoption probe now requires the bare ring to build. The [product provider probe](../../../cad/wasm/tests/layered-adoption-reachability.cjs) requires one STEP solid/shell, exact surface counts/units and independently checks the164mm³ baseline difference, material probes and full bounds from its f64 analysis output.

## Evidence and remaining gates

Logs: `/home/chris/.t3/worktrees/ts-boardstudio2/t3-f6df9ae7/.evidence/b1-ring/`. The Core, CAD and WASM manifests resolve to distinct existing lane targets; no manual builds share a target. Project concurrency/debuginfo settings are retained, and no source copies, custom manifests or targets are placed in evidence.

| Check | Result / receipt |
| --- | --- |
| Native positive bare test before implementation |Expected RED naming `opening-1`/`material island`; `native-red.log`. |
| Four new native geometry controls |Pass on final source in `native-layered.log`; `native-ring.log` retains the earlier focused GREEN before the final explicit complete-cutter application. |
| Affected native layered/case/PCB tests |46 pass; `native-layered.log`. Includes prior cone/junction fixes, ordering/split/sealed-refusal/source-ownership controls, all14 PCB checks and the retained63 generated gasket bodies. |
| Actual public Core reachability replay |4 pass; `core-replay.log`. Exact emitted-body equality pins the native producer → committed IR → CAD test/provider handoff. |
| Actual current WASM rebuild |`CARGO_PROFILE_DEV_DEBUG=false wasm-pack build cad/wasm --target nodejs --dev --out-dir /home/chris/.cache/boardstudio-cad-b1-ring-wasm-pkg --locked` passes; `wasm-build.log`. Previous provider packages are preserved. |
| Rebuilt-provider adoption probe |`actual-wasm-adoption.log`: seven positive outputs and three retained refusals, including all prior PCB cases. Bare ring volume1116mm³, unchanged bounds/material expectations, `meshClosedOriented:true`, accuracy bound still unknown. |
| Rebuilt-provider product construction/STEP and preview observations |`actual-wasm-build-material.log`: one solid/shell, eight cylindrical walls, exact-plane/cylinder STEP in millimetres, all three product material probes pass. Preview-mesh baseline40753.36177779459mm³ versus40589.36177779459mm³ gives exactly164mm³ removal. These Float32 preview observations are not a STEP/analysis accuracy certificate. |
| Rebuilt-provider strict f64 product probe |Pass; `actual-wasm-product.log`. Exactly164mm³ removed, observed remaining volume40583.337010385345mm³, required material probes and full bounds pass. Top observed Z8.200000000000001mm satisfies the unchanged1e−8mm bounds comparison. `meshClosedOriented:false`, `analysisErrorBoundMm:null` remain qualification limits. The first10-minute attempt timed out while the large native run was active (`actual-wasm-product-timeout.log`); a30-minute-bound retry after native completion passed without changing any assertion. No speed comparison or product performance limit is inferred. |
| Production CAD Clippy and changed-file checks |`cargo clippy --manifest-path cad/monstertruck/Cargo.toml --locked --lib --no-deps -- -D warnings` passes (`cad-clippy.log`). Rustfmt, JavaScript syntax and diff checks pass. All previous test/helper names and old geometry oracles are retained; doc links add no failures to the286 existing missing-evidence diagnostics. |

This slice does not qualify the generated analysis accuracy, collision bands, vendor geometry, independent external STEP interoperability or browser/download delivery. The product's f64 edge-incidence flag is false even though the exact native B-rep passes closed topology and geometric consistency; no mesh welding, tolerance relaxation or metadata rewrite is attempted. The generated-analysis/014 owner retains that qualification gap. Full migration acceptance, final CI including ZIP/security, and the parent's once-per-completed-slice Standards/Spec review remain open. No full browser/performance/C++ suite or review is run here. Parent integration that changes Core preparation, contour/topology, the exporter or mesher requires the focused native/source replay and rebuilt-provider probes again.

Cone-wall and enclosed round-seat production reachability remain unproved by the retained bounded hardware probes. The reachable sealed polygon opening still receives the named sealed-shell refusal permitted by013; no inward-shell assignment or feature exclusion is introduced. These dispositions are unchanged by fixing the floor-connected ring island.
