# Map: Complete CAD backend replacement

## Current vendor-preview policy (2026-10-19)

[ADR-0017](../../adr/0017-forgiving-vendor-previews-with-preserved-source.md) and
[ADR-0018](../../adr/0018-distinguish-collision-observations-from-clearance-guarantees.md)
replace certification and completeness as prerequisites for vendor preview admission.
Show safely usable repaired or partial geometry with explicit warnings/errors, retain
the untouched original, and keep unsupported accuracy and whole-model clearance
uncertain. No usable geometry means a visible terminal failure. Input/resource safety,
units/placements, existing known-budget guards, supported complete-model conclusions
and exact generated case/PCB output remain in force.

The [successor spec](spec.md) and tickets 015–018 govern this change. Older destination,
closure rules, parent 007 and resolved 010/014 criteria are historical where they
require complete or universally certified geometry merely for preview/check admission.
The obsolete 007 owner-path, region-proof and package exits are not prerequisites
for preview admission. Parent 007 remains `claimed`; its body, evidence, acceptance
boxes and current owner work are untouched. Coordinate overlapping source
ownership before implementation.
Ticket 015 reconciles remaining reference-contract and earlier-plan contradictions;
publication is not implementation acceptance or a claim that forgiving delivery works.

## Destination

Replace BoardStudio's entire Cadrum CAD backend with Monstertruck, preserve intended features and accuracy, and remove OCCT/C++ from CAD production, build, packaging and required-test paths. Functional cutover is accepted: standard verification, removal audit, the Monstertruck-only performance baseline and final independent Standards/Spec review have passed.

Completion requires the [feature assessment](../../research/cad/monstertruck-feature-assessment-2026-10-16.md) as amended by the dated scope decisions in current production code, passing required native/WASM/browser checks, a source-bound Monstertruck-only performance baseline, and an actual dependency/build/packaged-artifact audit demonstrating CAD no longer requires OCCT/C++. Exact geometry and exact STEP export are required for generated **cases and PCBs only**, built through [013](issues/013-layered-case-construction.md)'s layered 2.5D path. Preview meshes cannot substitute for those exact solids. Keycaps use appearance/optional-print meshes; vendor STEP follows the [current forgiving-preview policy](#current-vendor-preview-policy-2026-10-19), with explicit incomplete-model diagnostics rather than mandatory complete presentation. Preserve independent accuracy; internal STEP roundtrips alone do not prove interoperability. No general exact 3D Boolean engine is required, and OCCT/C++ cannot remain as a final fallback.

Vendor STEP assets retain immutable original bytes and available original source/face/occurrence information, with correct unit conversion and placement. Assume millimetres when units are absent, not when declared units or placements are invalid. Missing geometry is reported explicitly while safely usable geometry remains visible. Existing declared/default/selected repair budgets and known over-budget enforcement stay unchanged; an unknown complete repair certificate alone is not an admission refusal. Keep declared precision, selected budget and known/unknown actual movement distinct. Basic collision checks may report approximate observations for partial or repaired meshes; supported tolerance bands remain evidence-bound, refinement is on demand and `0.01 mm` is an explicit tighter request. Unsupported accuracy and whole-model clearance from incomplete geometry remain uncertain.

The [qualification record](../../research/cad/monstertruck-qualification-2026-10-16.md) and [collision research](../../research/cad/vendor-collision-validation-2026-10-17.md) document baseline findings and limitations.

