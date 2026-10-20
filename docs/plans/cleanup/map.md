# Map: Codebase cleanup and deduplication

Label: wayfinder:map

## Destination

Remove retained dead vendored code, consolidate confirmed first-party code duplication identified in [docs/research/duplication-refactoring-opportunities.md](../../research/duplication-refactoring-opportunities.md), separate the inline mirrored-half reconciliation test suite and retire obsolete pre-activation suppressions, and fence off vendored hardware models and generated catalogue snapshots with `.ignore` and `.gitattributes` so searches, diffs, and agents operate on authored source without breaking zero-setup offline builds and tests.

## Notes

- Domain terms: [GLOSSARY.md](../../../GLOSSARY.md) (accepted document, pending edit, field edit, scope).
- Architecture & ownership: [docs/architecture.md](../../architecture.md) and [docs/research/duplication-refactoring-opportunities.md](../../research/duplication-refactoring-opportunities.md).
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Skills: `tdd`, `codebase-design`, `rust-best-practices`, `code-review` before resolving.

## Order

```text
01 (retire cgmath-0.18.0 fallback)
02 (consolidate OwnedEdits<K> into ui-shared)
03 (deduplicate RendererHost page snapshot)
04 (extract reconcile tests & retire mirrored-half suppressions)
05 (fence vendored assets and generated catalogue snapshots)
```

All five tickets touch disjoint areas and can execute independently in parallel.

Follow-ups from the 2026-10-19 repository audit; all four are independent:

```text
06 (gate Pages deploy on checks)
07 (delete unused BrowserStore save paths)
08 (one SHA-256 hex helper)
09 (README Node requirement)
```

The audit's split of the page shell's `Editor` component has a separate [workspace ownership specification](../editor-workspace-extraction/issues/001-editor-workspace-ownership.md), not an implementation ticket in this map.

## Tickets

- [01: Retire the unused vendored cgmath 0.18.0 fallback](issues/01-retire-unused-cgmath-0-18-0-fallback.md) — `resolved`.
- [02: Consolidate OwnedEdits<K> into ui-shared](issues/02-consolidate-owned-edits-into-ui-shared.md) — `resolved`.
- [03: Deduplicate the RendererHost page snapshot across host and runtime](issues/03-deduplicate-renderer-host-page-snapshot.md) — `resolved`.
- [04: Extract mirrored-half reconcile tests and retire stale pre-activation suppressions](issues/04-extract-reconcile-tests-and-retire-stale-mirrored-half-suppressions.md) — `resolved`.
- [05: Fence vendored hardware assets and generated catalogue snapshots from search and diff](issues/05-fence-vendored-assets-and-generated-catalogue-json.md) — `resolved`.
- [06: Gate the GitHub Pages deploy on the validation checks](issues/06-gate-pages-deploy-on-checks.md) — `ready-for-agent`.
- [07: Delete the unused BrowserStore save paths](issues/07-delete-unused-browser-store-save-paths.md) — `resolved`.
- [08: Use one SHA-256 hex helper for asset identity](issues/08-one-sha256-hex-helper.md) — `resolved`.
- [09: State the Node requirement in the README](issues/09-readme-node-requirement.md) — `ready-for-agent`.

## Decisions and verification

- [07](issues/07-delete-unused-browser-store-save-paths.md#answer): only the expected-token project save path remains; storage-lifetime browser tests and `quick` passed.
- [08](issues/08-one-sha256-hex-helper.md#answer): Core owns lowercase SHA-256 asset text; model-byte identity is exact; the redundant import self-check is gone. Post-review `quick` passed; the Runtime build-script dependency may affect cold wasm builds.

- [01](issues/01-retire-unused-cgmath-0-18-0-fallback.md): remove only the unused fallback; both locked workspaces continue to use the unchanged reviewed successor.
- [02](issues/02-consolidate-owned-edits-into-ui-shared.md): share the existing helper without changing controller policy; public-method tests exercise retirement, mounted binding replacement, exact drafts and request pruning.
- [03](issues/03-deduplicate-renderer-host-page-snapshot.md): place the checked page adapter beside the canonical renderer lifetime and re-export it from Runtime, avoiding additional forwarding primitives or public internal state. Behavioral lifecycle tests replace source-text equality.
- Combined review against starting commit `b3c27cdbca4e30025cf2e8cc1cba99acfff5c746`: Standards — no blocking violations, one non-blocking pre-existing duplicated JS-error formatter; Spec — no findings. The formatter is outside this cleanup's scope.
- Focused verification: 3 native helper tests and 9 browser renderer lifecycle tests passed; all 6 behavioral mutants were killed, with no compile failures. Full acceptance evidence is recorded in the tickets.
- Acceptance: repo/tooling/security/lint passed. The full run passed build and provider/drag smoke checks but timed out launching the isolated setup-guide browser test. A serialized `quick` retry passed all its checks: 2,024 native tests, every crate browser launch and all 77 page tests. Receipts: `.evidence/cleanup/full.json.md` and `.evidence/cleanup/quick-retry.json.md`.
- [04](issues/04-extract-reconcile-tests-and-retire-stale-mirrored-half-suppressions.md): extracted the unchanged reconciliation suite while preserving shared fixture visibility; retired obsolete activation suppressions, gated test-only helpers and deleted unused items.
- [05](issues/05-fence-vendored-assets-and-generated-catalogue-json.md): fence the vendored Ergogen library and two generated catalogue snapshots from default search; mark imported models and snapshots for Linguist and diff handling while retaining offline inputs and authored manifests.
- 04/05 verification: `repo tooling` passed 27/27 commands; `quick` passed 54/54 commands, including all 2,074 baselined native tests, 14 browser crate launches, all 77 page tests, provider/browser preflights and CAD tests. Independent Haiku/Pi Standards and Spec reviews completed with no blocking issues; review corrections are recorded in the tickets. Receipts and reports: `.evidence/cleanup-04-05/`. The full packaged build and drag smoke were not run for this slice.
