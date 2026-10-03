# Full package provenance review — b9e74e37

**Bounded package/source/served-byte provenance CLEAR** for full build `frontend-layout-case-transport-integrated-20261002`, source `b9e74e37fc34948be9fee97c918b644200778bca`, served at `http://127.0.0.1:34737/` and `/boardstudio/`. This authorizes using these exact packaged bytes for the pending browser acceptance journey; it does not claim browser workflow acceptance.

Build directory: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-layout-case-transport-integrated-20261002`.

Provenance SHA-256 independently verified: `4bc42957cd9528c7dc11cb7c93de7bd61e3b663984c3f91af6fc58377c295cdb`. Status is `complete` and source commit is exact. Every command record matches the frozen maintained builder's expected full-build argv, cwd, environment and log label; all 22 exit successfully, logs exist and are nonempty. Retained page logs explicitly report successful client builds for both prefixes. Command log hashes and tails are retained in the machine audit.

All **1,342 recorded inputs** equal the current bounded input inventory, with no extra, missing or changed input. **1,335 Git-owned inputs** independently equal the exact commit's immutable blobs, not merely current working files. The remaining seven are generated `core/pkg` outputs (`.gitignore`, README, declarations, JS, WASM and package metadata), whose local hashes match provenance and whose production command completed in this full build; they are not claimed to be Git source. Current root HEAD at audit is `2db850b205971a9ad66996933896a4abf46be4a3`; the bounded maintained source remains byte-identical to the frozen source. Unrelated existing workspace modifications were preserved.

Each route has **145 packaged files**. Independent recursive inventory/hash checks found zero extra/missing/mismatched files and no declared path escapes. Route offline manifests each enumerate exactly that inventory and have distinct versions. Independently verified manifest SHA-256: root `96171dc29983191a1f2ccd1b392605ed5ecd4cef9d54f9afd2114f4089b4c49b`; subpath `02e749a97a693b0e99f33d72a00acec600634cff6e929dbf215b43ed4ccff353`.

Actual HTTP GETs independently verified **21 critical assets per route**, including navigation/index, both CSS files, page JS/WASM, core/CAD worker entrypoints and JS/WASM, CAD and renderer JS/WASM, offline worker/service-worker, generators and the Sofle archive. Every response is HTTP 200 and byte-identical to its own route's package hash. All checked responses include `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Navigation to each prefix returns that route's exact index.

| Served artifact | Root SHA-256 | Subpath SHA-256 |
| --- | --- | --- |
| index.html | `aa95be5e3231fc0c104e08f6410d276f7fc74de5d6619365d5514c645d9b92d0` | `421a68c129e83b04d2e5fc7d886b4c834fa6fca7cbb40dafd0193066e47c101d` |
| assets/m1.css | `096f24e191223580878d00d9a34458b0a54f031b2f1ef4ca357413f8ef756977` | same |
| page WASM | `8550024a771a4e6ffb3af9856f5d28d071ffa98463f93a7f4b05a4e44e3f3f55` | same |
| service-worker.js | `af93a9f08f166b1d1a2c45e32d6f09c4b8a3a8c1a32a7cbf5dbe760f4437090b` | `c0be42ddd125b5493ca361255eec33dd43103154d33e132e2a4d7d3004b39080` |

Machine audit: `.scratch/dioxus-frontend-v1/evidence/sol-review-wave-20261002/full-package-provenance-audit-b9e74e37-20261002.json`, SHA-256 **`ac8cbb0fbbe3d6f4de7ae2fa7cd1f89d155cb80cf4173c8f3d74c33b1660dc08`**. It retains every local asset hash, source comparison counts, all command log hashes and all critical HTTP URLs/bytes/hashes/headers. No errors.

Independent checks used filesystem hashing, immutable Git blob reads and bounded actual HTTP retrieval. No source changes, rebuilds, heavy tests or browser automation were performed. Case11 repair `09cddc677f81eb8ca5a9923e61c792e6fb709b99` is not in this source ancestry; no Case11 inclusion/acceptance is claimed. Layout model-layer controls and Case13 transport behavior still need their paired workflow evidence. Offline installation/reopen, rendered UI, CAD results, accepted history and all parent/public joins remain separate existing gates. No new refactoring takeaway observed.
