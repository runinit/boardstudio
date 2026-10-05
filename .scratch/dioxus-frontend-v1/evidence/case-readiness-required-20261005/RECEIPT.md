# Required Case readiness batch

Three test-only Rust files add the missing failed-current/stale-scene/retry and Session preview/gesture readiness joins. The two focused tests each passed 1/1. Independent review found that the original local-export test used a document-wide Refresh lookup while the new fixtures left duplicate IDs in mounted roots. This is a fixture attribution defect, not a product defect. The author is applying root-scoped lookup and owned-root cleanup without changing assertions.

The first combined gate passed wasm page, native245 (one ignored) and reachability. Its broad headless run was explicitly stopped by the coordinator for the required correction; wrapper, delivery and runner PIDs were independently observed missing. It is incomplete, never reported passing. Raw log and termination reason are retained beside this receipt.

Independent review confirmed the presentation change is solely a wasm-test context bridge, and the Runtime change is a per-instance cfg(test) model override, defaulting to None and populated only by these CAD fixtures. Existing hash-bound owner entries now select all four CAD mounted tests for these exact HEAD/base and current bytes. Direct cad_presentation module selection is retained to execute the four together. Any mismatched source/base falls back to broad selection; runner and mandatory native/page/reachability gates are unchanged. The unchanged runner suite executed 37 tests in1.149s, all passed. Final corrected module, source handshake and exact guarded aggregate remain pending.

This avoids a second full unchanged presentation-child run for a local fixture correction. No tests are deleted or excluded; no forged/rebound receipt is used. Published cc728fc6 remains the application candidate because this batch changes test code only.

## Settled executed result

The corrected module executed all four tests together:4passed/0failed in the focused guard. Final combined gate exited0: wasm-page passed; native245passed/1ignored; reachability passed; strict headless4passed/0failed/0excluded/0incomplete in56.37seconds. Four owned source/config paths were staged before preparation; hashes and raw gate log are retained. Root-scoped Refresh and removal of the three export roots preserve original assertions and correct fixture attribution. The previous broad run remains explicitly incomplete, not a comparison pass or an invented timing saving.
