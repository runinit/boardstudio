# 003: Kernel tests build through the production case builder

Status: resolved
Type: build
Blocked by: None
Parent: [CAD deepening](../map.md)

## What to build

Production builds every case through the kernel's case builder over a prepared case.
The kernel also exports a second profile builder that only tests use, so export, profile
validation and cache-complexity tests verify an interface production never calls. Move
those tests onto the case builder with a prepared plate, and make the profile builder
internal so the kernel has one public solid builder.

## Acceptance criteria

- [x] The profile builder and its input type are no longer public kernel exports.
- [x] Export, profile-validation and cache-complexity tests build through the case builder.
- [x] Native test count is unchanged apart from deliberate renames recorded here.
- [x] Native kernel tests and `check.py test` pass.

## Comments

Deleted the production `build_profile`, `ProfileInput`, `ProfileRegion`, `ProfileKind` and
their private validation and wire helpers. A `#[cfg(test)]` fixture (`test_support.rs`) keeps
the same test inputs but builds a prepared case and calls `build_case`. Kernel test total is
unchanged (285 passed, 1 ignored); the near-collinear-hole test moved to
`contracts/rust/tests/case_profile_region.rs`, where `normalize_case_profile_region` owns the
exactness property. Through `build_case` that input (coordinates near 1e13 mm, far beyond
Core's 1e6 mm limit) passes the exact check and then panics inside `cavalier_contours`
instead of returning an error. Core's `prepare` rejects such input first, so this is not
reachable today, but the kernel does not defend against it.
