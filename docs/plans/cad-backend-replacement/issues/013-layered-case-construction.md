# 013: Build exact cases and PCB boards through layered 2.5D construction

Status: claimed
Owner: /root/sol_layered_case_013
Type: prototype
Blocked by: None
Related: 005, 006, 008, 003, 011

## What to build

The priority exact-construction path for generated cases and PCB board solids: normalize vertical features, partition authored heights, evaluate line/arc profiles, assemble exact B-rep planes/cylinders/cones directly and merge compatible faces. Deliver a reviewed private prototype and adoption recommendation for006, with exact STEP coverage in008. Include PCB outline plus drilled holes at authored board thickness. No general3D Boolean or exact keycap prerequisite is required; production migration remains006/010/012.

## Why

Every case operation in `build_region` / `apply_features` / `apply_openings` (`cad/wasm/src/model/construction.rs:136–491`) is a Z-axis feature: a 2D profile or circle extruded between two heights. The operations are:

- base extrusion;
- cavity, gasket, mount-hole, round-seat, conical-seat and opening subtractions;
- boss and support-prism fusions.

None is tilted, filleted or lofted. The current port reproduces this as a chain of general 3D Booleans. That chain is where the open failures are:

- duplicate intersection segments on cylinder cuts (006 private fix);
- solids touching exactly along a face;
- conical cuts.

A solid built only from Z features has horizontal profile bands, with exact continuously varying circles for conical features. Building its boundary directly removes general3D surface-surface intersection from cases and PCB boards. Keycaps009 use a separate mesh pipeline.

## Proposed flow

1. **Normalize** each feature to `{ profile (lines + arcs), z_bottom, z_top, add | remove }`. Circles become exact arc loops. A cone becomes a coaxial feature whose radius varies linearly with z.
2. **Slice heights:** collect and sort authored z bounds without tolerance-merging distinct depths. Reject a positive height whose represented top equals its bottom, naming the feature. Retain continuously varying cone radius and account for any profile-topology event within a band; unsupported events must not produce a wrong constant section.
3. **Layer profiles:** evaluate active line/arc operations in their authored/legacy-safe order. In particular, do not move earlier mount holes past later overlapping bosses unless the existing safe-reordering proof applies. Preserve all output loops and material ownership; a cone uses a varying exact profile, not a stepped cylinder.
4. **2D validation,** reported with feature ids:
   - invalid, self-crossing or self-touching contours, finite coordinates and output-loop ownership;
   - material connectivity across the3D stack, including legitimate post-opening splits and enclosed void-shell ownership; per-layer disconnection alone is not invalid;
   - existing authored wall/roof and support rules, without inventing a general minimum-wall policy;
   - a cone crossing a vertical wall where not yet supported. Reject with the feature id and record the adoption gap; no general Boolean fallback is assumed.
5. **Direct B-rep assembly:**
   - side walls come from profile edges × layer span (plane or exact cylinder; cone face within a cone band);
   - horizontal faces come from the differences between consecutive layer profiles;
   - edges and vertices are shared by construction.
6. **Merge** coincident coplanar or cocylindrical faces across layers, so a through-hole wall is one face.
7. **Output** exact solids to mesh, STEP, bounds and material paths through existing prepared-case contracts. PCB prototype inputs reuse a plate region at board thickness with native round drills from authored drill data, not circles reconstructed from polygon preview holes. Record any source-contract gap before production adoption.

## Scope of this ticket (prototype)

- Evaluate a pure-Rust 2D line/arc Boolean for step 3. The candidate is `cavalier_contours`. `i_overlay` is polyline-only and would approximate circles, which is not acceptable for exact output. Record version, licence, robustness on tangent and coincident arcs, and WASM build.
- Implement steps 1–6 behind a private seam for these features:
  - base with holes;
  - cavity;
  - boss with through-hole;
  - round seat;
  - conical seat;
  - opening.
  - PCB outline, polygonal cutouts and exact circular drilled holes at board thickness.
- Retain the existing polygon outline contract and identify any missing authored arc/slot data needed for complete PCB output. Never silently polygonize an available exact circle/arc.
- Compare the layered case build with the current Cadrum build using identical prepared inputs and equivalent output/check settings. This bounded comparison complements011's later integrated measurements.
- No production cutover in this prototype. Cadrum remains production until006/010/012 acceptance.

## Acceptance criteria

- [ ] **Fixtures** each produce one consistently oriented solid with verified closed topology and shared-edge incidence, with no general3D Boolean calls:
  1. the 006 round-seat failure case;
  2. a boss with a through-hole;
  3. a round seat flush with the plate top (touching-face case);
  4. a countersink inside material;
  5. a cavity plus an opening through a wall;
  6. a PCB with a nonconvex outline and multiple exact circular drilled holes at authored thickness.
