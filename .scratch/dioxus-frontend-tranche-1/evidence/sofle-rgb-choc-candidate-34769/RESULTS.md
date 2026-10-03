# Sofle RGB / Choc candidate journey — 34769

Candidate URL: `http://127.0.0.1:34769/boardstudio/`  
Served source: `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`  
Package provenance SHA-256: `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`  
Package proof: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/candidate-34769-20261003/package-proof.json` (22 commands, 1,373 source inputs, 154 assets on each route, zero mismatches).

The pinned React comparison is [`../sofle-rgb-choc-reference-20261003/REFERENCE.md`](../sofle-rgb-choc-reference-20261003/REFERENCE.md), source `5a472a9426e6e38993361da402cd4ec730feb369`. It records the reference variant data and repeated Choc identity behavior.

## Journey

On a fresh candidate browser profile, the Open a keyboard library showed the Sofle v2, Sofle RGB, and Sofle Choc cards, each with a key-layout preview and `58 keys · 2 boards` summary. The Project menu also exposed Start Sofle RGB and Start Sofle Choc. The demo-card screenshot is [`demo-cards.png`](demo-cards.png), SHA-256 `5352b516d3e9e19ad24b2e86a82efbacbea3f7ea0c5500c5d4747fd482d0b4b6`.

Starting RGB produced `Sofle RGB`, project ID `1b6fda53-d6db-46b5-983b-00214794b862`, revision 2, two boards, 212 parts (106 per board), and 14 SK6812 underglow parts: D31–D37 on the back of each board. The matrix parts use the MX hotswap RGB generator family. The document carries seven referenced STEP models: `Nice_Nano_V2.step`, `OLED-Module-with-Pins.step`, `PJ-320A.step`, `SK6812MINI-E v1.step`, `SW_Hotswap_Kailh_MX.stp`, `SW_Cherry_MX_PCB.stp`, and `Diode_1N4148W.step`.

Starting Choc twice produced distinct saved project IDs: `fa0027af-4a5c-4d0c-a8ea-a0616a7d80e4` and `4c0a1088-d3e2-4671-9f84-313cc83e7303`. Both are `Sofle Choc`, revision 2, two boards, 198 parts (99 per board). Each side retains a 24-key plus 5-thumb matrix, Choc V1 hotswap switch-family parts, and no D31–D37 underglow parts. The documents reference `Nice_Nano_V2.step`, `OLED-Module-with-Pins.step`, `PJ-320A.step`, `Choc_V1_Hotswap.step`, `Choc_V1_Switch.step`, `Choc_V1_Keycap_MBK_Black_1u.step`, `Diode_1N4148W.step`, and `SK6812MINI-E v1.step`. The RGB and Choc model assets remained attached to their respective saved documents.

The Project menu listed all three saved copies. Opening the newest Choc saved card returned to that same revision-2 document; its current card was visibly marked and the other Choc copy remained independently listed. [`saved-copies.png`](saved-copies.png) SHA-256 is `1c58729002261f1fb81ee7b13b90e7b4e02e274f7818dd63f673bbc5694c7bca`.

The exact packaged fixture hashes are:

| Fixture | SHA-256 |
| --- | --- |
| `sofle-rgb.boardstudio` | `e61e0eb2ffba5035fee65461af243d69665f2109b3be1a0b384b17882006e1a6` |
| `sofle-rgb.json` | `1b31fcd872a315c045094f8d70b3565fee5d7ff41f61ea97de4fbf5ff3e37185` |
| `sofle-choc.boardstudio` | `a9e4d7b21e74241193b247c503d9247fe56a62fc1f54431d216218c3e5ef8d3d` |
| `sofle-choc.json` | `8bbcedec174db6ecaad4a180e9d0a9e9d5aa4a91787848aa38178afa5909d347` |

The fixture continuation retained the earlier failed artifacts and changed only `scripts/prepare-m1-fixtures.mjs`; the RGB URL-encoded model filename regression was recorded RED/GREEN by the package owner. Generic `.boardstudio` import identity behavior is reused from Issue 20 and its direct shared-import-path evidence; it was not exercised again here.

## Scope and remaining acceptance

This qualifies the changed cards, representative variant document/model content, repeated Choc fresh-copy identity, and a saved Choc reopen. It does not cover a representative edit plus Undo on each new variant, RGB saved-card reopen, preview/load failure or supersession injection, or all F2.1 family criteria. No additional code change was needed in this journey. The existing RF-001/provider and accepted-project ownership criteria remain in force; this result does not close the demo lifecycle or parent tickets.
