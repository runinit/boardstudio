# 04: Measure `line-tables-only` against debug info off

Status: ready-for-agent
Type: task
Blocked by: 03
Map: [map.md](../map.md)

## Question

Does `-C debuginfo=line-tables-only` (what the repo's `wasm-dev` profile uses) keep enough
source-level information to be worth its size and launch cost compared with debug info off?

## Scope

AFK measurement. For the catalogue, keycaps and layout crates, build the test wasm in a throwaway
`CARGO_TARGET_DIR` under `$TMPDIR` with each setting and record: file size, `wasm-bindgen` plus
Chrome launch wall time, and whether a panic and a `console.error` stack still name the Rust
function and line. Do not change repo config.

## Answer

Record the numbers and a recommendation: keep ticket 03's setting, or switch to
`line-tables-only`. If switching, say which scripts change; a follow-up build ticket makes it.

Optional follow-up, not needed for the destination: [03](03-wasm-test-debuginfo-off.md) already meets the browser target with debug info off.
