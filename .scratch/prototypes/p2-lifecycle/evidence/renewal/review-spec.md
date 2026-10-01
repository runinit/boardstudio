# Independent Spec review: P2-r1-R1

Reviewer: fresh `gpt-6-luna/high`, read-only. Compared integration base
`689f8962f77e04b1150a9541128fec7aca28addc` with candidate
`57bd6eceed3bce9ad9b854121e93d8adc22f7bf8`; executable source is
`b2cf39db55db7826368031e61bb1b033bb652800`.

**Required — executable acceptance blocked:** [web.rs](/home/chris/.local/share/boardstudio/worktrees/p2-lifecycle-r1-20261001/.scratch/prototypes/p2-lifecycle/wasm/src/web.rs:365) samples the SVG coordinate frame before `svg.focus()` at line395, then pointermove/up remeasure the shifted frame. At1280×577, focus scrolls139px; the actual worker commits the copied fixture target from `(147.285,-51.02)` to `(160.4281,-122.4877)`, offboard. This violates [SPEC-editor-session.md](/home/chris/.local/share/boardstudio/worktrees/p2-handoff-20261001/SPEC-editor-session.md:147): preserve characterized coordinate behavior. Evidence: `../renewal-worker/p2-offboard-focus-repro.json`. Repair budget2/2 is exhausted; no source fix was attempted.

Provider core formatting/strict Clippy and renderer strict Clippy fail. Accessibility is unverified: the raw axe run/report was not retained, though the coordinator directly observed an empty `html.lang`; contrast, screen-reader and focus-equivalence evidence are incomplete. Do not accept or integrate executable P2.

**Status-only handoff:** current renewal authority, task status, evidence and boundaries identify the runnable candidate and its blockers. One inherited index, [README.md](/home/chris/.local/share/boardstudio/worktrees/p2-handoff-20261001/.scratch/prototypes/p2-lifecycle/README.md:3), still calls the predecessor P2 preflight unstarted and links the old authority. It is recorded as historical in current task status; use the approved renewal status and `docs/migration/RUN.md` for current state. No other scope/ownership finding.
