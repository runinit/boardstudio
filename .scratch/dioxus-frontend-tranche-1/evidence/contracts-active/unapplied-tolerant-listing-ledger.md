# Unapplied tolerant-listing implementation/API ledger

Status: **UNAPPROVED / UNAPPLIED**. Prepared at `cb8203fc` in `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001` on 2026-10-02. This artifact does not authorize repository edits or establish acceptance.

Patch: `/tmp/frontend-run/unapplied-tolerant-listing.patch`.
SHA-256: `28fced8e56078ee2f5bae63b0760348e611c5553081cd917bba9012783d838da`.
Exact before/proposed file hashes: `unapplied-tolerant-listing-manifest.json`.
Full proposed storage file for inspection/formatting only: `unapplied-tolerant-listing-storage.rs`.

## Exact new public surface

```rust
pub enum SavedDocumentEntry {
    Document(Box<ProjectDoc>),
    PreviewUnavailable { id: String, name: Option<String> },
    IdentityUnavailable { name: Option<String> },
}

impl BrowserStore {
    pub async fn list_document_entries(&self)
        -> Result<Vec<SavedDocumentEntry>, PersistError>;
}
```

The enum derives Clone/Debug and is re-exported through the existing public `boardstudio_web::host` module. Document is boxed from the initial definition to avoid the large-enum-variant layout hazard relative to the small fallback variants. No existing member visibility is widened. This is an additive PUBLIC API nonetheless and requires the user's explicit decision under AUTHORITY/ACCEPTANCE/CONSTRAINTS before applying it.

## Source change ledger

| File | Change | Reason / authority boundary |
| --- | --- | --- |
| web/src/host/storage.rs | Add SavedDocumentEntry before BrowserStore | Typed success plus identified/unidentified malformed entries; preserve every record. |
| web/src/host/storage.rs | Add list_document_entries beside existing list_documents | Same BrowserStore instance, same private database_name, existing one-transaction list_values; malformed records no longer erase healthy discovery entries. |
| web/src/host/storage.rs | Add private saved_document_entry / string_property | Decode ProjectDoc once; on error recover only exact string ID/name without fabricating a document. |
| web/src/host/mod.rs | Re-export new enum | Existing page binary must name the return type; private library items are unreachable there. |

Existing list_documents, load_document, save_document, writes, list_values, transaction helpers, DB/version/store/key names and all schema logic are byte-for-byte unchanged by the patch. It does not switch Library callers yet; adoption remains a separate private caller patch after API approval.

## Observable data/error contract

One list_values call opens one projects readonly transaction, runs getAll, awaits request result AND transaction completion, closes the database, and preserves getAll ordering. The new method classifies one entry for each returned value; no filter/filter_map or record skip. Provider/transaction/open failures remain top-level PersistError and drive the existing failed-list/retry state. The owned task/oneshot lifetime follows existing BrowserStore methods; a dropped caller cannot mutate saved data, and the UI still must guard obsolete completions.

Successful serde_wasm_bindgen decoding yields the boxed original ProjectDoc. Its non-finite/unsafe geometry remains a private per-card projection decision. Decoding failure extracts only string id/name with Reflect; a property-access failure is treated as absent for that one record. Exact ID strings are preserved, INCLUDING empty/whitespace strings: existing load_document(String) can address these IndexedDB keys, so the patch does not invent ID validation or coerce numeric keys. This intentionally corrects the earlier draft's nonblank-ID rule. Exact name strings, including blank/whitespace, are passed to the private reference name-fallback helper; provider does not trim them.

PreviewUnavailable retains the usable ID so the card's existing open callback still calls strict load_document through Runtime.open_saved; this does not promise malformed documents become loadable. IdentityUnavailable explicitly retains the row with no fabricated ID. Proposed UI disposition is a disabled/non-actionable fallback entry; that unsupported-identity edge behavior is part of the explicit decision, not claimed as demonstrated React parity.

No full raw JS record crosses the public API. JsValue.clone clones its handle for the decode attempt; it does not clone the full JS object. Successful documents incur one Box allocation and are moved into the result. Fallback metadata is limited to two optional strings. New return values are immutable discovery projections, not another writable document store.

## Executed checks

- `git apply --check /tmp/frontend-run/unapplied-tolerant-listing.patch`: exit 0 against cb8203fc; checked applicability only, NOT applied.
- `rustfmt --edition 2024 --check /tmp/frontend-run/unapplied-tolerant-listing-storage.rs`: exit 0; formatted proposal only.
- Source inspection: unchanged existing method bodies and one reused readonly list_values transaction.
- No cargo, build, compiler, Clippy, native/WASM tests or application browser execution of proposed code. No shared build directory touched. No claim that boxing proves all Clippy requirements.

Initial generation failed an assertion because an insertion marker matched both a method and free function. It produced no patch and touched no repository source. The generator was corrected to a newline-anchored private free function marker; the successful checks above apply to the resulting artifact.

## Required after explicit approval

Before applying the fix, retain a public failing mixed-record discovery repro through the current app. After applying the provider addition AND private Library caller: test healthy + malformed identifiable + healthy, missing/nonstring/empty exact-ID cases, malformed names, typed unsafe preview geometry, list transaction failure, obsolete list/unmount, and strict-open failure preserving the active project. Confirm existing list_documents still fails on malformed typed records for its existing callers. Confirm stored values/revision/history/selection are unchanged by listing/retry. Run affected established formatting/lint/WASM/native/public checks and a DIFFERENT independent reviewer. See tolerant-listing-api-proposal.md for the full bounded acceptance matrix.
