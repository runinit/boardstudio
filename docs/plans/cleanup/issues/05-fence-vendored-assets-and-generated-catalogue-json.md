# 05: Fence vendored hardware assets and generated catalogue snapshots from search and diff

Status: claimed
Type: build
Blocked by: None (can start immediately)

**What to build:** Configure repository ignore and git-attribute rules so default code search, agent file traversal, git diffs, and language statistics skip the vendored 3D/KiCad hardware library and generated module/part catalogue snapshots while keeping those files tracked in-repo for zero-setup offline builds and native tests.

## Acceptance criteria

- [ ] Root `.ignore` excludes the vendored Ergogen hardware library and generated module/part catalogue JSON snapshots from default `rg` and agent traversal without affecting `git` tracking or Cargo builds.
- [ ] Root `.gitattributes` marks the vendored hardware library as `linguist-vendored` and marks vendored CAD/PCB model files and generated catalogue JSON snapshots as `linguist-generated` and `-diff`.
- [ ] Default `rg` at the repository root skips those asset and generated-catalogue paths unless `--no-ignore` is passed, and `python3 scripts/check.py repo tooling` passes.
