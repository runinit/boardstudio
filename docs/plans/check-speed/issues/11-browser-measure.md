# 11: Measure where the browser step spends its time

Status: ready-for-agent
Type: task
Blocked by: [33 (typed-core-edits)](../../typed-core-edits/issues/33-test-speed-rust.md)
Map: [map.md](../map.md) · Research: the 2026-10-10 browser deep dive (summarised below)

## Request

User, 2026-10-10: "the browser checks and tests have a lot of room for improvement; do a deep dive, an audit, then more."
A read-only deep dive found the warm `browser` step (about 159 s) is roughly half Chrome-launch overhead and half
test bodies, that overhead scales with test-wasm size (about 0.25 s per MB across 12 launches, R² 0.95, inference),
that the `name` custom section is about half of every test wasm even with debug info off, and that core is
rebuilt for wasm32 three times after a core edit (about 75 s, inference from timestamps) because third-party
feature sets diverge. It also found 18 wasm-pack launches (14 crate launches, a serial `--list` at `check.py:86`,
and the page runner's own `--list` plus two filtered launches) and that the host crate's `renderer_host`,
`boundary_parity` and `cad_worker` tests do not run in `browser`. Nothing was measured: this ticket measures.

## What to do (sequential, nothing else running, sandbox lifted for wasm-pack and Chrome)

1. Warm baseline with per-launch walls: `BOARDSTUDIO_CHECK_JOBS=1`, `4`, `6`, `8` of `python3 scripts/check.py browser`; record
   suite wall, command wall and overhead per crate.
2. Cargo timings: `cargo test --no-run --target wasm32-unknown-unknown -p <crate> --lib --locked --timings` for
   catalogue, layout and the page bin; then one union `-p` invocation for all crates, comparing compile units and
   how many times `boardstudio_core` compiles.
3. Decompose one launch (catalogue, layout, page): `wasm-bindgen-test-runner <wasm> --list`, a non-matching
   filter, the full `wasm-pack test`, and Chrome/chromedriver start alone. Does `--list` start a browser?
4. Strip A/B: build catalogue with `CARGO_PROFILE_DEV_STRIP=symbols` (throwaway on-disk target dir, not /tmp),
   compare wasm size and launch time; check a panic keeps its file and line.
5. Time the tail items alone: the serial `--list` launch and the `case_workspace` isolated launch.
6. Core-edit scenario: touch one core source file, then time the browser build set with `--timings`.
7. Try `WASM_BINDGEN_NO_DEBUG` and `CHROMEDRIVER_REMOTE` (check `--help`) and report the effect.

## Acceptance

- [ ] A measurements table in the Outcome, each row with the command and its output file, replacing every "inference" above with a measured value or marking it unmeasured.
- [ ] A ranked list of changes with measured gain, so [12](12-browser-quick-wins.md) and [13](13-browser-structural.md) are chosen from data.

## Verification

Per [verification-process.md](../../../agents/verification-process.md). No source changes; docs only (`repo`).
