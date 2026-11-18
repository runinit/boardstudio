---
title: "Measure one producer overlap in packaged validation"
type: chore
ticket: 2
created: '2026-11-17'
status: done
baseline_revision: 'a6cfa51839bf5d4c11b558e4221935db5d08013b'
route: full
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

Measure whether overlapping the offline service-worker build with other packaged work reduces the time until the last required dev check finishes. Use the existing sequence as the control. A local implementation recommendation requires equivalent coverage and a repeatable wall-time saving.

## Boundaries & Constraints

Both arms use clean source from `a6cfa51839bf5d4c11b558e4221935db5d08013b`. Keep the experiment local, use a private build target, and preserve the ticket workspace, cache-story status edit and installed BMad tooling. Preserve the pinned dev commands, browser isolation, staged renderer and release behavior. Both schedules inherit the same core budget, flags, installed tools and initial cache contents. Include the cost of building dependencies again if the overlap requires a separate target.

| Observation | Decision |
|---|---|
| No producer has a safe interval before its first read | Record the dependency and close the spike. |
| A required check fails or test names, outcomes, isolation or receipts differ | Identify the mismatch and retain sequential execution. |
| First pair shows negligible savings or contention rules out adoption | Report the pair and stop. |
| First pair shows a credible saving with equivalent checks | Reverse the order for one more matched pair. |
| Both pairs favor overlap and the smaller saving exceeds the larger within-schedule repeat difference | Recommend a separate implementation ticket. Otherwise retain sequential execution. |

</frozen-after-approval>

## Code Map

- `scripts/build-web.py` defines provider commands, stages runtime assets, invokes the Dioxus page build and packages each route. Core, CAD worker, CAD Wasm and renderer outputs pass through staging before the page build. The offline worker writes JS and Wasm under `web/target/providers/offline-worker`. Its packaging consumer is `embed_offline_worker`, after `copy_runtime_assets`. Keep Ergogen model staging and the imported module catalogue ahead of bundling.
- `.evidence/build-audit/commands.log` lists ten packaged commands. Resolve their environment, prerequisites and receipt paths from the pinned dev entry points. Keep the drag runner, CAD controls, cold-response checks, Wasm inventory check, CAD provider, model-lookup gate, Ergogen source parity, face completeness, CAD browser and page runner.
- `scripts/run-wasm-tests.py` owns the page inventory and the `presentation::case_workspace::` isolation rule. Preserve its arguments and capture actual individual test results.

## Tasks & Acceptance

- [x] In the detached experiment checkout, trace each generated asset from its producer through its first read by staging, compilation, bundling and browser checks. Verify the offline-worker output has no earlier reader and identify each command’s actual Cargo target.
- [x] Use a temporary local measurement driver outside tracked source. Use `python3 scripts/check.py build browser-packaged --receipt <arm-receipt>` in both arms with the same pinned patched Wasm tool pair and `CARGO_PROFILE_DEV_DEBUG=false`, as in the dev packaged job. Keep the sequential arm's existing order and target reuse. In the overlap arm launch the offline-worker command at its original position, use its own Cargo target, continue to the renderer and remaining stages, and join before `embed_offline_worker`. Provision that extra target empty in both arms. Charge its compilation to the arm that uses it. Preserve lock inheritance and propagate failures. Keep all other stages and browser lanes in their original order.
- [x] Start each arm at the same absolute paths from a reproducible cache state. Record tool versions, cache preparation, source revision and diff, CPU affinity, Cargo jobs, target mapping and ambient load. Keep profiles and generated assets isolated between runs. Sample per-core busy time from `/proc/stat` throughout and capture timestamped Cargo lock messages, including the observed duration where available.
- [x] Under `.evidence/packaged-build-overlap/`, save per-arm logs from monotonic start through final gate completion, stage intervals, named outcomes, gate receipts and actual page inventory. Compare test and gate identities and outcomes, allowing per-run paths, timestamps and timings to differ. Compare staged and packaged renderer files with the matching producer output.
- [x] Record times, evidence paths, coverage comparison and decision in this plan. Run sequential then overlap. Only if warranted run overlap then sequential from the same starting conditions.

**Acceptance criteria**

- Given the pinned dev source, when dependency tracing finishes, then the plan identifies the selected producer, its first consumer, and the point at which waiting for it is mandatory.
- Given two matched starts, when each schedule reaches its last packaged check, then the logs establish elapsed wall time, CPU activity, lock observations, asset staging, browser isolation, gate receipts and individual page names and outcomes.
- Given the completed pair or pairs, when applying the ticket's local decision rule, then the plan reports every measured time and calculation, or identifies the exact dependency or failed check that prevents a valid comparison.

## Implementation Notes

