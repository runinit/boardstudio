# Focused pointer Inspector cache run (`e7d29ce6`)

This is one focused component session against `http://127.0.0.1:46912/`, source commit `e7d29ce6dd703a4d9b064cb842366538987cf442`. The candidate adds the reviewed Inspector `Rc` item cache on top of the range-ID `Rc` candidate. It is not a maintained release: the focused stage omits service-worker/offline initialization. Exact source and 46-asset hashes are in `qa-provenance-inspector-cache-e7d29ce6.json`, which is copied into the immutable run record.

The run imported the existing 30/100/200-key public `.boardstudio` archives through the visible file input and used 10 warmups plus 100 measured native browser mouse moves for each size. The endpoint starts in the capture listener when `pointermove` is received and ends at the first `requestAnimationFrame` opportunity after `MutationObserver` sees the target SVG transform change. It is not a physical-display presentation measurement. The existing p95 caps remain 33/50/100 ms.

| Keys | p95 | Existing cap | Result |
|---:|---:|---:|---|
| 30 | 18.6 ms | 33 ms | pass |
| 100 | 24.1 ms | 50 ms | pass |
| 200 | 33.3 ms | 100 ms | pass |

Each size has 100 measured samples, 110 changed frames including warmups, and zero missed input markers. The per-session run verdict is `absolute-budgets-pass`. The wrapper summary was emitted before its single-session label bug was corrected and incorrectly says `five-session-absolute-budgets-pass` while also recording `sessionsRequested: 1`; preserve that raw summary as generated. The wrapper now emits a focused session label for future non-five-session runs. This result is a single focused component observation, not five-session, paired, final-release, or M1 acceptance evidence. The prior 9a5b focused run remains separately preserved as a failure (p95 22.7/96.3/344.5 ms).
