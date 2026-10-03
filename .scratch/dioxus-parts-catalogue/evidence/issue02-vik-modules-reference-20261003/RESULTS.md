# Issue 02 VIK module row and variant reference pin

- React URL: `http://127.0.0.1:5173/`
- React source: `5a472a9426e6e38993361da402cd4ec730feb369`
- Fixture: `VIK module review · above and below`
- Source data: `app/src/modules/imported-modules.json`, SHA-256 `dce6dba69eb7f8d672f7ba499442c9e86f656a473016be56943425915d9724b8`
- Dedicated browser profile: `parts-issue02-react-34762-2318ea1a1376`

The Parts catalogue lists 29 VIK rows from 38 module snapshots. The row “various display adapters in different sizes” exposes three exact variants: `pcb/2inch-0.8mm/pcb`, `pcb/1.47inch/pcb`, and `pcb/1.28inch-round/pcb`. Selecting the row selects the first variant. The right Inspector exposes the exact variant selector, a pinned upstream source link, and recorded hardware readiness by Footprint, Electrical, Case, 3D model, and Firmware. Switching to `pcb/1.47inch/pcb` updates the Case gate from “1 to review” to “2 to review”; the other visible counts remain tied to the selected source snapshot.

Screenshots: [first variant](selected-first-variant.png) (SHA-256 `b08d33f0b62db17adc81e610cb6dfc5f893ea9eba0ebba96eb35211e296fcd10`) and [1.47-inch variant](selected-147-inch-variant.png) (SHA-256 `e4473b72c01ea7ea43a6934cc00b3b92613a7de9ef499a32583d4a68c8868904`). This is a read-only reference pin, not candidate evidence. It does not cover every module, search edge, project override, loading failure/re-entry, viewport/theme, or any mounted placement/electrical behavior.
