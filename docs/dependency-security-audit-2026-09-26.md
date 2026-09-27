# Dependency and security audit — 2026-09-26

> The findings below preserve the original audit. See the remediation record at the end for the implemented changes and current validation.

Audited branch `dev`, revision `bc406fe94329315b0e9caa3dd710a12f734ed7a7`. Registry/advisory results are a point-in-time snapshot. Existing and concurrently appearing workspace changes were preserved. This is an audit: no dependency or production-code upgrades were applied.

## Findings, in priority order

1. **Update vulnerable development tools.** `pnpm audit --json` returned 23 advisory entries: 1 critical, 8 high, 12 moderate, 2 low. These are advisory/package entries, not 23 independently exploitable application bugs. All were classified as development dependencies. Vitest 3.2.4 has a critical UI/API file-read/execution advisory ([GHSA-5xrq-8626-4rwp](https://github.com/advisories/GHSA-5xrq-8626-4rwp)); its documented exposure conditions are network-exposed UI/API or Windows UI/browser mode. Current `app/package.json` runs `vitest run`, and `app/vitest.config.ts` does not enable these features. Vite 6.3.5 has several file-disclosure advisories, including [WebSocket arbitrary file read](https://github.com/advisories/GHSA-p9ff-h696-f583). Development binds to 127.0.0.1, which reduces exposure but is not a reason to retain vulnerable versions. Production is a static Pages build, not a Vite server.
2. **Renderer transitively includes an unsound, unmaintained math library.** `cgmath 0.18.0` comes through both `three-d 0.19.0` and `three-d-asset 0.10.0`. [RUSTSEC-2026-0197](https://rustsec.org/advisories/RUSTSEC-2026-0197.html) describes undefined behavior when `Matrix2/3/4::swap_columns` receives identical indices. OSV lists no fixed version. Targeted searches found no `swap_columns` calls in project Rust code or the installed three-d/three-d-asset sources; exploitability in this application was not demonstrated. Both parent libraries are already latest stable, so upgrading direct dependencies alone will not clear this. Track an upstream fix, maintained replacement, or narrowly reviewed patch.
3. **File import limits are applied too late on two paths.** `app/src/createProjectActions.ts:66` reads the entire project file before Rust checks its 128 MiB compressed limit. Model import at line 192 similarly reads the whole file without checking `File.size`. Oversized user-selected files can exhaust browser memory before rejection. This is a local availability issue requiring file selection, not demonstrated remote code execution. Add pre-read limits and regression tests asserting oversized files are rejected without calling `arrayBuffer()`. `BoardReferencePanel.tsx:7` already demonstrates pre-read checking. Rust's archive limits should remain as defense in depth.
4. **Maintenance warnings remain in current Rust dependencies.** `rhai 1.26.1 → smartstring 1.0.1` ([RUSTSEC-2026-0249](https://rustsec.org/advisories/RUSTSEC-2026-0249.html)); `three-d → instant 0.1.13` ([RUSTSEC-2024-0384](https://rustsec.org/advisories/RUSTSEC-2024-0384.html)); and cgmath ([RUSTSEC-2026-0196](https://rustsec.org/advisories/RUSTSEC-2026-0196.html)). These three are maintenance notices, separate from the cgmath unsoundness advisory. `instant` appears with `cargo tree --target all`, not the native-only default tree.
5. **CI does not continuously enforce dependency security.** The checked workflows run application validation but no npm/Rust advisory gate; no tracked Dependabot/Renovate configuration was found. Actions use mutable major tags and several are behind current majors. Add scheduled/PR dependency checks, with separately reviewed handling for maintenance warnings and the unfixed cgmath issue. Pin third-party actions to reviewed commit SHAs. Rust has no tracked toolchain pin, while local Node 26 differs from CI Node 24. Both can be intentional, but reproducibility and test coverage should be explicit.

## JavaScript version inventory

Latest means the npm registry's `latest` tag at audit time, not a tested migration target. Major migrations need compatibility validation.

| Package | Current | Latest |
| --- | --- | --- |
| fake-indexeddb | 6.2.4 | 6.2.5 |
| @playwright/test | 1.55.1 | 1.63.0 |
| @types/react | 18.3.28 | 19.3.0 |
| @types/react-dom | 18.3.7 | 19.3.0 |
| @vitejs/plugin-react | 4.7.0 | 6.1.1 |
| react | 18.3.1 | 19.3.0 |
| react-dom | 18.3.1 | 19.3.0 |
| typescript | 5.9.3 | 7.0.2 |
| vite | 6.3.5 | 8.3.1 |
| vitest | 3.2.4 | 5.0.2 |
| fflate | 0.8.2 | 0.8.3 |
| libcascade | 3.0.2 | 3.0.2 |
| pnpm (root packageManager) | 11.26.0 | 12.6.0 |

React and React DOM, their type packages, Vite/plugin-react/Vitest, and both workspace TypeScript declarations should be upgraded in coordinated groups. The workspace manifests pin most versions, so reinstalling alone will not advance them.

## JavaScript advisories

`pnpm audit --prod --json` reported zero advisories across its five production dependency entries. This does **not** cover Rust/WASM or prove the shipped application is vulnerability-free. fflate is used in tests/e2e/boundary scripts; the app's project archive runtime uses Rust ZIP code.

| Package | Severity | Advisory | Fixed range reported by audit |
| --- | --- | --- | --- |
| vite | low | [GHSA-g4jq-h2w9-997c](https://github.com/advisories/GHSA-g4jq-h2w9-997c) | `>=6.3.6` |
| vite | low | [GHSA-jqfw-vq24-v9c3](https://github.com/advisories/GHSA-jqfw-vq24-v9c3) | `>=6.3.6` |
| vite | moderate | [GHSA-93m4-6634-74q7](https://github.com/advisories/GHSA-93m4-6634-74q7) | `>=6.4.1` |
| vite | moderate | [GHSA-4w7w-66w2-5vf9](https://github.com/advisories/GHSA-4w7w-66w2-5vf9) | `>=6.4.2` |
| vite | high | [GHSA-p9ff-h696-f583](https://github.com/advisories/GHSA-p9ff-h696-f583) | `>=6.4.2` |
| postcss | moderate | [GHSA-qx2v-qp2m-jg93](https://github.com/advisories/GHSA-qx2v-qp2m-jg93) | `>=8.5.10` |
| vite | moderate | [GHSA-v6wh-96g9-6wx3](https://github.com/advisories/GHSA-v6wh-96g9-6wx3) | `>=6.4.3` |
| vite | high | [GHSA-fx2h-pf6j-xcff](https://github.com/advisories/GHSA-fx2h-pf6j-xcff) | `>=6.4.3` |
| postcss | high | [GHSA-6g55-p6wh-862q](https://github.com/advisories/GHSA-6g55-p6wh-862q) | `>=8.5.12` |
| postcss | moderate | [GHSA-fxqj-rqcc-2cmp](https://github.com/advisories/GHSA-fxqj-rqcc-2cmp) | `>=8.5.23` |
| undici | moderate | [GHSA-8xcm-r25x-g524](https://github.com/advisories/GHSA-8xcm-r25x-g524) | `>=7.29.0` |
| undici | high | [GHSA-4cwx-7wf7-3272](https://github.com/advisories/GHSA-4cwx-7wf7-3272) | `>=7.29.0` |
| undici | moderate | [GHSA-m8rv-5g2x-5cg5](https://github.com/advisories/GHSA-m8rv-5g2x-5cg5) | `>=7.29.0` |
| undici | moderate | [GHSA-jr45-8vmc-qm54](https://github.com/advisories/GHSA-jr45-8vmc-qm54) | `>=7.29.0` |
| undici | moderate | [GHSA-v3r7-h72x-cjcm](https://github.com/advisories/GHSA-v3r7-h72x-cjcm) | `>=7.29.0` |
| nanoid | high | [GHSA-28wg-ghj8-5hjv](https://github.com/advisories/GHSA-28wg-ghj8-5hjv) | `>=3.3.16` |
| nanoid | high | [GHSA-2v37-7h3g-55p8](https://github.com/advisories/GHSA-2v37-7h3g-55p8) | `>=3.3.18` |
| postcss | high | [GHSA-r28c-9q8g-f849](https://github.com/advisories/GHSA-r28c-9q8g-f849) | `>=8.5.18` |
| vitest | critical | [GHSA-5xrq-8626-4rwp](https://github.com/advisories/GHSA-5xrq-8626-4rwp) | `>=3.2.6` |
| nanoid | high | [GHSA-xwg4-73v4-xw9w](https://github.com/advisories/GHSA-xwg4-73v4-xw9w) | `>=3.3.12` |
| fflate | moderate | [GHSA-px8p-9vwx-vf98](https://github.com/advisories/GHSA-px8p-9vwx-vf98) | `>=0.8.3` |
| vitest | moderate | [GHSA-82fw-gwwq-j7x9](https://github.com/advisories/GHSA-82fw-gwwq-j7x9) | `>=4.1.11` |
| @vitest/mocker | moderate | [GHSA-82fw-gwwq-j7x9](https://github.com/advisories/GHSA-82fw-gwwq-j7x9) | `>=4.1.11` |

For a security-focused upgrade, account for **all** advisories per package: Vitest 3.2.6 addresses the critical report but does not clear the later mocker traversal report, which requires at least 4.1.11. Vite's listed advisories require at least 6.4.3; transitive PostCSS, nanoid and undici must also advance. Re-run the audit on the resulting lockfile rather than assuming direct upgrades clear everything. fflate needs 0.8.3.

## Rust direct dependency inventory

All three Cargo.lock files were checked; 163 unique registry package/version pairs were queried against OSV. No advisory matches were returned for the CAD lockfile. `cargo audit` is unavailable; no scanner was installed. The OSV fallback checks published advisories but does not reproduce cargo-audit's yanked-crate or complete policy checks.

| Crate | Locked | Latest stable | Workspace |
| --- | --- | --- | --- |
| i_overlay | 9.0.0 | 9.0.0 | core, renderer |
| kiutils_sexpr | 0.1.1 | 0.1.1 | core |
| serde | 1.0.229 | 1.0.229 | core, renderer, cad/wasm |
| serde_json | 1.0.151 | 1.0.151 | core |
| sha2 | 0.10.9 | 0.11.0 | core |
| wasm-bindgen | 0.2.128 | 0.2.129 | core, renderer, cad/wasm |
| js-sys | 0.3.105 | 0.3.106 | core, renderer, cad/wasm |
| zip | 8.6.0 | 8.6.0 | core |
| ts-rs | 12.0.1 | 12.0.1 | core |
| rhai | 1.26.1 | 1.26.1 | core |
| lyon_path | 1.0.19 | 1.0.19 | renderer |
| lyon_tessellation | 1.0.19 | 1.0.22 | renderer |
| three-d | 0.19.0 | 0.19.0 | renderer |
| three-d-asset | 0.10.0 | 0.10.0 | renderer |
| serde-wasm-bindgen | 0.6.5 | 0.6.5 | renderer, cad/wasm |
| web-sys | 0.3.105 | 0.3.106 | renderer |
| cadrum | 0.8.20 | 0.8.20 | cad/wasm |

Update wasm-bindgen, js-sys and web-sys together across all three manifests/lockfiles, then regenerate WASM bindings and validate all consumers. Current direct crate versions come from lockfiles; latest stable versions were retrieved from the crates.io API. Latest transitive crate versions were not exhaustively inventoried; all locked registry crates were included in the advisory query.

## Build tools and bundled native code

| Tool | Current/configured | Latest checked |
| --- | --- | --- |
| Node local | 26.10.0 | 26.10.0 |
| Node CI | 24.x | 24.21.0 within 24.x |
| Cargo local | 1.98.0 | Not independently checked |
| wasm-pack CI | 0.15.0 | 0.15.0 |
| OpenCascade archive | 8_0_1_rev2 | Upstream OCCT V8.0.1 |
| actions/checkout | v4 | v7.0.1 |
| actions/setup-node | v4 | v7.0.0 |
| pnpm/action-setup | v4 | v6.1.0 |
| actions/upload-artifact | v4 | v7.0.1 |
| actions/upload-pages-artifact | v3 | v5.0.0 |
| actions/deploy-pages | v4 | v5.0.1 |

Tool versions were checked through npm, nodejs.org, crates.io and upstream GitHub release APIs. OCCT archive naming includes a cadrum packaging revision; it is not a distinct upstream OCCT version. `cad/scripts/prepare-cadrum-occt.mjs` verifies pinned SHA-256 checksums before extraction, which is a useful supply-chain control. CI's KiCad 10 package and generated firmware's ZMK v0.3.0 were identified but not assessed for latest release/security status. Native OCCT/C++ dependencies are outside the npm/OSV Cargo scan; no exhaustive native CVE assessment was performed.

## Code review and validation

- Archive implementation enforces compressed/uncompressed/entry-count limits, bounded decompression, strict paths, duplicate rejection, CRC checks and asset hashes.
- Rhai uses `Engine::new_raw()` with source, operation, string, array, map, call-depth and expression-depth limits.
- Targeted production-source searches found no `eval`, `new Function`, `innerHTML` or `dangerouslySetInnerHTML` sinks in the inspected app/core/CAD/ergogen source paths. Reviewed SVG label generation escapes user text. This is not a full taint analysis or penetration test.
- `cargo test --manifest-path core/Cargo.toml --locked archive::`: 15 passed, including six ZIP directory tests. Expected size-limit test emitted a ZipWriter finalization warning while passing.
- `cargo test --manifest-path core/Cargo.toml --locked zip_directory::`: six passed (subset of the preceding 15).
- Initial `script::` test filter matched no tests; `cargo test --manifest-path core/Cargo.toml --locked --test core script -- --nocapture` subsequently passed both script integration tests, including atomic failure and execution limits.
- Full builds, browser/e2e suite, fuzzing, secret-history scanning and upgrade compatibility tests were not run. No production fixes or upgrades were made.

## Recommended implementation order

1. Upgrade vulnerable Vite/Vitest and transitive tools plus fflate; use supported versions that clear every advisory, then run application tests/build/e2e.
2. Add regression tests for oversized file imports and enforce pre-read limits.
3. Resolve or explicitly track cgmath's unsoundness and Rust maintenance dependencies with upstream evidence.
4. Upgrade React, TypeScript, Playwright and remaining tools in reviewable groups; validate contracts, WASM, CAD, renderer and browser behavior using `pnpm run check`.
5. Add continuous advisory checks and automated update proposals; align/pin supported toolchains and review action upgrades.


## Remediation record — 2026-09-26

Implementation began from `07b4da91` on `dev`, after concurrent repository work
was committed. This record accompanies the remediation commit. The user's final renderer
decision was to retain three-d/three-d-asset and apply a minimal cgmath patch;
no renderer migration was performed.

### Implemented

- Upgraded React/React DOM/types to 19.3.0, TypeScript to 7.0.2 in both
  workspaces, Vite to 8.3.1, plugin-react to 6.1.1, Vitest to 5.0.2,
  Playwright to 1.63.0, fake-indexeddb to 6.2.5, fflate to 0.8.3, and pnpm to
  12.6.0. Added explicit jsdom 30.1.1 for the test environment and refreshed
  transitive dependencies. `pnpm outdated -r --format json` returned `{}`.
- Adapted React refs and test element types without weakening typechecking.
  Vite now uses `rolldownOptions` for its existing HTML entries. The repository
  checker now uses TypeScript 7's native syntax API; its existing five tests pass.
- Disabled Vite preview CORS middleware for same-origin assets: its new
  `Vary: Origin` response header prevented precached assets from matching offline
  module requests. Playwright now launches Vite directly with Node because
  pnpm 12's separate child process group prevented server teardown.
- Upgraded sha2 to 0.11.0 and lyon_tessellation to 1.0.22. The sha2 update
  required byte-wise hexadecimal formatting at two call sites; output remains
  lowercase, two digits per byte. Synchronized wasm-bindgen 0.2.129 and
  js-sys/web-sys 0.3.106 across all WASM packages and the CAD container CLI.
- Added pre-read file checks: nonempty project archives up to and including
  128 MiB, and model files up to and including 32 MiB. Ten additional import
  tests cover empty/oversized rejection, all supported model extensions,
  exact-boundary acceptance and absence of reads or persistence on rejection.
- Vendored the checksum-verified cgmath 0.18.0 release and replaced six unsafe
  matrix column/element swaps with safe value swaps. Both rendering libraries
  resolve to that copy. See [patch provenance and maintenance](../renderer/vendor/README.md).
- Added `pnpm run check:security`: npm audit plus cargo-audit 0.22.2 for all
  application lockfiles and the vendored test lockfile. The gate checks vendor
  file hashes and Cargo resolution, restores cgmath's registry identity in
  temporary scan input, and permits only the specifically fixed
  RUSTSEC-2026-0197 advisory. Future advisories and unresolved unsoundness fail.
  Three gate tests cover audit identity restoration, unknown versions, modified
  or added vendor files and incorrect dependency resolution.
- Added weekly/PR/push security CI, Miri validation, weekly Dependabot proposals,
  and updated Actions pinned to upstream release commit SHAs. Application CI
  now covers Node 24.21.0 and 26.10.0. Local default Node and Rust are pinned to
  24.21.0 and 1.98.0; deployment retains Node 24. No deployment was performed.

### Verification

- Baseline: 51 app files / 227 tests passed before changes.
- Import regressions: six cases failed before the fix; all 13 tests in the
  expanded action test file passed after it (including three preexisting cases).
- cgmath: Miri reproduced aliasing undefined behavior on the original source;
  all three matrix-size suites passed after patching, under both native tests
  and Miri nightly-2026-08-12. Tests include equal/distinct indices and invalid
  column/row indices. Ordinary native tests alone do not detect the original UB.
- Node 26: 51 app files / 237 tests passed after migration.
- App and CAD TypeScript checks passed; a production app build passed.
- The Node 24 repository check passed repository and contract checks, native
  core/renderer/CAD tests, ergogen and KiCad tests, all 237 app tests, production
  WASM/app builds, CAD checks, and native/WASM parity (13 core and nine archive
  requests). Its first browser run failed two offline cases; those existing
  regressions passed after the preview fix. The separate complete browser rerun
  passed all 156 tests in 4.5 minutes, with normal teardown. The unaffected
  build/native stages were not repeated.
- All nine development-server browser tests and the Pages subpath/offline test
  passed with normal server teardown after the launcher fix.
- Frozen install with pnpm 12.6.0 passed on Node 24.21.0; no peer issues remain.
- Updated npm audit reports zero advisories. Rust checks found no unresolved
  vulnerabilities/unsoundness with the verified patch applied. The smartstring,
  cgmath and instant maintenance notices remain visible.
- YAML syntax checks passed for the three workflows and Dependabot configuration.
- Performance passed using the existing AMD Ryzen 9 8945HS / Chromium
  153.0.8010.47 baseline, five sessions and unchanged thresholds. Median worker
  p95 was 3.3 ms for both 100-part scenarios and 5.2 ms for both 200-part
  scenarios; painted p95 was 33.5–34.2 ms. All three matrix, outline and pointer
  interaction checks also passed. The performance baseline was not modified.
- Final `pnpm run check:security` (including its three regression tests) and
  whitespace checks for authored changes passed. The vendored archive retains
  upstream whitespace (five findings in four files) to minimize source changes
  and preserve the reviewed provenance; these are not security findings.

The early upgrade run exposed sha2's removed `LowerHex` implementation, fixed
above. A subsequent cold native archive build exceeded the first storage test's
five-second timeout; the warmed Node 26 suite passed without increasing timeouts
or weakening tests. Remote GitHub workflow execution and dependency-update PRs
cannot be verified until these local changes are pushed.
