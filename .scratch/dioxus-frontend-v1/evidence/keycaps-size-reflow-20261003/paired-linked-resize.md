# F6C.3 selected linked-key resize — paired browser receipt (2026-10-03)

This is one linked-key resize journey on the existing Layout size/reflow owner. It qualifies the mounted selected-key path for one linked-half edit and its accepted history/reopen behavior; it does not close all F6C.3 or F3.2/F3.5 joins.

## Pinned run

- React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Dioxus candidate: `http://127.0.0.1:34759/`, source `04c85b88eae2894b416979f785a1f0060d2eb27d`, build provenance SHA-256 `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`.
- Viewport: 1280×577. Both apps imported the same derived fixture.
- Derived archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/keycaps-linked-layout.browser.boardstudio`, SHA-256 `4e3e69cc8f0ded781ed5ce807df7abbbabf934bdbe35b956bda7c94bd997abdd`; embedded `project.json` SHA-256 `1482efccba5e942bf9d124f9876a6fb44d71286f100f53790433af9ac08fba76`.
- Fixture derivation: copied the retained Sofle archive `/home/chris/.local/share/boardstudio/retained-tmp/20261002/dioxus-34726-fixture-models-false.boardstudio` (SHA-256 `c6ea3c0f72f999ee6736d1e65b6b2c36105b52c5a8d511285ffdc01695aa9d7e`), then applied the same fixture-only mirror relationship used by the linked-Core characterization: `right-keys-layout` points to `left-keys-layout`, both matrix layouts are on Left PCB, and the right half's parts are members of that board. Neither source archive nor project was modified.

## Journey and result

After import, both inspectors showed the same linked key at row 1/column 1, 1u×1u, with the React **Unlink halves** control visible (Dioxus tree identified the `keys` matrices as Linked). Four trusted ArrowRight inputs on Width changed the selected linked key to 2u×1u. The accepted edit updated the mirrored counterpart too: selecting the opposite key after the edit showed 2u×1u in both apps. The adjacent row-1/column-2 placements moved outward symmetrically; React's accessible placement readout changed from X ±28.23 mm to X ±37.78 mm. Both apps reported the remaining pair overlap (`right-keys-SW1, left-keys-SW1`) and retained the existing actionable warning.

Undo restored 1u×1u and removed that warning; Redo restored 2u×1u and the warning. The Dioxus document advanced to revision 10 for the resize, 11 for Undo, and 12 for Redo. After reload and reselecting the linked key, both apps still showed 2u×1u and the same overlap warning. The warning is expected for this fixture: the mirrored row-1/column-1 pair is centered on the mirror axis and the 2u envelopes extend across it. No geometry engine or selection policy was changed in this journey.

The live path exercised the existing `KeySizeControls` → private accepted-document planner → `ReplaceDocument`/Core history flow. The separate linked-Core receipt remains useful domain-path evidence; this receipt shows the user-facing control reaches that path. The same source already contains the mounted delayed-owner and feedback-owner corrections recorded in [`owner-lifecycle-repair.md`](../../../dioxus-keycaps-workflow/evidence/size-reflow/owner-lifecycle-repair.md); no duplicate tests were run here.

## Captures

- [`react-linked-reload.png`](react-linked-reload.png), SHA-256 `2b4b69d1946bb755af34dae14ef36f38fc6e49902dece3aa4dbf6bb3c469c7ea`.
- [`dioxus-linked-reload.png`](dioxus-linked-reload.png), SHA-256 `76cc54fe5b646759bb89b80700749d76c218b6fd28a98a17e28c7eb3aa841622`.

Both are post-reload captures with the linked SW1 key selected, 2u×1u accepted size, Saved state, and the remaining overlap warning. F6C.3 still lacks a paired journey selecting both mirrored halves together, and the broader F3.2/F3.5 joins remain open. Existing RF-005/RF-001/RF-009 are carried forward; this browser run produced no new refactoring finding.
