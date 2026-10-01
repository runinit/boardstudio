# Spec: editor-session for the representative layout/case milestone

Status: **Phase 1 proposed for review**, 2026-10-01. Module id: `editor-session`
in the [approved capability map](CAPABILITY-MAP.md). Scope approval accepts
[ADR 0003](docs/adr/0003-rust-application-ownership.md); it does not approve
this specification, complete a runtime probe or authorize production cutover.

Source reference: the isolated assessment at `96dd51d3e790c28f5554a8c9888a147c8814e2a7`.
Reconcile protected concurrent main-checkout work before implementation; this
document does not silently import that overlay into the reference.

## Assumptions and objective

Use the approved Chromium/static-web trial, isolated storage and copied
REVIUNG41/Sofle fixtures. Consume existing public engine and service contracts;
do not replace the engine, define a new document format or widen private CAD
and renderer APIs. The session runs on the page thread and outlives individual
panels. Dioxus subscribes to its read models; browser handles live in host
adapters. A headless Rust transition/effect model is proposed below, not created.

Build one authority for editor command ordering and interaction lifetimes so
an editor can preview a gesture, commit once, save, Undo/Redo and export the
accepted revision without scope races or an engine/UI durability mismatch.
This specification covers the session portion of the representative milestone:
open, select, drag/numeric edit, save recovery, history, archive/reload,
case-setting changes, exact-generation coordination and committed STEP export.

Geometry, snapping/domain validity, scene encoding, CAD construction/cache
algorithms, artifact formats, storage schema, worker bootstrap, offline policy
and Dioxus components remain with their approved owners. Other workspaces,
generator authoring, PCB/firmware implementation and complete frontend parity
are outside this module's acceptance. Preserve their data when opening/saving
supported fixtures; omission does not establish full-Rust completion.

## Observed behavior and proposed changes

The existing [session coordinator](app/src/useProjectSession.ts) saves a
committed reply before publishing it, but resumes queued work after save
failure. The engine can then be ahead of the accepted UI snapshot.
[CoreEngine](core/src/lib.rs) adds history for each successful commit;
`transaction_id` does not deduplicate or merge repeated commits.
[Canvas interaction tests](app/src/ui/createCanvasInteractions.test.ts) capture
gesture ownership and cancellation, while [export tests](app/src/createProjectExporter.test.ts)
reject changed scopes even when document IDs and revisions match.

ADR 0003 supplies the architectural rationale and approved save-recovery
change. These additional choices are proposed for this module:

| Choice and observed problem | Improvement | Alternatives/tradeoffs | Preserved contracts / intentional changes | Evidence required |
| --- | --- | --- | --- | --- |
| Ordering is distributed across React refs, promises and component lifetimes | A headless session consumes events and emits typed effects; host completion events return through the same serialized mailbox | A Rust async coordinator with injected ports is viable but harder to test at every interleaving; Dioxus-owned coordination retains component lifetime coupling. Explicit effects add protocol types | Preserve serialized mutations, save-before-publication and one authority; change the internal coordination model | Native tests with a scripted executor/storage adapter; no framework needed |
| JS export guards currently depend on document/scene object identity | Issue an immutable, session-local accepted-snapshot token with each published document/scene pair | Rust pointer identity is possible but brittle across adapters; revision alone misses reopen/same-revision replacement. Tokens add metadata | Preserve scope/identity rejection; tokens are not persisted and do not alter provider encodings | Same document ID/revision reopened in another session; copied read models cannot gain current-snapshot identity |
| Save failure leaves queued intentions ambiguous | Enter recovery, retain the completed commit, and settle queued dependent intents as blocked; new mutations/exports are rejected until recovery completes | Parking and replaying intents after retry retains clicks but risks executing work the user considered failed. Retry targets the retained save only | Approved recovery gating is retained; explicit terminal blocked outcomes and reissuing later intents are new session behavior proposed here | Save-abort/retry tests count engine commits/history entries and caller settlements |

## State and ownership

