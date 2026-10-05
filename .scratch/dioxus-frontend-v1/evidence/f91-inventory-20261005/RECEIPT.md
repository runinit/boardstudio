# F9.1-C01 inventory accounting receipt

**Purpose:** Reconcile the authoritative React/Dioxus source inventory to F9.1-C01 without changing tasks, claiming acceptance, auditing new product behavior, or running a test suite. Current worktree `HEAD=6bb3d52024e898f0f4111baac9b84e307224b9e7`; the source inventory remains pinned to React `dev` revision `5a472a9426e6e38993361da402cd4ec730feb369`.

## Count and source identity

- The inventory has **79/79 existing TSX paths**: 63 production, 15 test/fixture, and one benchmark. All 79 files exist. All 63 production TSX SHA-256 values match their pinned inventory values; no source identity drift was found.
- It lists **18/18** `app/src/**/*.css` files; the source tree has exactly those 18. It lists **10** production UI hooks; the `useWorkbenchNavigation.test.tsx` test is correctly kept in `verification_tsx`, not double-counted as a hook. It lists both source font files under `app/src/assets/fonts`.
- Test accounting is **15 React test rows plus one retained benchmark row**. All now have an explicit disposition. The existing five specific dispositions/evidence mappings remain intact; formerly unspecified React tests are explicitly retained as reference tests, without any Rust test-port claim. The benchmark is explicitly development-only and not a Dioxus performance result. Each of the 15 verification rows and the benchmark row now carries an explicit lead workflow, shared-consumer list, state, and test disposition. `Workbench.performance.test.tsx` is assigned to F4 Parts catalogue behavior (not F3 Layout) and its unrelated Layout evidence links were removed; no Rust cache-test claim is made.

## Reconciliation made

Updated `tsx-inventory.json` with normalized lead workflow, shared-consumer list, port/coverage disposition, and truthful milestone-test disposition on each production TSX row. It now explicitly distinguishes **34 partial-port rows** from **29 inventory-only/open rows**; no whole-file completion is inferred. Added per-row workflow/state/test dispositions for all 18 CSS files, 10 production hooks, two font assets, and five already-listed non-TSX orchestration entrypoints (`useCaseGeneration`, `useProjectSession`, `createProjectActions`, `createProjectExporter`, `useElectricalPlanning`). Test dispositions state that milestone tests are not automatically row-complete coverage.

Removed stale active axe/assistive-technology wording from crosscutting milestone acceptance; removed compact/mobile qualification wording from active milestone criteria. The mobile review image is retained separately as an excluded historical reference. The scope note records that axe/AT and mobile/compact qualification are excluded, ordinary keyboard/focus/Escape remains in scope, and F9.3 is not assigned as work. Historical artifacts are not credited as passing excluded criteria.

## Remaining truth and limits

The inventory is an accounting ledger, not a qualification result. Inventory-only production rows remain open. Partial rows retain their exact row-level acceptance/evidence bounds; milestone test lists are references and do not prove every component. CSS/hooks/font dispositions point to owning workflow evidence where applicable and make no independent stylesheet/font completion claim. Accessibility, axe/AT, mobile/compact, optional cleanup, and F9.3 work were not reopened. No source, task ledger, runner, or config was changed; no tests/browser/compiler commands were run.

## Changed files

- `.scratch/dioxus-frontend-v1/evidence/tsx-inventory.json` — SHA-256 `db5dad5cbcafb264f817311d517383f3251f688b3d7281c124864e61b37e5fb7`
- `.scratch/dioxus-frontend-v1/evidence/f91-inventory-20261005/RECEIPT.md` — scoped audit detail
