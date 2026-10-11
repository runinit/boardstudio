# 136: Publish saved-project archives

Status: claimed
Type: build
Blocked by: none

## Purpose

Publish the canonical Sofle ordinary saved project as a tracked archive that the existing Archive/Artifact flow can open and export. This is a bounded producer and presentation slice under the shared board contract in [25](25-shipped-boards-review.md). It does not wait for all of 25, shared mirror/link tickets 19/22, or engineering parent 106.

## Scope

- Use tracked `content/archives/sofle.boardstudio` as authoritative project data; accepted archive SHA256 a2509dd893761b9a85ab0ba7085f701653cd8cbe77120de8844c5b76e404f63e (1,616,286 bytes). Generalize the existing REVIUNG41 source-archive publication route in `core/examples/demo_projects/mod.rs::prepare`, producer slice `prepare/split_projects/SOFLE_VARIANTS`; pre-edit source SHA256 1da55618139c9f0d00b542bae1a0447a2f3348929e26dd971c3c96ebfb7f245e. Preserve archive JSON, assets, provenance, existing REVIUNG41 behavior, other demos, and failure/replacement safety.
- Add a narrow generic publication regression in `core/tests/demo_projects.rs`; pre-edit source SHA256 431ffdd275bb46730e4467087daa662fca6796b299250ac1b3c3de3cb4b3db46. Do not edit `scripts/build-web.py` unless the implementation proves it necessary and root grants its exact path.
- Separately, replace the three bespoke Sofle library cards with one canonical Sofle card while retaining MX/RGB/Choc as shared Inspector assembly choices. UI lease is limited to listing/rendering in `web/crates/library/src/library.rs`; pre-edit source SHA256 20bcd68a214b7ad866b82737d2814d50ff9258bd79aa2979184ed5d59cdbc3f4. Keep the UI change small and validate its actual WASM/typecheck/browser surface.

No per-board builder, new API/schema/visibility, publisher framework, headless tool, or bespoke generated Sofle outputs. Preserve ordinary archive operations and all other demos.

## Acceptance

- Demonstrate meaningful producer RED/GREEN and deterministic regeneration with at least two named archives, nonempty project/assets/provenance controls, and complete output manifests.
- Repeated preparation preserves the accepted archive; invalid or missing source cannot destroy the last accepted outputs.
- Existing REVIUNG41 publication remains equivalent; verify the generic artifact build/open/export path.
- The canonical card has no dangling library references and preserves shared assembly choices. Run affected typecheck and actual browser/WASM coverage.
- A fresh, different Sol 6.1 xhigh verifier independently checks the restored, source-bound delta and meaningful mutations. Root retains integration, final checks and GitNexus ownership.

## Allocation

Claimed for the existing F coordinator. Original claim commit: 104edc1f. F's final DATA verifier PASS is recorded in /home/chris/.cache/boardstudio-baselines/sofle-next-unit/verifier-all14-final-verdict.json, SHA256 062429a83f0345ddadb3e767349e89072a72b428ab8bec2b18363e7c2b040d74. Accepted JSON SHA256 fd6a047498d794528060f30b3fffdc946c319858a7be2c0b8a3b6179b06813aa (233,838 bytes). DATA acceptance covers fresh public-API normalization equality for all14 definitions, 232 actual PCB pad nets, 116 matrix parts, 58 LINK entries, 10 rows/12 columns, four layouts, 58 poses, seven assets, history and save. This is DATA acceptance only: UI/assembly-swap, mirror, bench, fit and case gates remain open; publication136 is not delivered.

Exact product leases: (1) new `content/archives/sofle.boardstudio` equals the accepted archive bytes above; (2) `core/examples/demo_projects/mod.rs`, pre-edit SHA256 1da55618139c9f0d00b542bae1a0447a2f3348929e26dd971c3c96ebfb7f245e, producer `prepare/split_projects/SOFLE_VARIANTS` publication-only; (3) `core/tests/demo_projects.rs`, pre-edit SHA256 431ffdd275bb46730e4467087daa662fca6796b299250ac1b3c3de3cb4b3db46, narrow generic publication regression; (4) separate UI slice `web/crates/library/src/library.rs`, pre-edit SHA256 20bcd68a214b7ad866b82737d2814d50ff9258bd79aa2979184ed5d59cdbc3f4, listing/render only, three Sofle cards to one canonical card. No broader UI or assembly redesign.

Implementation worktree: /home/chris/.codex/worktrees/sofle-adaptations/ts-boardstudio2, branch codex/sofle-adaptations, HEAD 9125d990e631894641472e9ec94e7b79e3d28d25. F explicitly released global foreground to root. Product START remains held until root commits this exact allocation and explicitly grants START. Fresh Luna HIGH implementation then different fresh Sol 6.1 xhigh verification; root retains integration, checks and GitNexus ownership. Preserve REVIUNG41 behavior, assets, provenance, error safety and deterministic regeneration. No builder/schema/API/headless tool/goldens/scripts/build-web.py edits. Parent25, sign-off56, hardware118 and shared mirror/link19/22/106 remain open.