- [ ] **Volume** matches an independent value, `Σ layer 2D area × layer height` for constant profiles and the analytic area integral for varying-radius cone bands (`πh(r_bottom² + r_bottom r_top + r_top²)/3` for a frustum), computed from analytic circle and polygon geometry rather than from the built solid. Tolerance is stated per fixture; conical bands must not become stepped cylinders.
- [ ] **Bounds and material probes** (points known inside or outside) agree with the expected geometry.
- [ ] **Review regressions:** preserve interleaved overlapping mount semantics; represent or explicitly reject sealed voids rather than exporting them as positive bodies; reject unrepresentable positive spans; preserve valid post-opening material splits.
- [ ] **Countersink rejection:** a countersink crossing a vertical wall is rejected with the feature id. Record it as an adoption gap; neither silent wrong geometry nor an unqualified general Boolean fallback is accepted.
- [ ] **Faces:** a through-hole wall is one cylindrical face after merge, and circles are exact (not polygonal).
- [ ] **PCB:** outline/cutouts plus round drills retain thickness, positions and radii, exact cylindrical hole faces, independently computed area-minus-holes times thickness, bounds/material probes and closed topology through mesh and STEP.
- [ ] **Builds and reviews:** native tests, WASM library build and independent Standards/Spec reviews pass on the fixed source. The2D library evaluation is recorded in `docs/research/cad/`.
- [ ] **Performance:** measure comparable Cadrum versus layered case builds on identical retained prepared inputs, source/artifact revisions, machine/load, build profile, output options and cold/warm conditions. Report elapsed time, sample count and memory where available; do not claim speedup from unlike or failed builds. No arbitrary speed threshold is invented.
- [ ] **Recommendation** on006 production adoption for cases and PCBs, with the complete feature inventory and remaining unsupported interactions/source contracts. Name any actual feature this approach cannot express before proposing residual006 work; general3D Boolean capability is not a prerequisite by itself.

## Decision needed

User approval is recorded below.013 is now the priority exact-construction route for cases and PCBs. Adoption into006 remains gated by the recommendation, independent review and applicable acceptance; keycaps and vendor assets use the separately approved mesh contracts. No general exact Boolean fallback is required for012.

## Comments

2026-10-17 current gate: [Root integration report](../../../../.evidence/layered-case-013/root-integration-v5/root-applied/REPORT.md) binds162 native baseline passes including all63 internal-gasket bodies, then53 affected passes after the reviewed equivalent four-file lint/format follow-up. Actual full WASM library/tests compile, scoped test Clippy and formatting pass with506 final inputs unchanged. Fixed current library artifacts and exact five-package lock audit are retained. The source-backed arc/slot/all-drill PCB producer/consumer remains private006 work; no application switch or full013/006 completion is claimed.

2026-10-17 v5 integration: Both axes clear the [v5 successor](../../../../.evidence/layered-case-013/builder/v5/HANDOFF.md) and corrected public registration. The first public-test proposal incorrectly accessed opaque model internals; v2 uses only public build/mesh/STEP. The review's alleged fixture-newline drift was retracted after actual Git replay proved all19 original fixture/source postimages exact. Root has applied the reviewed19 paths; [lock audit](../../../../.evidence/layered-case-013/root-integration-v5/root-applied/LOCK-AUDIT.json) proves exactly five proposed dependency additions and unchanged132 old package versions/sources/checksums. Actual combined native/WASM checks are in progress. Curved source-PCB and remaining006 interactions still gate adoption; no app cutover is inferred.

2026-10-17 v4 checkpoint: [Frozen handoff](../../../../.evidence/layered-case-013/builder/v4/HANDOFF.md) binds29 native checks, all63 actual internal-gasket bodies, actual WASM codegen, strict library Clippy, formatting and six killed mutations. The separate native revolution frame correction is both-reviewed and [root-integrated](../../../../.evidence/monstertruck-qualification/vendor-integration/native-revolution-v47/REPORT.md):49 actual shared native tests, WASM library/tests and strict scoped modeling Clippy pass. A separately retained rotated half-cylinder notch RED exposed recomputation of an exactly shared corner; [read-only diagnosis](../../../../.evidence/layered-case-013/rotated-cap-diagnosis/REPORT.md) identifies the backtracking edge. The same owner closes that and authored-plane merge in an additive v5 before combined review against last-reviewed v3. No full013/006 adoption or source-arc/slot coverage is inferred.

