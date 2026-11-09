# 05: Fence vendored hardware assets and generated catalogue snapshots from search and diff

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Configure repository ignore and git-attribute rules so default code search, agent file traversal, git diffs, and language statistics skip the vendored 3D/KiCad hardware library and generated module/part catalogue snapshots while keeping those files tracked in-repo for zero-setup offline builds and native tests.

## Acceptance criteria

- [x] Root `.ignore` excludes the vendored Ergogen hardware library and generated module/part catalogue JSON snapshots from default `rg` and agent traversal without affecting `git` tracking or Cargo builds.
- [x] Root `.gitattributes` marks the vendored hardware library as `linguist-vendored` and marks vendored CAD/PCB model files and generated catalogue JSON snapshots as `linguist-generated` and `-diff`.
- [x] Default `rg` at the repository root skips those asset and generated-catalogue paths unless `--no-ignore` is passed, and `python3 scripts/check.py repo tooling` passes.

## Answer

- Root `.ignore` excludes `ergogen/library/vendor/`, `catalogue/modules/imported-modules.json` and `catalogue/parts/imported-parts.json`. Authored importer manifests, the asset ledger, library metadata and importer source remain searchable. Files stay tracked and available to offline builds.
- Root `.gitattributes` marks the vendor tree `linguist-vendored`, its STEP/STP/STL/WRL and KiCad models `linguist-generated -diff`, and the two generated JSON snapshots `linguist-generated -diff`.
- Before/after search observations confirm default `rg` excludes the targeted paths and `--no-ignore` restores them. Attributes were verified for all 107 tracked vendor model files and both snapshots; authored inputs remain visible. No asset or catalogue index entries changed.
- `python3 scripts/check.py repo tooling` passed all 27 commands; the broader `quick` run passed all 54 commands. Receipts: `.evidence/cleanup-04-05/repo-tooling.json` and `.evidence/cleanup-04-05/quick-acceptance.json`.
- Independent Haiku/Pi Standards and Spec reviews found no blocking issue. Removed the unused `.kicad_pcb` pattern flagged by Spec; retained `.kicad_mod` fencing because imported KiCad footprints are the requested PCB model assets. The authored manifests and importers remain visible. Reports and responses: `.evidence/cleanup-04-05/reviews.md`.
