# Model-delivery ports: final Spec review

**Clear for implementation within the stated private boundary.** Reviewed complete corrected spec SHA-256 `8db0731d531d7dccbed90154c093d4fa381f7db8991e990db84c1592056ddfd5` against the approved `e3529ac7` model contract and the three findings in `model-delivery-private-ports-spec-review.md`.

All three findings are closed:

1. `ModelOwnerIdentity` explicitly excludes renderer sequence; `ModelBatchIdentity` adds accepted revision and replacement-batch generation. Full Scope/token/viewer/projection identity and exact source-scene pointer govern model work. The viewer allocates a fresh renderer sequence independently for partial/complete scene submission and preserves the renderer’s acceptance result. Publishing one healthy row cannot invalidate other current model loads.
2. Completed immutable SHA meshes and pending tasks have distinct cache states. Pending sharing is restricted to the exact batch; new batches restart old pending slots. Exact task tokens guard cache updates and eviction, so stale cleanup cannot remove replacement work. Superseded work publishes neither errors nor meshes. Scope/unmount clearing, the 80-entry bound and current failure retry remain explicit.
3. Decoder validation now requires nonempty complete triangles, equal-length normals, finite buffers and same-length optional RGB colors. STEP receives that final shape gate in addition to existing reply/bounds validation. Invalid models remain isolated from healthy rows.

The added shared mesh handles avoid copying buffers per instance. Host namespace retention stays root-owned, with an explicit source/snapshot normalization delta and synchronization test; reusable library/public WASM surfaces remain unchanged. Asset precedence, memory-first SHA verification, BoardReference transforms, exact model IDs and current-board reference picks retain prior clearance.

This approves the bounded implementation specification, not source implementation or F7.3 acceptance. Native KiCad generation, unarchived bundled-byte packaging, actual row/unique-decode measurements, STL/WRL delivery, stale/retry controls, public picking and offline/lifetime evidence remain required joins. No source edits, Cargo or browser execution.
