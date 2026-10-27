# BoardStudio

BoardStudio is an offline-capable keyboard design workbench built with Dioxus and
Rust. It combines layouts, PCB preparation, keymaps, keycaps, mechanical assemblies
and portable exports. KiCad and CAD tools handle downstream routing and fabrication.

## Workbench updates

Visibility controls have distinct scopes: Canvas visibility (Layout), PCB layers,
Keymap layers, Assembly visibility (Case), and Parts preview layers. Canvas, PCB,
assembly, and parts visibility controls sit at the bottom of Objects; Keymap layers
remain in the Keymap Objects list. Visibility changes affect the view without adding
document edits or undo entries. Hold Ctrl when a Stagger drag starts to offset the
whole column.

## Development

Install the Rust version in `rust-toolchain.toml`, `wasm-pack` 0.15.0, Dioxus CLI
0.7.10 and Python 3.11+. Node is not needed to build or run the app; it is required for the `tooling` check step and the browser drag smoke (the Browser tests paragraph covers the `NODE_PATH` detail). KiCad CLI 10 is required for PCB integration checks. Browser tests require
Chrome/Chromium and its compatible ChromeDriver.

```sh
python3 scripts/dev-web.py
```

The development server runs at `http://127.0.0.1:8080/`. It builds missing runtime
providers on the first run, then Dioxus watches page edits in the working tree.
Edits that only change `rsx!` markup are hot-reloaded without recompiling; other Rust
edits rebuild the page. The page builds with the `wasm-dev` profile in `Cargo.toml`,
which keeps line tables only; set `debug = true` there to inspect variables in the
browser debugger, at roughly twice the rebuild time.
Commits, agent registration and migration records are not required.
After changing a worker, bundled example or CAD provider, run
`python3 scripts/build-web.py --providers-only` to regenerate those assets before testing the page.

## Build and preview

```sh
# Checked, unoptimized build for local feature testing
python3 scripts/build-web.py --fast
python3 scripts/serve-web.py web/target/site 4173
```

`--fast` skips `wasm-opt` for runtime providers and builds the Dioxus page without
release optimization. Use `python3 scripts/build-web.py` for the optimized release
build.

The build creates `web/target/site/site-subpath/boardstudio`, the `/boardstudio/` route
GitHub Pages publishes. Add `--routes subpath,root` to also package the root route in
`web/target/site/site-root`. Preview serves the built routes at `http://127.0.0.1:4173/`
(redirecting `/` to `/boardstudio/` when only the subpath was built) with the isolation
headers required by browser workers. Production build and preview are separate
from the watched development loop. Development uses an online server; use the
production preview to test offline behavior.

The normal Dioxus service worker supports offline use and application updates.
React, its deployment handoff and rollback tooling are retired. This integration lane reads
document format 7 only; older projects are retained but cannot be converted by this build.
Current projects save locally and round-trip through `.boardstudio` exports. Existing user
files are never erased by build or cleanup commands. Do not promote this lane to user-data
stores before the layout-parity acceptance gate.

## Local project safety

If another tab saves the same keyboard, a conflicting save stops and keeps your pending
edits. Download those edits before choosing to reload the saved version. Unreadable saved
projects appear separately, with raw-data download and confirmed deletion actions.

Undo keeps the latest 100 committed edits. Project imports are limited to 128 MiB.
Unused imported assets are reclaimed at a later safe startup, after other editors close.
Cleanup is deferred when browser coordination is unavailable or any saved project is unreadable.

## Checks

Two commands, by purpose:

```sh
python3 scripts/check.py quick   # the routine loop: only what your changes touch
python3 scripts/check.py build   # checked web build without wasm-opt
python3 scripts/check.py browser # mounted browser tests and browser smoke checks
python3 scripts/check.py         # the full pre-PR check
```

A single Core Rust test runs with `cargo test -p boardstudio_core --test integration <filter>`.

**Routine loop.** `quick` takes the files changed against `main` (plus untracked files; or name
them with `--files`) and runs the documentation checks, rustfmt and Clippy, WASM page compilation,
the native tests of the changed crates and everything that depends on them, and the browser tests
of the affected presentation crates (page tests by file). It never runs `build`. A shared input
(`Cargo.toml`, `Cargo.lock`, `.cargo/`) or a path it cannot place falls back to every Rust check and
says so. Documentation-only changes run only the documentation checks.

**Full check.** The default runs documentation link checks, build and content tool tests, rustfmt and
Clippy, native Rust and provider tests, a checked web build, native/WASM boundary checks and mounted
headless browser tests. Name steps (`python3 scripts/check.py --list`) to run only some. Each command
and step prints its wall time. The checked `build` skips `wasm-opt` and the optimized page build; the
opt-in `release` step (`python3 scripts/check.py release`) keeps both and is what ships.
Running the `browser` step by itself first rebuilds the checked page so its real-page drag smoke
cannot pass against stale frontend assets. Run `python3 scripts/check.py drag-smoke` for that small
drag preflight by itself before a longer browser or performance run.

**KiCad in native tests.** The native tests call `kicad-cli` (KiCad 10) and fail with a clear message when it
is missing; CI installs it. Only the generator export check (`core/tests/generator_export.rs`) can be skipped
on purpose, with `BOARDSTUDIO_ALLOW_NO_KICAD=1`; the KiCad integration tests have no skip.

