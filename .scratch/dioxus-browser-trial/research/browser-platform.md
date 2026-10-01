# Dioxus browser platform evidence

Investigated 2026-09-30 for **Verify the Dioxus browser platform and toolchain**.
This is research for a browser-first trial covering layout editing and case generation;
it does not select the architecture or demonstrate a working application.

## Version and toolchain

GitHub's latest-release endpoint currently redirects to **v0.7.10**. Use that as a
conditional trial candidate, pinning `dioxus = "=0.7.10"` with its `web` feature
and matching **dioxus-cli 0.7.10** if the architecture decision adopts Dioxus web.
The published crate documents `web` separately from fullstack/server: a browser
client need not introduce a Rust server. [Release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10),
[crate reference](https://docs.rs/dioxus/0.7.10/dioxus/).

Tagged manifests declare Rust **1.83.0** for Dioxus and **1.82.0** for the CLI.
The current repository's `rust-toolchain.toml`, read without modification, pins
**1.98.0** and `wasm32-unknown-unknown`. Those declared floors do not establish a
need to change it; transitive dependencies and an actual build still need testing.
[Dioxus manifest](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/dioxus/Cargo.toml),
[CLI manifest](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/Cargo.toml).

The release workspace specifies semver dependency requirements `wasm-bindgen
0.2.100`, `web-sys 0.3.77`, `js-sys 0.3.77`, and `wasm-bindgen-futures 0.4.50`;
these are not exact lockfile resolutions. Commit the trial's resolved Cargo.lock
and inspect it before selecting browser binding versions. The CLI determines
wasm-bindgen tooling from the application's resolved crate version and verifies
that tool; it also gets/installs esbuild for JavaScript asset processing. Provision
these tools explicitly for restricted/offline builds rather than assuming the CLI
is self-contained. [Workspace requirements](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/Cargo.toml),
[tagged web bundler](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/build/web.rs).

Parent-agent read-only environment lookup found `cargo` and `rustc` at
`/usr/bin`, but no `dx` executable on PATH. Provisioning the pinned CLI is a
concrete build prerequisite; this research did not install it.

## Capability boundaries

| Area | Documented evidence | Application work / trial gate |
| --- | --- | --- |
| Pointer, keyboard, wheel, focus | Dioxus 0.7.10 exposes PointerData, KeyboardData, WheelData, mount and pointer-capture event handlers. | Implement selection, dragging, capture ownership, coordinate transforms, cancellation, shortcuts, and focus policy; verify actual browser behavior. |
| SVG layout | Dioxus provides RSX; official SVG example shows svg/rect/circle and events. Context7 returned the SVG example from main, so this particular example is not version proof. | Compile representative SVG against the exact pin, then test viewBox coordinates, zoom, nested targets and drag outside the canvas. |
| Canvas mount and effects | Versioned hook documentation shows onmounted, measurement and effect-driven canvas access. Its sample uses document::eval. | Rust application code can instead use browser bindings; exact DOM retrieval/cast must be compile-proven. Ensure initialization happens after mount and release observers/listeners/frame loops/resources on teardown. |

[Versioned event reference](https://docs.rs/dioxus-html/0.7.10/dioxus_html/events/index.html),
[versioned effects guide](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/hooks/docs/side_effects.md),
[official SVG example, unpinned](https://github.com/DioxusLabs/dioxus/blob/main/examples/02-building-ui/svg.rs).

`web-sys` supplies feature-gated browser WebIDL bindings. The inspected current
0.3.106 reference includes canvas/WebGL, ResizeObserver, Worker, IndexedDB,
Blob/File/URL, and service-worker/cache types. This establishes the Rust browser
adapter route, not compatibility of every exact signature with the trial's future
lockfile; 0.3.77 API pages were unavailable. Re-check version-matched APIs after
resolution. [Binding reference](https://docs.rs/web-sys/0.3.106/web_sys/).

- **3D:** Dioxus mounts UI/canvas; it is not evidence that the application's CAD
  engine and existing Rust three-d renderer work correctly when connected to
  Dioxus's mounted canvas lifecycle. Verify mesh buffers, camera/orbit/picking,
  WebGL context loss, resize/DPR and renderer teardown in that integration.
  Confirm the existing renderer's WASM features separately. This is an integration
  inference from browser binding availability, not a renderer recommendation.
- **Workers:** Browser workers run scripts in a separate global and communicate
  through messages. Rust computation can target WASM inside a worker, but the
  worker entry script/loading strategy, module URLs, serialization/transfer,
  cancellation, lifetime and packaging remain to be proven. A Dioxus async task
  alone does not establish off-main-thread CPU execution. [HTML worker standard](https://html.spec.whatwg.org/multipage/workers.html).
- **Persistence/files:** The binding types allow Rust browser adapters for IDB,
  file inputs, Blob URLs and downloads. Project serialization, IDB transactions
  and upgrades, reload integrity, error handling and URL cleanup remain app work.
  The repository already has Rust-owned archive processing that may be reused;
  verify its browser integration and feature compatibility. Neither Dioxus file
  events nor a Blob performs ZIP encoding. These are design
  implications, not tested integrations. [Browser bindings](https://docs.rs/web-sys/0.3.106/web_sys/).

## Static hosting, subpaths and JavaScript

The tagged bundler describes SPA output containing public files and no server.
It generates WASM plus JavaScript bindings, injects a module loading script, and
substitutes `{base_path}`, `{wasm_path}` and `{js_path}`. Its generated asset and
split-chunk URLs incorporate the configured base path. This supports investigating
static subpath hosting; it does not prove router reload fallback, manually authored
asset/worker URLs, or compatibility with the existing deployment. Compile and
serve the production bundle at both `/` and a non-root path. [Tagged bundler](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/build/web.rs).

Generated JavaScript remains part of the web runtime, consistent with the user's
allowance for generated glue. Rust handlers do not require adopting the eval-based
canvas sample as maintained application JS. Worker and service-worker entrypoints
are a separate packaging gate: classify authored runtime policy versus generated
bootstrap, and prove a Rust-authored/generation route before declaring the final
codebase free of maintained application JavaScript.

## Offline bootstrap and deployment

Version-matched Context7 returned an official PWA example with custom HTML,
manifest and service worker, and explicitly says the worker must be edited for a
real project. Its sample registers an assets-path worker. Do not copy that URL
without checking scope: the service-worker standard constrains default scope by
script location, with broader scope requiring appropriate server permission.
[Dioxus PWA guide](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/examples/10-integrations/pwa/README.md),
[Service Worker standard](https://w3c.github.io/ServiceWorker/).

Offline support requires an application cache/update policy, trustworthy-origin
registration, successful initial download/install and control. Cache the complete
boot dependency graph (HTML, generated loader/bindings, WASM, worker/CAD binaries,
assets and any optional chunks); verify actual fetches rather than assuming a
manifest covers them. Test offline reload after online installation and a version
upgrade with existing project data. A first-ever offline visit cannot fetch an
uncached app from the network. These gates follow from service-worker install,
control and cache semantics; Dioxus bundling alone provides no offline guarantee.
[Service Worker standard](https://w3c.github.io/ServiceWorker/).

## Decision input and remaining gates

If the architecture decision chooses a client-side Dioxus DOM app, 0.7.10 is a
verified candidate with no declared Rust-floor obstacle. Preserve the existing
Rust pin until actual dependency resolution or build evidence requires otherwise.
No documentation establishes feasibility, performance or feature parity for this
specific application.

Candidate evidence for the human architecture/acceptance decision includes:
a locked successful web build; SVG drag/keyboard
and Undo behavior; a Rust 3D renderer and case-generation path; responsiveness and
worker startup; persistence/reload and real export files; resource teardown/remount;
production subpath boot; offline boot/update; and the runtime-JS inventory. Those
results could inform the staged migration decision rather than be inferred from
framework support. The dependent human decision still chooses the exact trial
scope and acceptance gates.

Research used Context7 `/dioxuslabs/dioxus/v0.7.10` and primary source references.
Some Context7 results referred to main; these were not treated as tagged guarantees.
Several exact docs/source URLs were unavailable through the web tool, and direct
shell fetching failed DNS. No tools were installed, no app was built, and no
production code/configuration was changed.
