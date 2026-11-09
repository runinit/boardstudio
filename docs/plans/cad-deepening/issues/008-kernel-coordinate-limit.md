# 008: Kernel rejects coordinates Core would never send

Status: resolved
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the coordinate-limit decision in [006](006-review-follow-ups-spec.md). Today a hole ring
near 1e13 mm panics inside `cavalier_contours` through `build_case`.

## Acceptance criteria

- [x] Contracts owns the 1,000,000 mm limit; Core's literal copies use it.
- [x] `build_case` returns `Err` naming the body for a non-finite or out-of-range point in any ring.
- [x] A test shows the 1e13 mm near-collinear hole returns `Err`, and a ring at the limit still builds.

## Comments

Contracts exports `MAX_COORDINATE_MM`; Core's five limit copies (kicad, source, mechanical, mechanical_checks, edit.rs including the `-limit` lower bounds) use it with comparisons unchanged. `check_ring_coordinates` in `layered.rs` runs first in `build_layered_case` (before the PCB branch) over outer, holes, cavities and gasket outer/holes, naming the body. RED: the 1e13 hole panicked in `cavalier_contours` with the check off. GREEN: kernel 288 passed 1 ignored, contracts and core pass, `cad/wasm` 57 passed, clippy clean.

Surprises: two existing kernel tests used out-of-range input. `raw_accuracy_conditions_volume_on_world_placement` placed a box at 1,000,000.1 mm (moved to 999,990.1), and `non_finite_contours_are_rejected` now hits the earlier check, so it expects "not finite". Mount positions are not rings and are not checked. The kernel suite is not in the native-test baseline, so no baseline change.