The engine owns document/history/geometry. The session owns accepted document
and committed scene, durable acknowledgment, logical selection, active
board/instance, 2D camera/navigation, gestures, queue and job identity. Read
models carry immutable references or owned snapshots; callers cannot mutate
them. Presentation owns form text/focus/panel state. A form's domain preview
transaction belongs to the session. Renderer-owned 3D camera/GPU state and
host-owned DOM/worker/storage handles never enter this model.

Reconcile logical selection and active board/instance with each accepted
document, preserving existing pruning/fallback behavior. Camera and selection
changes alone do not become durable document edits or storage writes.

| Session state | Meaning and permitted progress |
| --- | --- |
| Empty / Opening | No accepted document, or an ordered open is resolving. Validate/load through existing services; publish only after required save completion. An open rejection preserves the last accepted snapshot where one exists. |
| Ready | Engine and accepted snapshot agree. Preview and ordered commands may begin. |
| Applying | One engine mutation/open/history request is outstanding. Other ordered intentions wait; selection/navigation may update without losing the request's document identity. |
| Saving | Retain the completed reply as pending; await storage transaction completion. The previous accepted snapshot remains the durable read model. |
| RecoveryRequired | Save failed, or a core failure made mutation outcome uncertain. Block dependent operations; expose reason, last accepted snapshot and any known pending reply. Retry a known save or perform an explicit recovery choice. |
| Closing / Closed | Stop new work; cancel replaceable jobs and settle callers. Graceful close must resolve a known pending save or expose recovery before disposing its owner. Closed accepts no late publication. |

Preview/generation status is separate from these states. Required, preparing,
running, ready, blocked, failed and cancelled remain distinct, preserving
[generation status meaning](app/src/generationState.ts). An old exact mesh may
remain visible with truthful stale/paused status; it cannot satisfy current
generation or export readiness merely because it is displayed.

## Session interface contracts

Ports below belong to this consumer and are implemented by host composition.
Provider payloads retain their existing types and semantics. These are boundary
contracts, not new executable APIs or a decision to share every type publicly.

| Boundary | Input / output and required behavior |
| --- | --- |
| Commands and completion events | Open copied document, edit intent, gesture begin/sample/end/cancel, Undo/Redo, board/instance navigation, generation, archive/STEP export, retry save and close. Every externally submitted operation has an operation ID and exactly one terminal outcome: completed, rejected, superseded, cancelled, persistence-failed, blocked-by-recovery or executor-failed. Progress is not completion. |
| Engine port | Existing [CoreRequest/CoreReply/EditCommand](core/src/model.rs), through public `CoreEngine::handle`. Envelope adds operation/session/executor identity outside the payload. Accept only the expected reply variant/request ID; preserve domain errors. Do not reinterpret any reply carrying a document as a successful saved scene automatically. |
| Persistence port | Captured document, required asset/import scope and save-attempt identity; completion means the corresponding atomic transaction completed. Failure/abort is explicit. Session holds the pending committed reply until acknowledgment; host owns encoding and transaction handles. Archive/asset semantics remain service-owned. |
| Generation/export ports | Captured accepted document/scene token, board/instance, revision, job generation and provider inputs; progress, result or failure returns with identity. Services retain readiness, exact/provisional fidelity, artifact construction and cache protocols. Host delivers final bytes only after session authorization. |
| Interaction host | Normalized pointer ID, button/modifiers, samples and viewport metrics; capture/release and frame requests are effects. Host performs DOM measurement/event conversion using session camera state. Session owns logical gesture/policy; host owns actual pointer capture and scheduled-frame handles. |
| Read model subscription | Accepted snapshot token/document/scene, display preview, selection/camera, save/recovery and generation status. Dioxus may derive presentation; it does not write session state through a signal. Panel unmount unsubscribes without disposing the application session. |

Use a session epoch that changes on successful document open/replacement,
including reopening the same ID/revision. Maintain separate executor epochs
for core, preview CAD, export CAD and scene preparation. Preview/job identity
also includes board/instance, base revision, accepted-snapshot token and a
monotonic gesture/draft/job generation where applicable. Identifiers are
session/transport metadata, never new durable document fields.

### Ordered commands and durable acceptance

1. Capture the command's intended document/session scope when submitted.
   At dequeue, reject a changed scope. Ordinary discrete edits use the latest
   accepted revision; captured gestures and replace-document intents retain
   their captured revision and must fail/cancel if it is no longer applicable.
   Do not rebase an old gesture silently onto new geometry.
