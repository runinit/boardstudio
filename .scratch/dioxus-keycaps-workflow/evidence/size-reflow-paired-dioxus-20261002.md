# Key-size/reflow paired Dioxus replay — 2026-10-02

This is the Dioxus side of the existing pinned-React Key-size/reflow journey. It supplies evidence for the F6C.3 browser gate only. It does not accept F6C.3's complete joins or F6C.4 Keycaps workbench acceptance.

## Source and fixture identity

- React oracle: `http://127.0.0.1:5173/`, commit `5a472a9426e6e38993361da402cd4ec730feb369`, exact clean run and source evidence in [the React packet](/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-react-paired-journey-5173.md) (SHA-256 `8b009cfbb3c248a7844ce777c6c4bb0006207a6eafe45b96f5658cf733667727`).
- Dioxus candidate: `http://127.0.0.1:34732/`, full-package source commit `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`; build manifest `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-workbench-parity-joined-20261002/provenance.json`, SHA-256 `4bc856174b33998c5cf99f4c05a8ba35ad7ab4ce7705bfece6510cdd4467544e`. The manifest says `status: complete`, records 1,328 source hashes and 145 packaged assets. Root reports the package/route inventory was independently verified.
- Candidate server: PID 3303978, `frontend-workbench-parity-joined-server-20261002.py`; root `/home/chris/01_Projects/ts-boardstudio2`, Keycaps build served from `web/target/builds/frontend-workbench-parity-joined-20261002/site-root` with subpath assets served from its `site-subpath`.
- Browser: isolated `agent-browser` session `keycaps-dioxus-joined-9d34`, URL above. The retained Dioxus PNG captures are 1280×577; the browser viewport was not independently recorded, so this is not viewport-matched visual evidence against the 1280×940 React captures. The user’s in-app browser was not navigated.
- Input archive: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/dioxus-34726-fixture-models-false.boardstudio`, SHA-256 `c6ea3c0f72f999ee6736d1e65b6b2c36105b52c5a8d511285ffdc01695aa9d7e`; embedded project JSON SHA-256 `981c8978101286985b2f5220e8af18a612037f5fc7cd892a52fcb881f39eae7f`. Project ID `m1-sofle-v2-copy`, Sofle v2, revision 9, 140 parts, two boards and keys/thumbs matrices; no authored keycap overrides. This is the distinct retained c6 fixture. Missing original f2 provenance remains missing and unwaived.

## Dioxus replay result

The page accepted the imported project before edits. The sequence followed the pinned React packet:

1. Left PCB → `keys` → Column 1; four trusted ArrowRight inputs moved Width from 1u to 2u. The Inspector reported 2u×1u and the page showed Saved.
2. A trusted Control-click on the visible row-0/column-1 key added the part to Column 1. The active context became Key 2.1, the Key size section reported `Mixed`, and its draft remained 2u×1u. Nine scene parts were selected (five switches and their four attached diodes), matching the React packet.
3. Tall applied the shared draft across the mixed selection: 1u×2u. One Height ArrowRight committed 2.25u; twelve Width ArrowRight inputs committed 4u. The rendered overlap message exactly matched React: `Keycaps overlap: left-keys-SW7, left-keys-SW2, left-thumbs-SW1, left-thumbs-SW2, left-keys-SW9, left-thumbs-SW3. Adjust their rows or columns to clear the overlap.`
4. Undo returned 1u×2.25u and removed the overlap warning. Redo restored 4u×2.25u and the same six-reference warning.
5. Reload retained the saved 4u×2.25u geometry. Reselecting Column 1 showed its selected-overlap subset `left-keys-SW7, left-keys-SW2, left-thumbs-SW1`. Tall returned 2.25u×4u with the SW7/SW2 subset; Wide restored 4u×2.25u with SW7/SW2/thumbsSW1.

The final browser reported no page errors. The final Keycaps route showed the same imported fixture with its matrix/profile/settings contextual Inspector and a current clean resolver state: zero findings because this archive has no keycap profile overrides. That clean state does not qualify target-bearing finding navigation; a configured-finding fixture remains required for F6C.4.

## Captures

All PNGs are retained under `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-joined-9d34/`.

| Capture | SHA-256 |
| --- | --- |
| `layout-width-2.png` | `15d6979c510e8fcec21b1cbc0124343b4b24c9f9b94e7a4080dfad92750b890d` |
| `layout-overlap-4u.png` | `3c1a00c281c75d0ada73e392dd267192bb5acc619483c508c84eb0b454daabbc` |
| `layout-redo-4u.png` | `feffa1489489bd34f7fe11132bc03cc568bf8faf1ab18a4d344b1e9ec9f7e2fa` |
| `layout-reload-reselected.png` | `62fb5b2afe0a99533a3eca7a6d6d1eb3ce4cfcec9ac9223b3fbde3b306d703e8` |
| `layout-reload-tall.png` | `725b0913910b38ccc732ddf6db34260d6aa155189dd6b4393884b4604373870c` |
| `layout-reload-wide.png` | `c1c7fa3bc378b0f90523454bda05a4cd2ce8d6e1d55440db383b2f5b98a8059d` |
| `keycaps-workbench.png` | `39982f57209af771c1fe5b3c56da459f89ac4215524b2167c4c1a4ece05b7d4b` |

The parent F6C.3/F3.2/F3.5 acceptance joins remain open. F6C.4/INT.2, F6C.5, and F6.6 remain open. This paired Layout size-control pass is not Keycaps workbench completion.
