# Guarded page-only provider reuse

**Status:** independently reviewed bounded implementation contract, 2026-10-02; not an activated cache or acceptance result. Root serializes packaging. RF-001/RF-009 supply the integration/evidence rationale; all62 canonical parents and their original acceptance joins remain unchanged.

## Problem and evidence

`scripts/build-m1.py` executes22 commands: six tool-version receipts, ten provider/fixture/generator/staging commands, then page/offline/embed for each route. It hashes source before/after and rejects drift. Historical `.scratch/dioxus-frontend-v1/evidence/build-frontend.py` executes8 commands (layout generator, preview generator, then page/offline/embed for each route) while reusing a fixed full baseline. That historical script is local evidence, not the maintained cache API: it hard-codes a baseline, permits broader changed paths, uses removable Python assertions, and starts work before every required final identity check. Repeated context/manual verification should become an explicit guarded option.

The current Case/Firmware wave is **ineligible**. The author's comparison to full build `frontend-inspector-idb-integrated-20261002`, source `71a144ec…`, found drift in Ergogen source/generator, web dependencies/lock/build script, archive/bundled model/main/shared presentation/runtime, Keymap and model-delivery inputs plus CSS. These exceed the bounded allowlist. A changed build helper itself also requires a fresh full baseline. Reuse must reject this wave and any pre-helper baseline; a future independently verified full22 build establishes a usable helper-matched baseline.

## Public tooling contract

- Preserve `python scripts/build-m1.py <unique-build-id>` as the full22 build. Add `python scripts/build-m1.py <unique-build-id> --reuse-providers-from <full-build-id>`. Resolve the baseline beneath the established builds directory, validating its identifier without path traversal. Reuse is explicit; no automatic cached fallback or silent full build follows a rejection.
- `--help` exits0 before output creation, source enumeration or any build command. Unknown flags, missing values, invalid/option-looking IDs and path traversal exit nonzero before side effects. Use ordinary explicit validation/errors so `python -O` retains every guard.
- The baseline is a verified **complete full** build:22 successful recorded commands with expected labels/argv/features, root/subpath prefixes, retained tool-version outputs, provider/staged assets and complete source/asset manifests. A reuse-derived, failed, incomplete or malformed baseline is rejected. Retain its source commit identity; current HEAD may differ through committed allowlisted page changes or docs-only work. Compare exact consumed source identities rather than requiring equal HEAD.
- Capture complete current source path sets/hashes using the existing source guard coverage plus relevant untracked inputs. Preserve recursive model discovery and all dependency/config/generator/tool inputs. Additions, deletions, missing sources, changed providers, lock/config/tool drift and nonallowlisted changes reject reuse before output/build side effects. Generated output exclusions must be explicit; absence from Git alone is not proof that a file is unconsumed.

## Initial audited allowlist

Only these existing changed paths may differ from a complete baseline:

| Path | Why page-only |
|---|---|
| `web/src/presentation/panels.rs` | `main.rs` imports presentation only under WASM `page`; presentation privately imports panels. No library/worker registration imports this leaf. |
| `web/src/presentation/panels_scroll_tests.rs` | Imported by panels only for WASM tests; not a reused production worker input. |
| `web/assets/m1.css` | Page stylesheet linked by presentation and staged into each fresh site; no provider compilation consumes CSS. |

Record and validate this dependency proof against exact `web/src/main.rs`, `web/src/lib.rs`, `web/src/presentation.rs`, `web/Cargo.toml`, `web/build.rs` and the packaging sources. Those proof files must match the full baseline and may not change in reuse mode. A blanket presentation wildcard is unsafe: `lib.rs` directly imports a presentation objects geometry file for core-worker tests. Shared Runtime/main/worker/CAD/archive/model/generator changes are outside this initial contract. Further paths need separate source graph proof and independent contract/source review.

## Candidate construction and retained provenance

1. Before creating output, validate baseline provenance SHA/path sets, every stored root/subpath asset and every copied provider/fixture byte; capture the actual current tool versions and compare to baseline. Recheck the baseline manifest itself and immutable reused trees through completion. Missing/tampered/additional baseline artifacts invalidate identity. Compare expected site manifests rather than only iterating stored file keys.
2. Reserve a unique output directory; preserve baseline artifacts and any prior Dioxus public output. Copy only validated provider/staged assets into fresh route sites. Do not carry stale page/offline hashes or overwrite baseline files. Restage allowed CSS from current source.
3. Execute the eight actual commands: layout-generators, preview-generator, page-root, offline-worker-root, embed-offline-root, page-subpath, offline-worker-subpath, embed-offline-subpath. Use the same locked page/service-worker feature commands as full mode. Both fresh offline manifests include the exact new page/assets under `/` and `/boardstudio/` with distinct current build versions.
4. Compare complete current source path sets/hashes, baseline provenance hash and reused artifact path sets/hashes after work. Any pre/post drift fails; retain failed logs/result and never mark it complete. End-of-build invalidity is distinct from guard rejection before side effects.
5. Record mode, full baseline/source/provenance identity, changed allowlisted inputs, validated dependency proof, actual current source hashes, tool observations, every reused provider asset, both fresh manifest/site identities, commands actually run and pre/post comparisons. The reuse build has8 build commands plus separately recorded tool validation receipts, if executed; it does not claim22 rebuilt commands. Inherited full22 lineage is a separate field. Full mode retains its22 command lineage and existing artifacts.

Unique output and immutable provider reuse reduce repeated packaging work; they do not prove frontend parity. Root/subpath public/offline journeys and affected source/Standards/Spec/browser gates remain required. No invented timing saving or parent acceptance follows from this option.

## Meaningful fail-closed verification plan

Use disposable fixture repositories and stub build executors to validate guards without running real heavy builds. Retain the expected failing tests before implementation for the CLI defect and each guard; a mock asserting implementation details alone is insufficient.

- Run the real CLI `--help`, malformed options and invalid IDs from a fixture directory while a build sentinel is installed. Assert exit/output, zero executor calls, unchanged files and no output tree; repeat guard coverage with `python -O`.
- A complete full22 fixture plus only panel/CSS edits permits reuse, leaves baseline bytes unchanged, emits both correct routes and fresh offline versions, and records the exact8 executed commands separately from inherited22. The full invocation still executes its original22 and stages all providers.
- Change one real provider, dependency/lock, feature-proof file, generator source, build script or shared Runtime/main file; add/delete a consumed/untracked source or vendor model. Assert nonzero before any executor or output directory. Include representative current Case/Firmware drift as a negative fixture.
- Reject incomplete/failed/malformed or reuse-derived baselines, wrong route prefixes, missing source/tool receipts, tool-version differences, tampered/missing provider or root/subpath assets, extra stale baseline files and altered provenance. Exercise both source and artifact path-set coverage, not only hash mismatch.
- Introduce source, baseline manifest or provider asset drift during a stubbed build. Assert retained failure/no complete status. Verify fresh manifest contents exclude stale page/offline artifacts and include the current page and staged assets for each prefix.
- Retain and run existing build-source/catalogue/model staging tests. After exact-source review, establish a fresh real full22 baseline and run one eligible page-only reuse candidate with provider hashes equal and fresh root/subpath/offline browser evidence. Current Case/Firmware stays full-build-only.

Contract reviewed by `/root/sol_shared_integration_review`, requested Sol6.1 High, before implementation; author `/root/packaging_fast_path_author` independently implements in an isolated worktree. This clearance covers the described private tooling contract. Independent final source review, required checks and public artifact verification remain open until executed.
