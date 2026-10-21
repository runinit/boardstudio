---
status: deprecated
---

# Constrain geometry scripts and apply their results atomically

## Scope correction (2026-10-20)

The user clarified that user-authored geometry scripting is outside the intended
product scope. The discussion below assumed an intended workflow from existing code;
that premise was incorrect. The earlier agreement about constrained execution does
not authorize exposing, extending or preserving scripting as a product feature.
Manual-versus-script placement ownership is therefore not a product decision to make.

This deprecates the feature premise, not the safety of code that still exists. No
script engine, editor, stored script data or tests are removed by this record;
removal and saved-project compatibility need separately scoped work. Keep existing
execution/output limits and atomic failure behavior while that code remains.

## Earlier decision (deprecated)

User scripts are constrained geometry generators, not unrestricted plugins. Expose approved geometry operations rather than arbitrary document mutation or browser, filesystem or network access, and enforce execution and output limits. Apply a generated result only if the entire attempted evaluation and its output validation succeed; on error or budget exhaustion, report the failure and leave the accepted document unchanged. The user confirmed this boundary on 2026-10-20, choosing predictable bounded generation over a general-purpose plugin system.

Do not publish partial script output or let a script catch budget exhaustion and resume publishing. This is distinct from [ADR-0017](0017-forgiving-vendor-previews-with-preserved-source.md)'s salvage of existing vendor geometry: a script generates an attempted design edit, so its failure must not partially commit that edit. Normal accepted-state persistence and recovery still follow [ADR-0013](0013-save-before-publication-and-explicit-recovery.md). Specific numerical limits remain implementation details; this decision neither claims a process-wide memory bound nor substitutes for a security audit.

Evidence: [Core script evaluation and atomic application](../../core/src/script.rs), `collect` and `apply_scripts`, and existing regressions including `collect_stays_failed_after_a_script_tries_to_catch_the_budget_error` and `apply_scripts_keeps_the_document_when_the_emission_budget_is_exceeded`. The restricted engine registers geometry emission functions, applies work/output budgets and builds a candidate document before publication. These are inspected implementation facts, not newly executed test evidence or recovered historical intent.
