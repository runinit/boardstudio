# 104: Keep the original REVIUNG41 archive as a historical reference

Status: ready-for-agent
Type: task
Blocked by: [103](103-reviung41-native-mechanics.md)
Parent: [25](25-shipped-boards-review.md) · Spec: [spec.md](../spec.md)

## Source evidence

The [phase 1 audit](../audit/shipped-boards-review.md#19-reviung41-original-archive) records the original archive at revision 3, with 85 parts, one 2u stabiliser and legacy gasket settings (four supports; no closure mounts or case body). Its measured counterpart has the same part poses but is the editable rebuild target. [Human decision 7](../audit/shipped-boards-review.md#human-decisions-2026-10-10) requires preserving the archive.

## One-hour slice and ownership

Own only the original-versus-measured provenance description in `content/demos/README.md` and this ticket’s evidence. Record the existing archive SHA-256 before/after the final generated-output batch. Verify `prepare_demo_projects` still copies `content/archives/reviung41-original.boardstudio` without rewriting it; compare the source bytes and shipped archive bytes and reopen the copied result through the public archive route. Keep the measured recipe/library card separately named. Do not edit archive bytes, imported metadata, key poses or legacy mounting data.

## Acceptance and verification

- [ ] The original archive is visibly a historical comparison; measured REVIUNG41 is the editable native example.
- [ ] Source archive hash unchanged, generated copy byte-identical, archive reopen evidence recorded with its historical parts/configuration distinguished from format migration.
- [ ] Documentation links pass `repo`. Reuse ticket 23’s actual archive/build/reopen results if they already establish the same current outputs; otherwise request the designated output owner’s build slot. Do not add a hash-only test that duplicates a generator implementation.

The original’s gasket/hardware fit remains unqualified. Do not resolve ticket 04 or 23 here. VIK remains deferred until a host is selected and receives no rebuild work in this ticket.
