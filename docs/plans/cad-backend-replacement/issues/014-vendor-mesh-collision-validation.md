# 014: Validate mesh collisions and clearance with tolerance bands

Status: claimed
Owner: /root/luna_collision_scope_014 (private candidate; final integration waits007)
Type: build
Blocked by: 007: Complete vendor STEP asset import
Related: 010: Worker and browser integration; 013: Generated case and PCB geometry

## What to build

Use pure-Rust `parry3d-f64` mesh queries for basic vendor-part collision validation against other parts and generated-case/PCB meshes. Report intersection, closest clearance and contact/witness locations with part/occurrence identity and an explicit tolerance band. Distinguish mesh observations from a supported source-geometry conclusion; insufficient error or topology information returns uncertain, never a fabricated clear result.009 already owns mesh keycaps, so no second keycap ticket is needed.

## Acceptance criteria

- [ ] Agree the mesh/query result seam with007/010: millimetres, occurrence transforms, mesh revisions, source-face/triangle provenance, known versus unknown error bounds and non-finite/invalid input errors. Independent fixture work may proceed before007; final integration consumes its real output.
- [ ] Qualify intersection, distance/clearance and contact/witness queries on the actual chosen `parry3d-f64` version for native/WASM and a pure-Rust dependency graph. Handle unsupported query pairs explicitly; no assumed triangle-mesh pair API or sampled substitute may silently weaken the result.
- [ ] Cover separated, touching, crossing, contained-without-surface-intersection, open/nonmanifold, degenerate and transformed multi-occurrence inputs with independently known expectations. A surface-only miss cannot prove absence of solid containment. Do not infer penetration depth or a unique contact point from a zero unsigned distance.
- [ ] Report conservative pair tolerance bands from applicable authenticated source uncertainty, source-to-mesh approximation, repair displacement and numerical error without double counting. Unknown or one-sided-only coverage cannot be relabelled a certified symmetric bound. Report contact-location uncertainty as well as clearance uncertainty.
- [ ] Use supported tolerance bands for default basic checks and refine only on demand. Reserve0.01 mm for an explicit tighter accuracy request; return uncertain when the qualified pair budget cannot support it. Keep requested accuracy distinct from the physical clearance threshold. Do not force refinement for display admission, claim unsupported accuracy or accept incomplete source geometry.
- [ ] Verify analytic cube/sphere or equivalent independent fixtures, error-band boundary cases, retained part/face provenance, native/WASM and both review axes.010 delivers the results through the actual worker/browser validation flow with cancellation and revision checks.

## Comments

2026-10-17 user approved explicit per-file vendor preview repair budgets. Retain declared precision, chosen repair limit and measured actual movement separately. A requested repair budget is not a source/mesh/query accuracy certificate; report supported collision bands and uncertain outcomes, accounting for qualified actual repair contributions without double counting.

2026-10-17 checkpoint review: [Spec](../../../../.evidence/vendor-collision-014/spec-review/REPORT.md) and [Standards](../../../../.evidence/vendor-collision-014/standards-review/REPORT.md) clear the bounded private raw-surface query adapter. Seven native checks and WASM compilation pass; provenance and unsupported/uncertain paths are retained. Degenerate contact normals may be zero and local Contact.dist is not a global penetration/clearance certificate. Ordinary Rust build scripts in transitive dependencies are not native links. Supported source bands, eligible containment, real007 and010 production integration remain open; no014 completion claimed.

2026-10-17 implementation: Root agreed the [private query seam](../../../../.evidence/vendor-collision-014/candidate/SEAM_PROPOSAL.md), narrowed to007's already-placed world-mm f64 triangle soup and face-occurrence tags. The owner implements actual Parry observations/witness provenance and invalid-input/uncertain controls first; no new scene/pose/cache framework or caller certification boolean. This bounded checkpoint is not full014 completion: actual source error budgets, eligible containment decisions and007/010 integration remain required.

2026-10-17 collision decision: User approved the research recommendation: supported tolerance bands by default, on-demand refinement and0.01 mm as an explicit tighter check. Insufficient accuracy stays uncertain. [Primary-source research](../../../research/cad/vendor-collision-validation-2026-10-17.md) distinguishes triangle-surface observations, containment eligibility and error-bound certification; no library result itself certifies source-to-mesh accuracy.

2026-10-17: User explicitly requested this additional collision-validation slice, including `parry3d-f64`, intersection, clearance, contact location and tolerance bands.009 is redefined in place as the mesh-keycap slice. Root owns integration and the target decision; a bounded research assignment owns only its note/evidence, not production implementation.

2026-10-17: Existing007 [cost separation](../../../../.evidence/production-step-007/collision-target-cost-scope-v1.md) shows no measured0.01 mm analysis cost: current previews perform none. Source completeness, missing faces, master/trim fidelity and method topology requirements remain blockers at any target. Finer certified analysis and combined pair-budget accounting are additional work; a source connectivity declaration is not automatically a physical surface-error bound.
