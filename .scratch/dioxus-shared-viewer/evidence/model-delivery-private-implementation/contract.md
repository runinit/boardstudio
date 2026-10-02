# Private model-delivery implementation seam

**Status:** unmounted private implementation; root integration and browser acceptance remain open.  
**Reviewed contract:** `e3529ac79f1b7d69211a8fa4cdc9e4d1170ec39a3c6510ab458dddc3a634b946`.  
**Implementation specification:** `/tmp/frontend-run/model-delivery-private-ports.md`, reviewed SHA `8db0731d531d7dccbed90154c093d4fa381f7db8991e990db84c1592056ddfd5`.

The new `web/src/presentation/model_delivery.rs` implements only a private presentation seam. It does not register itself in `presentation.rs`, change Runtime or renderer-host code, or publish a public API. Root owns the native/WASM registration, adapters, and compilation.

## Implemented types and behavior

- `ModelOwnerIdentity` captures `Scope`, `SnapshotToken`, viewer instance, projection generation, and a weak pointer to the exact `CadScene`. It deliberately has no renderer submission sequence. `is_current_owner` checks every captured owner field and pointer identity against the current scene.
- `ModelBatchIdentity` adds accepted document revision and batch generation. Its liveness check compares owner, revision, and current batch generation.
- `select_model_asset` follows the source lookup order: enabled board-reference mapping when supplied, native preview path table, then injected version-matched Ergogen ID lookup. It resolves only an exact `ProjectDoc.assets` row and uses that row's SHA/name. Missing `ergogen:model:*` bytes remain `MissingBundledProvider`; no inferred URL or asset identity is created.
- `VerifiedModelBytes::verify` checks the 1-byte–32-MiB range and SHA-256 before creating the private verified-byte wrapper. `ModelDeliveryPorts` exposes SHA-only byte loading and separate STL, WRL, and exact-owner STEP callback ports. Runtime/host implementations are still required.
- `ModelFormat` chooses STL, WRL, STEP, or STP case-insensitively. `ValidatedMesh` requires a nonempty positions array whose length is divisible by nine, equal-length finite normals, and optional finite colors exactly matching the position count. Its buffers are `Rc`-backed immutable slices.
- `ModelMeshCache` has an 80-entry LRU keyed by SHA. Ready meshes can be shared as `Rc`s. Pending entries belong to one batch and carry unique task tokens. A later batch replaces a pending slot and returns the superseded task's exact token so its waiters can be cancelled/settled; only the exact current token may complete or remove the replacement. Failed current work is evicted for retry; stale settlement is ignored. LRU eviction separately returns an evicted pending token, but only drops retention: a still-current task must keep delivering its live rows even though `complete` returns false because the slot is gone.
- `merge_model_rows` preserves preview order, exact `PcbModel.id` for successful rows, and `PcbModel.reference` for explicit terminal failures. Missing outcomes are exposed as pending row IDs, not failures; the adapter must add explicit failures for terminal missing asset/provider results. It does not manufacture a Part mapping or placeholder mesh.

## Native pure tests included, not run here

The new module contains unit tests for asset-source precedence and absent bundled bytes, byte digest/range checks, format routing, complete/finite mesh validation, same-batch pending joins, batch supersession and stale-token rejection (including the returned exact displaced token), retry after failure, capacity/eviction-token behavior, pending-versus-terminal-error rows, and exact renderer model-row IDs. Per root's compiler ownership, no Cargo or compiler command was run. These tests are not evidence of compilation until root registers and runs them.

## Integration still required

Root-owned Runtime byte loading must verify both current-import and BrowserStore bytes. Root-owned renderer host must call the existing renderer WASM STL/WRL exports. The existing CAD worker must implement the STEP port with captured snapshot/job validation. The Case viewer must supply real preview rows, board-reference context, native path IDs, owner/current checks, preserve healthy rows and per-reference errors, create renderer rows from shared mesh handles, and allocate renderer scene sequences independently. Browser/archive, real STL/WRL, STEP, stale-stage, teardown, transform, and pick-identity acceptance remain outstanding. The unarchived Ergogen static-byte provider remains an explicit missing-model case until its packaging bridge is separately implemented and verified.
