# 11: Measure where the browser step spends its time

Status: resolved
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

- [x] A measurements table in the Outcome, each row with the command and its output file, replacing every "inference" above with a measured value or marking it unmeasured.
- [x] A ranked list of changes with measured costs and explicit gain limits, so [12](12-browser-quick-wins.md) and [13](13-browser-structural.md) are chosen from data rather than invented savings.

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

## Outcome

Source: local `dev` commit `e80da69dc` in worktree
`/home/chris/.t3/worktrees/ts-boardstudio2/t3-b7f6909e`, isolated target
`/home/chris/.cache/boardstudio-target/check-speed-11-dev`. Logs below are in
that worktree's gitignored `.evidence/REPORT.md`. That full table is local
evidence, not a committed acceptance artifact.

| Item | Command / scope | Result and class | Evidence |
|---|---|---|---|
| Full baseline | `python3 scripts/check.py browser` (default jobs=4) | **MEASURED-GREEN:** build 52.3s, browser 181.4s; 77 page tests, 0 failed | `checkspeed11-full-browser-after-build.log` |
| Jobs sweep | `BOARDSTUDIO_CHECK_JOBS=1/4/6/8 python3 scripts/check.py browser` | **MEASURED-GREEN:** browser 301.0/193.4/175.8s for 1/4/6; all 77/0. **FAILED-RUN:** jobs=8 timed out in PCB and Keycaps before test observation; no comparable wall. | `checkspeed11-green-jobs{1,4,6,8}.log` |
| Same-setting spread | default=4 versus explicit jobs=4 | **MEASURED-GREEN:** 181.4 versus 193.4s, a 12.0s observed spread (n=2, not a statistical noise bound). Jobs=6 is 5.6s faster than the better jobs=4 run, within that spread. | baseline and jobs4 logs |
| Original partial sweep | `BOARDSTUDIO_CHECK_JOBS=1/4/6/8 python3 .evidence/measure_browser.py` | **FAILED-RUN:** 255.6/133.3/121.4/121.1s; omitted drag/CAD commands and failed the model decoder due missing renderer. Not a speed comparison. | `warm-jobs{1,4,6,8}.log` |
| Cargo rebuilds | `cargo test --no-run --target wasm32-unknown-unknown -p <crate> --locked --timings` (lib for catalogue/layout, bin for page); union; touched Core | **MEASURED:** warm catalogue/layout/page 0.25/0.25/0.19s, union 14.39s; touched-Core union 15.59s, two Core compile units. Full green Core-edit browser delta **UNMEASURED**. | `cargo-{catalogue,layout,page-bin,union}.log`, `core-edit-union.log` |
| Launch anatomy | `wasm-bindgen-test-runner` list/nonmatch/one-test; catalogue `wasm-pack test`; chromedriver alone | **MEASURED:** 1.97/1.94/6.99/8.14s; Chrome session setup/teardown about 0.22s. Fetch, compile and instantiate separately **UNMEASURED**. | `decompose/{list,nomatch,onetest,full,chromedriver.txt}`, `10-decomposition-batch.log` |
| Strip A/B | `CARGO_PROFILE_DEV_STRIP=symbols` catalogue and panic probe | **MEASURED SIZE:** 46.1→22.9 MB; `name` section about half. **INVALID GAIN:** 6.94→6.26s runs different tests, and raw-trap function names disappear although file:line remains in a throwaway probe. Whole-step effect **UNMEASURED**. | `strip-ab.txt`, `strip-ab-repeat.txt`, `probe-{plain,stripped}-run.log` |
| Serial tail | page `--list`, isolated `case_workspace::`, `presentation::` | **MEASURED:** page list 5.2–6.2s in four green full runs; isolated case workspace 18.34s and presentation 21.62s standalone. Merged-filter saving **UNMEASURED**. | green sweep logs, `tail-{page-list,case-workspace,presentation}.log` |
| Core-edit browser | touch Core then partial browser subset | **FAILED-RUN:** 171.4s versus failed 121.4s baseline; +50.0s is not a green regression estimate. | `core-edit-browser.log` |
| Runner flags | `WASM_BINDGEN_NO_DEBUG=1`, `CHROMEDRIVER_REMOTE` | **MEASURED STANDALONE:** 6.32s unchanged and 6.30s versus 6.32s; a bad remote URL exits 1. | `flag-{baseline,no-debug,remote-missing}.log`, `tail-and-flags.txt` |
| Dev opt-level A/B | `core` opt-level 0/1, typecheck and partial browser repeats | **INVALID:** all six browser attempts exited 1; two level-0 runs took ~527s without rebuild evidence. Comparative warm effect **UNMEASURED**; no revert. | `20-optlevel.log`, `optlevel-*.log` |

