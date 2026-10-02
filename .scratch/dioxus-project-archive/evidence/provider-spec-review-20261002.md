# Packaged-model provider and archive handoff — independent Spec review

Reviewed exact SHA-256:
- Provider spec `6c2a6ea69bbf120f31b509909c7c892e028b068c3915dded953535a63ad12d7d`.
- Provider ticket02 `d642e06d35163f645d5256de3c82054f1ac402a375c629fd5d50619827945351`.
- Archive spec `548ca2b9420db143b610d0bb314162b80dc66d22c077141552df72b17b6a1fa7`.
- Archive ticket01 `dbd91e7d2bae8ae4f100004179957cf2b7fbdf656fa0346612a92da73768a4d3`.

**Hold for two bounded source clarifications.**

1. **Pin alias direction and exact source paths.** `app/src/bundledModels.ts:14–20` maps actual sanitized source filenames to historic saved/display aliases. The provider spec instead describes source names containing URL-fragment characters. Explicitly require `THQWGD001-rotation.stp` → `THQWGD001 #1.stp`, `THQWGD001C-2pin.stp` → `THQWGD001C [2pin] #1.stp`, and `THQWGD001C-4pin.stp` → `THQWGD001C [4pin] #1.stp`, for vendor `thqwgd001`. Static retrieval uses the left-hand source file, while saved ID and archive filename use the right-hand alias. Preserve safe URL encoding without constructing a nonexistent '#1' source file.

2. **Make the definition.models closure exception explicit.** Archive spec says unused models are absent and requests “unused bundled definitions” tests, but `storage.ts:178–180` adds recognized `definition.models` from every document definition even with no placed part. Only the generator-derived definition scan at 173–176 is gated by actual document parts. Require a positive unplaced-definition-with-models fixture and a separate negative unused-generator-definition-without-model-references fixture; unused catalogue entries remain absent. Do not optimize this into a placed-parts-only closure.

The earlier provider ownership and private-bool findings are closed: ticket02 now owns deterministic packaged ID/metadata/raw-byte resolution, and ticket01 owns its newly introduced bool plus closure/packing. F8 retains visible control ownership; issue16 consumes capability01 without a new parent edge. F2.1/INT.2 and F8.6→F2.2 are unchanged.

Opaque bytes, computed catalogue digest, existing Core archive validation, definition/module/circuit/assembly/reference closure, source ID preservation, static root/subpath deployment and explicit offline-manifest verification are appropriate. The provider must stay separate from renderer validation/decoding and must not imply full F7 model delivery. No invented expected digest or remote fallback is required.

No source/target specification/ticket/ledger edits; only this review artifact written. No build, Cargo or browser checks run; source and planning review cannot establish provider/offline/archive acceptance.

## Final amendment — Spec-clear

Verified exact hashes: provider spec `2ca043c22d6e6590aeac9a7720fa7d5283b7985f2cc778a084cf3d433240c605`; provider ticket02 `043f410ac013d9d1299e2986d03fca95df64ce6c3b91047baeea0d8bbd563582`; archive spec `b234a83b4af54ae8220d4fdd8983c46c1afd5541e216bb17e2d8134fc1eb5108`; archive ticket01 `cb6a4f16452e54387d9680c5ed3fa9a3012fc8e956181e644797f72055891a3f`; Project-menu issue16 `9a41ca5bfe251ad642f741c497bb31cca15fe9beb3559500bacc400ec033f776`.

Both findings are closed. All three source-to-saved-name aliases are explicit and preserve the source retrieval path independently of saved identity. Archive spec, capability01 and issue16 now require recognized definition.models on unplaced document definitions and distinguish unused library/no-part generator negatives. Opaque deterministic staging paths preserve byte/identity parity without putting original URL-sensitive names in the offline manifest. Feature catalogue/staging ownership and coordinator build/provenance wiring are separate and concrete. The existing safe emitted-path rule governs provider URLs; original vendor paths remain trusted metadata/source inputs.

No remaining Spec blocker in this exact packet. Provider02 → pack01 → menu16 is a private capability chain; private bool creation, F8 shared preference integration, F2/F8 canonical joins and renderer separation remain unchanged. This is planning clearance only: exact-byte root/subpath/offline delivery, archive round-trip, public menu/preference journeys and full parent acceptance remain unexecuted gates.
