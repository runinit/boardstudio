# Packaged model provider — independent Standards review

Exact reviewed SHA-256:

- Provider spec: `6c2a6ea69bbf120f31b509909c7c892e028b068c3915dded953535a63ad12d7d`
- Provider ticket02: `d642e06d35163f645d5256de3c82054f1ac402a375c629fd5d50619827945351`
- Archive spec: `548ca2b9420db143b610d0bb314162b80dc66d22c077141552df72b17b6a1fa7`
- Archive ticket01: `dbd91e7d2bae8ae4f100004179957cf2b7fbdf656fa0346612a92da73768a4d3`

**One bounded build-boundary amendment before dispatch.** Provider Implementation Decisions and ticket02 static/offline criteria must distinguish normalized staged filesystem/manifest paths from encoded fetch URLs. `web/build.rs:45–56` rejects percent signs, fragments and query delimiters in manifest entries; `service_worker.rs:61–66` constructs precache requests directly from those entries. Encoding special-character source filenames into URLs is insufficient if those URLs are emitted verbatim into the manifest. Specify a deterministic manifest-safe staging path mapped from trusted catalogue entries, preserving React saved IDs/original filenames as metadata, or explicitly separate raw admissible staged paths from encoded URLs. Catalogue requests, staged files and precache requests must resolve to identical bytes without relaxing the shared validator. Current 88 source files contain no forbidden manifest characters (nine contain spaces); this is a planning ambiguity, not a reproduced failure.

Assign the generator/private provider to the feature and final page/dev/release staging, source/asset provenance and offline composition to the coordinator. Include generator/index/vendor-source hashes and retain the existing unchanged-provider guards; no broad build-guard relaxation is implied.

Otherwise clear: opaque bytes remain separate from renderer validation; archive ticket01 owns closure, private boolean, computed digest, packed-document augmentation and existing Core invocation. No persisted schema/public visibility or second archive/provider authority is needed. React alias and media-type mappings are source-grounded. Unknown-ID rejection avoids arbitrary fetches.

Root/subpath byte comparison, catalogue uniqueness/completeness and actual offline installation/retrieval are meaningful checks. The roughly 155.5 MB catalogue makes real installation evidence necessary; no cache/storage success is inferred from manifest inclusion alone. Existing F2.1/INT.2/F8 joins, issue16 dependency and coordinator RF ownership remain intact. No new refactoring takeaway beyond existing build/provider-boundary accounting; no source, Cargo, browser or shared-ledger edits performed.

## Corrected packet — Standards clear

Verified exact current hashes:

- Provider spec `2ca043c22d6e6590aeac9a7720fa7d5283b7985f2cc778a084cf3d433240c605`
- Provider ticket02 `043f410ac013d9d1299e2986d03fca95df64ce6c3b91047baeea0d8bbd563582`
- Archive spec `b234a83b4af54ae8220d4fdd8983c46c1afd5541e216bb17e2d8134fc1eb5108`
- Archive ticket01 `cb6a4f16452e54387d9680c5ed3fa9a3012fc8e956181e644797f72055891a3f`
- Project-menu issue16 `9a41ca5bfe251ad642f741c497bb31cca15fe9beb3559500bacc400ec033f776`

The required amendment is resolved: deterministic opaque staged paths are explicitly shared by generated provider metadata, deployment-aware requests and offline manifest; original vendor paths and saved aliases remain metadata. Existing manifest validation is retained. The general URL-encoding sentence must be read with these explicit requirements: requests use the emitted safe path, never the original vendor path. Coordinator owns shared build invocation/provenance; feature owns the staging helper/catalogue/provider.

The three sanitized-source-to-saved-name aliases now match `bundledModels.ts`. Archive tests and issue16 explicitly preserve `storage.ts`'s unconditional recognized `definition.models` traversal for unplaced document definitions, separately excluding unused library content and unreferenced generated models. This removes a misleading interpretation of “used models” without changing the existing closure authority.

No remaining material Standards planning finding. Ready for bounded private provider preparation and subsequent capability01 work once the actual callable provider exists; root integration remains coordinated. Actual root/subpath/offline installation and byte evidence, archive round-trip, shared preference integration and parent joins remain open. No source/build/browser or shared-ledger edits were performed.
