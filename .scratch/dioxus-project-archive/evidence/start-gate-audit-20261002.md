# Bundle-aware archive capability start-gate audit — 2026-10-02

## Decision

Issue 16 is **not dispatch-ready**. The current Rust/Dioxus code proves its accepted-snapshot/local-asset/archive-worker/delivery path, but the packaged model-byte provider seam and shared embedding-setting input do not exist in the Rust page integration. The packaged-byte provider is a separately owned prerequisite seam consumed by capability 01; it is not part of that ticket’s closure/option adapter implementation. A private boolean input permits adapter work before the visible F8 checkbox is integrated; the checkbox is paired integration acceptance. Per the task direction, no source implementation or workaround was attempted. Draft capability 01 was created to cover the missing private pack capability; independent Spec and Standards reviews are pending for the amended packet.

## Exact planning artifacts

- F2.2 parent: `.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`, SHA-256 `c344151f4c4b69bb305032a01e9fc3d1096ba076ac408fca074524891590709d`.
- F8 parent: `.scratch/dioxus-frontend-v1/issues/08-export.md`, SHA-256 `22bec97ff2c6895b037f4385ead2a2e99ae2b9cac9ceaeed0ddfbff3ab109b15`.
- Issue16 before its blocker-gate amendment was spec/standards reviewed at SHA-256 `4391010a63ea561416dd71c038e3e6960f5bcc106b5353344d5ffe84fe45203d`. Its amended body is pending fresh independent review.
- Capability spec: `.scratch/dioxus-project-archive/spec.md`, SHA-256 `97578fe347c706cf6804cf03295ce58b035d2b691b7753562b412cf2a5c37daa`.
- Capability ticket: `.scratch/dioxus-project-archive/drafts/01-bundle-aware-portable-pack.md`, SHA-256 `f20181b39d1bf8da633b7174517868114920e25bb9d94d7778cbfa7cf82fa331`.
- Existing issue15 focus discrepancy remains at SHA-256 `28d421c1839c87591b46066bee3ef46b7d204a01b4f7c67b13d51b7d9ee0f06a`; no change in this archive work.

## Current Rust capability proof

The existing path is useful and sufficient to avoid a second archive engine:

- `application::AcceptedSnapshot`, `Session::StartExport`, scope/revision token checks, `Effect::RunExport`, and guarded artifact delivery provide current accepted snapshot identity and stale/cancel protection (`application/src/session.rs`; `web/src/runtime.rs` around the StartExport/RunExport lifecycle).
- The Project menu is mounted as `details.m1-project-menu` in `web/src/presentation.rs` and renders `Library`; a portable-copy action is absent and is owned by issue16.
- `Runtime::pack_archive` loads each current `document.assets` byte buffer via `BrowserStore::load_asset`, verifies storage SHA identity, passes `pack-project` to `CoreWorker::archive`, and returns bytes through the existing export artifact/download path (`web/src/runtime.rs`; `web/src/host/storage.rs`). This includes already-owned local assets.
- `ArchiveRequest::PackProject` already accepts optional `archiveJson`; Core archive packing verifies project asset references, paths, digests, and configured limits (`core/src/model.rs`; `core/src/archive.rs`). No archive schema or public API extension is justified for the option metadata.

The missing behavior is also concrete:

- Rust `Runtime::pack_archive` serializes the unchanged accepted document, supplies only existing `document.assets`, omits `archiveJson`, and has no `embedUsedModels` input. `Effect::RunExport` assigns the constant filename `keyboard.boardstudio`.
- The current Rust page has no shared `embedUsedModels` state/control or packaged archive byte-provider seam. F8 remains owner of the visible Export checkbox; a private boolean input is sufficient for capability-01 adapter work, and the checkbox is paired integration acceptance. F2 issue16 consumes the same setting for the Project-menu action.
- `web/src/presentation/model_delivery.rs::select_model_asset` can classify document-owned model bytes and returns `MissingBundledProvider` for a missing `ergogen:model:*` ID. It is a renderer selection seam, not a source/catalogue byte provider for portable archives.
- Rust source search found no `modelAssetIds` or packaged `bundledModelBytes` implementation in the `web` page. Existing `BrowserStore::load_asset` only reads project-owned bytes keyed by SHA. Thus the accepted-snapshot and local-asset gates pass; bundled model closure/bytes and the shared-setting input are not callable.

## Reference behavior and parent ownership

Pinned React `5a472a9426e6e38993361da402cd4ec730feb369` provides the oracle:

- `app/src/storage.ts::packProject` always includes local document assets; with `embedUsedModels=true` it collects referenced models from used Ergogen instances, definitions, modules/circuit definitions, assemblies including effective member overrides, and board references; it adds their asset records/bytes only to the packed document; then sends `archiveJson` with the boolean. False omits newly resolved bundled catalogue assets only.
- `app/src/bundledModels.ts` resolves catalogue IDs and obtains bundled bytes from statically packaged model assets; runtime operation does not fetch arbitrary Internet URLs.
- `app/src/exports/context.ts` carries the same `embedUsedModels` setting as the current snapshot/context; `app/src/main.tsx` initializes it true and supplies both value/change callback to Workbench.
- `app/src/exports/documents.ts` names the result `${document.name}.boardstudio`; `app/src/createProjectExporter.ts` rechecks current context before delivery.
- F2.2 assigns portable archive behavior to F2; F8 assigns Export route checkbox/control and provider coordination to F8. This ticket and issue16 add no top-level task graph edges. F2.2's F2.1 start and INT.2 acceptance join and F8.6→F2.2 remain unchanged.

## Verification and remaining gates

The user-confirmed TDD/acceptance seam is the visible archive-save journey with isolated archive round-trip and paired React/Dioxus evidence. True and false settings must both be exercised; local assets must survive both. Ticket 01 also requires stale/failure/no-download and offline checks. Model bytes remain opaque: do not add empty/invalid-format content rejection beyond existing provider availability/read, independently declared expected-digest checks (if present), archive path, and Core size validation. React's catalogue supplies no expected digest; compute a digest for the packed asset record/path and rely on Core to check that path against payload bytes. Do not route this archive path through the renderer's stricter model-byte validator. This planning pass ran no build, test, archive generation, browser action, or project-store mutation. `git diff --check` passed after drafting.

Issue16's Project-menu action is still missing, but adding it is in issue16's scope and is not an upstream provider blocker. Issue16 remains blocked on the new capability 01 and a callable shared preference value from the existing Export composition boundary. The Export route does not need full F8 acceptance to expose that value; no F8 parent completion edge is added. No refactor ledger entry or new RF identity was made: this gap is already described by parent F2.2/F8 and existing RF-009; report no new post-port takeaway unless implementation uncovers a distinct design issue.
