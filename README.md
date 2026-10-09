# BoardStudio

BoardStudio is an offline-capable keyboard design workbench built with Dioxus and
Rust. It combines layouts, PCB preparation, keymaps, keycaps, mechanical assemblies
and portable exports. KiCad and CAD tools handle downstream routing and fabrication.

## Development

Install the Rust version in `rust-toolchain.toml`, `wasm-pack` 0.15.0, Dioxus CLI
0.7.10 and Python 3.11+. No Node, npm or pnpm is needed. KiCad CLI 10 is required for PCB integration checks. Browser tests require
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
python3 scripts/build-web.py
python3 scripts/serve-web.py web/target/site 4173
```

The build creates `web/target/site/site-subpath/boardstudio`, the `/boardstudio/` route
GitHub Pages publishes. Add `--routes subpath,root` to also package the root route in
`web/target/site/site-root`. Preview serves the built routes at `http://127.0.0.1:4173/`
(redirecting `/` to `/boardstudio/` when only the subpath was built) with the isolation
headers required by browser workers. Production build and preview are separate
from the watched development loop. Development uses an online server; use the
production preview to test offline behavior.

The normal Dioxus service worker supports offline use and application updates.
React, its deployment handoff and rollback tooling are retired. Version 1 projects migrate to document format version 2 when loaded; current
projects save locally and round-trip through `.boardstudio` exports. Existing user files are never erased by
build or cleanup commands.

## Checks

Two commands, by purpose:

```sh
python3 scripts/check.py quick   # the routine loop: only what your changes touch
python3 scripts/check.py         # the full pre-PR check
```

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

**Browser tests.** The runner checks that every listed test actually executes. Each presentation crate
under `web/crates/` is one test launch (`wasm-pack test --headless --chrome web/crates/<crate> --lib`,
which starts its own `chromedriver` and Chrome); the Case workspace, panel and setup guide tests run
separately because their mounted fixtures need an isolated DOM. The check runs up to four crate launches
at once (`BOARDSTUDIO_CHECK_BROWSER_JOBS`, `1` for serial) and builds test wasm without debug info
(`CARGO_PROFILE_DEV_DEBUG=false`, which is most of a launch's cost; run `wasm-pack test` by hand,
without the variable, for source-level debugging). Each batch has a bounded two-minute timeout,
overridable with `WASM_BINDGEN_TEST_TIMEOUT`. A native test run alone does not compile WASM-only
presentation code. Browser setup installs the locked wasm-bindgen runner before executing tests.

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
`target/`), so dependencies compile once per profile and target. `cad/wasm` and
`cad/step-oracle` stay outside it because they build against the pinned WASI and
native OCCT toolchains.

The repository has no TypeScript or Node dependency; Python scripts handle build, content preparation and checks. Footprint generation runs in Rust inside Core and the page. User-supplied
JavaScript generators are not executed. Bundled model export includes used assets;
missing or unsupported model paths fail explicitly.

See [architecture](docs/architecture.md), [domain vocabulary](CONTEXT.md),
[component onboarding](docs/hardware/component-onboarding.md) and
[deferred issues](docs/backlog.md). Migration history is preserved in Git at
`323967ff`; it imposes no development or acceptance process on this branch.
The [documentation index](docs/README.md) links feature references, investigations,
design decisions and historical research.