### Closure and cold-build breakdown

Closed by maintainer decision. Keep the sequential packaged schedule. The completed sequential arm took 779.802 s through the final check, including 375.231 s for the fast site build and approximately 404.6 s for validation and runner overhead. The measurement starts with empty Cargo targets and populated dependency sources. Patched tool installation happened before the timer.

| Fast site build, in execution order | Wall seconds | Evidence inside the stage |
|---|---:|---|
| Core worker | 107.263 | First release Wasm build on the cold workspace target. Cargo build took 96.54 s and its full metadata query took 8.32 s. |
| CAD worker | 6.735 | Next worker on the same target. The compile log lists the web host and web crate. |
| Offline worker | 12.901 | Service-worker feature build. Cargo took 12.51 s. |
| Renderer | 19.842 | Cargo release build took 19.33 s. |
| CAD Wasm provider | 41.128 | Separate CAD target. Cargo release build took 39.44 s. |
| Dioxus page | 185.464 | The page-feature Cargo invocation under the wasm-dev profile took 157.85 s. Dioxus reported about 26.3 s from starting bundling to client completion, including wasm-bindgen. |
| Remaining staging, embedding and orchestration | 1.898 | Residual of the fast build after the six stages above. |

Dioxus and the core worker account for 78% of the site build. The fast path skips wasm-opt but the providers still compile optimized Rust. The worker and page use different profiles and features. Both compilation logs include BoardStudio core, contracts, footprints, web host and web. Those shared names show where to investigate profile and feature costs; they do not give per-crate critical-path times.

The two largest validation commands were the page suite (217.457 s) and drag smoke (110.681 s). The page suite spent 155.90 s on its own Cargo test build; the three filtered browser runs took 12.217 s, 31.637 s and 14.347 s including runner overhead. Drag smoke first built the native browser harness in its separate target (48.03 s). The remaining eight gates together took 75.44 s, led by face completeness (27.746 s) and CAD capture controls (18.795 s). The core-worker Cargo build, page Cargo build and page test Cargo build alone occupied 410.29 s of sequential wall time across separate invocations. These measurements describe this cold local run. Warm-cache savings and per-crate bottlenecks were not measured.

[Stage and gate events](../../.evidence/packaged-overlap-fixed-drag/01-sequential/stages.jsonl) · [Cargo invocation trace](../../.evidence/packaged-overlap-fixed-drag/01-sequential/cargo.jsonl) · [Dioxus and compiler output](../../.evidence/packaged-overlap-fixed-drag/01-sequential/run.log) · [Page filter timings](../../.evidence/packaged-overlap-fixed-drag/01-sequential/page-result.json)

### Completed benchmark after the drag harness repair

**Retain sequential execution.** The first complete sequential/overlap pair passed equivalent dev coverage. Overlap took 77.186 s longer (9.90%) through the last required packaged check. The ticket reserves the reverse-order pair for a promising first result, so measurement ended after these two arms.

| Local schedule | Fast build | Last page command | Start through last required check | Process exit |
|---|---:|---:|---:|---:|
| Sequential | 375.231 s | 217.457 s | 779.802 s | 779.876 s, exit 0 |
| Overlap | 410.381 s | 260.126 s | 856.989 s | 857.069 s, exit 0 |

Both arms checked out dev `a6cfa51839bf5d4c11b558e4221935db5d08013b` and applied the same single-file drag harness from `e390f885514cd6c427b6d9d14c31d9e909119061`. Each started with empty Cargo targets at identical paths, the same patched tool pair, and a fresh private copy of one offline Cargo registry and Git dependency snapshot. Both ran on the same 16 logical CPUs with 12 Cargo jobs per invocation. OS page cache and other host load varied. The observed difference includes 35.150 s in the build and 42.669 s in the subsequent page-test command. One pair establishes the local stopping decision, not the cause of each stage difference.

The worker and renderer ran concurrently for about 36.6 s on separate targets. The worker had finished before embedding began. Cargo reported zero package-cache lock notices sequentially and eight with overlap, with individual upper bounds of 30–304 ms to the next output or process exit. Mean host CPU busy time across the two runs was 39.5% and 40.1%, respectively.

The command inventories, named outcomes and isolation filters matched. All ten packaged commands passed, including model lookup, all 15 drag checks, and 126 named page tests (one renderer decode, 121 presentation, four isolated case-workspace). Each arm verified all six renderer files at both destinations and matched the packaged offline worker to its own producer. The earlier drag-failing runs below are historical evidence and are excluded from this comparison.

[Comparison and stage traces](../../.evidence/packaged-overlap-fixed-drag/comparison.log) · [Sequential gate receipt](../../.evidence/packaged-overlap-fixed-drag/01-sequential/check-receipt.json.md) · [Overlap gate receipt](../../.evidence/packaged-overlap-fixed-drag/02-overlap/check-receipt.json.md)

