# 11: Measure where the browser step spends its time

Status: claimed
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

8. Dev profile A/B (from typed-core-edits 33): `boardstudio_core` at `opt-level = 1` gave native tests about 7-9 s but the one browser run
   after the change took 312.9 s (cold: every wasm-pack job recompiled core). Run `typecheck browser` twice at each level and compare the
   warm second runs; if browser is not slower at level 1, `git revert 020d66886` restores the `test` gain (commit `448aa3f53`).

## Acceptance

- [ ] A measurements table in the Outcome, each row with the command and its output file, replacing every "inference" above with a measured value or marking it unmeasured.
- [ ] A ranked list of changes with measured gain, so [12](12-browser-quick-wins.md) and [13](13-browser-structural.md) are chosen from data.

## Verification

Per [verification-process.md](../../../agents/verification-process.md). No source changes; docs only (`repo`).

## Progress — 2026-10-20

The first measurement pass ran from local `dev` commit `e80da69dc` in
`t3/measure-browser-step-time`; logs and the provisional table are in that
worktree's gitignored `.evidence/REPORT.md`. Warm browser sweeps, launch
decomposition, Cargo timings, strip A/B, tail timing and a Core-edit run produced
partial observations. **No browser-step result is accepted as a passing baseline:**
the measured runs exited 1 because
`model_decode_boundary_tests::refuses_hostile_wrl_at_the_application_decoder_without_poisoning_the_shared_module`
could not load `assets/renderer/boardstudio_renderer_wasm.js` from the fresh
worktree. The helper also omitted drag smoke and two CAD commands because their
packaged assets had not been built. These failures and omissions prevent a full
browser-step comparison.

The opt-level A/B repeated failed runs instead of stopping. Level 0's three
attempts took 509.56, 527.84 and 527.34 seconds; level 1's took 362.18, 101.09
and 100.82 seconds. All exited 1. These are **invalid speed comparisons**, not
evidence to choose an optimization. The lane was stopped after preserving logs.
Its temporary `Cargo.toml` change is restored and its tracked worktree is clean.
Before another sweep, build the packaged renderer and other browser prerequisites
once, prove the focused model-decoder test passes, and stop on the first failed
launch. Then run only bounded, comparable warm cases. No source change or ticket
resolution is claimed.

## Recovery — 2026-10-20

The missing renderer was an environment setup failure, not a decoder regression:
`renderer/pkg` and `web/assets/renderer` were absent in the fresh worktree. The
focused `model_decode_boundary_tests::` browser run reproduced the missing-module
error, then passed (1/1) after building and staging only the renderer. A subsequent
warm browser subset passed in 118.9 seconds with 77 page tests and no failures.
One `python3 scripts/check.py build` prepared the remaining providers/site in
286.0 seconds; then one full `python3 scripts/check.py browser` passed (`build`
52.3 seconds, `browser` 181.4 seconds, 77 page tests with zero failures).
Evidence: `.evidence/checkspeed11-repro-red.log`,
`.evidence/checkspeed11-renderer-green.log`,
`.evidence/checkspeed11-warm-after-renderer.log`,
`.evidence/checkspeed11-package-build.log` and
`.evidence/checkspeed11-full-browser-after-build.log` in the measurement worktree.
The temporary helper now stops on a failed launch. The earlier failed A/B timings
remain invalid; no further opt-level sweep was run during this recovery.
