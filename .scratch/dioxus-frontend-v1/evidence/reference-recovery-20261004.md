# Pinned TypeScript reference recovery — 2026-10-04

## Recovery result

The prior public receipts identify the React reference as `http://127.0.0.1:5175/` and source commit `5a472a9426e6e38993361da402cd4ec730feb369`, but no old launch command or served-output identity was retained. The prior reload's `net::ERR_CONNECTION_REFUSED` was operational, not evidence of a product regression. The exact pinned source has now been rebuilt in an isolated temporary tree and is serving again at the same address.

At investigation time, port 5175 had no listener. The primary checkout and continuation worktree `app/dist` directories differ (`index.html` hashes `4114223c42fd444840928c11472c7a9284db8629c787a83ba7642e31102273aa` and `1295e19edc6a2e6d5d6cdc257b923b5484398bc03e890cc62d27327b8ac3cf49`), so neither was served.

The pinned commit exists in the primary repository with tree `6ac218ea5b817aeccab7e2453ded405491cd0a7a`. The exact source was extracted with `git archive` to `/home/chris/.local/share/boardstudio/tmp/reference-5a472a9-20261004`; no branch was switched and no source was changed in either repository checkout. Pinned manifest hashes were verified: root `package.json` `f6863675224ba92c7a9c42b171241087c48c2289cedb31f8e66a63f4a287a870`, `pnpm-lock.yaml` `3d89c789caeba7f0cc32071cf28793506c2a15b921f69984a9bf655fd35e639d`, `pnpm-workspace.yaml` `41511d99dd8b1a4d7d1f392ab542f683aca7c69b4d1c126ded999591e2dd1ffa`. `pnpm install --offline --frozen-lockfile` completed from local pnpm store v11 (98 packages reused, zero downloads) with pnpm 12.6.0. Toolchain: Node v26.10.0, Rust 1.98.0.

## Build and server provenance

All build commands ran in the isolated source tree above, sequentially (one heavy provider job at a time):

- `pnpm install --offline --frozen-lockfile`
- `pnpm run build:core` — success; generated `core/pkg/boardstudio_core_bg.wasm`, SHA-256 `b5046804f866f5f65270e735e29e17c1849eb7ad3d6a8abdae6dd5b2847834f1`.
- `pnpm run build:cad` — success, using the pinned CAD WASI container/toolchain and pinned CAD source; generated `cad/wasm/pkg/boardstudio_cadrum_wasm_bg.wasm`, SHA-256 `e8f6a988fe143b99486f0e4f6f1a3d3eff1e2ee5976079b648e160dda603a40c`.
- `pnpm run build:renderer:dev` — success; generated `renderer/pkg/boardstudio_renderer_wasm_bg.wasm`, SHA-256 `59dc3f2ab8ee8c00c8dc549ecb5ff63a12061dd05a339650135f9ad5a5a40afc`.
- `pnpm --dir app exec vite build --outDir /home/chris/.local/share/boardstudio/tmp/reference-5a472a9-20261004/output --emptyOutDir` — success with Vite 8.3.1. The normal `app` build wrapper was not used because its service-worker writer targets `app/dist`. `sw.js` was generated from the exact pinned writer logic with its output redirected to the temporary output directory; no source file was edited.
- After confirming port 5175 was free, started detached with `setsid -f ... pnpm --dir app exec vite preview --host 127.0.0.1 --port 5175 --strictPort --outDir /home/chris/.local/share/boardstudio/tmp/reference-5a472a9-20261004/output`. Vite listener PID is **2306232**. It serves `http://127.0.0.1:5175/`.

Verified `GET /`, `GET /sw.js`, and `GET /assets/main-CYxchQWA.js` return HTTP 200. Served `/` exactly matches output `index.html` SHA-256 `10ebd0aae42700240237c25c23778d3788058e5640b0a09814e3be5ace79a114`. Other key output hashes: `sw.js` `9b2d83b463acf8044e5647e0864d65536c10a00ef962fafe141fba88206feb5c`; main chunk `main-CYxchQWA.js` `13f3a98e85a03cbacdb9d6990cd74ef9f4f2c5d12441e608ce4420837f76980c`; CAD JS `boardstudio_cadrum_wasm-C0w4x1Vj.js` `c24663e394a18929f6d1e5f6d51c6475b2c9c570a347e42df711296a08ef6146`; renderer JS `boardstudio_renderer_wasm-CeORum37.js` `df26b49022db3c957fcc94608012a8da9bca297b10a7d1e89056375b01cda972`.

The exact prior historical build is still not identifiable. The currently restored server is now reproducibly attributable to the pinned commit and generated provider/output hashes above. Root's follow-up browser observation confirmed that the loaded DOM references the expected `main-CYxchQWA.js`; the saved `PCB grouped qualification20261004` / `Archived STL WRL qualification20261004` fixture reopens both model rows with persistent UUIDs `f01fbbf9-052a-46fc-af6c-7e650ae4f25c` (WRL) and `44497083-8d0e-45f2-aaad-6f24209d0376` (STL). The recovered reference renderer then showed 0/2 models and the real alert `WebGL2 could not start. The 2D editor remains available.` This is a visible WebGL/GPU limitation in the recovered environment, not a source/build attribution failure; do not claim reference 3D decode/render success from this run.

## Fresh-tab diagnostic continuation

A fresh browser tab reopened the saved two-model recipe on the same recovered bundle `assets/main-CYxchQWA.js`. It reported `2 / 2 models · 1.6 mm PCB` with no alert. A full reload followed by Parts → `Archived STL WRL qualification20261004` again reached 2/2 with no WebGL2 startup alert. No source, build, server configuration, or browser flags changed between the failing observation and these successful checks. The earlier failure is preserved; its cause is not established. The current reference route is usable for subsequent paired qualification. This is observed recovery, not a diagnosed or repaired product defect.
