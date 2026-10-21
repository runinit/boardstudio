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

- **Rust conventions:** audit six production `unwrap` sites (`find`-by-id in `core/src/inputs.rs`, `core/src/modules/circuit.rs`, `core/src/mechanical/gasket.rs`; `accepted.unwrap()` in `application/src/session.rs`) for panics reachable from real documents, and drop 17 dead `#[allow(unused_imports)]` re-exports. Measurements, rejected advice and ordering are in the [Rust conventions review](investigations/rust-conventions-review.md). Reproduce before repairing.
- **Undo/Redo:** investigate outstanding history behavior separately. Existing controls and tests remain; earlier qualification was deferred.
- **Generator errors:** replace raw JavaScript stack text in Parts validation with an actionable message (`web/src/presentation/parts/`).
- **Feature ownership:** Editor now composes private workspace handles, and panels share keyed pending-edit settlement. The next ownership review should examine preview pipelines and project lifecycle in `web/crates/runtime/src/runtime.rs`. Keep domain projections and request metadata with their callers; avoid a general framework rewrite. See the [module-deepening cleanup](plans/module-deepening/issues/13-cleanup.md#outcome).
- **Geometry ownership:** consolidate frontend geometric planning and explicit canonical/physical-instance/sample scopes where ambiguity causes a reproduced defect.
- **Host interfaces:** review exposed internal browser types, reflective renderer calls and the single-observer Runtime contract when extending those interfaces.
- **Asynchronous operations:** distinguish worker cancellation from kernel cancellation; make provider-failure tests deterministic and preserve scope checks on delivery.
- **CAD transport:** the JavaScript revision envelope has a safe-integer ceiling. Preserve rejection above the ceiling; revisit representation if requirements change.
- **Archives:** clarify ownership between archive packing, options and asset resolution before extending export formats.
- **Identity:** review persistent outline ID generation and accepted-versus-provisional Case scene identities when changing these workflows; collision and stale-action guards already exist.
- **Outline edit ownership:** from the typed-core-edits outline phase reviews ([ticket 15](plans/typed-core-edits/issues/15-outline-integrated-verification.md#outcome)). (1) Core reissues removed feature, connection and protected-gap ids, so a queued edit could reach a recreated one; only version ids are reserved. (2) The web adapter still picks the version id and default name for Copy and Refine and the connection id (`web/crates/layout/src/outline/adapter.rs`); Core validates them. (3) Dispatch-time capture of connection anchors and `Copy`/`Refine` ids happens when the action is built, not when the queued edit runs. (4) Core keeps two lifecycle implementations (`outline_versions::apply` copy/select/rename/remove and `outline_edit` copy/activate/rename/delete) and two identity-namespace walks (`persisted_identities`, `identity_namespace`); the legacy operations have no production producer. (5) `outline_edit` repeats the contour re-id loop and version push; `SettingsField` mirrors eight `OutlineChange` variants. (6) `Admission::Malformed` evaluates the edit twice and the second path ends in `unreachable!` (`outline/hook.rs`). (7) Version-name length is checked in bytes in Core and was UTF-16 units in the old web check. (8) The inspector label "Linked refinement" and the Refine/Freeze actions are not in `GLOSSARY.md`. (9) `outline/hook.rs` builds the linked finding id `board:{board}:outline:linked:{version}` itself instead of reading it from Core. Reproduce each before repairing.
- **Catalogue:** keep imported project definitions separate from reusable library choices as catalogue features expand.
- **Testing:** panel native tests now drive the real Runtime/Session/Core with shared gates instead of duplicated presentation drivers. Preserve mounted WASM interface coverage and deterministic provider-failure fixtures when extending it. The complete native and browser gates pass after the CAD fixture and runner/environment repairs; future native Runtime decomposition must retain that shared test contract.
- **Controls:** consolidate selection liveness, pointer ownership, keyboard scope and dynamic-select handling only where future changes demonstrate repeated policy.
- **Mechanical settings:** align family defaults, target eligibility and authored-body readiness through existing owners; bounded functional repairs are already present.
React-only findings and migration-accounting proposals are retired with their code.
Mobile, accessibility, unfinished VIK features and visual redesign are not requirements
of this cleanup. Reintroducing them requires a separate product decision.
