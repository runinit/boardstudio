# 001: Compile rust-ui components against our Dioxus 0.7.10 web build

Status: open
Type: task
Blocked by: None
Parent: [Map](../map.md)

## Question

Do copied rust-ui `button`, `popover`, `select` and `dialog` sources, with `tw_merge`,
compile and render in a disposable scratch crate pinned to our `dioxus = "=0.7.10"` web
target (upstream pins 0.7.9)? Record the upstream commit, extra crates and features they
pull in (`js-sys`, `web-sys`, `gloo-timers`, `icons`, `time`), wasm build result, and any
source edits needed. Drives AFK; nothing tracked changes.

## Comments

**2026-10-21, progress (claim released for 009):** In the gitignored scratch crate
`.scratch/rust-ui-spike` (own target, outside the workspace), `button`, `popover`, `select`
and `dialog` at upstream `7120636e`, plus hooks `use_random` and `use_can_scroll_vertical`,
pass `cargo check --target wasm32-unknown-unknown` against `dioxus =0.7.10` with our
features (`minimal`, `web`, `mounted`). Needed: `tw_merge 0.1`, `icons 0.18` (`dioxus`),
`strum 0.28` (`derive`, for `SelectPosition`), and one edit: `Button`'s `href` branch uses
the router's `Link`, which our build lacks; replaced with `a`. Remaining: a browser render,
chiefly whether the inline `<script>` blocks that drive popover, select and dialog execute
when Dioxus inserts them.
