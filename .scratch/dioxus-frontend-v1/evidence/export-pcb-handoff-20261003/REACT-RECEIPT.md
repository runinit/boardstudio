# Pinned React full/draft KiCad handoff receipt

**Reference:** React source `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/`.

**Fixture:** `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`.

**Journey:** Open the fixture, choose Export, and activate `Export KiCad board` then `Export Draft KiCad board`. Both controls are enabled on the ready `Left PCB` board. The Dioxus comparison is not claimed by this React receipt.

| Output | Download name | Bytes | SHA-256 | Signature |
| --- | --- | ---: | --- | --- |
| Full | `Sofle-v2-pcb-handoff.zip` | 1,522,941 | `6806f92904db3924c890ba6de21d502f6c6dc7aa68216a099caf5fdac6789b7d` | ZIP `PK` |
| Draft | `Sofle-v2-draft-pcb-handoff.zip` | 1,522,925 | `33a59fd9b3f104461d8593d9b034071824b07de0041fde3a921e842cdcecd0df` | ZIP `PK` |

Both outer archives contain `Left PCB-kicad.zip`, `wiring-report.json`, and `ASSEMBLY.md`. The report records `draft: false` or `draft: true`, revision 12, 29 assignments, and no diagnostics for this ready fixture. The inner ZIP has `Left_PCB.kicad_pcb` (136,274 bytes; SHA-256 `a0b71c10f2f77742d113e0688889df5384e2f4c6244f5521da3eda87d60ac487`) and six model files whose content-hash path matches each payload. Full assembly copy says the package is ready for routing but does not certify routed-board DRC/fabrication readiness; draft assembly copy starts with the explicit `DRAFT` finding-review instruction.

The actual downloaded files and source/DOM captures are retained outside Git under `/home/chris/.local/share/boardstudio/reviews/export-workspace-20261003/full-draft-react/`. This receipt establishes only the pinned React output contract and successful baseline journey; it does not establish Dioxus output parity or F8 acceptance.
