# Test case generation and the Rust 3D renderer in Dioxus

Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: open
Assignee: none
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: 03

## Question

Can the agreed Dioxus browser trial change a keyboard's case settings, generate
the exact case with the existing Rust CAD engine, display and inspect it using
the Rust GPU renderer, and preserve the agreed editing/persistence behavior
without maintaining TypeScript application adapters?

Use the architecture/acceptance answer and `prototype`. Reuse the isolated trial
infrastructure established by either prototype; layout implementation is not a
prerequisite for this question because a copied fixture supplies case inputs.
Keep expensive CAD work off the UI thread using the agreed Rust worker design.
Exercise progress/cancellation or supersession, revision/context protection,
mesh transfer, canvas mount/unmount/resize and camera interaction to the extent
the acceptance answer requires. Preserve exact geometry and export readiness;
document any untested export or full-assembly behavior rather than assuming it.

Provide a launch command and walkthrough. Run affected Rust/CAD/renderer checks
and live browser verification with `agent-browser`; record responsiveness and
evidence limits without claiming Dioxus speeds up kernel computation. Persist
only in the trial namespace. Capture branch/revision and artifact links, then
ask the user to judge the real trial. Resolve only through that live review.
