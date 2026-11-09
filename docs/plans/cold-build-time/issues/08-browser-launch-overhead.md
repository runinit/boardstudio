# 08: Reduce browser launch overhead

Status: wontfix

## Resolution (2026-10-30): neither hypothesis clears 20 s

Measured from the jobs4 cold log instead of a new dry run. First-launch window: 170 units,
237 CPU-s over 134 s wall, of which the DUI stack (host/runtime/web_sys/js_sys) is only ~26
CPU-s — the wall is first-party compile chains (layout/case/parts/core), not the dry stack.
Per-launch durations: case 140 / pcb 137 / parts 134 (cold compile of real suites) then
host 44 / runtime 41 / keycaps-library-keymap-layout 31-36 / catalogue 25 / ui-model 21 /
ui-shared 16. Launches run 4-at-a-time, so stripping any non-critical-path launch (e.g. the
16 s ui-shared from the 44 s batch) saves ~0 s wall; only batch maximums matter. H02's 6 s
bindgen likewise cannot move a 44 s batch by 20 s. Reopen if launches go serial or the dry
stack exceeds half of a batch's critical path.
Type: task
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: [check-speed 11](../../check-speed/issues/11-browser-measure.md), [check-speed 12](../../check-speed/issues/12-browser-quick-wins.md), [check-speed 13](../../check-speed/issues/13-browser-structural.md)

## Question

No compile fix touches the roughly 257 s of browser execution. Why do drag smoke and `run-wasm-tests`
take so long, and what can go without weakening a test or a count guard?

## Evidence

**`run-wasm-tests`: 96.6 s, of which test bodies are 23.2 s.** The result logs
(`web/target/test-results/browser.json.logs/`) report 0.14 s for one test, 22.34 s for 109 tests, and
0.70 s for two. The other 73 s is three launches at about 24 s each: the runner's own listing, the
bindgen pass and Chrome compiling a test wasm of 153 to 158 MB
(`target/wasm32-unknown-unknown/debug/deps/boardstudio_web-*.wasm`).

The three launches exist on purpose. `--depth 1` makes `model_decode_boundary_tests::` its own group, and
`check.py:88` isolates `presentation::case_workspace::`, because a crash or leaked state in a shared page
would mask the rest (`run-wasm-tests.py:461-462`). They must stay separate.

**Drag smoke: 88.0 s, of which the run is 47.3 s.** The rest is harness compile, handled by
[06](06-harness-and-tooling-overlap.md). The run has real work: 21.3 s in the collision scenario (8.8 s
waiting on the observed result), five document loads, each pulling a 54.6 MB unoptimized page wasm, and
11 core worker fetches. The harness prints no timestamps, so nothing finer can be said. **Which of
these dominates is not known.**

**Existing leads, none measured validly.** `check-speed` 11 inferred about 0.25 s per MB across launches
and that the wasm `name` section is about half of each test wasm. Its measured runs all exited 1, so
none is accepted. `check-speed` 12 lists stripping `name` as a candidate and is still open.

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -40 to -70 s if stripping holds (page -30 to -50 s; crate launches -10 to -20 s) | 0 | same, on every run |

The inference above is the only basis for this gain. This is the only ticket here that also speeds up
warm runs, since these costs repeat on every browser run. It does not help drag smoke's run, for which
the likely answer is that its time is real work.

Effort: half a day to measure; half a day to apply. Risk: low to medium. Wasm stack traces lose function
names, and `format_wasm_failure_summary` may rely on them.

## Work

1. **Measure.** Timestamp each phase of one page launch with `scripts/run-wasm-tests.py`: listing, bindgen,
   Chrome start, wasm compile, test body. Add elapsed-time prefixes to `drag-smoke:` harness output.
   Record both in this ticket. Do not change behaviour yet.
2. **A/B strip.** Set `CARGO_PROFILE_DEV_STRIP=symbols` in the browser step's environment, next to the
   existing forced `CARGO_PROFILE_DEV_DEBUG=false`, and compare the catalogue, layout and page launches.
   Do not put it in `[profile.dev]`: `cargo test` backtraces need the names.
3. Keep it only if a seeded panic still reports file and line, and a seeded failure still produces a
   useful `summarize_wasm_failure` result.
4. Report the drag phases. Do not remove reloads, checks, exact-Undo comparisons, collision scenarios or
   timeouts to make it faster. If no phase is overhead, resolve that part as "real work".

## Verification

```sh
python3 -B scripts/test-run-wasm-tests.py
python3 scripts/check.py browser > .evidence/launch-before.log 2>&1
CARGO_PROFILE_DEV_STRIP=symbols python3 scripts/check.py browser > .evidence/launch-after.log 2>&1
ls -l target/wasm32-unknown-unknown/debug/deps/boardstudio_web-*.wasm
python3 -c "import json; d=json.load(open('web/target/test-results/browser.json')); print(d['passed_count'], d['complete'], d['failed_count'])"
python3 scripts/check.py browser                   # three consecutive passes
```

Mutation: a deliberately failing test in the page batch and one crate launch must still fail the step,
with file and line shown. Expected tests stay 112 page, 693 crate, 15 drag.

## Stop rule

Revert stripping if the page launch gains under 15 s across its three launches, if a panic loses its
location, or if a failure summary gets worse. Stop the drag work after measuring if overhead is below
10 s. Structural change (`check-speed` 13's one-build pool) needs its own decision, not this ticket.
