# 006: Review follow-ups for CAD deepening

Status: ready-for-agent
Type: spec
Blocked by: none
Parent: [CAD deepening](../map.md)
Related: [002](002-polygon-predicates.md), [003](003-tests-through-build-case.md), [004](004-qualification-mesh-with-evidence.md)

## Problem Statement

The CAD deepening tickets closed with gaps the code review found. Polygon area still has
several copies. The kernel can panic instead of returning an error on coordinates Core
would never send. The proof machinery for a **Collision check** still lives inside profile
meshing. Kernel test fixtures are duplicated. Placement-transform errors are worded in two
places. And 14 `kicad-cli` tests fail whenever the suite is launched from an AppImage-hosted
terminal, so a full run looks red when nothing is wrong.

## Solution

Close each gap at its owner: one area function beside the containment predicate, one
coordinate limit shared by Core and the kernel, proof modules under qualification behind an
observer, kernel tests on one fixture, one error wording in contracts, and `kicad-cli` tests
that ignore a leaked AppImage environment.

## User Stories

1. As a maintainer, I want one polygon area function, so that sign and orientation rules cannot drift between Core, the layout overlays and the CAD worker.
2. As a maintainer, I want deliberate area variants (exact integer, f32) kept and commented, so that I know they differ on purpose.
3. As a CAD worker caller, I want `build_case` to return an error for out-of-range or non-finite coordinates, so that a bad input never aborts the worker.
4. As a maintainer, I want Core and the kernel to share one coordinate limit, so that the kernel accepts exactly what Core can produce.
5. As a maintainer, I want profile meshing to know nothing about proofs, so that I can change meshing without reading 4,000 lines of proof code.
6. As a qualification author, I want the proof modules under qualification, so that the **Collision check** evidence has one home.
7. As a qualification author, I want an observer seam in the meshing loop, so that proof tracing does not need a qualification type in meshing's signature.
8. As a test author, I want one prepared-case fixture for kernel tests, so that a contract change updates one place.
9. As a user, I want the same placement-transform error wording from the host and the CAD worker, so that the same problem reads the same everywhere.
10. As a developer, I want the full test suite to pass from any terminal, including one started by an AppImage, so that red means broken.
11. As a reviewer, I want the qualification gate binaries' predicates marked as independent oracles, so that nobody "deduplicates" them into the code they check.

## Implementation Decisions

- **Area.** The contracts polygon module gains a signed area for the same point forms as
  containment. f64 copies without a deliberate difference call it; callers needing
  magnitude take `abs` themselves. Exact integer, exact rational and f32 copies stay with a
  one-line comment. Append the audit (copy, sign convention, outcome) to this ticket.
- **Coordinate limit.** Contracts owns the 1,000,000 mm coordinate limit. Core's literal
  copies of it use the constant. `build_case` rejects any non-finite point, or any point
  beyond the limit, in every region ring with an error naming the body, before any kernel
  call.
- **Proof modules.** `final_trace`, `generated_qualification` and `cylinder_proof` move
  under the qualification module. Profile meshing takes a small observer trait instead of
  the `AnalysisTrace` parameter; qualification implements it. The public entry points
  (`prepare_generated_analysis`, `prepare_vendor_analysis`) keep their signatures. The
  `SourceBound` alias gets a name that does not read as a sibling of `WorldSourceBound`.
- **Fixture.** The two kernel integration tests become unit tests using the test-only
  fixture module; the integration `common` module is deleted. The native-test baseline is
  updated for the renamed tests.
- **Error wording.** `AffineTransformError` implements `Display` in contracts. The host's
  mapping function is removed in favour of it; the worker prefixes the operand side. This
  changes user-visible text on at least one side, which is approved.
- **kicad-cli.** When the test process environment shows an AppImage launcher
  (`APPIMAGE` or `APPDIR` set), the tests' `kicad-cli` invocations drop `APPDIR`,
  `APPIMAGE`, `LD_LIBRARY_PATH` and `GSETTINGS_SCHEMA_DIR`. Without those markers, the
  environment passes through untouched. Production code does not spawn `kicad-cli`.
- **Gate oracles.** The float and exact-rational containment copies in the qualification
  gate binaries stay, each with a comment saying they are independent oracles.

## Testing Decisions

- Test external behaviour at the highest seam. For the kernel that seam is `build_case`; for
  polygons it is the contracts module's public functions; for errors it is the `Display`
  output; for qualification it is the two public entry points.
- Kernel: a near-collinear hole at about 1e13 mm returns `Err`, not a panic, and a ring at
  the limit still builds. Prior art: `boss_no_wider_than_its_hole_is_rejected` in the
  layered tests.
- Area: winding sign, degenerate rings and agreement between the two point forms. Prior
  art: the containment tests in the contracts polygon module.
- Qualification: existing kernel (285) and CAD worker (57) results stay identical.
- kicad-cli: the 14 tests pass with the AppImage variables set, and also without them.

## Out of Scope

- Moving the gate binaries onto contracts.
- Any change to how the f32 renderer or the core test oracles classify points.
- Pushing or opening a PR.

## Further Notes

Tickets [007](007-kicad-cli-under-appimage.md), [008](008-kernel-coordinate-limit.md),
[009](009-transform-error-display.md), [010](010-polygon-area.md),
[011](011-kernel-tests-one-fixture.md) and [012](012-proof-modules-under-qualification.md)
implement this spec. They run in that order, one at a time, because they share one
worktree and one Cargo `target`.

## Comments

Combined review of 007–012 (2026-10-20). No documented-standard breaches. Fixed after review: a
doc comment in the kernel had drifted onto the new coordinate check; the case settings panel
still spelled the coordinate limit as a literal; and the 1e13 mm kernel test was rejected by its
outer ring, so a separate test now shows an out-of-range hole alone is rejected. Kept, as
judgement calls: the two `kicad-cli` environment helpers (a unit test cannot reach the
integration-test support module), the paired `[f64; 2]`/`Vec2` area functions (matching the
containment pair), and the `EmissionObserver` method names. The worker's error message no longer
names the `rowTransformF64` field.
