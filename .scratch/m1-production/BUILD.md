# M1 reproducible build and staging

Status: complete release `m1-release-20261001-b9748745` passed all 18 build commands, 925 source hashes and both 47-file prefix inventories, with one current page WASM per prefix. [Portable provenance](evidence/integration/release-b9748745-provenance.json) retains the exact inputs. It includes the reviewed pointer allocation repairs; bounded root/subpath browser smoke passes; startup restoration/instance UI repairs and final five-session checks remain pending under [the ledger](ACCEPTANCE.md). Previous builds and failures remain retained; never overwrite earlier outputs.

## Toolchain and package layout

Keep Rust 1.98.0, Dioxus library/CLI 0.7.10, wasm-bindgen 0.2.129, js-sys/web-sys 0.3.106, serde-wasm-bindgen 0.6.5 and wasm-bindgen-futures 0.4.79. Use installed version-matched primary source for uncertain framework APIs. Core, renderer, CAD and contracts retain independent manifests. New application and web packages commit independent Cargo.lock files; no root workspace conversion.

Expected maintained packages: headless application crate with public native behavior tests; web package with separate host/presentation modules and feature-gated core/CAD/page/offline entrypoints. Production modules have no runtime dependency on scratch probes. Adopted source decisions receive dedicated verification on the reconciled provider base.

## Ordered build inputs

1. Record integration HEAD, tracked source/manifest/lock hashes and embedded provider inputs. Source may not change between captured provenance and final artifacts.
2. Prepare the existing catalog and copied fixture inputs using the accepted reference services. Freeze complete REVIUNG41 and Sofle documents/assets with hashes and preparation commands. Do not substitute outline-only data for saved project fixtures.
3. Build core and CAD/renderer providers using their pinned locked toolchains. Native CAD fixture prerequisites and generated contracts must be fresh for the reconciled source. Do not silently reuse older WASM after provider changes.
4. Build the maintained Rust worker packages and generated initialization-only JavaScript. CAD package initialization stays separately built and reviewed; no kernel/private API substitution. Worker transport retains JSON text identities and owned binary transfers.
5. Build the Rust service-worker policy independently of page/archive/core dependencies. Generate its synchronous initializer and complete static asset manifest from actual release outputs. Cache policy remains Rust; loader source must contain no authored policy.
6. Run locked Dioxus release builds separately for / and /boardstudio/. Stage each into a new uniquely named output directory, retaining exact original output. Preserve same-origin current-prefix worker/CAD/renderer/fixture URLs.
7. Reverify captured source and copied asset hashes. Record tool versions, command argv/cwd/features, return codes, output logs, prefix URLs and all staged hashes. Fail on missing/mismatched files rather than inventing a successful manifest.

## Native and WASM checks

Run fmt, locked tests and strict all-targets Clippy on each affected manifest, plus strict supported WASM feature combinations. Root reference commands remain check:repo, check:contracts, test:contracts, affected package suites/builds, check:boundaries and the applicable browser matrix. Application native tests consume the real public CoreEngine; web browser tests execute actual workers/storage/renderer.

## Serving and copied data

Owned local servers use available unique ports and serve only their staged candidate. Root/subpath routes receive fresh named agent-browser sessions, with recorded viewport/DPR and startup request errors. Never attach to or terminate a pre-existing browser/server. Trial database and preference keys remain distinct from boardstudio-v2; a scoped archive exchange adapter is the only bridge to the reference.

Cached offline checks run after real install and registration control, then sever network on the owned origin. A fresh browser context without service worker/cache must fail honestly. Test asset availability, root/subpath separation and owned cache version update, preserving unrelated responses.

## User-test handoff

After acceptance, provide a complete editable copied-project demo and concrete local launch instructions with tested artifacts. Describe any unavailable checks or compatibility limits. Do not offer a built probe as completed M1, change production data, push, deploy or remove React.

## Maintained build entrypoint

Run `python3 scripts/build-m1.py <unique-build-id>` from the integration checkout.
The script refuses an existing `web/target/builds/<unique-build-id>` directory,
rebuilds locked core/core-worker/CAD-worker/renderer/CAD modules, prepares complete
copied fixtures, and stages separate Dioxus root and subpath releases. Each stage
receives its own full manifest and Rust offline policy. `provenance.json` retains
source hashes, argv, cwd, features, environment, command times, exit codes, logs
and staged file hashes. A source change during the build fails provenance.

Serve the resulting `site-root` and `site-subpath` directories with separate owned
local HTTP servers. Open `/` on the root server and `/boardstudio/` on the subpath
server. Trial storage uses `boardstudio-m1-root` and `boardstudio-m1-boardstudio`;
production `boardstudio-v2` is untouched. Browser acceptance must use these
release artifacts. A Dioxus development page with separately staged provider
assets is useful for repairs but is not final release evidence.

The current canvas shows exact case bodies and a nominal PCB contour reference.
M1 permits embedded part definitions without executing generator authoring/PCB
export (ADR 0003); this does not assert populated PCB preview or generator parity.
Physical-instance case settings use the reference shared-construction policy and
the existing undoable document replacement command. Setting controls wait for
durable edits and position previews to finish before constructing that update.

## Fresh Dioxus page output

Before each prefix build the builder preserves the previous Dioxus `public`
directory at `previous-dx-public-<next-mode>` inside the unique build directory.
The suffix identifies the next build: the `subpath` snapshot therefore contains
the preceding root output. Only the freshly rebuilt public directory enters
that prefix's stage and offline manifest. Earlier hashed page WASM files never
enter the new stage. The source and all staged assets are reverified after both
prefixes finish. The actual stale-output regression and fresh inventory are
retained under `evidence/integration/`.
