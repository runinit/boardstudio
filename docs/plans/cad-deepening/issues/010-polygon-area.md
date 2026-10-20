# 010: One polygon area function

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)
Related: [002: One polygon containment and area module](002-polygon-predicates.md)

## What to build

See the area decision in [006](006-review-follow-ups-spec.md). Known copies: `core/src/case.rs`,
`core/src/modules/occupancy.rs`, `core/src/outline.rs`, `core/src/outline_cleanup.rs`,
`web/crates/layout/src/outline/overlays.rs` and the CAD worker's lettering.

## Acceptance criteria

- [ ] Contracts' polygon module has a signed area with tests for winding sign, degenerate rings and agreement between point forms.
- [ ] f64 copies without a deliberate difference call it; the rest are commented.
- [ ] The audit (copy, sign convention, outcome) is appended under `## Comments`.

## Comments
