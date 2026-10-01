# Focused pointer range-cache run (9a5b2098)

This is one focused component session against the provider-built page artifact at `http://127.0.0.1:46911/`, source commit `9a5b20985a9db1f4c0f0d635c43e1dd5439bbfd9`. It is not a maintained release: the focused stage omitted service-worker/offline initialization, and it cannot establish release or offline acceptance. The complete asset map and source/build identity are in `qa-provenance-pointer-range-cache-a3172ee6.json` and the immutable run copy.

The session used the existing 30/100/200-key public `.boardstudio` archives, visible file import, 10 warmups and 100 measured pointer moves per size. The endpoint begins when the native `pointermove` reaches the page capture listener and ends at the first `requestAnimationFrame` opportunity after a `MutationObserver` sees the target SVG transform change. It is not a physical-display presentation measurement. The frozen p95 caps remain 33/50/100 ms.

| Keys | p95 | Existing cap | Result |
|---:|---:|---:|---|
| 30 | 22.7 ms | 33 ms | pass |
| 100 | 96.3 ms | 50 ms | fail |
| 200 | 344.5 ms | 100 ms | fail |

All three scenarios recorded 100 measured samples, 110 changed frames including warmups, and zero missed input markers. The wrapper verdict is `incomplete-or-failed-runs-retained` because it intentionally expects the full five-session requirement; the session itself records `absolute-budget-fail`. Raw commands, environment, samples and failures are retained under `runs/focused-pointer-9a5b2098-20261001T2345Z/`.

This provides focused evidence that the proposed presentation change improves the prior 8f509433 observations at 100 and 200 keys, while still missing both absolute limits. It does not authorize changing those limits or support an M1 performance pass claim. Modifier-based Range/Toggle correctness in the focused stage used DOM-dispatched PointerEvents; this run measures only unmodified native mouse drags.
