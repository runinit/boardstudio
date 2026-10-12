# Lane prompts

Paste-ready prompts for [parallel-lanes.md](parallel-lanes.md). Each names its worktree, branch and tickets; the
ticket files hold the detail. The orchestrator sets the lane up first (see "Setting up a lane" in that page).
Replace `<agent>` in commit trailers with the app that makes the commit (for example `Codex`, `Claude Sonnet 5.5`).

## Common rules (every lane prompt starts with these)

> Read `AGENTS.md`, `docs/agents/agent-workflow.md`, `docs/agents/verification-process.md` and
> `docs/plans/typed-core-edits/parallel-lanes.md`. Work only in your lane worktree on your lane branch.
> Read the central integration `lane-registry.md` before dispatch/edit/check/verification handoff; compare file/symbol leases
> and escalate actual ownership conflicts before contested edits. Read-only shell commands/searches/diffs/graph queries and approved disjoint source work run freely in separate worktrees. Builds/compilation, test execution, browser, timing and index refresh require the explicit GLOBAL HEAVY slot; protect shared writable outputs and one writer per worktree.
> Use its absolute path as `exec_command`'s `workdir` on every shell call; report the path, branch and status before editing.
> Follow the initial claim rule and standing-authority exception in agent-workflow.md; approved B chain continuation does not wait for another root START/clerical claim checkpoint per private helper, test or fixed-scope child. Reviewers keep awaiting-verification status until acceptance holds. Implement test-first; run only the affected checks, with sequential command execution within each lane. Obtain one complete bounded debugging + affected-check HEAVY cycle, carry approved RED → fix → GREEN without per-command/single-probe handoffs, and release it explicitly. Timing requires the machine alone; resource-heavy runs never overlap. Follow `verification-process.md` for the active app's sandbox policy. Stage explicit paths; no
> `git stash`, no `git add -A`, no push, no merge. Maps (`docs/plans/*/map.md`), `CONTEXT.md` and `docs/adr/*` belong to
> the orchestrator; propose text in your ticket Outcome. Generated goldens (`core/tests/fixtures/boundary/transcript.json`,
> `core/tests/fixtures/demo-projects-baseline.json`) are root-owned; propose required regeneration in your Outcome.
> For affected browser tests or configuration, compile the actual target; intermediate RED/fix work stays in the same writer unit, and the writer stops at a completed evidence-backed handoff. Run GitNexus impact before edits and
> detect-changes before commits, bound to your checkout. Do not refresh GitNexus or register for its Stop hook; confirm
> stale, empty or UNKNOWN caller answers with `rg` and report the limitation. Never run `scripts/check.py release`.
> Use the evidence-reuse review policy in `verification-process.md`: one fresh Sol6.1 medium review for code/behaviour;
> author checks plus root review may cover low-risk docs, mechanical copy and tracking. Implementers finish with
> `Status: implemented (awaiting verification)` and an honest `## Outcome`: commands with
> timings, what you did not run, decisions, deviations. Report exactly what you observed.

## Lane B: mirror core (worktree `.worktrees/lane-mirror-core`, branch `lane/mirror-core`)

### B1: ticket 31, companion placement modes

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/31-companion-placement-modes.md`. Also read
> ADR-0010 (all amendments), tickets 29, 30 and 18 (Verification), `core/src/mirrored_half.rs`,
> `core/src/mechanical/halves.rs`, `core/src/model.rs` (pair and `halves` types). Check steps: `repo`, `lint`,
> native `test`, `typecheck`, and the explicit wasm run `wasm-pack test --headless --chrome web/crates/host
> --locked --features page --lib -- boundary_parity::` only if the transcript changes (it is a generated golden:
> root owns any required `BOUNDARY_BLESS=1` regeneration; report the required entries in your Outcome).

### B2: ticket 32, optional labels

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/32-mirrored-half-labels.md`. Start after B1 is
> merged into your lane (same branch). Check steps as B1. Keep the ticket-26 equivalence test in
> `web/crates/catalogue` passing; shrink its pinned difference list only for the relabelling rows when labels are passed.

### B3: ticket 19, reconcile on every edit (run alone in the lane, after B1 and B2 are resolved)

