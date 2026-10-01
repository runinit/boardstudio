# boardstudio-application

`boardstudio-application` is the headless, consumer-owned owner of editor session ordering. It depends on the existing `boardstudio_core` provider and leaves document/wire types unchanged. The crate owns no Dioxus signal, browser handle, worker, IndexedDB transaction, CAD object, renderer object, URL, or file handle.

## Host loop

Create one `Session` for the application lifetime, read `core_executor_epoch()` when constructing the core worker, and feed user events through `submit`. Execute each returned effect, then return its identified completion through `complete`. The host should publish `read_model()` after each transition. A panel subscription may stop observing without dropping the session.

```rust,ignore
let mut session = Session::new();
let core_epoch = session.core_executor_epoch();
let effects = session.submit(Event::Open { operation_id, document });
// For Effect::Core, move `*request` into the typed worker request call.
// Return Completion::Core with its request_id and executor_epoch.
// For Effect::Persist, commit document and all referenced assets atomically;
// return only on IndexedDB transaction complete or abort.
```

The host dispatches `Effect::Core { request_id, executor_epoch, request: Box<CoreRequest>, .. }` to the long-lived CoreEngine worker. It returns `Completion::Core { request_id, executor_epoch, reply: Box<CoreReply> }`. `Completion::CoreFailed` means the mutation outcome may be unknown; the session emits `RestartCoreExecutor` and blocks dependent work. Reopen only after an explicit `RecoverWithDocument` choice.

`Effect::Persist` carries the exact retained `Arc<ProjectDoc>`, `SaveAttemptId`, operation ID and each document asset's ID/hash/media type. The host resolves those references to original bytes and reports `Committed` only when the underlying transaction completes; request success is not enough. An abort enters `RecoveryRequired`. `RetrySave` emits another persistence effect for the same document and does not issue an engine request.

## Session events

- `Open` queues an ordered provider open; the result is persisted before a new accepted snapshot and session epoch become visible. `Edit`, `Undo` and `Redo` are serialized and likewise publish only after save completion. A regular discrete edit is assigned the latest accepted revision when dequeued. Captured gesture edits retain their start revision.
- `SelectParts`, `Navigate` and `SetCamera` update session-only read-model state; they do not write a document or storage. `SelectionMode` supports replace, add, toggle and ordered range selection.
- `GestureBegin` captures pointer, target IDs, transaction, starting positions, pitch, grid snap fraction, geometry-snap and gap policy, and Alt state. Host converts DOM samples to world positions and sends `GestureSample`; one `RequestFrame` is emitted for the pending sample batch and the host returns `GestureFrame`. The session applies grid and physical envelope snapping, coalesces previews through Core, consumes the final pointer-up sample and sends one commit. `GestureCancel` releases capture and makes late replies inert. Host executes the returned capture/release/frame effects.
- `StartGeneration` and `StartExport` are ordered behind pending document work. They capture an accepted-snapshot token and document/board/instance scope. Completions publish or deliver only while that identity remains current. `Navigate`, replacement open, accepted document change and `Close` cancel eligible work.
- `Close` settles after an outstanding known save; a failed save leaves the session in recovery and reports that close is blocked.

## Read model and IDs

`ReadModel.accepted` contains the immutable durable `Arc<ProjectDoc>` and matching committed `Arc<SceneDelta>` with a session-local `SnapshotToken`. `display_preview` is transient and never becomes accepted state. The token changes on replacement open even when document ID and revision repeat; consumers must use it with document, session epoch, board and instance scope instead of comparing revision alone.

IDs in events/effects are outside `CoreRequest`, `CoreReply`, `ProjectDoc` and persisted schema. `RequestId` identifies one core call, `ExecutorEpoch` fences a worker lifetime, `SaveAttemptId` identifies one atomic write attempt, `OperationId` identifies the caller, `JobId` identifies replaceable generation and `SnapshotToken` fences read/export scope. Every caller operation has at most one `Effect::Settled` terminal outcome.

## Verification and current limits

Native public-session tests use a real `CoreEngine` with explicitly controlled effect completions. Run:

```sh
cargo test --manifest-path application/Cargo.toml --locked
cargo fmt --manifest-path application/Cargo.toml --check
cargo clippy --manifest-path application/Cargo.toml --locked --all-targets -- -D warnings
```

These commands prove headless behavior only. Browser worker transport, IndexedDB abort/completion, archive byte round-trip, offline reopen, root/subpath hosting, Dioxus interaction/accessibility, exact case generation, renderer resources and STEP packaging/delivery still require the web/acceptance tickets. Current geometry snap uses all project parts as candidates when no board membership is available; multi-board visibility filtering and camera-to-world DOM conversion remain host integration work. The session currently exposes center/zoom camera state, not renderer/camera resource ownership. It does not replace React or authorize production cutover.
