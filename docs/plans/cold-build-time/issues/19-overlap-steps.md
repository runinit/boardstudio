# 19: Run lint, test and build together

Status: resolved

## Resolution (2026-10-31)

`_run_plan` now splits a contiguous `lint`/`test`/`build` block into child `check.py <step>`
processes on the shared `run_parallel` machinery (process groups, cancellation) when
`OVERLAP_MIN_CPUS = 8` CPUs are present, `check_jobs() > 1`, and no `--receipt` is active.
Serial `repo`/`tooling` run first, then the group, then `browser`; any group failure skips
`browser` and every failed step is reported. Each child's output prints whole in plan order
(`in_order`), with immediate start/end markers; step times are per-child walls and the total
is run wall, not a sum. `lint` gets `CARGO_TARGET_DIR=target/lint` only when overlapping
(the harness clippy keeps its `--manifest-path`; only its target dir moves, which Cargo
supports). `--update-native-baseline` passes to the `test` child only. The fifo jobserver
(`cpu_count - 1` tokens, `MAKEFLAGS=--jobserver-auth=fifo:<path>`) is kept: a scratch-target
proof build of `-p boardstudio-contracts` showed both tokens held mid-build with peak rustc
3 against 5 uncapped, and tokens back to full afterwards. An outer `MAKEFLAGS` jobserver is
reused instead of replaced. Hosted `repo tooling lint test` / `build browser` plans,
`--only`, quick plans, `BOARDSTUDIO_CHECK_JOBS=1` and under-8-CPU machines stay serial with
unchanged command order. `test-check.py` grew 14 tests (82 green). Still to be measured from
the parent shell: the full cold run, the Clippy/native-test failure mutations, and the
stop-rule numbers.
Type: build
Blocked by: none
Parent: [map](../map.md)

## Evidence

Full cold run at `2145e8630`: 868.6 s. `lint` 116.7 s, `build` 191.3 s and `test` 201.1 s run one after
another (509 s), and the run averages about 5 of 16 threads. Only `browser` needs `build` to finish first.

## Design

- Default plan order: `repo`, `tooling` in series (timing-sensitive interrupt tests), then `lint`, `test`
  and `build` at once, then `browser`.
- Overlap only when `os.cpu_count() >= 8`, `check_jobs() > 1`, no `--receipt`, and all of those steps are in
  the plan. Otherwise behaviour is unchanged, which keeps hosted 4 vCPU runners serial.
- Each overlapped step runs as a child `scripts/check.py <step>` with output captured and printed whole on
  completion, in plan order, so the native test guard and its messages keep working. Pass
  `--update-native-baseline` through. Let the other steps finish after a failure, then report all failures
  and skip `browser`.
- The children's own start, end and step-time lines stay intact. The parent's step times are per-child wall,
  and its total is real wall, not a sum.
- `lint` gets `CARGO_TARGET_DIR=target/lint`, because Cargo locks `target/debug` and `lint` and `test` would
  otherwise queue on it. `cargo fmt` is unaffected. Interrupt handling must reach every child's process group.
- A fifo jobserver shared by all children caps rustc across lanes at `os.cpu_count()`: `mkfifo`, write
  `cpu_count - 1` tokens, export `MAKEFLAGS=--jobserver-auth=fifo:<path>`, remove it afterwards.
  Prove with a focused run that cargo, `wasm-pack` and `dx` honour it, and that `.cargo/config.toml` `jobs = 12`
  stays as it is.

## Gain

| Cold wall (estimate) | CPU | Warm |
|---:|---:|---|
| -200 to -250 s, local only | up about 70 s (Clippy's proc macros) | similar |

## Verification

Unit tests for grouping, failure reporting, and the serial cases. One full cold run, then counts as in
[16](16-browser-launch-concurrency.md), and a mutation: a failing Clippy lint and a failing native test each
fail the run with their output visible. Measured from the Claude shell.

## Stop rule

Revert if it gains under 100 s, if any run exhausts memory, or if a test fails only under overlap. Drop the
separate lint target or the jobserver alone if removing it costs under 15 s.
