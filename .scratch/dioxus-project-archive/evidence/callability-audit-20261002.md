# F2.2 archive capability callability audit — 2026-10-02

## Disposition

Issue 16 cannot start. The public accepted-snapshot/archive-worker/local-asset/download path is callable, but the Rust app does not expose a packaged Ergogen model-byte provider or an `embedUsedModels` value/input. In particular, current ticket 01 incorrectly calls the private bool and a “separately owned” provider seam pre-edit prerequisites even though neither is present as a callable Rust interface. The bool is a reasonable private input for this capability to add; the packaged provider needs a named owner and concrete provider contract/ticket (or a bounded scope assignment) before issue16 can depend on it. No source code or parent graph was changed.

## Proven existing path

- `application/src/session.rs:347-351,1765-1780` admits `StartExport` only against a current accepted snapshot/scope and emits `Effect::RunExport` with that snapshot and scope.
- `web/src/runtime.rs:603-640` runs archive packing, rechecks the export scope/cancellation, and stages an artifact for guarded delivery.
- `web/src/runtime.rs:1131-1175` loads current `document.assets` by SHA through `BrowserStore`, serializes the unchanged document, invokes the existing Core archive worker, and extracts archive bytes. It accepts no model resolver or embedding preference.
- `web/src/host/storage.rs:30-34,37-49` exposes `AssetBytes { sha256, bytes }` and an IndexedDB-backed `BrowserStore`; it is a project-local asset store, not a bundled catalogue provider.
- `core/src/model.rs:40-52` already defines `ArchiveRequest::PackProject` with optional `archiveJson`; `core/src/archive.rs:42-72,194-210` already checks archive references and content-addressed paths, verifies each payload digest, and preserves opaque payloads under existing limits. Transport/schema work is unnecessary for `embedUsedModels` metadata.
- `web/src/presentation.rs:371-379` mounts the project dropdown and only renders `Library`; `web/src/presentation.rs:575-591` renders the Export panel with archive and STEP buttons but no embedding checkbox.

## Missing Rust integration

- Repository search finds no `embedUsedModels` identifier in `web/src`. Current Runtime has no such state or parameter: `pack_archive` is called at `web/src/runtime.rs:603-611` and takes only operation ID, accepted snapshot, and scope (`:1131-1136`). Its `archive.json` metadata is omitted from the manually serialized request at `:1159`.
- There is no packaged model catalogue/provider or model files in the Rust `web` tree. The only Rust Ergogen model marker is a diagnostic branch in `web/src/presentation/model_delivery.rs:140-168`: a missing `ergogen:model:*` source is classified as `MissingBundledProvider`; it does not resolve identity, filename, bytes, or source attribution.
- `web/src/presentation/model_delivery.rs:170-200` defines renderer-only `VerifiedModelBytes`; `verify` rejects empty bytes and caps them at 32 MiB, then bytes feed renderer decoding paths. It cannot satisfy archive's opaque-byte contract and is not an archive provider.
- Core already allows archive metadata and applies a 64 MiB per-entry limit (`core/src/archive.rs:10-26`), so archive packing must remain independent of that renderer validator.

## React reference

- `app/src/bundledModels.ts:2-8,10-35` produces catalogue identities from Vite-packaged Ergogen model files and obtains raw bytes from their bundled URL. It exposes no expected catalogue digest.
- `app/src/storage.ts:167-215` takes a private option defaulting true, resolves used-model IDs, adds each bundled model's filename/media type/source and computed SHA-256 to a copy of the project document, always includes local assets, and sends the option in `archiveJson`. Notably, lines `178-180` add `definition.models` for every document definition, regardless of whether it has a placed part; closure parity must retain that behavior.
- `app/src/main.tsx:23-27,45-49,167-174` owns the shared boolean (initially true), includes it in the export snapshot, and passes the value/change callback to Workbench. `app/src/exports/context.ts:10-18` includes it in `ExportSnapshot`.

## Capability gate conclusion

The archive worker, accepted snapshot/scope, local-asset resolver, and guarded delivery are callable. The packaged catalogue byte provider is absent and has no identified Rust owner/interface in this packet; the private bool is absent but can be made a private input by ticket 01 without waiting for the visible F8 checkbox. Therefore issue16 remains blocked. Before dispatch, capability planning must explicitly assign/define the provider prerequisite and move creation of the private boolean input into the implementing capability ticket; F8's checkbox remains paired integration acceptance. Preserve Core's existing opaque-byte archive validation; do not use `VerifiedModelBytes` for archive bytes.

No new refactoring opportunity is recorded here. These gaps are already tracked under existing RF-009 and the F2.2/F8 planning notes; shared ledger ownership remains unchanged.
