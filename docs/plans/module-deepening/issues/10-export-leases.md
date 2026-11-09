# 10: One export-lease module owns export capture and currency

Status: resolved
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

- [x] Every export kind (STEP, mechanical, keycaps STEP, firmware, footprints, KiCad
  board, board outline, project copy where it captures) takes a lease; the four capture
  structs, per-kind maps and per-kind `*_is_current` functions are gone.
- [x] The firmware worker-replacement tests (`firmware_export_tests`, ~7534) are
  generalised to run per export kind through the lease interface.
- [x] Mechanical package formatting has native tests outside `runtime.rs`.
- [x] Export commits keep strict captured-revision checks (ADR-0005); existing export
  tests pass.

## Verification

```sh
cargo test -p boardstudio-web-runtime --locked
python3 scripts/check.py lint typecheck test browser
```

## Outcome

Merged implementation commits `28b9d81d2`, `3d432f520`, `11ac3786b` from
`deepening/10-export-leases`. One typed lease registry captures the accepted snapshot,
scope, executor epoch and worker identity for all eight export kinds, owns finish and
cancel, and replaces the per-kind captures/maps/currency helpers. Runtime retains
production and delivery. Mechanical package serialization is a native-tested module.

Production firmware browser tests exposed a lifecycle regression: Session removes
operation ownership before delivery. The corrected delivery check validates captured
source/scope/token/executor/worker; active work also requires Session ownership.
Terminal reports validate scope. Standards and final Spec reviews found no issues.

After rebasing over ES-20 and the wave-2 claims, lint, WASM page typecheck, Rust
workspace tests and footprints tests passed. Runtime browser 29/29 and page runner
42/42 passed; downstream Catalogue 12, Case 60, Parts 50, PCB 38, Layout 92 and setup
guide 1 also passed. Host 6, UI model 4, shared UI 8+1, Keycaps 27 and Library 13 passed.

Full gates remain blocked: the final full browser script timed out/SIGKILL at the
Keymap failed-binding test, reproduced on exact starting revision `19598aea0` while
its isolated test passed. An earlier complete ticket-10 browser run passed Keymap
21/21. The canonical unchanged CAD native gate failed its internal-gasket baseline
volume fixture (49 passed, 1 failed, 4 ignored); see
[ES-20 verification details](../../edit-settlement/issues/20-preview-only-direct-edit-event.md#outcome).
No Core, CAD or contract sources were changed by this ticket.
