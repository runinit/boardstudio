# 001: Compile rust-ui components against our Dioxus 0.7.10 web build

Status: claimed
Type: task
Blocked by: None
Parent: [Map](../map.md)

## Question

Do copied rust-ui `button`, `popover`, `select` and `dialog` sources, with `tw_merge`,
compile and render in a disposable scratch crate pinned to our `dioxus = "=0.7.10"` web
target (upstream pins 0.7.9)? Record the upstream commit, extra crates and features they
pull in (`js-sys`, `web-sys`, `gloo-timers`, `icons`, `time`), wasm build result, and any
source edits needed. Drives AFK; nothing tracked changes.
