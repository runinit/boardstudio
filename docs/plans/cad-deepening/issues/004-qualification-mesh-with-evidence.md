# 004: Qualification owns mesh with evidence

Status: resolved
Type: build
Blocked by: [003: Kernel tests build through the production case builder](003-tests-through-build-case.md)
Parent: [CAD deepening](../map.md)

## What to build

A **Collision check** needs a mesh with a proven error bound. Today profile meshing carries
an optional proof trace and STEP meshing builds qualification's mesh settings, so meshing
knows about the proof machinery. Give qualification the one entry point that meshes and
returns the bound, so profile and STEP meshing no longer see qualification types.
Speculative: the coupling is already narrow; take this only if qualification keeps changing.

## Acceptance criteria

- [x] Profile and STEP meshing compile without `crate::qualification` types. (The proof hook inside profile meshing remains; see comments.)
- [x] Qualification exposes one mesh-with-evidence entry point used by the CAD worker. (`prepare_generated_analysis` and `prepare_vendor_analysis` already were; nothing else is called.)
- [x] Collision qualification results are identical on the existing fixtures (kernel 285 passed, 1 ignored; CAD worker 57).
- [x] Native kernel and qualification tests pass.

## Comments

Done: the two places where meshing named qualification types. STEP no longer builds
`qualification::MeshSettings` (it exposes its import options and qualification composes the
settings), and profile meshing returns a plain `Result<[f64; 3], Vec<String>>` for the bound
instead of qualification's alias. Both had a single caller, in `qualification.rs`.

Not done, on purpose: the proof modules (`final_trace`, `generated_qualification`,
`cylinder_proof`, about 4,000 lines) still live under `profile/`, and profile meshing still
takes the optional `AnalysisTrace` hook. Moving them means rewriting their `use super::*`
paths through the crate's proof-critical code, and removing the hook means replacing it
with an observer trait in the meshing loop. That is the speculative part of this ticket;
the gain is small because the hook is already one narrow, optional parameter. Reopen it only
if qualification keeps changing.
