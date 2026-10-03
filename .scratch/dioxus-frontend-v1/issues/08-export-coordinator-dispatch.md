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

## First implementation slice

Unify route actions behind one private export intent/coordinator in Runtime.
Each accepted intent captures the current accepted document and scene, token,
revision, session epoch, board scope, applicable physical instance, Core worker
identity/epoch, and relevant UI/workspace generation. It uses existing
`StartExport`/`CancelExport` settlement and existing browser delivery cleanup.
After every await, reject a result whose captured owner is no longer current;
never deliver an older board or session result.

The first coordinator packet covers the provider-backed outputs whose full
provider paths are already present in the React reference and current Rust
owners: project archive; SVG/DXF; ZMK firmware; full/draft board package; and
standalone footprint ZIP. It reuses the existing Core artifact prepare/finish
requests and preview-generator conversions, then uses Core archive packing for
the existing package entries. It preserves the exact React names, bytes,
media types, model-file inclusion and scope, and does not add a public Rust
contract. Authored Case STEP, generated mechanical package and local Keycaps
STEP stay with their F7/F6 owners and are integrated as separate coordinator
intents only when their current provider adapters are ready.

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
  journey; a standalone Core `FinishExport` artifact is not that handoff.
- `Runtime::export_step` is the current generated assembly STEP provider. It
  does not by itself establish authored Case STEP semantics or a complete
  generated mechanical ZIP.
- Actual physical file picker behavior is not a host gate; browser delivery
  evidence should record filename, media type, byte signature, and cleanup.
