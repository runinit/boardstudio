# PCB board-mode control after an early workspace switch during import

This is a separate reproduction from the F5.2d Apply source packet.

## Pinned inputs

- Served candidate: `frontend-chooser-pcb-navigation-integrated-20261002`, source `a8fd8988f649c70c11393066314095447535dce5`, provenance SHA-256 `183c7cd7dd133c6cdaf9c423d49877e37b1bfbe0ed2469e85a26fb02dd048c52` at `http://127.0.0.1:34739/boardstudio/`.
- Independent package review: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/sol-review-wave-20261002/chooser-pcb-navigation-package-review-a8fd8988-sol-20261002.md`, SHA-256 `6e3854102e893a4fa457649af1275d6fbaa3139667d5b8cb9a040edd6acd7b77`.
- Imported archive: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Browser: isolated named agent-browser session with its own Chrome profile at `/home/chris/.local/share/boardstudio/browser-profiles/pcb-apply-fixture-race-20261002`.

## Reproduction

Imported the archive through the normal file input and switched to PCB immediately. After import settled, the page showed Left PCB, physical instance `left half`, the accepted controller `left-U1`, plan readiness, and revision 9 Saved; both Wiring mode options were disabled. The screenshot is `immediate-import-pcb.png` (SHA-256 `1ae95081fc29c0ae7fea22c46fcefd982930cf037bd8c83685eb729dc094764a`).

Reloading returned the editor to Layout. Selecting PCB again for the same revision 9 Saved document and current plan made Matrix and Direct GPIO available. The settled screenshot is `after-reload-pcb.png` (SHA-256 `fead23119fec9d960e947cf45e60cd594361a9a6316401252782dbedfc44c144`). No board edit was made during this reproduction. Root independently reproduced the disabled-before-reload/enabled-after-reload transition in a second named profile.

The source-level Apply/mode owner gate is `current_snapshot`: it requires PCB workspace, a current instance selection, Ready and Saved Session state, no display preview or gesture, matching active board/instance and document/session scope, matching plan identity, selected part and selection generation. The public snapshot does not expose enough information to attribute this reproduction to one of those guards. The failure therefore remains open for a focused production-mounted import/session transition regression; this receipt does not prescribe a code change or add diagnostic public API.
