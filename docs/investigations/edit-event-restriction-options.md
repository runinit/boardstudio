# Event edit restriction options

Investigation for [ticket 18](../plans/edit-settlement/issues/18-decide-restrict-event-edit.md), updated against the working tree after ticket 15 merged and while ticket 17 is in progress. This is evidence for a human decision; it does not resolve ticket 18 or make a product decision.

## Current boundary

`Event::Edit` accepts an unrestricted `EditCommand` ([session.rs](../../application/src/session.rs#L241-L255)). The command accepts any `EditOperation`, including `ReplaceDocument` ([model.rs](../../core/src/model.rs#L958-L960), [model.rs](../../core/src/model.rs#L1101-L1104)). On ordinary queued edits, Session overwrites `base_revision` with the accepted document revision before submitting to Core ([session.rs](../../application/src/session.rs#L1595-L1610)); this applies to commits as well as previews. The separate `ResolveEdit` route invokes an owner-captured closure with the accepted snapshot, then resets its command revision to the snapshot used by the resolver ([session.rs](../../application/src/session.rs#L191-L215), [session.rs](../../application/src/session.rs#L1492-L1515)).

The refresh exists to make queued replacement payloads safe only if rebuilt against current state. The existing `queued_discrete_edits_use_each_preceding_durable_revision` test submits two moves captured at revision 0 and asserts the second Core request runs at revision 1 ([durable_session.rs](../../application/tests/durable_session.rs#L591-L620)). Removing refresh globally changes this tested behavior: the second edit will be rejected as stale, requiring resubmission/re-entry. That is the relevant evidence behind ADR-0005's choice to resolve queued user edits on the latest accepted document ([ADR-0005](../adr/0005-resolve-queued-edits-at-execution.md)).

Strict captured-revision checks already exist for protected electrical remap review and export commits ([session.rs](../../application/src/session.rs#L754-L768), [session.rs](../../application/src/session.rs#L1530-L1552), [session.rs](../../application/src/session.rs#L1615-L1645)). Session's separate pointer gesture API also queues a `GestureCommit` with `strict_revision: true` against its captured revision ([session.rs](../../application/src/session.rs#L2190-L2224)). Keep these paths strict. The Layout Transform Toolbar's custom matrix drag is a different path: it submits preview updates through `Event::Edit`, then submits the final drag result through an `EditTicket` resolver ([layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L454-L462), [layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L534-L545), [layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L989-L998)). Do not cite that toolbar drag as a direct commit or as evidence about Session's strict pointer-gesture path.

The `EditResolver` API itself does not establish field scope. Its closure can read and clone the whole accepted document and return any `EditCommand`; the session only requires commit phase ([session.rs](../../application/src/session.rs#L191-L215), [session.rs](../../application/src/session.rs#L1492-L1514)). Ticket 19's separate typed-Core work can improve operation semantics. An event-level restriction alone cannot promise that resolver closures only affect one field.

## Option assessment

### 1. Keep the API and rely on review / the intent path

**Cost:** no production call-site, Core-type, or test migration. Review and the intent path remain the guardrails. The current web source has three production preview submission paths; other textual matches are test fixtures/assertions or event matchers, not outstanding product commits.

**Protects:** nothing at the Rust type boundary against a future caller sending a whole-document replacement on the refreshing path. Existing runtime revision handling continues to work as implemented. Strict export/remap/gesture paths stay independent.

**Concrete misuse:** a newly added name panel clones the accepted `ProjectDoc`, changes `name`, and submits `Event::Edit` with `ReplaceDocument`. If an earlier unrelated edit lands first, Session refreshes the base revision but the old clone still carries the previous document, silently restoring unrelated values. Similar closure behavior remains possible with `ResolveEdit`.

**Tradeoff:** zero migration, but leaves the precise regression ticket 18 asks about possible.

### 2. Restrict `Event::Edit` to previews; all product commits use intents

**Cost:** no product commit migration remains in the current web tree: all three production `Event::Edit` submission paths are preview-only. The old position Inspector calls `submit_position` only with Preview ([inspector.rs](../../web/src/presentation/inspector.rs#L179), [second caller](../../web/src/presentation/inspector.rs#L188)); its Apply/Enter paths use intents. Another is the outline perimeter live preview, guarded by `phase: EditPhase::Preview` before it submits ([outline_lifecycle.rs](../../web/crates/layout/src/outline_lifecycle.rs#L1304-L1322)). The third is `submit_transform_edit` in the Layout Transform Toolbar; its only production callers pass `Preview` during pointer movement and pointer-up sampling ([layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L454-L462), [layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L534-L545), [layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L1364-L1383)). The final toolbar drag commit is `commit_transform_drag` through `EditTicket`, not a direct event ([layout_transform_toolbar.rs](../../web/crates/layout/src/objects/layout_transform_toolbar.rs#L989-L998)). Other textual matches are test-only submissions, event matchers, or test adapters. The Parts direct commit callers previously listed—including module attach—have migrated under ticket 15; module attach now has no `Event::Edit` submit. No production strict direct-commit exception remains on this event. Strict paths are separate Session/API events: captured `GestureCommit`, `ExportCommit`, and `ReviewElectricalRemap` ([session.rs](../../application/src/session.rs#L1530-L1552), [session.rs](../../application/src/session.rs#L1615-L1645), [session.rs](../../application/src/session.rs#L2190-L2224)).

Runtime rejection and a type-level preview-only API are different scopes. A runtime guard in the `Event::Edit` arm can reject `phase == Commit` and retain the current generic payload; it would formalize the current production convention and needs a regression test for rejection. A new event variant carrying a preview-specific command type can make commits unrepresentable through that variant, but `EditCommand` currently has a runtime `phase` field ([model.rs](../../core/src/model.rs#L1115-L1130)); a real type boundary needs a phase-free preview payload or a conversion that cannot accept `Commit`. Current production preview callers can be migrated to that payload; tests that deliberately exercise direct commits may need to use another test path. The separate `GestureCommit`, `ExportCommit`, and `ReviewElectricalRemap` strict paths stay separate and unchanged.

**Protects:** code that follows the intended production boundary cannot accidentally send a product commit through the generic revision-refresh route. Easy visual review of a new call site becomes a useful convention.

**Does not protect:** no type-level protection if `Event::Edit` still accepts a commit command. `ResolveEdit` closures remain capable of returning `ReplaceDocument`; if an intent resolver captures a whole document at commit time rather than deriving it from the accepted snapshot, the resolver path can still overwrite newer state. And broad enforcement needs a separate strict route for preview commands if their captured revision must remain valid.

**Concrete misuse:** a developer adds a product commit to a preview interaction or test helper and sets `phase: Commit`; the type accepts it and Session refreshes the revision. Separately, a resolver clones an old snapshot captured outside the closure and returns it. Neither is stopped by the event name alone.

**Tradeoff:** smallest change that makes product intent usage conspicuous, but it is a convention unless `Event::Edit` also rejects commits at runtime/type level. Keeping legacy `Event::Edit` as preview-only is likely a reversible intermediate step.

### 3. Split operation type into field-scoped and whole-document operations; permit only field-scoped operations on `Event::Edit`

**Cost:** at least the operation definitions and all constructors/pattern matches that cross the new boundary need classification; the full count is not yet evidenced. The Core enum currently contains all ordinary and replacing operations together, with `ReplaceDocument` in the same `EditOperation` as field-level operations ([model.rs](../../core/src/model.rs#L958-L960), [model.rs](../../core/src/model.rs#L1101-L1104)). The named direct production constructors above show the required call-site categories: field setters, one-shot additions, and whole-document replacements. Additional `EditResolver` builders and test fixtures must also be classified. Ticket 19's typed-operation inventory is the prerequisite for a defensible count. If the split changes serialized representation, Core contract tests must cover compatibility; no exact test count is claimed here.

**Protects:** callers using the restricted event cannot provide a `ReplaceDocument` operation if the restricted type contains only true field-scoped operations. This blocks the specific stale-clone overwrite at compile time for that path.

**Does not protect:** the classification is semantic work: some apparently local operations change linked entities or normalized structures. Whole-document operations remain available elsewhere, and any `ResolveEdit` closure returning a whole-document operation bypasses the restriction unless the resolver result is also split. A resolver closure can still modify more fields than its label suggests. It also must not weaken strict captured-revision checks for Session's captured gesture route, export commits, or remap.

**Concrete misuse:** a rename editor routes through a field operation, but a new resolver constructs that field operation by cloning a `ProjectDoc` and independently rewrites other values before submitting a whole-document replacement. If only `Event::Edit` is restricted, the resolver route still admits the overwrite. Alternatively, a developer misclassifies a linked-layout update as a field operation, giving the type a false safety signal.

**Tradeoff:** strongest type boundary for the generic event when classification and resolver boundary are both covered; highest migration and wire-contract risk. A split confined to `Event::Edit` risks overclaiming safety.

### 4. Remove base-revision refresh; make `Event::Edit` strict

**Cost:** no Core type change is needed to stop refreshing, but every direct event caller relying on ordinary queued edits composing would need a behavioral review. One known test conflicts directly: `queued_discrete_edits_use_each_preceding_durable_revision` asserts the second captured revision-0 command is sent to Core at revision 1 ([durable_session.rs](../../application/tests/durable_session.rs#L591-L620)). That test must change if strict rejection is intended. Other dependent tests and production callers are not counted until ticket 17's inventory is complete. Product callers with asynchronous queueing may need explicit retry behavior or migration to `ResolveEdit`.

**Protects:** an edit built from revision R cannot silently apply to R+1 through `Event::Edit`; Core/session rejects it as stale. This prevents stale full-document payloads from replacing newer accepted state through this route.

**Does not protect:** this does not encode operation scope, and a whole-document replacement at a matching revision remains valid. Retained direct-event callers that capture the same revision for rapid sequential edits would see later edits rejected. Migrated product edits use `ResolveEdit`, which independently sets the command's revision from the accepted snapshot at execution ([session.rs](../../application/src/session.rs#L1492-L1515)); their queued composition is preserved. Strictness for the direct event alone therefore need not conflict with ADR-0005's product intent behavior.

**Concrete misuse:** two retained direct callers capture revision 0; the first saves revision 1 and the second is rejected as stale. By contrast, two resolver-backed fields continue composing. A resolver returning an old captured whole-document clone is still dangerous because its revision is refreshed independently of `Event::Edit`.

**Tradeoff:** a small direct-event change with an observable behavioral/test migration for retained direct callers. It adds a stale-revision guard there without defining field scope or constraining resolver outputs. Session's captured gesture route and export/remap strict checks remain independent.

## Provisional cost inventory

Current search evidence across `web`: three production submit sites, all preview-only (old position Inspector, outline perimeter and transform toolbar); remaining matches are test submissions/assertions, adapters, or event matching. There are no current production direct commits or strict direct-commit exceptions through `Event::Edit`. Minimum work remains option-specific: none for keeping the API; preview-only enforcement can formalize the invariant with a runtime guard/test or a preview-specific type; a field/whole split still needs ticket 19's inventory of operation constructors, consumers, resolver outputs, and serialized contracts; strict revision on this direct event still changes the verified `queued_discrete_edits_use_each_preceding_durable_revision` test ([durable_session.rs](../../application/tests/durable_session.rs#L591-L620)) and should be assessed against any tests that intentionally submit direct commits. Ticket 15's direct production commit callers have migrated.

| Option | Verified minimum / named scope |
|---|---|
| Keep API | No required production call-site, Core type, or test edits. |
| Preview-only event | Three production direct-event submit paths remain, all previews: old position Inspector, outline perimeter and transform toolbar. Runtime rejection needs a regression test; a type-only preview event needs a preview-specific payload. No direct product commit exception remains in the current tree. |
| Field/whole split | Inventory all operation constructors/consumers, resolver return paths, and serialized contracts; exact count awaits ticket 19. |
| Strict event | At least one named test changes: `queued_discrete_edits_use_each_preceding_durable_revision`; review all direct callers for queueing behavior. |

## Decision tree after ticket 17

The first human choice is the enforcement scope: should ticket 18 guard only legacy direct product commits on `Event::Edit`, or also constrain whole-document payloads returned by `ResolveEdit`? A staged proposal is to enforce a preview-only direct event first, then use ticket 19's typed-intent work to narrow selected resolver outputs. This is a recommendation for discussion, not an accepted decision.

After that scope choice, ask which enforcement level is acceptable (review convention, runtime rejection, or compile-time restriction) and which operation classification/bulk exceptions it needs. These are dependent questions, not a single frontier round with assumed answers.

Established constraints stay in place: ordinary resolver-backed edits compose against accepted state under ADR-0005; Session's captured gesture route and export/remap operations retain strict captured revisions. Do not re-decide those rules here. The current source classification finds no direct product commit exceptions; ticket 17 should record that inventory as its cleanup work completes. This report leaves the enforcement choice and typed-operation scope to the human decisions in tickets 18 and 19.

## Feasible scope and reversible path

Smallest feasible staged scope: complete ticket 17's inventory, then make the production rule explicit that product commits use `ResolveEdit`, with `Event::Edit` retained for previews and any specifically recorded exceptions. Keep ordinary resolved edits refreshed against their execution snapshot, and preserve strict captured revisions for export commits, remap review, and Session's captured gesture route. This is useful review pressure, but should be described as a convention unless commit phase is actually rejected on the direct event path.

Reversible follow-up: once ticket 19 inventories operations and settles semantic classification, add a distinct typed operation boundary and cover resolver outputs as well as direct event payloads. Do not infer whole-document safety from `EditResolver`: the closure is unrestricted today. The human decision still needs to settle the intended scope and acceptable migration stage above; this report does not answer ticket 18.
