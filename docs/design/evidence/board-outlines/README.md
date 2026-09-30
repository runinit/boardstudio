# Outline baseline, 2026-09-29

The project was saved through the workbench at `http://127.0.0.1:4328/`, with
REVIUNG41 active. The archive and outline exports are retained unchanged. The
project JSON was extracted from the archive into the core regression fixture;
it contains 85 parts, three matrices and the user's controller/reset placements.

- [Original project archive](reviung41-original.boardstudio)
- [Original SVG](reviung41-before.svg) and [DXF](reviung41-before.dxf)
- [Exact core input](../../../../core/tests/fixtures/reviung41-outline-original.json)
- [Regression assertions](../../../../core/tests/outline_repairs.rs)

Archive SHA-256: `672d5f581e65bf74b9a5df336f734167bba7642865fa7a0dfa12a96347d64a8f`.
Project JSON SHA-256: `b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae`.

Before changes, `cargo test --manifest-path core/Cargo.toml --locked --test
outline_repairs` failed the structural-recess assertion for both thumb notches,
the controller slot and the reset recess. The wide centre valley and required
material assertion passed. Existing outline/geometry tests cover separate-board,
mirrored/shared-board and authored-cutout behavior and remain required gates.

Build and browser checks performed before this baseline do not validate the new
outline features. [Stage 1 results and the testing walkthrough](stage1.md) record
later source/build, browser, artifact and latency evidence separately. Human
acceptance and physical fabrication remain unperformed.
