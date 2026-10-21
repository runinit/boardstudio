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
  [README.md](../../../README.md), not in `GLOSSARY.md`, which is the product glossary.
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

- [Destination, scope and the eight settled choices](issues/01-destination-and-scope.md):
  targets of 3 and 6 minutes; debug info off for test runs only; fast `build` plus an opt-in
  `release` step; a `quick` routine loop; up to 4 parallel launches; test dropping `touch`; four
  options out of scope; coverage parity required.

## Progress

<!-- one line per resolved build ticket -->

- Timing baseline and per-step output: [Timing and test-count baseline](issues/02-timing-and-count-baseline.md#outcome)
  (old check 889.7 s).
- [Debug info off for wasm test runs](issues/03-wasm-test-debuginfo-off.md#outcome): test wasm 198-560 MB
  down to 35-72 MB; launches 2-3x faster.
- [Fast build and opt-in release step](issues/05-fast-build-and-release-steps.md#outcome): warm `build`
  192 s to 52.5 s.
- [Parallel test launches](issues/06-parallel-test-launches.md#outcome): `browser` 598 s to 157.6-189.3 s.
- [`check.py quick`](issues/08-quick-mode.md#outcome): docs 0.1 s, layout edit 85.7 s, core edit 213.8 s.
- [Docs, final timing and closing the map](issues/09-docs-final-timing-and-close.md#outcome): full check
  14.8 min to about 5.2-5.7 min.

- [Measure `line-tables-only` against debug info off](issues/04-line-tables-only-measurement.md): keep debug
  info off; `line-tables-only` is about 48% slower per launch and twice the size.
- [Test whether dropping `touch core/src/lib.rs` is safe](issues/07-drop-the-touch.md): it is not needed;
  cargo rebuilds on a real edit and results match without it.

- [Nested worktrees run every check](issues/10-nested-worktrees.md#outcome): resolved. The CAD crates
  declare their own `[workspace]`; the full `check.py` passes from `.worktrees/tce-matrix` (480.1 s)
  with no second checkout. The `wasm-pack` `invalid type: sequence` error was two overlapping builds.

## Ready for implementation

- Nothing is open. The destination is reached; see the final timings in the closing ticket's Outcome.

## Not yet specified

- **Narrowing native tests for a core edit** in the routine loop (213.8 s today): the dependents of
  `core` are nearly the whole workspace, so a finer map would need module-level test selection.

## Out of scope

- **CI changes and merging all crates' tests into one wasm binary:** restructuring beyond tooling
  scripts. See [01](issues/01-destination-and-scope.md).
- **`sccache` and `mold`:** they do not help the wasm path, and native linking already uses
  `rust-lld`.
- **Reusing one `chromedriver` (`CHROMEDRIVER_REMOTE`):** a second or two per launch. Reopen only
  as a new effort if launch overhead still dominates after the in-scope tickets.

## Browser step follow-up (2026-10-10)

User asked for a deep dive on the browser checks. The read-only study is summarised in
[11](issues/11-browser-measure.md); next [12](issues/12-browser-quick-wins.md) and
[13](issues/13-browser-structural.md). Sleep replacement is
[typed-core-edits 34](../typed-core-edits/issues/34-test-speed-browser.md).

Ticket 11 is resolved with [source-bound green, failed-run, invalid and unmeasured
rows](issues/11-browser-measure.md#outcome). Ticket 12 can test the duplicate
serial page listing and overlap candidates against that green baseline; ticket
13 remains `needs-info` until 12 measures a gain. The opt-level A/B was invalid,
so no profile revert is justified by 11.
