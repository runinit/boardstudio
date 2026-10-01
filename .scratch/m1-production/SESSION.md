# Maintained M1 session design

`application/src/session.rs` is the headless authority. `Session::submit` accepts
identified intents; `Session::complete` accepts identified host completions.
`ReadModel` publishes accepted document/scene snapshots through shared immutable
ownership. Presentation drafts and browser handles do not live in that crate.

## Durable transitions

Open/edit/Undo/Redo are serialized through the real public CoreEngine. A core
commit creates a retained pending snapshot and emits an identified persistence
effect. Only transaction completion publishes that snapshot as accepted. Abort
retains the same engine commit for save-only retry; retry cannot add history.
Dependent work remains ordered behind durability. Unknown executor failure
requires explicit recovery rather than replaying a possibly committed command.
Recovery can discard failed retained work and reopen the supplied durable copy;
late executor-restart completions cannot replace a newer executor epoch.

The host preserves exact core integers and floats in JSON text frames. Worker
IDs and executor epochs are correlated independently of domain payloads. Each
save attempt has its own identity; stale completions cannot acknowledge a newer
write. IndexedDB projects and all required assets commit in one transaction.

## Interaction and presentation

Captured gestures retain their pointer, targets, transaction and starting
positions. Coalesced frames create display previews; the final release sample
commits once. Escape/cancellation restores accepted geometry without history or
persistence. Active-board filtering prevents snapping against parts on another
board. Dioxus form text, DOM coordinate conversion and viewport effects feed
these public session events. Selection, navigation and camera are read-model
state rather than additional writable document stores.

## Jobs and exports

Session epoch and accepted-snapshot token distinguish equal-ID/equal-revision
reopens. CAD and export scopes also capture document, board and physical
instance. Document work, navigation, cancellation and close invalidate relevant
jobs. Current generation distinguishes preparing/running, preview/exact,
blocked/failed/cancelled and replaced results.

The web Runtime prepares captured input through public core requests. CAD runs
in a separate worker/WASM memory; its persistent preview cache is scoped by
session/document/board/instance and keyed by body geometry rather than revision.
Exact assembly uses the existing public kernel provider. Last geometry can
remain visible after a same-scope edit only with a stale label. Scope changes
release the old canvas and worker rather than displaying the previous board.

STEP export uses an independent worker and prepared committed input. It checks
scope/token after preparation, readiness and generation, and again immediately
before file delivery. Rendered meshes are never STEP authority. Export workers,
transient artifacts and object URLs have explicit disposal. CAD rejects revision
values above the JavaScript safe integer range before the provider boundary.

## Evidence and limits

Thirteen public native session tests exercise the real CoreEngine and controlled effect
completions, including abort/retry, explicit recovery, stale identities, gesture
history and board-scoped snapping. Twelve native web tests validate captured preparation,
instance reflection, operation/identity/payload rejection and safe-range limits. Exact assembly results may omit bounds because the public CAD provider emits none; STEP readback still requires valid bounds. A failed export removes its current registration before caller settlement, so later reopen cannot cancel an already settled export.
See [session evidence](evidence/session/implementation.md),
[CAD evidence](evidence/cad-jobs/verification.md) and the
[acceptance ledger](ACCEPTANCE.md). Native coverage does not substitute for the
current browser, exact geometry, resource, accessibility or performance gates.
