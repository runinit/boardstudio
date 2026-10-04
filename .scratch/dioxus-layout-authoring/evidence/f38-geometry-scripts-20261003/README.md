# F3.8 Geometry scripts editor

## Pinned reference and candidate gap

- React source: `5a472a9426e6e38993361da402cd4ec730feb369`, served at `http://127.0.0.1:5173/` in named session `f38-geometry-scripts-react-5173`.
- Dioxus RED source: `900068a0df413059732f9f365abd598734b7f86c`, served at `http://127.0.0.1:34762/boardstudio/`, provider provenance SHA-256 `319f378ba0d2eb0855059d2368e1e8028c0c6267e0a50a22b9ed8e8a195e1251` in named session `f38-geometry-scripts-red-34762`.
- React browser source path: `app/src/ui/useScriptEditor.tsx`; the existing bounded Core-backed e2e characterization is `app/e2e/workbench.spec.ts` (`runs a bounded script to add an outline hole`).
- Fixture: `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- The browser sessions `f38-geometry-scripts-react-5173`, `f38-geometry-scripts-red-34762`, and `f38-geometry-scripts-red-full-34762` are closed and had empty browser error buffers.

On React, Add object → Board geometry contains `Board outline…` and `Geometry scripts…`. Opening Geometry scripts shows Back to selection, Scripts, and + New script. After creating a new script, `Script 1` is selected, `Name` is `Script 1`, `Rhai source` is empty, `Enable on Apply` is unchecked, and Apply script is disabled. The selector, three draft fields, Core findings list, and Apply action match `useScriptEditor.tsx`. This is the current pinned behavior, not a new script language contract.

On the Dioxus RED candidate, the imported fixture shows Board outline… but no Geometry scripts… entry in the Board geometry menu. Evidence:

- [React Geometry scripts with a new empty script](react-geometry-scripts-new-empty.png)
- [Dioxus Add object missing Geometry scripts](dioxus-add-menu-missing-scripts-red.png)

## Ownership and scope

`ProjectDoc.scripts` and Core `script::apply_scripts` already own persistence, Rhai execution, generated geometry, findings and history. The Dioxus child adds the contextual route, transient form drafts and calls the existing `ReplaceDocument` edit path against the current accepted snapshot. No second store, script runtime, language feature, live-run or delete affordance is introduced. Invalid enabled source is rejected by Core; the accepted document/scene remain authoritative and the draft remains editable.

The root Editor owns workspace switching and the Inspector-page callback. The private editor only commits while its captured accepted session/token/revision is still current and Ready. The existing Core/worker boundary remains unchanged: one typed edit command reaches the existing Core owner. React removal remains gated on integrated F3.8 route, draft/reset, Core output/findings, rejection, history and reopen parity with equivalent behavioral coverage. RF-001, RF-006 and RF-009 are preserved; no new refactoring takeaway was observed.

## Candidate GREEN receipt

On 2026-10-04, the published Dioxus candidate
`frontend-outline-reopen-final-20261004` (source
`3f9e8e2c4475c92cb0700ab5a17732b5d250ca01`, root route
`http://127.0.0.1:34796/`) and the pinned React app at
`http://127.0.0.1:5175/` completed the same Sofle v2 public journey in
separate named browser sessions. The Dioxus package proof is
`.scratch/dioxus-frontend-v1/evidence/frontend-outline-reopen-final-20261004/package-proof.json`.

In both apps, Layout → Add object → Geometry scripts → New script produced
`Script 1` with a blank source, disabled Apply and Enable on Apply off after
the accepted edit settled. We named it `Probe hole`, entered
`rect("extra", "hole", 30.0, -10.0, 3.0, 3.0, 0.0, "subtract");`, enabled
it and applied. The public Layout SVG then contained one hole on each app.
Both showed the same two Core corner-size warnings, including the 0.022 mm
and 0.023 mm minima. One Undo removed the hole and restored the blank,
disabled script; one Redo restored the hole and applied name. Reload kept the
hole. The Dioxus editor selected the saved `Probe hole` and displayed the
same source and enabled flag after reopening its Inspector route. The browser
error buffers were empty and both owned sessions were closed.

Portable project exports after reload are
[`dioxus-probe-hole-34796.boardstudio`](dioxus-probe-hole-34796.boardstudio)
(SHA-256 `afee6cd90b786608c70b9a64517a0dbe7609981576c2c77ac9a867e21b7a05c2`)
and [`react-probe-hole-5175.boardstudio`](react-probe-hole-5175.boardstudio)
(SHA-256 `6520dd1fbc0c3665348fd18e6ff511d9a7f695f4df1baffcaa0d01205423683d`).
Each `project.json` contains one enabled `Probe hole` script with the exact
Rhai source. Dioxus persisted ID
`geometry-script-058e2693-43cb-4b8c-80f3-023c430990d4`; React used its
own `ui-0c726749-4de1-4a48-9885-0c7ba3e65377` ID. These are different
valid identities, not an expected cross-app ID match.

This qualifies F3.8-C01 and C03 on the current candidate, including Core
execution, one-step Apply history and persisted script data. F3.8-C02 remains
open: the blank-source guard was observed, but the full set of draft reset
boundaries has not been paired. Parent acceptance still depends on that
criterion and its F3.5 join. No new architectural finding was observed.

## F3.8-C02 reference-boundary receipt

On 2026-10-04, named sessions `f38-c02-reference-5175` (pinned React
`5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5175/`) and
`f38-c02-candidate-34796` (published Dioxus
`3f9e8e2c4475c92cb0700ab5a17732b5d250ca01`,
`http://127.0.0.1:34796/`) repeated the draft boundary scenarios on Sofle v2.

For script selection, each app created `Script 1` and `Script 2`; on Script 1
we entered Name `Unsaved left draft`, source `left draft source`, and enabled
the checkbox, then selected Script 2. Both showed accepted Script 2 defaults:
Name `Script 2`, blank source, Enable on Apply off. This is evidence that
selection discards the unsaved Script 1 fields. For board switching, on
Script 2 we entered Name `Unsaved board draft` and source `board draft
source`, then changed Left PCB to Right PCB. Both retained those exact fields
and the off checkbox. Board switching is therefore not a draft reset boundary
in the pinned reference or this candidate.

For project switching, with that unsaved Script 2 draft visible, both apps
started the REVIUNG41 demo (candidate project board `Keyboard PCB`). React
returned to the default Board Inspector and closed Geometry scripts. Dioxus
kept the Geometry scripts panel mounted with an empty Scripts list and `+
New script`; no old draft fields were present. Creating a script in the new
project showed `Script 1`, blank source, Apply disabled and Enable on Apply
off. The old project draft did not reappear. Thus draft values reset across
the project switch in both; the panel route itself differs (closed in React,
still open and empty in Dioxus), so this receipt does not claim route parity.

Both browser error buffers were empty, and both named sessions were closed.
This qualifies the characterized script-selection, board-switch and
project-switch draft boundaries for F3.8-C02. No stale draft or submission
was observed; the project-switch panel-route difference is recorded as an
observed UI difference, not classified as a draft-reset defect.
