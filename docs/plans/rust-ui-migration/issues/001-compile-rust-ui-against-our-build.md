# 001: Compile rust-ui components against our Dioxus 0.7.10 web build

Status: resolved
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

## Answer

**Yes, with four small edits.** In the gitignored scratch crate `.scratch/rust-ui-spike`
(standalone workspace, own target), `button`, `popover`, `select` and `dialog` at upstream
`7120636e`, with hooks `use_random` and `use_can_scroll_vertical`, pass
`cargo check --target wasm32-unknown-unknown` and a release `dx build --platform web`
(39 s) against `dioxus =0.7.10` with our features (`minimal`, `web`, `mounted`). Needed:

- crates `tw_merge 0.1`, `icons 0.18` (`dioxus`; replaced if the icon set changes, see
  [008](008-choose-icon-set.md)) and `strum 0.28` (`derive`, for `SelectPosition`);
- `Button`'s `href` branch uses the router's `Link`, which our build lacks: replaced with `a`;
- the registry modules import each other through `crate::hooks::…`, so copies need paths adjusted.

**Browser check (headless Chromium, no Tailwind CSS in the spike, so look is not assessed):**
the page renders with no console errors; the inline scripts do execute under client-side
Dioxus. Popover and Select open on click and close on Escape; Dialog opens and closes on Escape;
Button click updates Rust state; Select click picks a value and reports it through `on_change`.
Confirmed gaps: Enter on a focused Select option does nothing; the Select trigger shows the raw
value (`b`) not the label (`Beta`); the trigger has no `role` or `aria-*`; icon `<title>` text is
included in the trigger's text. Not tested: styling, anchor positioning, theme tokens,
Dioxus 0.7.9 vs 0.7.10 differences beyond compiling.

Consequence: running rust-ui as shipped is possible, but [009](009-close-rust-ui-behaviour-gaps.md)
already replaces its behaviour, so only markup and classes are taken from these four.
