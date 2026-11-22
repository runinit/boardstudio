# boardstudio-application

The headless session owns browser-independent editor state and runs document edits through its
in-process Core engine. The host supplies the Open admission check, dispatches events through
`Session::submit`, and runs the returned effects. Effects that finish asynchronously return a
`Completion` to the same session.

## Session events

- `Open` validates a candidate in a fresh engine. It publishes the accepted snapshot and advances
  the session epoch after the opened document is saved.
- `ResolveEdit`, `Undo`, and `Redo` publish accepted state synchronously. A single in-flight save
  coalesces edits made while it is pending and follows up with the latest accepted revision.
- `Effect::Persist` carries the document and save attempt ID. The host atomically writes the
  document and required assets and returns the transaction result. An aborted write leaves the
  accepted edits available for `RetrySave`. `Close` waits for an in-flight save and stays open
  if saving fails.
- `SelectParts`, `SelectMatrixCell`, `Navigate`, and `SetCamera` change session state without
  persisting a document.
- A gesture captures its start revision, targets, pointer and snap options. Samples coalesce into
  one preview per frame. Pointer-up commits the final position, and cancellation releases the
  pointer and clears the gesture’s preview. `ClearPreview` clears only a matching transaction.
- CAD generation requires a saved accepted snapshot. Exports can capture a ready accepted
  snapshot while a save is pending. Both use identified completions and cancel when their scope
  or source is superseded.

## Read model and IDs

`ReadModel.accepted` holds the immutable document and committed scene. `Durability` records its
persistence status, and `display_preview` holds transient geometry. Consumers can use
`Session::snapshot_is_current` to check the token with the document, epoch, board and instance
scope. Each caller supplies a fresh `OperationId`, and each admitted operation settles at most
once.

## Verification and current limits

Native public-session tests use a real `CoreEngine` with explicitly controlled effect completions. Run:

```sh
cargo test --manifest-path application/Cargo.toml --locked
cargo fmt --manifest-path application/Cargo.toml --check
cargo clippy --manifest-path application/Cargo.toml --locked --all-targets -- -D warnings
```

These commands prove headless behavior only. Browser worker transport, IndexedDB abort/completion, archive byte round-trip, offline reopen, root/subpath hosting, Dioxus interaction/accessibility, exact case generation, renderer resources and STEP packaging/delivery still require the web/acceptance tickets. Current geometry snap uses all project parts as candidates when no board membership is available; multi-board visibility filtering and camera-to-world DOM conversion remain host integration work. The session currently exposes center/zoom camera state, not renderer/camera resource ownership. It does not replace React or authorize production cutover.
