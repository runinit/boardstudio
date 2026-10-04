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
