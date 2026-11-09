# Monstertruck production performance baseline — 2026-10-18

This single-backend baseline completes revised ticket 011. It measures the current Monstertruck implementation; it makes no Cadrum comparison, speedup, regression or hard-memory-bound claim. Deep 007 repair accounting is a separate quality track.

## Source and method

- Served source: `7e148b976e010e082900cbeb24202de48362d483`, dedicated checkout `t3-011-measure`, optimized release provider and site. Later parent commits change measurement tooling and ticket documentation, not served CAD/worker/renderer code.
- Retained source/artifact/profile/workload/cache bindings are checked before and after each accepted capture. Runtime reduction accepted 17 captures with no failed observations, plus the successful clean-build capture.
- Each of 15 CAD workloads has one cold operation and five warm repetitions in the same worker session. The table reports the cold result and warm median, not five independent startup sessions. Each of two startup modes has one observation. This is an initial descriptive baseline, not a statistical latency guarantee.
- Recorded host environment: Linux 7.2.10 CachyOS x86_64, glibc 2.44, Python 3.14.8; about 30.6 GiB physical RAM. The first runtime capture recorded load averages 1.89 / 1.94 / 3.27. Per-capture environment files retain the actual load; no claim of an idle machine or numerical acceptance threshold is made.

## Build and artifact size

- One clean release web build: **606.90 s**.
- CAD WASM: **7,317,515 bytes**, **2,624,504 bytes gzip-9**.
- Largest sampled build-process-tree PSS: **3621.70 MiB**, across 11,637 snapshots. 451 individual process samples were unavailable; unavailable samples and the sampling interval prevent a true peak claim.
- The retained size CSV also reports the CAD worker, Core worker and renderer artifacts. No no-op, incremental-build, second-profile or Cadrum measurement is included.

## CAD operation latency and sampled process memory

Latency is dispatch to validated reply. PSS is the maximum sampled sum across the Node controller and its Chromium descendant tree, measured nominally every 50 ms. Browser-only classification was unreliable in these captures, so browser-only zeroes are **not** reported as memory measurements. These totals include runtime overhead and are not WASM heap size, added-report allocation, allocator-inclusive bounds or instantaneous high-water marks.

| Workload | Cold ms | Warm median ms | Sampled process-tree PSS MiB | Unavailable process samples |
| --- | ---: | ---: | ---: | ---: |
| `C1plate:step` | 36.69 | 28.35 | 643.94 | 6 |
| `C2trayboss:preview` | 148.62 | 2.11 | 667.73 | 4 |
| `C2trayboss:step` | 102.88 | 82.25 | 671.09 | 5 |
| `C3gasket:preview` | 32.00 | 1.31 | 660.63 | 3 |
| `C3gasket:step` | 25.18 | 21.50 | 653.31 | 5 |
| `K1keycap:preview` | 34.37 | 10.21 | 692.25 | 4 |
| `K1keycap:print` | 7378.79 | 7253.23 | 705.78 | 5 |
| `P1analyticPCB:preview` | 28.40 | 0.75 | 647.51 | 34 |
| `P1analyticPCB:step` | 22.94 | 18.72 | 656.43 | 6 |
| `Qvendor:default` | 41.01 | 20.81 | 752.30 | 3 |
| `Qvendor:tight` | 44.40 | 20.36 | 759.86 | 3 |
| `V1switch:import` | 398.83 | 189.44 | 757.57 | 5 |
| `V2vendor:import` | 1028.20 | 701.22 | 783.41 | 4 |
| `V2vendor:refine` | 1080.87 | 739.78 | 789.91 | 6 |
| `C1plate:preview` | 45.03 | 1.20 | 626.17 | 4 |

## Startup and page/export validation

| Mode | Navigation to usable landing ms | Worker ready ms | Initial CAD reply ms |
| --- | ---: | ---: | ---: |
| `startup:cold` | 252.62 | 75.23 | 36.82 |
| `startup:warm` | 140.48 | 87.76 | 39.74 |

The frozen U1 project was imported and rendered, and its mechanical and printable-keycap ZIP downloads completed. That path emits diagnostic phases and an empty runtime CSV; it is **validation only** and excluded from latency samples. Its wall time is not a page benchmark. The CAD STEP and print workloads in the table are the measured export operations.

## Coverage, checks and limits

- The selected release baseline covers **37/37 expected cells**: cold/warm results for 15 CAD workloads, the existing three startup observations for each startup mode, and one clean-build wall time. Raw generic harness coverage still lists unmeasured checked-profile, growth, no-op and incremental modes; those are outside this explicitly selected baseline, not hidden passes.
- Raw evidence: `.evidence/011-report-final/` (summary and selected coverage); `.evidence/011-rt-*/` (source bindings, runtime CSVs, environment and memory samples); `.evidence/011-monstertruck-only/build-release-clean/` (build and sizes); `.evidence/011-rt-u1page/` (validation and downloads). The earlier `.evidence/011-report/` is retained as the runtime-only generic reduction.
- Standard final verification at the production-source checkpoint reports repo, tooling, lint, build, test, typecheck, browser, security and deny passing in 707.6 s. Measurement-harness changes receive focused checks separately; no repeated corpus census is required.
- No historical invalid build binding or validation-only runtime capture was included. Output validation, source binding and uncertain collision results were preserved. These measurements neither certify unsupported geometry nor close deferred 007 quality work.
