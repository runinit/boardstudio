# Bounded proposal: tolerant saved-project discovery

Status: UNAPPROVED additive API proposal, no repository edits. Baseline reviewed: 5e5d3370 (host source 598b2c). Scope: BrowserStore read-only discovery plus private Library projection. No provider rewrite, schema change, migration, second database/store, new Session, or widened existing member visibility.

## Problem and source oracle

React storage.ts:45–60 reads all records without typed deserialization; ProjectLibrary.tsx:60–71 isolates exceptions in each preview, preserving healthy records and opening identified damaged records through the normal open callback. BrowserStore::list_documents (web/src/host/storage.rs:203–225) collects typed deserialization Results, so one record with a valid id/name but missing parts rejects discovery globally. Its private database_name/list_values are inaccessible to page-binary wrappers. An aggregate PersistError contains neither recoverable IDs nor healthy values.

Typed ProjectDoc geometry errors remain presentation-only. They do not require this API. The new boundary is only to preserve identity and discovery when full typed decoding fails.

## Smallest explicit public addition

Keep list_documents() and load_document() unchanged. Add one exported discovery type and one BrowserStore method in the existing host library, re-exported alongside BrowserStore as needed:

```rust
pub enum SavedDocumentEntry {
    Document(ProjectDoc),
    PreviewUnavailable { id: String, name: Option<String> },
    IdentityUnavailable { name: Option<String> },
}

impl BrowserStore {
    pub async fn list_document_entries(
        &self,
    ) -> Result<Vec<SavedDocumentEntry>, PersistError>;
}
```

The third variant makes genuinely unusable identities explicit instead of fabricating IDs or silently dropping records. It is included in this approval request; if the owner prefers a different treatment, settle that exact behavior before code. A raw schema can contain a numeric IndexedDB key despite the normal ProjectDoc string-id format. The existing open_saved(String) cannot faithfully open an arbitrary numeric key; converting it to a string changes lookup meaning. Expanding open to arbitrary IDB keys is out of scope.

Implementation stays beside list_documents and reuses the existing private list_values and database_name. One read-only list transaction and one linear pass, no per-card database loads. For each record: attempt current serde_wasm_bindgen typed decode; on success return Document without projecting geometry at the provider; on failure extract only id/name via safe string field access. A nonblank string id yields PreviewUnavailable. A missing/nonstring/empty unusable id yields IdentityUnavailable. Optional malformed name yields None; preserve a nonblank string's original whitespace and let private presentation apply the reference blank-name fallback. A field-access failure is a record classification, not a dropped row. Transaction/open/request errors still return PersistError for the whole list and drive Try again. Existing write/strict-open behavior is untouched. No arbitrary raw JS values or full malformed JSON escape the provider.

Caller: Library handles Document with its pure thumbnail projection and name ordering, PreviewUnavailable with the existing Preview unavailable / Open to check this keyboard card and existing runtime.open_saved(id), and IdentityUnavailable as the same fallback presentation without an enabled open action (it has no supported project identity). Do not claim that last case matches a supported React open workflow; it is an explicit edge disposition in this proposal. Current-document dedup and stable list keys use usable IDs; unusable-identity entries need request-local stable positional keys only, not invented domain IDs.

## Approval boundary

This adds a Rust host-library public API consumed by the page binary. AUTHORITY/ACCEPTANCE/CONSTRAINTS preserve explicit public API decisions. Therefore this exact addition and unusable-identity behavior require user approval before applying code. The existing database_name/list_values/private fields stay private. A page-only solution cannot implement this contract with the currently exposed information.

## Checks after approval

1. Real isolated IndexedDB provider: healthy document, valid-id/name record missing parts, second healthy document. All appear; damaged preview falls back; either healthy project opens; trying the damaged one reports normal strict load failure and preserves current usable project.
2. Geometry-only invalid typed document takes the private projection fallback, not malformed-record branch; missing/invalid name falls back without hiding healthy records.
3. Numeric/unusable identity retains explicit disabled discovery entry; no fabricated ID, no string coercion, no attempted destructive write.
4. Provider list failure still shows retry and retains prior cards; late/retried/unmounted list results cannot overwrite newer UI.
5. Existing list_documents remains fail-fast for its existing callers; load_document rejects malformed representation; writes/schema keys unchanged. Browser read-only listing leaves revision/history/selection and stored values unchanged.
6. Native/WASM affected checks, public reference/candidate evidence and independent integrated review. Additive API proposal alone does not satisfy those gates.

RF: extend RF-002 with BrowserStore/page-binary boundary and tolerant discovery example; preserve exact source evidence. No broad architecture redesign is requested.
