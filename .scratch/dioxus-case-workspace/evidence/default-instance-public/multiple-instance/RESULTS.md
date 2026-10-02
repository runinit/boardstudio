# Same-board physical-instance preference: public fixture gate

## Result

The requested public regression sequence is **unverified** because I could not create the required accepted project through the public UI. I stopped before exporting/importing or claiming behavioral evidence. The candidate only offered a genuine one-instance REVIUNG project in its fresh task-owned profile. The public setup panel can replace the current hardware plan with one unibody assembly or two split halves; it has no action to add a third assembly while retaining an existing pair. Creating a second board through React's `New board` action did not provide a way to retain two instances on board A and add one for board B.

I did not mutate an archive, IndexedDB, React/Dioxus state, or inject test rows. Therefore no `.boardstudio` archive was produced for this case and there are no reference/candidate import results.

## Candidate observation

- Candidate URL: `http://127.0.0.1:34665/` (parent identified this as candidate commit `ed242c0258f59af6fde65d8ffe1580334e2836c4`).
- Browser: agent-browser 0.38.1, named isolated session `default-instance-candidate`; profile `/var/tmp/frontend-run/default-instance-multi/candidate`.
- Opened the real built-in `REVIUNG41 copy` from the candidate's public start screen, then navigated to Case. The Objects panel showed a single board (`Keyboard PCB`) and a single physical-instance button (`main`). The captured public snapshot and screenshot are `candidate-fresh-case.snapshot.txt` and `candidate-fresh-case.png`.
- The candidate does render public instance buttons in Case, confirming the selection surface exists. It does not supply the requested 2+1 fixture in a fresh profile.

## Public producer/source seam

The React producer was opened in a separate task-owned session/profile at `http://127.0.0.1:5173/` (`default-instance-producer`, `/var/tmp/frontend-run/default-instance-multi/producer`). A real `REVIUNG41` demo was opened through the public start screen; `New board` created a second board. The public Setup guide exposed `One keyboard` and `Split keyboard`. Choosing split changes the hardware plan to two halves for the selected board. There is no Add assembly control in this workflow, and reconfiguring to a single keyboard replaces the split plan.

Source verification:

- `app/src/ui/HardwareInstancesPanel.tsx` defines only `configure(false)` / `configure(true)` from the public One keyboard / Split keyboard controls; `configure` creates at most one or two instances and replaces `hardware.instances`.
- `web/src/presentation/objects.rs` filters accepted `hardware.instances` by `board_id` and renders selection buttons only for those existing instances. It contains no instance-creation control.
- `web/src/presentation/instance_selection.rs` resolves the preference against those accepted instances and board order; it is not a public fixture-creation seam.

At the time of inspection, `migration-m1-continuation-20261001` was at HEAD `ed242c0258f59af6fde65d8ffe1580334e2836c4`. Source hashes:

```
39954b6247b2f0529bd26a3206ad199afea060597242da2feb35a4011176d9d5  web/src/presentation/objects.rs
28dd03ba57c403940ae3ab9d9e7c11c039c5ce3d933d2c9587f3a72c6a07a2b9  web/src/presentation/instance_selection.rs
eb82625d5546ed481aa91c897ac92b1bd74f846beb218ef47d4b0aa5308e857e  app/src/ui/HardwareInstancesPanel.tsx
```

A prior public Sofle archive exists under `.scratch/dioxus-frontend-v1/evidence/case-keymap-current/keymap-layered-public/fixture/`; its accepted document has two distinct boards (`left`, `right`) and one instance per board. That is a real project but does not exercise two instances sharing board A, so I did not treat it as a substitute.

## Proof limits and next seam

The selection behavior for A2 → B1 fallback → retained A2, and the explicit B1 preference reset → A1, remains untested in the public browser. A valid follow-up requires a supported public Add assembly workflow or an already accepted/exported project whose actual `hardware.instances` contains two IDs for one board and one for another. The native preference tests and UI code review are separate evidence and do not establish this browser behavior.
