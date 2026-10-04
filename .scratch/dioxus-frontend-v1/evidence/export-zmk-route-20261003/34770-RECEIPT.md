# Export-route ZMK journey — candidate 34770

**Scope:** one ready-state Export-route ZMK action on the frozen Dioxus candidate, paired against pinned React with the same retained project. This qualifies the Export entrypoint and emitted archive only; it does not close F8.4 or F8.6.

## Source and fixture

- Dioxus URL: `http://127.0.0.1:34770/boardstudio/`
- Dioxus source commit prefix: `47623521`; candidate provenance SHA-256 `a4677af663c0e8286ae5e923bf1884b0230bcc6dc852a3030cc35a68d453365c`
- React URL: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`
- Both fresh isolated browser sessions imported that exact fixture and opened Export with `Left PCB` selected.

## Behavior and artifact

Both Export routes enabled **Export ZMK firmware**. React describes the row as a ZMK v0.3.0 configuration with an editable starter keymap; the Dioxus route exposed the same action. Each click completed an actual ZIP download without a surfaced page error. Browser console contained only Vite connection diagnostics (Dioxus) and React DevTools notice (React).

The output filenames follow the same existing owner contract: React `exportFirmware` returns `${document.name}-zmk.zip`, and Dioxus Runtime uses `format!("{}-zmk.zip", snapshot.document.name)`. The fixture name is `Sofle v2`; the browser download helper stored each file under an explicit evidence basename.

Retained downloads:

| Build | Bytes | SHA-256 | Signature |
| --- | ---: | --- | --- |
| React | 12,460 | `6fe42bdb05fca711c15b144036663ece07e1981a86ffe99aa2194deacc7877d7` | ZIP (`PK\x03\x04`) |
| Dioxus | 12,176 | `3ea4423cc13ce62b98a96111cef91f8d7172a8e4c48d7febf6a702ad2ec56237` | ZIP (`PK\x03\x04`) |

The archives contain the same 15 paths: workflow, README, local build files, ZMK shield Kconfig/README/keymap/overlays/configs, `config/west.yml`, and `electrical-plan.json`. The 14 generated source/config entries are byte-identical. `electrical-plan.json` has equal parsed JSON trees; its raw bytes differ in serialization order. Both outputs therefore have semantic archive parity, not byte-identical ZIPs. Existing paired Keymap-local firmware evidence remains unchanged and is not used as a substitute for this route click.

External retained files and screenshots are under `/home/chris/.local/share/boardstudio/reviews/export-zmk-20261003/34770/`. The evidence archives are `react-export-zmk.zip` and `dioxus-export-zmk.zip`; route screenshots are `react-export-route.png` and `dioxus-export-route.png`.

## Limits and refactoring observation

This ready fixture confirms the enabled-row/download path only. The disabled/not-ready explanation, firmware-generation failure and retry, stale owner suppression, and broader F8.2/F8.6 ordering/join criteria remain open. No new refactoring observation arose in this browser-only qualification; no source or RF ledger change was made.

## Supplementary F8.2-C03 public failure and same-action retry

On 2026-10-03, the same real layered Sofle fixture (`imported-layered-sofle.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`) was imported in isolated sessions against Dioxus `http://127.0.0.1:34784/` (`f8c2-c03-dioxus2-7dd31abc1fc4`) and pinned React `http://127.0.0.1:5175/` (`f8c2-c03-react2-7dd31abc1fc4`). Both began with Left PCB encoder-A/B/push pins P1/P0/P2. In each public PCB Wiring UI, encoder-A was changed to **Unresolved** and the same public Keymap **Export ZMK source** action was invoked. Neither attempt downloaded a ZIP; Dioxus showed `No resolved GPIO for encoder left/SW25`, and React's global alert showed `left/SW25/encoder-a requires ; review the pin assignment before changing it; No resolved GPIO for encoder left/SW25`.

For recovery, the original P1 assignment was restored through the public wiring controls (React's Undo restored the field; Dioxus restored the field and showed `Wiring pin saved.`). Re-invoking the same ZMK action then produced valid ZIPs. Dioxus downloaded `Sofle v2-zmk (2).zip` (12,176 bytes, SHA-256 `3ea4423cc13ce62b98a96111cef91f8d7172a8e4c48d7febf6a702ad2ec56237`); React downloaded `Sofle v2-zmk (3).zip` (12,460 bytes, SHA-256 `6fe42bdb05fca711c15b144036663ece07e1981a86ffe99aa2194deacc7877d7`). Both ZIPs pass `unzip -t`, contain the expected 15 paths, and their `electrical-plan.json` reports encoder-A P1, encoder-B P0 and push P2. The output hashes match the already retained successful 34770 downloads. React kept the prior dismissible global error alert visible after its successful retry; Dioxus cleared its alert on retry. This is recorded as observed reference behavior, not a packaging failure.

This paired leg qualifies only the real provider failure and recovery on the Keymap-local ZMK entrypoint. It does not exercise an in-flight stale board/document change, package/protection ordering, Export-route disabled-readiness guidance, or other provider failures; F8.2 and its wider joins remain open. Both task-owned browser sessions were closed.

## Bounded F8.2-C03 PCB package/protection source audit

The production KiCad path preserves the required ordering in `web/src/runtime.rs`: `pcb_handoff_bytes` may first accept `ApplyElectrical` when the resolved wiring is not yet applied, then calls `pcb_handoff::build_handoff(...).await?`, checks the captured owner again, and only then submits `ProtectElectricalHandoff`. Therefore a build, Core archive-packaging, serialization, or current-owner error returned before that protection call cannot commit protection. One distinction matters: if export had to apply wiring before packaging, that accepted preparation edit can remain after a later packaging failure; the guard establishes “no protection on failed packaging,” not rollback of preparation.

The delivery path also has no stale artifact insertion on an error: `Effect::RunExport` inserts into `artifacts` only in its `Ok` arm; the `Err` arm completes `ExportFailed`. `application/src/session.rs` turns that into `ExecutorFailed`, and the Runtime settled observer reports the reason through its existing global status path. Successful results are revalidated by the session before `DeliverExport`; Runtime removes the artifact from its map before checking token, scope, and the current accepted snapshot. A retry through the same UI action creates a fresh operation and capture (`export_kicad_board`) and clears the prior alert before dispatch.

This audit is source proof only. Existing `production_run_export_injects_both_provider_failures_and_retries_without_editing_session` in `web/src/runtime.rs` exercises controlled firmware-generation and firmware-ZIP failures, not the PCB handoff transaction. No focused PCB package-failure/protection regression or public PCB failed-package/retry journey was run for this packet. The paired unresolved encoder-A failure/recovery above is also Keymap-local ZMK, not package failure. F8.2-C03 therefore still needs a PCB-path test or public scenario that observes failed packaging before `ProtectElectricalHandoff`, verifies global error and no protection/stale delivery, then retries the same action successfully; an in-flight stale board/revision delivery remains unqualified as well. No source change or build was made.
