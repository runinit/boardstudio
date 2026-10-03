# F5.5 mounted-module placement Save/reopen receipt

Candidate: `http://127.0.0.1:34765/boardstudio/`
Frozen source: `17ba32b984c13f3621a2145efe1dd478862ab5cd`
Candidate provenance SHA-256: `273ca2fa2315b32fec91b0490d2f70fb03b5426d759310bced6e934ad73947d5`
Fixture: `vik-module-review.boardstudio`, SHA-256 `952b26565a4df3ae1122fd0716017560279c938ee19fb6c15af966c88c1d35b0`
Named browser profile: `pcb-mounted-module-inspector-20261003`

Selected existing instance `review/splitter-above` in the PCB canvas. The accepted export before editing was revision 6: X 29 mm, Y 20 mm, yaw 0°, gap 3 mm, host face Front, facing face Back, attachment Board. Changed the local draft to X 32.5 mm, gap 5 mm, host face Back and attachment Case. Layer-menu rerender retained the unsaved draft. Save through the mounted Inspector's ordinary action path produced accepted revision 7. Exported `project.json` was parsed directly from the retained `.boardstudio` archives and contained the expected values. `serviceClearance=4`, both mount supports and the existing connection remained present and unchanged. After reloading the page, selecting the same module showed the saved placement values.

Existing screenshots and archive downloads are retained under `/home/chris/.local/share/boardstudio/reviews/pcb-mounted-module-inspector-20261003/`:

- `placement-before.png` SHA-256 `621eb60fba229127640977f45d4a3fa1d1cc0fbfe79ca85a5eb5b2fed73d41f8`
- `placement-draft-retained.png` SHA-256 `0430e525c291ce2b08f2f4bf1518478ff22a5f6e64374712a19561bfcd6e3106`
- `placement-after-save.png` SHA-256 `e81b62d5ac711ae9e1e0fca3f9119113686508df28cf522f4a128a938e298b19`
- `placement-after-reopen.png` SHA-256 `dc4202380e6aaefbc2961942da7f2d1bf96ccb596ea558521bc922d4fba4ae33`
- `before.boardstudio` SHA-256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`
- `after-save-before-reload.boardstudio` SHA-256 `9cfc7be0593fa7847310956ca08b9641318a2c2eb73c1f5bf0f0d60dfdb849d6`

The settled mounted Undo/Redo DOM check is recorded separately at `/home/chris/.local/share/boardstudio/reviews/pcb-mounted-module-inspector-20261003/undo-redo-settled-34765.md` (receipt SHA-256 `90dc01973a42657cae720e46110da3250b6e8455f11aa7efe71cd1f0189ac47b`). This receipt covers ordinary placement persistence only. Service-clearance/support controls and their save journey are the next child; no complete F5.5, F4.7 or parent acceptance is claimed.
