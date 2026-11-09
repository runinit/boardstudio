# 136: Publish saved-project archives

Status: resolved
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

Claimed for the existing F coordinator. Original claim commit: 104edc1f. F's final DATA verifier PASS is recorded in /home/chris/.cache/boardstudio-baselines/sofle-next-unit/verifier-all14-final-verdict.json, SHA256 062429a83f0345ddadb3e767349e89072a72b428ab8bec2b18363e7c2b040d74. Accepted JSON SHA256 fd6a047498d794528060f30b3fffdc946c319858a7be2c0b8a3b6179b06813aa (233,838 bytes). DATA acceptance covers fresh public-API normalization equality for all14 definitions, 232 actual PCB pad nets, 116 matrix parts, 58 LINK entries, 10 rows/12 columns, four layouts, 58 poses, seven assets, history and save. At the DATA verifier checkpoint, UI/assembly-swap, mirror, bench, fit and case gates remained open; publication was later independently verified and integrated as recorded in Outcome.

Exact product leases: (1) new `content/archives/sofle.boardstudio` equals the accepted archive bytes above; (2) `core/examples/demo_projects/mod.rs`, pre-edit SHA256 1da55618139c9f0d00b542bae1a0447a2f3348929e26dd971c3c96ebfb7f245e, producer `prepare/split_projects/SOFLE_VARIANTS` publication-only; (3) `core/tests/demo_projects.rs`, pre-edit SHA256 431ffdd275bb46730e4467087daa662fca6796b299250ac1b3c3de3cb4b3db46, narrow generic publication regression; (4) separate UI slice `web/crates/library/src/library.rs`, pre-edit SHA256 20bcd68a214b7ad866b82737d2814d50ff9258bd79aa2979184ed5d59cdbc3f4, listing/render only, three Sofle cards to one canonical card. No broader UI or assembly redesign.

**Historical implementation handoff:** Original claim committed at 104edc1f; exact allocation committed at 8adb9942. The F writer completed, STOPPED and explicitly released before fresh verification. Root verified the exact four leased paths and hashes and refreshed the source-bound F index successfully in 32.7 s; all three source metadata/live bindings matched, with the dirty warning separate. Binding: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-pre-verifier-source-binding.json`. Writer-reported checks passed: repo/lint/typecheck/build/native tests/browser 315.4 s (15 CAD and 71 presentation), generic RED-to-GREEN, deterministic full manifest with two archives/assets/provenance, missing/corrupt-output preservation, open and exports. These preceded the independent acceptance below.

**Final allocation state:** The new, different fresh Sol 6.1 xhigh publication verifier completed PASS and committed the exact four product paths at `4183cc0f1a08b554567faa3f9a01b74232689b7c`. F explicitly released foreground; root merged the verified lane with `--no-ff` into integration `6163b9221`. Publication is complete; F has no active command lease and root owns foreground commands, integration, index and tracking.

## Outcome

Resolved after fresh independent Sol 6.1 xhigh publication acceptance, accepted source commit `4183cc0f1a08b554567faa3f9a01b74232689b7c`, root integration merge `6163b9221`, and the full integration cohort PASS. The shared saved-archive route preserves REVIUNG41, assets and provenance, deterministic regeneration, and accepted outputs on missing/corrupt input; one canonical Sofle card replaces the three variant cards.

- Independent verification passed six meaningful mutations with exact restores and final GREEN, native publication tests 2/2, and Chrome/WASM Library tests 16/16. Complete source-bound change analysis covered 14/14 symbols and four files at MEDIUM risk, with no partial/truncated flags.
- Final source/output binding: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-source-output-binding.json`; graph evidence: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-final-graph-evidence.json`; mutation evidence: `/home/chris/.cache/boardstudio-baselines/sofle-next-unit/publication136-verifier-mutation-results.json`.
- Full default integration cohort passed at `6163b9221` in 382.2 s: repo 0.1, tooling 3.0, lint 6.5, build 56.9, test 65.6 and browser 250.1 s. Log: `/tmp/boardstudio-cohort-publication136-retry.log`. The earlier attempt stopped at repo preflight on three incorrect root tracking links; those links were corrected before this successful full run, with no product failure or new product acceptance cycle.

Final product SHA256 hashes:

| Path | SHA256 |
| --- | --- |
| `content/archives/sofle.boardstudio` | `a2509dd893761b9a85ab0ba7085f701653cd8cbe77120de8844c5b76e404f63e` |
| `core/examples/demo_projects/mod.rs` | `6d1430dbf3e3ed839b3030ad44bb7c7595d70342b390db2a99dd819d45e9101b` |
| `core/tests/demo_projects.rs` | `6522698d367a9c851cb81294f9e2f794f8546a111e0ea8c91d109ca7e4f1ebfa` |
| `web/crates/library/src/library.rs` | `df998744571e27dc623bfdc2c71a002829d3d54f4b5e6b80c72e7e518b5ffb84` |

This resolves publication136 only. Parent25, human sign-off56, hardware118 and shared mirror/link19/22/106 remain open. Live Inspector/assembly-swap, bulk/propagation, bench, physical fit and case acceptance are not established by this outcome.
