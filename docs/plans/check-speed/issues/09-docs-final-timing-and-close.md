# 09: Docs, final timing and closing the map

Status: resolved
Type: build
Blocked by: 05, 06, 07, 08
Map: [map.md](../map.md)

## Goal

Everything is documented in one place, the destination is measured, and the map is closed.

## Scope

- README and `AGENTS.md` describe the routine loop (`quick`), the full pre-PR check, the opt-in
  `release` step, the test environment for debug info, the parallel cap, and the outcome of the
  `touch` experiment. Remove or correct the stale `CARGO_BUILD_JOBS=2` and `touch` guidance in live
  docs only.
- Run the full check and `quick` on a clean tip; record times against the targets (3 and 6
  minutes) and the [02](02-timing-and-count-baseline.md) baseline, with executed counts.
- If a target is missed, name the dominant remaining cost and either add a ticket for it or record
  why it is accepted.
- Close the map: Progress lines, final Decisions so far, and the *Not yet specified* patches
  graduated, dropped or kept with a reason.

## Verification

`python3 scripts/check.py` (the new full check) passes; documentation links check passes; the
counts match the baseline.

## Outcome

Resolved 2026-10-09 on branch `tooling/check-speed`. README and `AGENTS.md` describe `quick`, the full
check, the opt-in `release` step, the test-wasm debug-info setting and the parallel cap; the `touch` and
`CARGO_BUILD_JOBS=2` guidance appears only in superseded plan records, which stay as history.

Final timings (same machine, 12 jobs, executed counts equal the baseline in every run):

| Step | Before | After (warm) |
| --- | --- | --- |
| `repo` + `tooling` + `lint` | 7.7 s | 6.9 s |
| `build` | 192.0 s | 52.5 s |
| `test` | 91.9 s | 95.9 s |
| `browser` | 598.0 s | 157.6-189.3 s |
| **Full check** | **889.7 s (14.8 min)** | **about 313-344 s (5.2-5.7 min)** |

The full pre-PR target (6 minutes) was met by the runs before the final profile change, with about
20 s of margin at the slow end of the browser variance. The routine loop (3 minutes) is met for a docs edit (0.1 s) and a layout edit (85.7 s) and
missed for a core edit (213.8 s), because nearly every crate depends on core: native tests take 106.8 s
and the dependent browser launches 103.4 s. `test` is unchanged; it is now the second-largest step and
the next place to look (a core edit could narrow native tests to the changed module).

Disposition of the original options: debug info off, fast `build` with opt-in `release`, affected-only
tests (`quick`), parallel launches, dropping the `touch` and the debug `dx` build are implemented.
Overlapping the release `wasm-pack` builds is dropped: with `--no-opt` in the checked build there is no
`wasm-opt` left to overlap, and `release` is not on the routine path. Reusing one `chromedriver` stays
out of scope: a launch is now 6-24 s and parallel. The optional `line-tables-only` measurement
([04](04-line-tables-only-measurement.md)) stays open. The `release` step was not run, only unit-tested.

Final run at `82b7f2e44` (all counts equal the baseline: crate suites, 60 page tests, core 271 + 463,
CAD 51 / 4 ignored): `repo` 0.1 s, `tooling` 53.1 s, `lint` 50.7 s, `build` 410.2 s, `browser` 351.5 s,
`test` 124.7 s. These are **cold** numbers, not steady state: that commit added
`[profile.dev.package."*"] opt-level = 2`, which changes the dependency build profile, so every
dependency recompiled once on native and wasm. A warm second run was skipped at the user's request,
so the steady-state effect of that last change on the full check is **not measured**. Measured in
isolation: the 61 s demo-project acceptance test (the whole core integration run) takes 25 s with
optimized dependencies; wasm test files shrank further (catalogue 31 MB, keycaps 41 MB). The earlier
warm figures in the table (build 52.5 s, browser 157.6-189.3 s) predate that change. Cost to expect on
the first build after pulling this: one full dependency rebuild, roughly 6-7 minutes of extra build time.
If a later warm run shows no net gain from the optimized dependencies, remove the Cargo.toml profile
entry; nothing else depends on it.