2. Serialize engine work and durability transitions. Use existing domain
   validation/readiness rules. A rejected engine edit leaves history and
   accepted state intact and settles the caller with the existing reason.
3. Retain a successful mutation reply, request its save, and publish the new
   document/committed scene/token only after matching save completion. Undo
   and Redo use the same ordering. A request-level IndexedDB success is not
   transaction completion; preserve the current browser-local durability
   contract without promising physical-media flush.
4. On save failure, retain that exact reply and operation identity, enter
   recovery and settle the original caller as persistence-failed. Settle
   dependent queued intentions as blocked. Retry creates another save attempt
   for the same document, never another engine edit, Undo or `Open` request.
   Successful retry publishes once and returns Ready if the engine is healthy.
5. Project switch/reload and graceful close cannot discard a known pending
   commit. Recovery abandonment/reopen requires an explicit user choice that
   states unsaved work/history consequences. A core crash may lose Undo;
   unknown mutation outcomes are not automatically replayed. Host storage
   recovery and a fresh engine must reconcile before mutations resume.

A board/instance or panel change invalidates presentation jobs, not a completed
durable edit. A mutation already dispatched must finish saving to its captured
document; never drop its reply using a transient current-board check. Document
switches are ordered behind that resolution so an old commit cannot publish
into a new session. Opening the already active saved project remains a no-op
where [existing action behavior](app/src/createProjectActions.test.ts) preserves
history. Do not introduce routine engine reopening as a save-retry mechanism.

### Gestures and preview

Capture pointer ID, gesture generation, transaction ID, intended scope/base
revision, target IDs and starting domain values at begin. Rendering/rerendering
cannot replace these captured values. Preserve existing selection, modifier,
snap/Alt, threshold, locked-part and coordinate behavior through baseline
characterization; this spec does not choose new tolerances or gesture UX.

Preview commands use `EditPhase::Preview`: no durable document publication,
storage write, revision advance or Undo entry. Coalesce pending preview samples
for the same gesture; settle superseded requests explicitly. Preserve engine
ordering, and never coalesce away the final commit. Pointer-up processes the
final sample even when the last queued frame has not run, then submits exactly
one `EditPhase::Commit` if the gesture changed the domain. Its save completes
before successful gesture completion is reported.

Escape, pointercancel/lost capture, scope replacement or teardown invalidates
the gesture before releasing host capture so late events are inert. Unrelated
pointers cannot update, cancel or replace it. Cancelled uncommitted gestures
restore the captured/accepted display, cancel pending frames and add no history
or save; never replay a restoration preview into a newly opened document.
Numeric domain previews follow the same captured transaction rules; text
formatting/focus drafts stay in presentation. A conflicting accepted mutation
invalidates an uncommitted gesture instead of rewriting its base revision.

### Jobs, cancellation and scene publication

Session requests generation from a captured, eligible snapshot and assigns job
identity. Provider/service readiness and fidelity remain authoritative. After
each async completion, verify executor epoch, session, scope, snapshot/revision
and job generation before publishing progress or scenes. Revision equality
alone is insufficient. Preserve the existing
[case-preview/fingerprint rules](app/src/casePreviewContext.ts) for reuse;
this spec does not invent a new geometry cache key.

Supersession/cancellation first removes display eligibility and settles the
caller, then asks the host/provider to cancel. Synchronous OCCT computation may
finish before a cooperative boundary. A valid raced body/cache delta must be
consumed according to its provider protocol before stale display is discarded;
session cancellation must not corrupt the next job's cache. Worker termination
is an executor failure, with separate cleanup and caller settlement, not proof
of cheap mid-operation cancellation. Replacement jobs remain bounded according
to existing worker/cache policy; no unbounded backlog of superseded samples.

Session supplies identity and overlay state to renderer-independent scene
consumers. Providers own full/delta/base identity, body IDs, units, transforms,
materials, readiness and fidelity; adapters must reject missing delta bases.
Renderer handles, browser objects and CAD objects stay outside session state.
Engine provisional scenes, current exact CAD and old retained geometry cannot
be conflated into one export-ready flag.

