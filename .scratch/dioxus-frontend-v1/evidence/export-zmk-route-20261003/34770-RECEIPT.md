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