Use native tools as authorized. The clean detached experiment checkout is `/home/chris/.cache/tmp/packaged-overlap.8F80HoAl/dev`. Evidence and this plan belong in the ticket workspace.

### Producer and consumer trace

| Output | First consumer and later use | Cargo target from the trace or pinned command |
|---|---|---|
| Core worker and CAD worker JS and Wasm | `stage_runtime_assets` copies the provider directories into `web/assets` and writes the worker entry modules before `dx build`. `copy_runtime_assets` supplies the same runtime paths in the packaged site. Page runtime starts the workers from those paths. The CAD control, cold-response and CAD browser gates exercise the packaged worker paths. | `dev/target` |
| CAD Wasm | `stage_runtime_assets` copies `cad/wasm/pkg` into `web/assets/cad`. The packaged CAD worker imports the copied JS at startup. CAD provider regressions read the raw package. Model lookup serves its own JavaScript and recipe, and the Ergogen source gate serves its case definitions. Both remain in the required inventory. | `dev/cad/wasm/target`, set by `cad_build.build_wasm` |
| Renderer JS and Wasm | `stage_runtime_assets` first copies `renderer/pkg` into `web/assets/renderer` before Dioxus. `copy_runtime_assets` copies from the same provider into the packaged site. The renderer host loads that module by URL. The page runner's `model_decode_boundary_tests::` runs the application decoder against the staged renderer. | `dev/target` |
| Ergogen static model files and imported module catalogue | `stage-ergogen-models.py` stages vendored models under `web/assets/ergogen-models` before Dioxus and writes its provider manifest. The runtime build script generates the compiled model inventory from the vendored originals. `prepare_runtime_providers` copies the imported catalogue into `web/assets` before Dioxus. `copy_runtime_assets` packages the staged models and catalogue for runtime fetches. | Python staging has no Cargo target. The runtime build script uses the page build's Cargo target. |
| Dioxus page, generated Tailwind CSS and public bundle | `dx build` reads the staged page inputs. `build` copies `dev/target/dx/boardstudio-web/debug/web/public` into the packaged route before runtime asset copying and offline embedding. Packaged browser gates run after that route is complete. The page test build reads the generated CSS and staged assets. | `dev/target`, confirmed by the Dioxus Cargo trace |
| Offline worker JS and Wasm | The provider writes its packaged JS and Wasm to `web/target/providers/offline-worker`. The staging functions enumerate the core worker, CAD worker, CAD Wasm and renderer directories. Dioxus receives those staged outputs. `embed_offline_worker` is the first file consumer, after `copy_runtime_assets`. It copies the JS and Base64-embeds the Wasm into the route bootstrap. The overlap must join successfully before entering that function. | `dev/target` in the control. The overlap uses the sibling `offline-target`. |
| Browser harness and page-test executable | Drag, CAD capture, cold-response, CAD provider, model lookup, source parity and face-completeness call the browser harness. The Wasm inventory guard inspects source. The CAD browser script launches ChromeDriver and serves the packaged route. The last check lists the page test executable and runs the same page tests in distinct filters, including `presentation::case_workspace::`. | `dev/tools/browser-harness/target` for native harness Cargo. `dev/target` for the page runner. |

### Measurement setup

The external driver lives in `/home/chris/.cache/tmp/packaged-overlap.8F80HoAl/driver`. It invokes the pinned `check.py` command in the same detached checkout for each arm. Both schedules use the same instrumentation, inherited build lock, all 16 available logical CPUs and the checkout's 12 jobs per Cargo invocation. The overlap adds no cores and can compete for them. It starts the worker in its original position and joins immediately before its first read. The existing browser invocations and arguments are used without substitution.

The workspace, CAD Wasm and harness build targets start empty. The extra worker target exists and starts empty in every arm. Generated assets and experiment temp files are cleared between arms at the same paths. Cargo uses the prepopulated registry with offline resolution to keep downloads outside the measurements. The driver configures both schedules to use the same patched 0.2.129 pair produced by the pinned builder, Rust 1.98.0, wasm-pack 0.15.0, Dioxus 0.7.10, Chrome for Testing and ChromeDriver 156.0.8078.73, and `CARGO_PROFILE_DEV_DEBUG=false`. Browser harness and WebDriver runs create fresh temporary profiles. The drag harness uses its existing shallow temporary path. The page runner retains its separate case-workspace invocation.

