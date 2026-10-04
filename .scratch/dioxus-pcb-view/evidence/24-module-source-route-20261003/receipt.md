# PCB mounted-module source route and findings comparison

Candidate `http://127.0.0.1:34769/boardstudio/`; frozen source `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`; provenance SHA-256 `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`. Root package proof records the full 22-command lineage, 1,373 verified sources, 154 matching assets per route and no root/subpath asset mismatch. The browser journey itself used the root route only.

Both apps imported the same VIK archive, SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`. React was pinned at source `5a472a9426e6e38993361da402cd4ec730feb369`; candidate and React ran in separate named `agent-browser` sessions.

## Mounted module route

On candidate PCB, the selected mounted-module target was `review/splitter-above`, definition `pcb/vik-splitter/vik-splitter`. A physical pointer click at the unobscured mounted-footprint point opened the visible Parts workspace and the matching source Inspector. Focusing that same module target and pressing Enter, then repeating with Space, each opened the same Parts source entry. The source entry's **Edit selected mounted placement** action returned to the exact PCB placement Inspector each time.

The returned placement was Main board / Board-wide / host face Front / facing face Back / X 29 mm / Y 20 mm / yaw 0° / gap 3 mm / Board attachment / service clearance 4 mm. Existing support rows PART0 and PART3 remained present. React's visible Parts route selected the same `pcb/vik-splitter/vik-splitter` source and placement `Main board · front · splitter-above`; its return label is **Back to PCB**. The internal React mode name is `Library`, mapped to the visible Parts label. The React keyboard route is separately recorded in [the route pin](/home/chris/.local/share/boardstudio/reviews/pcb-public-next-20261003/react-route-pin.md).

The candidate `.boardstudio` export before the route, after pointer/keyboard route returns, and after both findings-layer visibility states had the same SHA-256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`. Each archive's `project.json` is byte-identical, SHA-256 `bca6ba22f56e91456d70d6c5676e7d3d70f6e8e85eded62ed8812a9dfea7eb42`, project `vik-module-review`, revision 6. No edit, undo, redo or save operation was performed during this route-only journey. Existing placement persistence and accepted-history proof is reused from [Save/reopen](../22-mounted-module-inspector-20261003/placement-save-reopen.md), [settled Undo/Redo](../22-mounted-module-inspector-20261003/undo-redo-settled-34765.md), and [support/clearance Save, Undo/Redo and reopen](../23-mounted-module-supports-public-34767.md).

## Findings layer result

The comparison uses current-board SVG marker IDs from the same imported archive in both apps. The retained raw ID arrays and a derived comparison are in [`findings/`](findings/).

| App | Findings & clearances off | All visible | Unique IDs off → all |
| --- | ---: | ---: | ---: |
| React | 47 | 78 | 47 → 72 |
| Dioxus candidate | 44 | 75 | 44 → 72 |

The all-visible unique ID sets match. In the captured default DOM, React shows three source-caveat IDs that candidate Dioxus omits:

- `module/review/ec11-rotary/gate/source-caveat`
- `module/review/splitter-above/gate/source-caveat`
- `module/review/splitter-below/gate/source-caveat`

When all visible, the captured DOM has three React marker instances per caveat ID versus two Dioxus instances. These are observed DOM counts, not a contract to reproduce. A live React Workbench inspection confirmed all three accepted PCB error findings target their current mounted-module instance and host board, so React's pinned `moduleFindingIds` predicate classifies them as module findings, matching the Dioxus predicate. React's JSX then filters every marker with those IDs when that layer is hidden. Root's browser console independently reported duplicate React keys for these same `module/.../gate/source-caveat` IDs; the live DOM also retained matching groups after the filter's inputs said they should be excluded. The discrepancy therefore includes stale/duplicate React DOM behavior and does not establish a candidate classification defect. Do not port those stale nodes or add marker-ID-specific filtering. This receipt preserves the initial paired DOM observation; the changed candidate browser check remains open. Findings state remains transient and the three project archive exports above are identical.

## Captures

Screenshots and exact exports remain under `/home/chris/.local/share/boardstudio/reviews/pcb-public-next-20261003/`:

- React source route: `react-keyboard-to-module-parts.png` (SHA-256 `d351df9d4fef582e4d7781465e542fe8334de9104f0fc44f852ffdbf415743d1`).
- Candidate PCB baseline: `candidate-pcb-before.png` (SHA-256 `2fa91c31abc579c688270f4246491e1ab2123af9d999ef98b82681fffbe14291`).
- Candidate pointer → Parts: `candidate-pointer-module-parts.png` (SHA-256 `1e54f9d02d25c9232494154efbb6912e61a85c54c9800ecc5d685f0d2be65901`).
- Candidate pointer return: `candidate-pointer-placement-return.png` (SHA-256 `0fc86ef0c996dd1404ebb58dbb516b29290787cd720297518d86ccac43414754`).
- Candidate Space → Parts: `candidate-space-module-parts.png` (SHA-256 `1cd6c6bf1ca3ad871f1e5eae5b7eceb37bf02c149f47d78cd5bfb8de5c2e0368`).
- Candidate findings off/all visible: `candidate-findings-default.png` (`ce27a1b24e7e4a71ceca3abee5b28a4312819f38f947d7a9e7b0b848834d1c3c`) and `candidate-findings-all-visible.png` (`e17fa5adc36b7340bb5a82904affbd71065dc9891550f8e80efdbc7a65862df2`). React comparison captures: `react-findings-default.png`, `react-findings-all-visible.png`.
- Candidate project exports: `candidate-before-route.boardstudio`, `candidate-after-route.boardstudio`, and `candidate-after-findings-toggle.boardstudio`; all three have the same archive hash above.

