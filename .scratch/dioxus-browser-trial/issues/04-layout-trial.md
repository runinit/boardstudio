# Test layout editing, Undo and persistence in Dioxus

Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: open
Assignee: none
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: 03

## Question

Can the agreed Dioxus browser trial open a copied current keyboard, display its
2D layout, select/edit with preview and commit, Undo, save and reload through
Rust application code while preserving the agreed behavior and responsiveness?

Use the preceding architecture/acceptance answer and `prototype`. Build a
clearly marked, runnable artifact on an isolated trial branch/worktree using
the actual Rust core and contracts. Persistence is explicitly part of the
question: use a separate trial database/origin and copies of fixtures. Provide
one documented launch command, a user-driven walkthrough and observable
committed/draft/revision state. Do not stand in for the user's experience or
resolve the ticket merely because it compiles.

Run the agreed affected Rust and browser checks and live verification via
`agent-browser`. Record what works, what fails and what is omitted, including
ordering/Undo/reload behavior and comparable performance evidence where agreed.
Capture the prototype branch/revision and artifact links. Resolve only after
the live human review; findings may graduate more decisions from the map's fog.
