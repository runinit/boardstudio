# Corrected public Case keyboard and validation checks

Candidate build `ed242c02` at `http://127.0.0.1:34665/`, paired with the pinned React reference at `http://127.0.0.1:5173/`. Each check used fresh task-owned disk profiles and a public import of the same Sofle archive. No source changes or direct storage writes were used.

## Corrected key invocation

The valid agent-browser form is `agent-browser press Enter` / `agent-browser press Escape` with focus already on the intended control. Earlier records that used `press @e... Enter`, `press @e... Escape`, or `press @e... Space` are preserved but are invalid keyboard evidence; see `case-drafts/RESULTS.md` and `ESCAPE_DRIVER_NOTE.md`. These corrected reruns supersede those outcomes.

## Enter commits and invalid-value feedback

On a fresh imported Sofle project, the candidate and reference each publicly created a case body. With the Thickness spinbutton focused and showing `4.5`, a real `press Enter` committed thickness 4.5 in both; document revision advanced from 5 to 6. `candidate-enter-before.json`, `candidate-enter-after.json`, and corresponding `reference-enter-*` files record focus/value and persisted result.

In separate fresh sessions, Thickness was filled with `-1` while focused, then a real `press Enter` was sent. Both apps displayed `Enter a value above 0.` and marked the input invalid. Persisted thickness remained 3 and revision remained 4 in both. Before/after focus and field validity are in `invalid-{candidate,reference}-{before,after}-enter.json`; readonly persisted records are `invalid-{candidate,reference}-after.json`; final accessibility snapshots and same-state screenshots are included.

## Escape draft behavior

With persisted Z offset 0, a focused input draft of 1.5, and a real `press Escape`, candidate visibly reset the focused input to 0 and React visibly retained 1.5. Both persisted records remain Z=0 at revision 5. This is a public UI difference only; saved documents did not diverge. Before/after DOM state, saved records, and accessibility snapshots are `candidate-escape-*` and `reference-escape-*`.

Screenshots: `candidate-invalid.png` and `reference-invalid.png`.
