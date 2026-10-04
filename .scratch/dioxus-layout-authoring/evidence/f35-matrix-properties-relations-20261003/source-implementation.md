# F3.5 matrix-context Properties / Relations source implementation

**Source base:** `e57fd3a87a8b3821ac5458cf8d2988a596e14bfc`. **React reference:** `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx:1319,1400-1401`.

Dioxus now shares the Layout Properties/Relations tab state between its existing standalone-component Inspector and Matrix/Row/Column/Key contexts. Matrix-owned Relations projects the pinned precedence (linked layout, independent layout, active selected-part constraint, then empty state), keeps the matrix geometry note, offers the reference Edit placement relationship route for a live selected member, and is reachable from Align → Relationships. The action reuses the current accepted scoped tree-selection callback. Existing matrix structural/transform controls, component constraint editor and findings page remain with their current owners.

The accepted source also now resolves matrix-member layout context like React and hides standalone Layout assignment for those members. This is needed when the Relations action enters the explicit component context. No new edit operation, document store, public API or refactoring finding is introduced; preserve RF-001 and RF-006.

**Qualification:** Source implementation only. No build, tests or browser journey was run for this packet; integration compilation and the changed mounted journey remain open. This does not accept F3.5 or its parent criteria.

## Pick origin focused qualification (2026-10-04)

**Verdict: partial; Escape cancellation fails on the Dioxus candidate.** Candidate
`14434d50` was served at `http://127.0.0.1:34800/`; the coordinator verified the
live root/subpath HTML and JavaScript hashes against this packaged source. The
pinned React reference was served at `http://127.0.0.1:5175/` from the existing
`5a472a9426e6e38993361da402cd4ec730feb369` reference. Both sessions used the
editable **Start Sofle v2** demo, Left PCB, and Layout → Transform → Splay & origin
to select `keys · Column 1`; no byte-identical archive fixture was available.

On Dioxus, with Splay set to 12°, Pick origin followed by a blank-canvas click
changed the accepted Origin X/Y from about `9.18 / -78.048` to `160.68 / -32.03`.
One Undo restored the original origin while leaving Splay at 12°, demonstrating
that the pick is one undoable accepted edit in the active Sofle project. The
sampled key-cell transforms showed no material displacement from the origin
change. Undo then restored the temporary Splay setup to 0°.

Escape cancellation differs: after arming Pick origin, pressing Escape and then
clicking the blank canvas still changed the Dioxus origin (to about
`155.95 / -34.55`). Undo restored the original origin. On React, the same
sequence left the origin unchanged; its handler clears `originPicking` on Escape
(`app/src/ui/Workbench.tsx:541-546`), and the Inspector action arms it at
`Workbench.tsx:1095`. The React canvas handler applies the point at
`Workbench.tsx:847`. The reference was returned to Splay 0° and its original
origin by Undo.

The action and one-step Undo work on the current project and selected column.
The Dioxus pending-pick route still needs Escape cancellation before this
focused interaction can pass; this result does not close F3.5-C01 or F3.5.

### Escape cancellation repair: paired GREEN (2026-10-04)

On repaired candidate `9eda1b5d4c194a97b7216a43544036b5709a65f8` at
`http://127.0.0.1:34801/`, the same editable Start Sofle v2 demo was opened on
Left PCB and `keys · Column 1` selected. The Pick origin button retained focus
when clicked; pressing Escape and then clicking the empty canvas left Origin
X/Y unchanged at about `9.18 / -78.048`. Undo availability was unchanged across
the cancellation. A subsequent Pick origin and canvas click changed Origin X/Y
to about `158.14 / -42.16`, showing the canceled arm did not disable a live pick;
one Undo restored the original origin. The project remained Sofle v2 and the
selected context remained Column 1.

The pinned React reference at `http://127.0.0.1:5175/` uses the same Start
Sofle v2 / Left PCB / Column 1 route. Its Escape cancellation and subsequent
live-pick behavior were observed in the paired run above (`f35-pick-react`);
no duplicate reference journey was run for this repair. The focused Escape
journey is GREEN. F3.5-C01 and F3.5 remain open for their other acceptance
clauses.