## Scope

This proves pointer, Enter and Space routing for the selected splitter-above placement on this fixture, exact source projection, guarded return, and no project/history mutation from route or layer toggles. It does not prove every mounted instance, variant-change stale-route rejection, subpath browser journey, module finding focus/navigation, nonempty clearance geometry, or complete Issue22/Issue15, F5.4/F5.5 or parent acceptance. Issue22's edit history and Issue23 changed-field history remain independently qualified by the reused receipts above.

## PCB focused-finding visibility follow-up

On 2026-10-04, the candidate at `http://127.0.0.1:34800/boardstudio/` served the packaged subpath bundle for `frontend-functional-batch-20261004`, source `14434d50c6bee16c595609a39f34396edf37cbd1` (subpath JS SHA-256 `d2ac74558aa05f82fd933dcbfc9f67642eb70025e1b4c5c6f95a25fb8090008a`). Imported the same retained VIK project archive (`candidate-before-route.boardstudio`, archive SHA-256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`, revision 6) into named `agent-browser` session `f54-focus-7dd31abc1fc4`.

With Board outlines, Clearance & service, and Findings & clearances hidden, the complete PCB scene DOM had three mounted-module groups and no visible module outlines or clearance polygons. After exposing module findings, keyboard focusing `module/review/splitter-above/gate/source-caveat` with Enter produced the focused marker and temporarily revealed/accented only `review/splitter-above`'s board outline; the other two module groups remained hidden. Escape cleared focus and removed that outline. Focusing a host finding also left all module outlines hidden. Layer controls retained the pre-focus outline and clearance hidden states throughout; Findings & clearances was restored to its initial hidden state at the end. No save or project edit was performed.

Pointer activation of that finding marker was not qualified: the browser click was intercepted by the overlapping `.m1-outline` polygon, and the marker polygon has `pointer-events: none`. The retained fixture has no clearance polygons, so clearance reveal remains unobservable here. This qualifies keyboard focus, Escape, host-finding focus, and visibility preference restoration only; pointer activation and nonempty clearance reveal remain open.

### Finding-list action parity correction (2026-10-04)

The earlier pointer observation concerned the decorative PCB marker polygon (`pointer-events: none`) and is not a required action. I compared the actual **Layout findings → Select affected geometry** action on React TS `http://127.0.0.1:5175/` in named session `f54-ts-compare-2318ea1a1376`, using the same retained archive (`candidate-before-route.boardstudio`, SHA-256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`). With PCB Board outlines, Clearance & service, and Findings & clearances hidden, selecting the first SW2 row (“Mounted geometry overlaps a host component…”) changed the active tab from PCB to Layout and selected SW2. Returning to PCB left those same three visibility controls hidden. This matches the candidate behavior; the earlier marker-polygon interception is not a cross-app finding-focus mismatch. No project edits or saves were made.

### RF-024 marker semantics correction (source follow-up, 2026-10-04)

The PCB SVG finding marker is now decorative, matching the React canvas contract and its existing `pointer-events: none` CSS: the Dioxus marker no longer emits button/keyboard semantics or activation handlers. This also removes the marker-only Enter/Escape focus path and its temporary module outline/clearance override; the separately paired **Layout findings → Select affected geometry** action is unchanged. Module-outline and clearance visibility once again follows the user's layer switches, and module selection handlers are unchanged. The earlier packaged keyboard-focus observation above predates this correction and is not evidence for the current source. A focused mounted-WASM regression first failed because the marker rendered `role="button"`, then passed after the correction (1 passed, 217 filtered). `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check` pass. No package or browser retest was run.

### RF-024 candidate verification (2026-10-04)

Candidate `http://127.0.0.1:34805/`, build `frontend-keyboard-focus-20261004`, source `916a40549444e7ac73c144e422f632a129fd2eaa`; imported the retained splitter-above VIK archive (SHA-256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`). All 44 PCB finding SVG groups were decorative (`role`, `tabindex`, `aria-label` absent; 0 tab stops; computed `pointer-events:none`). Dispatching click and Enter on a marker left the current PCB workspace/selection unchanged. The SW2 **Select affected geometry** row worked by pointer and by focusing its button and pressing Enter; both routed to Layout and focused `module/review/splitter-above/host-component/matrix/matrix/r0c1`, matching the already-retained TS 5175 route result. Returning to PCB preserved the initially hidden Board outlines, Clearance & service, and Findings & clearances controls; the scene retained 33 host parts and 3 mounted-module selection buttons. Pointer selection of host SW2 still selected its PCB inspector; mounted-module pointer/Enter/Space routing remains covered by the earlier paired route receipt. The focused mounted-WASM semantic regression is green (1 passed, 217 filtered); no source/tracker/run/build edits or package/browser suite were run for this candidate.
