# Map: Codebase cleanup and deduplication

Label: wayfinder:map

## Destination

Remove retained dead vendored code and consolidate confirmed first-party code duplication identified in [docs/research/duplication-refactoring-opportunities.md](../../research/duplication-refactoring-opportunities.md), shrinking the workspace and maintenance surface without weakening sequence-checked renderer submission, owner-scoped edit settlement, or dependency provenance gates.

## Notes

- Domain terms: [CONTEXT.md](../../../CONTEXT.md) (accepted document, pending edit, field edit, scope).
- Architecture & ownership: [docs/architecture.md](../../architecture.md) and [docs/research/duplication-refactoring-opportunities.md](../../research/duplication-refactoring-opportunities.md).
- Tracker conventions: [docs/agents/issue-tracker.md](../../agents/issue-tracker.md).
- Skills: `tdd`, `codebase-design`, `rust-best-practices`, `code-review` before resolving.

## Order

```text
01 (retire cgmath-0.18.0 fallback)
02 (consolidate OwnedEdits<K> into ui-shared)
03 (deduplicate RendererHost page snapshot)
```

All three tickets touch disjoint crates and can execute independently in parallel.

## Tickets

- [01: Retire the unused vendored cgmath 0.18.0 fallback](issues/01-retire-unused-cgmath-0-18-0-fallback.md) — `resolved`.
- [02: Consolidate OwnedEdits<K> into ui-shared](issues/02-consolidate-owned-edits-into-ui-shared.md) — `resolved`.
- [03: Deduplicate the RendererHost page snapshot across host and runtime](issues/03-deduplicate-renderer-host-page-snapshot.md) — `resolved`.

## Decisions and verification

- [01](issues/01-retire-unused-cgmath-0-18-0-fallback.md): remove only the unused fallback; both locked workspaces continue to use the unchanged reviewed successor.
- [02](issues/02-consolidate-owned-edits-into-ui-shared.md): share the existing helper without changing controller policy; public-method tests exercise retirement, mounted binding replacement, exact drafts and request pruning.
- [03](issues/03-deduplicate-renderer-host-page-snapshot.md): place the checked page adapter beside the canonical renderer lifetime and re-export it from Runtime, avoiding additional forwarding primitives or public internal state. Behavioral lifecycle tests replace source-text equality.
- Combined review against starting commit `b3c27cdbca4e30025cf2e8cc1cba99acfff5c746`: Standards — no blocking violations, one non-blocking pre-existing duplicated JS-error formatter; Spec — no findings. The formatter is outside this cleanup's scope.
- Focused verification: 3 native helper tests and 9 browser renderer lifecycle tests passed; all 6 behavioral mutants were killed, with no compile failures. Full acceptance evidence is recorded in the tickets.
- Acceptance: repo/tooling/security/lint passed. The full run passed build and provider/drag smoke checks but timed out launching the isolated setup-guide browser test. A serialized `quick` retry passed all its checks: 2,024 native tests, every crate browser launch and all 77 page tests. Receipts: `.evidence/cleanup/full.json.md` and `.evidence/cleanup/quick-retry.json.md`.
