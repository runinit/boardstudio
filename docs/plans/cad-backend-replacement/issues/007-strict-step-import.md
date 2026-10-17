# 007: Integrate complete vendor STEP asset import

Status: claimed
Owner: /root/sol_production_step_007
Type: build
Blocked by: None

## What to build

Import vendor STEP into the production Monstertruck backend for display, alignment and basic collision validation using tolerance-bounded meshes, with correct units, assemblies, placements, bounds, immutable original bytes and source-face information. Exact vendor solids are not required; exact generated cases/PCBs and their STEP export remain separate.014 owns collision queries and reported tolerance bands.

## Acceptance criteria

- [ ] Preserve the existing input-size/byte policy and all source occurrences, geometry, placement and trim information; reject incomplete imports, including missing parts or source faces.
- [ ] Assume millimetres only when STEP units are absent; retain authenticated unit conversion and assembly placement checks.
- [ ] Port from qualified APIs rather than a gate executable, retain source bytes for downstream assets, and return mesh/bounds through the backend import boundary.
- [ ] Keep original source geometry and face/occurrence associations distinct from derived meshes and repairs; retain triangle-to-source-face provenance.
- [ ] Bound repair by an explicitly selected per-file preview repair budget, or authenticated source precision by default, in the correct units/placement. Retain declared precision and selected limit separately; record actual movement and approximation error, reject over-budget or incomplete repair, and never invent an unselected allowance or drop geometry.
- [ ] Supply014 with per-occurrence geometry/provenance and supported source/mesh/repair error bounds, explicitly marking unknown bounds. Ordinary checks report supported tolerance bands; refine only on demand, with0.01 mm reserved for an explicit tighter check. Insufficient accuracy remains uncertain; a display mesh alone does not certify an error bound.
- [ ] Verify corpus, malformed/oversized/unit/assembly cases with independent references and both reviews. Apply 001+002 to this asset contract, preserve strict completeness and retain exact generated-CAD gates separately.

## Comments

2026-10-17 additional user decision: Explicit per-file preview repair budgets are approved. Declared precision remains the default; a selected larger limit is recorded separately with actual movement, original bytes/source geometry remain immutable, and incomplete or over-budget repairs refuse. [THQ local feasibility](../../../../.evidence/production-step-007/admission-root-v11/feasibility-thq-v1/REPORT.md) proves why the strict0.001 mm default cannot repair every original constraint; it does not qualify a complete repaired import. Carry settings through worker/request/derived-cache identity and keep collision bands independent.

2026-10-17 follow-up: The reviewed three-file lint correction is integrated. Current [combined gates](../../../../.evidence/layered-case-013/root-integration-v5/root-applied/REPORT.md) pass53 affected native controls, WASM test compilation, scoped Clippy and formatting after the162-test native baseline. The [actual-root eight-file baseline](../../../../.evidence/production-step-007/admission-root-v11/REPORT.md) returns complete face inventories for KnobRibbed/KailhMX; three other files refuse unmeshed faces and three THQ files refuse conversion/attachment loss.007 owns first THQ diagnosis; a separate read-only owner diagnoses KnobLow. The original bytes,506 source inputs and fixed library artifact are verified unchanged. Full007 acceptance remains open.

2026-10-17 actual-root integration: [V10 report](../../../../.evidence/production-step-007/root-integration-v10/REPORT.md) records135 native passes and45/45 actual root WASM/Node controls, with unchanged478 source inputs. WASM library/tests compilation and formatting pass. Scoped test Clippy failed on three newly integrated diagnostics; the same owner prepares an additive minimal correction. Full corpus/source-complete preview admission, line-capable worker delivery and certified analysis bands remain open. No application switch or final007 acceptance is claimed.

2026-10-17 collision decision: User approved supported tolerance bands by default and refinement on demand, with0.01 mm as an explicit tighter check. This supersedes the earlier mandatory/default target, without relaxing source completeness, authenticated repair bounds or uncertain outcomes when accuracy is insufficient. Existing imports do not gain an analysis certificate from this decision.

2026-10-17 scope reconciliation: Reassess the0.01 mm target against basic validation using existing evidence; any proposed revision is a product decision, not an implicit relaxation. Current v8 preview controls do no0.01 mm analysis work, so their source-completeness/meshing blockers remain regardless of that target. Preserve v3/v6/v7/v8 freezes and ongoing import ownership. Collision implementation and the tolerance-band proposal are tracked in014.

2026-10-17: User decision reaffirmed: vendor STEP is preview, alignment and collision geometry, with basic collision accuracy as a validation check. Never lose parts or faces; repairs are bounded by declared source precision. Keycaps no longer share exact-geometry requirements with this path.

2026-10-17: User explicitly approved the separate vendor-asset contract and 0.01 mm clearance target. Exact closed-solid/interior-extrema certification is no longer a universal vendor-preview admission requirement. Historical strict-gate failures remain diagnostic evidence, not automatically accepted assets. The owner preserves frozen review-v3 and implements an additive successor; exact generated geometry/export and complete-source requirements remain mandatory.

2026-10-17: User approved this follow-on slice and its blocking edges. Existing 001–004 assignments and evidence continue unchanged. Qualification milestones do not establish production acceptance.

2026-10-17: The frozen private review-v1 has51 native passes and13 mutation kills, but independent Spec review reproduces silent mapped-occurrence loss and accepted source Plane/boundary inconsistency. These block bounded integration as well as final acceptance. The owner is fixing an additive successor; general trimmed interior extrema and composed master/source fidelity remain open. See the [review](../../../../.evidence/production-step-007/spec-review/REPORT.md).
