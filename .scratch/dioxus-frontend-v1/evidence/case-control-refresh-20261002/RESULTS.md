# Case workspace control placement refresh

This is a visual/control inventory using the default Sofle v2 demo in isolated browser profiles, with no Case edits. Reference source is TypeScript `5a472a9426e6e38993361da402cd4ec730feb369`; candidate is Dioxus source `fc5e17e3526afe8a2c55efe9afdbc0f0faff3efa` (build `frontend-case-keycaps-pcb-reviewgreen-20261002`).

- [TypeScript capture](typescript.png)
- [Dioxus capture](dioxus.png)

Both expose the left/right Case assembly tree and Case construction/Case stack controls in the Inspector. Their central control surfaces differ: TypeScript places `Configure case` inside the Case preview header and renders Shaded/Wireframe/Hybrid, hidden-line and camera controls around the canvas; Dioxus places an always-visible `Generate case` action in its Case assembly header, presents `Configure mechanical stack` in the Inspector and does not show those same view controls for this ungenerated demo state. Dioxus's empty canvas and enabled Generate button are separately tracked in the exact-fixture readiness/projection evidence.

These demo captures are not the exact layered-Sofle import and do not establish generation parity. The paired public acceptance still needs the same imported fixture, matched setup state, actual control interactions, accepted edits and save/reopen.