2026-10-17 continuation: Both review axes clear [v2 fixes](../../../../.evidence/layered-case-013/builder/v2/HANDOFF.md) and [v3 PCB successor](../../../../.evidence/layered-case-013/builder/v3/HANDOFF.md):21 native checks, actual WASM codegen, strict library Clippy and formatting pass. V3 remains frozen while the same owner closes holed gasket and actual Core support/seat composition gaps in a separate v4. A reviewed ignored Cadrum full-case comparator is applied; [measurement protocol](../../../../.evidence/layered-case-013/performance/root-run-v1/PROTOCOL.md) requires five alternating pairs per workload and verified workload/source/binary digests. That build and the paired run are now complete; see the subsequent performance comment. Production adoption remains open.

2026-10-17 scope reconciliation: The first frozen [builder handoff](../../../../.evidence/layered-case-013/builder/HANDOFF.md) remains immutable. Independent [Spec review](../../../../.evidence/layered-case-013/spec-review/REPORT.md) found mount reordering, sealed-void ownership and unrepresentable-height defects; [Standards review](../../../../.evidence/layered-case-013/standards-review/REPORT.md) found an unnecessary public re-export. The same owner continues additive fixes and PCB work, with review of subsequent changed files before integration. Root coordinates comparable performance; no duplicate builder or restarted assignment.

2026-10-17: User decision: PCB board solids (outline plus drilled holes, extruded to thickness) are in scope alongside cases; the board already flows through the case builder. If adopted, 013 is the exact construction path for all generated geometry that requires exact STEP.

2026-10-17: User explicitly approved this approach and directed the private prototype to start. Status moved to ready-for-agent. Preserve the existing Boolean candidates and production Cadrum path. The independent cone-volume expectation uses the analytic varying-area integral, and slice-height normalization must not silently change authored geometry. These clarify exactness without changing the approved prototype scope.

2026-10-17: Claimed by `/root/sol_layered_case_013` for the private builder. `/root/luna_layered_contours_013` owns the separate primary-source 2D-library evaluation and bounded line/arc probes. Root retains ticket/map and shared integration ownership. Existing006 general Boolean and009 keycap work continues; no production adoption or source overlay is authorized by this prototype.

2026-10-17 performance result: [Five alternating pairs per fixture](../../../../.evidence/layered-case-013/performance/root-run-v1/REPORT.md),20 successful invocations, matched fresh optimized native executables and exact prepared inputs. Frozen-v3 round-seat medians: layered0.818 ms/Cadrum9.326 ms; flush-seat0.184/1.836 ms. Build-only, two fixtures; no qualified memory or integrated011 claim. The optimized functional preflight accidentally rewrote seven emitted STEP header timestamps; three exact originals were restored, four original full-file hashes remain unavailable. Complete geometry bytes and source358/input/binary receipts are unchanged. [Incident and retained original/current hashes](../../../../.evidence/layered-case-013/performance/root-run-v1/OUTPUT-DRIFT-NOTE.md). V4 moves emission behind an explicit output directory; its final reviewed source-bound gates and adoption recommendation remain required.

2026-10-17 PCB source loop ownership integrated: [Reviewed seven-path root continuation](../../../../.evidence/production-case-006/root-pcb-loop-ownership-v2/REPORT.md) replaces the blanket multi-loop refusal with source-only analytic line/arc ownership, including disjoint exteriors, nested islands and nearest material owners. Authored supports/IDs stay intact; named crossing/tangent/coincident/self-touch refusals remain. Current-provider module compatibility retains outline holes in the existing centered frame. Three genuine REDs precede fixes. Both reviews pass;146 affected native tests, two actual Chrome Core/Runtime checks, full-page WASM tests, Core strict all-target Clippy/format/registration and all656 baselined integration-name retention pass. The three original raw probes now succeed; two actual Core-emitted nesting/reflection inputs pass bounded closed-mesh/material/volume checks on the authenticated frozen exact-case WASM artifact.716 source inputs stay stable; exactly two Rust packages are added with425 old identities/checksums unchanged. Strict all-target WASM Clippy retains nine unchanged RED sites. This closes the diagnosed producer loop gap; broader source/consumer/export, shipping-provider, performance and removal acceptance remain open. No STEP work resumes.

2026-10-18 B1 adoption recommendation: [Complete producer/refusal inventory and literal fixtures](../../../research/cad/layered-adoption-inventory-2026-10-18.md) preserves the layered approach, mount order, support cutoff, void ownership and authored PCB mappings. The prototype cone-wall refusal remains a real production adoption gap, alongside sealed/multi-ring void and intersecting PCB-cut composition; none is silently out of scope. Two small layered fixes pass affected native and actual-WASM geometry checks. Parent retains completed-slice review and final006/013 disposition; this ticket stays claimed.