**Browser tests.** The runner checks that every listed test actually executes. Each presentation crate
under `web/crates/` is one test launch (`wasm-pack test --headless --chrome web/crates/<crate> --lib`,
which starts its own `chromedriver` and Chrome); the Case workspace, panel and setup guide tests run
separately because their mounted fixtures need an isolated DOM. The check runs up to four crate launches
at once (`BOARDSTUDIO_CHECK_JOBS`, `1` for serial) and builds test wasm without debug info
(`CARGO_PROFILE_DEV_DEBUG=false`, which is most of a launch's cost; run `wasm-pack test` by hand,
without the variable, for source-level debugging). Use `python3 scripts/check.py browser --only <crate>...`
to run only the named presentation crate launch(es) without the full page build or drag smoke (reported
as a partial subset, not full acceptance), and add `--keep-going` to continue across independent browser
launches while still exiting nonzero if any failed. Each batch has a bounded two-minute timeout,
overridable with `WASM_BINDGEN_TEST_TIMEOUT`. A native test run alone does not compile WASM-only
presentation code. Browser setup installs the locked wasm-bindgen runner before executing tests.
The browser step opens the packaged page and runs a trusted mouse-drag smoke over
Part, Column, Stagger and Ctrl+Offset. It checks live preview, pointer capture, saved document state
and exact Undo. Its success receipt and failure log are kept under `web/target/test-results/drag-browser/`.
CI installs the pinned `playwright-core` runner; local browser checks need it on `NODE_PATH` and a
Chrome or Chromium executable available as `CHROME_BIN`.

The manual Graphite feedback check has an experimental Rust entry point,
`python3 scripts/run-graphite-feedback.py`, against `BOARDSTUDIO_PREVIEW_URL`
(default `http://127.0.0.1:8182/`). It accepts `CHROME_BIN` and optional
`BOARDSTUDIO_SCREENSHOTS`. The Node feedback and usability commands remain
the established entry points while Rust parity and failure controls are open.
`python3 scripts/run-graphite-usability.py` runs the same experimental Rust
workflow, matching the Node usability entry point. Set `BOARDSTUDIO_GRAPHITE_IDENTITY=1`
to print the checkout, HEAD, dirty source digest and preview URL before either runner. Set
`BOARDSTUDIO_GRAPHITE_BUILD_DIR` to also digest a local build directory. These are launch-time
observations: an arbitrary server is not bound to that build or source, so its served identity
remains unverified unless established separately.

**Verification receipts.** Add `--receipt PATH` to any check invocation to write a JSON receipt at
`PATH` and a reviewer summary at `PATH.md`, both derived only from the output that invocation
already captured. The receipt keeps native `cargo test` binaries, direct `wasm-pack test` crate
launches, the `web/src` page suite and the provider/product smoke checks in separate scopes, each
with its full observed test names and outcomes, module-prefix group counts, and its own exit
status, completeness and problems. Planned-but-unrun commands are `not_run`; malformed or
interrupted output is incomplete, never zero failures, and a complete page run with a failing test
is not reported as passing. The native section reuses the name guard's matched/missing/new
retention semantics and identifies the guard list's recording revision and local changes.
Before/after group deltas stay `unknown` unless a base run is supplied with `--base-receipt PATH`,
which is then labelled reused with its original run and source identity. Provide the summary (or
the receipt) at review handoff.

## Source layout

| Directory | Purpose |
| --- | --- |
| `web/` | Dioxus UI, browser host, workers and offline behavior |
| `application/` | Accepted state, interactions and scoped operations |
| `core/` | Document engine, geometry, artifacts and archives |
| `renderer/`, `cad/` | Rendering and CAD providers |
| `contracts/` | Shared Rust boundary types |
| `footprints/` | Built-in Rust footprint generators, normalization and model references |
| `ergogen/library/` | Vendored models, source manifests and attribution |
| `catalogue/`, `content/` | Components, models, licences and bundled examples |
| `core/examples/demo_projects/`, `content/layouts/` | Native bundled-project recipes and measured source layouts |
| `scripts/` | Build, content preparation and checks |

The Rust crates share one Cargo workspace (root `Cargo.toml`, `Cargo.lock` and
`target/`), so dependencies compile once per profile and target. `cad/wasm` stays
outside it because it patches the vendored Monstertruck crates with its own lockfile. It declares an empty `[workspace]` so cargo treats it as its own
root, which also lets it build from a worktree nested under another checkout.

The repository has no TypeScript or Node dependency; Python scripts handle build, content preparation and checks. Footprint generation runs in Rust inside Core and the page. User-supplied
JavaScript generators are not executed. Bundled model export includes used assets;
missing or unsupported model paths fail explicitly.

See [architecture](docs/architecture.md), [domain vocabulary](GLOSSARY.md),
[component onboarding](docs/hardware/component-onboarding.md) and
[deferred issues](docs/backlog.md). Migration history is preserved in Git at
`323967ff`; it imposes no development or acceptance process on this branch.
The [documentation index](docs/README.md) links feature references, investigations,
design decisions and historical research.
