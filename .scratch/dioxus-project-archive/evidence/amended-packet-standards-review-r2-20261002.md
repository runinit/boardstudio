# Amended F2.2 archive packet — Standards re-review r2

Reviewed 2026-10-02 by `/root/keycaps_stream_audit/keycaps_standards_review`.

Disposition: **Standards clear** for the exact snapshots below. No remaining actionable Standards findings in this amendment. This report supersedes the prior Standards report for these revised documents; it does not establish provider readiness or implementation/browser acceptance.

| Reviewed file | SHA-256 |
| --- | --- |
| `.scratch/dioxus-project-archive/spec.md` | `a2d98c8e8e03d043999fc207f72257c263ff2a6463d2058477a1a8583a353f4b` |
| `.scratch/dioxus-project-archive/drafts/01-bundle-aware-portable-pack.md` | `a5de20b13439107a76fec448456902324bb78eb5c4dc198149b431edca908e2e` |
| `.scratch/dioxus-frontend-tranche-1/drafts/16-project-menu-portable-copy.md` | `b958b4ab0befc1b4879efbbd2ccbdba5ea62be1ed058c111eac262b7883d0ade` |

## Verified amendments

1. **Computed bundled digest, without an invented expected digest.** Spec lines 31/44, capability ticket lines 15/18, and issue16 line 14 now distinguish computing SHA-256 for newly embedded catalogue bytes from verifying an independently declared hash. Pinned React `app/src/bundledModels.ts:2` declares only ID, URL and filename; `app/src/storage.ts:191–199` reads bytes, computes SHA-256, and constructs the asset record/path. Core `core/src/archive.rs:52–69,194–210` checks document references and path/payload consistency. Existing document-owned assets retain their declared digest verification. The amended wording follows these existing contracts.
2. **Opaque archive bytes bypass renderer validation.** The same clauses explicitly avoid `VerifiedModelBytes` and renderer format/content checks. Current `web/src/presentation/model_delivery.rs:179–185` rejects empty payloads and files above 32 MiB; Core's archive entry limit is 64 MiB (`core/src/archive.rs:20–26`) and its asset validation imposes no model-format parser or minimum nonzero length. React's packaged-byte read also performs no content parsing (`app/src/bundledModels.ts:30–35`). The packet preserves Core archive limits and digest/path validation without importing renderer restrictions.
3. **Ownership and acceptance remain intact.** The separately callable packaged-byte provider remains a start prerequisite; the capability ticket consumes that provider rather than implementing another provider. A private boolean supports adapter work before F8's checkbox; the shared visible preference and paired Project/Export journey remain integration acceptance. Public API/schema and parent graph joins are unchanged.

React source was verified at commit `5a472a9426e6e38993361da402cd4ec730feb369`. Reused the prior review's project standards, parent graph and ownership evidence, and re-read the affected contract source and all amended requirements. No builds or browser runs were required for this document re-review. No target document, source or shared ledger was modified; only this report was created.

No new refactoring takeaway observed in this amendment. Existing RF-001/RF-002/RF-009 ownership observations remain applicable.

## Final exact-hash acknowledgement

The final spec wording now scopes user-story 5's hash mismatch to an independently declared expected digest. Re-read that wording and verified the final snapshots: **Standards clear**, with no new findings.

- Spec: `cad28df64b3d2a03adb9377c1afb414c5efaad98de2cd86df614d8f3260d559d` (supersedes the spec hash in the table above).
- Capability ticket 01: `a5de20b13439107a76fec448456902324bb78eb5c4dc198149b431edca908e2e`.
- Project-menu issue16: `b958b4ab0befc1b4879efbbd2ccbdba5ea62be1ed058c111eac262b7883d0ade`.

The revised user story is consistent with the computed bundled-hash/Core path-verification contract reviewed above. Provider start prerequisites and public paired acceptance remain open; no target/source/ledger edits were made by this reviewer.
