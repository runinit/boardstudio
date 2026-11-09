# 006: Adopt layered cases and PCBs and close remaining feature gaps

Status: resolved
Type: build
Blocked by: 013: Layered case and PCB construction
Related: 008 (Track A closure)

## What to build

Adopt the reviewed013 layered builder for production exact cases and PCB board solids. Close only the concrete feature gaps identified by its adoption recommendation: cavities, openings, gaskets, supports, bosses, seats, outline and drills. Reuse005 contracts and existing validation. No general exact3D Boolean engine, contact/cone/containment hardening or keycap intersection seam is required. A remaining unsupported feature is an explicit blocker or product decision, never a silent omission or fallback to OCCT.

## Acceptance criteria

Rescoped 2026-10-18 (user; see the [map rescope](../map.md#closure-rescope-2026-10-18-user)). Track A, second step. Closes on one explicit adoption decision with named limitations, not on universal feature coverage.

- [x] **Adoption decision.** Under `## Answer`, record that the layered builder is the production exact path for cases and PCBs (integrated through 010/012) and give the limitation table:

  | Interaction | Disposition |
  |---|---|
  | Cone-wall crossing | Named refusal with the feature id (accepted adoption gap) |
  | Fully enclosed round seat | Named refusal |
  | Sealed polygon opening (reachable from Core) | Named refusal; never empty or positive output |
  | Ring-island opening | Supported (integrated) |
  | Edge, intersecting and redundant PCB cuts; contained or height-clipped cones; PCB outline and drills | Supported |

- [x] **Refusals are visible.** Each named refusal is covered by an existing literal refused-feature fixture that asserts a named error through the public or Core path. Add a test only for a limitation without one.
- [x] **Preserved behaviour.** Name the existing tests covering dimensions, feature order, support cutoff, mount reordering and void ownership (the affected layered/PCB suite and Core replays). No new checks.
- [x] Set `Status: resolved` once 013 is resolved, and update the map. The final Track A review in 008 covers 006.

Not required (cut by the rescope): a general 3D Boolean engine, implementing the named refusals, or reading blind-depth language as universal support.

### Superseded criteria (before 2026-10-18)

- Record013's adoption recommendation and feature-by-feature gaps; integrate the reviewed builder without unrelated overlays. If no residual feature needs general Booleans, record that conclusion and do not build that engine.
- Preserve case and PCB dimensions, feature identity/order, body counts, region/material ownership and intended blind/partial-depth behavior. Reordering overlapping mounts requires the existing proof that it is safe.
- Reject self-crossing supports and enforce the written support cutoff exactly through Core and direct CAD inputs.
- Cover full cavities, holed gasket profiles, supports, bosses, openings, flush seats, countersinks and PCB outline/drill inputs with independent analytic volume, material, bounds and closed-topology checks. Preserve legitimate splits and void ownership; errors cannot become empty successful output.
- Preserve native planes/cylinders/cones and authored depths; unsupported cone-wall crossings name the feature and remain an explicit adoption gap until resolved or decided.
- Pass affected native/WASM tests and both reviews; combine with008 export and001+002 tessellation checks. Production caller/cache migration remains010.

## Comments

2026-10-17 root producer prerequisite integrated: Reviewed source-fidelity fixes (`../../../../.evidence/production-case-006/root-pcb-producer-v1/REPORT.md`) preserve absolute raw pad angles, legacy drills, complete source inventory and one-body ownership. Source failures block the whole exact preview while retaining models/findings. Both review axes also clear the current-provider compatibility fix: retain polygon holes, suppress duplicate region mount cuts, and keep body source/mounts authoritative for Monstertruck. Fresh root60 affected native tests pass; all670 native tests compile/register; Core/Case/PCB/Runtime WASM tests compile and changed-file formatting passes. All17 reviewed postimages and694 Core/web inputs remain stable. Eight external STEP edits are preserved outside this check's dependency scope. Direct BoardReference, raw multi-loop ownership, legacy arcs, broader consumer arrangements and full006/010 acceptance remain open; no application cutover, STEP rerun or performance/browser claim.

2026-10-17 root prerequisite integrated: 26 reviewed paths (`../../../../.evidence/production-case-006/root-pcb-integration-v1/REPORT.md`) add compatible source types, initialize all shared literals and install the exact consumer plus independent8π circle certificate. Nine fresh PCB tests, seven Core case/serialization checks and CAD/Case/Host WASM test compilation pass; all177 native names compile, retaining the prior168. The source producer remains separate: raw pad-angle, completeness and body ownership defects are not approved by this consumer integration. The old14°slot fixture validates received IR only. Remaining source variants/arrangements, Clippy/full-suite/browser/performance and full006 acceptance stay open. User cost recalibration stands down private lanes at their current checkpoints; root continues serially.

2026-10-17 source contract agreed: Core and exact-consumer owners proceed privately from the PCB contract (`../../../../.evidence/production-case-006/pcb-adapter/CONTRACT.md`). One optional body-level profile retains authored support/loop identity; existing body mounts carry all round drills, with retained exact oval slots. Rectangle/polygon pieces normalize to source lines; circular radius and signed sweep survive placement without refitting. Unsupported source modifiers/crossing cuts must be named refusals. Actual vik-splitter has38 drills and four midpoint-defined arcs whose authored radii are slightly different; an idealized nominal-radius outline is not an independent exact oracle. Core producer and CAD consumer have disjoint ownership; shared source and acceptance are not yet changed.

2026-10-17 PCB source gap: Read-only adapter assessment (`../../../../.evidence/production-case-006/pcb-adapter/ASSESSMENT.md`) traces the real `vik-splitter` board: four authored Edge.Cuts arcs and36 pad/via drills are flattened before CAD; two mechanical mount holes retain separate metadata but are not wired into PCB case mounts.013 manual native-hole fixtures do not establish this source path. Residual006 work must preserve typed source curves/drills/slots into prepared geometry, avoid duplicate polygon cuts, and reuse the source-backed front/back STEP fixture with independent authored-area/drill expectations. An additive compatible source-contract sketch is assigned; no source/API change or wholePCB exactness claim yet. This is within the approved outline/drill feature scope, not general Boolean work.

2026-10-17 scope reconciliation: General Boolean assignments are stood down; frozen v1–v4 and cone/contact REDs (`../../../../.evidence/production-case-boolean-006/SCOPE-STANDDOWN-2026-10-17.md`) and old adapter (`../../../../.evidence/production-case-006/HANDOFF.md`) remain intact. Retain integrated v45 fixes and already-applied reviewed successors; no new kernel-hardening work is authorized by this ticket. No feature has yet been demonstrated inexpressible in013. Earlier claims that all bosses may precede all holes are superseded by the authored-order regression in013's independent review.

2026-10-17: User decision: generated cases and PCBs must stay exact, built through 013's layered 2.5D construction rather than a general 3D Boolean chain. Retain the integrated v1/v2 vendor fixes. Pause new contact, cone and containment hardening of the general Boolean; this ticket narrows to whatever 013 cannot express, pending 013's recommendation.

2026-10-17: Root integrated the both-reviewed v1/v2 vendor prerequisites for oriented multi-boundary classification and exact local interference-segment deduplication. Fresh shared46 vendor tests and full qualification WASM compile pass; the subsequent notice-only delta also clears both reviews. See v45 integration (`../../../../.evidence/monstertruck-qualification/vendor-integration/boolean-round-v45/REPORT.md`). The private case builder/physical-solid seam and full006 remain unaccepted; cone/contact/valid-empty/containment limits are explicit. The user-approved013 layered-case prototype now evaluates avoiding the general3D Boolean chain for vertical features, with no production switch yet.

2026-10-17: Prerequisite005 is integrated and resolved. Private006 continues against its reviewed exact-model/contracts seam; generated curved Boolean correctness and full feature integration remain open.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.

2026-10-17: Private implementation assigned against the agreed opaque exact-model contract. Integration remains blocked by reviewed005 acceptance; existing application and unrelated changes are preserved.

2026-10-17 legacy PCB arc source gap corrected: Reviewed root integration (`../../../../.evidence/production-case-006/root-legacy-arc-v1/REPORT.md`) fixes the shared board preview parser, which falsely refused closed legacy KiCad arcs and removed compatibility holes despite retained exact drill metadata. A genuine public import RED precedes the fix; ±180° supports/sweeps,1mm drill retention, modern midpoint bounds and malformed-source refusals are independently pinned. Final Spec/Standards pass the four-path successor; one private parent helper preserves arithmetic and existing visibility. Fresh80 native checks pass (12 import+68 artifact, one inherited KiCad CLI ignore); Core WASM tests compile, strict native library Clippy/format/registration pass, and714 scoped inputs remain stable. Direct BoardReference, raw multi-loop ownership, broader case/consumer arrangements and full006/010 acceptance stay open. No STEP work, provider cutover or performance claim.

2026-10-17 direct source contract continuation: use the existing public Core artifact seam for `read-board-source` (verified raw source, a Pose2 placement, and local-X reflection). Return the existing ModuleBoard shape in the placed host XY frame: complete exact profile/slots, every round drill, source thickness and compatibility polygons. No centering or model offsets. Reject incomplete sources as a whole; no PcbPreview geometry reconstruction. Mechanical export retains its snapshot/asset-SHA owner, composes BoardReference placement with physical-half reflection once, and replaces the nominal PCB reference with this source body. Existing module placement math is shared privately with unchanged saved arithmetic; no existing public/member visibility widening. This is within approved006/010, not a new ticket or STEP resumption.

2026-10-17 source-backed PCB mechanical export integrated: Reviewed nine-path root continuation (`../../../../.evidence/production-case-006/root-board-reference-v1/REPORT.md`) adds the existing Core artifact source reader, original accepted-reference/SHA capture and physical placement/reflection once; Runtime replaces the nominal PCB only with complete source geometry and rejects bad bytes, incomplete outlines or thickness mismatch without fallback. Legacy arc midpoint witnesses preserve the authored carrier. Both reviews pass.270 affected native checks pass (one inherited KiCad CLI ignore), four refusal cases pass through actual Runtime/Core Chrome, three literal placed-source inputs pass the authenticated frozen exact-case WASM provider, and full-page WASM test compilation/native strict library Clippy/format/registration pass.715 scoped inputs remain stable. Strict all-target WASM Clippy retains nine unchanged RED sites, unsuppressed. The positive WASM check is frozen-provider compatibility/mesh evidence, not exact-volume/latest-provider or full package/download certification. Raw multi-loop ownership, broader arrangements, shipping activation, performance and removal stay open;006/010 remain claimed and STEP stays idle.

2026-10-17 next PCB source gap diagnosed: Three public Core probes (`../../../../.evidence/production-case-006/root-pcb-loop-ownership-v1/REPORT.md`) confirm the blanket multi-loop guard rejects an annulus, disjoint islands and an island inside a hole, although all seven constituent single loops succeed. No production source/dependency or STEP change. Explicitly owned diagnostic controls reach the frozen exact-case WASM provider and pass independent material/volume/bounds observations. Circular controls fail bit-exact mesh pairing only at measured2.45e−15/4.90e−15mm cardinal residuals; this is not a demonstrated B-rep/Boolean failure or a qualified closure pass, and no tolerance is weakened. Next006 work is source-bound loop ownership with named intersection/tangency refusals and complete emitted-source acceptance; no guard removal or new tracker.

2026-10-17 PCB source loop ownership integrated: Reviewed seven-path root continuation (`../../../../.evidence/production-case-006/root-pcb-loop-ownership-v2/REPORT.md`) replaces the blanket multi-loop refusal with source-only analytic line/arc ownership, including disjoint exteriors, nested islands and nearest material owners. Authored supports/IDs stay intact; named crossing/tangent/coincident/self-touch refusals remain. Current-provider module compatibility retains outline holes in the existing centered frame. Three genuine REDs precede fixes. Both reviews pass;146 affected native tests, two actual Chrome Core/Runtime checks, full-page WASM tests, Core strict all-target Clippy/format/registration and all656 baselined integration-name retention pass. The three original raw probes now succeed; two actual Core-emitted nesting/reflection inputs pass bounded closed-mesh/material/volume checks on the authenticated frozen exact-case WASM artifact.716 source inputs stay stable; exactly two Rust packages are added with425 old identities/checksums unchanged. Strict all-target WASM Clippy retains nine unchanged RED sites. This closes the diagnosed producer loop gap; broader source/consumer/export, shipping-provider, performance and removal acceptance remain open. No STEP work resumes.

2026-10-18 B1 continuation: [Actual-authorable adoption inventory](../../../research/cad/layered-adoption-inventory-2026-10-18.md) traces every layered refusal family to Core/public inputs and prior intent. Two bounded RED→GREEN fixes preserve height-clipped exact cones and collinear overlapping openings; literal source/prepared fixtures retain seven valid refused features (real cone-wall cuts, two sealed voids, material-island pockets and three PCB cut combinations). Native/actual-WASM checks pass within the recorded scope. No feature exclusion, new public contract or closure is approved; parent review/integration remains pending.

2026-10-18 B1 PCB checkpoint/scope correction: [Current classification and evidence](../../../research/cad/layered-adoption-inventory-2026-10-18.md#current-scope-correction-and-pcb-checkpoint) supersedes the prior inventory's claim that all seven direct-input refusals establish new production feature gaps. No reopened regression is demonstrated. Three raw-source PCB cut combinations now pass exact native/actual-WASM analytic checks; cone-wall and sealed voids retain their documented prototype dispositions, while the ring-island's specific assembled product reachability still needs verification. Existing fixes/fixtures/evidence remain intact. No other feature implementation, scope exclusion, ticket closure or independent review occurred; parent owns completed-slice review/integration and combined acceptance.

2026-10-18 B1 ring-island integration: The reviewed ring chain is integrated; the only production change is the 15-line `layered/contours.rs` void-union fix (source hashes in013). Blind/partial-depth behavior, feature order and material ownership are preserved; only the ring fixture's expected-refusal metadata changes (`material island`→`null`), geometry and independent volumes unchanged. Fresh combined-parent checks pass: affected native layered/PCB **48/48**, 4 Core replays, scoped Clippy/fmt/JS/docs/repo/diff, and one fresh release/locked/no-opt Node WASM build covering vendor+ring passes the ring product f64 analysis (164 mm³ removed, exact bounds), the bare ring, retained PCB cases, the vendor sliver and the shared generated contract. Analysis accuracy/collision bands, STEP interoperability and delivery remain unqualified; cone-wall/enclosed-seat and the sealed-opening refusal stay unchanged. See [implementation](../../../research/cad/ring-island-implementation-2026-10-18.md); 006/013 stay open.

## Answer

**Adoption decision: adopted.** The layered builder is the production exact path for cases and PCB board solids, integrated through 010/012. This ticket closes on one recorded decision with named limitations; no universal feature coverage is claimed.

| Interaction | Disposition |
| --- | --- |
| Cone-wall crossing | Named refusal with the feature id (accepted adoption gap) |
| Fully enclosed round seat | Named refusal |
| Sealed polygon opening (reachable from Core) | Named refusal; never empty or positive output |
| Ring-island opening | Supported (integrated) |
| Edge, intersecting and redundant PCB cuts; contained or height-clipped cones; PCB outline and drills | Supported |

**Refusals are visible.** Each refusal is asserted by an existing literal refused-feature fixture through the public/Core path; no test was added.

| Refusal | Fixture | Test and asserted reason |
| --- | --- | --- |
| Cone-wall crossing | `cone-wall.json` (`expectedRefusal: "vertical wall"`) | `cone_crossing_vertical_wall_is_rejected_with_feature_identity` — names the feature id and "vertical wall" |
| Fully enclosed round seat | `sealed-round-seat.json` (`expectedRefusal: "sealed"`) | `enclosed_void_is_refused_instead_of_emitting_a_second_body` — refuses the disconnected sealed-void shell. Unlike cone-wall, the message does not echo the feature id; recorded as a diagnostic limit, not a missing refusal. |
| Sealed polygon opening | `sealed-opening.json` (`expectedRefusal: "sealed"`) | `reviewed_sealed_opening_is_refused_with_opening_context` — names `opening-0` and "sealed" |
| Unrepresentable positive span | direct `PreparedCaseIR` | `reviewed_unrepresentable_positive_span_is_refused_with_feature_context` |
| Empty PCB cut result | direct `PreparedCaseIR` | `adoption_pcb_cuts_cannot_return_empty_success` |

Supported controls carrying the same interactions: the six `adoption_ring_island_*` tests, `adoption_pcb_edge_drill_keeps_exact_half_cylinder`, `adoption_pcb_overlapping_drills_subtract_their_exact_union`, `adoption_pcb_drill_within_slot_preserves_the_complete_slot_boundary`, `adoption_clipped_cone_retains_closed_exact_topology_and_native_geometry`, `adoption_overlapping_openings_preserve_collinear_authored_segments`.

**Preserved behaviour.** No new check was written; the affected layered/PCB suite and the Core replays already carry these rules.

| Required rule | Existing coverage |
| --- | --- |
| Dimensions and independent analytic volume/material | `original_round_seat_chain_matches_independent_volume_and_material` (authored depths, analytic volume, material probes), `pcb_polygon_outline_and_authored_round_drill_are_exact`, `pcb_nonconvex_polygon_with_diagonal_and_multiple_drills_is_exact` |
| Feature order and mount reordering (only under the existing safe proof) | `reviewed_authored_mount_order_preserves_later_boss_hole` |
| Support cutoff | `core/tests/gasket_features.rs::support_owner_cutoff_is_exact_and_strict_for_prepared_inputs` (unchanged Core contract, strict at 1e-8 and permissive just below; not rerun in this pass) plus the direct CAD composition `prepared_wall_support_and_round_seat_preserve_clipped_circular_wall` |
| Void ownership and source holes | `enclosed_void_is_refused_instead_of_emitting_a_second_body`, `source_contour_hole_ownership_is_preserved`, `source_declared_owner_is_not_silently_reassigned_to_nested_island` |
| Core → CAD producer parity | four replays in `.evidence/b1-ring-integration/parent-core-replay.log` |

**Evidence.** One native pass at this commit: 64 layered/PCB tests pass with 0 failures; 360 passed / 0 failed / 2 ignored overall for `cad/monstertruck`. The final Track A Standards+Spec review runs in [008](008-step-export-independent-accuracy.md) and covers this ticket.
