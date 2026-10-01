# M1 provider reconciliation (ticket 01)

## Inputs and provenance

- Integration source: `codex/m1-production-20261001` at `75e68feec8006e7bff2ded3434dfde7ad78c2eed` when this isolated worker was created.
- Pinned provider source: `reference/dev-20261001` at `5a472a9426e6e38993361da402cd4ec730feb369`; merge base: `96dd51d3e790c28f5554a8c9888a147c8814e2a7`.
- Worker branch: `codex/m1-providers-worker-20261001`, created from the integration source. The original `dev` checkout remains at `5a472a9426e6e38993361da402cd4ec730feb369`; its existing two stashes were inspected read-only and remain present. The pinned provider import and latest available integration tip `225871cb` were merged; candidate HEAD at that point is `a79fa492`. The coordinator has since integrated host/session/build work; that newer integration tip still needs to be merged before this candidate is ready.
- Authority read: `.scratch/m1-production/spec.md`, `PLAN.md`, `issues/01-providers.md`, `CONSTRAINTS.md`, and all six root module specs. The component/footprint source and integration requirements in `docs/hardware/component-onboarding.md` were read before adopting the provider library assets.

## Reconciliation decisions

- Adopt the pinned encoder and VIK provider work, including source-owned library assets and provenance, mounted-module/circuit resolution, circuit reuse, host connector placement, board supports, scoped rotary export, and output-specific qualification gates.
- Keep accepted wire/document behavior. Existing boxed fields for `ReplaceDocument`, `CreateMirroredPair`, `SetMechanical`, and `CoreReply::Scene.document` remain boxed. The encoder test helper unboxes the `CoreReply::Scene.document` value explicitly; the provider and serialized JSON are unchanged.
- Behavior-preserving Clippy corrections were applied in the incoming provider code. With the user's explicit approval for transparent boxing, the new provider payloads `SetMountedModule.instance`, `SetMountedModule.definition`, `SetMountedModule.host_connector_definition`, and `ArtifactReply::ImportModuleBoard.result` are boxed. `ArtifactReply::PreparePreview.result` is also boxed as `Box<ExportPlan>` to satisfy strict Clippy. Serde serialization and generated TypeScript output remain unchanged; these are the intentional Rust type/ABI changes approved for this reconciliation. The exact PreparePreview source change is retained in `prepare-preview-boxing.patch`.
- Git's broad whitespace check reports trailing spaces in imported upstream READMEs/licences. Those source assets are retained byte-for-byte; no blanket whitespace rewrite was made.

## Verification

See adjacent command logs for full output. Fresh post-approval core strict Clippy, core locked tests, formatting, contract freshness, and contract runtime checks passed. CAD strict Clippy passed after behavior-preserving fixes in geometry, cache, gasket, planar mesh, and keycap code; CAD formatting and native tests passed (23 passed, 4 ignored). Rust boundary, Ergogen catalog/library, KiCad, and CAD JavaScript checks passed. The app build passed, all 496 Vitest tests passed, and all 11 provider Playwright tests passed. The app build and provider tests also passed after merging integration tip `225871cb`; the Vitest run needed `--maxWorkers=2` because two default-worker attempts each timed out the same 5-second storage test, while the earlier isolated full run passed at default settings. `provider-e2e.log` captures an initial setup attempt before `app/dist` existed; `provider-e2e-after-build.log` and `provider-e2e-after-integration.log` are successful runs. A newer integration tip arrived after those checks; it contains host/session/build work and still needs merging and relevant verification.

The first app Vitest run was started before fresh core/renderer/CAD WASM packages existed and reported missing imports plus a storage timeout while builds competed. A concurrent duplicate CAD WASM build raced on generated package metadata and failed; the serial pinned build then completed. App reruns use the freshly built core, renderer, and CAD artifacts. No repository test baseline, constraint, or budget was changed.
