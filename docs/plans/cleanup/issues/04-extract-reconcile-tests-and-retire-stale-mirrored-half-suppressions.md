# 04: Extract mirrored-half reconcile tests and retire stale pre-activation suppressions

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Separate the inline mirrored-half reconciliation test suite into its own test module file—matching the sibling mirrored-half modules so the production reconciliation module holds only production code—and remove the obsolete pre-activation `#[allow(dead_code)]` / `#[allow(unused_imports)]` suppressions and *"deferred to ticket 106–116"* comments across mirrored-half reconciliation, lifecycle, matrix addition, and inverse translation. Any helper that is genuinely test-only is gated under `#[cfg(test)]`, and any unreferenced item is deleted.

## Acceptance criteria

- [x] The mirrored-half reconciliation module declares `#[cfg(test)] mod tests;` with its test suite in a dedicated child test file, preserving all existing tests and behavior.
- [x] Obsolete pre-activation `#[allow(dead_code)]` and `#[allow(unused_imports)]` attributes and *"deferred to ticket 106–116"* comments across the mirrored-half modules are removed; test-only helpers are gated with `#[cfg(test)]` and any item with zero callers is deleted.
- [x] `python3 scripts/check.py quick` passes with no native test-count regressions.

## Answer

- Moved the reconciliation suite to `core/src/mirrored_half/reconcile/tests.rs`, retaining its existing `pub(in crate::mirrored_half)` visibility for sibling lifecycle fixtures and relocating the outline-test include. The moved body matches the original after dedenting, formatting and the include-path correction; all 801 library and 707 integration test names are unchanged.
- Removed obsolete activation suppressions and comments. Test-only transaction and exception helpers use `#[cfg(test)]`; unused snapshot/coverage helpers, an unused test constant and never-constructed inverse refusal variants were deleted. One documented suppression remains on a shared test-fixture module whose integration target uses additional helpers.
- Focused verification: 393 mirrored-half library tests and 123 mirrored integration tests passed; `cargo check -p boardstudio_core` passed. The parent removed the stale inverse-module suppression; Clippy then exposed three unused test helpers and their imports, which were deleted. Workspace Clippy now passes with warnings denied.
- `python3 scripts/check.py quick` passed all 54 commands: repo/tooling, native and WASM lint, page typechecking, all 2,074 baselined native tests, all 14 browser crate launches, all 77 page tests, provider/browser preflights and CAD tests. Receipt: `.evidence/cleanup-04-05/quick-acceptance.json`. The earlier Clippy failures remain recorded in `quick.log` and `quick-final.log`.
- Independent Haiku/Pi reviews completed against `0e64c21d38da236a7a7d5efbc7cd06f67f0a35f2`. Standards: no blocking violation; corrected the preparation module documentation to name its production entry points. Spec: no proven behavior error. Full reports and responses: `.evidence/cleanup-04-05/reviews.md`.
- Scope: this is `quick` acceptance, which omits the full packaged build and drag smoke. The final post-check Rust edit only clarified the preparation module documentation; rustfmt and documentation links were checked again.
