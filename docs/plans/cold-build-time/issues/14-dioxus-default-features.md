# 14: Remove Dioxus default features

Status: wontfix
Type: research
Blocked by: none
Parent: [map](../map.md)
Related: [07](07-inflector-regex.md)

## Question

Can BoardStudio stop pulling Dioxus's `devtools` and `logger` through `dioxus-primitives`, and is it worth it?

## Answer

It works, but no. It saves 41 CPU-s and about 3 s of wall, and needs a permanent vendored fork plus a behaviour
change. The regex cost it targeted is elsewhere: see [07](07-inflector-regex.md).

## Evidence

- `dioxus-components@9a75825/primitives/Cargo.toml` has `dioxus.workspace = true`, and its workspace has
  `dioxus = "0.7.8"` with default features. BoardStudio's own Dioxus lines all set `default-features = false`.
  `cargo tree -e features -i dioxus` confirms `dioxus-primitives` as the source of `default`.
- Dioxus 0.7.10: `default = ["launch", "devtools", "logger", "lib"]`.
- Measured with `cargo tree --prune dioxus-logger --prune dioxus-devtools`, packages that leave:
  - wasm: `dioxus-devtools`, `dioxus-devtools-types`, `dioxus-logger`, `lazy_static`, `matchers`, `regex-automata`,
    `regex-syntax`, `sharded-slab`, `thread_local`, `tracing-subscriber`, `tracing-wasm`. Cost: 26.5 CPU-s.
  - host: also `tungstenite`, `http`, `httparse`, `sha1`, `data-encoding`, `rand`, others. Cost: 14.7 CPU-s. Host
    regex stays, because `core/Cargo.toml` has it as a dev-dependency.
- Of about 170 CPU-s of regex-family crates: CAD 130, harness 11, root host 14, root wasm 18.5. Only the last is here.
- Attributed wall: about 3 s, all early and parallel.

## Why it is not worth it

- **Fork.** A dependency cannot have its dependency's features removed from outside. This needs a
  `[patch."https://github.com/DioxusLabs/dioxus-components"]` with a vendored `dioxus-primitives` and
  `dioxus-attributes`, kept in step with upstream, or an upstream change and a pin bump.
- **Dev feature.** `dx serve` hot reload relies on `devtools`, which arrives only by this accident today.
  `scripts/dev-web.py:212-214` would need a new `devtools` feature, and a miss degrades hot reload quietly.
- **Behaviour.** `logger` controls `launch()`'s default tracing setup, so browser console output changes, and
  shipped pages may lose logs. Not traced further, since the gain does not pay for it.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| about -3 s | -41 (-1.3%) | none |

Effort: 1 day plus upkeep. Risk: medium.

## Reopen when

Upstream `dioxus-components` sets `default-features = false` on its workspace `dioxus`. Then this is a pin bump plus
adding `devtools` in `dev-web.py`.

Verification then: `cargo tree --locked --offline -p boardstudio-web --no-default-features --features page --target
wasm32-unknown-unknown -i tracing-subscriber` finds nothing, and hot reload still works under `python3
scripts/dev-web.py`. Stop rule: revert if hot reload or console output degrades.
