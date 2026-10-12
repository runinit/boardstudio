# 04: Measure `line-tables-only` against debug info off

Status: resolved
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

## Answer

Measured 2026-10-09 with the final config (dependencies at `opt-level 2`), warm, in throwaway target
directories on disk. `-C debuginfo=line-tables-only` against debug info off:

| Crate | Setting | Launch (warm) | Test wasm |
| --- | --- | --- | --- |
| catalogue | off | 5.8 s | 31 MB |
| catalogue | `line-tables-only` | 8.6 s (+49%) | 71 MB |
| keycaps | off | 9.6 s | 41 MB |
| keycaps | `line-tables-only` | 14.2 s (+47%) | 95 MB |

Cold builds were 51 s and 68 s (off) against 64 s and 84 s. Keep ticket
[03](03-wasm-test-debuginfo-off.md)'s setting: `line-tables-only` costs about half again per launch
for stack-trace line numbers that failing tests already get from panic locations. No follow-up build
ticket. Anyone debugging a specific test in DevTools can run `wasm-pack test` by hand without the
variable.

