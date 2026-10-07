# Product backlog

These are deferred issues and design risks, not prerequisites for ordinary development.
Historical investigation is available in Git at cutover commit `323967ff` under
`.scratch/dioxus-frontend-v1/refactor-findings.json`. Reproduce a current issue before
implementing a repair; historical symptoms may already have bounded fixes.

For new work, record the observed problem, affected behavior and completion
condition. Update the entry when the behavior is verified. Ordinary development
uses the commands in the root README.

Layout queued coordinate edits are resolved by the
[edit-settlement tracer bullet](plans/edit-settlement/issues/06-layout-inspector-tracer-bullet.md)
and subsequent cluster migrations; they are no longer deferred work.

- **Stale edit previews:** an `Event::Edit` preview queued behind a commit is rebased by Session but keeps its old payload, so it is drawn over the newly accepted document until the next commit or a matching clear (`application/src/session.rs`, preview handling). It never changes the accepted document. Affects the old position Inspector, transform drag and outline perimeter previews. The [edit settlement plan](plans/edit-settlement/map.md) leaves previews unchanged; reproduce before repairing.

- **Rust conventions:** audit six production `unwrap` sites (`find`-by-id in `core/src/inputs.rs`, `core/src/modules/circuit.rs`, `core/src/mechanical/gasket.rs`; `accepted.unwrap()` in `application/src/session.rs`) for panics reachable from real documents, and drop 17 dead `#[allow(unused_imports)]` re-exports. Measurements, rejected advice and ordering are in the [Rust conventions review](investigations/rust-conventions-review.md). Reproduce before repairing.
- **Undo/Redo:** investigate outstanding history behavior separately. Existing controls and tests remain; earlier qualification was deferred.
- **Generator errors:** replace raw JavaScript stack text in Parts validation with an actionable message (`web/src/presentation/parts/`).
- **Feature ownership:** reduce shared composition changes across `web/src/presentation.rs` and `web/src/runtime.rs` when concrete feature work exposes a useful boundary. Avoid a general framework rewrite.
- **Geometry ownership:** consolidate frontend geometric planning and explicit canonical/physical-instance/sample scopes where ambiguity causes a reproduced defect.
- **Host interfaces:** review exposed internal browser types, reflective renderer calls and the single-observer Runtime contract when extending those interfaces.
- **Asynchronous operations:** distinguish worker cancellation from kernel cancellation; make provider-failure tests deterministic and preserve scope checks on delivery.
- **CAD transport:** the JavaScript revision envelope has a safe-integer ceiling. Preserve rejection above the ceiling; revisit representation if requirements change.
- **Archives:** clarify ownership between archive packing, options and asset resolution before extending export formats.
- **Identity:** review persistent outline ID generation and accepted-versus-provisional Case scene identities when changing these workflows; collision and stale-action guards already exist.
- **Catalogue:** keep imported project definitions separate from reusable library choices as catalogue features expand.
- **Testing:** reduce the native test harness's duplicated presentation module declarations without losing mounted WASM coverage.
- **Controls:** consolidate selection liveness, pointer ownership, keyboard scope and dynamic-select handling only where future changes demonstrate repeated policy.
- **Mechanical settings:** align family defaults, target eligibility and authored-body readiness through existing owners; bounded functional repairs are already present.

React-only findings and migration-accounting proposals are retired with their code.
Mobile, accessibility, unfinished VIK features and visual redesign are not requirements
of this cleanup. Reintroducing them requires a separate product decision.
