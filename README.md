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
0.7.10 and Python 3.11+. The default checks use the Rust browser harness and do not require Node or Playwright. KiCad CLI 10 is required for PCB integration checks. Browser tests require
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

**Optional Node/npm execution check.** On Linux, install `strace` and run
`python3 scripts/verify-no-node.py -- COMMAND [ARG ...]` around the command to check.
The verifier first runs its own PATH and absolute-path positive controls, then traces
the command and descendants. It fails on attempts to execute a pathname named `node`
or `npm`, on command failure, or if tracing cannot be verified. It observes only the
traced process tree: renamed or embedded runtimes and the GitHub Actions runtime are
outside its scope. It does not change CI or replace the required verification checks.

**Routine loop.** `quick` takes the files changed against `main` (plus untracked files; or name
them with `--files`) and runs the documentation checks, rustfmt and Clippy, WASM page compilation,
the native tests of the changed crates and everything that depends on them, and the browser tests
of the affected presentation crates (page tests by file). It never runs `build`. A shared input
(`Cargo.toml`, `Cargo.lock`, `.cargo/`) or a path it cannot place falls back to every Rust check and
says so. Documentation-only changes run only the documentation checks. Rust checks first run a
Dioxus page build to generate the Tailwind stylesheet required by WASM test fixtures; they do not
run the full provider-and-packaging `build` step.

**Full check.** The default runs documentation link checks, build and content tool tests, rustfmt and
Clippy, native Rust and provider tests, a checked web build, native/WASM boundary checks and mounted
headless browser tests. Name steps (`python3 scripts/check.py --list`) to run only some. Each command
and step prints its wall time. The checked `build` skips `wasm-opt` and the optimized page build; the
opt-in `release` step (`python3 scripts/check.py release`) keeps both and is what ships.
Running the `browser` step by itself first rebuilds the checked page so its real-page drag smoke
uses current frontend assets.

**Old-CSS count.** The `repo` step reports how many `.board-*` and `.m1-*` selectors and `--wb-*` variables
remain under `web/` (distinct names, build output skipped) and never fails on them. `python3 scripts/check.py old-css`
(or `python3 scripts/check-old-css.py --strict`) fails while any remain; it is not in the default run and gates the
final merge to `dev` only.
Workbench Theme tokens and Tailwind utilities live in `web/tailwind.css`; document defaults,
fonts, and shared finding outlines live in `web/assets/workbench-base.css`.

**KiCad in native tests.** The native tests call `kicad-cli` (KiCad 10) and fail with a clear message when it
is missing; CI installs it. Only the generator export check (`core/tests/generator_export.rs`) can be skipped
on purpose, with `BOARDSTUDIO_ALLOW_NO_KICAD=1`; the KiCad integration tests have no skip.

**Browser tests.** The page runner checks that every listed test executes. The presentation crate suite
builds the selected lib test binaries in one Cargo invocation, then runs them with the locked
`wasm-bindgen-test-runner` from each crate directory. The checked crate build enables
`boardstudio-web-host/page` when testing `host`, and the separate page binary
test build enables `boardstudio-web-host/test-support` so both full builds resolve the same shared
features. Each launch starts its own `chromedriver` and Chrome.
Layout uses a desktop viewport for the setup guide and the other layout tests. The page filters
share the local browser job budget with the crate launches (up to six jobs by default, bounded by CPU
count, or `BOARDSTUDIO_CHECK_JOBS=1` for serial), with a two-minute timeout per
launch (`WASM_BINDGEN_TEST_TIMEOUT`). Use `python3 scripts/check.py browser --only <crate>...` for a
partial crate run without the packaged build or drag smoke. Add `--keep-going` to continue across
independent browser launches and still exit nonzero on failure. CSS-dependent subsets generate
the test stylesheet first. Preflight uses the patched pair in
`~/.cache/boardstudio/wasm-bindgen-patched/` (under `XDG_CACHE_HOME` when set) and builds it
on a cache miss. Set `BOARDSTUDIO_WASM_BINDGEN_DIR` to override both tools at the version in
`Cargo.lock`. If the patched build inputs are unavailable, the runner warns before using a
matching stock pair from `PATH` or wasm-pack's cache.
For source-level debugging, run `wasm-pack test --headless --chrome web/crates/<crate> --lib` by hand
with `CARGO_PROFILE_DEV_DEBUG` unset. Add `--features page` for `host`.
The browser step opens the packaged page and runs a trusted mouse-drag smoke over
Part, Column, Stagger and Ctrl+Offset. It checks live preview, pointer capture, saved document state
and exact Undo. Its success receipt and failure log are kept under `web/target/test-results/drag-browser/`.
Browser checks use the Rust harness and a Chrome or Chromium executable available as `CHROME_BIN`.

**Verification receipts.** Add `--receipt PATH` to any check invocation to write a JSON receipt at
`PATH` and a reviewer summary at `PATH.md`, both derived only from the output that invocation
already captured. The receipt keeps native `cargo test` binaries, browser crate
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
`target/`), so dependencies compile once per profile and target. The CAD workspace
at `cad/monstertruck` contains the library and the `cad/wasm` member. It has a
separate lockfile and patches the vendored Monstertruck crates. Native CAD tests
use `cad/monstertruck/target`, and the wasm provider uses `cad/wasm/target`.

The repository has no TypeScript or Node dependency; Python scripts handle build, content preparation and checks. Footprint generation runs in Rust inside Core and the page. User-supplied
JavaScript generators are not executed. Bundled model export includes used assets;
missing or unsupported model paths fail explicitly.

See [architecture](docs/architecture.md), [domain vocabulary](GLOSSARY.md),
[component onboarding](docs/hardware/component-onboarding.md) and
[deferred issues](docs/backlog.md). Migration history is preserved in Git at
`323967ff`; it imposes no development or acceptance process on this branch.
The [documentation index](docs/README.md) links feature references, investigations,
design decisions and historical research.
