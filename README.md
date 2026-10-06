# BoardStudio

BoardStudio is an offline-capable keyboard design workbench built with Dioxus and
Rust. It combines layouts, PCB preparation, keymaps, keycaps, mechanical assemblies
and portable exports. KiCad and CAD tools handle downstream routing and fabrication.

## Development

Install the Node version in `.node-version`, pnpm from `package.json`, the Rust
version in `rust-toolchain.toml`, `wasm-pack` 0.15.0, Dioxus CLI 0.7.10 and Python
3.11+. KiCad CLI 10 is required for PCB integration checks. Browser tests require
Chrome/Chromium and its compatible ChromeDriver.

```sh
pnpm install --frozen-lockfile
pnpm dev
```

The development server runs at `http://127.0.0.1:8080/`. It builds missing runtime
providers on the first run, then Dioxus watches page edits in the working tree.
Commits, agent registration and migration records are not required.
After changing a worker, bundled example or CAD provider, run
`pnpm build:providers` to regenerate those assets before testing the page.

## Build and preview

```sh
pnpm build
pnpm preview
```

The build creates `web/target/site/site-root` and
`web/target/site/site-subpath/boardstudio`. Preview serves the root and `/boardstudio/`
routes at `http://127.0.0.1:4173/` with the isolation headers required by browser workers.
GitHub Pages publishes the subpath build. Production build and preview are separate
from the watched development loop. Development uses an online server; use the
production preview to test offline behavior.

The normal Dioxus service worker supports offline use and application updates.
React, its deployment handoff and rollback tooling are retired. Version 1 projects migrate to document format version 2 when loaded; current
projects save locally and round-trip through `.boardstudio` exports. Existing user files are never erased by
build or cleanup commands.

## Checks

```sh
pnpm check
```

This runs repository/link and generated-contract checks, build-tool tests, native
Rust and provider tests, WASM page compilation, release packaging, native/WASM
boundary checks and mounted headless browser tests. Use individual scripts in
`package.json` for affected checks during development. The browser runner checks
that every listed test actually executes, grouped into broad batches to avoid repeated
WASM/browser startup costs. Three modules run separately because their mounted fixtures need an isolated
DOM. Each batch has a bounded two-minute timeout, overridable with
`WASM_BINDGEN_TEST_TIMEOUT`. A native test run alone does not compile
WASM-only presentation code. Browser setup installs the locked wasm-bindgen runner
before executing tests.

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
| `kicad/` | PCB integration checks against Core and KiCad |
| `catalogue/`, `content/` | Components, models, licences and bundled examples |
| `core/examples/demo_projects/`, `content/layouts/` | Native bundled-project recipes and measured source layouts |
| `scripts/` | Build, content preparation and checks |

TypeScript/JavaScript is retained for working providers and tooling. Footprint generation runs in Rust inside Core and the page. User-supplied
JavaScript generators are not executed. Bundled model export includes used assets;
missing or unsupported model paths fail explicitly.

See [architecture](docs/architecture.md), [domain vocabulary](CONTEXT.md),
[component onboarding](docs/hardware/component-onboarding.md) and
[deferred issues](docs/backlog.md). Migration history is preserved in Git at
`323967ff`; it imposes no development or acceptance process on this branch.
