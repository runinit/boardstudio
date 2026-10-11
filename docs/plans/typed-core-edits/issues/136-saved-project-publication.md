# 136: Publish saved-project archives

Status: claimed
Type: build
Blocked by: none

## Purpose

Publish the canonical Sofle ordinary saved project as a tracked archive that the existing Archive/Artifact flow can open and export. This is a bounded producer and presentation slice under the shared board contract in [25](25-shipped-boards-review.md). It does not wait for all of 25, shared mirror/link tickets 19/22, or engineering parent 106.

## Scope

- Use tracked `content/archives/sofle.boardstudio` as authoritative project data. Generalize the existing REVIUNG41 source-archive publication route in `core/examples/demo_projects/mod.rs::prepare`. Preserve archive JSON, assets, provenance, existing REVIUNG41 behavior, other demos, and failure/replacement safety.
- Add a narrow generic regression in the existing `core/tests/demo_projects.rs` integration test file (filename confirmed by directory listing only). Root binds its exact lease and source hashes before START. Do not edit `scripts/build-web.py` unless the implementation proves it necessary and root grants its exact path.
- Separately, replace the three bespoke Sofle library cards with one canonical Sofle card while retaining MX/RGB/Choc as shared Inspector assembly choices. The existing `web/crates/library/src/library.rs` filename is confirmed by directory listing only; root must bind the exact UI sublease before edits. Keep the UI change small and validate its actual WASM/typecheck/browser surface.

No per-board builder, new API/schema/visibility, publisher framework, headless tool, or bespoke generated Sofle outputs. Preserve ordinary archive operations and all other demos.

## Acceptance

- Demonstrate meaningful producer RED/GREEN and deterministic regeneration with at least two named archives, nonempty project/assets/provenance controls, and complete output manifests.
- Repeated preparation preserves the accepted archive; invalid or missing source cannot destroy the last accepted outputs.
- Existing REVIUNG41 publication remains equivalent; verify the generic artifact build/open/export path.
- The canonical card has no dangling library references and preserves shared assembly choices. Run affected typecheck and actual browser/WASM coverage.
- A fresh, different Sol 6.1 xhigh verifier independently checks the restored, source-bound delta and meaningful mutations. Root retains integration, final checks and GitNexus ownership.

## Allocation

Claimed for the existing F coordinator. Product START remains held until the candidate artifact is accepted, root commits this claim, binds the exact producer/test/UI paths and hashes, and explicitly grants the implementation START. Fresh artifact acceptance FAILED: after RemoveMatrix/SetMatrix, all 116 regenerated switch/diode parts had zero net pins; 58 LINK_MATRIX nets were empty with incomplete controller nets, and final layouts[] dropped four existing layouts. The 58 poses/assets/reset wiring passed. Root authorizes one bounded same-data-path correction: existing resolve-electrical → apply-electrical for both halves, plus ordinary SetLayout restoration, with generic all-matrix-part, export-pad, net, settings and layout-preservation checks. No shared bug or production builder is claimed. The failed candidate must not be published. Original Luna resumes only after the verifier stops and root regrants; the same independent Sol re-reviews after writer stop. If a fresh substantive failure repeats, stop and consolidate for root decision; no automatic retry loop. Parent25, sign-off56 and hardware contract118 remain open.
