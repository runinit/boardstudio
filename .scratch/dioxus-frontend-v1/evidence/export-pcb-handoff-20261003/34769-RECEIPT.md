# Dioxus full/draft KiCad journey — candidate 34769

**Candidate:** source `bbd4da1b7cc0609dd4ae6d8ec0332031b0690ea1`, public route `http://127.0.0.1:34769/boardstudio/`, provenance SHA-256 `38e1980f3c38f31af1d66676a56d2d6cf57c7a3e4130dea6be2d1cb0eeb2f680`. Root's package proof is [34769 package proof](../candidate-34769-20261003/package-proof.json). This receipt qualifies only the changed full/draft handoff journey; no broad output matrix was run.

**Browser:** isolated agent-browser session `keycaps-export-34769-20261003`. Imported the exact retained fixture `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`, SHA-256 `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`, then opened Export on `Left PCB`. Both full and draft KiCad actions were enabled. The session reported no page errors; the console was cleared after the full download before running the draft action.

| Output | Filename | Bytes | SHA-256 | Signature |
| --- | --- | ---: | --- | --- |
| Full | `Sofle-v2-pcb-handoff.zip` | 1,522,633 | `9a4a0f200bbdfab33832c0bfd2500503a6b0baa8d18c64ec2e9c3441b182c05a` | ZIP `PK` |
| Draft | `Sofle-v2-draft-pcb-handoff.zip` | 1,522,618 | `5ea5f5033110cdd142525e086b0b2ef3fb8b07a1e1b688244491f6b7811ccc7a` | ZIP `PK` |

The files are retained outside Git at `/home/chris/.local/share/boardstudio/reviews/export-workspace-20261003/dioxus-full-draft-34769/`. Both outer archives have exactly `Left PCB-kicad.zip`, `wiring-report.json`, and `ASSEMBLY.md`. Parsed report values match the pinned React baseline: selected `left` board, revision 12, 29 assignments, zero findings, one population, and the correct `draft` boolean. Both nested board ZIPs are byte-identical to the React baseline and contain the same six content-hash model files plus `Left_PCB.kicad_pcb`; the board file is 136,274 bytes with SHA-256 `a0b71c10f2f77742d113e0688889df5384e2f4c6244f5521da3eda87d60ac487`.

The paired comparison found one small remaining copy mismatch: the React assembly document adds `# PCB wiring and assembly` after the population heading, while 34769 omitted it. The source correction adds that heading; the 34769 immutable artifact remains the red receipt for this copy detail. Full/draft artifact generation, board bytes, model entries and report semantics were green on this candidate. The accepted fixture already had the resolved wiring applied, so this journey does not prove the conditional apply-wiring branch or a failure/cancellation race. The integrated history/durability gate and corrected-copy requalification remain open.
