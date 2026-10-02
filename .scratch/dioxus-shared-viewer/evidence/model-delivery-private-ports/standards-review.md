# Model-delivery private ports — Standards review

Reviewed final spec SHA-256 `8c47dc4316804e20a586cf8a7cdc1323fdcd39b980f2d4840c29abbbeb98b23f` against approved contract `e3529ac79f1b7d69211a8fa4cdc9e4d1170ec39a3c6510ab458dddc3a634b946`, worker `84c300a49ec6474e07c49f2d670c9e8205f2c14d`, and integration `9ef5bc5cad09ab3064711b45fc348b4cc09a74d4`. Also checked the complementary KiCad bridge ownership notes.

**Clear for bounded private archive-backed authoring.** No material Standards planning blocker. Runtime retains verified asset and CAD-worker authority; presentation owns scoped delivery/cache/status; the private page host retains the initialized renderer namespace for existing decoder exports. Source confirms the underlying packaged Ergogen index exposes modelAssetId even though its narrow facade does not. No new public provider, renderer export, visibility or domain transformation is required. Exact PcbModel.id joins and owner-side reference-to-Part picking preserve existing contracts; frontend double transforms are expressly excluded.

Implementation obligations to preserve this clearance:

- Decode-cache entries must share immutable mesh buffers through Rc or equivalent JS handles. The 124 model rows retain separate IDs/metadata while reusing four unique asset meshes; caching decode work must not become 124 deep mesh copies. The 80-entry retention cap is a count bound, not a demonstrated memory budget. Record buffer/transfer costs and avoid a second persistent raw-byte cache.
- Keep frozen batch ownership distinct from a newly allocated renderer submission sequence. Aggregate once or define guarded progressive delivery so the first accepted partial scene cannot invalidate all sibling completions; old owner completions must publish neither meshes nor errors. Async tasks must not keep a closed viewer/cache alive indefinitely.
- Root owns private host namespace retention and exact source-sync adjustments. Preserve the original library host and assert precisely enumerated page-only differences; no broad normalization that masks unrelated drift.
- The KiCad complement mentions packaged bundled-manifest lookup, while this slice correctly treats that manifest as absent. Preserve the explicit missing-model result until a separately reviewed static-byte port exists; do not silently invent that provider.

Source-backed STEP validation, same initialized STL/WRL namespace, full scope/token/projection guards and per-model failures are sufficiently concrete. RF002/RF012 remain existing boundary observations; no new structural takeaway established. Compiler, public model counts/transforms/picks, retry, supersession, teardown and parent acceptance remain open. No source edits, Cargo, browser or publication performed.

## Final delta review

Re-reviewed specification SHA-256 `8db0731d531d7dccbed90154c093d4fa381f7db8991e990db84c1592056ddfd5`. **Bounded private archive-backed implementation authoring remains clear; no material Standards finding.**

The revised private types separate owner/batch identity from renderer submission sequence. Pending tasks now belong to an exact batch and task token; stale completion/cleanup cannot overwrite or evict a replacement task. Completed meshes remain immutable shared handles, preserving separate exact model IDs without per-instance mesh copies. The complete-triangle gate (positions divisible by nine), matching normals/colors and finite buffers explicitly applies before submission, including STEP. Host changes remain root-owned, precisely enumerated page-normalization differences with the reusable host surface and generated exports unchanged.

The earlier obligations are now explicit implementation requirements. Implementation review must still verify weak/closed-owner lifetime, actual buffer sharing and transfers, exact host source-sync assertions, and all owner/batch checks across awaits. The 80-entry bound remains a count limit rather than a measured memory budget. No unarchived bundled-model provider is authorized by this slice. Compiler and public acceptance evidence remain open. No source edits or Cargo performed.
