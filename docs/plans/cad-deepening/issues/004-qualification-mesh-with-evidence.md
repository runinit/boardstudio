# 004: Qualification owns mesh with evidence

Status: needs-triage
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

- [ ] Profile and STEP meshing compile without qualification types.
- [ ] Qualification exposes one mesh-with-evidence entry point used by the CAD worker.
- [ ] Collision qualification results are identical on the existing fixtures.
- [ ] Native kernel and qualification tests pass.
