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

## Verification

```sh
python3 scripts/check.py
python3 scripts/check-doc-links.py
```
