# Case pre-CAD packet — independent Standards review

Date: 2026-10-02. Worktree `/home/chris/.local/share/boardstudio/worktrees/f73-case-preview-capability-20261002`; branch `codex/f73-case-preview-capability-20261002`; source baseline `42b9fdefe1941e3441ed7d43d2757e93e3588bde`.

The originally dispatched hashes changed during review. The author confirmed the following replacement packet frozen; this verdict applies only to these exact current snapshots:

| Packet file under `.scratch/dioxus-shared-viewer/` | SHA-256 |
| --- | --- |
| `drafts/10-pre-cad-case-pcb-preview-owner.md` | `4aa4fdaa52c6cc60d846c5bbde8eb8b4790e33a544d67cab01aca73f51631b8c` |
| `issues/07-case-imported-board-model-delivery.md` | `216d3cc516801e80c7dffc7bc5e3b6f9d9644a427d31c5ea1ec56076138adb05` |
| `issues/08-case-generated-board-model-delivery.md` | `0527f627834d7373718bbf9b0cdf260054d04c725ac3f1785c261af0027196c1` |
| `evidence/pre-cad-baseline-20261002/source-reconciliation.md` | `2300c138dde7e98ffce049e7d08dcbc22bdcb24729fd4922e28c7ef81cdd1eed` |

## Verdict

**Standards clear for this planning packet; zero unresolved findings.** This does not declare Issue07/08 implemented, the Issue08 provider capability gate satisfied, or public acceptance complete.

The packet preserves the CONSTRAINTS/ADR0003 single-authority and private-boundary requirements. Existing `CoreWorker::artifact` (`web/src/host/core_client.rs:186`) already transports public ArtifactRequest values through the actual worker. Runtime owns the private Core handle and current accepted snapshot; new coordination belongs there without exporting private library members. `cad_jobs::captured_case_scene` and `captured_case_document` (`web/src/cad_jobs.rs:241,252`) are already public page-feature functions. They project immutable accepted document/scene inputs, including physical-instance transforms, without requiring mechanical CAD. The private underlying `captured_case_inputs` need not be widened.

Core already defines PreparePreview/FinishPreview/PreviewBoard and remains geometry/validation authority. The provider's current `Weak<CadScene>` guard cannot represent a pre-CAD owner; the packet explicitly replaces that identity with accepted scope/token/revision/projection/batch identity and requires asynchronous revalidation. Retained preview/model snapshots are consumer data, not a second writable document authority. Issue07 retains bytes/SHA/decoding/cache/retry ownership; Issue08 coordinates native production; Issue12 consumes. Imported reference transforms and flipped-reference suppression remain separate contracts.

The frozen amendment correctly distinguishes retained `PcbPreview.models` placement rows from decoded meshes, and preserves React's batch-success publication after `Promise.all` (`app/src/assemblyPreview.ts:224–275`). Thus board-first visibility does not erase source row IDs or silently introduce incremental successful-mesh publication.

The existing worker packages actual `kicad/src/ergogen.ts`, `ergogen/src/index.ts`, and the generated catalogue. It is not a mock conversion or a second Core runtime. Root/subpath bundling, offline precache, actual worker execution/settlement, private module registration and Runtime/provider integration remain explicitly required. No public visibility, schema, authority, or gate relaxation is proposed.

## Executed verification and limits

- `node --test scripts/web/preview-generator-worker.test.mjs`: **6 passed**. Tests build/import the packaged module graph, validate relative imports, invoke real catalogue conversion, and cover ordered results/net snapshots, repeated reserved names, unresolved paths, unsafe integers, and correlated failures.
- `git diff --check`: **passed**.
- Exact packet hashes verified, then author confirmed freeze.

These Node tests exercise the actual packaged conversion functions, not browser Worker transport or production service-worker installation. No browser/renderer/offline acceptance or Rust implementation check was performed for this document-only packet. Current model_delivery remains unregistered and tied to CadScene until implemented; that is accurately recorded as unfinished capability, not a reason to invent a new provider.

## RF handoff

Retain the packet's existing **RF-003** source-to-preview/scene connection evidence and **RF-006** canonical/physical identity evidence. **RF-002** remains applicable to page-binary/library reachability; this inspection found sufficient existing public contracts, so no visibility expansion is needed. No new refactoring takeaway or RF ID observed beyond those recorded observations. No source or shared ledger was edited.
