# Paired Case body authoring trace

Fixture: exact Sofle archive hash `0e1e06beabfce1c5a2d6bfc472c85ed281435ff2382174b32f9a2bd3d6d58899`; fresh task-owned candidate/React disk profiles. Added one Right PCB plate via each public `+ New case body` control; only generated IDs differ.

- Public Body type → Tray matched in both (revision 5).
- The earlier Thickness 3→4 + Enter observation used the invalid command form `press @e... Enter`; it is not keyboard evidence. A corrected fresh paired run used `fill` on the focused Thickness input and `press Enter`, committing 4.5 in both apps (revision 5→6). See `.scratch/dioxus-case-workspace/evidence/default-instance-public/case-inputs/RESULTS.md`.
- Z offset draft 0→2, then switch Case→Keymap→Case: blur committed 2 in both (revision 7).
- Public Undo restored saved Z exactly to 0 in both (revision 8); Redo restored 2 in both (revision 9).
- The earlier Invalid Thickness=-1 + Enter observation also used `press @e... Enter`; the claimed absence of an app error is invalid driver evidence. A corrected fresh paired run showed the app validation message in both and left persisted thickness unchanged at 3/revision 4. See `.scratch/dioxus-case-workspace/evidence/default-instance-public/case-inputs/RESULTS.md`.
- Escape exploratory commands in this packet used the invalid selector-plus-key syntax. Correct `press Escape` reruns are recorded in `.scratch/dioxus-case-workspace/evidence/default-instance-public/case-inputs/RESULTS.md`: candidate resets the draft to 0 while React retains the visible 1.5 draft; both persisted values remain 0/revision 5. Treat that as an observed public UI difference, not as persisted-data divergence. `ESCAPE_DRIVER_NOTE.md` documents the superseded invocation.

Paired screenshots after successful tray/redo are at `../sofle-candidate-case-tray-redo.png` and `../sofle-reference-case-tray-redo.png`. `candidate-final.txt` and `reference-final.txt` are accessibility snapshots. Readonly current IndexedDB outputs are included.
