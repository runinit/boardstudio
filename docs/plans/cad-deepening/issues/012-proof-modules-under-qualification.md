# 012: Proof modules live under qualification

Status: ready-for-agent
Type: build
Blocked by: [011: Kernel tests share one prepared-case fixture](011-kernel-tests-one-fixture.md)
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)
Related: [004: Qualification owns mesh with evidence](004-qualification-mesh-with-evidence.md)

## What to build

See the proof-module decision in [006](006-review-follow-ups-spec.md). This is the part of 004
that was left undone; the user approved it.

## Acceptance criteria

- [ ] `final_trace`, `generated_qualification` and `cylinder_proof` live under the qualification module.
- [ ] Profile meshing takes an observer trait, not `AnalysisTrace`, and names no qualification type.
- [ ] `prepare_generated_analysis` and `prepare_vendor_analysis` keep their signatures.
- [ ] `SourceBound` is renamed so it does not read as a sibling of `WorldSourceBound`.
- [ ] Kernel (285 passed, 1 ignored) and CAD worker (57) results are unchanged.
- [ ] A combined spec and standards review is run, per the CAD migration review rule in `AGENTS.md`.

## Comments
