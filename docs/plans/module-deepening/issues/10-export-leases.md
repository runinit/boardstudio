# 10: One export-lease module owns export capture and currency

Status: ready-for-agent
Type: build
Blocked by: —
Spec: [spec.md](../spec.md) · Map: [map.md](../map.md) · Review: [candidate 03](../../../investigations/architecture-review-2026-10-07.html#c3)

## What to build

Line numbers at `915d305c0`, orientation only, in `web/crates/runtime/src/runtime.rs`:

- four capture structs (`FirmwareExportCapture`, `FootprintExportCapture`,
  `PcbHandoffCapture`, `MechanicalExportCapture`, ~268-312) repeat scope, token,
  revision, session epoch, document ID and executor epoch, plus worker identity and
  one extra field;
- per-kind maps on `Runtime` (~390-399);
- per-kind currency checks (`mechanical_export_capture_is_current` ~4772,
  `pcb_handoff_capture_is_current` ~4922, `require_pcb_handoff_current` ~4945,
  `footprint_export_capture_is_current` ~5000, `firmware_export_capture_is_current`
  ~5156) of which three are the same nine clauses;
- pure mechanical package formatting (~6633-7231: STL, XML escape, fabrication notes,
  critical-fit drawing).

Build an export-lease module in `web/crates/runtime/src/` that captures the accepted
snapshot, scope, executor epoch and worker identity when an export begins, answers one
`require_current` over `Session::export_is_current` plus executor identity, and owns
finish and cancellation. Runtime keeps the byte producers and delivery. Move the
mechanical package formatting into its own native-tested module.

## Acceptance criteria

- [ ] Every export kind (STEP, mechanical, keycaps STEP, firmware, footprints, KiCad
  board, board outline, project copy where it captures) takes a lease; the four capture
  structs, per-kind maps and per-kind `*_is_current` functions are gone.
- [ ] The firmware worker-replacement tests (`firmware_export_tests`, ~7534) are
  generalised to run per export kind through the lease interface.
- [ ] Mechanical package formatting has native tests outside `runtime.rs`.
- [ ] Export commits keep strict captured-revision checks (ADR-0005); existing export
  tests pass.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test browser
```
