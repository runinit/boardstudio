# Standards review: STEP-only CAD worker reply decoder

Reviewed commit `5a0972a2889a702051e9a7ab53b2727db07126e0` on `codex/m1-cad-worker-array-fix-20261001`, based on integration commit `e528b557`. The production source delta relative to the existing integration baseline is confined to `web/src/cad_worker.rs`; the other added files are a browser test harness and its recorded evidence.

The change follows the existing typed worker seam. `decode_result` now maps an omitted `bodies` field to an empty list, accepts arrays as before, and returns a normal decoder error for a present non-array value. The reply receiver sends that error through the pending request channel, avoiding the prior uncaught `Array.from(undefined)` exception and stranded request. The additional wasm-bindgen test entry point is gated behind `test-harness`, so it is not exported by production page or worker builds. No production API visibility, wire schema, or provider contract is widened.

The public-worker browser harness exercises both the omitted-field success case and malformed-object error case. Recorded checks for the candidate are: 12 native library tests passed; formatting passed; strict wasm Clippy passed with `page,cad-worker,test-harness`; the pre-fix browser harness failed by timing out on omitted bodies; the fixed harness passed both cases; and the independent focused public CasePanel export downloaded the expected STEP bytes and revoked its Blob URL. `git diff --check` also passed.

**Findings:** none in the reviewed decoder change. This review covers the focused source delta and test seam, not release freshness or full production offline acceptance; those still require the maintained integration build and final root/subpath checks.
