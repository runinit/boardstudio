# F9.6 adoption proposal (unapplied)

**Status:** proposal only, unapplied. F9.5 is accepted on immutable candidate frontend-layout-pointer-cache-20261005, source f956c0dfe9ec942cb79b9905565e9c404fb96732. F9.6-C01 inventory and C03 register are independently reviewed and verified. The repaired release candidate is frontend-worker-activation-20261005, source 82207f752e93d889877bab72c6e86b2d248620ce, at http://127.0.0.1:34834/. C02 copied-data rehearsal is complete and independently CLEAR; the default-entrypoint patch remains unapplied and no cutover or deployment is approved.

## Proposed production and development entry points

The Pages workflow is the hosted production entrypoint. Today it runs the React `pnpm run build` and publishes `app/dist`. Root `pnpm build` also produces the React distribution, while `pnpm start` aliases `pnpm dev` and runs the dirty-tree React development server.

The proposed patch switches Pages and root `build` to the existing complete Dioxus builder, `scripts/build-m1.py`. Root `start` independently builds a uniquely named immutable Dioxus package and serves it with `scripts/serve-candidate.py` on port 4173; it no longer aliases `dev`. The existing dirty-tree React workflow stays at `pnpm dev`, with a documented `pnpm dev:react` alias. `pnpm start:react` and `pnpm build:react` preserve explicit React fallback entrypoints. React verification commands remain available. No React source or test files are deleted.

The low-level `dx serve` command is not used as the production/default package path: `web/assets` alone omits the staged Core/CAD/renderer workers and offline files required by the complete app. The existing full builder packages those resources. It requires clean maintained inputs, so `start` and `build` are production-style build commands rather than dirty-tree hot reload. The patch pins Dioxus CLI to `0.7.10`, matching accepted package provenance (`dioxus 0.7.10 (57d6794)`). It checks Python 3.11+ for `tomllib`, keeps the existing Node 24/pnpm, wasm-pack 0.15, and Rust target setup, then publishes `site-subpath/boardstudio` with `/boardstudio/` base path.

The exact unapplied patch is [default-entrypoint.patch](default-entrypoint.patch); `git apply --check` passed against the current worktree. It was not applied.

## Production inventory and retirement limits

[inventory-reconciliation.md](inventory-reconciliation.md) maps all 63 production rows to current Rust owner modules/shared responsibilities, task criterion states, evidence/review records, and bounded remaining tails. It distinguishes true gaps from responsibilities split across owner modules and does not infer whole-file completion from an accepted parent. The 15 verification TSX rows and one benchmark remain retained. No source/test deletion is proposed.

## Rehearsal and rollback

Use the accepted immutable candidate and its exact committed provenance; all F9.6 prerequisite parents are accepted. In isolated staging, use a copied browser profile and copied `.boardstudio` archives, never a live user profile. Exercise the accepted save/edit workflow, reload/read back, and export a portable copy. Roll back to the React artifact and import/read back the Dioxus-produced copy, then check the reverse direction using existing F9.4 round-trip conventions. Leave original copies untouched and record both source/package identities and results.

Rollback restores the prior production entrypoints and serves an explicitly staged copy of the pinned React artifact with the compatible `service-worker.js` handoff. Serving the original artifact alone is insufficient once the Dioxus worker controls the origin. Run `node scripts/web/stage-rollback.mjs <pinned-react-site> <new-output-site>`; an existing output is rejected, every original asset stays byte-identical, and `rollback-provenance.json` records the overlay. Retain React sources/tests and the original Pages artifact. Transfer projects using saved `.boardstudio` copies; both browser stores remain intact. The same-origin rehearsal completed against the pinned reference and immutable candidate, with public archive import, save, export and reload in both directions, followed by Dioxus re-adoption. Their IndexedDB stores are separate: portable archive import is the compatibility boundary; automatic legacy browser-store migration is not supplied. Both stores and service-worker registrations/caches must be retained during rehearsal. Final C02 independent acceptance review is CLEAR. F9.7 requires explicit approval of the concrete patch and verification of the adopted local launch.

## Current package and required approval

The handoff repair passed independent source review, 3 packaging/lifecycle/rollback tests and all 41 provider-reuse tests. Package proof for source `82207f75` verifies 1,412 committed source inputs and 191 served assets at each root/subpath route with no mismatches or release warnings. The build performed 9 fresh commands, reused 23 verified commands, and took 119.36 seconds. The Rust page/application/provider source remains unchanged from the accepted performance and desktop workflow candidate; no new performance claim is inferred from packaging.

- Candidate proof: `../frontend-worker-activation-20261005/package-proof.json`.
- Reviewed repair and exact source hashes: `handoff-repair.md`, `handoff-source-manifest.json`, and the corrected `handoff-activation-diagnosis.md` / `handoff-activation-source-manifest.json`.
- Rollback artifact: `rollback-activation-publication.json`; http://127.0.0.1:34835/ serves all 143 pinned reference files plus the declared handoff worker.
- Actual cutover RED and subsequent continuation: `rehearsal/RECEIPT.md` and its linked raw records. Completed joined public evidence: `rehearsal/joined-cutover-receipt.md`, with exact output hashes and final worker/store observations in `rehearsal/run-worker-handoff-finish-20261005/completion-observations.json`.

The bridge intentionally reloads controlled pages once to load the deployed shell. Rehearsal and deployment use saved/exported projects; unsaved transient UI drafts are not a preservation claim.

After C02 is reviewed, the concrete approval is to apply `default-entrypoint.patch` to `package.json` and `.github/workflows/pages.yaml`, commit the approved default build/start/Pages configuration, and validate the local adopted launch. No push, hosted deployment, React source deletion, or removal of fallback/reference artifacts is included. The operating contract retains a separate approval for production cutover.

The first handoff package stalled in Chromium because activation waited for its own navigation. Source 82207f75 corrects that dependency; the revised regression keeps navigation pending until activation settles and passes 3/3. The earlier failed profile/evidence remains preserved. The completed journey retained one isolated profile at stable origin 34836, starting from a verified React saved-copy baseline. Forward and final Dioxus archive hashes are identical; the rollback React archive matches the original reference archive. Full field comparisons use the already accepted bounded numeric serialization tolerance; all six asset paths and hashes match exactly. Both IndexedDB and cache families remain intact. The temporary staging proxy has been released; its browser profile is preserved.

The local static rollback server returned stale 304 responses when switching artifacts with older modification times. The QA proxy therefore served fresh full bytes for `/`, `/index.html`, `/sw.js`, and `/service-worker.js`, stripping conditional validators upstream and returning `Cache-Control: no-store`. Hashed assets were unchanged. This is an explicit rehearsal serving condition, not proof of production Pages cache behavior. Original validator and menu automation failures remain separately recorded; the joined receipt does not claim an uninterrupted passing run.