### Paired mounted browser follow-up (2026-10-04)

One isolated `agent-browser` session used React `http://localhost:5175/` and
Dioxus `http://localhost:34801/`. Each origin imported the same saved fixture:
`/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/initial-sofle.boardstudio`.
No saved-browser state was shared between origins.

**C02 — partial.** On React, Layout tree → `keys` → Properties, Rotation `0 → 5`
with Enter, then Undo restored `0`. The same action on Dioxus left a `5°` field
draft and showed: “The saved transform does not contain the requested value.
Review the current field and retry.” Undo left the draft unchanged; a subsequent
Undo reported “History is empty.” Dioxus Pitch X `-1` showed
`aria-invalid=true` and “Enter a finite number greater than zero.” Escape closed
Inspect and restored `19.05`. This Sofle fixture has independent `keys`/`thumbs`
layouts and no existing driven or locked relation, so those states are
unqualified.

**C03 — partial.** Both full findings DOMs contained exactly two live outline
warnings (Left PCB and Right PCB), each with a working `Show outline` action.
That action selected the Generated outline and fitted it (React displayed 84%;
Dioxus 80%). Dioxus disabled Fit selection in the resulting outline inspector.
For component focus-fit, selecting `left-U1` and using Fit selection reached
213% in React. On Dioxus, the SVG part target was initially covered by the open
inspector; closing it exposed the target, and Fit selection reached 206%. No
missing/stale finding target occurs in this fixture, so that branch is
unqualified.

**C04 — partial.** React keyboard Enter on `Back to selection` from the outline
inspector returned to the prior `left-U1` context and focused its Properties tab.
After Dioxus `Show outline`, the full interactive DOM exposed no Back to
selection control and focus was on Close inspector. At 720×800, Dioxus kept its
370 px inspector usable; Tab from Relations focused Edit placement relationship.
This covers the compact relation-action focus path, while the outline
return-path remains unqualified on Dioxus. C01 Escape evidence above was reused;
Escape was not repeated for that criterion.

This receipt records mounted behavior only; no source, tracker, or run files were
changed.

### Rotation worker boundary diagnostic (2026-10-04)

On Dioxus `http://localhost:34801/`, a fresh browser-origin import of the same
`initial-sofle.boardstudio` fixture started `left-keys` at rotation 0 (project
revision 3). After selecting Layout → `keys` → Properties, filling Rotation `5`
and pressing Enter, the mounted field read `5` with no saved-transform mismatch
alert. A post-action `.boardstudio` export had revision 6 and `left-keys.rotation`
`5.0`; the pre-action export had revision 3 and no rotation value. One Undo
returned the field to `0` (undo reply scene revision 7).

A boundary tap on the existing Worker and MessagePort `postMessage` methods
captured the edit as outer Worker `kind=core`, `request_id=m1-4`; its frame was
`kind=edit`, `id=m1-4`, command phase `commit`, `baseRevision=5`, operation
`set-matrix`, target `left-keys`, rotation `5.0`. The matching Worker reply used
`request_id=m1-4`, frame `kind=scene`, with document revision 6 and
`left-keys.rotation=5.0`. The only subsequent worker traffic before export was
keycaps/electrical resolution; no Open/replace request arrived in this edit
window. Undo was Worker `core` request `m1-5`, frame `kind=undo`; reply scene
revision 7 restored the rotation to 0. All observed traffic used Worker; no
MessagePort traffic was recorded. Thus this mounted build did not reproduce the
saved-transform mismatch noted in the earlier paired follow-up; the earlier
failure remains unqualified pending reproduction on its original build state.

### Updated Dioxus finding-return check (2026-10-04; source `f399d8c2`)

On `http://127.0.0.1:34803/`, a fresh import of the saved
`initial-sofle.boardstudio` fixture selected the SVG component target
`left/U1` (`left-U1` inspector, Properties selected). Layout findings showed
both outline targets; `Show outline` for Left PCB selected the Generated outline
and exposed a visible `Back to selection` button. Focusing that control and
pressing Enter returned to the prior `left-U1` component inspector with
Properties selected and keyboard focus on the Properties tab. This covers the
Dioxus C04 finding-return path on the updated package; prior paired React and
34801 observations remain as recorded above. No component-target branch was
missing in this fixture. No source, tracker, or run files were changed.