B owns standing planning → Luna implementation → ONE fresh Sol 6.1 medium acceptance per coherent unit → dependency-ready continuation through105–117. Group tightly coupled private slices when each acceptance and ownership stays explicit; do not require a review/session or root START per ticket number/private helper/test. Preserve full19 acceptance/counterexamples and111 activation gates. The user-approved removal-only wrapper in106 lifts its prior visibility HOLD, with remap_parts/PartChanges remaining private; no blanket visibility waiver. Current grants survive compaction; handoff is notification, not an implicit unfinished-work review or STOP. Root still schedules complete GLOBAL HEAVY cycles and owns refresh/integration/goldens; read-only commands and disjoint source work run freely alongside ordinary checks under the user-approved lighter policy, superseding older all-command restrictions. Escalate only actual new decisions/representability, ownership conflicts, unapproved API/schema/config/visibility or destructive changes, or concrete failed-acceptance scope.

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/19-reconcile-mirrored-halves.md`, including its
> "Contracts from other tickets". It edits the CRITICAL-impact apply path: run GitNexus impact and report its callers and
> risk before changing it; confirm unresolved answers with `rg`. Keep `core/tests/matrix_performance.rs` passing and record
> the added per-edit cost. Check steps: `repo`,
> `lint`, `test`, `typecheck`, wasm boundary parity.

## Lane A: web test speed (worktree `.worktrees/lane-web-sleeps`, branch `lane/web-sleeps`; set up after ticket 36 is resolved)

### A1: ticket 37, remaining fixed sleeps (includes the work carried from 36)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/37-sleeps-layout-library-ui-page.md` including its
> "Carried from 36" section. Read tickets 34 and 36 (Outcome and Verification) first: the lessons there are binding
> (the predicate must be the precondition of the NEXT assertion; negative assertions need a sentinel or a note; every gate
> is bounded), and use `web/crates/ui-shared/src/test_wait.rs` (`wait_until`, `wait_until_step`, `wait_until_future`).
> Check steps: `repo`, `lint`, `typecheck`, `browser` three times in a row at the end, run alone on the machine if you can
> (note `uptime` load before each timed run). Diff the crate `--list` outputs before and after for every crate you touch.
> Mutation-check three waits per suite family. Remove Chromium profiles older than 30 minutes when you finish.

### A2: ticket 38, merge exact duplicate browser tests (after A1 is verified)

> Common rules above. Ticket: `docs/plans/typed-core-edits/issues/38-merge-duplicate-browser-tests.md`. Re-read both
> bodies first; merge only exact duplicates; list what each merge preserves. Check steps as A1.

## Lane F: demo retirement138

> F’s existing coordinator owns ticket138 under its exact manifest and a complete GLOBAL HEAVY debugging/affected-check cycle. Read-only commands and disjoint source work need no global lease; finish/restore a running probe before any writer switch, then fresh review after completed writer STOP. Remove shipped/demo board-specific code/tests; retain shared assertions only through independently specified minimal synthetic fixtures/contracts, never copied demo geometry. Prospective test-only retirement borrowers may port fixtures through an exact handoff amendment with file ownership recorded; no production algorithm bug fixes belong in138. Root owns generated-golden/output removals, refresh and integration. Preserve E137’s synthetic Export regression, shared components, user projects and required mirror/link coverage. Board authoring/reintroduction is deferred.

## Reviewer prompts (Sol 6.1, medium by default; implementers use Luna 6 at the lowest sufficient effort)

Open Codex with the lane worktree as its working folder, paste the common rules, then the prompt below with the ticket named.
Commit trailers: `Co-Authored-By: Codex <noreply@openai.com>`.

> Common rules above, plus: you are the ONE fresh independent REVIEWER after the completed Luna handoff. Use Sol6.1 medium;
> justify xhigh only from actual complexity or escalation. Review the final diff and raw source-bound writer evidence. Reuse
> sufficient checks; rerun targeted verification only for a concrete risk/evidence gap, not another full suite by default.
> Mutations are for important claimed guarantees needing proof; preserve existing counted gates until explicitly reassessed.
> Restore/touch/hash sources after any mutation. Fix only small clearly correct defects (about a screen plus a test); report
> larger blockers in `## Verification`. Resolve only when acceptance holds, recording checks reused/run and unverified items.
