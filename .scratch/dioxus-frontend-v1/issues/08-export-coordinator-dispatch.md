# F8.2 private export coordinator dispatch packet

This is a bounded implementation packet for the existing F8.2 task. It does
not change F8.2/F8.3 acceptance or close the parent F8 workflow.

## Source-backed boundary

The pinned React implementation at `5a472a9426e6e38993361da402cd4ec730feb369`
separates `createProjectExporter.ts` (queue and final delivery),
`exports/context.ts` (document/scene/session/board/instance identity), each
domain output module (`documents.ts`, `pcb.ts`, `firmware.ts`, `cases.ts`, and
`keycaps.ts`), and `ExportClient.ts`/`export.worker.ts` (provider worker
transport). The Dioxus implementation already has `Session::StartExport` /
`CancelExport`, `Runtime::export_current`, accepted snapshots, `CoreWorker`
Core/Artifact/Archive transport, the packaged preview-generator worker,
archive delivery, firmware, generic Case STEP, and Keycaps STEP owners. The
React `ExportClient` itself is not bundled in the Dioxus page.

Keep the Dioxus bridge private and use those existing owners. Do not add a
second TypeScript export worker or a second Core/CAD/archive client. The current
SVG/DXF action is a direct, scope-guarded Core artifact call; subsequent work
should route it through the same export intent owner rather than adding more
per-format lifecycle code.

## First runnable provider child

The mounted standalone KiCad footprints action is tracked separately in [F8.2a](08-export-footprints-slice.md). It uses the existing Core standalone-footprints prepare/finish provider, preview-generator conversion, and Core archive pack path. The full/draft selected-board provider is tracked in [F8.3a](08-export-pcb-handoff-slice.md). Both children preserve the parent criteria and do not claim generic F8.2/F8.3 acceptance.

## First implementation slice

Unify route actions behind one private export intent/coordinator in Runtime.
Each accepted intent captures the current accepted document and scene, token,
revision, session epoch, board scope, applicable physical instance, Core worker
identity/epoch, and relevant UI/workspace generation. It uses existing
`StartExport`/`CancelExport` settlement and existing browser delivery cleanup.
After every await, reject a result whose captured owner is no longer current;
never deliver an older board or session result.

The current mounted providers are project archive, SVG/DXF, ZMK firmware,
standalone footprint ZIP, full/draft KiCad board handoff, and ordinary authored Case STEP. Public evidence is
bounded to the actual SVG/DXF downloads on 34763, the footprint ZIP on 34767,
the Keymap-local ZMK download, local Keycaps STEP, and the full/draft KiCad
downloads on 34769 with the corrected assembly heading on 34770; see the
[parent criteria map](08-export.md#current-f8-evidence-and-remaining-criteria-2026-10-03)
and [F8.3a receipts](../evidence/export-pcb-handoff-20261003/34769-RECEIPT.md).
Project archive remains without paired Export-route verification. Full/draft
KiCad use the Session `ExportCommit`/`ExportCommitRequest` path for accepted
wiring and protection mutations. The matching owner advances its token only
after persisted acceptance, and the post-protection token reaches final owner
validation, artifact identity, and `ExportFinished`. The receipts qualify ZIP
outputs but the fixture already had wiring applied; the own-apply branch,
failure ordering, stale races and persisted history/reopen remain unqualified.
Generated mechanical package is still unavailable. Ordinary
authored Case STEP works only when the selected board has no generated
mechanical configuration; the configured path resolves the generated stack
and does not establish authored-only STEP semantics. Generated mechanical ZIP
remains a separate provider gap. Local Keycaps STEP stays with F6.

For full/draft PCB, preserve this ordering from `exports/pcb.ts`:

1. Resolve wiring for the selected board; reject full export when error
   findings remain, while draft export retains findings.
2. If wiring edits are not already applied, commit the accepted electrical
   preparation and advance only this coordinator's expected accepted snapshot.
3. Check selected-board PCB readiness, capture required model files and
   contours, build/pack the KiCad output, and resolve applicable physical
   populations.
4. Protect electrical handoff only after packaging succeeds, accept that
   coordinator-owned document change, re-check current ownership, then deliver.

Unrelated edits, session/document/board/instance changes, executor replacement,
failed packaging, and canceled/superseded intents must prevent stale delivery;
failed package work must not protect wiring. Keep global error reporting and
same-action retry; do not add progress, cancel, or retry controls to the UI.

## Verification boundary

Reuse existing Core/artifact/archive, worker transport, firmware, STEP and
accepted-history evidence. The author run for this packet is one affected
strict page check assigned by root, one changed paired browser journey on the
same retained project, and one consolidated Sol review after the source is
frozen. Do not run the full output matrix or repeat unchanged F2/F6/F7
journeys. F8.2 and F8.3 remain open until root integrates the source, the
required check passes, actual downloads match React, and every remaining
acceptance criterion is mapped to evidence.

## Unresolved scope items

- Browser behavior for failed packaging and stale completion must be exercised
  at the coordinator boundary, not inferred from per-format state helpers.
- Full/draft KiCad's model assets, multi-instance population files, accepted
  wiring commit, package, and protection ordering require an actual combined
  Dioxus journey; the pinned React baseline and source implementation are not
  that proof.
- `Runtime::export_step` can export authored Case STEP when no generated
  mechanical configuration is present; the configured/generated case branch
  resolves the generated stack and must not be relabeled as authored-only.
  Neither branch by itself establishes a complete generated mechanical ZIP.
- Actual physical file picker behavior is not a host gate; browser delivery
  evidence should record filename, media type, byte signature, and cleanup.
