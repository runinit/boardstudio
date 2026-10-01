# Maintained release component QA (not final acceptance)

This run exercises the provenance-verified maintained release at source commit `050e9282e076628b411db99e9def21677616a0f4`, build `web/target/builds/m1-release-20261001-050e9282`. Root and subpath were served together on `http://127.0.0.1:44087/` and `http://127.0.0.1:44087/boardstudio/`. The root had 47 staged files (50,142,841 bytes); subpath had 48 (50,210,543 bytes). See `release-inventory-050e9282.json` for SHA-256 and byte counts.

This release predates the review fix for failed export registration cleanup. Its unchanged service-worker and renderer behavior is component evidence only; it is not final M1 acceptance.

## Browser observations

- Chromium agent-browser session `m1release050e-7dd31abc1fc4` independently activated worker scopes `/` and `/boardstudio/`, with cache names `boardstudio-m1-offline-path-2f-m1-release-20261001-050e9282-root` and `boardstudio-m1-offline-path-2f626f61726473747564696f2f-m1-release-20261001-050e9282-subpath`.
- The same origin held separate IndexedDB version-1 databases `boardstudio-m1-root` and `boardstudio-m1-boardstudio`. REVIUNG41 was saved in the root database and reopened after reload. Sofle v2 was saved in the subpath database (record id `m1-sofle-v2-copy`, revision 3, six asset references) and reopened after reload. Asset bytes are stored separately from the document object, consistent with the production database layout.
- Both installed pages reloaded and rendered from their scoped offline caches. A fresh session with no prior worker/cache (`m1cold-20261001-7dd31abc1fc4`) loaded the page while online, then failed on offline reload with `chrome-error://chromewebdata/`, confirming that initial startup requires the network before installation/cache population.
- A controlled root-worker update used a QA-only second worker (`qa-root-update-20261001`; service-worker initializer SHA-256 `1b6ab349cf1284e6d278b1532f0bbc2182199f68d7f7fd3139c8ee9b93e7859e`, module `1c02dac3443cbc9b8c312b6c0d78181f6e76c53077af235a3d92f67cc9842c61`). The new root cache activated and replaced the prior root cache; the subpath cache and an explicitly seeded unrelated cache with sentinel `preserve-me` remained. This tests scoped cache cleanup and preservation.
- A request for an unlisted lazy path returned 404 while the browser reported `navigator.onLine === false`; agent-browser's offline emulation does not block all fetches in the existing context, so this does not establish the offline lazy-asset failure behavior. The cold-context navigation is the direct offline failure evidence.
- On Sofle v2, “Generate case” mounted the CasePanel canvas (`1210×320`, renderer state `active`) but generation failed with `invalid CAD result: exact assembly mesh, bodies, or bounds are invalid`; UI reported “PCB reference is unpopulated; case bodies use exact CAD geometry.” REVIUNG41 correctly blocked generation as its selected board was not ready. Thus this artifact cannot establish successful generated-mesh lifecycle, resize/DPR, WebGL context loss/recovery, scope-change geometry clearing, or export URL/worker late-result cleanup. Screenshot: `web/target/builds/m1-release-20261001-050e9282/qa-casepanel-sofle-error.png` (SHA-256 `9e077f6a02f1ef3f1292424ee9c8e6a845538c9463f0c95304a8105dc6b03ead`).

## Screen-reader limit

`orca` was unavailable. `speech-dispatcher`, `spd-say`, and `espeak` exist, but they provide a speech service/synthesizer and are not an interactive screen reader. No spoken screen-reader pass is claimed. A DOM accessibility tree alone is not treated as a screen-reader pass.

Raw axe audits were subsequently run with agent-browser's embedded axe 4.12.1 and are recorded in [`accessibility/release-050e9282`](../accessibility/release-050e9282/README.md). All audited M1 library/editor/error states had zero violations; axe marked SVG label contrast incomplete and those labels received a manual color check. The real interactive screen-reader blocker remains.

## Required final-release refresh

Repeat only freshness-sensitive checks against the corrected, provenance-verified release: startup bytes and exact asset hashes for root/subpath; cache install/control and offline reopen on both scopes; missing lazy request behavior; cache update preserving unrelated entries; and Sofle case generation/render lifecycle once the accepted CAD fixture succeeds. Re-run export cleanup/URL lifetime checks on the corrected release. Do not reuse this build as final M1 evidence.
