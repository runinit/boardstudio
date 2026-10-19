# 04: Extract mirrored-half reconcile tests and retire stale pre-activation suppressions

Status: claimed
Type: build
Blocked by: None (can start immediately)

**What to build:** Separate the inline mirrored-half reconciliation test suite into its own test module file—matching the sibling mirrored-half modules so the production reconciliation module holds only production code—and remove the obsolete pre-activation `#[allow(dead_code)]` / `#[allow(unused_imports)]` suppressions and *"deferred to ticket 106–116"* comments across mirrored-half reconciliation, lifecycle, matrix addition, and inverse translation. Any helper that is genuinely test-only is gated under `#[cfg(test)]`, and any unreferenced item is deleted.

## Acceptance criteria

- [ ] The mirrored-half reconciliation module declares `#[cfg(test)] mod tests;` with its test suite in a dedicated child test file, preserving all existing tests and behavior.
- [ ] Obsolete pre-activation `#[allow(dead_code)]` and `#[allow(unused_imports)]` attributes and *"deferred to ticket 106–116"* comments across the mirrored-half modules are removed; test-only helpers are gated with `#[cfg(test)]` and any item with zero callers is deleted.
- [ ] `python3 scripts/check.py quick` passes with no native test-count regressions.
