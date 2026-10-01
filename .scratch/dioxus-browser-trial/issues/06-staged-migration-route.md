# Decide the staged route from the trials to a Rust application

Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: 04, 05

## Question

What staged migration route follows from both user-reviewed Dioxus trials,
including a credible way to eliminate all maintained TypeScript/JavaScript
application logic while preserving saved projects and existing workflows?

Use `grilling` and `domain-modeling`, zooming into the research and prototype
answers as needed. Decide workspace migration order, shared Rust browser
infrastructure, remaining CAD/renderer/worker/export/storage work and the
executable Ergogen replacement route. Separate established reuse from work
that still needs a prototype or investigation. Decide compatibility/cutover
gates, regression/performance evidence and when the React app can be retired;
do not claim a schedule or cost without assumptions supported by trial evidence.

If a prerequisite is still uncertain, create a precise decision ticket and
dependency rather than pretending the route is clear. Record the human's
chosen route and bounded implementation handoff. Complete production migration
lies beyond this focused trial effort; new scope requires a separate effort.
