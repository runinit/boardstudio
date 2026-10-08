# 13: Cleanup and record

Status: resolved
Type: task
Blocked by: 07, 08, 09, 10, 12
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md)

## What to do

- Search for leftovers: `rg -n "struct \w*Submission\b|fn owner_is_live|base_revision: 0|transaction_id: String::new\(\)|_capture_is_current" web`.
  Each remaining hit is either deleted or justified in `## Outcome`.
- `docs/architecture.md`: describe the pending-edits module, the native test Runtime
  and the export-lease module where it describes edit tickets and the runtime crate.
- `docs/backlog.md`: update "Testing" (native harness) and "Feature ownership".
- Map: confirm every ticket's Progress line; record what the next review should look
  at (preview pipelines and project lifecycle in `runtime.rs`).
- After the panel migrations integrate, inspect the duplicated Case/Keymap
  OwnedEdits holder and the similar Keycaps owner/binding mechanism. Consolidate
  shared lifecycle mechanics only when the existing helper can absorb them behind
  a small interface; keep domain metadata and projections with their callers.
  Any new or widened public helper API needs approval. Do not add unbind_all or a
  second draft/failure pair type speculatively; FieldView already represents the pair.
  Ticket09's accepted integration retains one nonblocking duplication smell here;
  binding resolver native extraction is a later seam decision. Reuse its complete
  mounted binding coverage when deciding the smallest meaningful extraction.
- Verify the [published helper contract update](16-pending-edit-ui-helpers.md#coordinated-integration-contract-update-2026-10-08)
  against all final consumers: binding epochs, composite unbind behavior, K: Clone
  submission and typed drafts are now documented after integrated checks. Replace
  unexplained owner-generation sentinels only after verifying the owner semantics.
- Keep baseline repairs separate from the panel acceptance fixes: root owns the
  browser-runner diagnosis and complete test-list coverage under one Chrome lease;
  investigate the CAD gasket volume mismatch through its existing domain test.
  Do not weaken the geometry assertion or exclude a mounted test to claim a pass.

## Verification

```sh
python3 scripts/check.py
python3 scripts/check-doc-links.py
```

## Comments

Root claimed cleanup after the final integrated Parts/Layout panel gates passed.
Production build is finishing; full acceptance remains gated on its completion.

## Outcome

Root audited the final consumers and recorded architecture/backlog ownership.
No new public helper interface or visibility widening is needed. Parts/Layout
panel source, shared epoch/typed bindings and submission release are accepted at
`eebc703d`; the final integration record supplies verification and review pins.
The CAD volume repair remains a separate commit (`77fe9a8e9`) and leaves expected
volumes/tolerances and production selection unchanged.

### Leftover search

The required search has 15 hits; each is retained for an explicit reason:

- `ui-shared/src/pending_edit_helpers.rs:114`: private Submission is the single
  module's submitted-value/epoch policy, not a panel's parallel settlement layer.
- `runtime/src/runtime.rs:3509,3524,3636`: two calls and the implementation of
  parts_preview_capture_is_current protect disposable preview delivery. Preview
  pipelines and project lifecycle are the next Runtime review, outside this map.
- `layout/src/mirrored_pair_geometry.rs:179`: base_revision 0 belongs to an isolated
  ProjectMatrices geometry fixture, not product edit admission.
- Empty transaction identities occur only in SceneDelta/test fixtures:
  runtime export_lease:158, case_preview:709, case_gesture_preview:179;
  host cad_jobs:1031,1134,1211; layout outline_lifecycle_tests:29;
  case shared_viewer:3766 and case_generation_admission:42; PCB pcb_scene:385.
  These fixtures do not submit durable edits. There is no remaining fn owner_is_live
  or per-panel Submission holder in the searched web tree.

### Caller ownership and helper contract

Case/Keymap OwnedEdits and Keycaps' owner/binding projection retain owner reset,
request metadata and binding preservation. The existing helper cannot absorb that
policy without a new public owner interface. Keep this nonblocking duplication
with its callers; no speculative unbind_all or second draft/failure pair is added.
FieldView already represents the pair. Binding resolver extraction remains a later
seam decision; complete mounted binding coverage is preserved.

Final consumers bind actual String/typed draft values, submit Clone keys, and
unbind fields and controls independently for composite keys. Binding epochs prevent
writes to replacement children, and terminal submitted values are released. Panel
ordinary dirty state remains distinct from submitted-value settlement. The shared
contract is documented in [the helper Outcome](16-pending-edit-ui-helpers.md#terminal-submission-lifetime-2026-10-08).

Case's owner generation 0 is a constant unused dimension, not an unadopted-owner
sentinel. Controller instance identity, captured Scope and current instance selection
reject departed admission; unmount clears the portal and child bindings. Fresh
mounts get fresh controllers; accepted revisions do not retire the owner. Keycaps'
unadopted OwnerTracker 0 has separate semantics and advances when adopted. Neither
requires replacing the constant without a behavior change.

Every module-deepening ticket has a resolution pointer; separate typed Core
mechanical/outline proposals retain their human approval requirements. Next review:
Runtime preview pipelines and project lifecycle. The two final Standards heuristics
(resolver-builder naming and geometry-script tuple drafts) are nonblocking follow-ups.

### Verification

Full source lint, WASM typecheck, native tests and browser gates pass, including
all 51 page outcomes and CAD 51 / 4 ignored. Repo/tooling and fresh documentation
checks are recorded in the [integration acceptance record](../integration-acceptance-2026-10-08.md).
The isolated CAD container workspace boundary is repaired in `bf3e26af` and
independently reviewed with no blockers. Its Python regressions reproduced the
missing workspace/cache boundary and pass (17 tests). The full tooling and production
build steps pass, including the real CAD container and Dioxus site packaging.
Fresh repo and documentation links pass after the cleanup record. No source changed
after these gates; the retained standalone CAD Clippy limitation is documented in
the acceptance record. This task and the module-deepening ticket frontier are closed.
