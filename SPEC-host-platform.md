# Spec: host-platform milestone contracts

Status: Phase 1 authorized in advance; technical validation required, 2026-10-01.
Module id: `host-platform`; base `96dd51d3`. Continue [the map](CAPABILITY-MAP.md)
and [ADR 0003](docs/adr/0003-rust-application-ownership.md).

## Assumptions and objective

Chromium web/static hosting is the first acceptance target, at root and
`/boardstudio/`, including cached offline reopen. Native/server and broader
browser support are outside M1. Host modules share one web application package
with Dioxus presentation but own distinct APIs/lifetimes. They implement
[session ports](SPEC-editor-session.md), never become another session store.

Provide actual browser executors, storage/assets, DOM/canvas resources, files
and offline/static integration for copied projects in an isolated origin and
database. Rust owns application scheduling/policy; generated upstream bindings
and initialization glue remain permitted. No production schema change is chosen.

## Adapter contracts

| Adapter | Required behavior |
| --- | --- |
| Core executor | One long-lived CoreEngine in a real worker. Initialize WASM before accepting work; typed `handle` inside that worker. Preserve request/reply encoding at transport; envelope supplies session/operation/executor identity. Serialize mutations and settle all callers on init failure, crash, termination or close. |
| CAD / scene executors | Preview CAD, export CAD and heavy scene preparation have independent bounded lifetimes/caches off the page thread. Respect service cancellation/cache-delta protocol; no promise of interrupting synchronous OCCT by future cancellation. |
| Binary transport | Transfer owned buffers when eligible; record sender detachment, receiver ownership and remaining copies/crossings. Do not transfer live buffers still owned by a renderer/service or pointers across WASM memories. |
| Storage | Implement session save-attempt completion/failure against actual IndexedDB transaction completion/abort. Preserve `boardstudio-v2` version 1 stores/keys, active preference, hashes and representation for eventual compatibility; trial uses a distinct namespace/origin. Default serde-to-JS maps/optional values are not automatically storage-compatible. |
| Import/assets | Use existing archive validation and asset hashing/deduplication/atomic import behavior. Validate all required content before publishing imported state; preserve existing asset size limits, absent/null meaning and supported source data. |
| Events/canvas | Normalize input/viewport measurements using session camera state, apply pointer capture/release and RAF/resize/DPR effects. Host owns real DOM elements/listeners/observers/handles; session owns logical gestures. Explicitly remove/dispose on teardown. |
| File delivery | Deliver only the artifact authorized by current session/export guards; manage bytes/object URLs and cleanup. A stale result cannot trigger a download. File format/readiness/filename semantics belong to services. |
| Offline/static | Resolve pinned assets and worker/WASM URLs under both deployment prefixes. Cached reopen requires the necessary cached shell/data/assets; unavailable lazy assets fail honestly. Service-worker policy belongs in Rust; generated JS initialization is distinct from authored JS caching policy. |

Scope/revision checks happen at consumers before publication/delivery, after
each async boundary. Host completion does not silently advance the accepted
document. Core-client transport snapshots cannot stand in for a durable
session checkpoint. Storage failure returns the exact save identity/reason so
session can retry without another engine edit.

## Rationale, tradeoffs and compatibility

Current TS host/session facades mingle browser policy and application authority.
Rust adapters with consumer-owned ports make lifetimes explicit and headless
tests possible, but retain unavoidable JS/WASM and browser crossings. Storage
libraries or direct web-sys calls are adapter choices, not substitutes for
representation/transaction tests. A native host now would widen the platform
scope; fullstack/server functions would break the static-host requirement.

Separate copied-document origins/databases preserve the working reference and
one active writer per document. Browser handles cannot migrate into the engine
or persist in a document. Export/cache algorithms remain with their providers.
Reopening a document to recover ordinary failed saves would lose history and
is intentionally replaced by the approved session recovery contract.

Rust-WASM service-worker initialization/event readiness is still an uncertain
P3 integration. If it needs authored JS policy beyond permitted generated
initialization, report a boundary failure before proposing an exception; do not
silently call maintained JS “generated glue”. No offline proof is claimed here.

## Structure and code style

Propose browser adapters and worker entrypoints in the web package, separate
modules from components. Session imports only its contracts/ports, not host.
Keep bindings versions 0.2.129 / 0.3.106 / 0.6.5 as accepted. No crate or manifest
is created by this spec. Use typed enums and exhaustive matching; a real
[core test boundary](core/tests/core.rs) demonstrates the convention:

```rust
fn preview(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Preview { scene, .. } => scene,
        other => panic!("expected preview: {other:?}"),
    }
}
```

Production adapter failures return explicit outcomes, not test-helper panics or
empty catches. Do not invent new browser APIs from memory; exact web-sys source
and browser transaction/worker standards are linked in ADR 0003.

## Commands and testing strategy

Existing reference commands, with fresh builds required by task gates:

```sh
pnpm run build
pnpm run check:boundaries
pnpm --dir app exec vitest run src/storage.test.ts src/storageReset.test.ts src/CoreClient.test.ts src/CaseClient.test.ts src/ExportClient.test.ts
pnpm run test:e2e
pnpm run test:e2e:dev
pnpm run test:e2e:pages
```

These target React today, not the Rust host. P1 validates real worker/CAD
initialization, root/subpath assets, caller settlement and transfers. P2
validates event/canvas lifecycle. P3 validates real IndexedDB abort/retry,
archive exchange and cached/uncached offline behavior. Fake storage and an
about:blank browser launch do not replace these checks. Preserve frozen resource
budgets and failures; use the actual [constraint tiers](CONSTRAINTS.md).

## Success criteria

- H1: Core/CAD/scene work executes in intended workers; initialization and
  worker failure settle callers with matching identities and no unknown replay.
- H2: Binary ownership/detachment and measured copy/crossing behavior are recorded;
  cancellation/close cannot leave pending callers or corrupt cache bases.
- H3: Transaction completion alone authorizes durable acknowledgment; failed saves
  and retry preserve committed reply/history, stores/keys and value representation.
- H4: Archive/assets round-trip through the reference with faithful hashes/data
  and failed validation/import does not publish partial state.
- H5: Canvas/event/frame/URL resources are released and late effects cannot
  publish or deliver into a replaced/disposed scope.
- H6: Root/subpath production assets and cached offline reopen work in Chromium;
  absent lazy assets and failed service-worker initialization remain explicit.
- H7: Rust owns application policy, with upstream/generated dependencies and
  temporary bridges inventoried honestly; no second active document writer.

## Boundaries and open validation

Always implement consumer contracts and preserve protected data/worktrees.
Ask before schema/public API changes or JS-policy exceptions. Never acknowledge
request success as save completion, mutate a session through transport snapshots,
delete the React reference before parity gates, or promote prototype code.

Actual worker/CAD/canvas/storage/offline integration is unproven until P1–P3.
No browser support beyond the approved target is inferred from dated examples.
Any temporary adapter needs owner/removal/exit evidence before introduction.
