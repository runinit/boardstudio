# 010: One polygon area function

Status: resolved
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)
Related: [002: One polygon containment and area module](002-polygon-predicates.md)

## What to build

See the area decision in [006](006-review-follow-ups-spec.md). Known copies: `core/src/case.rs`,
`core/src/modules/occupancy.rs`, `core/src/outline.rs`, `core/src/outline_cleanup.rs`,
`web/crates/layout/src/outline/overlays.rs` and the CAD worker's lettering.

## Acceptance criteria

- [x] Contracts' polygon module has a signed area with tests for winding sign, degenerate rings and agreement between point forms.
- [x] f64 copies without a deliberate difference call it; the rest are commented.
- [x] The audit (copy, sign convention, outcome) is appended under `## Comments`.

## Comments

Shared functions: `path_signed_area(&[[f64; 2]])` and `polygon_signed_area(&[Vec2])` in contracts' polygon module (counter-clockwise positive, true area, so half the shoelace sum; fewer than three points give 0). Core re-exports `polygon_signed_area` through `core::model`, which the web layout crate already imports.

| Copy | Convention | Outcome |
| --- | --- | --- |
| `core/src/outline.rs` `area` | signed, half | Migrated (`path_signed_area`). |
| `core/src/outline_cleanup.rs` `area` | signed, half | Migrated. |
| `core/src/outline_validation.rs` `area` | abs, half | Migrated; callers take `.abs()`. |
| `core/src/modules/occupancy.rs` `polygon_area` | abs, half, `< 3` guard | Migrated; the call site takes `.abs()`. |
| `web/crates/layout/src/outline/overlays.rs` `polygon_area` | signed, half, no guard | Migrated (`polygon_signed_area`); `inspector.rs` follows. |
| `cad/wasm/.../lettering.rs` `signed_area_f64` | signed, half | Migrated (`path_signed_area`). |
| `cad/wasm/.../lettering.rs` `signed_area_f32` | signed, half, f32 products | Kept, commented: f32 rounding is part of the glyph orientation test. |
| `core/src/case.rs` `signed_area_i128` | signed, doubled, exact | Kept, commented: the winding decision must not round. |
| `core/src/case.rs` `signed_area_float` / `area_float` | signed, doubled, origin-translated | Kept, commented: translating by the first point limits cancellation at large offsets. |
| `core/src/case.rs` test-local `area` | abs, half | Now wraps `polygon_signed_area(..).abs()`. |
| `contracts/.../support_owner.rs` `area2` | signed, doubled, exact rational | Kept, commented: orientation must be exact. |

Summation order is unchanged for every migrated copy (same left-to-right sum of `ax*by - bx*ay`, then `* 0.5` in place of `/ 2.0`, both exact), so results are bit-identical. The only behavioural difference is a ring of fewer than three points, which was already 0 up to sign (and the web draft copy would have divided by zero for an empty ring). Qualification binaries and vendored crates have their own area helpers on other types and were not in scope.
