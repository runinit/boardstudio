# Decide the trial architecture and acceptance contract

Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: none
Parent: [Dioxus browser trial and Rust application migration](../map.md)
Blocked by: 01, 02

## Question

Given the platform and integration evidence, which isolated Rust/Dioxus trial
architecture and observable acceptance criteria fairly test both layout editing
and case generation before committing to a staged application migration?

Work with the user using `grilling`, `domain-modeling` and, if needed,
`codebase-design`. Choose representative copied saved-project fixtures; the
minimum editing/selection/gesture/Undo/save/reload behavior; one case-setting
edit, exact generation and GPU inspection workflow; and failure/cancellation
cases that are essential to judging the framework. Decide Rust engine/worker
ownership, snapshot/project compatibility boundaries, offline/static-host
expectations and how to compare responsiveness/build/startup/memory evidence
without inventing new performance targets or loosening frozen budgets.

Confirm which temporary omissions are acceptable for the focused trial, and
which Rust implementations are necessary to make its results meaningful.
Preserve the current interaction language and design unless the user explicitly
chooses otherwise. Record the human's choices and the precise questions each
prototype will answer. This decision does not approve production cutover or
claim full-workspace compatibility.
