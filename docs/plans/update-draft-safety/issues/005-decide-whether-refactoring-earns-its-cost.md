# 005: Decide whether refactoring earns its cost

Status: resolved
Type: grilling
Label: wayfinder:grilling
Mode: HITL
Assignee: Chris D
Blocked by: [Choose the draft-safety interface and lifetime contract](004-choose-draft-safety-interface.md)
Parent: [Draft-safe update adoption](../map.md)
Related: [Draft-safety research and change map](../../../research/2026-11-07-019-draft-safety-change-map.md)

## Question

Against the selected draft-safety contract, does any narrowly scoped refactor
both reduce testing effort and improve code quality, enough to earn its cost?
An explicit decision to do no refactor is a complete valid answer.

## Comparison to prepare

- Start with the concrete local candidate: `GeometryScriptsEditor` JSON-encodes its
  name/source/enabled draft while `PendingEditSignals<K, D>` already supports typed
  drafts. Verify current source and compare keeping it unchanged with a typed form.
- Show which complexity, duplicated setup or test effort disappears and what
  tests still cross the real interface. Contrast that with churn, new abstraction,
  owner/lifetime risks and new verification obligations; merely moving lines is not
  evidence of either benefit. Do not claim fewer tests means weaker coverage is safe.
- Use native Gortex search and impact for actual target symbols. Evaluate reuse of
  `PendingEditSignals`/`OwnedEdits` without a broad helper rewrite; inspect a real code
  action before promising a particular mechanical extraction is available.
- Keep behavior-preserving restructuring distinct from adding missing owner reports,
  which changes product behavior and needs its own RED/GREEN proof during implementation.

## Resolution and boundaries

Use `grilling`, `domain-modeling` and `codebase-design`. The user selects none or a
precise justified scope, with evidence for **both** benefits, unchanged safety proof,
ordering and rollback considerations. Record the decision here, not by editing product
code. Any later execution uses `gortex-refactor`, signature verification if applicable,
and post-change detect/tests/guards/contract plus the required behavioral gates.
Claim first. Refactoring is not a prerequisite unless this decision makes it one.

## Answer

Resolved 2026-11-07 by live grilling (two rounds; user accepted all
recommendations and chose revisit option (a)). **Decision: NO refactor is
included in this effort.** `GeometryScriptsEditor`'s JSON-encoded
name/source/enabled draft stays unchanged; refactoring is not a prerequisite
for [Agree the verification plan and implementation handoff](006-agree-verification-and-implementation-handoff.md)
or for implementation.

### Why it does not earn inclusion (charter bar: BOTH benefits, with evidence)

- **Code quality: real but modest gain, not sufficient alone.** Typing the
  draft as `PendingEditSignals<ScriptActionKey, ScriptDraft>` would delete
  `encode_script_draft` (`web/crates/ui-shared/src/geometry_scripts.rs:100`),
  ~8 encode sites, the `from_str` decode effect (:209) and its silent
  parse-failure branch, and would match existing typed precedents (module
  profile's `ModuleProfileDraft`, legacy position's `(String, String)` tuple,
  mounted-module placement). Native Gortex: conversion touches only
  `geometry_scripts.rs`; `PendingEditSignals<K, D>`
  (`web/crates/ui-shared/src/pending_edit_helpers.rs:136`) already supports a
  typed `D` via `begin_value`/`settle`, so no helper change is needed (the
  helper's own change impact is CRITICAL, 13 direct / 17 transitive
  dependents; the editor's is MEDIUM, 2 direct / 6 transitive). The three
  separate `name`/`source`/`enabled` signals and their manual sync remain
  either way, and typing does not fix the `Apply("")` shared-key collapse.
- **Reduced testing effort: NOT shown.** The 4 existing wasm tests in
  `geometry_scripts.rs` (`many_scripts_use_a_searchable_picker`,
  `held_script_failure_is_retired_when_active_script_changes`,
  `script_apply_queued_behind_an_unrelated_edit_keeps_that_edit`,
  `two_queued_new_scripts_both_exist_and_apply_on_a_deleted_script_retires`)
  assert none of JSON round-trip, string equality or dirty detection; typing
  deletes 0 tests, adds 0, and leaves setup/assertions crossing the same real
  interface. Future registry-adapter tests would be near-identical under
  either form. Merely moving lines is not evidence of reduced effort, so the
  both-benefits condition in the [map charter](../map.md) fails.
- **No mechanical shortcut exists.** Gortex `change code_actions` on
  `geometry_scripts.rs` fails (no rust-analyzer / `lsp.code_actions`
  disabled); any conversion would be a manual one-file type-shape change,
  with `change edit_plan` confirming the single-file touch list.

### Boundaries settled with the user

- Adding Geometry scripts' missing owner report (the `is_unresolved`
  registry adapter under [Choose the draft-safety interface and lifetime contract](004-choose-draft-safety-interface.md))
  is **product behavior for implementation, not refactoring**: it proceeds
  regardless of this decision, with its own RED/GREEN proof, comparing the
  live encoded draft against the owner-canonically encoded live-projected
  baseline / in-flight submission per admission key.
- **Safety proof unchanged:** the packaged A→B assertion from
  [Establish a real-editor draft-loss probe](002-establish-real-editor-draft-loss-probe.md),
  the 4 Geometry tests passing unmodified, and the packaged proof to be
  fixed by [Agree the verification plan and implementation handoff](006-agree-verification-and-implementation-handoff.md)
  all remain mandatory. No test is removed or weakened by this decision.
- **Revisit trigger (option (a)):** final for this effort and excluded from
  the verification/handoff scope. Reopening requires a fresh ticket carrying
  new evidence of BOTH benefits (e.g. adapter tests against the encoded form
  demonstrating concrete duplicated setup, or an actual encoding/equality
  defect) — not a re-argument of today's evidence, and never opportunistic
  conversion during implementation. If ever approved later, it lands as its
  own behavior-preserving commit before adapter work (rollback = revert that
  commit alone), never interleaved. The user's goal of a minimal, tight
  codebase is recorded by parking the opportunity, with this evidence, in
  [Duplication & refactoring opportunities](../../../research/duplication-refactoring-opportunities.md)
  rather than letting it creep into this effort's scope.

No product code changed in this ticket; no ADR (no existing decision is
reversed, and `PendingEditSignals` usage follows its established typed
precedents).
