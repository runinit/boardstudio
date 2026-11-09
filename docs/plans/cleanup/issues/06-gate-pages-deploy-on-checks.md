# 06: Gate the GitHub Pages deploy on the validation checks

Status: claimed
Type: build
Blocked by: None (can start immediately)

**What to build:** A push to `main` publishes the site only after the native and web validation
jobs pass for that same commit. Today the Pages workflow runs on its own trigger, so a commit whose
native or browser tests fail still builds and deploys, and the service worker then keeps that
broken release on users' machines until the next one.

Move the release build and deploy into the validation workflow as jobs that depend on the native
and web jobs and run only for pushes to `main` (and manual dispatch), then delete the separate Pages
workflow. Keep everything the current deploy relies on: the optimized release build of the
`/boardstudio/` route, the read-only cache restore of the shared web cache, the `pages`
concurrency group without cancel-in-progress, and `pages: write` / `id-token: write` granted only
to the deploy job, with the workflow default staying `contents: read`.

Before starting, ask the maintainer whether `main` branch protection already requires these checks.
If it does, this ticket still closes the direct-push and manual-dispatch gap, but say so in the
answer.

## Acceptance criteria

- [x] A push to `main` runs native and web validation first; the release build and deploy start only when both succeed.
- [x] Pull requests and pushes to `dev` validate but never build the release or deploy.
- [x] The separate Pages workflow is deleted; no other workflow deploys Pages.
- [x] Deploy-only permissions, the `pages` concurrency group and the read-only web cache restore are preserved.
- [ ] The workflow parses (`actionlint` if available, otherwise a YAML load) and `python3 scripts/check.py repo` passes.
- [ ] The first `main` run after merge shows the deploy job waiting on both validation jobs (record the run link in the answer).

## Progress

Commits `bc594dc70` and `d368f8980` move release build/deploy into validation after both native and web jobs. Release runs share the workflow-level `pages` concurrency group without cancellation; PR and `dev` validation use unique groups. The existing release steps, read-only caches and deploy-only permissions are preserved. The isolated lane's actionlint and structural gate checks passed; the post-review concurrency expression passed a YAML load and an assert-based scenario check. Branch protection was checked through GitHub: `main` is not protected and no ruleset requires these checks. The isolated `repo` check could not pass because the current committed cleanup map points to untracked documentation; the dirty root's `repo` check separately fails on pre-existing `CONTEXT.md` links. No workflow change caused those failures. A first live `main` run and its link remain required before resolution. Manual dispatch from a non-`main` ref still deploys, as the former Pages workflow allowed.
