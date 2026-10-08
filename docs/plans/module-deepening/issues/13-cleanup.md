# 13: Cleanup and record

Status: ready-for-agent
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