Evidence is under `.evidence/packaged-build-overlap/`. Each arm records the clean checkout, invocation environment and target metadata in `preparation.log`, all command output in `run.log`, per-core ticks and busy fractions in `cpu.jsonl`, Cargo invocations and any lock diagnostics in `cargo.jsonl`, and build and gate intervals in `stages.jsonl`. The driver compares renderer bytes with the producer at both copy destinations and checks the packaged offline worker against its producer. `pair-build-online.log` records preparation of the patched pair before timing.

### Measured result

**Retain sequential execution.** Both fast builds succeeded. Both packaged runs failed on the first browser gate, so neither provides a time through successful completion of the final required check. The overlap build took 44.731 s longer in these two runs with empty build targets. The reverse-order pair is reserved for a promising first pair with equivalent passing coverage.

| Schedule | Fast build | Drag gate, exit 1 | Start to failed gate end | Start to process exit |
|---|---:|---:|---:|---:|
| Sequential | 385.941 s | 82.051 s | 468.164 s | 468.216 s |
| Overlap | 430.672 s | 68.622 s | 499.466 s | 499.521 s |

In each run, the first browser command reached `Part drag starts` and timed out waiting for the document's save state to become `saved`. Both logs also report a WebSocket reset. The cause remains unknown. Each receipt's ten planned packaged commands match the dev audit in order. Drag exited 1 and the runner recorded the other nine as `not_run`, including model lookup and the isolated page suite. Neither run produced a passing drag receipt or an actual page-test inventory. The end-to-end verification task above remains incomplete. [Sequential receipt](../../.evidence/packaged-build-overlap/01-sequential/check-receipt.json.md) · [Overlap receipt](../../.evidence/packaged-build-overlap/02-overlap/check-receipt.json.md)

The offline worker finished before embedding in each run. On the sequential schedule it ran from +105.289 to +119.027 s. On the overlap schedule it ran from +106.579 to +125.626 s, concurrently with the renderer (+106.580 to +126.854 s). Embedding began at +430.102 s in the overlap arm. The Cargo trace confirms distinct concurrent targets. Both arms verified the packaged worker JS and embedded Wasm against the producer, plus all six renderer files at both staging destinations. [Pair verification](../../.evidence/packaged-build-overlap/pair-verification.log)

Cargo emitted zero lock notices in the control and seven package-cache lock notices in the overlap arm. Two notices were followed by compiler output after 66 and 116 ms. The other five were followed by metadata process exit after 101–259 ms. These are upper bounds on individual waits. [Cargo trace](../../.evidence/packaged-build-overlap/02-overlap/cargo.jsonl) Mean host CPU utilization across all 16 logical CPUs was 43.3% sequential and 38.5% overlap, from 433 and 425 sampling intervals respectively. Both arms used the same offline Cargo registry, but its starting contents were neither restored nor verified. The host file cache was 13.7 GiB at the sequential start and 4.5 GiB at the overlap start. Available host memory also differed (17.9 vs 8.1 GiB). The measurements do not isolate a cause for the build-time difference. Both runs used the pinned tool pair. Dioxus used Rust 1.100.0-nightly for its Cargo unit graph and stable 1.98.0 for the page build. [Comparison and lock observations](../../.evidence/packaged-build-overlap/comparison.log) · [Sequential preparation](../../.evidence/packaged-build-overlap/01-sequential/preparation.log) · [Overlap preparation](../../.evidence/packaged-build-overlap/02-overlap/preparation.log)

## Plan Change Log

- 2026-11-17. The first sequential drag failure ruled out adoption. Completed the specified first pair to measure the intervention and determine which pinned gates actually ran in each arm. Both exit-1 receipts are retained.

## Review Triage Log

Quick review returned four findings (two medium, two low).

| Finding | Verdict and disposition |
|---|---|
| End-to-end coverage | Medium. Both receipts confirm the Part drag timeout and nine unrun checks. The failed-gate decision applies. Drag timeout diagnosis remains open outside the spike. |
| CAD Wasm dependency trace | Low. Inspection of the pinned model-lookup and Ergogen route handlers confirmed their independent inputs. Corrected the trace above. |
| Starting caches | Medium. The driver resets build targets and generated files. Shared registry contents and host file-cache state were uncontrolled. Left the reproducible-cache task open and qualified the timing comparison. |
| Cargo lock observations | Low. Verified all seven notices against the raw trace and reported the two next-output and five process-exit bounds. |

## Verification

Verify `git rev-parse HEAD` in the experiment checkout before every arm. Use the pinned `check.py` entry point above, which runs `scripts/build-web.py --fast` followed by `browser-packaged`. Diff each arm’s actual ten-command inventory against the audit and compare each named page result and gate outcome across arms. Check that the offline worker succeeded before its consumer and that concurrent Cargo invocations had distinct resolved targets. For a recommendation, calculate both paired savings and both absolute within-schedule differences directly from monotonic durations.