The [2026-10-18 acceptance reconciliation](../../research/cad/migration-acceptance-2026-10-18.md#current-update--2026-10-18) preserves historical source-bound findings. Current standard verification passes all nine project steps, including native/WASM/browser/build/security checks. Production supports generated/vendor/mixed collision delivery; unsupported geometry and insufficient accuracy stay Unknown. The [Monstertruck release baseline](../../research/cad/monstertruck-performance-baseline-2026-10-18.md) supplies current-only measurements. Complete per-route 007 repair certification and 001/002 quality exits are non-blocking follow-ons under the user's revised acceptance.

## Scope decisions (2026-10-17, user)

- **Exact geometry:** cases and PCBs only, built via [013](issues/013-layered-case-construction.md): vertical features → authored slice heights → per-band line/arc profiles → direct planes/cylinders/cones → compatible face merge. PCB outline plus drilled holes at thickness is included; cones vary continuously rather than becoming stepped cylinders. Exact STEP export [008](issues/008-step-export-independent-accuracy.md) covers cases and PCBs.
- **Keycaps:** display and optional 3D printing only. Mesh pipeline, with no exact keycap STEP. [004](issues/004-valid-bounded-dish-cutters.md) is off the critical path, and [009](issues/009-complete-keycaps.md) is mesh-only.
- **Vendor STEP:** preview, alignment and basic collision validation ([007](issues/007-strict-step-import.md) / [014](issues/014-vendor-mesh-collision-validation.md)). All 34/34 vendor STEP files import under the approved preview allowance.
- **General 3D Booleans:** no longer on the critical path. [006](issues/006-complete-case-features.md) adopts [013](issues/013-layered-case-construction.md) and closes only demonstrated case/PCB gaps.
- **OCCT/C++ removal ([012](issues/012-final-cutover-remove-cadrum.md)):** unchanged.

## Collision acceptance (2026-10-18, user)

Migration acceptance requires supported tolerance bands for generated case/PCB bodies and qualified vendor geometry classes. The qualification record must name the supported classes, restrictions and supporting fixtures, including positive vendor-pair results. An implementation leaving every vendor pair uncertain does not satisfy this coverage requirement. Unsupported vendor geometry, ineligible containment queries and requested accuracies that cannot be justified remain explicitly uncertain; this decision does not certify unknown bounds or require every vendor geometry class to be qualified.

## Closure and workflow decisions (2026-10-18, user)

Approved from the Opus orchestration/migration analysis (local `.evidence/orchestration-t3code-audit/opus-analysis.md`). None relaxes geometry, allowances, source preservation, the 34-asset contract, the repair policy, collision acceptance, API approval rules or the existing 32 MiB/256 MiB/1,000,000-record caps.

- **Review:** one combined spec+standards reviewer at ticket closure, or when a change touches public API, geometry, allowances or a cross-crate contract. Private storage or test-only slices get the owner's RED/GREEN evidence and root's diff read. Corrections are checked against their finding. The two-agent `code-review` stays available on request and for 012's final review.
- **Tickets:** one short CURRENT block each; history stays in git.
- **007 package 1, option (b):** enforced per-crate local counters with sticky-incomplete state plus borrow-instead-of-clone fixes. Bigint stays on ordinary allocation (documented); the IO and mesher 256 MiB counters are per-crate, not an aggregate or process bound. S1 is delivered; further accounting and packages 2–6 are deferred quality work ([scope](issues/007-strict-step-import.md#current--2026-10-18)).
- **007 exit:** packages 2–6 close against a finite owner-route inventory that package 2 delivers; 007 closes when every listed path terminates as identity, bounded repair or refusal.
- **Strict 007 terminal policy:** packages 2–6 are deferred to an independent quality track. This supersedes the earlier decision to require packages 5–6 before 012. Known repair-limit enforcement, original source preservation and truthful Unknown results remain in place.
- **011:** baseline timings may run now on current source; only operations on hot paths that 007 later changes are re-measured. Prerequisites live in the [011 CURRENT block](issues/011-integrated-cad-performance.md#current--2026-10-18).

## Workflow & Execution Guardrails

Follow the [existing tracker conventions](../../agents/issue-tracker.md) and [AGENTS.md](../../../AGENTS.md):
- Work solo on the critical path by default. The user approved parallel 010/014 worktrees on 2026-10-17: 010 owns web/worker/cache/UI delivery. Remaining007 work owns actual importer repair movement and vendor source/mesh bounds;014 owns generated-body, query/contact and containment qualification, consuming007's evidence. Root owns shared-contract changes, integration and final acceptance; no lane closes the combined gates independently.
- Review per the 2026-10-18 decision above; never review every intermediate commit, slice or lint fix.
- Never create Cargo `target` directories or copy source trees inside `.evidence/`.
- Never generate `PREIMAGES.json`, `POSTIMAGES.json`, `before/` or `after/` file copies, or custom SHA-256 manifest files—Git tracks source state.
- Never pass `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, or `CARGO_PROFILE_DEV_DEBUG=2`.

## Slices & Current Status

- [001: Certify closed STEP surface boundaries and retain trim coverage](issues/001-certify-closed-step-boundaries.md) — `resolved`; the closed-chart, collapsed-pole (v49) and partial closed-spline (v50) regressions pass in one native Track B run, the choc-v41 recovered faces are recorded as diagnostics from the existing receipts, and the joint review with 002 passed. Known `table::read` `source_records` snapshot failure stays with 007 and is filtered, not allowed.
- [002: Enforce shared-edge angular sampling without whole-model regressions](issues/002-preserve-shared-angular-meshing.md) — `resolved`; the invalid-option and failing-edge (THQ C-2 edge 167) regressions pass in the shared Track B run, 001's choc-v41 30-mode comparison serves both tickets, and edge-turn, face-normal and linear accuracy stay distinct.
- [003: Support contained blind and partial-depth prism cuts](issues/003-contained-blind-prism-cuts.md) — `resolved`.
- [004: Construct valid bounded keycap dish cutters](issues/004-valid-bounded-dish-cutters.md) — `wontfix` (superseded by [009](issues/009-complete-keycaps.md)).
- [005: Production case profiles](issues/005-case-profiles.md) — `resolved`.
- [006: Adopt layered cases and PCBs](issues/006-complete-case-features.md) — `resolved`; the layered builder is the production exact path for cases and PCBs. Cone-wall crossing, the fully enclosed round seat and the sealed polygon opening are visible named refusals through the public/Core path; ring-island openings and the PCB cut combinations are supported. Limitation table and preserved-behaviour tests are in its Answer.
- [007: Complete vendor STEP asset import](issues/007-strict-step-import.md) — `claimed`; historical complete-source and repair-certification work remains recorded in the owning ticket. Its admission mandate is superseded by the [current policy](#current-vendor-preview-policy-2026-10-19) and successor 015–018; known safety/budget guards and source preservation remain. No parent claim, criterion or existing work is changed by this map.
- [008: Exact case/PCB STEP and independent accuracy](issues/008-step-export-independent-accuracy.md) — `resolved`; the export suite (52 tests, 10 authored-reference comparisons) and the cone countersink certificate pass in one native run, the FreeCAD receipt is reused with the exporter diff empty, keycap obligations are mapped to 009, and the single Track A Standards+Spec review passed with its findings recorded.
- [009: Mesh keycaps](issues/009-complete-keycaps.md) — `resolved`; display migration, MX and Alps/ChocV1 closed stems, pure-Rust Manifold printable keycap provider, and STL ZIP delivery integrated. All acceptance criteria met; 27 native tests pass; WASM compiles.
- [010: Production caches, worker and browser](issues/010-production-cache-worker-browser.md) — `resolved`; qualification-v2 supported generated/vendor/mixed delivery, source provenance, refinement and production Case/physical-PCB/vendor collision flow are integrated. Actual provider/worker/Runtime/page default and explicit-tight controls, full browser/build gate, native tests and strict lint pass at `01680d84`, recorded in `1abc1b379`. Functional acceptance recorded; deep 007 repair movement and 014 qualification remain separate follow-ons.
- [011: Integrated CAD performance](issues/011-integrated-cad-performance.md) — `resolved`; the source-bound [Monstertruck release baseline](../../research/cad/monstertruck-performance-baseline-2026-10-18.md) covers clean build/size, startup, cold/warm CAD and STEP export, plus sampled process-tree memory. No Cadrum comparison or deep-007 predecessor.
- [012: Final cutover and removal](issues/012-final-cutover-remove-cadrum.md) — `resolved`; functional removal, current standard checks, the Monstertruck baseline and final independent Standards/Spec review are accepted. 001/002 and deep 007 remain independent quality follow-ons.
- [013: Layered exact cases and PCBs](issues/013-layered-case-construction.md) — `resolved`; the layered 2.5D builder is adopted, with 64 layered/PCB native tests passing in one run, the recorded paired performance accepted, named refusals recorded in 006, and the v5 review plus the Track A review covering the changes.
- [014: Mesh collision validation](issues/014-vendor-mesh-collision-validation.md) — `resolved`; supported generated/vendor/mixed source bands, exact query intervals/contact boxes and qualified slab decisions are delivered through 010 at `01680d84`. Unsupported geometry, insufficient accuracy and unproved global containment remain Unknown; supported slab decisions do not establish general solid containment. Deep 007 repair-movement accounting remains a separate follow-on.
- [015: Reconcile forgiving-preview policy](issues/015-reconcile-forgiving-preview-policy.md) — `claimed`; documentation changes integrated, repository check and source-ownership coordination pending. Parent 007 remains untouched.
- [016: Show partial meshes with errors](issues/016-show-partial-meshes-with-errors.md) — `ready-for-agent`; blocked by 015. Actual provider-to-page surviving geometry plus an explicit incomplete-model error.
- [017: Recover conversion and presentation losses](issues/017-recover-local-conversion-and-presentation-losses.md) — `ready-for-agent`; blocked by 016. Extend safe partial recovery and preserve original-byte round-trips.
- [018: Show approximate collision observations](issues/018-show-collision-observations-for-incomplete-previews.md) — `ready-for-agent`; blocked by 016, not 017. Useful observations with explicit uncertainty and no false whole-model clearance.

## Closure rescope (2026-10-18, user)

The remaining claimed tickets asked for universal certification of code that is already integrated and tested. They now close on existing evidence, named limitations and one combined review per track, following the approach used for 007's import validation ([step-validation-simplification](../step-validation-simplification/map.md)).

- **Track A, generated construction and export:** [013](issues/013-layered-case-construction.md) → [006](issues/006-complete-case-features.md) → [008](issues/008-step-export-independent-accuracy.md). 013 closes on its recommendation, without waiting for 006 to implement every limitation. 006 closes on one adoption decision; cone-wall crossing, the fully enclosed round seat and the sealed polygon opening are named refusals, not missing features. 008 closes on bounded fixture checks, the reused FreeCAD receipt and one cone export check.
- **Track B, importer and mesher quality:** [001](issues/001-certify-closed-step-boundaries.md) and [002](issues/002-preserve-shared-angular-meshing.md) close together on one shared baseline comparison. Recovered-face volume changes are diagnostics; mesh volume on open shells is not an acceptance measure.
- **One combined Standards+Spec review per track,** not per ticket: Track A's review runs in 008 and Track B's in 001.
- **Reuse evidence.** Do not rerun the 34-asset census, FreeCAD, WASM or browser gates for unchanged code. Run one native pass of the affected suites at the current commit, and investigate only rows that changed.
- **Not built:** a general 3D Boolean engine, a general B-rep or STEP validator, a new sampling framework, or implementations of the named refusals.
- **Executed 2026-10-18:** all five tickets close on the single native pass per track and the two combined reviews recorded in their `## Answer` sections.

The tracks are independent. Each ticket's original criteria remain under "Superseded criteria" for traceability.

## Rescope (2026-10-18, user)

010 and 014 are resolved on functional/qualified-band acceptance; unsupported or insufficient-accuracy outcomes remain Unknown. 011 measures current Monstertruck only, with no Cadrum baseline requirement. 012 is resolved on actual removal/build/artifact evidence and standard final verification. 007 packages 2–6 and 001/002 quality follow-ons remain explicit and non-blocking.
