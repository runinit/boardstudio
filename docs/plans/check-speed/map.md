# Map: Faster checks

## Destination

Cut the repo's full check from about 16 minutes (`build` + `browser`) to a **routine loop of
3 minutes or less** and a **full pre-PR check of 6 minutes or less**, measured on this machine
with the 12-job cargo default, by implementing the measured build-speed options (or ruling each
out with a recorded reason), without running fewer tests.

## Notes

- **Execution is carried into the map.** The user asked to implement the changes, so most tickets
  are `build` tickets that change `scripts/`, `.cargo/config.toml` and docs, not only decisions.
  Each build ticket still resolves with an Outcome that records timings and test counts.
- **Vocabulary.** *Routine loop*: the everyday command (`check.py quick`). *Full pre-PR check*: the
  default `check.py`. *Test launch*: one `wasm-pack test` invocation, which starts its own
  `chromedriver` and Chrome. *Test wasm*: the `.wasm` a launch loads. These terms live in
  [README.md](../../../README.md), not in `CONTEXT.md`, which is the product glossary.
- **Work in the `.worktrees/check-speed` worktree** (branch `tooling/check-speed`). Follow
  [AGENTS.md](../../../AGENTS.md): stage explicit paths, never `git stash`, run the affected steps of
  `python3 scripts/check.py` before finishing, and keep `scripts/test-check.py` and the other script
  tests green when a step changes.
- **Environment.** Use `CARGO_TARGET_DIR=/home/chris/01_Projects/ts-boardstudio2/target`. Cargo uses
  12 jobs from `.cargo/config.toml`; never pass `CARGO_BUILD_JOBS=2`. `wasm-pack`, Chrome and the
  podman CAD build need the Claude sandbox lifted. No outbound network. One orchestrator owns the
  browser at a time.
- **Coverage parity is the acceptance bar.** The baseline executed counts are in
  [02](issues/02-timing-and-count-baseline.md); a faster check that runs fewer tests is not done.
- **Measured facts that started this effort** (2026-10-09, 16 cores): `build` + `browser` took
  979 s; about 86% of each debug test wasm is DWARF; with debug info off a launch fell from
  17.3 s to 6.2 s (catalogue), 25.1 s to 10.9 s (keycaps) and 49.0 s to 23.6 s (layout);
  three launches in parallel ran about 1.4-1.9x faster than serial; `wasm-opt` took about
  85 s (core worker) and 70 s (CAD worker).

## Decisions so far

- [01: Destination, scope and the eight settled choices](issues/01-destination-and-scope.md):
  targets of 3 and 6 minutes; debug info off for test runs only; fast `build` plus an opt-in
  `release` step; a `quick` routine loop; up to 4 parallel launches; test dropping `touch`; four
  options out of scope; coverage parity required.

## Progress

<!-- one line per resolved build ticket -->

## Ready for implementation

- [02: Timing and test-count baseline](issues/02-timing-and-count-baseline.md) is the frontier.

## Not yet specified

- **Overlapping the release `wasm-pack` builds.** Only worth doing if the opt-in `release` step
  stays slow after [05](issues/05-fast-build-and-release-steps.md). Revisit with 05's timings.
- **Native test selection for workspace-wide edits** (`Cargo.lock`, `.cargo/config.toml`, shared
  `core` types) in the routine loop. [08](issues/08-quick-mode.md) decides the mapping; the edge
  cases it cannot place come back here.

## Out of scope

- **CI changes and merging all crates' tests into one wasm binary:** restructuring beyond tooling
  scripts. See [01](issues/01-destination-and-scope.md).
- **`sccache` and `mold`:** they do not help the wasm path, and native linking already uses
  `rust-lld`.
- **Reusing one `chromedriver` (`CHROMEDRIVER_REMOTE`):** a second or two per launch. Reopen only
  as a new effort if launch overhead still dominates after the in-scope tickets.
