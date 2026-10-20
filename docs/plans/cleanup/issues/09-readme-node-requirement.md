# 09: State the Node requirement in the README

Status: claimed
Type: build
Blocked by: None (can start immediately)

**What to build:** The README's development setup says no Node, npm or pnpm is needed. But the
default full check includes the `tooling` step, which first runs `node --version` and then runs the
`.cjs` tool tests. The browser drag smoke also needs `playwright-core` on `NODE_PATH`. A new
developer who follows the setup text gets a failing full check. Correct the setup text: Node is not
needed to build or run the app, but it is needed for the `tooling` check and the browser drag
smoke. Point to the existing browser-test paragraph for the `NODE_PATH` detail rather than
repeating it.

## Acceptance criteria

- [x] The development setup no longer claims Node is never needed. It names the `tooling` step and the drag smoke as the parts that need it.
- [x] Building and serving the app is still described as Node-free.
- [ ] `python3 scripts/check.py repo` passes.

## Progress

Commit `6627033` corrects the development prerequisite while pointing to the existing Browser tests paragraph for `NODE_PATH`. Standards and Spec review found no implementation issue. `python3 scripts/check.py repo` in the isolated worktree failed only on pre-existing cleanup-map links to untracked documents; in the dirty root it fails on unrelated references to the already-deleted `CONTEXT.md`. Leave the repository check unchecked until the documentation migration is integrated and the check passes; no Cargo tests are needed for this wording change.
