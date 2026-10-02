# PCB08 paired browser walkthrough (open)

## Reference journey

Use the TypeScript application at the retained reference build `http://127.0.0.1:5175/` and the exact packaged Dioxus candidate identified by the integration owner at test time. Open **Sofle v2**, enter **PCB**, and select the left rotary encoder (`left/SW25`). In `app/src/demos/sofle.ts`, this part is created from `ceoloide/rotary_encoder_ec11_ec12`; it is explicitly excluded from the keyboard matrix. In `app/src/ui/usePcbWorkspace.tsx`, this exact generator source still receives the Press input branch, with `Direct GPIO` as the default and Matrix disabled when the part has no accepted matrix membership.

Compare the same right Inspector in both applications: `Press input`, `Scan mode`, selected value, the Matrix/Direct GPIO/Unassigned choices, and the note that changing press connection requires applying the board wiring plan. Change Direct GPIO to Unassigned. Verify the saved part's `properties.pressScanMode`, one accepted revision, Undo to Direct GPIO, Redo to Unassigned, then save/reopen and confirm the value persists. Change selection to a built-in switch and controller to confirm React's earlier inherited-Wiring and board-Wiring branches remain intact; PCB08 must not route either into its generic controls.

## Generator-binding journey

The stock Sofle fixture contains no generic `infused-kim/smd_0805` part, so do not claim the generated net-binding controls were paired on that demo. Use a source-stamped project fixture containing one board-member `infused-kim/smd_0805` part and at least two named nets on the active board. In both apps, select the part and compare the `Ergogen bindings` fields (`net_1_from`, `net_1_to`, and remaining supported nonterminal net parameters), option ordering and default value. Choose a net, clear it to Default, verify out-of-board net choices cannot be submitted, and check accepted document, Undo/Redo and save/reopen. Preserve another board's nets and the part's other generator overrides.

The checked-in packaged module currently reports no `anchor`-typed parameter in its 36-source catalogue; the generic binding form supports anchors conditionally but there is no current package-backed anchor field to pair. Keep that scenario open until a real packaged generator exposes one. The 2-test browser harness imports the exact generated package and currently asserts real `infused-kim/smd_0805` schema values; it does not stand in for the Inspector journey.

## Gates still open

- The Editor's real mounted PCB scene/selection path must expose `left/SW25` and the generic-part fixture in the packaged Dioxus UI.
- Capture paired screenshots/snapshots, actual values and browser-console results from the same source-stamped build.
- Verify the ordinary edit/history path and persisted reopen for each control, plus stale selection/workspace rejection under delayed packaged schema loading.
- Verify Light, Dark and System themes and narrow Inspector scrolling.
- Keep F5.2, F5.3 and F5 parent criteria open; this packet covers neither terminal/pad assignment nor board mode/Apply/remap.
