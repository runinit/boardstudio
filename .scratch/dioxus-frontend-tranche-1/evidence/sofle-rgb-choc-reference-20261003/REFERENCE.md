# Sofle RGB and Choc — pinned React reference

- Reference URL: `http://127.0.0.1:5173/`
- React source: `5a472a9426e6e38993361da402cd4ec730feb369`
- Isolated agent-browser session: `parts-sofle-variants-20261003`
- Read-only document inspection: IndexedDB `boardstudio-v2` / `projects`.

The Project menu lists `Start Sofle v2`, `Start Sofle RGB`, and `Start Sofle Choc`. Starting RGB and Choc opens their named Layout projects. The RGB workspace exposes MX switches and 14 back-side D31–D37 underglow LEDs across the two halves; Choc exposes Choc V1/V2 switches and no D31–D37 underglow parts. Both retain left/right boards and independent 24-key plus 5-thumb matrices per half.

| Variant copy | Project ID | Stored name / source parameter | Revision | Boards | Parts |
|---|---|---|---:|---:|---:|
| RGB | `519b9d57-a22a-4dc7-ac4a-4992f2ef2fd8` | Sofle RGB / `sofle-rgb` | 2 | 2 | 212 |
| Choc, first start | `44c950f9-9d58-4e09-aaf1-7c1e0914a49a` | Sofle Choc / `sofle-choc` | 2 | 2 | 198 |
| Choc, repeated start | `fb9447e9-9974-420f-acea-f0d65c138c87` | Sofle Choc / `sofle-choc` | 2 | 2 | 198 |

The repeated Choc action created a different project ID while retaining the same two-board document shape. These are fresh editable copies in the same reference browser store. The RGB and first Choc document facts were captured at revision 2 immediately after open, before the separate custom-editor work in that reference session changed RGB; the repeated Choc copy remains untouched at revision 2. This read-only pin does not cover RGB/Choc edit, Undo or saved-card reopen behavior.

The Project menu screenshot shows the three Sofle start actions (among the wider React demo catalogue): `react-project-menu.png`, SHA-256 `d3019c86680c35079b741698daf274f9fe1ef19a2b9b94b332f76beb73ab825f`.
