# Tailwind setup

This setup gives the UI lane a tracked Tailwind v4 input while leaving the current
screens and class names unchanged. Dioxus CLI 0.7.10 detects Tailwind from the
presence of `web/tailwind.css` and selects Tailwind 4.1.5 when no Tailwind v3
configuration file is present. The generated file is `web/assets/tailwind.css`;
it is ignored and created by Dioxus during `dx serve` or `dx build`. Upstream
integration supports watching the input during `dx serve`, but this lane's CSS
input edits did not refresh that generated file in the running session. Treat the
generated asset as the visual source of truth before assessing a CSS change; this
observed freshness gap is not a diagnosis of a Dioxus or Tailwind defect.

The input imports Tailwind's theme and utilities layers without importing its
Preflight/base reset. `source(none)` disables automatic scanning; the two `@source`
patterns include Rust presentation code under `web/src` and `web/crates`, including
class strings in RSX, while excluding generated `target` directories and worktrees.
Theme aliases point to existing `--wb-*` light/dark theme variables for colors,
fonts, type sizes and motion. They establish utility names for later batches without
changing the existing palette or migrating existing classes.

The stylesheet link follows the current unlayered stylesheets. Tailwind emits its
utilities in a cascade layer, and normal unlayered CSS takes precedence over layered
CSS even when the layered stylesheet is linked later. Existing component rules may
therefore continue to win when both stylesheets set the same property; new utility
classes should be evaluated against the rendered UI before migrating those rules.

For the private UI development loop, run from this worktree root with its own target
directory:

```sh
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 BOARDSTUDIO_CHECK_JOBS=1 BINARYEN_CORES=4 \
  CARGO_TARGET_DIR=/home/chris/.codex/worktrees/2182/ts-boardstudio2/target \
  python3 scripts/dev-web.py --port 8182
```

The proxy is at `http://127.0.0.1:8182`; it starts Dioxus on `127.0.0.1:8183` and
serves the lane's live assets. `dev-web.py` prepares missing providers before it
starts. To build the Dioxus page directly from `web/`, set the same worktree-local
`CARGO_TARGET_DIR` and run:

```sh
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=4 BOARDSTUDIO_CHECK_JOBS=1 BINARYEN_CORES=4 \
  CARGO_TARGET_DIR=/home/chris/.codex/worktrees/2182/ts-boardstudio2/target \
  dx build --web --no-default-features --features page --cargo-args=--locked
```

If a running `dx serve` session leaves the generated stylesheet stale, regenerate
CSS without rebuilding Rust or providers using the DX-provided Tailwind binary. The
path below is the installed Linux path in this worktree's host; on other installations,
use the Tailwind binary supplied by that DX version:

```sh
/home/chris/.local/share/dx/tools/tailwindcss-v4.1.5/tailwindcss \
  --input web/tailwind.css --output web/assets/tailwind.css
```

Confirm that `web/assets/tailwind.css` contains the new rules before judging browser
behavior. Regenerate provider assets with
`python3 scripts/build-web.py --providers-only` before judging browser behavior if
the provider inputs changed; `dev-web.py` only prepares missing provider assets.

Version and behavior provenance:

- [Dioxus CLI 0.7.10 Tailwind integration](https://raw.githubusercontent.com/DioxusLabs/dioxus/v0.7.10/packages/cli/src/tailwind.rs)
  defines the detected Tailwind 4.1.5 version, input and output paths, and serve/build
  behavior.
- [Tailwind CSS source detection](https://tailwindcss.com/docs/detecting-classes-in-source-files)
  documents `source(none)` and explicit `@source` paths.
- [Tailwind CSS theme variables](https://tailwindcss.com/docs/theme) documents
  mapping design tokens through `@theme inline`.
