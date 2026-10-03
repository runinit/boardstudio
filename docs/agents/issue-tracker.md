# Migration issue tracker

This bounded migration uses local Markdown. Canonical task status is root
[TODO](../../TODO.md), plan [P1-r1](../../tasks/plan.md) and run ledger
[RUN](../migration/RUN.md). The P1-U64 repair issue and exact authority are
[recorded in task-start](../../.scratch/prototypes/p1-core/evidence/u64-repair/task-start.json).
Historical Wayfinder decision tickets remain under `.scratch/dioxus-browser-trial/`.
No remote issue publication or tracker change is authorized by this mapping.

The original P1-CAD P1-r1 blocked continuation is recorded in the [run ledger](../migration/RUN.md#p1-cad-blocked-reference-comparison) and [preserved task/review record](../../.scratch/migration-specs-validation/p1-cad-blocked-review.json). Its failed worktree remains intact.

Current P1-CAD-F1-r1: [bounded findings repair](../../.scratch/prototypes/p1-cad/evidence/findings-repair/PLAN.md), [exact authority](../../.scratch/prototypes/p1-cad/evidence/findings-repair/task-start.json), and [current ledger](../migration/RUN.md#p1-cad-f1-bounded-findings-repair). Only the coordinator updates local status; no remote issue operation or automatic production adoption follows.

P2-LIFECYCLE P2-r1: [approved plan](../../.scratch/prototypes/p2-lifecycle/PLAN.md),
[authority](../../.scratch/prototypes/p2-lifecycle/evidence/task-start.json) and
[blocked preflight](../migration/RUN.md#p2-r1-preflight-blocked). Exact formatting
prerequisite is proposed, not applied; all executable/runtime work remains open.

Current P2-r1-R1: [approved human execution decision](../../.scratch/dioxus-browser-trial/issues/08-choose-p2-execution-policy.md),
[exact task authority](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/task-start.json)
and [current status](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/task-status.json).
Historical failures remain. The renderer formatter prerequisite passes;
provider fmt/strict Clippy, prototype accessibility and short-viewport focus-scroll
drag failures block acceptance.
The r2 release runs locally while bounded independent verification finishes;
both renewed repair budgets are exhausted and P2 stays unchecked.

## Current production M1 run

The historical checkpoints above are retained for provenance. P2/P3 feasibility
is accepted at continuation `cd1efb34`; the current remaining M1 task set is
[the production plan](../../.scratch/m1-production/PLAN.md),
[specification](../../.scratch/m1-production/spec.md), six local tickets under
`.scratch/m1-production/issues/`, and
[canonical state](../migration/m1-production-run.json).
Root TODO and RUN govern closure after integrated verification; partial native
or probe evidence never closes the complete production workflow.

## Current Dioxus frontend v1 continuation

The user’s 2026-10-01 clarification makes the active scope frontend-only. The
[frontend roadmap](../migration/DIOXUS-FRONTEND-V1.md),
[specification](../../.scratch/dioxus-frontend-v1/spec.md),
[task graph](../../.scratch/dioxus-frontend-v1/PLAN.md) and
[current state](../migration/dioxus-frontend-v1-run.json) cover F1–F9.
Root TODO/RUN remain canonical pointers. M1 acceptance evidence and limits stay
separate; placeholders do not complete the corresponding full frontend milestone.


### Refactoring observations during the frontend port

The user requires architectural, design, theoretical and software-quality issues
encountered during the rewrite to be retained for the later major refactoring phase.
Authors report new findings in their single packet receipt. The coordinator updates
the [machine register](../../.scratch/dioxus-frontend-v1/refactor-findings.json);
the [post-port takeaways](../migration/POST-PORT-REFACTOR.md) are generated from it.
A scoped “No new refactoring takeaway observed” stays in the receipt. Use stable RF IDs,
source evidence, confidence, impact, current mitigation, later proposal and validation.
Required parity/correctness remains current work; deferred structural cleanup never
waives acceptance or authorizes unrelated API/schema changes. F9 consolidates the
register into the post-port refactoring handoff.

### Frontend agent model policy

The latest user decision assigns independent reviews to Sol 6.1 High. Historical
Astra reviews remain retained, and the earlier Astra High/Extra High bug-fixing
allocation remains available. Use Luna Medium for ordinary bounded implementation, Low for mechanical packets,
and High for specified state/gesture/async/adapter work. Follow the
[dispatch/model policy](../../.scratch/dioxus-frontend-v1/AGENT-ROUTING.md) and
[task routing](../../.scratch/dioxus-frontend-v1/agent-policy.json). Prepare exact
packets only for runnable work, retain all acceptance/review/RF gates, and record
requested versus observable runtime settings. Do not change global model/provider
configuration or invent a per-agent Fast parameter.


### Automatic frontend ticket frontier

On 2026-10-02 the user authorized implementation and automatic creation of next
bounded tickets through agents. The [authority](../../.scratch/dioxus-frontend-tranche-1/AUTHORITY.md),
[published tickets](../../.scratch/dioxus-frontend-tranche-1/issues/01-private-workspace-composition.md)
and [execution state](../../.scratch/dioxus-frontend-tranche-1/execution.json) govern
this run. Apply to-tickets vertical slicing, true blockers and per-file publication;
routine ticket breakdown/publication no longer requires another user quiz.
Source-check the consumed capabilities, refine the existing spec/ticket and
implement without a separate planning approval. The user’s 2026-10-03 direction
reduces repeated testing: one required affected compile/check, one changed paired
browser journey and one consolidated candidate review, reusing unchanged evidence.
Keep shared integration serial and six queues active, with one waiting packet per
stream. Necessary migration API/design/visibility changes are authorized by the
2026-10-02 user decision in CONSTRAINTS.md; external cutover remains separate. Every handoff continues to record RF takeaways.


### Frontend delivery records

**Current execution phase — user update 2026-10-03:** integrate more, test later.
Authors implement and freeze mounted work without a new per-packet test/browser/review
cycle. The coordinator integrates a larger coherent candidate before packaging,
changed public qualification and the single consolidated review. Checks already
running may finish; results are retained. Source integration is not parent acceptance.
Final criteria, history/reopen evidence and real dependency joins remain required
before acceptance, using sufficient existing proof where unchanged.

Read current progress with `python3 .scratch/dioxus-frontend-v1/progress.py show`.
It reads the [live operations record](../migration/dioxus-frontend-v1-run.json)
and derives parent counts from the [canonical graph](../../.scratch/dioxus-frontend-v1/tasks.json).
Use `show --json` for the current fields and `check` for record consistency.
Historical checkpoints are disclosed through the live record's `history` pointer.

1. Authors refine the existing issue/spec, implement in their isolate, and return
   one commit plus one receipt pointer. The receipt names the changed React journey,
   actual checks and limitations, and any new RF evidence. A chat message with these
   references is sufficient for queueing; another handoff document is optional.
2. The coordinator joins ready commits serially, updates only `current_progress`
   for the served candidate and six queues, and runs one combined affected check.
   RUN, PLAN, stream reconciliation and issue prose link here instead of maintaining
   another live counter, candidate summary or synchronized JSON state.
3. Sol reviews the frozen candidate once. Finalize one candidate review/AUDIT pair
   at a stable repository path. Parent decisions reference that pair by path/hash;
   they own their criterion verdict and retained joins. Reuse unchanged evidence.
   Earlier review copies remain historical; create no future per-parent review copies.
4. The coordinator changes a parent with `progress.py set-status <ID> <status>
   --reason <reason>`; acceptance also requires `--decision <DECISION.json>`.
   The command checks final joins and review hashes, records a decision reference,
   and refreshes derived counts and the readable RF report. Parent criteria and
   the dependency graph remain in their existing canonical rows.
5. For a new architecture/design/theory/quality observation, the coordinator updates
   the RF JSON once, then runs `progress.py sync`. The readable report is generated.
   A no-new-finding note stays in the packet receipt and creates no ledger copy.

Historical issue status and boundary-gap paragraphs can describe the planning baseline.
Verify the relevant current source before treating them as a missing capability or
creating another implementation. The canonical graph owns parent status; the live
record owns current operations. An assigned source base remains usable across
coordinator record-only and unrelated implementation commits. Refresh it only
when a consumed interface or overlapping owned source changes; do not create
another isolate, rebase or wait for base/ownership confirmation just to follow HEAD.

After the coordinator joins a packet, its author continues the next real gap in
the assigned ownership without another dispatch/approval handoff. Publish a shared
helper's settled signature and source early when other streams consume it; those
streams continue independent controls while the helper is prepared. Keep one owner
per overlapping source area and merge narrow mounts serially. Current-source checks
prevent stale issue prose from generating another implementation of existing code.

A combined compiler failure produces one repair list routed to the original
owners. Repair the affected captures/types in the existing source; do not reopen
unchanged specs, create duplicate tickets, repeat reviews or restart unrelated
work. Later packets retain already joined fixes. The coordinator runs
the [CI's Dioxus source compile](../../.github/workflows/check.yaml) on the combined
batch and repairs compiler diagnostics before retrying the full preview package. This is compilation only;
behavioral testing and consolidated review remain deferred under the current
source-first instruction. A build never counts as acceptance.

This replaces per-slice planning/source/merger approvals and repeated document
publication for the active frontend run. New tickets are created only for distinct
runnable work; existing tickets are refined for clarifications or discovered gaps.
Necessary API/design changes retain the user authorization in CONSTRAINTS.md.
Cutover and external publication remain separate. Browser authors own named sessions
and profiles; lifecycle cleanup is restricted to that named session.
