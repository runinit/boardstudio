# Model delivery — independent Standards source review

Reviewed unmounted worker source `web/src/presentation/model_delivery.rs`, SHA-256 `715c5dc39a9f85bd78fc0de17919fab319a2afd28bec9df7465b0143706f6c24`, and its private implementation contract against ports `8db0731d`.

**Two P2 findings; source not yet clear.**

1. **Preserve pending as a distinct outcome** (`merge_model_rows`, lines 490–508). An absent outcome becomes a visible “missing model” error. During the approved progressive scene delivery, the first healthy completion therefore reports every still-loading row as missing. CONSTRAINTS.md UI requires preserved loading/error feedback; the ports require per-model pending/missing/error states and healthy partial delivery. Represent pending explicitly or treat absent outcomes as pending, and record actual missing assets explicitly as terminal failures. Add a mixed ready/pending/missing regression; the current empty-map test instead locks in the conflation.

2. **Return the replaced task for cancellation** (`claim`, lines 381–398; `make_room_for`, 441–443). Replacing a same-SHA pending entry from another batch overwrites its token, while `make_room_for` returns None because the key exists. Thus `Start.evicted` cannot tell the owner to cancel that old task, despite the documented cancellation seam. Token checks prevent stale cache mutation but do not bound old decode work across repeated replacement. CONSTRAINTS.md Architecture and Rust/Dioxus require bounded resource use, cancellation and deliberate async lifetimes. Return the previous pending token on replacement, and cover that return plus stale cleanup in the existing supersession test. Document owner-wide cancellation when clearing the cache.

Separate tooling observation: `CacheSlot` derives Debug while contained `ModelBatchIdentity` does not. Resolve before root registration; no compiler was run.

Source otherwise preserves exact asset lookup precedence, SHA verification, complete finite triangle buffers, exact model IDs/reference failures, shared Rc mesh storage, monotonic task tokens and an 80-entry cache. No frontend transforms or existing API widening. Tests meaningfully cover pure rules but are unexecuted and do not establish async owner checks, actual cancellation, native registration or browser delivery. Root must guard every awaited port and retain shared JS buffer handles without per-row copies. RF: no additional structural smell; bounded helper module remains coherent.

## Corrected delta — clear

Re-reviewed source SHA `303099456dd9c52195dfb0a476c1f407b821559b8280cd6c9154b6f49e6d7eae` and evidence SHA `03759b9dab926985900b6df180b6f9ec43750116dd999f3321edf6fc99999c1d`. Both prior P2 findings are resolved. Absent outcomes now produce pending exact model IDs; only explicit Err outcomes produce reference-labelled failures. Replacement returns the exact superseded token separately from LRU eviction, preserving the distinction between obsolete work and still-current uncached delivery. Tests assert pending/failure separation and exact displaced token plus stale settlement rejection. The unnecessary CacheSlot Debug derive is removed.

No new material Standards finding. Cache retention remains bounded to80 entries; live task concurrency and cancellation remain root adapter responsibilities. Actual owner-wide clear/unmount cancellation, port await guards, shared JS buffers, compilation and public delivery remain unproven. Source-only clearance permits commit/integration; no Cargo was run.
