# Amended F2.2 archive packet — independent Standards review

Reviewed 2026-10-02 by `/root/keycaps_stream_audit/keycaps_standards_review`.

Disposition: **Standards clear for these exact planning snapshots.** No actionable Standards findings remain in the amended scope. This is not implementation acceptance or proof that the packaged-byte provider currently exists.

| Reviewed file | SHA-256 |
| --- | --- |
| `.scratch/dioxus-project-archive/spec.md` | `f0fa448487dd81c81f073bba1cb067bc2e99c8ed262f37c070919504823d3e03` |
| `.scratch/dioxus-project-archive/drafts/01-bundle-aware-portable-pack.md` | `46ddfc6216bb79961375230ac5d6f833853306c758d8966ebcdcabc5d2573f35` |
| `.scratch/dioxus-frontend-tranche-1/drafts/16-project-menu-portable-copy.md` | `3b1201fce52652f3db4ebee25e26fc65dec4fda7ae8147718220381416fdaa7b` |

## Review evidence

- Opaque bytes: spec lines 34 and 44 and capability ticket line 15 explicitly preserve empty/content-unrecognized byte payloads when existing availability, digest, path and size checks pass. This matches pinned React `app/src/bundledModels.ts:30–35` (reads an ArrayBuffer without model parsing), `app/src/storage.ts:195–199` (hashes those bytes), and current Core `core/src/archive.rs:42–72,194–210` (archive/reference/hash validation, without model-content validation).
- Provider ownership: spec line 31 and capability ticket lines 7–9 require separately proven callable packaged identity/metadata/bytes access. The adapter owns reference closure, option handling, immutable packed-document construction and archive invocation; it does not silently acquire provider implementation or a network-source mandate. Current `web/src/presentation/model_delivery.rs` distinguishes a missing bundled provider, and `web/src/runtime.rs:1131–1174` currently packages document-owned assets through the existing worker. The stated prerequisite therefore remains real and explicit.
- Private boolean versus visible checkbox: spec lines 28–29, capability ticket lines 7–9 and 19, and issue16 lines 7–9 correctly let private adapter/menu work begin from a proven boolean input. F8 retains its visible checkbox and shared-entrypoint paired acceptance. Issue16 line 15 preserves one shared preference, initially true, including a changed preference flowing from Export to Project-menu export.
- Archive behavior: local assets remain included for both option values; the packed-document augmentation does not mutate the accepted snapshot or stores. This follows pinned React `app/src/storage.ts:167–215`. Existing optional archive metadata is sufficient (`core/src/archive.rs:44–55`); no new schema or archive engine is needed. The project-name filename matches pinned `app/src/exports/documents.ts:7–9`.
- Boundaries and gates: shared Runtime/public-contract edits retain normal owner/review requirements (capability ticket line 24). F2.2 retains its F2.1 start and INT.2 acceptance join; F8.6 retains F2.2, consistent with the existing workflow records. Paired browser/output/import evidence is still required; private helper tests cannot close parents.

React source was inspected at pinned commit `5a472a9426e6e38993361da402cd4ec730feb369`. Review also applied `CONSTRAINTS.md`, project issue-tracker/ownership instructions, and the current F2/F8 workflow records. No target document, source, or shared ledger was edited; only this report was created. No builds or browser journeys were run because this is a planning-document review.

No new refactoring takeaway observed in this amended scope. Existing RF-001/RF-002/RF-009 remain applicable; ledger ownership is unchanged.