Per-crate `wasm-pack test --headless --chrome web/crates/<crate>` launches in the
two green runs. Filters distinguish the paired Layout and UI-shared launches.
"Overhead" is launch wall minus reported test-body time, including WASM setup;
parallel launch walls must **not** be added to estimate step wall. Source:
`checkspeed11-green-jobs{4,6}.log`.

| Crate / filter | Jobs=4 launch | Test bodies | Jobs=4 overhead | Jobs=6 launch |
|---|---:|---:|---:|---:|
| case | 27.4s | 9.0s | 18.4s | 30.8s |
| parts | 21.9s | 3.5s | 18.4s | 25.1s |
| pcb | 24.0s | 6.7s | 17.3s | 27.1s |
| layout / main | 29.9s | 9.4s | 20.4s | 33.9s |
| layout / setup guide | 18.5s | 0.4s | 18.1s | 24.3s |
| host | 19.5s | 7.4s | 12.1s | 24.3s |
| runtime | 17.1s | 3.6s | 13.5s | 19.1s |
| ui-model | 11.7s | 0.0s | 11.7s | 13.3s |
| ui-shared / main | 16.1s | 2.5s | 13.6s | 18.2s |
| ui-shared / panels | 13.3s | 0.3s | 13.0s | 14.5s |
| keycaps | 13.3s | 0.7s | 12.6s | 15.3s |
| library | 14.5s | 1.6s | 12.9s | 14.7s |
| keymap | 13.5s | 1.9s | 11.6s | 14.4s |
| catalogue | 9.3s | 0.3s | 9.0s | 9.1s |

The green full step includes a 41–43s drag smoke and about 7–8s of CAD scripts
that the original partial helper excluded. At jobs=1/4/6, the derived
setup / crate batch / page tail split is 51.1/180.0/70.0s,
53.3/68.4/71.7s and 52.5/51.9/71.4s respectively. This is one run per setting.
The size/launch fit in `10-decomposition-batch.log` is 0.137 **s**/MB + 0.70s
(R² 0.989, n=20 including duplicate crate hashes and excluding the page bin),
not the earlier 0.25s/MB inference and not a causal strip benefit. The browser
plan has 19 wasm-pack invocations; host renderer/boundary/CAD worker tests already
run (45 host tests), contrary to the original premise.

### Ranked decisions for 12 and 13

1. **Test pooling drag smoke and the page runner with the crate batch in 12.**
   Their measured serial costs are 41–43s and 62–66s. A 50–70s whole-step
   saving is only an estimate until measured under contention. Before pooling,
   prepare each command's receipt so a stale `browser.json` cannot masquerade as
   a passing crashed runner; preserve the isolated page filters and test counts.
2. **Remove the duplicate serial page `--list` in 12.** It costs 5.2–6.2s
   in green runs and the page runner lists again. The after-change gain is
   unmeasured; update command-order tests and compare executed-name receipts.
3. **Keep the settled four-job default.** Jobs=6 took 175.8s once versus
   jobs=4 at 181.4/193.4s, but the difference from the better four-job run
   is below the observed 12s spread. Jobs=8 timed out. Do not reopen ticket
   01's up-to-four limit from these data.
4. **Do not promote the other 12 candidates:** keymap is not second-slowest;
   host tests already run; stripping's time A/B is confounded and loses raw-trap
   names; no-install and the CI debug profile lack a comparable whole-step
   result. The 18.34s case-workspace launch includes test work a merge would
   still execute and risks DOM leakage. Investigate the 41–43s drag smoke
   before claiming a saving. Ticket 13 remains `needs-info` pending measured
   gains from 12; no structural rewrite is justified.

No production source or test coverage was changed. The temporary opt-level edit
was restored; the measurement worktree is clean. The repository `repo` check
in the dirty `dev` checkout remains blocked by unrelated deleted-`CONTEXT.md`
links, not by this ticket's Markdown update.