### Export coordination

Capture requested scope at intent submission; capture the accepted snapshot
when the ordered export begins after preceding saves. Reject recovery or an
unresolved committed scene. Use a snapshot token as the Rust equivalent of
[ExportContext's identity guard](app/src/exports/context.ts), checking document,
session, board/instance and revision through preparation and just before file
delivery. Reopening an identical document/revision invalidates the old export.

Use the existing archive and case/STEP service semantics, asset identities,
readiness and separate export CAD cache. Provisional display meshes cannot
become the authoritative STEP source. Preserve the general accepted-edit
adoption rule: an export may advance its expected snapshot only through its
own validated, durably accepted edit, never an unrelated edit. M1 does not
implement the PCB/electrical workflows that use this mechanism today.

Cancellation, scope change, packaging failure, worker failure or disposal
settles the export and prevents delivery. Host owns byte/URL lifetimes and
file I/O; session grants delivery for the still-current artifact. Successful
archive reload must round-trip via the existing Rust boundary and React
reference; importing a fixture cannot execute or reinterpret stored generator
definitions through this session.

## Project structure and code style

Propose `application/` as the standalone headless Rust crate
`boardstudio-application`, with session, interactions and port-contract modules
under `application/src/` and public behavior tests in `application/tests/`.
These paths do not exist yet. Use existing core/contracts providers; avoid a
root workspace conversion or a crate for each state/gesture. Keep Dioxus,
web-sys/JS values, storage transactions, worker imports and GPU types out of
this crate. It need not impose `Send` on page-thread adapters. The separate
web package supplies adapters later; its specification is not written here.

Use existing Rust naming and formatting, typed enums and exhaustive matching;
tests exercise public request behavior. This is an actual style excerpt from
[core's existing tests](core/tests/core.rs), not prototype session code:

```rust
fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
    match reply {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, document),
        other => panic!("expected scene: {other:?}"),
    }
}
```

Production errors are typed outcomes, not test-helper panics. Preserve domain
error meaning at adapters; report storage/transport errors distinctly. Exact
Rust signatures, reference-counting representation and any new dependency
must be justified in the subsequent bounded plan, with this contract intact.

## Commands and testing strategy

Run existing commands from this worktree; they characterize providers/reference
behavior and do not constitute Dioxus or session acceptance:

```sh
node scripts/repo-check.mjs
git diff --check
cargo test --manifest-path core/Cargo.toml --locked --test core preview_does_not_commit_and_stale_edit_fails -- --exact
pnpm --dir app exec vitest run src/createProjectActions.test.ts src/createProjectExporter.test.ts src/ui/createCanvasInteractions.test.ts src/storage.test.ts src/storageReset.test.ts
pnpm --dir app exec playwright test e2e/canvas-interactions.spec.ts e2e/workbench-selection.spec.ts e2e/project-library.spec.ts e2e/startup-recovery.spec.ts
```

After the proposed crate exists with its own committed lockfile, its native
checks would be:

```sh
cargo test --manifest-path application/Cargo.toml --locked
cargo fmt --manifest-path application/Cargo.toml --check
cargo clippy --manifest-path application/Cargo.toml --locked --all-targets -- -D warnings
```

These prospective commands are unavailable now; do not claim they pass.
Use native Rust tests with a real CoreEngine behind a scripted port, controllable
completion/failure ordering and an effect trace. Assert documents/history and
caller outcomes, not only private enum shape. Tests for the save-failure
correction must first fail for the observed engine/accepted-state drift.
Characterize reference interactions before relocating their ownership.

Host integration needs real workers, IndexedDB transaction abort/completion,
canvas/pointer lifecycle and production archive exchange in Chromium. Existing
Vitest fake storage and dependency resolution cannot prove those behaviors.
Use the map's isolated P1–P3 probes and then a paired M1 workflow; do not promote
probe code automatically. Preserve existing assertions and measured budgets
from [CONSTRAINTS.md](CONSTRAINTS.md); no invented coverage, startup, size or
crossing threshold. Native success alone does not pass browser/parity/resource
gates. Full integration and resource commands remain in constraints.

## Success criteria

| ID | Required observable result | Verification level |
| --- | --- | --- |
| S1 | Two discrete edits, Undo and Redo observe sequential accepted revisions and save completion; queued commands never use an unintended document. Engine/domain rejection does not drift accepted state | Native real-engine/effect trace, then paired browser workflow |
| S2 | Many drag/numeric previews plus a final sample produce one commit, one Undo entry and one save; preview/cancel produces none. Late/unrelated pointer events cannot resurrect a cancelled gesture | Native ownership tests plus Chromium capture/frame test |
| S3 | After an engine commit and failed save, accepted state stays old, pending reply is retained and dependent callers settle blocked. Retry saves that reply once, publishes once and adds no second history entry | Native failure trace and real IndexedDB abort/retry |
| S4 | A board/panel change during save does not drop the committed document. Ordered project switch cannot publish an old commit into the new session; identical ID/revision reopen invalidates transient jobs | Native delayed completions and browser project/board switching |
| S5 | Out-of-order generation/progress and stale executor replies never publish; valid raced cache deltas remain usable. Superseded, cancelled, failed and disposed callers all terminate; frame/worker resources remain bounded | Native scripted job events; P1/P2 and existing cache/soak assertions |
| S6 | Core failure with uncertain commit does not replay the mutation. Known pending-save data survives ordinary recovery; explicit reopen acknowledges lost history. Graceful close cannot silently erase pending work | Native crash/close matrix; real worker failure and lifecycle probe |
| S7 | Archive and STEP export use a durably accepted, eligible snapshot and distinct export cache. Scope/revision change or failure prevents delivery; export-owned accepted-edit adoption cannot adopt unrelated changes | Native export trace; P1/P3 and paired React/Dioxus artifacts |
| S8 | The copied fixtures preserve supported data/assets through save/archive/reload, edits/history and exact case/STEP. Presentation subscribes without owning a second writable session; existing responsive/keyboard behavior survives | Paired M1 browser, archive/geometry/readiness and affected resource gates |

S8 is the integration contribution of this module, not evidence that every
other capability has been implemented. Session acceptance must name which
criteria have native evidence and which still require browser integration;
production adoption is blocked by missing required gates.

## Boundaries and unresolved validation

**Always:** preserve accepted provider contracts, formats, protected work and
source fixtures; characterize moved behavior, test failures/order/lifetimes,
settle every caller and retain failed/unavailable evidence. Separate previews
from durable state and compare using the approved oracle and frozen budgets.

**Ask first:** changing this approved scope, durable schema or contract,
widening existing private APIs, activating new gates, weakening a required
budget/check, discarding work, rewriting history, or publication. Routine
reversible work within the approved documentation scope needs no new approval.

**Never:** create a second writable document/session authority, replay an
uncertain commit, treat an aborted save as accepted, deliver stale/provisional
STEP, hide failures with suppressed/skipped assertions, or promote prototype
code automatically. No migration implementation or whole-migration backlog
is produced in this phase.

The approved web target is Dioxus/CLI 0.7.10 with minimal/web/mounted and the
existing binding pins; the headless crate does not depend on it. Reuse the
[exact-version official-source record](docs/adr/0003-rust-application-ownership.md#versioned-official-source-verification).
In particular, Dioxus task/resource cancellation cannot cancel a dispatched
worker job or acknowledge a save. IndexedDB completion must follow its actual
transaction lifecycle. P0 resolves dependencies online only; its offline cache
failure, untested CLI/packaging/runtime and the
[baseline failed/unavailable checks](docs/dioxus-context-baseline.md#baseline-validation)
remain outstanding.

Before implementation, validate worker/CAD interop, Dioxus/canvas lifecycle and
actual storage/offline behavior through the scoped probes. If a result changes
a boundary, update the relevant decision/spec before planning adoption. The
exact host delivery handshake and stale cache-delta consumption must be proven
against their adapters. Assign any temporary adapter's accountable owner,
removal item and exit evidence before introducing it. No adapter exists here.

Review this module's transition/effect interface, snapshot-token semantics,
terminal recovery outcomes and S1–S8 before Phase 2 planning. Scope approval is
already recorded; this is the next, separate spec-driven-development gate.
