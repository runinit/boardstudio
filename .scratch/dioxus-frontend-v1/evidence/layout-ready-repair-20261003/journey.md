# Layout-ready repair qualification — 2026-10-03

Reference: React `5a472a9426e6e38993361da402cd4ec730feb369`, http://127.0.0.1:5175/, isolated session `layout-shortcut-reference-20261003`, Sofle v2. Original Dioxus candidate: `frontend-source-batch-20261003-5`, source `5c8161cf80d2c93c6214cb7caed6dffc4463f64d`, http://127.0.0.1:34774/.

## Reference observations

- Selected `keys` in Objects, then Relations. Transform → Position & rotation returned to Properties with matrix controls.
- Transform → Splay & origin selected a column; Transform → Row offsets selected Row 1 and Properties with numeric offsets. React handlers explicitly select Properties and reveal the Inspector (`app/src/ui/Workbench.tsx:1310–1312`).
- Row 1 Offset X accepted `2`. Undo was clicked, but the immediate snapshot preceded settlement; no separate final row-Undo assertion was captured.
- Outline → Copy outline selected `Edited outline 1`. Edit perimeter points exposed point coordinates, grid, insert/remove, point selection and Done.
- Point 1 X changed from about `-3.82` to `-5`; a settled read confirmed `-5`. Undo restored `-3.82`; Redo restored `-5`. Reload, reselect Outline, Edit perimeter points retained `Edited outline 1` and X `-5`.
- Visual evidence: `react-outline.png`, `react-outline-reopened.png` (1280×577, default light theme).

## Original candidate failure

Fresh isolated `layout-shortcut-before-20261003` opened and Start Sofle v2 returned. Snapshot command yielded execution session40177; its final result was retrieved: exit1, `CDP command timed out: DOM.enable`. Unlike the previous attempt, this is a completed command failure. `agent-browser doctor --offline --quick` reported11pass/0warn/0fail. Direct CDP `Runtime.evaluate(1+1)` also timed out; Sol reproduced and captured a debugger stack/CPU profile. The specific app cause and repaired-candidate results follow below; no parent acceptance is inferred.

## Implemented repair and focused regressions

Application source: `cbce292b`.

- The root Properties callback now selects the Properties tab while retaining the current-owner guard. Splay & origin and Row offsets invoke it before changing selection, because selection changes the owner generation. Their own availability gates remain; Position retains its properties-availability gate.
- The hidden PCB reference editor's no-reference reset tracked `request_generation()` and wrote it immediately, causing an effect feedback loop. The production reset helper uses `peek()` for its invalidation counter. `board-reference-effect-red.log` records eight runs versus one expected; `board-reference-effect-green.log` records one passing mounted regression. Command: `cargo test --manifest-path web/Cargo.toml --locked --no-default-features --test board_reference_effect`.
- The mounted production toolbar test failed when the two selection-shortcut callbacks were removed (`false:1` versus `true:1`), then passed with them restored (1 passed, 179 filtered). Output was retained in agent tool results only; no stdout log file was produced. Invocation:

```sh
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=/home/chris/.cache/.wasm-pack/wasm-bindgen-9411bdb1a3e2bbb9/wasm-bindgen-test-runner \
CHROMEDRIVER=/usr/bin/chromedriver WASM_BINDGEN_TEST_ONLY_WEB=1 \
cargo test --locked --manifest-path web/Cargo.toml --target wasm32-unknown-unknown \
 --no-default-features --features page --bin boardstudio-web \
 all_transform_property_shortcuts_route_to_properties_before_selection_changes_owner -- --nocapture
```

Three stale test API callsites were updated to allow this focused test to compile; no existing assertions were removed.

The reference key-size journey was also completed with settled reads: select `keys`, focus Key width slider, ArrowRight changed `1`→`1.25`; Undo restored `1`, Redo restored `1.25`; reload and reselect `keys` retained `1.25`. Screenshot: `react-properties-reopened.png`.

## Repaired public candidate

`frontend-layout-ready-repair-20261003`, packaged source `2caace52ee2bd9a813030586bf88a1de7aa88a0c`, root http://127.0.0.1:34775/, subpath http://127.0.0.1:34775/boardstudio/. Both route assets and all 1,385 frozen source hashes verified by `record-candidate`; eight fresh commands and 22 inherited provider commands. Package proof is `package-proof.json`. Compiler and package succeeded. Two unused-mut warnings were subsequently removed by source-only `e0e89924`; `warning-cleanup-check.log` confirms clean compilation. That qualifier-only cleanup is not in the immutable preview package.

Isolated browser session `layout-repaired-20261003`, 1280×577, light theme:

| Focused behavior | Dioxus result | Paired reference |
| --- | --- | --- |
| Start Sofle v2; read and interact with mounted Layout | PASS: responsive after open; subsequent selection, edits, Undo and reload all responded | Responsive |
| Matrix Relations → Transform → Position & rotation | PASS: Properties selected, matrix position controls present | Same transition |
| Relations → Splay & origin | PASS: Column 1, Properties and splay/origin controls present | Same transition |
| Relations → Row offsets | PASS: Row 1, Properties and numeric offsets present | Same transition |
| Matrix key width edit/history/reopen | PASS: 1→1.25; Undo→1; Redo→1.25; reload/reselect retained1.25 | Same settled values and persistence |
| Generated outline copy/perimeter edit/history/reopen | PASS: Edited outline1 created, Point1 X changed to-5; Undo restored prior-3.97, Redo→-5; reload/reselect/edit retained-5 | Same operations/persistence; reference initial X≈-3.82 |

Screenshots: `dioxus-properties-reopened.png`, `dioxus-outline.png`, `dioxus-outline-reopened.png`; paired reference screenshots named above. Key width was returned to1 before the Dioxus outline journey; the reference outline was copied before its key-width journey. The0.15mm initial point difference therefore needs matched pristine-fixture/size-roundtrip characterization; these runs do not establish geometry equivalence.

## Remaining parity findings — not accepted

- Matrix Inspector order/headings differ: Dioxus places a Matrix section with Rows/Columns/Pitch before Key size; React presents Layout name, Independent layout, Key size then matrix controls. Selection summary typography/breadcrumb and the Objects selection footer differ.
- After selecting the `keys` matrix from Objects, the Dioxus toolbar still reads `Select: Key`; React reads `Select: Matrix`. Selection-kind synchronization needs follow-up under the existing toolbar/selection work.
- During perimeter editing, Dioxus retains the general Select/Transform/Align/Snap pill. React switches to `Outline points`, grid/Snap and Done. The dedicated perimeter grid control is absent from the Dioxus Inspector snapshot.
- Dioxus perimeter handle circles are visibly much larger than the reference at the same viewport/zoom; the Objects outline/version hierarchy is also not expanded like the reference. Contextual Inspector scrolling and a Routed PCB reference section appear in the captured outline surface and need comparison against intended context.
- This pass verifies the listed editing and persistence operations, not all constraints, pointer cancellation, feature drawing, snapping or full parent acceptance. Existing 62 parent criteria remain intact.
