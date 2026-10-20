# 012: Proof modules live under qualification

Status: resolved
Type: build
Blocked by: [011: Kernel tests share one prepared-case fixture](011-kernel-tests-one-fixture.md)
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)
Related: [004: Qualification owns mesh with evidence](004-qualification-mesh-with-evidence.md)

## What to build

See the proof-module decision in [006](006-review-follow-ups-spec.md). This is the part of 004
that was left undone; the user approved it.

## Acceptance criteria

- [x] `final_trace`, `generated_qualification` and `cylinder_proof` live under the qualification module.
- [x] Profile meshing takes an observer trait, not `AnalysisTrace`, and names no qualification type.
- [x] `prepare_generated_analysis` and `prepare_vendor_analysis` keep their signatures.
- [x] `SourceBound` is renamed so it does not read as a sibling of `WorldSourceBound`.
- [x] Kernel (290 passed, 1 ignored) and CAD worker (57) results are unchanged.
- [x] A combined spec and standards review is run, per the CAD migration review rule in `AGENTS.md`.

## Comments

- Layout: `final_trace.rs` (with `final_trace/{cylinder_proof,retained}.rs` and `PROOF.md`) and
  `generated_qualification.rs` moved with `git mv` to `cad/monstertruck/src/qualification/`,
  private to `qualification.rs`. `prepare_qualified_emission` moved there too, as a private
  function. `profile.rs` no longer has any proof module and names no qualification type.
- Observer: `profile::EmissionObserver` (`pub(crate)`) has `face(source, face, mesh_orientation,
  polygon)` and `triangle(occurrence_index, polygon, vertices, positions_mm_f64,
  rendered_positions_f32)`. `()` is the no-op; `AnalysisTrace` implements it. The meshing loop is
  `mesh_profile_observed(model, target, &mut impl EmissionObserver)`.
- Rename: `SourceBound` is now `LocalMeshBound`, private, beside `prepare_qualified_emission`.
- Counts: kernel lib 290 passed, 1 ignored (the criterion's 285 was stale); CAD worker 57. Before and
  after test names match once the module prefix is normalised. Logs: `.evidence/012-before.log`,
  `.evidence/012-after.log`.
- Left alone: `cad/wasm/tests/fixtures/generated-analysis-contract.json` names the old
  `profile/final_trace/cylinder_proof.rs` path, but as an immutable recipe pinned to a commit.

Review (2026-10-20, with 007–011): no documented-standard breaches; findings and fixes are in [006](006-review-follow-ups-spec.md).
