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
