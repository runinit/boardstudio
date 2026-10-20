# 008: Kernel rejects coordinates Core would never send

Status: ready-for-agent
Type: build
Blocked by: none
Parent: [006: Review follow-ups](006-review-follow-ups-spec.md)

## What to build

See the coordinate-limit decision in [006](006-review-follow-ups-spec.md). Today a hole ring
near 1e13 mm panics inside `cavalier_contours` through `build_case`.

## Acceptance criteria

- [ ] Contracts owns the 1,000,000 mm limit; Core's literal copies use it.
- [ ] `build_case` returns `Err` naming the body for a non-finite or out-of-range point in any ring.
- [ ] A test shows the 1e13 mm near-collinear hole returns `Err`, and a ring at the limit still builds.

## Comments
