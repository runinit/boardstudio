# 05: Overlap the page build with the provider builds

Status: resolved

## Resolution (2026-10-30)

`provider_lanes()` (A: 4 wasm-pack serial, B: CAD wasm) plus a `dx` lane for fast single-route
builds; ergogen staging serial first; shipped, multi-route and `--skip-providers` unchanged.
Lanes confirm all three starting at ~1-5 s. H1 holds (page wasm hash identical serial vs overlap).
Browser gate green. Three consecutive cold `build` runs: 198.4/197.6/196.4 s vs 282.1 s
rebaseline (-85 s, above the 40 s stop line). `test-build-web.py` 13 green.
Type: build
Blocked by: [02](02-measurement-kit.md)
Parent: [map](../map.md)
Related: `ci-wall-time` tickets 02 and 05 (branch `ci-wall-time-plan`)

## Question

The brief asks for CAD wasm alongside the other providers, "up to about 35 s". What is the real gain,
and is there more to take?

## Answer to plan against

CAD alone is the small part. The larger gain is `dx`, which runs last and alone.

Lanes in the `build` step (seconds from step start, parallelism in brackets):

| Lane | Span | Serial today | CPU-s |
|---|---|---|---:|
| A: four `wasm-pack` providers, root target, `release` | 1-135 | yes | 203 |
| B: CAD wasm, own target | 135-180 (4.7x) | yes | 259 |
| C: `dx` page, root target, `wasm-dev` | 183-279 (2.4x), then about 18 s of bindgen and copying | yes | 299 |

Running A, B and C together puts the critical path near lane A's 135 s plus contention.

## Evidence

- `scripts/build-web.py:195-196` runs every provider in series and `:164` runs providers before `dx`.
- Lane A has 1.4x parallelism because Core (47 s at O3) and `boardstudio_web` (22.5 s) are a chain,
  so it leaves room for the other lanes.
- Cargo's build lock is per profile directory. A uses `target/release` and
  `target/wasm32-unknown-unknown/release`; C uses `target/wasm-dev` and its wasm32 counterpart; B has
  its own dir. **Not verified by an overlapping run.** Only `--fast` has this property: the shipped
  build's `dx --release` shares A's profile and would just wait, so it keeps `dx` after the providers.
- Prior art: `research/local-web-builds@0f7c63087` has `provider_lanes()` and scheduling tests
  (`scripts/build-web.py` +44, `scripts/test-build-web.py` +45). Its measured gains on an older head
  are provider-phase only: `--fast` 150.3 s to 133.6 s, shipped 433.3 s to 355.7 s. That confirms
  the brief's 35 s was an upper bound for CAD alone (-16.7 s, with contention eating the rest).
- `ci-wall-time` ticket 05 found `dx` output byte-identical with and without staged providers
  ("H1"), on an older head. No `web/` source includes a provider path at compile time
  (`rg 'target/providers|assets/core-worker' web/src web/crates` finds only a runtime URL at
  `web/crates/runtime/src/runtime.rs:407` and a test doc comment). `web/crates/runtime/build.rs`
  reads vendored source.
- Ordering that must stay: `stage_runtime_assets()` and `imported-modules.json` come after providers
  (`:197-199`). `copy_runtime_assets` and `embed_offline_worker` need all lanes finished (`:188-189`).
- `run()` at `:36` shares the inherited lock descriptor. Reusing one open-file description across
  threads re-locks without blocking, so the lock does not serialize lanes. That lock is only present
  when `BOARDSTUDIO_BUILD_LOCK_FD` is set (`scripts/browser_tools.py:40-57`).

## Gain

| Cold wall | CPU | Warm |
|---:|---:|---|
| -100 to -135 s (`build` 298 s to 165-200 s) | 0, a little more under contention | about 0, warm lanes are short |

CI: `check.py build` is `--fast`, so this applies there, and the 1.4x to 2.4x lanes leave room even
on 4 vCPU. Probably the best CI build-step change available. Memory is the risk there.

Effort: 3 hours. Risk: medium. `check-speed` ticket 10 traced a `wasm-pack` `invalid type: sequence`
error to two overlapping builds, and `wasm-pack` plus `dx` overlap has not been tried.

## Work

1. Cherry-pick `0f7c63087`'s `provider_lanes()` and tests. Drop its lane C (Ergogen staging): run that
   first, in series; it takes a second and runs no Cargo. A later ticket can revisit it.
2. For `fast` with `prepare_providers`, add `dx` as a third lane in the same pool. Keep `dx` after the
   providers for shipped builds and `--skip-providers`.
3. Move `public` cleanup, staging, copying and `embed_offline_worker` after the pool is done. Capture
   each lane's output and print it whole, so a failure stays readable.
4. Keep `--providers-only` (used by `dev-web.py`) on the provider lanes only.

## Verification

```sh
python3 -B scripts/test-build-web.py
python3 scripts/build-time/cold.py build-lanes build
python3 scripts/build-time/report.py rebaseline build-lanes --lanes
# H1 on this head: page output must not depend on overlap
sha256sum target/dx/boardstudio-web/debug/web/public/wasm/*.wasm > .evidence/dx-overlap.sha
python3 scripts/build-web.py --fast                                # second run for comparison
python3 scripts/check.py browser > .evidence/build-lanes-browser.log 2>&1
for i in 1 2 3; do python3 scripts/build-time/cold.py build-lanes-$i build; done
```

Compare the `wasm` hashes against a serial build at the same head.

## Stop rule

Revert the `dx` lane, keeping CAD and provider lanes if they still pay, if `build` gains less than
40 s, if any of three consecutive cold runs fails, or if the page wasm hash differs from a serial
build. Revert the provider lanes too if they gain under 10 s.
