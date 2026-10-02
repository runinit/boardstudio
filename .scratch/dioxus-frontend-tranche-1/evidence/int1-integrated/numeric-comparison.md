# INT1 public Inspector numeric behavior trace

## Setup

- Baseline server: `http://127.0.0.1:34643/`, immutable production build `f44a3d1b` served by the existing PID 1190604.
- Candidate server: `http://127.0.0.1:34645/`, production build `598b2c`.
- Browser sessions: `frontend-int1-driver-before5` and `frontend-int1-numeric-after` (separate agent-browser sessions).
- Fixture: `REVIUNG41 copy`, opened from the first visible landing Library entry. No saved board was edited; the copy lived in the isolated browser session.
- Visible target: `matrix/main-right-keys/r0c0`; starting X=147.285, Y=-51.02, transform `translate(147.285,-51.02) rotate(10) `, document revision 3. The second target was visible `matrix/main-right-keys/r0c1`.
- Driver: `run_numeric.py SESSION URL OUTDIR`. It uses only visible Library, hit-area, X input, tab and Undo/Apply controls, with DOM reads of their displayed values/transforms.

## Observed before and candidate results

| Action | Baseline | Candidate |
|---|---|---|
| Fill X with 150.285 | Visible transform previews at X=150.285; revision stays 3; “Preview only…” status appears. | Same. |
| Press Escape | Input and visible transform return to X=147.285; revision remains 3. | Same. |
| Fill X=150.285, press Enter | Commit reaches revision 4; Undo restores X/transform at revision 5. | Same. |
| Fill X=150.285, click Apply position | Commit reaches revision 6; Undo restores X/transform at revision 7. | Same. |
| With X preview active, select visible r0c1 | Old r0c0 preview returns to its start; inspector switches to r0c1 (X=165.415); revision unchanged. | Same. |
| With X preview active, switch to PCB then back to Layout | Inspector input unmounts; pending preview is discarded; r0c0 returns to X=147.285 and revision remains 7. | Same. |
| Clear X to an empty numeric input | Field is blank, no geometry preview or revision change; Escape restores X=147.285. | Same. |

No divergence or regression was observed in the specified INT1 public workflow. This is behavior evidence for these interactions, not a broader product claim.

## Evidence

- Baseline machine-readable observations: `rerun-check5/{initial,escape-preview,escape-restored,enter-committed,enter-undo,apply-committed,apply-undo,selection-changed,after-inspector-unmount,layout-return-restored,invalid-empty,invalid-escape}.json`.
- Candidate machine-readable observations: `/tmp/frontend-run/int1-behavior-after/` with matching filenames.
- Each full command trace is in the corresponding `steps.json`.

The earlier Case tab/CaseNumber exploration is excluded because it is outside INT1 scope.