### Paired contextual Inspector selection check (2026-10-04)

React `http://127.0.0.1:5175/` and updated Dioxus candidate
`http://127.0.0.1:34803/` each imported the same saved
`initial-sofle.boardstudio` fixture. In the empty part-selection / Left PCB
context, both showed the Board name field and no Properties/Relations tabs. On
selecting the `keys` Matrix, both exposed Properties and Relations; Properties
contained matrix setup/transform controls, and Relations showed Relationships
and Edit placement relationship. The visible matrix relation control was
present in both full interactive DOMs.

**First mismatch (stopped here):** with `keys` selected, switching both
Inspectors to Relations and then selecting tree child `Key 1.1 left-keys-SW1`
reset React to Key Properties. Dioxus updated the selected-context heading to
`keys · Key 1.1` but retained Relations and displayed Relationships, hiding the
key's Properties. This is a tab-scope mismatch when the selected object changes,
not a missing control. The receipt's earlier component/finding journey remains
reused; no additional component or finding flow was run. Multiple selection,
its Inspector state, and single-component Relations in this paired pass remain
unqualified because the journey stopped at this first divergence; C01 remains
partial.

The focused regression `matrix_to_key_selection_resets_relations_tab_to_properties`
was RED before the fix: after Relations then Matrix→Key, selected tab remained
Relations (expected Properties). The Layout-owned shared tab hook now compares
full `ScopedTreeContext` and resets to Properties only on context change; same
context retains the tab across unrelated accepted revisions. GREEN:
`cargo test --target wasm32-unknown-unknown --bin boardstudio-web matrix_to_key_selection_resets_relations_tab_to_properties`
with the wasm-bindgen Chrome runner passed (1 passed, 216 filtered). Rustfmt
check passed for the three touched Layout files. The 34803 package was not
rebuilt or rechecked; no TS source, package, tracker, or run file was changed.

### C01 matrix→Key tab reset retest (2026-10-04; source `58145c17`)

On updated candidate `frontend-context-gap-20261004` at
`http://127.0.0.1:34804/`, a fresh import of the same saved Sofle fixture
selected `keys`, switched Relations, expanded Column 1, and selected tree child
`Key 1.1 left-keys-SW1`. The inspector context became `keys · Key 1.1`, Properties
was selected, and Key Properties (Local X/Y and Key rotation) were visible;
Relations was no longer selected. This is GREEN for the exact Matrix→Key
transition that failed on 34803 and was already covered by the focused RED/GREEN
regression. The paired React 5175 RED comparison and regression receipts above
are reused; no other selection contexts were tested. Browser session closed.

### C04 Board and Outline return focus follow-up (2026-10-04; source `58145c17`)

Sol review found that the Back return effect only focused the selected Properties
tab. That covered a prior component selection but left keyboard focus without a
destination after returning to Board or Outline contexts. The return effect now
focuses the first enabled Board or Outline inspector control, and falls back to
the restored inspector context when its controls are disabled. Component returns
continue to focus Properties.

The focused mounted wasm regression dispatched Enter on Back in Board and
Outline inspector DOMs and verified focus moved to the Board name and Active
outline controls. A stale board scope left focus on Back and did not redirect to
the other board. The same regression keeps a valid empty Outline context
returnable. Against the previous tabs-only focus lookup, the focused test was
RED: Enter left focus on Back instead of the Board name input. After the
context-aware focus lookup, `wasm-pack test --headless --chrome --mode no-install web
--no-default-features --features page --bin boardstudio-web --
mounted_back_enter_focuses_board_and_outline_inspectors_and_rejects_stale_scope`
passed (1 passed, 0 failed). `rustfmt --edition 2024 web/src/presentation.rs`
and `git diff --check -- web/src/presentation.rs` passed. This source-level
mounted regression does not qualify a paired public React/Dioxus journey.
