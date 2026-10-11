# Lane prompts

Paste-ready prompts for [parallel-lanes.md](parallel-lanes.md). Each names its worktree, branch and tickets; the
ticket files hold the detail. The orchestrator sets the lane up first (see "Setting up a lane" in that page).
Replace `<agent>` in commit trailers with the app that makes the commit (for example `Codex`, `Claude Sonnet 5.5`).

## Common rules (every lane prompt starts with these)

> Read `AGENTS.md`, `docs/agents/agent-workflow.md`, `docs/agents/verification-process.md` and
> `docs/plans/typed-core-edits/parallel-lanes.md`. Work only in your lane worktree on your lane branch.
> Read the central integration `lane-registry.md` before dispatch/edit/check/verification handoff; compare file/symbol leases
> and escalate overlaps or unexpected dirty state to root before contested edits. Obtain root's global command window.
> Use its absolute path as `exec_command`'s `workdir` on every shell call; report the path, branch and status before editing.
> Implementers claim a ticket by setting `Status: claimed` and committing; verifiers keep its awaiting-verification status
> until acceptance holds. Implement test-first; run only the check steps the process table names, one command at a time in
> the foreground. Follow `verification-process.md` for the active app's sandbox policy. Stage explicit paths; no
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

Current dispatch uses the reviewed child chain 105–117 in parent 19, beginning with 105 after 39; root releases one
child at a time. The original parent acceptance remains binding; the parent prompt below is reference, not a single-session scope.

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

## Lane F: shipped board artifacts

> Common rules above. Ticket 25 is the canonical board-project contract. F is the existing sole Sofle coordinator. The exact reserved fixture lease is web/assets/fixtures/sofle.boardstudio and its existing web/assets/fixtures/sofle.json companion. Product START remains held until root commits the reviewed claim and explicitly grants it. After grant, prepare the single Sofle artifact with its existing Inspector assembly choices and hand off the artifact plus assembly-swap evidence to ticket 56. Follow ticket 25 for component evidence, shared mirror/link separation, fit limits, and all remaining board sequencing.

## Reviewer prompts (Sol 6.1, medium by default; implementers use Luna 6 at the lowest sufficient effort)

Open Codex with the lane worktree as its working folder, paste the common rules, then the prompt below with the ticket named.
Commit trailers: `Co-Authored-By: Codex <noreply@openai.com>`.

> Common rules above, plus: you are the ONE fresh independent REVIEWER after the completed Luna handoff. Use Sol6.1 medium;
> justify xhigh only from actual complexity or escalation. Review the final diff and raw source-bound writer evidence. Reuse
> sufficient checks; rerun targeted verification only for a concrete risk/evidence gap, not another full suite by default.
> Mutations are for important claimed guarantees needing proof; preserve existing counted gates until explicitly reassessed.
> Restore/touch/hash sources after any mutation. Fix only small clearly correct defects (about a screen plus a test); report
> larger blockers in `## Verification`. Resolve only when acceptance holds, recording checks reused/run and unverified items.
